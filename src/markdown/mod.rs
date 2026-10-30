//! Markdown read into Statac's own tree, every node placed in the source
//! file, and written out as HTML.
//!
//! pulldown-cmark reads the Markdown. Statac's own syntax, snippet calls
//! and dollar maths, is found and masked before it does (see `mask`), and
//! comes back as nodes of its own. A call that wraps writing up to an
//! `@end` (see `block`) holds that writing, read as Markdown of its own, as
//! its children. An `@url(...)` is no snippet's: it is read as the address
//! it stands for inside the site, in a link's address as anywhere a call
//! is read. A control character is left out before the parser reads
//! the text (see `controls`). The author's raw HTML is followed through
//! the elements it opens and closes as it is read, to tell a tag that
//! reaches past the writing (see `unmatched`). Passes over the finished
//! tree change it in place, and reading ends with the one that links each
//! address written bare (see `autolinks`); questions about the writing
//! (its words, links and headings) are functions over it.

mod autolinks;
mod block;
mod call;
mod code_lines;
mod controls;
mod html;
mod ids;
mod mask;
mod maths;
mod node;
mod notes;
mod opening;
mod pieces;
mod questions;
mod read;
#[cfg(test)]
mod tests;
mod typing;
mod typography;
mod unmatched;

pub(crate) use call::{Call, IN_SITE, Value, Within, from_the_top};
pub(crate) use code_lines::LINE_WORDS;
pub(crate) use html::escape_href;
pub(crate) use html::{Copies, Options, Picture, Placement, Snippets, to_html, write};
pub(crate) use ids::give_heading_ids;
pub use node::{Alignment, CalloutKind, Kind, Node};
pub(crate) use opening::{
    Called, Written, as_written, closed_up, lines_of, references_read, words_of,
};
pub(crate) use pieces::{Piece, PieceValue, language_word};
pub(crate) use questions::{has_callout, headings, uses_maths};
#[cfg(test)]
pub(crate) use questions::{links, plain_text};
pub(crate) use typing::{in_a_definition, opens_a_fence, reads_a_call, reads_a_link};
pub use typography::{Typography, set_typography, set_words};

use crate::problem::CallFault;
use crate::problem::{Kind as ProblemKind, Problem, PublishedAsText};
use crate::source::{Place, SourceId, Sources, Span};

/// What each snippet does with the writing a call stands above: `Some(true)`
/// for one that wraps it up to an `@end`, `Some(false)` for one that
/// doesn't, and none for a name the site has no snippet for, or one whose
/// snippet can't be read.
pub(crate) type Wraps<'a> = &'a dyn Fn(&str) -> Option<bool>;

pub struct Document {
    /// A `Kind::Document` holding the writing in the order written, without
    /// its footnotes.
    pub root: Node,
    /// The footnotes the writing points at, in the order of their numbers.
    pub notes: Vec<Node>,
    /// Every line defining a link reference, `[label]: /address`.
    pub(crate) definitions: Vec<Definition>,
    /// Every `@url(...)` read as the address it stands for, in the order
    /// written.
    pub(crate) in_site: Vec<InSite>,
    pub(crate) problems: Vec<Problem>,
    /// What raw HTML leaves open inside a paragraph at the top of the
    /// writing, which the paragraph's end closes: told of a value shown as
    /// that paragraph's words alone, without its `<p>`.
    pub(crate) held_in_paragraph: Vec<Problem>,
    /// Each piece of raw HTML opening with a comment inside an element the
    /// raw HTML opened before it and left open: where the piece starts,
    /// and the innermost such element's name as written.
    pub(crate) held_open: Vec<(u32, String)>,
}

impl Document {
    /// Where the `@url(...)` standing for `address` is written within
    /// `place`: the last there, as a link's address follows its words.
    pub(crate) fn in_site_at(&self, address: &str, place: Place) -> Option<Place> {
        let within = |stands: &&InSite| {
            let at = stands.place.span;
            place.span.start <= at.start && at.end <= place.span.end
        };
        let mut written = self.in_site.iter().rev().filter(within);
        written
            .find(|stands| stands.address == address)
            .map(|stands| stands.place)
    }
}

/// An `@url(...)` in the writing: an address inside the site, wherever the
/// site is published.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct InSite {
    /// The address it stands for, the site's own path included.
    pub(crate) address: String,
    /// The call as written.
    pub(crate) place: Place,
}

/// A line defining a link reference, which every link naming its label
/// follows.
#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Definition {
    pub(crate) label: String,
    pub(crate) destination: String,
    /// The whole line.
    pub(crate) place: Place,
}

/// What reading writing tells that is told of a value a template reads
/// as Markdown too. A call there is shown as written and a value has no
/// excerpt or heading ids, so what reading a call, a more marker or an id
/// meets is not.
const TOLD_OF_A_VALUE: &[crate::problem::Code] = &[
    crate::problem::Code::MalformedMaths,
    crate::problem::Code::ControlCharacter,
    crate::problem::Code::NestedTooDeep,
    crate::problem::Code::MissingDefinition,
    crate::problem::Code::FootnoteNeverShown,
    crate::problem::Code::DefinitionWrittenTwice,
    crate::problem::Code::UnmatchedTag,
    crate::problem::Code::WrongKind,
    crate::problem::Code::NoSuchLine,
];

