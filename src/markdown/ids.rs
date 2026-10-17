//! An id for every heading, so a link can land on the part of the page it
//! means.

use std::collections::{HashMap, HashSet};

use super::Document;
use super::node::{Kind, Node};
use super::questions::plain_text;
use crate::content::slug::{anchor, made_slug};
use crate::problem::{Braced, Kind as ProblemKind, Problem, Taken};
use crate::source::{Place, Span};

/// Gives each heading without an id of the author's an id from its words,
/// the headings in the writing first and then those in its footnotes. Two
/// headings with the same words are told apart by a number, and none takes
/// an id a footnote has or the author gave another heading or a table. An
/// id the author gave is theirs and kept as written, given twice or not,
/// and told where an earlier heading, table or footnote has it, as links
/// to it land on that. Words that make no id, a heading of punctuation
/// alone, are named from `path`, the file the writing is in, and the
/// heading's place among them. `text` is that file's.
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
    let mut written: HashMap<String, (Place, Braced)> = HashMap::new();
    let mut twice = Vec::new();
    for top in tops() {
        top.walk(&mut |node| {
            let (id, on) = match &node.kind {
                Kind::Heading { id: Some(id), .. } => (id, Braced::Heading),
                Kind::Table { id: Some(id), .. } => (id, Braced::Table),
                _ => return,
            };
            let taken = match written.get(id) {
                Some(&(first, Braced::Heading)) => Some(Taken::Heading(first)),
                Some(&(first, Braced::Table)) => Some(Taken::Table(first)),
                None if footnotes.contains(id) => Some(Taken::Footnote),
                None => None,
            };
            // A table as the first holder is shown by its braces alone.
            let first = match on {
                Braced::Heading => node.place,
                Braced::Table => written_at(node.place, id, text),
            };
            written.entry(id.clone()).or_insert((first, on));
            ids.taken.insert(id.clone());
            if let Some(taken) = taken {
                let kind = ProblemKind::IdUsedTwice {
                    id: id.clone(),
                    taken,
                    on,
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

/// Where a heading or table written at `braced` writes the id `id`: the
/// `#id` in the braces ending it, or the whole of it where that can't be
/// found.
fn written_at(braced: Place, id: &str, text: &str) -> Place {
    let range = braced.span.range();
    let written = format!("#{id}");
    match text[range.clone()].rfind(&written) {
        Some(at) => Place {
            source: braced.source,
            span: Span::new(range.start + at, range.start + at + written.len()),
        },
        None => braced,
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
