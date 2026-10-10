//! Front matter and settings written as YAML that Statac reads back to the
//! same values: each text plain where Statac's reader takes it as that
//! text, and in double quotes where it wouldn't.

use crate::origin::Style;
use crate::source::Sources;
use crate::yaml::{self, Node, Resolved, Value as Yaml};

/// A value in front matter or settings, as a reader found it.
#[derive(Clone, PartialEq, Debug)]
pub enum Value {
    Text(String),
    /// A number, as written: digits, a sign, a point, an exponent.
    Number(String),
    Truth(bool),
    Nothing,
    List(Vec<Value>),
    /// Fields nested beneath one, in the order written.
    Fields(Vec<(String, Value)>),
}

/// The words a plain scalar is read as true or false by YAML 1.1, which
/// Statac reads as text and other readers don't: written in quotes, so no
/// reader takes them for anything but text.
const TRUTHS_ELSEWHERE: &[&str] = &["yes", "no", "on", "off", "y", "n"];

/// `fields` as lines of a YAML mapping, each line ending with a line break.
pub fn fields<'a>(fields: impl IntoIterator<Item = (&'a str, &'a Value)>) -> String {
    let mut out = String::new();
    for (name, value) in fields {
        out.push_str(&key(name));
        out.push(':');
        node(&mut out, value, 2);
    }
    out
}

/// Writes `value` just after a `key:` or a `-`, its own lines indented by
/// `indent`.
fn node(out: &mut String, value: &Value, indent: usize) {
    let pad = " ".repeat(indent);
    match value {
        Value::Nothing => out.push('\n'),
        Value::List(items) if items.is_empty() => out.push_str(" []\n"),
        Value::Fields(entries) if entries.is_empty() => out.push_str(" {}\n"),
        Value::List(items) => {
            out.push('\n');
            for item in items {
                out.push_str(&pad);
                out.push('-');
                match item {
                    // The first field shares the line of its `-`, the rest
                    // line up beneath it.
                    Value::Fields(entries) if !entries.is_empty() => {
                        for (at, (name, value)) in entries.iter().enumerate() {
                            match at {
                                0 => out.push(' '),
                                _ => out.push_str(&" ".repeat(indent + 2)),
                            }
                            out.push_str(&key(name));
                            out.push(':');
                            node(out, value, indent + 4);
                        }
                    }
                    _ => node(out, item, indent + 2),
                }
            }
        }
        Value::Fields(entries) => {
            out.push('\n');
            for (name, value) in entries {
                out.push_str(&pad);
                out.push_str(&key(name));
                out.push(':');
                node(out, value, indent + 2);
            }
        }
        scalar => {
            out.push(' ');
            out.push_str(&scalar_text(scalar));
            out.push('\n');
        }
    }
}

/// A scalar as written: a number or true or false as itself, text plain
/// or quoted.
fn scalar_text(value: &Value) -> String {
    match value {
        Value::Truth(truth) => truth.to_string(),
        Value::Number(number)
            if matches!(yaml::resolve(number), Resolved::Int(_) | Resolved::Float(_)) =>
        {
            number.clone()
        }
        Value::Number(text) | Value::Text(text) => text_value(text),
        _ => String::new(),
    }
}

/// `text` as a value Statac reads back as that text.
pub fn text_value(text: &str) -> String {
    let elsewhere = TRUTHS_ELSEWHERE.contains(&text.to_ascii_lowercase().as_str());
    let plain = !elsewhere
        && !text.contains(['\n', '\r'])
        && one_entry(&format!("value: {text}\n")).is_some_and(|(_, value)| {
            value.resolved() == Some(Resolved::Str)
                && matches!(&value.value, Yaml::Scalar(read) if read == text)
        });
    match plain {
        true => text.to_owned(),
        false => double_quoted(text),
    }
}

/// A field's name, plain where Statac reads it back as written.
fn key(name: &str) -> String {
    let plain = !name.contains(['\n', '\r'])
        && one_entry(&format!("{name}: x\n")).is_some_and(|(key, _)| {
            key.style == Style::Plain && matches!(&key.value, Yaml::Scalar(read) if read == name)
        });
    match plain {
        true => name.to_owned(),
        false => double_quoted(name),
    }
}

/// The one entry `text` holds as a mapping, where it reads as one.
fn one_entry(text: &str) -> Option<(Node, Node)> {
    let mut sources = Sources::default();
    let source = sources.add("value.yaml", text.to_owned());
    let node = yaml::read(&sources, source).ok()?;
    match node.value {
        Yaml::Mapping(mut entries) if entries.len() == 1 => {
            let entry = entries.pop()?;
            Some((entry.key, entry.value))
        }
        _ => None,
    }
}

fn double_quoted(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04X}", u32::from(c))),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// What `node` holds, each plain scalar read by `plain` and every other
/// scalar as text.
pub fn value_of(node: &Node, plain: &dyn Fn(&str) -> Value) -> Value {
    match &node.value {
        Yaml::Scalar(text) => match (node.style, node.tag) {
            (Style::Plain, None) => plain(text),
            (_, None) => Value::Text(text.clone()),
            (_, Some(_)) => match node.resolved() {
                Some(Resolved::Str) | None => Value::Text(text.clone()),
                Some(_) => plain(text),
            },
        },
        Yaml::Sequence(items) => {
            Value::List(items.iter().map(|item| value_of(item, plain)).collect())
        }
        Yaml::Mapping(entries) => Value::Fields(
            entries
                .iter()
                .map(|entry| (key_text(&entry.key), value_of(&entry.value, plain)))
                .collect(),
        ),
    }
}

/// A mapping key as text, whatever it holds.
pub fn key_text(key: &Node) -> String {
    match &key.value {
        Yaml::Scalar(text) => text.clone(),
        _ => String::new(),
    }
}

/// A plain scalar as Statac reads it, by YAML's core schema.
pub fn core(text: &str) -> Value {
    match yaml::resolve(text) {
        Resolved::Null => Value::Nothing,
        Resolved::Bool(truth) => Value::Truth(truth),
        Resolved::Int(_) | Resolved::Float(_) => Value::Number(text.to_owned()),
        Resolved::Str => Value::Text(text.to_owned()),
    }
}

/// The fields `text` holds, read as Statac reads front matter; none where
/// it isn't a mapping that reads.
pub fn read_back(text: &str) -> Option<Vec<(String, Value)>> {
    let mut sources = Sources::default();
    let source = sources.add("value.yaml", text.to_owned());
    let node = yaml::read(&sources, source).ok()?;
    match value_of(&node, &core) {
        Value::Fields(fields) => Some(fields),
        Value::Nothing => Some(Vec::new()),
        _ => None,
    }
}
