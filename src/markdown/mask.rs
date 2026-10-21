//! Statac's own syntax, found in the text before Markdown reads it, and
//! three of Markdown's rules that Statac decides for itself.
//!
//! Each snippet call, escaped call and stretch of dollar maths is replaced
//! by placeholders the parser reads as an ordinary word: private-use
//! characters, neither space nor punctuation, so emphasis, links and line
//! structure around them read as they would around the word. A token over
//! several lines gets one placeholder per line, after the line's own
//! indentation and quotation markers, so the parser still sees every line
//! where it was; restored in code, each line keeps the indentation the
//! parser left it. An offset map carries every parser offset back to the
//! source. A call that wraps writing is masked whole, from its `@` to its
//! `@end`, with a placeholder on every line, blank ones too, so the parser
//! reads it as one paragraph; its writing is read on its own (see `block`).
//! A character of the author's that placeholders are made of is masked
//! too, and put back as itself, so it is never read as a placeholder.
//!
//! The three rules: dollar maths by pandoc's rule, so `$30 and $60` stays
//! prices; a colon opening a line starts a definition only with a space
//! after it, so `:-)` stays text; and tabs after a closing code fence count
//! as the spaces CommonMark allows there.

use std::borrow::Cow;

use super::call::{self, Call, Fault, Value};
use super::{Malformed, Wraps, block, controls};
use crate::problem::Problem;
use crate::source::{SourceId, Span};

pub(super) const OPEN: char = '\u{E000}';
const CLOSE: char = '\u{E001}';
const DIGIT: u32 = 0xE010;

pub(super) struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

pub(super) enum TokenKind {
    Call {
        call: Box<Call>,
        /// The writing a call wraps, masked and waiting to be read.
        wrapping: Option<Box<Wrapping>>,
        problems: Vec<Problem>,
    },
    /// `\@name(...)`, which shows the call as text; just `\@` where the
    /// call never closes.
    Escape,
    Maths {
        display: bool,
        /// Without the dollars, or any quotation markers of later lines.
        text: String,
    },
    /// A character the parser must not read as syntax, put back as itself.
    Literal,
}

/// The writing a call wraps, with its margin taken off, and masked. It is
/// read once every definition on the page is known, so a link or footnote
/// marker inside finds one written outside, and the other way about.
pub(super) struct Wrapping {
    pub wrapped: block::Wrapped,
    pub masked: Masked,
}

/// One line's share of a token.
#[derive(Copy, Clone)]
pub(super) struct Piece {
    pub token: usize,
    pub span: Span,
    pub first: bool,
    pub last: bool,
}

pub(super) struct Masked {
    /// The body, with placeholders.
    pub text: String,
    pub tokens: Vec<Token>,
    pub pieces: Vec<Piece>,
    pub map: OffsetMap,
    /// Calls that couldn't be read, said only where they aren't in code.
    pub unread: Vec<Fault>,
    /// Each control character, left out of the text and of every token.
    pub left_out: Vec<(Span, char)>,
}

impl Masked {
    /// Whether a call wraps writing, which is then read apart.
    pub(super) fn wraps_writing(&self) -> bool {
        self.tokens.iter().any(|token| {
            matches!(
                token.kind,
                TokenKind::Call {
                    wrapping: Some(_),
                    ..
                }
            )
        })
    }

    /// The piece whose placeholder starts `s`, and the placeholder's length.
    pub(super) fn piece_at(&self, s: &str) -> Option<(&Piece, usize)> {
        let mut chars = s.char_indices();
        if chars.next()?.1 != OPEN {
            return None;
        }
        let mut index = 0;
        for (i, c) in chars {
            if c == CLOSE {
                return Some((&self.pieces[index], i + c.len_utf8()));
            }
            index = index * 16 + (c as u32).checked_sub(DIGIT).filter(|d| *d < 16)? as usize;
        }
        None
    }
}

/// Carries offsets in the masked text back to the source.
#[derive(Default)]
pub(super) struct OffsetMap {
    base: usize,
    /// Each placeholder: its start and end in the masked text, and the
    /// start and end of what it replaced.
    segments: Vec<(usize, usize, usize, usize)>,
}

impl OffsetMap {
    pub(super) fn offset(&self, masked: usize) -> usize {
        let i = self.segments.partition_point(|s| s.0 <= masked);
        if i == 0 {
            return self.base + masked;
        }
        let (start, end, from, to) = self.segments[i - 1];
        if masked == start {
            from
        } else if masked < end {
            to
        } else {
            to + (masked - end)
        }
    }

