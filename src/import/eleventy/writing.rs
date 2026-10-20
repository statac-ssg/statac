//! An Eleventy page's writing: the template tags the old site ran over it
//! first, in Liquid or Nunjucks, each converted where Statac's writing has
//! an equal and otherwise left exactly as written; the excerpt marker; and
//! the Markdown the old site's settings read differently, each line end
//! inside a paragraph made a line break where they had `breaks: true`, and
//! the rest counted. Nothing inside a code block or a code span is
//! converted, but for what `raw` held.

use std::collections::HashSet;

use crate::import::common::{ends_line, opens_line};
use crate::import::liquid::{
    code_ranges, excerpt, excerpts_inside, finding, highlight, include_snippet, quoted,
    quoted_value, raw, split_word, tag_at, within,
};
use crate::import::{Finding, Spot};
use crate::markdown::{self, Document, Kind, Node};
use crate::source::{Sources, Span};

use super::{ATTRIBUTES, EXCERPT_LEFT, HTML_SHOWN, LINK_NOT_MADE, TAG_IN_CODE, TAG_LEFT};

/// The language a page's tags are written in.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Engine {
    Liquid,
    Nunjucks,
    /// None ran, or one whose tags Statac doesn't read.
    Not,
}

/// What the writing is converted with.
pub struct Context<'a> {
    /// Whether the new site has the starter, whose snippets a call names
    /// or is kept clear of.
    pub starter: bool,
    pub engine: Engine,
    /// Whether the page is Markdown, rather than HTML.
    pub markdown: bool,
    pub separator: Option<&'a str>,
    /// The tags of the site's own that some call anywhere wraps writing
    /// with.
    pub wrapping: &'a HashSet<String>,
    pub breaks: bool,
    pub attributes: bool,
    pub linkify: bool,
    /// Whether the old site's Markdown showed HTML as text.
    pub html_off: bool,
    /// The path the old site was published under, as `/blog`, or empty.
    pub base: &'a str,
}

/// A snippet call put in place of an include or a tag of the site's own.
#[derive(Clone, Debug)]
pub struct Call {
    pub snippet: String,
    /// What it stands for, as the snippet's comment says it: ``include
    /// `note.html` `` or ``tag `image` ``.
    pub what: String,
    pub include: bool,
    /// The names it hands, in order.
    pub names: Vec<String>,
    /// Whether it hands values without names.
    pub positional: bool,
    pub wraps: bool,
    pub spot: Spot,
}

/// The writing converted, with what didn't convert.
pub struct Converted {
    pub writing: String,
    pub findings: Vec<Finding>,
    pub calls: Vec<Call>,
    pub fences: Vec<usize>,
    pub called: Vec<usize>,
    /// How many line ends were made line breaks.
    pub breaks: usize,
    /// How many places hold the separator alone on a line, made
    /// `<!--more-->`.
    pub separators: usize,
}

/// The tags Liquid and Nunjucks know themselves, and those of Eleventy's
/// own plugins, none of which is one of the site's own.
const BUILT_IN: &[&str] = &[
    "assign",
    "capture",
    "case",
    "when",
    "else",
    "comment",
    "cycle",
    "decrement",
    "increment",
    "echo",
    "for",
    "break",
    "continue",
    "if",
    "elsif",
    "elif",
    "elseif",
    "unless",
    "layout",
    "block",
    "liquid",
    "tablerow",
    "set",
    "macro",
    "call",
    "filter",
    "import",
    "from",
    "extends",
    "asyncEach",
    "asyncAll",
    "ifAsync",
    "renderTemplate",
    "renderFile",
    "endhighlight",
    "endraw",
    "endverbatim",
];

/// Whether `name` closes one of [`BUILT_IN`].
fn closes_built_in(name: &str) -> bool {
    name.strip_prefix("end").is_some_and(|opened| {
        BUILT_IN.contains(&opened)
            || matches!(opened, "each" | "all" | "capture" | "case" | "comment")
    })
}

/// A value handed to a tag of the site's own: text in quotes, a number,
/// `true` or `false`, each parted from the next by a comma or a space.
fn values(mut rest: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
    loop {
        rest = rest.trim_start_matches(|c: char| c.is_whitespace() || c == ',');
        if rest.is_empty() {
            return Some(out);
        }
        if let Some((value, after)) = quoted_value(rest) {
            out.push(quoted(&value));
            rest = after;
            if !(rest.is_empty() || rest.starts_with(|c: char| c.is_whitespace() || c == ',')) {
                return None;
            }
            continue;
        }
        let end = rest
            .find(|c: char| c.is_whitespace() || c == ',')
            .unwrap_or(rest.len());
        let word = &rest[..end];
        let number = word.parse::<f64>().is_ok() && !word.starts_with('.');
        if !(number || word == "true" || word == "false") {
            return None;
        }
        out.push(word.to_owned());
        rest = &rest[end..];
    }
}

