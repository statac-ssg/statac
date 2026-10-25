//! Hugo's tags in the writing, `{{< name >}}` and `{{% name %}}`, each
//! converted where Statac's writing has an equal, made a snippet call
//! where it can be, and otherwise left exactly as written, and the forms of Hugo's Markdown that
//! Statac's reads differently, counted. Nothing inside a code block or a
//! code span is converted, but `{{</* name */>}}`, Hugo's way of showing a
//! tag as text, is the tag as text wherever it stands.

use std::collections::HashSet;
use std::ops::Range;

use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};

use super::{
    ATTRIBUTE_LINES, CODE_OPTIONS, EMOJI, LINK_NOWHERE, RAW_HTML, SUMMARY_CUT, TAG_IN_ADDRESS,
    TAG_IN_CODE, TAG_LEFT,
};
use crate::html::escaped;
use crate::import::common::{emoji_names, ends_line, opens_line};
use crate::import::liquid::{code_ranges, finding, options, quoted, within};
use crate::import::writing::Marked;
use crate::import::{Finding, Spot};

/// What the writing's tags are converted with.
pub(super) struct Context<'a> {
    /// The path the site is under, as `/blog`; empty at the top of its
    /// host.
    pub base: &'a str,
    /// The site's whole address, with its path, where it is known.
    pub whole: Option<&'a str>,
    /// The address, from the top of the site, of the page a `ref` names,
    /// in the language named, where there is one.
    pub refs: &'a dyn Fn(&str, Option<&str>) -> Option<String>,
    /// A value of the page's own, or else the site's, by its name.
    pub param: &'a dyn Fn(&str) -> Option<String>,
    /// The names of the site's own tags, from its layouts.
    pub own: &'a [String],
    /// The tags some call anywhere in the site wraps writing with.
    pub wrapping: &'a HashSet<String>,
    pub html: bool,
    pub unsafe_html: bool,
    pub emoji: bool,
    pub block_attributes: bool,
    /// The words Hugo cut a summary at, where the page's summary is one
    /// Hugo cut.
    pub summary_length: Option<usize>,
}

/// The address a `ref` or `relref` tag is written as: from the top of the
/// site's host with the path the site is under, whole for `ref` where the
/// site's address is known, and a place on its own page as it is. None
/// inside where it names no page, and none at all where its arguments
/// aren't read.
fn referred(tag: &Written, cx: &Context) -> Option<Option<String>> {
    let mut path = None;
    let mut language = None;
    for (name, value) in &tag.args {
        match name.as_deref() {
            None | Some("path") if path.is_none() => path = Some(value.as_str()),
            Some("lang") => language = Some(value.as_str()),
            _ => return None,
        }
    }
    let address = (cx.refs)(path?, language);
    Some(address.map(|address| match (tag.name.as_str(), cx.whole) {
        _ if address.starts_with('#') => address,
        ("ref", Some(whole)) => format!("{whole}{address}"),
        _ => format!("{}{address}", cx.base),
    }))
}

/// Whether the tag at `at` stands inside an address: a link's, after
/// `](`; a link definition's, after `[label]:`; or an attribute's, inside
/// its quotes.
fn in_address(body: &str, at: usize) -> bool {
    let line = body[..at].rfind('\n').map_or(0, |i| i + 1);
    let before = &body[line..at];
    let unbroken = |address: &str| !address.contains(char::is_whitespace);
    if let Some(open) = before.rfind("](")
        && !before[open..].contains(')')
        && unbroken(&before[open + 2..])
    {
        return true;
    }
    let defined = before.trim_start();
    if defined.starts_with('[')
        && let Some(label) = defined.find("]:")
        && unbroken(defined[label + 2..].trim_start())
    {
        return true;
    }
    ['"', '\''].iter().any(|quote| {
        before
            .rfind(&format!("={quote}"))
            .is_some_and(|open| !before[open + 2..].contains(*quote))
    })
}