    pub(super) fn span(&self, range: std::ops::Range<usize>) -> Span {
        Span::new(self.offset(range.start), self.offset(range.end))
    }
}

/// Masks the body at `body` of `source`, whose text is `text`, `nested`
/// calls deep in the writing of calls that wrap it.
pub(super) fn mask(
    text: &str,
    source: SourceId,
    body: Span,
    wraps: Wraps<'_>,
    nested: usize,
) -> Masked {
    let fixed = fence_tabs(&text[..body.end as usize], body.start as usize);
    let text: &str = &fixed;
    let start = body.start as usize;
    let mut masker = Masker {
        text,
        b: text.as_bytes(),
        out: String::with_capacity(text.len() - start + 16),
        copied: start,
        base: start,
        tokens: Vec::new(),
        pieces: Vec::new(),
        segments: Vec::new(),
        line: Line::default(),
        own: Vec::new(),
    };
    let b = text.as_bytes();
    let mut unread = Vec::new();
    let mut left_out = Vec::new();
    // Calls that may have an `@end` of their own though they wrap nothing:
    // one told of as wrapping nothing, and one to a name the site has no
    // snippet for, which is told of as that.
    let mut ended: Vec<usize> = Vec::new();
    let mut unclosed = 0;
    let mut i = start;
    while let Some(at) = next_special(b, i) {
        i = at + 1;
        match b[at] {
            b'$' => {
                let line = masker.line.of(b, at).0;
                if let Some((to, display)) = maths(text, at, line, &mut unclosed) {
                    let inner = if display {
                        at + 2..to - 2
                    } else {
                        at + 1..to - 1
                    };
                    let depth = masker.depth(at);
                    let text_of = masker.without_prefixes(inner, depth);
                    let text_of = controls::without(&text_of).into_owned();
                    left_out.extend(controls::found(text, at, to));
                    let kind = TokenKind::Maths {
                        display,
                        text: text_of,
                    };
                    masker.replace(at, to, kind, false);
                    i = to;
                } else if b.get(at + 1) == Some(&b'$') {
                    i = at + 2;
                }
            }
            b'{' => {
                if let Some(escaped) = heading_brace(b, start, at, masker.line.of(b, at))
                    && (!escaped || at > masker.copied)
                {
                    match escaped {
                        true => masker.replace(at - 1, at + 1, TokenKind::Escape, false),
                        false => masker.replace(at, at + 1, TokenKind::Literal, false),
                    }
                }
            }
            b':' => {
                let (line, _) = masker.line.of(b, at);
                let opens_line = b[line..at].iter().all(|c| matches!(c, b' ' | b'\t' | b'>'));
                let spaced = b
                    .get(at + 1)
                    .is_none_or(|c| matches!(c, b' ' | b'\t' | b'\n' | b'\r'));
                if opens_line && !spaced {
                    masker.replace(at, at + 1, TokenKind::Literal, false);
                }
            }
            b'@' => {
                if at > start && b[at - 1] == b'\\' {
                    let escaped_backslash = at > start + 1 && b[at - 2] == b'\\';
                    if !escaped_backslash
                        && !glued(text, at - 1)
                        && let Some(read) = call::read(text, source, at, masker.depth(at))
                    {
                        // A call shown as text shows its `@end` as text too.
                        if let Ok(read) = &read
                            && wraps(&read.call.name.value) == Some(true)
                        {
                            ended.push(at);
                        }
                        let to = read.map_or(at + 1, |read| read.end);
                        left_out.extend(controls::found(text, at - 1, to));
                        masker.replace(at - 1, to, TokenKind::Escape, false);
                        i = to;
                    }
                    continue;
                }
                if glued(text, at) {
                    continue;
                }
                let line = masker.line.of(b, at).0;
                match call::read(text, source, at, call::depth(b, line)) {
                    None if block::is_end(b, line, at) => {
                        let end = Span::new(at, at + "@end".len());
                        // An `@end` below a call already told of closes it.
                        match ended.iter().position(|&head| head < at) {
                            Some(head) => {
                                ended.remove(head);
                            }
                            None => unread.push((end, Malformed::StrayEnd)),
                        }
                    }
                    None => {}
                    Some(Ok(mut read)) => {
                        left_out.extend(controls::found(text, at, read.end));
                        for argument in &mut read.call.arguments {
                            if let Value::Text(value) = &mut argument.value.value {
                                *value = controls::without(value).into_owned();
                            }
                        }
                        let name = read.call.name.value.as_str();
                        let own = name == call::IN_SITE;
                        let wrapping = !own && nested < block::DEEPEST && wraps(name) == Some(true);
                        let alone = block::stands_alone(b, line, at, read.end)
                            && !(wrapping && block::fenced(b, start, at));
                        let head = Span::new(at, read.end);
                        if wrapping && alone {
                            let Some(end) = block::end_of(text, source, read.end, wraps) else {
                                let name = name.to_owned();
                                unread.push((head, Malformed::NoEnd(name)));
                                continue;
                            };
                            let from = block::next_line(b, read.end).unwrap_or(end);
                            let to = block::line_start(b, end);
                            let wrapped = block::Wrapped::new(text, at, from, to);
                            let span = Span::new(0, wrapped.text.len());
                            let masked = mask(&wrapped.text, source, span, wraps, nested + 1);
                            read.call.head = Some(head);
                            let to = end + "@end".len();
                            let kind = TokenKind::Call {
                                call: Box::new(read.call),
                                wrapping: Some(Box::new(Wrapping { wrapped, masked })),
                                problems: read.problems,
                            };
                            masker.replace(at, to, kind, true);
                            i = to;
                            continue;
                        }
                        if wrapping || (alone && !own && wraps(name).is_none()) {
                            ended.push(at);
                        }
                        let kind = TokenKind::Call {
                            call: Box::new(read.call),
                            wrapping: None,
                            problems: read.problems,
                        };
                        masker.replace(at, read.end, kind, false);
                        i = read.end;
                    }
                    Some(Err(fault)) => unread.push(fault),
                }
            }
            _ => {
                let c = text[at..].chars().next().expect("a character");
                let to = at + c.len_utf8();
                if controls::left_out(c) {
                    left_out.push((Span::new(at, to), c));
                    masker.remove(at, to);
                    i = to;
                } else if in_placeholders(c) {
                    masker.own(at, c);
                    i = to;
                }
            }
        }
    }
    let rest = &text[masker.copied..];
    masker.out.push_str(rest);
    Masked {
        text: masker.out,
        tokens: masker.tokens,
        pieces: masker.pieces,
        map: OffsetMap {
            base: masker.base,
            segments: masker.segments,
        },
        unread,
        left_out,
    }
}