/// An include, or Liquid's `render`, as a snippet call, and the call as
/// written. None where the file or a value is a variable.
fn include(rest: &str, starter: bool) -> Option<(Call, String)> {
    let (file, mut params) = match quoted_value(rest) {
        Some((file, after)) => (file, after),
        None => {
            let end = rest
                .find(|c: char| c.is_whitespace() || c == ',')
                .unwrap_or(rest.len());
            let file = &rest[..end];
            if !file.contains('.') || file.contains(['{', '}', '"', '\'']) {
                return None;
            }
            (file.to_owned(), &rest[end..])
        }
    };
    if file.is_empty() || file.contains(['{', '}']) {
        return None;
    }
    let mut names = Vec::new();
    let mut handed = Vec::new();
    loop {
        params = params.trim_start_matches(|c: char| c.is_whitespace() || c == ',');
        if params.is_empty() {
            break;
        }
        let end = params.find([':', '='])?;
        let key = params[..end].trim();
        if !key.starts_with(|c: char| c.is_ascii_alphabetic())
            || !key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return None;
        }
        let (value, after) = quoted_value(params[end + 1..].trim_start())?;
        let name = key.replace('-', "_");
        handed.push(format!("{name}={}", quoted(&value)));
        names.push(name);
        params = after;
    }
    let snippet = include_snippet(&file, starter);
    let text = format!("@{snippet}({})", handed.join(", "));
    Some((
        Call {
            snippet,
            what: format!("include `{file}`"),
            include: true,
            names,
            positional: false,
            wraps: false,
            spot: no_spot(),
        },
        text,
    ))
}

fn no_spot() -> Spot {
    Spot {
        file: String::new(),
        line: None,
        text: String::new(),
        times: None,
    }
}

/// The snippet a tag of the site's own calls: its name, kept clear of the
/// starter's where the new site has it, with each character a snippet's
/// name can't hold as `_`.
fn own_snippet(tag: &str, starter: bool) -> String {
    let name: String = tag
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let name = match name.starts_with(|c: char| c.is_ascii_alphabetic()) {
        true => name,
        false => format!("tag_{name}"),
    };
    match starter && crate::new::has_snippet(&name) {
        true => format!("old_{name}"),
        false => name,
    }
}

/// Whether `name` can be a tag of the site's own.
fn own_name(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        && !BUILT_IN.contains(&name)
        && !closes_built_in(name)
        && !matches!(
            name,
            "include" | "render" | "raw" | "verbatim" | "highlight"
        )
}

/// The names of the tags of the site's own in `body` that wrap writing:
/// each with its `end` tag after it.
pub fn paired(body: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut at = 0;
    while let Some(found) = body[at..].find("{%") {
        let start = at + found;
        let Some((length, inner, _)) = tag_at(body, start) else {
            break;
        };
        let name = split_word(&inner).0;
        if own_name(name)
            && !names.iter().any(|n| n == name)
            && crate::import::liquid::find_statement(body, start + length, &format!("end{name}"))
                .is_some()
        {
            names.push(name.to_owned());
        }
        at = start + length;
    }
    names
}

/// What a tag of the site's own left open stands for when its `end` tag
/// comes.
enum Open {
    Called(String),
    Left(String),
}

