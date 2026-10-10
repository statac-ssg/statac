//! The fetcher's tests, all on this machine: a server standing in for the
//! open web, a lookup standing in for the system's, and a child of the
//! test process standing in for the author's environment.

use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use super::*;
use url::Url;

/// A server on this machine, answering each request with what `rule`
/// makes of it and remembering every request as it read it.
struct Server {
    addr: SocketAddr,
    requests: Arc<Mutex<Vec<String>>>,
}

impl Server {
    fn start(rule: impl Fn(&str) -> Vec<u8> + Send + Sync + 'static) -> Server {
        Server::serve(TcpListener::bind("127.0.0.1:0").unwrap(), rule, false)
    }

    /// A server that answers as `start` does and then holds each
    /// connection open, saying nothing more, for the tests of an answer
    /// that stops part way through.
    fn start_holding(rule: impl Fn(&str) -> Vec<u8> + Send + Sync + 'static) -> Server {
        Server::serve(TcpListener::bind("127.0.0.1:0").unwrap(), rule, true)
    }

    /// A server on a listener of the test's own, for the tests that need
    /// to know its address before the rule can name it.
    fn on(listener: TcpListener, rule: impl Fn(&str) -> Vec<u8> + Send + Sync + 'static) -> Server {
        Server::serve(listener, rule, false)
    }

    fn serve(
        listener: TcpListener,
        rule: impl Fn(&str) -> Vec<u8> + Send + Sync + 'static,
        holds: bool,
    ) -> Server {
        let addr = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let requests_for_the_server = requests.clone();
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else {
                    continue;
                };
                let request = read_request(&mut stream);
                requests_for_the_server
                    .lock()
                    .unwrap()
                    .push(request.clone());
                let response = rule(&request);
                let _ = stream.write_all(&response);
                if holds {
                    thread::sleep(Duration::from_secs(60));
                }
            }
        });
        Server { addr, requests }
    }

    fn requests(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }
}

/// Reads the request as it comes, up to the end of its headers.
fn read_request(stream: &mut TcpStream) -> String {
    let mut text = String::new();
    let mut buffer = [0; 1024];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(read) => {
                text.push_str(&String::from_utf8_lossy(&buffer[..read]));
                if text.contains("\r\n\r\n") || text.len() > 16 * 1024 {
                    break;
                }
            }
        }
    }
    text
}

/// A fetch within a test: the addresses of the servers started here are
/// allowed, every other address judged as ever.
fn fetch_on(allowed: &[SocketAddr], address: &str, limit: u64) -> Result<Fetched, Refusal> {
    run(
        &Plan::for_a_test(Lookup::System, allowed.to_vec(), TIME),
        address,
        limit,
    )
}

/// The path of a request as the first line of it names it.
fn path_of(request: &str) -> &str {
    request
        .split("\r\n")
        .next()
        .and_then(|line| line.split(' ').nth(1))
        .unwrap_or_default()
}

/// A `200` answer of `body` as `content_type`.
fn answered(body: &str, content_type: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}

/// An answer redirecting to `location` with `status`.
fn redirected(status: u16, location: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 {status} Elsewhere\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    )
    .into_bytes()
}

