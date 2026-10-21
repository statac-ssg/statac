//! Save-time stamps: a front matter value that is, whole, one of Statac's
//! words for a date, such as `=today` or `=monday 09:00`, and the date
//! written in its place.
//!
//! A build never picks the date: it would differ from one build to the
//! next. `statac stamp` and `statac serve` write it into the file, once,
//! and a build meeting a stamp still unwritten stops at it.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use jiff::civil;

use crate::build;
use crate::content::words::key_name;
use crate::date::{Date, Time};
use crate::dates::{Dates, ShowDates};
use crate::feeds::{self, IdsToWrite};
use crate::origin::Style;
use crate::platform;
use crate::problem::{Kind, Missing, Problem, Severity, Word};
use crate::rewrite::{self, Edit};
use crate::source::{SourceId, Sources, Span};
use crate::suggest;
use crate::yaml::{Node, Value};

const DAYS: [(&str, Day); 4] = [
    ("today", Day::Today),
    ("now", Day::Now),
    ("tomorrow", Day::Tomorrow),
    ("yesterday", Day::Yesterday),
];

/// Monday first, as [`Day::Weekday`] counts them.
const WEEKDAYS: [&str; 7] = [
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
];

/// The shortest opening of a weekday's name taken as meant for it.
const SHORTEST: usize = 3;

/// The shortest word taken as a slip in one of Statac's: any shorter, and
/// too many words of the author's own are one letter from `now`.
const SHORTEST_SLIP: usize = 5;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Day {
    Today,
    /// The present date and time.
    Now,
    Tomorrow,
    Yesterday,
    /// The next such day after today, Monday being 0.
    Weekday(u8),
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Stamp {
    pub day: Day,
    /// The hour and minute written after the day.
    pub time: Option<(u8, u8)>,
}

/// Why a value meant as a stamp isn't one.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Refused {
    /// Another spelling of the stamp given.
    Respelt(String),
    /// A slip in a word, with the stamp it is nearest.
    Misspelt(String),
    /// A time no clock shows, as written.
    NoSuchTime(String),
    /// A time written as figures alone, which could be read two ways.
    UnclearTime,
}

/// What follows a stamp's day, read as a time.
enum Clock {
    At(u8, u8),
    NoSuch,
    Unclear,
}

impl Stamp {
    /// The one way the stamp is written.
    pub fn spelling(&self) -> String {
        let day = match self.day {
            Day::Today => "today",
            Day::Now => "now",
            Day::Tomorrow => "tomorrow",
            Day::Yesterday => "yesterday",
            Day::Weekday(day) => WEEKDAYS[usize::from(day)],
        };
        match self.time {
            Some((hour, minute)) => format!("={day} {hour:02}:{minute:02}"),
            None => format!("={day}"),
        }
    }

    /// The date written in the stamp's place at `moment`, as the site's
    /// date reader takes it. None where the day falls outside the years
    /// a date can be written in.
    pub fn written(&self, moment: &Moment) -> Option<String> {
        let days = match self.day {
            Day::Now => return Some(spelt(&moment.now)),
            Day::Today => 0,
            Day::Tomorrow => 1,
            Day::Yesterday => -1,
            Day::Weekday(day) => {
                let today = weekday(&moment.today)?;
                // From 1 to 7: the day itself is next week's.
                (i64::from(day) - i64::from(today) + 6).rem_euclid(7) + 1
            }
        };
        let today = civil_date(&moment.today)?;
        let day = today.checked_add(jiff::Span::new().days(days)).ok()?;
        let year = u16::try_from(day.year()).ok()?;
        let date = Date {
            year: i32::from(year),
            month: day.month().unsigned_abs(),
            day: day.day().unsigned_abs(),
            time: self.time.map(|(hour, minute)| Time {
                hour,
                minute,
                second: None,
            }),
            offset: None,
        };
        Some(spelt(&date))
    }
}

