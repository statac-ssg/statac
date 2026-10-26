//! Single values read from placed YAML into Statac's types, each with its
//! origin, and every refusal a typed problem at the value.
//!
//! A value means what YAML's standard reading makes of it. The one bend is
//! for text: a bare scalar YAML reads as something else is taken as typed
//! where that reading keeps its characters (`title: 1984`), and refused
//! with the quoted spelling offered where it would change them (`3.10`).

use crate::date::{self, Date, Fault, When};
use crate::origin::{Origin, Sourced, Style};
use crate::problem::{Expected, Found, Kind, Missing, Problem};
use crate::source::{Place, Sources, Span};
use crate::yaml::{Node, Resolved, Value as Yaml, keeps_characters};

/// A value of one of Statac's own words or settings.
#[derive(Clone, PartialEq, Debug)]
pub(crate) enum Value {
    Text(String),
    Bool(bool),
    Whole(u64),
    Texts(Vec<Sourced<String>>),
    Date(Date),
    /// Names and the words they are given, as `tags.slugs` holds them.
    Table(Vec<(Sourced<String>, Sourced<String>)>),
    /// Written, and refused with a problem already given: whatever it
    /// stands in for is not to be guessed at.
    Refused,
}

impl Value {
    pub(crate) fn as_text(&self) -> Option<&str> {
        match self {
            Value::Text(text) => Some(text),
            _ => None,
        }
    }

    pub(crate) fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub(crate) fn as_whole(&self) -> Option<u64> {
        match self {
            Value::Whole(n) => Some(*n),
            _ => None,
        }
    }

    pub(crate) fn as_texts(&self) -> Option<&[Sourced<String>]> {
        match self {
            Value::Texts(texts) => Some(texts),
            _ => None,
        }
    }

    pub(crate) fn as_date(&self) -> Option<&Date> {
        match self {
            Value::Date(date) => Some(date),
            _ => None,
        }
    }
}

/// Reads values for the field or setting named `field`.
pub(crate) struct Reader<'a> {
    pub(crate) sources: &'a Sources,
    pub(crate) field: &'a str,
    /// Whether the value is a secret, never to be quoted back.
    pub(crate) secret: bool,
}

type Read<T> = Result<Option<Sourced<T>>, Problem>;

impl<'a> Reader<'a> {
    pub(crate) fn new(sources: &'a Sources, field: &'a str) -> Self {
        Reader {
            sources,
            field,
            secret: false,
        }
    }

    /// Reads a value that holds a secret: a problem with it names the field
    /// and never quotes the value.
    pub(crate) fn secret(sources: &'a Sources, field: &'a str) -> Self {
        Reader {
            secret: true,
            ..Reader::new(sources, field)
        }
    }

    fn found(&self, node: &Node) -> Found {
        match &node.value {
            Yaml::Sequence(_) => Found::List,
            Yaml::Mapping(_) => Found::Mapping,
            Yaml::Scalar(_) if self.secret => Found::Hidden,
            Yaml::Scalar(_) => Found::Written(self.sources.spelling(node.place).to_owned()),
        }
    }

    pub(crate) fn wrong(
        &self,
        node: &Node,
        expected: Expected,
        respelt: Option<String>,
    ) -> Problem {
        Problem::at(
            Kind::WrongKind {
                field: self.field.to_owned(),
                expected,
                found: self.found(node),
                respelt: respelt.map(String::into_boxed_str),
            },
            node.place,
        )
    }

    fn sourced<T>(node: &Node, value: T) -> Sourced<T> {
        Sourced::new(value, node.origin())
    }

    /// Text by the rule above. No value is no text.
    pub(crate) fn text(&self, node: &Node) -> Read<String> {
        let Yaml::Scalar(text) = &node.value else {
            return Err(self.wrong(node, Expected::Text, None));
        };
        match node.resolved() {
            Some(Resolved::Null) => Ok(None),
            Some(Resolved::Str) => Ok(Some(Self::sourced(node, text.clone()))),
            Some(resolved) if keeps_characters(text, resolved) => {
                Ok(Some(Self::sourced(node, text.clone())))
            }
            Some(_) if self.secret => Err(self.wrong(node, Expected::Text, None)),
            Some(resolved) => Err(Problem::at(
                Kind::NeedsQuotes {
                    written: self.sources.spelling(node.place).to_owned(),
                    reading: reading(resolved),
                },
                node.place,
            )),
            None => Err(self.wrong(node, Expected::Text, None)),
        }
    }

