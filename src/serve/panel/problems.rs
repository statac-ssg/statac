//! Problems in their parts, for the page to lay out for itself: each
//! place with its lines, the pages whose builds met it, and what `statac
//! explain` says of its kind. And how each round went, told the same way.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;

use super::pieced;
use crate::address::on_the_wire;
use crate::json;
use crate::problem::Quiet;
use crate::problem::{Code, Kind, Named, Problem, Severity, Word, unanchored};
use crate::report::{Excerpt, NAMED, Report, shown};
use crate::serve::session::{Read, Took};
use crate::source::Place;
use crate::wording;

/// What turns a round's problems into their parts.
pub(super) struct Parts<'a> {
    pub(crate) report: &'a Report<'a>,
    pub(crate) quiet: &'a Quiet,
    /// The site as the round read it, where it was, to find where the
    /// pages a problem was met building are published.
    pub(crate) read: Option<&'a Read>,
    pub(crate) addresses: &'a OnceLock<Addresses>,
}

impl Parts<'_> {
    /// One problem, as JSON.
    pub(super) fn of(&self, problem: &Problem) -> String {
        let report = self.report;
        let kind = &problem.kind;
        let null = || "null".to_owned();
        let (file, line, column, excerpt) = match problem.place {
            None => (null(), null(), null(), "[]".to_owned()),
            Some(place) if kind.of_a_whole_file() => (
                json::string(&report.file(place.source)),
                null(),
                null(),
                "[]".to_owned(),
            ),
            Some(place) => {
                let excerpt = report.excerpt(place, true);
                (
                    json::string(&report.file(place.source)),
                    excerpt.first.to_string(),
                    excerpt.column.to_string(),
                    lines(&excerpt),
                )
            }
        };
        let related: Vec<String> = (problem.related().iter())
            .map(|related| {
                let (file, line, excerpt) = match related.place {
                    Some(place) => {
                        let excerpt = report.excerpt(place, false);
                        let file = json::string(&report.file(place.source));
                        (file, excerpt.first.to_string(), lines(&excerpt))
                    }
                    None => (null(), null(), "[]".to_owned()),
                };
                format!(
                    "{{\"label\":{},\"file\":{file},\"line\":{line},\"excerpt\":{excerpt}}}",
                    pieced(&shown(&related.label)),
                )
            })
            .collect();
        let pages: Vec<_> = problem.built().collect();
        let named = pages.len().min(1 + NAMED);
        let built: Vec<String> = (pages[..named].iter())
            .map(|&(page, place)| {
                let said = match (page, place) {
                    (Some(page), _) => shown(&wording::named(page)),
                    (None, Some(place)) => format!("`{}`", report.file(place.source)),
                    (None, None) => String::new(),
                };
                let href = (self.read).and_then(|read| {
                    self.addresses
                        .get_or_init(|| Addresses::of(read))
                        .href(page, place)
                });
                let (file, line) = match place {
                    Some(place) => (
                        json::string(&report.file(place.source)),
                        match report.whole(place) {
                            true => null(),
                            false => report.first_line(place).to_string(),
                        },
                    ),
                    None => (null(), null()),
                };
                format!(
                    "{{\"page\":{},\"href\":{},\"file\":{file},\"line\":{line}}}",
                    pieced(&said),
                    href.as_deref().map_or_else(null, json::string),
                )
            })
            .collect();
        let printed = match wording::printed(kind) {
            Some((heading, said)) => {
                let said: Vec<String> =
                    said.iter().map(|line| json::string(&shown(line))).collect();
                format!(
                    "{{\"heading\":{},\"lines\":[{}]}}",
                    pieced(&heading),
                    said.join(",")
                )
            }
            None => null(),
        };
        let suggestion = problem.suggestion();
        format!(
            "{{\"severity\":{},\"kind\":{},\"headline\":{},\"file\":{file},\"line\":{line},\
             \"column\":{column},\"excerpt\":{excerpt},\"related\":[{}],\"built\":[{}],\
             \"builtMore\":{},\"printed\":{printed},\"suggestion\":{},\"suggests\":{},\
             \"explanation\":{},\"quietened\":{}}}",
            json::string(wording::severity(kind.severity())),
            json::string(kind.name()),
            pieced(&shown(&problem.headline())),
            related.join(","),
            built.join(","),
            pages.len() - named,
            suggestion.map_or_else(null, |said| pieced(&shown(&said))),
            suggests(kind).map_or_else(null, |name| json::string(&shown(name))),
            json::string(&problem.explanation_link()),
            self.quiet.quietens(kind),
        )
    }
}

