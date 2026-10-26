//! Snippet calls as the author writes them, `@name(...)`, and what each
//! argument was given.
//!
//! A call is read from the text exactly as typed, before Markdown sees it,
//! so nothing in its arguments is ever read as emphasis, a link or HTML.

use crate::origin::{Origin, Sourced, Style};
use crate::problem::CallFault;
use crate::problem::{Kind, Problem};
use crate::source::{Place, SourceId, Span};

#[derive(Clone, PartialEq, Debug)]
pub struct Call {
    /// The name after the `@`, placed without it.
    pub name: Sourced<String>,
    pub arguments: Vec<Argument>,
    pub within: Within,
    /// For a call that wraps the writing below it up to an `@end`, the call
    /// itself as far as its `)`. Its node is placed at the whole, `@end`
    /// and all, and holds the writing as its children.
    pub head: Option<Span>,
}

impl Call {
    pub fn wraps(&self) -> bool {
        self.head.is_some()
    }

    /// The value given under `name`.
    pub fn named(&self, name: &str) -> Option<&Sourced<Value>> {
        self.arguments
            .iter()
            .find(|argument| argument.name.as_ref().is_some_and(|n| n.value == name))
            .map(|argument| &argument.value)
    }

    /// The values given without a name, in order.
    pub fn unnamed(&self) -> impl Iterator<Item = &Sourced<Value>> {
        self.arguments
            .iter()
            .filter(|argument| argument.name.is_none())
            .map(|argument| &argument.value)
    }
}

/// The call that stands for an address inside the site, wherever the site
/// is published. It is Statac's own, and no snippet's.
pub const IN_SITE: &str = "url";

impl Call {
    /// The address an `@url(...)` is handed: one value with no name, from
    /// the top of the site. None for a call handed anything else.
    pub(super) fn address(&self) -> Option<&str> {
        match self.arguments.as_slice() {
            [argument] if argument.name.is_none() => match &argument.value.value {
                Value::Text(address) if from_the_top(address) => Some(address),
                _ => None,
            },
            _ => None,
        }
    }
}

/// Whether an address starts at the top of a site: with one `/`. A browser
/// reads `/\` as it reads `//`, the start of another site's name.
pub fn from_the_top(address: &str) -> bool {
    address.starts_with('/') && !address[1..].starts_with(['/', '\\'])
}

/// What surrounds a call, which decides what its HTML may be.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Within {
    Prose,
    /// Raw HTML, between its tags.
    Html,
    /// Inside a tag in raw HTML: what the call writes is part of the tag.
    Attribute,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Argument {
    /// The name of a value given as `name=value`.
    pub name: Option<Sourced<String>>,
    /// Placed at the value as written, quotes included.
    pub value: Sourced<Value>,
}

/// A quoted value is always text. An unquoted one is a number or `true` or
/// `false` where it reads as one, and otherwise text as written.
#[derive(Clone, PartialEq, Debug)]
pub enum Value {
    Text(String),
    Number(f64),
    Bool(bool),
}

/// A call read from the text, before Markdown decides where it landed.
pub(super) struct Read {
    pub call: Call,
    /// Just past the closing `)`.
    pub end: usize,
    /// Said only where the call turns out to be a call, not text in code.
    pub problems: Vec<Problem>,
}

/// Where a call can't be read: the place to point at and what is wrong.
pub(super) type Fault = (Span, CallFault);

/// Reads the call whose `@` is at `at`. None where the text there is no
/// call at all: `@` with no name, or a name with no `(` after it.
pub(super) fn read(
    text: &str,
    source: SourceId,
    at: usize,
    depth: usize,
) -> Option<Result<Read, Fault>> {
    let b = text.as_bytes();
    let name_start = at + 1;
    let after = &text[name_start..];
    // A snippet's file may be named with a hyphen, which no call can name.
    let word = after
        .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-'))
        .unwrap_or(after.len());
    let unnameable =
        !after[..word].is_ascii() || (after[..word].contains('-') && !after[..word].ends_with('-'));
    if unnameable && after.starts_with(char::is_alphabetic) && after[word..].starts_with('(') {
        let name = &after[..word];
        return Some(Err((
            Span::new(at, name_start + word),
            CallFault::Name(name.to_owned()),
        )));
    }
    if !b.get(name_start).is_some_and(u8::is_ascii_alphabetic) {
        return None;
    }
    let mut name_end = name_start;
    while b
        .get(name_end)
        .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
    {
        name_end += 1;
    }
    if b.get(name_end) != Some(&b'(') {
        return None;
    }
    let mut reader = Reader {
        text,
        b,
        source,
        depth,
        i: name_end + 1,
        problems: Vec::new(),
    };
    let unclosed = || {
        (
            Span::new(at, name_end + 1),
            CallFault::Unclosed(text[at..=name_end].to_owned()),
        )
    };
    let arguments = match reader.arguments() {
        Ok(arguments) => arguments,
        Err(Stop::Unclosed) => return Some(Err(unclosed())),
        Err(Stop::Fault(fault)) => return Some(Err(fault)),
    };
    let place = |span| Place { source, span };
    Some(Ok(Read {
        call: Call {
            name: Sourced::new(
                text[name_start..name_end].to_owned(),
                Origin::written(place(Span::new(name_start, name_end)), Style::Plain),
            ),
            arguments,
            within: Within::Prose,
            head: None,
        },
        end: reader.i + 1,
        problems: reader.problems,
    }))
}