/// Converts the writing of `file`, whose first line is line `first_line`
/// of the file.
pub fn convert(body: &str, first_line: usize, file: &str, cx: &Context) -> Converted {
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
    let code = code_ranges(body);
    if cx.engine != Engine::Not {
        let mut open: Vec<Open> = Vec::new();
        let mut at = 0;
        while let Some(found) = body[at..].find('{') {
            let start = at + found;
            let rest = &body[start..];
            if cx.engine == Engine::Nunjucks && rest.starts_with("{#") {
                let end = rest.find("#}").map_or(body.len(), |n| start + n + 2);
                match within(&code, start) {
                    true => findings.push(finding(&TAG_IN_CODE, spot(start, &body[start..end]))),
                    false => findings.push(finding(&TAG_LEFT, spot(start, &body[start..end]))),
                }
                at = end;
                continue;
            }
            if !(rest.starts_with("{%") || rest.starts_with("{{")) {
                at = start + 1;
                continue;
            }
            let Some((length, inner, statement)) = tag_at(body, start) else {
                break;
            };
            let end = start + length;
            let written = &body[start..end];
            let (word, words) = split_word(&inner);
            at = end;
            // Both languages take `raw` out and leave what it held alone,
            // inside code as anywhere.
            let closes = match word {
                "raw" => Some("endraw"),
                "verbatim" if cx.engine == Engine::Nunjucks => Some("endverbatim"),
                _ => None,
            };
            if statement
                && let Some(close) = closes
                && let Some((from, held, resume)) = raw(body, start, end, close)
            {
                out.push_str(&body[copied..from]);
                out.push_str(&body[held]);
                copied = resume;
                at = resume;
                continue;
            }
            if within(&code, start) {
                findings.push(finding(&TAG_IN_CODE, spot(start, written)));
                continue;
            }
            let mut put = |out: &mut String, text: &str, resume: usize| {
                out.push_str(&body[copied..start]);
                out.push_str(text);
                copied = resume;
            };
            let left = |findings: &mut Vec<Finding>| {
                findings.push(finding(&TAG_LEFT, spot(start, written)));
            };
            if !statement {
                match address(&inner, cx.base) {
                    Some(address) => put(&mut out, &address, end),
                    None => left(&mut findings),
                }
                continue;
            }
            match word {
                "highlight" => {
                    let language = quoted_value(words).map_or(words.to_owned(), |(l, _)| l);
                    match highlight(body, start, end, &language) {
                        Some((text, resume)) => {
                            out.push_str(&body[copied..start]);
                            fences.push(out.len());
                            out.push_str(&text);
                            copied = resume;
                            at = resume;
                        }
                        None => left(&mut findings),
                    }
                }
                "include" | "render" => match include(words, cx.starter) {
                    Some((call, text)) => {
                        out.push_str(&body[copied..start]);
                        called.push(out.len());
                        out.push_str(&text);
                        copied = end;
                        calls.push(Call {
                            spot: spot(start, written),
                            ..call
                        });
                    }
                    None => left(&mut findings),
                },
                name if name.starts_with("end") && own_name(&name[3..]) => {
                    let opened = &name[3..];
                    match open.last() {
                        Some(Open::Called(n)) if n == opened => {
                            open.pop();
                            out.push_str(&body[copied..start]);
                            called.push(out.len());
                            out.push_str("@end");
                            copied = end;
                        }
                        Some(Open::Left(n)) if n == opened => {
                            open.pop();
                            left(&mut findings);
                        }
                        _ => left(&mut findings),
                    }
                }
                name if own_name(name) => {
                    let close =
                        crate::import::liquid::find_statement(body, end, &format!("end{name}"));
                    let wraps = close.is_some() || cx.wrapping.contains(name);
                    let alone = opens_line(body, start) && ends_line(body, end);
                    let handed = values(words);
                    let fits = match close {
                        Some((close, length)) => {
                            alone && opens_line(body, close) && ends_line(body, close + length)
                        }
                        None => !wraps || alone,
                    };
                    match handed.filter(|_| fits) {
                        Some(handed) => {
                            let snippet = own_snippet(name, cx.starter);
                            out.push_str(&body[copied..start]);
                            called.push(out.len());
                            out.push_str(&format!("@{snippet}({})", handed.join(", ")));
                            if wraps && close.is_none() {
                                out.push('\n');
                                called.push(out.len());
                                out.push_str("@end");
                            }
                            copied = end;
                            if close.is_some() {
                                open.push(Open::Called(name.to_owned()));
                            }
                            calls.push(Call {
                                snippet,
                                what: format!("tag `{name}`"),
                                include: false,
                                names: Vec::new(),
                                positional: !handed.is_empty(),
                                wraps,
                                spot: spot(start, written),
                            });
                        }
                        None => {
                            left(&mut findings);
                            if close.is_some() {
                                open.push(Open::Left(name.to_owned()));
                            }
                        }
                    }
                }
                _ => left(&mut findings),
            }
        }
    }
    out.push_str(&body[copied..]);
    let mut separators = 0;
    if let Some(separator) = cx
        .separator
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "<!--more-->")
    {
        let before = out.matches("<!--more-->").count();
        out = excerpt(&out, separator, [&mut fences, &mut called]);
        separators = out.matches("<!--more-->").count() - before;
        for start in excerpts_inside(body, separator) {
            findings.push(finding(&EXCERPT_LEFT, spot(start, separator)));
        }
    }
    let mut breaks = 0;
    if cx.markdown {
        let document = read(body, &HashSet::new());
        if cx.attributes {
            for (at, text) in braces(&document, body) {
                findings.push(finding(&ATTRIBUTES, spot(at, &text)));
            }
        }
        if cx.linkify {
            for (at, text) in bare_addresses(&document, body) {
                findings.push(finding(&LINK_NOT_MADE, spot(at, &text)));
            }
        }
        if cx.html_off {
            let separator = cx.separator.map(str::trim);
            for (at, text) in html(&document, body) {
                if Some(text.as_str()) == separator {
                    continue;
                }
                findings.push(finding(&HTML_SHOWN, spot(at, &text)));
            }
        }
        if cx.breaks {
            let wrapping: HashSet<String> = cx
                .wrapping
                .iter()
                .map(|name| own_snippet(name, cx.starter))
                .collect();
            breaks = line_breaks(&mut out, &wrapping, [&mut fences, &mut called]);
        }
    }
    findings.sort_by_key(|finding| finding.spots[0].line);
    Converted {
        writing: out,
        findings,
        calls,
        fences,
        called,
        breaks,
        separators,
    }
}

