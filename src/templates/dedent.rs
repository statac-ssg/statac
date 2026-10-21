//! Indentation a block's body has only for the template's sake, which the
//! built page leaves out.
//!
//! An author indents a loop's body under its tag to read the template, not
//! to indent the page. The excess is the template's when the tag sits where
//! the content would: level with the line before it, or one step inside an
//! element that line leaves open. Where the body sits one step inside that
//! element, or the tags sit further left than content already at its depth,
//! the indent is the markup's and is kept. So is anything Statac can't
//! read with confidence: tags sharing a line, or saying what becomes of
//! the spaces round them; branches starting at different depths; a body
//! whose markup closes what it didn't open or leaves open what it opened;
//! a tag inside `<pre>`, `<textarea>`, `<script>` or `<style>`. Lines
//! inside `<pre>` and `<textarea>` never move.
//!
//! A level is the file's commonest step into an element between lines with
//! no tag line between them.
//!
//! The same reading gives what `landing` needs to place each line, and
//! what the page leaves out of the template's text. The engine writes every
//! space; a tag opening or closing a line takes the line's indent or its
//! line ending with it, as Jinja's `lstrip_blocks` and `trim_blocks` would,
//! except where the line says otherwise: a line holding more than tags
//! keeps both, the tag closing a branch, alone on its line, keeps its line
//! ending to stand in for one the branch takes with it, and a `call` or
//! `filter` alone on an indented line of markup keeps its indent, for what
//! it writes to line up at. A loop's tag alone on its line keeps its line
//! ending, for each pass to start a line, where the loop's end tag closes
//! the last line of a body opening indented. A body the engine holds back,
//! such as a macro's, is read as any other.

use std::collections::HashMap;
use std::ops::Range;

use minijinja::machinery::{Token, tokenize};

use crate::html::PREFORMATTED;

/// Blocks whose body is written where the block stands, as a `block`'s
/// is where its template is the one rendered.
const FLOWING: [&str; 5] = ["for", "if", "with", "autoescape", "block"];

/// Blocks whose body is written somewhere else, or held back.
const PLACED: [&str; 4] = ["macro", "call", "filter", "set"];

/// Elements whose text is not markup.
const SCRIPTED: [&str; 2] = ["script", "style"];

/// Elements with no end tag.
const VOID: [&str; 16] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track",
    "wbr", "param", "keygen", "command",
];

/// Where `source` has lines that give up indentation they have only for
/// the template's sake: each such line's start, with how many bytes of its
/// indent are left out. Nothing for a template the engine can't read.
#[cfg(test)]
fn cuts(source: &str) -> Vec<(usize, usize)> {
    let Some(read) = Read::of(source) else {
        return Vec::new();
    };
    let mut template = Template::new(source, read);
    template.decide();
    template.cuts()
}

/// What placing a template's lines in the built page needs to know of
/// them, `markup` saying whether it writes markup. Nothing for a template
/// the engine can't read.
pub(super) fn facts(source: &str, markup: bool) -> Option<Facts> {
    let mut template = Template::new(source, Read::of(source)?);
    template.decide();
    Some(template.facts(markup))
}

/// What the page leaves out of a template whose lines are written as
/// typed: the spaces its tags take with them, in order. Nothing for a
/// template the engine can't read.
pub(super) fn dropped(source: &str) -> Vec<Range<usize>> {
    Read::of(source).map(|read| read.gaps).unwrap_or_default()
}

pub(super) struct Facts {
    pub(super) lines: Vec<Fact>,
    /// The `block`s, in the order they open.
    pub(super) blocks: Vec<Named>,
    /// The lines holding `{{ super() }}` and nothing else.
    pub(super) supers: Vec<usize>,
    pub(super) calls: Vec<Recursion>,
    /// The template's text that the engine writes and the page leaves out,
    /// in order: the spaces round its tags.
    pub(super) dropped: Vec<Range<usize>>,
}

/// What is written of the spaces before a line's first tag or text.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Head {
    /// As typed, or as `cut` and where the line lands say.
    Typed,
    /// Nothing: a tag opening the line takes them with it.
    Dropped,
    /// As typed where the line starts a line of the page, and nothing
    /// where it doesn't: a tag opens a line holding more, or a `call` or
    /// `filter` whose body lines up at the line.
    AtLineStart,
}

/// What is written of the line ending after a line's last tag or text.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Tail {
    Typed,
    /// Nothing: a tag closing the line takes it with it.
    Dropped,
    /// The line ending, where something has been written for it to end.
    Owed,
}

pub(super) struct Fact {
    pub(super) start: usize,
    /// The bytes of its indent.
    pub(super) indent: usize,
    /// How many of them it is written without.
    pub(super) cut: usize,
    /// It holds more than spaces.
    pub(super) holds: bool,
    /// It writes markup, not inside `<pre>` or `<textarea>`.
    pub(super) content: bool,
    /// Its indent may be written otherwise: it isn't inside a tag, `raw`,
    /// `<pre>` or `<textarea>`.
    pub(super) movable: bool,
    /// It holds tags and nothing else.
    pub(super) bare: bool,
    /// Its indent is followed by a value.
    pub(super) valued: bool,
    /// That value is a body the engine holds back, written whole: a call
    /// of a macro or the name of a `set` block, of this template's own, or
    /// a call of `caller`. Or the line holds a `call` or `filter` writing
    /// one, its indent kept.
    pub(super) held: bool,
    pub(super) head: Head,
    pub(super) tail: Tail,
    /// Where its text ends and its line ending starts.
    pub(super) end: usize,
}

pub(super) struct Named {
    pub(super) name: String,
    pub(super) opening: usize,
    pub(super) end: usize,
    pub(super) site: Site,
}

/// Where a `block` stands, which says where a body written in its place
/// lines up.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Site {
    /// Its tags share their line with nothing else: at that line's indent.
    Line,
    /// Its tags stand alone round its body: where its body's lines are.
    Body,
    /// Among other things on a line: nowhere.
    Among,
}

/// A recursive loop's call on a line of its own, the template's text
/// written right up to it.
pub(super) struct Recursion {
    pub(super) line: usize,
    /// The line of the loop's `for` tag.
    pub(super) opening: usize,
    pub(super) span: Range<usize>,
}

