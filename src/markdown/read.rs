//! pulldown-cmark's events folded into the tree, with what was masked put
//! back where the parser left it.

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use pulldown_cmark::{
    Alignment as Aligned, BlockQuoteKind, BrokenLink, CodeBlockKind, Event, LinkType, Options,
    Parser, Tag, TagEnd,
};

use super::call::{IN_SITE, Within};
use super::html::{escape, escape_href};
use super::mask::{Masked, OPEN, Piece, TokenKind, Wrapping};
use super::node::{Alignment, CalloutKind, Kind, Node};
use super::typography::SETS;
use super::{Definition, Document, InSite, controls};
use crate::problem::{Kind as ProblemKind, Problem};
use crate::source::{Place, SourceId, Span};

/// Dollar maths is left to the mask: the parser's own reads prices as maths.
fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_GFM
        | Options::ENABLE_DEFINITION_LIST
        | Options::ENABLE_HEADING_ATTRIBUTES
}

/// The definitions written anywhere on a page whose calls wrap writing,
/// which is read in parts: each part's links and footnote markers find
/// the definitions written in the others.
#[derive(Default)]
pub(super) struct Definitions {
    /// Each link reference's label as written, destination and title, by
    /// its label as matched.
    links: BTreeMap<String, (String, String, String)>,
    /// Each footnote's label, as matched.
    notes: BTreeSet<String>,
}

impl Definitions {
    /// The definitions in the masked writing `masked` of `text`, and in the
    /// writing each of its calls wraps.
    pub fn gather(text: &str, masked: &Masked, base: &str) -> Definitions {
        let mut definitions = Definitions::default();
        definitions.add(text, masked, base);
        definitions
    }

    fn add(&mut self, text: &str, masked: &Masked, base: &str) {
        let restored = |s: &str| restore(masked, text, s);
        let parser = Parser::new_ext(&masked.text, options());
        for (label, definition) in parser.reference_definitions().iter() {
            let label = restored(label);
            self.links.entry(matched(&label)).or_insert_with(|| {
                let title = definition.title.as_deref().unwrap_or_default();
                let destination = address(masked, text, &definition.dest, base, |_, _| {});
                (label, destination, restored(title))
            });
        }
        for event in parser {
            if let Event::Start(Tag::FootnoteDefinition(label)) = event {
                self.notes.insert(restored(&label).to_lowercase());
            }
        }
        for token in &masked.tokens {
            if let TokenKind::Call {
                wrapping: Some(wrapping),
                ..
            } = &token.kind
            {
                self.add(&wrapping.wrapped.text, &wrapping.masked, base);
            }
        }
    }
}

