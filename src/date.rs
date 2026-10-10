//! Dates as files write them: ISO 8601, with a time and an offset if wanted.
//! Any other spelling is refused with the ISO form offered where there is
//! one.

use std::fmt;

/// A date as written: the calendar day the author wrote, and the time and
/// offset if they wrote them. Nothing is converted to another zone here.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Date {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub time: Option<Time>,
    /// Minutes east of UTC.
    pub offset: Option<i16>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Time {
    pub hour: u8,
    pub minute: u8,
    pub second: Option<u8>,
}

/// As much of a date as the author knows.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum When {
    Year(i32),
    Month(i32, u8),
    Date(Date),
}

/// Why a value isn't a date.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Fault {
    /// In no shape Statac takes, with the ISO spelling where the meaning is
    /// clear.
    NotIso {
        iso: Option<String>,
    },
    /// Two readings, each as ISO and in words.
    Unclear {
        readings: [(String, String); 2],
    },
    NoSuchDay {
        day: u8,
        month: u8,
        year: i32,
    },
    NoSuchMonth(u8),
    NoSuchTime(String),
    NoSuchOffset(String),
}

impl Date {
    pub fn day_only(year: i32, month: u8, day: u8) -> Self {
        Date {
            year,
            month,
            day,
            time: None,
            offset: None,
        }
    }

