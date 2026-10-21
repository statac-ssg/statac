//! The YAML tree: every node, every mapping key included, with the place it
//! was written.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ops::Range;

use super::parser::{Event, Events, Fault};
use super::schema::{Resolved, resolve};
use crate::origin::{Origin, Style};
use crate::problem::Malformed;
use crate::problem::Repeated;
use crate::problem::{Kind, Problem};
use crate::source::{Place, Sources, Span};

/// The most nodes aliases may repeat in one document, so a few lines of
/// anchors and aliases can't expand into an enormous tree.
pub const ALIAS_LIMIT: usize = 10_000;
/// The most scalar text, in bytes, aliases may repeat in one document: a
/// few aliases of one long scalar would otherwise stay under the node limit.
pub const ALIAS_TEXT_LIMIT: usize = 1_000_000;

#[derive(Clone, PartialEq, Debug)]
pub struct Node {
    /// For an alias, the place of the anchor it repeats.
    pub place: Place,
    /// `Style::None` for a list or a mapping.
    pub style: Style,
    pub tag: Option<Tag>,
    pub value: Value,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Value {
    /// The text with quoting, escapes and folding undone. A plain scalar
    /// still has to be resolved to know what it means.
    Scalar(String),
    Sequence(Vec<Node>),
    Mapping(Vec<Entry>),
}

#[derive(Clone, PartialEq, Debug)]
pub struct Entry {
    pub key: Node,
    pub value: Node,
}

/// A tag from YAML's core schema, written on a node (`!!str 3.10`), or
/// `!!binary`. Any other tag is refused when read.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Tag {
    Str,
    Int,
    Float,
    Bool,
    Null,
    Seq,
    Map,
    /// `!!binary` whose bytes are UTF-8: the scalar is their text.
    Binary,
    /// `!!binary` whose bytes aren't text: the scalar is empty, and the
    /// value stands for bytes nothing can show.
    Bytes,
}

impl Node {
    pub fn origin(&self) -> Origin {
        Origin::written(self.place, self.style)
    }

    /// What YAML's standard reading makes of a scalar: by its tag where it
    /// has one, a string where it is quoted or a block, and otherwise by the
    /// core schema. None for a list or a mapping, and for a scalar whose
    /// text doesn't fit its tag (`!!int abc`).
    pub fn resolved(&self) -> Option<Resolved> {
        let Value::Scalar(text) = &self.value else {
            return None;
        };
        let resolved = match self.tag {
            Some(Tag::Str | Tag::Binary) => return Some(Resolved::Str),
            Some(Tag::Seq | Tag::Map | Tag::Bytes) => return None,
            None if self.style != Style::Plain => return Some(Resolved::Str),
            _ => resolve(text),
        };
        match (self.tag, resolved) {
            (None, _)
            | (Some(Tag::Null), Resolved::Null)
            | (Some(Tag::Bool), Resolved::Bool(_))
            | (Some(Tag::Int), Resolved::Int(_))
            | (Some(Tag::Float), Resolved::Float(_)) => Some(resolved),
            (Some(Tag::Float), Resolved::Int(Some(n))) => Some(Resolved::Float(n as f64)),
            _ => None,
        }
    }

    /// The value under a scalar key spelt `key`, in a mapping.
    pub fn get(&self, key: &str) -> Option<&Node> {
        let Value::Mapping(entries) = &self.value else {
            return None;
        };
        entries
            .iter()
            .find(|entry| matches!(&entry.key.value, Value::Scalar(text) if text == key))
            .map(|entry| &entry.value)
    }
}

/// The most bytes read again for one text to take the lines of its texts
/// in quotes that stand one space short: enough for any front matter or
/// settings many times over. Past it the line is told as the reader
/// tells it. Each reading again takes in a whole text in quotes, so the
/// work beside the reading stays in step with the bytes read.
const READ_AGAIN: usize = 16 * 1024 * 1024;

pub(super) fn build(sources: &Sources, at: Place) -> Result<Node, Problem> {
    let written = &sources.text(at.source)[at.span.range()];
    let mut mends = Mends::default();
    let mut read_again = 0;
    loop {
        let stopped = match mends.read(at, written) {
            Ok(node) => return Ok(node),
            Err(stopped) => stopped,
        };
        read_again += written.len();
        if read_again > READ_AGAIN || !mends.deepen(&stopped.why) {
            return Err(stopped.problem);
        }
    }
}