/// An answer of `status` and nothing else.
fn answered_status(status: u16) -> Vec<u8> {
    format!("HTTP/1.1 {status} Whatever\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
        .into_bytes()
}

/// A lookup standing in for the system's, answering each name it is asked
/// with the next of `answers`, and the last again after that. Gives back
/// how many names it was asked, for the tests that ask.
fn stand_in_for(answers: Vec<io::Result<Vec<SocketAddr>>>) -> (Lookup, Arc<Mutex<usize>>) {
    let asks = Arc::new(Mutex::new(0));
    let answers = Mutex::new(std::collections::VecDeque::from(answers));
    let asked = asks.clone();
    let lookup: StandInLookup = Arc::new(move |_host_port| {
        *asked.lock().unwrap() += 1;
        let mut answers = answers.lock().unwrap();
        match answers.pop_front() {
            Some(answer) => answer,
            None => Err(io::Error::other("asked beyond what it answers")),
        }
    });
    (Lookup::StandIn(lookup), asks)
}

#[test]
fn the_open_web_starts_where_each_special_range_ends() {
    let refused = [
        "0.0.0.0",
        "0.255.255.255",
        "10.0.0.0",
        "10.255.255.255",
        "100.64.0.0",
        "100.127.255.255",
        "127.0.0.0",
        "127.255.255.255",
        "169.254.0.0",
        "169.254.169.254",
        "172.16.0.0",
        "172.31.255.255",
        "192.0.0.0",
        "192.0.0.255",
        "192.0.2.0",
        "192.0.2.255",
        "192.31.196.0",
        "192.31.196.255",
        "192.52.193.0",
        "192.52.193.255",
        "192.88.99.0",
        "192.88.99.255",
        "192.168.0.0",
        "192.168.255.255",
        "192.175.48.0",
        "192.175.48.255",
        "198.18.0.0",
        "198.19.255.255",
        "198.51.100.0",
        "198.51.100.255",
        "203.0.113.0",
        "203.0.113.255",
        "224.0.0.0",
        "239.255.255.255",
        "240.0.0.0",
        "255.255.255.255",
    ];
    let allowed = [
        "1.0.0.0",
        "9.255.255.255",
        "11.0.0.0",
        "100.63.255.255",
        "100.128.0.0",
        "126.255.255.255",
        "128.0.0.0",
        "169.253.255.255",
        "169.255.0.0",
        "169.255.255.255",
        "172.15.255.255",
        "172.32.0.0",
        "192.0.1.0",
        "192.0.3.0",
        "192.31.195.255",
        "192.31.197.0",
        "192.52.192.255",
        "192.52.194.0",
        "192.88.98.255",
        "192.88.100.0",
        "192.167.255.255",
        "192.169.0.0",
        "192.175.47.255",
        "192.175.49.0",
        "198.17.255.255",
        "198.20.0.0",
        "198.51.99.255",
        "198.51.101.0",
        "203.0.112.255",
        "203.0.114.0",
        "223.255.255.255",
    ];
    for address in refused {
        assert!(
            !on_the_open_web(address.parse().unwrap()),
            "{address} judged on the open web"
        );
    }
    for address in allowed {
        assert!(
            on_the_open_web(address.parse().unwrap()),
            "{address} judged off the open web"
        );
    }
}

#[test]
fn the_open_web_of_ipv6_starts_where_each_special_range_ends() {
    let refused = [
        "::",
        "::1",
        "::ffff:0.0.0.0",
        "::ffff:127.0.0.1",
        "::127.0.0.1",
        "::2",
        "64:ff9b::",
        "64:ff9b::102:304",
        "64:ff9b:1::",
        "64:ff9b:1:ffff::",
        "100::",
        "100::1",
        "100:0:0:1::",
        "2001::",
        "2001:1ff:ffff::",
        "2001:2::",
        "2001:20::",
        "2001:db8::",
        "2001:db8:ffff::",
        "2002::",
        "2002:ffff::",
        "2620:4f:8000::",
        "3fff::",
        "3fff:f::",
        "5f00::",
        "5f00:ffff::",
        "fc00::",
        "fdff::",
        "fe80::",
        "febf::",
        "fec0::",
        "feff::",
        "ff00::",
        "ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff",
    ];
    let allowed = [
        "::ffff:1.2.3.4",
        "64:ff9b:2::",
        "100:0:0:2::",
        "100:0:1::",
        "2001:200::",
        "2001:db9::",
        "2003::",
        "3fff:10::",
        "5f01::",
        "2620:4f:8001::",
        "fe00::",
    ];
    for address in refused {
        assert!(
            !on_the_open_web(address.parse().unwrap()),
            "{address} judged on the open web"
        );
    }
    for address in allowed {
        assert!(
            on_the_open_web(address.parse().unwrap()),
            "{address} judged off the open web"
        );
    }
}

#[test]
fn a_literal_host_is_judged_as_the_address_it_stands_for_in_any_spelling() {
    for address in [
        "http://127.1/x",
        "http://0x7f.0.0.1/x",
        "http://2130706433/x",
        "http://[::ffff:127.0.0.1]/x",
        "http://[::1]/x",
    ] {
        assert!(
            matches!(fetch(address, 8), Err(Refusal::PrivateAddress { .. })),
            "{address} was not refused"
        );
    }
}

#[test]
fn a_host_written_as_an_ipv6_literal_is_judged_and_fetched() {
    let listener = TcpListener::bind("[::1]:0").unwrap();
    let server = Server::on(listener, |_| answered("here", "text/plain"));
    let fetched = fetch_on(&[server.addr], &format!("http://{}/here", server.addr), 64).unwrap();
    assert_eq!(fetched.bytes, b"here");
}

#[test]
fn a_scheme_other_than_http_or_https_is_refused() {
    assert_eq!(
        fetch("ftp://example.org/still", 8),
        Err(Refusal::Scheme {
            scheme: "ftp".to_owned()
        })
    );
}

#[test]
fn an_address_carrying_a_name_or_password_is_refused() {
    assert_eq!(
        fetch("http://name@example.org/still", 8),
        Err(Refusal::Credentials)
    );
    assert_eq!(
        fetch("http://name:word@example.org/still", 8),
        Err(Refusal::Credentials)
    );
}

#[test]
fn an_http_or_https_address_without_a_host_is_refused_as_such() {
    assert_eq!(fetch("http://:80/still", 8), Err(Refusal::NoHost));
}

#[test]
fn an_address_that_reads_as_nothing_at_all_is_refused_as_such() {
    assert_eq!(fetch("an address", 8), Err(Refusal::NotAnAddress));
    assert_eq!(fetch("http://[", 8), Err(Refusal::NotAnAddress));
}

#[test]
fn a_plain_answer_gives_its_bytes_its_address_and_its_content_type() {
    let server = Server::start(|_| answered("hello", "text/plain"));
    let fetched = fetch_on(&[server.addr], &format!("http://{}/plain", server.addr), 64).unwrap();
    assert_eq!(fetched.bytes, b"hello");
    assert_eq!(
        fetched.from,
        Url::parse(&format!("http://{}/plain", server.addr)).unwrap()
    );
    assert_eq!(fetched.content_type.as_deref(), Some("text/plain"));
}

#[test]
fn every_redirecting_status_is_followed_to_what_it_points_at() {
    for status in [301, 302, 303, 307, 308] {
        let server = Server::start(move |request| match path_of(request) {
            "/moved" => redirected(status, "/arrived"),
            _ => answered("here", "text/plain"),
        });
        let fetched =
            fetch_on(&[server.addr], &format!("http://{}/moved", server.addr), 64).unwrap();
        assert_eq!(fetched.bytes, b"here");
        assert_eq!(
            fetched.from,
            Url::parse(&format!("http://{}/arrived", server.addr)).unwrap()
        );
    }
}

#[test]
fn a_relative_redirect_points_from_the_address_of_its_answer() {
    let server = Server::start(|request| match path_of(request) {
        "/moved" => redirected(302, "arrived"),
        _ => answered("here", "text/plain"),
    });
    let fetched = fetch_on(&[server.addr], &format!("http://{}/moved", server.addr), 64).unwrap();
    assert_eq!(
        fetched.from,
        Url::parse(&format!("http://{}/arrived", server.addr)).unwrap()
    );
}

#[test]
fn an_absolute_redirect_to_another_address_is_judged_and_followed() {
    let second = Server::start(|_| answered("there", "text/plain"));
    let first = Server::start(move |request| match path_of(request) {
        "/moved" => redirected(301, &format!("http://{}/x", second.addr)),
        _ => answered("wrong", "text/plain"),
    });
    let fetched = fetch_on(
        &[first.addr, second.addr],
        &format!("http://{}/moved", first.addr),
        64,
    )
    .unwrap();
    assert_eq!(fetched.bytes, b"there");
    assert_eq!(second.requests().len(), 1);
}

#[test]
fn a_redirect_to_an_address_off_the_open_web_is_refused_at_that_hop() {
    let private = Server::start(|_| answered("nothing may come here", "text/plain"));
    let server = Server::start(move |request| match path_of(request) {
        "/moved" => redirected(301, &format!("http://{}", private.addr)),
        _ => answered("wrong", "text/plain"),
    });
    let refusal =
        fetch_on(&[server.addr], &format!("http://{}/moved", server.addr), 64).unwrap_err();
    assert_eq!(
        refusal,
        Refusal::PrivateAddress {
            host: "127.0.0.1".to_owned(),
            address: IpAddr::V4(Ipv4Addr::LOCALHOST)
        }
    );
    assert!(private.requests().is_empty());
    assert_eq!(server.requests().len(), 1);
}

#[test]
fn a_redirect_to_an_address_written_private_is_refused_at_that_hop() {
    let server = Server::start(|request| match path_of(request) {
        "/moved" => redirected(301, "http://10.0.0.1:80/x"),
        _ => answered("wrong", "text/plain"),
    });
    let refusal =
        fetch_on(&[server.addr], &format!("http://{}/moved", server.addr), 64).unwrap_err();
    assert_eq!(
        refusal,
        Refusal::PrivateAddress {
            host: "10.0.0.1".to_owned(),
            address: IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))
        }
    );
    assert_eq!(server.requests().len(), 1);
}