    /// The day alone, as `2026-10-04`.
    pub fn iso_day(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    pub fn same_day(&self, other: &Date) -> bool {
        (self.year, self.month, self.day) == (other.year, other.month, other.day)
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.iso_day())?;
        if let Some(time) = self.time {
            write!(f, "T{:02}:{:02}", time.hour, time.minute)?;
            if let Some(second) = time.second {
                write!(f, ":{second:02}")?;
            }
        }
        match self.offset {
            Some(0) => f.write_str("Z"),
            Some(minutes) => {
                let sign = if minutes < 0 { '-' } else { '+' };
                let minutes = minutes.unsigned_abs();
                write!(f, "{sign}{:02}:{:02}", minutes / 60, minutes % 60)
            }
            None => Ok(()),
        }
    }
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn checked(year: i32, month: u8, day: u8) -> Result<Date, Fault> {
    if !(1..=12).contains(&month) {
        return Err(Fault::NoSuchMonth(month));
    }
    if day == 0 || day > days_in_month(year, month) {
        return Err(Fault::NoSuchDay { day, month, year });
    }
    Ok(Date::day_only(year, month, day))
}

/// Reads a date written as ISO 8601: `2026-10-04`, then optionally `T` or a
/// space and `HH:MM`, `:SS` and a fraction of a second, then optionally an
/// offset: `Z`, `+01:00`, `+0100` or `+01`, after a space if wanted.
pub fn read(text: &str) -> Result<Date, Fault> {
    let text = text.trim();
    let Some((year, month, day, rest)) = iso_day(text) else {
        return Err(not_iso(text));
    };
    let mut date = checked(year, month, day)?;
    if rest.is_empty() {
        return Ok(date);
    }
    let rest = rest
        .strip_prefix(['T', 't', ' '])
        .ok_or(Fault::NotIso { iso: None })?;
    let clock_text = rest;
    let (time, rest) = clock(rest).ok_or(Fault::NotIso { iso: None })?;
    if time.hour > 23 || time.minute > 59 || time.second.is_some_and(|s| s > 59) {
        let written = &clock_text[..clock_text.len() - rest.len()];
        return Err(Fault::NoSuchTime(written.to_owned()));
    }
    date.time = Some(time);
    let rest = rest.trim_start();
    if !rest.is_empty() {
        let minutes = offset(rest).ok_or(Fault::NotIso { iso: None })?;
        if minutes.abs() > 14 * 60 {
            return Err(Fault::NoSuchOffset(rest.to_owned()));
        }
        date.offset = Some(minutes);
    }
    Ok(date)
}

/// Reads a date whose day is written in English words, `4 October 2026` or
/// `Oct 4, 2026`, then optionally a time and an offset as [`read`] takes
/// them. Nothing with two readings is read.
pub fn read_words(text: &str) -> Option<Date> {
    let text = text.trim();
    let mut words = 0;
    let mut within = false;
    let mut end = text.len();
    for (at, c) in text.char_indices() {
        let parts = c.is_whitespace() || c == ',';
        if within && parts && words == 3 {
            end = at;
            break;
        }
        if !within && !parts {
            words += 1;
        }
        within = !parts;
    }
    let day = in_words(&text[..end])?;
    let rest = text[end..].trim_start_matches(',').trim_start();
    match rest.is_empty() {
        true => read(&day).ok(),
        false => read(&format!("{day} {rest}")).ok(),
    }
}

/// Reads a year, a year and month, or a whole date.
pub fn read_when(text: &str) -> Result<When, Fault> {
    let text = text.trim();
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if text.len() == 4 && digits(text) {
        return Ok(When::Year(text.parse().expect("four digits")));
    }
    if text.len() == 7 && digits(&text[..4]) && &text[4..5] == "-" && digits(&text[5..]) {
        let month: u8 = text[5..].parse().expect("two digits");
        if !(1..=12).contains(&month) {
            return Err(Fault::NoSuchMonth(month));
        }
        return Ok(When::Month(text[..4].parse().expect("four digits"), month));
    }
    read(text).map(When::Date)
}

/// `YYYY-MM-DD` at the start of `text`, and what follows it.
fn iso_day(text: &str) -> Option<(i32, u8, u8, &str)> {
    let b = text.as_bytes();
    if b.len() < 10 {
        return None;
    }
    let digits = |range: std::ops::Range<usize>| range.into_iter().all(|i| b[i].is_ascii_digit());
    if !digits(0..4) || b[4] != b'-' || !digits(5..7) || b[7] != b'-' || !digits(8..10) {
        return None;
    }
    Some((
        text[..4].parse().ok()?,
        text[5..7].parse().ok()?,
        text[8..10].parse().ok()?,
        &text[10..],
    ))
}

/// A date at the start of a file name, `2026-10-04-post-1`, and the name
/// after it and its hyphen. Only a real day counts.
pub fn in_name(name: &str) -> Option<(Date, &str)> {
    let (year, month, day, rest) = iso_day(name)?;
    let rest = match rest {
        "" => "",
        rest => rest.strip_prefix('-')?,
    };
    checked(year, month, day).ok().map(|date| (date, rest))
}

/// `HH:MM`, optionally `:SS` and a fraction, and what follows.
fn clock(text: &str) -> Option<(Time, &str)> {
    let b = text.as_bytes();
    let two = |at: usize| -> Option<u8> {
        let pair = text.get(at..at + 2)?;
        pair.bytes()
            .all(|b| b.is_ascii_digit())
            .then(|| pair.parse().ok())?
    };
    let hour = two(0)?;
    if b.get(2) != Some(&b':') {
        return None;
    }
    let minute = two(3)?;
    let mut rest = &text[5..];
    let mut second = None;
    if rest.starts_with(':') {
        second = Some(two(6)?);
        rest = &text[8..];
        if let Some(fraction) = rest.strip_prefix(['.', ',']) {
            let n = fraction.bytes().take_while(u8::is_ascii_digit).count();
            if n == 0 {
                return None;
            }
            rest = &fraction[n..];
        }
    }
    Some((
        Time {
            hour,
            minute,
            second,
        },
        rest,
    ))
}

/// `Z`, `+01:00`, `+0100` or `+01`, in minutes east of UTC.
fn offset(text: &str) -> Option<i16> {
    if text.eq_ignore_ascii_case("z") {
        return Some(0);
    }
    let sign: i16 = match text.as_bytes().first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let digits: String = text[1..].chars().filter(|&c| c != ':').collect();
    if !digits.bytes().all(|b| b.is_ascii_digit()) || text[1..].matches(':').count() > 1 {
        return None;
    }
    let (hours, minutes): (i16, i16) = match digits.len() {
        2 => (digits.parse().ok()?, 0),
        4 => (digits[..2].parse().ok()?, digits[2..].parse().ok()?),
        _ => return None,
    };
    (minutes <= 59).then_some(sign * (hours * 60 + minutes))
}

/// A date in another shape, read for the ISO form to offer.
fn not_iso(text: &str) -> Fault {
    if let Some(fault) = numeric(text) {
        return fault;
    }
    Fault::NotIso {
        iso: in_words(text),
    }
}

/// `3/4/2026`, `03.04.2026` or `2026/04/03`.
fn numeric(text: &str) -> Option<Fault> {
    let separator = text.chars().find(|c| matches!(c, '/' | '.'))?;
    let parts: Vec<&str> = text.split(separator).collect();
    let [a, b, c] = parts.as_slice() else {
        return None;
    };
    if ![a, b, c]
        .iter()
        .all(|p| !p.is_empty() && p.len() <= 4 && p.bytes().all(|b| b.is_ascii_digit()))
    {
        return None;
    }
    let number = |p: &str| p.parse::<u32>().ok();
    if a.len() == 4 {
        let (year, month, day) = (number(a)? as i32, number(b)?, number(c)?);
        let date = checked(year, u8::try_from(month).ok()?, u8::try_from(day).ok()?).ok()?;
        return Some(Fault::NotIso {
            iso: Some(date.iso_day()),
        });
    }
    if c.len() != 4 {
        return None;
    }
    let year = number(c)? as i32;
    let (a, b) = (
        u8::try_from(number(a)?).ok()?,
        u8::try_from(number(b)?).ok()?,
    );
    let day_first = checked(year, b, a).ok();
    let month_first = checked(year, a, b).ok();
    let words = |date: &Date| format!("{} {}", date.day, MONTHS[usize::from(date.month) - 1]);
    Some(match (day_first, month_first) {
        (Some(one), Some(other)) if one != other => Fault::Unclear {
            readings: [
                (one.iso_day(), words(&one)),
                (other.iso_day(), words(&other)),
            ],
        },
        (Some(date), _) | (None, Some(date)) => Fault::NotIso {
            iso: Some(date.iso_day()),
        },
        (None, None) => Fault::NotIso { iso: None },
    })
}

/// `4 October 2026`, `October 4, 2026`, `4 Oct 2026` and so on.
fn in_words(text: &str) -> Option<String> {
    let words: Vec<&str> = text
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|w| !w.is_empty())
        .collect();
    let month_of = |word: &str| {
        let word = word.trim_end_matches('.').to_lowercase();
        MONTHS.iter().position(|m| {
            let m = m.to_lowercase();
            word == m || (word.len() >= 3 && m.starts_with(&word))
        })
    };
    let (day, month, year) = match words.as_slice() {
        [d, m, y] if month_of(m).is_some() => (*d, month_of(m)?, *y),
        [m, d, y] if month_of(m).is_some() => (*d, month_of(m)?, *y),
        _ => return None,
    };
    let day: u8 = day
        .trim_end_matches(|c: char| c.is_ascii_alphabetic())
        .parse()
        .ok()?;
    if year.len() != 4 {
        return None;
    }
    let date = checked(year.parse().ok()?, month as u8 + 1, day).ok()?;
    Some(date.iso_day())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn time(hour: u8, minute: u8, second: Option<u8>) -> Option<Time> {
        Some(Time {
            hour,
            minute,
            second,
        })
    }

