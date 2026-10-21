//! Typography: straight quotes set as the language's own marks, `--` and
//! `---` as dashes, `...` as an ellipsis, and a space typed against a mark
//! the language keeps it with made one a line can't break at. Only
//! punctuation changes; the one thing added is the space a language writes
//! just inside the quotes typography sets.
//!
//! A block's words are read as one run, so a quotation pairs across
//! emphasis and links. A quote's side is read from its neighbours as
//! CommonMark reads emphasis, over the characters as written: a space
//! before and a letter after opens. Code, raw HTML, maths, calls, a link's
//! address, an address written bare and whatever reaches the page as typed
//! are never changed, and a quote among them pairs with none outside.

mod addresses;
#[cfg(test)]
mod tests;

use std::ops::Range;

use unicode_normalization::char::is_combining_mark;

use super::Document;
use super::node::{Kind, Node};
use crate::html::{Markup, markup};
use crate::language::{Quotes, Words};
use crate::source::{SourceId, Sources, Span};

/// The characters typography may change among the words.
pub(super) const SETS: [char; 4] = ['\'', '"', '-', '.'];

/// The marks a language's writing is set in.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Typography<'a> {
    /// None where the language gives no marks: quotes then stay as typed,
    /// though an apostrophe, a dash and an ellipsis are still set.
    pub quotes: Option<Quotes>,
    /// The space written just inside each pair of outer quotes typography
    /// sets.
    pub inside_quotes: Option<char>,
    /// Marks a typed space before them is kept with.
    pub no_break_before: &'a [char],
    /// Marks a typed space after them is kept with.
    pub no_break_after: &'a [char],
}

impl<'a> Typography<'a> {
    pub fn of(words: &'a Words) -> Self {
        Typography {
            quotes: words.quotes,
            inside_quotes: words.quotes.and(words.inside_quotes),
            no_break_before: &words.no_break_before,
            no_break_after: &words.no_break_after,
        }
    }
}

const STRAIGHT: Quotes = Quotes {
    open: '"',
    close: '"',
    inner_open: '\'',
    inner_close: '\'',
};

const APOSTROPHE: char = '\u{2019}';
const NO_BREAK: char = '\u{a0}';
/// What a call stands as among the words: neither space nor punctuation.
const WORD: char = 'x';
/// What a block's edge and a line break stand as.
const LINE: char = '\n';

/// Sets the punctuation of every block's words, the footnotes' too.
pub fn set_typography(document: &mut Document, sources: &Sources, typography: &Typography<'_>) {
    let mut set = |nodes: &mut Vec<Node>, source| run(nodes, sources.text(source), typography);
    blocks(&mut document.root, &mut set);
    for note in &mut document.notes {
        blocks(note, &mut set);
    }
}

/// Words that reach the page without ever standing in the writing, such as
/// a caption handed to a call, set as the writing around them is: read as
/// Markdown of their own, with only the words changed and every other
/// character, emphasis and code among them, as typed.
pub fn set_words(text: &str, typography: &Typography<'_>) -> String {
    fn texts(node: &Node, out: &mut Vec<(Span, String)>) {
        node.walk(&mut |n| {
            if let Kind::Text(text) = &n.kind {
                out.push((n.place.span, text.clone()));
            }
        });
    }
    let mut sources = Sources::default();
    let id = sources.add("", text.to_owned());
    let mut document = super::read(&sources, id, Span::new(0, text.len()), &|_| None, "");
    let (mut before, mut after) = (Vec::new(), Vec::new());
    for top in std::iter::once(&document.root).chain(&document.notes) {
        texts(top, &mut before);
    }
    set_typography(&mut document, &sources, typography);
    for top in std::iter::once(&document.root).chain(&document.notes) {
        texts(top, &mut after);
    }
    let mut out = String::with_capacity(text.len());
    let mut written = 0;
    for ((span, was), (_, now)) in before.iter().zip(&after) {
        let range = span.range();
        // Words the parser read from escapes are left as typed whole.
        if was != now && range.start >= written && text.get(range.clone()) == Some(was) {
            out.push_str(&text[written..range.start]);
            out.push_str(now);
            written = range.end;
        }
    }
    out.push_str(&text[written..]);
    out
}

