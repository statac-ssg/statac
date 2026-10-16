//! A template the engine can't read. The engine says where its reader
//! stopped; the template is read again here, as the engine reads it, for
//! the tags and blocks open there.

use std::ops::Range;

use minijinja::machinery::{Token, tokenize};
use minijinja::syntax::SyntaxConfig;
use minijinja::{Error, ErrorKind};

use super::super::check::WHITESPACE;
use super::{line_of, within};
use crate::problem::{Awaited, Opener, Rewrite, SyntaxFault};
use crate::suggest::nearest;

/// Every tag a template may write, for the nearest to one it can't.
const TAGS: &[&str] = &[
    "if",
    "elif",
    "else",
    "endif",
    "for",
    "endfor",
    "block",
    "endblock",
    "extends",
    "set",
    "endset",
    "macro",
    "endmacro",
    "call",
    "endcall",
    "filter",
    "endfilter",
    "with",
    "endwith",
    "autoescape",
    "endautoescape",
    "raw",
    "endraw",
    "break",
    "continue",
    "do",
];

/// The tags that open a block, each closed by `end` and its keyword.
const BLOCKS: &[&str] = &[
    "if",
    "for",
    "block",
    "set",
    "macro",
    "filter",
    "with",
    "autoescape",
    "call",
    "raw",
];

/// What the engine couldn't read in `text`, and where.
pub(in crate::templates) fn read_syntax(text: &str, error: &Error) -> (SyntaxFault, Range<usize>) {
    let at = within(text, error.range());
    let detail = error
        .detail()
        .map_or_else(|| error.kind().to_string(), str::to_owned);
    let reading = Reading::new(text, at.clone());
    if let Some(stop) = &reading.stopped {
        // The engine's reader may take a character it can't read for the
        // template's end.
        let same = stop.kind() == error.kind() && stop.detail() == error.detail();
        if (same || detail.starts_with("unexpected end of input"))
            && let Some(read) = reading.unreadable(stop)
        {
            return read;
        }
    }
    reading
        .read(&detail)
        .unwrap_or((SyntaxFault::Engine(detail), at))
}

/// A `{{` or a `{%` and what closes it, by its tokens.
struct Tag {
    /// Its opening token's index.
    open: usize,
    /// Its closing token's index, where it has one.
    close: Option<usize>,
}

/// A block open where the engine stopped.
struct Block<'a> {
    keyword: &'a str,
    /// From the opening tag's `{%` to its `%}`.
    tag: Range<usize>,
    /// The index of the opening tag's `{%`.
    open: usize,
    has_else: bool,
}

struct Reading<'a> {
    text: &'a str,
    at: Range<usize>,
    tokens: Vec<(Token<'a>, Range<usize>)>,
    /// What stopped the engine's lexer short of the end, where it did.
    stopped: Option<Error>,
    tags: Vec<Tag>,
    /// The blocks open at `at`, outermost first.
    open: Vec<Block<'a>>,
}

