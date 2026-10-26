//! Dates as a site shows them: on its pages in its `date_format`, in each
//! page's language, and in its feeds as moments.
//!
//! A date written with an offset keeps it, and shows the day and time it
//! was written for. A date written without one is read in the site's
//! `timezone`, so the moment it names is the one the author's clock showed.

pub(crate) mod format;
#[cfg(test)]
mod tests;

use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

use jiff::tz::TimeZone;

use crate::config::Settings;
use crate::date::{Date, Time, When};
use crate::ids::{Id, LanguageId};
use crate::language::{Language, Languages, Whence};
use crate::origin::Sourced;
use crate::problem::{FormatFault, Kind, Look, Problem, Word, WordsFrom};
use crate::source::{Place, Sources, Span};
use crate::value::place_within;

pub(crate) use format::{DateWords, Format, Reading, Refusal};

/// What templates ask of a site's dates.
pub trait ShowDates {
    /// A date as the site writes it in `language`, or as a template's
    /// `example` of the reference moment, or the look it names, does.
    fn format(
        &self,
        language: LanguageId,
        date: &Date,
        example: Option<&str>,
    ) -> Result<String, Refusal>;

    /// The moment a date names, as Atom writes one:
    /// `2025-12-31T15:04:00+01:00`.
    fn moment(&self, date: &Date) -> String;

    /// The same moment as RSS writes one: `Wed, 31 Dec 2025 15:04:00 +0100`.
    fn moment_rss(&self, date: &Date) -> String;

    /// A date, a month or a year, as much as the author knows, in the
    /// site's style in `language`.
    fn when(&self, language: LanguageId, when: &When) -> String;

    /// The day a date falls on, in the words of `language`.
    fn weekday(&self, language: LanguageId, date: &Date) -> Option<&str>;

    /// The build's date, `site.today`.
    fn today(&self) -> Date;

    /// Two dates in the order of the moments they name.
    fn order(&self, a: &Date, b: &Date) -> Ordering;

    /// The newest of `dates`; of two naming one moment, the later given.
    fn newest<'a>(&self, dates: &[&'a Date]) -> Option<&'a Date> {
        dates
            .iter()
            .copied()
            .reduce(|newest, date| match self.order(date, newest) {
                Ordering::Less => newest,
                _ => date,
            })
    }
}

/// A site's dates: each language's words and format, its zone and its
/// today.
pub struct Dates {
    /// Each language the site declares, main first, by its id.
    languages: Vec<InLanguage>,
    zone: Option<TimeZone>,
    today: Date,
}

/// How dates are written in one of a site's languages.
struct InLanguage {
    language: Language,
    words: DateWords,
    format: Format,
    /// The other language whose words read the site's `date_format`, where
    /// this one's couldn't.
    format_read_in: Option<LanguageId>,
    /// Each example a template has asked for, read once.
    examples: RwLock<HashMap<String, Example>>,
}

/// An example of the reference moment as one language reads it, and the
/// other language whose words read it, where its own couldn't.
#[derive(Clone)]
struct Example {
    format: Result<Format, Refusal>,
    read_in: Option<LanguageId>,
}

impl Example {
    /// Reads `example` with `read` in a language's `own` words, or else in
    /// the first of `others` that reads it, to be written in its own. The
    /// languages whose words the example holds are tried first, so that a
    /// word of one is never taken for text by another.
    fn read(
        example: &str,
        own: &DateWords,
        others: &[(LanguageId, &DateWords)],
        read: fn(&str, &DateWords) -> Result<Format, Refusal>,
    ) -> Example {
        let mut tried: Vec<(Option<LanguageId>, &DateWords)> = std::iter::once((None, own))
            .chain(others.iter().map(|&(id, words)| (Some(id), words)))
            .collect();
        if tried.len() > 1 {
            tried.sort_by_key(|(_, words)| !written_in(example, &words.words));
        }
        let mut refused = Vec::with_capacity(tried.len());
        for (id, words) in tried {
            match (read(example, words), id) {
                (Ok(format), None) => {
                    return Example {
                        format: Ok(format),
                        read_in: None,
                    };
                }
                (Ok(format), Some(id)) => {
                    return Example {
                        format: format.in_words(own, example),
                        read_in: Some(id),
                    };
                }
                (Err(refusal), _) => refused.push((words, refusal)),
            }
        }
        Example {
            format: Err(to_tell(example, refused)),
            read_in: None,
        }
    }
}

