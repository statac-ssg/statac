//! Jekyll's template tags in the writing, each converted where Statac's
//! writing has an equal and otherwise left exactly as written, and the
//! forms of Jekyll's Markdown that Statac's reads differently, counted.
//! Nothing inside a code block or a code span is touched, but for what
//! Jekyll took out there too.

use std::collections::HashMap;
use std::ops::Range;

use pulldown_cmark::{Event, Parser, Tag};

use super::{
    ABBREVIATIONS, ATTRIBUTE_LINES, CODE_LINES, EXCERPT_LEFT, EXTENSIONS, INLINE_MATHS,
    LINK_NOWHERE, MARKDOWN_IN_HTML, TAG_IN_CODE, TAG_LEFT, TOC,
};
use crate::import::liquid::{
    code_ranges, excerpt, excerpts_inside, finding, highlight, include, options, raw, split_word,
    tag_at, within,
};
use crate::import::{Finding, Spot};

/// What the writing's tags are converted with.
pub(super) struct Context<'a> {
    /// The path the site is under, as `/blog`; empty at the top of its
    /// host.
    pub(crate) base: &'a str,
    /// The site's address without its path.
    pub(crate) host: Option<&'a str>,
    /// Each post's address by the name `post_url` gives it.
    pub(crate) posts: &'a HashMap<String, String>,
    /// Each page's, post's or file's address by its path in the old site.
    pub(crate) links: &'a HashMap<String, String>,
    /// The site's or the page's `excerpt_separator`, where it is one.
    pub(crate) separator: Option<&'a str>,
    /// The fields of the page's own front matter that hold plain text or
    /// a number, as Jekyll showed each.
    pub(crate) page: &'a [(String, String)],
}

/// A snippet call put in place of an include.
#[derive(Clone, Debug)]
pub(super) struct Call {
    pub(crate) snippet: String,
    /// The include's file, as the tag names it.
    pub(crate) include: String,
    /// The names it hands, in order.
    pub(crate) names: Vec<String>,
    pub(crate) spot: Spot,
}

/// The writing converted, with what didn't convert.
pub(super) struct Converted {
    pub(crate) writing: String,
    pub(crate) findings: Vec<Finding>,
    pub(crate) calls: Vec<Call>,
    /// Where each code fence written here starts in the writing, in
    /// order.
    pub(crate) fences: Vec<usize>,
    /// Where each snippet call written here starts, in order.
    pub(crate) called: Vec<usize>,
    /// How many footnotes were given a blank line between them and the
    /// link definition after.
    pub(crate) set_apart: usize,
}

