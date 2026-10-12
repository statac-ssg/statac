//! The one place granit-parser is used. Its events become this module's,
//! with the spans it gets wrong put right, and its errors become what the
//! author would recognise, placed where the author made them.

use std::ops::Range;

use granit_parser::{ErrorKind, Event as Granit, Parser, ScalarStyle, ScanError, StrInput};

use super::Malformed;
use super::tree::Tag;
use crate::origin::Style;

/// Spans are byte ranges in the text handed to `Events::new`.
pub(super) enum Event {
    DocumentStart(Range<usize>),
    Scalar {
        text: String,
        style: Style,
        anchor: usize,
        tag: Option<Tag>,
        span: Range<usize>,
    },
    Alias {
        anchor: usize,
        span: Range<usize>,
    },
    SequenceStart {
        anchor: usize,
        tag: Option<Tag>,
        start: usize,
    },
    MappingStart {
        anchor: usize,
        tag: Option<Tag>,
        start: usize,
    },
    /// Closes the innermost list or mapping, which ends at `end`.
    End {
        end: usize,
    },
}

pub(super) enum Fault {
    Malformed(Malformed, Range<usize>),
    /// A tag outside the core schema, as written.
    UnknownTag(String, Range<usize>),
}

pub(super) struct Events<'a> {
    text: &'a str,
    parser: Parser<'a, StrInput<'a>>,
    /// Where the last node ended. A block scalar's `|` or `>` is the first
    /// one after it, and so is the opening of an unclosed quote.
    last_end: usize,
    /// Each open collection: whether it is flow, and where it starts.
    open: Vec<(bool, usize)>,
    failed: bool,
}

impl<'a> Events<'a> {
    pub(super) fn new(text: &'a str) -> Self {
        let options = granit_parser::options! { emit_comments: false };
        Events {
            text,
            parser: Parser::new_from_str_with_options(text, options),
            last_end: 0,
            open: Vec::new(),
            failed: false,
        }
    }

    fn convert(
        &mut self,
        event: Granit<'_>,
        span: granit_parser::Span,
    ) -> Result<Option<Event>, Fault> {
        let bytes = span.byte_range().unwrap_or(0..0);
        let event = match event {
            Granit::DocumentStart(..) => Event::DocumentStart(bytes),
            Granit::Scalar(value, style, anchor, tag) => {
                let tag = self.tag(tag.as_deref(), &span)?;
                let (span, style) = match style {
                    ScalarStyle::SingleQuoted => (bytes, Style::SingleQuoted),
                    ScalarStyle::DoubleQuoted => (bytes, Style::DoubleQuoted),
                    ScalarStyle::Literal => (self.block(bytes), Style::Literal),
                    ScalarStyle::Folded => (self.block(bytes), Style::Folded),
                    _ => (bytes, Style::Plain),
                };
                // The parser reads a node written as nothing as `~`.
                let text = if span.is_empty() && style == Style::Plain {
                    String::new()
                } else {
                    value.into_owned()
                };
                self.last_end = span.end;
                Event::Scalar {
                    text,
                    style,
                    anchor,
                    tag,
                    span,
                }
            }
            Granit::Alias(anchor) => {
                self.last_end = bytes.end;
                Event::Alias {
                    anchor,
                    span: bytes,
                }
            }
            Granit::SequenceStart(style, anchor, tag) => {
                let tag = self.tag(tag.as_deref(), &span)?;
                let flow = style == granit_parser::StructureStyle::Flow;
                self.open.push((flow, bytes.start));
                Event::SequenceStart {
                    anchor,
                    tag,
                    start: bytes.start,
                }
            }
            Granit::MappingStart(style, anchor, tag) => {
                let tag = self.tag(tag.as_deref(), &span)?;
                let flow = style == granit_parser::StructureStyle::Flow;
                self.open.push((flow, bytes.start));
                Event::MappingStart {
                    anchor,
                    tag,
                    start: bytes.start,
                }
            }
            Granit::SequenceEnd | Granit::MappingEnd => {
                let (flow, start) = self.open.pop().expect("the parser balances its events");
                // A flow collection ends at its bracket, a block one at its last value.
                let end = if flow {
                    bytes.end
                } else {
                    self.last_end.max(start)
                };
                self.last_end = end;
                Event::End { end }
            }
            _ => return Ok(None),
        };
        Ok(Some(event))
    }