/// The words of a link's or picture's title, which reach the page in an
/// attribute rather than among the words, set as words of their own.
pub(super) fn tidied(words: &str, typography: &Typography<'_>) -> String {
    let mut glyphs = vec![Glyph::edge(LINE, Is::Parting)];
    glyphs.extend(words.chars().map(|c| Glyph {
        c,
        is: Is::Writing,
        text: Some(0),
    }));
    glyphs.push(Glyph::edge(LINE, Is::Parting));
    set(&mut glyphs, 1, typography).pop().unwrap_or_default()
}

/// Hands `f` the words of every block under `node` that holds words, with
/// the source they are written in.
pub(super) fn blocks(node: &mut Node, f: &mut impl FnMut(&mut Vec<Node>, SourceId)) {
    match node.kind {
        Kind::CodeBlock { .. } => {}
        Kind::Paragraph { .. } | Kind::Heading { .. } | Kind::TableCell | Kind::Term => {
            f(&mut node.children, node.place.source);
        }
        _ => {
            for child in &mut node.children {
                blocks(child, f);
            }
        }
    }
}

/// One block's words, read as a run and written back.
fn run(nodes: &mut [Node], source: &str, typography: &Typography<'_>) {
    let mut reading = Reading::of(nodes, source);
    let texts = reading.texts;
    let set = set(&mut reading.glyphs, texts, typography);
    write(nodes, &mut set.into_iter(), typography);
}

/// An address written bare among a block's words, placed by the text and
/// literal nodes it is written in, counted in the order written through
/// emphasis, links and pictures.
pub(super) struct Bare {
    /// The node it starts in, and the character it starts at there.
    pub from: (usize, usize),
    /// The node it ends in, and the character just past its end there.
    pub to: (usize, usize),
    /// Where a link to it points.
    pub destination: String,
    /// Whether it is all writing, rather than inside a link, a picture's
    /// description or an `<a>`, or in part typed as an escape or an entity,
    /// or shown as typed in raw HTML.
    pub linkable: bool,
    /// Whether it is a mail address.
    pub mail: bool,
}

/// Every address written bare among a block's words, found as typography
/// finds them.
pub(super) fn bare_addresses(nodes: &[Node], source: &str) -> Vec<Bare> {
    let reading = Reading::of(nodes, source);
    let words = &reading.words;
    address_ranges(&reading.glyphs)
        .into_iter()
        .map(|(range, address)| {
            let first = words.partition_point(|word| word.end <= range.start);
            let last = words.partition_point(|word| word.end < range.end);
            let mut destination = address.scheme.to_owned();
            destination.extend(reading.glyphs[range.clone()].iter().map(|glyph| glyph.c));
            Bare {
                from: (first, range.start - words[first].start),
                to: (last, range.end - words[last].start),
                destination,
                linkable: words[first..=last].iter().all(|word| word.linkable),
                mail: address.mail,
            }
        })
        .collect()
}

/// What a character among the words is to typography.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Is {
    /// The writing, which typography may set.
    Writing,
    /// Writing that reaches the page as typed.
    AsTyped,
    /// A character of an address written bare: as typed, and parting the
    /// words around it.
    Address,
    /// A call, which reads as a word.
    Word,
    /// The edge of emphasis or of a link's words, which the words run on
    /// through.
    Through,
    /// What parts the words: a block's edge, a line break, code, raw HTML,
    /// maths, a note's marker, a picture's edges.
    Parting,
}

#[derive(Copy, Clone, Debug)]
struct Glyph {
    /// The character as written; for an edge, the one written there.
    c: char,
    is: Is,
    /// The text node it belongs to, by its place among the run's.
    text: Option<usize>,
}

impl Glyph {
    fn edge(c: char, is: Is) -> Glyph {
        Glyph { c, is, text: None }
    }

    fn words(self) -> bool {
        matches!(self.is, Is::Writing | Is::AsTyped)
    }
}

struct Reading<'s> {
    source: &'s str,
    glyphs: Vec<Glyph>,
    texts: usize,
    /// How many elements of the author's HTML shown as typed are open:
    /// `kbd` and `samp`, whose words are keys and a program's output.
    shown_as_typed: usize,
    /// Each text and literal node read, by the glyphs it became.
    words: Vec<Word>,
    /// How many links and pictures the words are inside.
    links: usize,
    /// How many of the author's `<a>` elements are open.
    anchors: usize,
}

