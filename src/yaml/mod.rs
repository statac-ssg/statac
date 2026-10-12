//! YAML read into a tree where every node keeps the place it was written,
//! and a serde `Deserializer` over that tree.
//!
//! Places are offsets into the whole source, so front matter read from a
//! Markdown file is placed in that file, not in an extracted string. The
//! text as written is never stored twice: `Sources::spelling` recovers it.

mod binary;
mod de;
mod front_matter;
mod json;
mod parser;
mod schema;
#[cfg(test)]
mod tests;
mod tree;

pub use de::{Error, Unknown, from_node, from_node_noting_unknown};
pub use front_matter::{Split, Unclosed, split};
pub use json::JsonFault;
pub use schema::{Resolved, keeps_characters, resolve};
pub use tree::{ALIAS_LIMIT, ALIAS_TEXT_LIMIT, Entry, Node, Repeated, Tag, Value};

use crate::problem::{Kind, Problem, Word};
use crate::source::{Place, SourceId, Sources, Span};

/// What the parser found where the YAML stops making sense.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Malformed {
    /// Placed at the opening quote, not where the parser gave up.
    UnclosedQuote(char),
    /// A later line of a text in quotes that does close, standing too far
    /// left for the reader to take as more of the text. Placed at the
    /// line: the quote opens `back` bytes before it, and `spaces` before
    /// each later line are enough.
    ShallowLine {
        quote: char,
        back: usize,
        spaces: usize,
    },
    UnclosedBracket(char),
    MismatchedBracket {
        open: char,
        close: char,
    },
    /// An escape in double quotes YAML doesn't define, as written (`\q`).
    UnknownEscape(String),
    TextAfterQuote,
    Indentation,
    Tab,
    /// An alias to no anchor, as written (`*name`).
    UnknownAnchor(String),
    SecondDocument,
    TooDeep,
    EndsEarly,
    /// A character YAML allows nowhere, such as a C1 control left by text
    /// saved in an old encoding.
    ControlCharacter(char),
    /// Text the parser couldn't place, as written.
    Unexpected(String),
    /// A value tagged `!!binary` that isn't base64.
    NotBase64,
}

/// Whether a later line of a text in quotes, `line` after its `spaces`,
/// standing no deeper than the line the text opens on, is surely more of
/// the text: not a line that reads as a field or an item of a list, which
/// a quote never closed would take in, nor the mark that ends a document.
pub(crate) fn reads_as_text(line: &str, spaces: usize) -> bool {
    let word = line.split(' ').next().unwrap_or(line);
    if word == "-" || (spaces == 0 && matches!(word, "---" | "...")) {
        return false;
    }
    let name = match line.split_once(": ") {
        Some((name, _)) => name,
        None => match line.trim_end().strip_suffix(':') {
            Some(name) => name,
            None => return true,
        },
    };
    name.is_empty() || (name.contains(' ') && !name.starts_with(['"', '\'']))
}

/// Reads the whole of a source as one YAML document.
pub fn read(sources: &Sources, source: SourceId) -> Result<Node, Problem> {
    let span = Span::new(0, sources.text(source).len());
    read_at(sources, Place { source, span })
}

/// Reads the whole of a source as one JSON value, into the tree YAML is
/// read into.
pub fn read_json(sources: &Sources, source: SourceId) -> Result<Node, Problem> {
    let text = sources.text(source);
    let place = Place {
        source,
        span: Span::new(0, text.len()),
    };
    json::read(text, place)
}

/// Reads the YAML at `place`, one document, placing every node in the
/// whole source.
pub fn read_at(sources: &Sources, place: Place) -> Result<Node, Problem> {
    tree::build(sources, place)
}

/// Reads a Markdown file's front matter, if it has any, and gives the span
/// of its body.
pub fn front_matter(sources: &Sources, source: SourceId) -> Result<(Option<Node>, Span), Problem> {
    let split = split(sources.text(source)).map_err(|unclosed| {
        Problem::at(
            Kind::UnclosedFrontMatter,
            Place {
                source,
                span: unclosed.opening,
            },
        )
    })?;
    let node = match split.front_matter {
        Some(span) => Some(read_at(sources, Place { source, span })?),
        None => {
            if let Some(span) = front_matter::plus_opening(sources.text(source)) {
                let kind = Kind::OtherSpelling {
                    word: Word::Marker,
                    written: "+++".into(),
                    ours: "---".into(),
                };
                return Err(Problem::at(kind, Place { source, span }));
            }
            None
        }
    };
    Ok((node, split.body))
}