/// Converts the writing of `file`, whose first line is line `first_line`
/// of the file.
pub(super) fn convert(body: &str, first_line: usize, file: &str, cx: &Context) -> Converted {
    let code = code_ranges(body);
    let line_of = |at: usize| first_line + body[..at].matches('\n').count();
    let spot = |at: usize, text: &str| Spot {
        file: file.to_owned(),
        line: Some(line_of(at)),
        text: text.to_owned(),
        times: None,
    };
    let mut findings = Vec::new();
    let mut calls = Vec::new();
    let mut fences = Vec::new();
    let mut called = Vec::new();
    let mut out = String::with_capacity(body.len());
    let mut copied = 0;
    let mut at = 0;
    while let Some(found) = body[at..].find(['{']) {
        let start = at + found;
        if !(body[start..].starts_with("{%") || body[start..].starts_with("{{")) {
            at = start + 1;
            continue;
        }
        let Some((length, inner, statement)) = tag_at(body, start) else {
            break;
        };
        let written = &body[start..start + length];
        let end = start + length;
        let (word, rest) = split_word(&inner);
        // Jekyll took `raw` and `endraw` out and left what they held
        // alone, inside code as anywhere.
        if statement
            && word == "raw"
            && let Some((from, held, resume)) = raw(body, start, end, "endraw")
        {
            out.push_str(&body[copied..from]);
            out.push_str(&body[held]);
            copied = resume;
            at = resume;
            continue;
        }
        if within(&code, start) {
            // What a tag made of nothing but the page's own words Jekyll
            // wrote inside code as anywhere.
            match (!statement).then(|| own_words(&inner, cx)).flatten() {
                Some(text) => {
                    out.push_str(&body[copied..start]);
                    out.push_str(&text);
                    copied = end;
                }
                None => findings.push(finding(&TAG_IN_CODE, spot(start, written))),
            }
            at = end;
            continue;
        }
        let leave = |findings: &mut Vec<Finding>, kind| {
            findings.push(finding(kind, spot(start, written)));
        };
        let replaced: Option<(String, usize)> = match (statement, word) {
            (true, "highlight") => highlighted(rest).and_then(|tag| {
                let (text, resume, lines) =
                    highlight(body, start, end, tag.language, tag.numbered, &tag.marked)?;
                if lines {
                    findings.push(finding(&CODE_LINES, spot(start, written)));
                }
                Some((text, resume))
            }),
            (true, "post_url") => {
                let address = cx.posts.get(rest.trim_start_matches('/'));
                match address {
                    Some(address) => Some((format!("{}{address}", cx.base), end)),
                    None => {
                        leave(&mut findings, &LINK_NOWHERE);
                        at = end;
                        continue;
                    }
                }
            }
            (true, "link") => match cx.links.get(rest.trim_start_matches('/')) {
                Some(address) => Some((format!("{}{address}", cx.base), end)),
                None => {
                    leave(&mut findings, &LINK_NOWHERE);
                    at = end;
                    continue;
                }
            },
            (true, "include") => match include(rest) {
                Some((snippet, names, call)) => {
                    calls.push(Call {
                        snippet,
                        include: split_word(rest).0.to_owned(),
                        names,
                        spot: spot(start, written),
                    });
                    Some((call, end))
                }
                None => None,
            },
            (false, _) => expression(&inner, &body[end..], cx).map(|text| (text, end)),
            _ => None,
        };
        match replaced {
            Some((text, resume)) => {
                out.push_str(&body[copied..start]);
                match (statement, word) {
                    (true, "highlight") => fences.push(out.len()),
                    (true, "include") => called.push(out.len()),
                    _ => {}
                }
                out.push_str(&text);
                copied = resume;
                at = resume;
            }
            None => {
                leave(&mut findings, &TAG_LEFT);
                at = end;
            }
        }
    }
    out.push_str(&body[copied..]);
    if let Some(separator) = cx
        .separator
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "<!--more-->")
    {
        out = excerpt(&out, separator, [&mut fences, &mut called]);
        for start in excerpts_inside(body, separator) {
            findings.push(finding(&EXCERPT_LEFT, spot(start, separator)));
        }
    }
    findings.extend(markdown_forms(body, &code, &spot));
    let set_apart = definitions_after_footnotes(&mut out, [&mut fences, &mut called]);
    Converted {
        writing: out,
        findings,
        calls,
        fences,
        called,
        set_apart,
    }
}

/// What `{{ }}` wrote where it holds only words of the page's own: text
/// in quotes with no filter as the text, and `page.name` as the field of
/// that name in the page's front matter.
fn own_words(inner: &str, cx: &Context) -> Option<String> {
    if let Some(name) = inner.strip_prefix("page.") {
        return (cx.page.iter())
            .find(|(field, _)| field == name)
            .map(|(_, value)| value.clone());
    }
    let quote = inner.chars().next().filter(|c| matches!(c, '"' | '\''))?;
    let text = inner.strip_prefix(quote)?.strip_suffix(quote)?;
    (inner.len() > 1 && !text.contains(quote)).then(|| text.to_owned())
}

/// `{{ }}` as Statac's writing takes it: `site.baseurl` as the path the
/// site is under, `site.url` as nothing before an address and as the
/// site's address elsewhere, and an address quoted through `relative_url`
/// or `absolute_url` as the address from the top of the site, and the
/// page's own words as [`own_words`] has them. Either of the first two is
/// nothing before a tag written as an address with that path on it, so
/// the path is written once.
fn expression(inner: &str, after: &str, cx: &Context) -> Option<String> {
    match inner {
        "site.baseurl" => {
            return Some(match from_the_top(after, cx) {
                true => String::new(),
                false => cx.base.to_owned(),
            });
        }
        "site.url" => {
            let before_address =
                after.starts_with('/') || after.starts_with("{{") || from_the_top(after, cx);
            return match before_address {
                true => Some(String::new()),
                false => cx.host.map(|host| format!("{host}{}", cx.base)),
            };
        }
        _ => {}
    }
    if let Some(text) = own_words(inner, cx) {
        return Some(text);
    }
    filtered(inner, cx)
}

/// An address quoted through `relative_url` or `absolute_url`, as the
/// address from the top of the site's host.
fn filtered(inner: &str, cx: &Context) -> Option<String> {
    let (value, filter) = inner.split_once('|')?;
    if !matches!(filter.trim(), "relative_url" | "absolute_url") {
        return None;
    }
    let value = value.trim();
    let quote = value.chars().next().filter(|c| matches!(c, '"' | '\''))?;
    let address = value.strip_prefix(quote)?.strip_suffix(quote)?;
    if address.contains(quote) || address.contains("://") {
        return None;
    }
    let address = match address.starts_with('/') {
        true => address.to_owned(),
        false => format!("/{address}"),
    };
    Some(format!("{}{address}", cx.base))
}

