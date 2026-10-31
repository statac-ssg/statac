//! The step's tests, all on this machine: a server standing in for the
//! sites a page links to, each kind of answer at a host of its own, or a
//! looker standing in for the fetcher where the network itself is what
//! fails.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::*;
use crate::source::Sources;
use crate::testing::Scratch;

/// One request a server or a stand-in was asked: its method, its host and
/// path, and when it began and ended.
#[derive(Clone, Debug)]
struct Request {
    method: String,
    host: String,
    path: String,
    began: Instant,
    ended: Instant,
}

type Requests = Arc<Mutex<Vec<Request>>>;

/// A server on this machine answering each request with what `rule` makes
/// of its method, host and path, each connection on a thread of its own.
struct Server {
    addr: SocketAddr,
    requests: Requests,
}

impl Server {
    fn start(rule: impl Fn(&str, &str, &str) -> String + Send + Sync + 'static) -> Server {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let requests: Requests = Arc::default();
        let kept = requests.clone();
        let rule = Arc::new(rule);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else {
                    continue;
                };
                let (rule, kept) = (rule.clone(), kept.clone());
                std::thread::spawn(move || {
                    let began = Instant::now();
                    let head = read_head(&mut stream);
                    let mut words = head.split(' ');
                    let method = words.next().unwrap_or_default().to_owned();
                    let path = words.next().unwrap_or_default().to_owned();
                    let host = (head.lines())
                        .find_map(|line| line.strip_prefix("host: "))
                        .unwrap_or_default()
                        .to_owned();
                    let answer = rule(&method, &host, &path);
                    let _ = stream.write_all(answer.as_bytes());
                    let ended = Instant::now();
                    let request = Request {
                        method,
                        host,
                        path,
                        began,
                        ended,
                    };
                    kept.lock().unwrap().push(request);
                });
            }
        });
        Server { addr, requests }
    }
}