/// A link reference's label as Markdown matches it: in any case, with its
/// spaces run together.
fn matched(label: &str) -> String {
    label
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// The most levels of writing, quotations, lists, emphasis and the like,
/// one inside another, that are read.
pub const DEEPEST_NESTING: usize = 200;

/// Whether `kind` is a level of nesting. A list and its items are one
/// level, counted at the list.
fn is_level(node: &Node) -> bool {
    if let Kind::Call(call) = &node.kind {
        return call.wraps();
    }
    matches!(
        node.kind,
        Kind::Quotation
            | Kind::Callout(_)
            | Kind::List { .. }
            | Kind::DefinitionList
            | Kind::FootnoteDefinition { .. }
            | Kind::Emphasis
            | Kind::Strong
            | Kind::Strikethrough
            | Kind::Link { .. }
            | Kind::Image { .. }
    )
}

/// How many levels `node` holds one inside another, itself included.
fn levels(node: &Node) -> usize {
    let below = node.children.iter().map(levels).max().unwrap_or(0);
    below + usize::from(is_level(node))
}

/// Reads the writing `masked` of `text`, the part of `source` at `body`,
/// and the writing its calls wrap. `page` holds every definition on the
/// page where it is read in parts, and `base` is the path the site is
/// published under.
pub(super) fn build(
    text: &str,
    source: SourceId,
    body: Span,
    masked: &Masked,
    page: Option<&Definitions>,
    base: &str,
) -> Document {
    let mut bodies = Vec::with_capacity(masked.tokens.len());
    let mut definitions = Vec::new();
    let mut in_site = Vec::new();
    for token in &masked.tokens {
        let TokenKind::Call {
            wrapping: Some(wrapping),
            ..
        } = &token.kind
        else {
            bodies.push(None);
            continue;
        };
        let Wrapping { wrapped, masked } = wrapping.as_ref();
        let span = Span::new(0, wrapped.text.len());
        let inner = build(&wrapped.text, source, span, masked, page, base);
        let mut body = inner.root.children;
        let mut problems = inner.problems;
        wrapped.place(&mut body, &mut problems);
        for mut definition in inner.definitions {
            wrapped.span(&mut definition.place.span);
            definitions.push(definition);
        }
        for mut stands in inner.in_site {
            wrapped.span(&mut stands.place.span);
            in_site.push(stands);
        }
        let levels = body.iter().map(levels).max().unwrap_or(0);
        bodies.push(Some(Body {
            nodes: body,
            problems,
            levels,
        }));
    }

    let root = Node::new(Kind::Document, Place { source, span: body });
    let mut builder = Builder {
        text,
        source,
        masked,
        bodies,
        page,
        base,
        in_site: Vec::new(),
        stack: vec![root],
        depth: 0,
        skipping: 0,
        html: Html::default(),
        image: 0,
        open: None,
        literal: Vec::new(),
        keep_literal: !masked.unread.is_empty() || masked.text.contains("]["),
        bracket: None,
        unset: Vec::new(),
        notes: BTreeSet::new(),
        row: None,
        problems: Vec::new(),
    };
    let broken = RefCell::new(Vec::new());
    let found = |link: BrokenLink<'_>| {
        let label = restore(masked, text, &link.reference);
        if let Some((_, destination, title)) =
            page.and_then(|page| page.links.get(&matched(&label)))
        {
            return Some((destination.clone().into(), title.clone().into()));
        }
        broken.borrow_mut().push((link.span, link.link_type, label));
        None
    };
    let parser = Parser::new_with_broken_link_callback(&masked.text, options(), Some(found));
    let mut labels = Vec::new();
    for (label, definition) in parser.reference_definitions().iter() {
        let label = restore(masked, text, label);
        let place = Place {
            source,
            span: masked.map.span(definition.span.clone()),
        };
        let destination = builder.address(&definition.dest);
        labels.push(label.clone());
        definitions.push(Definition {
            label,
            destination,
            place,
        });
    }
    labels.sort_unstable();
    definitions.sort_by_key(|definition| definition.place.span.start);
    for (event, range) in parser.into_offset_iter() {
        builder.event(event, range);
    }
    builder.unbracket();
    while builder.stack.len() > 1 {
        builder.close();
    }
    let root = builder.stack.pop().expect("the root");
    in_site.append(&mut builder.in_site);
    in_site.sort_by_key(|stands| stands.place.span.start);
    let mut problems = builder.problems;
    let labels: Vec<&str> = match page {
        Some(page) => page
            .links
            .values()
            .map(|(label, ..)| label.as_str())
            .collect(),
        None => labels.iter().map(String::as_str).collect(),
    };
    for (range, link_type, label) in broken.into_inner() {
        let mut span = masked.map.span(range);
        if link_type == LinkType::Collapsed && text[span.end as usize..].starts_with("[]") {
            span.end += 2;
        }
        // `[i][j]` after a word is an index, not a link, and words inside
        // raw HTML's code are shown as typed.
        let glued = text[..span.start as usize]
            .chars()
            .next_back()
            .is_some_and(char::is_alphanumeric);
        let literal = builder
            .literal
            .iter()
            .any(|l| l.start <= span.start && span.start < l.end);
        if !matches!(link_type, LinkType::Reference | LinkType::Collapsed) || glued || literal {
            continue;
        }
        let nearest = crate::suggest::nearest(&label, labels.iter().copied());
        problems.push(Problem::at(
            ProblemKind::MissingDefinition {
                written: format!("[{label}]"),
                nearest: nearest.map(|label| format!("[{label}]")),
            },
            Place { source, span },
        ));
    }
    let notes: Vec<String> = match page {
        Some(page) => page.notes.iter().cloned().collect(),
        None => builder.notes.into_iter().collect(),
    };
    for (label, span) in builder.unset {
        let nearest = crate::suggest::nearest(&label, notes.iter().map(String::as_str));
        problems.push(Problem::at(
            ProblemKind::MissingDefinition {
                written: format!("[^{label}]"),
                nearest: nearest.map(|label| format!("[^{label}]")),
            },
            Place { source, span },
        ));
    }
    for (span, found) in &masked.unread {
        if !builder
            .literal
            .iter()
            .any(|l| l.start <= span.start && span.end <= l.end)
        {
            problems.push(Problem::at(
                ProblemKind::MalformedCall {
                    found: found.clone(),
                },
                Place {
                    source,
                    span: *span,
                },
            ));
        }
    }
    for &(span, character) in &masked.left_out {
        problems.push(Problem::at(
            ProblemKind::ControlCharacter { character },
            Place { source, span },
        ));
    }
    problems.sort_by_key(|p| p.place.map(|place| place.span.start));
    Document {
        root,
        notes: Vec::new(),
        definitions,
        in_site,
        problems,
    }
}

/// The writing a call wraps, read.
struct Body {
    nodes: Vec<Node>,
    problems: Vec<Problem>,
    /// The most levels of nesting one inside another in `nodes`.
    levels: usize,
}

struct Builder<'a> {
    text: &'a str,
    source: SourceId,
    masked: &'a Masked,
    /// The writing each call that wraps writing wraps, read, and its
    /// problems, by token; taken when the call becomes a node.
    bodies: Vec<Option<Body>>,
    page: Option<&'a Definitions>,
    base: &'a str,
    /// Each `@url(...)` read as the address it stands for.
    in_site: Vec<InSite>,
    /// The nodes open, innermost last, the root first.
    stack: Vec<Node>,
    /// How many of them are levels of nesting.
    depth: usize,
    /// While above 0, the events of a level past the deepest are being
    /// read past: how many nodes are open inside it, itself included.
    skipping: usize,
    html: Html,
    /// How many images' descriptions the words are inside.
    image: usize,
    /// A token over several lines that has become a node, whose later
    /// pieces and the line breaks between them are dropped.
    open: Option<usize>,
    /// What the parser read as text that is code, maths or excluded raw
    /// HTML, where an unreadable call or a reference is nobody's mistake.
    literal: Vec<Span>,
    keep_literal: bool,
    /// The texts since a `[` that may open a footnote marker the parser
    /// found no note for, each with its place.
    bracket: Option<Vec<(String, Span)>>,
    /// Footnote markers with no note on the page: each label, and where
    /// the marker is.
    unset: Vec<(String, Span)>,
    /// The labels of the footnotes met, as matched.
    notes: BTreeSet<String>,
    /// The table row being read: where it ends in the masked text, and
    /// where the last cell the parser keeps of it ends.
    row: Option<(usize, usize)>,
    problems: Vec<Problem>,
}

