//! A page's writing made to read in Statac as it showed in the old site,
//! where a blank line or a backslash does it. What Statac reads is asked
//! of its own Markdown reader, and asked again after every change.
//!
//! A block of HTML ends at the first blank line, unless it opens with
//! `<pre>`, `<script>`, `<style>` or `<textarea>`, when it ends at the
//! closing tag. One of those written straight after other HTML is part of
//! that block, so a blank line inside it would end the HTML and leave the
//! rest to be read as Markdown: each that holds a blank line is given one
//! before it, and so a block of its own. A code fence a reader wrote straight after HTML
//! is part of the HTML too, and is given a blank line the same way.
//!
//! `@name(` opens a snippet call wherever Statac reads writing outside
//! code, and meant nothing to the tool the writing came from. Each one a
//! reader didn't write itself is escaped, `\@name(`, and shows as it did.

use crate::markdown::{self, Document, Kind, Malformed, Node};
use crate::problem::{Kind as ProblemKind, MoreAt, WrongPlace};
use crate::source::{Sources, Span};

/// The elements whose block of HTML ends only at a closing tag.
const ENDS_AT_ITS_TAG: [&str; 4] = ["pre", "script", "style", "textarea"];

/// What was changed in writing, by how many places.
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct Settled {
    /// The tags inside other HTML given a blank line before them.
    pub tags: usize,
    /// The code fences inside HTML given a blank line before them.
    pub fences: usize,
    /// The places Statac would read as a snippet call, escaped.
    pub escaped: usize,
    /// The `<!--more-->` markers given a line of their own.
    pub markers_moved: usize,
    /// The `<!--more-->` markers taken out.
    pub markers_out: usize,
    /// The dollar signs Statac would read as opening maths, escaped.
    pub dollars: usize,
}

/// Whether the old site set maths written between dollar signs.
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct Maths {
    /// Between one dollar sign and the next, inside a line.
    pub inline: bool,
    /// Between `$$` and `$$`.
    pub display: bool,
}

impl Settled {
    pub fn add(&mut self, other: &Settled) {
        self.tags += other.tags;
        self.fences += other.fences;
        self.escaped += other.escaped;
        self.markers_moved += other.markers_moved;
        self.markers_out += other.markers_out;
        self.dollars += other.dollars;
    }
}

/// How many times one kind of change is made to a page's writing and the
/// writing read again: each pass makes every change of its kind the
/// reading shows, and a later pass is for what the one before uncovered,
/// which halves at worst. A page is read at most `4 * PASSES + 2` times
/// and written as often, whatever it holds; what is left after that is
/// left as written, for `statac check` to show.
const PASSES: usize = 24;

/// Changes `writing` so Statac reads it as the old site showed it.
/// `fences` and `calls` are where a reader wrote a code fence or a
/// snippet call, and are moved with the writing. `maths` says what the
/// old site read as maths.
pub fn settle(
    writing: &mut String,
    fences: &mut [usize],
    calls: &mut [usize],
    maths: Maths,
) -> Settled {
    let mut settled = Settled::default();
    let mut document = read(writing);
    // A tag set apart can bring another into a block of HTML, so each
    // pass reads the writing again.
    for _ in 0..PASSES {
        let tags = tags_inside(&document, writing);
        if tags.is_empty() {
            break;
        }
        let edits = tags.iter().map(|at| set_apart(writing, *at)).collect();
        edit(writing, edits, &mut [&mut *fences, &mut *calls]);
        settled.tags += tags.len();
        document = read(writing);
    }
    let opened = fences_read(&document);
    let inside: Vec<usize> = fences
        .iter()
        .copied()
        .filter(|fence| {
            let line = line_start(writing, *fence);
            let from = opened.partition_point(|start| *start < line);
            !opened.get(from).is_some_and(|start| *start <= *fence)
        })
        .collect();
    let edits = (inside.iter())
        .map(|fence| {
            let line = line_start(writing, *fence);
            (line..line, "\n")
        })
        .collect();
    edit(writing, edits, &mut [&mut *fences, &mut *calls]);
    settled.fences = inside.len();
    if settled.tags + settled.fences > 0 {
        document = read(writing);
    }
    // A marker Statac refuses stops the build. Each on a line of its own
    // inside HTML is given a blank line each side, and one Statac refuses
    // still, or refuses anywhere else, is taken out, with any blank line
    // it was given.
    let mut apart: Vec<usize> = Vec::new();
    // Whether each marker in `apart` was given a line before it and after.
    let mut added: Vec<(bool, bool)> = Vec::new();
    for _ in 0..PASSES {
        let refused = refused_markers(&document);
        if refused.is_empty() {
            break;
        }
        let mut edits = Vec::new();
        let mut set_apart_now = Vec::new();
        for (marker, at) in refused {
            let line = line_start(writing, marker.start);
            let line_end = writing[marker.end..]
                .find('\n')
                .map_or(writing.len(), |i| marker.end + i);
            let alone = writing[line..marker.start].trim().is_empty()
                && writing[marker.end..line_end].trim().is_empty();
            let tried = apart.binary_search(&marker.start).ok();
            if at == MoreAt::Html && alone && tried.is_none() {
                let after =
                    line_end < writing.len() && !writing[line_end + 1..].starts_with(['\n', '\r']);
                if after {
                    edits.push((line_end + 1..line_end + 1, "\n"));
                }
                if line > 0 {
                    edits.push((line..line, "\n"));
                }
                set_apart_now.push((marker.start, (line > 0, after)));
                settled.markers_moved += 1;
                continue;
            }
            let (before, after) = tried.map_or((false, false), |at| added[at]);
            if tried.is_some() {
                settled.markers_moved -= 1;
            }
            let next = (line_end + 1).min(writing.len());
            edits.push(match alone {
                true => (line - usize::from(before)..next + usize::from(after), ""),
                false => (marker, ""),
            });
            settled.markers_out += 1;
        }
        set_apart_now.sort_unstable();
        (apart, added) = set_apart_now.into_iter().unzip();
        edit(writing, edits, &mut [&mut *fences, &mut *calls, &mut apart]);
        document = read(writing);
    }
    // An escape can leave what the call held to be read on its own, so
    // the writing is read again until nothing in it is a call but the
    // reader's own.
    for _ in 0..PASSES {
        let found = calls_read(&document, writing, calls);
        if found.is_empty() {
            break;
        }
        let edits = found.iter().map(|at| (*at..*at, "\\")).collect();
        edit(writing, edits, &mut [&mut *fences, &mut *calls]);
        settled.escaped += found.len();
        document = read(writing);
    }
    // An escaped dollar can leave the next to open maths with one after
    // it, so the writing is read again until none is left.
    for _ in 0..PASSES {
        let found = dollars_read(&document, writing, maths);
        if found.is_empty() {
            break;
        }
        let edits = found.iter().map(|at| (*at..*at, "\\")).collect();
        edit(writing, edits, &mut [&mut *fences, &mut *calls]);
        settled.dollars += found.len();
        document = read(writing);
    }
    settled
}

