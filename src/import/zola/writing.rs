//! Zola's tags in the writing, `{{ name(a="b") }}` and `{% name(a="b") %}`
//! up to `{% end %}`, and the components that replaced them in Zola 0.23,
//! `{{ <name a="b" /> }}` and `{% <name a="b"> %}` up to `{% </name> %}`,
//! each converted where the starter has an equal and otherwise a snippet
//! call, or left exactly as written; links to a page
//! by its file, `@/folder/page.md`, made the page's address; and the forms
//! of Zola's Markdown that Statac's reads differently, counted. Nothing
//! inside a code block or a code span is converted, but `{{/* name() */}}`,
//! Zola's way of showing a tag as text, is the tag as text wherever it
//! stands.

use std::collections::HashSet;
use std::ops::Range;

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag};

use super::{CODE_OPTIONS, EMOJI, LINK_NOWHERE, TAG_IN_CODE, TAG_LEFT};
use crate::import::writing::Marked;
use crate::import::{Finding, FindingKind, Spot};

/// What the writing is converted with.
pub struct Context<'a> {
    /// Whether the new site has the starter, whose snippets a call names
    /// or is kept clear of.
    pub starter: bool,
    /// The path the site is under, as `/blog`; empty at the top of its
    /// host.
    pub base: &'a str,
    /// The address, from the top of the site, of the page an `@/` link
    /// names by its path in `content/`, where there is one.
    pub refs: &'a dyn Fn(&str) -> Option<String>,
    /// The names of the site's own tags, from its templates.
    pub own: &'a [String],
    /// The tags some call anywhere in the site wraps writing with.
    pub wrapping: &'a HashSet<String>,
    pub emoji: bool,
}

/// A snippet call put in place of a tag.
#[derive(Clone, Debug)]
pub struct Call {
    pub snippet: String,
    /// The tag's name, as written.
    pub tag: String,
    /// The names it hands, in order.
    pub names: Vec<String>,
    pub wraps: bool,
    pub spot: Spot,
}

/// The writing converted, with what didn't convert.
pub struct Converted {
    pub writing: String,
    pub findings: Vec<Finding>,
    pub calls: Vec<Call>,
    /// Where each snippet call written here starts, and each `@end`, in
    /// order.
    pub called: Vec<usize>,
}

/// A value handed to a tag.
#[derive(Clone, PartialEq, Debug)]
enum Given {
    Text(String),
    /// A number, `true` or `false`, as written.
    Bare(String),
    List,
    /// What only a template can work out: an expression in braces, a
    /// spread, or a name handing the variable of that name.
    Worked,
}

/// What a tag is.
#[derive(Clone, PartialEq, Debug)]
enum Form {
    /// `{{ name() }}`.
    Alone,
    /// `{% name() %}`, wrapping writing up to `{% end %}`.
    Opening,
    /// `{% end %}`, or `{% </name> %}`.
    End,
    /// `{{/* name() */}}`, and the text Zola showed for it.
    Shown(String),
}

/// A tag as written.
#[derive(Clone, Debug)]
struct Written {
    start: usize,
    end: usize,
    form: Form,
    name: String,
    args: Vec<(String, Given)>,
}

