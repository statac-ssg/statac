//! What a list or a description shows of a page's writing: its excerpt and
//! its opening words, and how many words a reader reads in it.

use std::ops::Range;

use super::Document;
use super::node::{Kind, Node};
use crate::html::{Markup, markup};
use crate::problem::{Kind as ProblemKind, MoreAt, Problem, Word, WrongPlace};
use crate::source::{Place, Span};

/// The writing as HTML, with what is shown of it elsewhere.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Written {
    pub html: String,
    /// The writing before the more marker, or else its opening paragraph,
    /// as HTML, without the footnotes' markers; none where that is empty.
    pub excerpt: Option<String>,
    /// Each piece of the excerpt copied from `html`: where it is in the
    /// excerpt, and where it starts in `html`.
    pub excerpt_from: Vec<(Range<usize>, usize)>,
    /// Where the writing the excerpt shows is written in the page's file;
    /// none for a page written in HTML.
    pub excerpt_span: Option<Span>,
    /// Whether the writing has a more marker, so the excerpt ends where the
    /// author said.
    pub excerpt_marked: bool,
    /// The opening paragraph's words as plain text, each run of space
    /// closed up to one.
    pub opening: Option<String>,
    /// How many words a reader reads: code blocks, the footnotes' markers
    /// and their ways back left out.
    pub words: usize,
    /// Whether the writing holds any maths.
    pub maths: bool,
    /// Where each heading with an id starts in `html`, with its id, in
    /// order: where the part of the writing it heads begins. Footnotes
    /// written at the end fall in the last part.
    pub headings: Vec<(usize, String)>,
    /// Where in `html` each part of the writing is written, with where it
    /// is written in the page's file: nested, a part inside the part
    /// holding it.
    pub traced: Vec<(Range<usize>, Place)>,
    /// Where in `html` each line a snippet call wrote is written.
    pub called: Vec<Called>,
}

/// A line, or the whole, of what a snippet call wrote, as the writing's
/// HTML holds it: where it is in that HTML, where it starts in what the
/// call wrote, and the call's place as the snippets were handed it.
#[derive(Clone, PartialEq, Debug)]
pub struct Called {
    pub at: Range<usize>,
    pub within: usize,
    pub place: Place,
}

/// Which of the writing's blocks holds the more marker that ends the
/// excerpt, and whether it ends after that block: the first `<!--more-->`
/// standing on a line of its own at the top of the writing, as a block of
/// its own or as the last line of a block of HTML, outside every element
/// the author's HTML holds open.
pub(super) fn more_marker(document: &Document) -> Option<(usize, bool)> {
    let root = &document.root;
    root.children.iter().enumerate().find_map(|(at, block)| {
        let (written, after, _) = ending_marker(block)?;
        let held = held_open(document, block, after);
        (is_more(&written) && held.is_none()).then_some((at, after))
    })
}

/// The innermost element the author's HTML holds open round the marker
/// `ending_marker` finds in `block`, by its name as written.
fn held_open<'a>(document: &'a Document, block: &Node, after: bool) -> Option<&'a str> {
    let line = match after {
        true => block.children.last(),
        false => block.children.first(),
    }?;
    (document.held_open.iter())
        .find(|(at, _)| *at == line.place.span.start)
        .map(|(_, name)| name.as_str())
}

/// A marker in any case at the top of the writing, as written, where it
/// could end the excerpt: the whole of a block of HTML, or its last line.
/// With it, whether it is that last line, and where it is written.
fn ending_marker(block: &Node) -> Option<(String, bool, Place)> {
    let whole = block_text(block)?;
    if more_spelled(whole.trim()).is_some() {
        return Some((whole.trim().to_owned(), false, block.place));
    }
    let last = block.children.last()?;
    match &last.kind {
        Kind::Html(line) if more_spelled(line.trim()).is_some() => {
            Some((line.trim().to_owned(), true, last.place))
        }
        _ => None,
    }
}