#[test]
fn a_sixth_redirecting_answer_is_refused_as_too_many() {
    let server = Server::start(|request| {
        match path_of(request).trim_start_matches("/hop-").parse::<u16>() {
            Ok(n) if n < 5 => redirected(307, &format!("/hop-{}", n + 1)),
            Ok(5) => redirected(307, "/hop-6"),
            _ => answered("past the limit", "text/plain"),
        }
    });
    let refusal =
        fetch_on(&[server.addr], &format!("http://{}/hop-0", server.addr), 64).unwrap_err();
    assert_eq!(refusal, Refusal::Redirects { limit: 5 });
    let requests = server.requests();
    let paths: Vec<&str> = requests.iter().map(|request| path_of(request)).collect();
    assert_eq!(
        paths,
        ["/hop-0", "/hop-1", "/hop-2", "/hop-3", "/hop-4", "/hop-5"]
    );
}

#[test]
fn a_redirect_pointing_nowhere_is_refused_as_nowhere() {
    let server = Server::start(|request| match path_of(request) {
        "/moved" => {
            b"HTTP/1.1 302 Elsewhere\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec()
        }
        _ => answered("wrong", "text/plain"),
    });
    let refusal =
        fetch_on(&[server.addr], &format!("http://{}/moved", server.addr), 64).unwrap_err();
    assert_eq!(
        refusal,
        Refusal::NoRedirectTarget {
            host: "127.0.0.1".to_owned(),
            location: None
        }
    );
}