/// The tag starting at `at`, where one does.
fn tag_at(text: &str, at: usize) -> Option<Written> {
    let rest = &text[at..];
    let (open, close) = match rest.get(..2)? {
        "{{" => ("{{", "}}"),
        "{%" => ("{%", "%}"),
        _ => return None,
    };
    let mut i = 2;
    let mut tag = i;
    skip(rest, &mut tag);
    if rest[tag..].starts_with('<') {
        return component_at(rest, at, close, tag + 1);
    }
    if rest[i..].starts_with("/*") {
        let inner_end = rest[i..].find("*/")? + i;
        let mut after = inner_end + 2;
        skip(rest, &mut after);
        if !rest[after..].starts_with(close) {
            return None;
        }
        return Some(Written {
            start: at,
            end: at + after + 2,
            form: Form::Shown(format!("{open}{}{close}", &rest[i + 2..inner_end])),
            name: String::new(),
            args: Vec::new(),
        });
    }
    skip(rest, &mut i);
    let name = ident(&rest[i..])?;
    i += name.len();
    skip(rest, &mut i);
    if open == "{%" && name == "end" && rest[i..].starts_with(close) {
        return Some(Written {
            start: at,
            end: at + i + 2,
            form: Form::End,
            name: String::new(),
            args: Vec::new(),
        });
    }
    if !rest[i..].starts_with('(') {
        return None;
    }
    i += 1;
    let mut args = Vec::new();
    loop {
        skip(rest, &mut i);
        if rest[i..].starts_with(')') {
            i += 1;
            break;
        }
        let arg = ident(&rest[i..])?;
        i += arg.len();
        skip(rest, &mut i);
        if !rest[i..].starts_with('=') {
            return None;
        }
        i += 1;
        skip(rest, &mut i);
        let (given, used) = value(&rest[i..])?;
        i += used;
        args.push((arg.to_owned(), given));
        skip(rest, &mut i);
        if rest[i..].starts_with(',') {
            i += 1;
        } else if !rest[i..].starts_with(')') {
            return None;
        }
    }
    skip(rest, &mut i);
    if !rest[i..].starts_with(close) {
        return None;
    }
    Some(Written {
        start: at,
        end: at + i + 2,
        form: match open {
            "{{" => Form::Alone,
            _ => Form::Opening,
        },
        name: name.to_owned(),
        args,
    })
}

/// The component call in `rest`, which starts at `at` in the writing and
/// closes with `close`, its tag's name starting at `i`.
fn component_at(rest: &str, at: usize, close: &str, mut i: usize) -> Option<Written> {
    let alone = close == "}}";
    if !alone && rest[i..].starts_with('/') {
        i += 1;
        let name = component_name(&rest[i..])?;
        i += name.len();
        skip(rest, &mut i);
        i += rest[i..].starts_with('>').then_some(1)?;
        skip(rest, &mut i);
        return rest[i..].starts_with(close).then(|| Written {
            start: at,
            end: at + i + 2,
            form: Form::End,
            name: name.to_owned(),
            args: Vec::new(),
        });
    }
    let name = component_name(&rest[i..])?;
    i += name.len();
    let mut args = Vec::new();
    loop {
        let before = i;
        skip(rest, &mut i);
        let ends = if alone { "/>" } else { ">" };
        if rest[i..].starts_with(ends) {
            i += ends.len();
            break;
        }
        if i == before {
            return None;
        }
        if rest[i..].starts_with('{') {
            i += braced(&rest[i..])?;
            args.push((String::new(), Given::Worked));
            continue;
        }
        let arg = attribute(&rest[i..])?;
        i += arg.len();
        if !rest[i..].starts_with('=') {
            args.push((arg.to_owned(), Given::Worked));
            continue;
        }
        i += 1;
        let given = match rest[i..].chars().next()? {
            '"' | '\'' => {
                let (given, used) = value(&rest[i..])?;
                i += used;
                given
            }
            '{' => {
                let used = braced(&rest[i..])?;
                let inner = rest[i + 1..i + used - 1].trim();
                i += used;
                match value(inner) {
                    Some((given @ (Given::Text(_) | Given::Bare(_)), n)) if n == inner.len() => {
                        given
                    }
                    _ => Given::Worked,
                }
            }
            _ => return None,
        };
        args.push((arg.to_owned(), given));
    }
    skip(rest, &mut i);
    rest[i..].starts_with(close).then(|| Written {
        start: at,
        end: at + i + 2,
        form: match alone {
            true => Form::Alone,
            false => Form::Opening,
        },
        name: name.to_owned(),
        args,
    })
}

/// A component's name at the start of `text`: names joined by `.`.
fn component_name(text: &str) -> Option<&str> {
    let mut end = ident(text)?.len();
    while text[end..].starts_with('.')
        && let Some(next) = ident(&text[end + 1..])
    {
        end += 1 + next.len();
    }
    Some(&text[..end])
}

/// An attribute's name at the start of `text`: a name, which may hold `-`
/// after its first character.
fn attribute(text: &str) -> Option<&str> {
    ident(text)?;
    let end = text
        .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '-')))
        .unwrap_or(text.len());
    Some(&text[..end])
}

