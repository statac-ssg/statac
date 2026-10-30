//! The last step of `statac refresh`, where `check_external_links: true`
//! asks for it: each link to another site that the site's pages hold and
//! `unchecked_external_links` doesn't list is asked about once its answer
//! in `fetched/links` is due, and the answer recorded with the day.
//!
//! The asking is polite: one request at a time to a host, with a pause
//! between the end of one and the start of the next, and a few hosts at
//! once, the busiest first since it decides how long the step takes. A
//! link is asked with `HEAD`, and again with `GET` where that gets an
//! answer that isn't a success or none in time, since some sites answer
//! `HEAD` wrongly. A host that can't be reached at all at its first link
//! answers for the rest of its links. A host asking Statac to wait is
//! waited for up to a minute, and past that left for the next refresh. A
//! challenge, a page asking the visitor to prove they are a person, is
//! recorded as no answer, never got round.
//!
//! An answer that no connection was made for may only mean the computer
//! is offline. Such answers are recorded only once some site has answered
//! since, or at the end where some site answered at all; many in a row
//! stop the step, recording none of them.

use std::collections::{BTreeMap, HashSet, VecDeque};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, mpsc};
use std::time::Duration;

use crate::config::Settings;
use crate::config::refused::Refused;
use crate::date::Date;
use crate::fetch::{Ending, Looked, Method, Refusal};
use crate::fetched::links::{self as record, Answer, Line, Links};
use crate::report::Report;
use crate::wording;

/// The setting that asks for links to other sites to be checked.
pub(crate) const CHECK_EXTERNAL: &str = "check_external_links";

/// The setting listing the URLs not to check.
pub(crate) const UNCHECKED_EXTERNAL: &str = "unchecked_external_links";

/// The pause between two requests to one host.
const PAUSE: Duration = Duration::from_secs(1);

/// How many hosts are asked at once.
const HOSTS_AT_ONCE: usize = 8;

/// The longest a host asking Statac to wait is waited for, in seconds.
const LONGEST_WAIT: u64 = 60;

/// How many answers in a row with no connection made stop the step.
const SILENT_IN_A_ROW: usize = 16;

/// How many answers are recorded between two writings of the record, and
/// between two lines saying how far the step has got.
const KEPT_EVERY: usize = 200;

/// What asks for a URL without fetching it: [`Network`], or whatever a
/// test stands in with.
pub trait Look: Sync {
    fn look(&self, address: &str, method: Method) -> Looked;

    /// How long to wait between two requests to one host.
    fn pause(&self) -> Duration;
}

/// The open web, asked politely.
pub struct Network;

impl Look for Network {
    fn look(&self, address: &str, method: Method) -> Looked {
        crate::fetch::look(address, method, PAUSE)
    }

    fn pause(&self) -> Duration {
        PAUSE
    }
}

/// What the step is handed.
pub(super) struct Checking<'a> {
    pub(super) folder: &'a Path,
    /// What `fetched/links` held as the site was read.
    pub(super) was: &'a Links,
    pub(super) settings: &'a Settings,
    /// Every URL a page links to on another site, sorted, each once.
    pub(super) urls: &'a [String],
    pub(super) today: Date,
    pub(super) report: &'a Report<'a>,
}

/// What a worker tells the step.
enum Told {
    /// What `urls`, all on one host, answered, by one request: whether
    /// any connection was made for it.
    Asked {
        urls: Vec<String>,
        answer: Answer,
        connected: bool,
    },
    /// A host that asked Statac to wait longer than it waits, with how
    /// many of its links are left unasked.
    Left { host: String, count: usize },
}

