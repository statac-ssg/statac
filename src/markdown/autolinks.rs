//! Addresses written bare among the words, made links as GitHub Flavored
//! Markdown makes them: `www.example.org` points at
//! `http://www.example.org`, a mail address at itself after `mailto:`, and
//! the rest at themselves. Typography finds them, so the two never
//! disagree about where an address is; only an address written wholly in
//! the writing is linked, never one inside a link, a picture's description
//! or an `<a>`, one typed in part as an escape or an entity, one shown as
//! typed in raw HTML, or a `www.` or scheme address inside a `[` not yet
//! closed. Code, maths and a call's arguments are no part of the words.

use super::Document;
use super::controls;
use super::node::{Kind, Node};
use super::typography::{self, Bare};
use crate::source::{Sources, Span};

/// Makes every address written bare a link, the footnotes' too.
pub(super) fn link(document: &mut Document, sources: &Sources) {
    let mut link = |nodes: &mut Vec<Node>, source| linked(nodes, sources.text(source));
    typography::blocks(&mut document.root, &mut link);
    for note in &mut document.notes {
        typography::blocks(note, &mut link);
    }
}

/// One block's words, each address in them made a link.
fn linked(nodes: &mut Vec<Node>, source: &str) {
    // Every address holds `www.`, `://` or a mail address's `@`, and one
    // this pass links lies within one text node.
    let mut maybe = false;
    for node in nodes.iter() {
        node.walk(&mut |n| {
            maybe |= matches!(&n.kind, Kind::Text(text)
                if text.contains('@') || text.contains("://") || text.contains("www."));
        });
    }
    if !maybe {
        return;
    }
    let found = typography::bare_addresses(nodes, source);
    let bracketed = bracketed(nodes, source, &found);
    let found: Vec<Bare> = found
        .into_iter()
        .zip(bracketed)
        .filter(|(bare, bracketed)| bare.linkable && (bare.mail || !bracketed))
        .map(|(bare, _)| bare)
        .collect();
    if found.is_empty() {
        return;
    }
    let mut cutting = Cutting {
        source,
        found: &found,
        next: 0,
        word: 0,
        open: None,
    };
    cutting.nodes(nodes);
}

/// For each address found, whether a `[` the writing typed is open where it
/// starts. cmark-gfm links no `www.` or scheme address while it reads what
/// may be a link's words, from a `[` to the next `]`, whether or not they
/// turn out to be a link (`match` in `extensions/autolink.c`); a mail
/// address is linked all the same. A bracket inside an address found is
/// part of the address.
fn bracketed(nodes: &[Node], source: &str, found: &[Bare]) -> Vec<bool> {
    let mut brackets = Vec::new();
    brackets_in(nodes, source, &mut 0, true, &mut brackets);
    let inside = |at: (usize, usize)| found.iter().any(|bare| bare.from <= at && at < bare.to);
    let mut events = brackets
        .into_iter()
        .filter(|(at, _)| !inside(*at))
        .peekable();
    let mut open = 0usize;
    found
        .iter()
        .map(|bare| {
            while let Some((_, opens)) = events.next_if(|(at, _)| *at < bare.from) {
                open = if opens {
                    open + 1
                } else {
                    open.saturating_sub(1)
                };
            }
            open > 0
        })
        .collect()
}

/// Each bracket typed among the words, by the text or literal node and the
/// character it is, and whether it opens. One inside a link or a picture
/// closes within it, and one typed after a backslash is no bracket.
fn brackets_in(
    nodes: &[Node],
    source: &str,
    word: &mut usize,
    counted: bool,
    out: &mut Vec<((usize, usize), bool)>,
) {
    for node in nodes {
        match &node.kind {
            Kind::Text(text) | Kind::Literal(text) => {
                if counted && text.contains(['[', ']']) {
                    let spelling = &source[node.place.span.range()];
                    let typed = source.as_bytes();
                    for (index, (at, c)) in text.char_indices().enumerate() {
                        if !matches!(c, '[' | ']') {
                            continue;
                        }
                        // Where the bracket or the backslash before it is.
                        let from = node.place.span.start as usize
                            + written_at(text, spelling, at).unwrap_or(at);
                        let backslashes = typed[..from]
                            .iter()
                            .rev()
                            .take_while(|b| **b == b'\\')
                            .count();
                        if typed[from] != b'\\' && backslashes % 2 == 0 {
                            out.push(((*word, index), c == '['));
                        }
                    }
                }
                *word += 1;
            }
            Kind::Emphasis | Kind::Strong | Kind::Strikethrough => {
                brackets_in(&node.children, source, word, counted, out);
            }
            Kind::Link { .. } | Kind::Image { .. } => {
                brackets_in(&node.children, source, word, false, out);
            }
            _ => {}
        }
    }
}

/// Walks a block's words in the order typography read them, cutting each
/// text node where an address starts or ends.
struct Cutting<'a> {
    source: &'a str,
    found: &'a [Bare],
    /// The next address to link.
    next: usize,
    /// How many text and literal nodes have been passed.
    word: usize,
    /// The link being filled, where an address runs on into another node.
    open: Option<Node>,
}

