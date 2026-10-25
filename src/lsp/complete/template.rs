//! Completion in a template or a snippet: the names and fields a template
//! may read, the filters, tests and statements it may write, the blocks
//! of the template it extends, and the templates, addresses and files it
//! may name.
//!
//! Where the cursor stands is found from the text before it. A name or a
//! field is then asked of the template read up to the cursor with a name
//! written there, its brackets, tag and blocks closed after it (see
//! `Renderer::readable`), so what is offered is what the check takes
//! there.

#[cfg(test)]
mod tests;

use std::ops::Range;

use super::writing::{addresses, in_quote, names_left, takes_written};
use super::{Completion, Item, ItemKind};
use crate::described::{Described, Group};
use crate::problem::Holds;
use crate::templates::{Name, Readable, Renderer, TYPED};
use crate::wording;

/// The statements that open a block, which `end` and the word close.
const BLOCKS: &[&str] = &[
    "autoescape",
    "block",
    "call",
    "filter",
    "for",
    "if",
    "macro",
    "set",
    "with",
];

/// The statements a tag may open with, besides those closing or going on
/// with the blocks open.
const STATEMENTS: &[&str] = &[
    "autoescape",
    "block",
    "call",
    "do",
    "extends",
    "filter",
    "for",
    "if",
    "macro",
    "raw",
    "set",
    "with",
];

/// What may be written at `offset` in `text`, the text of the site's
/// template or snippet at `path`; none where nothing is offered there.
/// `snippets` is whether the editor fills in places in what it writes.
pub(super) fn complete(
    text: &str,
    offset: usize,
    path: &str,
    renderer: Option<&Renderer>,
    snippets: bool,
) -> Option<Completion> {
    let before = before(text, offset)?;
    let tokens = lex(text, before.inner..offset);
    let offer = Offer {
        text,
        path,
        renderer,
        snippets,
    };
    if let Some(Lexed {
        token:
            Token::Text {
                quote,
                start,
                closed: false,
            },
        ..
    }) = tokens.last()
    {
        let ahead = &tokens[..tokens.len() - 1];
        return offer.named(ahead, before.statement, *quote, *start..offset);
    }
    let (typed, rest) = match tokens.split_last() {
        Some((last, rest)) if last.span.end == offset => match last.token {
            Token::Name(_) => (last.span.clone(), rest),
            Token::Number => return None,
            _ => (offset..offset, &tokens[..]),
        },
        _ => (offset..offset, &tokens[..]),
    };
    let previous = rest.last().map(|lexed| lexed.token);
    let first = rest.first().map(|lexed| lexed.token);
    let items = match previous {
        Some(Token::Mark(".")) => offer.fields(&before, rest, typed.start, opened(text, offset))?,
        Some(Token::Mark("|")) => filters(),
        Some(Token::Name("filter")) if before.statement && rest.len() == 1 => filters(),
        Some(Token::Name("is")) => tests(),
        Some(Token::Name("not"))
            if rest.len() > 1 && rest[rest.len() - 2].token == Token::Name("is") =>
        {
            tests()
        }
        None if before.statement => statements(&before.open),
        Some(Token::Name("block")) if before.statement && rest.len() == 1 => {
            let extended = before.extends.as_deref();
            let blocks = (renderer.zip(extended).into_iter())
                .flat_map(|(renderer, extended)| renderer.blocks(extended));
            (blocks.filter(|block| !before.blocks.contains(block)))
                .map(|block| item(&block, None, ItemKind::Reference, &block))
                .collect()
        }
        Some(Token::Name("extends" | "macro")) if before.statement && rest.len() == 1 => {
            return None;
        }
        _ if before.statement && first == Some(Token::Name("for")) && !in_rest(rest, "in") => {
            return None;
        }
        _ => {
            let left = offer.names_left(rest);
            match offer.names(&before, rest, typed.start, opened(text, offset)) {
                Some(mut names) => {
                    names.extend(left);
                    names
                }
                // After a value given by name only another named value
                // may follow, which a name alone can't be read as.
                None if !left.is_empty() => left,
                None => return None,
            }
        }
    };
    Some(Completion::of(typed, items))
}