/// The request's head, its header names in small letters.
fn read_head(stream: &mut TcpStream) -> String {
    let mut text = String::new();
    let mut buffer = [0; 1024];
    while !text.contains("\r\n\r\n") {
        match stream.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(read) => text.push_str(&String::from_utf8_lossy(&buffer[..read])),
        }
    }
    (text.lines())
        .map(|line| match line.split_once(": ") {
            Some((name, value)) => format!("{}: {value}", name.to_ascii_lowercase()),
            None => line.to_owned(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// An answer of `status` with `headers`, each a line, and nothing else.
fn answer(status: u16, headers: &str) -> String {
    format!("HTTP/1.1 {status} Whatever\r\n{headers}Content-Length: 0\r\nConnection: close\r\n\r\n")
}

/// The fetcher's own looks, every name standing for the server.
struct Local {
    server: SocketAddr,
    pause: Duration,
}

impl Look for Local {
    fn look(&self, address: &str, method: Method) -> Looked {
        crate::fetch::look_on(self.server, address, method)
    }

    fn pause(&self) -> Duration {
        self.pause
    }
}

/// A looker standing in for the fetcher, answering as `answer` says and
/// remembering each address and method asked.
struct StandIn<F> {
    answer: F,
    asked: Mutex<Vec<(String, Method)>>,
}

impl<F: Fn(&str, Method) -> Looked + Sync> StandIn<F> {
    fn new(answer: F) -> StandIn<F> {
        StandIn {
            answer,
            asked: Mutex::default(),
        }
    }

    fn asked(&self) -> Vec<(String, Method)> {
        self.asked.lock().unwrap().clone()
    }
}

impl<F: Fn(&str, Method) -> Looked + Sync> Look for StandIn<F> {
    fn look(&self, address: &str, method: Method) -> Looked {
        self.asked
            .lock()
            .unwrap()
            .push((address.to_owned(), method));
        (self.answer)(address, method)
    }

    fn pause(&self) -> Duration {
        Duration::ZERO
    }
}

fn status(status: u16) -> Looked {
    Looked {
        redirects: Vec::new(),
        end: Ending::Status {
            status,
            challenge: false,
            retry_after: None,
        },
    }
}

fn refused(refusal: Refusal) -> Looked {
    Looked {
        redirects: Vec::new(),
        end: Ending::Refused(refusal),
    }
}

fn no_host(address: &str) -> Looked {
    let host = url::Url::parse(address)
        .unwrap()
        .host_str()
        .unwrap()
        .to_owned();
    refused(Refusal::UnknownHost { host })
}

fn day(written: &str) -> Date {
    crate::date::read(written).unwrap()
}

/// What the step makes of `urls` on a site whose settings are `settings`
/// and whose `fetched/links` holds `was`, checked with `looker` on 6
/// October 2026: what it said, its exit code, and `fetched/links` after,
/// where it is written.
fn check_with(
    looker: &dyn Look,
    settings: &str,
    urls: &[&str],
    was: &str,
) -> (String, u8, Option<String>) {
    let site = Scratch::new("links-step");
    let folder = site.join("fetched");
    let mut sources = Sources::default();
    let source = sources.add("statac.yaml", settings.to_owned());
    let (settings, problems) = crate::config::read_source(&sources, source);
    assert_eq!(problems, []);
    let was = match was.is_empty() {
        true => Links::default(),
        false => {
            let source = sources.add(record::shown(), was.to_owned());
            let (was, problems) = Links::read(&sources, source);
            assert_eq!(problems, []);
            std::fs::create_dir_all(&folder).unwrap();
            std::fs::write(folder.join(record::NAME), was.text(|_| true)).unwrap();
            was
        }
    };
    let mut urls: Vec<String> = urls.iter().map(|url| (*url).to_owned()).collect();
    urls.sort_unstable();
    let report = Report::new(&sources, &site, false);
    let checking = Checking {
        folder: &folder,
        was: &was,
        settings: &settings,
        urls: &urls,
        today: day("2026-10-06"),
        report: &report,
    };
    let mut said = String::new();
    let code = check(&checking, looker, &mut |line| said.push_str(line));
    let written = std::fs::read_to_string(folder.join(record::NAME)).ok();
    (said, code, written)
}

fn check_all(looker: &dyn Look, urls: &[&str]) -> (String, u8, Option<String>) {
    check_with(looker, "check_external_links: true\n", urls, "")
}

#[test]
fn each_kind_of_answer_a_site_gives_is_recorded_as_it_says() {
    let server = Server::start(|method, host, path| match (method, host, path) {
        (_, "ok.example", _) => answer(200, ""),
        ("HEAD", "head.example", _) => answer(405, ""),
        ("HEAD", "wrongly.example", _) => answer(404, ""),
        ("GET", "head.example" | "wrongly.example", _) => answer(200, ""),
        (_, "gone.example", _) => answer(410, ""),
        (_, "missing.example", _) => answer(404, ""),
        (_, "moved.example", "/old/") => answer(301, "Location: /older/\r\n"),
        (_, "moved.example", "/older/") => answer(308, "Location: http://new.example/now/\r\n"),
        (_, "new.example", "/now/") => answer(302, "Location: /today/\r\n"),
        (_, "new.example", "/today/") => answer(200, ""),
        (_, "moved.example", "/lost/") => {
            answer(301, "Location: http://moved.example/nothing/\r\n")
        }
        (_, "moved.example", "/nothing/") => answer(404, ""),
        (_, "short.example", _) => answer(302, "Location: http://ok.example/\r\n"),
        (_, "challenge.example", _) => answer(403, "cf-mitigated: challenge\r\n"),
        (_, "walled.example", _) => answer(403, ""),
        (_, "down.example", _) => answer(503, ""),
        (_, "busy.example", _) => answer(429, ""),
        _ => answer(500, ""),
    });
    let looker = Local {
        server: server.addr,
        pause: Duration::ZERO,
    };
    let (said, code, written) = check_all(
        &looker,
        &[
            "http://ok.example/",
            "http://head.example/",
            "http://wrongly.example/",
            "http://gone.example/",
            "http://missing.example/",
            "http://moved.example/old/",
            "http://moved.example/lost/",
            "http://short.example/",
            "http://challenge.example/",
            "http://walled.example/",
            "http://down.example/",
            "http://busy.example/",
        ],
    );
    assert_eq!(code, 0);
    assert_eq!(
        written.unwrap(),
        "statac links 1\n\
         http://busy.example/\tunanswered 429\t2026-10-06\n\
         http://challenge.example/\tunanswered challenge\t2026-10-06\n\
         http://down.example/\tunanswered 503\t2026-10-06\n\
         http://gone.example/\t410\t2026-10-06\n\
         http://head.example/\tok\t2026-10-06\n\
         http://missing.example/\t404\t2026-10-06\n\
         http://moved.example/lost/\t404\t2026-10-06\n\
         http://moved.example/old/\tmoved http://new.example/now/\t2026-10-06\n\
         http://ok.example/\tok\t2026-10-06\n\
         http://short.example/\tok\t2026-10-06\n\
         http://walled.example/\tunanswered 403\t2026-10-06\n\
         http://wrongly.example/\tok\t2026-10-06\n"
    );
    assert_eq!(
        said,
        "Checking 12 external links on 11 sites.\n\
         Checked 12 external links: 5 work, 3 broken, 4 didn't answer. \
         Run `statac check` to see the broken ones.\n"
    );
    // `HEAD` first, and `GET` only where it got no success.
    let requests = server.requests.lock().unwrap().clone();
    let asked = |host: &str| {
        let mut methods: Vec<String> = (requests.iter())
            .filter(|request| request.host == host)
            .map(|request| request.method.clone())
            .collect();
        methods.sort_unstable();
        methods
    };
    assert!(asked("ok.example").iter().all(|method| method == "HEAD"));
    assert_eq!(asked("head.example"), ["GET", "HEAD"]);
    assert_eq!(asked("missing.example"), ["GET", "HEAD"]);
}

#[test]
fn a_link_check_names_itself_and_the_page_explaining_it() {
    let heads: Arc<Mutex<Vec<String>>> = Arc::default();
    let kept = heads.clone();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            kept.lock().unwrap().push(read_head(&mut stream));
            let _ = stream.write_all(answer(200, "").as_bytes());
        }
    });
    let looker = Local {
        server: addr,
        pause: Duration::ZERO,
    };
    check_all(&looker, &["http://a.example/post-1/"]);
    let heads = heads.lock().unwrap().clone();
    let version = env!("CARGO_PKG_VERSION");
    assert_eq!(heads.len(), 1);
    let head = &heads[0];
    assert!(head.starts_with("HEAD /post-1/ HTTP/1.1\n"), "{head}");
    assert!(
        head.contains(&format!(
            "\nuser-agent: Statac/{version} (link check; +https://statac.dev/link-check/)\n"
        )),
        "{head}"
    );
    assert!(head.contains("\naccept-encoding: gzip\n"), "{head}");
    assert!(!head.contains("cookie"), "{head}");
}

