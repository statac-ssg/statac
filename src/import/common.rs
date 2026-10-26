//! What more than one tool's reader uses: paths and names in the old
//! site, front matter and the values in it, positions in a page's
//! writing, and words for the report.

use std::path::Path;

use super::report::code;
use super::{Failed, Field, Unread, Value};
use crate::content::address::Address;
use crate::date::{self, Date};
use crate::problem::{FeedFormat, Kind, Problem, Reason};
use crate::source::{Place, Sources};
use crate::yaml::Value as Yaml;

/// The names in the folder `path`, A to Z.
pub(super) fn names_in(path: &Path) -> Result<Vec<String>, Box<Failed>> {
    let mut names: Vec<String> = std::fs::read_dir(path)
        .map_err(|error| {
            Box::<Failed>::from(Problem::nowhere(Kind::CantRead {
                path: path.to_path_buf(),
                reason: Reason::of(&error),
            }))
        })?
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    Ok(names)
}

/// `rel` joined to `folder` as a path in the old site, `""` for its top.
pub(super) fn joined(folder: &str, rel: &str) -> String {
    match (folder.is_empty(), rel.is_empty()) {
        (true, _) => rel.to_owned(),
        (_, true) => folder.to_owned(),
        _ => format!("{folder}/{rel}"),
    }
}

/// The parent of a path, `""` at the top.
pub(super) fn parent(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(parent, _)| parent)
}

/// The last name in a path.
pub(super) fn last(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// The date a name opens with, `2026-10-04-`, and the rest of the name.
pub(super) fn dated_name(name: &str) -> Option<(Date, &str)> {
    let rest = name.get(11..).filter(|rest| !rest.is_empty())?;
    let date = date::read(name.get(..10)?).ok()?;
    (name.as_bytes().get(10) == Some(&b'-')).then_some((date, rest))
}

/// Whether `text` matches the pattern `glob`: `*` any run within one
/// name, `**` any run at all, `?` one character, `{a,b}` either.
pub(super) fn glob_matches(glob: &str, text: &str) -> bool {
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
pub(super) struct Front {
    pub fields: Vec<Field>,
    pub body: String,
    pub body_line: usize,
    /// The front matter, where it is YAML Statac's reader stopped at: the
    /// page is then read as if it had none.
    pub unread: Option<Unread>,
}

/// Reads `text`'s front matter, written in YAML between `---`, TOML
/// between `+++`, or where `json` says as a JSON object; or gives the
/// line it stopped at and that line's text, where it isn't YAML.
pub(super) fn front(text: &str, path: &str, json: bool) -> Result<Front, (usize, String)> {
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
            unread: None,
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
            unread: None,
        });
    }
    if first != "---" {
        return Ok(Front {
            fields: Vec::new(),
            body: text.to_owned(),
            body_line: 1,
            unread: None,
        });
    }
    let source = sources.add(path, text.to_owned());
    let split = crate::yaml::split(text).map_err(|_| (1, first.to_owned()))?;
    let mut fields = Vec::new();
    let mut unread = None;
    if let Some(span) = split.front_matter {
        match crate::yaml::read_at(&sources, Place { source, span }) {
            Ok(node) => fields = mapping(&sources, &node),
            Err(problem) => unread = Some(unread_at(&sources, &problem, path, &text[span.range()])),
        }
    }
    Ok(Front {
        fields,
        body: text[split.body.range()].to_owned(),
        body_line: line_of(split.body.start as usize),
        unread,
    })
}