/// A block of the author's HTML as written, where it holds no call.
fn block_text(block: &Node) -> Option<String> {
    if !matches!(block.kind, Kind::HtmlBlock) {
        return None;
    }
    let mut text = String::new();
    for line in &block.children {
        match &line.kind {
            Kind::Html(html) => text.push_str(html),
            _ => return None,
        }
    }
    Some(text)
}

/// `<!--more-->`, with any space either side of `more`.
fn is_more(text: &str) -> bool {
    more_spelled(text) == Some(true)
}

/// Whether `text` is the more marker spelled as Statac spells it: some
/// where it is, none where it is `more` in no case at all.
fn more_spelled(text: &str) -> Option<bool> {
    let inside = text.strip_prefix("<!--")?.strip_suffix("-->")?.trim();
    inside
        .eq_ignore_ascii_case("more")
        .then_some(inside == "more")
}

/// Every more marker Statac can't act on: one spelled another way and one
/// where it can't end an excerpt, which stop; and one after the marker
/// that does, which is warned of.
pub(super) fn misplaced_markers(document: &Document, text: &str) -> Vec<Problem> {
    let mut markers = Markers {
        text,
        first: None,
        problems: Vec::new(),
    };
    for block in &document.root.children {
        match ending_marker(block) {
            Some((written, after, place)) => {
                if after {
                    for line in &block.children[..block.children.len() - 1] {
                        markers.comments(line, MoreAt::Html);
                    }
                }
                let place = markers.narrowed(place, &written);
                let held = held_open(document, block, after);
                let within = held.map(|name| MoreAt::Element(name.to_owned()));
                markers.marker(&written, place, within);
            }
            None => markers.visit(block, None),
        }
    }
    for note in &document.notes {
        markers.visit(note, Some(MoreAt::Footnote));
    }
    markers.problems
}

struct Markers<'a> {
    text: &'a str,
    /// The marker that ends the excerpt, as written.
    first: Option<(String, Place)>,
    problems: Vec<Problem>,
}

impl Markers<'_> {
    /// A marker as written at `place`, inside `within`, or standing at the
    /// top of the writing where that is none.
    fn marker(&mut self, written: &str, place: Place, within: Option<MoreAt>) {
        let at = |why| ProblemKind::WrongPlace {
            key: written.to_owned(),
            why: WrongPlace::More(why),
        };
        let problem = match (more_spelled(written), within, &self.first) {
            (Some(false), ..) => Problem::at(
                ProblemKind::OtherSpelling {
                    word: Word::Marker,
                    written: written.to_owned(),
                    ours: "<!--more-->".into(),
                },
                place,
            ),
            (_, Some(within), _) => Problem::at(at(within), place),
            (_, None, None) => {
                self.first = Some((written.to_owned(), place));
                return;
            }
            (_, None, Some((first, first_at))) => Problem::at(
                ProblemKind::MoreWrittenTwice {
                    written: first.clone(),
                    first: *first_at,
                },
                place,
            ),
        };
        self.problems.push(problem);
    }

    fn visit(&mut self, node: &Node, within: Option<MoreAt>) {
        let within = match &node.kind {
            Kind::List { .. } | Kind::DefinitionList => Some(MoreAt::List),
            Kind::Quotation | Kind::Callout(_) => Some(MoreAt::Quotation),
            Kind::Call(call) => Some(MoreAt::Call(call.name.value.clone())),
            Kind::FootnoteDefinition { .. } => Some(MoreAt::Footnote),
            Kind::CodeBlock { .. } | Kind::Code(_) | Kind::Literal(_) => return,
            Kind::HtmlBlock => {
                match block_text(node) {
                    Some(written) if more_spelled(written.trim()).is_some() => {
                        self.marker(written.trim(), node.place, within);
                    }
                    _ => {
                        for line in &node.children {
                            self.comments(line, MoreAt::Html);
                        }
                    }
                }
                return;
            }
            _ => within,
        };
        let alone = node.children.iter().all(|child| {
            matches!(child.kind, Kind::Html(_) | Kind::SoftBreak)
                || matches!(&child.kind, Kind::Text(text) if text.trim().is_empty())
        });
        for child in &node.children {
            match &child.kind {
                Kind::Html(html) if alone && more_spelled(html.trim()).is_some() => {
                    let place = child.place;
                    self.marker(
                        html.trim(),
                        place,
                        Some(within.clone().unwrap_or(MoreAt::Words)),
                    );
                }
                Kind::Html(_) => self.comments(child, MoreAt::Words),
                _ => self.visit(child, within.clone()),
            }
        }
    }

    /// The markers among the comments of one line of HTML: alone on the
    /// line, `alone` says where they stand; otherwise among other writing.
    fn comments(&mut self, line: &Node, alone: MoreAt) {
        let Kind::Html(html) = &line.kind else {
            return;
        };
        for comment in super::html::comments_in(html) {
            let written = &html[comment.clone()];
            if more_spelled(written).is_none() {
                continue;
            }
            let within = match html.trim() == written {
                true => alone.clone(),
                false => MoreAt::Words,
            };
            let place = self.narrowed(line.place, written);
            self.marker(written, place, Some(within));
        }
    }

    /// The place of `written` inside `place`, where it is found as typed.
    fn narrowed(&self, place: Place, written: &str) -> Place {
        let start = place.span.start as usize;
        match self.text[place.span.range()].find(written) {
            Some(at) => Place {
                span: Span::new(start + at, start + at + written.len()),
                ..place
            },
            None => place,
        }
    }
}