/// A tag of the template's own: a block's opening, middle or end, a
/// statement, or a comment.
struct Tag {
    span: Range<usize>,
    role: Role,
    /// Written with `-` or `+` to say what happens to the spaces round it.
    controlled: bool,
    /// The name a `block` tag gives.
    name: Option<String>,
    /// The name a `macro` or `set` tag gives the body it holds back.
    holds: Option<String>,
    /// Opens a `call` or `filter`, which writes where it stands a body
    /// the engine holds back.
    writes: bool,
    /// Opens a `for`, whose body is written once for each pass.
    repeats: bool,
    /// A `for` tag's loop can be run again from inside it.
    recursive: bool,
    /// Opens or closes `raw`.
    raw: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Role {
    Opens(Kind),
    Middle,
    Ends,
    /// A tag that writes something, as `include` does.
    Writes,
    Quiet,
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Flowing,
    Placed,
}

/// A value that is nothing but a call writing template text of its own.
#[derive(Clone, Copy, PartialEq)]
enum Calls {
    /// `loop(...)`, running a recursive loop's body again.
    Loop,
    /// `super()`, writing the body of the block this one replaces.
    Super,
}

/// What the engine's tokenizer reads in a template.
#[derive(Default)]
struct Read {
    tags: Vec<Tag>,
    /// Values printed, from `{{` to `}}`.
    values: Vec<Range<usize>>,
    /// Those values that are a call and nothing more.
    calls: Vec<(Range<usize>, Calls)>,
    /// Where each value that is a name, or a call of one, and nothing
    /// more starts, with the name.
    names: Vec<(usize, String)>,
    /// The template's own text.
    text: Vec<Range<usize>>,
    /// Text between `raw` and `endraw`.
    raw: Vec<Range<usize>>,
    /// The runs of whitespace the engine's own whitespace control leaves
    /// out, as [`super::check::reading`] sets it: before a tag opening its
    /// line and the line ending after a tag closing one.
    gaps: Vec<Range<usize>>,
}

impl Read {
    fn of(source: &str) -> Option<Self> {
        let mut read = Self::default();
        let mut tokens = tokenize(source, false, super::check::reading()).peekable();
        let mut done = 0;
        let mut in_raw = false;
        // Where the text the engine writes, or a tag, last ended, and the
        // stretches from there to the next, which hold nothing else.
        let mut written = 0;
        let mut between = Vec::new();
        while let Some(token) = tokens.next() {
            let (token, span) = token.ok()?;
            let start = span.start_offset as usize;
            // Comments and `raw`'s own tags give no token.
            read.quiet(source, done..start, &mut in_raw);
            let tagged = if let Token::TemplateData(text) = token {
                // Its span is the raw text before the engine trims it.
                let at = text.as_ptr() as usize - source.as_ptr() as usize;
                between.push(written..at);
                written = at + text.len();
                false
            } else {
                between.push(written..start);
                true
            };
            match token {
                Token::TemplateData(_) => {
                    let range = start..span.end_offset as usize;
                    done = range.end;
                    if in_raw {
                        read.raw.push(range);
                    } else {
                        read.text.push(range);
                    }
                }
                Token::VariableStart => {
                    let mut call = None;
                    let mut name = None;
                    let mut seen = 0;
                    let mut depth = 0;
                    let mut whole = true;
                    done = upto(&mut tokens, |token| {
                        if matches!(token, Token::VariableEnd) {
                            return true;
                        }
                        match (seen, token) {
                            (0, Token::Ident(ident)) => {
                                name = Some(*ident);
                                call = match *ident {
                                    "loop" => Some(Calls::Loop),
                                    "super" => Some(Calls::Super),
                                    _ => None,
                                };
                            }
                            (1, Token::ParenOpen) => depth = 1,
                            (0 | 1, _) => whole = false,
                            _ if depth == 0 => whole = false,
                            (_, Token::ParenOpen) => depth += 1,
                            (_, Token::ParenClose) => depth -= 1,
                            _ => {}
                        }
                        seen += 1;
                        false
                    })?;
                    read.values.push(start..done);
                    if whole && depth == 0 {
                        if let Some(call) = call {
                            read.calls.push((start..done, call));
                        }
                        if let Some(name) = name {
                            read.names.push((start, name.to_owned()));
                        }
                    }
                }
                Token::BlockStart => {
                    let mut assigns = false;
                    let word = match tokens.peek() {
                        Some(Ok((Token::Ident(word), _))) => *word,
                        _ => "",
                    };
                    let mut idents = 0;
                    let mut name = None;
                    let mut holds = None;
                    let mut recursive = false;
                    done = upto(&mut tokens, |token| {
                        assigns |= matches!(token, Token::Assign);
                        if let Token::Ident(ident) = token {
                            idents += 1;
                            if idents == 2 && word == "block" {
                                name = Some((*ident).to_owned());
                            }
                            if idents == 2 && matches!(word, "macro" | "set") {
                                holds = Some((*ident).to_owned());
                            }
                            recursive |= word == "for" && *ident == "recursive";
                        }
                        matches!(token, Token::BlockEnd)
                    })?;
                    let role = match word {
                        "else" | "elif" => Role::Middle,
                        "include" => Role::Writes,
                        "set" if assigns => Role::Quiet,
                        word if word.starts_with("end") => Role::Ends,
                        word if FLOWING.contains(&word) => Role::Opens(Kind::Flowing),
                        word if PLACED.contains(&word) => Role::Opens(Kind::Placed),
                        _ => Role::Quiet,
                    };
                    read.tags.push(Tag {
                        span: start..done,
                        role,
                        controlled: controlled(&source[start..done]),
                        name,
                        holds,
                        writes: matches!(word, "call" | "filter"),
                        repeats: word == "for",
                        recursive,
                        raw: false,
                    });
                }
                _ => return None,
            }
            if tagged {
                written = done;
            }
        }
        read.quiet(source, done..source.len(), &mut in_raw);
        between.push(written..source.len());
        for range in between {
            read.gaps(source, range);
        }
        Some(read)
    }

    /// Notes the runs of whitespace in `range`, a stretch between the
    /// engine's tokens, outside the comments and `raw`'s tags in it.
    fn gaps(&mut self, source: &str, range: Range<usize>) {
        let mut run: Option<usize> = None;
        let mut at = range.start;
        while at < range.end {
            let quiet = self
                .tags
                .binary_search_by_key(&at, |tag| tag.span.start)
                .ok()
                .map(|found| self.tags[found].span.end);
            let c = source[at..].chars().next().expect("a character");
            // As the engine's lexer counts whitespace.
            let space = quiet.is_none() && c.is_whitespace();
            match (space, run) {
                (true, None) => run = Some(at),
                (false, Some(start)) => {
                    self.gaps.push(start..at);
                    run = None;
                }
                _ => {}
            }
            // A line ending closes a run: what follows is the next line's.
            if let (Some(start), '\n') = (run, c) {
                self.gaps.push(start..at + 1);
                run = None;
            }
            at = quiet.unwrap_or(at + c.len_utf8()).min(range.end);
        }
        if let Some(start) = run {
            self.gaps.push(start..range.end);
        }
    }

