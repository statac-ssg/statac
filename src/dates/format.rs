//! A date format spelled by example: the reference moment, Wednesday 31
//! December 2025 at 15:04, written the way the author wants every date
//! written, read once into a pattern of pieces and the text between them.
//!
//! Every piece of the reference moment is a different number or word, so
//! an example can only be read one way: `15:04` is the hour and the
//! minute, because nothing else in the moment is 15 or 4.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

use crate::language::Words;
use crate::problem::{FormatFault, Kind, Piece, WordsFrom, WordsWanted};

const DAY: u32 = 31;
const MONTH: u32 = 12;
const YEAR: u32 = 2025;
const YEAR_SHORT: u32 = 25;
const HOUR: u32 = 15;
const HOUR_TWELVE: u32 = 3;
const MINUTE: u32 = 4;
/// Wednesday, in a list of weekdays Monday first.
const WEEKDAY: usize = 2;
/// December, in a list of months January first.
const MONTH_LISTED: usize = 11;
/// The reference day in figures, for a site with no words for months.
const YEAR_FIRST: &str = "2025-12-31";
/// The reference day in figures, offered beside its spelling in words.
const IN_FIGURES: &str = "31/12/2025";

/// How languages write the reference day, where it isn't the day, the
/// month in words and the year: what a site naming no `date_format`
/// writes, and what Statac offers as an example. Each is the Unicode
/// CLDR's long date pattern for the language, as is a language's left out.
const HOUSE: &[(&str, &str)] = &[
    ("de", "31. Dezember 2025"),
    ("es", "31 de diciembre de 2025"),
    ("ja", "2025年12月31日"),
    ("ko", "2025년 12월 31일"),
];

/// How the language of `tag` writes the reference day, where
/// [`HOUSE`] says.
pub fn house_example(tag: &str) -> Option<&'static str> {
    let language = tag.split('-').next().unwrap_or(tag);
    HOUSE
        .iter()
        .find(|(known, _)| known.eq_ignore_ascii_case(language))
        .map(|(_, example)| *example)
}

/// A date as a format writes it: the calendar day, the weekday it falls
/// on, and the time of day where the date has one.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Reading {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    /// Monday first.
    pub weekday: usize,
    pub time: Option<(u8, u8)>,
}

/// The words a site's dates are written in, and how a format refers to
/// them.
#[derive(Clone, Debug)]
pub struct DateWords {
    pub words: Arc<Words>,
    /// Where the words come from, for saying which has none.
    pub from: WordsFrom,
    /// Whether a day or a month in figures keeps its leading zero.
    pub zeros: bool,
    /// How the language writes the reference day, where it isn't the
    /// day, the month in words and the year.
    pub house: Option<&'static str>,
}

impl DateWords {
    pub fn english() -> DateWords {
        DateWords {
            words: crate::language::english(),
            from: WordsFrom::Statac("en".into()),
            zeros: true,
            house: None,
        }
    }

    /// Whether these are English's months, which alone the reference
    /// moment is written out in.
    fn english_like(&self) -> bool {
        self.words.months.is_some() && self.words.months == crate::language::english().months
    }

    /// The reference moment as these words write it, for saying what an
    /// example is read against. Other languages have it in figures, year
    /// first: Statac knows no other language's order of a date's words.
    pub fn moment(&self) -> String {
        let months = match &self.words.months {
            Some(months) if self.english_like() => months,
            _ => return format!("{YEAR_FIRST} 15:04"),
        };
        let weekday = self
            .words
            .weekdays
            .as_ref()
            .map(|weekdays| format!("{} ", weekdays[WEEKDAY]))
            .unwrap_or_default();
        format!(
            "{weekday}31 {} 2025, {}",
            months[MONTH_LISTED],
            self.clock()
        )
    }

    fn clock(&self) -> String {
        self.twelve_hour().unwrap_or_else(|| "15:04".into())
    }

    fn twelve_hour(&self) -> Option<String> {
        let am_pm = self.words.am_pm.as_ref()?;
        Some(format!("3.04{}", am_pm[1]))
    }

    /// The reference day with its month in words, where there are words,
    /// as the language writes it.
    fn in_words(&self) -> Option<String> {
        let months = self.words.months.as_ref()?;
        Some(match self.house {
            Some(example) => example.to_owned(),
            None => format!("31 {} 2025", months[MONTH_LISTED]),
        })
    }

    /// The reference day as these words best write it.
    fn day(&self) -> String {
        self.in_words().unwrap_or_else(|| YEAR_FIRST.into())
    }

    fn in_figures(&self) -> &'static str {
        match self.words.months {
            Some(_) => IN_FIGURES,
            None => YEAR_FIRST,
        }
    }

    /// Spellings of the reference day to offer as examples.
    fn examples(&self) -> Vec<String> {
        std::iter::once(self.in_figures().to_owned())
            .chain(self.in_words())
            .collect()
    }

    /// The ways these words write the reference moment's month, each once.
    fn month_spellings(&self) -> Vec<String> {
        let mut spellings: Vec<String> = Vec::new();
        let lists = [&self.words.months, &self.words.months_short];
        let words = lists.into_iter().flatten().map(|m| m[MONTH_LISTED].clone());
        for spelling in words.chain(std::iter::once("12".to_owned())) {
            if !spellings.contains(&spelling) {
                spellings.push(spelling);
            }
        }
        spellings
    }
}

/// Why an example can't be read, and the stretch of it to point at.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Refusal {
    pub fault: FormatFault,
    /// Bytes of the example, as written.
    pub at: Range<usize>,
}

