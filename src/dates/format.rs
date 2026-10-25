//! A date format spelled by example: the reference moment, Wednesday 31
//! December 2025 at 15:04, written the way the author wants every date
//! written, read once into a pattern of pieces and the text between them.
//!
//! Every piece of the reference moment is a different number or word, so
//! an example can only be read one way: `15:04` is the hour and the
//! minute, because nothing else in the moment is 15 or 4.
//!
//! A look a template names is read instead from the Unicode CLDR's pattern
//! for the language, in the CLDR's letters: `d MMMM y`.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

use crate::language::{Patterns, Words};
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
/// The reference day in figures, for a site with words for months but no
/// patterns.
const IN_FIGURES: &str = "31/12/2025";

/// A look a template names a date by, each written as the language's
/// Unicode CLDR data has it.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Look {
    /// The CLDR's long date: `31 December 2025`.
    Long,
    /// Its medium date: `31 Dec 2025`.
    Short,
    /// The day and the month: `31 Dec`.
    Day,
    /// The medium date and the short time: `31 Dec 2025, 15:04`.
    Stamp,
}

impl Look {
    pub const ALL: [Look; 4] = [Look::Long, Look::Short, Look::Day, Look::Stamp];

    /// As a template names it.
    pub fn name(self) -> &'static str {
        match self {
            Look::Long => "long",
            Look::Short => "short",
            Look::Day => "day",
            Look::Stamp => "stamp",
        }
    }

    pub fn named(name: &str) -> Option<Look> {
        Look::ALL.into_iter().find(|look| look.name() == name)
    }
}

/// The reference day, with no time.
const REFERENCE_DAY: Reading = Reading {
    year: YEAR as i32,
    month: MONTH as u8,
    day: DAY as u8,
    weekday: WEEKDAY,
    time: None,
};

/// The first day of a month, for writing a month or a year alone: no
/// format writing either writes the day or the weekday.
fn first_day(year: i32, month: u8) -> Reading {
    Reading {
        year,
        month,
        day: 1,
        weekday: 0,
        time: None,
    }
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
}

impl DateWords {
    pub fn english() -> DateWords {
        DateWords {
            words: crate::language::english(),
            from: WordsFrom::Statac("en".into()),
            zeros: true,
        }
    }

    /// Whether these are English's months, which alone the reference
    /// moment is written out in.
    fn english_like(&self) -> bool {
        self.words.months.is_some() && self.words.months == crate::language::english().months
    }

    /// The reference moment as these words write it, for saying what an
    /// example is read against. Other languages have it in figures, year
    /// first.
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
    /// as the language's long look writes it in 0 to 9.
    fn in_words(&self) -> Option<String> {
        let months = self.words.months.as_ref()?;
        let long = (self.words.dates.as_ref()).and_then(|_| Format::look(Look::Long, self).ok());
        Some(match long {
            Some(long) => long.written(&REFERENCE_DAY, false),
            None => format!("31 {} 2025", months[MONTH_LISTED]),
        })
    }

    /// The reference day as these words best write it.
    fn day(&self) -> String {
        self.in_words().unwrap_or_else(|| YEAR_FIRST.into())
    }

    /// The reference day in figures, as the language's short date writes
    /// it but with the year in full, as a site's `date_format` must.
    fn in_figures(&self) -> String {
        let figures = (self.words.dates.as_ref()).and_then(|p| Format::patterned(&p.figures, self));
        match (figures, &self.words.months) {
            (Some(mut figures), _) => {
                for part in &mut figures.parts {
                    if let Part::Component(Component::Year { short }) = part {
                        *short = false;
                    }
                }
                figures.written(&REFERENCE_DAY, false)
            }
            (None, Some(_)) => IN_FIGURES.into(),
            (None, None) => YEAR_FIRST.into(),
        }
    }

