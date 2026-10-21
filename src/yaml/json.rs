//! JSON, as RFC 8259 defines it, read into the same tree as YAML: every
//! value placed where it is written, so whatever reads a tree reads a JSON
//! file alike. A string is a quoted scalar and a number, `true`, `false`
//! or `null` a plain one, which YAML's core schema reads as JSON does.

use std::collections::HashMap;
use std::ops::Range;

use super::tree::{Entry, Node, Value};
use crate::origin::Style;
use crate::problem::JsonFault;
use crate::problem::{Kind, Problem};
use crate::source::{Place, Span};

/// How deeply lists and objects may nest, as YAML's reader allows.
const DEPTH: usize = 256;

/// Reads `text`, the whole of the source `at` places, as one JSON value.
pub(super) fn read(text: &str, at: Place) -> Result<Node, Problem> {
    let mut reader = Reader {
        text,
        bytes: text.as_bytes(),
        at: usize::from(text.starts_with('\u{feff}')) * 3,
        base: at,
    };
    let node = reader.value(0)?;
    reader.blank();
    if reader.at < text.len() {
        return Err(reader.unexpected(reader.at));
    }
    Ok(node)
}

struct Reader<'a> {
    text: &'a str,
    bytes: &'a [u8],
    at: usize,
    base: Place,
}