    fn tag(
        &self,
        tag: Option<&granit_parser::Tag>,
        span: &granit_parser::Span,
    ) -> Result<Option<Tag>, Fault> {
        let Some(tag) = tag else {
            return Ok(None);
        };
        let core = match tag.core_suffix() {
            Some("str") => Tag::Str,
            Some("int") => Tag::Int,
            Some("float") => Tag::Float,
            Some("bool") => Tag::Bool,
            Some("null") => Tag::Null,
            Some("seq") => Tag::Seq,
            Some("map") => Tag::Map,
            _ if tag
                .suffix_in_namespace("tag:yaml.org,2002:")
                .is_some_and(|suffix| suffix == "binary") =>
            {
                Tag::Binary
            }
            _ => {
                let start = span
                    .tag_start()
                    .and_then(|m| m.byte_offset())
                    .or_else(|| span.byte_range().map(|r| r.start))
                    .unwrap_or(0);
                let rest = &self.text[start..];
                let range = match rest.strip_prefix("!<").and_then(|r| r.find('>')) {
                    Some(end) => start..start + end + 3,
                    None => self.word(start),
                };
                let written = match &self.text[range.clone()] {
                    "" => tag.original(),
                    written => written.to_owned(),
                };
                return Err(Fault::UnknownTag(written, range));
            }
        };
        Ok(Some(core))
    }

    /// A block scalar's bytes: from its `|` or `>` to the end of its last
    /// line with words on it. The parser's span starts at the first line of
    /// content and runs on over the line breaks and blank lines after it.
    fn block(&self, content: Range<usize>) -> Range<usize> {
        let source = self.text.as_bytes();
        let mut start = content.start;
        let mut i = self.last_end.min(content.start);
        let mut line_start = true;
        while i < content.start {
            match source[i] {
                b'#' if line_start || source[i - 1].is_ascii_whitespace() => {
                    while i < content.start && source[i] != b'\n' {
                        i += 1;
                    }
                    continue;
                }
                b'|' | b'>'
                    if i == self.last_end
                        || source[i - 1].is_ascii_whitespace()
                        || source[i - 1] == b':' =>
                {
                    start = i;
                    break;
                }
                b'\n' => line_start = true,
                b' ' | b'\t' | b'\r' => {}
                _ => line_start = false,
            }
            i += 1;
        }
        let written = &self.text[content.clone()];
        let end = match written.rfind(|c: char| !c.is_whitespace()) {
            Some(at) => {
                let after = &written[at..];
                content.start + at + after.find(['\r', '\n']).unwrap_or(after.len())
            }
            // No words: the block is its header, `|` and any indicators after it.
            None => {
                let header = &self.text[start + 1..];
                start + 1 + header.len()
                    - header
                        .trim_start_matches(|c: char| c.is_ascii_digit() || c == '+' || c == '-')
                        .len()
            }
        };
        start..end
    }