impl<'a> Builder<'a> {
    fn event(&mut self, event: Event<'_>, range: std::ops::Range<usize>) {
        if self.skipping > 0 {
            match event {
                Event::Start(_) => self.skipping += 1,
                Event::End(_) => self.skipping -= 1,
                _ => {}
            }
            return;
        }
        let masked_start = range.start;
        match &event {
            Event::Start(Tag::TableHead | Tag::TableRow) => {
                self.row = Some((range.end, range.start))
            }
            // A cell the parser adds to fill a short row starts at its end.
            Event::Start(Tag::TableCell) => {
                if let Some((end, last)) = &mut self.row
                    && range.start < *end
                {
                    *last = range.end;
                }
            }
            _ => {}
        }
        let span = self.masked.map.span(range);
        if let Event::Text(text) = &event
            && self.bracketed(text, span)
        {
            return;
        }
        self.unbracket();
        match event {
            Event::Start(tag) => self.start(tag, span),
            Event::End(tag) => self.end(tag),
            Event::Text(text) => self.text(&text, span),
            Event::Code(code) => {
                self.mark_literal(span);
                let code = self.restored(&code);
                self.push(Node::new(Kind::Code(code), self.place(span)));
            }
            Event::Html(html) => {
                let span = Span::new(span.start as usize, self.trimmed(span, &['\n', '\r']));
                self.html(&html, span, masked_start);
            }
            Event::InlineHtml(html) => self.html(&html, span, masked_start),
            Event::FootnoteReference(label) => {
                let label = self.restored(&label);
                let kind = Kind::FootnoteReference {
                    label,
                    number: 0,
                    repeat: 0,
                };
                self.push(Node::new(kind, self.place(span)));
            }
            Event::SoftBreak | Event::HardBreak if self.open.is_some() => {}
            Event::SoftBreak => self.push(Node::new(Kind::SoftBreak, self.place(span))),
            Event::HardBreak => self.push(Node::new(Kind::HardBreak, self.place(span))),
            Event::Rule => {
                let span = Span::new(span.start as usize, self.trimmed(span, &['\n', '\r']));
                self.push(Node::new(Kind::Rule, self.place(span)));
            }
            Event::TaskListMarker(done) => {
                self.push(Node::new(Kind::Task { done }, self.place(span)));
            }
            // Maths is the mask's, so the parser never reads any.
            Event::InlineMath(_) | Event::DisplayMath(_) => {}
        }
    }

    /// Holds back the texts that may make a footnote marker the parser
    /// left as words, `[`, `^label` and `]`, and says whether `text` was
    /// held or made a marker. A marker whose note is written in another
    /// part of the page becomes one; one with no note anywhere stays words
    /// and is told.
    fn bracketed(&mut self, text: &str, span: Span) -> bool {
        let Some(held) = &mut self.bracket else {
            let opens = text == "["
                && !self.escaped(span)
                && !self.html.literal()
                && !matches!(
                    self.stack.last().map(|node| &node.kind),
                    Some(Kind::CodeBlock { .. })
                );
            if opens {
                self.bracket = Some(vec![(text.to_owned(), span)]);
            }
            return opens;
        };
        if held.len() == 1 && !text.starts_with('^') {
            self.unbracket();
            return self.bracketed(text, span);
        }
        if !text.starts_with(']') {
            held.push((text.to_owned(), span));
            return true;
        }
        let held = self.bracket.take().expect("held");
        let label: String = held[1..].iter().map(|(text, _)| text.as_str()).collect();
        let label = label
            .strip_prefix('^')
            .filter(|label| !label.is_empty() && !label.contains(char::is_whitespace))
            .map(|label| self.restored(label));
        let close = span.start as usize + 1;
        let marker = Span::new(held[0].1.start as usize, close);
        if let Some(label) = label {
            let noted = |page: &Definitions| page.notes.contains(&label.to_lowercase());
            if !self.page.is_some_and(noted) {
                self.unset.push((label, marker));
            } else {
                let kind = Kind::FootnoteReference {
                    label,
                    number: 0,
                    repeat: 0,
                };
                self.push(Node::new(kind, self.place(marker)));
                if text.len() > 1 {
                    self.text(&text[1..], Span::new(close, span.end as usize));
                }
                return true;
            }
        }
        self.bracket = Some(held);
        self.unbracket();
        false
    }

    /// Writes the texts held back as words.
    fn unbracket(&mut self) {
        for (text, span) in self.bracket.take().unwrap_or_default() {
            self.text(&text, span);
        }
    }