/// A page written in HTML, read as it is published: its excerpt ends at a
/// more marker on a line of its own, and its opening is its first `<p>`.
pub fn as_written(html: &str) -> Written {
    let mut at = 0;
    let mut more_at = None;
    for line in html.split_inclusive('\n') {
        if is_more(line.trim()) {
            more_at = Some(at);
            break;
        }
        at += line.len();
    }
    let opening = first_html_paragraph(html);
    let (excerpt, excerpt_from) = match (more_at, opening.clone()) {
        (Some(at), _) => {
            let kept = html[..at].trim_end();
            let start = kept.len() - kept.trim_start().len();
            let kept = &kept[start..];
            let from = (!kept.is_empty()).then_some((0..kept.len(), start));
            (
                Some(kept.to_owned()).filter(|kept| !kept.is_empty()),
                Vec::from_iter(from),
            )
        }
        (None, Some(inner)) => {
            let opens = "<p>".len();
            let from = vec![(opens..opens + inner.len(), inner.start)];
            (Some(format!("<p>{}</p>", &html[inner])), from)
        }
        (None, None) => (None, Vec::new()),
    };
    Written {
        html: html.to_owned(),
        excerpt,
        excerpt_from,
        excerpt_span: None,
        excerpt_marked: more_at.is_some(),
        opening: opening.map(|inner| closed_up(&words_of(&html[inner]))),
        words: count_words(html),
        maths: false,
        headings: headings_in_html(html),
        traced: Vec::new(),
        called: Vec::new(),
    }
}

/// Where each heading with an id starts in `html`, with its id as written.
fn headings_in_html(html: &str) -> Vec<(usize, String)> {
    let heading = |name: &str| matches!(name, "h1" | "h2" | "h3" | "h4" | "h5" | "h6");
    markup(html)
        .filter_map(|(at, piece)| match piece {
            Markup::Start(tag) if heading(&tag.name) => {
                let id = tag.get("id").filter(|id| !id.is_empty())?;
                Some((at.start, id.to_owned()))
            }
            _ => None,
        })
        .collect()
}

