//! An id for every heading, so a link can land on the part of the page it
//! means.

use std::collections::{HashMap, HashSet};

use super::Document;
use super::node::{Kind, Node};
use super::questions::plain_text;
use crate::content::slug::{anchor, made_slug};
use crate::problem::{Kind as ProblemKind, Problem, Taken};
use crate::source::{Place, Span};

/// Gives each heading without an id of the author's an id from its words,
/// the headings in the writing first and then those in its footnotes. Two
/// headings with the same words are told apart by a number, and none takes
/// an id a footnote has or the author gave another heading. An id the
/// author gave is theirs and kept as written, given twice or not, and
/// told where an earlier heading or a footnote has it, as links to it land
/// on that. Words that make no id, a heading of punctuation alone, are
/// named from `path`, the file the writing is in, and the heading's place
/// among them. `text` is that file's.
pub fn give_heading_ids(document: &mut Document, path: &str, text: &str) {
    let mut ids = Ids::default();
    let tops = || std::iter::once(&document.root).chain(&document.notes);
    for top in tops() {
        top.walk(&mut |node| match &node.kind {
            Kind::FootnoteDefinition { number, .. } => {
                ids.taken.insert(format!("note-{number}"));
            }
            Kind::FootnoteReference { number, repeat, .. } => {
                ids.taken.insert(match repeat {
                    0 | 1 => format!("back-{number}"),
                    n => format!("back-{number}-{n}"),
                });
            }
            _ => {}
        });
    }
    let footnotes = ids.taken.clone();
    let mut written: HashMap<String, Place> = HashMap::new();
    let mut twice = Vec::new();
    for top in tops() {
        top.walk(&mut |node| {
            let Kind::Heading { id: Some(id), .. } = &node.kind else {
                return;
            };
            let taken = match written.get(id) {
                Some(first) => Some(Taken::Heading(*first)),
                None if footnotes.contains(id) => Some(Taken::Footnote),
                None => None,
            };
            written.entry(id.clone()).or_insert(node.place);
            ids.taken.insert(id.clone());
            if let Some(taken) = taken {
                let kind = ProblemKind::IdUsedTwice {
                    id: id.clone(),
                    taken,
                };
                twice.push(Problem::at(kind, written_at(node.place, id, text)));
            }
        });
    }
    if !twice.is_empty() {
        document.problems.extend(twice);
        document
            .problems
            .sort_by_key(|p| p.place.map(|place| place.span.start));
    }
    let mut count = 0;
    let mut give = |node: &mut Node| {
        let Kind::Heading { id, .. } = &node.kind else {
            return;
        };
        count += 1;
        if id.is_some() {
            return;
        }
        let words = anchor(&plain_text(node));
        let wanted = match words.is_empty() {
            true => made_slug(&format!("{path}#{count}")),
            false => words,
        };
        let minted = ids.mint(wanted);
        if let Kind::Heading { id, .. } = &mut node.kind {
            *id = Some(minted);
        }
    };
    document.root.walk_mut(&mut give);
    for note in &mut document.notes {
        note.walk_mut(&mut give);
    }
}

/// Where a heading written at `heading` writes the id `id`: the `#id` in
/// its braces, or the whole heading where that can't be found.
fn written_at(heading: Place, id: &str, text: &str) -> Place {
    let range = heading.span.range();
    let written = format!("#{id}");
    match text[range.clone()].rfind(&written) {
        Some(at) => Place {
            source: heading.source,
            span: Span::new(range.start + at, range.start + at + written.len()),
        },
        None => heading,
    }
}

#[derive(Default)]
struct Ids {
    taken: HashSet<String>,
    /// The number to try next after each id wanted twice.
    next: HashMap<String, usize>,
}

impl Ids {
    /// `wanted` where it is free, and numbered where it isn't.
    fn mint(&mut self, wanted: String) -> String {
        if self.taken.insert(wanted.clone()) {
            return wanted;
        }
        let mut number = self.next.get(&wanted).copied().unwrap_or(1);
        loop {
            let id = format!("{wanted}-{number}");
            number += 1;
            if self.taken.insert(id.clone()) {
                self.next.insert(wanted, number);
                return id;
            }
        }
    }
}