/// The languages a language other than `language` looks to for an example
/// its own words can't read: the main language, then the rest as declared.
fn others<'a>(
    language: usize,
    words: impl Iterator<Item = &'a DateWords>,
) -> Vec<(LanguageId, &'a DateWords)> {
    words
        .enumerate()
        .filter(|(at, _)| *at != language)
        .map(|(at, words)| (LanguageId::from_index(at), words))
        .collect()
}

/// Which refusal to tell of an example no language reads, given each
/// language's in the order tried, its own first where the example holds
/// no language's words: the one in the words the example is written in,
/// where it holds or comes near one; else its own, offering figures rather
/// than words of a language the example isn't written in.
fn to_tell(example: &str, mut refused: Vec<(&DateWords, Refusal)>) -> Refusal {
    if refused.len() > 1
        && let Some(at) = refused.iter().position(|(words, refusal)| {
            written_in(example, &words.words) || near_a_word(&refusal.fault)
        })
    {
        return refused.swap_remove(at).1;
    }
    let several = refused.len() > 1;
    let (_, mut refusal) = refused.swap_remove(0);
    if several && example.chars().any(char::is_alphabetic) {
        let figures = |spellings: &mut Vec<String>| {
            spellings.retain(|spelling| !spelling.chars().any(char::is_alphabetic));
        };
        match &mut refusal.fault {
            FormatFault::Missing { months, .. } => figures(months),
            FormatFault::NoPattern { examples, .. } | FormatFault::Empty { examples, .. } => {
                figures(examples)
            }
            _ => {}
        }
    }
    refusal
}

/// Whether `example` holds a month's, a weekday's or the time of day's
/// word in `words`, whole.
fn written_in(example: &str, words: &crate::language::Words) -> bool {
    let example = example.to_lowercase();
    let apart = |c: char| c.is_whitespace() || c.is_ascii_punctuation() || c.is_ascii_digit();
    let pieces: Vec<&str> = example.split(apart).filter(|p| !p.is_empty()).collect();
    let lists = [
        words.months.as_ref().map(|list| &list[..]),
        words.months_alone.as_ref().map(|list| &list[..]),
        words.months_short.as_ref().map(|list| &list[..]),
        words.months_short_alone.as_ref().map(|list| &list[..]),
        words.weekdays.as_ref().map(|list| &list[..]),
        words.weekdays_short.as_ref().map(|list| &list[..]),
        words.am_pm.as_ref().map(|list| &list[..]),
    ];
    lists.into_iter().flatten().flatten().any(|word| {
        let word = word.trim_end_matches('.').to_lowercase();
        pieces.contains(&word.as_str())
    })
}

/// Whether a refusal comes near a word of the language it was read in,
/// offering that word.
fn near_a_word(fault: &FormatFault) -> bool {
    match fault {
        FormatFault::OtherMonth { .. } | FormatFault::OtherWeekday { .. } => true,
        FormatFault::NotInReference {
            written,
            corrected: Some(_),
            ..
        } => written.chars().any(char::is_alphabetic),
        _ => false,
    }
}

