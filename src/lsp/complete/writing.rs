//! Completion in a page's writing: the snippets a call may name, the
//! values a call takes, the addresses, headings and files a link,
//! a picture or an `@url(...)` may lead to, and the words asking how a
//! code block's lines are shown.
//!
//! Where the cursor stands is found from the text before it, then asked
//! of the writing as it reads (see `markdown::reads_a_call`), so nothing
//! is offered in code, in text shown as typed, or where a call or link
//! would not be read as one.

#[cfg(test)]
mod tests;

use std::ops::Range;

use super::{Completion, Item, ItemKind};
use crate::ids::PageId;
use crate::markdown::{self, IN_SITE, LINE_WORDS};
use crate::problem::Sort;
use crate::source::Span;
use crate::templates::{Renderer, Snippet, Takes};
use crate::wording;

/// How many of the site's addresses are offered at once.
const MOST_ADDRESSES: usize = 200;

/// Where the cursor stands in the writing.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) enum Spot {
    /// In the name of a call, after its `@`.
    Name {
        /// The `@`.
        at: usize,
        /// The name typed so far, and any more of it after the cursor
        /// where a bracket follows it.
        typed: Range<usize>,
        /// Whether the name already has its bracket after it.
        opened: bool,
        /// Whether the call stands alone on its line so far.
        alone: bool,
    },
    /// In a value of a call.
    Value {
        /// The `@`.
        at: usize,
        name: String,
        /// The value typed so far, inside any quote.
        typed: Range<usize>,
        quote: Option<char>,
        /// The name it is given by, where it is given one.
        named: Option<String>,
        /// How many values without names come before it.
        place: usize,
        /// The names the call gives values by besides.
        given: Vec<String>,
    },
    /// In a link's or a picture's address, after the bracket at `open`,
    /// or the address a line defining a link reference gives.
    Address {
        open: usize,
        typed: Range<usize>,
        definition: bool,
    },
    /// In a word after the first on the line starting at `line`, where it
    /// opens a fence: a word asking how the block's lines are shown.
    Fence {
        line: usize,
        typed: Range<usize>,
        /// Those words the line has already.
        written: Vec<&'static str>,
    },
}

/// Where `offset` stands in the writing at `body` of `text`, from the
/// text before it alone; none where it is in nothing completed.
pub(super) fn find(text: &str, body: Span, offset: usize) -> Option<Spot> {
    let start = body.start as usize;
    if offset < start || offset > body.end as usize {
        return None;
    }
    let line = text[..offset].rfind('\n').map_or(0, |at| at + 1).max(start);
    in_fence(text, line, offset)
        .or_else(|| in_value(text, start, offset))
        .or_else(|| in_name(text, line, offset))
        .or_else(|| in_address(text, line, offset))
}

/// The cursor in a word after the first on a line written as a fence's
/// opening, outside a quoted caption.
fn in_fence(text: &str, line: usize, offset: usize) -> Option<Spot> {
    let before = &text[line..offset];
    let opening = before.trim_start_matches([' ', '\t', '>']);
    let marker = opening.chars().next().filter(|c| matches!(c, '`' | '~'))?;
    let info = opening.trim_start_matches(marker);
    if opening.len() - info.len() < 3 {
        return None;
    }
    let word = info.len() - info.trim_end_matches(is_name_char).len();
    let ahead = &info[..info.len() - word];
    let quoted = ['"', '\'']
        .iter()
        .any(|q| ahead.matches(*q).count() % 2 == 1);
    if ahead.trim().is_empty() || !ahead.ends_with(char::is_whitespace) || quoted {
        return None;
    }
    let end = text[offset..]
        .find('\n')
        .map_or(text.len(), |at| offset + at);
    let whole = &text[line..end];
    let written = (LINE_WORDS.into_iter())
        .filter(|name| {
            whole
                .split_whitespace()
                .any(|w| w.starts_with(&format!("{name}=")))
        })
        .collect();
    Some(Spot::Fence {
        line,
        typed: offset - word..offset,
        written,
    })
}