struct Masker<'a> {
    text: &'a str,
    b: &'a [u8],
    out: String,
    /// How far into the text has been written to `out`.
    copied: usize,
    base: usize,
    tokens: Vec<Token>,
    pieces: Vec<Piece>,
    segments: Vec<(usize, usize, usize, usize)>,
    line: Line,
    /// Each of the author's characters placeholders are made of, and the
    /// piece every one of them is masked as.
    own: Vec<(char, usize)>,
}

impl Masker<'_> {
    /// How many quotation markers the line holding `at` opens with.
    fn depth(&mut self, at: usize) -> usize {
        call::depth(self.b, self.line.of(self.b, at).0)
    }

    /// Masks `from..to` as one token. `every_line` gives a blank line a
    /// placeholder too, so the parser reads the whole of a call that wraps
    /// writing as one paragraph, whatever the writing holds.
    fn replace(&mut self, from: usize, to: usize, kind: TokenKind, every_line: bool) {
        self.out.push_str(&self.text[self.copied..from]);
        let token = self.tokens.len();
        self.tokens.push(Token {
            kind,
            span: Span::new(from, to),
        });
        let depth = self.depth(from);
        let first_piece = self.pieces.len();
        let mut line = from;
        loop {
            let newline = self.b[line..to]
                .iter()
                .position(|&c| c == b'\n')
                .map(|p| line + p);
            let end = match newline {
                Some(n) if n > line && self.b[n - 1] == b'\r' => n - 1,
                Some(n) => n,
                None => to,
            };
            if end > line || every_line {
                let masked = self.out.len();
                write_placeholder(&mut self.out, self.pieces.len());
                self.segments.push((masked, self.out.len(), line, end));
                self.pieces.push(Piece {
                    token,
                    span: Span::new(line, end),
                    first: self.pieces.len() == first_piece,
                    last: false,
                });
            }
            let Some(newline) = newline else { break };
            let next = call::past_prefix(self.b, newline + 1, depth).min(to);
            self.out.push_str(&self.text[end..next]);
            line = next;
        }
        if let Some(piece) = self.pieces[first_piece..].last_mut() {
            piece.last = true;
        }
        self.copied = to;
    }

    /// Masks the author's `c` at `at`, one of the characters placeholders
    /// are made of, as every other `c` is, so labels holding it still match.
    fn own(&mut self, at: usize, c: char) {
        let to = at + c.len_utf8();
        let Some(&(_, piece)) = self.own.iter().find(|(own, _)| *own == c) else {
            self.own.push((c, self.pieces.len()));
            self.replace(at, to, TokenKind::Literal, false);
            return;
        };
        self.out.push_str(&self.text[self.copied..at]);
        let masked = self.out.len();
        write_placeholder(&mut self.out, piece);
        self.segments.push((masked, self.out.len(), at, to));
        self.copied = to;
    }

    /// Leaves `from..to` out of the masked text.
    fn remove(&mut self, from: usize, to: usize) {
        self.out.push_str(&self.text[self.copied..from]);
        let masked = self.out.len();
        self.segments.push((masked, masked, from, to));
        self.copied = to;
    }

    /// The text of `range`, each later line without the indentation and
    /// quotation markers that put it where it is.
    fn without_prefixes(&self, range: std::ops::Range<usize>, depth: usize) -> String {
        let mut out = String::with_capacity(range.len());
        let mut line = range.start;
        loop {
            let newline = self.b[line..range.end]
                .iter()
                .position(|&c| c == b'\n')
                .map(|p| line + p);
            let Some(newline) = newline else {
                out.push_str(&self.text[line..range.end]);
                return out;
            };
            out.push_str(self.text[line..newline].trim_end_matches('\r'));
            out.push('\n');
            line = call::past_prefix(self.b, newline + 1, depth).min(range.end);
        }
    }
}