    fn start(&mut self, tag: Tag<'_>, span: Span) {
        // Nothing below the deepest level is built, so no tree is deeper.
        if self.depth == DEEPEST_NESTING
            && matches!(
                tag,
                Tag::BlockQuote(_)
                    | Tag::List(_)
                    | Tag::DefinitionList
                    | Tag::FootnoteDefinition(_)
                    | Tag::Emphasis
                    | Tag::Strong
                    | Tag::Strikethrough
                    | Tag::Link { .. }
                    | Tag::Image { .. }
            )
        {
            let start = span.start as usize;
            let opening = self.text[start..].chars().next().map_or(0, char::len_utf8);
            let at = self.place(Span::new(start, start + opening));
            let problem = Problem::at(ProblemKind::NestedTooDeep, at);
            self.problems.push(problem);
            self.skipping = 1;
            return;
        }
        let kind = match tag {
            Tag::Paragraph => Kind::Paragraph { tight: false },
            Tag::Heading {
                level, id, classes, ..
            } => Kind::Heading {
                level: level as u8,
                id: id.map(|id| self.restored(&id)),
                classes: classes.iter().map(|class| self.restored(class)).collect(),
            },
            Tag::BlockQuote(None) => Kind::Quotation,
            Tag::BlockQuote(Some(kind)) => Kind::Callout(match kind {
                BlockQuoteKind::Note => CalloutKind::Note,
                BlockQuoteKind::Tip => CalloutKind::Tip,
                BlockQuoteKind::Important => CalloutKind::Important,
                BlockQuoteKind::Warning => CalloutKind::Warning,
                BlockQuoteKind::Caution => CalloutKind::Caution,
            }),
            Tag::CodeBlock(kind) => {
                self.mark_literal(span);
                let (info, fenced) = match kind {
                    CodeBlockKind::Fenced(info) => (self.restored(&info), true),
                    CodeBlockKind::Indented => (String::new(), false),
                };
                Kind::CodeBlock {
                    info,
                    text: String::new(),
                    fenced,
                }
            }
            Tag::HtmlBlock => Kind::HtmlBlock,
            Tag::List(start) => Kind::List { start },
            Tag::Item => Kind::Item,
            Tag::FootnoteDefinition(label) => {
                let label = self.restored(&label);
                self.notes.insert(label.to_lowercase());
                Kind::FootnoteDefinition { label, number: 0 }
            }
            Tag::DefinitionList => Kind::DefinitionList,
            Tag::DefinitionListTitle => Kind::Term,
            Tag::DefinitionListDefinition => Kind::Definition,
            Tag::Table(alignments) => Kind::Table(
                alignments
                    .iter()
                    .map(|a| match a {
                        Aligned::None => Alignment::None,
                        Aligned::Left => Alignment::Left,
                        Aligned::Center => Alignment::Centre,
                        Aligned::Right => Alignment::Right,
                    })
                    .collect(),
            ),
            Tag::TableHead => Kind::TableRow { head: true },
            Tag::TableRow => Kind::TableRow { head: false },
            Tag::TableCell => Kind::TableCell,
            Tag::Emphasis => Kind::Emphasis,
            Tag::Strong => Kind::Strong,
            Tag::Strikethrough => Kind::Strikethrough,
            Tag::Link {
                link_type,
                dest_url,
                title,
                ..
            } => {
                let destination = self.address(&dest_url);
                Kind::Link {
                    destination: match link_type {
                        LinkType::Email => format!("mailto:{destination}"),
                        _ => destination,
                    },
                    title: self.restored(&title),
                }
            }
            Tag::Image {
                dest_url, title, ..
            } => {
                self.image += 1;
                Kind::Image {
                    destination: self.address(&dest_url),
                    title: self.restored(&title),
                }
            }
            Tag::Superscript | Tag::Subscript | Tag::MetadataBlock(_) => {
                unreachable!("not enabled")
            }
        };
        // Each block of writing reads its raw HTML afresh, so a tag left
        // open in one never silences the next.
        if matches!(
            kind,
            Kind::Paragraph { .. }
                | Kind::Heading { .. }
                | Kind::HtmlBlock
                | Kind::TableCell
                | Kind::Term
                | Kind::Item
                | Kind::Definition
        ) {
            self.html = Html::default();
        }
        let node = Node::new(kind, self.place(span));
        if node.is_inline() {
            self.open_tight();
        } else {
            self.close_tight();
        }
        self.depth += usize::from(is_level(&node));
        self.stack.push(node);
    }

    fn end(&mut self, tag: TagEnd) {
        self.close_tight();
        if matches!(tag, TagEnd::Image) {
            self.image -= 1;
        }
        self.close();
    }

    /// Closes the innermost open node into its parent.
    fn close(&mut self) {
        let mut node = self.stack.pop().expect("an open node");
        self.depth -= usize::from(is_level(&node));
        if let (Kind::TableRow { .. }, Some(Kind::Table(columns))) =
            (&node.kind, self.stack.last().map(|table| &table.kind))
            && let Some(problem) = self.cells_past(columns.len())
        {
            self.problems.push(problem);
        }
        if !node.is_inline() && !matches!(node.kind, Kind::Paragraph { tight: true }) {
            let mut end = self.trimmed(node.place.span, &['\n', '\r', ' ', '\t']);
            // The parser can run a definition list on over the paragraph
            // after it.
            if matches!(node.kind, Kind::DefinitionList | Kind::Definition)
                && let Some(last) = node.children.last()
            {
                end = end.min(last.place.span.end as usize);
            }
            let mut start = node.place.span.start as usize;
            if matches!(node.kind, Kind::TableCell) {
                let cell = &self.text[start..end];
                start += cell.len() - cell.trim_start_matches([' ', '\t']).len();
            }
            // A loose task item's checkbox is before its paragraph.
            if let Some(first) = node.children.first() {
                start = start.min(first.place.span.start as usize);
            }
            node.place.span = Span::new(start, end);
            // The parser can end a block of HTML with the blank line after it.
            if matches!(node.kind, Kind::HtmlBlock) {
                for child in &mut node.children {
                    let span = &mut child.place.span;
                    span.end = span.end.min(end as u32);
                    span.start = span.start.min(span.end);
                }
            }
        }
        // A paragraph that is one call and nothing else is that call,
        // standing as a block: `push` would open a tight paragraph around
        // it in a list item.
        if matches!(node.kind, Kind::Paragraph { tight: false })
            && node.children.len() == 1
            && matches!(node.children[0].kind, Kind::Call(_))
        {
            let call = node.children.pop().expect("one child");
            self.add(call);
            return;
        }
        self.settle(node);
    }