impl Refusal {
    /// The problem to tell, placed by whoever knows where the example is
    /// written.
    pub fn into_kind(self) -> Kind {
        Kind::UnusableDateFormat {
            fault: Box::new(self.fault),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Case {
    Lower,
    Upper,
    /// The first letter a capital, the rest as the word is written.
    Title,
    /// As the language writes the word.
    Kept,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum MonthForm {
    Figures,
    Full,
    Short,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Component {
    Weekday {
        full: bool,
        case: Case,
    },
    /// `worded` where the day is written from the language's `days`.
    Day {
        worded: bool,
    },
    Month {
        form: MonthForm,
        case: Case,
    },
    Year {
        short: bool,
    },
    Hour {
        twelve: bool,
        padded: bool,
    },
    Minute {
        padded: bool,
    },
    AmPm {
        dots: bool,
        case: Case,
    },
}

impl Component {
    fn piece(self) -> Piece {
        match self {
            Component::Weekday { .. } => Piece::Weekday,
            Component::Day { .. } => Piece::Day,
            Component::Month { .. } => Piece::Month,
            Component::Year { .. } => Piece::Year,
            Component::Hour { .. } => Piece::Hour,
            Component::Minute { .. } => Piece::Minute,
            Component::AmPm { .. } => Piece::AmPm,
        }
    }

    fn is_time(self) -> bool {
        matches!(self.piece(), Piece::Hour | Piece::Minute | Piece::AmPm)
    }

    fn is_date(self) -> bool {
        matches!(self.piece(), Piece::Day | Piece::Month | Piece::Year)
    }

    /// The list of words this piece is written from, where the language
    /// hasn't got it. A month with no day beside it stands alone.
    fn unwritable(self, has_day: bool, words: &Words) -> Option<WordsWanted> {
        let missing = match self {
            Component::Weekday { full: true, .. } => words.weekdays.is_none(),
            Component::Weekday { full: false, .. } => words.weekdays_short.is_none(),
            Component::Month {
                form: MonthForm::Full,
                ..
            } => match has_day {
                true => words.months.is_none(),
                false => words.months_with_no_day().is_none(),
            },
            Component::Month {
                form: MonthForm::Short,
                ..
            } => match has_day {
                true => words.months_short.is_none(),
                false => words.months_short_with_no_day().is_none(),
            },
            Component::AmPm { .. } => words.am_pm.is_none(),
            Component::Day { worded: true } => words.days.is_none(),
            _ => false,
        };
        missing.then(|| wanted(self))
    }
}

fn wanted(component: Component) -> WordsWanted {
    match component {
        Component::Weekday { full: true, .. } => WordsWanted::Weekdays,
        Component::Weekday { full: false, .. } => WordsWanted::WeekdaysShort,
        Component::Month {
            form: MonthForm::Short,
            ..
        } => WordsWanted::MonthsShort,
        Component::AmPm { .. } => WordsWanted::AmPm,
        Component::Day { .. } => WordsWanted::Days,
        _ => WordsWanted::Months,
    }
}

#[derive(Clone, PartialEq, Debug)]
enum Part {
    Literal(String),
    Component(Component),
}

/// A date format read from an example.
#[derive(Clone, Debug)]
pub struct Format {
    parts: Vec<Part>,
    words: DateWords,
}

/// A format's day written shorter, each an example for the `date` filter.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Shorter {
    /// The day, the month and the year.
    pub short: String,
    /// The day and the month.
    pub day: String,
    /// The day, the month and the year, then the time.
    pub stamp: String,
}

impl Format {
    /// Reads an example a template hands the `date` filter. It may ask for
    /// any piece alone, but a day and a year with no month between them is
    /// an example whose month is in words the site hasn't got.
    pub fn for_a_template(example: &str, words: &DateWords) -> Result<Format, Refusal> {
        let format = Format::read(example, words)?;
        if format.writes(Piece::Day) && format.writes(Piece::Year) && !format.writes(Piece::Month) {
            return Err(whole(
                example,
                FormatFault::Missing {
                    example: example.to_owned(),
                    pieces: vec![Piece::Month],
                    months: words.month_spellings(),
                },
            ));
        }
        Ok(format)
    }

    /// Reads a site's `date_format`, the spelling of every date on the
    /// site, so it must write the day, the month and the year, and the
    /// year in full.
    pub fn for_the_site(example: &str, words: &DateWords) -> Result<Format, Refusal> {
        let format = Format::read(example, words)?;
        if !format.parts_of(Component::is_date) {
            let fault = match format.parts_of(Component::is_time) {
                true => FormatFault::OnlyATime {
                    example: example.to_owned(),
                    with_date: format!("{}, {example}", words.day()),
                },
                false => FormatFault::OnlyAWeekday {
                    example: example.to_owned(),
                    with_date: words
                        .words
                        .weekdays
                        .as_ref()
                        .map(|weekdays| format!("{} {}", weekdays[WEEKDAY], words.day())),
                },
            };
            return Err(whole(example, fault));
        }
        let pieces: Vec<Piece> = [Piece::Day, Piece::Month, Piece::Year]
            .into_iter()
            .filter(|piece| !format.writes(*piece))
            .collect();
        if !pieces.is_empty() {
            let fault = FormatFault::Missing {
                example: example.to_owned(),
                pieces,
                months: words.month_spellings(),
            };
            return Err(whole(example, fault));
        }
        let short_year =
            |part: &Part| matches!(part, Part::Component(Component::Year { short: true }));
        if format.parts.iter().any(short_year) {
            let fault = FormatFault::TwoDigitYear {
                example: example.to_owned(),
            };
            return Err(whole(example, fault));
        }
        Ok(format)
    }

    /// The format a site naming no `date_format` writes its dates in: the
    /// reference day with its month in the site's words, or in figures
    /// where it has none.
    pub fn house(words: &DateWords) -> Format {
        let in_figures =
            || Format::read(YEAR_FIRST, words).expect("a date in figures needs no words");
        match words.in_words() {
            Some(example) => Format::for_the_site(&example, words).unwrap_or_else(|_| in_figures()),
            None => in_figures(),
        }
    }

    /// The same pieces written in another language's `words`, the text
    /// between them as written in `example`. A piece written in words that
    /// language hasn't got is refused.
    pub fn in_words(&self, words: &DateWords, example: &str) -> Result<Format, Refusal> {
        let has_day = self.writes(Piece::Day);
        for part in &self.parts {
            if let Part::Component(component) = part
                && let Some(list) = component.unwritable(has_day, &words.words)
            {
                let fault = FormatFault::NoWords {
                    list,
                    from: words.from.clone(),
                };
                return Err(whole(example, fault));
            }
        }
        let mut format = Format {
            parts: self.parts.clone(),
            words: words.clone(),
        };
        format.settle_the_day();
        Ok(format)
    }

    /// Reads an example in the site's words. Where the site writes its own
    /// digits, an example holding them is read as in 0 to 9, and any
    /// refusal told as written.
    pub fn read(example: &str, words: &DateWords) -> Result<Format, Refusal> {
        let digits = words.words.digits.as_ref();
        let Some(plain) = digits.and_then(|digits| Plain::of(example, digits)) else {
            let format = Format::read_as_written(example, words)?;
            return format.without_stray_digits(example, words);
        };
        Format::read_as_written(&plain.text, words).or_else(|refused| {
            Format::read_as_written(example, words)
                .and_then(|format| format.without_stray_digits(example, words))
                .map_err(|_| plain.as_written(refused))
        })
    }

    /// Read as written, the site's own digits can only stand as text
    /// between the pieces, which is no reading of them.
    fn without_stray_digits(self, example: &str, words: &DateWords) -> Result<Format, Refusal> {
        let digits = words.words.digits.as_ref();
        let stray = self.parts.iter().any(|part| match (part, digits) {
            (Part::Literal(text), Some(digits)) => Plain::of(text, digits).is_some(),
            _ => false,
        });
        match stray {
            true => Err(whole(
                example,
                FormatFault::NoPattern {
                    example: example.to_owned(),
                    reference: words.moment(),
                    examples: words.examples(),
                },
            )),
            false => Ok(self),
        }
    }

    fn read_as_written(example: &str, words: &DateWords) -> Result<Format, Refusal> {
        if example.trim().is_empty() {
            return Err(Refusal {
                fault: FormatFault::Empty {
                    reference: words.moment(),
                    examples: words.examples(),
                },
                at: 0..example.len(),
            });
        }
        let tokens = tokenize(example, &words.words);
        let resolved: Vec<Resolution> = tokens
            .iter()
            .map(|token| resolve(token, &words.words))
            .collect();
        let refuse = |fault: FormatFault, at: Range<usize>| Err(Refusal { fault, at });
        let no_words = |list: WordsWanted, at: Range<usize>| {
            refuse(
                FormatFault::NoWords {
                    list,
                    from: words.from.clone(),
                },
                at,
            )
        };
        let spliced = |at: &Range<usize>, with: &str| {
            format!("{}{with}{}", &example[..at.start], &example[at.end..])
        };

        // English never stands in for another language's words.
        for (token, resolution) in tokens.iter().zip(&resolved) {
            if let Resolution::Unworded(list) = resolution {
                return no_words(*list, token.span.clone());
            }
        }

        // A weekday's word is the reference moment's, a Wednesday's, or
        // none; and a month's is December's or none.
        for (token, resolution) in tokens.iter().zip(&resolved) {
            match resolution {
                Resolution::OtherWeekday { full } => {
                    let names = match full {
                        true => words.words.weekdays.as_ref(),
                        false => words.words.weekdays_short.as_ref(),
                    }
                    .or(words.words.weekdays.as_ref())
                    .expect("a weekday's word is read from a list of them");
                    let named = words
                        .words
                        .weekdays
                        .as_ref()
                        .unwrap_or(names)
                        .get(WEEKDAY)
                        .cloned()
                        .unwrap_or_default();
                    return refuse(
                        FormatFault::OtherWeekday {
                            written: example[token.span.clone()].to_owned(),
                            day: words.day(),
                            weekday: named,
                            corrected: spliced(&token.span, &names[WEEKDAY]),
                        },
                        token.span.clone(),
                    );
                }
                Resolution::OtherMonth { list } => {
                    let names = list
                        .in_words(&words.words)
                        .expect("a month's word is read from a list of them");
                    let named = words
                        .words
                        .months_with_no_day()
                        .or(words.words.months_short_with_no_day())
                        .map(|months| months[MONTH_LISTED].clone())
                        .unwrap_or_default();
                    return refuse(
                        FormatFault::OtherMonth {
                            written: example[token.span.clone()].to_owned(),
                            day: words.day(),
                            month: named,
                            corrected: spliced(&token.span, &names[MONTH_LISTED]),
                        },
                        token.span.clone(),
                    );
                }
                _ => {}
            }
        }

        // A piece written twice, told at the second.
        let mut seen: HashMap<Piece, Range<usize>> = HashMap::new();
        for (token, resolution) in tokens.iter().zip(&resolved) {
            let Resolution::Component(component) = resolution else {
                continue;
            };
            let piece = component.piece();
            if let Some(first) = seen.get(&piece) {
                return refuse(
                    FormatFault::Twice {
                        first: example[first.clone()].to_owned(),
                        second: example[token.span.clone()].to_owned(),
                        piece,
                    },
                    token.span.clone(),
                );
            }
            seen.insert(piece, token.span.clone());
        }
        let has = |piece: Piece| seen.contains_key(&piece);

        // A word beside the day or the year of an example writing no
        // month is its month, in words the site hasn't got.
        if !has(Piece::Month)
            && words.words.months_with_no_day().is_none()
            && let Some(token) = month_beside(&tokens, &resolved)
        {
            return no_words(WordsWanted::Months, token.span.clone());
        }

        // Two numbers matching neither the day nor the month, both still
        // open, are the day and the month themselves.
        let orphans: Vec<&Token> = tokens
            .iter()
            .zip(&resolved)
            .filter(|(_, r)| matches!(r, Resolution::Orphan))
            .map(|(token, _)| token)
            .collect();
        let typo = resolved.iter().any(|r| matches!(r, Resolution::Typo(_)));
        if !has(Piece::Day) && !has(Piece::Month) && orphans.len() == 2 && !typo {
            let (first, second) = (&orphans[0].span, &orphans[1].span);
            let corrected = format!(
                "{}{DAY}{}{MONTH}{}",
                &example[..first.start],
                &example[first.end..second.start],
                &example[second.end..]
            );
            let at = first.start..second.end;
            return refuse(
                FormatFault::NotInReference {
                    written: example[at.clone()].to_owned(),
                    reference: words.moment(),
                    corrected: Some(corrected),
                },
                at,
            );
        }

        // The first number or word, left to right, that matches nothing.
        for (token, resolution) in tokens.iter().zip(&resolved) {
            let corrected = match resolution {
                Resolution::Orphan => open_slot(&seen).map(|value| spliced(&token.span, &value)),
                Resolution::Typo(word) => Some(spliced(&token.span, word)),
                _ => continue,
            };
            return refuse(
                FormatFault::NotInReference {
                    written: example[token.span.clone()].to_owned(),
                    reference: words.moment(),
                    corrected,
                },
                token.span.clone(),
            );
        }

        // An hour on a twelve-hour clock wants its morning or afternoon.
        let twelve = tokens.iter().zip(&resolved).find(|(_, r)| {
            matches!(
                r,
                Resolution::Component(Component::Hour { twelve: true, .. })
            )
        });
        if let Some((token, _)) = twelve
            && !has(Piece::AmPm)
        {
            return match &words.words.am_pm {
                Some([am, pm]) => refuse(
                    FormatFault::LoneHour {
                        written: example[token.span.clone()].to_owned(),
                        am: am.clone(),
                        pm: pm.clone(),
                    },
                    token.span.clone(),
                ),
                None => no_words(WordsWanted::AmPm, token.span.clone()),
            };
        }
        if let Some(minute) = seen.get(&Piece::Minute)
            && !has(Piece::Hour)
        {
            return refuse(
                FormatFault::MinuteWithoutHour {
                    written: example[minute.clone()].to_owned(),
                    examples: std::iter::once("15:04".to_owned())
                        .chain(words.twelve_hour())
                        .collect(),
                },
                minute.clone(),
            );
        }

        // Every word the format writes comes from one of the language's
        // lists, told at the first word wanting one it hasn't got.
        for (token, resolution) in tokens.iter().zip(&resolved) {
            if let Resolution::Component(component) = resolution
                && let Some(list) = component.unwritable(has(Piece::Day), &words.words)
            {
                return no_words(list, token.span.clone());
            }
        }

        let mut parts = Vec::new();
        let mut literal: Option<Range<usize>> = None;
        for (token, resolution) in tokens.iter().zip(&resolved) {
            match resolution {
                Resolution::Component(component) => {
                    if let Some(span) = literal.take() {
                        parts.push(Part::Literal(example[span].to_owned()));
                    }
                    parts.push(Part::Component(*component));
                }
                Resolution::Literal => {
                    literal = Some(match literal {
                        Some(span) => span.start..token.span.end,
                        None => token.span.clone(),
                    });
                }
                _ => unreachable!("refused before the pattern is put together"),
            }
        }
        if let Some(span) = literal {
            parts.push(Part::Literal(example[span].to_owned()));
        }
        let parts = attached_apart(parts);
        if !parts.iter().any(|part| matches!(part, Part::Component(_))) {
            return Err(whole(
                example,
                FormatFault::NoPattern {
                    example: example.to_owned(),
                    reference: words.moment(),
                    examples: words.examples(),
                },
            ));
        }
        let mut format = Format {
            parts,
            words: words.clone(),
        };
        format.settle_the_day();
        Ok(format)
    }

    /// A day the language writes the 31st of in figures (French `31`,
    /// whose first is `1er`) is written from `days` only beside a month in
    /// words: among figures the day is a number, as those around it are.
    fn settle_the_day(&mut self) {
        let in_figures = self
            .words
            .words
            .days
            .as_ref()
            .is_some_and(|days| days[30].chars().all(|c| c.is_ascii_digit()));
        if !in_figures || !self.day_padded() {
            return;
        }
        for part in &mut self.parts {
            if let Part::Component(Component::Day { worded }) = part {
                *worded = false;
            }
        }
    }

    fn writes(&self, piece: Piece) -> bool {
        self.parts_of(|component| component.piece() == piece)
    }

    fn parts_of(&self, wanted: impl Fn(Component) -> bool) -> bool {
        self.parts
            .iter()
            .any(|part| matches!(part, Part::Component(c) if wanted(*c)))
    }

    /// Whether this format writes any part of the time of day.
    pub fn wants_time(&self) -> bool {
        self.parts_of(Component::is_time)
    }

    /// `reading` written in this format, its numbers in the site's digits.
    /// A date with no time of day never has one invented: the time's
    /// pieces go, with the text beside them.
    pub fn write(&self, reading: &Reading) -> String {
        let machine = self.for_a_machine();
        let written = Written {
            day_padded: (self.words.zeros || machine) && self.day_padded(),
            month_padded: self.words.zeros || machine,
            has_day: self.writes(Piece::Day),
            words: &self.words.words,
            digits: match machine {
                true => None,
                false => self.words.words.digits.as_ref(),
            },
        };
        let dropped = match reading.time {
            Some(_) => vec![false; self.parts.len()],
            None => self.dropped_without_a_time(),
        };
        let mut out = String::new();
        for (part, dropped) in self.parts.iter().zip(dropped) {
            match (part, dropped) {
                (_, true) => {}
                (Part::Component(component), false) => {
                    written.component(&mut out, *component, reading)
                }
                (Part::Literal(text), false) => out.push_str(text),
            }
        }
        out
    }

    /// The reference day as this format writes it.
    pub fn shown(&self) -> String {
        self.write(&Reading {
            year: 2025,
            month: 12,
            day: 31,
            weekday: WEEKDAY,
            time: None,
        })
    }

    /// This format's day three shorter ways, in its own order, or none
    /// where it writes a time or the pieces don't come apart cleanly. No
    /// weekday is written, and a month in words is in the language's short
    /// ones where it has them.
    pub fn shorter(&self) -> Option<Shorter> {
        if self.wants_time() {
            return None;
        }
        let mut short = self.clone();
        short.leave_out(Piece::Weekday)?;
        let shortened = Component::Month {
            form: MonthForm::Short,
            case: Case::Kept,
        };
        if shortened.unwritable(true, &self.words.words).is_none() {
            for part in &mut short.parts {
                if let Part::Component(Component::Month { form, .. }) = part
                    && *form == MonthForm::Full
                {
                    *form = MonthForm::Short;
                }
            }
        }
        let mut day = short.clone();
        day.leave_out(Piece::Year)?;
        let clock = in_digits(&format!("{HOUR}:{MINUTE:02}"), short.digits());
        let stamp = match short.day_padded() {
            true => format!("{} {clock}", short.shown()),
            false => format!("{}, {clock}", short.shown()),
        };
        Some(Shorter {
            short: short.shown(),
            day: day.shown(),
            stamp,
        })
    }

    /// Takes `piece` out where it opens or closes the format, with the
    /// text between it and the rest. None where it stands inside.
    fn leave_out(&mut self, piece: Piece) -> Option<()> {
        let is = |part: &Part| matches!(part, Part::Component(c) if c.piece() == piece);
        let Some(at) = self.parts.iter().position(is) else {
            return Some(());
        };
        let literal = |at: usize| matches!(self.parts.get(at), Some(Part::Literal(_)));
        let range = match at {
            0 if literal(1) => 0..2,
            0 => 0..1,
            _ if at + 1 == self.parts.len() && literal(at - 1) => at - 1..at + 1,
            _ if at + 1 == self.parts.len() => at..at + 1,
            _ => return None,
        };
        self.parts.drain(range);
        Some(())
    }

    /// Which parts a date with no time of day leaves out: every piece of
    /// the time, the text between them, and the text leading into the
    /// run. A run with writing on both sides keeps the text after it, so
    /// the halves stay apart.
    fn dropped_without_a_time(&self) -> Vec<bool> {
        let literal = |at: usize| matches!(self.parts.get(at), Some(Part::Literal(_)));
        let mut dropped: Vec<bool> = self
            .parts
            .iter()
            .map(|part| matches!(part, Part::Component(c) if c.is_time()))
            .collect();
        for at in 1..self.parts.len() {
            let inside = dropped[at - 1] && dropped.get(at + 1).copied().unwrap_or(false);
            if literal(at) && inside {
                dropped[at] = true;
            }
        }
        let mut at = 0;
        while at < self.parts.len() {
            if !dropped[at] {
                at += 1;
                continue;
            }
            let start = at;
            while at < self.parts.len() && dropped[at] {
                at += 1;
            }
            let before = start.checked_sub(1).filter(|at| literal(*at));
            let after = Some(at).filter(|at| literal(*at));
            let between = before.is_some_and(|at| at > 0)
                && after.is_some_and(|at| at + 1 < self.parts.len());
            if let Some(before) = before {
                dropped[before] = true;
            }
            if let Some(after) = after
                && !between
            {
                dropped[after] = true;
            }
            at += 1;
        }
        dropped
    }

    /// A month and a year in this format's style, for a date that names no
    /// day, the month standing alone. A format writing its month in figures
    /// gets the one spelling of a month and a year read only one way.
    pub fn month_and_year(&self, year: i32, month: u8) -> String {
        let words = &self.words.words;
        let at = usize::from(month).wrapping_sub(1);
        let form = self.parts.iter().find_map(|part| match part {
            Part::Component(Component::Month { form, case }) => Some((*form, *case)),
            _ => None,
        });
        let named = match form {
            Some((MonthForm::Full, case)) => words
                .months_with_no_day()
                .and_then(|months| months.get(at))
                .map(|month| cased(month, case)),
            Some((MonthForm::Short, case)) => words
                .months_short_with_no_day()
                .and_then(|months| months.get(at))
                .map(|month| cased(month, case)),
            _ => None,
        };
        let written = match named {
            Some(month) => format!("{month} {year}"),
            None => format!("{year:04}-{month:02}"),
        };
        in_digits(&written, self.digits())
    }

    pub fn year_alone(&self, year: i32) -> String {
        in_digits(&year.to_string(), self.digits())
    }

    fn digits(&self) -> Option<&[char; 10]> {
        match self.for_a_machine() {
            true => None,
            false => self.words.words.digits.as_ref(),
        }
    }

    /// Whether this format is a date as ISO 8601 writes one, `2025-12-31`
    /// or `2025-12-31 15:04`: a machine's to read, in 0 to 9 with its zeros
    /// whatever the site's digits and `date_zeros`.
    fn for_a_machine(&self) -> bool {
        let literal = |part: &Part, wanted: &[&str]| matches!(part, Part::Literal(written) if wanted.contains(&written.as_str()));
        let (date, time) = self.parts.split_at(self.parts.len().min(5));
        let a_date = match date {
            [
                Part::Component(Component::Year { short: false }),
                dash,
                Part::Component(Component::Month {
                    form: MonthForm::Figures,
                    ..
                }),
                other_dash,
                Part::Component(Component::Day { worded: false }),
            ] => literal(dash, &["-"]) && literal(other_dash, &["-"]),
            _ => false,
        };
        let a_time = match time {
            [] => true,
            [
                between,
                Part::Component(Component::Hour { twelve: false, .. }),
                colon,
                Part::Component(Component::Minute { .. }),
            ] => literal(between, &[" ", "T"]) && literal(colon, &[":"]),
            _ => false,
        };
        a_date && a_time
    }

    /// A day in figures keeps its zero, save beside a month in words:
    /// `07/03/2026` but `7 March 2026`.
    fn day_padded(&self) -> bool {
        !self.parts.iter().any(|part| {
            matches!(part, Part::Component(Component::Month { form, .. }) if *form != MonthForm::Figures)
        })
    }
}

/// The text between a piece of the date and the time that follows it,
/// parted where it begins with what belongs to the date: the letters a
/// language writes after a figure, as `日` in `31日`, and a closing
/// bracket. A date with no time then keeps them.
fn attached_apart(parts: Vec<Part>) -> Vec<Part> {
    let mut apart = Vec::with_capacity(parts.len() + 1);
    for (at, part) in parts.iter().enumerate() {
        let (Part::Literal(text), Some(Part::Component(before)), Some(Part::Component(after))) = (
            part,
            at.checked_sub(1).and_then(|at| parts.get(at)),
            parts.get(at + 1),
        ) else {
            apart.push(part.clone());
            continue;
        };
        let figures = matches!(
            before,
            Component::Day { worded: false }
                | Component::Month {
                    form: MonthForm::Figures,
                    ..
                }
                | Component::Year { .. }
        );
        let belongs =
            |c: char| (figures && c.is_alphabetic()) || matches!(c, ')' | ']' | '）' | '」' | '』');
        let split = text
            .char_indices()
            .find(|(_, c)| !belongs(*c))
            .map_or(text.len(), |(at, _)| at);
        if before.is_time() || !after.is_time() || split == 0 || split == text.len() {
            apart.push(part.clone());
            continue;
        }
        apart.push(Part::Literal(text[..split].to_owned()));
        apart.push(Part::Literal(text[split..].to_owned()));
    }
    apart
}

fn whole(example: &str, fault: FormatFault) -> Refusal {
    Refusal {
        fault,
        at: 0..example.len(),
    }
}

/// Where only one of the day, the month and the year is still open, an
/// unmatched number was meant for it.
fn open_slot(seen: &HashMap<Piece, Range<usize>>) -> Option<String> {
    let open = |piece| !seen.contains_key(&piece);
    match (open(Piece::Day), open(Piece::Month), open(Piece::Year)) {
        (true, false, false) => Some(DAY.to_string()),
        (false, true, false) => Some(MONTH.to_string()),
        (false, false, true) => Some(YEAR.to_string()),
        _ => None,
    }
}

/// What a format needs to write each piece.
struct Written<'a> {
    day_padded: bool,
    month_padded: bool,
    has_day: bool,
    words: &'a Words,
    digits: Option<&'a [char; 10]>,
}

impl Written<'_> {
    fn figures(&self, out: &mut String, figures: String) {
        out.push_str(&in_digits(&figures, self.digits));
    }

    fn padded(&self, out: &mut String, n: u32, padded: bool) {
        match padded {
            true => self.figures(out, format!("{n:02}")),
            false => self.figures(out, n.to_string()),
        }
    }

    fn component(&self, out: &mut String, component: Component, at: &Reading) {
        let words = self.words;
        let (hour, minute) = at.time.unwrap_or_default();
        match component {
            Component::Weekday { full, case } => {
                let list = match full {
                    true => words.weekdays.as_ref(),
                    false => words.weekdays_short.as_ref(),
                };
                out.push_str(&cased(from(list, at.weekday), case));
            }
            Component::Day { worded: true } => {
                out.push_str(from(words.days.as_ref(), usize::from(at.day) - 1));
            }
            Component::Day { worded: false } => {
                self.padded(out, u32::from(at.day), self.day_padded);
            }
            Component::Month { form, case } => {
                let at_month = usize::from(at.month) - 1;
                let list = match (form, self.has_day) {
                    (MonthForm::Figures, _) => {
                        return self.padded(out, u32::from(at.month), self.month_padded);
                    }
                    (MonthForm::Full, true) => words.months.as_ref(),
                    (MonthForm::Full, false) => words.months_with_no_day(),
                    (MonthForm::Short, true) => words.months_short.as_ref(),
                    (MonthForm::Short, false) => words.months_short_with_no_day(),
                };
                out.push_str(&cased(from(list, at_month), case));
            }
            Component::Year { short: true } => {
                self.figures(out, format!("{:02}", at.year.rem_euclid(100)));
            }
            Component::Year { short: false } => self.figures(out, format!("{:04}", at.year)),
            Component::Hour {
                twelve: true,
                padded,
            } => {
                let hour = match hour % 12 {
                    0 => 12,
                    other => other,
                };
                self.padded(out, u32::from(hour), padded);
            }
            Component::Hour { twelve: false, .. } => self.padded(out, u32::from(hour), true),
            Component::Minute { padded } => self.padded(out, u32::from(minute), padded),
            Component::AmPm { dots, case } => {
                let word = cased(from(words.am_pm.as_ref(), usize::from(hour >= 12)), case);
                match dots {
                    true => {
                        for letter in word.chars() {
                            out.push(letter);
                            out.push('.');
                        }
                    }
                    false => out.push_str(&word),
                }
            }
        }
    }
}

/// One word of a list. A format is read only where its lists are there.
fn from<const N: usize>(list: Option<&[String; N]>, at: usize) -> &str {
    list.and_then(|list| list.get(at))
        .map_or("", String::as_str)
}

/// `written` with 0 to 9 in `digits`, zero first, and all else as it is.
pub fn in_digits(written: &str, digits: Option<&[char; 10]>) -> String {
    let Some(digits) = digits else {
        return written.to_owned();
    };
    written
        .chars()
        .map(|c| match c.to_digit(10) {
            Some(figure) => digits[figure as usize],
            None => c,
        })
        .collect()
}

/// An example respelt with the site's own digits as 0 to 9, and the way
/// back to where each byte stood as written.
struct Plain<'a> {
    written: &'a str,
    text: String,
    /// For each byte of `text` and its end, the byte it stood at.
    at: Vec<usize>,
    digits: &'a [char; 10],
}

impl<'a> Plain<'a> {
    /// None where `written` holds none of the digits.
    fn of(written: &'a str, digits: &'a [char; 10]) -> Option<Plain<'a>> {
        let mut text = String::with_capacity(written.len());
        let mut at = Vec::with_capacity(written.len() + 1);
        let mut any = false;
        for (from, c) in written.char_indices() {
            let figure = digits.iter().position(|d| *d == c && !c.is_ascii_digit());
            let c = match figure {
                Some(figure) => {
                    any = true;
                    char::from_digit(figure as u32, 10).expect("a figure")
                }
                None => c,
            };
            at.extend(std::iter::repeat_n(from, c.len_utf8()));
            text.push(c);
        }
        at.push(written.len());
        any.then_some(Plain {
            written,
            text,
            at,
            digits,
        })
    }