impl<'a> Reading<'a> {
    // `SyntaxConfig` is a unit struct unless the engine's `custom_syntax`
    // feature is on.
    #[allow(clippy::default_constructed_unit_structs)]
    fn new(text: &'a str, at: Range<usize>) -> Reading<'a> {
        let mut tokens = Vec::new();
        let mut stopped = None;
        let lexed = tokenize(text, false, SyntaxConfig::default(), WHITESPACE);
        for token in lexed {
            match token {
                Ok((token, span)) => {
                    tokens.push((token, span.start_offset as usize..span.end_offset as usize));
                }
                Err(error) => {
                    stopped = Some(error);
                    break;
                }
            }
        }
        let mut tags: Vec<Tag> = Vec::new();
        for (i, (token, _)) in tokens.iter().enumerate() {
            match token {
                Token::BlockStart | Token::VariableStart => tags.push(Tag {
                    open: i,
                    close: None,
                }),
                Token::BlockEnd | Token::VariableEnd => {
                    if let Some(tag) = tags.last_mut().filter(|t| t.close.is_none()) {
                        tag.close = Some(i);
                    }
                }
                _ => {}
            }
        }
        let mut reading = Reading {
            text,
            at,
            tokens,
            stopped,
            tags,
            open: Vec::new(),
        };
        reading.open = reading.blocks_open(reading.at.start);
        reading
    }

    /// The blocks open at `to`, by the tags closed before it.
    fn blocks_open(&self, to: usize) -> Vec<Block<'a>> {
        let mut open: Vec<Block<'a>> = Vec::new();
        for tag in &self.tags {
            let Some(close) = tag.close else { continue };
            if self.tokens[close].1.end > to
                || !matches!(self.tokens[tag.open].0, Token::BlockStart)
            {
                continue;
            }
            let Some(&(Token::Ident(keyword), _)) = self.tokens.get(tag.open + 1) else {
                continue;
            };
            let range = self.tokens[tag.open].1.start..self.tokens[close].1.end;
            match keyword {
                "set" if self.assigns(tag.open + 2, close) => {}
                "else" => {
                    if let Some(block) = open.last_mut() {
                        block.has_else = true;
                    }
                }
                _ if BLOCKS.contains(&keyword) => open.push(Block {
                    keyword,
                    tag: range,
                    open: tag.open,
                    has_else: false,
                }),
                _ => {
                    if let Some(ended) = keyword.strip_prefix("end")
                        && open.last().is_some_and(|b| b.keyword == ended)
                    {
                        open.pop();
                    }
                }
            }
        }
        open
    }

    /// Whether the tokens from `from` to `to` set a value with `=`, rather
    /// than open a block.
    fn assigns(&self, from: usize, to: usize) -> bool {
        let mut depth = 0i32;
        for (token, _) in &self.tokens[from..to] {
            match token {
                Token::ParenOpen | Token::BracketOpen | Token::BraceOpen => depth += 1,
                Token::ParenClose | Token::BracketClose | Token::BraceClose => depth -= 1,
                Token::Assign if depth == 0 => return true,
                _ => {}
            }
        }
        false
    }

    /// Where the last token before the lexer stopped ends.
    fn lexed_to(&self) -> usize {
        self.tokens.last().map_or(0, |(_, range)| range.end)
    }

    /// The first offset from `from` that isn't whitespace.
    fn after_spaces(&self, from: usize) -> usize {
        let rest = &self.text[from..];
        from + rest.len() - rest.trim_start().len()
    }

    /// What stopped the lexer, read from the template.
    fn unreadable(&self, stop: &Error) -> Option<(SyntaxFault, Range<usize>)> {
        let from = self.lexed_to();
        let next = self.after_spaces(from);
        let stopped_at = within(self.text, stop.range());
        if stop.kind() == ErrorKind::BadEscape {
            let (written, range) = bad_escape(self.text, next)?;
            return Some((SyntaxFault::Escape(written), range));
        }
        match stop.detail()? {
            "unexpected end of string" => {
                let quote = self.text[next..].chars().next()?;
                Some((
                    SyntaxFault::Unclosed(Opener::Quote(quote)),
                    next..next + quote.len_utf8(),
                ))
            }
            "unexpected end of comment" => {
                let open = from + self.text[from..].find("{#")?;
                Some((SyntaxFault::Unclosed(Opener::Comment), open..open + 2))
            }
            "unexpected end of raw block" => {
                let open = from + self.text[from..].find("{%")?;
                let end = self.text[open..]
                    .find("%}")
                    .map_or(open + 2, |e| open + e + 2);
                Some((
                    SyntaxFault::Unclosed(Opener::Block("raw".into())),
                    open..end,
                ))
            }
            "unexpected character" => {
                let start = stopped_at.start;
                let first = self.text[start..].chars().next()?;
                let len = self.text[start..]
                    .chars()
                    .take_while(|&c| c == first)
                    .map(char::len_utf8)
                    .sum::<usize>();
                let range = start..start + len;
                Some((SyntaxFault::Foreign(self.text[range.clone()].into()), range))
            }
            "'_' may not occur at end of number" | "invalid float" => {
                let range = next..stopped_at.start.max(next);
                Some((
                    SyntaxFault::NotANumber(self.text[range.clone()].into()),
                    range,
                ))
            }
            "invalid integer (too large)" => {
                let range = next..stopped_at.start.max(next);
                let written = &self.text[range.clone()];
                let fault = match is_digits(written) {
                    true => SyntaxFault::TooLarge(written.into()),
                    false => SyntaxFault::NotANumber(written.into()),
                };
                Some((fault, range))
            }
            _ => None,
        }
    }