#[test]
fn having_no_such_thing_is_a_refusal_of_its_own_and_other_statuses_are_theirs() {
    let server = Server::start(|request| match path_of(request) {
        "/not-there" => answered_status(404),
        "/gone" => answered_status(410),
        "/broken" => answered_status(500),
        _ => answered("here", "text/plain"),
    });
    assert_eq!(
        fetch_on(
            &[server.addr],
            &format!("http://{}/not-there", server.addr),
            64
        ),
        Err(Refusal::Missing { status: 404 })
    );
    assert_eq!(
        fetch_on(&[server.addr], &format!("http://{}/gone", server.addr), 64),
        Err(Refusal::Missing { status: 410 })
    );
    assert_eq!(
        fetch_on(
            &[server.addr],
            &format!("http://{}/broken", server.addr),
            64
        ),
        Err(Refusal::Status { status: 500 })
    );
}

#[test]
fn an_answer_that_says_it_is_too_much_is_refused_before_its_body_is_read() {
    let server = Server::start(|_| {
        "HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\nsmall".into()
    });
    let refusal =
        fetch_on(&[server.addr], &format!("http://{}/plain", server.addr), 50).unwrap_err();
    assert_eq!(
        refusal,
        Refusal::TooLarge {
            limit: 50,
            content_length: Some(100)
        }
    );
}

#[test]
fn an_answer_that_runs_past_the_limit_is_refused_and_none_of_it_is_kept() {
    let server = Server::start(|_| {
        let mut response = b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n".to_vec();
        response.extend_from_slice(&[b'a'; 9]);
        response
    });
    let refusal =
        fetch_on(&[server.addr], &format!("http://{}/plain", server.addr), 8).unwrap_err();
    assert_eq!(
        refusal,
        Refusal::TooLarge {
            limit: 8,
            content_length: None
        }
    );
}

#[test]
fn an_answer_that_never_comes_runs_out_of_time() {
    let server = Server::start(|_| {
        thread::sleep(Duration::from_secs(60));
        Vec::new()
    });
    let plan = Plan::for_a_test(
        Lookup::System,
        vec![server.addr],
        Duration::from_millis(150),
    );
    assert_eq!(
        run(&plan, &format!("http://{}/plain", server.addr), 8),
        Err(Refusal::TimedOut)
    );
}

#[test]
fn a_body_that_stops_part_way_runs_out_of_time() {
    let server = Server::start_holding(|_| {
        b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\npart".to_vec()
    });
    let plan = Plan::for_a_test(
        Lookup::System,
        vec![server.addr],
        Duration::from_millis(150),
    );
    assert_eq!(
        run(&plan, &format!("http://{}/plain", server.addr), 100),
        Err(Refusal::TimedOut)
    );
}

#[test]
fn a_request_sends_nothing_but_what_it_needs() {
    let server = Server::start(|_| answered("plain", "text/plain"));
    fetch_on(&[server.addr], &format!("http://{}/plain", server.addr), 64).unwrap();
    let request = &server.requests()[0];
    let mut lines = request.split("\r\n");
    assert_eq!(lines.next(), Some("GET /plain HTTP/1.1"));
    let mut sent: Vec<String> = lines
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect();
    sent.sort();
    assert_eq!(
        sent,
        vec![
            "accept-encoding: identity".to_owned(),
            format!("host: {}", server.addr),
            format!("user-agent: statac/{}", env!("CARGO_PKG_VERSION")),
        ]
    );
}