/// Checks each link due, saying on `say` what it does, and writes the
/// record. Gives 1 where the record couldn't be written.
pub(super) fn check(checking: &Checking, looker: &dyn Look, say: &mut dyn FnMut(&str)) -> u8 {
    let unchecked = Refused::listed(checking.settings, UNCHECKED_EXTERNAL);
    let wanted: Vec<&str> = (checking.urls.iter())
        .map(String::as_str)
        .filter(|url| unchecked.covering(url).is_none())
        .collect();
    let kept: HashSet<&str> = wanted.iter().copied().collect();
    let today = checking.today;
    let due: Vec<&str> = (wanted.iter().copied())
        .filter(|url| (checking.was.get(url)).is_none_or(|line| line.due(url, &today)))
        .collect();
    let mut links = checking.was.clone();
    let mut code = 0;
    let mut keep = |links: &Links, say: &mut dyn FnMut(&str)| {
        let text = links.text(|url| kept.contains(url));
        if text == checking.was.text(|_| true) {
            return;
        }
        if let Err(error) = super::keep(checking.folder, record::NAME, text.as_bytes()) {
            say(&super::unwritten(checking.report, &record::shown(), &error));
            code = 1;
        }
    };
    if due.is_empty() {
        keep(&links, say);
        say(&format!("{}\n", wording::NO_LINKS_DUE));
        return code;
    }
    let mut hosts: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for url in &due {
        let host = url::Url::parse(url).ok();
        let host = host.and_then(|url| url.host_str().map(str::to_owned));
        hosts
            .entry(host.unwrap_or_default())
            .or_default()
            .push((*url).to_owned());
    }
    let mut queue: Vec<(String, Vec<String>)> = hosts.into_iter().collect();
    queue.sort_by_key(|(_, urls)| std::cmp::Reverse(urls.len()));
    say(&format!(
        "{}\n",
        wording::checking_links(due.len(), queue.len())
    ));
    let workers = queue.len().min(HOSTS_AT_ONCE);
    let queue = Mutex::new(VecDeque::from(queue));
    let stop = AtomicBool::new(false);
    let (sent, received) = mpsc::channel();
    let mut tally = Tally::default();
    std::thread::scope(|scope| {
        for _ in 0..workers {
            let sent = sent.clone();
            let (queue, stop) = (&queue, &stop);
            scope.spawn(move || {
                while let Some((host, urls)) = next(queue) {
                    if stop.load(Ordering::Relaxed) {
                        break;
                    }
                    ask_host(looker, &host, &urls, stop, &sent);
                }
            });
        }
        drop(sent);
        // Answers with no connection made, waiting for some site to answer.
        let mut waiting: Vec<(Vec<String>, Answer)> = Vec::new();
        let mut in_a_row = 0;
        let mut since_kept = 0;
        for told in received {
            let (urls, answer, connected) = match told {
                Told::Left { host, count } => {
                    tally.left.push((host, count));
                    continue;
                }
                Told::Asked {
                    urls,
                    answer,
                    connected,
                } => (urls, answer, connected),
            };
            if tally.stopped && !connected {
                continue;
            }
            let mut set = |urls: &[String], answer: &Answer, tally: &mut Tally| {
                for url in urls {
                    let line = Line {
                        answer: answer.clone(),
                        day: today,
                    };
                    links.set(url, line);
                    tally.count(answer);
                }
                since_kept += urls.len();
            };
            match connected {
                true => {
                    tally.connected = true;
                    in_a_row = 0;
                    for (urls, answer) in waiting.drain(..) {
                        set(&urls, &answer, &mut tally);
                    }
                    set(&urls, &answer, &mut tally);
                }
                false => {
                    in_a_row += 1;
                    waiting.push((urls, answer));
                    if in_a_row >= SILENT_IN_A_ROW && !tally.stopped {
                        tally.stopped = true;
                        stop.store(true, Ordering::Relaxed);
                        waiting.clear();
                    }
                }
            }
            if since_kept >= KEPT_EVERY {
                since_kept = 0;
                keep(&links, say);
                let line = wording::links_so_far(tally.asked(), due.len());
                say(&format!("{line}\n"));
            }
        }
        if tally.connected {
            for (urls, answer) in waiting.drain(..) {
                for url in &urls {
                    let line = Line {
                        answer: answer.clone(),
                        day: today,
                    };
                    links.set(url, line);
                    tally.count(&answer);
                }
            }
        } else if !waiting.is_empty() {
            tally.unheard = true;
        }
    });
    keep(&links, say);
    for (host, count) in &tally.left {
        say(&format!("{}\n", wording::links_left(host, *count)));
    }
    if tally.stopped {
        say(&format!("{}\n", wording::links_stopped(SILENT_IN_A_ROW)));
    } else if tally.unheard {
        say(&format!("{}\n", wording::LINKS_UNHEARD));
    }
    if tally.asked() > 0 {
        say(&format!(
            "{}\n",
            wording::links_checked(
                tally.asked(),
                tally.answered,
                tally.broken,
                tally.unanswered
            )
        ));
    }
    code
}

/// What the step found.
#[derive(Default)]
struct Tally {
    answered: usize,
    broken: usize,
    unanswered: usize,
    /// The hosts left for the next refresh, with how many links each.
    left: Vec<(String, usize)>,
    /// Whether any request made a connection.
    connected: bool,
    /// Whether too many answers in a row made none, stopping the step.
    stopped: bool,
    /// Whether answers that made no connection were left unrecorded, no
    /// request having made one.
    unheard: bool,
}

impl Tally {
    fn count(&mut self, answer: &Answer) {
        match answer {
            answer if answer.answered() => self.answered += 1,
            answer if answer.broken() => self.broken += 1,
            _ => self.unanswered += 1,
        }
    }

    fn asked(&self) -> usize {
        self.answered + self.broken + self.unanswered
    }
}

fn next(queue: &Mutex<VecDeque<(String, Vec<String>)>>) -> Option<(String, Vec<String>)> {
    let mut queue = queue
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    queue.pop_front()
}

