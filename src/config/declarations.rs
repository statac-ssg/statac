//! The author's own fields, typed by their declarations in `statac.yaml`:
//! `fields.isbn: text`, and `types.book.isbn: required text` for the pages
//! whose `type` is `book`. One field has one kind wherever it is declared.
//! A dotted name declares a nested field: `fields.book.author: text` is
//! `author:` written beneath `book:`.
//!
//! `one of data.people` and `one of folder.books` hold a field to the names
//! of a table in `data/` or of the pages in a folder. Which names those are
//! is known only once the site is read, so such a value is held to them
//! there.

use std::collections::BTreeMap;

use crate::problem::{Expected, Kind, Problem, Word};
use crate::source::{Place, Sources};
use crate::value::Reader;
use crate::yaml::{Node, Value as Yaml};

/// What a declared field holds.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum FieldKind {
    Text,
    Number,
    TrueFalse,
    List,
    Url,
    Date,
    /// A date, or a year and month, or a year.
    When,
    OneOf(Vec<String>),
    /// One of the names something in the site holds.
    Named(Names),
}

/// What holds the names a field's value must be one of.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Names {
    /// A table in `data/`, by its name after `data.`: `people`.
    Data(String),
    /// A folder of pages, by its path after `folder.`: `books`.
    Folder(String),
}

/// The kinds as the author writes them.
pub(crate) const KIND_WORDS: &[&str] = &[
    "text",
    "number",
    "true/false",
    "list",
    "url",
    "date",
    "when",
    "one of",
];

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Declared {
    pub(crate) kind: FieldKind,
    /// Where the kind is written.
    pub(crate) place: Place,
    /// The declaration's name as written: `types.review.book`.
    pub(crate) written: String,
}

#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub(crate) struct Declarations {
    fields: BTreeMap<String, Declared>,
    /// Fields every page must carry.
    everywhere: Vec<String>,
    /// Each type declared, with the fields its pages must carry.
    types: BTreeMap<String, Vec<String>>,
}

impl Declarations {
    pub(crate) fn kind(&self, field: &str) -> Option<&FieldKind> {
        self.fields.get(field).map(|declared| &declared.kind)
    }

    /// How the field `name` is declared, where it is.
    pub(crate) fn declared(&self, field: &str) -> Option<&Declared> {
        self.fields.get(field)
    }

    /// Every field declared, by name.
    pub(crate) fn names(&self) -> impl Iterator<Item = &str> {
        self.fields.keys().map(String::as_str)
    }

    /// Every field declared to be one of the names something holds, by
    /// name.
    pub(crate) fn naming(&self) -> impl Iterator<Item = (&str, &Declared, &Names)> {
        self.fields
            .iter()
            .filter_map(|(name, declared)| match &declared.kind {
                FieldKind::Named(names) => Some((name.as_str(), declared, names)),
                _ => None,
            })
    }

