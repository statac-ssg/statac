//! Text written as YAML that reads back as the same text: plain where
//! Statac's reader and others take it as that text, and in double quotes
//! with every character that needs it escaped where they wouldn't.

use super::{Node, Resolved, Value};
use crate::origin::Style;
use crate::source::Sources;

/// The words a plain scalar is read as true or false by YAML 1.1, which
/// Statac reads as text and other readers don't: written in quotes, so no
/// reader takes them for anything but text.
const TRUTHS_ELSEWHERE: &[&str] = &["yes", "no", "on", "off", "y", "n"];

/// `text` as a value that reads back as that text.
pub(crate) fn text(text: &str) -> String {
    let plain = plain_anywhere(text)
        && one_entry(&format!("value: {text}\n")).is_some_and(|(_, value)| {
            value.resolved() == Some(Resolved::Str)
                && matches!(&value.value, Value::Scalar(read) if read == text)
        });
    match plain {
        true => text.to_owned(),
        false => quoted(text),
    }
}

/// A field's name, plain where it reads back as written.
pub(crate) fn key(name: &str) -> String {
    let plain = plain_anywhere(name)
        && one_entry(&format!("{name}: x\n")).is_some_and(|(key, _)| {
            key.style == Style::Plain && matches!(&key.value, Value::Scalar(read) if read == name)
        });
    match plain {
        true => name.to_owned(),
        false => quoted(name),
    }
}

/// `text` in double quotes, every character a reader would take as other
/// than itself escaped.
pub(crate) fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if printable(c) => out.push(c),
            c if u32::from(c) <= 0xFFFF => out.push_str(&format!("\\u{:04X}", u32::from(c))),
            c => out.push_str(&format!("\\U{:08X}", u32::from(c))),
        }
    }
    out.push('"');
    out
}

/// Whether no reader takes `text` written plain for other than text, or
/// for other characters: it is no word YAML 1.1 reads as true or false,
/// and holds only tabs and characters YAML reads as themselves in double
/// quotes. YAML 1.1 takes U+2028 and U+2029 for line breaks.
fn plain_anywhere(text: &str) -> bool {
    !TRUTHS_ELSEWHERE.contains(&text.to_ascii_lowercase().as_str())
        && text.chars().all(|c| c == '\t' || printable(c))
}

/// Whether YAML reads `c` as itself in double quotes.
fn printable(c: char) -> bool {
    matches!(
        u32::from(c),
        0x20..=0x7E | 0xA0..=0x2027 | 0x202A..=0xD7FF | 0xE000..=0xFEFE | 0xFF00..=0xFFFD | 0x10000..
    )
}

/// The one entry `text` holds as a mapping, where it reads as one.
fn one_entry(text: &str) -> Option<(Node, Node)> {
    let mut sources = Sources::default();
    let source = sources.add("value.yaml", text.to_owned());
    let node = super::read(&sources, source).ok()?;
    match node.value {
        Value::Mapping(mut entries) if entries.len() == 1 => {
            let entry = entries.pop()?;
            Some((entry.key, entry.value))
        }
        _ => None,
    }
}
