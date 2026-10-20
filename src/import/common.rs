//! What more than one tool's reader uses: paths and names in the old
//! site, front matter and the values in it, positions in a page's
//! writing, and words for the report.

use std::path::Path;

use super::{Failed, Field, Value};
use crate::date::{self, Date};
use crate::problem::{Kind, Problem, system_reason};
use crate::source::{Place, Sources};
use crate::yaml::Value as Yaml;

/// The names in the folder `path`, A to Z.
pub fn names_in(path: &Path) -> Result<Vec<String>, Box<Failed>> {
    let mut names: Vec<String> = std::fs::read_dir(path)
        .map_err(|error| {
            Box::<Failed>::from(Problem::nowhere(Kind::CantRead {
                path: path.to_path_buf(),
                reason: system_reason(&error),
            }))
        })?
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    Ok(names)
}

/// `rel` joined to `folder` as a path in the old site, `""` for its top.
pub fn joined(folder: &str, rel: &str) -> String {
    match (folder.is_empty(), rel.is_empty()) {
        (true, _) => rel.to_owned(),
        (_, true) => folder.to_owned(),
        _ => format!("{folder}/{rel}"),
    }
}

/// The parent of a path, `""` at the top.
pub fn parent(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(parent, _)| parent)
}

/// The last name in a path.
pub fn last(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// The date a name opens with, `2026-10-04-`, and the rest of the name.
pub fn dated_name(name: &str) -> Option<(Date, &str)> {
    let rest = name.get(11..).filter(|rest| !rest.is_empty())?;
    let date = date::read(name.get(..10)?).ok()?;
    (name.as_bytes().get(10) == Some(&b'-')).then_some((date, rest))
}

/// Whether `text` matches the pattern `glob`: `*` any run within one
/// name, `**` any run at all, `?` one character, `{a,b}` either.
pub fn glob_matches(glob: &str, text: &str) -> bool {
    if let Some(open) = glob.find('{')
        && let Some(close) = glob[open..].find('}').map(|n| open + n)
    {
        return glob[open + 1..close].split(',').any(|choice| {
            glob_matches(
                &format!("{}{choice}{}", &glob[..open], &glob[close + 1..]),
                text,
            )
        });
    }
    let pattern: Vec<char> = glob.chars().collect();
    let text: Vec<char> = text.chars().collect();
    fn from(p: &[char], t: &[char]) -> bool {
        match p.first() {
            None => t.is_empty(),
            Some('*') if p.get(1) == Some(&'*') => {
                let rest = match p.get(2) {
                    Some('/') => &p[3..],
                    _ => &p[2..],
                };
                (0..=t.len()).any(|n| from(rest, &t[n..]))
            }
            Some('*') => (0..=t.len())
                .take_while(|n| *n == 0 || t[n - 1] != '/')
                .any(|n| from(&p[1..], &t[n..])),
            Some('?') => t.first().is_some_and(|c| *c != '/') && from(&p[1..], &t[1..]),
            Some(c) => t.first() == Some(c) && from(&p[1..], &t[1..]),
        }
    }
    from(&pattern, &text)
}

/// A file read for its front matter: each field with its line, and the
/// writing after it with the line it starts on.
pub struct Front {
    pub fields: Vec<Field>,
    pub body: String,
    pub body_line: usize,
}

/// Reads `text`'s front matter, written in YAML between `---`, TOML
/// between `+++`, or where `json` says as a JSON object; or gives the
/// line it stopped at and that line's text.
pub fn front(text: &str, path: &str, json: bool) -> Result<Front, (usize, String)> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let line_text = |line: usize| text.lines().nth(line - 1).unwrap_or_default().to_owned();
    let line_of = |at: usize| text[..at.min(text.len())].matches('\n').count() + 1;
    let first = text.lines().next().unwrap_or_default().trim_end();
    if first == "+++" {
        let start = text.find('\n').map_or(text.len(), |n| n + 1);
        let mut at = start;
        let mut end = None;
        for line in text[start..].split_inclusive('\n') {
            if line.trim_end() == "+++" {
                end = Some((at, at + line.len()));
                break;
            }
            at += line.len();
        }
        let (end, body) = end.ok_or_else(|| (1, first.to_owned()))?;
        let fields = super::toml::read(&text[start..end]).map_err(|stopped| {
            let line = stopped.line + 1;
            (line, line_text(line))
        })?;
        let fields = fields
            .into_iter()
            .map(|field| Field {
                line: field.line.map(|line| line + 1),
                ..field
            })
            .collect();
        return Ok(Front {
            fields,
            body: text[body..].to_owned(),
            body_line: line_of(body),
        });
    }
    let mut sources = Sources::default();
    if json && text.starts_with('{') {
        let end = json_end(text).ok_or_else(|| (1, first.to_owned()))?;
        let source = sources.add(path, text[..end].to_owned());
        let node = crate::yaml::read_json(&sources, source).map_err(|problem| {
            let line = problem.place.map_or(1, |place| {
                sources.line_number(place.source, place.span.start as usize)
            });
            (line, line_text(line))
        })?;
        let body = text[end..]
            .strip_prefix("\r\n")
            .or_else(|| text[end..].strip_prefix('\n'))
            .unwrap_or(&text[end..]);
        let body_start = text.len() - body.len();
        return Ok(Front {
            fields: mapping(&sources, &node),
            body: body.to_owned(),
            body_line: line_of(body_start),
        });
    }
    if first != "---" {
        return Ok(Front {
            fields: Vec::new(),
            body: text.to_owned(),
            body_line: 1,
        });
    }
    let mended = super::mend::mended(text, false).text;
    let text = mended.as_str();
    let source = sources.add(path, text.to_owned());
    let split = crate::yaml::split(text).map_err(|_| (1, first.to_owned()))?;
    let mut fields = Vec::new();
    if let Some(span) = split.front_matter {
        let node = crate::yaml::read_at(&sources, Place { source, span }).map_err(|problem| {
            let line = problem.place.map_or(1, |place| {
                sources.line_number(place.source, place.span.start as usize)
            });
            (line, line_text(line))
        })?;
        fields = mapping(&sources, &node);
    }
    Ok(Front {
        fields,
        body: text[split.body.range()].to_owned(),
        body_line: line_of(split.body.start as usize),
    })
}