    /// `true` or `false`, as YAML reads them. `yes` and its kind are another
    /// spelling, refused with Statac's.
    pub(crate) fn truth(&self, node: &Node) -> Read<bool> {
        match node.resolved() {
            Some(Resolved::Null) => Ok(None),
            Some(Resolved::Bool(b)) => Ok(Some(Self::sourced(node, b))),
            _ => Err(self.wrong(node, Expected::TrueFalse, respelt_truth(node))),
        }
    }

    pub(crate) fn whole(&self, node: &Node, from: u64) -> Read<u64> {
        let expected = Expected::WholeNumber { from };
        match node.resolved() {
            Some(Resolved::Null) => Ok(None),
            Some(Resolved::Int(Some(n))) if n >= i128::from(from) => u64::try_from(n)
                .map(|n| Some(Self::sourced(node, n)))
                .map_err(|_| self.wrong(node, expected, None)),
            Some(Resolved::Str) => {
                let respelt = scalar(node)
                    .filter(|t| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit()))
                    .filter(|t| t.parse::<u64>().is_ok_and(|n| n >= from))
                    .map(str::to_owned);
                Err(self.wrong(node, expected, respelt))
            }
            _ => Err(self.wrong(node, expected, None)),
        }
    }

    /// A list of text, or one text standing for a list of one.
    pub(crate) fn texts(&self, node: &Node) -> Read<Vec<Sourced<String>>> {
        match &node.value {
            Yaml::Sequence(items) => {
                let mut texts = Vec::with_capacity(items.len());
                for item in items {
                    if let Some(text) = self.text(item)? {
                        texts.push(text);
                    }
                }
                Ok(Some(Self::sourced(node, texts)))
            }
            Yaml::Mapping(_) => Err(self.wrong(node, Expected::List, None)),
            Yaml::Scalar(_) => Ok(self.text(node)?.map(|text| {
                let origin = text.origin.clone();
                Sourced::new(vec![text], origin)
            })),
        }
    }

    pub(crate) fn date(&self, node: &Node) -> Read<Date> {
        let Some(text) = self.date_text(node, Expected::Date)? else {
            return Ok(None);
        };
        date::read(text)
            .map(|date| Some(Self::sourced(node, date)))
            .map_err(|fault| self.date_fault(node, text, fault, Expected::Date))
    }

    pub(crate) fn when(&self, node: &Node) -> Read<When> {
        let Some(text) = self.date_text(node, Expected::When)? else {
            return Ok(None);
        };
        date::read_when(text)
            .map(|when| Some(Self::sourced(node, when)))
            .map_err(|fault| self.date_fault(node, text, fault, Expected::When))
    }

    fn date_text<'n>(
        &self,
        node: &'n Node,
        expected: Expected,
    ) -> Result<Option<&'n str>, Problem> {
        match (&node.value, node.resolved()) {
            (_, Some(Resolved::Null)) => Ok(None),
            (Yaml::Scalar(text), Some(_)) => Ok(Some(text)),
            _ => Err(self.wrong(node, expected, None)),
        }
    }

    fn date_fault(&self, node: &Node, text: &str, fault: Fault, expected: Expected) -> Problem {
        let written = text.trim().to_owned();
        let kind = match fault {
            Fault::NotIso { iso } => return self.wrong(node, expected, iso),
            Fault::Unclear { readings } => Kind::UnclearDate {
                written,
                readings: Box::new(readings),
            },
            Fault::NoSuchDay { day, month, year } => Kind::NoSuchDate {
                written,
                missing: Missing::Day { day, month, year },
            },
            Fault::NoSuchMonth(month) => Kind::NoSuchDate {
                written,
                missing: Missing::Month(month),
            },
            Fault::NoSuchTime(time) => Kind::NoSuchDate {
                written,
                missing: Missing::Time(time),
            },
            Fault::NoSuchOffset(offset) => Kind::NoSuchDate {
                written,
                missing: Missing::Offset(offset),
            },
        };
        Problem::at(kind, node.place)
    }

    /// Text that must be one of `words`.
    pub(crate) fn one_of(&self, node: &Node, words: &[&str]) -> Read<String> {
        let expected = || Expected::OneOf(words.iter().map(|w| (*w).to_owned()).collect());
        let text = match self.text(node) {
            Ok(text) => text,
            Err(_) => return Err(self.wrong(node, expected(), None)),
        };
        match text {
            Some(text) if !words.contains(&text.value.as_str()) => {
                Err(self.wrong(node, expected(), None))
            }
            text => Ok(text),
        }
    }

    /// One of `words` where `false` is taken too, so a problem lists it
    /// last, and offers it for `no` and its kind, in quotes or not.
    pub(crate) fn one_of_or_false(&self, node: &Node, words: &[&str]) -> Read<String> {
        let expected = || {
            let words = words.iter().copied().chain(["false"]);
            Expected::OneOf(words.map(str::to_owned).collect())
        };
        match self.text(node) {
            Ok(Some(text)) if !words.contains(&text.value.as_str()) => {
                let word = text.value.to_ascii_lowercase();
                let respelt = matches!(word.as_str(), "no" | "n" | "off").then(|| "false".into());
                Err(self.wrong(node, expected(), respelt))
            }
            Ok(text) => Ok(text),
            Err(_) => Err(self.wrong(node, expected(), None)),
        }
    }

    /// An address on the web, or one in the site.
    pub(crate) fn address(&self, node: &Node) -> Read<String> {
        match self.text(node)? {
            Some(text) if !address_shaped(&text.value) => {
                Err(self.wrong(node, Expected::Address, None))
            }
            text => Ok(text),
        }
    }

    /// A whole number or a fraction.
    pub(crate) fn number(&self, node: &Node) -> Result<(), Problem> {
        match node.resolved() {
            Some(Resolved::Null | Resolved::Int(Some(_)) | Resolved::Float(_)) => Ok(()),
            Some(Resolved::Str) => {
                let respelt = scalar(node)
                    .filter(|t| crate::yaml::resolve(t.trim()) != Resolved::Str)
                    .map(|t| t.trim().to_owned());
                Err(self.wrong(node, Expected::Number, respelt))
            }
            _ => Err(self.wrong(node, Expected::Number, None)),
        }
    }

    pub(crate) fn list(&self, node: &Node) -> Result<(), Problem> {
        match (&node.value, node.resolved()) {
            (Yaml::Sequence(_), _) | (_, Some(Resolved::Null)) => Ok(()),
            _ => Err(self.wrong(node, Expected::List, None)),
        }
    }
}