    /// Reads what lies between two tokens: spaces the engine leaves out,
    /// comments, and the tags opening and closing `raw`.
    fn quiet(&mut self, source: &str, range: Range<usize>, in_raw: &mut bool) {
        let mut at = range.start;
        while let Some(found) = source[at..range.end].find('{') {
            let start = at + found;
            let rest = &source[start..range.end];
            let raw = rest.starts_with("{%");
            let close = if rest.starts_with("{#") {
                "#}"
            } else if raw {
                *in_raw = !*in_raw;
                "%}"
            } else {
                at = start + 1;
                continue;
            };
            let end = rest[2..]
                .find(close)
                .map_or(range.end, |end| start + 2 + end + 2);
            self.tags.push(Tag {
                span: start..end,
                role: Role::Quiet,
                controlled: controlled(&source[start..end]),
                name: None,
                holds: None,
                writes: false,
                repeats: false,
                recursive: false,
                raw,
            });
            at = end;
        }
    }
}

/// Takes tokens to the one `last` picks, giving where it ends.
fn upto<'s>(
    tokens: &mut impl Iterator<Item = Result<(Token<'s>, minijinja::machinery::Span), minijinja::Error>>,
    mut last: impl FnMut(&Token<'s>) -> bool,
) -> Option<usize> {
    for token in tokens {
        let (token, span) = token.ok()?;
        if last(&token) {
            return Some(span.end_offset as usize);
        }
    }
    None
}

/// Whether a tag is written with `-` or `+` inside its delimiters.
fn controlled(tag: &str) -> bool {
    let bytes = tag.as_bytes();
    let inner = |at: usize| bytes.get(at).is_some_and(|b| matches!(b, b'-' | b'+'));
    inner(2) || bytes.len() >= 3 && inner(bytes.len() - 3)
}

#[derive(Clone, Copy, PartialEq)]
enum Shape {
    /// Nothing, or spaces.
    Blank,
    /// Tags alone, which the engine writes nothing for.
    Tags,
    /// What is written on the page.
    Content,
    /// Inside a tag begun on an earlier line.
    Within,
    /// Inside `raw`.
    Raw,
}

struct Line {
    start: usize,
    /// The bytes of its indent.
    indent: usize,
    /// How wide its indent is.
    column: usize,
    shape: Shape,
    /// The tags on it, by their place in [`Read::tags`].
    tags: Vec<usize>,
    /// Elements it leaves open, less those it closes.
    opens: isize,
    /// Its markup begins with an end tag.
    closes_first: bool,
    /// The line where a start tag ending on it began.
    began: Option<usize>,
    /// It starts inside `<pre>` or `<textarea>`.
    shown: bool,
    /// It starts inside `<script>` or `<style>`.
    scripted: bool,
    /// It opens with a comment, the markup's or the template's.
    comment: bool,
}

struct Block {
    kind: Kind,
    /// Its opening tag, by its place in [`Read::tags`].
    tag: usize,
    opening: usize,
    middles: Vec<usize>,
    end: Option<usize>,
    parent: Option<usize>,
    /// Every tag stands alone on its line, written plainly.
    alone: bool,
    /// The columns its body gives up.
    cut: usize,
    /// The column its opening tag is written at.
    floor: usize,
}

impl Block {
    /// Each branch's lines, from after its tag to before the next.
    fn branches(&self) -> Vec<Range<usize>> {
        let end = self.end.expect("a closed block");
        let mut edges = vec![self.opening];
        edges.extend(&self.middles);
        edges.push(end);
        edges.windows(2).map(|pair| pair[0] + 1..pair[1]).collect()
    }
}

struct Template<'s> {
    source: &'s str,
    lines: Vec<Line>,
    blocks: Vec<Block>,
    /// The innermost block whose body holds each line.
    owner: Vec<Option<usize>>,
    /// Whether the file is indented with tabs, a column each.
    tabs: bool,
    step: Option<usize>,
    read: Read,
}

impl<'s> Template<'s> {
    fn new(source: &'s str, read: Read) -> Self {
        let mut starts = vec![0];
        starts.extend(source.match_indices('\n').map(|(at, _)| at + 1));
        let line_of = |at: usize| starts.partition_point(|&start| start <= at) - 1;
        let indents: Vec<&str> = starts
            .iter()
            .map(|&start| {
                let rest = &source[start..];
                &rest[..rest.len() - rest.trim_start_matches([' ', '\t']).len()]
            })
            .collect();
        let tabbed = indents.iter().filter(|i| i.starts_with('\t')).count();
        let spaced = indents.iter().filter(|i| i.starts_with(' ')).count();
        let tabs = tabbed > spaced;
        let mut lines: Vec<Line> = starts
            .iter()
            .zip(&indents)
            .map(|(&start, indent)| Line {
                start,
                indent: indent.len(),
                column: width(indent, tabs),
                shape: Shape::Blank,
                tags: Vec::new(),
                opens: 0,
                closes_first: false,
                began: None,
                shown: false,
                scripted: false,
                comment: false,
            })
            .collect();

        let mut content = vec![false; lines.len()];
        let mut within = vec![None; lines.len()];
        // The lines after the first that a tag, a value or `raw` spans.
        let mut spans = |range: &Range<usize>, shape: Shape| {
            let last = line_of(range.end.saturating_sub(1).max(range.start));
            within[line_of(range.start) + 1..=last].fill(Some(shape));
        };
        for (at, tag) in read.tags.iter().enumerate() {
            let line = line_of(tag.span.start);
            lines[line].tags.push(at);
            content[line] |= tag.role == Role::Writes;
            spans(&tag.span, Shape::Within);
        }
        for value in &read.values {
            content[line_of(value.start)] = true;
            spans(value, Shape::Within);
        }
        for raw in &read.raw {
            content[line_of(raw.start)] = true;
            spans(raw, Shape::Raw);
        }
        for text in &read.text {
            let mut at = text.start;
            for piece in source[text.clone()].split_inclusive('\n') {
                if !piece.trim().is_empty() {
                    content[line_of(at)] = true;
                }
                at += piece.len();
            }
        }
        for (number, line) in lines.iter_mut().enumerate() {
            line.shape = match within[number] {
                Some(shape) => shape,
                None if content[number] => Shape::Content,
                None if !line.tags.is_empty() => Shape::Tags,
                None => Shape::Blank,
            };
            let rest = source[line.start + line.indent..].trim_start();
            line.comment = rest.starts_with("<!--") || rest.starts_with("{#");
        }

        let mut blanked = source.as_bytes().to_vec();
        let mut blank = |range: &Range<usize>, with: u8| {
            for byte in &mut blanked[range.clone()] {
                if *byte != b'\n' {
                    *byte = with;
                }
            }
        };
        for tag in &read.tags {
            blank(&tag.span, b' ');
        }
        for range in read.values.iter().chain(&read.raw) {
            blank(range, b'x');
        }
        markup(&mut lines, &blanked);

        let mut template = Template {
            source,
            owner: vec![None; lines.len()],
            lines,
            blocks: Vec::new(),
            tabs,
            step: None,
            read: Read::default(),
        };
        template.nest(&read);
        template.step = template.step();
        template.read = read;
        template
    }

    /// Finds the blocks from their tags, in the order they open.
    fn nest(&mut self, read: &Read) {
        let mut open: Vec<usize> = Vec::new();
        for (number, line) in self.lines.iter().enumerate() {
            let alone = line.shape == Shape::Tags;
            for &at in &line.tags {
                let tag = &read.tags[at];
                let plain = alone && !tag.controlled;
                match tag.role {
                    Role::Opens(kind) => {
                        open.push(self.blocks.len());
                        self.blocks.push(Block {
                            kind,
                            tag: at,
                            opening: number,
                            middles: Vec::new(),
                            end: None,
                            parent: open.iter().rev().nth(1).copied(),
                            alone: plain,
                            cut: 0,
                            floor: 0,
                        });
                    }
                    Role::Middle => {
                        if let Some(&block) = open.last() {
                            self.blocks[block].middles.push(number);
                            self.blocks[block].alone &= plain;
                        }
                    }
                    Role::Ends => {
                        if let Some(block) = open.pop() {
                            self.blocks[block].end = Some(number);
                            self.blocks[block].alone &= plain;
                        }
                    }
                    Role::Writes | Role::Quiet => {}
                }
            }
        }
        for (at, block) in self.blocks.iter().enumerate() {
            // A block may open and close on one line.
            if let Some(end) = block.end.filter(|&end| end > block.opening) {
                for owner in &mut self.owner[block.opening + 1..end] {
                    *owner = Some(at);
                }
            }
        }
    }

    /// The commonest step into an element between two lines with no tag
    /// line between them.
    fn step(&self) -> Option<usize> {
        let mut steps: HashMap<usize, usize> = HashMap::new();
        let mut before: Option<&Line> = None;
        for line in &self.lines {
            match line.shape {
                Shape::Tags => before = None,
                Shape::Content if !line.shown => {
                    if let Some(before) = before
                        && line.column > before.column
                        && before.opens > 0
                        && before.began.is_none()
                    {
                        *steps.entry(line.column - before.column).or_default() += 1;
                    }
                    before = Some(line);
                }
                _ => {}
            }
        }
        // The smaller step where two are as common.
        steps
            .into_iter()
            .max_by_key(|&(step, count)| (count, std::cmp::Reverse(step)))
            .map(|(step, _)| step)
    }

    /// The column a line is written at once the blocks round it have
    /// given up what they give, none taking it left of its own tag. A body
    /// the engine holds back is written where it is called, so the blocks
    /// round the one holding it back give it nothing.
    fn column(&self, line: usize) -> usize {
        let mut column = self.lines[line].column;
        if self.lines[line].shown {
            return column;
        }
        let mut chain = Vec::new();
        let mut owner = self.owner[line];
        while let Some(block) = owner {
            if self.blocks[block].kind == Kind::Placed {
                break;
            }
            chain.push(block);
            owner = self.blocks[block].parent;
        }
        for &block in chain.iter().rev() {
            let block = &self.blocks[block];
            if block.cut > 0 {
                column = column.saturating_sub(block.cut).max(block.floor);
            }
        }
        column
    }

    fn decide(&mut self) {
        for block in 0..self.blocks.len() {
            self.blocks[block].floor = self.column(self.blocks[block].opening);
            self.blocks[block].cut = self.excess(block).unwrap_or(0);
        }
    }

