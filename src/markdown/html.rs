//! The tree written as HTML, laid out as a person would lay it out by hand:
//! one block to a line, indented by how deep it sits, and never a space
//! added inside a line of writing or of code.

use std::ops::Range;

use super::Document;
use super::call::Call;
use super::node::{Alignment, Kind, Node};
use super::opening::{self, Written, words_of};
use super::pieces::{self, Piece, PieceValue};
use crate::highlight::{self, Highlighter, Language};
use crate::source::Place;

/// What writes a snippet call's HTML, and the pieces of the writing the
/// site writes itself.
pub trait Snippets {
    /// `place` is the call as written, as far as its `)`. `body` is the
    /// writing a call that wraps writing wraps, as HTML.
    fn html(&self, call: &Call, place: Place, placement: Placement, body: Option<&str>) -> String;

    /// Whether the site writes `piece` itself.
    fn writes(&self, _piece: Piece) -> bool {
        false
    }

    /// Whether the site's file for `piece` can read what it is handed as
    /// `name`. Where it can't, the value need not be made.
    fn reads(&self, _piece: Piece, _name: &str) -> bool {
        true
    }

    /// `piece` as the site writes it, handed `values`, for the writing at
    /// `place`. None where the site's file couldn't write it, which is
    /// told; the piece is then written as Statac writes it.
    fn piece(
        &self,
        _piece: Piece,
        _values: Vec<(&'static str, PieceValue)>,
        _place: Place,
    ) -> Option<String> {
        None
    }

    /// The picture written as `address` at `place`, where the site has
    /// such a file: its size, and where `tagged` says Statac writes its
    /// tag itself, how the tag reaches it and a photograph's copies,
    /// which are then made.
    fn picture(&self, _address: &str, _place: Place, _tagged: bool) -> Option<Picture> {
        None
    }

    /// Says that the picture written as `address` at `place` is shown
    /// with no description; `found` is the site's file, where it is one.
    fn undescribed(&self, _address: &str, _found: Option<&Picture>, _place: Place) {}
}

/// A picture the site has, as its tag is written.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Picture {
    /// The address that reaches the file from the page.
    pub address: String,
    /// Its width and height, where its header gives them.
    pub size: Option<(u32, u32)>,
    /// A photograph's copies, where it is published as copies.
    pub copies: Option<Copies>,
    /// What the file describing it says for a reader who can't see it, or
    /// empty.
    pub alt: String,
    /// Whether the file describing it marks it as decoration.
    pub decorative: bool,
    /// The file, as a problem names it.
    pub file: Option<String>,
    /// Whether a file beside it may describe it.
    pub described_beside: bool,
}

/// The copies of a photograph, as a tag names them. Each address is
/// written as an attribute carries it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Copies {
    /// The AVIF copies, as a `srcset`.
    pub avif: String,
    /// The JPEG or PNG copies, as a `srcset`.
    pub fallback: String,
    /// The copy a browser without `srcset` shows.
    pub src: String,
    /// How wide the picture is shown, as a `sizes`.
    pub sizes: String,
}

/// Where a call stands, which decides the HTML that suits it: a picture in
/// a sentence is an image, the same picture standing alone a figure.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Placement {
    Inline,
    /// In place of a paragraph.
    Standalone,
}

pub struct Options<'a> {
    pub snippets: &'a dyn Snippets,
    /// What a screen reader says for a footnote's way back to the text, in
    /// the site's words.
    pub back_to_the_text: Option<&'a str>,
    /// The ten digits the page's language writes a footnote's number in,
    /// zero first; none where it writes 0 to 9.
    pub digits: Option<&'a [char; 10]>,
    /// Whether the page's language is written right to left, which turns
    /// the arrow of a footnote's way back.
    pub right_to_left: bool,
    /// What colours code; none leaves every block as it is.
    pub highlighter: Option<&'a Highlighter>,
}

/// Past this depth the nesting is still the page's; only the indentation
/// in front of it stops growing, so a deeply nested page costs no more
/// than its length.
const INDENT_DEPTH: usize = 32;

pub fn to_html(document: &Document, options: &Options<'_>) -> String {
    write(document, options).html
}

/// The writing as HTML, with its excerpt, its opening words and how many
/// words a reader reads in it.
pub fn write(document: &Document, options: &Options<'_>) -> Written {
    let mut writer = Writer::new(options, Vec::new());
    writer.main = true;
    let more = opening::more_marker(document);
    let mut more_at = None;
    for (at, child) in document.root.children.iter().enumerate() {
        if more == Some((at, false)) {
            more_at = Some(writer.out.len());
        }
        writer.block(child);
        if more == Some((at, true)) {
            more_at = Some(writer.out.len());
        }
    }
    if !document.notes.is_empty() {
        writer.notes(&document.notes);
    }
    let opening = writer.opening.or(writer.html_opening);
    let read = |range: Range<usize>| uncut(&writer.out, &writer.cuts, range);
    let excerpt = match more_at {
        Some(at) => Some(read(0..at).trim_end().to_owned()).filter(|kept| !kept.is_empty()),
        None => opening
            .as_ref()
            .map(|opening| format!("<p>{}</p>", read(opening.inner.clone()))),
    };
    let words = opening::count_words(&read(0..writer.out.len()));
    Written {
        excerpt,
        excerpt_marked: more.is_some(),
        opening: opening.map(|opening| opening.words),
        words,
        maths: super::uses_maths(document),
        html: writer.out,
    }
}