/// The cursor in the name of a call.
fn in_name(text: &str, line: usize, offset: usize) -> Option<Spot> {
    let before = &text[line..offset];
    let word = before.len() - before.trim_end_matches(is_name_char).len();
    let at = (offset - word).checked_sub(1)?;
    if at < line || text.as_bytes()[at] != b'@' {
        return None;
    }
    if text[at + 1..offset].starts_with(|c: char| !c.is_ascii_alphabetic()) {
        return None;
    }
    let after = &text[offset..];
    let more = after.len() - after.trim_start_matches(is_name_char).len();
    let opened = after[more..].starts_with('(');
    let end = if opened { offset + more } else { offset };
    let leading = &text[line..at];
    let alone = leading.trim_start_matches([' ', '\t', '>']).is_empty()
        && text[offset..]
            .split('\n')
            .next()
            .is_some_and(|rest| rest.trim().is_empty());
    Some(Spot::Name {
        at,
        typed: at + 1..end,
        opened,
        alone,
    })
}

/// The cursor in a value of a call, the nearest whose bracket is open
/// before it within its paragraph.
fn in_value(text: &str, start: usize, offset: usize) -> Option<Spot> {
    let paragraph = text[start..offset]
        .rfind("\n\n")
        .map_or(start, |at| start + at + 2);
    let mut from = offset;
    while let Some(at) = text[paragraph..from].rfind('@').map(|at| paragraph + at) {
        from = at;
        let rest = &text[at + 1..offset];
        let name_len = rest.len() - rest.trim_start_matches(is_name_char).len();
        if name_len == 0
            || !rest.starts_with(|c: char| c.is_ascii_alphabetic())
            || !rest[name_len..].starts_with('(')
        {
            continue;
        }
        let name = rest[..name_len].to_owned();
        let open = at + 1 + name_len;
        return values(text, open, offset).map(|value| Spot::Value {
            at,
            name,
            typed: value.typed,
            quote: value.quote,
            named: value.named,
            place: value.place,
            given: value.given,
        });
    }
    None
}

/// A value being typed, read from a call's bracket.
struct Typing {
    typed: Range<usize>,
    quote: Option<char>,
    named: Option<String>,
    place: usize,
    given: Vec<String>,
}

/// The value the cursor at `offset` is in, among the values after the
/// bracket at `open`; none where the call closes before it.
fn values(text: &str, open: usize, offset: usize) -> Option<Typing> {
    let mut place = 0;
    let mut given = Vec::new();
    let mut at = open + 1;
    loop {
        // One value: an optional name and `=` or `:`, then the value.
        let rest = &text[at..offset];
        let value_at = at + (rest.len() - rest.trim_start().len());
        let rest = &text[value_at..offset];
        let name_len = rest.len() - rest.trim_start_matches(is_name_char).len();
        let after_name = rest[name_len..].trim_start_matches([' ', '\t']);
        let named = (name_len > 0 && after_name.starts_with(['=', ':']))
            .then(|| rest[..name_len].to_owned());
        let value_at = match &named {
            Some(_) => {
                let sign = offset - after_name.len() + 1;
                sign + (text[sign..offset].len() - text[sign..offset].trim_start().len())
            }
            None => value_at,
        };
        let quote = text[value_at..offset]
            .chars()
            .next()
            .filter(|c| matches!(c, '"' | '\''));
        let begun = value_at + quote.map_or(0, char::len_utf8);
        // Where the value ends, where it ends before the cursor.
        let ends = match quote {
            Some(quote) => closing_quote(&text[begun..offset], quote).map(|at| begun + at + 1),
            None => text[value_at..offset]
                .find([',', ')'])
                .map(|at| value_at + at),
        };
        let Some(end) = ends else {
            if quote.is_some() && text[begun..offset].contains('\n') {
                return None;
            }
            return Some(Typing {
                typed: begun..offset,
                quote,
                named,
                place,
                given,
            });
        };
        let after = text[end..offset].trim_start();
        match after.chars().next() {
            Some(',') => {
                match named {
                    Some(name) => given.push(name),
                    None => place += 1,
                }
                at = offset - after.len() + 1;
            }
            _ => return None,
        }
    }
}

/// Where the quote closing a value opened with `quote` is in `text`, past
/// any it holds with a backslash before it.
fn closing_quote(text: &str, quote: char) -> Option<usize> {
    let mut escaped = false;
    for (at, c) in text.char_indices() {
        match c {
            '\\' if !escaped => escaped = true,
            c if c == quote && !escaped => return Some(at),
            _ => escaped = false,
        }
    }
    None
}