    /// What block `at`'s body gives up; none where it gives up nothing.
    fn excess(&self, at: usize) -> Option<usize> {
        let block = &self.blocks[at];
        let end = block.end?;
        if block.kind != Kind::Flowing || !block.alone {
            return None;
        }
        let opening = &self.lines[block.opening];
        if opening.shown || opening.scripted {
            return None;
        }
        let column = opening.column;
        if self.lines[end].column != column
            || block
                .middles
                .iter()
                .any(|&m| self.lines[m].column != column)
        {
            return None;
        }
        let branches = block.branches();
        let body = || branches.iter().flat_map(Clone::clone);
        if body().any(|line| self.lines[line].shape == Shape::Raw) {
            return None;
        }
        // Spaces worth a tab can't be given up from a tab.
        if !self.tabs
            && body().any(|line| {
                let line = &self.lines[line];
                self.source[line.start..line.start + line.indent].contains('\t')
            })
        {
            return None;
        }

        let tag = block.floor;
        let mut firsts = branches.iter().filter_map(|branch| {
            branch
                .clone()
                .find(|&line| matches!(self.lines[line].shape, Shape::Content | Shape::Tags))
        });
        let first = self.column(firsts.next()?);
        if firsts.any(|line| self.column(line) != first) {
            return None;
        }
        let excess = first.checked_sub(tag).filter(|&excess| excess > 0)?;

        let before = (0..block.opening)
            .rev()
            .find(|&line| self.lines[line].shape == Shape::Content);
        let (outside, opens) = match before {
            Some(line) => {
                let began = self.lines[line].began.unwrap_or(line);
                (self.column(began), self.lines[line].opens > 0)
            }
            None => (0, false),
        };
        let template = match (opens, self.step) {
            (true, Some(step)) => {
                if first == outside + step {
                    false
                } else if tag == outside + step {
                    true
                } else {
                    return None;
                }
            }
            (true, None) => tag > outside,
            (false, _) => first != outside && tag == outside,
        };
        if !template {
            return None;
        }

        for branch in &branches {
            let mut depth = 0;
            let mut lowest = 0;
            for line in branch.clone() {
                let line = &self.lines[line];
                // Inside `<pre>` a line opens nothing, but may close it.
                if line.shape == Shape::Content {
                    depth += line.opens;
                    lowest = lowest.min(depth);
                }
            }
            if depth != 0 || lowest < 0 {
                return None;
            }
        }
        for line in body() {
            let shape = self.lines[line].shape;
            if !matches!(shape, Shape::Content | Shape::Tags) || self.lines[line].shown {
                continue;
            }
            let column = self.column(line);
            let label = column == tag && self.lines[line].comment;
            let closing = shape == Shape::Content && self.lines[line].closes_first && column >= tag;
            if column < tag + excess && !label && !closing {
                return None;
            }
        }
        Some(excess)
    }

    fn facts(&self, markup: bool) -> Facts {
        let bytes = self.source.as_bytes();
        let tags = &self.read.tags;
        let line_of = |at: usize| self.lines.partition_point(|line| line.start <= at) - 1;
        // Where each line's text ends, before its line ending.
        let ends: Vec<usize> = (0..self.lines.len())
            .map(|number| {
                let Some(next) = self.lines.get(number + 1) else {
                    return self.source.len();
                };
                let end = next.start - 1;
                if end > 0 && bytes[end - 1] == b'\r' {
                    end - 1
                } else {
                    end
                }
            })
            .collect();
        let holds: Vec<bool> = (0..self.lines.len())
            .map(|number| {
                !self.source[self.lines[number].start..ends[number]]
                    .trim()
                    .is_empty()
            })
            .collect();

        let mut blocks = Vec::new();
        for block in &self.blocks {
            let (Some(name), Some(end)) = (&tags[block.tag].name, block.end) else {
                continue;
            };
            let line = &self.lines[block.opening];
            let site = if end == block.opening
                && line.shape == Shape::Tags
                && line.tags.len() == 2
                && line.tags.iter().all(|&at| !tags[at].controlled)
            {
                Site::Line
            } else if block.alone && end > block.opening {
                Site::Body
            } else {
                Site::Among
            };
            blocks.push(Named {
                name: name.clone(),
                opening: block.opening,
                end,
                site,
            });
        }

        // A `call` or `filter` whose tags stand alone on an indented line
        // of markup writes a body held back where that line stands, so the
        // engine keeps the indent for the body to line up at.
        let writing: Vec<bool> = (0..self.lines.len())
            .map(|number| {
                let line = &self.lines[number];
                markup
                    && line.indent > 0
                    && line.shape == Shape::Tags
                    && !line.shown
                    && !line.scripted
                    && line.tags.iter().all(|&at| !tags[at].controlled)
                    && self.blocks.iter().any(|block| {
                        tags[block.tag].writes
                            && line.tags[0] == block.tag
                            && matches!(bytes[tags[block.tag].span.start + 2], b' ' | b'\t')
                            && (line.tags.len() == 1
                                || line.tags.len() == 2 && block.end == Some(number))
                    })
            })
            .collect();
        // Where a tag starts or ends that keeps the spaces next to it, which
        // it would otherwise take with it.
        let mut keeps = Vec::new();
        let mut later = vec![false; self.lines.len()];
        let mut writes = false;
        let mut kept = vec![false; self.lines.len()];
        let mut kept_ends = vec![false; self.lines.len()];
        for number in (0..self.lines.len()).rev() {
            let line = &self.lines[number];
            let placed = blocks
                .iter()
                .any(|block| block.opening == number && block.site == Site::Line);
            let more = line.shape == Shape::Content || placed;
            let first = line.tags.first().map(|&at| &tags[at]).filter(|tag| {
                (more || writing[number])
                    && tag.span.start == line.start + line.indent
                    && !tag.raw
                    && !tag.controlled
            });
            if let Some(first) = first
                && line.indent > 0
                && matches!(bytes[first.span.start + 2], b' ' | b'\t')
            {
                keeps.push(first.span.start);
                kept[number] = true;
            }
            let last = line
                .tags
                .last()
                .map(|&at| &tags[at])
                .filter(|tag| tag.span.end == ends[number] && !tag.raw && !tag.controlled);
            later[number] = writes;
            if more
                && writes
                && let Some(last) = last
                && matches!(bytes[last.span.end - 3], b' ' | b'\t')
            {
                keeps.push(last.span.end);
                kept_ends[number] = true;
            }
            // Tags, comments among them, write nothing after it.
            writes |= more;
        }
        // The tag a line holds and nothing else, written plainly with a
        // space inside each delimiter.
        let lone = |number: usize| {
            let line = &self.lines[number];
            let [only] = line.tags[..] else {
                return None;
            };
            let tag = &tags[only];
            (line.shape == Shape::Tags
                && tag.span.start == line.start + line.indent
                && tag.span.end == ends[number]
                && !tag.controlled
                && matches!(bytes[tag.span.start + 2], b' ' | b'\t')
                && matches!(bytes[tag.span.end - 3], b' ' | b'\t'))
            .then_some(tag)
        };
        // A line ending kept on a block's opening or middle line goes with
        // the branch. Where the branch isn't written, the line ending after
        // the tag closing it stands in: kept for a line of tags alone, the
        // writer leaving it out where nothing was written for it to end.
        //
        // A loop whose tag stands alone on its line and whose end tag
        // closes the last line of its body writes no line ending between
        // its passes. Where the body opens indented, the spaces between the
        // passes say each was meant for lines of its own, so the line
        // ending after its tag goes with each pass, the writer leaving it
        // out where the line before is already ended. Unindented, or
        // where spaces show or aren't markup, the passes run together as
        // typed, a line ending between them being something a reader
        // could see. Either way the line ending after the end tag is left
        // out where no pass was written.
        for block in &self.blocks {
            let Some(end) = block.end else {
                continue;
            };
            if block.kind != Kind::Flowing {
                continue;
            }
            if let Some(closer) = self.last_line_closer(block, &ends)
                && lone(block.opening).is_some()
            {
                let first = &self.lines[block.opening + 1];
                if markup
                    && first.shape == Shape::Content
                    && first.indent > 0
                    && (block.opening..=end)
                        .all(|line| !self.lines[line].shown && !self.lines[line].scripted)
                {
                    keeps.push(tags[block.tag].span.end);
                }
                if later[end] && !kept_ends[end] {
                    keeps.push(closer.span.end);
                    kept_ends[end] = true;
                }
                continue;
            }
            let mut edges = vec![block.opening];
            edges.extend(&block.middles);
            edges.push(end);
            for pair in edges.windows(2) {
                let (from, to) = (pair[0], pair[1]);
                if to == from || !kept_ends[from] {
                    continue;
                }
                let line = &self.lines[to];
                let [only] = line.tags[..] else {
                    continue;
                };
                let tag = &tags[only];
                if later[to]
                    && line.shape == Shape::Tags
                    && tag.span.start == line.start + line.indent
                    && tag.span.end == ends[to]
                    && !tag.controlled
                    && matches!(bytes[tag.span.end - 3], b' ' | b'\t')
                {
                    keeps.push(tag.span.end);
                    kept_ends[to] = true;
                }
            }
        }
        keeps.sort_unstable();

        // A tag written with `-` takes all the spaces next to it whatever
        // the tag beyond them keeps.
        let taking = |gap: &Range<usize>| {
            let before = self.source[..gap.start].trim_end();
            let after = self.source[gap.end..].trim_start();
            ["-%}", "-}}", "-#}"]
                .iter()
                .any(|end| before.ends_with(end))
                || ["{%-", "{{-", "{#-"]
                    .iter()
                    .any(|start| after.starts_with(start))
        };
        // What the page leaves out: the spaces the tags take with them,
        // less those kept.
        let dropped: Vec<Range<usize>> = self
            .read
            .gaps
            .iter()
            .filter(|gap| {
                keeps.binary_search(&gap.end).is_err() && keeps.binary_search(&gap.start).is_err()
                    || taking(gap)
            })
            .cloned()
            .collect();
        // Whether the page leaves out all of `range`.
        let left_out = |range: Range<usize>| {
            let at = dropped.partition_point(|gap| gap.end <= range.start);
            dropped
                .get(at)
                .is_some_and(|gap| gap.start <= range.start && gap.end >= range.end)
        };

        let alone = |span: &Range<usize>| {
            let number = line_of(span.start);
            let line = &self.lines[number];
            (span.start == line.start + line.indent && span.end == ends[number]).then_some(number)
        };
        let mut supers = Vec::new();
        let mut calls = Vec::new();
        for (span, call) in &self.read.calls {
            let Some(number) = alone(span) else {
                continue;
            };
            match call {
                Calls::Super => supers.push(number),
                Calls::Loop => {
                    // The line before ends with a tag the engine leaves the
                    // line ending out after.
                    let trimmed = number == 0
                        || !kept_ends[number - 1]
                            && self.lines[number - 1]
                                .tags
                                .last()
                                .is_some_and(|&at| tags[at].span.end == ends[number - 1]);
                    if self.lines[number].indent == 0 && trimmed {
                        continue;
                    }
                    let opening = self
                        .blocks
                        .iter()
                        .filter(|block| {
                            tags[block.tag].recursive
                                && block.opening < number
                                && block.end.is_some_and(|end| end > number)
                        })
                        .map(|block| block.opening)
                        .max();
                    if let Some(opening) = opening {
                        calls.push(Recursion {
                            line: number,
                            opening,
                            span: span.clone(),
                        });
                    }
                }
            }
        }

        let holding: Vec<&str> = self
            .blocks
            .iter()
            .filter(|block| block.kind == Kind::Placed)
            .filter_map(|block| tags[block.tag].holds.as_deref())
            // Writing the body of the `call` that called the macro.
            .chain(["caller"])
            .collect();
        let names = &self.read.names;
        let lines = self
            .lines
            .iter()
            .enumerate()
            .map(|(number, line)| {
                let movable = matches!(line.shape, Shape::Content | Shape::Blank | Shape::Tags)
                    && !line.shown;
                let cut = if movable {
                    let cut = line.column - self.column(number);
                    if self.tabs { cut } else { cut.min(line.indent) }
                } else {
                    0
                };
                let opening = line.start + line.indent;
                let values = &self.read.values;
                let valued = values
                    .get(values.partition_point(|value| value.start < opening))
                    .is_some_and(|value| value.start == opening);
                let held = writing[number]
                    || names
                        .get(names.partition_point(|(start, _)| *start < opening))
                        .is_some_and(|(start, name)| {
                            *start == opening && holding.contains(&name.as_str())
                        });
                Fact {
                    start: line.start,
                    indent: line.indent,
                    cut,
                    holds: holds[number],
                    content: line.shape == Shape::Content && !line.shown,
                    movable,
                    bare: line.shape == Shape::Tags,
                    valued,
                    held,
                    head: match line.tags.first() {
                        _ if kept[number] => Head::AtLineStart,
                        Some(&at)
                            if tags[at].span.start > line.start
                                && left_out(line.start..tags[at].span.start) =>
                        {
                            Head::Dropped
                        }
                        _ => Head::Typed,
                    },
                    tail: match self.lines.get(number + 1) {
                        _ if kept_ends[number] => Tail::Owed,
                        Some(next) if left_out(ends[number]..next.start) => Tail::Dropped,
                        _ => Tail::Typed,
                    },
                    end: ends[number],
                }
            })
            .collect();
        Facts {
            lines,
            blocks,
            supers,
            calls,
            dropped,
        }
    }

