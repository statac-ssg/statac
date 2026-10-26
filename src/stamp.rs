//! Writing save-time stamps: `statac stamp`, and serve as a file is
//! saved, write the date each stamp names in its place, once.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::build;
use crate::content::stamps::{Moment, Stamp, of_a_refused_stamp, read};
use crate::date::Date;
use crate::feeds::{self, IdsToWrite};
use crate::platform;
use crate::problem::{Kind, Problem, Severity};
use crate::rewrite::{self, Edit};
use crate::source::{SourceId, Sources, Span};
use crate::yaml::Value;
use crate::yaml::key_name;

/// One stamp written.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Stamped {
    /// The file, from the site's folder, with forward slashes.
    pub file: String,
    pub field: String,
    /// The date written in the stamp's place.
    pub written: String,
}

/// What became of the stamps a site read was found to hold, and of the
/// ids its feeds' posts were due.
#[derive(Default)]
pub struct Pass {
    pub stamped: Vec<Stamped>,
    /// The files given an `id:` line, each as [`Stamped`] names it.
    pub ids: Vec<String>,
    /// Each file written, as [`Stamped`] names it, and what it now holds.
    pub(crate) files: Vec<(String, blake3::Hash)>,
    /// The files that weren't written, each as a problem in no file
    /// saying why.
    pub refused: Vec<Problem>,
    /// Whether stamps were left as they were because the site's settings
    /// have an error: which day it is there can't be known.
    pub unsettled: bool,
    /// Whether posts were left without an `id:` because no build has yet
    /// kept what feed readers know.
    pub unrecorded: bool,
}

impl Pass {
    /// Whether any file was written.
    pub fn wrote(&self) -> bool {
        !self.files.is_empty()
    }
}

/// What is written into one file.
#[derive(Default)]
struct Due<'a> {
    stamps: Vec<(Span, &'a str, Stamp)>,
    /// The `id:` line, as the edit that writes it.
    id: Option<Edit>,
}

/// Which of the two things written into a site's files.
#[derive(Copy, Clone, PartialEq, Eq)]
enum Part {
    Stamps,
    Ids,
}

/// Writes into the files of the site at `site`, which `prepared` read as
/// `run` asks, a date over every stamp its problems tell of, and then an
/// `id:` line for each post `write_feed_ids` asks one for: a post is
/// known for one only once its dates are written. `fixed` is whether the
/// day was given, so that the run can be repeated. Gives the site as it
/// reads once written, the texts of its pages taken from `texts` as
/// [`crate::content::read_with`] does.
pub(crate) fn write(
    site: &Path,
    run: &crate::content::Run,
    mut prepared: build::Prepared,
    fixed: bool,
    texts: &mut crate::content::Texts,
) -> (Pass, build::Prepared) {
    let mut pass = written(site, &prepared, fixed, Part::Stamps);
    if pass.wrote() {
        prepared = build::prepared_with(site, run, texts);
    }
    let ids = written(site, &prepared, fixed, Part::Ids);
    if ids.wrote() {
        prepared = build::prepared_with(site, run, texts);
    }
    pass.ids = ids.ids;
    pass.files.extend(ids.files);
    pass.refused.extend(ids.refused);
    pass.unrecorded = ids.unrecorded;
    (pass, prepared)
}

fn written(site: &Path, prepared: &build::Prepared, fixed: bool, part: Part) -> Pass {
    let renderer = &prepared.renderer;
    let (sources, settings) = (renderer.sources(), renderer.settings().source);
    let problems = &prepared.problems;
    let mut due: BTreeMap<SourceId, Due> = BTreeMap::new();
    for problem in problems.iter().filter(|_| part == Part::Stamps) {
        if let (Kind::NotStamped { field, written }, Some(place)) = (&problem.kind, problem.place)
            && let Some(Ok(stamp)) = read(written)
        {
            let stamps = &mut due.entry(place.source).or_default().stamps;
            stamps.push((place.span, field, stamp));
        }
    }
    let mut pass = Pass::default();
    if settings_stop(sources, settings, problems).next().is_some() {
        pass.unsettled = !due.is_empty();
        return pass;
    }
    let pages = &renderer.site().pages;
    let ids = match part {
        Part::Stamps => IdsToWrite::Lines(Vec::new()),
        Part::Ids => feeds::ids_to_write(
            &prepared.known,
            renderer.site(),
            renderer.settings(),
            renderer.dates(),
        ),
    };
    match ids {
        IdsToWrite::NoRecord => pass.unrecorded = true,
        IdsToWrite::Lines(lines) => {
            for (post, line) in lines {
                let Some(source) = pages.get(post).source else {
                    continue;
                };
                match id_line(sources, source, &line) {
                    Some(edit) => due.entry(source).or_default().id = Some(edit),
                    None => pass.refused.push(Problem::nowhere(Kind::CantWrite {
                        path: sources.path(source).to_path_buf(),
                        reason: crate::problem::Reason::NoPlaceForId,
                    })),
                }
            }
        }
    }
    if due.is_empty() {
        return pass;
    }
    let moment = Moment::of(renderer.dates(), fixed);
    let content = site.join("content");
    for (source, mut due) in due {
        due.stamps.sort_by_key(|(span, ..)| span.start);
        let path = sources.path(source);
        let Ok(file) = platform::slashed(path) else {
            continue;
        };
        let Some(rel) = file.strip_prefix("content/") else {
            continue;
        };
        let mut edits = Vec::new();
        let mut stamped = Vec::new();
        for (span, field, stamp) in due.stamps {
            let Some(written) = stamp.written(&moment) else {
                continue;
            };
            edits.push(Edit {
                span,
                with: written.clone(),
            });
            stamped.push(Stamped {
                file: file.clone(),
                field: field.to_owned(),
                written,
            });
        }
        let id = due.id.is_some();
        edits.extend(due.id);
        if edits.is_empty() {
            continue;
        }
        match rewrite::rewrite(&content, rel, sources.text(source), &edits) {
            Ok(text) => {
                pass.files.push((file.clone(), blake3::hash(&text)));
                pass.stamped.extend(stamped);
                if id {
                    pass.ids.push(file);
                }
            }
            Err(reason) => pass.refused.push(Problem::nowhere(Kind::CantWrite {
                path: path.to_path_buf(),
                reason,
            })),
        }
    }
    pass
}