/// What the word or quoted text a hover points at in a template is.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(in crate::lsp) enum Pointed {
    /// A name or a field the template reads, as written up to the word,
    /// `post.title`, what it holds, and what it is where it is Statac's.
    Value {
        range: Range<usize>,
        written: String,
        holds: Holds,
        about: Option<&'static Described>,
    },
    /// A filter, by its name, where it is one a template may use.
    Filter { range: Range<usize>, name: String },
    /// A call to the site's snippet `name`.
    Snippet { range: Range<usize>, name: String },
    /// The address `url(...)` is handed.
    Address {
        range: Range<usize>,
        address: String,
    },
    /// The template `extends` names.
    Extended { range: Range<usize>, name: String },
}

/// What the character at `offset` in `text`, the text of the site's
/// template or snippet at `path`, is part of: a name or field the
/// template reads where it stands, a snippet it calls, or an address or
/// a template it names; none where it is anything else.
pub(in crate::lsp) fn pointed(
    text: &str,
    offset: usize,
    path: &str,
    renderer: &Renderer,
) -> Option<Pointed> {
    let before = before(text, offset)?;
    let close = if before.statement { "%}" } else { "}}" };
    let end = closed(text, before.inner..text.len(), close).unwrap_or(text.len());
    let tokens = lex(text, before.inner..end);
    let at = (tokens.iter()).position(|lexed| lexed.span.contains(&offset))?;
    let (ahead, lexed) = (&tokens[..at], &tokens[at]);
    let range = lexed.span.clone();
    let words: Vec<Token> = ahead.iter().map(|lexed| lexed.token).collect();
    let word = match lexed.token {
        Token::Text {
            start,
            closed: true,
            ..
        } => {
            let named = text[start..range.end - 1].to_owned();
            return match words.as_slice() {
                [Token::Name("extends")] if before.statement => {
                    Some(Pointed::Extended { range, name: named })
                }
                [.., Token::Mark("."), Token::Name("url"), Token::Mark("(")] => None,
                [.., Token::Name("url"), Token::Mark("(")] => Some(Pointed::Address {
                    range,
                    address: named,
                }),
                _ => None,
            };
        }
        Token::Name(word) => word,
        _ => return None,
    };
    let filter = || {
        (crate::templates::filter_names())
            .any(|filter| filter == word)
            .then(|| Pointed::Filter {
                range: range.clone(),
                name: word.to_owned(),
            })
    };
    match words.as_slice() {
        [] if before.statement => return None,
        [.., Token::Mark("|")] => return filter(),
        [Token::Name("filter")] if before.statement => return filter(),
        [.., Token::Name("is" | "not")] => return None,
        [Token::Name("block" | "extends" | "macro")] if before.statement => {
            return None;
        }
        _ => {}
    }
    let offer = Offer {
        text,
        path,
        renderer: Some(renderer),
        snippets: false,
    };
    let start = range.start;
    let found = match offer.readable(&before, ahead, start)? {
        Readable::Fields(fields) if words.last() == Some(&Token::Mark(".")) => fields,
        Readable::Names(names) if words.last() != Some(&Token::Mark(".")) => names,
        _ => return None,
    };
    let Name { holds, about, .. } = found.into_iter().find(|name| name.name == word)?;
    if holds == Holds::Call && about.is_none() {
        let snippets = renderer.snippets();
        let called = (snippets.iter()).any(|(snippet, _)| snippet.name == word);
        let ours = words.last() != Some(&Token::Mark("."));
        return (called && ours).then(|| Pointed::Snippet {
            range,
            name: word.to_owned(),
        });
    }
    // The fields leading to the word, back to the name they start from,
    // over any call or index along the way, as `page.image('a.jpg').src`.
    let mut from = at;
    while from >= 2 && ahead[from - 1].token == Token::Mark(".") {
        match ahead[from - 2].token {
            Token::Name(_) => from -= 2,
            Token::Mark(")" | "]") => match called_from(&ahead[..from - 1]) {
                Some(name) => from = name,
                None => break,
            },
            _ => break,
        }
    }
    let written = text[tokens[from].span.start..range.end].to_owned();
    (holds != Holds::Unknown || about.is_some()).then_some(Pointed::Value {
        range,
        written,
        holds,
        about,
    })
}

