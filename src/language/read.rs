//! A language file read into its words, every refusal placed at what was
//! written.

use crate::origin::Sourced;
use crate::problem::{Expected, Found, Kind, Problem, Word, WordList};
use crate::source::{Place, SourceId, Sources, Span};
use crate::value::{Reader, place_within};
use crate::yaml::{Node, Resolved, Value as Yaml};

use super::{Direction, Patterns, Quotes, Words};

/// Every key a language file reads.
pub(crate) const KEYS: &[&str] = &[
    "months",
    "months_alone",
    "months_short",
    "months_short_alone",
    "weekdays",
    "weekdays_short",
    "days",
    "am_pm",
    "digits",
    "quotes",
    "no_break_before",
    "no_break_after",
    "inside_quotes",
    "direction",
    "back_to_the_text",
    "callouts",
];

/// The keys of the `dates` of Statac's own files, each a pattern.
const PATTERNS: &[&str] = &[
    "full",
    "long",
    "short",
    "figures",
    "day",
    "time",
    "date_and_time",
    "month_and_year",
    "month_short_and_year",
    "year",
];

/// Reads the language file `source`. A key whose value can't be read is
/// left out of the words, with the problem said.
#[cfg(test)]
pub(crate) fn read(sources: &Sources, source: SourceId) -> (Words, Vec<Problem>) {
    read_onto(sources, source, Words::default())
}

/// Reads the language file `source` over `words`: a key it writes takes
/// the place of theirs, with nothing after its colon as much as with a
/// value, and a key it leaves out, or whose value can't be read, keeps
/// theirs.
pub(crate) fn read_onto(
    sources: &Sources,
    source: SourceId,
    words: Words,
) -> (Words, Vec<Problem>) {
    read_file(sources, source, words, false)
}

/// Reads one of Statac's own files over `words`, as [`read_onto`] does,
/// and its `dates` too, pattern by pattern.
pub(crate) fn read_statacs(
    sources: &Sources,
    source: SourceId,
    words: Words,
) -> (Words, Vec<Problem>) {
    read_file(sources, source, words, true)
}

fn read_file(
    sources: &Sources,
    source: SourceId,
    mut words: Words,
    statacs: bool,
) -> (Words, Vec<Problem>) {
    let mut problems = Vec::new();
    let node = match crate::yaml::read(sources, source) {
        Ok(node) => node,
        Err(problem) => return (words, vec![problem]),
    };
    let entries = match &node.value {
        Yaml::Mapping(entries) => entries,
        Yaml::Scalar(_) if node.resolved() == Some(Resolved::Null) => return (words, problems),
        _ => {
            let path = sources.path(source).to_string_lossy().into_owned();
            let reader = Reader::new(sources, &path);
            return (words, vec![reader.wrong(&node, Expected::Fields, None)]);
        }
    };
    for entry in entries {
        let key = crate::content::words::key_name(&entry.key);
        let file = File {
            sources,
            key: &key,
            value: &entry.value,
        };
        let read = match key.as_str() {
            "months" => file.list(WordList::Months).map(|l| words.months = l),
            "months_alone" => file.list(WordList::Months).map(|l| words.months_alone = l),
            "months_short" => file.list(WordList::Months).map(|l| words.months_short = l),
            "months_short_alone" => file
                .list(WordList::Months)
                .map(|l| words.months_short_alone = l),
            "weekdays" => file.list(WordList::Weekdays).map(|l| words.weekdays = l),
            "weekdays_short" => file
                .list(WordList::Weekdays)
                .map(|l| words.weekdays_short = l),
            "days" => file.list(WordList::Days).map(|l| words.days = l),
            "am_pm" => file.list(WordList::AmPm).map(|l| words.am_pm = l),
            "digits" => file.digits().map(|d| words.digits = d),
            "quotes" => file.quotes().map(|q| words.quotes = q),
            "no_break_before" => file
                .marks()
                .map(|m| words.no_break_before = m.unwrap_or_default()),
            "no_break_after" => file
                .marks()
                .map(|m| words.no_break_after = m.unwrap_or_default()),
            "inside_quotes" => file.space().map(|s| words.inside_quotes = s),
            "direction" => file.direction().map(|d| words.direction = d),
            "back_to_the_text" => file.text().map(|t| words.back_to_the_text = t),
            "callouts" => file.list(WordList::Callouts).map(|l| words.callouts = l),
            "dates" if statacs => file
                .patterns(words.dates.take().unwrap_or_default())
                .map(|p| words.dates = Some(p)),
            _ => Err(vec![Problem::at(
                Kind::UnknownWord {
                    word: Word::Setting,
                    written: key.clone(),
                    nearest: crate::suggest::nearest(&key, KEYS.iter().copied()).map(str::to_owned),
                },
                entry.key.place,
            )]),
        };
        if let Err(found) = read {
            problems.extend(found);
        }
    }
    (words, problems)
}