/// A snippet call put in place of a tag.
#[derive(Clone, Debug)]
pub(super) struct Call {
    pub snippet: String,
    /// The tag's name, as written.
    pub tag: String,
    /// The names it hands, in order.
    pub names: Vec<String>,
    pub wraps: bool,
    pub spot: Spot,
}

/// The writing converted, with what didn't convert.
pub(super) struct Converted {
    pub writing: String,
    pub findings: Vec<Finding>,
    pub calls: Vec<Call>,
    /// Where each code fence written here starts in the writing, in
    /// order.
    pub fences: Vec<usize>,
    /// Where each snippet call written here starts, and each `@end`, in
    /// order.
    pub called: Vec<usize>,
}

/// The names Hugo's own tags give the values they take without one.
const POSITIONS: &[(&str, &[&str])] = &[
    ("gist", &["user", "id", "file"]),
    ("instagram", &["id"]),
    ("tweet", &["id"]),
    ("x", &["user", "id"]),
    ("qr", &["text"]),
    ("vimeo", &["id"]),
    ("youtube", &["id"]),
];

/// A tag as written.
#[derive(Clone, Debug)]
struct Written {
    start: usize,
    end: usize,
    /// The text Hugo showed for `{{</* name */>}}`.
    shown: Option<String>,
    closing: bool,
    self_closing: bool,
    name: String,
    args: Vec<(Option<String>, String)>,
}

/// The tag starting at `at`, where one does.
fn tag_at(text: &str, at: usize) -> Option<Written> {
    let rest = &text[at..];
    let percent = rest.starts_with("{{%");
    if !percent && !rest.starts_with("{{<") {
        return None;
    }
    let close = if percent { "%}}" } else { ">}}" };
    let open = &rest[..3];
    let mut i = 3;
    let skip = |i: &mut usize| {
        while rest[*i..].starts_with(char::is_whitespace) {
            *i += rest[*i..].chars().next().map_or(1, char::len_utf8);
        }
    };
    if rest[i..].starts_with("/*") {
        let inner_end = rest[i..].find("*/")? + i;
        let mut after = inner_end + 2;
        skip(&mut after);
        if !rest[after..].starts_with(close) {
            return None;
        }
        return Some(Written {
            start: at,
            end: at + after + 3,
            shown: Some(format!("{open}{}{close}", &rest[i + 2..inner_end])),
            closing: false,
            self_closing: false,
            name: String::new(),
            args: Vec::new(),
        });
    }
    skip(&mut i);
    let closing = rest[i..].starts_with('/');
    if closing {
        i += 1;
        skip(&mut i);
    }
    let name_end = rest[i..]
        .find(|c: char| c.is_whitespace() || c == '"' || c == '`')
        .map_or(rest.len(), |n| i + n);
    let mut name = &rest[i..name_end];
    if let Some(at_close) = name.find(close) {
        name = &name[..at_close];
    }
    let mut self_closing = false;
    if let Some(bare) = name.strip_suffix('/') {
        name = bare;
        self_closing = true;
    }
    if name.is_empty() {
        return None;
    }
    i += name.len() + usize::from(self_closing);
    let mut args = Vec::new();
    loop {
        skip(&mut i);
        if rest[i..].starts_with(close) {
            break;
        }
        if rest[i..].starts_with('/') {
            let mut after = i + 1;
            skip(&mut after);
            if rest[after..].starts_with(close) {
                self_closing = true;
                i = after;
                break;
            }
        }
        let (first, used, bare) = token(&rest[i..], close)?;
        i += used;
        if bare && rest[i..].starts_with('=') {
            i += 1;
            let (value, used, _) = token(&rest[i..], close)?;
            i += used;
            args.push((Some(first), value));
        } else {
            args.push((None, first));
        }
    }
    Some(Written {
        start: at,
        end: at + i + 3,
        shown: None,
        closing,
        self_closing,
        name: name.to_owned(),
        args,
    })
}