/// Where the name is that the call or index closing `tokens` follows:
/// before the `(` or `[` its last bracket closes.
fn called_from(tokens: &[Lexed]) -> Option<usize> {
    let mut depth = 0usize;
    for (at, lexed) in tokens.iter().enumerate().rev() {
        match lexed.token {
            Token::Mark(")" | "]") => depth += 1,
            Token::Mark("(" | "[") => {
                depth -= 1;
                if depth == 0 {
                    let name = at.checked_sub(1)?;
                    return matches!(tokens[name].token, Token::Name(_)).then_some(name);
                }
            }
            _ => {}
        }
    }
    None
}

/// Whether a bracket follows the name `offset` is in.
fn opened(text: &str, offset: usize) -> bool {
    text[offset..]
        .trim_start_matches(|c: char| c.is_alphanumeric() || c == '_')
        .starts_with('(')
}

/// Whether the tag holds the word `word` before the cursor.
fn in_rest(rest: &[Lexed], word: &str) -> bool {
    rest.iter().any(|lexed| lexed.token == Token::Name(word))
}

/// Where the cursor stands, from what the template holds before it.
struct Before {
    /// Where what the tag the cursor is in holds begins, past its opening
    /// and any `-` or `+` after it.
    inner: usize,
    /// Whether the tag is a statement, `{% ... %}`, rather than a value.
    statement: bool,
    /// The blocks open, innermost last, each by the word opening it.
    open: Vec<String>,
    /// The template the template extends, where it names one.
    extends: Option<String>,
    /// The blocks it has opened, by name.
    blocks: Vec<String>,
}

/// What the template holds before `offset`, where `offset` is in a tag;
/// none where it is in a comment, in a raw block, or outside any tag.
fn before(text: &str, offset: usize) -> Option<Before> {
    let mut found = Before {
        inner: 0,
        statement: false,
        open: Vec::new(),
        extends: None,
        blocks: Vec::new(),
    };
    let mut at = 0;
    loop {
        let start = at + text[at..offset].find('{')?;
        let opening = text[start + 1..offset].chars().next();
        let (statement, close) = match opening {
            Some('#') => {
                let end = text[start + 2..offset].find("#}")?;
                at = start + 2 + end + 2;
                continue;
            }
            Some('{') => (false, "}}"),
            Some('%') => (true, "%}"),
            _ => {
                at = start + 1;
                continue;
            }
        };
        let mut inner = start + 2;
        if text[inner..offset].starts_with(['-', '+']) {
            inner += 1;
        }
        let Some(end) = closed(text, inner..offset, close) else {
            found.inner = inner;
            found.statement = statement;
            return Some(found);
        };
        at = end + close.len();
        if !statement {
            continue;
        }
        let tokens = lex(text, inner..end);
        let words: Vec<&str> = (tokens.iter())
            .filter_map(|lexed| match lexed.token {
                Token::Name(name) => Some(name),
                _ => None,
            })
            .collect();
        let Some(&word) = words.first() else {
            continue;
        };
        match word {
            "raw" => at = end_of_raw(text, at, offset)?,
            "extends" => {
                if let Some(Lexed {
                    token:
                        Token::Text {
                            start,
                            closed: true,
                            ..
                        },
                    span,
                }) = tokens.get(1)
                {
                    found.extends = Some(text[*start..span.end - 1].to_owned());
                }
            }
            "set" if tokens.iter().any(|lexed| lexed.token == Token::Mark("=")) => {}
            word if BLOCKS.contains(&word) => {
                if word == "block"
                    && let Some(&name) = words.get(1)
                {
                    found.blocks.push(name.to_owned());
                }
                found.open.push(word.to_owned());
            }
            word => {
                if let Some(ended) = word.strip_prefix("end")
                    && let Some(at) = found.open.iter().rposition(|open| open == ended)
                {
                    found.open.truncate(at);
                }
            }
        }
    }
}

/// Where in `range` of `text` the tag ends with `close`, outside its
/// strings; none where it doesn't before the range does.
fn closed(text: &str, range: Range<usize>, close: &str) -> Option<usize> {
    let held = &text[range.clone()];
    let mut quote = None;
    let mut chars = held.char_indices();
    while let Some((at, c)) = chars.next() {
        match quote {
            Some(_) if c == '\\' => {
                chars.next();
            }
            Some(open) if c == open => quote = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => quote = Some(c),
            None if held[at..].starts_with(close) => return Some(range.start + at),
            None => {}
        }
    }
    None
}