/// The edit that writes `line` as the last line of a file's front matter,
/// with the line ending and the indent the lines there have, giving the
/// file front matter where it has none. None where the line wouldn't be
/// read back as a field of the page: front matter never closed, or
/// written some other way than a line to a field.
fn id_line(sources: &Sources, source: SourceId, line: &str) -> Option<Edit> {
    let text = sources.text(source);
    let split = crate::yaml::split(text).ok()?;
    let ending = match text.find('\n') {
        Some(at) if text[..at].ends_with('\r') => "\r\n",
        _ => "\n",
    };
    let (at, with) = match split.front_matter {
        Some(front) => {
            let indent = match crate::yaml::front_matter(sources, source) {
                Ok((Some(node), _)) => match &node.value {
                    Value::Mapping(entries) => entries.first().map(|first| {
                        let key = first.key.place.span.range().start;
                        let from = text[..key].rfind('\n').map_or(0, |end| end + 1);
                        &text[from..key]
                    }),
                    _ => None,
                },
                _ => None,
            };
            let indent = indent.unwrap_or_default();
            (front.range().end, format!("{indent}{line}{ending}"))
        }
        None => (
            split.body.range().start,
            format!("---{ending}{line}{ending}---{ending}"),
        ),
    };
    // Read back as a build will read it.
    let mut after = Sources::default();
    let written = format!("{}{with}{}", &text[..at], &text[at..]);
    let read = after.add(sources.path(source), written);
    let (field, id) = line.split_once(": ")?;
    let lands = match crate::yaml::front_matter(&after, read) {
        Ok((Some(node), _)) => match &node.value {
            Value::Mapping(entries) => entries.iter().any(|entry| {
                key_name(&entry.key) == field
                    && matches!(&entry.value.value, Value::Scalar(value) if value == id)
            }),
            _ => false,
        },
        _ => false,
    };
    lands.then(|| Edit {
        span: Span::new(at, at),
        with,
    })
}

/// The errors in a site's settings file, or in reading it.
fn settings_stop<'a>(
    sources: &'a Sources,
    settings: Option<SourceId>,
    problems: &'a [Problem],
) -> impl Iterator<Item = &'a Problem> {
    let file = Path::new(crate::config::FILE_NAME);
    problems
        .iter()
        .filter(|problem| problem.kind.severity() == Severity::Error)
        .filter(move |problem| match (problem.place, &problem.kind) {
            (Some(place), _) => {
                Some(place.source) == settings || sources.path(place.source) == file
            }
            (None, Kind::CantRead { path, .. }) => path == file,
            _ => false,
        })
}

/// What `statac stamp` is asked.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Options {
    /// The site's folder.
    pub site: PathBuf,
    /// The day stamps are counted from; today in the site's zone where
    /// none is given.
    pub today: Option<Date>,
}

/// What `statac stamp` did, and what it has to tell.
pub struct Outcome {
    held: Option<build::Prepared>,
    pub pass: Pass,
    /// What stopped it: a folder that can't be read, the errors in the
    /// site's settings, each file that couldn't be written, and each
    /// value meant as a stamp and not written as one.
    pub problems: Vec<Problem>,
}

impl Outcome {
    /// The files the problems are placed in.
    pub(crate) fn sources(&self) -> Option<&Sources> {
        let held = self.held.as_ref()?;
        Some(held.renderer.sources())
    }
}

/// Reads the site as a build does and writes a date over every stamp in
/// it.
pub fn run(options: &Options) -> Outcome {
    if let Err(problem) = build::a_folder(&options.site) {
        return Outcome {
            held: None,
            pass: Pass::default(),
            problems: vec![problem],
        };
    }
    let run = crate::content::Run {
        today: options.today,
        drafts: false,
        url: None,
        renamed: Vec::new(),
    };
    let prepared = build::prepared(&options.site, &run);
    let fixed = options.today.is_some();
    let unkept = &mut crate::content::Texts::default();
    let (mut pass, mut prepared) = write(&options.site, &run, prepared, fixed, unkept);
    // Of the site as it reads now, which the places are in.
    let read = std::mem::take(&mut prepared.problems);
    let renderer = &prepared.renderer;
    let (sources, settings) = (renderer.sources(), renderer.settings().source);
    let mut problems = std::mem::take(&mut pass.refused);
    if pass.unsettled {
        let stop: Vec<Problem> = settings_stop(sources, settings, &read).cloned().collect();
        problems.extend(stop);
    }
    let refused = read.into_iter();
    problems.extend(refused.filter(|problem| of_a_refused_stamp(&problem.kind)));
    Outcome {
        held: Some(prepared),
        pass,
        problems,
    }
}