fn write_placeholder(out: &mut String, index: usize) {
    out.push(OPEN);
    let digits = usize::BITS - index.max(1).leading_zeros();
    let mut shift = digits.div_ceil(4) * 4;
    while shift > 0 {
        shift -= 4;
        let digit = (index >> shift) & 0xf;
        out.push(char::from_u32(DIGIT + digit as u32).expect("a private-use character"));
    }
    out.push(CLOSE);
}

/// Whether `c` is one of the characters placeholders are made of, which
/// the author's text must not be read as.
fn in_placeholders(c: char) -> bool {
    c == OPEN || c == CLOSE || (DIGIT..DIGIT + 16).contains(&u32::from(c))
}

/// The bytes that may start a call, maths, a colon opening a line, braces
/// ending a heading, a control character or a character placeholders are
/// made of: `0xC2` leads every C1 control in UTF-8, and `0xEE` every
/// placeholder character.
const SPECIAL: [bool; 256] = {
    let mut special = [false; 256];
    let mut byte = 0;
    while byte < 256 {
        special[byte] = matches!(
            byte as u8,
            b'@' | b'$' | b':' | b'{' | 0x00..=0x08 | 0x0b | 0x0c | 0x0e..=0x1f | 0x7f | 0xc2 | 0xee
        );
        byte += 1;
    }
    special
};

fn next_special(b: &[u8], from: usize) -> Option<usize> {
    b.get(from..)?
        .iter()
        .position(|&c| SPECIAL[usize::from(c)])
        .map(|p| p + from)
}

/// Whether the `{` at `at`, on the line `line` gives the start and end of,
/// opens braces that end a heading's line and aren't an id and classes,
/// which Markdown must then read as text, as a reader without heading
/// attributes does; and whether a backslash escapes it, which makes it text
/// whatever the braces hold.
pub(super) fn heading_brace(
    b: &[u8],
    start: usize,
    at: usize,
    (opens, end): (usize, usize),
) -> Option<bool> {
    let line = &b[at..end];
    let kept = line.len()
        - line
            .iter()
            .rev()
            .take_while(|c| matches!(c, b' ' | b'\t' | b'\r'))
            .count();
    let line = &line[..kept];
    if line.len() < 2 || line[line.len() - 1] != b'}' {
        return None;
    }
    let inside = &line[1..line.len() - 1];
    if inside
        .iter()
        .any(|c| matches!(c, b'{' | b'}' | b'<' | b'>' | b'\\'))
    {
        return None;
    }
    let escaped = at > start && b[at - 1] == b'\\' && !(at > start + 1 && b[at - 2] == b'\\');
    if !escaped && is_id_and_classes(inside) {
        return None;
    }
    heading_line(b, opens, at, end).then_some(escaped)
}