/// The cursor in a link's or picture's address, on the line it is on.
fn in_address(text: &str, line: usize, offset: usize) -> Option<Spot> {
    let before = &text[line..offset];
    let typed_len = before.len()
        - before
            .trim_end_matches(|c: char| !c.is_whitespace() && c != '(')
            .len();
    let typed = offset - typed_len..offset;
    if text[typed.clone()].contains([')', '<']) {
        return None;
    }
    let lead = &text[line..typed.start];
    if lead.ends_with("](") {
        return Some(Spot::Address {
            open: typed.start - 1,
            typed,
            definition: false,
        });
    }
    // `[label]: ` at the line's start, the address after it.
    let indent = lead.len() - lead.trim_start_matches(' ').len();
    let label = lead[indent..].trim_end();
    let defines = indent <= 3
        && label.starts_with('[')
        && label.ends_with("]:")
        && !label.starts_with("[^")
        && lead.len() > label.len() + indent;
    defines.then_some(Spot::Address {
        open: typed.start,
        typed,
        definition: true,
    })
}

fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// What may be written at the spot, where the writing reads it as a call
/// or a link.
pub(super) struct Offer<'a> {
    pub(crate) text: &'a str,
    pub(crate) body: Span,
    pub(crate) renderer: Option<&'a Renderer>,
    /// The page being written, where the site as last read holds it.
    pub(crate) page: Option<PageId>,
    /// The folder under `content/` the page's file is in, ending with
    /// `/`, or empty at the top.
    pub(crate) folder: &'a str,
    /// Whether the editor fills in places in what it writes, moving
    /// between them as the author types.
    pub(crate) snippets: bool,
}