/// `{{ }}` as Statac's writing takes it: an address in quotes through
/// Eleventy's `url` filter as the address the filter gave, which is the
/// address itself with `base`, the path the old site was under, before one
/// written from the top of the site.
fn address(inner: &str, base: &str) -> Option<String> {
    let (value, filter) = inner.split_once('|')?;
    if filter.trim() != "url" {
        return None;
    }
    let (address, after) = quoted_value(value.trim())?;
    if !after.trim().is_empty() {
        return None;
    }
    Some(
        match address.starts_with('/') && !address.starts_with("//") {
            true => format!("{base}{address}"),
            false => address,
        },
    )
}

/// The writing as Statac reads it, the snippets of `wrapping` taking the
/// writing below a call to them.
fn read(writing: &str, wrapping: &HashSet<String>) -> Document {
    let mut sources = Sources::default();
    let source = sources.add("writing", writing.to_owned());
    let wraps = |name: &str| Some(wrapping.contains(name));
    markdown::read(&sources, source, Span::new(0, writing.len()), &wraps, "")
}

/// Each node of `document`'s writing, but what is inside a link, a
/// picture, code or HTML, in order.
fn words_of(document: &Document) -> Vec<&Node> {
    fn gather<'d>(node: &'d Node, out: &mut Vec<&'d Node>) {
        match node.kind {
            Kind::Link { .. } | Kind::Image { .. } | Kind::CodeBlock { .. } | Kind::HtmlBlock => {}
            Kind::Text(_) => out.push(node),
            _ => node.children.iter().for_each(|child| gather(child, out)),
        }
    }
    let mut out = Vec::new();
    gather(&document.root, &mut out);
    for note in &document.notes {
        gather(note, &mut out);
    }
    out
}

/// Each `{...}` in the words of the writing that a plugin for attributes
/// read: braces holding an `#id`, a `.class` or a `name=value`, by its
/// offset.
fn braces(document: &Document, body: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    for node in words_of(document) {
        let span = node.place.span.range();
        let text = &body[span.clone()];
        let mut from = 0;
        while let Some(open) = text[from..].find('{').map(|n| from + n) {
            let Some(close) = text[open..].find('}').map(|n| open + n) else {
                break;
            };
            let inside = text[open + 1..close].trim();
            let first = inside.split_whitespace().next().unwrap_or_default();
            let comment = text[open..].starts_with("{#") && text[..close].ends_with('#');
            let reads = !comment && first.len() > 1 && first.starts_with(['.', '#'])
                || first
                    .split_once('=')
                    .is_some_and(|(name, _)| !name.is_empty() && !name.contains(['{', '"']));
            if reads && !inside.contains('\n') {
                found.push((span.start + open, text[open..=close].to_owned()));
            }
            from = close + 1;
        }
    }
    found
}

/// The endings of a name Eleventy's Markdown linked written bare, as
/// `example.org`: those it knows of three letters or more, and any of two.
const ENDINGS: &[&str] = &[
    "biz", "com", "edu", "gov", "net", "org", "pro", "web", "xxx", "aero", "asia", "coop", "info",
    "museum", "name", "shop",
];