/// Whether braces holding `inside` give a heading or a table its id and
/// classes: `#id`, `.class`, or both, and nothing else.
pub(super) fn is_id_and_classes(inside: &[u8]) -> bool {
    let mut words = inside
        .split(u8::is_ascii_whitespace)
        .filter(|word| !word.is_empty())
        .peekable();
    words.peek().is_some() && words.all(|word| word.len() > 1 && matches!(word[0], b'#' | b'.'))
}

/// Whether the line from `line` to `end`, holding `at`, is a heading: one
/// opening with `#`s, or one a line of `=` or `-` underlines.
fn heading_line(b: &[u8], line: usize, at: usize, end: usize) -> bool {
    let opening = past_markers(b, line, at);
    let hashes = b[opening..at].iter().take_while(|&&c| c == b'#').count();
    if (1..=6).contains(&hashes) && matches!(b.get(opening + hashes), Some(b' ' | b'\t')) {
        return true;
    }
    let Some(next) = (end < b.len()).then_some(end + 1) else {
        return false;
    };
    let under = call::past_prefix(b, next, call::depth(b, line));
    let Some(&rule @ (b'=' | b'-')) = b.get(under) else {
        return false;
    };
    let run = b[under..].iter().take_while(|&&c| c == rule).count();
    call::blank_from(b, under + run)
}

/// Past the indentation, quotation markers and list markers opening the
/// line at `line`, short of `at`.
fn past_markers(b: &[u8], line: usize, at: usize) -> usize {
    let mut i = line;
    loop {
        while i < at && matches!(b[i], b' ' | b'\t' | b'>') {
            i += 1;
        }
        let digits = b[i..at].iter().take_while(|c| c.is_ascii_digit()).count();
        let marker = match b.get(i + digits) {
            Some(b'.' | b')') if digits > 0 => digits + 1,
            Some(b'-' | b'*' | b'+') if digits == 0 => 1,
            _ => return i,
        };
        if i + marker >= at || !matches!(b.get(i + marker), Some(b' ' | b'\t')) {
            return i;
        }
        i += marker;
    }
}

/// The line a scan moving forward is on, found once for the line: found
/// again at each brace or colon, one long line of them is read through as
/// many times as it has them.
#[derive(Default)]
pub(super) struct Line(Option<(usize, usize)>);

impl Line {
    /// Where the line holding `at` starts and where it ends.
    pub(super) fn of(&mut self, b: &[u8], at: usize) -> (usize, usize) {
        match self.0 {
            Some(line) if line.0 <= at && at <= line.1 => line,
            _ => {
                let end = b[at..]
                    .iter()
                    .position(|&c| c == b'\n')
                    .map_or(b.len(), |p| at + p);
                *self.0.insert((line_start(b, at), end))
            }
        }
    }
}

fn line_start(b: &[u8], at: usize) -> usize {
    b[..at]
        .iter()
        .rposition(|&c| c == b'\n')
        .map_or(0, |p| p + 1)
}

/// Whether the character before `at` makes it part of a word, an address
/// or an email rather than the start of a call or of maths.
fn glued(text: &str, at: usize) -> bool {
    text[..at].chars().next_back().is_some_and(|c| {
        c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | '@' | '\\' | '+')
    })
}

