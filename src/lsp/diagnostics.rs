//! Statac's problems as the protocol's diagnostics, by the file each is
//! told in.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::{fixes, uri};
use crate::build::Outcome;
use crate::json;
use crate::problem::{Problem, Severity};
use crate::report::shown;
use crate::source::{Place, SourceId, Sources};

/// How the editor counts a position's characters along its line.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(super) enum Encoding {
    /// In bytes of UTF-8.
    Utf8,
    /// In units of UTF-16, as JavaScript counts: the protocol's default.
    Utf16,
}

impl Encoding {
    pub(super) fn name(self) -> &'static str {
        match self {
            Encoding::Utf8 => "utf-8",
            Encoding::Utf16 => "utf-16",
        }
    }
}

/// The start of a file, where a problem with the whole file is told.
pub(super) const START: &str =
    "{\"start\":{\"line\":0,\"character\":0},\"end\":{\"line\":0,\"character\":0}}";

/// A line and a character in it, both counted from 0, as the protocol
/// has them.
pub(super) type Position = (usize, usize);

/// A problem as the editor is told it, with where it is and its fixes.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) struct Told {
    /// The protocol's diagnostic.
    pub diagnostic: String,
    pub from: Position,
    pub to: Position,
    pub fixes: Vec<Offered>,
}

/// A fix as it is offered: `text` in place of `was`, written from `from`
/// to `to` in the file the problem is told in.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) struct Offered {
    pub title: String,
    pub from: Position,
    pub to: Position,
    pub was: String,
    pub text: String,
}

/// Every problem `outcome` holds that isn't quietened, as diagnostics by
/// the file each is told in. A problem in no file is told at the start of
/// `nowhere`.
pub(super) fn by_file(
    outcome: &Outcome,
    site: &Path,
    nowhere: &Path,
    encoding: Encoding,
) -> BTreeMap<PathBuf, Vec<Told>> {
    let sources = outcome.sources();
    let base = outcome
        .settings()
        .map(|settings| settings.base())
        .unwrap_or_default();
    let ends = |place: Place| {
        let from = position(sources, place.source, place.span.start as usize, encoding);
        let end = (place.span.end as usize).max(place.span.start as usize);
        (from, position(sources, place.source, end, encoding))
    };
    let mut found: BTreeMap<PathBuf, Vec<Told>> = BTreeMap::new();
    for problem in &outcome.problems {
        if outcome.quiet.quietens(&problem.kind) {
            continue;
        }
        let (file, (from, to)) = match problem.place {
            Some(place) if problem.kind.of_a_whole_file() => {
                (path(sources, site, place.source), ((0, 0), (0, 0)))
            }
            Some(place) => (path(sources, site, place.source), ends(place)),
            None => (nowhere.to_path_buf(), ((0, 0), (0, 0))),
        };
        let fixes = (fixes::fixes(problem, sources, &base).into_iter())
            .map(|fix| {
                let (from, to) = ends(fix.place);
                let was = sources.text(fix.place.source)[fix.place.span.range()].to_owned();
                Offered {
                    title: shown(&fix.title),
                    from,
                    to,
                    was,
                    text: fix.text,
                }
            })
            .collect();
        let diagnostic = diagnostic(problem, &ranged(from, to), sources, site, encoding);
        found.entry(file).or_default().push(Told {
            diagnostic,
            from,
            to,
            fixes,
        });
    }
    found
}

/// The protocol's range from `from` to `to`.
pub(super) fn ranged(from: Position, to: Position) -> String {
    format!(
        "{{\"start\":{{\"line\":{},\"character\":{}}},\
         \"end\":{{\"line\":{},\"character\":{}}}}}",
        from.0, from.1, to.0, to.1
    )
}