/// Where the text a raw block holds after `at` ends, past its closing
/// tag; none where it doesn't before `offset`.
fn end_of_raw(text: &str, mut at: usize, offset: usize) -> Option<usize> {
    loop {
        let inner = at + text[at..offset].find("{%")? + 2;
        let word = text[inner..offset]
            .trim_start_matches(['-', '+'])
            .trim_start();
        at = inner;
        if word.starts_with("endraw") {
            return closed(text, inner..offset, "%}").map(|end| end + 2);
        }
    }
}

/// One piece of what a tag holds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Token<'t> {
    Name(&'t str),
    /// A string, by its quote and where its text begins, and whether its
    /// quote is closed.
    Text {
        quote: char,
        start: usize,
        closed: bool,
    },
    Number,
    Mark(&'t str),
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct Lexed<'t> {
    token: Token<'t>,
    span: Range<usize>,
}

/// The pieces `range` of `text` holds, read as the inside of a tag.
fn lex(text: &str, range: Range<usize>) -> Vec<Lexed<'_>> {
    let held = &text[range.clone()];
    let mut tokens = Vec::new();
    let mut at = 0;
    while let Some(c) = held[at..].chars().next() {
        let start = at;
        let rest = &held[at..];
        let token = if c.is_whitespace() {
            at += c.len_utf8();
            continue;
        } else if c.is_alphabetic() || c == '_' {
            at += rest
                .find(|c: char| !(c.is_alphanumeric() || c == '_'))
                .unwrap_or(rest.len());
            Token::Name(&held[start..at])
        } else if c.is_ascii_digit() {
            let mut end = 0;
            for (i, c) in rest.char_indices() {
                let decimal = c == '.' && rest[i + 1..].starts_with(|c: char| c.is_ascii_digit());
                if !(c.is_ascii_digit() || c == '_' || decimal) {
                    break;
                }
                end = i + 1;
            }
            at += end;
            Token::Number
        } else if c == '"' || c == '\'' {
            let mut chars = rest.char_indices().skip(1);
            let mut end = None;
            while let Some((i, d)) = chars.next() {
                if d == '\\' {
                    chars.next();
                } else if d == c {
                    end = Some(i + 1);
                    break;
                }
            }
            at += end.unwrap_or(rest.len());
            Token::Text {
                quote: c,
                start: range.start + start + 1,
                closed: end.is_some(),
            }
        } else {
            let two = ["==", "!=", "<=", ">=", "//", "**"];
            let len = match two.iter().any(|mark| rest.starts_with(mark)) {
                true => 2,
                false => c.len_utf8(),
            };
            at += len;
            Token::Mark(&held[start..at])
        };
        tokens.push(Lexed {
            token,
            span: range.start + start..range.start + at,
        });
    }
    tokens
}

/// What the template may hold at the cursor.
struct Offer<'a> {
    text: &'a str,
    /// The file's path in the site.
    path: &'a str,
    renderer: Option<&'a Renderer>,
    snippets: bool,
}