/// The HTML in `range` without the cuts inside it. Where a cut stood
/// between two spaces, one of them goes with it.
fn uncut(html: &str, cuts: &[Range<usize>], range: Range<usize>) -> String {
    let mut kept = String::with_capacity(range.len());
    let mut from = range.start;
    for cut in cuts
        .iter()
        .filter(|cut| range.start <= cut.start && cut.end <= range.end)
    {
        kept.push_str(&html[from..cut.start]);
        from = cut.end;
        if kept.ends_with(' ') && html[from..range.end].starts_with(' ') {
            from += 1;
        }
    }
    kept.push_str(&html[from..range.end]);
    kept
}

/// The paragraph the writing opens with: where its words stand in the
/// HTML, and the words themselves as plain text.
struct Opening {
    inner: Range<usize>,
    words: String,
}

struct Writer<'a> {
    options: &'a Options<'a>,
    out: String,
    /// What in `out`, in order, a reader doesn't read as the writing: the
    /// footnotes' markers and their ways back. Those in a call's body
    /// reach `out` only through the snippet's HTML, and are no cuts.
    cuts: Vec<Range<usize>>,
    /// Whether this writes the page's own writing, rather than a call's
    /// body or a footnote, and so finds its opening paragraph.
    main: bool,
    opening: Option<Opening>,
    /// The first `<p>` of the author's own HTML, the opening where the
    /// writing has no paragraph of Markdown.
    html_opening: Option<Opening>,
    /// One for each block open, innermost last: whether it holds a block,
    /// so its closing tag goes on a line of its own.
    frames: Vec<bool>,
    /// Where a comment among the words was left out just after a space,
    /// so the words either side aren't left two spaces apart.
    after_comment: Option<usize>,
}

impl<'a> Writer<'a> {
    fn new(options: &'a Options<'a>, frames: Vec<bool>) -> Self {
        Writer {
            options,
            out: String::new(),
            cuts: Vec::new(),
            main: false,
            opening: None,
            html_opening: None,
            frames,
            after_comment: None,
        }
    }

    fn blocks(&mut self, nodes: &[Node]) {
        for node in nodes {
            self.block(node);
        }
    }

