//! Fetching an address off the open web: the one place Statac speaks to
//! the network. The fetching commands reach out from the author's own
//! machine to addresses a stranger chose, so every hop of the way is
//! judged before anything is sent: `http` and `https` only, no name or
//! password in the address, every address the host stands for judged as
//! one the open internet could rightly hold, the connection made only to
//! addresses that were judged, and no proxy from the environment.
//!
//! A fetch takes the most bytes the caller will take, and a most of time
//! for the whole of it. It asks for no compressed encoding and
//! decompresses nothing, sends no cookie, and keeps nothing of one fetch
//! for the next. The client follows no redirect of its own: the fetcher
//! reads where each redirect points, judges that hop like a first
//! address, and follows five at most.
//!
//! A fetch gives the bytes back, or one refusal saying exactly why not.
//! A refusal carries the bare facts; how it is worded for the author
//! belongs to what shows it.

use std::io::{self, Read};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use rustls::crypto::aws_lc_rs;
use ureq::config::Config;
use ureq::http::{HeaderMap, Uri, header};
use ureq::tls::{RootCerts, TlsConfig, TlsProvider};
use ureq::unversioned::resolver::{ResolvedSocketAddrs, Resolver};
use ureq::unversioned::transport::{DefaultConnector, NextTimeout};
use ureq::{Agent, Error as UreqError};
use url::{Host, ParseError, Url};

/// The whole of a fetch, name lookups and redirects included.
const TIME: Duration = Duration::from_secs(30);

/// The most redirects a fetch follows. The answer that redirects a sixth
/// time is refused.
const REDIRECTS: u32 = 5;

/// The most addresses of one hop given to the client, as the client keeps
/// no more itself.
const ADDRESSES_KEPT: usize = 16;

/// What a fetch brought back.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Fetched {
    /// The bytes the answer carried.
    pub bytes: Vec<u8>,
    /// The address the bytes finally came from, after any redirects.
    pub from: Url,
    /// The content type the server gave, if any, as text.
    pub content_type: Option<String>,
}

/// Why a fetch brought nothing back. The facts are bare: how a refusal
/// is worded for the author belongs to what shows it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Refusal {
    /// The address could not be read as an address at all.
    NotAnAddress,
    /// A scheme other than `http` or `https`.
    Scheme { scheme: String },
    /// An `http` or `https` address with no host.
    NoHost,
    /// The address carries a user name or password.
    Credentials,
    /// No name server answered for the host.
    UnknownHost { host: String },
    /// The host stands for an address off the open web.
    PrivateAddress { host: String, address: IpAddr },
    /// The server answered `404` or `410`, that it has no such thing,
    /// which a caller remembers and stops asking about.
    Missing { status: u16 },
    /// The server answered a status that is neither a success nor a
    /// redirect.
    Status { status: u16 },
    /// The server redirected more times than the fetch follows.
    Redirects { limit: u32 },
    /// The server redirected with nowhere usable to go.
    NoRedirectTarget {
        /// The host the redirect came from.
        host: String,
        /// Where it pointed, as it read, if it read at all.
        location: Option<String>,
    },
    /// The answer held more bytes than the fetch was given a limit for.
    TooLarge {
        limit: u64,
        content_length: Option<u64>,
    },
    /// The fetch ran out of time.
    TimedOut,
    /// The secure connection to the host could not be made.
    Tls { host: String },
    /// Anything else the network or the system said.
    Failed { reason: String },
}

/// Fetches the bytes at `address`, judging the host before anything is
/// sent and each hop of a redirect the same way, or refuses.
pub(crate) fn fetch(address: &str, limit: u64) -> Result<Fetched, Refusal> {
    run(&Plan::for_a_fetch(), address, limit)
}

/// What a fetch is given beyond the caller's address and limit: the whole
/// time it may take, how a name is looked up, and the addresses it may
/// reach however they judge. A fetch looks names up with the system,
/// within its time, and is allowed nowhere.
struct Plan {
    total: Duration,
    lookup: Lookup,
    allowed: Vec<SocketAddr>,
}

impl Plan {
    fn for_a_fetch() -> Plan {
        Plan {
            total: TIME,
            lookup: Lookup::System,
            allowed: Vec::new(),
        }
    }

    #[cfg(test)]
    fn for_a_test(lookup: Lookup, allowed: Vec<SocketAddr>, total: Duration) -> Plan {
        Plan {
            total,
            lookup,
            allowed,
        }
    }
}

/// How a name is looked up.
enum Lookup {
    /// The system's lookup of names, done on another thread so a lookup
    /// that never answers cannot hold the fetch past its time.
    System,
    /// A lookup stood in for by a test, answering what the test says.
    #[cfg(test)]
    StandIn(StandInLookup),
}

#[cfg(test)]
type StandInLookup = Arc<dyn Fn(&str) -> io::Result<Vec<SocketAddr>> + Send + Sync>;