impl Dates {
    /// Reads the site's `language`, `timezone`, `date_format` and
    /// `date_zeros`, and each language's words from `languages/` or
    /// Statac's own. `site.today` is the current date in the site's zone
    /// until a build fixes it with [`Dates::with_today`].
    pub fn read(root: &Path, settings: &Settings, sources: &mut Sources) -> (Dates, Vec<Problem>) {
        let mut problems = Vec::new();
        let languages = Languages::find(root, &mut problems);
        let tags = settings.languages();
        let found: Vec<Language> = match tags.is_empty() {
            true => vec![Language::unnamed()],
            false => tags
                .into_iter()
                .map(|tag| languages.language(Some(tag), sources, &mut problems))
                .collect(),
        };
        let read: Vec<&str> = (found.iter())
            .filter_map(|language| match &language.whence {
                Whence::Site(path) => Some(path.as_str()),
                _ => None,
            })
            .collect();
        for path in languages
            .paths()
            .filter(|path| !read.contains(&path.as_str()))
        {
            let kind = Kind::UnusedLanguageFile {
                path,
                languages: found.iter().filter_map(|l| l.tag.clone()).collect(),
            };
            problems.push(Problem::nowhere(kind));
        }
        let zone = settings
            .text("timezone")
            .and_then(|name| match TimeZone::get(name.value) {
                Ok(zone) => Some(zone),
                Err(_) => {
                    let kind = Kind::UnknownWord {
                        word: Word::TimeZone,
                        written: name.value.to_owned(),
                        nearest: nearest_zone(name.value),
                    };
                    problems.extend(name.origin.place.map(|place| Problem::at(kind, place)));
                    None
                }
            });
        let zeros = settings.truth("date_zeros").unwrap_or(true);
        let words: Vec<DateWords> = found
            .iter()
            .map(|language| DateWords {
                words: Arc::clone(&language.words),
                from: match &language.whence {
                    Whence::Site(path) => WordsFrom::Site(path.clone()),
                    Whence::Statac(tag) => WordsFrom::Statac((*tag).to_owned()),
                    Whence::Nothing => WordsFrom::Nowhere(language.tag.clone().unwrap_or_default()),
                },
                zeros,
            })
            .collect();
        let mut told_of_the_plain = false;
        let mut languages = Vec::with_capacity(found.len());
        for (at, (language, own)) in found.into_iter().zip(&words).enumerate() {
            let for_language = language
                .tag
                .as_deref()
                .and_then(|tag| settings.for_language(tag, "date_format"))
                .and_then(|value| Some(Sourced::new(value.value.as_text()?, value.origin.clone())));
            // A line for the language alone is read in its words; the line
            // for every language may be read in another's, and is told
            // once where no language reads it.
            let (written, others, plain) = match for_language {
                Some(written) => (Some(written), Vec::new(), false),
                None => (settings.text("date_format"), others(at, words.iter()), true),
            };
            let read = match &written {
                Some(written) => Example::read(written.value, own, &others, Format::for_the_site),
                None => Example {
                    format: Ok(Format::house(own)),
                    read_in: None,
                },
            };
            let format = match read.format {
                Ok(format) => format,
                Err(refusal) => {
                    if let Some(written) = &written
                        && !(plain && told_of_the_plain)
                    {
                        problems.extend(refused(sources, written, refusal));
                    }
                    told_of_the_plain |= plain;
                    Format::house(own)
                }
            };
            languages.push(InLanguage {
                language,
                words: own.clone(),
                format,
                format_read_in: read.read_in,
                examples: RwLock::default(),
            });
        }
        let today = today_in(zone.as_ref());
        let dates = Dates {
            languages,
            zone,
            today,
        };
        (dates, problems)
    }

    /// The same dates with `site.today` fixed, so a build can be repeated.
    pub(crate) fn with_today(self, today: Date) -> Dates {
        Dates { today, ..self }
    }

    /// Whether the moment `date` names falls after the build's day ends on
    /// the site's clock.
    pub(crate) fn after_today(&self, date: &Date) -> bool {
        let today = &self.today;
        let Ok(tomorrow) = jiff::civil::Date::new(
            i16::try_from(today.year).unwrap_or(i16::MAX),
            i8::try_from(today.month).unwrap_or(1),
            i8::try_from(today.day).unwrap_or(1),
        )
        .and_then(|day| day.tomorrow()) else {
            return false;
        };
        let tomorrow = Date::day_only(
            i32::from(tomorrow.year()),
            tomorrow.month().unsigned_abs(),
            tomorrow.day().unsigned_abs(),
        );
        self.instant(date) >= self.instant(&tomorrow)
    }

    /// The present minute on the site's clock, as a date written with no
    /// offset names it there.
    pub(crate) fn now(&self) -> Date {
        let zone = self.zone.clone().unwrap_or_else(TimeZone::system);
        let now = jiff::Timestamp::now().to_zoned(zone);
        minute_of(&now, self.zone.is_some())
    }

    /// The language by `id`, its words and where they came from.
    pub fn language(&self, id: LanguageId) -> &Language {
        &self.languages[id.index()].language
    }

    /// The site's languages, main first, in the order of their ids.
    pub(crate) fn languages(&self) -> impl Iterator<Item = &Language> {
        self.languages.iter().map(|language| &language.language)
    }

    /// What the site's zone makes of `date`: the moment it names, with the
    /// offset in force there then. A written offset settles the moment on
    /// its own. A date with no time is taken at the start of its day.
    fn settle(&self, date: &Date) -> Settled {
        let (hour, minute, second) = date.time.map_or((0, 0, 0), |time| {
            (time.hour, time.minute, time.second.unwrap_or(0))
        });
        let face = Settled {
            year: date.year,
            month: date.month,
            day: date.day,
            hour,
            minute,
            second,
            offset: date.offset.map(|minutes| i32::from(minutes) * 60),
        };
        if date.offset.is_some() {
            return face;
        }
        let Some(zone) = &self.zone else {
            return face;
        };
        in_zone(zone, &face).unwrap_or(face)
    }