/// A value at the start of `text`: quoted, raw between backticks, or bare;
/// with how much of `text` it took, and whether it was bare.
fn token(text: &str, close: &str) -> Option<(String, usize, bool)> {
    let mut chars = text.char_indices();
    match chars.next()? {
        (_, '"') => {
            let mut value = String::new();
            while let Some((i, c)) = chars.next() {
                match c {
                    '\\' => value.push(chars.next()?.1),
                    '"' => return Some((value, i + 1, false)),
                    c => value.push(c),
                }
            }
            None
        }
        (_, '`') => {
            let end = text[1..].find('`')? + 1;
            Some((text[1..end].to_owned(), end + 1, false))
        }
        _ => {
            let end = text
                .char_indices()
                .find(|&(i, c)| c.is_whitespace() || c == '=' || text[i..].starts_with(close))
                .map_or(text.len(), |(i, _)| i);
            (end > 0).then(|| (text[..end].to_owned(), end, true))
        }
    }
}

/// Where the tag `name` opened before `from` is closed: the closing tag,
/// counting others of its name opened between.
fn closing_of(text: &str, from: usize, name: &str) -> Option<Written> {
    let mut depth = 0;
    let mut at = from;
    while let Some(found) = text[at..].find("{{") {
        let start = at + found;
        match tag_at(text, start) {
            Some(tag) if tag.shown.is_none() && tag.name == name => {
                match (tag.closing, tag.self_closing) {
                    (true, _) if depth == 0 => return Some(tag),
                    (true, _) => depth -= 1,
                    (false, false) => depth += 1,
                    (false, true) => {}
                }
                at = tag.end;
            }
            Some(tag) => at = tag.end,
            None => at = start + 2,
        }
    }
    None
}

/// The names of the tags closed somewhere in `body`, so wrapping writing.
pub(super) fn paired(body: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut at = 0;
    while let Some(found) = body[at..].find("{{") {
        let start = at + found;
        match tag_at(body, start) {
            Some(tag) => {
                if tag.closing && !names.contains(&tag.name) {
                    names.push(tag.name.clone());
                }
                at = tag.end;
            }
            None => at = start + 2,
        }
    }
    names
}

/// What a tag left open stands for when its closing tag comes.
enum Open {
    End,
    Details,
    Left,
}