impl Offer<'_> {
    /// What a string the cursor is in may hold, at `typed`, where what
    /// comes before it in the tag, `ahead`, names a template or asks for
    /// an address or a file by name.
    fn named(
        &self,
        ahead: &[Lexed],
        statement: bool,
        quote: char,
        typed: Range<usize>,
    ) -> Option<Completion> {
        let renderer = self.renderer?;
        let tokens: Vec<Token> = ahead.iter().map(|lexed| lexed.token).collect();
        let call = match tokens.as_slice() {
            [Token::Name("extends")] if statement => {
                let items = (renderer.layouts().into_iter())
                    .filter(|(_, path)| path != self.path)
                    .map(|(name, path)| {
                        item(
                            &name,
                            Some(path),
                            ItemKind::File,
                            &in_quote(&name, Some(quote)),
                        )
                    })
                    .collect();
                return Some(Completion::of(typed, items));
            }
            [.., Token::Mark("."), Token::Name(_), Token::Mark("(")] => return None,
            [.., Token::Name(call), Token::Mark("(")] => *call,
            _ => return None,
        };
        let ending = match call {
            "url" => {
                let text = self.text;
                return Some(addresses(
                    Some(renderer),
                    None,
                    text,
                    typed,
                    "",
                    Some(quote),
                ));
            }
            "asset" => "",
            "css" => ".css",
            "js" => ".js",
            _ => return None,
        };
        let items = (renderer.asset_names().into_iter())
            .filter(|name| name.ends_with(ending))
            .map(|name| item(&name, None, ItemKind::File, &in_quote(&name, Some(quote))))
            .collect();
        Some(Completion::of(typed, items))
    }

    /// The template read up to `typed`, with [`TYPED`] written there and
    /// what is open closed after it.
    fn probe(&self, before: &Before, rest: &[Lexed], typed: usize) -> String {
        let mut probe = format!("{}{TYPED}", &self.text[..typed]);
        let mut brackets = Vec::new();
        for lexed in rest {
            match lexed.token {
                Token::Mark("(") => brackets.push(')'),
                Token::Mark("[") => brackets.push(']'),
                Token::Mark("{") => brackets.push('}'),
                Token::Mark(")" | "]" | "}") => {
                    brackets.pop();
                }
                _ => {}
            }
        }
        probe.extend(brackets.iter().rev());
        let mut open = before.open.clone();
        if !before.statement {
            probe.push_str(" }}");
        } else {
            probe.push_str(" %}");
            if let Some(Token::Name(word)) = rest.first().map(|lexed| lexed.token)
                && BLOCKS.contains(&word)
                && !(word == "set" && rest.iter().any(|lexed| lexed.token == Token::Mark("=")))
            {
                open.push(word.to_owned());
            }
        }
        for word in open.iter().rev() {
            probe.push_str(&format!("{{% end{word} %}}"));
        }
        probe
    }

    fn readable(&self, before: &Before, rest: &[Lexed], typed: usize) -> Option<Readable> {
        let probe = self.probe(before, rest, typed);
        self.renderer?.readable(self.path, &probe)
    }

    /// The names a template may read at the cursor, `opened` being whether
    /// a bracket follows what is typed.
    fn names(
        &self,
        before: &Before,
        rest: &[Lexed],
        typed: usize,
        opened: bool,
    ) -> Option<Vec<Item>> {
        let Readable::Names(names) = self.readable(before, rest, typed)? else {
            return None;
        };
        let snippets = self.renderer.map(Renderer::snippets).unwrap_or_default();
        let items = names.into_iter().map(|Name { name, holds, about }| {
            let snippet = (snippets.iter()).find(|(snippet, _)| snippet.name == name);
            let item = match (holds, snippet) {
                (Holds::Call, Some((snippet, path))) => Item {
                    label: format!("{name}({})", takes_written(snippet)),
                    filter: Some(name.clone()),
                    group: 1,
                    ..item(
                        &name,
                        Some(path.clone()),
                        ItemKind::Snippet,
                        &self.call(&name, opened),
                    )
                },
                (Holds::Call, None) => Item {
                    group: 1,
                    ..item(&name, None, ItemKind::Function, &self.call(&name, opened))
                },
                (holds, _) => item(&name, said(holds), ItemKind::Variable, &name),
            };
            Item { about, ..item }
        });
        Some(items.collect())
    }

    /// The fields of the value before the dot at the cursor, `opened`
    /// being whether a bracket follows what is typed.
    fn fields(
        &self,
        before: &Before,
        rest: &[Lexed],
        typed: usize,
        opened: bool,
    ) -> Option<Vec<Item>> {
        let Readable::Fields(fields) = self.readable(before, rest, typed)? else {
            return None;
        };
        let items = fields.into_iter().map(|Name { name, holds, about }| {
            let item = match holds {
                Holds::Call => item(&name, None, ItemKind::Function, &self.call(&name, opened)),
                holds => item(&name, said(holds), ItemKind::Field, &name),
            };
            Item { about, ..item }
        });
        Some(items.collect())
    }

    /// The names a snippet whose call's brackets the cursor is in, where
    /// a value begins, has been given no value for yet, each with its `=`.
    fn names_left(&self, rest: &[Lexed]) -> Vec<Item> {
        let Some(Token::Mark("(" | ",")) = rest.last().map(|lexed| lexed.token) else {
            return Vec::new();
        };
        let mut open = Vec::new();
        for (at, lexed) in rest.iter().enumerate() {
            match lexed.token {
                Token::Mark("(" | "[" | "{") => open.push(at),
                Token::Mark(")" | "]" | "}") => {
                    open.pop();
                }
                _ => {}
            }
        }
        let Some(&bracket) = open.last() else {
            return Vec::new();
        };
        let ahead: Vec<Token> = rest[..bracket].iter().map(|lexed| lexed.token).collect();
        let called = match ahead.as_slice() {
            [.., Token::Mark("."), Token::Name(_)] => return Vec::new(),
            [.., Token::Name(name)] => *name,
            _ => return Vec::new(),
        };
        let snippets = self.renderer.map(Renderer::snippets).unwrap_or_default();
        let Some((snippet, _)) = snippets.iter().find(|(snippet, _)| snippet.name == called) else {
            return Vec::new();
        };
        let (place, given) = given(&rest[bracket + 1..]);
        names_left(snippet, place, &given)
    }

    /// A call to `name` as it is written in place of its name: with its
    /// brackets, the cursor between them, where it has none yet.
    fn call(&self, name: &str, opened: bool) -> String {
        match (opened, self.snippets) {
            (true, _) => name.to_owned(),
            (false, true) => format!("{name}($1)"),
            (false, false) => format!("{name}("),
        }
    }
}

