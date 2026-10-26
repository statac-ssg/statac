//! `statac check --json`: every problem as one JSON document, for an
//! editor or another program to read.
//!
//! A place is its file, from the site's folder with forward slashes, and
//! where in the file it starts and ends: each a line and a column counted
//! from 1, the column in characters, and the offset in bytes from the
//! file's start. The end is just past the place's last character. A place
//! that is a whole file has its file and no start or end.

use std::path::Path;

use super::{Report, shown};
use crate::json;
use crate::problem::Problem;
use crate::problem::Quiet;
use crate::source::Place;
use crate::wording;

/// The version of the document's shape. A change that could break a
/// program reading it takes the next number.
const FORMAT: u32 = 1;

impl Report<'_> {
    /// Every problem, quietened or not, as JSON, in the order `check`
    /// draws them.
    pub(crate) fn check_json(&self, problems: &[Problem], quiet: &Quiet) -> String {
        let site = std::path::absolute(self.root).ok();
        let site = site.map(|site| json::string(&site.display().to_string()));
        let problems: Vec<String> = (self.ordered(problems).into_iter())
            .map(|problem| self.problem_json(problem, quiet))
            .collect();
        format!(
            "{{\"statac\":{},\"format\":{FORMAT},\"site\":{},\"problems\":[{}]}}\n",
            json::string(env!("CARGO_PKG_VERSION")),
            site.as_deref().unwrap_or("null"),
            problems.join(","),
        )
    }

    fn problem_json(&self, problem: &Problem, quiet: &Quiet) -> String {
        let kind = &problem.kind;
        let place = match problem.place {
            Some(place) if kind.of_a_whole_file() => self.file_json(place),
            Some(place) => self.place_json(place),
            None => self.nowhere_json(),
        };
        let related: Vec<String> = (problem.related().iter())
            .map(|related| {
                let place = related
                    .place
                    .map_or_else(|| self.nowhere_json(), |place| self.place_json(place));
                format!(
                    "{{\"message\":{},{place}}}",
                    json::string(&shown(&related.label))
                )
            })
            .collect();
        let pages: Vec<String> = problem
            .built()
            .map(|(page, place)| {
                let place = match place {
                    Some(place) if self.whole(place) => self.file_json(place),
                    Some(place) => self.place_json(place),
                    None => self.nowhere_json(),
                };
                let page = page.map(|page| json::string(&shown(&wording::named(page))));
                format!("{{\"page\":{},{place}}}", page.as_deref().unwrap_or("null"))
            })
            .collect();
        let suggestion = (problem.suggestion()).map(|said| json::string(&shown(&said)));
        format!(
            "{{\"severity\":{},\"kind\":{},\"message\":{},{place},\"suggestion\":{},\
             \"explanation\":{},\"quietened\":{},\"related\":[{}],\"pages\":[{}]}}",
            json::string(wording::severity(kind.severity())),
            json::string(kind.name()),
            json::string(&shown(&problem.headline())),
            suggestion.as_deref().unwrap_or("null"),
            json::string(&problem.explanation_link()),
            quiet.quietens(kind),
            related.join(","),
            pages.join(","),
        )
    }

    /// `place`'s file, start and end, as the fields of an object.
    fn place_json(&self, place: Place) -> String {
        let id = place.source;
        let text = self.sources.text(id);
        let start = text.floor_char_boundary(place.span.start as usize);
        let end = text.ceil_char_boundary((place.span.end as usize).max(start));
        let at = |offset: usize| {
            let at = self.sources.line_column(id, offset);
            format!(
                "{{\"line\":{},\"column\":{},\"offset\":{offset}}}",
                at.line, at.column
            )
        };
        format!(
            "\"file\":{},\"start\":{},\"end\":{}",
            json::string(&self.relative(self.sources.path(id))),
            at(start),
            at(end)
        )
    }

    /// `place`'s file alone, for a place that is the whole file.
    fn file_json(&self, place: Place) -> String {
        let file = self.relative(self.sources.path(place.source));
        format!(
            "\"file\":{},\"start\":null,\"end\":null",
            json::string(&file)
        )
    }

    fn nowhere_json(&self) -> String {
        "\"file\":null,\"start\":null,\"end\":null".to_owned()
    }

    /// `path` from the site's folder, with forward slashes, where it is in
    /// the folder; as it is otherwise.
    fn relative(&self, path: &Path) -> String {
        match path.strip_prefix(self.root) {
            Ok(rel) => crate::platform::slashed(rel).unwrap_or_else(|lossy| lossy),
            Err(_) => path.display().to_string(),
        }
    }
}