/// An excerpt's lines, each as its number and its pieces: the words, and
/// the place as `{"mark": ...}`, empty where the place is.
pub(super) fn lines(excerpt: &Excerpt) -> String {
    let lines: Vec<String> = (excerpt.lines.iter())
        .map(|line| {
            let text = |words: &str| format!("{{\"text\":{}}}", json::string(words));
            let mut pieces = Vec::new();
            if !line.before.is_empty() {
                pieces.push(text(&line.before));
            }
            if let Some(marked) = &line.marked {
                pieces.push(format!("{{\"mark\":{}}}", json::string(marked)));
            }
            if !line.after.is_empty() {
                pieces.push(text(&line.after));
            }
            let runs = match line.marked.is_some() && excerpt.last > excerpt.first {
                true => format!(",\"runs\":{}", pieced(&wording::runs_to(excerpt.last))),
                false => String::new(),
            };
            format!(
                "{{\"line\":{},\"text\":[{}]{runs}}}",
                line.number,
                pieces.join(",")
            )
        })
        .collect();
    format!("[{}]", lines.join(","))
}

/// The name a problem's suggestion offers in its place, where it is one
/// of the page's or the site's that a template writes, which the panel
/// lists.
fn suggests(kind: &Kind) -> Option<&str> {
    let name = match kind {
        Kind::UnknownWord {
            word: Word::Field,
            nearest: Some(name),
            ..
        }
        | Kind::AlwaysFalse { meant: name, .. } => name,
        _ => return None,
    };
    (name.starts_with("page.") || name.starts_with("site.")).then_some(name.as_str())
}

/// Where each page a round built is published, by how a problem met
/// building it names it: by what names it and where, as the round marks
/// such a problem. None where two pages are named alike.
pub(super) struct Addresses(HashMap<(Option<Named>, Option<Place>), Option<String>>);

impl Addresses {
    pub(super) fn of(read: &Read) -> Addresses {
        let renderer = &read.renderer;
        let (site, sources) = (renderer.site(), renderer.sources());
        let base = renderer.settings().base();
        let mut addresses = HashMap::new();
        let mut add = |named: Option<Named>, head: Option<Place>, address: &str| {
            let href = on_the_wire(&format!("{base}{address}"));
            addresses
                .entry((named, head))
                .and_modify(|held: &mut Option<String>| {
                    if held.as_ref() != Some(&href) {
                        *held = None;
                    }
                })
                .or_insert_with(|| Some(href.clone()));
        };
        for (id, page) in site.pages.iter() {
            // A page of the author's is named by its place alone, and a
            // list's page Statac made by its list.
            let named = match (page.source, page.speaks_for) {
                (Some(_), _) => None,
                (None, Some(_)) => Some(site.named(id, sources)),
                (None, None) => continue,
            };
            add(named, site.head(id, sources), page.address.as_str());
        }
        for &(id, number) in read.pages.values() {
            let page = site.pages.get(id);
            let later = (page.speaks_for)
                .filter(|_| number > 1)
                .and_then(|list| site.lists.get(list).later_address(number));
            if let Some(address) = later {
                let named = Named::later(number, format!("{base}{}", page.address.as_str()));
                add(Some(named), site.head(id, sources), address.as_str());
            }
        }
        Addresses(addresses)
    }

    fn href(&self, named: Option<&Named>, head: Option<Place>) -> Option<String> {
        if let Some(Named::Feed(feed)) = named {
            return Some(on_the_wire(&feed.address));
        }
        self.0.get(&(named.cloned(), head)).cloned().flatten()
    }
}

/// What `statac explain` says of a kind, as the panel shows it.
struct Explained {
    /// Its heading.
    title: String,
    /// Its heading, its opening and how to fix it, as JSON.
    json: String,
}

/// The heading of the section of an explanation that says how to fix it.
const FIX: &str = "How to fix it";

fn explained(code: Code) -> &'static Explained {
    static EXPLAINED: OnceLock<HashMap<Code, Explained>> = OnceLock::new();
    let every = EXPLAINED.get_or_init(|| {
        let every = Code::ALL.iter();
        every.map(|&code| (code, explain(code))).collect()
    });
    &every[&code]
}

/// The heading of the explanation of a kind.
pub(super) fn title(code: Code) -> &'static str {
    &explained(code).title
}

/// The explanation of each of `codes`, once each, by its kind's name.
pub(super) fn explanations(codes: impl IntoIterator<Item = Code>) -> String {
    let mut seen = Vec::new();
    let mut said = Vec::new();
    for code in codes {
        if !seen.contains(&code) {
            seen.push(code);
            said.push(format!(
                "{}:{}",
                json::string(code.name()),
                explained(code).json
            ));
        }
    }
    format!("{{{}}}", said.join(","))
}

