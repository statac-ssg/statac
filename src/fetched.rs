//! What Statac fetched from elsewhere: the files `statac refresh` brought
//! into the site's `fetched/` folder, and the record of each. The author
//! commits the folder with the site, so a build of it is the same on any
//! machine, and `build`, `check` and `serve` only ever read it.
//!
//! The record, `fetched/record`, is text Statac alone writes: its first
//! line `statac fetched 1`, then a line a file, sorted by path, of three
//! fields parted by tabs: the file's path under `fetched/`, the first 16
//! hex digits of the BLAKE3 hash of its bytes or `absent`, and the day it
//! was fetched. `absent` is a file the site asked for that the other site
//! answered it has none of, or that `statac refresh` chose not to keep,
//! such as a refused mention, which is not asked for again. A file with no
//! line is the author's own; a line whose file has gone is dropped the
//! next time the record is written.
//!
//! What the sites a page links to answered is kept apart, in
//! `fetched/links`, since a link has no file.

pub(crate) mod links;
pub(crate) mod mentions;
pub(crate) mod stills;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::date::Date;
use crate::problem::{Kind, Problem, Reason, RecordFault, Unread};
use crate::source::{Place, SourceId, Sources, Span};

/// The folder at the top of the site.
pub(crate) const FOLDER: &str = "fetched";

/// The record's name in the folder.
pub(crate) const RECORD: &str = "record";

const OPENING: &str = "statac fetched 1";

/// How many hex digits of a file's hash the record keeps.
const HASHED: usize = 16;

/// What the record says a file is.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Held {
    /// The opening of the hash of the bytes fetched.
    Hash(String),
    /// The other site answered that it has no such thing.
    Absent,
}

/// One file's line.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Line {
    pub(crate) held: Held,
    /// The day it was fetched, or found absent.
    pub(crate) day: Date,
}

/// The record, each file's line by its path under `fetched/`.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub(crate) struct Record {
    lines: BTreeMap<String, Line>,
}

/// What the record keeps of `bytes`.
pub(crate) fn hash_of(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex()[..HASHED].to_owned()
}

/// A file under `fetched/` as a build's record of reads knows it, by its
/// path there.
pub(crate) fn key(path: &str) -> u64 {
    let hash = blake3::hash(path.as_bytes());
    let mut opening = [0; 8];
    opening.copy_from_slice(&hash.as_bytes()[..8]);
    u64::from_le_bytes(opening)
}

impl Record {
    /// Reads the record held in `source`, with a problem at each line that
    /// doesn't read; such a line is left out.
    pub(crate) fn read(sources: &Sources, source: SourceId) -> (Record, Vec<Problem>) {
        let text = sources.text(source);
        let mut record = Record::default();
        let mut problems = Vec::new();
        if text.is_empty() {
            let place = Place {
                source,
                span: Span::new(0, 0),
            };
            let kind = Kind::MalformedRecord {
                fault: RecordFault::Opening,
            };
            return (record, vec![Problem::at(kind, place)]);
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
            let malformed = |fault, at: Place| Problem::at(Kind::MalformedRecord { fault }, at);
            if number == 0 {
                if line != OPENING {
                    problems.push(malformed(RecordFault::Opening, place(0, line.len())));
                    return (record, problems);
                }
                continue;
            }
            let mut fields = Vec::new();
            let mut from = 0;
            for field in line.split('\t') {
                fields.push((field, place(from, field.len())));
                from += field.len() + 1;
            }
            let [(path, path_at), (held, held_at), (day, day_at)] = fields[..] else {
                let fault = RecordFault::Fields(fields.len());
                problems.push(malformed(fault, place(0, line.len())));
                continue;
            };
            if path.is_empty() || !crate::platform::stays_inside(path) {
                problems.push(malformed(RecordFault::Path(path.into()), path_at));
                continue;
            }
            let hex = |hash: &str| hash.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
            let held = match held {
                "absent" => Held::Absent,
                hash if hash.len() == HASHED && hex(hash) => Held::Hash(hash.to_owned()),
                other => {
                    problems.push(malformed(RecordFault::Held(other.into()), held_at));
                    continue;
                }
            };
            // Only as Statac writes a day: no time, no offset, no other
            // spelling.
            let day = match crate::date::read(day) {
                Ok(read) if read.time.is_none() && read.iso_day() == day => read,
                _ => {
                    problems.push(malformed(RecordFault::Day(day.into()), day_at));
                    continue;
                }
            };
            if record.lines.contains_key(path) {
                problems.push(malformed(RecordFault::Twice(path.into()), path_at));
                continue;
            }
            record.lines.insert(path.to_owned(), Line { held, day });
        }
        (record, problems)
    }

    /// The line of the file at `path` under `fetched/`.
    pub(crate) fn get(&self, path: &str) -> Option<&Line> {
        self.lines.get(path)
    }

    pub(crate) fn set(&mut self, path: &str, line: Line) {
        self.lines.insert(path.to_owned(), line);
    }

    /// Every line, by path, in order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&str, &Line)> {
        self.lines.iter().map(|(path, line)| (path.as_str(), line))
    }

    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// The record as written, but for the lines of files `standing` says
    /// are gone: a line saying `absent` stands for no file, and stays.
    pub(crate) fn text(&self, standing: impl Fn(&str) -> bool) -> String {
        let mut text = format!("{OPENING}\n");
        for (path, line) in &self.lines {
            let held = match &line.held {
                Held::Hash(_) if !standing(path) => continue,
                Held::Hash(hash) => hash.as_str(),
                Held::Absent => "absent",
            };
            text.push_str(&format!("{path}\t{held}\t{}\n", line.day.iso_day()));
        }
        text
    }
}