    fn span(&self, span: &Range<usize>) -> Range<usize> {
        let end = match span.end {
            0 => 0,
            end => {
                let last = self.at[end - 1];
                last + self.written[last..]
                    .chars()
                    .next()
                    .map_or(0, char::len_utf8)
            }
        };
        self.at[span.start]..end
    }

    fn figures(&self, text: &str) -> String {
        in_digits(text, Some(self.digits))
    }

    /// A refusal of the example in 0 to 9, told as written.
    fn as_written(&self, refusal: Refusal) -> Refusal {
        let example = || self.written.to_owned();
        let fault = match refusal.fault {
            FormatFault::Twice {
                first,
                second,
                piece,
            } => FormatFault::Twice {
                first: self.figures(&first),
                second: self.figures(&second),
                piece,
            },
            FormatFault::NotInReference {
                written,
                reference,
                corrected,
            } => FormatFault::NotInReference {
                written: self.figures(&written),
                reference,
                corrected: corrected.map(|c| self.figures(&c)),
            },
            FormatFault::LoneHour { written, am, pm } => FormatFault::LoneHour {
                written: self.figures(&written),
                am,
                pm,
            },
            FormatFault::MinuteWithoutHour { written, examples } => {
                FormatFault::MinuteWithoutHour {
                    written: self.figures(&written),
                    examples,
                }
            }
            FormatFault::NoPattern {
                reference,
                examples,
                ..
            } => FormatFault::NoPattern {
                example: example(),
                reference,
                examples,
            },
            FormatFault::OnlyATime { with_date, .. } => FormatFault::OnlyATime {
                example: example(),
                with_date,
            },
            FormatFault::OnlyAWeekday { with_date, .. } => FormatFault::OnlyAWeekday {
                example: example(),
                with_date,
            },
            FormatFault::Missing { pieces, months, .. } => FormatFault::Missing {
                example: example(),
                pieces,
                months,
            },
            FormatFault::TwoDigitYear { .. } => FormatFault::TwoDigitYear { example: example() },
            FormatFault::OtherWeekday {
                written,
                day,
                weekday,
                corrected,
            } => FormatFault::OtherWeekday {
                written: self.figures(&written),
                day,
                weekday,
                corrected: self.figures(&corrected),
            },
            FormatFault::OtherMonth {
                written,
                day,
                month,
                corrected,
            } => FormatFault::OtherMonth {
                written: self.figures(&written),
                day,
                month,
                corrected: self.figures(&corrected),
            },
            fault @ (FormatFault::Empty { .. } | FormatFault::NoWords { .. }) => fault,
        };
        Refusal {
            fault,
            at: self.span(&refusal.at),
        }
    }
}