fn civil_date(date: &Date) -> Option<civil::Date> {
    civil::Date::new(
        i16::try_from(date.year).ok()?,
        i8::try_from(date.month).ok()?,
        i8::try_from(date.day).ok()?,
    )
    .ok()
}

/// The weekday `date` falls on, Monday being 0.
fn weekday(date: &Date) -> Option<i8> {
    Some(civil_date(date)?.weekday().to_monday_zero_offset())
}

/// A date as a person would write it for the date reader: the day, then a
/// space and the time, then the offset where it has one.
fn spelt(date: &Date) -> String {
    let mut text = date.iso_day();
    if let Some(time) = date.time {
        text.push_str(&format!(" {:02}:{:02}", time.hour, time.minute));
    }
    if let Some(minutes) = date.offset {
        let sign = if minutes < 0 { '-' } else { '+' };
        let minutes = minutes.unsigned_abs();
        text.push_str(&format!(" {sign}{:02}:{:02}", minutes / 60, minutes % 60));
    }
    text
}

/// When stamps are written: the site's day, and the present minute.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Moment {
    pub today: Date,
    pub now: Date,
}

impl Moment {
    /// The moment on the site's clock. Where the day is `fixed`, so that
    /// a run can be repeated, the present minute is its midnight.
    pub fn of(dates: &Dates, fixed: bool) -> Moment {
        let today = dates.today();
        let midnight = Time {
            hour: 0,
            minute: 0,
            second: None,
        };
        let now = match fixed {
            true => Date {
                time: Some(midnight),
                ..today
            },
            false => dates.now(),
        };
        Moment { today, now }
    }
}

/// Reads a front matter value as a stamp. None for the author's own text:
/// anything not opening with `=`, and anything that isn't one of Statac's
/// words for a day, or near one, with nothing after it but a time. So
/// `=IF(A1)`, `=TODAY()` and `=today's news` are text, and `=Today`,
/// `=mon`, `=next monday`, `=tomorow` and `=today 9am` are refused.
pub fn read(text: &str) -> Option<Result<Stamp, Refused>> {
    let written = text.strip_prefix('=')?.trim();
    // Statac's words are ASCII, and lowering only ASCII keeps every
    // letter where it was.
    let lower = written.to_ascii_lowercase();
    let (mut word, mut after) = word_and_rest(&lower);
    if word == "next" {
        // A weekday already means the next one.
        let (second, more) = word_and_rest(after);
        named_weekday(second)?;
        (word, after) = (second, more);
    }
    let named = DAYS.iter().find(|(name, _)| *name == word);
    let (day, slip) = match (named, named_weekday(word)) {
        (Some(&(_, day)), _) | (None, Some(day)) => (day, false),
        (None, None) if word.len() >= SHORTEST_SLIP => {
            let names = DAYS.iter().map(|(name, _)| *name).chain(WEEKDAYS);
            (day_named(suggest::nearest(word, names)?)?, true)
        }
        _ => return None,
    };
    let time = match after {
        "" => None,
        after => match clock(after)? {
            Clock::At(hour, minute) => Some((hour, minute)),
            Clock::NoSuch => {
                let time = &written[written.len() - after.len()..];
                return Some(Err(Refused::NoSuchTime(time.to_owned())));
            }
            Clock::Unclear => return Some(Err(Refused::UnclearTime)),
        },
    };
    let stamp = Stamp {
        // The present has no other time: a time after `=now` is today's.
        day: match (day, time) {
            (Day::Now, Some(_)) => Day::Today,
            _ => day,
        },
        time,
    };
    let ours = stamp.spelling();
    Some(match (ours == text, slip) {
        (true, _) => Ok(stamp),
        (false, false) => Err(Refused::Respelt(ours)),
        (false, true) => Err(Refused::Misspelt(ours)),
    })
}

/// The letters `text` opens with, and what follows them and any space.
fn word_and_rest(text: &str) -> (&str, &str) {
    let letters = text.bytes().take_while(u8::is_ascii_lowercase).count();
    (&text[..letters], text[letters..].trim_start())
}