fn run(plan: &Plan, address: &str, limit: u64) -> Result<Fetched, Refusal> {
    let deadline = Instant::now() + plan.total;
    let mut url = Url::parse(address).map_err(|error| match error {
        ParseError::EmptyHost => Refusal::NoHost,
        _ => Refusal::NotAnAddress,
    })?;
    let judged = Arc::new(Mutex::new(None));
    let mut redirects = 0;
    loop {
        let within = deadline
            .checked_duration_since(Instant::now())
            .ok_or(Refusal::TimedOut)?;
        let host = judged_hop(plan, &url, within, &judged)?;
        let agent = agent_for(within, &judged);
        let mut response = agent
            .get(url.as_str())
            .call()
            .map_err(|error| refused(error, &host))?;
        let status = response.status().as_u16();
        if is_a_redirect(status) {
            redirects += 1;
            if redirects > REDIRECTS {
                return Err(Refusal::Redirects { limit: REDIRECTS });
            }
            url = where_it_pointed(response.headers(), &url)?;
            continue;
        }
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        if !(200..300).contains(&status) {
            return Err(match status {
                404 | 410 => Refusal::Missing { status },
                _ => Refusal::Status { status },
            });
        }
        let body = response.body_mut();
        let claimed = body.content_length();
        if let Some(length) = claimed.filter(|length| *length > limit) {
            return Err(Refusal::TooLarge {
                limit,
                content_length: Some(length),
            });
        }
        let mut taken = body.as_reader().take(limit.saturating_add(1));
        let mut bytes = Vec::new();
        taken.read_to_end(&mut bytes).map_err(body_failed)?;
        if bytes.len() as u64 > limit {
            return Err(Refusal::TooLarge {
                limit,
                content_length: claimed,
            });
        }
        return Ok(Fetched {
            bytes,
            from: url,
            content_type,
        });
    }
}

/// Judges one hop before anything is sent to it: the scheme, any name or
/// password in the address, then the host. A name is looked up within
/// what remains of the time, and every address it stands for is judged;
/// one rightly refused refuses the whole hop, and nothing is sent to any
/// of them. What is judged is given to the client as the only addresses
/// the hop may connect to.
fn judged_hop(
    plan: &Plan,
    url: &Url,
    within: Duration,
    judged: &Arc<Mutex<Option<Granted>>>,
) -> Result<String, Refusal> {
    let scheme = url.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(Refusal::Scheme {
            scheme: scheme.to_owned(),
        });
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(Refusal::Credentials);
    }
    let Some(host) = url.host() else {
        return Err(Refusal::NoHost);
    };
    let Some(port) = url.port_or_known_default() else {
        return Err(Refusal::NoHost);
    };
    let (host, candidates) = match host {
        Host::Domain(domain) => (domain.to_owned(), looked_up(plan, domain, port, within)?),
        Host::Ipv4(ip) => (ip.to_string(), vec![SocketAddr::new(IpAddr::V4(ip), port)]),
        Host::Ipv6(ip) => (ip.to_string(), vec![SocketAddr::new(IpAddr::V6(ip), port)]),
    };
    let mut addrs = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        if connectable(plan, candidate) {
            addrs.push(candidate);
        } else {
            return Err(Refusal::PrivateAddress {
                host: host.clone(),
                address: candidate.ip(),
            });
        }
    }
    addrs.truncate(ADDRESSES_KEPT);
    *judged.lock().unwrap_or_else(PoisonError::into_inner) = Some(Granted {
        host: host.to_ascii_lowercase(),
        port,
        addrs,
    });
    Ok(host)
}

/// Whether this exact address may be connected to: one a test stands in
/// for, or one the open internet could rightly hold.
fn connectable(plan: &Plan, address: SocketAddr) -> bool {
    plan.allowed.contains(&address) || on_the_open_web(address.ip())
}

/// Whether the open internet could rightly hold an address, worked from
/// the IANA registries of special-purpose addresses: every range in them
/// is refused, as is anything multicast, broadcast or reserved.
fn on_the_open_web(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(v4) => ipv4_on_the_open_web(v4),
        IpAddr::V6(v6) => ipv6_on_the_open_web(v6),
    }
}

