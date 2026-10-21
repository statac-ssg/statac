//! What the readers of tools that run Liquid, or a language written like
//! it, share: where Markdown reads code, where a tag `{% %}` or `{{ }}`
//! starts and ends, `raw`, `highlight` as a fenced code block, an include
//! as a snippet call and the snippet standing for it, and an excerpt
//! marker as `<!--more-->`.

use std::ops::Range;

use pulldown_cmark::{Event, Options, Parser, Tag};

use super::{Finding, FindingKind, Spot};

pub(super) fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_DEFINITION_LIST
}

/// The ranges of `text` Markdown reads as code: code blocks, fenced or
/// indented, and code spans.
pub(super) fn code_ranges(text: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    for (event, range) in Parser::new_ext(text, options()).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(_)) | Event::Code(_) => ranges.push(range),
            _ => {}
        }
    }
    ranges
}

pub(super) fn within(ranges: &[Range<usize>], at: usize) -> bool {
    ranges.iter().any(|range| range.contains(&at))
}

pub(super) fn finding(kind: &'static FindingKind, spot: Spot) -> Finding {
    Finding {
        kind,
        about: None,
        spots: vec![spot],
    }
}

/// The tag starting at `at`, `{%` or `{{`, as far as its end: its whole
/// length and what is inside, without the marks of whitespace control.
pub(super) fn tag_at(text: &str, at: usize) -> Option<(usize, String, bool)> {
    let statement = text[at..].starts_with("{%");
    let close = if statement { "%}" } else { "}}" };
    let end = text[at + 2..].find(close)? + at + 2;
    let inner = text[at + 2..end]
        .trim_start_matches('-')
        .trim_end_matches('-')
        .trim()
        .to_owned();
    Some((end + 2 - at, inner, statement))
}

/// The first word of a statement and the rest.
pub(super) fn split_word(inner: &str) -> (&str, &str) {
    match inner.split_once(char::is_whitespace) {
        Some((word, rest)) => (word, rest.trim()),
        None => (inner, ""),
    }
}

/// Where the statement named `name` next stands from `from`: its start
/// and its length.
pub(super) fn find_statement(text: &str, from: usize, name: &str) -> Option<(usize, usize)> {
    let mut at = from;
    while let Some(found) = text[at..].find("{%") {
        let start = at + found;
        if let Some((length, inner, _)) = tag_at(text, start) {
            if split_word(&inner).0 == name {
                return Some((start, length));
            }
            at = start + length;
        } else {
            return None;
        }
    }
    None
}

/// The whole line of the tag at `start..end`, its line ending with it,
/// where the tag stands alone on its line, so taking it out leaves no
/// empty line; else the tag alone.
pub(super) fn whole_line(body: &str, start: usize, end: usize) -> (usize, usize) {
    let line = body[..start].rfind('\n').map_or(0, |i| i + 1);
    let next = body[end..].find('\n').map_or(body.len(), |i| end + i + 1);
    match body[line..start].trim().is_empty() && body[end..next].trim().is_empty() {
        true => (line, next),
        false => (start, end),
    }
}

/// `raw` at `start..end`, up to its `close` statement, taken out and what
/// it held kept: where the writing before it ends, what it held, and where
/// the writing after it starts. None where it is never closed.
pub(super) fn raw(
    body: &str,
    start: usize,
    end: usize,
    close: &str,
) -> Option<(usize, Range<usize>, usize)> {
    let (closed, close_length) = find_statement(body, end, close)?;
    let (from, held_from) = whole_line(body, start, end);
    let (held_to, resume) = whole_line(body, closed, closed + close_length);
    Some((from, held_from.min(held_to)..held_to, resume))
}

/// `{% highlight lang %}` to its `{% endhighlight %}` as a fenced code
/// block, `linenos` dropped. None where it opens no line, holds another
/// option, or is never closed.
pub(super) fn highlight(
    body: &str,
    start: usize,
    end: usize,
    rest: &str,
) -> Option<(String, usize)> {
    let mut words = rest.split_whitespace();
    let language = words.next().unwrap_or_default();
    if words.any(|word| word != "linenos") {
        return None;
    }
    let opens_line = body[..start].rfind('\n').map_or(0, |i| i + 1);
    if !body[opens_line..start].trim().is_empty() {
        return None;
    }
    let (close, close_length) = find_statement(body, end, "endhighlight")?;
    let inside = &body[end..close];
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
    Some((
        format!("{fence}{language}\n{inside}\n{fence}"),
        close + close_length,
    ))
}

/// The snippet standing for the include `file`: its name with each
/// character a snippet's name can't hold as `_`, kept clear of the
/// starter's where the new site has it.
pub(super) fn include_snippet(file: &str, starter: bool) -> String {
    let stem = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
    let mut snippet: String = stem
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    if !snippet.starts_with(|c: char| c.is_ascii_alphabetic()) {
        snippet = format!("include_{snippet}");
    }
    if starter && crate::new::has_snippet(&snippet) {
        snippet = format!("old_{snippet}");
    }
    snippet
}