/// How many values a call's brackets hold without names before the
/// cursor, and the names of those with them, from what the brackets hold.
fn given(held: &[Lexed]) -> (usize, Vec<String>) {
    let mut place = 0;
    let mut given = Vec::new();
    let mut depth = 0;
    let mut value: Vec<Token> = Vec::new();
    for lexed in held {
        match lexed.token {
            Token::Mark("(" | "[" | "{") => depth += 1,
            Token::Mark(")" | "]" | "}") => depth -= 1,
            Token::Mark(",") if depth == 0 => {
                match value.as_slice() {
                    [Token::Name(name), Token::Mark("="), ..] => given.push((*name).to_owned()),
                    [] => {}
                    _ => place += 1,
                }
                value.clear();
                continue;
            }
            _ => {}
        }
        value.push(lexed.token);
    }
    (place, given)
}

/// What a value holds, said beside it; nothing where that isn't known.
fn said(holds: Holds) -> Option<String> {
    Some(wording::panel_holds(holds))
        .filter(|said| !said.is_empty())
        .map(str::to_owned)
}

fn item(label: &str, detail: Option<String>, kind: ItemKind, insert: &str) -> Item {
    Item {
        label: label.to_owned(),
        detail,
        kind,
        insert: insert.to_owned(),
        group: 0,
        text: false,
        filter: None,
        about: None,
    }
}

fn filters() -> Vec<Item> {
    (crate::templates::filter_names())
        .map(|name| Item {
            about: Group::Filters.of(name),
            ..item(name, None, ItemKind::Function, name)
        })
        .collect()
}

fn tests() -> Vec<Item> {
    (crate::templates::test_names())
        .map(|name| Item {
            about: Group::Tests.of(name),
            ..item(name, None, ItemKind::Function, name)
        })
        .collect()
}

/// The statements a tag may open with inside the blocks `open`: those
/// opening a block or standing alone, and those closing or going on with
/// the innermost block, or reaching the loop it is in.
fn statements(open: &[String]) -> Vec<Item> {
    let mut words: Vec<String> = STATEMENTS.iter().map(|word| (*word).to_owned()).collect();
    if let Some(innermost) = open.last() {
        words.push(format!("end{innermost}"));
        match innermost.as_str() {
            "if" => words.extend(["elif".to_owned(), "else".to_owned()]),
            "for" => words.push("else".to_owned()),
            _ => {}
        }
    }
    if open.iter().any(|word| word == "for") {
        words.extend(["break".to_owned(), "continue".to_owned()]);
    }
    words.sort();
    (words.iter())
        .map(|word| item(word, None, ItemKind::Keyword, word))
        .collect()
}