    /// The cells of the row just read past the table's `columns`, which the
    /// parser drops, told where any holds something. What follows the last
    /// cell kept is read again as the row of a table with room for every
    /// cell, so the cells part where the parser parts them.
    fn cells_past(&mut self, columns: usize) -> Option<Problem> {
        let (end, last) = self.row.take()?;
        let rest = self.masked.text[last..end].trim_end_matches(['\n', '\r']);
        if rest.trim_matches([' ', '\t', '|']).is_empty() {
            return None;
        }
        let room = format!(
            "{}|\n{}|\n",
            "|x".repeat(rest.len()),
            "|-".repeat(rest.len())
        );
        let read = format!("{room}{rest}\n");
        let row_end = room.len() + rest.len();
        let mut body = false;
        let mut cells = Vec::new();
        for (event, range) in Parser::new_ext(&read, options()).into_offset_iter() {
            match event {
                Event::End(TagEnd::TableHead) => body = true,
                Event::Start(Tag::TableCell) if body && range.start < row_end => {
                    let cell = &read[range.start..range.end.min(row_end)];
                    let start = range.start + cell.len() - cell.trim_start().len();
                    let held = cell.trim();
                    if !held.is_empty() {
                        let start = last + start - room.len();
                        cells.push(self.masked.map.span(start..start + held.len()));
                    }
                }
                _ => {}
            }
        }
        let span = Span::new(cells.first()?.start as usize, cells.last()?.end as usize);
        let kind = ProblemKind::CellNeverShown {
            cells: (cells.iter())
                .map(|&cell| self.spelling(cell).into_owned())
                .collect(),
            columns,
        };
        Some(Problem::at(kind, self.place(span)))
    }

    /// Adds `node` to its parent, a paragraph holding a call that wraps
    /// writing parted around it: the call is a block of its own, though no
    /// blank line sets it apart from the words either side.
    fn settle(&mut self, mut node: Node) {
        let wrapping = |n: &Node| matches!(&n.kind, Kind::Call(call) if call.wraps());
        if !matches!(node.kind, Kind::Paragraph { .. }) || !node.children.iter().any(wrapping) {
            self.push(node);
            return;
        }
        let mut words = Vec::new();
        for child in std::mem::take(&mut node.children) {
            if !wrapping(&child) {
                words.push(child);
                continue;
            }
            self.words(&node, std::mem::take(&mut words));
            self.push(child);
        }
        self.words(&node, words);
    }

    /// Words parted from a paragraph, as a paragraph of the same kind, the
    /// line breaks at either end left out.
    fn words(&mut self, paragraph: &Node, mut words: Vec<Node>) {
        let breaks = |n: &Node| matches!(n.kind, Kind::SoftBreak | Kind::HardBreak);
        while words.last().is_some_and(breaks) {
            words.pop();
        }
        let leading = words.iter().take_while(|n| breaks(n)).count();
        words.drain(..leading);
        let (Some(first), Some(last)) = (words.first(), words.last()) else {
            return;
        };
        let span = Span::new(
            first.place.span.start as usize,
            last.place.span.end as usize,
        );
        let mut part = Node::new(paragraph.kind.clone(), self.place(span));
        part.children = words;
        self.push(part);
    }

    /// Opens a tight paragraph around words written straight into a list
    /// item or definition, so every item holds blocks.
    fn open_tight(&mut self) {
        let top = self.stack.last().expect("the root");
        if matches!(top.kind, Kind::Item | Kind::Definition) {
            let place = top.place;
            self.stack
                .push(Node::new(Kind::Paragraph { tight: true }, place));
        }
    }

    fn close_tight(&mut self) {
        let top = self.stack.last().expect("the root");
        if !matches!(top.kind, Kind::Paragraph { tight: true }) {
            return;
        }
        let mut node = self.stack.pop().expect("a tight paragraph");
        if let (Some(first), Some(last)) = (node.children.first(), node.children.last()) {
            node.place.span = Span::new(
                first.place.span.start as usize,
                last.place.span.end as usize,
            );
        }
        self.settle(node);
    }

    /// Adds `node` to the innermost open node, inside a tight paragraph
    /// where it is words written straight into a list item or definition.
    fn push(&mut self, node: Node) {
        if node.is_inline() {
            self.open_tight();
        }
        self.add(node);
    }

    /// Adds `node` to the innermost open node, joining neighbouring text so
    /// the tree reads the same however finely the parser split it.
    fn add(&mut self, node: Node) {
        let text = self.text;
        let parent = self.stack.last_mut().expect("the root");
        // A backslash escape is a gap of one character between two texts.
        if let Some(last) = parent.children.last_mut()
            && (last.place.span.end == node.place.span.start
                || last.place.span.end + 1 == node.place.span.start
                    && text.as_bytes()[last.place.span.end as usize] == b'\\')
        {
            match (&mut last.kind, &node.kind) {
                (Kind::Text(before), Kind::Text(after))
                | (Kind::Literal(before), Kind::Literal(after)) => {
                    before.push_str(after);
                    last.place.span.end = node.place.span.end;
                    return;
                }
                _ => {}
            }
        }
        parent.children.push(node);
    }