    /// The index of the token the engine stopped at.
    fn found(&self) -> Option<usize> {
        self.tokens
            .iter()
            .position(|(_, range)| range.start == self.at.start && !range.is_empty())
    }

    /// The tag `index` is in: its opening token's index.
    fn tag_of(&self, index: usize) -> Option<&Tag> {
        self.tags.iter().rev().find(|tag| tag.open <= index)
    }

    /// The keyword of the block tag holding token `index`.
    fn keyword_of(&self, index: usize) -> Option<&'a str> {
        let tag = self.tag_of(index)?;
        match self.tokens.get(tag.open + 1) {
            Some(&(Token::Ident(keyword), _)) if tag.open + 1 < index => Some(keyword),
            _ => None,
        }
    }

    /// The whole of the tag holding token `index`, or its opening where it
    /// is never closed.
    fn whole_tag(&self, index: usize) -> Range<usize> {
        match self.tag_of(index) {
            Some(tag) => {
                let start = self.tokens[tag.open].1.start;
                let end = tag
                    .close
                    .map_or(self.tokens[tag.open].1.end, |c| self.tokens[c].1.end);
                start..end
            }
            None => self.at.clone(),
        }
    }

    /// The tag around the place the engine stopped, where it is never
    /// closed and that is what stopped it: the engine read on to the end,
    /// onto a later line, or into the next tag.
    fn unclosed_tag(&self, at_end: bool) -> Option<(SyntaxFault, Range<usize>)> {
        let tag = self
            .tags
            .iter()
            .rev()
            .find(|tag| self.tokens[tag.open].1.start <= self.at.start)?;
        if tag.close.is_some() {
            return None;
        }
        let (token, opener) = &self.tokens[tag.open];
        let later_line = line_of(self.text, self.at.start) > line_of(self.text, opener.start);
        let next_tag = self.text[self.at.start..].starts_with('{')
            && self.text[self.at.start + 1..].starts_with(['{', '%', '#']);
        if !(at_end || later_line || next_tag) {
            return None;
        }
        let fault = match token {
            Token::VariableStart => Opener::Output,
            _ => Opener::Tag,
        };
        Some((SyntaxFault::Unclosed(fault), opener.clone()))
    }

    /// What the engine stopped at, by its words.
    fn read(&self, detail: &str) -> Option<(SyntaxFault, Range<usize>)> {
        let at = self.at.clone();
        if let Some(awaited) = detail.strip_prefix("unexpected end of input, expected ") {
            return Some(self.at_end(awaited));
        }
        if let Some(name) = detail.strip_prefix("unknown statement ") {
            return Some(self.unknown_statement(name));
        }
        if let Some(rest) = detail.strip_prefix("unknown ")
            && rest.ends_with(", expected statement")
        {
            let index = self.found()?;
            if matches!(self.tokens[index].0, Token::BlockEnd)
                && index > 0
                && matches!(self.tokens[index - 1].0, Token::BlockStart)
            {
                return Some((SyntaxFault::EmptyTag, self.whole_tag(index)));
            }
            return Some(self.unexpected(index, Awaited::TagName));
        }
        if let Some(rest) = detail.strip_prefix("unexpected ") {
            let awaited = match rest.split_once(", expected ") {
                Some((_, awaited)) => awaited_of(awaited),
                None => Awaited::Value,
            };
            let index = self.found()?;
            return Some(self.unexpected(index, awaited));
        }
        let fault = match detail {
            "template exceeds maximum recursion limits" => SyntaxFault::TooDeep,
            "non-keyword arg after keyword arg" => SyntaxFault::NamedFirst,
            "'break' must be placed inside a loop" | "'continue' must be placed inside a loop" => {
                let index = self.found()?;
                let keyword = self.text[at].to_owned();
                return Some((SyntaxFault::OutsideLoop(keyword), self.whole_tag(index)));
            }
            "block tags in macros are not allowed" => {
                let within = self
                    .open
                    .iter()
                    .rev()
                    .find(|b| matches!(b.keyword, "macro" | "call"))
                    .map_or("macro", |b| b.keyword);
                SyntaxFault::BlockInMacro(within.into())
            }
            "expected a filter" => SyntaxFault::FilterUnnamed,
            "Required blocks can only contain comments or whitespace" => {
                let block = self.open.last()?;
                let close = self.tags.iter().find(|t| t.open == block.open)?.close?;
                let required = (block.open..close)
                    .find(|&i| matches!(self.tokens[i].0, Token::Ident("required")))?;
                return Some((SyntaxFault::RequiredHolds, self.tokens[required].1.clone()));
            }
            _ if detail.starts_with("cannot assign to reserved variable name ") => {
                SyntaxFault::Kept(self.text[at.clone()].into())
            }
            _ if detail.starts_with("block '") && detail.ends_with("' defined twice") => {
                let index = self.found()?;
                let tag = self.tag_of(index)?;
                let (_, name) = self.tokens.get(tag.open + 2)?;
                return Some((
                    SyntaxFault::BlockTwice(self.text[name.clone()].into()),
                    name.clone(),
                ));
            }
            _ if detail.starts_with("mismatching name on block.") => {
                let index = self.found()?;
                let (_, written) = self.tokens.get(index + 1)?;
                let block = self.open.last()?;
                let (_, name) = self.tokens.get(block.open + 2)?;
                return Some((
                    SyntaxFault::EndNames {
                        written: self.text[written.clone()].into(),
                        block: self.text[name.clone()].into(),
                    },
                    written.clone(),
                ));
            }
            _ if detail.starts_with("expected call expression in call block") => {
                let tag = self.tags.iter().rev().find(|t| {
                    self.tokens[t.open].1.start <= at.start
                        && matches!(self.tokens[t.open].0, Token::BlockStart)
                })?;
                let (_, keyword) = self.tokens.get(tag.open + 1)?;
                let start = self.after_spaces(keyword.end);
                return Some((
                    SyntaxFault::NoCall(self.text[keyword.clone()].into()),
                    start..at.end.max(start),
                ));
            }
            _ => return None,
        };
        Some((fault, at))
    }

    /// The template ended where `awaited` was due: a tag or a block never
    /// closed, if one is open.
    fn at_end(&self, awaited: &str) -> (SyntaxFault, Range<usize>) {
        if let Some(read) = self.unclosed_tag(true) {
            return read;
        }
        // The engine places the end on the template's last token, which may
        // be the end of the tag opening the block.
        if let Some(block) = self.blocks_open(self.text.len()).last() {
            return (
                SyntaxFault::Unclosed(Opener::Block(block.keyword.into())),
                block.tag.clone(),
            );
        }
        (
            SyntaxFault::Unexpected {
                found: None,
                awaited: awaited_of(awaited),
                rewrite: None,
            },
            self.at.clone(),
        )
    }

    /// A tag's keyword the engine has no statement for.
    fn unknown_statement(&self, name: &str) -> (SyntaxFault, Range<usize>) {
        let whole = self.found().map_or(self.at.clone(), |i| self.whole_tag(i));
        let innermost = self.open.last();
        if let Some(ended) = name.strip_prefix("end")
            && BLOCKS.contains(&ended)
        {
            let open = innermost.map(|b| (b.keyword.to_owned(), line_of(self.text, b.tag.start)));
            let fault = SyntaxFault::ClosesNothing {
                end: name.into(),
                open,
            };
            return (fault, whole);
        }
        if matches!(name, "else" | "elif") {
            let fault = match innermost {
                Some(b) if name == "elif" && b.keyword == "if" && b.has_else => {
                    SyntaxFault::ElifAfterElse
                }
                Some(b) if name == "else" && matches!(b.keyword, "if" | "for") && b.has_else => {
                    SyntaxFault::SecondElse(b.keyword.into())
                }
                _ => SyntaxFault::Astray(name.into()),
            };
            return (fault, whole);
        }
        let nearest = match name {
            "elseif" => Some("elif".to_owned()),
            "end" => innermost.map(|b| format!("end{}", b.keyword)),
            _ => nearest(name, TAGS.iter().copied()).map(str::to_owned),
        };
        let fault = SyntaxFault::UnknownTag {
            written: name.into(),
            nearest,
        };
        (fault, self.at.clone())
    }

    /// The token at `index` where the template language needs `awaited`.
    fn unexpected(&self, index: usize, awaited: Awaited) -> (SyntaxFault, Range<usize>) {
        if let Some(read) = self.unclosed_tag(false) {
            return read;
        }
        let token = &self.tokens[index].0;
        let range = self.tokens[index].1.clone();
        let before = |back: usize| index.checked_sub(back).map(|i| &self.tokens[i]);
        let names = matches!(awaited, Awaited::Name | Awaited::NameOrNumber);
        match before(1) {
            Some((Token::Pipe, pipe)) if awaited == Awaited::Name => {
                if matches!(token, Token::Pipe) && pipe.end == range.start {
                    let both = pipe.start..range.end;
                    return (SyntaxFault::Foreign("||".into()), both);
                }
                return (SyntaxFault::NoFilter, pipe.clone());
            }
            Some((Token::Dot, dot)) if names => return (SyntaxFault::NoField, dot.clone()),
            Some((Token::Ident("block"), keyword))
                if awaited == Awaited::Name
                    && matches!(before(2), Some((Token::BlockStart, _))) =>
            {
                return (SyntaxFault::BlockUnnamed, keyword.clone());
            }
            _ => {}
        }
        let rewrite = self.rewrite(index, &awaited);
        let fault = SyntaxFault::Unexpected {
            found: Some(self.text[range.clone()].into()),
            awaited,
            rewrite,
        };
        (fault, range)
    }

    /// How to write what the engine stopped at, where that is clear.
    fn rewrite(&self, index: usize, awaited: &Awaited) -> Option<Rewrite> {
        let token = &self.tokens[index].0;
        let range = &self.tokens[index].1;
        let before = |back: usize| index.checked_sub(back).map(|i| &self.tokens[i]);
        let closing = match awaited {
            Awaited::Written(closing) if closing == "}}" || closing == "%}" => {
                Some(closing.as_str())
            }
            _ => None,
        };
        if let Some(closing) = closing
            && matches!(token, Token::BraceClose)
        {
            return Some(Rewrite::Close(closing.into()));
        }
        if closing.is_some()
            && matches!(token, Token::Str(_) | Token::String(_) | Token::Colon)
            && let Some((Token::Ident(filter), _)) = before(1)
            && let Some((Token::Pipe, _)) = before(2)
        {
            let first = match token {
                Token::Colon => index + 1,
                _ => index,
            };
            let value = self.value_from(first)?;
            return Some(Rewrite::Call(format!("{filter}({value})")));
        }
        let keyword = self.keyword_of(index);
        if closing == Some("%}")
            && matches!(token, Token::Assign)
            && matches!(keyword, Some("if" | "elif"))
        {
            return Some(Rewrite::Compare);
        }
        if matches!(token, Token::Ident("if"))
            && let Some((Token::Ident("else"), _)) = before(1)
            && let Some((Token::BlockStart, _)) = before(2)
        {
            return Some(Rewrite::Elif);
        }
        let names = || {
            let tag = self.tag_of(index)?;
            let (_, keyword) = self.tokens.get(tag.open + 1)?;
            let names = self.text[keyword.end..range.start].trim();
            (!names.is_empty()).then(|| names.to_owned())
        };
        if *awaited == Awaited::Written("in".into())
            && keyword == Some("for")
            && matches!(token, Token::Ident(_))
        {
            return names().map(Rewrite::In);
        }
        if *awaited == Awaited::Written("=".into()) && keyword == Some("set") {
            return names().map(Rewrite::Assign);
        }
        if matches!(token, Token::Ident(_))
            && let Some((Token::Str(_) | Token::String(_), quoted)) = before(1)
            && quoted.end == range.start
            && self.text[quoted.clone()].starts_with('\'')
        {
            let rest = &self.text[range.start..];
            let line = rest.split('\n').next().unwrap_or(rest);
            let close = line.find('\'')?;
            let words = &self.text[quoted.start + 1..range.start + close];
            return Some(Rewrite::DoubleQuotes(words.to_owned()));
        }
        None
    }

    /// The text of a value starting at token `first`, as far as the next
    /// `|` or the tag's end outside any brackets.
    fn value_from(&self, first: usize) -> Option<String> {
        let mut depth = 0i32;
        let mut last = None;
        for (i, (token, _)) in self.tokens.iter().enumerate().skip(first) {
            match token {
                Token::Pipe | Token::VariableEnd | Token::BlockEnd if depth == 0 => break,
                Token::ParenOpen | Token::BracketOpen | Token::BraceOpen => depth += 1,
                Token::ParenClose | Token::BracketClose | Token::BraceClose => depth -= 1,
                _ => {}
            }
            last = Some(i);
        }
        let last = last?;
        Some(self.text[self.tokens[first].1.start..self.tokens[last].1.end].to_owned())
    }
}