    /// The reference day with its weekday, as the language's full date
    /// writes it in 0 to 9, else the weekday before the day.
    fn with_weekday(&self) -> Option<String> {
        let full = (self.words.dates.as_ref())
            .and_then(|p| Format::patterned(&p.full, self))
            .map(|full| full.written(&REFERENCE_DAY, false));
        full.or_else(|| {
            (self.words.weekdays.as_ref())
                .map(|weekdays| format!("{} {}", weekdays[WEEKDAY], self.day()))
        })
    }

    /// Spellings of the reference day to offer as examples.
    fn examples(&self) -> Vec<String> {
        std::iter::once(self.in_figures())
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
    /// The refusal of an example that could have named a look instead.
    fn offering_looks(mut self) -> Refusal {
        if let FormatFault::NoPattern { looks, .. } | FormatFault::Empty { looks, .. } =
            &mut self.fault
        {
            *looks = true;
        }
        self
    }

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
    Figures {
        /// None where the format's own rule decides its zero.
        padded: Option<bool>,
    },
    Full,
    Short,
}

/// Which of the language's lists a month in words is written from.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Stands {
    /// As an example has it: alone where no day is beside it.
    ByTheDay,
    /// As inside a date, the CLDR's `M`.
    InADate,
    /// As with no day beside it, the CLDR's `L`.
    Alone,
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
        /// None where the format's own rule decides its zero.
        padded: Option<bool>,
    },
    Month {
        form: MonthForm,
        case: Case,
        stands: Stands,
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
    /// hasn't got it.
    fn unwritable(self, has_day: bool, words: &Words) -> Option<WordsWanted> {
        let missing = match self {
            Component::Weekday { full: true, .. } => words.weekdays.is_none(),
            Component::Weekday { full: false, .. } => words.weekdays_short.is_none(),
            Component::Month {
                form: MonthForm::Figures { .. },
                ..
            } => false,
            Component::Month { form, stands, .. } => {
                month_list(form, stands, has_day, words).is_none()
            }
            Component::AmPm { .. } => words.am_pm.is_none(),
            Component::Day { worded: true, .. } => words.days.is_none(),
            _ => false,
        };
        missing.then(|| wanted(self))
    }
}