/// The length of the braces opening `text`, with what they hold, braces
/// within and quoted text included.
fn braced(text: &str) -> Option<usize> {
    let mut depth = 0;
    let mut quote = None;
    let mut chars = text.char_indices();
    while let Some((i, c)) = chars.next() {
        match (quote, c) {
            (Some(_), '\\') => {
                chars.next();
            }
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'' | '`') => quote = Some(c),
            (None, '{') => depth += 1,
            (None, '}') => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
    }
    None
}

/// The names of the components `template` defines, each written
/// `{% component name(...) %}`.
pub fn components(template: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut at = 0;
    while let Some(found) = template[at..].find("{%") {
        let mut i = at + found + 2;
        at = i;
        i += usize::from(template[i..].starts_with('-'));
        skip(template, &mut i);
        let Some(rest) = template[i..].strip_prefix("component") else {
            continue;
        };
        let mut j = 0;
        skip(rest, &mut j);
        if j == 0 {
            continue;
        }
        if let Some(name) = component_name(&rest[j..])
            && rest[j + name.len()..].trim_start().starts_with('(')
            && !names.iter().any(|n| n == name)
        {
            names.push(name.to_owned());
        }
    }
    names
}

fn skip(text: &str, at: &mut usize) {
    while let Some(c) = text[*at..].chars().next().filter(|c| c.is_whitespace()) {
        *at += c.len_utf8();
    }
}

/// The name at the start of `text`: a letter or `_`, then letters,
/// figures and `_`.
fn ident(text: &str) -> Option<&str> {
    if !text.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') {
        return None;
    }
    let end = text
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(text.len());
    Some(&text[..end])
}

/// A value at the start of `text`, with how much of it it took.
fn value(text: &str) -> Option<(Given, usize)> {
    let first = text.chars().next()?;
    match first {
        '"' | '\'' | '`' => {
            let mut out = String::new();
            let mut chars = text.char_indices().skip(1);
            while let Some((i, c)) = chars.next() {
                match c {
                    '\\' if first == '"' => out.push(chars.next()?.1),
                    c if c == first => return Some((Given::Text(out), i + 1)),
                    c => out.push(c),
                }
            }
            None
        }
        '[' => {
            let mut at = 1;
            loop {
                skip(text, &mut at);
                if text[at..].starts_with(']') {
                    return Some((Given::List, at + 1));
                }
                let (_, used) = value(&text[at..])?;
                at += used;
                skip(text, &mut at);
                if text[at..].starts_with(',') {
                    at += 1;
                }
            }
        }
        _ => {
            let end = text
                .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+')))
                .unwrap_or(text.len());
            let word = &text[..end];
            let number = word.parse::<f64>().is_ok();
            (end > 0 && (number || word == "true" || word == "false"))
                .then(|| (Given::Bare(word.to_owned()), end))
        }
    }
}

/// Where the tag opened before `from` is closed: the `{% end %}`, counting
/// others opened between.
fn end_of(text: &str, from: usize) -> Option<Written> {
    let mut depth = 0;
    let mut at = from;
    while let Some(found) = text[at..].find("{%") {
        let start = at + found;
        match tag_at(text, start) {
            Some(tag) => {
                match tag.form {
                    Form::End if depth == 0 => return Some(tag),
                    Form::End => depth -= 1,
                    Form::Opening => depth += 1,
                    _ => {}
                }
                at = tag.end;
            }
            None => at = start + 2,
        }
    }
    None
}

/// The names of the tags in `body` that wrap writing.
pub fn paired(body: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut at = 0;
    while let Some(found) = body[at..].find("{%") {
        let start = at + found;
        match tag_at(body, start) {
            Some(tag) => {
                if tag.form == Form::Opening && !names.contains(&tag.name) {
                    names.push(tag.name.clone());
                }
                at = tag.end;
            }
            None => at = start + 2,
        }
    }
    names
}

fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_HEADING_ATTRIBUTES
}

/// The ranges of `text` Markdown reads as code.
fn code_ranges(text: &str) -> Vec<Range<usize>> {
    Parser::new_ext(text, options())
        .into_offset_iter()
        .filter(|(event, _)| matches!(event, Event::Start(Tag::CodeBlock(_)) | Event::Code(_)))
        .map(|(_, range)| range)
        .collect()
}

fn within(ranges: &[Range<usize>], at: usize) -> bool {
    ranges.iter().any(|range| range.contains(&at))
}