    /// The field `name` and the fields nested inside `value`, each
    /// declared to be one of the names something holds: its name as
    /// declared, with what is written for it.
    pub(crate) fn naming_in<'a>(&'a self, name: &str, value: &'a Node) -> Vec<(&'a str, &'a Node)> {
        let mut found = Vec::new();
        if let Some((declared, _)) = (self.fields.get_key_value(name))
            .filter(|(_, declared)| matches!(declared.kind, FieldKind::Named(_)))
        {
            found.push((declared.as_str(), value));
        }
        if let Yaml::Mapping(entries) = &value.value
            && self.nests(name)
        {
            for entry in entries {
                let path = format!("{name}.{}", crate::yaml::key_name(&entry.key));
                found.extend(self.naming_in(&path, &entry.value));
            }
        }
        found
    }

    /// Every type a field is declared for, by name.
    pub(crate) fn types(&self) -> impl Iterator<Item = &str> {
        self.types.keys().map(String::as_str)
    }

    /// Whether a field is declared nested under `name`, as `book.author`
    /// is under `book`.
    pub(crate) fn nests(&self, name: &str) -> bool {
        let prefix = format!("{name}.");
        self.fields
            .range(prefix.clone()..)
            .next()
            .is_some_and(|(declared, _)| declared.starts_with(&prefix))
    }

    /// The fields a page of `type` must carry, and whether each is required
    /// of every page or of the type.
    pub(crate) fn required<'a>(
        &'a self,
        page_type: Option<&'a str>,
    ) -> impl Iterator<Item = (&'a str, Option<&'a str>)> {
        let typed = page_type
            .and_then(|name| self.types.get(name).map(|fields| (name, fields)))
            .into_iter()
            .flat_map(|(name, fields)| fields.iter().map(move |f| (f.as_str(), Some(name))));
        self.everywhere
            .iter()
            .map(|f| (f.as_str(), None))
            .chain(typed)
    }

    /// Reads `fields.<name>: [required] <kind>`, or with `type` from
    /// `types.<type>.<name>`.
    pub(crate) fn declare(
        &mut self,
        sources: &Sources,
        key: &Node,
        written_key: &str,
        name: &str,
        page_type: Option<&str>,
        value: &Node,
    ) -> Vec<Problem> {
        if let Some(page_type) = page_type {
            self.types.entry(page_type.to_owned()).or_default();
        }
        let head = name.split('.').next().unwrap_or(name);
        if name.is_empty() || crate::config::words::is_statacs(head) {
            return vec![Problem::at(
                Kind::ReservedName {
                    name: format!("page.{head}"),
                },
                key.place,
            )];
        }
        let reader = Reader::new(sources, written_key);
        let text = match reader.text(value) {
            Ok(Some(text)) => text,
            _ => return vec![reader.wrong(value, Expected::FieldKind, None)],
        };
        let (required, kind) = match read_kind(&text.value) {
            Ok(read) => read,
            Err(kind) => return vec![Problem::at(kind, value.place)],
        };
        let declared = Declared {
            kind,
            place: value.place,
            written: written_key.to_owned(),
        };
        match self.fields.get(name) {
            Some(first) if first.kind != declared.kind => {
                return vec![Problem::at(
                    Kind::DuplicateKey {
                        key: name.to_owned(),
                        first: first.place,
                    },
                    value.place,
                )];
            }
            Some(_) => {}
            None => {
                self.fields.insert(name.to_owned(), declared);
            }
        }
        if required {
            let list = match page_type {
                Some(page_type) => self.types.entry(page_type.to_owned()).or_default(),
                None => &mut self.everywhere,
            };
            if !list.iter().any(|f| f == name) {
                list.push(name.to_owned());
            }
        }
        Vec::new()
    }

    /// The problems with `value` for the field `name` and the fields
    /// declared nested inside it, each declared and not its kind.
    pub(crate) fn check_all(&self, sources: &Sources, name: &str, value: &Node) -> Vec<Problem> {
        let mut problems: Vec<Problem> = self.check(sources, name, value).into_iter().collect();
        if let Yaml::Mapping(entries) = &value.value
            && self.nests(name)
        {
            for entry in entries {
                let path = format!("{name}.{}", crate::yaml::key_name(&entry.key));
                problems.extend(self.check_all(sources, &path, &entry.value));
            }
        }
        problems
    }

    /// The problem with `value` for the field `name`, if it is declared and
    /// the value isn't its kind.
    pub(crate) fn check(&self, sources: &Sources, name: &str, value: &Node) -> Option<Problem> {
        let kind = self.kind(name)?;
        let reader = Reader::new(sources, name);
        let checked = match kind {
            // Which name it must be is held once the site is read.
            FieldKind::Text | FieldKind::Named(_) => reader.text(value).map(drop),
            FieldKind::Number => reader.number(value),
            FieldKind::TrueFalse => reader.truth(value).map(drop),
            FieldKind::List => reader.list(value),
            FieldKind::Url => reader.address(value).map(drop),
            FieldKind::Date => reader.date(value).map(drop),
            FieldKind::When => reader.when(value).map(drop),
            FieldKind::OneOf(words) => {
                let words: Vec<&str> = words.iter().map(String::as_str).collect();
                reader.one_of(value, &words).map(drop)
            }
        };
        checked.err()
    }
}