fn day_named(name: &str) -> Option<Day> {
    let named = DAYS.iter().find(|(known, _)| *known == name);
    named.map(|&(_, day)| day).or_else(|| named_weekday(name))
}

/// The weekday `word` names, in full or by how its name opens.
fn named_weekday(word: &str) -> Option<Day> {
    let at = WEEKDAYS.iter().position(|name| {
        (word.len() >= SHORTEST && name.starts_with(word))
            || (word == "weds" && *name == "wednesday")
    })?;
    Some(Day::Weekday(u8::try_from(at).ok()?))
}

/// What follows a stamp's day, read as a time however it is written:
/// `09:00`, `9:00`, `9.00`, `9am`, `at 9 pm`, `0900`, `9h`. None where it
/// isn't a time at all, and the value is the author's own text.
fn clock(written: &str) -> Option<Clock> {
    let mut rest = written;
    for lead in ["at ", "@", "t"] {
        if let Some(after) = rest.strip_prefix(lead) {
            rest = after.trim_start();
            break;
        }
    }
    let digits = |text: &str| text.bytes().take_while(u8::is_ascii_digit).count();
    let figures = digits(rest);
    let (first, mut rest) = rest.split_at(figures);
    let mut minute = None;
    let mut marked = false;
    if let Some(after) = rest.strip_prefix([':', '.', 'h']) {
        match digits(after) {
            2 => (minute, rest) = (Some(&after[..2]), &after[2..]),
            0 if rest.starts_with('h') => rest = after,
            _ => return None,
        }
        marked = true;
        // Seconds, which no stamp keeps.
        if let Some(seconds) = rest.strip_prefix(':').filter(|_| minute.is_some()) {
            match digits(seconds) {
                2 => rest = &seconds[2..],
                _ => return None,
            }
        }
    }
    let afternoon = match rest.trim_start() {
        "" => None,
        "am" | "a.m." => Some(false),
        "pm" | "p.m." => Some(true),
        _ => return None,
    };
    let (hour, minute) = match (figures, minute) {
        (1 | 2, Some(minute)) => (first, minute),
        (1 | 2, None) => (first, "00"),
        (3 | 4, None) => first.split_at(figures - 2),
        _ => return None,
    };
    if !marked && afternoon.is_none() {
        return Some(Clock::Unclear);
    }
    let (mut hour, minute): (u8, u8) = (hour.parse().ok()?, minute.parse().ok()?);
    if let Some(afternoon) = afternoon {
        if hour == 0 || hour > 12 {
            return Some(Clock::NoSuch);
        }
        hour = hour % 12 + if afternoon { 12 } else { 0 };
    }
    Some(match hour > 23 || minute > 59 {
        true => Clock::NoSuch,
        false => Clock::At(hour, minute),
    })
}

/// The problem with each value in a file's front matter that is a stamp,
/// or was meant as one, placed at the value. A value repeated by an alias
/// is told once, where it is written. A value written as a block is the
/// author's text whatever it says.
pub fn problems(front: &Node) -> Vec<Problem> {
    let mut found = Vec::new();
    walk(front, "", &mut found);
    let mut placed = HashSet::new();
    found.retain(|problem: &Problem| placed.insert(problem.place));
    found
}

fn walk(node: &Node, field: &str, found: &mut Vec<Problem>) {
    match &node.value {
        Value::Mapping(entries) => {
            for entry in entries {
                let name = key_name(&entry.key);
                let field = match field {
                    "" => name,
                    field => format!("{field}.{name}"),
                };
                walk(&entry.value, &field, found);
            }
        }
        Value::Sequence(items) => {
            for item in items {
                walk(item, field, found);
            }
        }
        Value::Scalar(text) => {
            let written = matches!(
                node.style,
                Style::Plain | Style::SingleQuoted | Style::DoubleQuoted
            );
            if let Some(read) = read(text).filter(|_| written && !field.is_empty()) {
                let kind = match read {
                    Ok(_) => Kind::NotStamped {
                        field: field.to_owned(),
                        written: text.clone(),
                    },
                    Err(refused) => refusal(text, refused),
                };
                found.push(Problem::at(kind, node.place));
            }
        }
    }
}

