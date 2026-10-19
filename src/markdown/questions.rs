//! Questions asked of the finished tree.

use super::Document;
use super::node::{Kind, Node};
use super::opening::UNSEEN;
use crate::html::Markup;
use crate::source::Place;

/// The words of `node` as plain text, for headings and opening words:
/// writing, code and maths as written, each block on a line of its own.
/// Raw HTML, calls, footnote markers and pictures' descriptions are no
/// part of it.
pub fn plain_text(node: &Node) -> String {
    text(node, false)
}

/// [`plain_text`] without the text of the elements a browser doesn't show,
/// for the words a reader reads.
pub(super) fn seen_text(node: &Node) -> String {
    text(node, true)
}

fn text(node: &Node, seen: bool) -> String {
    let mut out = String::new();
    text_of(node, &mut out, seen);
    let kept = out.trim_end().len();
    out.truncate(kept);
    out
}

fn text_of(node: &Node, out: &mut String, seen: bool) {
    match &node.kind {
        Kind::Text(text) | Kind::Literal(text) | Kind::Code(text) | Kind::Maths { text, .. } => {
            out.push_str(text)
        }
        Kind::CodeBlock { text, .. } => {
            out.push_str(text.trim_end_matches('\n'));
            out.push('\n');
        }
        Kind::SoftBreak | Kind::HardBreak => out.push(' '),
        Kind::Html(_)
        | Kind::Call(_)
        | Kind::FootnoteReference { .. }
        | Kind::Image { .. }
        | Kind::Task { .. }
        | Kind::Rule => {}
        _ => {
            // The element of the author's HTML open among the words whose
            // text isn't shown.
            let mut unseen: Option<String> = None;
            for child in &node.children {
                if let (true, Kind::Html(html)) = (seen, &child.kind) {
                    for (_, piece) in crate::html::markup(html) {
                        match piece {
                            Markup::Start(tag)
                                if unseen.is_none() && UNSEEN.contains(&tag.name.as_str()) =>
                            {
                                unseen = Some(tag.name);
                            }
                            Markup::End(tag) if unseen.as_ref() == Some(&tag.name) => unseen = None,
                            _ => {}
                        }
                    }
                }
                if unseen.is_none() {
                    text_of(child, out, seen);
                }
            }
            let holds_words =
                !node.is_inline() && node.children.first().is_some_and(Node::is_inline);
            if holds_words && !out.ends_with('\n') {
                out.push('\n');
            }
        }
    }
}

/// Whether the writing holds any maths, its footnotes and the writing its
/// calls wrap included.
pub fn uses_maths(document: &Document) -> bool {
    let mut found = false;
    for top in std::iter::once(&document.root).chain(&document.notes) {
        top.walk(&mut |node| found |= matches!(node.kind, Kind::Maths { .. }));
    }
    found
}

/// A link or a picture in the writing.
#[derive(Clone, PartialEq, Debug)]
pub struct Link<'a> {
    pub destination: &'a str,
    /// The whole link or picture as written.
    pub place: Place,
    pub picture: bool,
}

/// Every link and picture, in the order written, the footnotes' last.
pub fn links(document: &Document) -> Vec<Link<'_>> {
    let mut out = Vec::new();
    for top in std::iter::once(&document.root).chain(&document.notes) {
        top.walk(&mut |node| {
            if let Kind::Link { destination, .. } | Kind::Image { destination, .. } = &node.kind {
                out.push(Link {
                    destination,
                    place: node.place,
                    picture: matches!(node.kind, Kind::Image { .. }),
                });
            }
        });
    }
    out
}

/// Whether the writing holds a callout, its footnotes included.
pub fn has_callout(document: &Document) -> bool {
    let mut found = false;
    for top in std::iter::once(&document.root).chain(&document.notes) {
        top.walk(&mut |node| found |= matches!(node.kind, Kind::Callout(_)));
    }
    found
}

#[derive(Clone, PartialEq, Debug)]
pub struct Heading<'a> {
    pub level: u8,
    /// None until heading ids are given.
    pub id: Option<&'a str>,
    /// Its words, as plain text.
    pub text: String,
    pub place: Place,
}

/// Every heading in the writing, in the order written.
pub fn headings(document: &Document) -> Vec<Heading<'_>> {
    let mut out = Vec::new();
    document.root.walk(&mut |node| {
        if let Kind::Heading { level, id, .. } = &node.kind {
            out.push(Heading {
                level: *level,
                id: id.as_deref(),
                text: plain_text(node),
                place: node.place,
            });
        }
    });
    out
}