/// What could be read of YAML an author may be part way through typing.
#[derive(Clone, PartialEq, Debug)]
pub struct Partial {
    /// Every node read, placed as written: each quote and bracket left
    /// open is closed after what it holds, and each other stretch the
    /// reader stopped at is left out, with the lines after it deeper than
    /// its own.
    pub node: Node,
    /// The problem at each quote or bracket closed and each stretch left
    /// out, in the order the reader met them.
    pub problems: Vec<Problem>,
}

pub(super) fn build_partial(sources: &Sources, at: Place) -> Partial {
    let written = &sources.text(at.source)[at.span.range()];
    let mut mends = Mends::default();
    let mut problems = Vec::new();
    let mut read_again = 0;
    loop {
        let stopped = match mends.read(at, written) {
            Ok(node) => return Partial { node, problems },
            Err(stopped) => *stopped,
        };
        read_again += written.len();
        if read_again <= READ_AGAIN && mends.deepen(&stopped.why) {
            continue;
        }
        let base = at.span.range().start;
        let from = stopped
            .problem
            .place
            .map_or(written.len(), |place| place.span.range().start - base);
        problems.push(stopped.problem);
        if read_again > READ_AGAIN || !mends.step_past(written, from, &stopped.why) {
            return Partial {
                node: stopped.read,
                problems,
            };
        }
    }
}

/// What is changed in the text as written before it is read again.
#[derive(Default)]
struct Mends {
    /// The reader asks a later line of a text in quotes for one space
    /// more than YAML does. Where it stops at such a line the text is
    /// read again with a space before the line, and a line still refused
    /// then is one YAML refuses too. This is where each line given a
    /// space starts, as written.
    deepened: BTreeSet<usize>,
    /// The quotes and brackets put in to close what was left open, by
    /// where they go as written.
    closers: BTreeMap<usize, String>,
    /// Where each quote or bracket closed so stands, as written.
    closed: BTreeSet<usize>,
    /// The text as written with each stretch stepped past made spaces,
    /// once there is one.
    blanked: Option<String>,
}

impl Mends {
    fn read(&self, at: Place, written: &str) -> Result<Node, Box<Stopped>> {
        let written = self.blanked.as_deref().unwrap_or(written);
        let (text, added) = self.apply(written);
        read(at, text.as_deref().unwrap_or(written), &added)
    }

    /// `written` with each byte put in, and where each stands in it; none
    /// where nothing is put in.
    fn apply(&self, written: &str) -> (Option<String>, Vec<usize>) {
        if self.deepened.is_empty() && self.closers.is_empty() {
            return (None, Vec::new());
        }
        let mut puts: Vec<(usize, &str)> = self
            .closers
            .iter()
            .map(|(&at, closers)| (at, closers.as_str()))
            .chain(self.deepened.iter().map(|&line| (line, " ")))
            .collect();
        puts.sort_by_key(|&(at, _)| at);
        let mut text = String::with_capacity(written.len() + puts.len());
        let mut added = Vec::with_capacity(puts.len());
        let mut from = 0;
        for (at, put) in puts {
            text.push_str(&written[from..at]);
            added.extend(text.len()..text.len() + put.len());
            text.push_str(put);
            from = at;
        }
        text.push_str(&written[from..]);
        (Some(text), added)
    }

    /// Gives a space to the lines of a text in quotes the reader stopped
    /// at, where they haven't one already.
    fn deepen(&mut self, why: &Why) -> bool {
        let Why::Shallow(lines) = why else {
            return false;
        };
        // The line the reader stopped at is the first of them.
        if lines
            .first()
            .is_none_or(|line| self.deepened.contains(line))
        {
            return false;
        }
        self.deepened.extend(lines);
        true
    }