/// What the engine's words say it expected, in the template's terms.
fn awaited_of(words: &str) -> Awaited {
    match words {
        "end of block" => Awaited::Written("%}".into()),
        "end of variable block" => Awaited::Written("}}".into()),
        "identifier" => Awaited::Name,
        "identifier or integer" => Awaited::NameOrNumber,
        "expression" => Awaited::Value,
        "block keyword" | "keyword" => Awaited::TagName,
        "assignment operator" => Awaited::Written("=".into()),
        "comma" => Awaited::Written(",".into()),
        "missing keyword" => Awaited::Written("missing".into()),
        written => Awaited::Written(written.trim_matches('`').into()),
    }
}

/// Whether a number written too large for the engine is all digits of its
/// base, rather than not a number at all.
fn is_digits(written: &str) -> bool {
    let lower = written.to_ascii_lowercase();
    let (radix, digits) = match lower.get(..2) {
        Some("0x") => (16, &lower[2..]),
        Some("0o") => (8, &lower[2..]),
        Some("0b") => (2, &lower[2..]),
        _ => (10, lower.as_str()),
    };
    let digits: Vec<char> = digits.chars().filter(|&c| c != '_').collect();
    !digits.is_empty() && digits.iter().all(|c| c.is_digit(radix))
}

/// The first escape in the string opening at `start` that the engine
/// can't finish, as written, and where.
fn bad_escape(text: &str, start: usize) -> Option<(String, Range<usize>)> {
    let quote = text[start..].chars().next()?;
    let body = start + quote.len_utf8();
    let bytes = text.as_bytes();
    let hex_run = |from: usize, most: usize| {
        text[from..]
            .chars()
            .take(most)
            .take_while(char::is_ascii_hexdigit)
            .count()
    };
    let mut i = body;
    while i < text.len() {
        let c = bytes[i];
        if c == quote as u8 {
            return None;
        }
        if c != b'\\' {
            i += text[i..].chars().next().map_or(1, char::len_utf8);
            continue;
        }
        let &kind = bytes.get(i + 1)?;
        let end = match kind {
            b'u' => {
                let digits = hex_run(i + 2, 4);
                let end = i + 2 + digits;
                if digits < 4 {
                    Some(end)
                } else {
                    let unit = u16::from_str_radix(&text[i + 2..end], 16).ok()?;
                    let low = |at: usize| {
                        text[at..].starts_with("\\u")
                            && hex_run(at + 2, 4) == 4
                            && u16::from_str_radix(&text[at + 2..at + 6], 16)
                                .is_ok_and(|u| (0xDC00..=0xDFFF).contains(&u))
                    };
                    match unit {
                        0xD800..=0xDBFF if low(end) => {
                            i = end + 6;
                            continue;
                        }
                        0xD800..=0xDFFF => Some(end),
                        _ => None,
                    }
                }
            }
            b'x' => {
                let digits = hex_run(i + 2, 2);
                (digits < 2).then_some(i + 2 + digits)
            }
            b'0'..=b'7' => {
                let digits = 1 + text[i + 2..]
                    .chars()
                    .take(2)
                    .take_while(|c| ('0'..='7').contains(c))
                    .count();
                let end = i + 1 + digits;
                u8::from_str_radix(&text[i + 1..end], 8)
                    .is_err()
                    .then_some(end)
            }
            _ => None,
        };
        if let Some(end) = end {
            return Some((text[i..end].to_owned(), i..end));
        }
        i += 1 + text[i + 1..].chars().next().map_or(1, char::len_utf8);
    }
    None
}