enum Stop {
    /// No `)` before the writing runs out: a blank line, the end, or a
    /// backtick, so a call never swallows the edge of a code span.
    Unclosed,
    Fault(Fault),
}

struct Reader<'a> {
    text: &'a str,
    b: &'a [u8],
    source: SourceId,
    /// How many of a quotation's `>` markers open each line of the call.
    depth: usize,
    i: usize,
    problems: Vec<Problem>,
}

impl Reader<'_> {
    /// Leaves `i` at the closing `)`.
    fn arguments(&mut self) -> Result<Vec<Argument>, Stop> {
        let mut arguments = Vec::new();
        loop {
            self.space()?;
            match self.b.get(self.i) {
                None => return Err(Stop::Unclosed),
                Some(b')') => return Ok(arguments),
                // An empty argument, as a trailing comma leaves.
                Some(b',') => self.i += 1,
                Some(_) => {
                    arguments.push(self.argument()?);
                    self.space()?;
                    match self.b.get(self.i) {
                        Some(b')') => return Ok(arguments),
                        Some(b',') => self.i += 1,
                        _ => return Err(Stop::Unclosed),
                    }
                }
            }
        }
    }

    fn argument(&mut self) -> Result<Argument, Stop> {
        let name = self.name();
        while matches!(self.b.get(self.i), Some(b' ' | b'\t')) {
            self.i += 1;
        }
        let value = match self.b.get(self.i) {
            Some(&quote @ (b'\'' | b'"')) => self.quoted(quote)?,
            _ => self.unquoted()?,
        };
        Ok(Argument { name, value })
    }

    /// `name=` or `name:` before a value, leaving `i` past the sign.
    fn name(&mut self) -> Option<Sourced<String>> {
        let b = self.b;
        let start = self.i;
        if !b[start].is_ascii_alphabetic() {
            return None;
        }
        let mut end = start + 1;
        while b
            .get(end)
            .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
        {
            end += 1;
        }
        let mut sign = end;
        while matches!(b.get(sign), Some(b' ' | b'\t')) {
            sign += 1;
        }
        let colon = match (b.get(sign), b.get(sign + 1)) {
            (Some(b'='), next) if next != Some(&b'=') => false,
            // Only a colon followed by a space or a quote names anything:
            // `mailto:` and `https://` are the start of a value.
            (Some(b':'), Some(b' ' | b'\t' | b'\'' | b'"')) => true,
            _ => return None,
        };
        if colon {
            self.problems.push(Problem::at(
                Kind::ColonInCall {
                    name: self.text[start..end].to_owned(),
                },
                self.place(Span::new(start, sign + 1)),
            ));
        }
        self.i = sign + 1;
        Some(Sourced::new(
            self.text[start..end].to_owned(),
            Origin::written(self.place(Span::new(start, end)), Style::Plain),
        ))
    }

    /// A quoted value runs to the same quote on its own line, so an
    /// apostrophe can't carry a call off through the rest of a paragraph.
    fn quoted(&mut self, quote: u8) -> Result<Sourced<Value>, Stop> {
        let b = self.b;
        let open = self.i;
        let mut k = open + 1;
        let mut value = String::new();
        let mut from = k;
        loop {
            match b.get(k) {
                None | Some(b'\n' | b'\r') => {
                    return Err(Stop::Fault((
                        Span::new(open, open + 1),
                        CallFault::UnclosedQuote(char::from(quote)),
                    )));
                }
                Some(b'\\') if !matches!(b.get(k + 1), None | Some(b'\n' | b'\r')) => {
                    value.push_str(&self.text[from..k]);
                    let escaped = self.text[k + 1..].chars().next().unwrap_or_default();
                    value.push(escaped);
                    k += 1 + escaped.len_utf8();
                    from = k;
                }
                Some(&c) if c == quote => break,
                Some(_) => k += 1,
            }
        }
        value.push_str(&self.text[from..k]);
        self.i = k + 1;
        let style = match quote {
            b'\'' => Style::SingleQuoted,
            _ => Style::DoubleQuoted,
        };
        let sourced = Sourced::new(
            Value::Text(value),
            Origin::written(self.place(Span::new(open, k + 1)), style),
        );
        self.space()?;
        if !matches!(self.b.get(self.i), Some(b',' | b')')) {
            let tail = self.i;
            self.unquoted()?;
            let end = self.text[..self.i].trim_end().len();
            return Err(Stop::Fault((
                Span::new(tail, end.max(tail)),
                CallFault::TextAfterQuote,
            )));
        }
        Ok(sourced)
    }

    /// Runs to a `,` or `)` outside brackets, so a caption may hold a
    /// bracket of its own. Each further line's indentation and quotation
    /// markers are the line's, not the value's.
    fn unquoted(&mut self) -> Result<Sourced<Value>, Stop> {
        let b = self.b;
        let start = self.i;
        let mut text = String::new();
        let mut from = start;
        let mut depth = 0usize;
        let mut k = start;
        loop {
            match b.get(k) {
                None | Some(b'`') => return Err(Stop::Unclosed),
                Some(b'(') => depth += 1,
                Some(b')') if depth == 0 => break,
                Some(b')') => depth -= 1,
                Some(b',') if depth == 0 => break,
                Some(b'\n') => {
                    let next = past_prefix(b, k + 1, self.depth);
                    if blank_from(b, next) {
                        return Err(Stop::Unclosed);
                    }
                    text.push_str(self.text[from..k].trim_end_matches('\r'));
                    text.push('\n');
                    from = next;
                    k = next;
                    continue;
                }
                Some(_) => {}
            }
            k += 1;
        }
        text.push_str(&self.text[from..k]);
        self.i = k;
        let written = self.text[start..k].trim();
        let lead = self.text[start..k].len() - self.text[start..k].trim_start().len();
        let span = Span::new(start + lead, start + lead + written.len());
        Ok(Sourced::new(
            unquoted_value(text.trim()),
            Origin::written(self.place(span), Style::Plain),
        ))
    }

    /// Space between arguments, over line ends and onto the next line's
    /// writing, but never over a blank line.
    fn space(&mut self) -> Result<(), Stop> {
        loop {
            match self.b.get(self.i) {
                Some(b' ' | b'\t' | b'\r') => self.i += 1,
                Some(b'\n') => {
                    let next = past_prefix(self.b, self.i + 1, self.depth);
                    if blank_from(self.b, next) {
                        return Err(Stop::Unclosed);
                    }
                    self.i = next;
                }
                _ => return Ok(()),
            }
        }
    }

    fn place(&self, span: Span) -> Place {
        Place {
            source: self.source,
            span,
        }
    }
}