fn scalar(node: &Node) -> Option<&str> {
    match &node.value {
        Yaml::Scalar(text) => Some(text),
        _ => None,
    }
}

/// `true` or `false` for a plain word another tool reads as one.
fn respelt_truth(node: &Node) -> Option<String> {
    if node.style != Style::Plain {
        return None;
    }
    let word = scalar(node)?.to_ascii_lowercase();
    match word.as_str() {
        "yes" | "y" | "on" => Some("true".into()),
        "no" | "n" | "off" => Some("false".into()),
        _ => None,
    }
}

/// What YAML read a scalar as, in words for a headline.
fn reading(resolved: Resolved) -> String {
    match resolved {
        Resolved::Null => "no value".into(),
        Resolved::Bool(b) => b.to_string(),
        Resolved::Int(Some(n)) => n.to_string(),
        Resolved::Int(None) => "a number too large to hold".into(),
        Resolved::Float(f) if f.is_nan() => "not a number".into(),
        Resolved::Float(f) if f.is_infinite() && f > 0.0 => "infinity".into(),
        Resolved::Float(f) if f.is_infinite() => "minus infinity".into(),
        Resolved::Float(f) => f.to_string(),
        Resolved::Str => "text".into(),
    }
}

/// An address on the web, `https://` and a host with a dot, or one in the
/// site, from `/`.
pub(crate) fn address_shaped(text: &str) -> bool {
    if text.contains(char::is_control) {
        return false;
    }
    if text.starts_with('/') {
        return true;
    }
    let Some(rest) = text
        .strip_prefix("https://")
        .or_else(|| text.strip_prefix("http://"))
    else {
        return false;
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    host.contains('.') && !host.starts_with('.') && !host.ends_with('.')
}

/// A place inside a written value: `range` of the value's text, placed in
/// the source where the value is written plainly or in simple quotes, and
/// the whole value otherwise.
pub(crate) fn place_within(
    sources: &Sources,
    origin: &Origin,
    value: &str,
    range: Span,
) -> Option<Place> {
    let place = origin.place?;
    let spelling = sources.spelling(place);
    let quote = match origin.style {
        Style::Plain => 0,
        Style::SingleQuoted | Style::DoubleQuoted => 1,
        _ => return Some(place),
    };
    let inner = spelling.get(quote..spelling.len().saturating_sub(quote));
    if inner != Some(value) {
        return Some(place);
    }
    let start = place.span.start as usize + quote;
    Some(Place {
        source: place.source,
        span: Span::new(start + range.start as usize, start + range.end as usize),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceId;

    fn read(text: &str) -> (Sources, SourceId, Node) {
        let mut sources = Sources::default();
        let id = sources.add("statac.yaml", format!("v: {text}\n"));
        let node = crate::yaml::read(&sources, id).unwrap();
        let v = node.get("v").unwrap().clone();
        (sources, id, v)
    }

    fn kind(problem: Problem) -> Kind {
        problem.kind
    }

    #[test]
    fn text_keeps_a_bare_value_only_where_its_reading_keeps_the_characters() {
        for (written, value) in [
            ("1984", "1984"),
            ("2.5", "2.5"),
            ("true", "true"),
            ("404", "404"),
            ("Post 1", "Post 1"),
            ("'3.10'", "3.10"),
            ("\"0x10\"", "0x10"),
            ("null program", "null program"),
        ] {
            let (sources, _, node) = read(written);
            let text = Reader::new(&sources, "v").text(&node).unwrap().unwrap();
            assert_eq!(text.value, value, "{written}");
        }
        for (written, reading) in [
            ("3.10", "3.1"),
            ("1e3", "1000"),
            ("0x10", "16"),
            ("0o10", "8"),
            ("010", "10"),
            ("+12", "12"),
            ("True", "true"),
            (".inf", "infinity"),
            ("-.inf", "minus infinity"),
            (".nan", "not a number"),
        ] {
            let (sources, _, node) = read(written);
            let problem = Reader::new(&sources, "v").text(&node).unwrap_err();
            assert_eq!(
                kind(problem.clone()),
                Kind::NeedsQuotes {
                    written: written.into(),
                    reading: reading.into()
                },
                "{written}"
            );
            assert_eq!(sources.spelling(problem.place.unwrap()), written);
        }
        let (sources, _, node) = read("~");
        assert_eq!(Reader::new(&sources, "v").text(&node), Ok(None));
    }

    #[test]
    fn a_truth_is_true_or_false_and_other_spellings_are_offered_ours() {
        for (written, value) in [("true", true), ("False", false), ("TRUE", true)] {
            let (sources, _, node) = read(written);
            let b = Reader::new(&sources, "draft")
                .truth(&node)
                .unwrap()
                .unwrap();
            assert_eq!(b.value, value);
        }
        for (written, respelt) in [
            ("yes", Some("true")),
            ("Off", Some("false")),
            ("'yes'", None),
            ("perhaps", None),
            ("1", None),
        ] {
            let (sources, _, node) = read(written);
            let problem = Reader::new(&sources, "draft").truth(&node).unwrap_err();
            assert_eq!(
                problem.kind,
                Kind::WrongKind {
                    field: "draft".into(),
                    expected: Expected::TrueFalse,
                    found: Found::Written(written.into()),
                    respelt: respelt.map(Into::into),
                }
            );
        }
    }

    #[test]
    fn a_whole_number_has_a_floor() {
        let (sources, _, node) = read("10");
        let n = Reader::new(&sources, "posts_per_page").whole(&node, 0);
        assert_eq!(n.unwrap().unwrap().value, 10);
        for (written, from, respelt) in [
            ("-1", 0, None),
            ("0", 1, None),
            ("2.0", 0, None),
            ("'10'", 0, Some("10")),
            ("ten", 0, None),
        ] {
            let (sources, _, node) = read(written);
            let problem = Reader::new(&sources, "n").whole(&node, from).unwrap_err();
            assert_eq!(
                problem.kind,
                Kind::WrongKind {
                    field: "n".into(),
                    expected: Expected::WholeNumber { from },
                    found: Found::Written(written.into()),
                    respelt: respelt.map(Into::into),
                },
                "{written}"
            );
        }
    }

    #[test]
    fn a_list_of_text_may_be_one_text() {
        let (sources, _, node) = read("[Tag 1, 'Tag 2']");
        let texts = Reader::new(&sources, "tags").texts(&node).unwrap().unwrap();
        let values: Vec<&str> = texts.value.iter().map(|t| t.value.as_str()).collect();
        assert_eq!(values, ["Tag 1", "Tag 2"]);
        assert_eq!(texts.value[1].origin.style, Style::SingleQuoted);
        let (sources, _, node) = read("Tag 1");
        let texts = Reader::new(&sources, "tags").texts(&node).unwrap().unwrap();
        assert_eq!(texts.value.len(), 1);
        let (sources, _, node) = read("[Tag 1, 3.10]");
        let problem = Reader::new(&sources, "tags").texts(&node).unwrap_err();
        assert_eq!(sources.spelling(problem.place.unwrap()), "3.10");
    }

    #[test]
    fn dates_are_read_and_refused_in_the_date_words() {
        let (sources, _, node) = read("\"2026-10-04\"");
        let date = Reader::new(&sources, "date").date(&node).unwrap().unwrap();
        assert_eq!(date.value, Date::day_only(2026, 10, 4));
        let cases = [
            (
                "3/4/2026",
                Kind::UnclearDate {
                    written: "3/4/2026".into(),
                    readings: Box::new([
                        ("2026-04-03".into(), "3 April".into()),
                        ("2026-03-04".into(), "4 March".into()),
                    ]),
                },
            ),
            (
                "2026-02-30",
                Kind::NoSuchDate {
                    written: "2026-02-30".into(),
                    missing: Missing::Day {
                        day: 30,
                        month: 2,
                        year: 2026,
                    },
                },
            ),
            (
                "13/04/2026",
                Kind::WrongKind {
                    field: "date".into(),
                    expected: Expected::Date,
                    found: Found::Written("13/04/2026".into()),
                    respelt: Some("2026-04-13".into()),
                },
            ),
            (
                "[2026]",
                Kind::WrongKind {
                    field: "date".into(),
                    expected: Expected::Date,
                    found: Found::List,
                    respelt: None,
                },
            ),
        ];
        for (written, expected) in cases {
            let (sources, _, node) = read(written);
            let problem = Reader::new(&sources, "date").date(&node).unwrap_err();
            assert_eq!(problem.kind, expected, "{written}");
        }
    }

    #[test]
    fn one_of_a_set_of_words() {
        let (sources, _, node) = read("folder");
        let words = ["site", "folder", "series"];
        let reader = Reader::new(&sources, "neighbours");
        assert_eq!(
            reader.one_of(&node, &words).unwrap().unwrap().value,
            "folder"
        );
        let (sources, _, node) = read("all");
        let reader = Reader::new(&sources, "neighbours");
        assert!(matches!(
            reader.one_of(&node, &words).unwrap_err().kind,
            Kind::WrongKind {
                expected: Expected::OneOf(_),
                ..
            }
        ));
    }

    #[test]
    fn one_of_a_set_of_words_or_false_offers_false_for_no() {
        let words = ["site", "folder", "series"];
        let (sources, _, node) = read("series");
        let reader = Reader::new(&sources, "neighbours");
        let read_as = reader.one_of_or_false(&node, &words).unwrap().unwrap();
        assert_eq!(read_as.value, "series");
        let listed: Vec<String> = ["site", "folder", "series", "false"]
            .map(str::to_owned)
            .to_vec();
        for (written, offered) in [
            ("no", Some("false")),
            ("No", Some("false")),
            ("off", Some("false")),
            ("\"no\"", Some("false")),
            ("'Off'", Some("false")),
            ("yes", None),
            ("true", None),
            ("all", None),
        ] {
            let (sources, _, node) = read(written);
            let reader = Reader::new(&sources, "neighbours");
            let Kind::WrongKind {
                expected, respelt, ..
            } = reader.one_of_or_false(&node, &words).unwrap_err().kind
            else {
                panic!("{written}");
            };
            assert_eq!(expected, Expected::OneOf(listed.clone()), "{written}");
            assert_eq!(respelt.as_deref(), offered, "{written}");
        }
    }

    #[test]
    fn addresses_are_on_the_web_or_in_the_site() {
        for text in ["https://example.org", "http://example.org/a", "/about/"] {
            assert!(address_shaped(text), "{text}");
        }
        for text in ["example.org", "https://localhost", "about", "https://.org"] {
            assert!(!address_shaped(text), "{text}");
        }
    }

    #[test]
    fn a_place_within_a_value_is_found_through_its_quotes() {
        for (written, inner) in [
            ("/{yaer}/", "{yaer}"),
            ("'/{yaer}/'", "{yaer}"),
            ("\"/{yaer}/\"", "{yaer}"),
        ] {
            let (sources, _, node) = read(written);
            let text = Reader::new(&sources, "permalink")
                .text(&node)
                .unwrap()
                .unwrap();
            let at = text.value.find('{').unwrap();
            let place = place_within(
                &sources,
                &text.origin,
                &text.value,
                Span::new(at, at + inner.len()),
            )
            .unwrap();
            assert_eq!(sources.spelling(place), inner, "{written}");
        }
    }
}