    fn fault(&self, error: &ScanError) -> Fault {
        let at = error
            .marker()
            .byte_offset()
            .filter(|&at| self.text.is_char_boundary(at))
            .unwrap_or(self.text.len());
        let one = || {
            at..self.text[at..]
                .chars()
                .next()
                .map_or(at, |c| at + c.len_utf8())
        };
        let (found, range) = match error.kind() {
            ErrorKind::InvalidQuotedScalarIndent => match self.opening_quote(at) {
                Some((quote, start)) => match self.shallow_line(quote, start, at) {
                    Some(spaces) => {
                        let back = at - start;
                        let found = Malformed::ShallowLine {
                            quote,
                            back,
                            spaces,
                        };
                        (found, one())
                    }
                    None => (Malformed::UnclosedQuote(quote), start..start + 1),
                },
                None => (Malformed::Indentation, one()),
            },
            ErrorKind::UnclosedQuotedScalar | ErrorKind::DocumentIndicatorInQuotedScalar => {
                match self.opening_quote(at) {
                    Some((quote, start)) => (Malformed::UnclosedQuote(quote), start..start + 1),
                    None => (Malformed::Indentation, one()),
                }
            }
            ErrorKind::UnknownQuotedScalarEscape
            | ErrorKind::InvalidQuotedScalarHexEscape
            | ErrorKind::InvalidLowSurrogateHexEscape
            | ErrorKind::InvalidLowSurrogate
            | ErrorKind::MissingLowSurrogate
            | ErrorKind::UnpairedLowSurrogate
            | ErrorKind::InvalidUnicodeEscape => match self.bad_escape(at) {
                Some(range) => (
                    Malformed::UnknownEscape(self.text[range.clone()].to_owned()),
                    range,
                ),
                None => (self.unexpected(at), one()),
            },
            ErrorKind::UnclosedFlowCollection { open } => {
                (Malformed::UnclosedBracket(*open), one())
            }
            ErrorKind::UnexpectedEofFlowSequence => (Malformed::UnclosedBracket('['), one()),
            ErrorKind::UnexpectedEofFlowMapping | ErrorKind::UnexpectedEofImplicitFlowMapping => {
                (Malformed::UnclosedBracket('{'), one())
            }
            ErrorKind::MismatchedFlowCollectionEnd { open, close } => (
                Malformed::MismatchedBracket {
                    open: *open,
                    close: *close,
                },
                one(),
            ),
            ErrorKind::InvalidTrailingSingleQuotedScalar
            | ErrorKind::InvalidTrailingDoubleQuotedScalar => (Malformed::TextAfterQuote, one()),
            ErrorKind::InvalidIndentation
            | ErrorKind::InvalidBlockScalarIndent
            | ErrorKind::InvalidFlowScalarIndent => (Malformed::Indentation, one()),
            ErrorKind::TabNotAllowed
            | ErrorKind::TabInBlockIndentation
            | ErrorKind::TabInIndentation
            | ErrorKind::TabAtBlockScalarStart
            | ErrorKind::TabInPlainScalar => (Malformed::Tab, self.tab(at).unwrap_or_else(one)),
            ErrorKind::UnknownAnchor => {
                let range = self.word(at);
                (
                    Malformed::UnknownAnchor(self.text[range.clone()].to_owned()),
                    range,
                )
            }
            ErrorKind::RecursionLimitExceeded => (Malformed::TooDeep, one()),
            _ if at >= self.text.len() => (Malformed::EndsEarly, at..at),
            _ => match self.forbidden(at) {
                Some((c, range)) => (Malformed::ControlCharacter(c), range),
                None => {
                    let range = self.token(at);
                    (
                        Malformed::Unexpected(self.text[range.clone()].to_owned()),
                        range,
                    )
                }
            },
        };
        Fault::Malformed(found, range)
    }

    fn unexpected(&self, at: usize) -> Malformed {
        if at >= self.text.len() {
            Malformed::EndsEarly
        } else {
            Malformed::Unexpected(self.text[self.token(at)].to_owned())
        }
    }

    /// The tab the parser stopped for: the nearest at or before where it
    /// stopped on that line, or else the first after.
    fn tab(&self, at: usize) -> Option<Range<usize>> {
        let line_start = self.text[..at].rfind('\n').map_or(0, |i| i + 1);
        let through = (at + 1).min(self.text.len());
        let found = self.text[line_start..through]
            .rfind('\t')
            .map(|i| line_start + i)
            .or_else(|| self.text[at..].find('\t').map(|i| at + i))?;
        Some(found..found + 1)
    }