/// Each address in the words of the writing that Eleventy's Markdown
/// linked and Statac doesn't: a name and its ending, written without
/// `http` or `www.`, by its offset.
fn bare_addresses(document: &Document, body: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    for node in words_of(document) {
        let span = node.place.span.range();
        let text = &body[span.clone()];
        let mut at = 0;
        for word in text
            .split(|c: char| c.is_whitespace() || matches!(c, '(' | ')' | '<' | '>' | '"' | '\''))
        {
            let start = at;
            at += word.len() + 1;
            let word = word.trim_end_matches(['.', ',', ':', ';', '!', '?']);
            let host = word.split('/').next().unwrap_or_default();
            let labels: Vec<&str> = host.split('.').collect();
            let ending = labels.last().copied().unwrap_or_default();
            let named = labels.len() > 1
                && labels.iter().all(|label| {
                    !label.is_empty()
                        && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
                });
            let ending_known = ENDINGS.contains(&ending.to_ascii_lowercase().as_str())
                || (ending.len() == 2 && ending.chars().all(|c| c.is_ascii_alphabetic()));
            if named
                && ending_known
                && !host.to_ascii_lowercase().starts_with("www.")
                && !word.contains('@')
                && !word.contains("://")
            {
                found.push((span.start + start, word.to_owned()));
            }
        }
    }
    found
}

/// Each place the writing holds HTML, which Eleventy's Markdown showed as
/// text: a block of it once, at its first line, and each tag among the
/// words.
fn html(document: &Document, body: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    let mut look = |node: &Node| match node.kind {
        Kind::HtmlBlock => {
            let start = node.place.span.start as usize;
            let line = body[start..].lines().next().unwrap_or_default().trim();
            found.push((start, line.to_owned()));
        }
        Kind::Html(_) => {
            let range = node.place.span.range();
            found.push((range.start, body[range].trim().to_owned()));
        }
        _ => {}
    };
    fn walk(node: &Node, look: &mut dyn FnMut(&Node)) {
        look(node);
        if node.kind != Kind::HtmlBlock {
            node.children.iter().for_each(|child| walk(child, look));
        }
    }
    walk(&document.root, &mut look);
    found
}

/// Ends each line end Statac reads inside a paragraph with `\`, so it is
/// a line break, as `breaks: true` made it; gives how many. `marks` are
/// moved with what they mark.
fn line_breaks(
    writing: &mut String,
    wrapping: &HashSet<String>,
    marks: [&mut [usize]; 2],
) -> usize {
    let document = read(writing, wrapping);
    let mut ends = Vec::new();
    let mut look = |node: &Node| {
        if node.kind == Kind::SoftBreak {
            let start = node.place.span.start as usize;
            if let Some(n) = writing[start..].find('\n') {
                let mut end = start + n;
                if end > 0 && writing.as_bytes()[end - 1] == b'\r' {
                    end -= 1;
                }
                ends.push(end);
            }
        }
    };
    document.root.walk(&mut look);
    for note in &document.notes {
        note.walk(&mut look);
    }
    ends.sort_unstable();
    ends.dedup();
    let [fences, called] = marks;
    for end in ends.iter().rev() {
        writing.insert(*end, '\\');
        for mark in fences.iter_mut().chain(called.iter_mut()) {
            if *mark > *end {
                *mark += 1;
            }
        }
    }
    ends.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_are_read_in_either_quotes_and_with_or_without_commas() {
        assert_eq!(
            values(r#""photo-1.jpg", 'A photo' 3 true"#),
            Some(vec![
                "'photo-1.jpg'".to_owned(),
                "'A photo'".to_owned(),
                "3".to_owned(),
                "true".to_owned()
            ])
        );
        assert_eq!(values("page.url"), None);
        assert_eq!(values(""), Some(Vec::new()));
    }

    #[test]
    fn line_ends_in_paragraphs_become_breaks_and_nothing_else_does() {
        let mut writing =
            "Line 1\nline 2.\n\n- Item 1\n- Item 2\n  more\n\n```\na\nb\n```\n\n<p>\na\n</p>\n"
                .to_owned();
        let count = line_breaks(&mut writing, &HashSet::new(), [&mut [], &mut []]);
        assert_eq!(count, 2);
        assert_eq!(
            writing,
            "Line 1\\\nline 2.\n\n- Item 1\n- Item 2\\\n  more\n\n```\na\nb\n```\n\n<p>\na\n</p>\n"
        );
    }
}
