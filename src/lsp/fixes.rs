//! Fixes: where a problem's suggestion names one exact edit, that edit,
//! for the editor to make when asked. Each writes in place of text at the
//! problem's own place, and only where that text is what the problem
//! says was written there.

#[cfg(test)]
mod tests;

use crate::problem::{
    AddressFault, ArgumentFault, Chooser, Hint, Instead, Kind, Problem, Stopped, Word,
};
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
    let file = sources.text(place.source);
    let Some(at) = file.get(place.span.range()) else {
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
        if taken(file, start, start + written.len(), text) {
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
    // All that the problem is placed at, where that is `written`.
    let whole = |written: &str, text: &str| {
        let trimmed = at.trim();
        if trimmed != written || trimmed.is_empty() || trimmed.contains('\n') {
            return None;
        }
        let start = place.span.start as usize + (at.len() - at.trim_start().len());
        Some(Fix {
            title: wording::fix_written(text),
            place: Place {
                source: place.source,
                span: Span::new(start, start + trimmed.len()),
            },
            text: text.to_owned(),
        })
    };
    let fix = match &problem.kind {
        Kind::UnknownWord {
            written,
            nearest: Some(nearest),
            ..
        }
        | Kind::NamesNothing {
            written,
            nearest: Some(nearest),
            ..
        }
        | Kind::UnknownCodeLanguage {
            written,
            nearest: Some(nearest),
        }
        | Kind::MissingDefinition {
            written,
            nearest: Some(nearest),
        }
        | Kind::UnknownProblemName {
            written,
            suggestion: Some(nearest),
            ..
        }
        | Kind::UnusableUrl {
            fault:
                AddressFault::UnknownPiece {
                    written,
                    nearest: Some(nearest),
                },
        }
        | Kind::TextNeverShown {
            block: written,
            nearest: Some(nearest),
            ..
        }
        | Kind::MissingData {
            table: written,
            nearest: Some(nearest),
            ..
        }
        | Kind::WrongArgument {
            fault:
                ArgumentFault::NoSuchName {
                    given: written,
                    nearest: Some(nearest),
                },
            ..
        }
        | Kind::TemplateStopped {
            reason:
                Stopped::NoSuchArgument {
                    name: written,
                    nearest: Some(nearest),
                    ..
                },
        }
        | Kind::PageInSnippet {
            written,
            handed: nearest,
            ..
        }
        | Kind::ReadAsCustom {
            written, nearest, ..
        } => respelt(written, nearest),
        Kind::NoValue {
            written,
            instead:
                Instead::Handed(instead) | Instead::Default(instead) | Instead::Headline(instead),
            ..
        } => respelt(written, instead),
        Kind::AlwaysFalse { name, meant } => {
            let on = meant.rsplit_once('.').map_or("", |(on, _)| on);
            respelt(&format!("{on}.{name}"), meant)
        }
        Kind::MissingContentFolder {
            folder,
            nearest: Some(nearest),
            ..
        } => {
            let folder = folder.strip_prefix("content/").unwrap_or(folder);
            respelt(&format!("folder.{}", folder.trim_end_matches('/')), nearest)
        }
        Kind::MissingAnchor {
            id,
            suggestion: Some(suggestion),
            ..
        } => respelt(&format!("#{id}"), suggestion),
        Kind::FootnoteNeverShown {
            label,
            nearest: Some(nearest),
        } => respelt(&format!("[^{label}]"), &format!("[^{nearest}]")),
        // The spaces after the colon go with it, as `=` is written.
        Kind::ColonInCall { name } => {
            respelt(&format!("{name}:"), &format!("{name}=")).map(|mut fix| {
                let end = fix.place.span.end as usize;
                let after = &file[end..];
                let spaces = after.len() - after.trim_start_matches([' ', '\t']).len();
                fix.place.span = Span::new(fix.place.span.start as usize, end + spaces);
                fix
            })
        }
        Kind::CallInHtml { name } => respelt(&format!("@{name}"), &format!("\\@{name}")),
        Kind::MissingFile {
            path,
            hint: Some(Hint::Closest(name)),
        } => {
            // The path as far up as the name gives one.
            let parts = name.split('/').count();
            let from = (path.rmatch_indices('/').nth(parts - 1)).map_or(0, |(at, _)| at + 1);
            respelt(&path[from..], name)
        }
        // Front matter in TOML is rewritten, not respelt.
        Kind::OtherSpelling {
            word: Word::Marker,
            written,
            ..
        } if written == "+++" => None,
        Kind::OtherSpelling { written, ours, .. } => respelt(written, ours),
        Kind::NeedsQuotes { written, .. } => respelt(written, &wording::kept_quoted(written)),
        Kind::UnclearDate { written, readings } => {
            return (readings.iter())
                .filter_map(|(iso, _)| respelt(written, iso))
                .collect();
        }
        Kind::WrongKind {
            respelt: Some(respelt),
            ..
        } => whole(at.trim(), respelt),
        Kind::OneTagNotTwo { tag, parts, .. } => whole(tag, &format!("[{}]", parts.join(", "))),
        Kind::TemplateStopped {
            reason:
                Stopped::NoMethod {
                    name,
                    filter: Some(filter),
                    ..
                },
        } => {
            // The method as written, its receiver and what it was handed
            // being as the filter has them, where a filter handed nothing
            // goes without brackets.
            let (receiver, handed) = filter.rsplit_once(&format!(" | {name}")).unzip();
            let handed = handed.filter(|handed| !handed.is_empty()).unwrap_or("()");
            receiver.and_then(|receiver| whole(&format!("{receiver}.{name}{handed}"), filter))
        }
        Kind::MissingTemplate {
            path,
            chosen: Chooser::Written,
            nearest: Some(nearest),
        } => {
            let name = path.strip_prefix("templates/").unwrap_or(path);
            let name = name.strip_suffix(".html").unwrap_or(name);
            let nearest = nearest.strip_suffix(".html").unwrap_or(nearest);
            respelt(name, nearest)
        }
        Kind::BrokenLink {
            target,
            suggestion: Some(suggestion),
            ..
        } if writes(at.trim(), base).as_deref() == Some(target.trim()) => {
            whole(at.trim(), suggestion)
        }
        Kind::UrlGone { written, .. } if at.trim() == written.as_str() => {
            let (start, end) = lines(file, place.span);
            Some(Fix {
                title: wording::fix_forget(written),
                place: Place {
                    source: place.source,
                    span: Span::new(start, end),
                },
                text: String::new(),
            })
        }
        Kind::AddressesGone { under, gone, .. } => {
            let (start, end) = lines(file, place.span);
            let (taken, kept): (Vec<&str>, Vec<&str>) = (file[start..end].split_inclusive('\n'))
                .partition(|line| gone.iter().any(|address| line.trim() == address));
            (taken.len() == gone.len()).then(|| Fix {
                title: wording::fix_forget_all(gone.len(), under),
                place: Place {
                    source: place.source,
                    span: Span::new(start, end),
                },
                text: kept.concat(),
            })
        }
        _ => None,
    };
    fix.into_iter().collect()
}

/// The whole lines `span` of `file` is on, each with its ending.
fn lines(file: &str, span: Span) -> (usize, usize) {
    let (from, to) = (span.start as usize, span.end as usize);
    let start = file[..from].rfind('\n').map_or(0, |at| at + 1);
    let end = file[to..].find('\n').map_or(file.len(), |at| to + at + 1);
    (start, end)
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