    /// Mends the text so that reading it again gets past where the reader
    /// stopped, at `from` as written: closes a quote or the brackets left
    /// open after what they hold, or else makes spaces of the stretch from
    /// there. False where nothing is left to mend.
    fn step_past(&mut self, written: &str, from: usize, why: &Why) -> bool {
        let text = self.blanked.as_deref().unwrap_or(written);
        if let Why::Other { open, unclosed } = why {
            let quote = unclosed.and_then(|at| match text.as_bytes()[at] {
                quote @ (b'"' | b'\'') => Some((at, char::from(quote))),
                _ => None,
            });
            if let Some((at, quote)) = quote
                && self.closed.insert(at)
            {
                let after = after_held(text, at);
                self.closers.entry(after).or_default().push(quote);
                return true;
            }
            if let (Some(&outer), Some(&inner)) = (open.first(), open.last()) {
                // A later line no deeper than the one the brackets open on
                // can't be inside them.
                let outside = stretch_end(text, outer);
                let unclosed = unclosed.is_some_and(|at| open.contains(&at));
                let new: Vec<usize> = open
                    .iter()
                    .copied()
                    .filter(|open| !self.closed.contains(open))
                    .collect();
                if (unclosed || from >= outside) && inner < outside && !new.is_empty() {
                    self.closed.extend(&new);
                    let closers: String = new
                        .iter()
                        .rev()
                        .map(|&open| match text.as_bytes()[open] {
                            b'[' => ']',
                            _ => '}',
                        })
                        .collect();
                    let after = after_held(text, outer);
                    self.closers.entry(after).or_default().push_str(&closers);
                    return true;
                }
            }
        }
        let end = stretch_end(text, from);
        let (from, end) = if text[from..end].contains(|c| !is_space(c)) {
            (from, end)
        } else {
            // Nothing is left there to mend: the last line before it with
            // something on it.
            let Some(last) = text[..from].rfind(|c| !is_space(c)) else {
                return false;
            };
            let line = text[..last].rfind('\n').map_or(0, |n| n + 1);
            let start = line + depth(&text[line..]);
            (start, stretch_end(text, start))
        };
        let mut blanked = String::with_capacity(text.len());
        blanked.push_str(&text[..from]);
        for c in text[from..end].chars() {
            match c {
                '\n' | '\r' => blanked.push(c),
                _ => blanked.extend(std::iter::repeat_n(' ', c.len_utf8())),
            }
        }
        blanked.push_str(&text[end..]);
        self.blanked = Some(blanked);
        // A quote or bracket put in to close what is now spaces would
        // stand alone.
        self.closers.retain(|&at, _| at < from || at > end);
        true
    }
}

/// Where the last of what is opened at `opens` ends, in the stretch of
/// its line and the lines after it deeper than its own.
fn after_held(text: &str, opens: usize) -> usize {
    opens + 1 + text[opens + 1..stretch_end(text, opens)].trim_end().len()
}

fn is_space(c: char) -> bool {
    matches!(c, ' ' | '\r' | '\n')
}

