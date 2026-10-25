//! The id Zola gave each heading, under `[slugify]`'s `on`, kept where
//! Statac would give the heading another: written in braces at the end of
//! its line, `## C++ & C# {#c-c}`, which both read as the heading's id.

use std::collections::{HashMap, HashSet};
use std::ops::Range;

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

use super::slug::Strategy;
use crate::import::common::emoji_names;
use crate::markdown::{self, Kind};
use crate::source::{Sources, Span};

/// A heading as Zola reads it.
struct Heading {
    /// Where it starts in the writing.
    start: usize,
    /// Where Zola's id is written: at the end of its last line of words,
    /// or inside the braces already there.
    at: usize,
    /// Whether braces giving classes already end it.
    braced: bool,
    /// Zola's id for it, where Statac can be given one: none where the
    /// heading has an id of the author's, or an empty one, or one made
    /// from words Zola had from template code, a tag or an emoji's name,
    /// which Statac doesn't have, or where braces end it that Statac reads
    /// as words.
    id: Option<String>,
}

/// Writes Zola's id after each heading in `writing` that Statac would give
/// another, moving each of `calls`, offsets in the writing, along with
/// what is written before it. Gives the line each id was written on, as
/// it now reads, in order.
pub(super) fn keep(writing: &mut String, calls: &mut [usize], emoji: bool) -> Vec<String> {
    let mut headings = zolas(writing, calls, emoji);
    let mut kept: Vec<usize> = Vec::new();
    // Each id written takes one Statac would have given another heading,
    // so Statac is asked again until it gives every heading Zola's.
    loop {
        let statacs = statacs(writing);
        let mut wanted: Vec<usize> = (0..headings.len())
            .filter(|&at| !kept.contains(&at))
            .filter(|&at| {
                let heading = &headings[at];
                let given = statacs.get(&heading.start);
                heading.id.is_some() && given.is_some() && given != heading.id.as_ref()
            })
            .collect();
        if wanted.is_empty() {
            break;
        }
        wanted.sort_by_key(|&at| std::cmp::Reverse(headings[at].at));
        for &at in &wanted {
            let heading = &headings[at];
            let id = heading.id.as_deref().unwrap_or_default();
            let written = match heading.braced {
                true => format!(" #{id}"),
                false => format!(" {{#{id}}}"),
            };
            let place = heading.at;
            writing.insert_str(place, &written);
            for heading in headings.iter_mut() {
                if heading.start >= place {
                    heading.start += written.len();
                }
                if heading.at >= place {
                    heading.at += written.len();
                }
            }
            for call in calls.iter_mut() {
                if *call >= place {
                    *call += written.len();
                }
            }
        }
        kept.extend(wanted);
    }
    kept.sort_by_key(|&at| headings[at].start);
    kept.iter()
        .map(|&at| {
            // Where the id ends, on the line of words it was written on.
            let end = headings[at].at;
            let opens = writing[..end].rfind('\n').map_or(0, |i| i + 1);
            let closes = writing[end..].find('\n').map_or(writing.len(), |i| end + i);
            writing[opens..closes].trim().to_owned()
        })
        .collect()
}

/// Each heading in `writing`, with the id Zola gave it, in order.
fn zolas(writing: &str, calls: &[usize], emoji: bool) -> Vec<Heading> {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_HEADING_ATTRIBUTES;
    let events: Vec<_> = Parser::new_ext(writing, options)
        .into_offset_iter()
        .collect();
    // Zola keeps every id an author gave before making any.
    let mut taken: HashSet<String> = (events.iter())
        .filter_map(|(event, _)| match event {
            Event::Start(Tag::Heading { id: Some(id), .. }) => Some(id.to_string()),
            _ => None,
        })
        .collect();
    let mut headings = Vec::new();
    // The heading being read: where it is, whether the author gave its
    // id, whether braces give it classes, and whether they give it other
    // attributes, which Statac reads as words.
    let mut open: Option<(Range<usize>, bool, bool, bool)> = None;
    let mut words = String::new();
    let mut image = 0;
    for (event, range) in events {
        match event {
            Event::Start(Tag::Heading {
                id, classes, attrs, ..
            }) => {
                let braced = id.is_none() && !classes.is_empty();
                open = Some((range, id.is_some(), braced, !attrs.is_empty()));
                words.clear();
            }
            Event::Start(Tag::Image { .. }) => image += 1,
            Event::End(TagEnd::Image) => image -= 1,
            Event::Text(text) | Event::Code(text) if open.is_some() && image == 0 => {
                words.push_str(&text);
            }
            Event::End(TagEnd::Heading(_)) => {
                let Some((range, own, braced, attributes)) = open.take() else {
                    continue;
                };
                let (start, end) = (range.start, range.end);
                let written = &writing[range];
                let unknown = attributes
                    || written.contains("{{")
                    || written.contains("{%")
                    || calls.iter().any(|call| (start..end).contains(call))
                    || (emoji && !emoji_names(written).is_empty());
                let id = (!own).then(|| {
                    let id = anchor(&taken, Strategy::On.made(&words));
                    taken.insert(id.clone());
                    id
                });
                headings.push(Heading {
                    start,
                    at: end_of_words(writing, start, end, braced),
                    braced,
                    id: id.filter(|id| !unknown && !id.is_empty()),
                });
            }
            _ => {}
        }
    }
    headings
}

/// `name` where no heading has it, and otherwise `name-1`, `name-2` and
/// so on, the first no heading has, as Zola numbers them.
fn anchor(taken: &HashSet<String>, name: String) -> String {
    if !taken.contains(&name) {
        return name;
    }
    (1..)
        .map(|n| format!("{name}-{n}"))
        .find(|id| !taken.contains(id))
        .unwrap_or_default()
}

/// Where an id is written in the heading from `start` to `end`: before the
/// closing `}` of braces that end it, or at the end of its line, after any
/// closing `#`s, and before the underline of a heading that has one.
fn end_of_words(writing: &str, start: usize, end: usize, braced: bool) -> usize {
    let written = writing[start..end].trim_end_matches(['\n', '\r']);
    let line = match written.rfind('\n') {
        Some(under) => &written[..under],
        None => written,
    };
    let line = line.trim_end_matches(['\r', ' ', '\t']);
    match braced {
        true => start + line.len() - 1,
        false => start + line.len(),
    }
}

/// The id Statac gives each heading in `writing`, by where it starts.
fn statacs(writing: &str) -> HashMap<usize, String> {
    let mut sources = Sources::default();
    let source = sources.add("writing", writing.to_owned());
    let mut document = markdown::read(
        &sources,
        source,
        Span::new(0, writing.len()),
        &|_| Some(false),
        "",
    );
    markdown::give_heading_ids(&mut document, "writing", writing);
    let mut ids = HashMap::new();
    for top in std::iter::once(&document.root).chain(&document.notes) {
        top.walk(&mut |node| {
            if let Kind::Heading { id: Some(id), .. } = &node.kind {
                ids.insert(node.place.span.start as usize, id.clone());
            }
        });
    }
    ids
}