#[test]
fn a_host_is_asked_one_request_at_a_time_with_the_pause_between() {
    let server = Server::start(|_, _, _| {
        std::thread::sleep(Duration::from_millis(20));
        answer(200, "")
    });
    let pause = Duration::from_millis(60);
    let looker = Local {
        server: server.addr,
        pause,
    };
    let mut urls: Vec<String> = (1..=4)
        .map(|n| format!("http://one.example/post-{n}/"))
        .collect();
    urls.extend((1..=3).map(|n| format!("http://two.example/post-{n}/")));
    let urls: Vec<&str> = urls.iter().map(String::as_str).collect();
    let started = Instant::now();
    let (_, _, written) = check_all(&looker, &urls);
    assert_eq!(written.unwrap().lines().count(), 8);
    let requests = server.requests.lock().unwrap().clone();
    for host in ["one.example", "two.example"] {
        let mut asked: Vec<&Request> = (requests.iter())
            .filter(|request| request.host == host)
            .collect();
        asked.sort_by_key(|request| request.began);
        for pair in asked.windows(2) {
            let gap = pair[1].began.saturating_duration_since(pair[0].ended);
            // The server notes a request's end a moment after it answers.
            assert!(gap + Duration::from_millis(10) >= pause, "{host}: {gap:?}");
        }
    }
    // The two hosts are asked at once: one host alone would take four
    // requests and three pauses.
    let one: Vec<&Request> = (requests.iter())
        .filter(|request| request.host == "two.example")
        .collect();
    assert!(
        one.iter().any(|request| request.began < started + pause),
        "{requests:?}"
    );
}