/// Converts the writing of `file`, whose first line is line `first_line`
/// of the file.
pub(super) fn convert(body: &str, first_line: usize, file: &str, cx: &Context) -> Converted {
    let line_of = |at: usize| first_line + body[..at].matches('\n').count();
    let spot = |at: usize, text: &str| Spot {
        file: file.to_owned(),
        line: Some(line_of(at)),
        text: text.to_owned(),
        times: None,
    };
    let mut findings = Vec::new();
    let body = match cx.html {
        true => body.to_owned(),
        false => fences_bare(body, &spot, &mut findings),
    };
    let body = body.as_str();
    // Offsets from here on are into the writing with its options cut,
    // which has the same lines.
    let spot = |at: usize, text: &str| Spot {
        file: file.to_owned(),
        line: Some(first_line + body[..at].matches('\n').count()),
        text: text.to_owned(),
        times: None,
    };
    findings.extend(markdown_forms(body, cx, &spot));
    let code = match cx.html {
        true => Vec::new(),
        false => code_ranges(body),
    };
    let mut calls = Vec::new();
    let mut out = Marked::default();
    let mut open: Vec<(String, Open)> = Vec::new();
    let mut copied = 0;
    let mut at = 0;
    while let Some(found) = body[at..].find("{{") {
        let start = at + found;
        let Some(tag) = tag_at(body, start) else {
            at = start + 2;
            continue;
        };
        let written = &body[tag.start..tag.end];
        let mut replace = |out: &mut Marked, with: Marked, resume: usize| {
            out.push_str(&body[copied..start]);
            out.push(with);
            copied = resume;
        };
        if let Some(shown) = &tag.shown {
            replace(&mut out, shown.clone().into(), tag.end);
            at = tag.end;
            continue;
        }
        if within(&code, start) {
            findings.push(finding(&TAG_IN_CODE, spot(start, written)));
            at = tag.end;
            continue;
        }
        let left =
            |findings: &mut Vec<Finding>, kind| findings.push(finding(kind, spot(start, written)));
        if tag.closing {
            match open.iter().rposition(|(name, _)| *name == tag.name) {
                Some(found) => {
                    let (_, what) = open.remove(found);
                    match what {
                        Open::End => replace(&mut out, Marked::call("@end"), tag.end),
                        Open::Details => replace(&mut out, "</details>".to_owned().into(), tag.end),
                        Open::Left => left(&mut findings, &TAG_LEFT),
                    }
                }
                None => left(&mut findings, &TAG_LEFT),
            }
            at = tag.end;
            continue;
        }
        let closing = match tag.self_closing {
            true => None,
            false => closing_of(body, tag.end, &tag.name),
        };
        let own = cx.own.contains(&tag.name);
        let builtin = match tag.name.as_str() {
            _ if own => "",
            name => name,
        };
        // A call can't run inside an address, so a tag there that would
        // become one is left for the author.
        if !matches!(builtin, "ref" | "relref" | "param") && in_address(body, start) {
            left(&mut findings, &TAG_IN_ADDRESS);
            if closing.is_some() {
                open.push((tag.name.clone(), Open::Left));
            }
            at = tag.end;
            continue;
        }
        let done = match builtin {
            "ref" | "relref" if closing.is_none() => match referred(&tag, cx) {
                Some(Some(address)) => Some((address.into(), tag.end)),
                Some(None) => {
                    left(&mut findings, &LINK_NOWHERE);
                    at = tag.end;
                    continue;
                }
                None => None,
            },
            "highlight" => closing
                .as_ref()
                .filter(|close| opens_line(body, start) && ends_line(body, close.end))
                .and_then(|close| {
                    let (language, options) = match tag.args.as_slice() {
                        [] => ("", ""),
                        [(None, language)] => (language.as_str(), ""),
                        [(None, language), (None, options)] => {
                            (language.as_str(), options.as_str())
                        }
                        _ => return None,
                    };
                    if !options.trim().is_empty() {
                        findings.push(finding(&CODE_OPTIONS, spot(start, written)));
                    }
                    Some((fenced(language, &body[tag.end..close.start]), close.end))
                }),
            "details" => closing
                .as_ref()
                .filter(|close| {
                    opens_line(body, start)
                        && ends_line(body, tag.end)
                        && opens_line(body, close.start)
                        && ends_line(body, close.end)
                })
                .and_then(|_| details(&tag))
                .map(|text| {
                    open.push((tag.name.clone(), Open::Details));
                    (text.into(), tag.end)
                }),
            "param" if closing.is_none() => match tag.args.as_slice() {
                [(None | Some(_), name)] => (cx.param)(name).map(|value| (value.into(), tag.end)),
                _ => None,
            },
            "ref" | "relref" | "param" => None,
            _ => snippet_call(&tag, body, closing.as_ref(), cx).map(|(text, call)| {
                calls.push(Call {
                    spot: spot(start, written),
                    ..call
                });
                if closing.is_some() {
                    open.push((tag.name.clone(), Open::End));
                }
                (text, tag.end)
            }),
        };
        match done {
            Some((text, resume)) => {
                replace(&mut out, text, resume);
                at = resume;
            }
            None => {
                left(&mut findings, &TAG_LEFT);
                if closing.is_some() {
                    open.push((tag.name.clone(), Open::Left));
                }
                at = tag.end;
            }
        }
    }
    out.push_str(&body[copied..]);
    findings.sort_by_key(|finding| finding.spots[0].line);
    Converted {
        writing: out.text,
        fences: out.fences,
        called: out.called,
        findings,
        calls,
    }
}

/// The code `highlight` wraps as a fenced code block.
fn fenced(language: &str, inside: &str) -> Marked {
    let inside = inside
        .strip_prefix("\r\n")
        .or_else(|| inside.strip_prefix('\n'))
        .unwrap_or(inside);
    let inside = inside.trim_end_matches([' ', '\t']);
    let inside = inside.strip_suffix('\n').unwrap_or(inside);
    let inside = inside.strip_suffix('\r').unwrap_or(inside);
    let mut longest = 0;
    let mut run = 0;
    for c in inside.chars() {
        run = if c == '`' { run + 1 } else { 0 };
        longest = longest.max(run);
    }
    let fence = "`".repeat(longest.max(2) + 1);
    Marked::fence(format!("{fence}{language}\n{inside}\n{fence}"))
}