    /// The first quote from the end of the last node to where the parser
    /// stopped, outside comments.
    fn opening_quote(&self, at: usize) -> Option<(char, usize)> {
        let bytes = self.text.as_bytes();
        let until = (at + 1).min(bytes.len());
        let mut i = self.last_end;
        while i < until {
            match bytes[i] {
                b'#' if i == 0 || bytes[i - 1].is_ascii_whitespace() => {
                    while i < until && bytes[i] != b'\n' {
                        i += 1;
                    }
                    continue;
                }
                b'\'' => return Some(('\'', i)),
                b'"' => return Some(('"', i)),
                _ => {}
            }
            i += 1;
        }
        None
    }

    /// Where the parser stopped at a line, at `at`, of the text in quotes
    /// that opens with `quote` at `opens`: the spaces before each later
    /// line that are enough for it to be taken as more of the text. None
    /// where the text is more likely never closed: no quote closes it
    /// after the line, or a line no deeper than the one it opens on reads
    /// as a field or an item.
    fn shallow_line(&self, quote: char, opens: usize, at: usize) -> Option<usize> {
        let closes = closing_quote(self.text, quote, opens).filter(|closes| *closes >= at)?;
        let first = self.text[..opens].rfind('\n').map_or(0, |n| n + 1);
        // The line's own depth, with the `- ` of each list it opens.
        let depth = self.text.as_bytes()[first..opens]
            .iter()
            .take_while(|c| matches!(c, b' ' | b'-'))
            .count();
        for line in self.text[opens..closes].split('\n').skip(1) {
            let written = line.trim_start_matches(' ');
            let spaces = line.len() - written.len();
            if written.starts_with('\t') {
                return None;
            }
            let written = written.trim_end_matches('\r');
            if !written.is_empty() && spaces <= depth && !super::reads_as_text(written, spaces) {
                return None;
            }
        }
        // YAML asks a later line to be deeper than the field the text is
        // the value of, which stands no deeper than this, and as deep as
        // the `-` of an item that is the text alone.
        let before = self.text[first..opens].trim_end_matches(' ');
        Some(match before.ends_with('-') && before.len() <= depth {
            true => before.len() - 1,
            false => depth + 1,
        })
    }

    /// The first escape YAML doesn't define in the double-quoted scalar
    /// that opens at `quote`.
    fn bad_escape(&self, quote: usize) -> Option<Range<usize>> {
        let rest = self.text.get(quote + 1..)?;
        let mut chars = rest.char_indices().peekable();
        while let Some((i, c)) = chars.next() {
            match c {
                '"' => return None,
                '\\' => {}
                _ => continue,
            }
            let start = quote + 1 + i;
            let Some((_, escape)) = chars.next() else {
                return Some(start..start + 1);
            };
            let digits = match escape {
                '0' | 'a' | 'b' | 't' | '\t' | 'n' | 'v' | 'f' | 'r' | 'e' | ' ' | '"' | '/'
                | '\\' | 'N' | '_' | 'L' | 'P' | '\n' | '\r' => continue,
                'x' => 2,
                'u' => 4,
                'U' => 8,
                _ => return Some(start..start + 1 + escape.len_utf8()),
            };
            let hex: String = (0..digits)
                .map_while(|_| {
                    chars
                        .next_if(|(_, c)| c.is_ascii_hexdigit())
                        .map(|(_, c)| c)
                })
                .collect();
            let end = start + 2 + hex.len();
            let code = u32::from_str_radix(&hex, 16).ok();
            if hex.len() < digits {
                return Some(start..end);
            }
            match code {
                Some(0xD800..=0xDBFF) => {
                    // A high surrogate needs a `\u` low surrogate straight after it.
                    let low = self
                        .text
                        .get(end..end + 6)
                        .and_then(|s| s.strip_prefix("\\u"));
                    match low.and_then(|h| u32::from_str_radix(h, 16).ok()) {
                        Some(0xDC00..=0xDFFF) => {
                            for _ in 0..6 {
                                chars.next();
                            }
                        }
                        _ => return Some(start..end),
                    }
                }
                Some(code) if char::from_u32(code).is_some() => {}
                _ => return Some(start..end),
            }
        }
        None
    }

