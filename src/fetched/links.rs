//! What the sites a page links to answered when `statac refresh` asked
//! them, kept in `fetched/links` so `check` can tell a broken link with no
//! network of its own.
//!
//! The record is text Statac alone writes: its first line `statac links
//! 1`, then a line a URL, sorted, of three fields parted by tabs: the URL
//! as the URL standard writes it, without its fragment; the answer; and
//! the day it was asked. The answer is `ok`, `moved` and the URL a
//! permanent redirect led to, `404`, `410`, `no-host`, `certificate`, or
//! `unanswered` and why: a status, or one of [`REASONS`]. A line whose URL
//! no page links any more is dropped the next time the record is written.

use std::collections::BTreeMap;

use url::Url;

use crate::date::Date;
use crate::problem::{Kind, Problem, RecordFault};
use crate::source::{Place, SourceId, Sources, Span};

/// The record's name in `fetched/`.
pub(crate) const NAME: &str = "links";

const OPENING: &str = "statac links 1";

/// Why an answer said nothing of the link, besides a status.
pub(crate) const REASONS: &[&str] = &[
    // The site sent a page that asks a person to prove they are one.
    "challenge",
    "timeout",
    // A secure connection that couldn't be made, for any reason but an
    // expired certificate or one for another host.
    "tls",
    // The host stands for an address off the open web.
    "private",
    "redirects",
    // The connection was refused or broken, or anything else.
    "failed",
];

/// How long an answer that said nothing holds, in days.
const UNANSWERED_HOLDS: i64 = 7;

/// How long a success holds at least, in days, and how many days more
/// it may hold, spread by the URL so a site's links don't all fall due on
/// one day.
const ANSWERED_HOLDS: i64 = 30;
const ANSWERED_SPREAD: u8 = 16;

/// What a site answered for a link.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Answer {
    /// A success.
    Ok,
    /// A success, after permanent redirects to this URL.
    Moved(String),
    /// `404` or `410`.
    Missing(u16),
    /// No name server knows the host.
    NoHost,
    /// The host's certificate has expired or names another host.
    Certificate,
    /// Nothing that says whether the link works: a status, or one of
    /// [`REASONS`].
    Unanswered(String),
}

impl Answer {
    /// Whether the link is broken.
    pub(crate) fn broken(&self) -> bool {
        matches!(
            self,
            Answer::Missing(_) | Answer::NoHost | Answer::Certificate
        )
    }

    /// Whether the site answered with a success.
    pub(crate) fn answered(&self) -> bool {
        matches!(self, Answer::Ok | Answer::Moved(_))
    }

    fn written(&self) -> String {
        match self {
            Answer::Ok => "ok".into(),
            Answer::Moved(to) => format!("moved {to}"),
            Answer::Missing(status) => status.to_string(),
            Answer::NoHost => "no-host".into(),
            Answer::Certificate => "certificate".into(),
            Answer::Unanswered(reason) => format!("unanswered {reason}"),
        }
    }

    fn read(written: &str) -> Option<Answer> {
        Some(match written {
            "ok" => Answer::Ok,
            "404" => Answer::Missing(404),
            "410" => Answer::Missing(410),
            "no-host" => Answer::NoHost,
            "certificate" => Answer::Certificate,
            _ => match written.split_once(' ')? {
                ("moved", to) => Answer::Moved(link(to)?),
                ("unanswered", reason) if REASONS.contains(&reason) || status(reason) => {
                    Answer::Unanswered(reason.to_owned())
                }
                _ => return None,
            },
        })
    }
}

/// Whether `written` is a status, three digits from 100 to 599.
fn status(written: &str) -> bool {
    written.len() == 3
        && written
            .parse::<u16>()
            .is_ok_and(|n| (100..600).contains(&n))
}

/// `written` where it is a full URL as the record writes one.
fn link(written: &str) -> Option<String> {
    let url = Url::parse(written).ok()?;
    let key = key(&url)?;
    (key == written).then_some(key)
}

/// The URL a link is asked and recorded by, where it is an `http` or
/// `https` URL with a host: as the URL standard writes it, without its
/// fragment.
pub(crate) fn key(url: &Url) -> Option<String> {
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return None;
    }
    let mut url = url.clone();
    url.set_fragment(None);
    Some(url.into())
}

/// One URL's line.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Line {
    pub(crate) answer: Answer,
    /// The day it was asked.
    pub(crate) day: Date,
}