/// Where the first `<p>` holding words holds them in `html`, without the
/// space around them.
pub(super) fn first_html_paragraph(html: &str) -> Option<Range<usize>> {
    let mut pieces = markup(html);
    while let Some((at, piece)) = pieces.next() {
        let Markup::Start(tag) = piece else {
            continue;
        };
        if tag.name != "p" {
            continue;
        }
        let inside = at.end;
        let closes = pieces.find_map(|(at, piece)| match piece {
            Markup::End(tag) if tag.name == "p" => Some(at.start),
            _ => None,
        })?;
        let held = &html[inside..closes];
        if !closed_up(&words_of(held)).is_empty() {
            let start = inside + (held.len() - held.trim_start().len());
            return Some(start..inside + held.trim_end().len());
        }
    }
    None
}

/// Words with every run of space closed up to one, none at either end.
pub fn closed_up(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// How many words a reader reads in a piece of HTML, anything inside a
/// `<pre>` left out as code. A run between spaces holding a letter or a
/// figure is a word, so a lone dash or ampersand is none; in writing that
/// runs on without spaces, Chinese and Japanese, each character is one.
pub(super) fn count_words(html: &str) -> usize {
    let mut words = 0;
    for token in read_words(html, true, false).split_whitespace() {
        let mut in_a_word = false;
        for c in token.chars() {
            if runs_without_spaces(c) {
                words += 1;
                in_a_word = false;
            } else if c.is_alphanumeric() && !in_a_word {
                words += 1;
                in_a_word = true;
            }
        }
    }
    words
}

/// Whether `c` is written in Han, hiragana or katakana.
fn runs_without_spaces(c: char) -> bool {
    matches!(
        c,
        '\u{2e80}'..='\u{2fdf}'
            | '\u{3005}'..='\u{3007}'
            | '\u{3021}'..='\u{3029}'
            | '\u{3038}'..='\u{303b}'
            | '\u{3041}'..='\u{309f}'
            | '\u{30a1}'..='\u{30ff}'
            | '\u{31f0}'..='\u{31ff}'
            | '\u{3400}'..='\u{4dbf}'
            | '\u{4e00}'..='\u{9fff}'
            | '\u{f900}'..='\u{faff}'
            | '\u{ff66}'..='\u{ff9f}'
            | '\u{20000}'..='\u{323af}'
    )
}

/// The words of a piece of HTML, its tags taken off and its character
/// references read. A line break or a block's edge parts the words either
/// side, though no space stands between them.
pub fn words_of(html: &str) -> String {
    read_words(html, false, false)
}

/// [`words_of`] with a line break wherever a line break or a block's edge
/// parts the words, and every other run of white space one space.
pub fn lines_of(html: &str) -> String {
    let marked = read_words(html, false, true);
    let mut out = String::with_capacity(marked.len());
    let mut gap: Option<char> = None;
    for c in marked.chars() {
        if c.is_whitespace() {
            if c == EDGE || gap.is_none() {
                gap = Some(if c == EDGE { '\n' } else { ' ' });
            }
            continue;
        }
        if let Some(gap) = gap.take().filter(|_| !out.is_empty()) {
            out.push(gap);
        }
        out.push(c);
    }
    out
}

/// What [`read_words`] marks a block's edge with, for [`lines_of`]: the
/// paragraph separator, which is white space a text rarely holds.
const EDGE: char = '\u{2029}';

/// Elements whose text a browser reads without markup and doesn't show:
/// `iframe`, `noembed` and `noframes` show theirs only where the browser
/// can't do what they stand in for, and `noscript` only where scripts
/// are off.
pub(super) const UNSEEN: &[&str] = &[
    "script", "style", "title", "iframe", "noembed", "noframes", "noscript",
];

/// [`words_of`], each `<pre>` element taken out where `without_code`, a
/// space kept in its place so the words either side stay apart, and each
/// block's edge marked with [`EDGE`] where `edges`. What a browser doesn't
/// show is taken out always.
fn read_words(html: &str, without_code: bool, edges: bool) -> String {
    let mut out = String::with_capacity(html.len());
    let mut parted = false;
    let mut from = 0;
    let mut pieces = markup(html);
    while let Some((at, piece)) = pieces.next() {
        text_words(&mut out, &html[from..at.start], &mut parted, edges);
        from = at.end;
        let mut past_end = |name: &str| {
            pieces
                .find_map(|(at, piece)| match piece {
                    Markup::End(tag) if tag.name == name => Some(at.end),
                    _ => None,
                })
                .unwrap_or(html.len())
        };
        match piece {
            Markup::Start(tag) if without_code && tag.name == "pre" => {
                out.push(' ');
                from = past_end("pre");
            }
            Markup::Start(tag) if UNSEEN.contains(&tag.name.as_str()) => {
                from = past_end(&tag.name);
            }
            Markup::Start(tag) | Markup::End(tag) => parted |= parts_words(&tag.name),
            Markup::Other => {}
        }
    }
    text_words(&mut out, &html[from..], &mut parted, edges);
    out
}

/// The words of `text`, standing between tags, onto `out`: its character
/// references read, and parted from the words before where `parted` says,
/// by [`EDGE`] where `edges`.
fn text_words(out: &mut String, text: &str, parted: &mut bool, edges: bool) {
    if text.is_empty() {
        return;
    }
    if std::mem::take(parted) {
        match edges {
            true => out.push(EDGE),
            false => part(out, text),
        }
    }
    references_read_into(out, text);
}

/// `text` with its character references read as the characters they
/// stand for.
pub fn references_read(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    references_read_into(&mut out, text);
    out
}

fn references_read_into(out: &mut String, text: &str) {
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let end = rest[1..]
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '#'))
            .map(|n| n + 1);
        let reference = end.filter(|&e| rest.as_bytes().get(e) == Some(&b';'));
        match reference.and_then(|e| read_reference(&rest[..=e]).map(|read| (read, e))) {
            Some((read, e)) => {
                out.push_str(&read);
                rest = &rest[e + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
}

/// A space between the words so far and `next`, where neither brings one.
fn part(out: &mut String, next: &str) {
    if !out.is_empty()
        && !next.is_empty()
        && !out.ends_with(char::is_whitespace)
        && !next.starts_with(char::is_whitespace)
    {
        out.push(' ');
    }
}

/// Whether a tag named `name` stands between words: a line break, or a
/// block's start or end.
fn parts_words(name: &str) -> bool {
    const PARTING: &[&str] = &[
        "br",
        "p",
        "div",
        "li",
        "ul",
        "ol",
        "dl",
        "dt",
        "dd",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "blockquote",
        "pre",
        "hr",
        "table",
        "tr",
        "td",
        "th",
        "section",
        "article",
        "aside",
        "header",
        "footer",
        "figure",
        "figcaption",
    ];
    PARTING.contains(&name)
}

/// A character reference as written, `&amp;` or `&#8212;`, read as the
/// characters it stands for.
fn read_reference(written: &str) -> Option<String> {
    let name = &written[1..written.len() - 1];
    // Digits too many to read are a number past Unicode all the same.
    let number = |digits: &str, radix| {
        let all = !digits.is_empty() && digits.chars().all(|c| c.is_digit(radix));
        let read = u32::from_str_radix(digits, radix)
            .ok()
            .or(all.then_some(u32::MAX));
        read.map(crate::html::numbered)
    };
    let c = match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some('\u{a0}'),
        r => match r.strip_prefix("#x").or_else(|| r.strip_prefix("#X")) {
            Some(hex) => number(hex, 16),
            None if r.starts_with('#') => number(&r[1..], 10),
            // HTML's other names, which CommonMark reads alike.
            None => return named_reference(written),
        },
    };
    c.map(String::from)
}

fn named_reference(written: &str) -> Option<String> {
    use pulldown_cmark::{Event, Parser, Tag, TagEnd};
    let mut read = String::new();
    for event in Parser::new(written) {
        match event {
            Event::Text(text) => read.push_str(&text),
            Event::Start(Tag::Paragraph) | Event::End(TagEnd::Paragraph) => {}
            _ => return None,
        }
    }
    (read != written).then_some(read)
}