/// The spaces a line opens with.
fn depth(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

/// The end of the stretch made of the line holding `at` and the lines
/// after it deeper than it: the start of the first later line with
/// something on it that stands no deeper, or the end of the text.
fn stretch_end(text: &str, at: usize) -> usize {
    let start = text[..at].rfind('\n').map_or(0, |n| n + 1);
    let deepest = depth(&text[start..]);
    let mut line = text[at..].find('\n').map_or(text.len(), |n| at + n + 1);
    for written in text[line..].split_inclusive('\n') {
        if written.contains(|c| !is_space(c)) && depth(written) <= deepest {
            return line;
        }
        line += written.len();
    }
    text.len()
}

/// Why a text wasn't read, and what was read before.
struct Stopped {
    problem: Problem,
    why: Why,
    /// Each node read before the reader stopped, each list and mapping
    /// still open closed after the last.
    read: Node,
}

enum Why {
    /// At a later line of a text in quotes: the lines of the text, by
    /// where each starts as written, that a space more might mend, the
    /// one stopped at first.
    Shallow(Vec<usize>),
    Other {
        /// Where each bracket left open stands as written, outermost first.
        open: Vec<usize>,
        /// Where the quote or bracket stands, as written, that the reader
        /// found never closed, where that is what stopped it.
        unclosed: Option<usize>,
    },
}

/// Reads `text`, which is the text at `at` with a byte put in at each of
/// `added`. Every place is one in the text as written.
fn read(at: Place, text: &str, added: &[usize]) -> Result<Node, Box<Stopped>> {
    let mut builder = Builder {
        at,
        text,
        added,
        stack: Vec::new(),
        anchors: HashMap::new(),
        repeated: 0,
        repeated_bytes: 0,
        documents: 0,
        root: None,
        last_end: 0,
    };
    let mut events = Events::new(text);
    while let Some(event) = events.next() {
        let (problem, why) = match event {
            Ok(event) => match builder.event(event) {
                Ok(()) => continue,
                Err(problem) => (problem, builder.other(&events, None)),
            },
            Err(Fault::Malformed(
                Malformed::ShallowLine {
                    quote,
                    back,
                    spaces,
                },
                range,
            )) => {
                let (opens, line) = (range.start - back, range.start);
                let lines = super::parser::shallow_lines(text, quote, opens, line)
                    .into_iter()
                    .map(|line| builder.written(line))
                    .collect();
                let found = Malformed::ShallowLine {
                    quote,
                    back: builder.written(line) - builder.written(opens),
                    spaces,
                };
                let problem = builder.problem(Kind::MalformedYaml { found }, range);
                (problem, Why::Shallow(lines))
            }
            Err(Fault::Malformed(found, range)) => {
                let unclosed = matches!(
                    found,
                    Malformed::UnclosedBracket(_) | Malformed::UnclosedQuote(_)
                )
                .then(|| builder.written(range.start));
                let mut why = builder.other(&events, unclosed);
                // The bracket the reader found never closed may be one it
                // hadn't yet opened.
                if let Why::Other { open, .. } = &mut why
                    && let Malformed::UnclosedBracket(_) = found
                    && let Some(bracket) = unclosed
                    && !open.contains(&bracket)
                {
                    open.push(bracket);
                    open.sort_unstable();
                }
                (builder.problem(Kind::MalformedYaml { found }, range), why)
            }
            Err(Fault::UnknownTag(tag, range)) => (
                builder.problem(Kind::UnknownYamlTag { tag }, range),
                builder.other(&events, None),
            ),
        };
        let read = builder.into_read();
        return Err(Box::new(Stopped { problem, why, read }));
    }
    Ok(builder.into_read())
}

struct Frame {
    start: usize,
    tag: Option<Tag>,
    anchor: usize,
    value: Value,
    /// Nodes and scalar bytes inside so far, counted for the alias limits.
    size: usize,
    bytes: usize,
    key: Option<Node>,
    /// Each scalar key so far and where it was written.
    keys: HashMap<String, Place>,
}

struct Builder<'a> {
    at: Place,
    text: &'a str,
    /// Where each byte put into `text` stands, in order.
    added: &'a [usize],
    stack: Vec<Frame>,
    /// Each anchor's node, how many nodes it holds (itself included) and
    /// how many bytes of scalar text.
    anchors: HashMap<usize, (Node, usize, usize)>,
    repeated: usize,
    repeated_bytes: usize,
    documents: usize,
    root: Option<Node>,
    /// Where the last node read ends.
    last_end: usize,
}