    fn block(&mut self, node: &Node) {
        match &node.kind {
            Kind::Document => self.blocks(&node.children),
            Kind::Paragraph { tight: true } => self.line(&node.children),
            Kind::Paragraph { tight: false } => {
                if let Some(picture) = self.lone_picture(node)
                    && let Some(html) = self.image_piece(picture)
                {
                    // `<p><figure>` would close the paragraph early.
                    if opens_as_block(&html) {
                        self.new_block();
                        self.laid_out(&html);
                    } else {
                        self.open("<p>");
                        self.out.push_str(&html);
                        self.close("</p>");
                    }
                    return;
                }
                self.open("<p>");
                let start = self.out.len();
                self.line(&node.children);
                if self.main && self.opening.is_none() {
                    let words = opening::closed_up(&super::questions::seen_text(node));
                    if !words.is_empty() {
                        self.opening = Some(Opening {
                            inner: start..self.out.len(),
                            words,
                        });
                    }
                }
                self.close("</p>");
            }
            Kind::Heading { level, id, classes } => {
                if self.writes(Piece::Heading) {
                    let values = vec![
                        ("level", PieceValue::Number(u64::from(*level))),
                        ("id", PieceValue::Text(id.clone().unwrap_or_default())),
                        ("classes", PieceValue::Text(classes.join(" "))),
                        ("body", PieceValue::Html(self.inline_html(&node.children))),
                    ];
                    if self.pieced_block(Piece::Heading, values, node) {
                        return;
                    }
                }
                self.new_block();
                self.out.push_str("<h");
                self.out.push(char::from(b'0' + level));
                if let Some(id) = id {
                    self.out.push_str(" id=\"");
                    escape(id, &mut self.out);
                    self.out.push('"');
                }
                if !classes.is_empty() {
                    self.out.push_str(" class=\"");
                    escape(&classes.join(" "), &mut self.out);
                    self.out.push('"');
                }
                self.out.push('>');
                self.line(&node.children);
                self.out.push_str("</h");
                self.out.push(char::from(b'0' + level));
                self.out.push('>');
            }
            Kind::Quotation => self.wrapped("<blockquote>", node, "</blockquote>"),
            Kind::Callout(kind) => {
                if self.writes(Piece::Callout) {
                    let values = vec![
                        ("kind", PieceValue::Text(kind.word().to_owned())),
                        ("body", PieceValue::Html(self.block_html(&node.children))),
                    ];
                    if self.pieced_block(Piece::Callout, values, node) {
                        return;
                    }
                }
                let open = format!("<aside class=\"callout {}\">", kind.word());
                self.wrapped(&open, node, "</aside>");
            }
            Kind::List { start } => {
                let (open, close) = match start {
                    None => ("<ul>".to_owned(), "</ul>"),
                    Some(1) => ("<ol>".to_owned(), "</ol>"),
                    Some(n) => (format!("<ol start=\"{n}\">"), "</ol>"),
                };
                self.wrapped(&open, node, close);
            }
            Kind::Item => self.wrapped("<li>", node, "</li>"),
            Kind::DefinitionList => self.wrapped("<dl>", node, "</dl>"),
            Kind::Term => {
                self.open("<dt>");
                self.line(&node.children);
                self.close("</dt>");
            }
            Kind::Definition => self.wrapped("<dd>", node, "</dd>"),
            Kind::CodeBlock { info, text, .. } => {
                // The last line's ending only: a blank line before the
                // fence is the code's.
                let code = text.strip_suffix('\n').unwrap_or(text);
                let code = code.strip_suffix('\r').unwrap_or(code);
                let (language, caption) = pieces::fence(info);
                let mut body = None;
                if self.writes(Piece::Code) {
                    // Colouring is most of the cost of a block.
                    if self.options.snippets.reads(Piece::Code, "body") {
                        body = Some(self.code(info, code));
                    }
                    let values = vec![
                        ("code", PieceValue::Text(code.to_owned())),
                        ("body", PieceValue::Html(body.clone().unwrap_or_default())),
                        ("language", PieceValue::Text(language.to_owned())),
                        ("caption", PieceValue::Text(caption.to_owned())),
                    ];
                    if self.pieced_block(Piece::Code, values, node) {
                        return;
                    }
                }
                let body = body.unwrap_or_else(|| self.code(info, code));
                self.new_block();
                // Words with no letter or digit caption nothing, and the
                // line's first word is then the block's class, quoted or
                // not.
                let caption =
                    Some(caption).filter(|caption| caption.chars().any(char::is_alphanumeric));
                let class = match caption {
                    None => info.split_whitespace().next(),
                    Some(_) => Some(language).filter(|language| !language.is_empty()),
                };
                match caption {
                    None => self.out.push_str("<pre class=\"code\"><code"),
                    Some(caption) => {
                        self.out.push_str("<figure class=\"code\"><figcaption>");
                        escape(caption, &mut self.out);
                        self.out.push_str("</figcaption><pre><code");
                    }
                }
                if let Some(class) = class {
                    self.out.push_str(" class=\"language-");
                    escape(class, &mut self.out);
                    self.out.push('"');
                }
                self.out.push('>');
                self.out.push_str(&body);
                self.out.push_str("</code></pre>");
                if caption.is_some() {
                    self.out.push_str("</figure>");
                }
            }
            Kind::HtmlBlock => {
                let start = self.out.len();
                self.html_block(&node.children);
                if self.main && self.opening.is_none() && self.html_opening.is_none() {
                    self.html_opening =
                        opening::first_html_paragraph(&self.out[start..]).map(|inner| {
                            let inner = start + inner.start..start + inner.end;
                            let words = opening::closed_up(&words_of(&self.out[inner.clone()]));
                            Opening { inner, words }
                        });
                }
            }
            Kind::Rule => {
                self.new_block();
                self.out.push_str("<hr>");
            }
            Kind::Table {
                alignments,
                id,
                classes,
            } => {
                if self.writes(Piece::Table) {
                    let cells = |row: &Node| {
                        let cells = (row.children.iter().enumerate())
                            .map(|(column, cell)| {
                                let align = aligned(alignments.get(column));
                                PieceValue::Record(vec![
                                    ("body", PieceValue::Html(self.inline_html(&cell.children))),
                                    ("align", PieceValue::Text(align.to_owned())),
                                ])
                            })
                            .collect();
                        PieceValue::List(cells)
                    };
                    let mut head = PieceValue::List(Vec::new());
                    let mut rows = Vec::new();
                    for row in &node.children {
                        match row.kind {
                            Kind::TableRow { head: true } => head = cells(row),
                            _ => rows.push(cells(row)),
                        }
                    }
                    let values = vec![
                        ("head", head),
                        ("rows", PieceValue::List(rows)),
                        ("id", PieceValue::Text(id.clone().unwrap_or_default())),
                        ("classes", PieceValue::Text(classes.join(" "))),
                    ];
                    if self.pieced_block(Piece::Table, values, node) {
                        return;
                    }
                }
                self.table(node, alignments, id.as_deref(), classes);
            }
            Kind::Call(call) => {
                let html = self.call(call, node, Placement::Standalone);
                if !html.trim().is_empty() {
                    self.new_block();
                    self.laid_out(&html);
                }
            }
            // Gathered at the foot of the page.
            Kind::FootnoteDefinition { .. } => {}
            Kind::TableRow { .. } | Kind::TableCell => self.blocks(&node.children),
            _ => self.inline(node),
        }
    }

    fn wrapped(&mut self, open: &str, node: &Node, close: &str) {
        self.open(open);
        self.blocks(&node.children);
        self.close(close);
    }