impl Line {
    /// Whether `url`, answered as this line says, is due to be asked
    /// again on `today`.
    pub(crate) fn due(&self, url: &str, today: &Date) -> bool {
        let holds = match self.answer.answered() {
            true => ANSWERED_HOLDS + i64::from(spread(url)),
            false => UNANSWERED_HOLDS,
        };
        super::days_between(&self.day, today).is_none_or(|days| days >= holds)
    }
}

/// How many days past the least a success for `url` holds.
fn spread(url: &str) -> u8 {
    blake3::hash(url.as_bytes()).as_bytes()[0] % ANSWERED_SPREAD
}

/// The record, each URL's line by the URL.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub(crate) struct Links {
    lines: BTreeMap<String, Line>,
}

impl Links {
    /// Reads the record held in `source`, with a problem at each line that
    /// doesn't read; such a line is left out.
    pub(crate) fn read(sources: &Sources, source: SourceId) -> (Links, Vec<Problem>) {
        let text = sources.text(source);
        let mut links = Links::default();
        let mut problems = Vec::new();
        let malformed = |fault, at: Place| Problem::at(Kind::MalformedRecord { fault }, at);
        if text.is_empty() {
            let place = Place {
                source,
                span: Span::new(0, 0),
            };
            return (links, vec![malformed(RecordFault::LinksOpening, place)]);
        }
        let mut start = 0;
        for (number, line) in text.split_inclusive('\n').enumerate() {
            let at = start;
            start += line.len();
            let line = line.trim_end_matches('\n').trim_end_matches('\r');
            let place = |from: usize, len: usize| Place {
                source,
                span: Span::new(at + from, at + from + len),
            };
            if number == 0 {
                if line != OPENING {
                    let fault = RecordFault::LinksOpening;
                    problems.push(malformed(fault, place(0, line.len())));
                    return (links, problems);
                }
                continue;
            }
            let mut fields = Vec::new();
            let mut from = 0;
            for field in line.split('\t') {
                fields.push((field, place(from, field.len())));
                from += field.len() + 1;
            }
            let [(url, url_at), (answer, answer_at), (day, day_at)] = fields[..] else {
                let fault = RecordFault::LinkFields(fields.len());
                problems.push(malformed(fault, place(0, line.len())));
                continue;
            };
            let Some(url) = link(url) else {
                problems.push(malformed(RecordFault::NotALink(url.into()), url_at));
                continue;
            };
            let Some(answer) = Answer::read(answer) else {
                problems.push(malformed(RecordFault::Answer(answer.into()), answer_at));
                continue;
            };
            let day = match crate::date::read(day) {
                Ok(read) if read.time.is_none() && read.iso_day() == day => read,
                _ => {
                    problems.push(malformed(RecordFault::Day(day.into()), day_at));
                    continue;
                }
            };
            if links.lines.contains_key(&url) {
                problems.push(malformed(RecordFault::Twice(url), url_at));
                continue;
            }
            links.lines.insert(url, Line { answer, day });
        }
        (links, problems)
    }

    /// The line of `url`, as [`key`] writes it.
    pub(crate) fn get(&self, url: &str) -> Option<&Line> {
        self.lines.get(url)
    }

    pub(crate) fn set(&mut self, url: &str, line: Line) {
        self.lines.insert(url.to_owned(), line);
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.lines.len()
    }

    /// The record as written, with only the lines of the URLs `kept`.
    pub(crate) fn text(&self, kept: impl Fn(&str) -> bool) -> String {
        let mut text = format!("{OPENING}\n");
        for (url, line) in self.lines.iter().filter(|(url, _)| kept(url)) {
            let answer = line.answer.written();
            text.push_str(&format!("{url}\t{answer}\t{}\n", line.day.iso_day()));
        }
        text
    }
}

/// The record as a problem names it, from the site's folder.
pub(crate) fn shown() -> String {
    format!("{}/{NAME}", super::FOLDER)
}

/// What the record says of the links to other sites the site's pages
/// hold, as `check` and `build` say it.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub(crate) struct Tally {
    /// Every URL a page links to on another site, sorted, each once.
    pub(crate) urls: Vec<String>,
    /// Those a site answered with a success.
    pub(crate) answered: usize,
    pub(crate) broken: usize,
    /// Those whose answer said nothing of the link.
    pub(crate) unanswered: usize,
    /// Those the site's list says not to check.
    pub(crate) unchecked: usize,
    /// Those the record has no answer for.
    pub(crate) unasked: usize,
    /// How many days before the build's the oldest answer was given.
    pub(crate) oldest: Option<i64>,
}

impl Tally {
    /// How many URLs the record has an answer for.
    pub(crate) fn asked(&self) -> usize {
        self.answered + self.broken + self.unanswered
    }
}

#[cfg(test)]
mod tests;