/// Asks each of `urls`, all on `host`, one at a time, telling `sent` each
/// answer.
fn ask_host(
    looker: &dyn Look,
    host: &str,
    urls: &[String],
    stop: &AtomicBool,
    sent: &mpsc::Sender<Told>,
) {
    let mut wait = Duration::ZERO;
    for (at, url) in urls.iter().enumerate() {
        if stop.load(Ordering::Relaxed) {
            return;
        }
        std::thread::sleep(wait);
        let asked = ask(looker, url);
        wait = looker.pause();
        // A host that couldn't be reached at its first link answers for
        // the rest of its links the same.
        let whole_host = asked.unreached && at == 0;
        let answering = match whole_host {
            true => urls.to_vec(),
            false => vec![url.clone()],
        };
        let _ = sent.send(Told::Asked {
            urls: answering,
            answer: asked.answer,
            connected: asked.connected,
        });
        if whole_host {
            return;
        }
        if let Some(seconds) = asked.wait {
            if seconds > LONGEST_WAIT {
                let count = urls_left(urls.len(), at);
                if count > 0 {
                    let host = host.to_owned();
                    let _ = sent.send(Told::Left { host, count });
                }
                return;
            }
            wait = wait.max(Duration::from_secs(seconds));
        }
    }
}

fn urls_left(of: usize, at: usize) -> usize {
    of - at - 1
}

/// What asking one URL found.
struct Asked {
    answer: Answer,
    /// Whether any connection was made.
    connected: bool,
    /// Whether the URL's own host couldn't be reached at all.
    unreached: bool,
    /// How many seconds the host asked Statac to wait before the next
    /// request, where it asked.
    wait: Option<u64>,
}

/// Asks `url` with `HEAD`, then with `GET` where `HEAD` got an answer
/// that isn't a success, or none in time.
fn ask(looker: &dyn Look, url: &str) -> Asked {
    let head = looker.look(url, Method::Head);
    let again = match &head.end {
        Ending::Status {
            status, challenge, ..
        } => *challenge || !(200..300).contains(status),
        Ending::Refused(refusal) => matches!(refusal, Refusal::TimedOut | Refusal::Failed { .. }),
    };
    let looked = match again {
        true => {
            std::thread::sleep(looker.pause());
            looker.look(url, Method::Get)
        }
        false => head,
    };
    asked(&looked)
}

/// What `looked` says of the link.
fn asked(looked: &Looked) -> Asked {
    let unanswered = |reason: &str| Answer::Unanswered(reason.to_owned());
    match &looked.end {
        Ending::Status {
            status,
            challenge,
            retry_after,
        } => Asked {
            answer: match status {
                _ if *challenge => unanswered("challenge"),
                200..=299 => moved_to(&looked.redirects).map_or(Answer::Ok, Answer::Moved),
                404 | 410 => Answer::Missing(*status),
                status => unanswered(&status.to_string()),
            },
            connected: true,
            unreached: false,
            wait: retry_after.filter(|_| matches!(status, 429 | 503)),
        },
        Ending::Refused(refusal) => {
            let (answer, connected) = match refusal {
                Refusal::UnknownHost { .. } => (Answer::NoHost, false),
                Refusal::Certificate { .. } => (Answer::Certificate, true),
                Refusal::Tls { .. } => (unanswered("tls"), true),
                Refusal::TimedOut => (unanswered("timeout"), false),
                Refusal::PrivateAddress { .. } => (unanswered("private"), true),
                Refusal::Redirects { .. }
                | Refusal::NoRedirectTarget { .. }
                | Refusal::Scheme { .. }
                | Refusal::NoHost
                | Refusal::NotAnAddress => (unanswered("redirects"), true),
                Refusal::LookupFailed { .. }
                | Refusal::Failed { .. }
                | Refusal::Credentials
                | Refusal::Missing { .. }
                | Refusal::Status { .. }
                | Refusal::TooLarge { .. } => (unanswered("failed"), false),
            };
            // Past a redirect, it is another URL's host that wasn't
            // reached.
            let first = looked.redirects.is_empty();
            let unreached = first
                && matches!(
                    refusal,
                    Refusal::UnknownHost { .. }
                        | Refusal::LookupFailed { .. }
                        | Refusal::Certificate { .. }
                        | Refusal::Tls { .. }
                        | Refusal::TimedOut
                        | Refusal::PrivateAddress { .. }
                        | Refusal::Failed { .. }
                );
            Asked {
                answer,
                connected: connected || !first,
                unreached,
                wait: None,
            }
        }
    }
}

/// Where the permanent redirects `redirects` opens with led, where it
/// opens with one.
fn moved_to(redirects: &[(u16, url::Url)]) -> Option<String> {
    let permanent = redirects
        .iter()
        .take_while(|(status, _)| matches!(status, 301 | 308));
    record::key(&permanent.last()?.1)
}

#[cfg(test)]
mod tests;