/// Whether `after` begins with a tag written as an address from the top
/// of the site's host, the path the site is under included: a `post_url`
/// or `link` naming a page, or an address through `relative_url` or
/// `absolute_url`.
fn from_the_top(after: &str, cx: &Context) -> bool {
    if !(after.starts_with("{%") || after.starts_with("{{")) {
        return false;
    }
    let Some((_, inner, statement)) = tag_at(after, 0) else {
        return false;
    };
    let (word, rest) = split_word(&inner);
    let named = rest.trim_start_matches('/');
    match (statement, word) {
        (true, "post_url") => cx.posts.contains_key(named),
        (true, "link") => cx.links.contains_key(named),
        (true, _) => false,
        (false, _) => filtered(&inner, cx).is_some(),
    }
}

/// Puts a blank line before the first link definition written straight
/// after each footnote, and gives how many footnotes had one. Jekyll's Markdown read such a line as a
/// definition, and CommonMark reads it as more of the footnote. The
/// footnotes are found by the reader Statac's own Markdown is read with;
/// a line in one is a link definition by its shape, `[label]: address`.
/// `marks`, offsets into the text, are moved with what they mark.
fn definitions_after_footnotes(text: &mut String, marks: [&mut [usize]; 2]) -> usize {
    let mut footnotes = Vec::new();
    for (event, range) in Parser::new_ext(text, options()).into_offset_iter() {
        if let Event::Start(Tag::FootnoteDefinition(_)) = event {
            footnotes.push(range);
        }
    }
    let code = code_ranges(text);
    let mut found = Vec::new();
    for footnote in footnotes {
        let mut at = footnote.start;
        for line in text[footnote.clone()].split_inclusive('\n') {
            let start = at;
            at += line.len();
            if start == footnote.start {
                continue;
            }
            let lead = line.len() - line.trim_start_matches(' ').len();
            let shaped = line[lead..]
                .strip_prefix('[')
                .filter(|rest| !rest.starts_with('^'))
                .and_then(|rest| rest.split_once("]:"))
                .is_some_and(|(label, address)| {
                    !label.trim().is_empty()
                        && !label.contains(['[', ']'])
                        && !address.trim().is_empty()
                });
            if lead < 4 && shaped && !within(&code, start + lead) {
                found.push(start);
                break;
            }
        }
    }
    let [fences, called] = marks;
    for start in found.iter().rev() {
        text.insert(*start, '\n');
        for mark in fences.iter_mut().chain(called.iter_mut()) {
            if *mark >= *start {
                *mark += 1;
            }
        }
    }
    found.len()
}

/// Whether a tag on `line` asks for the Markdown inside it to be read:
/// `markdown="1"`, `"block"` or `"span"`, in either quotes or none, at an
/// offset in the line that isn't `in_code`.
fn asks_for_markdown(line: &str, in_code: &dyn Fn(usize) -> bool) -> bool {
    let Some(tag) = line.find('<') else {
        return false;
    };
    line[tag..].match_indices("markdown=").any(|(at, name)| {
        let before = line[tag..at + tag].chars().next_back();
        let value = line[tag + at + name.len()..].trim_start_matches(['"', '\'']);
        before.is_some_and(char::is_whitespace)
            && !in_code(tag + at)
            && ["1", "block", "span"].iter().any(|asked| {
                value.strip_prefix(asked).is_some_and(|after| {
                    after.is_empty() || after.starts_with(['"', '\'', ' ', '\t', '>', '/'])
                })
            })
    })
}