/// `text` as a value in a snippet call, in single quotes.
pub(super) fn quoted(text: &str) -> String {
    format!("'{}'", text.replace('\\', "\\\\").replace('\'', "\\'"))
}

/// A value in quotes at the start of `text`, `"` or `'`, with a backslash
/// keeping the character after it: the value and what follows it. None
/// where it isn't closed on its line.
pub(super) fn quoted_value(text: &str) -> Option<(String, &str)> {
    let quote = text.chars().next().filter(|c| matches!(c, '"' | '\''))?;
    let mut value = String::new();
    let mut chars = text[1..].char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '\\' => {
                if let Some((_, next)) = chars.next() {
                    value.push(next);
                }
            }
            c if c == quote => return Some((value, &text[i + 2..])),
            '\n' | '\r' => return None,
            c => value.push(c),
        }
    }
    None
}

/// An include as a snippet call: the snippet's name, the names handed,
/// and the call. None where a value is a variable, or a name can't be one
/// of a call's.
pub(super) fn include(rest: &str, starter: bool) -> Option<(String, Vec<String>, String)> {
    let (file, mut params) = match rest.split_once(char::is_whitespace) {
        Some((file, params)) => (file, params.trim()),
        None => (rest, ""),
    };
    if file.contains(['{', '}']) || file.is_empty() {
        return None;
    }
    let snippet = include_snippet(file, starter);
    let mut names = Vec::new();
    let mut handed = Vec::new();
    while !params.is_empty() {
        let (key, after) = params.split_once('=')?;
        let key = key.trim();
        if key.is_empty() || !key.starts_with(|c: char| c.is_ascii_alphabetic()) {
            return None;
        }
        if !key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return None;
        }
        let (value, after) = quoted_value(after.trim_start())?;
        params = after.trim_start();
        let name = key.replace('-', "_");
        handed.push(format!("{name}={}", quoted(&value)));
        names.push(name);
    }
    let call = format!("@{snippet}({})", handed.join(", "));
    Some((snippet, names, call))
}

/// The text of a snippet standing for `what`, such as ``include
/// `note.html` ``, that shows each of `names` in a comment saying it
/// wasn't converted, and with `values` the values handed without names.
pub(super) fn stand_in(what: &str, names: &[String], values: bool) -> String {
    let what = what.replace("--", "- -").replace(['{', '}'], "");
    let mut shown: String = names
        .iter()
        .map(|name| format!(" {name}: {{{{ {name} | default(\"\") }}}}"))
        .collect();
    let mut takes = String::new();
    if values {
        takes.push_str("{# takes values... #}\n");
        shown.push_str(" values: {{ values | join(\", \") }}");
    }
    format!(
        "{takes}<!-- Not converted: this snippet stands for the old site's {what}. Write it as \
         https://statac.dev/snippets/ describes, or take its calls out: see IMPORT.md.{shown} \
         -->\n"
    )
}

/// The writing with each line that holds the excerpt separator and
/// nothing else holding `<!--more-->`, outside code blocks. Each of
/// `marks`, offsets into the text in order, is moved with what it marks.
pub(super) fn excerpt(text: &str, separator: &str, mut marks: [&mut [usize]; 2]) -> String {
    let code = code_ranges(text);
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    let mut next = [0; 2];
    for line in text.split_inclusive('\n') {
        for (marks, next) in marks.iter_mut().zip(&mut next) {
            while *next < marks.len() && marks[*next] < at + line.len() {
                marks[*next] = marks[*next] - at + out.len();
                *next += 1;
            }
        }
        let bare = line.trim_end_matches(['\n', '\r']);
        if bare.trim() == separator && !within(&code, at) {
            out.push_str("<!--more-->");
            out.push_str(&line[bare.len()..]);
        } else {
            out.push_str(line);
        }
        at += line.len();
    }
    out
}

/// Where `separator` stands in `body` outside code with other writing on
/// its line, so it isn't made `<!--more-->`.
pub(super) fn excerpts_inside(body: &str, separator: &str) -> Vec<usize> {
    let code = code_ranges(body);
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(at) = body[from..].find(separator) {
        let start = from + at;
        from = start + separator.len();
        if within(&code, start) {
            continue;
        }
        let line_start = body[..start].rfind('\n').map_or(0, |i| i + 1);
        let line_end = body[start..].find('\n').map_or(body.len(), |i| start + i);
        if body[line_start..line_end].trim() != separator {
            found.push(start);
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_include_named_as_a_starter_snippet_is_kept_clear_of_it() {
        for (file, with, without) in [
            ("figure.html", "old_figure", "figure"),
            ("called.html", "old_called", "called"),
            ("thumb.html", "old_thumb", "thumb"),
            ("file.md", "old_file", "file"),
            ("card.html", "card", "card"),
            ("callout.html", "callout", "callout"),
            ("post-meta.html", "post_meta", "post_meta"),
        ] {
            assert_eq!(include_snippet(file, true), with, "{file}");
            assert_eq!(include_snippet(file, false), without, "{file}");
        }
    }
}