impl Offer<'_> {
    pub(super) fn complete(&self, spot: Spot, offset: usize) -> Option<Completion> {
        let renderer = self.renderer;
        let wraps = |name: &str| renderer.and_then(|renderer| renderer.wraps(name));
        let (text, body) = (self.text, self.body);
        match spot {
            Spot::Name {
                at,
                typed,
                opened,
                alone,
            } => {
                if !markdown::reads_a_call(text, body, at, offset, "", &wraps) {
                    return None;
                }
                let items = self.calls(opened, alone);
                Some(Completion::of(typed, items))
            }
            Spot::Value {
                at,
                name,
                typed,
                quote,
                named,
                place,
                given,
            } => {
                let closing = format!("{})", quote.map(String::from).unwrap_or_default());
                if !markdown::reads_a_call(text, body, at, offset, &closing, &wraps) {
                    return None;
                }
                if name == IN_SITE {
                    let page = self.page;
                    return Some(addresses(renderer, page, text, typed, "", quote));
                }
                let snippet = renderer?
                    .snippets()
                    .into_iter()
                    .find(|(s, _)| s.name == name)?
                    .0;
                let mut items = Vec::new();
                let param = named.clone().or_else(|| unnamed(snippet, place));
                if let Some(sort) = param.and_then(|param| {
                    (snippet.files.iter())
                        .find(|(name, _)| *name == param)
                        .map(|(_, sort)| *sort)
                }) {
                    items.extend(self.files(sort, quote, false));
                }
                let word = &text[typed.clone()];
                if named.is_none() && quote.is_none() && word.chars().all(is_name_char) {
                    items.extend(names_left(snippet, place, &given));
                }
                Some(Completion::of(typed, items))
            }
            Spot::Address {
                open,
                typed,
                definition,
            } => {
                let picture = match definition {
                    true => markdown::in_a_definition(text, body, offset, &wraps).then_some(false),
                    false => markdown::reads_a_link(text, body, open, offset, &wraps),
                }?;
                if picture {
                    let items = self.files(Sort::Picture, None, true);
                    return Some(Completion::of(typed, items));
                }
                let base = renderer.map(|r| r.settings().base()).unwrap_or_default();
                Some(addresses(renderer, self.page, text, typed, &base, None))
            }
            Spot::Fence {
                line,
                typed,
                written,
            } => {
                if !markdown::opens_a_fence(text, body, line, &wraps) {
                    return None;
                }
                let items = (LINE_WORDS.into_iter())
                    .filter(|name| !written.contains(name))
                    .map(|name| self.line_word(name))
                    .collect();
                Some(Completion::of(typed, items))
            }
        }
    }

    /// A word asking how a code block's lines are shown, written with its
    /// value where it has one: `numbered=true`, and `mark=` with the
    /// cursor after it.
    fn line_word(&self, name: &'static str) -> Item {
        let (label, insert, detail) = match name {
            "numbered" => (
                "numbered=true".to_owned(),
                "numbered=true".to_owned(),
                wording::OFFERED_NUMBERED,
            ),
            _ => (
                format!("{name}="),
                match self.snippets {
                    true => format!("{name}=$1"),
                    false => format!("{name}="),
                },
                wording::OFFERED_MARK,
            ),
        };
        Item {
            label,
            detail: Some(detail.to_owned()),
            kind: ItemKind::Field,
            insert,
            group: 0,
            text: false,
            filter: Some(name.to_owned()),
            about: None,
        }
    }

    /// The calls the writing may make: the site's snippets, then
    /// `@url(...)`, each written to be filled in.
    fn calls(&self, opened: bool, alone: bool) -> Vec<Item> {
        let mut items: Vec<Item> = (self.renderer.into_iter())
            .flat_map(Renderer::snippets)
            .filter(|(snippet, _)| snippet.readable)
            // What a template hands it, a page or a record, is no value
            // the writing can give.
            .filter(|(snippet, _)| {
                unnamed(snippet, 0).is_none_or(|first| !snippet.records.contains(&first))
            })
            .map(|(snippet, path)| {
                let wraps = !snippet.wrapped.is_empty();
                let first = unnamed(snippet, 0);
                Item {
                    label: format!("{}({})", snippet.name, takes_written(snippet)),
                    detail: Some(path),
                    kind: ItemKind::Snippet,
                    insert: self.call_written(
                        &snippet.name,
                        opened,
                        first.is_some(),
                        wraps && alone,
                    ),
                    group: 1,
                    text: false,
                    filter: Some(snippet.name.clone()),
                    about: None,
                }
            })
            .collect();
        items.push(Item {
            label: format!("{IN_SITE}(address)"),
            detail: Some(wording::OFFERED_IN_SITE.to_owned()),
            kind: ItemKind::Snippet,
            insert: match (opened, self.snippets) {
                (true, _) => IN_SITE.to_owned(),
                (false, true) => format!("{IN_SITE}($1)"),
                (false, false) => format!("{IN_SITE}("),
            },
            group: 0,
            text: false,
            filter: Some(IN_SITE.to_owned()),
            about: None,
        });
        items
    }

    /// A call to `name` as it is written in place of its name: its
    /// brackets, with the cursor in the quotes of its first value where
    /// it takes one without a name, and its `@end` where it wraps writing.
    fn call_written(&self, name: &str, opened: bool, quoted: bool, wraps: bool) -> String {
        if opened {
            return name.to_owned();
        }
        let call = match (self.snippets, quoted) {
            (true, true) => format!("{name}(\"$1\")"),
            (true, false) => format!("{name}($1)"),
            (false, _) => return format!("{name}("),
        };
        match wraps {
            true => format!("{call}\n$0\n@end"),
            false => call,
        }
    }

    /// The files beside the page of `sort`, by name, written in `quote`,
    /// or in quotes where there is none unless the name is `bare`, as a
    /// picture's address is.
    fn files(&self, sort: Sort, quote: Option<char>, bare: bool) -> Vec<Item> {
        let Some(renderer) = self.renderer else {
            return Vec::new();
        };
        let mut names: Vec<&str> = (renderer.site().files.iter())
            .filter_map(|file| file.path.strip_prefix(self.folder))
            .filter(|name| !name.contains('/') && crate::templates::fits(sort, name))
            .collect();
        names.sort_unstable();
        names.dedup();
        (names.into_iter())
            .map(|name| Item {
                label: name.to_owned(),
                detail: None,
                kind: ItemKind::File,
                insert: match (quote, bare) {
                    (None, true) => name.to_owned(),
                    (None, false) => format!("\"{}\"", in_quote(name, Some('"'))),
                    (Some(quote), _) => in_quote(name, Some(quote)),
                },
                group: 1,
                text: false,
                filter: None,
                about: None,
            })
            .collect()
    }
}

/// The name a call's value without a name at `place` is given, as the
/// snippet's `takes` line has it, or as the one name it reads.
pub(in crate::lsp) fn unnamed(snippet: &Snippet, place: usize) -> Option<String> {
    match &snippet.takes {
        Some(Takes::InOrder(names)) => names.get(place).cloned(),
        Some(Takes::AnyNumberOf(name)) => Some(name.clone()),
        None if snippet.params.len() == 1 && place == 0 => snippet.params.first().cloned(),
        None => None,
    }
}

