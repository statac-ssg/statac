//! Where in a template a link on a built page was written.
//!
//! A link in a tag that is a template's own text is placed by where the
//! build wrote that text from, which is exact. What follows places the
//! rest: a link in what a snippet wrote, or in a value a template printed.
//!
//! A template's text is read as HTML with each `{{ ... }}` and `{% ... %}`
//! standing for anything, so `href="{{ site.url }}/tags/{{ tag }}/"` is an
//! address that any `.../tags/.../` matches. A link that lands nowhere is
//! placed at the attribute that matches it, in a tag of the same name with
//! the same attributes, matching the tag it was found in, with the most of
//! the template's own letters: one written out whole wins over one made
//! from a value, and `<a class="crumb" href="{{ path }}">` over
//! `<a href="{{ url }}">` for a link in `<a class="crumb">`. A link in a tag
//! no attribute matches was printed whole, as HTML from a field or data
//! passed through `markdown` or `safe`, and is placed where that is printed.

use std::ops::Range;

use super::scan::{Attribute, Found, Tagged};
use crate::html::start_tags;

/// An address attribute in a template, as a pattern a link may match.
pub(super) struct Pattern {
    tag: TagPattern,
    attribute: Attribute,
    pieces: Vec<Piece>,
    /// The start tag before, where there is one.
    before: Option<TagPattern>,
    /// Where the attribute's value is written.
    at: Range<usize>,
}

/// A start tag in a template: its name, and each attribute it is always
/// written with, by name, its value as a pattern.
#[derive(Clone)]
struct TagPattern {
    name: String,
    attributes: Vec<(String, Vec<Piece>)>,
    /// Whether attributes are printed or chosen into it, so the page's tag
    /// may have others.
    open: bool,
}

/// A template read for where it writes addresses.
pub(super) struct Read {
    patterns: Vec<Pattern>,
    /// Where it prints HTML whole, `{{ ... | markdown }}` and the like,
    /// outside any tag, each with how likely it is to hold links, likeliest
    /// first.
    printed: Vec<(u8, Range<usize>)>,
}

impl Read {
    /// The address attribute whose value starts at `start`, as far as it
    /// is written.
    pub(super) fn written_at(&self, start: usize) -> Option<Range<usize>> {
        let pattern = self.patterns.iter().find(|p| p.at.start == start)?;
        Some(pattern.at.clone())
    }

    /// Whether `at` is where the template prints HTML whole.
    pub(super) fn prints(&self, at: &Range<usize>) -> bool {
        self.printed.iter().any(|(_, printed)| printed == at)
    }
}

#[derive(Clone, PartialEq, Debug)]
enum Piece {
    Text(String),
    /// What a value printed there, or a tag's choice, may make.
    Any,
}

/// What stands for printed text in the template read as HTML: a byte no
/// template's HTML holds.
const PRINTED: char = '\u{1}';
/// What stands for a tag such as `{% if %}`.
const CHOSEN: char = '\u{2}';

/// Every address attribute in `template`, as a pattern, and where it
/// prints HTML whole.
pub(super) fn read(template: &str) -> Read {
    let flat = flattened(template);
    let mut found = Vec::new();
    let mut before: Option<TagPattern> = None;
    start_tags(&flat, |tag| {
        // An attribute written inside a choice, such as `{% if %}`, takes
        // the choice's stand-ins into its name, and isn't always there.
        let open = tag
            .attributes
            .iter()
            .any(|attribute| attribute.name.contains([PRINTED, CHOSEN]));
        let this = TagPattern {
            name: tag.name.clone(),
            attributes: tag
                .attributes
                .iter()
                .filter(|attribute| !attribute.name.contains([PRINTED, CHOSEN]))
                .map(|attribute| (attribute.name.clone(), pieces(&flat[attribute.at.clone()])))
                .collect(),
            open,
        };
        for attribute in &tag.attributes {
            let Some(kind) = Attribute::of(&attribute.name).filter(|_| tag.name != "base") else {
                continue;
            };
            let mut written = pieces(&flat[attribute.at.clone()]);
            if kind == Attribute::Srcset {
                written.insert(0, Piece::Any);
                written.push(Piece::Any);
                written.dedup();
            }
            if written.is_empty() {
                continue;
            }
            let mut tag = this.clone();
            tag.attributes.retain(|(name, _)| *name != attribute.name);
            found.push(Pattern {
                tag,
                attribute: kind,
                pieces: written,
                before: before.clone(),
                at: attribute.at.clone(),
            });
        }
        before = Some(this);
    });
    Read {
        patterns: found,
        printed: printed(template, &flat),
    }
}

/// Where `template`, read as `flat`, prints HTML whole outside any tag:
/// through `markdown` or `safe` first, then a page's `content`.
fn printed(template: &str, flat: &str) -> Vec<(u8, Range<usize>)> {
    let b = flat.as_bytes();
    let mut found = Vec::new();
    let mut i = 0;
    while let Some(at) = flat[i..].find(PRINTED).map(|at| i + at) {
        let end = at + b[at..].iter().take_while(|&&c| c == PRINTED as u8).count();
        i = end;
        let opened = flat[..at].rfind('<');
        let closed = flat[..at].rfind('>');
        let in_tag = opened.is_some_and(|open| closed.is_none_or(|close| open > close));
        if in_tag {
            continue;
        }
        let expression = &template[at..end];
        let rank = if expression.contains("markdown") || expression.contains("safe") {
            0
        } else if expression.contains("content") {
            1
        } else {
            continue;
        };
        found.push((rank, at..end));
    }
    found
}