fn refusal(written: &str, refused: Refused) -> Kind {
    let written = written.to_owned();
    let word = Word::Stamp;
    match refused {
        Refused::Respelt(ours) => Kind::OtherSpelling {
            word,
            written,
            ours,
        },
        Refused::Misspelt(nearest) => Kind::UnknownWord {
            word,
            written,
            nearest: Some(nearest),
        },
        Refused::UnclearTime => Kind::UnknownWord {
            word,
            written,
            nearest: None,
        },
        Refused::NoSuchTime(time) => Kind::NoSuchDate {
            written,
            missing: Missing::Time(time),
        },
    }
}

/// Whether a problem is one [`problems`] tells of a value meant as a stamp
/// and not written as one.
fn of_a_refused_stamp(kind: &Kind) -> bool {
    match kind {
        Kind::OtherSpelling { word, .. } | Kind::UnknownWord { word, .. } => *word == Word::Stamp,
        Kind::NoSuchDate { written, .. } => written.starts_with('='),
        _ => false,
    }
}

/// One stamp written.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Stamped {
    /// The file, from the site's folder, with forward slashes.
    pub file: String,
    pub field: String,
    /// The date written in the stamp's place.
    pub written: String,
}

/// What became of the stamps a site read was found to hold, and of the
/// ids its feeds' posts were due.
#[derive(Default)]
pub struct Pass {
    pub stamped: Vec<Stamped>,
    /// The files given an `id:` line, each as [`Stamped`] names it.
    pub ids: Vec<String>,
    /// Each file written, as [`Stamped`] names it, and what it now holds.
    pub files: Vec<(String, blake3::Hash)>,
    /// The files that weren't written, each as a problem in no file
    /// saying why.
    pub refused: Vec<Problem>,
    /// Whether stamps were left as they were because the site's settings
    /// have an error: which day it is there can't be known.
    pub unsettled: bool,
    /// Whether posts were left without an `id:` because no build has yet
    /// kept what feed readers know.
    pub unrecorded: bool,
}

impl Pass {
    /// Whether any file was written.
    pub fn wrote(&self) -> bool {
        !self.files.is_empty()
    }
}

/// What is written into one file.
#[derive(Default)]
struct Due<'a> {
    stamps: Vec<(Span, &'a str, Stamp)>,
    /// The `id:` line, as the edit that writes it.
    id: Option<Edit>,
}

/// Which of the two things written into a site's files.
#[derive(Copy, Clone, PartialEq, Eq)]
enum Part {
    Stamps,
    Ids,
}

/// Writes into the files of the site at `site`, which `prepared` read as
/// `run` asks, a date over every stamp its problems tell of, and then an
/// `id:` line for each post `write_feed_ids` asks one for: a post is
/// known for one only once its dates are written. `fixed` is whether the
/// day was given, so that the run can be repeated. Gives the site as it
/// reads once written, the texts of its pages taken from `texts` as
/// [`crate::content::read_with`] does.
pub(crate) fn write(
    site: &Path,
    run: &crate::content::Run,
    mut prepared: build::Prepared,
    fixed: bool,
    texts: &mut crate::content::Texts,
) -> (Pass, build::Prepared) {
    let mut pass = written(site, &prepared, fixed, Part::Stamps);
    if pass.wrote() {
        prepared = build::prepared_with(site, run, texts);
    }
    let ids = written(site, &prepared, fixed, Part::Ids);
    if ids.wrote() {
        prepared = build::prepared_with(site, run, texts);
    }
    pass.ids = ids.ids;
    pass.files.extend(ids.files);
    pass.refused.extend(ids.refused);
    pass.unrecorded = ids.unrecorded;
    (pass, prepared)
}