    /// Seconds from the start of 1970 to the moment `date` names.
    pub(crate) fn instant(&self, date: &Date) -> i64 {
        let at = self.settle(date);
        days_from_civil(at.year, at.month, at.day) * 86_400
            + i64::from(at.hour) * 3_600
            + i64::from(at.minute) * 60
            + i64::from(at.second)
            - i64::from(at.offset.unwrap_or(0))
    }

    fn reading(&self, date: &Date) -> Reading {
        let at = self.settle(date);
        Reading {
            year: at.year,
            month: at.month,
            day: at.day,
            weekday: weekday(at.year, at.month, at.day),
            time: date.time.map(|_| (at.hour, at.minute)),
        }
    }

    /// A template's example as `language` writes it: the language's look
    /// where it names one, else read in its words, or else in the first
    /// other language's that reads it.
    fn example(&self, language: LanguageId, example: &str) -> Example {
        let own = &self.languages[language.index()];
        if let Some(read) = own
            .examples
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(example)
        {
            return read.clone();
        }
        let read = match Look::named(example) {
            Some(look) => Example {
                format: Format::look(look, &own.words),
                read_in: None,
            },
            None => {
                let all = self.languages.iter().map(|language| &language.words);
                let others = others(language.index(), all);
                Example::read(example, &own.words, &others, Format::for_a_template)
            }
        };
        own.examples
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(example.to_owned(), read.clone());
        read
    }

    /// The other language whose words `language` reads the site's format,
    /// or a template's `example`, in, where its own words can't.
    pub(crate) fn read_in(
        &self,
        language: LanguageId,
        example: Option<&str>,
    ) -> Option<LanguageId> {
        match example {
            None => self.languages[language.index()].format_read_in,
            Some(example) => self.example(language, example).read_in,
        }
    }
}

/// The problem with a `date_format` that can't be read, placed in it.
fn refused(sources: &Sources, written: &Sourced<&str>, refusal: Refusal) -> Option<Problem> {
    let span = Span::new(refusal.at.start, refusal.at.end);
    let place: Option<Place> = place_within(sources, &written.origin, written.value, span);
    let kind = refusal.into_kind();
    place.map(|place| Problem::at(kind, place))
}

impl ShowDates for Dates {
    fn format(
        &self,
        language: LanguageId,
        date: &Date,
        example: Option<&str>,
    ) -> Result<String, Refusal> {
        let reading = self.reading(date);
        match example {
            None => Ok(self.languages[language.index()].format.write(&reading)),
            Some(example) => Ok(self.example(language, example).format?.write(&reading)),
        }
    }

    fn moment(&self, date: &Date) -> String {
        let at = self.settle(date);
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}{}",
            at.year,
            at.month,
            at.day,
            at.hour,
            at.minute,
            at.second,
            offset(at.offset, ":")
        )
    }

    fn moment_rss(&self, date: &Date) -> String {
        const WEEKDAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        const MONTHS: [&str; 12] = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ];
        let at = self.settle(date);
        format!(
            "{}, {:02} {} {:04} {:02}:{:02}:{:02} {}",
            WEEKDAYS[weekday(at.year, at.month, at.day)],
            at.day,
            MONTHS[usize::from(at.month) - 1],
            at.year,
            at.hour,
            at.minute,
            at.second,
            offset(at.offset, "")
        )
    }

    fn when(&self, language: LanguageId, when: &When) -> String {
        let format = &self.languages[language.index()].format;
        match when {
            When::Year(year) => format.year_alone(*year),
            When::Month(year, month) => format.month_and_year(*year, *month),
            When::Date(date) => format.write(&self.reading(date)),
        }
    }

    fn weekday(&self, language: LanguageId, date: &Date) -> Option<&str> {
        let reading = self.reading(date);
        let words = &self.languages[language.index()].words;
        let weekdays = words.words.weekdays.as_ref()?;
        Some(weekdays[reading.weekday].as_str())
    }

    fn today(&self) -> Date {
        self.today
    }

    fn order(&self, a: &Date, b: &Date) -> Ordering {
        self.instant(a).cmp(&self.instant(b))
    }
}