#[test]
fn eight_hosts_at_most_are_asked_at_once_the_busiest_first() {
    let server = Server::start(|_, _, _| {
        std::thread::sleep(Duration::from_millis(40));
        answer(200, "")
    });
    let looker = Local {
        server: server.addr,
        pause: Duration::ZERO,
    };
    let mut urls: Vec<String> = (1..=20)
        .map(|n| format!("http://host-{n}.example/"))
        .collect();
    urls.extend((1..=3).map(|n| format!("http://busiest.example/post-{n}/")));
    let urls: Vec<&str> = urls.iter().map(String::as_str).collect();
    check_all(&looker, &urls);
    let mut requests = server.requests.lock().unwrap().clone();
    requests.sort_by_key(|request| request.began);
    let at_once = |at: Instant| {
        (requests.iter())
            .filter(|request| request.began <= at && at < request.ended)
            .count()
    };
    let most = requests.iter().map(|request| at_once(request.began)).max();
    assert!(most.is_some_and(|most| (2..=8).contains(&most)), "{most:?}");
    let first: Vec<&str> = requests[..8]
        .iter()
        .map(|request| request.host.as_str())
        .collect();
    assert!(first.contains(&"busiest.example"), "{first:?}");
}

#[test]
fn each_reason_nothing_answered_is_recorded_and_a_host_not_reached_answers_for_all_its_links() {
    let looker = StandIn::new(|address: &str, _| {
        let host = url::Url::parse(address)
            .unwrap()
            .host_str()
            .unwrap()
            .to_owned();
        match host.as_str() {
            "ok.example" => status(200),
            "nohost.example" => no_host(address),
            "expired.example" => refused(Refusal::Certificate { host }),
            "untraced.example" => refused(Refusal::Tls { host }),
            "slow.example" => refused(Refusal::TimedOut),
            "local.example" => refused(Refusal::PrivateAddress {
                host,
                address: "127.0.0.1".parse().unwrap(),
            }),
            "refusing.example" => refused(Refusal::Failed {
                reason: "Connection refused".into(),
            }),
            "unasked.example" => refused(Refusal::LookupFailed {
                host,
                reason: "Temporary failure in name resolution".into(),
            }),
            _ => refused(Refusal::Redirects { limit: 5 }),
        }
    });
    let mut urls = vec!["http://ok.example/", "http://looping.example/"];
    let hosts = [
        "nohost", "expired", "untraced", "slow", "local", "refusing", "unasked",
    ];
    let both: Vec<String> = (hosts.iter())
        .flat_map(|host| {
            [
                format!("http://{host}.example/a/"),
                format!("http://{host}.example/b/"),
            ]
        })
        .collect();
    urls.extend(both.iter().map(String::as_str));
    let (said, code, written) = check_all(&looker, &urls);
    assert_eq!(code, 0);
    let mut expected = vec![
        "http://ok.example/\tok\t2026-10-06".to_owned(),
        "http://looping.example/\tunanswered redirects\t2026-10-06".to_owned(),
    ];
    for (host, answer) in [
        ("nohost", "no-host"),
        ("expired", "certificate"),
        ("untraced", "unanswered tls"),
        ("slow", "unanswered timeout"),
        ("local", "unanswered private"),
        ("refusing", "unanswered failed"),
        ("unasked", "unanswered failed"),
    ] {
        for path in ["a", "b"] {
            expected.push(format!(
                "http://{host}.example/{path}/\t{answer}\t2026-10-06"
            ));
        }
    }
    expected.sort_unstable();
    let written = written.unwrap();
    let mut lines: Vec<&str> = written.lines().collect();
    assert_eq!(lines.remove(0), "statac links 1");
    assert_eq!(lines, expected);
    assert!(
        said.contains("Checked 16 external links: 1 works, 4 broken, 11 didn't answer."),
        "{said}"
    );
    // One link of each host not reached is asked; `GET` follows a `HEAD`
    // that timed out or failed.
    let asked = looker.asked();
    for host in hosts {
        let of_host: Vec<&(String, Method)> = (asked.iter())
            .filter(|(address, _)| address.contains(&format!("//{host}.")))
            .collect();
        let methods: Vec<Method> = of_host.iter().map(|(_, method)| *method).collect();
        let addresses: HashSet<&str> = of_host
            .iter()
            .map(|(address, _)| address.as_str())
            .collect();
        assert_eq!(addresses.len(), 1, "{host}: {of_host:?}");
        let again = matches!(host, "slow" | "refusing");
        let expected = match again {
            true => vec![Method::Head, Method::Get],
            false => vec![Method::Head],
        };
        assert_eq!(methods, expected, "{host}");
    }
}

