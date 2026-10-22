//! Fixes: where a problem's suggestion names one exact edit, that edit,
//! for the editor to make when asked. Each writes in place of text at the
//! problem's own place, and only where that text is what the problem
//! says was written there.

#[cfg(test)]
mod tests;

use crate::problem::{Chooser, Kind, Problem, Word};
use crate::source::{Place, Sources, Span};
use crate::wording;

/// One way to fix a problem: `text` written in place of what is at
/// `place`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) struct Fix {
    pub title: String,
    pub place: Place,
    pub text: String,
}

/// The fixes for `problem`, read against the files in `sources`, in a
/// site published under the path `base`.
pub(super) fn fixes(problem: &Problem, sources: &Sources, base: &str) -> Vec<Fix> {
    let Some(place) = problem.place else {
        return Vec::new();
    };
    if problem.kind.of_a_whole_file() {
        return Vec::new();
    }
    let whole = sources.text(place.source);
    let Some(at) = whole.get(place.span.range()) else {
        return Vec::new();
    };
    // `written` where it stands once in what the problem is placed at, and
    // isn't a key respelt as one its mapping has.
    let respelt = |written: &str, text: &str| {
        let mut found = at.match_indices(written);
        let (start, _) = found.next().filter(|_| !written.is_empty())?;
        if found.next().is_some() {
            return None;
        }
        let start = place.span.start as usize + start;
        if taken(whole, start, start + written.len(), text) {
            return None;
        }
        Some(Fix {
            title: wording::fix_written(text),
            place: Place {
                source: place.source,
                span: Span::new(start, start + written.len()),
            },
            text: text.to_owned(),
        })
    };
    match &problem.kind {
        Kind::UnknownWord {
            written,
            nearest: Some(nearest),
            ..
        } => respelt(written, nearest).into_iter().collect(),
        Kind::ReadAsYourOwn {
            written, nearest, ..
        } => respelt(written, nearest).into_iter().collect(),
        // Front matter in TOML is rewritten, not respelt.
        Kind::OtherSpelling {
            word: Word::Marker,
            written,
            ..
        } if written == "+++" => Vec::new(),
        Kind::OtherSpelling { written, ours, .. } => respelt(written, ours).into_iter().collect(),
        Kind::NeedsQuotes { written, .. } => {
            let quoted = wording::kept_quoted(written);
            respelt(written, &quoted).into_iter().collect()
        }
        Kind::UnclearDate { written, readings } => (readings.iter())
            .filter_map(|(iso, _)| respelt(written, iso))
            .collect(),
        Kind::MissingTemplate {
            path,
            chosen: Chooser::Written,
            nearest: Some(nearest),
        } => {
            let name = path.strip_prefix("templates/").unwrap_or(path);
            let name = name.strip_suffix(".html").unwrap_or(name);
            let nearest = nearest.strip_suffix(".html").unwrap_or(nearest);
            respelt(name, nearest).into_iter().collect()
        }
        Kind::BrokenLink {
            target,
            suggestion: Some(suggestion),
            ..
        } if writes(at.trim(), base).as_deref() == Some(target.trim()) => {
            let start = place.span.start as usize + (at.len() - at.trim_start().len());
            vec![Fix {
                title: wording::fix_written(suggestion),
                place: Place {
                    source: place.source,
                    span: Span::new(start, start + at.trim().len()),
                },
                text: suggestion.clone(),
            }]
        }
        _ => Vec::new(),
    }
}

/// The address `at` links to, where it is written whole: as an address,
/// as `@url(...)` in the writing, or as `{{ url("...") }}` in a template,
/// each handed one address as written, in a site published under `base`.
/// None where it is built from anything else.
fn writes(at: &str, base: &str) -> Option<String> {
    let handed = if let Some(call) = at.strip_prefix("@url(") {
        call.strip_suffix(')')?.trim()
    } else if let Some(printed) = at.strip_prefix("{{") {
        let printed = printed.strip_suffix("}}")?;
        let printed = printed.strip_prefix('-').unwrap_or(printed);
        let printed = printed.strip_suffix('-').unwrap_or(printed).trim();
        let handed = printed.strip_prefix("url(")?.strip_suffix(')')?.trim();
        // A template's text is always quoted.
        if !handed.starts_with(['"', '\'']) {
            return None;
        }
        handed
    } else {
        return Some(at.to_owned());
    };
    let handed = match handed.chars().next() {
        Some(quote @ ('"' | '\'')) => {
            let inner = handed.strip_prefix(quote)?.strip_suffix(quote)?;
            if inner.contains(quote) || inner.contains('\\') {
                return None;
            }
            inner
        }
        _ => handed,
    };
    Some(format!("{base}{handed}"))
}

/// Whether the text from `start` to `end` of `whole` is a key of YAML
/// written at the start of its line, in a mapping that already has the
/// key `key`.
fn taken(whole: &str, start: usize, end: usize, key: &str) -> bool {
    let line = whole[..start].rfind('\n').map_or(0, |at| at + 1);
    let indent = start - line;
    if whole[line..start].contains(|c| c != ' ') || !whole[end..].starts_with(':') {
        return false;
    }
    let holds = |line: &str| {
        let written = line.trim_start_matches(' ');
        let rest = written
            .strip_prefix(key)
            .and_then(|rest| rest.strip_prefix(':'));
        line.len() - written.len() == indent
            && rest.is_some_and(|rest| rest.is_empty() || rest.starts_with([' ', '\t', '\r']))
    };
    // The lines of the mapping, out from the key's own: to a line written
    // less deep, or the end of the front matter.
    let out = |lines: &mut dyn Iterator<Item = &str>| {
        for line in lines {
            let written = line.trim_start_matches(' ');
            if written.trim_end() == "---" && line.len() == written.len() {
                return false;
            }
            if written.trim().is_empty() || written.starts_with('#') {
                continue;
            }
            if line.len() - written.len() < indent {
                return false;
            }
            if holds(line) {
                return true;
            }
        }
        false
    };
    let next = whole[end..]
        .find('\n')
        .map_or(whole.len(), |at| end + at + 1);
    out(&mut whole[..line].lines().rev()) || out(&mut whole[next.min(whole.len())..].lines())
}