    fn text(&mut self, text: &str, span: Span) {
        if matches!(
            self.stack.last(),
            Some(Node {
                kind: Kind::CodeBlock { .. },
                ..
            })
        ) {
            let restored = self.restored(text);
            if let Some(Node {
                kind: Kind::CodeBlock { text: code, .. },
                ..
            }) = self.stack.last_mut()
            {
                code.push_str(&restored);
            }
            return;
        }
        let literal = self.html.literal();
        if literal {
            self.mark_literal(span);
        }
        let (mut text, mut span) = (text, span);
        // A character typography sets, typed as an escape or an entity,
        // reaches the page as typed.
        if !literal {
            if let Some(first) = text.chars().next().filter(|c| SETS.contains(c))
                && self.escaped(span)
            {
                let len = first.len_utf8();
                let start = span.start as usize;
                let escape = Span::new(start - 1, start + len);
                self.push(Node::new(
                    Kind::Literal(first.to_string()),
                    self.place(escape),
                ));
                if text.len() == len {
                    return;
                }
                text = &text[len..];
                span = Span::new(start + len, span.end as usize);
            } else if !text.contains(OPEN) && text.contains(SETS) && *self.spelling(span) != *text {
                self.push(Node::new(Kind::Literal(text.to_owned()), self.place(span)));
                return;
            }
        }
        if !text.contains(OPEN) {
            let kind = if literal {
                Kind::Literal(text.to_owned())
            } else {
                Kind::Text(text.to_owned())
            };
            self.push(Node::new(kind, self.place(span)));
            return;
        }
        self.split(text, span, literal);
    }

    /// Text holding placeholders, split into the words and what each
    /// placeholder stands for here.
    fn split(&mut self, text: &str, span: Span, literal: bool) {
        let words = |s: String| {
            if literal {
                Kind::Literal(s)
            } else {
                Kind::Text(s)
            }
        };
        let mut pending = String::new();
        let mut from = span.start as usize;
        let mut i = 0;
        while i < text.len() {
            let Some((&piece, len)) = self.masked.piece_at(&text[i..]) else {
                let c = text[i..].chars().next().expect("a character");
                pending.push(c);
                i += c.len_utf8();
                continue;
            };
            i += len;
            let masked: &'a Masked = self.masked;
            let token = &masked.tokens[piece.token];
            let at = piece.span.start as usize;
            let written = self.spelling(piece.span);
            let becomes_a_node = !literal && self.image == 0;
            match &token.kind {
                TokenKind::Literal => pending.push_str(&written),
                TokenKind::Call { .. } | TokenKind::Maths { .. }
                    if becomes_a_node && piece.first =>
                {
                    self.flush(&mut pending, from, at, words);
                    let node = self.token_node(piece, Within::Prose);
                    self.push(node);
                    if !piece.last {
                        self.open = Some(piece.token);
                    }
                    from = token.span.end as usize;
                }
                TokenKind::Call { .. } | TokenKind::Maths { .. }
                    if self.open == Some(piece.token) =>
                {
                    if piece.last {
                        self.open = None;
                    }
                    from = piece.span.end as usize;
                }
                TokenKind::Escape if !literal => {
                    self.flush(&mut pending, from, at, words);
                    let shown = if piece.first { &written[1..] } else { &written };
                    let node = Node::new(Kind::Literal(shown.to_owned()), self.place(piece.span));
                    self.push(node);
                    from = piece.span.end as usize;
                }
                _ => {
                    self.mark_literal(piece.span);
                    self.flush(&mut pending, from, at, words);
                    let node =
                        Node::new(Kind::Literal(written.into_owned()), self.place(piece.span));
                    self.push(node);
                    from = piece.span.end as usize;
                }
            }
        }
        self.flush(&mut pending, from, span.end as usize, words);
    }

    fn flush(
        &mut self,
        pending: &mut String,
        from: usize,
        to: usize,
        words: impl Fn(String) -> Kind,
    ) {
        if pending.is_empty() {
            return;
        }
        let span = Span::new(from, to.max(from));
        let node = Node::new(words(std::mem::take(pending)), self.place(span));
        self.push(node);
    }

    /// The node a call or maths token becomes where it is read as one.
    fn token_node(&mut self, piece: Piece, within: Within) -> Node {
        let token = &self.masked.tokens[piece.token];
        let place = self.place(token.span);
        match &token.kind {
            TokenKind::Call { call, .. } if call.name.value == IN_SITE => {
                let shown = match self.in_site(token.span, call.address()) {
                    Some(address) => address,
                    None => self.spelling(token.span).into_owned(),
                };
                Node::new(Kind::Literal(shown), place)
            }
            TokenKind::Call { call, problems, .. } => {
                self.problems.extend(problems.iter().cloned());
                let mut call = call.clone();
                call.within = within;
                let mut node = Node::new(Kind::Call(call), place);
                if let Some(body) = self.bodies[piece.token].take() {
                    self.problems.extend(body.problems);
                    if self.depth + 1 + body.levels > DEEPEST_NESTING {
                        let problem = Problem::at(ProblemKind::NestedTooDeep, place);
                        self.problems.push(problem);
                    } else {
                        node.children = body.nodes;
                    }
                }
                node
            }
            TokenKind::Maths { display, text } => Node::new(
                Kind::Maths {
                    display: *display,
                    text: text.clone(),
                    mathml: None,
                },
                place,
            ),
            TokenKind::Escape | TokenKind::Literal => {
                unreachable!("only calls and maths are nodes")
            }
        }
    }