/// Dollar maths by pandoc's rule: the opening `$` has no space after it,
/// the closing one has no space before it and no digit after it. Maths on
/// one line holds no backtick or `](`, and no maths holds an HTML tag, so
/// it never runs into a code span, a link's address or raw HTML, which the
/// scan can't see. Maths over several lines
/// must be a block, `$$` opening its line and closing the last, and never
/// crosses a blank line.
///
/// `unclosed` is how far an earlier `$` was read on with no `$` to close
/// it: one before there has none either, so a line of them is read once.
fn maths(text: &str, at: usize, line: usize, unclosed: &mut usize) -> Option<(usize, bool)> {
    let b = text.as_bytes();
    if at > 0 && b[at - 1] == b'\\' {
        return None;
    }
    if b.get(at + 1) == Some(&b'$') {
        let depth = call::depth(b, line);
        let opens_line = past_markers(b, line, at) == at;
        let mut k = at + 2;
        while k + 1 < b.len() {
            if b[k] == b'$' && b[k + 1] == b'$' {
                let several = b[at..k].contains(&b'\n');
                let closes_line = b[k + 2..]
                    .iter()
                    .take_while(|&&c| c != b'\n')
                    .all(|c| matches!(c, b' ' | b'\t' | b'\r'));
                if several && !(opens_line && closes_line) {
                    return None;
                }
                return (k > at + 2 && !holds_a_tag(&b[at + 2..k])).then_some((k + 2, true));
            }
            if !b[at..k].contains(&b'\n')
                && (b[k] == b'`' || b[k] == b']' && b.get(k + 1) == Some(&b'('))
            {
                return None;
            }
            if b[k] == b'\n'
                && (!opens_line || call::blank_from(b, call::past_prefix(b, k + 1, depth)))
            {
                return None;
            }
            k += 1;
        }
        return None;
    }
    if b.get(at + 1).is_none_or(u8::is_ascii_whitespace) {
        return None;
    }
    if glued(text, at) || at < *unclosed {
        return None;
    }
    let mut k = at + 1;
    while k < b.len() {
        match b[k] {
            b'`' | b'\n' => break,
            b']' if b.get(k + 1) == Some(&b'(') => break,
            b'\\' if b.get(k + 1) != Some(&b'\n') => {
                k += 2;
                continue;
            }
            b'$' if !b[k - 1].is_ascii_whitespace()
                && !b.get(k + 1).is_some_and(u8::is_ascii_digit) =>
            {
                return (!holds_a_tag(&b[at + 1..k])).then_some((k + 1, false));
            }
            _ => {}
        }
        k += 1;
    }
    *unclosed = k;
    None
}

/// Whether maths would hold an HTML tag, which is the page's markup, not
/// the maths: `$x</code> and <code>$y` is two stretches of code.
fn holds_a_tag(inner: &[u8]) -> bool {
    inner.windows(2).enumerate().any(|(i, pair)| {
        pair[0] == b'<'
            && (pair[1] == b'/' || pair[1].is_ascii_alphabetic())
            && inner[i..].contains(&b'>')
    })
}

/// Tabs after a closing code fence, which CommonMark allows and the parser
/// does not, become spaces: the same length, so no offset moves.
fn fence_tabs(text: &str, from: usize) -> Cow<'_, str> {
    fn fence_with_tab(line: &str) -> bool {
        let line = line.trim_end_matches(['\n', '\r']);
        let rest = line.trim_start_matches(' ');
        let Some(fence) = rest.chars().next().filter(|c| matches!(c, '`' | '~')) else {
            return false;
        };
        let run = rest.len() - rest.trim_start_matches(fence).len();
        let after = &rest[run..];
        line.len() - rest.len() <= 3
            && run >= 3
            && after.contains('\t')
            && after.chars().all(|c| c == ' ' || c == '\t')
    }
    let body = &text[from..];
    if !body.contains('\t') || !body.split_inclusive('\n').any(fence_with_tab) {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len());
    out.push_str(&text[..from]);
    for line in body.split_inclusive('\n') {
        if fence_with_tab(line) {
            out.push_str(&line.replace('\t', " "));
        } else {
            out.push_str(line);
        }
    }
    Cow::Owned(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every text of up to `length` of `letters`.
    fn every(letters: &[char], length: usize) -> Vec<String> {
        let mut all = vec![String::new()];
        let mut last = all.clone();
        for _ in 0..length {
            last = last
                .iter()
                .flat_map(|text| letters.iter().map(move |c| format!("{text}{c}")))
                .collect();
            all.extend(last.iter().cloned());
        }
        all
    }

    /// Remembering how far an unclosed `$` was read on changes nothing:
    /// each `$` of a line, asked in turn, is the maths it is asked alone.
    #[test]
    fn a_dollar_is_the_same_maths_after_one_that_never_closed() {
        let letters = ['$', 'a', ' ', '1', '\\', '`', ']', '(', '<', '>', '\n'];
        for text in every(&letters, 6) {
            let b = text.as_bytes();
            let mut unclosed = 0;
            for at in (0..b.len()).filter(|at| b[*at] == b'$') {
                let line = line_start(b, at);
                assert_eq!(
                    maths(&text, at, line, &mut unclosed),
                    maths(&text, at, line, &mut 0),
                    "{text:?} at {at}"
                );
            }
        }
    }
}