/// Whether only spaces stand between the start of `at`'s line and `at`.
fn opens_line(text: &str, at: usize) -> bool {
    let line = text[..at].rfind('\n').map_or(0, |i| i + 1);
    text[line..at].trim().is_empty()
}

/// Whether only spaces stand between `at` and the end of its line.
fn ends_line(text: &str, at: usize) -> bool {
    let end = text[at..].find('\n').map_or(text.len(), |i| at + i);
    text[at..end].trim().is_empty()
}

fn finding(kind: &'static FindingKind, spot: Spot) -> Finding {
    Finding {
        kind,
        about: None,
        spots: vec![spot],
    }
}

/// What a tag left open stands for when its `{% end %}` comes.
enum Open {
    End,
    Left,
}

/// Converts the writing of `file`, whose first line is line `first_line`
/// of the file.
pub fn convert(body: &str, first_line: usize, file: &str, cx: &Context) -> Converted {
    // Each step keeps every line where it was, so a place is told by
    // counting the lines of the writing the step is given.
    let spotter = |text: &str| {
        let text = text.to_owned();
        move |at: usize, written: &str| Spot {
            file: file.to_owned(),
            line: Some(first_line + text[..at].matches('\n').count()),
            text: written.to_owned(),
            times: None,
        }
    };
    let mut findings = Vec::new();
    let body = fences_bare(body, &spotter(body), &mut findings);
    let body = links_and_more(&body, cx, &spotter(&body), &mut findings);
    let body = body.as_str();
    let spot = spotter(body);
    if cx.emoji {
        findings.extend(emoji(body, &spot));
    }
    let code = code_ranges(body);
    let mut calls = Vec::new();
    let mut out = Marked::default();
    let mut open: Vec<Open> = Vec::new();
    let mut copied = 0;
    let mut at = 0;
    while let Some(found) = body[at..].find(['{']) {
        let start = at + found;
        let Some(tag) = tag_at(body, start) else {
            at = start + 1;
            continue;
        };
        let written = &body[tag.start..tag.end];
        let mut replace = |out: &mut Marked, with: Marked, resume: usize| {
            out.push_str(&body[copied..start]);
            out.push(with);
            copied = resume;
        };
        at = tag.end;
        if let Form::Shown(shown) = &tag.form {
            replace(&mut out, shown.clone().into(), tag.end);
            continue;
        }
        if within(&code, start) {
            findings.push(finding(&TAG_IN_CODE, spot(start, written)));
            continue;
        }
        let left =
            |findings: &mut Vec<Finding>| findings.push(finding(&TAG_LEFT, spot(start, written)));
        match tag.form {
            Form::End => match open.pop() {
                Some(Open::End) => replace(&mut out, Marked::call("@end"), tag.end),
                _ => left(&mut findings),
            },
            Form::Opening => {
                let close = end_of(body, tag.end);
                let alone = close.as_ref().is_some_and(|close| {
                    opens_line(body, start)
                        && ends_line(body, tag.end)
                        && opens_line(body, close.start)
                        && ends_line(body, close.end)
                });
                match snippet_call(&tag, cx).filter(|_| alone) {
                    Some((text, call)) => {
                        calls.push(Call {
                            spot: spot(start, written),
                            ..call
                        });
                        open.push(Open::End);
                        replace(&mut out, text, tag.end);
                    }
                    None => {
                        left(&mut findings);
                        if close.is_some() {
                            open.push(Open::Left);
                        }
                    }
                }
            }
            _ => {
                // Without the starter there is no snippet for a video to
                // call, so it is called as any other tag is.
                let starters = cx.starter && !cx.own.contains(&tag.name);
                let video = match (starters, tag.name.as_str(), tag.args.as_slice()) {
                    (true, "youtube" | "vimeo", [(name, Given::Text(id))]) if name == "id" => {
                        Some(Marked::call(format!("@{}({})", tag.name, quoted(id))))
                    }
                    (true, "youtube" | "vimeo", _) => None,
                    _ => {
                        let wraps = cx.wrapping.contains(&tag.name);
                        let alone = opens_line(body, start) && ends_line(body, tag.end);
                        let made = snippet_call(&tag, cx).filter(|_| !wraps || alone);
                        made.map(|(mut text, call)| {
                            calls.push(Call {
                                spot: spot(start, written),
                                ..call
                            });
                            if wraps {
                                text.push_str("\n");
                                text.push(Marked::call("@end"));
                            }
                            text
                        })
                    }
                };
                match video {
                    Some(text) => replace(&mut out, text, tag.end),
                    None => left(&mut findings),
                }
            }
        }
    }
    out.push_str(&body[copied..]);
    findings.sort_by_key(|finding| finding.spots[0].line);
    Converted {
        writing: out.text,
        findings,
        calls,
        called: out.called,
    }
}