    /// The end tag of `block`, where it is a loop with no `else` and the
    /// tag, written plainly, closes a line of its body holding more. `ends`
    /// is where each line's text ends.
    fn last_line_closer(&self, block: &Block, ends: &[usize]) -> Option<&Tag> {
        let tags = &self.read.tags;
        let end = block.end.filter(|&end| end > block.opening)?;
        let line = &self.lines[end];
        let last = &tags[*line.tags.last()?];
        // Its end tag is the last on the line where no block opened before
        // it ends there too, and the last closes a block opened before.
        let mut depth = 0;
        let mut outward = false;
        for &at in &line.tags {
            outward = false;
            match tags[at].role {
                Role::Opens(_) => depth += 1,
                Role::Ends if depth == 0 => outward = true,
                Role::Ends => depth -= 1,
                Role::Middle | Role::Writes | Role::Quiet => {}
            }
        }
        (tags[block.tag].repeats
            && block.middles.is_empty()
            && line.shape == Shape::Content
            && outward
            && !self
                .blocks
                .iter()
                .any(|other| other.opening < block.opening && other.end == Some(end))
            && last.span.end == ends[end]
            && !last.controlled
            && matches!(self.source.as_bytes()[last.span.end - 3], b' ' | b'\t'))
        .then_some(last)
    }

    #[cfg(test)]
    fn cuts(&self) -> Vec<(usize, usize)> {
        let mut cuts = Vec::new();
        for (number, line) in self.lines.iter().enumerate() {
            if !matches!(line.shape, Shape::Content | Shape::Blank) || line.indent == 0 {
                continue;
            }
            let cut = line.column - self.column(number);
            if cut == 0 {
                continue;
            }
            let indent = &self.source[line.start..line.start + line.indent];
            let bytes = if self.tabs {
                cut
            } else {
                // A tab in the indent left its block as typed.
                cut.min(indent.len())
            };
            cuts.push((line.start, bytes));
        }
        cuts
    }
}

/// How wide an indent is: a column for each character where the file is
/// indented with tabs, and tabs to every fourth column where with spaces.
fn width(indent: &str, tabs: bool) -> usize {
    if tabs {
        return indent.len();
    }
    indent.bytes().fold(0, |column, byte| {
        if byte == b'\t' {
            column / 4 * 4 + 4
        } else {
            column + 1
        }
    })
}

/// A start or end tag being read.
struct Open {
    name: String,
    closing: bool,
    /// The line it began on.
    began: usize,
    /// The quote an attribute's value is inside.
    quote: Option<u8>,
}

/// Reads the markup of each line, the template's own tags blanked out:
/// the elements it opens and closes, and whether it starts inside one that
/// isn't markup.
fn markup(lines: &mut [Line], blanked: &[u8]) {
    let mut tag: Option<Open> = None;
    let mut special: Option<String> = None;
    let mut comment = false;
    for number in 0..lines.len() {
        let start = lines[number].start;
        let end = lines
            .get(number + 1)
            .map_or(blanked.len(), |next| next.start);
        let text = &blanked[start..end];
        let line = &mut lines[number];
        line.shown = special
            .as_deref()
            .is_some_and(|name| PREFORMATTED.contains(&name));
        line.scripted = special
            .as_deref()
            .is_some_and(|name| SCRIPTED.contains(&name));
        let mut seen = false;
        let mut at = 0;
        while at < text.len() {
            if comment {
                match find(&text[at..], b"-->") {
                    Some(found) => {
                        comment = false;
                        at += found + 3;
                    }
                    None => break,
                }
                continue;
            }
            if let Some(open) = &mut tag {
                let byte = text[at];
                at += 1;
                match (open.quote, byte) {
                    (Some(quote), byte) if byte == quote => open.quote = None,
                    (None, b'"' | b'\'') => open.quote = Some(byte),
                    (None, b'>') => {
                        let ended = at >= 2 && text[at - 2] == b'/';
                        let Open {
                            name,
                            closing,
                            began,
                            ..
                        } = tag.take().expect("a tag being read");
                        if closing {
                            if special.as_deref() == Some(name.as_str()) {
                                special = None;
                                line.opens -= 1;
                            } else if special.is_none() {
                                line.opens -= 1;
                            }
                        } else if !VOID.contains(&name.as_str()) && !ended && special.is_none() {
                            line.opens += 1;
                            if PREFORMATTED.contains(&name.as_str())
                                || SCRIPTED.contains(&name.as_str())
                            {
                                special = Some(name);
                            }
                        }
                        if began != number {
                            line.began = Some(began);
                        }
                    }
                    _ => {}
                }
                continue;
            }
            let rest = &text[at..];
            if rest.starts_with(b"<!--") {
                comment = true;
                seen = true;
                at += 4;
                continue;
            }
            if let Some((closing, name)) = element(rest)
                && (special.is_none() || closing && special.as_deref() == Some(name.as_str()))
            {
                if !seen && closing {
                    line.closes_first = true;
                }
                seen = true;
                at += 1 + usize::from(closing) + name.len();
                tag = Some(Open {
                    name,
                    closing,
                    began: number,
                    quote: None,
                });
                continue;
            }
            if !text[at].is_ascii_whitespace() {
                seen = true;
            }
            at += 1;
        }
    }
}

/// The element a tag starting `text` names, in lower case, and whether it
/// is an end tag.
fn element(text: &[u8]) -> Option<(bool, String)> {
    let rest = text.strip_prefix(b"<")?;
    let (closing, rest) = match rest.strip_prefix(b"/") {
        Some(rest) => (true, rest),
        None => (false, rest),
    };
    if !rest.first()?.is_ascii_alphabetic() {
        return None;
    }
    let len = rest
        .iter()
        .take_while(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b':' | b'-'))
        .count();
    Some((
        closing,
        String::from_utf8_lossy(&rest[..len]).to_ascii_lowercase(),
    ))
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    /// The template with each line's indent as the built page has it.
    fn cut(source: &str) -> String {
        let mut out = source.to_owned();
        for (at, len) in super::cuts(source).into_iter().rev() {
            out.replace_range(at..at + len, "");
        }
        out
    }