/// An attribute's value written in a template, as pieces.
fn pieces(value: &str) -> Vec<Piece> {
    let mut pieces = Vec::new();
    for c in value.chars() {
        match c {
            PRINTED | CHOSEN => {
                if pieces.last() != Some(&Piece::Any) {
                    pieces.push(Piece::Any);
                }
            }
            _ => match pieces.last_mut() {
                Some(Piece::Text(text)) => text.push(c),
                _ => pieces.push(Piece::Text(c.to_string())),
            },
        }
    }
    pieces
}

fn letters(pieces: &[Piece]) -> usize {
    pieces
        .iter()
        .map(|piece| match piece {
            Piece::Text(text) => text.len(),
            Piece::Any => 0,
        })
        .sum()
}

/// How well a link matches an attribute: the attributes the template's
/// tag has that the page's lacks, fewest first; then the template's own
/// letters it matches, most first; then whether the tag before matches.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct Score {
    lacking: std::cmp::Reverse<usize>,
    letters: usize,
    before: bool,
}

impl TagPattern {
    /// How many of the template's attributes `tag` lacks, and how many of
    /// their letters it matches; none where an attribute's value differs.
    fn matched(&self, tag: &Tagged) -> Option<(usize, usize)> {
        if self.name != tag.name {
            return None;
        }
        let (mut lacking, mut matched) = (0, 0);
        for (name, pieces) in &self.attributes {
            match tag.attributes.iter().find(|(other, _)| other == name) {
                None => lacking += 1,
                Some((_, value)) if matches(pieces, value) => matched += letters(pieces),
                Some(_) => return None,
            }
        }
        Some((lacking, matched))
    }

    /// Whether `tag` has an attribute other than `own` that this one
    /// never writes.
    fn surplus(&self, tag: &Tagged, own: &str) -> bool {
        !self.open
            && tag.attributes.iter().any(|(name, _)| {
                name != own && !self.attributes.iter().any(|(other, _)| other == name)
            })
    }
}

impl Pattern {
    /// How well the link `found` matches this attribute and its tag, or
    /// none where it doesn't match.
    fn score(&self, found: &Found) -> Option<Score> {
        if self.attribute != found.attribute || !matches(&self.pieces, &found.raw) {
            return None;
        }
        let own = found
            .tag
            .attributes
            .iter()
            .find(|(name, _)| Attribute::of(name) == Some(self.attribute))
            .map_or("", |(name, _)| name.as_str());
        if self.tag.surplus(&found.tag, own) {
            return None;
        }
        let (lacking, matched) = self.tag.matched(&found.tag)?;
        let before = match (&self.before, &found.before) {
            (Some(pattern), Some(tag)) => pattern
                .matched(tag)
                .is_some_and(|(lacking, _)| lacking == 0),
            _ => false,
        };
        let letters = letters(&self.pieces) + matched;
        // An attribute written wholly from values matches any link in a
        // tag of its shape; the tag before it is then the only evidence.
        if letters == 0 && !before {
            return None;
        }
        Some(Score {
            lacking: std::cmp::Reverse(lacking),
            letters,
            before,
        })
    }
}

/// Whether `raw` is the pieces, each `Any` standing for any run of
/// characters, none included.
fn matches(pieces: &[Piece], raw: &str) -> bool {
    match pieces.split_first() {
        None => raw.is_empty(),
        Some((Piece::Text(text), rest)) => raw
            .strip_prefix(text.as_str())
            .is_some_and(|after| matches(rest, after)),
        Some((Piece::Any, rest)) => {
            let Some(Piece::Text(next)) = rest.first() else {
                return rest.is_empty();
            };
            raw.match_indices(next.as_str())
                .any(|(at, _)| matches(rest, &raw[at..]))
        }
    }
}

/// `template` with each `{{ ... }}` and `{% ... %}` replaced by as many
/// stand-in characters, and each `{# ... #}` by spaces, so every offset is
/// the template's own.
fn flattened(template: &str) -> String {
    let b = template.as_bytes();
    let mut out = b.to_vec();
    let mut i = 0;
    while i + 1 < b.len() {
        let (close, with) = match (b[i], b[i + 1]) {
            (b'{', b'{') => ("}}", PRINTED as u8),
            (b'{', b'%') => ("%}", CHOSEN as u8),
            (b'{', b'#') => ("#}", b' '),
            _ => {
                i += 1;
                continue;
            }
        };
        let end = template[i + 2..]
            .find(close)
            .map_or(b.len(), |at| i + 2 + at + 2);
        for byte in &mut out[i..end] {
            *byte = with;
        }
        i = end;
    }
    String::from_utf8(out).unwrap_or_else(|_| template.to_owned())
}

/// Where the link `found` was written: of `candidates`, each a template's
/// patterns in the order the page read them, the attribute matching it
/// best, the first where two match alike.
pub(super) fn best<'a, T: Copy>(
    candidates: impl IntoIterator<Item = (T, &'a Read)> + Clone,
    found: &Found,
) -> Option<(T, Range<usize>)> {
    let mut best: Option<(Score, T, Range<usize>)> = None;
    for (template, read) in candidates.clone() {
        for pattern in &read.patterns {
            if let Some(score) = pattern.score(found)
                && best.as_ref().is_none_or(|(most, ..)| score > *most)
            {
                best = Some((score, template, pattern.at.clone()));
            }
        }
    }
    if let Some((_, template, at)) = best {
        return Some((template, at));
    }
    let mut printed: Option<(u8, T, Range<usize>)> = None;
    for (template, read) in candidates {
        for (rank, at) in &read.printed {
            if printed.as_ref().is_none_or(|(least, ..)| rank < least) {
                printed = Some((*rank, template, at.clone()));
            }
        }
    }
    printed.map(|(_, template, at)| (template, at))
}