fn ipv4_on_the_open_web(a: Ipv4Addr) -> bool {
    let [first, second, third, _fourth] = a.octets();
    // 224.0.0.0/4 multicast, 240.0.0.0/4 reserved, limited broadcast with
    // them, which the registries hold as special-purpose or unroutable.
    first < 224
        && !matches!(
            (first, second, third),
            // "This network" (RFC 791).
            (0, _, _)
                // Private-Use (RFC 1918).
                | (10, _, _)
                // Shared Address Space, carrier-grade NAT (RFC 6598).
                | (100, 64..=127, _)
                // Loopback (RFC 1122).
                | (127, _, _)
                // Link Local (RFC 3927), the cloud metadata service with them.
                | (169, 254, _)
                // Private-Use (RFC 1918).
                | (172, 16..=31, _)
                // IETF Protocol Assignments (RFC 6890).
                | (192, 0, 0)
                // Documentation, TEST-NET-1 (RFC 5737).
                | (192, 0, 2)
                // AS112 nameservers (RFC 7535).
                | (192, 31, 196)
                // Automatic Multicast Tunneling (RFC 7450).
                | (192, 52, 193)
                // The retired 6to4 relay anycast (RFC 7526).
                | (192, 88, 99)
                // Private-Use (RFC 1918).
                | (192, 168, _)
                // AS112 direct delegation (RFC 7534).
                | (192, 175, 48)
                // Benchmarking (RFC 2544).
                | (198, 18..=19, _)
                // Documentation, TEST-NET-2 (RFC 5737).
                | (198, 51, 100)
                // Documentation, TEST-NET-3 (RFC 5737).
                | (203, 0, 113)
        )
}

fn ipv6_on_the_open_web(a: Ipv6Addr) -> bool {
    let s = a.segments();
    // An address carrying an IPv4 address inside it is judged by the IPv4
    // address it carries: IPv4-mapped ::ffff:0:0/96 and the retired
    // IPv4-compatible ::/96 (RFC 4291).
    if s[0..5].iter().all(|&n| n == 0) && (s[5] == 0 || s[5] == 0xffff) {
        let octets = [(s[6] >> 8) as u8, s[6] as u8, (s[7] >> 8) as u8, s[7] as u8];
        return ipv4_on_the_open_web(Ipv4Addr::from(octets));
    }
    !matches!(
        s,
        // 64:ff9b::/96, NAT64 translation (RFC 6052).
        [0x64, 0xff9b, 0, 0, 0, 0, _, _]
            // 64:ff9b:1::/48, local-use translation (RFC 8215).
            | [0x64, 0xff9b, 1, _, _, _, _, _]
            // 100::/64, discard-only (RFC 6666).
            | [0x100, 0, 0, 0, _, _, _, _]
            // 100:0:0:1::/64, a dummy prefix (RFC 9780).
            | [0x100, 0, 0, 1, _, _, _, _]
            // 2001::/23, IETF protocol assignments (RFC 2928), Teredo with them.
            | [0x2001, 0..=0x1ff, _, _, _, _, _, _]
            // 2001:db8::/32, documentation (RFC 3849).
            | [0x2001, 0xdb8, _, _, _, _, _, _]
            // 2002::/16, the retired 6to4 (RFC 7526).
            | [0x2002, _, _, _, _, _, _, _]
            // 2620:4f:8000::/48, AS112 direct delegation (RFC 7534).
            | [0x2620, 0x4f, 0x8000, _, _, _, _, _]
            // 3fff::/20, documentation (RFC 9637).
            | [0x3fff, 0..=0xf, _, _, _, _, _, _]
            // 5f00::/16, segment routing (RFC 9602).
            | [0x5f00, _, _, _, _, _, _, _]
            // fc00::/7, unique-local (RFC 4193).
            | [0xfc00..=0xfdff, _, _, _, _, _, _, _]
            // fe80::/10, link-local (RFC 4291).
            | [0xfe80..=0xfebf, _, _, _, _, _, _, _]
            // fec0::/10, the retired site-local range (RFC 3879).
            | [0xfec0..=0xfeff, _, _, _, _, _, _, _]
            // ff00::/8, multicast (RFC 4291).
            | [0xff00..=0xffff, _, _, _, _, _, _, _]
    )
}

/// The addresses a name stands for, looked up within what remains of the
/// fetch's time.
fn looked_up(
    plan: &Plan,
    host: &str,
    port: u16,
    within: Duration,
) -> Result<Vec<SocketAddr>, Refusal> {
    let host_port = format!("{}:{}", host, port);
    match &plan.lookup {
        Lookup::System => {
            let (sent, answers) = mpsc::channel();
            thread::spawn(move || {
                let _ = sent.send(
                    host_port
                        .to_socket_addrs()
                        .map(|all| all.collect::<Vec<SocketAddr>>()),
                );
            });
            match answers.recv_timeout(within) {
                Ok(Ok(addrs)) if !addrs.is_empty() => Ok(addrs),
                Ok(_) => Err(Refusal::UnknownHost {
                    host: host.to_owned(),
                }),
                Err(_) => Err(Refusal::TimedOut),
            }
        }
        #[cfg(test)]
        Lookup::StandIn(stand_in) => stand_in(&host_port)
            .ok()
            .filter(|addrs| !addrs.is_empty())
            .ok_or_else(|| Refusal::UnknownHost {
                host: host.to_owned(),
            }),
    }
}