    /// Raw HTML, a tag among the words or a line of a block, starting at
    /// `masked_start` in the masked text.
    fn html(&mut self, html: &str, span: Span, masked_start: usize) {
        // A line break between the lines of a call the parser handed on
        // apart is the call's.
        if self.open.is_some() && html.trim().is_empty() {
            return;
        }
        let (found, literal) = self.html.read(html);
        for stretch in literal {
            let stretch = self
                .masked
                .map
                .span(masked_start + stretch.start..masked_start + stretch.end);
            self.mark_literal(stretch);
        }
        if found.is_empty() {
            self.push(Node::new(Kind::Html(html.to_owned()), self.place(span)));
            return;
        }
        let mut pending = String::new();
        let mut from = span.start as usize;
        let mut last = 0;
        let mut continued = false;
        for (at, within, literal) in found {
            let (&piece, len) = self.masked.piece_at(&html[at..]).expect("a placeholder");
            pending.push_str(&html[last..at]);
            last = at + len;
            let masked: &'a Masked = self.masked;
            let token = &masked.tokens[piece.token];
            let written = self.spelling(piece.span);
            match &token.kind {
                // The address is part of the tag it is written in, which
                // stays one piece of HTML.
                TokenKind::Call { call, .. }
                    if !literal && self.image == 0 && piece.first && call.name.value == IN_SITE =>
                {
                    match (self.in_site(token.span, call.address()), within) {
                        (Some(address), Within::Attribute) => escape_href(&address, &mut pending),
                        (Some(address), _) => escape(&address, &mut pending),
                        (None, _) => pending.push_str(&self.spelling(token.span)),
                    }
                    if !piece.last {
                        self.open = Some(piece.token);
                    }
                }
                TokenKind::Call { .. } if !literal && self.image == 0 && piece.first => {
                    self.flush(&mut pending, from, piece.span.start as usize, Kind::Html);
                    let node = self.token_node(piece, within);
                    self.push(node);
                    if !piece.last {
                        self.open = Some(piece.token);
                    }
                    from = token.span.end as usize;
                }
                // The line's indentation went with the call.
                TokenKind::Call { .. } if self.open == Some(piece.token) => {
                    if pending.trim().is_empty() {
                        pending.clear();
                    }
                    if piece.last {
                        self.open = None;
                    }
                    continued = true;
                    from = piece.span.end as usize;
                }
                TokenKind::Escape if !literal => {
                    pending.push_str(if piece.first { &written[1..] } else { &written });
                    self.mark_literal(piece.span);
                }
                _ => {
                    pending.push_str(&written);
                    self.mark_literal(piece.span);
                }
            }
        }
        pending.push_str(&html[last..]);
        // A call's later lines leave no line break of their own: the call
        // keeps the one its first line ended with.
        if continued && pending.trim().is_empty() {
            pending.clear();
        }
        self.flush(&mut pending, from, span.end as usize, Kind::Html);
    }

    fn restored(&self, text: &str) -> String {
        if text.contains(OPEN) {
            restore(self.masked, self.text, text)
        } else {
            text.to_owned()
        }
    }

    /// A link's address as the parser read it, each `@url(...)` in it the
    /// address it stands for.
    fn address(&mut self, text: &str) -> String {
        if !text.contains(OPEN) {
            return text.to_owned();
        }
        let mut met = Vec::new();
        let read = address(self.masked, self.text, text, self.base, |span, stands| {
            met.push((span, stands.map(str::to_owned)));
        });
        for (span, stands) in met {
            self.in_site(span, stands.as_deref());
        }
        read
    }

    /// The address the `@url(...)` at `span` stands for, handed `address`,
    /// kept for the document; none for one handed anything else, which is
    /// told. One a definition holds is met again at each link naming it.
    fn in_site(&mut self, span: Span, address: Option<&str>) -> Option<String> {
        let place = self.place(span);
        let Some(address) = address else {
            let found = super::Malformed::NotOneAddress;
            let problem = Problem::at(ProblemKind::MalformedCall { found }, place);
            if !self.problems.contains(&problem) {
                self.problems.push(problem);
            }
            return None;
        };
        let address = format!("{}{address}", self.base);
        if !self.in_site.iter().any(|stands| stands.place == place) {
            self.in_site.push(InSite {
                address: address.clone(),
                place,
            });
        }
        Some(address)
    }

    /// The text at `span` as typed, less the control characters the mask
    /// left out.
    fn spelling(&self, span: Span) -> Cow<'a, str> {
        let typed = &self.text[span.range()];
        match self.masked.left_out.is_empty() {
            true => Cow::Borrowed(typed),
            false => controls::without(typed),
        }
    }

    /// Whether the text at `span` opens with a character a backslash
    /// escapes: one after an odd number of backslashes.
    fn escaped(&self, span: Span) -> bool {
        let before = &self.text.as_bytes()[..span.start as usize];
        before.iter().rev().take_while(|b| **b == b'\\').count() % 2 == 1
    }

    fn mark_literal(&mut self, span: Span) {
        if self.keep_literal {
            self.literal.push(span);
        }
    }

    fn place(&self, span: Span) -> Place {
        Place {
            source: self.source,
            span,
        }
    }

    /// The end of `span` with any of `trailing` taken off it.
    fn trimmed(&self, span: Span, trailing: &[char]) -> usize {
        let text = &self.text[span.range()];
        span.start as usize + text.trim_end_matches(trailing).len()
    }
}

