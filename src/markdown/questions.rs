//! Questions asked of the finished tree.

use super::Document;
use super::node::{Kind, Node};
use super::opening::UNSEEN;
use crate::html::Markup;
use crate::source::Place;

/// The words of `node` as plain text, for headings and opening words:
/// writing, code and maths as written, each block on a line of its own.
/// Raw HTML, calls, footnote markers and pictures' descriptions are no
/// part of it, nor is the text of the elements a browser doesn't show.
pub(crate) fn plain_text(node: &Node) -> String {
    let mut out = String::new();
    text_of(node, &mut out);
    let kept = out.trim_end().len();
    out.truncate(kept);
    out
}

fn text_of(node: &Node, out: &mut String) {
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
                if let Kind::Html(html) = &child.kind {
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
                    text_of(child, out);
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
pub(crate) fn uses_maths(document: &Document) -> bool {
    let mut found = false;
    for top in std::iter::once(&document.root).chain(&document.notes) {
        top.walk(&mut |node| found |= matches!(node.kind, Kind::Maths { .. }));
    }
    found
}

/// A link or a picture in the writing.
#[derive(Clone, PartialEq, Debug)]
#[cfg(test)]
pub(crate) struct Link<'a> {
    pub(crate) destination: &'a str,
    /// The whole link or picture as written.
    pub(crate) place: Place,
    pub(crate) picture: bool,
}

/// Every link and picture, in the order written, the footnotes' last.
#[cfg(test)]
pub(crate) fn links(document: &Document) -> Vec<Link<'_>> {
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
pub(crate) fn has_callout(document: &Document) -> bool {
    let mut found = false;
    for top in std::iter::once(&document.root).chain(&document.notes) {
        top.walk(&mut |node| found |= matches!(node.kind, Kind::Callout(_)));
    }
    found
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct Heading<'a> {
    pub(crate) level: u8,
    /// None until heading ids are given.
    pub(crate) id: Option<&'a str>,
    /// Its words, as plain text.
    pub(crate) text: String,
    pub(crate) place: Place,
}

/// Every heading in the writing, in the order written.
pub(crate) fn headings(document: &Document) -> Vec<Heading<'_>> {
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