impl Cutting<'_> {
    fn nodes(&mut self, nodes: &mut Vec<Node>) {
        for mut node in std::mem::take(nodes) {
            match node.kind {
                Kind::Text(_) | Kind::Literal(_) => self.word(node, nodes),
                Kind::Emphasis
                | Kind::Strong
                | Kind::Strikethrough
                | Kind::Link { .. }
                | Kind::Image { .. } => {
                    self.nodes(&mut node.children);
                    nodes.push(node);
                }
                _ => nodes.push(node),
            }
        }
    }

    fn word(&mut self, node: Node, out: &mut Vec<Node>) {
        let index = self.word;
        self.word += 1;
        let starts_here = self
            .found
            .get(self.next)
            .is_some_and(|bare| bare.from.0 == index);
        let Kind::Text(text) = &node.kind else {
            out.push(node);
            return;
        };
        if self.open.is_none() && !starts_here {
            out.push(node);
            return;
        }
        let spelling = &self.source[node.place.span.range()];
        let byte = |chars: usize| {
            text.char_indices()
                .nth(chars)
                .map_or(text.len(), |(at, _)| at)
        };
        let written = |at: usize| {
            let typed = written_at(text, spelling, at).unwrap_or(match at {
                0 => 0,
                _ => spelling.len(),
            });
            node.place.span.start as usize + typed
        };
        let piece = |from: usize, to: usize| {
            let mut piece = node.clone();
            if from > 0 || to < text.len() {
                piece.kind = Kind::Text(text[from..to].to_owned());
                piece.place.span = Span::new(written(from), written(to).max(written(from)));
            }
            piece
        };
        let mut from = 0;
        loop {
            if let Some(link) = &mut self.open {
                let bare = &self.found[self.next];
                let to = match bare.to.0 == index {
                    true => byte(bare.to.1),
                    false => text.len(),
                };
                if to > from {
                    link.children.push(piece(from, to));
                    link.place.span.end = u32::try_from(written(to)).expect("a file under 4 GiB");
                }
                if bare.to.0 != index {
                    return;
                }
                out.extend(self.open.take());
                self.next += 1;
                from = to;
                continue;
            }
            match self.found.get(self.next) {
                Some(bare) if bare.from.0 == index => {
                    let start = byte(bare.from.1);
                    if start > from {
                        out.push(piece(from, start));
                    }
                    let mut link = Node::new(
                        Kind::Link {
                            destination: bare.destination.clone(),
                            title: String::new(),
                        },
                        node.place,
                    );
                    link.place.span = Span::new(written(start), written(start));
                    self.open = Some(link);
                    from = start;
                }
                _ => {
                    if from < text.len() {
                        out.push(piece(from, text.len()));
                    }
                    return;
                }
            }
        }
    }
}

/// Where the character at byte `at` of `text` is written in `spelling`,
/// the text as typed that the parser read it from: an escape and an entity
/// are characters read from several typed, and a control character typed
/// there was left out. None where the two can't be matched.
fn written_at(text: &str, spelling: &str, at: usize) -> Option<usize> {
    if text == spelling {
        return Some(at);
    }
    let (mut t, mut s) = (0, 0);
    while t < at {
        let c = text[t..].chars().next()?;
        let rest = &spelling[s..];
        let mut typed = rest.chars();
        let first = typed.next()?;
        if first == '\\' && typed.next() == Some(c) && c.is_ascii_punctuation() {
            s += 2;
            t += 1;
            continue;
        }
        if first == '&'
            && let Some((length, decoded)) = entity(rest)
            && (decoded || c != '&')
        {
            // A few named entities stand for two characters.
            let mut read = text[t..].chars();
            let after = spelling[s + length..].chars().next();
            let two =
                read.nth(1).is_some_and(|second| Some(second) != after) && read.next() == after;
            t += text[t..]
                .chars()
                .take(if two { 2 } else { 1 })
                .map(char::len_utf8)
                .sum::<usize>();
            s += length;
            continue;
        }
        if first == c {
            s += c.len_utf8();
            t += c.len_utf8();
            continue;
        }
        if controls::left_out(first) {
            s += first.len_utf8();
            continue;
        }
        return None;
    }
    Some(s)
}

/// How long the entity `rest` opens with is, if it looks like one, and
/// whether it is surely read as a character rather than kept as typed: a
/// number always is, and `&amp;` is the one name standing for `&`.
fn entity(rest: &str) -> Option<(usize, bool)> {
    let body = rest.strip_prefix('&')?;
    let end = body.find(';')?;
    let name = &body[..end];
    let numeric = match name.strip_prefix('#') {
        Some(number) => match number.strip_prefix(['x', 'X']) {
            Some(hex) => (1..=6).contains(&hex.len()) && hex.bytes().all(|b| b.is_ascii_hexdigit()),
            None => (1..=7).contains(&number.len()) && number.bytes().all(|b| b.is_ascii_digit()),
        },
        None => false,
    };
    let named = (2..=32).contains(&name.len())
        && name.starts_with(|c: char| c.is_ascii_alphabetic())
        && name.bytes().all(|b| b.is_ascii_alphanumeric());
    if !numeric && !named {
        return None;
    }
    Some((end + 2, numeric || matches!(name, "amp" | "AMP")))
}

#[cfg(test)]
mod tests;