impl Builder<'_> {
    fn event(&mut self, event: Event) -> Result<(), Problem> {
        match event {
            Event::DocumentStart(span) => {
                self.documents += 1;
                if self.documents > 1 {
                    let found = Malformed::SecondDocument;
                    return Err(self.problem(Kind::MalformedYaml { found }, span));
                }
            }
            Event::Scalar {
                mut text,
                style,
                anchor,
                mut tag,
                span,
            } => {
                if tag == Some(Tag::Binary) {
                    let Some(decoded) = super::binary::decode(&text) else {
                        let found = Malformed::NotBase64;
                        return Err(self.problem(Kind::MalformedYaml { found }, span));
                    };
                    match String::from_utf8(decoded) {
                        Ok(decoded) => text = decoded,
                        Err(_) => {
                            text = String::new();
                            tag = Some(Tag::Bytes);
                        }
                    }
                }
                let bytes = text.len();
                self.last_end = span.end;
                let node = Node {
                    place: self.place(span),
                    style,
                    tag,
                    value: Value::Scalar(text),
                };
                self.finish(node, 1, bytes, anchor)?;
            }
            Event::Alias { anchor, span } => {
                let Some(&(_, size, bytes)) = self.anchors.get(&anchor) else {
                    let found = Malformed::UnknownAnchor(self.text[span.clone()].to_owned());
                    return Err(self.problem(Kind::MalformedYaml { found }, span));
                };
                // Checked before the anchor's node is copied, so passing a limit costs nothing.
                self.repeated += size;
                self.repeated_bytes += bytes;
                let passed = if self.repeated > ALIAS_LIMIT {
                    Some(Repeated::Values(ALIAS_LIMIT))
                } else if self.repeated_bytes > ALIAS_TEXT_LIMIT {
                    Some(Repeated::TextBytes(ALIAS_TEXT_LIMIT))
                } else {
                    None
                };
                if let Some(limit) = passed {
                    let alias = self.text[span.clone()].to_owned();
                    return Err(self.problem(Kind::AliasLimit { alias, limit }, span));
                }
                let node = self.anchors[&anchor].0.clone();
                self.last_end = span.end;
                self.add(node, size, bytes)?;
            }
            Event::SequenceStart { tag, start, .. } | Event::MappingStart { tag, start, .. }
                if tag == Some(Tag::Binary) =>
            {
                let found = Malformed::NotBase64;
                return Err(self.problem(Kind::MalformedYaml { found }, start..start + 1));
            }
            Event::SequenceStart { anchor, tag, start } => {
                self.open(start, tag, anchor, Value::Sequence(Vec::new()));
            }
            Event::MappingStart { anchor, tag, start } => {
                self.open(start, tag, anchor, Value::Mapping(Vec::new()));
            }
            Event::End { end } => {
                let frame = self.stack.pop().expect("the parser balances its events");
                self.last_end = end;
                let node = Node {
                    place: self.place(frame.start..end),
                    style: Style::None,
                    tag: frame.tag,
                    value: frame.value,
                };
                self.finish(node, frame.size + 1, frame.bytes, frame.anchor)?;
            }
        }
        Ok(())
    }

    fn open(&mut self, start: usize, tag: Option<Tag>, anchor: usize, value: Value) {
        self.stack.push(Frame {
            start,
            tag,
            anchor,
            value,
            size: 0,
            bytes: 0,
            key: None,
            keys: HashMap::new(),
        });
    }

    fn finish(
        &mut self,
        node: Node,
        size: usize,
        bytes: usize,
        anchor: usize,
    ) -> Result<(), Problem> {
        if anchor != 0 {
            self.anchors.insert(anchor, (node.clone(), size, bytes));
        }
        self.add(node, size, bytes)
    }

    fn add(&mut self, node: Node, size: usize, bytes: usize) -> Result<(), Problem> {
        let Some(frame) = self.stack.last_mut() else {
            self.root = Some(node);
            return Ok(());
        };
        frame.size += size;
        frame.bytes += bytes;
        match &mut frame.value {
            Value::Mapping(entries) => match frame.key.take() {
                Some(key) => entries.push(Entry { key, value: node }),
                None => {
                    if let Value::Scalar(text) = &node.value {
                        if let Some(&first) = frame.keys.get(text) {
                            let kind = Kind::DuplicateKey {
                                key: text.clone(),
                                first,
                            };
                            return Err(Problem::at(kind, node.place));
                        }
                        frame.keys.insert(text.clone(), node.place);
                    }
                    frame.key = Some(node);
                }
            },
            Value::Sequence(items) => items.push(node),
            Value::Scalar(_) => unreachable!("only lists and mappings are opened"),
        }
        Ok(())
    }

    /// A range of the text being read, placed in its whole source.
    /// Where the byte `at` of `text` stands in the text as written.
    fn written(&self, at: usize) -> usize {
        at - self.added.partition_point(|added| *added < at)
    }

    fn place(&self, range: Range<usize>) -> Place {
        let base = self.at.span.start as usize;
        let (start, end) = (self.written(range.start), self.written(range.end));
        Place {
            source: self.at.source,
            span: Span::new(base + start, base + end),
        }
    }

    fn problem(&self, kind: Kind, range: Range<usize>) -> Problem {
        Problem::at(kind, self.place(range))
    }

    fn other(&self, events: &Events, unclosed: Option<usize>) -> Why {
        let open = events.open_brackets().map(|at| self.written(at)).collect();
        Why::Other { open, unclosed }
    }

    /// The tree read so far: each list and mapping still open closed after
    /// the last node read, and a key with no value yet given an empty one
    /// there.
    fn into_read(mut self) -> Node {
        while let Some(mut frame) = self.stack.pop() {
            let end = self.last_end.max(frame.start);
            if let (Value::Mapping(entries), Some(key)) = (&mut frame.value, frame.key.take()) {
                let value = Node {
                    place: self.place(end..end),
                    style: Style::Plain,
                    tag: None,
                    value: Value::Scalar(String::new()),
                };
                entries.push(Entry { key, value });
            }
            let node = Node {
                place: self.place(frame.start..end),
                style: Style::None,
                tag: frame.tag,
                value: frame.value,
            };
            self.last_end = end;
            self.add(node, frame.size + 1, frame.bytes)
                .expect("only a scalar key can be one given twice");
        }
        match self.root {
            Some(node) => node,
            None => Node {
                place: self.place(0..0),
                style: Style::Plain,
                tag: None,
                value: Value::Scalar(String::new()),
            },
        }
    }
}