/// What the site's `fetched/` holds.
#[derive(Default, Debug)]
pub(crate) struct Fetched {
    pub(crate) record: Record,
    pub(crate) links: links::Links,
    /// Every file in the folder but the two records and the retired list
    /// of refused webmentions, by its path under it with forward slashes,
    /// with where it really is: its path past every link under `real`.
    pub(crate) files: Vec<(String, String)>,
    /// The site's real path.
    pub(crate) real: PathBuf,
}

/// How old the stills in `fetched/stills/`, the mentions in
/// `fetched/mentions/` and the answers in `fetched/links` are, as `build`
/// says.
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub(crate) struct Ages {
    pub(crate) stills: usize,
    /// How many days before `today` the oldest still the record dates
    /// was fetched.
    pub(crate) oldest: Option<i64>,
    /// The stills the record doesn't date.
    pub(crate) undated: usize,
    pub(crate) mentions: usize,
    /// How many days before `today` the newest mention the record dates
    /// was fetched.
    pub(crate) newest: Option<i64>,
    /// The links to other sites `fetched/links` has an answer for, where
    /// the site checks them.
    pub(crate) links: usize,
    /// How many days before `today` the oldest of those answers was given.
    pub(crate) links_oldest: Option<i64>,
}

impl Ages {
    /// The ages of the stills among `pictures`, the paths under
    /// `fetched/` of the pictures the site shows from there, and of the
    /// mentions `fetched` holds, on the day `today`.
    pub(crate) fn of<'a>(
        fetched: &Fetched,
        pictures: impl Iterator<Item = &'a str>,
        today: &Date,
    ) -> Ages {
        let mut ages = Ages::default();
        let folder = format!("{}/", stills::FOLDER);
        for still in pictures.filter(|path| path.starts_with(&folder)) {
            ages.stills += 1;
            let line = fetched.record.get(still);
            let fetched = line.filter(|line| matches!(line.held, Held::Hash(_)));
            match fetched.and_then(|line| days_between(&line.day, today)) {
                Some(days) => {
                    ages.oldest = Some(ages.oldest.map_or(days, |oldest| oldest.max(days)))
                }
                None => ages.undated += 1,
            }
        }
        let files = fetched.files.iter();
        for (path, _) in files.filter(|(path, _)| mentions::mention_file(path)) {
            ages.mentions += 1;
            let line = fetched.record.get(path);
            let fetched = line.filter(|line| matches!(line.held, Held::Hash(_)));
            if let Some(days) = fetched.and_then(|line| days_between(&line.day, today)) {
                ages.newest = Some(ages.newest.map_or(days, |newest| newest.min(days)));
            }
        }
        ages
    }
}

/// How many days `to` is after `from`.
pub(crate) fn days_between(from: &Date, to: &Date) -> Option<i64> {
    let civil = |date: &Date| {
        jiff::civil::Date::new(
            i16::try_from(date.year).ok()?,
            i8::try_from(date.month).ok()?,
            i8::try_from(date.day).ok()?,
        )
        .ok()
    };
    let span = civil(from)?.until(civil(to)?).ok()?;
    Some(i64::from(span.get_days()))
}

/// The record as a problem names it, from the site's folder.
pub(crate) fn record_shown() -> String {
    format!("{FOLDER}/{RECORD}")
}

/// Reads the site's `fetched/` as `content/` is read: only what is inside
/// it, a link followed only where it stays inside. A site with none has
/// nothing fetched, and nothing is said of it.
pub(crate) fn read(site: &Path, sources: &mut Sources) -> (Fetched, Vec<Problem>) {
    let mut problems = Vec::new();
    let real = std::fs::canonicalize(site).unwrap_or_else(|_| site.to_path_buf());
    let mut fetched = Fetched::default();
    for (path, at) in crate::assets::walk(site, &real, FOLDER, false, &mut problems) {
        match path.as_str() {
            RECORD | links::NAME => {}
            crate::config::refused::NAME => {
                problems.push(retired(sources, &real, &at));
                continue;
            }
            _ => {
                fetched.files.push((path, at));
                continue;
            }
        }
        let shown = match path.as_str() {
            RECORD => record_shown(),
            _ => links::shown(),
        };
        let text = match sources.unsaved().read(&real, &at) {
            Ok(text) => text,
            Err(error) => {
                problems.push(Problem::nowhere(Kind::CantRead {
                    path: shown.into(),
                    reason: Reason::of(&error),
                }));
                continue;
            }
        };
        let source = sources.add(shown, text);
        if path == RECORD {
            let (record, found) = Record::read(sources, source);
            fetched.record = record;
            problems.extend(found);
        } else {
            let (links, found) = links::Links::read(sources, source);
            fetched.links = links;
            problems.extend(found);
        }
    }
    fetched.real = real;
    (fetched, problems)
}

/// The problem with the retired list of refused webmentions at `at`,
/// carrying the URLs it holds.
fn retired(sources: &Sources, real: &Path, at: &str) -> Problem {
    let (urls, unread) = match sources.unsaved().read(real, at) {
        Ok(text) => match crate::config::refused::carried(&text) {
            (urls, 0) => (urls, Unread::Nothing),
            (urls, lines) => (urls, Unread::Lines(lines)),
        },
        Err(error) => (Vec::new(), Unread::File(Reason::of(&error))),
    };
    Problem::nowhere(Kind::RetiredFile { urls, unread })
}

#[cfg(test)]
mod tests;
