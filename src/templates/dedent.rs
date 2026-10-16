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

use std::collections::HashMap;
use std::ops::Range;

use minijinja::machinery::{Token, tokenize};

/// Blocks whose body is written where the block stands.
const FLOWING: [&str; 4] = ["for", "if", "with", "autoescape"];

/// Blocks whose body is written somewhere else, or held back.
const PLACED: [&str; 5] = ["block", "macro", "call", "filter", "set"];

/// Elements whose lines show their spaces.
const SHOWN: [&str; 2] = ["pre", "textarea"];

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
pub(super) fn cuts(source: &str) -> Vec<(usize, usize)> {
    let Some(read) = Read::of(source) else {
        return Vec::new();
    };
    let mut template = Template::new(source, read);
    template.decide();
    template.cuts()
}

/// A tag of the template's own: a block's opening, middle or end, a
/// statement, or a comment.
struct Tag {
    span: Range<usize>,
    role: Role,
    /// Written with `-` or `+` to say what happens to the spaces round it.
    controlled: bool,
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

/// What the engine's tokenizer reads in a template.
#[derive(Default)]
struct Read {
    tags: Vec<Tag>,
    /// Values printed, from `{{` to `}}`.
    values: Vec<Range<usize>>,
    /// The template's own text.
    text: Vec<Range<usize>>,
    /// Text between `raw` and `endraw`.
    raw: Vec<Range<usize>>,
}

impl Read {
    fn of(source: &str) -> Option<Self> {
        let mut read = Self::default();
        let mut tokens = tokenize(source, false, super::check::reading()).peekable();
        let mut done = 0;
        let mut in_raw = false;
        while let Some(token) = tokens.next() {
            let (token, span) = token.ok()?;
            let start = span.start_offset as usize;
            // Comments and `raw`'s own tags give no token.
            read.quiet(source, done..start, &mut in_raw);
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
                    done = upto(&mut tokens, |token| matches!(token, Token::VariableEnd))?;
                    read.values.push(start..done);
                }
                Token::BlockStart => {
                    let mut assigns = false;
                    let word = match tokens.peek() {
                        Some(Ok((Token::Ident(word), _))) => *word,
                        _ => "",
                    };
                    done = upto(&mut tokens, |token| {
                        assigns |= matches!(token, Token::Assign);
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
                    });
                }
                _ => return None,
            }
        }
        read.quiet(source, done..source.len(), &mut in_raw);
        Some(read)
    }

    /// Reads what lies between two tokens: spaces the engine leaves out,
    /// comments, and the tags opening and closing `raw`.
    fn quiet(&mut self, source: &str, range: Range<usize>, in_raw: &mut bool) {
        let mut at = range.start;
        while let Some(found) = source[at..range.end].find('{') {
            let start = at + found;
            let rest = &source[start..range.end];
            let close = if rest.starts_with("{#") {
                "#}"
            } else if rest.starts_with("{%") {
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
        };
        template.nest(&read);
        template.step = template.step();
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
    /// given up what they give, none taking it left of its own tag.
    fn column(&self, line: usize) -> usize {
        let mut column = self.lines[line].column;
        if self.lines[line].shown {
            return column;
        }
        let mut chain = Vec::new();
        let mut owner = self.owner[line];
        while let Some(block) = owner {
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
        line.shown = special.as_deref().is_some_and(|name| SHOWN.contains(&name));
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
                            if SHOWN.contains(&name.as_str()) || SCRIPTED.contains(&name.as_str()) {
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
    fn a_template_the_engine_cant_read_is_left() {
        assert!(super::cuts("<ul>\n  {% for e in x %}\n      <li>{{ e </li>\n").is_empty());
    }
}