    #[test]
    fn every_iso_form_the_sites_write_is_read() {
        let cases = [
            ("2026-10-04", None, None),
            ("2013-03-22T23:53:31+00:00", time(23, 53, Some(31)), Some(0)),
            ("2013-03-22 23:53:31", time(23, 53, Some(31)), None),
            ("2013-03-22T23:53:31Z", time(23, 53, Some(31)), Some(0)),
            (
                "2013-03-22T23:53:31-05:00",
                time(23, 53, Some(31)),
                Some(-300),
            ),
            ("2013-03-22 23:53:31Z", time(23, 53, Some(31)), Some(0)),
            ("2013-03-22 23:53", time(23, 53, None), None),
            (
                "2013-03-22 23:53:31 +0100",
                time(23, 53, Some(31)),
                Some(60),
            ),
            ("2013-03-22T23:53:31.123Z", time(23, 53, Some(31)), Some(0)),
            ("2013-03-22T23:53+05", time(23, 53, None), Some(300)),
            ("  2026-10-04  ", None, None),
        ];
        for (text, time, offset) in cases {
            let date = read(text).unwrap_or_else(|e| panic!("{text}: {e:?}"));
            assert_eq!(date.iso_day(), text.trim()[..10], "{text}");
            assert_eq!((date.time, date.offset), (time, offset), "{text}");
        }
    }

    #[test]
    fn a_date_is_written_back_in_iso_form() {
        assert_eq!(
            read("2013-03-22 23:53:31 +0100").unwrap().to_string(),
            "2013-03-22T23:53:31+01:00"
        );
        assert_eq!(read("2026-10-04").unwrap().to_string(), "2026-10-04");
    }