impl Reader<'_> {
    fn place(&self, range: Range<usize>) -> Place {
        let start = self.base.span.start as usize;
        Place {
            source: self.base.source,
            span: Span::new(start + range.start, start + range.end),
        }
    }

    fn fault(&self, found: JsonFault, range: Range<usize>) -> Problem {
        Problem::at(Kind::MalformedJson { found }, self.place(range))
    }

    /// Passes over the whitespace JSON allows between values.
    fn blank(&mut self) {
        while matches!(self.bytes.get(self.at), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.at += 1;
        }
    }

    /// The run of text at `from` up to whitespace or punctuation, or the
    /// one character there.
    fn token(&self, from: usize) -> Range<usize> {
        let rest = &self.text[from..];
        let end = rest
            .find(|c: char| c.is_whitespace() || ",:[]{}\"".contains(c))
            .unwrap_or(rest.len());
        match end {
            0 => from..from + rest.chars().next().map_or(0, char::len_utf8),
            end => from..from + end,
        }
    }

    fn unexpected(&self, from: usize) -> Problem {
        if from >= self.text.len() {
            return self.fault(JsonFault::EndsEarly, from..from);
        }
        let range = self.token(from);
        let written = self.text[range.clone()].to_owned();
        self.fault(JsonFault::Unexpected(written), range)
    }

    fn value(&mut self, depth: usize) -> Result<Node, Problem> {
        self.blank();
        let start = self.at;
        match self.bytes.get(start) {
            None => Err(self.fault(JsonFault::EndsEarly, start..start)),
            Some(b'{' | b'[') if depth >= DEPTH => {
                Err(self.fault(JsonFault::TooDeep, start..start + 1))
            }
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => {
                let (text, range) = self.string()?;
                Ok(self.scalar(text, Style::DoubleQuoted, range))
            }
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => {
                let range = self.token(start);
                match &self.text[range.clone()] {
                    word @ ("true" | "false" | "null") => {
                        self.at = range.end;
                        Ok(self.scalar(word.to_owned(), Style::Plain, range))
                    }
                    _ => Err(self.unexpected(start)),
                }
            }
        }
    }

    fn scalar(&self, text: String, style: Style, range: Range<usize>) -> Node {
        Node {
            place: self.place(range),
            style,
            tag: None,
            value: Value::Scalar(text),
        }
    }

    fn container(&self, value: Value, range: Range<usize>) -> Node {
        Node {
            place: self.place(range),
            style: Style::None,
            tag: None,
            value,
        }
    }

    /// After a value in a list or object: a comma, the closing bracket, or
    /// what is wrong. True where another value follows.
    fn after(&mut self, open: char, start: usize) -> Result<bool, Problem> {
        self.blank();
        let close = if open == '[' { b']' } else { b'}' };
        match self.bytes.get(self.at) {
            Some(b',') => {
                let comma = self.at;
                self.at += 1;
                self.blank();
                match self.bytes.get(self.at) {
                    Some(b']' | b'}') => {
                        Err(self.fault(JsonFault::TrailingComma, comma..comma + 1))
                    }
                    _ => Ok(true),
                }
            }
            Some(&b) if b == close => {
                self.at += 1;
                Ok(false)
            }
            Some(&b @ (b']' | b'}')) => Err(self.fault(
                JsonFault::MismatchedBracket {
                    open,
                    close: char::from(b),
                },
                self.at..self.at + 1,
            )),
            None => Err(self.fault(JsonFault::UnclosedBracket(open), start..start + 1)),
            Some(_) => Err(self.unexpected(self.at)),
        }
    }

    fn array(&mut self, depth: usize) -> Result<Node, Problem> {
        let start = self.at;
        self.at += 1;
        let mut items = Vec::new();
        self.blank();
        if self.bytes.get(self.at) == Some(&b']') {
            self.at += 1;
        } else {
            loop {
                if self.at >= self.text.len() {
                    return Err(self.fault(JsonFault::UnclosedBracket('['), start..start + 1));
                }
                items.push(self.value(depth + 1)?);
                if !self.after('[', start)? {
                    break;
                }
            }
        }
        Ok(self.container(Value::Sequence(items), start..self.at))
    }

    fn object(&mut self, depth: usize) -> Result<Node, Problem> {
        let start = self.at;
        self.at += 1;
        let mut entries = Vec::new();
        let mut keys: HashMap<String, Place> = HashMap::new();
        self.blank();
        if self.bytes.get(self.at) == Some(&b'}') {
            self.at += 1;
            return Ok(self.container(Value::Mapping(entries), start..self.at));
        }
        loop {
            self.blank();
            match self.bytes.get(self.at) {
                Some(b'"') => {}
                None => {
                    return Err(self.fault(JsonFault::UnclosedBracket('{'), start..start + 1));
                }
                Some(_) => return Err(self.unexpected(self.at)),
            }
            let (name, range) = self.string()?;
            let key = self.scalar(name.clone(), Style::DoubleQuoted, range);
            if let Some(&first) = keys.get(&name) {
                return Err(Problem::at(
                    Kind::DuplicateKey { key: name, first },
                    key.place,
                ));
            }
            keys.insert(name, key.place);
            self.blank();
            match self.bytes.get(self.at) {
                Some(b':') => self.at += 1,
                _ => return Err(self.unexpected(self.at)),
            }
            let value = self.value(depth + 1)?;
            entries.push(Entry { key, value });
            if !self.after('{', start)? {
                break;
            }
        }
        Ok(self.container(Value::Mapping(entries), start..self.at))
    }

    /// A string at the quote it opens with: its text, and where it is
    /// written, quotes and all.
    fn string(&mut self) -> Result<(String, Range<usize>), Problem> {
        let start = self.at;
        let mut text = String::new();
        let mut chars = self.text[start + 1..].char_indices();
        let at = |i: usize| start + 1 + i;
        while let Some((i, c)) = chars.next() {
            match c {
                '"' => {
                    self.at = at(i) + 1;
                    return Ok((text, start..self.at));
                }
                // A line break in a string is most likely a string never closed.
                '\n' | '\r' => break,
                '\u{0}'..='\u{1f}' => {
                    return Err(self.fault(JsonFault::ControlCharacter(c), at(i)..at(i) + 1));
                }
                '\\' => {
                    let from = at(i);
                    let escape = match chars.next() {
                        Some((_, escape)) => escape,
                        None => break,
                    };
                    let simple = match escape {
                        '"' => Some('"'),
                        '\\' => Some('\\'),
                        '/' => Some('/'),
                        'b' => Some('\u{8}'),
                        'f' => Some('\u{c}'),
                        'n' => Some('\n'),
                        'r' => Some('\r'),
                        't' => Some('\t'),
                        'u' => None,
                        _ => {
                            let end = from + 1 + escape.len_utf8();
                            let written = self.text[from..end].to_owned();
                            return Err(self.fault(JsonFault::UnknownEscape(written), from..end));
                        }
                    };
                    if let Some(c) = simple {
                        text.push(c);
                        continue;
                    }
                    let first = self.hex(from)?;
                    let c = match first {
                        0xD800..=0xDBFF => {
                            let low = self
                                .text
                                .get(from + 6..from + 8)
                                .filter(|next| *next == "\\u")
                                .and_then(|_| self.hex(from + 6).ok())
                                .filter(|low| (0xDC00..=0xDFFF).contains(low));
                            let Some(low) = low else {
                                let written = self.text[from..from + 6].to_owned();
                                return Err(
                                    self.fault(JsonFault::HalfCharacter(written), from..from + 6)
                                );
                            };
                            for _ in 0..6 {
                                chars.next();
                            }
                            0x10000 + ((first - 0xD800) << 10) + (low - 0xDC00)
                        }
                        0xDC00..=0xDFFF => {
                            let written = self.text[from..from + 6].to_owned();
                            return Err(
                                self.fault(JsonFault::HalfCharacter(written), from..from + 6)
                            );
                        }
                        code => code,
                    };
                    for _ in 0..4 {
                        chars.next();
                    }
                    text.push(char::from_u32(c).expect("a scalar value, surrogates paired"));
                }
                c => text.push(c),
            }
        }
        Err(self.fault(JsonFault::UnclosedString, start..start + 1))
    }

    /// The four hex digits after the `\u` at `from`.
    fn hex(&self, from: usize) -> Result<u32, Problem> {
        let digits = &self.bytes[from + 2..];
        let count = digits
            .iter()
            .take(4)
            .take_while(|b| b.is_ascii_hexdigit())
            .count();
        if count < 4 {
            let end = from + 2 + count;
            let written = self.text[from..end].to_owned();
            return Err(self.fault(JsonFault::UnknownEscape(written), from..end));
        }
        let hex = &self.text[from + 2..from + 6];
        Ok(u32::from_str_radix(hex, 16).expect("four hex digits"))
    }

    /// `-? (0 | [1-9][0-9]*) (. [0-9]+)? ([eE] [+-]? [0-9]+)?`
    fn number(&mut self) -> Result<Node, Problem> {
        let range = self.token(self.at);
        let written = &self.text[range.clone()];
        let b = written.as_bytes();
        let mut i = usize::from(b.first() == Some(&b'-'));
        let digits = |i: &mut usize| {
            let from = *i;
            while b.get(*i).is_some_and(u8::is_ascii_digit) {
                *i += 1;
            }
            *i - from
        };
        let whole = (i, digits(&mut i));
        let mut fits = whole.1 > 0 && !(whole.1 > 1 && b[whole.0] == b'0');
        if fits && b.get(i) == Some(&b'.') {
            i += 1;
            fits = digits(&mut i) > 0;
        }
        if fits && matches!(b.get(i), Some(b'e' | b'E')) {
            i += 1;
            if matches!(b.get(i), Some(b'+' | b'-')) {
                i += 1;
            }
            fits = digits(&mut i) > 0;
        }
        if !fits || i != b.len() {
            return Err(self.fault(JsonFault::NotANumber(written.to_owned()), range));
        }
        self.at = range.end;
        Ok(self.scalar(written.to_owned(), Style::Plain, range))
    }
}