/// The client for one hop: it follows no redirect, turns no status into a
/// failure, uses no proxy the environment names, keeps no cookie, asks
/// for no compressed encoding, and connects only to the addresses judged
/// for the hop, within what remains of the fetch's time.
fn agent_for(within: Duration, judged: &Arc<Mutex<Option<Granted>>>) -> Agent {
    let tls = TlsConfig::builder()
        .provider(TlsProvider::Rustls)
        .root_certs(RootCerts::WebPki)
        .unversioned_rustls_crypto_provider(Arc::new(aws_lc_rs::default_provider()))
        .build();
    let config = Config::builder()
        .proxy(None)
        .http_status_as_error(false)
        .max_redirects(0)
        .max_redirects_will_error(false)
        .user_agent(format!("statac/{}", env!("CARGO_PKG_VERSION")))
        .accept("")
        .accept_encoding("identity")
        .tls_config(tls)
        .timeout_global(Some(within))
        .build();
    Agent::with_parts(config, DefaultConnector::default(), Replay(judged.clone()))
}

/// The addresses granted to the client for one hop, judged for the host
/// and port they were judged for.
#[derive(Debug)]
struct Granted {
    host: String,
    port: u16,
    addrs: Vec<SocketAddr>,
}

/// The resolver the client is given: it hands back only the addresses
/// judged for the host and port asked, and looks nothing up itself. A
/// host or port nothing was judged for is not found, which no fetch
/// should reach.
#[derive(Debug)]
struct Replay(Arc<Mutex<Option<Granted>>>);

impl Resolver for Replay {
    fn resolve(
        &self,
        uri: &Uri,
        _config: &ureq::config::Config,
        _timeout: NextTimeout,
    ) -> Result<ResolvedSocketAddrs, UreqError> {
        let authority = uri.authority().ok_or(UreqError::HostNotFound)?;
        let scheme = uri.scheme().ok_or(UreqError::HostNotFound)?;
        let port = authority.port_u16().unwrap_or(match scheme.as_str() {
            "http" => 80,
            "https" => 443,
            _ => return Err(UreqError::HostNotFound),
        });
        let host = authority.host();
        // An address names a literal IPv6 host with its brackets; the
        // host was judged without them.
        let host = host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .to_ascii_lowercase();
        let granted = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let granted = granted
            .as_ref()
            .filter(|granted| granted.host == host && granted.port == port)
            .ok_or(UreqError::HostNotFound)?;
        let mut resolved = self.empty();
        for address in &granted.addrs {
            resolved.push(*address);
        }
        Ok(resolved)
    }
}

/// The statuses the fetch follows: the five that mean "it is elsewhere".
fn is_a_redirect(status: u16) -> bool {
    matches!(status, 301 | 302 | 303 | 307 | 308)
}

/// The address a redirect points at, read against the address the answer
/// came from: absolute, from the host's top, or relative. Nowhere usable
/// is a refusal.
fn where_it_pointed(headers: &HeaderMap, from: &Url) -> Result<Url, Refusal> {
    let raw = headers
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok());
    let Some(location) = raw.filter(|location| !location.trim().is_empty()) else {
        return Err(Refusal::NoRedirectTarget {
            host: from.host_str().unwrap_or_default().to_owned(),
            location: raw.map(str::to_owned),
        });
    };
    from.join(location).map_err(|_| Refusal::NoRedirectTarget {
        host: from.host_str().unwrap_or_default().to_owned(),
        location: Some(location.to_owned()),
    })
}

/// What the failure of reading the body says as a refusal: the fetch's
/// own time running out is said as that, anything else as what the
/// system said.
fn body_failed(error: io::Error) -> Refusal {
    // The client keeps its own error inside the reading error, and a
    // socket time out arrives with that kind instead.
    let out_of_time = error.kind() == io::ErrorKind::TimedOut
        || matches!(
            error
                .get_ref()
                .and_then(|inner| inner.downcast_ref::<UreqError>()),
            Some(UreqError::Timeout(_))
        );
    if out_of_time {
        Refusal::TimedOut
    } else {
        Refusal::Failed {
            reason: error.to_string(),
        }
    }
}

/// What the client's failure of a hop says as a refusal.
fn refused(error: UreqError, host: &str) -> Refusal {
    match error {
        UreqError::Timeout(_) => Refusal::TimedOut,
        UreqError::HostNotFound => Refusal::UnknownHost {
            host: host.to_owned(),
        },
        UreqError::Rustls(_) | UreqError::Tls(_) => Refusal::Tls {
            host: host.to_owned(),
        },
        UreqError::Io(error) => Refusal::Failed {
            reason: error.to_string(),
        },
        _ => Refusal::Failed {
            reason: error.to_string(),
        },
    }
}

#[cfg(test)]
mod tests;