/// What is said of `front`, the front matter of the old file `path`, as
/// written, where Statac's YAML reader stopped at `problem` in it.
pub(super) fn unread_at(sources: &Sources, problem: &Problem, path: &str, front: &str) -> Unread {
    let line = problem.place.map_or(1, |place| {
        sources.line_number(place.source, place.span.start as usize)
    });
    let text = problem.place.map_or("", |place| sources.text(place.source));
    Unread {
        from: path.to_owned(),
        line,
        text: text.lines().nth(line - 1).unwrap_or_default().to_owned(),
        front: front.to_owned(),
    }
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
pub(super) fn mapping(sources: &Sources, node: &crate::yaml::Node) -> Vec<Field> {
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
pub(super) fn text_of(value: &Value) -> Option<String> {
    match value {
        Value::Text(text) | Value::Number(text) => Some(text.clone()),
        Value::Truth(truth) => Some(truth.to_string()),
        _ => None,
    }
}

/// A value as a list of texts: a list's scalars, or one scalar.
pub(super) fn texts_of(value: &Value) -> Vec<String> {
    match value {
        Value::List(items) => items.iter().filter_map(text_of).collect(),
        Value::Nothing => Vec::new(),
        other => text_of(other).into_iter().collect(),
    }
}

/// A table's fields, or none.
pub(super) fn fields(value: &Value) -> &[(String, Value)] {
    match value {
        Value::Fields(fields) => fields,
        _ => &[],
    }
}

/// Whether only spaces stand between the start of `at`'s line and `at`.
pub(super) fn opens_line(text: &str, at: usize) -> bool {
    let line = text[..at].rfind('\n').map_or(0, |i| i + 1);
    text[line..at].trim().is_empty()
}

/// Whether only spaces stand between `at` and the end of its line.
pub(super) fn ends_line(text: &str, at: usize) -> bool {
    let end = text[at..].find('\n').map_or(text.len(), |i| at + i);
    text[at..end].trim().is_empty()
}

/// Each `:name:` in `line`, by its offset.
pub(super) fn emoji_names(line: &str) -> Vec<(usize, &str)> {
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
pub(super) fn counted(n: usize, one: &str, many: &str) -> String {
    match n {
        1 => format!("1 {one}"),
        _ => format!("{n} {many}"),
    }
}

/// `a`, `a and b`, `a, b and c`.
pub(super) fn listed(items: &[String]) -> String {
    match items.split_last() {
        Some((last, [])) => last.clone(),
        Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
        None => String::new(),
    }
}

/// The template the report names for each of `feeds`, a list's feeds by
/// their file names and their formats where known: for an RSS feed the one
/// named as its file, as `templates/rss.xml`, and for any other the one
/// `templates/feed.xml` or `templates/feed.json` would be where the site has
/// only those, the docs' `feed.xml` being Atom, unless an RSS feed's is
/// that one, when it too is the one named as its file.
pub(super) fn feed_templates(feeds: &[(String, Option<FeedFormat>)]) -> Vec<String> {
    let looked = |name: &str, has: &dyn Fn(&str) -> bool| {
        let address = Address::read_file(&format!("/{name}")).ok()?;
        crate::feeds::template(&address, has)
    };
    let own = |name: &str| looked(name, &|_| true);
    let rss: Vec<String> = (feeds.iter())
        .filter(|(_, format)| *format == Some(FeedFormat::Rss))
        .filter_map(|(name, _)| own(name))
        .collect();
    (feeds.iter())
        .map(|(name, format)| {
            let template = match format {
                Some(FeedFormat::Rss) => own(name),
                _ => looked(name, &|t| matches!(t, "feed.xml" | "feed.json"))
                    .filter(|template| !rss.contains(template))
                    .or_else(|| own(name)),
            };
            format!(
                "templates/{}",
                template.as_deref().unwrap_or(crate::feeds::FILE_NAME)
            )
        })
        .collect()
}

/// What is said of the templates for the feeds of each list in `lists`,
/// each feed by its file name and its format where known, as
/// [`feed_templates`] names them: of each feed built with the template
/// another of the list's feeds is, of another format or of one not known,
/// until the site has a template named for it; and which of the docs'
/// templates to copy as which where any feed is RSS, or else where they
/// are where any feed is so built; none where nothing is. A template so
/// shared is taken to be for the feed named as it, or else the first of
/// the format the docs' template of that name is, Atom for `feed.xml` and
/// JSON for `feed.json`, or else the first.
pub(super) fn feed_templates_said(lists: &[Vec<(String, Option<FeedFormat>)>]) -> Option<String> {
    let mut sentences: Vec<String> = Vec::new();
    // The templates to copy from the docs' Atom and RSS templates.
    let (mut atom, mut rss): (Vec<String>, Vec<String>) = (Vec::new(), Vec::new());
    for feeds in lists {
        let templates = feed_templates(feeds);
        let built: Vec<(&str, Option<FeedFormat>, &String)> = (feeds.iter().zip(&templates))
            .map(|((name, format), template)| (name.as_str(), *format, template))
            .collect();
        for &(name, format, template) in &built {
            let copies = match format {
                Some(FeedFormat::Atom) => Some(&mut atom),
                Some(FeedFormat::Rss) => Some(&mut rss),
                _ => None,
            };
            if let Some(copies) = copies
                && !copies.contains(template)
            {
                copies.push(template.clone());
            }
            let alike: Vec<_> = built.iter().filter(|(.., t)| *t == template).collect();
            let docs = match template.as_str() {
                "templates/feed.json" => FeedFormat::Json,
                _ => FeedFormat::Atom,
            };
            let Some(&&(kept, kept_format, _)) = (alike.iter())
                .find(|(n, ..)| format!("templates/{n}") == *template)
                .or_else(|| alike.iter().find(|(_, f, _)| *f == Some(docs)))
                .or(alike.first())
            else {
                continue;
            };
            if name == kept || (kept_format.is_some() && format == kept_format) {
                continue;
            }
            let sentence = format!(
                "{} needs {}: until you add it, it is built with {}, in the same format as {}.",
                code(name),
                code(&format!("templates/{name}")),
                code(template),
                code(kept)
            );
            if !sentences.contains(&sentence) {
                sentences.push(sentence);
            }
        }
    }
    let shown = |templates: &[String]| {
        let shown: Vec<String> = templates.iter().map(|t| code(t)).collect();
        listed(&shown)
    };
    let see = "https://statac.dev/feeds/#the-template";
    match (atom.is_empty(), rss.is_empty()) {
        (_, false) => sentences.push(match atom.is_empty() {
            true => format!("Copy the docs' RSS template as {}: see {see}.", shown(&rss)),
            false => format!(
                "Copy the docs' Atom template as {} and its RSS template as {}: see {see}.",
                shown(&atom),
                shown(&rss)
            ),
        }),
        _ if !sentences.is_empty() => sentences.push(format!(
            "The docs have an Atom template and an RSS template to copy: see {see}."
        )),
        _ => {}
    }
    (!sentences.is_empty()).then(|| sentences.join(" "))
}