/// Whether a problem met reading writing is told of a value a template
/// reads as Markdown too. An `@url(...)` is read in one, so one handed
/// the wrong thing is told.
pub(crate) fn told_of_a_value(kind: &ProblemKind) -> bool {
    let not_one_address = ProblemKind::MalformedCall {
        found: CallFault::NotOneAddress,
    };
    TOLD_OF_A_VALUE.contains(&kind.code()) || *kind == not_one_address
}

/// Reads the Markdown at `body` in `source`, every node placed in the whole
/// file, numbers its footnotes, makes its maths MathML and tells the more
/// markers and maths it can't act on, and the words asking how a code
/// block's lines are shown.
/// `wraps` says which snippets wrap the writing below a call to them, and
/// `base` is the path the site is published under, which an `@url(...)`
/// puts before its address.
/// Definitions are the page's, wherever on it they are written.
pub fn read(
    sources: &Sources,
    source: SourceId,
    body: Span,
    wraps: Wraps<'_>,
    base: &str,
) -> Document {
    let text = sources.text(source);
    let masked = mask::mask(text, source, body, wraps, 0);
    let page = masked
        .wraps_writing()
        .then(|| read::Definitions::gather(text, &masked, base));
    let mut document = read::build(text, source, body, &masked, page.as_ref(), base);
    notes::number(&mut document, text);
    let read = document.problems.len();
    maths::write(&mut document, sources.text(source));
    let misplaced = opening::misplaced_markers(&document, sources.text(source));
    document.problems.extend(misplaced);
    let others = another_tools(&document, text, body);
    document.problems.extend(others);
    let lines = code_lines::problems(&document, text);
    document.problems.extend(lines);
    if document.problems.len() > read {
        document
            .problems
            .sort_by_key(|p| p.place.map(|place| place.span.start));
    }
    autolinks::link(&mut document, sources);
    document
}

/// Another tool's calls and marks in the writing, which reach the page as
/// text. The parser can split one into words and HTML, so each is found in
/// what is written, and told only where the tree has it opening and closing
/// among the words: not in code or text shown as typed.
fn another_tools(document: &Document, text: &str, body: Span) -> Vec<Problem> {
    let end = body.end as usize;
    let b = &text.as_bytes()[..end];
    let mut found = Vec::new();
    let mut at = body.start as usize;
    let mut on = mask::Line::default();
    while let Some(open) = text[at..end].find('{').map(|p| at + p) {
        let bounds = on.of(b, open);
        let line = &text[open..bounds.1];
        let escaped = b[..open].iter().rev().take_while(|&&c| c == b'\\').count() % 2 == 1;
        // Braces closing a heading that aren't an id are the heading's words.
        let heading = mask::heading_brace(b, body.start as usize, open, bounds).is_some();
        match another_tools_braces(line).filter(|_| !escaped && !heading) {
            Some((len, why)) if why != PublishedAsText::Mark || alone_on_line(text, open, len) => {
                found.push((Span::new(open, open + len), why));
                at = open + len;
            }
            _ => at = open + if line.starts_with("{{") { 2 } else { 1 },
        }
    }
    if found.is_empty() {
        return Vec::new();
    }
    let mut words = Vec::new();
    for top in std::iter::once(&document.root).chain(&document.notes) {
        top.walk(&mut |node| {
            if let Kind::Text(_) = node.kind {
                words.push(node.place.span);
            }
        });
    }
    let among_words = |at: u32| words.iter().any(|w| w.start <= at && at < w.end);
    found
        .into_iter()
        .filter(|(span, _)| among_words(span.start) && among_words(span.end - 1))
        .map(|(span, why)| {
            let kind = ProblemKind::PublishedAsText {
                written: text[span.range()].to_owned(),
                why,
            };
            let source = document.root.place.source;
            Problem::at(kind, Place { source, span })
        })
        .collect()
}

/// The length of another tool's call or mark opening `line`, and which it
/// is: `{{< name >}}`, `{{ <name /> }}`, `{{% name %}}`, `{% name %}` or
/// `{: name}`, closed on the line and holding something.
fn another_tools_braces(line: &str) -> Option<(usize, PublishedAsText)> {
    // Tera's component call, with space before its tag; without, it opens
    // `{{<` as below. Doubled braces holding anything else may be words.
    if let Some(rest) = line.strip_prefix("{{")
        && let tag = rest.trim_start_matches([' ', '\t'])
        && tag.len() < rest.len()
        && let Some(name) = tag.strip_prefix('<')
        && name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
    {
        let inside = rest.find("}}")? + 2;
        return Some((inside + 2, PublishedAsText::Call));
    }
    let (open, close, why) = [
        ("{{<", ">}}", PublishedAsText::Call),
        ("{{%", "%}}", PublishedAsText::Call),
        ("{%", "%}", PublishedAsText::Call),
        ("{:", "}", PublishedAsText::Mark),
    ]
    .into_iter()
    .find(|(open, ..)| line.starts_with(open))?;
    let inside = line[open.len()..].find(close)? + open.len();
    let held = !line[open.len()..inside].trim().is_empty();
    held.then_some((inside + close.len(), why))
}

/// Whether the `len` bytes at `at` stand alone on their line, after any
/// indentation and quotation markers.
fn alone_on_line(text: &str, at: usize, len: usize) -> bool {
    let start = text[..at].rfind('\n').map_or(0, |p| p + 1);
    let end = text[at..].find('\n').map_or(text.len(), |p| at + p);
    text[start..at]
        .bytes()
        .all(|c| matches!(c, b' ' | b'\t' | b'>'))
        && text[at + len..end].trim().is_empty()
}