/// An explanation's heading, the paragraphs before its first section, and
/// those of the section saying how to fix it. A paragraph is a sentence
/// of Statac's own, its lines run together; an item of a list is one of
/// its own; a block of code is as written. Anything else is as `statac
/// explain` prints it.
fn explain(code: Code) -> Explained {
    let mut lines = code.explanation().lines();
    let title = (lines.next())
        .and_then(|line| line.strip_prefix("# "))
        .map(unanchored)
        .unwrap_or_default();
    let mut parts: [Vec<String>; 2] = Default::default();
    // The part being read, where it is one the panel shows.
    let mut part = Some(0);
    let mut said: Vec<&str> = Vec::new();
    // The backticks that opened the block of code being read, and its lines.
    let mut code: Option<(usize, Vec<&str>)> = None;
    let flush = |said: &mut Vec<&str>, part: Option<usize>, parts: &mut [Vec<String>; 2]| {
        if let Some(part) = part.filter(|_| !said.is_empty()) {
            parts[part].push(format!("{{\"said\":{}}}", pieced(&said.join(" "))));
        }
        said.clear();
    };
    for line in lines {
        let ticks = line.len() - line.trim_start_matches('`').len();
        if let Some((fence, held)) = &mut code {
            if ticks >= *fence && line[ticks..].trim().is_empty() {
                if let Some(part) = part {
                    let block = format!("{{\"code\":{}}}", json::string(&held.join("\n")));
                    parts[part].push(block);
                }
                code = None;
            } else {
                held.push(line);
            }
            continue;
        }
        if let Some(heading) = line.strip_prefix("## ") {
            flush(&mut said, part, &mut parts);
            part = (unanchored(heading) == FIX).then_some(1);
            continue;
        }
        if ticks >= 3 {
            flush(&mut said, part, &mut parts);
            code = Some((ticks, Vec::new()));
            continue;
        }
        let numbered = line
            .split_once(". ")
            .is_some_and(|(n, _)| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
        if line.trim().is_empty() || line.starts_with("- ") || numbered {
            flush(&mut said, part, &mut parts);
        }
        if !line.trim().is_empty() {
            said.push(line.trim());
        }
    }
    flush(&mut said, part, &mut parts);
    let [opening, fix] = parts;
    Explained {
        json: format!(
            "{{\"title\":{},\"opening\":[{}],\"fix\":[{}]}}",
            json::string(title),
            opening.join(","),
            fix.join(",")
        ),
        title: title.to_owned(),
    }
}

/// The problems not quietened, counted by severity, as JSON.
pub(super) fn counts(problems: &[Problem], quiet: &Quiet) -> String {
    let count = |severity| {
        let of = |problem: &&Problem| problem.kind.severity() == severity;
        let shown = |problem: &&Problem| !quiet.quietens(&problem.kind);
        problems.iter().filter(of).filter(shown).count()
    };
    format!(
        "{{\"error\":{},\"warning\":{},\"note\":{}}}",
        count(Severity::Error),
        count(Severity::Warning),
        count(Severity::Note)
    )
}

/// How long each part of a round took, in milliseconds, as JSON.
pub(super) fn took(took: &Took) -> String {
    let ms = |part: Duration| (part.as_secs_f64() * 10_000.0).round() / 10.0;
    format!(
        "{{\"before\":{},\"reading\":{},\"comparing\":{},\"building\":{},\"concluding\":{},\
         \"writing\":{}}}",
        ms(took.before),
        ms(took.reading),
        ms(took.comparing),
        ms(took.building),
        ms(took.concluding),
        ms(took.writing),
    )
}

/// The problems counted by kind, as the members of a JSON list: errors,
/// then warnings, then notes, the kind with most first within each. A
/// kind whose problems all suggest the same, its quietened ones only
/// where it has no others, has that suggestion.
pub(super) fn kinds(problems: &[Problem], quiet: &Quiet) -> String {
    let mut counted: HashMap<Code, [Vec<&Problem>; 2]> = HashMap::new();
    for problem in problems {
        let quietened = usize::from(quiet.quietens(&problem.kind));
        counted.entry(problem.kind.code()).or_default()[quietened].push(problem);
    }
    let mut counted: Vec<_> = counted.into_iter().collect();
    counted.sort_by_key(|(code, [heard, hushed])| {
        let count = heard.len() + hushed.len();
        (code.severity(), std::cmp::Reverse(count), code.name())
    });
    let kinds: Vec<String> = (counted.into_iter())
        .map(|(code, [heard, hushed])| {
            let (count, quietened) = (heard.len() + hushed.len(), hushed.len());
            let told = if heard.is_empty() { hushed } else { heard };
            let mut said = told.iter().map(|problem| problem.suggestion());
            let first = said.next().flatten();
            let shared = first.filter(|first| said.all(|other| other.as_ref() == Some(first)));
            format!(
                "{{\"kind\":{},\"severity\":{},\"title\":{},\"count\":{count},\
                 \"quietened\":{quietened},\"suggestion\":{}}}",
                json::string(code.name()),
                json::string(wording::severity(code.severity())),
                json::string(title(code)),
                shared.map_or_else(|| "null".to_owned(), |said| pieced(&shown(&said))),
            )
        })
        .collect();
    kinds.join(",")
}
