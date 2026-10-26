//! Save-time stamps: a front matter value that is, whole, one of Statac's
//! words for a date, such as `=today` or `=monday 09:00`, and the date
//! written in its place.
//!
//! A build never picks the date: it would differ from one build to the
//! next. `statac stamp` and `statac serve` write it into the file, once,
//! and a build meeting a stamp still unwritten stops at it.

use std::collections::HashSet;

use jiff::civil;

use crate::date::{Date, Time};
use crate::dates::{Dates, ShowDates};
use crate::origin::Style;
use crate::problem::{Kind, Missing, Problem, Word};
use crate::suggest;
use crate::yaml::key_name;
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
pub(crate) enum Day {
    Today,
    /// The present date and time.
    Now,
    Tomorrow,
    Yesterday,
    /// The next such day after today, Monday being 0.
    Weekday(u8),
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) struct Stamp {
    pub(crate) day: Day,
    /// The hour and minute written after the day.
    pub(crate) time: Option<(u8, u8)>,
}

/// Why a value meant as a stamp isn't one.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Refused {
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
    pub(crate) fn spelling(&self) -> String {
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
    pub(crate) fn written(&self, moment: &Moment) -> Option<String> {
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
pub(crate) struct Moment {
    pub(crate) today: Date,
    pub(crate) now: Date,
}

impl Moment {
    /// The moment on the site's clock. Where the day is `fixed`, so that
    /// a run can be repeated, the present minute is its midnight.
    pub(crate) fn of(dates: &Dates, fixed: bool) -> Moment {
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
pub(crate) fn read(text: &str) -> Option<Result<Stamp, Refused>> {
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
pub(crate) fn problems(front: &Node) -> Vec<Problem> {
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
pub(crate) fn of_a_refused_stamp(kind: &Kind) -> bool {
    match kind {
        Kind::OtherSpelling { word, .. } | Kind::UnknownWord { word, .. } => *word == Word::Stamp,
        Kind::NoSuchDate { written, .. } => written.starts_with('='),
        _ => false,
    }
}

#[cfg(test)]
mod tests;