#[test]
fn many_sites_in_a_row_not_answering_stop_the_step_and_none_is_recorded() {
    let looker = StandIn::new(|address: &str, _| {
        std::thread::sleep(Duration::from_millis(20));
        no_host(address)
    });
    let urls: Vec<String> = (1..=40)
        .map(|n| format!("http://host-{n}.example/"))
        .collect();
    let urls: Vec<&str> = urls.iter().map(String::as_str).collect();
    let (said, code, written) = check_all(&looker, &urls);
    assert_eq!(code, 0);
    assert_eq!(written, None);
    assert_eq!(
        said,
        "Checking 40 external links on 40 sites.\n\
         Stopped checking external links: 16 sites in a row didn't answer, so this computer \
         may be offline. Statac recorded nothing for them.\n"
    );
    // Past the sixteenth, only the hosts the other workers are already
    // asking as the step stops are asked, however the threads are run.
    assert!(
        looker.asked().len() < 16 + HOSTS_AT_ONCE,
        "{:?}",
        looker.asked()
    );
}

#[test]
fn a_few_sites_not_answering_are_recorded_only_where_some_site_answered() {
    let urls = [
        "http://a.example/",
        "http://b.example/",
        "http://c.example/",
    ];
    let looker = StandIn::new(|address: &str, _| no_host(address));
    let (said, _, written) = check_all(&looker, &urls);
    assert_eq!(written, None);
    assert_eq!(
        said,
        "Checking 3 external links on 3 sites.\n\
         No site answered, so this computer may be offline. Statac recorded nothing for those \
         links.\n"
    );

    let looker = StandIn::new(|address: &str, _| match address {
        "http://c.example/" => status(200),
        address => no_host(address),
    });
    let (said, _, written) = check_all(&looker, &urls);
    assert_eq!(
        written.unwrap(),
        "statac links 1\n\
         http://a.example/\tno-host\t2026-10-06\n\
         http://b.example/\tno-host\t2026-10-06\n\
         http://c.example/\tok\t2026-10-06\n"
    );
    assert!(said.ends_with("Checked 3 external links: 1 works, 2 broken. Run `statac check` to see the broken ones.\n"), "{said}");
}

#[test]
fn a_host_asking_to_wait_past_a_minute_is_left_for_the_next_refresh() {
    let looker = StandIn::new(|address: &str, _| match address {
        "http://busy.example/1/" => Looked {
            redirects: Vec::new(),
            end: Ending::Status {
                status: 429,
                challenge: false,
                retry_after: Some(3600),
            },
        },
        _ => status(200),
    });
    let urls = [
        "http://busy.example/1/",
        "http://busy.example/2/",
        "http://busy.example/3/",
        "http://other.example/",
    ];
    let (said, _, written) = check_all(&looker, &urls);
    assert_eq!(
        written.unwrap(),
        "statac links 1\n\
         http://busy.example/1/\tunanswered 429\t2026-10-06\n\
         http://other.example/\tok\t2026-10-06\n"
    );
    assert!(
        said.contains(
            "`busy.example` asked Statac to wait, so 2 links to it are left for the next \
             `statac refresh`.\n"
        ),
        "{said}"
    );
    let asked: Vec<String> = looker
        .asked()
        .into_iter()
        .map(|(address, _)| address)
        .collect();
    assert!(
        !asked
            .iter()
            .any(|address| address.ends_with("/2/") || address.ends_with("/3/"))
    );
}