struct Word {
    start: usize,
    end: usize,
    linkable: bool,
}

impl<'s> Reading<'s> {
    /// Reads a block's words, between the edges that stand for the block's.
    fn of(nodes: &[Node], source: &'s str) -> Self {
        let mut reading = Reading {
            source,
            glyphs: vec![Glyph::edge(LINE, Is::Parting)],
            texts: 0,
            shown_as_typed: 0,
            words: Vec::new(),
            links: 0,
            anchors: 0,
        };
        reading.nodes(nodes, false);
        reading.glyphs.push(Glyph::edge(LINE, Is::Parting));
        reading
    }

    fn nodes(&mut self, nodes: &[Node], as_typed: bool) {
        for node in nodes {
            let written = &self.source[node.place.span.range()];
            let first = written.chars().next().unwrap_or('`');
            let last = written.chars().next_back().unwrap_or('`');
            match &node.kind {
                Kind::Text(text) => {
                    let is = match as_typed || self.shown_as_typed > 0 {
                        true => Is::AsTyped,
                        false => Is::Writing,
                    };
                    let at = Some(self.texts);
                    self.texts += 1;
                    let start = self.glyphs.len();
                    self.glyphs
                        .extend(text.chars().map(|c| Glyph { c, is, text: at }));
                    self.words.push(Word {
                        start,
                        end: self.glyphs.len(),
                        linkable: self.links == 0 && self.anchors == 0,
                    });
                }
                Kind::Literal(text) => {
                    let start = self.glyphs.len();
                    self.glyphs
                        .extend(text.chars().map(|c| Glyph::edge(c, Is::AsTyped)));
                    self.words.push(Word {
                        start,
                        end: self.glyphs.len(),
                        linkable: false,
                    });
                }
                Kind::Call(_) => self.glyphs.push(Glyph::edge(WORD, Is::Word)),
                Kind::Html(html) => {
                    self.shown_as_typed = open_after(html, &["kbd", "samp"], self.shown_as_typed);
                    self.anchors = open_after(html, &["a"], self.anchors);
                    let last = html.trim_end().chars().next_back().unwrap_or('>');
                    self.glyphs.push(Glyph::edge(last, Is::Parting));
                }
                Kind::SoftBreak | Kind::HardBreak => {
                    self.glyphs.push(Glyph::edge(LINE, Is::Parting));
                }
                Kind::Emphasis | Kind::Strong | Kind::Strikethrough => {
                    self.within(node, first, last, Is::Through, as_typed);
                }
                // An address in angle brackets, or written bare, is its own
                // words.
                Kind::Link { .. } if first != '[' => {
                    self.within(node, first, last, Is::Parting, true);
                }
                Kind::Link { .. } => self.within(node, first, last, Is::Through, as_typed),
                Kind::Image { .. } => self.within(node, first, last, Is::Parting, as_typed),
                _ => self.glyphs.push(Glyph::edge(last, Is::Parting)),
            }
        }
    }

    fn within(&mut self, node: &Node, first: char, last: char, edge: Is, as_typed: bool) {
        let link = matches!(node.kind, Kind::Link { .. } | Kind::Image { .. });
        self.links += usize::from(link);
        self.glyphs.push(Glyph::edge(first, edge));
        self.nodes(&node.children, as_typed);
        self.glyphs.push(Glyph::edge(last, edge));
        self.links -= usize::from(link);
    }
}

/// How many elements named one of `names`, in small letters, are open
/// after the markup `html`, `open` of them before it. As in a browser, a
/// `/` before a tag's `>` closes nothing.
fn open_after(html: &str, names: &[&str], open: usize) -> usize {
    markup(html).fold(open, |open, (_, piece)| match piece {
        Markup::Start(tag) if names.contains(&tag.name.as_str()) => open + 1,
        Markup::End(tag) if names.contains(&tag.name.as_str()) => open.saturating_sub(1),
        _ => open,
    })
}