/// `text` as a value in a snippet call, in single quotes.
fn quoted(text: &str) -> String {
    format!("'{}'", text.replace('\\', "\\\\").replace('\'', "\\'"))
}

/// The snippet a tag's call names: its own name, kept clear of the
/// starter's where the new site has it.
pub fn snippet_name(tag: &str, starter: bool) -> String {
    match (starter && crate::new::has_snippet(tag)) || tag.starts_with('_') {
        true => format!("old_{tag}"),
        false => tag.to_owned(),
    }
}

/// A tag as a call to a snippet of its name, where its values can be
/// handed.
fn snippet_call(tag: &Written, cx: &Context) -> Option<(Marked, Call)> {
    // A snippet's name holds no `.`.
    if tag.name.contains('.') {
        return None;
    }
    let mut names = Vec::new();
    let mut handed = Vec::new();
    for (name, given) in &tag.args {
        let value = match given {
            Given::Text(text) if !text.contains(['\n', '\r']) => quoted(text),
            Given::Bare(bare) => bare.clone(),
            _ => return None,
        };
        if names.contains(name) {
            return None;
        }
        handed.push(format!("{name}={value}"));
        names.push(name.clone());
    }
    let snippet = snippet_name(&tag.name, cx.starter);
    Some((
        Marked::call(format!("@{snippet}({})", handed.join(", "))),
        Call {
            snippet,
            tag: tag.name.clone(),
            names,
            wraps: cx.wrapping.contains(&tag.name),
            spot: Spot {
                file: String::new(),
                line: None,
                text: String::new(),
                times: None,
            },
        },
    ))
}

/// The writing with Zola's options after a comma on each fenced code
/// block's first line, as in ```` ```rust,linenos ````, taken off, each
/// told.
fn fences_bare(
    body: &str,
    spot: &dyn Fn(usize, &str) -> Spot,
    findings: &mut Vec<Finding>,
) -> String {
    let mut cuts: Vec<Range<usize>> = Vec::new();
    for (event, range) in Parser::new_ext(body, options()).into_offset_iter() {
        let Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) = event else {
            continue;
        };
        if !info.contains(',') {
            continue;
        }
        let line_end = body[range.start..]
            .find('\n')
            .map_or(body.len(), |i| range.start + i);
        let line = &body[range.start..line_end];
        let Some(comma) = line.find(',') else {
            continue;
        };
        findings.push(finding(&CODE_OPTIONS, spot(range.start, line.trim())));
        let cut_to = line.trim_end_matches('\r').len();
        cuts.push(range.start + comma..range.start + cut_to);
    }
    let mut out = String::with_capacity(body.len());
    let mut copied = 0;
    for cut in cuts {
        out.push_str(&body[copied..cut.start]);
        copied = cut.end;
    }
    out.push_str(&body[copied..]);
    out
}