#[test]
fn a_name_standing_for_one_good_address_and_one_private_is_refused_before_anything_is_sent() {
    let server = Server::start(|_| answered("nothing may come here", "text/plain"));
    let (lookup, _asks) = stand_in_for(vec![Ok(vec![
        server.addr,
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 80),
    ])]);
    let plan = Plan::for_a_test(lookup, vec![server.addr], TIME);
    let refusal = run(
        &plan,
        &format!("http://mixed.example:{}/plain", server.addr.port()),
        64,
    )
    .unwrap_err();
    assert_eq!(
        refusal,
        Refusal::PrivateAddress {
            host: "mixed.example".to_owned(),
            address: IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))
        }
    );
    assert!(server.requests().is_empty());
}

#[test]
fn a_name_that_answers_differently_each_time_it_is_asked_is_connected_only_as_it_was_judged() {
    let server = Server::start(|_| answered("first", "text/plain"));
    let (lookup, asks) = stand_in_for(vec![
        Ok(vec![server.addr]),
        Ok(vec![SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            80,
        )]),
    ]);
    let plan = Plan::for_a_test(lookup, vec![server.addr], TIME);
    let fetched = run(
        &plan,
        &format!("http://moving.example:{}/here", server.addr.port()),
        64,
    )
    .unwrap();
    assert_eq!(fetched.bytes, b"first");
    assert_eq!(*asks.lock().unwrap(), 1);
    assert_eq!(server.requests().len(), 1);
}

#[test]
fn a_name_that_answers_differently_at_a_second_hop_is_judged_again_and_refused() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = Server::on(listener, move |request| match path_of(request) {
        "/there" => redirected(301, &format!("http://moving.example:{port}/again")),
        _ => answered("wrong", "text/plain"),
    });
    let (lookup, asks) = stand_in_for(vec![
        Ok(vec![server.addr]),
        Ok(vec![SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            80,
        )]),
    ]);
    let plan = Plan::for_a_test(lookup, vec![server.addr], TIME);
    let refusal = run(
        &plan,
        &format!("http://moving.example:{}/there", server.addr.port()),
        64,
    )
    .unwrap_err();
    assert_eq!(
        refusal,
        Refusal::PrivateAddress {
            host: "moving.example".to_owned(),
            address: IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))
        }
    );
    assert_eq!(*asks.lock().unwrap(), 2);
    assert_eq!(server.requests().len(), 1);
}

#[test]
fn a_name_nothing_answers_to_is_refused_as_such() {
    let (lookup, _asks) = stand_in_for(vec![Err(io::Error::other("no such name"))]);
    let plan = Plan::for_a_test(lookup, Vec::new(), TIME);
    assert_eq!(
        run(&plan, "http://absent.example/plain", 64),
        Err(Refusal::UnknownHost {
            host: "absent.example".to_owned()
        })
    );
}

#[test]
fn proxies_the_environment_names_are_not_used() {
    // The test runs itself again as a child, with the proxy variables
    // pointing at a server of this test's own: nothing may reach it, and
    // the fetch must go straight to the answering server.
    if let Ok(answer) = std::env::var("STATAC_FETCH_PROXY_TEST") {
        let answer: SocketAddr = answer.parse().unwrap();
        let plan = Plan::for_a_test(Lookup::System, vec![answer], TIME);
        let fetched = run(&plan, &format!("http://{answer}/plain"), 64).unwrap();
        assert_eq!(fetched.bytes, b"straight");
        return;
    }
    let proxy = Server::start(|_| answered("nothing may come here", "text/plain"));
    let answer = Server::start(|_| answered("straight", "text/plain"));
    let mut child = Command::new(std::env::current_exe().unwrap());
    for name in [
        "HTTP_PROXY",
        "http_proxy",
        "HTTPS_PROXY",
        "https_proxy",
        "ALL_PROXY",
        "all_proxy",
    ] {
        child.env(name, format!("http://{}", proxy.addr));
    }
    child
        .env("NO_PROXY", "example.org")
        .env("STATAC_FETCH_PROXY_TEST", answer.addr.to_string());
    let output = child
        .arg("fetch::tests::proxies_the_environment_names_are_not_used")
        .arg("--exact")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "child: {} | {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(proxy.requests().is_empty());
    assert_eq!(answer.requests().len(), 1);
}