/// The run's text nodes as typography sets them, in order.
fn set(glyphs: &mut [Glyph], texts: usize, typography: &Typography<'_>) -> Vec<String> {
    find_addresses(glyphs);
    let mut out: Vec<Option<String>> = vec![None; glyphs.len()];
    dashes_and_ellipses(glyphs, &mut out);
    quotes(glyphs, &mut out, typography);
    let mut flat = flattened(glyphs, &out);
    keep_together(&mut flat, typography);
    let mut written = vec![String::new(); texts];
    for piece in flat {
        if let Flat::Char {
            c, text: Some(at), ..
        } = piece
        {
            written[at].push(c);
        }
    }
    written
}

/// Marks every address written bare among the words.
fn find_addresses(glyphs: &mut [Glyph]) {
    for (range, _) in address_ranges(glyphs) {
        for glyph in &mut glyphs[range] {
            glyph.is = Is::Address;
        }
    }
}

/// Where each address written bare among the words is, by its glyphs, and
/// what was found there.
fn address_ranges(glyphs: &[Glyph]) -> Vec<(Range<usize>, addresses::Address)> {
    let mut ranges = Vec::new();
    let mut at = 0;
    while at < glyphs.len() {
        if !glyphs[at].words() {
            at += 1;
            continue;
        }
        let start = at;
        let mut words = String::new();
        let mut offsets = Vec::new();
        while at < glyphs.len() && glyphs[at].words() {
            offsets.push(words.len());
            words.push(glyphs[at].c);
            at += 1;
        }
        // Every address holds a full stop, an `@` or a scheme's `:`.
        if !words.contains(['.', '@', ':']) {
            continue;
        }
        let before = glyphs[start - 1].c;
        for found in addresses::found(&words, before) {
            let glyph = |offset: usize| start + offsets.partition_point(|o| *o < offset);
            ranges.push((glyph(found.at.start)..glyph(found.at.end), found));
        }
    }
    ranges
}

/// A run of hyphens as dashes, and three full stops as an ellipsis, as
/// pulldown-cmark sets them: two hyphens an en dash, three an em dash, and
/// a longer run as many em dashes as divide it evenly, then en dashes.
fn dashes_and_ellipses(glyphs: &[Glyph], out: &mut [Option<String>]) {
    let alike = |at: usize, c: char, of: &Glyph| {
        glyphs
            .get(at)
            .is_some_and(|g| g.is == Is::Writing && g.c == c && g.text == of.text)
    };
    let mut at = 0;
    while at < glyphs.len() {
        let glyph = &glyphs[at];
        if glyph.is != Is::Writing {
            at += 1;
            continue;
        }
        match glyph.c {
            '-' => {
                let count = (at..).take_while(|&k| alike(k, '-', glyph)).count();
                if count > 1 {
                    out[at] = Some(dashes(count));
                    for gone in &mut out[at + 1..at + count] {
                        *gone = Some(String::new());
                    }
                }
                at += count;
            }
            '.' if alike(at + 1, '.', glyph) && alike(at + 2, '.', glyph) => {
                out[at] = Some('\u{2026}'.to_string());
                out[at + 1] = Some(String::new());
                out[at + 2] = Some(String::new());
                at += 3;
            }
            _ => at += 1,
        }
    }
}

fn dashes(count: usize) -> String {
    let (ems, ens) = match count {
        2 => (0, 1),
        3 => (1, 0),
        _ => match count % 6 {
            0 | 3 => (count / 3, 0),
            2 | 4 => (0, count / 2),
            1 => (count / 3 - 1, 2),
            _ => (count / 3, 1),
        },
    };
    let mut written = "\u{2014}".repeat(ems);
    written.push_str(&"\u{2013}".repeat(ens));
    written
}