/// Each place the writing has a form of Jekyll's Markdown that Statac's
/// reads differently: HTML asking for the Markdown inside it to be read,
/// attribute lists, `{:toc}`, extensions such as `{::comment}`,
/// abbreviations, and maths between `$$` inside a line.
fn markdown_forms(
    body: &str,
    code: &[Range<usize>],
    spot: &dyn Fn(usize, &str) -> Spot,
) -> Vec<Finding> {
    let mut found = Vec::new();
    let mut at = 0;
    for line in body.split_inclusive('\n') {
        let start = at;
        at += line.len();
        let bare = line.trim_end_matches(['\n', '\r']);
        let trimmed = bare.trim();
        let Some(lead) = bare.find(|c: char| !c.is_whitespace()) else {
            continue;
        };
        if within(code, start + lead) {
            continue;
        }
        let kind = if asks_for_markdown(bare, &|at| within(code, start + at)) {
            Some(&MARKDOWN_IN_HTML)
        } else if trimmed == "{:toc}" {
            Some(&TOC)
        } else if trimmed.starts_with("{::") || trimmed.starts_with("{:/") {
            Some(&EXTENSIONS)
        } else if trimmed.starts_with("{:") && trimmed.ends_with('}') {
            Some(&ATTRIBUTE_LINES)
        } else if trimmed.starts_with("*[") && trimmed.contains("]:") {
            Some(&ABBREVIATIONS)
        } else if bare.contains("{:") && bare[bare.find("{:").unwrap_or(0)..].contains('}') {
            Some(&ATTRIBUTE_LINES)
        } else if !(trimmed.starts_with("$$") && trimmed.ends_with("$$"))
            && trimmed.matches("$$").count() >= 2
        {
            Some(&INLINE_MATHS)
        } else {
            None
        };
        if let Some(kind) = kind {
            found.push(finding(kind, spot(start + lead, trimmed)));
        }
    }
    found
}

/// A `highlight` tag as Jekyll reads it.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Highlighted<'a> {
    language: &'a str,
    numbered: bool,
    /// Counted from the block's first line as 1.
    marked: Vec<(i64, i64)>,
}

/// What follows the name of `{% highlight ruby linenos mark_lines="3 4" %}`,
/// as Jekyll reads it: `linenos` numbers the lines whatever value it is
/// given. None where Jekyll stops at the tag, or it has an option other
/// than those two.
fn highlighted(rest: &str) -> Option<Highlighted<'_>> {
    let space = |c: char| c.is_ascii_whitespace() || c == '\x0b';
    let word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let rest = rest.trim_matches(|c: char| space(c) || c == '\0');
    let end = rest
        .find(|c: char| !(word(c) || matches!(c, '.' | '+' | '#' | '-')))
        .unwrap_or(rest.len());
    if end == 0 {
        return None;
    }
    let (language, mut options) = rest.split_at(end);
    let mut numbered = false;
    let mut marked = Vec::new();
    while !options.is_empty() {
        let option = options.trim_start_matches(space);
        if option.len() == options.len() {
            return None;
        }
        let name_end = option.find(|c: char| !word(c)).unwrap_or(option.len());
        let name = &option[..name_end];
        options = &option[name_end..];
        let mut list = None;
        if let Some(given) = options.strip_prefix('=') {
            if let Some(quoted) = given.strip_prefix('"') {
                let close = quoted.find('"')?;
                let numbers = &quoted[..close];
                let digits_apart = numbers
                    .split(space)
                    .all(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
                if !digits_apart {
                    return None;
                }
                list = Some(numbers);
                options = &quoted[close + 1..];
            } else {
                let value_end = given.find(|c: char| !word(c)).unwrap_or(given.len());
                if value_end == 0 {
                    return None;
                }
                options = &given[value_end..];
            }
        }
        match (name, list) {
            ("", _) => return None,
            ("linenos", _) => numbered = true,
            ("mark_lines", Some(numbers)) => {
                marked = numbers
                    .split(space)
                    .filter_map(|n| n.parse().ok())
                    .map(|n| (n, n))
                    .collect();
            }
            _ => return None,
        }
    }
    Some(Highlighted {
        language,
        numbered,
        marked,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_highlight_tags_options_are_read_as_jekyll_reads_them() {
        let tag = |language, numbered, marked: &[(i64, i64)]| Highlighted {
            language,
            numbered,
            marked: marked.to_vec(),
        };
        for (rest, expected) in [
            ("ruby", Some(tag("ruby", false, &[]))),
            ("ruby linenos", Some(tag("ruby", true, &[]))),
            ("ruby linenos=table", Some(tag("ruby", true, &[]))),
            ("ruby linenos=false", Some(tag("ruby", true, &[]))),
            (
                "ruby mark_lines=\"1 3\" linenos",
                Some(tag("ruby", true, &[(1, 1), (3, 3)])),
            ),
            ("c++ mark_lines=\"2\"", Some(tag("c++", false, &[(2, 2)]))),
            ("ruby mark_lines=2", None),
            ("ruby mark_lines=\"1  3\"", None),
            ("ruby mark_lines=\"\"", None),
            ("ruby hl_lines=\"1\"", None),
            ("ruby linenos=", None),
            ("ruby,linenos", None),
            ("", None),
        ] {
            assert_eq!(highlighted(rest), expected, "{rest}");
        }
    }
}