/// What a snippet takes, as its call is shown in the list: the names its
/// `takes` line gives, or the one name it reads.
pub(in crate::lsp) fn takes_written(snippet: &Snippet) -> String {
    match &snippet.takes {
        Some(Takes::InOrder(names)) => names.join(", "),
        Some(Takes::AnyNumberOf(name)) => format!("{name}..."),
        None if snippet.params.len() == 1 => snippet.params[0].clone(),
        None => String::new(),
    }
}

/// The names the snippet reads that a call has given no value yet, after
/// `place` values without names, and by the names `given`, each written
/// with its `=`.
pub(super) fn names_left(snippet: &Snippet, place: usize, given: &[String]) -> Vec<Item> {
    let filled: Vec<String> = (0..place).filter_map(|at| unnamed(snippet, at)).collect();
    let any_number = matches!(snippet.takes, Some(Takes::AnyNumberOf(_)));
    (snippet.params.iter())
        .filter(|param| !given.contains(param) && !filled.contains(param))
        .filter(|param| !(any_number && unnamed(snippet, 0).as_ref() == Some(*param)))
        .map(|param| Item {
            label: param.clone(),
            detail: Some(wording::OFFERED_VALUE.to_owned()),
            kind: ItemKind::Field,
            insert: format!("{param}="),
            group: 2,
            text: false,
            filter: None,
            about: None,
        })
        .collect()
}

/// `value` written inside a call's `quote`, which a backslash keeps
/// from closing it; as it is where there is none.
pub(super) fn in_quote(value: &str, quote: Option<char>) -> String {
    match quote {
        Some(quote) => value
            .replace('\\', "\\\\")
            .replace(quote, &format!("\\{quote}")),
        None => value.to_owned(),
    }
}

/// The site's pages by their addresses under `base`, or, after a `#`
/// in what is typed, the headings of `page` or the page it names, `text`
/// being what `typed` is in; `typed` is
/// moved past the `#`. In `quote`, an address is written to read back
/// as itself.
pub(super) fn addresses(
    renderer: Option<&Renderer>,
    page: Option<PageId>,
    text: &str,
    typed: Range<usize>,
    base: &str,
    quote: Option<char>,
) -> Completion {
    let Some(renderer) = renderer else {
        return Completion {
            typed,
            items: Vec::new(),
            incomplete: false,
        };
    };
    let site = renderer.site();
    let written = &text[typed.clone()];
    if let Some(hash) = written.find('#') {
        let address = &written[..hash];
        let page = match address {
            "" => page,
            address => (site.pages.iter())
                .find(|(_, page)| format!("{base}{}", page.address.as_str()) == address)
                .map(|(id, _)| id),
        };
        let items = (page.into_iter())
            .flat_map(|page| renderer.headings(page))
            .map(|heading| Item {
                label: heading.id.clone(),
                detail: Some(heading.words),
                kind: ItemKind::Reference,
                insert: in_quote(&heading.id, quote),
                group: 1,
                text: false,
                filter: None,
                about: None,
            })
            .collect();
        return Completion {
            typed: typed.start + hash + 1..typed.end,
            items,
            incomplete: false,
        };
    }
    let mut pages: Vec<(String, PageId)> = (site.pages.iter())
        .filter(|(_, page)| page.lands())
        .map(|(id, page)| (format!("{base}{}", page.address.as_str()), id))
        .collect();
    pages.sort();
    pages.dedup_by(|a, b| a.0 == b.0);
    // A large site's pages are sent a part at a time, those whose
    // address begins with what is typed, the editor asking again as
    // more is typed.
    let incomplete = pages.len() > MOST_ADDRESSES;
    if incomplete {
        pages.retain(|(address, _)| address.starts_with(written));
        pages.truncate(MOST_ADDRESSES);
    }
    let items = (pages.into_iter())
        .map(|(address, id)| {
            let title = match &site.pages.get(id).title {
                Some(title) => title.value.clone(),
                None => renderer.named(id).0,
            };
            Item {
                label: address.clone(),
                detail: Some(title),
                kind: ItemKind::Reference,
                insert: in_quote(&address, quote),
                group: 1,
                text: false,
                filter: None,
                about: None,
            }
        })
        .collect();
    Completion {
        typed,
        items,
        incomplete,
    }
}