/// Every straight quote among the writing set as a mark. Single quotes
/// pair as CommonMark's smart punctuation pairs them, and any left over is
/// an apostrophe. Double quotes pair from the inside out; where the
/// neighbours can't tell a quote's side, as between two words of Chinese
/// or two spaces, quotes take turns; and one that closes nothing, or never
/// finds its other half, is left as typed.
fn quotes(glyphs: &[Glyph], out: &mut [Option<String>], typography: &Typography<'_>) {
    let marks = typography.quotes.unwrap_or(STRAIGHT);
    let space = typography.quotes.and(typography.inside_quotes);
    let opening = |at: usize| match space {
        Some(space) if !glyphs[at + 1].c.is_whitespace() => format!("{}{space}", marks.open),
        _ => marks.open.to_string(),
    };
    let closing = |at: usize| match space {
        Some(space) if !glyphs[at - 1].c.is_whitespace() => format!("{space}{}", marks.close),
        _ => marks.close.to_string(),
    };
    let mut single: Option<usize> = None;
    // Each double quote open, with whether its side could be told.
    let mut double: Vec<(usize, bool)> = Vec::new();
    for at in 1..glyphs.len().saturating_sub(1) {
        let glyph = glyphs[at];
        if glyph.is != Is::Writing || !matches!(glyph.c, '\'' | '"') {
            continue;
        }
        let (before, after) = (glyphs[at - 1].c, glyphs[at + 1].c);
        let can_open = can_open(glyph.c, before, after);
        let can_close = can_close(before, after);
        if glyph.c == '\'' {
            // Chinese, Japanese and Korean write a word straight after a
            // closing quote, as Korean does its particles, `'Python'을`,
            // where an apostrophe never stands.
            let can_close = can_close || (!can_open && is_cjk(after) && !before.is_whitespace());
            match single {
                Some(open) if can_close => {
                    out[open] = Some(marks.inner_open.to_string());
                    out[at] = Some(marks.inner_close.to_string());
                    single = None;
                }
                _ => {
                    if can_open {
                        single = Some(at);
                    }
                    out[at] = Some(APOSTROPHE.to_string());
                }
            }
            continue;
        }
        // Punctuation after a quote tells nothing either: `" oui ",`
        // closes as surely as `"...et"` opens.
        let told = can_open != can_close
            && !is_cjk(before)
            && !is_cjk(after)
            && !(can_open && is_punctuation(after));
        match double.last() {
            Some(&(open, _)) if !(told && can_open) => {
                double.pop();
                out[open] = Some(opening(open));
                out[at] = Some(closing(at));
            }
            _ if told && can_close => {}
            _ => double.push((at, told)),
        }
    }
    // A quotation running on into the next paragraph opens each one and
    // closes only the last.
    for (open, told) in double {
        if told {
            out[open] = Some(opening(open));
        }
    }
}

/// Whether a quote between `before` and `after` can open, by CommonMark's
/// left-flanking rule.
fn can_open(quote: char, before: char, after: char) -> bool {
    if after.is_whitespace() {
        return false;
    }
    before.is_whitespace()
        || is_punctuation(before) && (quote != '\'' || !matches!(before, ']' | ')'))
}

/// Whether a quote between `before` and `after` can close, by CommonMark's
/// right-flanking rule.
fn can_close(before: char, after: char) -> bool {
    !before.is_whitespace() && (after.is_whitespace() || is_punctuation(after))
}

/// Unicode punctuation as CommonMark has it, its marks and symbols:
/// whatever is no letter, figure, space, combining mark or character
/// that shows nothing.
fn is_punctuation(c: char) -> bool {
    if c.is_ascii() {
        return c.is_ascii_punctuation();
    }
    !(c.is_alphanumeric()
        || c.is_whitespace()
        || c.is_control()
        || is_combining_mark(c)
        || u32::from(c) > 0x1_FBCA
        || matches!(c,
            '\u{ad}'
            | '\u{600}'..='\u{605}'
            | '\u{61c}'
            | '\u{6dd}'
            | '\u{70f}'
            | '\u{180e}'
            | '\u{200b}'..='\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2060}'..='\u{206f}'
            | '\u{e000}'..='\u{f8ff}'
            | '\u{feff}'
            | '\u{fff9}'..='\u{fffb}'))
}

/// Whether `c` is written in Chinese, Japanese or Korean, where words run
/// on without spaces and a quote's neighbours can't tell its side.
fn is_cjk(c: char) -> bool {
    matches!(c,
        '\u{1100}'..='\u{11FF}'
        | '\u{2E80}'..='\u{2FDF}'
        | '\u{3000}'..='\u{303F}'
        | '\u{3040}'..='\u{30FF}'
        | '\u{3100}'..='\u{318F}'
        | '\u{31A0}'..='\u{31FF}'
        | '\u{3400}'..='\u{4DBF}'
        | '\u{4E00}'..='\u{9FFF}'
        | '\u{A960}'..='\u{A97F}'
        | '\u{AC00}'..='\u{D7FF}'
        | '\u{F900}'..='\u{FAFF}'
        | '\u{FE30}'..='\u{FE4F}'
        | '\u{FF00}'..='\u{FFEF}'
        | '\u{20000}'..='\u{3FFFF}')
}