fn diagnostic(
    problem: &Problem,
    at: &str,
    sources: &Sources,
    site: &Path,
    encoding: Encoding,
) -> String {
    let kind = &problem.kind;
    let severity = match kind.severity() {
        Severity::Error => 1,
        Severity::Warning => 2,
        Severity::Note => 3,
    };
    let mut message = shown(&problem.headline());
    if let Some(suggestion) = problem.suggestion() {
        message.push('\n');
        message.push_str(&shown(&suggestion));
    }
    let located = |place: Place, said: &str| {
        let at = uri::of(&path(sources, site, place.source));
        format!(
            "{{\"location\":{{\"uri\":{},\"range\":{}}},\"message\":{}}}",
            json::string(&at),
            range(sources, place, encoding),
            json::string(said)
        )
    };
    let mut related: Vec<String> = (problem.related().iter())
        .filter_map(|related| Some(located(related.place?, &shown(&related.label))))
        .collect();
    related.extend(problem.built().filter_map(|(page, place)| {
        let page = shown(&crate::wording::named(page?));
        Some(located(place?, &page))
    }));
    format!(
        "{{\"range\":{at},\"severity\":{severity},\"code\":{},\
         \"codeDescription\":{{\"href\":{}}},\"source\":\"statac\",\"message\":{},\
         \"relatedInformation\":[{}]}}",
        json::string(kind.name()),
        json::string(&problem.explanation_link()),
        json::string(&message),
        related.join(","),
    )
}

/// The file a source was read from, from the site's folder where its path
/// is relative.
fn path(sources: &Sources, site: &Path, id: SourceId) -> PathBuf {
    site.join(sources.path(id))
}

/// Where `place` is, as the protocol's range.
pub(super) fn range(sources: &Sources, place: Place, encoding: Encoding) -> String {
    let (line, character) = position(sources, place.source, place.span.start as usize, encoding);
    let start = format!("{{\"line\":{line},\"character\":{character}}}");
    let end = (place.span.end as usize).max(place.span.start as usize);
    let (line, character) = position(sources, place.source, end, encoding);
    format!("{{\"start\":{start},\"end\":{{\"line\":{line},\"character\":{character}}}}}")
}

/// The line and character of `offset` in `text`, both counted from 0, the
/// character as `encoding` counts it.
pub(super) fn position_in(text: &str, offset: usize, encoding: Encoding) -> (usize, usize) {
    let offset = text.floor_char_boundary(offset.min(text.len()));
    let mut start = text[..offset].rfind('\n').map_or(0, |at| at + 1);
    let line = text[..start].matches('\n').count();
    // A byte-order mark is no character the editor shows.
    if start == 0 && text.starts_with('\u{feff}') && offset >= '\u{feff}'.len_utf8() {
        start = '\u{feff}'.len_utf8();
    }
    (line, count(&text[start..offset], encoding))
}

/// Where the protocol's `line` and `character` stand in `text`, in bytes:
/// at the end of the line where the character is past it, and none where
/// the text has no such line.
pub(super) fn offset_in(
    text: &str,
    line: usize,
    character: usize,
    encoding: Encoding,
) -> Option<usize> {
    let mut start = 0;
    for _ in 0..line {
        start += text[start..].find('\n')? + 1;
    }
    if start == 0 && text.starts_with('\u{feff}') {
        start = '\u{feff}'.len_utf8();
    }
    let end = text[start..].find('\n').map_or(text.len(), |at| start + at);
    let mut counted = 0;
    for (at, c) in text[start..end].char_indices() {
        if counted >= character {
            return Some(start + at);
        }
        counted += match encoding {
            Encoding::Utf8 => c.len_utf8(),
            Encoding::Utf16 => c.len_utf16(),
        };
    }
    Some(end)
}

fn count(text: &str, encoding: Encoding) -> usize {
    match encoding {
        Encoding::Utf8 => text.len(),
        Encoding::Utf16 => text.encode_utf16().count(),
    }
}

/// The line and character of `offset` in a source, both counted from 0,
/// the character as `encoding` counts it.
pub(super) fn position(
    sources: &Sources,
    id: SourceId,
    offset: usize,
    encoding: Encoding,
) -> (usize, usize) {
    let text = sources.text(id);
    let offset = text.floor_char_boundary(offset.min(text.len()));
    let line = sources.line_number(id, offset);
    let mut start = sources.line_start(id, line);
    // A byte-order mark is no character the editor shows.
    let mark = '\u{feff}'.len_utf8();
    if line == 1 && text.starts_with('\u{feff}') && offset >= mark {
        start = mark;
    }
    (line - 1, count(&text[start..offset], encoding))
}