    #[test]
    fn impossible_dates_say_what_is_missing() {
        let cases = [
            (
                "2026-02-30",
                Fault::NoSuchDay {
                    day: 30,
                    month: 2,
                    year: 2026,
                },
            ),
            (
                "2026-02-29",
                Fault::NoSuchDay {
                    day: 29,
                    month: 2,
                    year: 2026,
                },
            ),
            ("2026-13-01", Fault::NoSuchMonth(13)),
            ("2026-00-01", Fault::NoSuchMonth(0)),
            ("2026-10-04T25:70", Fault::NoSuchTime("25:70".into())),
            ("2026-10-04 23:00:61", Fault::NoSuchTime("23:00:61".into())),
            (
                "2026-10-04T10:00+15:00",
                Fault::NoSuchOffset("+15:00".into()),
            ),
        ];
        for (text, fault) in cases {
            assert_eq!(read(text), Err(fault), "{text}");
        }
        assert!(read("2024-02-29").is_ok());
        assert!(read("2000-02-29").is_ok());
        assert!(read("1900-02-29").is_err());
    }

    #[test]
    fn other_spellings_offer_the_iso_form() {
        let unclear = |a: (&str, &str), b: (&str, &str)| Fault::Unclear {
            readings: [(a.0.into(), a.1.into()), (b.0.into(), b.1.into())],
        };
        let iso = |s: &str| Fault::NotIso {
            iso: Some(s.into()),
        };
        let cases = [
            (
                "3/4/2026",
                unclear(("2026-04-03", "3 April"), ("2026-03-04", "4 March")),
            ),
            ("13/04/2026", iso("2026-04-13")),
            ("04/13/2026", iso("2026-04-13")),
            ("4/4/2026", iso("2026-04-04")),
            ("2026/04/13", iso("2026-04-13")),
            (
                "03.04.2026",
                unclear(("2026-04-03", "3 April"), ("2026-03-04", "4 March")),
            ),
            ("4 October 2026", iso("2026-10-04")),
            ("October 4, 2026", iso("2026-10-04")),
            ("Oct 4 2026", iso("2026-10-04")),
            ("4th Oct. 2026", iso("2026-10-04")),
            ("soon", Fault::NotIso { iso: None }),
            ("2026", Fault::NotIso { iso: None }),
            ("2026-10-04x", Fault::NotIso { iso: None }),
            ("2026-10-4", Fault::NotIso { iso: None }),
            ("40/40/2026", Fault::NotIso { iso: None }),
        ];
        for (text, fault) in cases {
            assert_eq!(read(text), Err(fault), "{text}");
        }
    }

    #[test]
    fn a_day_in_words_is_read_with_its_time_and_offset() {
        let cases = [
            ("4 October 2026", Some("2026-10-04")),
            ("4 Oct 2026", Some("2026-10-04")),
            ("October 4, 2026", Some("2026-10-04")),
            ("oct 4 2026", Some("2026-10-04")),
            ("4 Oct 2026 09:30", Some("2026-10-04T09:30")),
            (
                "4 Oct 2026 09:30:15 +0100",
                Some("2026-10-04T09:30:15+01:00"),
            ),
            ("October 4, 2026 09:30 Z", Some("2026-10-04T09:30Z")),
            ("  4 Oct 2026  ", Some("2026-10-04")),
            ("31 Feb 2026", None),
            ("4 Oct 26", None),
            ("4 Oct 2026 25:00", None),
            ("4 Oct 2026 soon", None),
            ("4 Oct", None),
            ("04/10/2026", None),
            ("13/10/2026", None),
            ("2026-10-04", None),
            ("soon", None),
        ];
        for (text, iso) in cases {
            let read = read_words(text).map(|date| date.to_string());
            assert_eq!(read.as_deref(), iso, "{text}");
        }
    }

    #[test]
    fn as_much_of_a_date_as_is_known() {
        assert_eq!(read_when("2026"), Ok(When::Year(2026)));
        assert_eq!(read_when("2026-10"), Ok(When::Month(2026, 10)));
        assert_eq!(read_when("2026-13"), Err(Fault::NoSuchMonth(13)));
        assert_eq!(
            read_when("2026-10-04"),
            Ok(When::Date(Date::day_only(2026, 10, 4)))
        );
    }

    #[test]
    fn a_file_name_opening_with_a_real_day_is_dated() {
        let cases = [
            ("2024-01-02-post-1", Some(("2024-01-02", "post-1"))),
            ("2024-01-02", Some(("2024-01-02", ""))),
            ("2024-01-02post", None),
            ("2024-02-30-post", None),
            ("post-1", None),
            ("2024-1-2-post", None),
        ];
        for (name, dated) in cases {
            let found = in_name(name).map(|(date, rest)| (date.iso_day(), rest));
            assert_eq!(found, dated.map(|(d, r)| (d.to_owned(), r)), "{name}");
        }
    }
}