/// The run as it will be written, a character at a time.
#[derive(Copy, Clone, Debug)]
enum Flat {
    Char {
        c: char,
        /// Reaching the page as typed, so never a space kept with a mark.
        typed: bool,
        text: Option<usize>,
    },
    Through,
    Parting,
}

fn flattened(glyphs: &[Glyph], out: &[Option<String>]) -> Vec<Flat> {
    let mut flat = Vec::with_capacity(glyphs.len());
    for (at, glyph) in glyphs.iter().enumerate() {
        let typed = |c| Flat::Char {
            c,
            typed: true,
            text: glyph.text,
        };
        match glyph.is {
            Is::Through => flat.push(Flat::Through),
            Is::Parting => flat.push(Flat::Parting),
            Is::Address => {
                let address = |g: Option<&Glyph>| g.is_some_and(|g| g.is == Is::Address);
                if !address(at.checked_sub(1).and_then(|before| glyphs.get(before))) {
                    flat.push(Flat::Parting);
                }
                flat.push(typed(glyph.c));
                if !address(glyphs.get(at + 1)) {
                    flat.push(Flat::Parting);
                }
            }
            Is::AsTyped | Is::Word => flat.push(typed(glyph.c)),
            Is::Writing => match &out[at] {
                None => flat.push(Flat::Char {
                    c: glyph.c,
                    typed: false,
                    text: glyph.text,
                }),
                Some(set) => flat.extend(set.chars().map(|c| Flat::Char {
                    c,
                    typed: false,
                    text: glyph.text,
                })),
            },
        }
    }
    flat
}

/// Every space typed against a mark the language keeps it with made one a
/// line can't break at: a space just before a `no_break_before` mark, or
/// just after a `no_break_after` one. A quote typography set counts as the
/// mark it was set as. The words run on through emphasis and a link's
/// words, and stop at whatever parts them.
fn keep_together(flat: &mut [Flat], typography: &Typography<'_>) {
    if typography.no_break_before.is_empty() && typography.no_break_after.is_empty() {
        return;
    }
    // Where the last character was a space typed among the writing.
    let mut space: Option<usize> = None;
    let mut after_a_mark = false;
    for at in 0..flat.len() {
        let (c, typed) = match flat[at] {
            Flat::Through => continue,
            Flat::Parting => {
                space = None;
                after_a_mark = false;
                continue;
            }
            Flat::Char { c, typed, .. } => (c, typed),
        };
        if !typed
            && typography.no_break_before.contains(&c)
            && let Some(Flat::Char { c: before, .. }) = space.map(|s| &mut flat[s])
        {
            *before = NO_BREAK;
        }
        let mut now = c;
        if c == ' ' && after_a_mark && !typed {
            now = NO_BREAK;
            if let Flat::Char { c, .. } = &mut flat[at] {
                *c = NO_BREAK;
            }
        }
        space = (now == ' ' && !typed).then_some(at);
        after_a_mark = !typed && typography.no_break_after.contains(&c);
    }
}

/// Writes the run's text nodes back in the order they were read, and sets
/// each title among them.
fn write(nodes: &mut [Node], set: &mut std::vec::IntoIter<String>, typography: &Typography<'_>) {
    for node in nodes {
        match &mut node.kind {
            Kind::Text(text) => *text = set.next().expect("one for each text read"),
            Kind::Link { title, .. } | Kind::Image { title, .. } if !title.is_empty() => {
                *title = tidied(title, typography);
            }
            _ => {}
        }
        if matches!(
            node.kind,
            Kind::Emphasis
                | Kind::Strong
                | Kind::Strikethrough
                | Kind::Link { .. }
                | Kind::Image { .. }
        ) {
            write(&mut node.children, set, typography);
        }
    }
}