/// One key of a language file and its value.
struct File<'a> {
    sources: &'a Sources,
    key: &'a str,
    value: &'a Node,
}

type Read<T> = Result<T, Vec<Problem>>;

impl File<'_> {
    fn reader(&self) -> Reader<'_> {
        Reader::new(self.sources, self.key)
    }

    fn wrong(&self, place: Place, expected: Expected, found: Found) -> Problem {
        Problem::at(
            Kind::WrongKind {
                field: self.key.to_owned(),
                expected,
                found,
                respelt: None,
            },
            place,
        )
    }

    /// Where an item of the value was written.
    fn place_of(&self, item: &Sourced<String>) -> Place {
        item.origin.place.unwrap_or(self.value.place)
    }

    fn text(&self) -> Read<Option<String>> {
        let text = self.reader().text(self.value).map_err(|p| vec![p])?;
        Ok(text
            .map(|t| t.value.trim().to_owned())
            .filter(|t| !t.is_empty()))
    }

    /// A list of exactly as many words as `list` holds. A list with two
    /// months or two digits alike can't be read back, so it is refused.
    fn list<const N: usize>(&self, list: WordList) -> Read<Option<[String; N]>> {
        let Some(items) = self.items(list)? else {
            return Ok(None);
        };
        if list == WordList::Months
            && let Some(problem) = self.alike(&items, |a, b| a.to_lowercase() == b.to_lowercase())
        {
            return Err(vec![problem]);
        }
        let words: Vec<String> = items.into_iter().map(|item| item.value).collect();
        Ok(Some(words.try_into().expect("counted")))
    }

    /// The list's items, as many as `list` holds.
    fn items(&self, list: WordList) -> Read<Option<Vec<Sourced<String>>>> {
        let reader = self.reader();
        let Some(items) = reader.texts(self.value).map_err(|p| vec![p])? else {
            return Ok(None);
        };
        let items = items.value;
        let wanted = count(list);
        if items.len() == wanted {
            return Ok(Some(items));
        }
        // A list written without its brackets is one value, its commas inside it.
        let respelt = match (&self.value.value, items.as_slice()) {
            (Yaml::Scalar(_), [one]) if one.value.split(',').count() == wanted => {
                Some(format!("[{}]", one.value))
            }
            _ => None,
        };
        Err(vec![Problem::at(
            Kind::WrongKind {
                field: self.key.to_owned(),
                expected: Expected::Words(list),
                found: Found::Items(items.len()),
                respelt: respelt.map(String::into_boxed_str),
            },
            self.value.place,
        )])
    }

    /// The second of two items alike, refused at its place.
    fn alike(
        &self,
        items: &[Sourced<String>],
        same: impl Fn(&str, &str) -> bool,
    ) -> Option<Problem> {
        let (later, item) = items.iter().enumerate().find(|(at, item)| {
            items[..*at]
                .iter()
                .any(|earlier| same(&earlier.value, &item.value))
        })?;
        let list = match self.key {
            "digits" => WordList::Digits,
            _ => WordList::Months,
        };
        Some(self.wrong(
            self.place_of(&items[later]),
            Expected::Words(list),
            Found::Twice(item.value.clone()),
        ))
    }

    /// Ten digits, zero first, each one character and no two alike.
    fn digits(&self) -> Read<Option<[char; 10]>> {
        let Some(items) = self.items(WordList::Digits)? else {
            return Ok(None);
        };
        let mut problems: Vec<Problem> = items
            .iter()
            .filter(|item| item.value.chars().count() != 1)
            .map(|item| {
                self.wrong(
                    self.place_of(item),
                    Expected::OneCharacter,
                    Found::Written(item.value.clone()),
                )
            })
            .collect();
        if problems.is_empty() {
            problems.extend(self.alike(&items, |a, b| a == b));
        }
        if !problems.is_empty() {
            return Err(problems);
        }
        let digits: Vec<char> = items
            .iter()
            .map(|item| item.value.chars().next().expect("one character"))
            .collect();
        Ok(Some(digits.try_into().expect("counted")))
    }

    /// Marks written one after another with spaces between, each one
    /// character.
    fn marks(&self) -> Read<Option<Vec<char>>> {
        let reader = self.reader();
        let Some(text) = reader.text(self.value).map_err(|p| vec![p])? else {
            return Ok(None);
        };
        let mut marks = Vec::new();
        let mut problems = Vec::new();
        for (at, mark) in pieces(&text.value) {
            let mut chars = mark.chars();
            match (chars.next(), chars.next()) {
                (Some(one), None) => marks.push(one),
                _ => {
                    let span = Span::new(at, at + mark.len());
                    let place = place_within(self.sources, &text.origin, &text.value, span)
                        .unwrap_or(self.value.place);
                    problems.push(self.wrong(
                        place,
                        Expected::OneCharacter,
                        Found::Written(mark.to_owned()),
                    ));
                }
            }
        }
        match problems.is_empty() {
            true => Ok(Some(marks)),
            false => Err(problems),
        }
    }

    /// Four marks: the outer pair, then the inner pair.
    fn quotes(&self) -> Read<Option<Quotes>> {
        let Some(marks) = self.marks()? else {
            return Ok(None);
        };
        match marks.as_slice() {
            &[open, close, inner_open, inner_close] => Ok(Some(Quotes {
                open,
                close,
                inner_open,
                inner_close,
            })),
            _ => Err(vec![self.wrong(
                self.value.place,
                Expected::Words(WordList::Quotes),
                Found::Items(marks.len()),
            )]),
        }
    }

    /// One space, of whatever width.
    fn space(&self) -> Read<Option<char>> {
        let reader = self.reader();
        let Some(text) = reader.text(self.value).map_err(|p| vec![p])? else {
            return Ok(None);
        };
        let mut chars = text.value.chars();
        match (chars.next(), chars.next()) {
            (Some(one), None) if is_a_space(one) => Ok(Some(one)),
            _ => Err(vec![reader.wrong(self.value, Expected::Space, None)]),
        }
    }

    /// Patterns by name, each over the one in `onto`.
    fn patterns(&self, mut onto: Patterns) -> Read<Patterns> {
        let Yaml::Mapping(entries) = &self.value.value else {
            return Err(vec![self.reader().wrong(
                self.value,
                Expected::Fields,
                None,
            )]);
        };
        let mut problems = Vec::new();
        for entry in entries {
            let key = crate::content::words::key_name(&entry.key);
            let reader = Reader::new(self.sources, &key);
            let pattern = match key.as_str() {
                "full" => &mut onto.full,
                "long" => &mut onto.long,
                "short" => &mut onto.short,
                "figures" => &mut onto.figures,
                "day" => &mut onto.day,
                "time" => &mut onto.time,
                "date_and_time" => &mut onto.date_and_time,
                "month_and_year" => &mut onto.month_and_year,
                "month_short_and_year" => &mut onto.month_short_and_year,
                "year" => &mut onto.year,
                _ => {
                    problems.push(Problem::at(
                        Kind::UnknownWord {
                            word: Word::Setting,
                            written: key.clone(),
                            nearest: crate::suggest::nearest(&key, PATTERNS.iter().copied())
                                .map(str::to_owned),
                        },
                        entry.key.place,
                    ));
                    continue;
                }
            };
            match reader.text(&entry.value) {
                Ok(Some(text)) => *pattern = text.value,
                Ok(None) => problems.push(reader.wrong(&entry.value, Expected::Text, None)),
                Err(problem) => problems.push(problem),
            }
        }
        match problems.is_empty() {
            true => Ok(onto),
            false => Err(problems),
        }
    }

    fn direction(&self) -> Read<Direction> {
        let reader = self.reader();
        let words = ["ltr", "rtl"];
        match reader.one_of(self.value, &words).map_err(|p| vec![p])? {
            Some(text) if text.value == "rtl" => Ok(Direction::RightToLeft),
            _ => Ok(Direction::LeftToRight),
        }
    }
}

fn count(list: WordList) -> usize {
    match list {
        WordList::Months => 12,
        WordList::Weekdays => 7,
        WordList::Days => 31,
        WordList::AmPm => 2,
        WordList::Digits => 10,
        WordList::Quotes => 4,
        WordList::Callouts => 5,
    }
}

/// The runs of `text` between whitespace, each with where it starts.
fn pieces(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.split_whitespace()
        .map(move |piece| (piece.as_ptr() as usize - text.as_ptr() as usize, piece))
}

/// The characters Unicode parts words with, wide, narrow and unbreakable.
fn is_a_space(c: char) -> bool {
    matches!(
        c,
        ' ' | '\u{a0}' | '\u{1680}' | '\u{202f}' | '\u{205f}' | '\u{3000}'
    ) || ('\u{2000}'..='\u{200a}').contains(&c)
}
