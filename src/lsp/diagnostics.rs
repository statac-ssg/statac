//! Statac's problems as the protocol's diagnostics, by the file each is
//! told in.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::uri;
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
const START: &str = "{\"start\":{\"line\":0,\"character\":0},\"end\":{\"line\":0,\"character\":0}}";

/// Every problem `outcome` holds that isn't quietened, as diagnostics by
/// the file each is told in. A problem in no file is told at the start of
/// `nowhere`.
pub(super) fn by_file(
    outcome: &Outcome,
    site: &Path,
    nowhere: &Path,
    encoding: Encoding,
) -> BTreeMap<PathBuf, Vec<String>> {
    let sources = outcome.sources();
    let mut found: BTreeMap<PathBuf, Vec<String>> = BTreeMap::new();
    for problem in &outcome.problems {
        if outcome.quiet.quietens(&problem.kind) {
            continue;
        }
        let (file, range) = match problem.place {
            Some(place) if problem.kind.of_a_whole_file() => {
                (path(sources, site, place.source), START.to_owned())
            }
            Some(place) => (
                path(sources, site, place.source),
                range(sources, place, encoding),
            ),
            None => (nowhere.to_path_buf(), START.to_owned()),
        };
        let told = diagnostic(problem, &range, sources, site, encoding);
        found.entry(file).or_default().push(told);
    }
    found
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

fn range(sources: &Sources, place: Place, encoding: Encoding) -> String {
    let (line, character) = position(sources, place.source, place.span.start as usize, encoding);
    let start = format!("{{\"line\":{line},\"character\":{character}}}");
    let end = (place.span.end as usize).max(place.span.start as usize);
    let (line, character) = position(sources, place.source, end, encoding);
    format!("{{\"start\":{start},\"end\":{{\"line\":{line},\"character\":{character}}}}}")
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
    let before = &text[start..offset];
    let character = match encoding {
        Encoding::Utf8 => before.len(),
        Encoding::Utf16 => before.encode_utf16().count(),
    };
    (line - 1, character)
}