/// The opening of a `details` element, as Hugo wrote it.
fn details(tag: &Written) -> Option<String> {
    let mut summary = "Details".to_owned();
    let mut attributes = String::new();
    let mut opened = false;
    for (name, value) in &tag.args {
        match name.as_deref()? {
            "summary" => summary = value.clone(),
            "open" => opened = value == "true",
            name @ ("class" | "name" | "title") => {
                attributes.push_str(&format!(" {name}=\"{}\"", escaped(value)));
            }
            _ => return None,
        }
    }
    if opened {
        attributes.push_str(" open");
    }
    Some(format!(
        "<details{attributes}>\n<summary>{}</summary>\n",
        escaped(&summary)
    ))
}

/// The snippet a tag's call names: its own name, made one a call can
/// name.
pub(super) fn snippet_name(tag: &str) -> String {
    let mut name: String = tag
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    if !name.starts_with(|c: char| c.is_ascii_alphabetic()) {
        name = format!("old_{name}");
    }
    name
}

/// A tag as a call to a snippet of its name, where its values can be
/// handed: on a line of its own where the snippet wraps writing.
fn snippet_call(
    tag: &Written,
    body: &str,
    closing: Option<&Written>,
    cx: &Context,
) -> Option<(Marked, Call)> {
    let wraps = cx.wrapping.contains(&tag.name);
    if wraps {
        let alone = opens_line(body, tag.start) && ends_line(body, tag.end);
        let closed_alone =
            closing.is_none_or(|close| opens_line(body, close.start) && ends_line(body, close.end));
        if !alone || !closed_alone {
            return None;
        }
    }
    let positions = POSITIONS
        .iter()
        .find(|(name, _)| *name == tag.name && !cx.own.contains(&tag.name))
        .map_or(&[][..], |(_, names)| *names);
    let mut names = Vec::new();
    let mut handed = Vec::new();
    let mut place = 0;
    for (name, value) in &tag.args {
        if value.contains(['\n', '\r']) {
            return None;
        }
        let name = match name {
            Some(name) => name
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
                .collect::<String>(),
            None => {
                place += 1;
                positions
                    .get(place - 1)
                    .map_or_else(|| format!("value_{place}"), |n| (*n).to_owned())
            }
        };
        if !name.starts_with(|c: char| c.is_ascii_alphabetic()) || names.contains(&name) {
            return None;
        }
        handed.push(format!("{name}={}", quoted(value)));
        names.push(name);
    }
    let snippet = snippet_name(&tag.name);
    let mut text = Marked::call(format!("@{snippet}({})", handed.join(", ")));
    if wraps && closing.is_none() {
        text.push_str("\n");
        text.push(Marked::call("@end"));
    }
    Some((
        text,
        Call {
            snippet,
            tag: tag.name.clone(),
            names,
            wraps,
            spot: Spot {
                file: String::new(),
                line: None,
                text: String::new(),
                times: None,
            },
        },
    ))
}

/// The writing with the options on each fenced code block's first line,
/// such as `{linenos=true}`, taken off, each told.
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
        let Some(open) = info.find('{') else { continue };
        if !info.trim_end().ends_with('}') {
            continue;
        }
        let line_end = body[range.start..]
            .find('\n')
            .map_or(body.len(), |i| range.start + i);
        let line = &body[range.start..line_end];
        let Some(brace) = line.find('{') else {
            continue;
        };
        let cut_from = line[..brace].trim_end().len();
        let kept = info[..open].trim_end();
        if line[..cut_from]
            .trim_start()
            .trim_start_matches(['`', '~'])
            .trim()
            != kept
        {
            continue;
        }
        let cut_to = line.trim_end_matches('\r').len();
        findings.push(finding(&CODE_OPTIONS, spot(range.start, line.trim())));
        cuts.push(range.start + cut_from..range.start + cut_to);
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