fn written(site: &Path, prepared: &build::Prepared, fixed: bool, part: Part) -> Pass {
    let renderer = &prepared.renderer;
    let (sources, settings) = (renderer.sources(), renderer.settings().source);
    let problems = &prepared.problems;
    let mut due: BTreeMap<SourceId, Due> = BTreeMap::new();
    for problem in problems.iter().filter(|_| part == Part::Stamps) {
        if let (Kind::NotStamped { field, written }, Some(place)) = (&problem.kind, problem.place)
            && let Some(Ok(stamp)) = read(written)
        {
            let stamps = &mut due.entry(place.source).or_default().stamps;
            stamps.push((place.span, field, stamp));
        }
    }
    let mut pass = Pass::default();
    if settings_stop(sources, settings, problems).next().is_some() {
        pass.unsettled = !due.is_empty();
        return pass;
    }
    let pages = &renderer.site().pages;
    let ids = match part {
        Part::Stamps => IdsToWrite::Lines(Vec::new()),
        Part::Ids => feeds::ids_to_write(
            &prepared.known,
            renderer.site(),
            renderer.settings(),
            renderer.dates(),
        ),
    };
    match ids {
        IdsToWrite::NoRecord => pass.unrecorded = true,
        IdsToWrite::Lines(lines) => {
            for (post, line) in lines {
                let Some(source) = pages.get(post).source else {
                    continue;
                };
                match id_line(sources, source, &line) {
                    Some(edit) => due.entry(source).or_default().id = Some(edit),
                    None => pass.refused.push(Problem::nowhere(Kind::CantWrite {
                        path: sources.path(source).to_path_buf(),
                        reason: crate::problem::Reason::NoPlaceForId,
                    })),
                }
            }
        }
    }
    if due.is_empty() {
        return pass;
    }
    let moment = Moment::of(renderer.dates(), fixed);
    let content = site.join("content");
    for (source, mut due) in due {
        due.stamps.sort_by_key(|(span, ..)| span.start);
        let path = sources.path(source);
        let Ok(file) = platform::slashed(path) else {
            continue;
        };
        let Some(rel) = file.strip_prefix("content/") else {
            continue;
        };
        let mut edits = Vec::new();
        let mut stamped = Vec::new();
        for (span, field, stamp) in due.stamps {
            let Some(written) = stamp.written(&moment) else {
                continue;
            };
            edits.push(Edit {
                span,
                with: written.clone(),
            });
            stamped.push(Stamped {
                file: file.clone(),
                field: field.to_owned(),
                written,
            });
        }
        let id = due.id.is_some();
        edits.extend(due.id);
        if edits.is_empty() {
            continue;
        }
        match rewrite::rewrite(&content, rel, sources.text(source), &edits) {
            Ok(text) => {
                pass.files.push((file.clone(), blake3::hash(&text)));
                pass.stamped.extend(stamped);
                if id {
                    pass.ids.push(file);
                }
            }
            Err(reason) => pass.refused.push(Problem::nowhere(Kind::CantWrite {
                path: path.to_path_buf(),
                reason,
            })),
        }
    }
    pass
}