/// Where the JSON object `text` opens with ends, just past its `}`.
fn json_end(text: &str) -> Option<usize> {
    let mut depth = 0;
    let mut in_string = false;
    let mut escaped = false;
    for (at, c) in text.char_indices() {
        match (in_string, c) {
            (true, _) if escaped => escaped = false,
            (true, '\\') => escaped = true,
            (true, '"') => in_string = false,
            (true, _) => {}
            (false, '"') => in_string = true,
            (false, '{' | '[') => depth += 1,
            (false, '}' | ']') => {
                depth -= 1;
                if depth == 0 {
                    return Some(at + 1);
                }
            }
            _ => {}
        }
    }
    None
}

/// A mapping's fields, each with its line.
pub fn mapping(sources: &Sources, node: &crate::yaml::Node) -> Vec<Field> {
    match &node.value {
        Yaml::Mapping(entries) => entries
            .iter()
            .map(|entry| {
                let line = sources
                    .line_number(entry.key.place.source, entry.key.place.span.start as usize);
                let value = super::yaml::value_of(&entry.value, &super::yaml::core);
                Field::new(&super::yaml::key_text(&entry.key), value, Some(line))
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// A value as text, where it is a scalar.
pub fn text_of(value: &Value) -> Option<String> {
    match value {
        Value::Text(text) | Value::Number(text) => Some(text.clone()),
        Value::Truth(truth) => Some(truth.to_string()),
        _ => None,
    }
}

/// A value as a list of texts: a list's scalars, or one scalar.
pub fn texts_of(value: &Value) -> Vec<String> {
    match value {
        Value::List(items) => items.iter().filter_map(text_of).collect(),
        Value::Nothing => Vec::new(),
        other => text_of(other).into_iter().collect(),
    }
}

/// A table's fields, or none.
pub fn fields(value: &Value) -> &[(String, Value)] {
    match value {
        Value::Fields(fields) => fields,
        _ => &[],
    }
}

/// Whether only spaces stand between the start of `at`'s line and `at`.
pub fn opens_line(text: &str, at: usize) -> bool {
    let line = text[..at].rfind('\n').map_or(0, |i| i + 1);
    text[line..at].trim().is_empty()
}

/// Whether only spaces stand between `at` and the end of its line.
pub fn ends_line(text: &str, at: usize) -> bool {
    let end = text[at..].find('\n').map_or(text.len(), |i| at + i);
    text[at..end].trim().is_empty()
}

/// Each `:name:` in `line`, by its offset.
pub fn emoji_names(line: &str) -> Vec<(usize, &str)> {
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

/// `n` of a thing, the plural right.
pub fn counted(n: usize, one: &str, many: &str) -> String {
    match n {
        1 => format!("1 {one}"),
        _ => format!("{n} {many}"),
    }
}

/// `a`, `a and b`, `a, b and c`.
pub fn listed(items: &[String]) -> String {
    match items.split_last() {
        Some((last, [])) => last.clone(),
        Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
        None => String::new(),
    }
}