/// `[required] <kind>`, or the problem with it.
fn read_kind(written: &str) -> Result<(bool, FieldKind), Kind> {
    let trimmed = written.trim();
    let (required, rest) = match trimmed.strip_prefix("required") {
        Some(rest) if rest.starts_with(char::is_whitespace) => (true, rest.trim_start()),
        _ => (false, trimmed),
    };
    if let Some(words) = rest
        .strip_prefix("one of")
        .filter(|words| words.is_empty() || words.starts_with(char::is_whitespace))
    {
        let words: Vec<String> = words
            .split(',')
            .map(|w| w.trim().to_owned())
            .filter(|w| !w.is_empty())
            .collect();
        if words.is_empty() {
            return Err(unknown(rest, None));
        }
        if let [word] = words.as_slice()
            && let Some(names) = names(word)
        {
            return Ok((required, FieldKind::Named(names)));
        }
        return Ok((required, FieldKind::OneOf(words)));
    }
    let kind = match rest {
        "text" => FieldKind::Text,
        "number" => FieldKind::Number,
        "true/false" => FieldKind::TrueFalse,
        "list" => FieldKind::List,
        "url" => FieldKind::Url,
        "date" => FieldKind::Date,
        "when" => FieldKind::When,
        "yes/no" => {
            return Err(Kind::OtherSpelling {
                word: Word::FieldKind,
                written: rest.to_owned(),
                ours: "true/false".into(),
            });
        }
        _ => {
            let mut known: Vec<&str> = KIND_WORDS.to_vec();
            if !required {
                known.push("required");
            }
            let first = rest.split_whitespace().next().unwrap_or(rest);
            let nearest = crate::suggest::nearest(first, known).map(str::to_owned);
            return Err(unknown(rest, nearest));
        }
    };
    Ok((required, kind))
}

/// What `data.people` or `folder.books` names, standing alone after
/// `one of`; none for a word to be written, as `data` alone is.
fn names(word: &str) -> Option<Names> {
    let named = |prefix: &str| word.strip_prefix(prefix).filter(|rest| !rest.is_empty());
    if let Some(table) = named("data.") {
        return Some(Names::Data(table.to_owned()));
    }
    let folder = named("folder.")?.trim_matches('/');
    (!folder.is_empty()).then(|| Names::Folder(folder.to_owned()))
}

fn unknown(written: &str, nearest: Option<String>) -> Kind {
    Kind::UnknownWord {
        word: Word::FieldKind,
        written: written.to_owned(),
        nearest,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_are_read_with_or_without_required() {
        let cases = [
            ("text", false, FieldKind::Text),
            ("required url", true, FieldKind::Url),
            ("true/false", false, FieldKind::TrueFalse),
            ("number", false, FieldKind::Number),
            ("list", false, FieldKind::List),
            ("date", false, FieldKind::Date),
            ("when", false, FieldKind::When),
            (
                "one of a, b ,c",
                false,
                FieldKind::OneOf(vec!["a".into(), "b".into(), "c".into()]),
            ),
            (
                "required one of مقال, ملخص",
                true,
                FieldKind::OneOf(vec!["مقال".into(), "ملخص".into()]),
            ),
            (
                "one of data.people",
                false,
                FieldKind::Named(Names::Data("people".into())),
            ),
            (
                "required one of data.people.staff",
                true,
                FieldKind::Named(Names::Data("people.staff".into())),
            ),
            (
                "required one of folder.books",
                true,
                FieldKind::Named(Names::Folder("books".into())),
            ),
            (
                "one of  folder.books/shelf-1/",
                false,
                FieldKind::Named(Names::Folder("books/shelf-1".into())),
            ),
        ];
        for (written, required, kind) in cases {
            assert_eq!(read_kind(written), Ok((required, kind)), "{written}");
        }
    }

    #[test]
    fn a_name_among_words_or_with_nothing_after_its_dot_is_a_word() {
        let cases: [(&str, &[&str]); 7] = [
            ("one of data.people, film", &["data.people", "film"]),
            ("one of film, folder.books", &["film", "folder.books"]),
            ("one of data", &["data"]),
            ("one of data.", &["data."]),
            ("one of folder.", &["folder."]),
            ("one of folder./", &["folder./"]),
            ("one of Data.people", &["Data.people"]),
        ];
        for (written, words) in cases {
            let words = words.iter().map(|word| (*word).to_owned()).collect();
            assert_eq!(
                read_kind(written),
                Ok((false, FieldKind::OneOf(words))),
                "{written}"
            );
        }
    }

    #[test]
    fn an_unknown_kind_is_offered_the_nearest() {
        let cases = [
            ("txt", Some("text")),
            ("requird text", Some("required")),
            ("colour", None),
            ("one of", None),
        ];
        for (written, nearest) in cases {
            assert_eq!(
                read_kind(written),
                Err(unknown(written, nearest.map(Into::into))),
                "{written}"
            );
        }
        assert_eq!(
            read_kind("yes/no"),
            Err(Kind::OtherSpelling {
                word: Word::FieldKind,
                written: "yes/no".into(),
                ours: "true/false".into(),
            })
        );
    }
}
