//! Footnotes, numbered in the order the writing first points at each, and
//! taken out of the writing to be shown at its foot. A note nothing points
//! at is shown nowhere, and told.

use std::collections::HashMap;

use super::Document;
use super::node::{Kind, Node};
use crate::problem::{Kind as ProblemKind, Problem};
use crate::source::{Place, Span};

/// Numbers the footnotes of `document`, the writing of `text`.
pub(super) fn number(document: &mut Document, text: &str) {
    // A label names the same note whatever its case. Where two notes share
    // one, the later is the one shown.
    let mut defined: HashMap<String, usize> = HashMap::new();
    let mut count = 0;
    document.root.walk(&mut |node| {
        if let Kind::FootnoteDefinition { label, .. } = &node.kind {
            defined.insert(label.to_lowercase(), count);
            count += 1;
        }
    });
    if defined.is_empty() {
        return;
    }

    let mut numbers: HashMap<usize, (u32, u32)> = HashMap::new();
    document.root.walk_mut(&mut |node| {
        let Kind::FootnoteReference {
            label,
            number,
            repeat,
            id,
            note,
        } = &mut node.kind
        else {
            return;
        };
        let Some(&at) = defined.get(&label.to_lowercase()) else {
            return;
        };
        let next = u32::try_from(numbers.len() + 1).expect("too many footnotes");
        let (n, seen) = numbers.entry(at).or_insert((next, 0));
        *seen += 1;
        *number = *n;
        *repeat = *seen;
        *id = back_id(*n, *seen);
        *note = format!("note-{n}");
    });

    let mut definitions = Vec::new();
    take_definitions(&mut document.root, &mut definitions);
    never_shown(document, &definitions, &defined, &numbers, text);
    let mut notes: Vec<Node> = definitions
        .into_iter()
        .enumerate()
        .filter_map(|(i, mut note)| {
            let (n, _) = numbers.get(&i)?;
            if let Kind::FootnoteDefinition {
                number, id, back, ..
            } = &mut note.kind
            {
                *number = *n;
                *id = format!("note-{n}");
                *back = back_id(*n, 1);
            }
            Some(note)
        })
        .collect();
    notes.sort_by_key(|note| match note.kind {
        Kind::FootnoteDefinition { number, .. } => number,
        _ => 0,
    });
    document.notes = notes;
}

/// The id of the `repeat`th marker pointing at note `number`.
fn back_id(number: u32, repeat: u32) -> String {
    match repeat {
        1 => format!("back-{number}"),
        n => format!("back-{number}-{n}"),
    }
}

/// A warning at each note nothing points at, offering a marker that has no
/// note of its own where its label is near, and at each note whose label a
/// later note takes, which is the later one's to show.
fn never_shown(
    document: &mut Document,
    definitions: &[Node],
    defined: &HashMap<String, usize>,
    numbers: &HashMap<usize, (u32, u32)>,
    text: &str,
) {
    let unset: Vec<String> = document
        .problems
        .iter()
        .filter_map(|problem| match &problem.kind {
            ProblemKind::MissingDefinition { written, .. } => {
                Some(written.strip_prefix("[^")?.strip_suffix(']')?.to_owned())
            }
            _ => None,
        })
        .collect();
    let mut told = false;
    for (i, note) in definitions.iter().enumerate() {
        let Kind::FootnoteDefinition { label, .. } = &note.kind else {
            continue;
        };
        let kind = match defined.get(&label.to_lowercase()) {
            Some(&shown) if shown != i => ProblemKind::DefinitionWrittenTwice {
                label: label.clone(),
                shown: Some(opening(&definitions[shown], text)),
            },
            _ if numbers.contains_key(&i) => continue,
            _ => {
                let nearest = crate::suggest::nearest(label, unset.iter().map(String::as_str));
                ProblemKind::FootnoteNeverShown {
                    label: label.clone(),
                    nearest: nearest.map(str::to_owned),
                }
            }
        };
        document
            .problems
            .push(Problem::at(kind, opening(note, text)));
        told = true;
    }
    if told {
        document
            .problems
            .sort_by_key(|p| p.place.map(|place| place.span.start));
    }
}

/// The `[^label]` opening a note, where it can be found; else the note.
fn opening(note: &Node, text: &str) -> Place {
    let start = note.place.span.start as usize;
    let span = match text[start..].starts_with("[^") {
        true => text[start..]
            .find(']')
            .map_or(note.place.span, |end| Span::new(start, start + end + 1)),
        false => note.place.span,
    };
    Place {
        source: note.place.source,
        span,
    }
}

/// Takes every footnote out of the tree, in the order written.
fn take_definitions(node: &mut Node, out: &mut Vec<Node>) {
    if node.children.iter().all(Node::is_inline) {
        return;
    }
    let children = std::mem::take(&mut node.children);
    for mut child in children {
        if matches!(child.kind, Kind::FootnoteDefinition { .. }) {
            let mut nested = Vec::new();
            take_definitions(&mut child, &mut nested);
            out.push(child);
            out.extend(nested);
        } else {
            take_definitions(&mut child, out);
            node.children.push(child);
        }
    }
}