/// The writing with each `@/` link outside code made the address of the
/// page it names, and each `<!-- more -->` written as Statac spells it: the
/// shared part sees to one standing where Statac takes none.
fn links_and_more(
    body: &str,
    cx: &Context,
    spot: &dyn Fn(usize, &str) -> Spot,
    findings: &mut Vec<Finding>,
) -> String {
    let code = code_ranges(body);
    let mut out = String::with_capacity(body.len());
    let mut copied = 0;
    let mut at = 0;
    while let Some(found) = body[at..].find(['@', '<']) {
        let start = at + found;
        at = start + 1;
        if within(&code, start) {
            continue;
        }
        if body[start..].starts_with("<!--") {
            let mut i = start + 4;
            skip(body, &mut i);
            if !body[i..].starts_with("more") {
                continue;
            }
            i += 4;
            skip(body, &mut i);
            if !body[i..].starts_with("-->") {
                continue;
            }
            let end = i + 3;
            at = end;
            out.push_str(&body[copied..start]);
            out.push_str("<!--more-->");
            copied = end;
            continue;
        }
        if !body[start..].starts_with("@/") {
            continue;
        }
        let before = body[..start].trim_end_matches([' ', '\t']);
        let linked = before.ends_with(['(', '<']) || before.ends_with("]:");
        if !linked {
            continue;
        }
        let end = body[start..]
            .find(|c: char| c.is_whitespace() || matches!(c, ')' | '>' | '"' | '\''))
            .map_or(body.len(), |n| start + n);
        let written = &body[start..end];
        let (path, anchor) = match written[2..].split_once('#') {
            Some((path, anchor)) => (path, format!("#{anchor}")),
            None => (&written[2..], String::new()),
        };
        match (cx.refs)(path) {
            Some(address) => {
                out.push_str(&body[copied..start]);
                out.push_str(&format!("{}{address}{anchor}", cx.base));
                copied = end;
            }
            None => findings.push(finding(&LINK_NOWHERE, spot(start, written))),
        }
        at = end;
    }
    out.push_str(&body[copied..]);
    out
}

/// Each `:name:` outside code that Zola showed as an emoji.
fn emoji(body: &str, spot: &dyn Fn(usize, &str) -> Spot) -> Vec<Finding> {
    let code = code_ranges(body);
    let mut found = Vec::new();
    let mut at = 0;
    for line in body.split_inclusive('\n') {
        let start = at;
        at += line.len();
        for (offset, name) in emoji_names(line.trim_end_matches(['\n', '\r'])) {
            if !within(&code, start + offset) {
                found.push(finding(&EMOJI, spot(start + offset, name)));
            }
        }
    }
    found
}