    fn table(
        &mut self,
        node: &Node,
        alignments: &[Alignment],
        id: Option<&str>,
        classes: &[String],
    ) {
        self.open("<div class=\"scroll\">");
        let mut open = String::from("<table");
        if let Some(id) = id {
            open.push_str(" id=\"");
            escape(id, &mut open);
            open.push('"');
        }
        if !classes.is_empty() {
            open.push_str(" class=\"");
            escape(&classes.join(" "), &mut open);
            open.push('"');
        }
        open.push('>');
        self.open(&open);
        for row in &node.children {
            let head = matches!(row.kind, Kind::TableRow { head: true });
            if head {
                self.open("<thead>");
            }
            self.open("<tr>");
            for (column, cell) in row.children.iter().enumerate() {
                let tag = if head { "th" } else { "td" };
                let open = match aligned(alignments.get(column)) {
                    "" => format!("<{tag}>"),
                    align => format!("<{tag} style=\"text-align: {align}\">"),
                };
                self.open(&open);
                self.line(&cell.children);
                self.close(&format!("</{tag}>"));
            }
            self.close("</tr>");
            if head {
                self.close("</thead>");
            }
        }
        self.close("</table>");
        self.close("</div>");
    }

    /// A block of the author's own HTML, a line at a time, re-indented to
    /// the depth it sits at except inside a `<pre>`, and without its
    /// comments.
    fn html_block(&mut self, children: &[Node]) {
        let mut html = String::new();
        for child in children {
            match &child.kind {
                Kind::Html(text) => html.push_str(text),
                Kind::Call(call) => {
                    let written = self.call(call, child, Placement::Inline);
                    html.push_str(&written);
                }
                _ => {}
            }
        }
        let mut written = false;
        let mut preformatted = false;
        for (line, joins) in without_comments(&html) {
            if joins && written {
                self.out.push_str(&line);
            } else if written {
                self.out.push('\n');
                if !preformatted {
                    self.indent();
                }
                self.out.push_str(&line);
            } else {
                self.new_block();
                self.out.push_str(&line);
                written = true;
            }
            preformatted = preformatted_after(preformatted, &line);
        }
    }

    /// HTML a snippet wrote, standing as a block at the depth it sits at.
    fn laid_out(&mut self, html: &str) {
        let mut preformatted = false;
        for (n, line) in html.trim().lines().enumerate() {
            if n > 0 {
                self.out.push('\n');
                if !preformatted {
                    self.indent();
                }
            }
            self.out.push_str(line);
            preformatted = preformatted_after(preformatted, line);
        }
    }

    /// A footnote's number as a reader sees it, in the language's digits.
    fn shown(&self, number: u32) -> String {
        crate::dates::format::in_digits(&number.to_string(), self.options.digits)
    }

    /// The footnotes, at the foot of the page, each with its way back.
    fn notes(&mut self, notes: &[Node]) {
        if self.writes(Piece::Footnotes) {
            let listed = notes
                .iter()
                .filter_map(|note| match note.kind {
                    Kind::FootnoteDefinition { number, .. } => Some(PieceValue::Record(vec![
                        ("number", PieceValue::Number(u64::from(number))),
                        ("shown", PieceValue::Text(self.shown(number))),
                        ("id", PieceValue::Text(format!("note-{number}"))),
                        ("back", PieceValue::Text(format!("back-{number}"))),
                        (
                            "body",
                            PieceValue::Html(self.block_html(&note.children).trim().to_owned()),
                        ),
                    ])),
                    _ => None,
                })
                .collect();
            let label = self.options.back_to_the_text.unwrap_or_default().to_owned();
            let values = vec![
                ("notes", PieceValue::List(listed)),
                ("label", PieceValue::Text(label)),
            ];
            if let Some(first) = notes.first()
                && self.pieced_block(Piece::Footnotes, values, first)
            {
                return;
            }
        }
        self.open("<section class=\"footnotes\">");
        self.open("<ol>");
        for note in notes {
            let Kind::FootnoteDefinition { number, .. } = note.kind else {
                continue;
            };
            self.open(&format!("<li id=\"note-{number}\">"));
            let mut inside = Writer::new(self.options, vec![false; 3]);
            inside.blocks(&note.children);
            let written = inside.out.trim();
            let mut back = format!(" <a href=\"#back-{number}\" class=\"back\"");
            if let Some(label) = self.options.back_to_the_text {
                back.push_str(" aria-label=\"");
                escape(label, &mut back);
                back.push('"');
            }
            back.push('>');
            back.push(match self.options.right_to_left {
                true => '\u{21aa}',
                false => '\u{21a9}',
            });
            back.push_str("</a>");
            self.new_block();
            let shift = self.out.len() - (inside.out.len() - inside.out.trim_start().len());
            self.cuts.extend(
                inside
                    .cuts
                    .iter()
                    .map(|cut| cut.start + shift..cut.end + shift),
            );
            let body = written.strip_suffix("</p>");
            self.out.push_str(body.unwrap_or(written));
            self.cuts.push(self.out.len()..self.out.len() + back.len());
            self.out.push_str(&back);
            if body.is_some() {
                self.out.push_str("</p>");
            }
            self.close("</li>");
        }
        self.close("</ol>");
        self.close("</section>");
    }