/// Each place the writing has a form of Hugo's Markdown that Statac's
/// reads differently: raw HTML Hugo left out, `:emoji:` names Hugo showed
/// as emoji, attribute lists below blocks, and a summary Hugo cut.
fn markdown_forms(body: &str, cx: &Context, spot: &dyn Fn(usize, &str) -> Spot) -> Vec<Finding> {
    let mut found = Vec::new();
    if cx.html {
        return found;
    }
    let code = code_ranges(body);
    let mut lines_told = HashSet::new();
    let mut paragraphs: Vec<(usize, usize)> = Vec::new();
    let mut words: Option<(usize, usize)> = None;
    let mut depth = 0;
    for (event, range) in Parser::new_ext(body, options()).into_offset_iter() {
        match event {
            // A block of HTML is told of once, at its first line, and a
            // comment not at all, however many lines it runs to.
            Event::Start(Tag::HtmlBlock) | Event::InlineHtml(_) if !cx.unsafe_html => {
                let line_start = body[..range.start].rfind('\n').map_or(0, |i| i + 1);
                let trimmed = body[range.clone()].trim();
                if trimmed.starts_with("<!--")
                    || trimmed.is_empty()
                    || !lines_told.insert(line_start)
                {
                    continue;
                }
                let first = trimmed.lines().next().unwrap_or_default();
                found.push(finding(&RAW_HTML, spot(range.start, first)));
            }
            Event::Start(Tag::Paragraph) if depth == 0 => words = Some((range.start, 0)),
            Event::End(TagEnd::Paragraph) if depth == 0 => {
                if let Some(paragraph) = words.take() {
                    paragraphs.push(paragraph);
                }
            }
            Event::Start(
                Tag::BlockQuote(_) | Tag::List(_) | Tag::Item | Tag::FootnoteDefinition(_),
            ) => depth += 1,
            Event::End(
                TagEnd::BlockQuote(_) | TagEnd::List(_) | TagEnd::Item | TagEnd::FootnoteDefinition,
            ) => depth -= 1,
            Event::Text(text) | Event::Code(text) => {
                if let Some((_, count)) = &mut words {
                    *count += text.split_whitespace().count();
                }
            }
            _ => {}
        }
    }
    if let Some(length) = cx.summary_length
        && let [(start, first), _, ..] = paragraphs.as_slice()
        && *first < length
    {
        found.push(finding(&SUMMARY_CUT, spot(*start, "")));
    }
    let mut at = 0;
    for line in body.split_inclusive('\n') {
        let start = at;
        at += line.len();
        let bare = line.trim_end_matches(['\n', '\r']);
        let Some(lead) = bare.find(|c: char| !c.is_whitespace()) else {
            continue;
        };
        if within(&code, start + lead) {
            continue;
        }
        let trimmed = bare.trim();
        if cx.block_attributes
            && trimmed.starts_with('{')
            && trimmed.ends_with('}')
            && trimmed.contains(['.', '#', '='])
            && !trimmed.starts_with("{{")
        {
            found.push(finding(&ATTRIBUTE_LINES, spot(start + lead, trimmed)));
        }
        if cx.emoji {
            for (offset, name) in emoji_names(bare) {
                if !within(&code, start + offset) {
                    found.push(finding(&EMOJI, spot(start + offset, name)));
                }
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tag_is_read_with_its_values() {
        let text =
            r#"{{< figure src="a >}}.jpg" alt=`raw "alt"` >}} {{% gist user 1 /%}} {{< /aside >}}"#;
        let tag = tag_at(text, 0).unwrap();
        assert_eq!(tag.name, "figure");
        assert_eq!(
            tag.args,
            vec![
                (Some("src".into()), "a >}}.jpg".into()),
                (Some("alt".into()), "raw \"alt\"".into()),
            ]
        );
        let gist = tag_at(text, tag.end + 1).unwrap();
        assert_eq!((gist.name.as_str(), gist.self_closing), ("gist", true));
        assert_eq!(gist.args, vec![(None, "user".into()), (None, "1".into())]);
        let closing = tag_at(text, gist.end + 1).unwrap();
        assert_eq!((closing.name.as_str(), closing.closing), ("aside", true));
        assert_eq!(
            tag_at("{{</* youtube 1 */>}}", 0).unwrap().shown.as_deref(),
            Some("{{< youtube 1 >}}")
        );
        assert!(tag_at("{{< figure src=\"open >}}", 0).is_none());
    }

    #[test]
    fn a_tag_closed_later_is_found_past_others_of_its_name() {
        let text = "{{< aside >}}a{{< aside >}}b{{< /aside >}}c{{< /aside >}}";
        let close = closing_of(text, 13, "aside").unwrap();
        assert_eq!(&text[close.start..], "{{< /aside >}}");
        assert_eq!(paired(text), vec!["aside".to_owned()]);
    }

    #[test]
    fn private_use_characters_are_kept_and_never_taken_for_a_call_or_a_fence() {
        let wrapping = HashSet::from(["aside".to_owned()]);
        let cx = Context {
            base: "",
            whole: None,
            refs: &|_, _| None,
            param: &|_| None,
            own: &[],
            wrapping: &wrapping,
            html: false,
            unsafe_html: true,
            emoji: false,
            block_attributes: false,
            summary_length: None,
        };
        // The writing, what it becomes, and the line from each call made
        // and from each fence made.
        let cases: [(&str, &str, &[&str], &[&str]); 6] = [
            (
                "A \u{E000} and \u{E001}.\n\n<div title=\"\u{E001}\">\u{E000}</div>\n",
                "A \u{E000} and \u{E001}.\n\n<div title=\"\u{E001}\">\u{E000}</div>\n",
                &[],
                &[],
            ),
            (
                "```\n\u{E001}\u{E000}\n```\n",
                "```\n\u{E001}\u{E000}\n```\n",
                &[],
                &[],
            ),
            (
                "{{< highlight text >}}\n\u{E001}\u{E000}\n{{< /highlight >}}\n",
                "```text\n\u{E001}\u{E000}\n```\n",
                &[],
                &["```text"],
            ),
            (
                "\u{E000}{{< youtube \"\u{E000}\" >}}\n\n{{< aside \"\u{E001}\" >}}\n",
                "\u{E000}@youtube(id='\u{E000}')\n\n@aside(value_1='\u{E001}')\n@end\n",
                &[
                    "@youtube(id='\u{E000}')",
                    "@aside(value_1='\u{E001}')",
                    "@end",
                ],
                &[],
            ),
            (
                "{{< aside >}}\n\u{E000}\u{E001}\n{{< /aside >}}\n",
                "@aside()\n\u{E000}\u{E001}\n@end\n",
                &["@aside()", "@end"],
                &[],
            ),
            (
                "{{< figure src=\"a.jpg\" caption=\"\u{E000}\" >}}\n",
                "@figure(src='a.jpg', caption='\u{E000}')\n",
                &["@figure(src='a.jpg', caption='\u{E000}')"],
                &[],
            ),
        ];
        for (body, writing, called, fences) in cases {
            let converted = convert(body, 1, "a.md", &cx);
            assert_eq!(converted.writing, writing, "{body:?}");
            let lines = |marks: &[usize]| -> Vec<String> {
                (marks.iter())
                    .map(|at| writing[*at..].lines().next().unwrap_or_default().to_owned())
                    .collect()
            };
            assert_eq!(lines(&converted.called), called, "{body:?}");
            assert_eq!(lines(&converted.fences), fences, "{body:?}");
        }
    }

    #[test]
    fn emoji_names_are_told_from_times() {
        let names: Vec<&str> = emoji_names("a :smile: :+1: 10:30:00 b:c:")
            .into_iter()
            .map(|(_, name)| name)
            .collect();
        assert_eq!(names, vec![":smile:", ":+1:"]);
    }
}