/// The warning for a site naming no `timezone` where a date it uses has no
/// written offset, which is then read as UTC: said once, of the site.
pub(crate) fn unzoned<'a>(
    settings: &Settings,
    mut dates: impl Iterator<Item = &'a Date>,
) -> Option<Problem> {
    let named = settings.get("timezone").is_some();
    (!named && dates.any(|date| date.offset.is_none())).then(|| Problem::nowhere(Kind::NoTimeZone))
}

/// A date as a zone settles it, with the offset in force: none where the
/// site names no zone and the date writes no offset.
#[derive(Copy, Clone, Debug)]
struct Settled {
    year: i32,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    /// Seconds east of UTC.
    offset: Option<i32>,
}

/// The wall time `at` in `zone`. A time the clocks skip names the moment
/// after the change, as the time it then reads.
fn in_zone(zone: &TimeZone, at: &Settled) -> Option<Settled> {
    let civil = jiff::civil::DateTime::new(
        i16::try_from(at.year).ok()?,
        i8::try_from(at.month).ok()?,
        i8::try_from(at.day).ok()?,
        i8::try_from(at.hour).ok()?,
        i8::try_from(at.minute).ok()?,
        i8::try_from(at.second).ok()?,
        0,
    )
    .ok()?;
    let zoned = zone.to_ambiguous_zoned(civil).compatible().ok()?;
    Some(Settled {
        year: i32::from(zoned.year()),
        month: u8::try_from(zoned.month()).ok()?,
        day: u8::try_from(zoned.day()).ok()?,
        hour: u8::try_from(zoned.hour()).ok()?,
        minute: u8::try_from(zoned.minute()).ok()?,
        second: u8::try_from(zoned.second()).ok()?,
        offset: Some(zoned.offset().seconds()),
    })
}

/// The current date in `zone`, or on the machine's clock with no zone.
fn today_in(zone: Option<&TimeZone>) -> Date {
    let zone = zone.cloned().unwrap_or_else(TimeZone::system);
    let now = jiff::Timestamp::now().to_zoned(zone);
    Date::day_only(
        i32::from(now.year()),
        u8::try_from(now.month()).unwrap_or(1),
        u8::try_from(now.day()).unwrap_or(1),
    )
}

/// The minute `now` is in, as its own clock shows it. Where the clocks
/// there show that time twice and this is the second, it carries its
/// offset, since a time written without one is read as the first; `zoned`
/// says whether times are read on that clock at all.
fn minute_of(now: &jiff::Zoned, zoned: bool) -> Date {
    let mut date = Date {
        year: i32::from(now.year()),
        month: now.month().unsigned_abs(),
        day: now.day().unsigned_abs(),
        time: Some(Time {
            hour: now.hour().unsigned_abs(),
            minute: now.minute().unsigned_abs(),
            second: None,
        }),
        offset: None,
    };
    let read = now.time_zone().to_ambiguous_zoned(now.datetime());
    let read_as = read.compatible().map(|read| read.offset());
    if zoned && read_as.is_ok_and(|offset| offset != now.offset()) {
        date.offset = i16::try_from(now.offset().seconds() / 60).ok();
    }
    date
}

/// `+01:00` or `+0100`, in whole minutes; `+00:00` with none.
fn offset(seconds: Option<i32>, between: &str) -> String {
    let minutes = seconds.unwrap_or(0) / 60;
    let sign = if minutes < 0 { '-' } else { '+' };
    let minutes = minutes.unsigned_abs();
    format!("{sign}{:02}{between}{:02}", minutes / 60, minutes % 60)
}

/// The weekday a day falls on, Monday first.
fn weekday(year: i32, month: u8, day: u8) -> usize {
    // 1970-01-01 was a Thursday, three days after a Monday.
    (days_from_civil(year, month, day) + 3).rem_euclid(7) as usize
}

/// Days from 1970-01-01, after Howard Hinnant's algorithm.
fn days_from_civil(year: i32, month: u8, day: u8) -> i64 {
    let year = i64::from(year) - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let of_era = year - era * 400;
    let month = (i64::from(month) + 9) % 12;
    let of_year = (153 * month + 2) / 5 + i64::from(day) - 1;
    let of_era_days = of_era * 365 + of_era / 4 - of_era / 100 + of_year;
    era * 146_097 + of_era_days - 719_468
}

/// The zone named most like `written`, where one is near enough.
pub(crate) fn nearest_zone(written: &str) -> Option<String> {
    let names: Vec<String> = jiff::tz::db()
        .available()
        .map(|name| name.as_str().to_owned())
        .collect();
    crate::suggest::nearest(written, names.iter().map(String::as_str)).map(str::to_owned)
}