    /// The words of a block that ends a line.
    fn line(&mut self, nodes: &[Node]) {
        self.inlines(nodes);
        if self.after_comment.take() == Some(self.out.len()) {
            self.out.pop();
        }
    }

    fn inlines(&mut self, nodes: &[Node]) {
        for node in nodes {
            self.inline(node);
        }
    }

    fn inline(&mut self, node: &Node) {
        if self.after_comment.take() == Some(self.out.len()) {
            match &node.kind {
                Kind::Text(text) | Kind::Literal(text) if text.starts_with(' ') => {
                    escape(&text[1..], &mut self.out);
                    return;
                }
                Kind::SoftBreak | Kind::HardBreak => {
                    self.out.pop();
                }
                _ => {}
            }
        }
        match &node.kind {
            Kind::Text(text) | Kind::Literal(text) => escape(text, &mut self.out),
            Kind::Code(code) => {
                if self.writes(Piece::InlineCode)
                    && let Some(html) = self.piece(
                        Piece::InlineCode,
                        vec![("code", PieceValue::Text(code.clone()))],
                        node,
                    )
                {
                    self.out.push_str(&html);
                    return;
                }
                self.out.push_str("<code>");
                escape(code, &mut self.out);
                self.out.push_str("</code>");
            }
            Kind::Html(html) if is_comment(html) => {
                if self.out.ends_with(' ') {
                    self.after_comment = Some(self.out.len());
                }
            }
            Kind::Html(html) => self.out.push_str(html),
            Kind::Maths {
                mathml: Some(mathml),
                ..
            } => self.out.push_str(mathml),
            Kind::Maths { display, text, .. } => {
                self.out.push_str(if *display {
                    "<span class=\"math display\">"
                } else {
                    "<span class=\"math\">"
                });
                escape(text, &mut self.out);
                self.out.push_str("</span>");
            }
            Kind::SoftBreak => self.out.push('\n'),
            Kind::HardBreak => self.out.push_str("<br>\n"),
            Kind::Emphasis => self.tagged("<em>", node, "</em>"),
            Kind::Strong => self.tagged("<strong>", node, "</strong>"),
            Kind::Strikethrough => self.tagged("<del>", node, "</del>"),
            Kind::Link { destination, title } => {
                if self.writes(Piece::Link) {
                    let values = vec![
                        ("address", PieceValue::Text(destination.clone())),
                        ("title", PieceValue::Text(title.clone())),
                        ("body", PieceValue::Html(self.inline_html(&node.children))),
                    ];
                    if let Some(html) = self.piece(Piece::Link, values, node) {
                        self.out.push_str(&html);
                        return;
                    }
                }
                self.out.push_str("<a href=\"");
                escape_href(destination, &mut self.out);
                self.out.push('"');
                if !title.is_empty() {
                    self.out.push_str(" title=\"");
                    escape(title, &mut self.out);
                    self.out.push('"');
                }
                self.out.push('>');
                self.inlines(&node.children);
                self.out.push_str("</a>");
            }
            Kind::Image { destination, title } => {
                if let Some(html) = self.image_piece(node) {
                    self.out.push_str(&html);
                    return;
                }
                let shown = self.options.snippets.picture(destination, node.place, true);
                let description = self.description(node, destination, shown.as_ref());
                let size = shown.as_ref().and_then(|shown| shown.size);
                let copies = shown.as_ref().and_then(|shown| shown.copies.as_ref());
                if let Some(copies) = copies {
                    self.out
                        .push_str("<picture><source type=\"image/avif\" srcset=\"");
                    self.out.push_str(&copies.avif);
                    self.out.push_str("\" sizes=\"");
                    escape(&copies.sizes, &mut self.out);
                    self.out.push_str("\"><img src=\"");
                    self.out.push_str(&copies.src);
                    self.out.push_str("\" srcset=\"");
                    self.out.push_str(&copies.fallback);
                    self.out.push_str("\" sizes=\"");
                    escape(&copies.sizes, &mut self.out);
                    self.out.push('"');
                    if let Some((width, height)) = size {
                        self.out
                            .push_str(&format!(" width=\"{width}\" height=\"{height}\""));
                    }
                } else {
                    self.out.push_str("<img src=\"");
                    let address = shown.as_ref().map_or(destination, |shown| &shown.address);
                    escape_href(address, &mut self.out);
                    self.out.push('"');
                }
                self.out.push_str(" alt=\"");
                escape(&description, &mut self.out);
                self.out.push('"');
                if !title.is_empty() {
                    self.out.push_str(" title=\"");
                    escape(title, &mut self.out);
                    self.out.push('"');
                }
                if let (None, Some((width, height))) = (copies, size) {
                    self.out
                        .push_str(&format!(" width=\"{width}\" height=\"{height}\""));
                }
                self.out.push_str(" loading=\"lazy\" decoding=\"async\">");
                if copies.is_some() {
                    self.out.push_str("</picture>");
                }
            }
            Kind::FootnoteReference { number, repeat, .. } => {
                let back = match repeat {
                    0 | 1 => format!("back-{number}"),
                    n => format!("back-{number}-{n}"),
                };
                let start = self.out.len();
                if self.writes(Piece::Footnote) {
                    let values = vec![
                        ("number", PieceValue::Number(u64::from(*number))),
                        ("shown", PieceValue::Text(self.shown(*number))),
                        ("id", PieceValue::Text(back.clone())),
                        ("note", PieceValue::Text(format!("note-{number}"))),
                    ];
                    if let Some(html) = self.piece(Piece::Footnote, values, node) {
                        self.out.push_str(&html);
                        self.cuts.push(start..self.out.len());
                        return;
                    }
                }
                let shown = self.shown(*number);
                self.out.push_str(&format!(
                    "<sup class=\"footnote\"><a href=\"#note-{number}\" id=\"{back}\">{shown}</a></sup>"
                ));
                self.cuts.push(start..self.out.len());
            }
            Kind::Task { done } => self.out.push_str(if *done {
                "<input type=\"checkbox\" disabled checked> "
            } else {
                "<input type=\"checkbox\" disabled> "
            }),
            Kind::Call(call) => {
                let html = self.call(call, node, Placement::Inline);
                self.out.push_str(&html);
            }
            _ => self.block(node),
        }
    }

