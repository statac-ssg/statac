//! TOML read into the values a reader takes from YAML, so a reader never
//! knows which a file was written in: fields in the order written, each
//! field at the top with its line, and a date or time as the text it is
//! written as.

use std::path::Path;

use ::toml::Spanned;
use ::toml::de::{DeTable, DeValue};

use super::{Failed, Field, Value};
use crate::problem::{Kind, Problem, Reason};
use crate::source::{Place, Sources, Span};

/// Where a file stopped being TOML that reads.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Stopped {
    pub(crate) line: usize,
}

/// The fields `text` holds, each top-level one with its line.
pub(crate) fn read(text: &str) -> Result<Vec<Field>, Stopped> {
    let table = DeTable::parse(text).map_err(|error| Stopped {
        line: line_of(text, error.span().map_or(0, |span| span.start)),
    })?;
    Ok(in_order(table.get_ref())
        .into_iter()
        .map(|(key, value)| {
            Field::new(
                key.get_ref(),
                value_of(text, value),
                Some(line_of(text, key.span().start)),
            )
        })
        .collect())
}

/// Why the settings file at `path`, holding `text`, can't be read, placed
/// at the line where it stopped.
pub(crate) fn failed(path: &Path, text: String, stopped: &Stopped) -> Box<Failed> {
    let start = text
        .split_inclusive('\n')
        .take(stopped.line - 1)
        .map(str::len)
        .sum::<usize>()
        .min(text.len());
    let end = text[start..]
        .find(['\r', '\n'])
        .map_or(text.len(), |n| start + n);
    let mut sources = Sources::default();
    let source = sources.add(path, text);
    let problem = Problem::at(
        Kind::CantRead {
            path: path.to_path_buf(),
            reason: Reason::NotToml,
        },
        Place {
            source,
            span: Span::new(start, end),
        },
    );
    Box::new(Failed { sources, problem })
}

/// The line `at`, a byte offset into `text`, is on.
fn line_of(text: &str, at: usize) -> usize {
    text[..at.min(text.len())].matches('\n').count() + 1
}

/// A table's entries in the order they are written, which the reader's own
/// map doesn't keep.
fn in_order<'t, 'i>(
    table: &'t DeTable<'i>,
) -> Vec<(
    &'t Spanned<std::borrow::Cow<'i, str>>,
    &'t Spanned<DeValue<'i>>,
)> {
    let mut entries: Vec<_> = table.iter().collect();
    entries.sort_by_key(|(key, _)| key.span().start);
    entries
}