    /// A run of text up to whitespace or a flow indicator: an alias, an
    /// anchor or a tag as written.
    fn word(&self, at: usize) -> Range<usize> {
        let rest = &self.text[at..];
        at..at
            + rest
                .find(|c: char| c.is_whitespace() || ",[]{}".contains(c))
                .unwrap_or(rest.len())
    }

    /// The text at `at` as the author would see it: one indicator, or a word.
    /// The first character YAML allows nowhere on the line holding `at`.
    fn forbidden(&self, at: usize) -> Option<(char, Range<usize>)> {
        let start = self.text[..at].rfind('\n').map_or(0, |i| i + 1);
        let end = self.text[at..]
            .find('\n')
            .map_or(self.text.len(), |i| at + i);
        self.text[start..end]
            .char_indices()
            .find(|&(_, c)| {
                matches!(
                    c,
                    '\u{0}'..='\u{8}'
                        | '\u{b}'
                        | '\u{c}'
                        | '\u{e}'..='\u{1f}'
                        | '\u{7f}'..='\u{84}'
                        | '\u{86}'..='\u{9f}'
                        | '\u{fffe}'
                        | '\u{ffff}'
                )
            })
            .map(|(i, c)| (c, start + i..start + i + c.len_utf8()))
    }

    fn token(&self, at: usize) -> Range<usize> {
        match self.text[at..].chars().next() {
            Some(c) if !c.is_alphanumeric() => at..at + c.len_utf8(),
            _ => {
                let rest = &self.text[at..];
                at..at
                    + rest
                        .find(|c: char| c.is_whitespace() || ",[]{}:".contains(c))
                        .unwrap_or(rest.len())
            }
        }
    }
}

/// The quote that closes the text in quotes opening with `quote` at
/// `opens`: in single quotes `''` is a quote written, and in double
/// quotes a backslash takes the next character with it.
fn closing_quote(text: &str, quote: char, opens: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut at = opens + 1;
    loop {
        match (quote, *bytes.get(at)?) {
            ('"', b'\\') => at += 1,
            ('\'', b'\'') if bytes.get(at + 1) == Some(&b'\'') => at += 1,
            (_, c) if c == quote as u8 => return Some(at),
            _ => {}
        }
        at += 1;
    }
}

/// Where each line starts that stands no deeper than the line at `line`,
/// from it to the end of the text in quotes that opens with `quote` at
/// `opens`, lines with nothing on them aside: the line itself first.
pub(super) fn shallow_lines(text: &str, quote: char, opens: usize, line: usize) -> Vec<usize> {
    let Some(closes) = closing_quote(text, quote, opens) else {
        return Vec::new();
    };
    let start = text[..line].rfind('\n').map_or(0, |n| n + 1);
    let depth = line - start;
    let mut lines = Vec::new();
    let mut at = start;
    for written in text[start..=closes.max(start)].split_inclusive('\n') {
        let spaces = written.len() - written.trim_start_matches(' ').len();
        if !written.trim().is_empty() && spaces <= depth {
            lines.push(at);
        }
        at += written.len();
    }
    lines
}

impl Iterator for Events<'_> {
    type Item = Result<Event, Fault>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.failed {
            return None;
        }
        loop {
            let converted = match self.parser.next()? {
                Ok((event, span)) => self.convert(event, span),
                Err(error) => Err(self.fault(&error)),
            };
            match converted {
                Ok(None) => continue,
                Ok(Some(event)) => return Some(Ok(event)),
                Err(fault) => {
                    self.failed = true;
                    return Some(Err(fault));
                }
            }
        }
    }
}
