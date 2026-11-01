//! An Eleventy page's writing: the template tags the old site ran over it
//! first, in Liquid or Nunjucks, each converted where Statac's writing has
//! an equal and otherwise left exactly as written; the lines the
//! highlighting plugin marked; the excerpt marker; and
//! the Markdown the old site's settings read differently, each line end
//! inside a paragraph made a line break where they had `breaks: true`, and
//! the rest counted. Nothing inside a code block or a code span is
//! converted, but for what `raw` held.

use std::collections::HashSet;
use std::ops::Range;

use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};

use crate::import::common::{ends_line, line_words, opens_line};
use crate::import::liquid::{
    code_ranges, excerpt, excerpts_inside, find_statement, finding, highlight, include_snippet,
    options, quoted, quoted_value, raw, split_word, tag_at, within,
};
use crate::import::{Finding, Spot};
use crate::markdown::{self, Document, Kind, Node};
use crate::source::{Sources, Span};

use super::code;
use super::{
    ATTRIBUTES, CODE_LINES, CODE_OPTIONS, EXCERPT_LEFT, HTML_SHOWN, LINK_NOT_MADE, TAG_IN_CODE,
    TAG_LEFT,
};

/// The language a page's tags are written in.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(super) enum Engine {
    Liquid,
    Nunjucks,
    /// None ran, or one whose tags Statac doesn't read.
    Not,
}

/// What the writing is converted with.
pub(super) struct Context<'a> {
    pub(crate) engine: Engine,
    /// Whether the page is Markdown, rather than HTML.
    pub(crate) markdown: bool,
    pub(crate) separator: Option<&'a str>,
    /// The tags of the site's own that some call anywhere wraps writing
    /// with.
    pub(crate) wrapping: &'a HashSet<String>,
    pub(crate) breaks: bool,
    pub(crate) attributes: bool,
    pub(crate) linkify: bool,
    /// Whether the old site's Markdown showed HTML as text.
    pub(crate) html_off: bool,
    /// Whether the old site used Eleventy's highlighting plugin, which
    /// reads ```` ```js/1,3 ```` as the language `js` with lines to mark.
    pub(crate) highlighter: bool,
    /// The path the old site was published under, as `/blog`, or empty.
    pub(crate) base: &'a str,
}

/// A snippet call put in place of an include or a tag of the site's own.
#[derive(Clone, Debug)]
pub(super) struct Call {
    pub(crate) snippet: String,
    /// What it stands for, as the snippet's comment says it: ``include
    /// `note.html` `` or ``tag `image` ``.
    pub(crate) what: String,
    pub(crate) include: bool,
    /// The names it hands, in order.
    pub(crate) names: Vec<String>,
    /// Whether it hands values without names.
    pub(crate) positional: bool,
    pub(crate) wraps: bool,
    pub(crate) spot: Spot,
}