fn value_of(text: &str, value: &Spanned<DeValue>) -> Value {
    match value.get_ref() {
        DeValue::String(string) => Value::Text(string.to_string()),
        DeValue::Integer(integer) => {
            let digits = integer.as_str().replace('_', "");
            match integer.radix() {
                10 => Value::Number(digits.trim_start_matches('+').to_owned()),
                radix => match i128::from_str_radix(&digits, radix) {
                    Ok(number) => Value::Number(number.to_string()),
                    Err(_) => Value::Text(text[value.span()].to_owned()),
                },
            }
        }
        DeValue::Float(float) => {
            let written = float.as_str().replace('_', "");
            let unsigned = written.trim_start_matches(['+', '-']);
            let minus = if written.starts_with('-') { "-" } else { "" };
            Value::Number(match unsigned {
                "inf" => format!("{minus}.inf"),
                "nan" => ".nan".to_owned(),
                _ => written.trim_start_matches('+').to_owned(),
            })
        }
        DeValue::Boolean(truth) => Value::Truth(*truth),
        DeValue::Datetime(_) => Value::Text(text[value.span()].to_owned()),
        DeValue::Array(items) => {
            Value::List(items.iter().map(|item| value_of(text, item)).collect())
        }
        DeValue::Table(table) => Value::Fields(
            in_order(table)
                .into_iter()
                .map(|(key, value)| (key.get_ref().to_string(), value_of(text, value)))
                .collect(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import::yaml;

    fn text(t: &str) -> Value {
        Value::Text(t.into())
    }

    fn number(n: &str) -> Value {
        Value::Number(n.into())
    }

    fn values(written: &str) -> Vec<(String, Value)> {
        read(written)
            .unwrap()
            .into_iter()
            .map(|field| (field.name, field.value))
            .collect()
    }

    #[test]
    fn every_kind_of_value_reads() {
        let read = values(
            "basic = \"a \\\"b\\\" \\u00e9\"\n\
             literal = 'C:\\path'\n\
             multi = \"\"\"\nline 1\nline 2\"\"\"\n\
             multi_literal = '''\nraw \\n'''\n\
             integer = +1_000\n\
             negative = -7\n\
             hex = 0xff\n\
             octal = 0o17\n\
             binary = 0b101\n\
             float = 3.10\n\
             exponent = 6.02e+23\n\
             infinite = -inf\n\
             not_a_number = nan\n\
             yes = true\n\
             no = false\n",
        );
        assert_eq!(
            read,
            vec![
                ("basic".into(), text("a \"b\" é")),
                ("literal".into(), text("C:\\path")),
                ("multi".into(), text("line 1\nline 2")),
                ("multi_literal".into(), text("raw \\n")),
                ("integer".into(), number("1000")),
                ("negative".into(), number("-7")),
                ("hex".into(), number("255")),
                ("octal".into(), number("15")),
                ("binary".into(), number("5")),
                ("float".into(), number("3.10")),
                ("exponent".into(), number("6.02e+23")),
                ("infinite".into(), number("-.inf")),
                ("not_a_number".into(), number(".nan")),
                ("yes".into(), Value::Truth(true)),
                ("no".into(), Value::Truth(false)),
            ]
        );
    }

    #[test]
    fn dates_and_times_are_their_text_as_written() {
        let read = values(
            "offset = 2026-10-04T09:30:00+01:00\n\
             spaced = 2026-10-04 09:30:00Z\n\
             local = 2026-10-04T09:30:00\n\
             day = 2026-10-04\n\
             time = 09:30:00.5\n",
        );
        assert_eq!(
            read,
            vec![
                ("offset".into(), text("2026-10-04T09:30:00+01:00")),
                ("spaced".into(), text("2026-10-04 09:30:00Z")),
                ("local".into(), text("2026-10-04T09:30:00")),
                ("day".into(), text("2026-10-04")),
                ("time".into(), text("09:30:00.5")),
            ]
        );
    }

    #[test]
    fn tables_keep_the_order_written_and_each_field_its_line() {
        let written = "title = \"Site 1\"\n\
                       zebra = 1\n\
                       \n\
                       [params]\n\
                       b = 2\n\
                       a = { y = 1, x = [1, \"two\", { z = 3 }] }\n\
                       \n\
                       [[links]]\n\
                       name = \"Link 1\"\n\
                       [[links]]\n\
                       name = \"Link 2\"\n\
                       \n\
                       [params.nested.deeper]\n\
                       c = 3\n\
                       \n\
                       [alpha]\n\
                       dotted.key = 4\n";
        let read = read(written).unwrap();
        let lines: Vec<(&str, Option<usize>)> = read
            .iter()
            .map(|field| (field.name.as_str(), field.line))
            .collect();
        assert_eq!(
            lines,
            vec![
                ("title", Some(1)),
                ("zebra", Some(2)),
                ("params", Some(4)),
                ("links", Some(8)),
                ("alpha", Some(16)),
            ]
        );
        assert_eq!(
            read[2].value,
            Value::Fields(vec![
                ("b".into(), number("2")),
                (
                    "a".into(),
                    Value::Fields(vec![
                        ("y".into(), number("1")),
                        (
                            "x".into(),
                            Value::List(vec![
                                number("1"),
                                text("two"),
                                Value::Fields(vec![("z".into(), number("3"))]),
                            ])
                        ),
                    ])
                ),
                (
                    "nested".into(),
                    Value::Fields(vec![(
                        "deeper".into(),
                        Value::Fields(vec![("c".into(), number("3"))])
                    )])
                ),
            ])
        );
        assert_eq!(
            read[3].value,
            Value::List(vec![
                Value::Fields(vec![("name".into(), text("Link 1"))]),
                Value::Fields(vec![("name".into(), text("Link 2"))]),
            ])
        );
        assert_eq!(
            read[4].value,
            Value::Fields(vec![(
                "dotted".into(),
                Value::Fields(vec![("key".into(), number("4"))])
            )])
        );
    }

    #[test]
    fn a_file_that_doesnt_read_is_told_at_its_line() {
        assert_eq!(read("a = 1\nb = \n"), Err(Stopped { line: 2 }));
        assert_eq!(read("a = 1\n\n[a]\n"), Err(Stopped { line: 3 }));
        assert_eq!(read("a = \"open\n"), Err(Stopped { line: 1 }));
        assert_eq!(read("x = 1\nx = 2\n"), Err(Stopped { line: 2 }));
    }

    #[test]
    fn what_is_written_as_yaml_reads_back_the_same() {
        let written = "title = \"yes\"\n\
                       year = \"1984\"\n\
                       count = 1984\n\
                       ratio = 3.10\n\
                       big = 6.02e23\n\
                       huge = inf\n\
                       hex = 0x10\n\
                       when = 2026-10-04T09:30:00Z\n\
                       truth = true\n\
                       empty = []\n\
                       note = \"a: b\"\n\
                       [params]\n\
                       handle = \"@name\"\n\
                       list = [[1, 2], [\"a\", \"b: c\"]]\n\
                       [[params.links]]\n\
                       title = \"Link 1\"\n\
                       tags = [\"x\"]\n\
                       [params.empty]\n";
        let fields = read(written).unwrap();
        let yaml = yaml::fields(fields.iter().map(|f| (f.name.as_str(), &f.value)));
        let back = yaml::read_back(&yaml).unwrap();
        let read: Vec<(String, Value)> = fields.into_iter().map(|f| (f.name, f.value)).collect();
        assert_eq!(back, read, "{yaml}");
    }
}