fn unquoted_value(text: &str) -> Value {
    match text {
        "true" | "True" | "TRUE" => return Value::Bool(true),
        "false" | "False" | "FALSE" => return Value::Bool(false),
        _ => {}
    }
    if is_number(text)
        && let Ok(n) = text.parse()
    {
        return Value::Number(n);
    }
    Value::Text(text.to_owned())
}

/// `-?[0-9]+(\.[0-9]+)?`
fn is_number(text: &str) -> bool {
    let unsigned = text.strip_prefix('-').unwrap_or(text);
    let (whole, fraction) = match unsigned.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (unsigned, None),
    };
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    digits(whole) && fraction.is_none_or(digits)
}

/// Where the writing on the line starting at `start` begins: past its
/// indentation and up to `depth` of a quotation's `>` markers.
pub(super) fn past_prefix(b: &[u8], start: usize, depth: usize) -> usize {
    let mut i = start;
    let mut markers = 0;
    loop {
        while matches!(b.get(i), Some(b' ' | b'\t')) {
            i += 1;
        }
        if markers < depth && b.get(i) == Some(&b'>') {
            markers += 1;
            i += 1;
            continue;
        }
        return i;
    }
}

/// How many quotation markers open the line starting at `start`.
pub(super) fn depth(b: &[u8], start: usize) -> usize {
    let mut i = start;
    let mut markers = 0;
    loop {
        while matches!(b.get(i), Some(b' ' | b'\t')) {
            i += 1;
        }
        if b.get(i) != Some(&b'>') {
            return markers;
        }
        markers += 1;
        i += 1;
    }
}

/// Whether nothing but space is left of the line from `i`, or the text
/// has ended.
pub(super) fn blank_from(b: &[u8], i: usize) -> bool {
    b[i.min(b.len())..]
        .iter()
        .take_while(|&&c| c != b'\n')
        .all(|c| matches!(c, b' ' | b'\t' | b'\r'))
}