/// The writing converted, with what didn't convert.
pub(super) struct Converted {
    pub(crate) writing: String,
    pub(crate) findings: Vec<Finding>,
    pub(crate) calls: Vec<Call>,
    pub(crate) fences: Vec<usize>,
    pub(crate) called: Vec<usize>,
    /// How many line ends were made line breaks.
    pub(crate) breaks: usize,
    /// How many places hold the separator alone on a line, made
    /// `<!--more-->`.
    pub(crate) separators: usize,
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
fn include(rest: &str) -> Option<(Call, String)> {
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
    let snippet = include_snippet(&file);
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

/// The snippet a tag of the site's own calls: its name, with each
/// character a snippet's name can't hold as `_`.
fn own_snippet(tag: &str) -> String {
    let name: String = tag
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    match name.starts_with(|c: char| c.is_ascii_alphabetic()) {
        true => name,
        false => format!("tag_{name}"),
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
pub(super) fn paired(body: &str) -> Vec<String> {
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
pub(super) fn convert(body: &str, first_line: usize, file: &str, cx: &Context) -> Converted {
    let spot_in = |body: &str, at: usize, text: &str| Spot {
        file: file.to_owned(),
        line: Some(first_line + body[..at].matches('\n').count()),
        text: text.to_owned(),
        times: None,
    };
    let mut findings = Vec::new();
    let fenced;
    let body = match cx.markdown && cx.highlighter {
        true => {
            fenced = fences_converted(body, &|at, text| spot_in(body, at, text), &mut findings);
            fenced.as_str()
        }
        false => body,
    };
    // Offsets from here on are into the writing with its fences
    // converted, which has the same lines.
    let spot = |at: usize, text: &str| spot_in(body, at, text);
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
                    let given = quoted_value(words).map_or(words.to_owned(), |(l, _)| l);
                    let (language, asked) = code::from_tag(&given);
                    let block = match language.contains(|c: char| c.is_whitespace() || c == '`') {
                        true => None,
                        false => highlighted(body, start, end, language, &asked),
                    };
                    match block {
                        Some((text, resume, marked, changed)) => {
                            if marked {
                                findings.push(finding(&CODE_LINES, spot(start, written)));
                            }
                            if changed {
                                findings.push(finding(&CODE_OPTIONS, spot(start, written)));
                            }
                            out.push_str(&body[copied..start]);
                            fences.push(out.len());
                            out.push_str(&text);
                            copied = resume;
                            at = resume;
                        }
                        None => left(&mut findings),
                    }
                }
                "include" | "render" => match include(words) {
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
                            let snippet = own_snippet(name);
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
            let wrapping: HashSet<String> =
                cx.wrapping.iter().map(|name| own_snippet(name)).collect();
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

/// The `highlight` tag at `start..end` as a fenced code block in
/// `language`, with the lines `asked` marks marked: the block, where the
/// writing resumes, whether it marks lines, and whether lines the plugin
/// marked as added or removed were taken off. None where
/// [`highlight`] gives none. The plugin counts lines from 0 in the code
/// with the blank lines round it trimmed, which the block keeps, but for
/// the line ending straight after the tag.
fn highlighted(
    body: &str,
    start: usize,
    end: usize,
    language: &str,
    asked: &code::Asked,
) -> Option<(String, usize, bool, bool)> {
    let (close, _) = find_statement(body, end, "endhighlight")?;
    let inside = &body[end..close];
    let count = i64::try_from(inside.trim().split('\n').count()).unwrap_or(i64::MAX);
    let trimmed = i64::try_from(
        inside[..inside.len() - inside.trim_start().len()]
            .matches('\n')
            .count(),
    )
    .unwrap_or(0);
    let dropped = i64::from(inside.starts_with('\n') || inside.starts_with("\r\n"));
    let first = 1 + trimmed - dropped;
    let placed = |runs: &[(i64, i64)]| -> Vec<(i64, i64)> {
        runs.iter()
            .map(|&(from, to)| (from, to.min(count - 1)))
            .filter(|(from, to)| from <= to)
            .map(|(from, to)| (from + first, to + first))
            .collect()
    };
    let changed = !placed(&asked.changed).is_empty();
    let (text, resume, marked) =
        highlight(body, start, end, language, false, &placed(&asked.marked))?;
    Some((text, resume, marked, changed))
}

/// The writing with the first word of each fenced code block's opening
/// line that the highlighting plugin read as a language and lines to mark,
/// `js/1,3`, written as the language with Statac's words for the marked
/// lines at the line's end, `js mark=2,4`, and the lines it marked as added
/// or removed taken off, each told.
fn fences_converted(
    body: &str,
    spot: &dyn Fn(usize, &str) -> Spot,
    findings: &mut Vec<Finding>,
) -> String {
    let mut changes: Vec<(Range<usize>, String)> = Vec::new();
    let mut open: Option<(usize, String)> = None;
    let mut code = String::new();
    for (event, range) in Parser::new_ext(body, options()).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) => {
                open = Some((range.start, info.into_string()));
                code.clear();
            }
            Event::Text(text) if open.is_some() => code.push_str(&text),
            Event::End(TagEnd::CodeBlock) => {
                if let Some((start, info)) = open.take()
                    && let Some(change) = fence_converted(body, start, &info, &code, spot, findings)
                {
                    changes.push(change);
                }
            }
            _ => {}
        }
    }
    let mut out = String::with_capacity(body.len());
    let mut copied = 0;
    for (cut, words) in changes {
        out.push_str(&body[copied..cut.start]);
        out.push_str(&words);
        copied = cut.end;
    }
    out.push_str(&body[copied..]);
    out
}

/// The opening line of the fenced code block at `start`, whose info is
/// `info` and code `code`, from its first word to its end, and what to
/// write in its place, each change told. None where the first word isn't
/// in the plugin's form.
fn fence_converted(
    body: &str,
    start: usize,
    info: &str,
    code: &str,
    spot: &dyn Fn(usize, &str) -> Spot,
    findings: &mut Vec<Finding>,
) -> Option<(Range<usize>, String)> {
    let line_end = body[start..].find('\n').map_or(body.len(), |i| start + i);
    let line = body[start..line_end].trim_end_matches('\r');
    let fence = line.find(['`', '~'])?;
    let after = line[fence..].trim_start_matches(['`', '~']).trim_start();
    let word_start = line.len() - after.len();
    let word_end = word_start + after.find(char::is_whitespace).unwrap_or(after.len());
    let word = &line[word_start..word_end];
    if info.split_whitespace().next() != Some(word) {
        return None;
    }
    let (language, asked) = code::from_fence(word)?;
    let counted_from_1 = |runs: &[(i64, i64)]| -> Vec<(i64, i64)> {
        runs.iter()
            .map(|&(from, to)| (from.saturating_add(1), to.saturating_add(1)))
            .collect()
    };
    let words = line_words(false, &counted_from_1(&asked.marked), code);
    if !words.is_empty() {
        findings.push(finding(&CODE_LINES, spot(start, line.trim())));
    }
    if !line_words(false, &counted_from_1(&asked.changed), code).is_empty() {
        findings.push(finding(&CODE_OPTIONS, spot(start, line.trim())));
    }
    let rest = line[word_end..].trim_end();
    Some((
        start + word_start..start + line.len(),
        format!("{language}{rest}{words}"),
    ))
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
    fn a_fence_is_read_for_marked_lines_only_where_the_site_used_the_plugin() {
        let wrapping = HashSet::new();
        let cx = |highlighter| Context {
            engine: Engine::Liquid,
            markdown: true,
            separator: None,
            wrapping: &wrapping,
            breaks: false,
            attributes: false,
            linkify: false,
            html_off: false,
            highlighter,
            base: "",
        };
        for (body, converted) in [
            (
                "```js/1,3\na\nb\nc\nd\n```\n",
                "```js mark=2,4\na\nb\nc\nd\n```\n",
            ),
            ("~~~ js/0 \r\na\r\n~~~\r\n", "~~~ js mark=1\r\na\r\n~~~\r\n"),
            (
                "> ```js/1\n> a\n> b\n> ```\n",
                "> ```js mark=2\n> a\n> b\n> ```\n",
            ),
            ("- ```js/0\n  a\n  ```\n", "- ```js mark=1\n  a\n  ```\n"),
            ("```text/html\na\n```\n", "```text/html\na\n```\n"),
            ("    js/1\n", "    js/1\n"),
        ] {
            assert_eq!(convert(body, 1, "post.md", &cx(false)).writing, body);
            assert_eq!(convert(body, 1, "post.md", &cx(true)).writing, converted);
        }
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