    #[test]
    fn a_bodys_indent_for_the_templates_sake_is_left_out() {
        let cases = [
            // The tag level with the line before.
            (
                "<ul>\n  <li>Home</li>\n  {% for e in x %}\n      <li>{{ e }}</li>\n  {% endfor %}\n</ul>\n",
                "<ul>\n  <li>Home</li>\n  {% for e in x %}\n  <li>{{ e }}</li>\n  {% endfor %}\n</ul>\n",
            ),
            // The tag one step inside the element the line before opens.
            (
                "<ul>\n  {% for e in x %}\n    <li>{{ e }}</li>\n  {% endfor %}\n</ul>\n",
                "<ul>\n  {% for e in x %}\n  <li>{{ e }}</li>\n  {% endfor %}\n</ul>\n",
            ),
            // Every branch, and blocks inside it adding up.
            (
                "<div>\n  {% if a %}\n    {% for e in x %}\n      <p>{{ e }}</p>\n    {% endfor %}\n  {% else %}\n    <p>\n      none\n    </p>\n  {% endif %}\n</div>\n",
                "<div>\n  {% if a %}\n    {% for e in x %}\n  <p>{{ e }}</p>\n    {% endfor %}\n  {% else %}\n  <p>\n    none\n  </p>\n  {% endif %}\n</div>\n",
            ),
            // A comment level with the tag, heading the next branch.
            (
                "<div>\n  {% if a %}\n    <p>a</p>\n  <!-- b -->\n  {% else %}\n    <p>b</p>\n  {% endif %}\n</div>\n",
                "<div>\n  {% if a %}\n  <p>a</p>\n  <!-- b -->\n  {% else %}\n  <p>b</p>\n  {% endif %}\n</div>\n",
            ),
            // Indented with tabs.
            (
                "<ul>\n\t{% for e in x %}\n\t\t<li>{{ e }}</li>\n\t{% endfor %}\n</ul>\n",
                "<ul>\n\t{% for e in x %}\n\t<li>{{ e }}</li>\n\t{% endfor %}\n</ul>\n",
            ),
        ];
        for (template, built) in cases {
            assert_eq!(cut(template), built, "{template}");
        }
    }

    #[test]
    fn indent_the_markup_calls_for_or_that_cant_be_told_is_kept() {
        let cases = [
            // The body one step inside the element, the tag level with it.
            "<ul>\n{% for e in x %}\n    <li>{{ e }}</li>\n{% endfor %}\n</ul>\n",
            // The body at the content's depth, the tags further left.
            "<ul>\n  <li>Home</li>\n{% for e in x %}\n  <li>{{ e }}</li>\n{% endfor %}\n</ul>\n",
            // Written flat.
            "<ul>\n  {% for e in x %}\n  <li>{{ e }}</li>\n  {% endfor %}\n</ul>\n",
            // Branches starting at different depths.
            "<p>\n  {% if a %}\n    a\n  {% else %}\n      b\n  {% endif %}\n</p>\n",
            // A body line with less indent than the first.
            "<p>\n  {% if a %}\n      a\n    b\n  {% endif %}\n</p>\n",
            // A body closing what it didn't open.
            "<ol>\n  {% for y in x %}\n    {% if y %}\n      </ol>\n      <ol>\n    {% endif %}\n  {% endfor %}\n</ol>\n",
            // A body leaving open what it opened.
            "<div>\n  {% if a %}\n    <section>\n  {% endif %}\n</div>\n",
            // A tag sharing its line.
            "<ul>\n  {% for e in x %}<!-- e -->\n    <li>{{ e }}</li>\n  {% endfor %}\n</ul>\n",
            // An end tag not under the opening one.
            "<ul>\n  {% for e in x %}\n    <li>{{ e }}</li>\n    {% endfor %}\n</ul>\n",
            // The engine told what to do with the spaces.
            "<ul>\n  {%- for e in x %}\n    <li>{{ e }}</li>\n  {% endfor %}\n</ul>\n",
            // Inside a script.
            "<script>\n  {% if a %}\n    go();\n  {% endif %}\n</script>\n",
            // A block whose body is written elsewhere.
            "<ul>\n  {% macro item(e) %}\n    <li>{{ e }}</li>\n  {% endmacro %}\n</ul>\n",
            // Spaces worth a tab.
            "<ul>\n  {% for e in x %}\n\t<li>{{ e }}</li>\n  {% endfor %}\n</ul>\n",
        ];
        for template in cases {
            assert_eq!(cut(template), template);
        }
    }

    #[test]
    fn inside_pre_nothing_moves() {
        assert_eq!(
            cut("<div>\n  {% if a %}\n    <pre>\n      one\n    </pre>\n  {% endif %}\n</div>\n"),
            "<div>\n  {% if a %}\n  <pre>\n      one\n    </pre>\n  {% endif %}\n</div>\n",
        );
        let shown = "<pre>\n{% if a %}\n    one\n{% endif %}\n</pre>\n";
        assert_eq!(cut(shown), shown);
    }

    #[test]
    fn a_blocks_body_written_in_place_gives_up_its_indent_as_a_loops_does() {
        assert_eq!(
            cut("<ul>\n  {% block items %}\n    <li>a</li>\n  {% endblock %}\n</ul>\n"),
            "<ul>\n  {% block items %}\n  <li>a</li>\n  {% endblock %}\n</ul>\n",
        );
    }

    /// The template written as the engine's own whitespace control would
    /// need it to leave out what the page does: `+` inside a tag's
    /// delimiter where it keeps the spaces next to it, `-` where it takes
    /// the line ending before it as well.
    fn kept(source: &str) -> String {
        marked(source, true)
    }

    fn marked(source: &str, markup: bool) -> String {
        let mut out = source.as_bytes().to_vec();
        let facts = super::facts(source, markup).expect("a template");
        let read = super::Read::of(source).expect("a template");
        let dropped = |range: &std::ops::Range<usize>| {
            facts
                .dropped
                .iter()
                .any(|gap| gap.start <= range.start && gap.end >= range.end)
        };
        for gap in read.gaps.iter().filter(|gap| !dropped(gap)) {
            // A line ending follows a tag; spaces before a line's end
            // precede one.
            let after = source[gap.clone()].ends_with('\n');
            for tag in &read.tags {
                if !after && tag.span.start == gap.end {
                    out[tag.span.start + 2] = b'+';
                }
                if after && tag.span.end == gap.start {
                    out[tag.span.end - 3] = b'+';
                }
            }
        }
        for gap in &facts.dropped {
            let lexed: usize = read
                .gaps
                .iter()
                .map(|ours| {
                    ours.end
                        .min(gap.end)
                        .saturating_sub(ours.start.max(gap.start))
                })
                .sum();
            if lexed < gap.len()
                && let Some(tag) = read.tags.iter().find(|tag| tag.span.start == gap.end)
            {
                out[tag.span.start + 2] = b'-';
            }
        }
        String::from_utf8(out).expect("text")
    }