/// The list a month in words is written from, where the language has it.
fn month_list(
    form: MonthForm,
    stands: Stands,
    has_day: bool,
    words: &Words,
) -> Option<&[String; 12]> {
    let alone = match stands {
        Stands::ByTheDay => !has_day,
        Stands::InADate => false,
        Stands::Alone => true,
    };
    match (form, alone) {
        (MonthForm::Full, false) => words.months.as_ref(),
        (MonthForm::Full, true) => words.months_with_no_day(),
        (MonthForm::Short, false) => words.months_short.as_ref(),
        (MonthForm::Short, true) => words.months_short_with_no_day(),
        (MonthForm::Figures { .. }, _) => None,
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

/// A date format read from an example, or from a look's pattern.
#[derive(Clone, Debug)]
pub struct Format {
    parts: Vec<Part>,
    words: DateWords,
    /// Whether it is one of the language's looks.
    of_a_look: bool,
}

impl Format {
    /// Reads an example a template hands the `date` filter. It may ask for
    /// any piece alone, but a day and a year with no month between them is
    /// an example whose month is in words the site hasn't got.
    pub fn for_a_template(example: &str, words: &DateWords) -> Result<Format, Refusal> {
        let format = Format::read(example, words).map_err(Refusal::offering_looks)?;
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
                    with_date: words.with_weekday(),
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
    /// long look.
    pub fn house(words: &DateWords) -> Format {
        Format::look(Look::Long, words).unwrap_or_else(|_| Format::unpatterned(Look::Long, words))
    }

    /// `look` as the language's Unicode CLDR pattern writes it, or as
    /// [`Format::unpatterned`] does where Statac has no patterns for the
    /// language. A pattern wanting words the language hasn't got is
    /// refused.
    pub fn look(look: Look, words: &DateWords) -> Result<Format, Refusal> {
        let Some(patterns) = &words.words.dates else {
            return Ok(Format::unpatterned(look, words));
        };
        let parts = match look {
            Look::Long => pattern(&patterns.long),
            Look::Short => pattern(&patterns.short),
            Look::Day => pattern(&patterns.day),
            Look::Stamp => stamp(patterns),
        }
        .expect("Statac's own patterns are read whole");
        let format = Format {
            parts,
            words: words.clone(),
            of_a_look: true,
        };
        format.writable(look.name())?;
        Ok(format)
    }

    /// The Unicode CLDR pattern `written`, where `words` can write it.
    fn patterned(written: &str, words: &DateWords) -> Option<Format> {
        let format = Format {
            parts: pattern(written)?,
            words: words.clone(),
            of_a_look: true,
        };
        format.writable("").ok()?;
        Some(format)
    }

    /// Every look of a language Statac has no patterns for: the reference
    /// day with its month in the language's words, or in figures where it
    /// has none, the stamp with the time after it.
    fn unpatterned(look: Look, words: &DateWords) -> Format {
        let time = match look {
            Look::Stamp => " 15:04",
            _ => "",
        };
        Format::read(&format!("{}{time}", words.day()), words)
            .or_else(|_| Format::read(&format!("{YEAR_FIRST}{time}"), words))
            .expect("a date in figures needs no words")
    }

    /// Refuses a format writing a word from a list the language hasn't got,
    /// told of the whole of `example`.
    fn writable(&self, example: &str) -> Result<(), Refusal> {
        let has_day = self.writes(Piece::Day);
        for part in &self.parts {
            if let Part::Component(component) = part
                && let Some(list) = component.unwritable(has_day, &self.words.words)
            {
                let fault = FormatFault::NoWords {
                    list,
                    from: self.words.from.clone(),
                };
                return Err(whole(example, fault));
            }
        }
        Ok(())
    }

    /// The same pieces written in another language's `words`, the text
    /// between them as written in `example`. A piece written in words that
    /// language hasn't got is refused.
    pub fn in_words(&self, words: &DateWords, example: &str) -> Result<Format, Refusal> {
        let mut format = Format {
            parts: self.parts.clone(),
            words: words.clone(),
            of_a_look: self.of_a_look,
        };
        format.writable(example)?;
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
                    looks: false,
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
                    looks: false,
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
                    looks: false,
                },
            ));
        }
        let mut format = Format {
            parts,
            words: words.clone(),
            of_a_look: false,
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
            if let Part::Component(Component::Day { worded, .. }) = part {
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
        self.written(reading, true)
    }

    /// `reading` written in this format, its numbers in 0 to 9 unless
    /// `in_its_digits`.
    fn written(&self, reading: &Reading, in_its_digits: bool) -> String {
        let machine = self.for_a_machine();
        let zeros = self.words.zeros || machine;
        let written = Written {
            day_padded: zeros && self.day_padded(),
            zeros,
            has_day: self.writes(Piece::Day),
            words: &self.words.words,
            digits: match machine || !in_its_digits {
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
        self.write(&REFERENCE_DAY)
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
    /// day, in the order of the language's Unicode CLDR pattern where
    /// Statac has one, else the month standing alone before the year. A
    /// format writing its month in figures by example gets the one
    /// spelling of a month and a year read only one way.
    pub fn month_and_year(&self, year: i32, month: u8) -> String {
        let form = self.parts.iter().find_map(|part| match part {
            Part::Component(Component::Month { form, case, .. }) => Some((*form, *case)),
            _ => None,
        });
        if let Some(format) =
            form.and_then(|(form, case)| self.month_and_year_patterned(form, case))
        {
            return format.write(&first_day(year, month));
        }
        let words = &self.words.words;
        let at = usize::from(month).wrapping_sub(1);
        let named = form.and_then(|(form, case)| {
            let month = month_list(form, Stands::Alone, false, words)?.get(at)?;
            Some(cased(month, case))
        });
        let written = match named {
            Some(month) => format!("{month} {year}"),
            None => format!("{year:04}-{month:02}"),
        };
        in_digits(&written, self.digits())
    }

    /// The language's pattern for a month and a year, its month in `form`
    /// written in `case`: a look's own month in figures is written as the
    /// language writes a month in full.
    fn month_and_year_patterned(&self, form: MonthForm, case: Case) -> Option<Format> {
        let patterns = self.words.words.dates.as_ref()?;
        let written = match form {
            MonthForm::Full => &patterns.month_and_year,
            MonthForm::Short => &patterns.month_short_and_year,
            MonthForm::Figures { .. } if self.of_a_look => &patterns.month_and_year,
            MonthForm::Figures { .. } => return None,
        };
        let mut parts = pattern(written)?;
        for part in &mut parts {
            if let Part::Component(Component::Month { case: as_cased, .. }) = part {
                *as_cased = case;
            }
        }
        let format = Format {
            parts,
            words: self.words.clone(),
            of_a_look: true,
        };
        format.writable("").ok()?;
        Some(format)
    }

    /// A year alone, as the language's Unicode CLDR pattern writes it
    /// where Statac has one and this format isn't a machine's.
    pub fn year_alone(&self, year: i32) -> String {
        let patterned = match self.for_a_machine() {
            true => None,
            false => (self.words.words.dates.as_ref()).and_then(|p| pattern(&p.year)),
        };
        match patterned {
            Some(parts) => {
                let format = Format {
                    parts,
                    words: self.words.clone(),
                    of_a_look: true,
                };
                format.write(&first_day(year, 1))
            }
            None => in_digits(&year.to_string(), self.digits()),
        }
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
                    form: MonthForm::Figures { .. },
                    ..
                }),
                other_dash,
                Part::Component(Component::Day { worded: false, .. }),
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
            matches!(part, Part::Component(Component::Month { form, .. }) if !matches!(form, MonthForm::Figures { .. }))
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
            Component::Day { worded: false, .. }
                | Component::Month {
                    form: MonthForm::Figures { .. },
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
    /// Whether a day in figures keeps its zero, by an example's rule.
    day_padded: bool,
    /// Whether a day or a month in figures may keep its zero at all.
    zeros: bool,
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
            Component::Day { worded: true, .. } => {
                out.push_str(from(words.days.as_ref(), usize::from(at.day) - 1));
            }
            Component::Day {
                worded: false,
                padded,
            } => {
                let padded = padded.map_or(self.day_padded, |padded| padded && self.zeros);
                self.padded(out, u32::from(at.day), padded);
            }
            Component::Month {
                form: MonthForm::Figures { padded },
                ..
            } => {
                let padded = padded.is_none_or(|padded| padded) && self.zeros;
                self.padded(out, u32::from(at.month), padded);
            }
            Component::Month { form, case, stands } => {
                let list = month_list(form, stands, self.has_day, words);
                out.push_str(&cased(from(list, usize::from(at.month) - 1), case));
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
            Component::Hour {
                twelve: false,
                padded,
            } => self.padded(out, u32::from(hour), padded),
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
                looks,
                ..
            } => FormatFault::NoPattern {
                example: example(),
                reference,
                examples,
                looks,
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

// Reading a pattern of the Unicode CLDR's.

/// The parts of a CLDR date pattern, or none where it holds a letter
/// Statac doesn't read. Text between quotes is written as it is, and two
/// quotes together are one.
fn pattern(pattern: &str) -> Option<Vec<Part>> {
    let mut parts = Vec::new();
    let mut literal = String::new();
    let mut rest = pattern;
    while let Some(first) = rest.chars().next() {
        if first == '\'' {
            rest = &rest[1..];
            if let Some(after) = rest.strip_prefix('\'') {
                literal.push('\'');
                rest = after;
                continue;
            }
            loop {
                let end = rest.find('\'')?;
                literal.push_str(&rest[..end]);
                rest = &rest[end + 1..];
                match rest.strip_prefix('\'') {
                    Some(after) => {
                        literal.push('\'');
                        rest = after;
                    }
                    None => break,
                }
            }
            continue;
        }
        if !first.is_ascii_alphabetic() {
            literal.push(first);
            rest = &rest[first.len_utf8()..];
            continue;
        }
        let run = rest.chars().take_while(|letter| *letter == first).count();
        rest = &rest[run..];
        let padded = Some(run == 2);
        let month = |form, stands| Component::Month {
            form,
            case: Case::Kept,
            stands,
        };
        let component = match (first, run) {
            ('d', 1 | 2) => Component::Day {
                worded: false,
                padded,
            },
            ('M', 1 | 2) | ('L', 1 | 2) => month(MonthForm::Figures { padded }, Stands::InADate),
            ('M', 3) => month(MonthForm::Short, Stands::InADate),
            ('M', 4) => month(MonthForm::Full, Stands::InADate),
            ('L', 3) => month(MonthForm::Short, Stands::Alone),
            ('L', 4) => month(MonthForm::Full, Stands::Alone),
            ('y', 2) => Component::Year { short: true },
            ('y', _) => Component::Year { short: false },
            ('E', 1..=3) => Component::Weekday {
                full: false,
                case: Case::Kept,
            },
            ('E', 4) => Component::Weekday {
                full: true,
                case: Case::Kept,
            },
            ('h', 1 | 2) => Component::Hour {
                twelve: true,
                padded: run == 2,
            },
            ('H', 1 | 2) => Component::Hour {
                twelve: false,
                padded: run == 2,
            },
            ('m', 1 | 2) => Component::Minute { padded: run == 2 },
            ('a', 1..=3) => Component::AmPm {
                dots: false,
                case: Case::Kept,
            },
            _ => return None,
        };
        if !literal.is_empty() {
            parts.push(Part::Literal(std::mem::take(&mut literal)));
        }
        parts.push(Part::Component(component));
    }
    if !literal.is_empty() {
        parts.push(Part::Literal(literal));
    }
    Some(parts)
}

/// The parts of the stamp: the medium date and the short time put
/// together as the language does. The text joining them stays apart from
/// the date's own, so a date with no time keeps all of its own.
fn stamp(patterns: &Patterns) -> Option<Vec<Part>> {
    let mut parts = Vec::new();
    let mut rest = patterns.date_and_time.as_str();
    loop {
        let date = rest.find("{1}").map(|at| (at, &patterns.short));
        let time = rest.find("{0}").map(|at| (at, &patterns.time));
        let next = [date, time].into_iter().flatten().min_by_key(|(at, _)| *at);
        let Some((at, inside)) = next else {
            parts.extend(pattern(rest)?);
            return Some(parts);
        };
        parts.extend(pattern(&rest[..at])?);
        parts.extend(pattern(inside)?);
        rest = &rest[at + "{0}".len()..];
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

/// Runs of figures, runs of letters, and each other character alone. A
/// run of letters is parted into the `pieces` it is wholly written in.
fn runs(s: &str, pieces: &[(&str, Joins)]) -> Vec<(Range<usize>, Class)> {
    let mut runs: Vec<(Range<usize>, Class)> = Vec::new();
    for (at, c) in s.char_indices() {
        let class = class(c);
        let end = at + c.len_utf8();
        match runs.last_mut() {
            Some((span, last)) if *last == class && class != Class::Other => span.end = end,
            _ => runs.push((at..end, class)),
        }
    }
    let mut parted = Vec::with_capacity(runs.len());
    let mut after_figures = false;
    for (span, class) in runs {
        let mut start = span.start;
        if class == Class::Letter {
            for at in partings(&s[span.clone()], pieces, after_figures) {
                parted.push((start..span.start + at, class));
                start = span.start + at;
            }
        }
        parted.push((start..span.end, class));
        after_figures = class == Class::Figure;
    }
    parted
}

/// The letters the language's Unicode CLDR patterns write between pieces,
/// as `日` in `y年M月d日EEEE`.
fn marks(words: &Words) -> Vec<String> {
    let Some(p) = &words.dates else {
        return Vec::new();
    };
    let patterns = [
        &p.full,
        &p.long,
        &p.short,
        &p.figures,
        &p.day,
        &p.time,
        &p.date_and_time,
        &p.month_and_year,
        &p.month_short_and_year,
        &p.year,
    ];
    let mut marks: Vec<String> = Vec::new();
    for part in patterns.into_iter().filter_map(|p| pattern(p)).flatten() {
        let Part::Literal(text) = part else {
            continue;
        };
        for (span, class) in runs(&text, &[]) {
            if class == Class::Letter && !marks.iter().any(|mark| *mark == text[span.clone()]) {
                marks.push(text[span].to_owned());
            }
        }
    }
    marks
}

/// How a word a run of letters may be parted into meets the word beside
/// it.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Joins {
    /// One of the language's patterns' marks, as `日`.
    Mark,
    /// A word of two letters or more in a script written without spaces
    /// between words, as `水曜日` and `午後`.
    Unspaced,
    Word,
}

impl Joins {
    const ALL: [Joins; 3] = [Joins::Mark, Joins::Unspaced, Joins::Word];

    fn of(word: &str, mark: bool) -> Joins {
        let unspaced =
            word.chars().nth(1).is_some() && word.chars().all(crate::markdown::runs_without_spaces);
        match (mark, unspaced) {
            (true, _) => Joins::Mark,
            (false, true) => Joins::Unspaced,
            (false, false) => Joins::Word,
        }
    }

    /// Whether letters part between a word joining so and the `next`: at
    /// a mark, and between two words of a script written without spaces.
    /// A single letter of such a script stays in the word it is written
    /// in, as `水` is in `水木`.
    fn parts_from(self, next: Joins) -> bool {
        matches!(
            (self, next),
            (Joins::Mark, _) | (_, Joins::Mark) | (Joins::Unspaced, Joins::Unspaced)
        )
    }
}

/// The words a run of letters may be parted into, the language's own and
/// its patterns' marks, each with how it joins the word beside it.
fn pieces<'a>(listed: &[(&'a str, Listed)], marks: &'a [String]) -> Vec<(&'a str, Joins)> {
    let own = listed
        .iter()
        .filter(|(_, list)| !matches!(list, Listed::Unworded(_)))
        .map(|(word, _)| *word);
    let mut pieces: Vec<(&str, Joins)> = Vec::new();
    for word in own.chain(marks.iter().map(String::as_str)) {
        if !word.is_empty() && !pieces.iter().any(|(piece, _)| *piece == word) {
            let mark = marks.iter().any(|mark| mark == word);
            pieces.push((word, Joins::of(word, mark)));
        }
    }
    pieces
}

/// Where a run of letters parts into words: nowhere, unless it is wholly
/// `pieces` and parts between each two, as `日水曜日` is the day's `日`
/// and the weekday `水曜日`, and `水曜日午後` the weekday and the
/// afternoon. Where it parts more than one way, the longest piece comes
/// first. In a script with spaces between words, letters with no mark
/// between them stay one word, so Arabic's `م`, its afternoon, is never
/// parted out of a longer word.
///
/// A run of Han, kana or Hangul straight `after_figures` that doesn't
/// part wholly still parts after the mark it opens with, the figures' own,
/// as `日` in `31日水曜目`, so the word after it is read alone. Where no
/// mark of the language's patterns opens it, as with a site's own words,
/// which carry no patterns, its first letter is taken as the figures' own
/// where the letters after it part wholly or are a slip from one of the
/// language's words: `日` in `31日星期三`, and `号` in `31号星期三`.
fn partings(run: &str, pieces: &[(&str, Joins)], after_figures: bool) -> Vec<usize> {
    if let Some(partings) = wholly(run, pieces) {
        return partings;
    }
    if !after_figures || !syllabic(run) {
        return Vec::new();
    }
    let mark = (pieces.iter())
        .filter(|(_, joins)| *joins == Joins::Mark)
        .filter_map(|(mark, _)| begins_with(run, mark))
        .max();
    let first = run.chars().next().map_or(0, char::len_utf8);
    let end = mark.unwrap_or(first);
    let rest = &run[end..];
    if rest.is_empty() {
        return Vec::new();
    }
    if mark.is_some() {
        return vec![end];
    }
    if let Some(after) = wholly(rest, pieces) {
        return std::iter::once(end)
            .chain(after.into_iter().map(|at| end + at))
            .collect();
    }
    let slip = pieces.iter().any(|(word, _)| {
        syllabic(word) && word.chars().count() >= 3 && crate::suggest::is_one_slip(rest, word)
    });
    match slip {
        true => vec![end],
        false => Vec::new(),
    }
}

/// Where `run` parts, if it is wholly `pieces` parting between each two.
fn wholly(run: &str, pieces: &[(&str, Joins)]) -> Option<Vec<usize>> {
    let pieces_at = |from: usize| {
        pieces.iter().filter_map(move |(piece, joins)| {
            begins_with(&run[from..], piece).map(|len| (from + len, *joins))
        })
    };
    let parts = |from: usize, before: Joins, joins: Joins| from == 0 || before.parts_from(joins);
    // Whether the letters from each point on part wholly, after a piece
    // joining each way.
    let mut whole = vec![[false; Joins::ALL.len()]; run.len() + 1];
    whole[run.len()] = [true; Joins::ALL.len()];
    for from in (0..run.len())
        .rev()
        .filter(|from| run.is_char_boundary(*from))
    {
        for before in Joins::ALL {
            whole[from][before as usize] = pieces_at(from)
                .any(|(end, joins)| parts(from, before, joins) && whole[end][joins as usize]);
        }
    }
    if !whole[0][Joins::Mark as usize] {
        return None;
    }
    let mut partings = Vec::new();
    let (mut from, mut before) = (0, Joins::Mark);
    while from < run.len() {
        let (end, joins) = pieces_at(from)
            .filter(|(end, joins)| parts(from, before, *joins) && whole[*end][*joins as usize])
            .max_by_key(|(end, _)| *end)
            .expect("a run that parts wholly parts at each step");
        if end < run.len() {
            partings.push(end);
        }
        (from, before) = (end, joins);
    }
    Some(partings)
}

/// Whether every letter of `word` is Han, kana or Hangul, each a syllable
/// or more.
fn syllabic(word: &str) -> bool {
    word.chars()
        .all(|c| crate::markdown::runs_without_spaces(c) || matches!(c, '\u{ac00}'..='\u{d7af}'))
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
fn named_at(
    s: &str,
    at: usize,
    listed: &[(&str, Listed)],
    partings: &[usize],
) -> Option<(usize, Named)> {
    let rest = &s[at..];
    let parted = |len: usize| partings.contains(&(at + len));
    let mut longest: Option<(usize, &str, Listed, bool)> = None;
    for (word, list) in listed {
        let clock = matches!(list, Listed::AmPm | Listed::Unworded(WordsWanted::AmPm));
        let found = spelled(rest, word, parted)
            .map(|len| (len, false))
            .or_else(|| {
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

/// How far `word` runs at the start of `rest`, in any case.
fn begins_with(rest: &str, word: &str) -> Option<usize> {
    let mut len = 0;
    let mut chars = rest.chars();
    for wanted in word.chars() {
        let c = chars.next()?;
        if !same_letter(c, wanted) {
            return None;
        }
        len += c.len_utf8();
    }
    Some(len)
}

/// How far `word` runs at the start of `rest`, in any case, where it ends
/// there: a word ending in a letter isn't the start of a longer word, save
/// where the letters are `parted` after it, nor one ending in a figure the
/// start of a longer number.
fn spelled(rest: &str, word: &str, parted: impl Fn(usize) -> bool) -> Option<usize> {
    let len = begins_with(rest, word)?;
    let last = word.chars().last()?;
    let ends = match rest[len..].chars().next() {
        None => true,
        Some(_) if parted(len) => true,
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
    let marks = marks(words);
    let runs = runs(s, &pieces(&listed, &marks));
    let partings: Vec<usize> = runs
        .windows(2)
        .filter(|pair| pair.iter().all(|(_, class)| *class == Class::Letter))
        .map(|pair| pair[1].0.start)
        .collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < runs.len() {
        let (span, class) = runs[i].clone();
        // Another weekday's word straight after figures is the language's
        // own word for them, as Korean writes the day `31일`.
        let after_figures =
            i > 0 && runs[i - 1].1 == Class::Figure && runs[i - 1].0.end == span.start;
        if class != Class::Other
            && let Some((end, named)) = named_at(s, span.start, &listed, &partings)
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
            Named::Month { form, case } => Resolution::Component(Component::Month {
                form,
                case,
                stands: Stands::ByTheDay,
            }),
            Named::AmPm { dots, case } => Resolution::Component(Component::AmPm { dots, case }),
            Named::Day => Resolution::Component(Component::Day {
                worded: true,
                padded: None,
            }),
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
            // A slip in another weekday's or month's word is refused as
            // the word itself is, the reference moment's winning a tie.
            let candidates: Vec<(&str, Meant)> = known_words(words)
                .map(|word| (word, Meant::Reference))
                .chain(other_names(words))
                .collect();
            // A letter of Han, kana or Hangul is a syllable or more, so one
            // slip from a word of three is a typo however short the word
            // written, where one from a word of two leaves half of it.
            let slipped = if syllabic(text) {
                candidates.iter().find(|(word, _)| {
                    syllabic(word)
                        && word.chars().count() >= 3
                        && crate::suggest::is_one_slip(text, word)
                })
            } else if text.chars().count() < 4 {
                // A short word near a known one is left alone: "on" is a
                // letter from "Mon".
                None
            } else {
                crate::suggest::nearest(text, candidates.iter().map(|(word, _)| *word))
                    .and_then(|nearest| candidates.iter().find(|(word, _)| *word == nearest))
            };
            match slipped {
                Some((word, Meant::Reference)) => Resolution::Typo((*word).to_owned()),
                Some((_, Meant::Weekday { full })) => Resolution::OtherWeekday { full: *full },
                Some((_, Meant::Month { list })) => Resolution::OtherMonth { list: *list },
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
        DAY => Component::Day {
            worded: false,
            padded: None,
        },
        MONTH => Component::Month {
            form: MonthForm::Figures { padded: None },
            case: Case::Lower,
            stands: Stands::ByTheDay,
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
            stands: Stands::ByTheDay,
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

/// Which word a slip was meant for.
enum Meant {
    /// The reference moment's weekday or month.
    Reference,
    Weekday {
        full: bool,
    },
    Month {
        list: MonthList,
    },
}

/// Every other weekday's and month's word in the language's lists.
fn other_names(words: &Words) -> impl Iterator<Item = (&str, Meant)> {
    let weekdays = [(&words.weekdays, true), (&words.weekdays_short, false)]
        .into_iter()
        .flat_map(|(list, full)| {
            list.iter()
                .flat_map(|names| names.iter().enumerate())
                .filter(|(day, _)| *day != WEEKDAY)
                .map(move |(_, name)| (name.as_str(), Meant::Weekday { full }))
        });
    let months = MonthList::ALL.into_iter().flat_map(move |list| {
        list.in_words(words)
            .into_iter()
            .flat_map(|names| names.iter().enumerate())
            .filter(|(month, _)| *month != MONTH_LISTED)
            .map(move |(_, name)| (name.as_str(), Meant::Month { list }))
    });
    weekdays.chain(months)
}