    /// A call's HTML, handed the writing it wraps where it wraps any.
    fn call(&self, call: &Call, node: &Node, placement: Placement) -> String {
        let body = call.wraps().then(|| {
            let mut inside = Writer::new(self.options, Vec::new());
            inside.blocks(&node.children);
            inside.out
        });
        let place = Place {
            span: call.head.unwrap_or(node.place.span),
            ..node.place
        };
        self.options
            .snippets
            .html(call, place, placement, body.as_deref())
    }

    /// A block's code as HTML: coloured where its fence names a language
    /// Statac colours and no line of it is too long, as it is otherwise.
    fn code(&self, info: &str, code: &str) -> String {
        let language =
            pieces::language_word(info).map(|word| highlight::first_word(word).language());
        match (self.options.highlighter, language) {
            (Some(highlighter), Some(Language::Known(name)))
                if highlight::too_long(code).is_none() =>
            {
                highlighter.html(name, code).as_ref().to_owned()
            }
            _ => {
                let mut body = String::new();
                escape(code, &mut body);
                body
            }
        }
    }

    fn writes(&self, piece: Piece) -> bool {
        self.options.snippets.writes(piece)
    }

    fn piece(
        &self,
        piece: Piece,
        values: Vec<(&'static str, PieceValue)>,
        node: &Node,
    ) -> Option<String> {
        self.options.snippets.piece(piece, values, node.place)
    }

    /// The picture a paragraph holds and nothing else, whitespace aside,
    /// where the site writes pictures itself. One inside a link's words is
    /// not alone.
    fn lone_picture<'n>(&self, paragraph: &'n Node) -> Option<&'n Node> {
        let mut held = paragraph.children.iter().filter(|child| match &child.kind {
            Kind::Text(text) => !text.trim().is_empty(),
            Kind::SoftBreak => false,
            _ => true,
        });
        let picture = held.next()?;
        let alone = matches!(picture.kind, Kind::Image { .. }) && held.next().is_none();
        (alone && self.writes(Piece::Image)).then_some(picture)
    }

    /// What a picture's tag says for a reader who can't see it: the words
    /// in its brackets, or with none there what the file describing it
    /// says. A picture with neither, and not marked as decoration, is told.
    fn description(&self, node: &Node, destination: &str, shown: Option<&Picture>) -> String {
        let mut description = String::new();
        described(node, &mut description);
        if !description.trim().is_empty() {
            return description;
        }
        match shown {
            Some(shown) if !shown.alt.trim().is_empty() => return shown.alt.clone(),
            Some(shown) if shown.decorative => {}
            _ => self
                .options
                .snippets
                .undescribed(destination, shown, node.place),
        }
        description
    }

    /// A picture as the site's file writes it, where the site has one that
    /// could.
    fn image_piece(&self, node: &Node) -> Option<String> {
        let Kind::Image { destination, title } = &node.kind else {
            return None;
        };
        if !self.writes(Piece::Image) {
            return None;
        }
        let shown = self
            .options
            .snippets
            .picture(destination, node.place, false);
        let description = self.description(node, destination, shown.as_ref());
        let (width, height) = match shown.as_ref().and_then(|shown| shown.size) {
            Some((width, height)) => (
                PieceValue::Number(u64::from(width)),
                PieceValue::Number(u64::from(height)),
            ),
            None => (
                PieceValue::Text(String::new()),
                PieceValue::Text(String::new()),
            ),
        };
        let values = vec![
            ("address", PieceValue::Text(destination.clone())),
            ("alt", PieceValue::Text(description)),
            ("title", PieceValue::Text(title.clone())),
            ("width", width),
            ("height", height),
        ];
        self.piece(Piece::Image, values, node)
    }

    /// A piece the site writes itself standing as a block, where its file
    /// could write it. One writing only whitespace leaves nothing behind.
    fn pieced_block(
        &mut self,
        piece: Piece,
        values: Vec<(&'static str, PieceValue)>,
        node: &Node,
    ) -> bool {
        let Some(html) = self.piece(piece, values, node) else {
            return false;
        };
        if !html.trim().is_empty() {
            self.new_block();
            self.laid_out(&html);
        }
        true
    }

    /// Words as HTML, for a piece to hold.
    fn inline_html(&self, nodes: &[Node]) -> String {
        let mut inside = Writer::new(self.options, Vec::new());
        inside.line(nodes);
        inside.out
    }

    /// Blocks as HTML, laid out from no depth, for a piece to hold.
    fn block_html(&self, nodes: &[Node]) -> String {
        let mut inside = Writer::new(self.options, Vec::new());
        inside.blocks(nodes);
        inside.out
    }

    fn tagged(&mut self, open: &str, node: &Node, close: &str) {
        self.out.push_str(open);
        self.inlines(&node.children);
        self.out.push_str(close);
    }

    fn open(&mut self, tag: &str) {
        self.new_block();
        self.out.push_str(tag);
        self.frames.push(false);
    }

    fn close(&mut self, tag: &str) {
        if self.frames.pop().unwrap_or(false) {
            self.newline_and_indent();
        }
        self.out.push_str(tag);
    }

    /// Starts a block on a line of its own, and tells the block it sits in
    /// that it holds one.
    fn new_block(&mut self) {
        self.newline_and_indent();
        if let Some(holds_a_block) = self.frames.last_mut() {
            *holds_a_block = true;
        }
    }

    fn newline_and_indent(&mut self) {
        if self.out.is_empty() {
            return;
        }
        if !self.out.ends_with('\n') {
            self.out.push('\n');
        }
        self.indent();
    }

    fn indent(&mut self) {
        for _ in 0..self.frames.len().min(INDENT_DEPTH) {
            self.out.push_str("  ");
        }
    }
}