    #[test]
    fn a_line_opening_with_a_tag_and_holding_more_keeps_its_indent_and_line_end() {
        let cases = [
            (
                "<div>\n  {% if a %}<p>a</p>{% endif %}\n</div>\n",
                "<div>\n  {%+if a %}<p>a</p>{% endif+%}\n</div>\n",
            ),
            // A line ending with what it holds keeps its line ending anyway.
            (
                "<div>\n  {% if a %}<p>a</p>\n  {% endif %}\n</div>\n",
                "<div>\n  {%+if a %}<p>a</p>\n  {% endif %}\n</div>\n",
            ),
            // At the left, there is no indent to keep.
            (
                "{# a #}<p>a</p>{# b #}\n<p>b</p>\n",
                "{# a #}<p>a</p>{# b+#}\n<p>b</p>\n",
            ),
            // A block written where its tags stand.
            (
                "<main>\n    {% block main %}{% endblock %}\n</main>\n",
                "<main>\n    {%+block main %}{% endblock+%}\n</main>\n",
            ),
            // The last line's ending would follow what a snippet writes.
            (
                "<p>a</p>\n  {% if a %}<p>b</p>{% endif %}\n",
                "<p>a</p>\n  {%+if a %}<p>b</p>{% endif %}\n",
            ),
        ];
        for (template, handed) in cases {
            assert_eq!(kept(template), handed, "{template}");
        }
    }

    #[test]
    fn a_line_ending_with_a_tag_and_holding_more_keeps_its_line_end() {
        let cases = [
            (
                "<div>\n  <p>a</p>{% if a %}\n  <p>b</p>{% endif %}\n  <p>c</p>\n</div>\n",
                "<div>\n  <p>a</p>{% if a+%}\n  <p>b</p>{% endif+%}\n  <p>c</p>\n</div>\n",
            ),
            // A recursive loop's level closing its list.
            (
                "<li>\n  {% if e %}\n  <ul>\n    {{ loop(e) }}\n  </ul>{% endif %}\n</li>\n",
                "<li>\n  {% if e %}\n  <ul>\n    {{ loop(e) }}\n  </ul>{% endif+%}\n</li>\n",
            ),
            // Opening with a tag that says what becomes of the spaces.
            (
                "<div>\n  {%- if a %}<p>a</p>{% endif %}\n</div>\n",
                "<div>\n  {%- if a %}<p>a</p>{% endif+%}\n</div>\n",
            ),
            // Where the line end shows.
            (
                "<pre>\na{% if a %}\nb{% endif %}\n</pre>\n",
                "<pre>\na{% if a+%}\nb{% endif+%}\n</pre>\n",
            ),
            (
                "<script>\n  go(){% if a %}\n  stop(){% endif %}\n</script>\n",
                "<script>\n  go(){% if a+%}\n  stop(){% endif+%}\n</script>\n",
            ),
        ];
        for (template, handed) in cases {
            assert_eq!(kept(template), handed, "{template}");
        }
    }

    #[test]
    fn in_a_body_held_back_a_line_closing_two_branches_keeps_its_line_end() {
        assert_eq!(
            kept(
                "{% macro m(a, b) %}\n<div>\n  {% if a %}\n  {% if b %}\n  <p>x</p>{% endif %}{% endif %}\n</div>\n{% endmacro %}\n"
            ),
            "{% macro m(a, b) %}\n<div>\n  {% if a %}\n  {% if b %}\n  <p>x</p>{% endif %}{% endif+%}\n</div>\n{% endmacro %}\n",
        );
    }

    #[test]
    fn a_branch_after_a_kept_line_end_keeps_the_line_end_of_the_tag_closing_it() {
        let cases = [
            (
                "<p>a</p>{% if a %}\n<p>b</p>\n{% endif %}\n<p>c</p>\n",
                "<p>a</p>{% if a+%}\n<p>b</p>\n{% endif+%}\n<p>c</p>\n",
            ),
            // Each branch in turn.
            (
                "<p>a</p>{% if a %}\n<p>b</p>\n  {% elif b %}\n<p>c</p>\n{% else %}\n<p>d</p>\n{% endif %}\n<p>e</p>\n",
                "<p>a</p>{% if a+%}\n<p>b</p>\n  {% elif b+%}\n<p>c</p>\n{% else+%}\n<p>d</p>\n{% endif+%}\n<p>e</p>\n",
            ),
            // Every block whose branch holds it.
            (
                "<p>a</p>{% if a %}{% if b %}\n<p>b</p>\n{% endif %}\n{% endif %}\n<p>c</p>\n",
                "<p>a</p>{% if a %}{% if b+%}\n<p>b</p>\n{% endif+%}\n{% endif+%}\n<p>c</p>\n",
            ),
            // Only those.
            (
                "<p>a</p>{% for e in x %}\n{% if e %}\n<p>b</p>\n{% endif %}\n{% endfor %}\n<p>c</p>\n",
                "<p>a</p>{% for e in x+%}\n{% if e %}\n<p>b</p>\n{% endif %}\n{% endfor+%}\n<p>c</p>\n",
            ),
            // Not where it shares its line, or nothing is written after it.
            (
                "<p>a</p>{% if a %}\n<p>b</p>\n{% endif %}{# c #}\n<p>c</p>\n",
                "<p>a</p>{% if a+%}\n<p>b</p>\n{% endif %}{# c #}\n<p>c</p>\n",
            ),
            (
                "<p>a</p>{% if a %}\n<p>b</p>\n{% endif %}\n",
                "<p>a</p>{% if a+%}\n<p>b</p>\n{% endif %}\n",
            ),
            // In a body the engine holds back as in the page.
            (
                "{% macro m(a) %}\n<p>a</p>{% if a %}\n<p>b</p>{% endif %}\n<p>c</p>{% if a %}\n<p>d</p>\n{% endif %}\n{% if a %}\n<p>e</p>{% endif %}\n<p>f</p>\n{% endmacro %}\n",
                "{% macro m(a) %}\n<p>a</p>{% if a+%}\n<p>b</p>{% endif+%}\n<p>c</p>{% if a+%}\n<p>d</p>\n{% endif+%}\n{% if a %}\n<p>e</p>{% endif+%}\n<p>f</p>\n{% endmacro %}\n",
            ),
        ];
        for (template, handed) in cases {
            assert_eq!(kept(template), handed, "{template}");
        }
    }

    #[test]
    fn in_a_body_held_back_the_tags_round_a_branch_are_read_as_in_the_page() {
        for body in [
            "  <p>a</p>{% if a %}\n  <p>b</p>\n  {% endif %}\n  <p>c</p>\n",
            "<p>a</p>{% if a %}\n<p>b</p>\n{% elif b %}\n<p>c</p>\n{% else %}\n<p>d</p>\n{% endif %}\n<p>e</p>\n",
            "  <p>a</p>\n  {% if a %}\n  <p>b</p>{% else %}\n  <p>c</p>{% endif %}\n  <p>d</p>\n",
            "<p>a</p>{% if a %}\n<p>b</p>\n{% endif %}\n",
            "<p>a</p>{% if a %}\n<p>b</p>\n{% endif %}\n{% if a %}\n<p>c</p>{% endif %}\n<p>d</p>\n",
            "<p>a</p>\n{% if a %}\n{% if b %}\n<p>b</p>{% endif %}\n<p>c</p>{% endif %}\n<p>d</p>\n",
            "<div>\n  {% if a %}\n  {% if b %}\n  <p>x</p>{% endif %}{% endif %}\n</div>\n",
            // A blank line, a comment or spaces before the tag closing it.
            "<p>a</p>{% if a %}\n<p>b</p>\n\n{% endif %}\n<p>c</p>\n",
            "<p>a</p>{% if a %}\n<p>b</p>{# b #}\n{% endif %}\n<p>c</p>\n",
            "<p>a</p>{% if a %}\n<p>b</p> \n{% endif %}\n<p>c</p>\n",
        ] {
            for (open, close) in [
                ("{% macro m(a, b) %}\n", "{% endmacro %}\n"),
                ("{% set s %}\n", "{% endset %}\n"),
            ] {
                assert_eq!(
                    kept(&format!("{open}{body}{close}")),
                    format!("{open}{}{close}", kept(body)),
                    "{body}"
                );
            }
        }
    }