/// Each `<!--more-->` Statac refuses for where it stands, by its place
/// in the writing, with where that is.
fn refused_markers(document: &Document) -> Vec<(std::ops::Range<usize>, MoreAt)> {
    (document.problems.iter())
        .filter_map(|problem| {
            let ProblemKind::WrongPlace {
                why: WrongPlace::More(at),
                ..
            } = &problem.kind
            else {
                return None;
            };
            Some((problem.place?.span.range(), at.clone()))
        })
        .collect()
}

/// Makes every one of `edits` to `writing` in one pass, each a piece of
/// it and the text to stand there, and moves each mark at or after a
/// piece by what the edits before it added or took. An edit that begins
/// inside the one before it is left out.
fn edit(
    writing: &mut String,
    mut edits: Vec<(std::ops::Range<usize>, &str)>,
    marks: &mut [&mut [usize]],
) {
    if edits.is_empty() {
        return;
    }
    edits.sort_by_key(|(piece, _)| (piece.start, piece.end));
    let added: usize = edits.iter().map(|(_, text)| text.len()).sum();
    let mut out = String::with_capacity(writing.len() + added);
    let mut copied = 0;
    // Where each edit's piece ended, with how far everything from there
    // on has moved: forwards, or else back.
    let mut moved: Vec<(usize, usize, usize)> = Vec::with_capacity(edits.len());
    let (mut forwards, mut back) = (0, 0);
    for (piece, text) in &edits {
        if piece.start < copied {
            continue;
        }
        out.push_str(&writing[copied..piece.start]);
        out.push_str(text);
        copied = piece.end;
        forwards += text.len();
        back += piece.len();
        moved.push((piece.end, forwards, back));
    }
    out.push_str(&writing[copied..]);
    *writing = out;
    for mark in marks.iter_mut().flat_map(|marks| marks.iter_mut()) {
        let before = moved.partition_point(|(end, _, _)| *end <= *mark);
        if let Some((_, forwards, back)) = before.checked_sub(1).map(|at| moved[at]) {
            *mark = *mark + forwards - back;
        }
    }
}

/// Where Statac reads a dollar sign as opening maths the old site didn't
/// set, by the offset of each, in order. Statac reads none inside a block
/// of HTML, where a backslash would show.
fn dollars_read(document: &Document, writing: &str, maths: Maths) -> Vec<usize> {
    fn walk(node: &Node, writing: &str, maths: Maths, found: &mut Vec<usize>) {
        if let Kind::Maths { display, .. } = &node.kind {
            let set = match display {
                true => maths.display,
                false => maths.inline,
            };
            let at = node.place.span.start as usize;
            if !set && writing[at..].starts_with('$') {
                found.push(at);
            }
        }
        for child in &node.children {
            walk(child, writing, maths, found);
        }
    }
    let mut found = Vec::new();
    walk(&document.root, writing, maths, &mut found);
    for note in &document.notes {
        walk(note, writing, maths, &mut found);
    }
    found.sort_unstable();
    found.dedup();
    found
}

fn read(writing: &str) -> Document {
    let mut sources = Sources::default();
    let source = sources.add("writing", writing.to_owned());
    let whole = Span::new(0, writing.len());
    markdown::read(&sources, source, whole, &|_| Some(false), "")
}