/// The edit that writes `line` as the last line of a file's front matter,
/// with the line ending and the indent the lines there have, giving the
/// file front matter where it has none. None where the line wouldn't be
/// read back as a field of the page: front matter never closed, or
/// written some other way than a line to a field.
fn id_line(sources: &Sources, source: SourceId, line: &str) -> Option<Edit> {
    let text = sources.text(source);
    let split = crate::yaml::split(text).ok()?;
    let ending = match text.find('\n') {
        Some(at) if text[..at].ends_with('\r') => "\r\n",
        _ => "\n",
    };
    let (at, with) = match split.front_matter {
        Some(front) => {
            let indent = match crate::yaml::front_matter(sources, source) {
                Ok((Some(node), _)) => match &node.value {
                    Value::Mapping(entries) => entries.first().map(|first| {
                        let key = first.key.place.span.range().start;
                        let from = text[..key].rfind('\n').map_or(0, |end| end + 1);
                        &text[from..key]
                    }),
                    _ => None,
                },
                _ => None,
            };
            let indent = indent.unwrap_or_default();
            (front.range().end, format!("{indent}{line}{ending}"))
        }
        None => (
            split.body.range().start,
            format!("---{ending}{line}{ending}---{ending}"),
        ),
    };
    // Read back as a build will read it.
    let mut after = Sources::default();
    let written = format!("{}{with}{}", &text[..at], &text[at..]);
    let read = after.add(sources.path(source), written);
    let (field, id) = line.split_once(": ")?;
    let lands = match crate::yaml::front_matter(&after, read) {
        Ok((Some(node), _)) => match &node.value {
            Value::Mapping(entries) => entries.iter().any(|entry| {
                key_name(&entry.key) == field
                    && matches!(&entry.value.value, Value::Scalar(value) if value == id)
            }),
            _ => false,
        },
        _ => false,
    };
    lands.then(|| Edit {
        span: Span::new(at, at),
        with,
    })
}

/// The errors in a site's settings file, or in reading it.
fn settings_stop<'a>(
    sources: &'a Sources,
    settings: Option<SourceId>,
    problems: &'a [Problem],
) -> impl Iterator<Item = &'a Problem> {
    let file = Path::new(crate::config::FILE_NAME);
    problems
        .iter()
        .filter(|problem| problem.kind.severity() == Severity::Error)
        .filter(move |problem| match (problem.place, &problem.kind) {
            (Some(place), _) => {
                Some(place.source) == settings || sources.path(place.source) == file
            }
            (None, Kind::CantRead { path, .. }) => path == file,
            _ => false,
        })
}

/// What `statac stamp` is asked.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Options {
    /// The site's folder.
    pub site: PathBuf,
    /// The day stamps are counted from; today in the site's zone where
    /// none is given.
    pub today: Option<Date>,
}

/// What `statac stamp` did, and what it has to tell.
pub struct Outcome {
    held: Option<build::Prepared>,
    pub pass: Pass,
    /// What stopped it: a folder that can't be read, the errors in the
    /// site's settings, each file that couldn't be written, and each
    /// value meant as a stamp and not written as one.
    pub problems: Vec<Problem>,
}

impl Outcome {
    /// The files the problems are placed in.
    pub fn sources(&self) -> Option<&Sources> {
        let held = self.held.as_ref()?;
        Some(held.renderer.sources())
    }
}

/// Reads the site as a build does and writes a date over every stamp in
/// it.
pub fn run(options: &Options) -> Outcome {
    if let Err(problem) = build::a_folder(&options.site) {
        return Outcome {
            held: None,
            pass: Pass::default(),
            problems: vec![problem],
        };
    }
    let run = crate::content::Run {
        today: options.today,
        drafts: false,
        url: None,
    };
    let prepared = build::prepared(&options.site, &run);
    let fixed = options.today.is_some();
    let unkept = &mut crate::content::Texts::default();
    let (mut pass, mut prepared) = write(&options.site, &run, prepared, fixed, unkept);
    // Of the site as it reads now, which the places are in.
    let read = std::mem::take(&mut prepared.problems);
    let renderer = &prepared.renderer;
    let (sources, settings) = (renderer.sources(), renderer.settings().source);
    let mut problems = std::mem::take(&mut pass.refused);
    if pass.unsettled {
        let stop: Vec<Problem> = settings_stop(sources, settings, &read).cloned().collect();
        problems.extend(stop);
    }
    let refused = read.into_iter();
    problems.extend(refused.filter(|problem| of_a_refused_stamp(&problem.kind)));
    Outcome {
        held: Some(prepared),
        pass,
        problems,
    }
}

#[cfg(test)]
mod tests;