    #[test]
    fn a_loop_closing_its_last_line_keeps_its_tags_line_end_where_its_body_is_indented() {
        let cases = [
            (
                "<div>\n  {% for i in l %}\n  <p>{{ i }}</p>{% endfor %}\n</div>\n",
                "<div>\n  {% for i in l+%}\n  <p>{{ i }}</p>{% endfor+%}\n</div>\n",
            ),
            // In a body held back the same.
            (
                "{% set s %}\n<div>\n  {% for i in l %}\n  <p>{{ i }}</p>{% endfor %}\n</div>\n{% endset %}\n",
                "{% set s %}\n<div>\n  {% for i in l+%}\n  <p>{{ i }}</p>{% endfor+%}\n</div>\n{% endset %}\n",
            ),
            // Closing a branch inside it too.
            (
                "<ul>\n  {% for i in l %}\n  <li>\n    {% if i %}\n    <b>{{ i }}</b>{% endif %}</li>{% endfor %}\n</ul>\n",
                "<ul>\n  {% for i in l+%}\n  <li>\n    {% if i %}\n    <b>{{ i }}</b>{% endif %}</li>{% endfor+%}\n</ul>\n",
            ),
            // Unindented, or where spaces show, the passes run together.
            (
                "<p>\n{% for i in l %}\n<b>{{ i }}</b>{% endfor %}\n</p>\n",
                "<p>\n{% for i in l %}\n<b>{{ i }}</b>{% endfor+%}\n</p>\n",
            ),
            (
                "{% macro m(l) %}\n<p>\n{% for i in l %}\n<b>{{ i }}</b>{% endfor %}\n</p>\n{% endmacro %}\n",
                "{% macro m(l) %}\n<p>\n{% for i in l %}\n<b>{{ i }}</b>{% endfor+%}\n</p>\n{% endmacro %}\n",
            ),
            (
                "<pre>\n  {% for i in l %}\n  {{ i }}{% endfor %}\n</pre>\n",
                "<pre>\n  {% for i in l %}\n  {{ i }}{% endfor+%}\n</pre>\n",
            ),
            // A loop with `else`, or more after its end tag, is left.
            (
                "<div>\n  {% for i in l %}\n  <p>{{ i }}</p>{% else %}\n  <p>none</p>{% endfor %}\n</div>\n",
                "<div>\n  {% for i in l %}\n  <p>{{ i }}</p>{% else+%}\n  <p>none</p>{% endfor+%}\n</div>\n",
            ),
            (
                "<p>\n  {% for i in l %}\n  <b>{{ i }}</b>{% endfor %} after\n</p>\n",
                "<p>\n  {% for i in l %}\n  <b>{{ i }}</b>{% endfor %} after\n</p>\n",
            ),
        ];
        for (template, handed) in cases {
            assert_eq!(kept(template), handed, "{template}");
        }
        // Where the template isn't markup, as plain text.
        assert_eq!(
            marked("{% for i in l %}\n  {{ i }}{% endfor %}\n.\n", false),
            "{% for i in l %}\n  {{ i }}{% endfor+%}\n.\n"
        );
    }

    #[test]
    fn tags_alone_or_saying_what_becomes_of_the_spaces_are_left() {
        for template in [
            "<div>\n  {% if a %}\n  <p>a</p>\n  {% endif %}\n</div>\n",
            "<div>\n  {%- if a %}<p>a</p>{% endif -%}\n</div>\n",
            "<div>\n  {%if a%}<p>a</p>{%endif%}\n</div>\n",
            "<div>\n  <p>{% if a %}a{% endif %}</p>\n</div>\n",
            "<div>\n  <p>a</p>{% if a -%}\n  <p>b</p>{%- endif %}\n</div>\n",
            "<div>\n  <p>a</p>{% raw %}{{ b }}{% endraw %}\n</div>\n",
            "<p>a</p>{% if a %}<p>b</p>{% endif %}\n",
            // A branch's end tag after a line ending that isn't kept.
            "<p>a</p>\n{% if a %}\n<p>b</p>\n{% endif %}\n<p>c</p>\n",
            "<p>a</p>{% if a -%}\n<p>b</p>\n{% endif %}\n<p>c</p>\n",
        ] {
            assert_eq!(kept(template), template);
        }
    }

    #[test]
    fn a_value_writing_a_body_held_back_is_known_at_its_line() {
        let held = |source: &str| -> Vec<usize> {
            let facts = super::facts(source, true).expect("a template");
            (0..facts.lines.len())
                .filter(|&number| facts.lines[number].held)
                .collect()
        };
        assert_eq!(
            held("{% macro m(a) %}\n<p>{{ a }}</p>\n{% endmacro %}\n  {{ m(1) }}\n"),
            [3]
        );
        assert_eq!(held("{% set s %}\n<p>a</p>\n{% endset %}\n{{ s }}\n"), [3]);
        assert_eq!(
            held("{% macro m() %}\n<p>\n  {{ caller() }}\n</p>\n{% endmacro %}\n"),
            [2]
        );
        for template in [
            "{% set s = 1 %}\n  {{ s }}\n",
            "{% macro m() %}\n<p>a</p>\n{% endmacro %}\n  {{ m() | upper }}\n",
            "{% macro m() %}\n<p>a</p>\n{% endmacro %}\n  <p>{{ m() }}</p>\n",
            "{% macro m() %}\n<p>a</p>\n{% endmacro %}\n  {{ n() }}\n",
        ] {
            assert!(held(template).is_empty(), "{template}");
        }
    }

    #[test]
    fn a_call_or_filter_alone_on_an_indented_line_keeps_its_indent_for_what_it_writes() {
        let cases = [
            (
                "<div>\n  {% filter upper %}\n    <p>a</p>\n  {% endfilter %}\n</div>\n",
                "<div>\n  {%+filter upper %}\n    <p>a</p>\n  {% endfilter %}\n</div>\n",
            ),
            (
                "<div>\n  {% call m() %}{% endcall %}\n</div>\n",
                "<div>\n  {%+call m() %}{% endcall %}\n</div>\n",
            ),
            // Inside a body held back as well.
            (
                "{% macro m() %}\n  {% filter upper %}\n    <p>a</p>\n  {% endfilter %}\n{% endmacro %}\n",
                "{% macro m() %}\n  {%+filter upper %}\n    <p>a</p>\n  {% endfilter %}\n{% endmacro %}\n",
            ),
        ];
        for (template, handed) in cases {
            assert_eq!(kept(template), handed, "{template}");
            let facts = super::facts(template, true).expect("a template");
            assert!(facts.lines[1].held, "{template}");
        }
        for template in [
            // At the left there is no indent to keep.
            "<div>\n{% filter upper %}\n  <p>a</p>\n{% endfilter %}\n</div>\n",
            "<div>\n  {% call m() %}{% endcall %}{# a #}\n</div>\n",
            "<div>\n  {%filter upper %}\n    <p>a</p>\n  {% endfilter %}\n</div>\n",
            "<div>\n  {%- filter upper %}\n    <p>a</p>\n  {% endfilter %}\n</div>\n",
            "<pre>\n  {% filter upper %}\n    a\n  {% endfilter %}\n</pre>\n",
            "<script>\n  {% filter upper %}\n    go();\n  {% endfilter %}\n</script>\n",
        ] {
            let facts = super::facts(template, true).expect("a template");
            assert!(facts.lines.iter().all(|line| !line.held), "{template}");
            assert_eq!(kept(template), template);
        }
        // Sharing its line, its indent is kept as any such line's is.
        let template = "<div>\n  {% filter upper %}<p>a</p>{% endfilter %}\n</div>\n";
        let facts = super::facts(template, true).expect("a template");
        assert!(!facts.lines[1].held);
        // Outside markup an indent may mean something.
        let source = "  {% filter upper %}\n    a\n  {% endfilter %}\n";
        let facts = super::facts(source, false).expect("a template");
        assert!(marked(source, false) == source && !facts.lines[0].held);
    }

    #[test]
    fn a_template_the_engine_cant_read_is_left() {
        assert!(super::cuts("<ul>\n  {% for e in x %}\n      <li>{{ e </li>\n").is_empty());
    }
}