fn line_start(writing: &str, at: usize) -> usize {
    writing[..at].rfind('\n').map_or(0, |i| i + 1)
}

/// Where each tag of [`ENDS_AT_ITS_TAG`] opens inside a block of HTML
/// that stands in no list or quotation and ends before the tag is
/// closed, other than as the block's first thing, in order.
fn tags_inside(document: &Document, writing: &str) -> Vec<usize> {
    let mut found = Vec::new();
    for block in &document.root.children {
        if block.kind != Kind::HtmlBlock {
            continue;
        }
        let start = block.place.span.start as usize;
        let html = &writing[block.place.span.range()];
        let lower = html.to_ascii_lowercase();
        let mut from = 0;
        while let Some(at) = lower[from..].find('<').map(|i| from + i) {
            let rest = &lower[at..];
            if rest.starts_with("<!--") {
                match rest.find("-->") {
                    Some(end) => from = at + end,
                    None => break,
                }
                continue;
            }
            let opened = ENDS_AT_ITS_TAG.iter().find(|name| {
                rest[1..].strip_prefix(**name).is_some_and(|after| {
                    after.is_empty() || after.starts_with([' ', '\t', '>', '\n', '\r'])
                })
            });
            let Some(name) = opened else {
                from = at + 1;
                continue;
            };
            // One closed inside the block holds no blank line, and is
            // read as it stands; setting it apart would leave the lines
            // after it to be read on their own.
            match rest.find(&format!("</{name}")) {
                Some(end) => from = at + end,
                None => {
                    if !html[..at].trim().is_empty() {
                        found.push(start + at);
                    }
                    break;
                }
            }
        }
    }
    found
}

/// The edit that gives the tag at `at` a blank line before it and the
/// start of its line, where a block of HTML can open.
fn set_apart(writing: &str, at: usize) -> (std::ops::Range<usize>, &'static str) {
    let line = line_start(writing, at);
    match writing[line..at].trim().is_empty() {
        true => (line..at, "\n"),
        false => (at..at, "\n\n"),
    }
}

/// Where Statac reads each fenced code block as opening, in order.
fn fences_read(document: &Document) -> Vec<usize> {
    let mut opened = Vec::new();
    let mut look = |node: &Node| {
        if matches!(node.kind, Kind::CodeBlock { fenced: true, .. }) {
            opened.push(node.place.span.start as usize);
        }
    };
    document.root.walk(&mut look);
    for note in &document.notes {
        note.walk(&mut look);
    }
    opened.sort_unstable();
    opened
}

/// Where Statac reads a snippet call in the writing, or the start of one
/// it can't read, other than at one of `own`: the offset of each one's
/// `@`, in order.
fn calls_read(document: &Document, writing: &str, own: &[usize]) -> Vec<usize> {
    let mut found = Vec::new();
    let mut look = |node: &Node| {
        if matches!(node.kind, Kind::Call(_)) {
            found.push(node.place.span.start as usize);
        }
    };
    document.root.walk(&mut look);
    for note in &document.notes {
        note.walk(&mut look);
    }
    for problem in &document.problems {
        let (ProblemKind::MalformedCall { found: fault }, Some(place)) =
            (&problem.kind, problem.place)
        else {
            continue;
        };
        let at = place.span.start as usize;
        match fault {
            // Placed at the quote, inside the call it is the nearest `@`
            // before.
            Malformed::UnclosedQuote(_) | Malformed::TextAfterQuote => {
                found.extend(writing[..at].rfind('@'));
            }
            _ => found.push(at),
        }
    }
    let mut own = own.to_vec();
    own.sort_unstable();
    found.retain(|at| writing[*at..].starts_with('@') && own.binary_search(at).is_err());
    found.sort_unstable();
    found.dedup();
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_are_made_in_one_pass_and_marks_move_with_them() {
        let mut writing = "one two three four".to_owned();
        let mut marks = [0, 4, 8, 14];
        let edits = vec![(14..14, "<"), (4..8, ""), (0..0, ">>"), (5..6, "never")];
        edit(&mut writing, edits, &mut [&mut marks]);
        assert_eq!(writing, ">>one three <four");
        // A mark at the start of a piece taken out stays where the piece
        // was.
        assert_eq!(marks, [2, 6, 6, 13]);
    }

    #[test]
    fn a_page_shaped_to_need_many_passes_is_settled_in_a_fixed_number() {
        // Thousands of markers Statac refuses, of dollar signs and of
        // words like calls, on one page.
        let mut writing = format!(
            "{}\n{}\n\n{}\n",
            "- one\n  <!--more-->\n".repeat(3_000),
            "$a$b".repeat(3_000),
            "@name(one ".repeat(3_000)
        );
        let started = std::time::Instant::now();
        let settled = settle(&mut writing, &mut [], &mut [], Maths::default());
        assert!(started.elapsed() < std::time::Duration::from_secs(20));
        assert_eq!(settled.markers_out, 3_000);
        assert!(!writing.contains("<!--more-->"));
    }
}