/// A picture's description as words alone: markup in it reaches neither the
/// description nor the page.
fn described(node: &Node, out: &mut String) {
    for child in &node.children {
        match &child.kind {
            Kind::Text(text) | Kind::Literal(text) | Kind::Code(text) => out.push_str(text),
            Kind::SoftBreak | Kind::HardBreak => out.push(' '),
            Kind::Html(_) => {}
            _ => described(child, out),
        }
    }
}

/// The elements HTML calls phrasing content, which may stand inside a
/// paragraph.
const PHRASING: &[&str] = &[
    "a",
    "abbr",
    "area",
    "audio",
    "b",
    "bdi",
    "bdo",
    "br",
    "button",
    "canvas",
    "cite",
    "code",
    "data",
    "datalist",
    "del",
    "dfn",
    "em",
    "embed",
    "i",
    "iframe",
    "img",
    "input",
    "ins",
    "kbd",
    "label",
    "link",
    "map",
    "mark",
    "math",
    "meta",
    "meter",
    "noscript",
    "object",
    "output",
    "picture",
    "progress",
    "q",
    "ruby",
    "s",
    "samp",
    "script",
    "select",
    "selectedcontent",
    "slot",
    "small",
    "span",
    "strong",
    "sub",
    "sup",
    "svg",
    "template",
    "textarea",
    "time",
    "u",
    "var",
    "video",
    "wbr",
];

/// The CSS keyword for a column's alignment, or empty where it has none.
fn aligned(alignment: Option<&Alignment>) -> &'static str {
    match alignment {
        Some(Alignment::Left) => "left",
        Some(Alignment::Centre) => "center",
        Some(Alignment::Right) => "right",
        Some(Alignment::None) | None => "",
    }
}

/// Whether HTML opens with an element that can't stand inside a paragraph,
/// such as `<figure>`. A custom element, its name holding a hyphen, can.
fn opens_as_block(html: &str) -> bool {
    let Some(rest) = html.trim_start().strip_prefix('<') else {
        return false;
    };
    let name: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect::<String>()
        .to_ascii_lowercase();
    rest.starts_with(|c: char| c.is_ascii_alphabetic())
        && !name.contains('-')
        && !PHRASING.contains(&name.as_str())
}

fn is_comment(html: &str) -> bool {
    html.len() >= "<!-->".len() && html.starts_with("<!--") && html.ends_with("-->")
}

/// Whether a `<pre>` is still open once `line` has been written.
fn preformatted_after(open: bool, line: &str) -> bool {
    let mut open = open;
    let mut rest = line;
    loop {
        let opens = rest.find("<pre");
        let closes = rest.find("</pre");
        let (at, opening) = match (opens, closes) {
            (Some(o), Some(c)) if o < c => (o, true),
            (_, Some(c)) => (c, false),
            (Some(o), None) => (o, true),
            (None, None) => return open,
        };
        open = opening;
        rest = &rest[at + if opening { 4 } else { 5 }..];
    }
}

