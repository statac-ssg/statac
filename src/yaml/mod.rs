//! YAML read into a tree where every node keeps the place it was written.
//!
//! Places are offsets into the whole source, so front matter read from a
//! Markdown file is placed in that file, not in an extracted string. The
//! text as written is never stored twice: `Sources::spelling` recovers it.

mod binary;
mod front_matter;
mod json;
mod parser;
mod schema;
#[cfg(test)]
mod tests;
mod tree;
pub mod write;

pub use front_matter::{Split, Unclosed, split};
pub use schema::{Resolved, keeps_characters, resolve};
pub use tree::{ALIAS_LIMIT, ALIAS_TEXT_LIMIT, Entry, Node, Partial, Tag, Value};

use crate::problem::{Kind, Problem, Word};
use crate::source::{Place, SourceId, Sources, Span};

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

/// Reads the YAML at `place` as an editor holds it while the author
/// types, reading on past each fault rather than stopping at the first.
pub fn read_partial(sources: &Sources, place: Place) -> Partial {
    tree::build_partial(sources, place)
}

/// Reads a Markdown file's front matter as an editor holds it while the
/// author types: front matter not yet closed runs to the end of the file.
/// None where the file has none.
pub fn front_matter_partial(sources: &Sources, source: SourceId) -> Option<Partial> {
    let (span, unclosed) = match split(sources.text(source)) {
        Ok(Split {
            front_matter: Some(span),
            ..
        }) => (span, None),
        Ok(_) => return None,
        Err(unclosed) => {
            let opening = Place {
                source,
                span: unclosed.opening,
            };
            (
                unclosed.rest,
                Some(Problem::at(Kind::UnclosedFrontMatter, opening)),
            )
        }
    };
    let mut partial = read_partial(sources, Place { source, span });
    if let Some(problem) = unclosed {
        partial.problems.insert(0, problem);
    }
    Some(partial)
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