/// `text` with every placeholder put back as the characters it replaced.
fn restore(masked: &Masked, source: &str, text: &str) -> String {
    put_back(masked, source, text, |_| None)
}

/// A link's address `text` with every placeholder put back: an `@url(...)`
/// on one line as the address it is handed, under `base`, and anything
/// else as the characters it replaced. `met` is handed where each
/// `@url(...)` is and the address it is handed, none where it is handed
/// anything else.
fn address(
    masked: &Masked,
    source: &str,
    text: &str,
    base: &str,
    mut met: impl FnMut(Span, Option<&str>),
) -> String {
    put_back(masked, source, text, |piece| {
        let token = &masked.tokens[piece.token];
        match &token.kind {
            TokenKind::Call { call, .. }
                if call.name.value == IN_SITE && piece.first && piece.last =>
            {
                met(token.span, call.address());
                call.address().map(|address| format!("{base}{address}"))
            }
            _ => None,
        }
    })
}

/// `text` with every placeholder put back: as what `stands` gives for its
/// piece, or as the characters it replaced.
fn put_back(
    masked: &Masked,
    source: &str,
    text: &str,
    mut stands: impl FnMut(&Piece) -> Option<String>,
) -> String {
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < text.len() {
        if let Some((piece, len)) = masked.piece_at(&text[i..]) {
            let typed = &source[piece.span.range()];
            match (stands(piece), masked.left_out.is_empty()) {
                (Some(stands), _) => out.push_str(&stands),
                (None, true) => out.push_str(typed),
                (None, false) => out.push_str(&controls::without(typed)),
            }
            i += len;
        } else {
            let c = text[i..].chars().next().expect("a character");
            out.push(c);
            i += c.len_utf8();
        }
    }
    out
}

/// Raw HTML read in the order written, enough to know where a call sits:
/// inside a tag, inside an element whose contents are shown as typed, or
/// in a comment.
#[derive(Default)]
struct Html {
    /// Open elements whose contents are shown as typed.
    excluded: Vec<&'static str>,
    in_comment: bool,
    /// The tag being read, and whether it closes an element.
    in_tag: Option<(Option<&'static str>, bool)>,
    quote: Option<char>,
}

/// Where a placeholder sits in raw HTML, what surrounds it, and whether it
/// is shown as typed there.
type Found = (usize, Within, bool);

const EXCLUDED: [&str; 4] = ["style", "script", "pre", "code"];

impl Html {
    fn literal(&self) -> bool {
        self.in_comment || !self.excluded.is_empty()
    }

    /// Reads on through `html`, giving where each placeholder sits, what
    /// surrounds it, and whether it is shown as typed there; and the
    /// stretches of `html` shown as typed.
    fn read(&mut self, html: &str) -> (Vec<Found>, Vec<std::ops::Range<usize>>) {
        let mut found = Vec::new();
        let mut stretches = Vec::new();
        let mut stretch = self.literal().then_some(0);
        let b = html.as_bytes();
        let mut i = 0;
        while i < html.len() {
            match (stretch, self.literal()) {
                (None, true) => stretch = Some(i),
                (Some(start), false) => {
                    stretches.push(start..i);
                    stretch = None;
                }
                _ => {}
            }
            let c = html[i..].chars().next().expect("a character");
            if c == OPEN {
                let within = if self.in_tag.is_some() {
                    Within::Attribute
                } else {
                    Within::Html
                };
                found.push((i, within, self.literal()));
            } else if self.in_comment {
                if html[i..].starts_with("-->") {
                    self.in_comment = false;
                    i += 3;
                    continue;
                }
            } else if let Some((name, closing)) = self.in_tag {
                match (self.quote, c) {
                    (Some(q), c) if c == q => self.quote = None,
                    (Some(_), _) => {}
                    (None, '"' | '\'') => self.quote = Some(c),
                    (None, '>') => {
                        if let Some(name) = name {
                            if closing {
                                if let Some(p) = self.excluded.iter().rposition(|n| *n == name) {
                                    self.excluded.truncate(p);
                                }
                            } else if b[i - 1] != b'/' {
                                self.excluded.push(name);
                            }
                        }
                        self.in_tag = None;
                    }
                    _ => {}
                }
            } else if c == '<' {
                if html[i..].starts_with("<!--") {
                    self.in_comment = true;
                    i += 4;
                    continue;
                }
                let closing = html[i + 1..].starts_with('/');
                let from = i + 1 + usize::from(closing);
                let len = html[from..]
                    .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
                    .unwrap_or(html.len() - from);
                let name = &html[from..from + len];
                if name.starts_with(|c: char| c.is_ascii_alphabetic()) {
                    let excluded = EXCLUDED
                        .iter()
                        .copied()
                        .find(|e| e.eq_ignore_ascii_case(name));
                    self.in_tag = Some((excluded, closing));
                    i = from + len;
                    continue;
                }
            }
            i += c.len_utf8();
        }
        if let Some(start) = stretch {
            stretches.push(start..html.len());
        }
        (found, stretches)
    }
}