/// The lines of a block of HTML without its comments, each with whether it
/// carries on the line before because a comment took the break between
/// them. A line that held only comments is gone, unless it is inside a
/// `<pre>`, where its break is part of what is shown.
fn without_comments(html: &str) -> Vec<(String, bool)> {
    let comments = comments_in(html);
    let mut lines = Vec::new();
    let mut preformatted = false;
    let mut open_written = false;
    let mut next = 0;
    let mut start = 0;
    for line in html.split_inclusive('\n') {
        let end = start + line.len();
        while comments.get(next).is_some_and(|c| c.end <= start) {
            next += 1;
        }
        let touching = &comments[next..next + comments[next..].partition_point(|c| c.start < end)];
        let (Some(first), Some(last)) = (touching.first(), touching.last()) else {
            let kept = line.trim_end_matches('\n');
            preformatted = preformatted_after(preformatted, kept);
            lines.push((kept.to_owned(), false));
            start = end;
            continue;
        };
        let mut kept = String::new();
        let mut from = start;
        for comment in touching {
            kept.push_str(&html[from..comment.start.max(from)]);
            from = comment.end.min(end);
        }
        kept.push_str(&html[from..end]);
        let kept = kept.trim_end_matches('\n');
        let gone = !preformatted && kept.trim().is_empty();
        if !gone {
            lines.push((kept.to_owned(), first.start < start && open_written));
        }
        if last.end > end && last.start >= start {
            open_written = !gone;
        }
        preformatted = preformatted_after(preformatted, kept);
        start = end;
    }
    lines
}

/// Every comment in a stretch of HTML, `<!--` to the first `-->`, as a
/// browser reads one: not inside a tag, nor inside an element whose
/// contents are raw text. One that never closes is left as written.
pub(super) fn comments_in(html: &str) -> Vec<std::ops::Range<usize>> {
    const RAW: [&str; 4] = ["script", "style", "textarea", "title"];
    let lowered = html.to_ascii_lowercase();
    let b = lowered.as_bytes();
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(at) = lowered[from..].find('<') {
        let open = from + at;
        let rest = &lowered[open + 1..];
        if rest.starts_with("!--") {
            // `<!-->` and `<!--->` close as they open.
            let Some(close) = lowered[open + 2..].find("-->") else {
                break;
            };
            let end = open + 2 + close + "-->".len();
            found.push(open..end);
            from = end;
            continue;
        }
        if !rest.starts_with(|c: char| c.is_ascii_alphabetic()) {
            from = open + 1;
            continue;
        }
        let Some(closed) = tag_end(b, open + 1) else {
            break;
        };
        from = closed;
        let name = &rest[..rest
            .find(|c: char| !c.is_ascii_alphanumeric())
            .unwrap_or(rest.len())];
        if RAW.contains(&name) {
            match lowered[from..].find(&format!("</{name}")) {
                Some(at) => from += at + "</".len(),
                None => break,
            }
        }
    }
    found
}

/// Just past the `>` closing the tag whose name starts at `at`, quoted
/// attribute values stepped over whole.
fn tag_end(html: &[u8], mut at: usize) -> Option<usize> {
    while let Some(&byte) = html.get(at) {
        at += 1;
        match byte {
            b'>' => return Some(at),
            b'=' => {
                while html.get(at).is_some_and(u8::is_ascii_whitespace) {
                    at += 1;
                }
                if let Some(&quote @ (b'"' | b'\'')) = html.get(at) {
                    at += 1 + html[at + 1..].iter().position(|b| *b == quote)? + 1;
                }
            }
            _ => {}
        }
    }
    None
}

/// The four characters that would otherwise be read as HTML.
pub(super) fn escape(text: &str, out: &mut String) {
    let mut rest = text;
    while let Some(at) = rest.find(['&', '<', '>', '"']) {
        out.push_str(&rest[..at]);
        out.push_str(match rest.as_bytes()[at] {
            b'&' => "&amp;",
            b'<' => "&lt;",
            b'>' => "&gt;",
            _ => "&quot;",
        });
        rest = &rest[at + 1..];
    }
    out.push_str(rest);
}

/// An address as an attribute carries it: what a URL may not hold
/// percent-encoded, and `&` and `'` as references.
pub(crate) fn escape_href(address: &str, out: &mut String) {
    for byte in address.bytes() {
        let safe = byte.is_ascii_alphanumeric()
            || matches!(
                byte,
                b'!' | b'#'
                    | b'$'
                    | b'%'
                    | b'('
                    | b')'
                    | b'*'
                    | b'+'
                    | b','
                    | b'-'
                    | b'.'
                    | b'/'
                    | b':'
                    | b';'
                    | b'='
                    | b'?'
                    | b'@'
                    | b'^'
                    | b'_'
                    | b'~'
            );
        match byte {
            _ if safe => out.push(char::from(byte)),
            b'&' => out.push_str("&amp;"),
            b'\'' => out.push_str("&#x27;"),
            _ => {
                const HEX: &[u8; 16] = b"0123456789ABCDEF";
                out.push('%');
                out.push(char::from(HEX[usize::from(byte >> 4)]));
                out.push(char::from(HEX[usize::from(byte & 0xf)]));
            }
        }
    }
}