#[test]
fn a_host_asking_to_wait_a_moment_is_waited_for() {
    let server = Server::start(|_, _, path| match path {
        "/1/" => answer(429, "Retry-After: 1\r\n"),
        _ => answer(200, ""),
    });
    let looker = Local {
        server: server.addr,
        pause: Duration::ZERO,
    };
    check_all(
        &looker,
        &["http://busy.example/1/", "http://busy.example/2/"],
    );
    let requests = server.requests.lock().unwrap().clone();
    let first = requests.iter().filter(|request| request.path == "/1/");
    let second = requests
        .iter()
        .find(|request| request.path == "/2/")
        .unwrap();
    let last_of_first = first.map(|request| request.ended).max().unwrap();
    assert!(second.began.duration_since(last_of_first) >= Duration::from_millis(990));
}

#[test]
fn only_what_is_due_is_asked_and_the_record_keeps_only_what_is_linked_and_checked() {
    let looker = StandIn::new(|_: &str, _| status(200));
    let was = "statac links 1\n\
               http://a.example/fresh/\tok\t2026-10-05\n\
               http://a.example/stale/\tok\t2026-08-01\n\
               http://b.example/\tunanswered 503\t2026-09-28\n\
               http://c.example/\t404\t2026-10-03\n\
               http://gone.example/\tok\t2026-10-05\n\
               http://skip.example/a/\tok\t2026-10-05\n";
    let settings =
        "check_external_links: true\nunchecked_external_links:\n  - https://www.skip.example\n";
    let urls = [
        "http://a.example/fresh/",
        "http://a.example/stale/",
        "http://b.example/",
        "http://c.example/",
        "http://new.example/",
        "http://skip.example/a/",
        "http://skip.example/b/",
    ];
    let (said, code, written) = check_with(&looker, settings, &urls, was);
    assert_eq!(code, 0);
    let mut asked: Vec<String> = looker
        .asked()
        .into_iter()
        .map(|(address, _)| address)
        .collect();
    asked.sort_unstable();
    assert_eq!(
        asked,
        [
            "http://a.example/stale/",
            "http://b.example/",
            "http://new.example/"
        ]
    );
    assert_eq!(
        written.unwrap(),
        "statac links 1\n\
         http://a.example/fresh/\tok\t2026-10-05\n\
         http://a.example/stale/\tok\t2026-10-06\n\
         http://b.example/\tok\t2026-10-06\n\
         http://c.example/\t404\t2026-10-03\n\
         http://new.example/\tok\t2026-10-06\n"
    );
    assert_eq!(
        said,
        "Checking 3 external links on 3 sites.\nChecked 3 external links: 3 work.\n"
    );
}

#[test]
fn with_nothing_due_nothing_is_asked_and_the_record_is_written_only_where_it_changes() {
    let looker = StandIn::new(|_: &str, _| status(200));
    let was = "statac links 1\nhttp://a.example/\tok\t2026-10-05\n";
    let (said, _, written) = check_with(
        &looker,
        "check_external_links: true\n",
        &["http://a.example/"],
        was,
    );
    assert_eq!(said, "No external links are due to be checked.\n");
    assert_eq!(written.as_deref(), Some(was));
    assert_eq!(looker.asked(), []);

    let (_, _, written) = check_with(&looker, "check_external_links: true\n", &[], "");
    assert_eq!(written, None);
    let (_, _, written) = check_with(&looker, "check_external_links: true\n", &[], was);
    assert_eq!(written.as_deref(), Some("statac links 1\n"));
}

#[test]
fn a_permanent_redirect_is_followed_to_where_the_permanent_ones_lead() {
    let url = |text: &str| url::Url::parse(text).unwrap();
    let led = |redirects: &[(u16, &str)]| {
        let redirects: Vec<(u16, url::Url)> = redirects
            .iter()
            .map(|(status, to)| (*status, url(to)))
            .collect();
        moved_to(&redirects)
    };
    assert_eq!(led(&[]), None);
    assert_eq!(led(&[(302, "https://a.example/")]), None);
    assert_eq!(
        led(&[(307, "https://a.example/"), (301, "https://b.example/")]),
        None
    );
    assert_eq!(
        led(&[
            (301, "https://a.example/#part"),
            (302, "https://b.example/")
        ])
        .as_deref(),
        Some("https://a.example/")
    );
    assert_eq!(
        led(&[(301, "https://a.example/"), (308, "https://b.example/x")]).as_deref(),
        Some("https://b.example/x")
    );
}

use std::collections::HashSet;