fn detect_case(word: &str) -> Case {
    let letters: String = word.chars().filter(|c| c.is_alphabetic()).collect();
    let Some(first) = letters.chars().next() else {
        return Case::Title;
    };
    if letters.chars().all(|c| !c.is_lowercase()) {
        Case::Upper
    } else if first.is_uppercase() && letters.chars().skip(1).all(char::is_lowercase) {
        Case::Title
    } else {
        Case::Lower
    }
}

/// The case `written` puts the language's `word` in: kept where the two
/// are alike, so a word with capitals inside it keeps them.
fn case_of(written: &str, word: &str) -> Case {
    if written == word {
        Case::Kept
    } else if written == word.to_uppercase() {
        Case::Upper
    } else if written == word.to_lowercase() {
        Case::Lower
    } else if written == capitalised(word) {
        Case::Title
    } else {
        detect_case(written)
    }
}

fn capitalised(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn cased(word: &str, case: Case) -> String {
    match case {
        Case::Lower => word.to_lowercase(),
        Case::Upper => word.to_uppercase(),
        Case::Title => capitalised(word),
        Case::Kept => word.to_owned(),
    }
}

// Reading an example into tokens.

#[derive(Clone, Debug)]
struct Token {
    kind: TokenKind,
    span: Range<usize>,
}

#[derive(Clone, Debug)]
enum TokenKind {
    /// Figures, with an English ordinal's letters (`3rd`) where written.
    Number { text: String, ordinal: bool },
    /// A word from one of the language's lists.
    Named(Named),
    /// Letters that are none of those.
    Word(String),
    /// One character that is neither a letter nor a figure.
    Punct,
}

#[derive(Copy, Clone, Debug)]
enum Named {
    Weekday {
        full: bool,
        case: Case,
    },
    Month {
        form: MonthForm,
        case: Case,
    },
    AmPm {
        dots: bool,
        case: Case,
    },
    /// The 31st, as the language's `days` write it.
    Day,
    /// English's word for a list the language hasn't got.
    Unworded(WordsWanted),
    OtherWeekday {
        full: bool,
    },
    OtherMonth {
        list: MonthList,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum MonthList {
    Months,
    MonthsAlone,
    MonthsShort,
    MonthsShortAlone,
}

impl MonthList {
    const ALL: [MonthList; 4] = [
        MonthList::Months,
        MonthList::MonthsAlone,
        MonthList::MonthsShort,
        MonthList::MonthsShortAlone,
    ];

    fn form(self) -> MonthForm {
        match self {
            MonthList::Months | MonthList::MonthsAlone => MonthForm::Full,
            MonthList::MonthsShort | MonthList::MonthsShortAlone => MonthForm::Short,
        }
    }

    fn in_words(self, words: &Words) -> Option<&[String; 12]> {
        match self {
            MonthList::Months => &words.months,
            MonthList::MonthsAlone => &words.months_alone,
            MonthList::MonthsShort => &words.months_short,
            MonthList::MonthsShortAlone => &words.months_short_alone,
        }
        .as_ref()
    }
}

/// Which list a word was found in.
#[derive(Copy, Clone, Debug)]
enum Listed {
    /// Which weekday, Monday first.
    Weekday {
        full: bool,
        day: usize,
    },
    /// Which month, January first.
    Month {
        list: MonthList,
        month: usize,
    },
    AmPm,
    Day,
    Unworded(WordsWanted),
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Class {
    Figure,
    Letter,
    Other,
}

fn class(c: char) -> Class {
    if c.is_ascii_digit() {
        Class::Figure
    } else if c.is_alphabetic() {
        Class::Letter
    } else {
        Class::Other
    }
}

/// Runs of figures, runs of letters, and each other character alone.
fn runs(s: &str) -> Vec<(Range<usize>, Class)> {
    let mut runs: Vec<(Range<usize>, Class)> = Vec::new();
    for (at, c) in s.char_indices() {
        let class = class(c);
        let end = at + c.len_utf8();
        match runs.last_mut() {
            Some((span, last)) if *last == class && class != Class::Other => span.end = end,
            _ => runs.push((at..end, class)),
        }
    }
    runs
}

/// Every word an example may name, with its list: the language's own,
/// then English's for each list the language hasn't got, so a format in
/// English on a site in another language is told so rather than read.
fn listed<'a>(words: &'a Words, english: &'a Words) -> Vec<(&'a str, Listed)> {
    let mut all = Vec::new();
    for (list, full) in [(&words.weekdays, true), (&words.weekdays_short, false)] {
        for (day, word) in list.iter().flatten().enumerate() {
            all.push((word.as_str(), Listed::Weekday { full, day }));
        }
    }
    for list in MonthList::ALL {
        for (month, word) in list.in_words(words).into_iter().flatten().enumerate() {
            all.push((word.as_str(), Listed::Month { list, month }));
        }
    }
    for word in words.am_pm.iter().flatten() {
        all.push((word.as_str(), Listed::AmPm));
    }
    // The day is only ever the 31st, the reference moment's.
    if let Some(days) = &words.days {
        all.push((days[30].as_str(), Listed::Day));
    }
    let unworded: [(bool, Vec<&'a String>, WordsWanted); 6] = [
        (
            words.weekdays.is_none(),
            english.weekdays.iter().flatten().collect(),
            WordsWanted::Weekdays,
        ),
        (
            words.weekdays_short.is_none(),
            english.weekdays_short.iter().flatten().collect(),
            WordsWanted::WeekdaysShort,
        ),
        (
            words.months_with_no_day().is_none(),
            english.months.iter().flatten().collect(),
            WordsWanted::Months,
        ),
        (
            words.months_short_with_no_day().is_none(),
            english.months_short.iter().flatten().collect(),
            WordsWanted::MonthsShort,
        ),
        (
            words.am_pm.is_none(),
            english.am_pm.iter().flatten().collect(),
            WordsWanted::AmPm,
        ),
        (
            words.days.is_none(),
            english.days.iter().map(|days| &days[30]).collect(),
            WordsWanted::Days,
        ),
    ];
    for (missing, list, wanted) in unworded {
        if missing {
            all.extend(
                list.into_iter()
                    .map(|w| (w.as_str(), Listed::Unworded(wanted))),
            );
        }
    }
    all
}

/// The longest listed word written at `at`, and where it ends. A tie goes
/// to the list named first, so a French `mai`, a month long and short, is
/// the month.
fn named_at(s: &str, at: usize, listed: &[(&str, Listed)]) -> Option<(usize, Named)> {
    let rest = &s[at..];
    let mut longest: Option<(usize, &str, Listed, bool)> = None;
    for (word, list) in listed {
        let clock = matches!(list, Listed::AmPm | Listed::Unworded(WordsWanted::AmPm));
        let found = spelled(rest, word).map(|len| (len, false)).or_else(|| {
            clock
                .then(|| dotted(rest, word))
                .flatten()
                .map(|len| (len, true))
        });
        if let Some((len, dots)) = found
            && longest.is_none_or(|(so_far, ..)| len > so_far)
        {
            longest = Some((len, word, *list, dots));
        }
    }
    let (len, word, list, dots) = longest?;
    let written: String = match dots {
        true => rest[..len].chars().filter(|c| c.is_alphabetic()).collect(),
        false => rest[..len].to_owned(),
    };
    let case = case_of(&written, word);
    let named = match list {
        Listed::Weekday { full, day } if day == WEEKDAY => Named::Weekday { full, case },
        Listed::Weekday { full, .. } => Named::OtherWeekday { full },
        Listed::Month { list, month } if month == MONTH_LISTED => Named::Month {
            form: list.form(),
            case,
        },
        Listed::Month { list, .. } => Named::OtherMonth { list },
        Listed::AmPm => Named::AmPm { dots, case },
        Listed::Day => Named::Day,
        Listed::Unworded(wanted) => Named::Unworded(wanted),
    };
    Some((at + len, named))
}

fn same_letter(one: char, other: char) -> bool {
    one == other || one.to_lowercase().eq(other.to_lowercase())
}

/// How far `word` runs at the start of `rest`, in any case, where it ends
/// there: a word ending in a letter isn't the start of a longer word, nor
/// one ending in a figure the start of a longer number.
fn spelled(rest: &str, word: &str) -> Option<usize> {
    let mut len = 0;
    let mut chars = rest.chars();
    for wanted in word.chars() {
        let c = chars.next()?;
        if !same_letter(c, wanted) {
            return None;
        }
        len += c.len_utf8();
    }
    let last = word.chars().last()?;
    let ends = match rest[len..].chars().next() {
        None => true,
        Some(next) if last.is_alphabetic() => !next.is_alphabetic(),
        Some(next) if last.is_ascii_digit() => !next.is_alphanumeric(),
        Some(_) => true,
    };
    ends.then_some(len)
}

/// How far a word of letters runs at the start of `rest` with a full stop
/// after each letter, the last one's optional: `p.m.` and `p.m` for `pm`.
fn dotted(rest: &str, word: &str) -> Option<usize> {
    let letters = word.chars().count();
    if letters < 2 || !word.chars().all(char::is_alphabetic) {
        return None;
    }
    let mut len = 0;
    let mut chars = rest.chars().peekable();
    for (at, wanted) in word.chars().enumerate() {
        let c = chars.next()?;
        if !same_letter(c, wanted) {
            return None;
        }
        len += c.len_utf8();
        if at + 1 < letters {
            if chars.next()? != '.' {
                return None;
            }
            len += 1;
        }
    }
    match chars.peek() {
        Some('.') => len += 1,
        Some(next) if next.is_alphabetic() => return None,
        _ => {}
    }
    Some(len)
}

fn tokenize(s: &str, words: &Words) -> Vec<Token> {
    let english = crate::language::english();
    let listed = listed(words, &english);
    let runs = runs(s);
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < runs.len() {
        let (span, class) = runs[i].clone();
        // Another weekday's word straight after figures is the language's
        // own word for them, as Korean writes the day `31일`.
        let after_figures =
            i > 0 && runs[i - 1].1 == Class::Figure && runs[i - 1].0.end == span.start;
        if class != Class::Other
            && let Some((end, named)) = named_at(s, span.start, &listed)
            && !(after_figures && matches!(named, Named::OtherWeekday { .. }))
        {
            tokens.push(Token {
                kind: TokenKind::Named(named),
                span: span.start..end,
            });
            while i < runs.len() && runs[i].0.start < end {
                i += 1;
            }
            continue;
        }
        match class {
            Class::Figure => {
                let suffix = runs.get(i + 1).filter(|(next, class)| {
                    *class == Class::Letter
                        && next.start == span.end
                        && matches!(
                            s[next.clone()].to_ascii_lowercase().as_str(),
                            "st" | "nd" | "rd" | "th"
                        )
                });
                let end = suffix.map_or(span.end, |(next, _)| next.end);
                tokens.push(Token {
                    kind: TokenKind::Number {
                        text: s[span.clone()].to_owned(),
                        ordinal: suffix.is_some(),
                    },
                    span: span.start..end,
                });
                i += if suffix.is_some() { 2 } else { 1 };
            }
            Class::Letter => {
                tokens.push(Token {
                    kind: TokenKind::Word(s[span.clone()].to_owned()),
                    span,
                });
                i += 1;
            }
            Class::Other => {
                tokens.push(Token {
                    kind: TokenKind::Punct,
                    span,
                });
                i += 1;
            }
        }
    }
    tokens
}

// Matching tokens against the reference moment.

enum Resolution {
    Component(Component),
    Literal,
    /// A number matching nothing.
    Orphan,
    /// A word a letter or so from one the language writes the reference
    /// moment with.
    Typo(String),
    Unworded(WordsWanted),
    OtherWeekday {
        full: bool,
    },
    OtherMonth {
        list: MonthList,
    },
}

fn resolve(token: &Token, words: &Words) -> Resolution {
    match &token.kind {
        TokenKind::Number { text, ordinal } => match number(text, *ordinal) {
            Some(component) => Resolution::Component(component),
            None => Resolution::Orphan,
        },
        TokenKind::Named(named) => match *named {
            Named::Weekday { full, case } => {
                Resolution::Component(Component::Weekday { full, case })
            }
            Named::Month { form, case } => Resolution::Component(Component::Month { form, case }),
            Named::AmPm { dots, case } => Resolution::Component(Component::AmPm { dots, case }),
            Named::Day => Resolution::Component(Component::Day { worded: true }),
            Named::Unworded(list) => Resolution::Unworded(list),
            Named::OtherWeekday { full } => Resolution::OtherWeekday { full },
            Named::OtherMonth { list } => Resolution::OtherMonth { list },
        },
        TokenKind::Word(text) => {
            if other_weekday_begun(text, words) {
                return Resolution::OtherWeekday { full: false };
            }
            if let Some(list) = other_month_begun(text, words) {
                return Resolution::OtherMonth { list };
            }
            if let Some(component) = begun(text, words) {
                return Resolution::Component(component);
            }
            // A short word near a known one is left alone: "on" is a
            // letter from "Mon".
            if text.chars().count() < 4 {
                return Resolution::Literal;
            }
            match crate::suggest::nearest(text, known_words(words)) {
                Some(word) => Resolution::Typo(word.to_owned()),
                None => Resolution::Literal,
            }
        }
        TokenKind::Punct => Resolution::Literal,
    }
}

fn number(text: &str, ordinal: bool) -> Option<Component> {
    // Only a day is written with a suffix, and a day the language's
    // `days` write is read as one of its words.
    if ordinal {
        return None;
    }
    let value: u32 = text.parse().ok()?;
    let len = text.len();
    Some(match value {
        DAY => Component::Day { worded: false },
        MONTH => Component::Month {
            form: MonthForm::Figures,
            case: Case::Lower,
        },
        YEAR if len == 4 => Component::Year { short: false },
        YEAR_SHORT if len == 2 => Component::Year { short: true },
        HOUR => Component::Hour {
            twelve: false,
            padded: true,
        },
        HOUR_TWELVE => Component::Hour {
            twelve: true,
            padded: len == 2,
        },
        MINUTE => Component::Minute { padded: len == 2 },
        _ => return None,
    })
}

/// Whether `text`, three letters or more, begins `word` and is shorter.
fn begins(text: &str, word: &str) -> bool {
    let letters = text.chars().count();
    letters >= 3
        && word.chars().count() > letters
        && text
            .chars()
            .zip(word.chars())
            .all(|(a, b)| same_letter(a, b))
}

/// The start of the reference moment's month or weekday, a short form the
/// language's lists don't give: `Decem`, `Wednes`.
fn begun(text: &str, words: &Words) -> Option<Component> {
    let case = detect_case(text);
    let months = [&words.months, &words.months_alone];
    if months
        .into_iter()
        .flatten()
        .any(|months| begins(text, &months[MONTH_LISTED]))
    {
        return Some(Component::Month {
            form: MonthForm::Short,
            case,
        });
    }
    words
        .weekdays
        .as_ref()
        .filter(|weekdays| begins(text, &weekdays[WEEKDAY]))
        .map(|_| Component::Weekday { full: false, case })
}

/// Whether `text` begins another weekday's name than the reference
/// moment's: `Thurs`.
fn other_weekday_begun(text: &str, words: &Words) -> bool {
    words.weekdays.as_ref().is_some_and(|weekdays| {
        weekdays
            .iter()
            .enumerate()
            .any(|(day, name)| day != WEEKDAY && begins(text, name))
    })
}

/// The list to offer December from, where `text` begins another month's
/// name: `Sept`. A beginning is a short form, so the short list where the
/// language has one.
fn other_month_begun(text: &str, words: &Words) -> Option<MonthList> {
    let found = [MonthList::Months, MonthList::MonthsAlone]
        .into_iter()
        .find(|list| {
            list.in_words(words).is_some_and(|months| {
                months
                    .iter()
                    .enumerate()
                    .any(|(month, name)| month != MONTH_LISTED && begins(text, name))
            })
        })?;
    Some(match words.months_short {
        Some(_) => MonthList::MonthsShort,
        None => found,
    })
}

/// A word beside the day or the year, only punctuation and spaces between,
/// that no list reads: where the example writes no month, its month. Of
/// several, the one between the day and the year, else beside the year,
/// else after the day.
fn month_beside<'t>(tokens: &'t [Token], resolved: &[Resolution]) -> Option<&'t Token> {
    let only_punctuation = |from: usize, to: usize| {
        tokens[from..to]
            .iter()
            .all(|token| matches!(token.kind, TokenKind::Punct))
    };
    let beside = |at: usize, piece: Piece| {
        resolved.iter().enumerate().any(|(other, r)| {
            matches!(r, Resolution::Component(c) if c.piece() == piece)
                && match other < at {
                    true => only_punctuation(other + 1, at),
                    false => only_punctuation(at + 1, other),
                }
        })
    };
    let after_the_day = |at: usize| {
        resolved[..at].iter().enumerate().any(|(other, r)| {
            matches!(r, Resolution::Component(Component::Day { .. }))
                && only_punctuation(other + 1, at)
        })
    };
    tokens
        .iter()
        .zip(resolved)
        .enumerate()
        .filter(|(_, (token, r))| {
            matches!(&token.kind, TokenKind::Word(text) if text.chars().count() >= 3)
                && matches!(r, Resolution::Literal | Resolution::Typo(_))
        })
        .filter_map(|(at, (token, _))| {
            let rank = match (beside(at, Piece::Day), beside(at, Piece::Year)) {
                (true, true) => 0,
                (_, true) => 1,
                (true, _) if after_the_day(at) => 2,
                (true, _) => 3,
                _ => return None,
            };
            Some((rank, token))
        })
        .min_by_key(|(rank, _)| *rank)
        .map(|(_, token)| token)
}

/// The words the language writes the reference moment's weekday and month
/// with, offered for a word mistyped. Another weekday's or month's is
/// refused, so never offered.
fn known_words(words: &Words) -> impl Iterator<Item = &str> {
    let weekdays = [&words.weekdays, &words.weekdays_short]
        .into_iter()
        .flatten()
        .map(|list| list[WEEKDAY].as_str());
    let months = [
        &words.months,
        &words.months_short,
        &words.months_alone,
        &words.months_short_alone,
    ]
    .into_iter()
    .flatten()
    .map(|list| list[MONTH_LISTED].as_str());
    weekdays.chain(months)
}