/// Each `:name:` in `line`, by its offset.
fn emoji_names(line: &str) -> Vec<(usize, &str)> {
    let mut names = Vec::new();
    let bytes = line.as_bytes();
    let mut at = 0;
    while let Some(found) = line[at..].find(':') {
        let start = at + found;
        let after = start + 1;
        let length = line[after..]
            .find(|c: char| {
                !(c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '+' | '-'))
            })
            .unwrap_or(line.len() - after);
        let end = after + length;
        let before_ok = start == 0 || !bytes[start - 1].is_ascii_alphanumeric();
        if length > 0
            && bytes.get(end) == Some(&b':')
            && before_ok
            && line[after..end]
                .starts_with(|c: char| c.is_ascii_lowercase() || c == '+' || c == '-')
        {
            names.push((start, &line[start..=end]));
            at = end + 1;
        } else {
            at = after;
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tag_is_read_with_its_values() {
        let text = r#"{{ gist(url="https://example.org/a", n=2, on=true, list=[1, "a"]) }}"#;
        let tag = tag_at(text, 0).unwrap();
        assert_eq!(tag.end, text.len());
        assert_eq!(tag.form, Form::Alone);
        assert_eq!(tag.name, "gist");
        assert_eq!(
            tag.args,
            vec![
                ("url".into(), Given::Text("https://example.org/a".into())),
                ("n".into(), Given::Bare("2".into())),
                ("on".into(), Given::Bare("true".into())),
                ("list".into(), Given::List),
            ]
        );
        let opening = tag_at("{% note(kind='tip') %}", 0).unwrap();
        assert_eq!(
            (opening.form, opening.name.as_str()),
            (Form::Opening, "note")
        );
        assert_eq!(tag_at("{%end%}", 0).unwrap().form, Form::End);
        assert_eq!(
            tag_at("{{/* youtube(id=\"a\") */}}", 0).unwrap().form,
            Form::Shown("{{ youtube(id=\"a\") }}".into())
        );
        assert!(tag_at("{{ page.title }}", 0).is_none());
        assert!(tag_at("{{ a(b=\"open) }}", 0).is_none());
    }

    #[test]
    fn a_component_call_is_read_with_its_values() {
        let text = r#"{{ <ui.button label="a" n={2} on={ true } text={'b'} /> }}"#;
        let tag = tag_at(text, 0).unwrap();
        assert_eq!(tag.end, text.len());
        assert_eq!((tag.form, tag.name.as_str()), (Form::Alone, "ui.button"));
        assert_eq!(
            tag.args,
            vec![
                ("label".into(), Given::Text("a".into())),
                ("n".into(), Given::Bare("2".into())),
                ("on".into(), Given::Bare("true".into())),
                ("text".into(), Given::Text("b".into())),
            ]
        );
        let tight = tag_at(r#"{{<button label="a"/>}}"#, 0).unwrap();
        assert_eq!((tight.form, tight.args.len()), (Form::Alone, 1));
        // What only a template can work out is read, to be told.
        let worked = tag_at("{% <aside title {...values} n={page.n} data-x='y'> %}", 0).unwrap();
        assert_eq!(worked.form, Form::Opening);
        assert_eq!(
            worked.args,
            vec![
                ("title".into(), Given::Worked),
                (String::new(), Given::Worked),
                ("n".into(), Given::Worked),
                ("data-x".into(), Given::Text("y".into())),
            ]
        );
        let end = tag_at("{% </aside> %}", 0).unwrap();
        assert_eq!((end.form, end.name.as_str()), (Form::End, "aside"));
        for text in [
            "{{ <aside> }}",
            "{% <aside /> %}",
            "{{ </aside> }}",
            "{% </aside %}",
            "{{ < aside /> }}",
            "{{ <aside a=\"b\"c=\"d\" /> }}",
            "{{ <aside a=b /> }}",
            "{{ <aside a={b /> }}",
        ] {
            assert!(tag_at(text, 0).is_none(), "{text}");
        }
    }

    #[test]
    fn a_template_names_the_components_it_defines() {
        let template = "{% component quote(author) %}{{ body }}{% endcomponent %}\n\
                        {%- component ui.button( label ) -%}x{%- endcomponent -%}\n\
                        {% component quote(author) %}{% endcomponent %}\n\
                        {% components %}{% component %}{{ component(a) }}";
        assert_eq!(components(template), ["quote", "ui.button"]);
    }

    #[test]
    fn private_use_characters_are_kept_and_never_taken_for_a_call() {
        let wrapping = HashSet::from(["aside".to_owned()]);
        let cx = Context {
            starter: true,
            base: "",
            refs: &|_| None,
            own: &[],
            wrapping: &wrapping,
            emoji: false,
        };
        // The writing, what it becomes, and the line from each call made.
        let cases: [(&str, &str, &[&str]); 5] = [
            (
                "A \u{E000} and \u{E001}.\n\n<div title=\"\u{E001}\">\u{E000}</div>\n",
                "A \u{E000} and \u{E001}.\n\n<div title=\"\u{E001}\">\u{E000}</div>\n",
                &[],
            ),
            (
                "```\n\u{E001}\u{E000}\n```\n",
                "```\n\u{E001}\u{E000}\n```\n",
                &[],
            ),
            (
                "\u{E000}{{ youtube(id=\"\u{E000}\") }}\n",
                "\u{E000}@youtube('\u{E000}')\n",
                &["@youtube('\u{E000}')"],
            ),
            (
                "{% aside(a=\"\u{E001}\") %}\n\u{E000}\n{% end %}\n",
                "@aside(a='\u{E001}')\n\u{E000}\n@end\n",
                &["@aside(a='\u{E001}')", "@end"],
            ),
            (
                "{{ aside(a=\"\u{E000}\") }}\n",
                "@aside(a='\u{E000}')\n@end\n",
                &["@aside(a='\u{E000}')", "@end"],
            ),
        ];
        for (body, writing, called) in cases {
            let converted = convert(body, 1, "a.md", &cx);
            assert_eq!(converted.writing, writing, "{body:?}");
            let lines: Vec<&str> = (converted.called.iter())
                .map(|at| writing[*at..].lines().next().unwrap_or_default())
                .collect();
            assert_eq!(lines, called, "{body:?}");
        }
    }

    #[test]
    fn a_tag_opened_is_ended_past_others_opened_between() {
        let text = "{% a() %}x{% b() %}y{% end %}z{% end %}";
        let end = end_of(text, 9).unwrap();
        assert_eq!(&text[end.start..], "{% end %}");
        assert_eq!(paired(text), vec!["a".to_owned(), "b".to_owned()]);
    }
}
