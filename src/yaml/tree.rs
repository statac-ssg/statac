//! The YAML tree: every node, every mapping key included, with the place it
//! was written.

use std::collections::HashMap;
use std::ops::Range;

use super::Malformed;
use super::parser::{Event, Events, Fault};
use super::schema::{Resolved, resolve};
use crate::origin::{Origin, Style};
use crate::problem::{Kind, Problem};
use crate::source::{Place, Sources, Span};

/// The most nodes aliases may repeat in one document, so a few lines of
/// anchors and aliases can't expand into an enormous tree.
pub const ALIAS_LIMIT: usize = 10_000;
/// The most scalar text, in bytes, aliases may repeat in one document: a
/// few aliases of one long scalar would otherwise stay under the node limit.
pub const ALIAS_TEXT_LIMIT: usize = 1_000_000;

/// Which of the two alias limits was passed.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Repeated {
    Values(usize),
    TextBytes(usize),
}

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

pub(super) fn build(sources: &Sources, at: Place) -> Result<Node, Problem> {
    let text = &sources.text(at.source)[at.span.range()];
    let mut builder = Builder {
        at,
        text,
        stack: Vec::new(),
        anchors: HashMap::new(),
        repeated: 0,
        repeated_bytes: 0,
        documents: 0,
        root: None,
    };
    for event in Events::new(text) {
        match event {
            Ok(event) => builder.event(event)?,
            Err(Fault::Malformed(found, range)) => {
                return Err(builder.problem(Kind::MalformedYaml { found }, range));
            }
            Err(Fault::UnknownTag(tag, range)) => {
                return Err(builder.problem(Kind::UnknownYamlTag { tag }, range));
            }
        }
    }
    Ok(match builder.root {
        Some(node) => node,
        None => Node {
            place: builder.place(0..0),
            style: Style::Plain,
            tag: None,
            value: Value::Scalar(String::new()),
        },
    })
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
    stack: Vec<Frame>,
    /// Each anchor's node, how many nodes it holds (itself included) and
    /// how many bytes of scalar text.
    anchors: HashMap<usize, (Node, usize, usize)>,
    repeated: usize,
    repeated_bytes: usize,
    documents: usize,
    root: Option<Node>,
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
    fn place(&self, range: Range<usize>) -> Place {
        let base = self.at.span.start as usize;
        Place {
            source: self.at.source,
            span: Span::new(base + range.start, base + range.end),
        }
    }

    fn problem(&self, kind: Kind, range: Range<usize>) -> Problem {
        Problem::at(kind, self.place(range))
    }
}
