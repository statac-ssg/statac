//! What the panel on a served page shows: how the last round went, what
//! there is to say of the page it is on, and how the site stands.
//!
//! Everything is worked out here and handed to the page as text for it to
//! place. Whatever comes from the site is text a page sets as text, never
//! as markup, and is shown as a terminal is shown it. A sentence of
//! Statac's own goes as its pieces, each name apart from the words round
//! it, where a terminal is shown it in backticks.

mod page;
mod problems;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use super::live::json_string;
use super::session::{Read, Round, Took};
use crate::content::address::on_the_wire;
use crate::origin::Arrival;
use crate::problem::{Code, Problem, Severity};
use crate::report::{Piece, Quiet, Report, pieces, shown};
use crate::templates::{Explained, Held, Row};
use crate::wording;
use problems::{Addresses, Parts};

/// The most problems the panel is handed at once.
const MOST: usize = 200;
/// How many rounds the panel keeps.
const ROUNDS: usize = 10;

/// What the panel shows after one round.
#[derive(Default)]
pub struct Panel {
    /// The errors of a round that stopped.
    stopped: Option<Stopped>,
    /// The site as the last round to build it left it.
    built: Arc<Built>,
    /// What the round wrote into the site's own files, a line for each.
    written: Vec<String>,
    /// The round, as JSON; none before the first.
    round: Option<String>,
    /// The last few rounds, the newest first, each as JSON.
    rounds: Vec<String>,
    /// The problems gone since the round before, said.
    gone: Option<String>,
    /// The site's folder, whole; none before the first round.
    root: Option<String>,
}

/// What serve knows of a round besides the round itself.
pub struct Besides<'a> {
    pub took: Took,
    /// The files it was for, as they are shown; none for the first.
    pub changed: &'a [String],
    /// What the terminal said of how it ended.
    pub said: &'a str,
    pub gone: Option<&'a str>,
}

struct Stopped {
    /// When it stopped, and how many errors stopped it.
    line: String,
    /// The first error, for the line a closed panel shows.
    first: Option<First>,
    /// How many errors there are after the first.
    more: usize,
    /// Each error, drawn as `check` draws it.
    drawn: String,
    /// When it stopped, and each error in its parts, with its kind.
    at: String,
    errors: Vec<(Code, String)>,
}

struct First {
    headline: String,
    /// Its file and line; empty for an error in no file.
    location: String,
    /// Its suggestion; empty where it has none.
    suggestion: String,
}

#[derive(Default)]
struct Built {
    /// When, by this machine's clock; empty before any round has built.
    line: String,
    /// The time alone; empty likewise.
    at: String,
    /// What there is to say of a page, by the file it is published as.
    pages: HashMap<String, Page>,
    /// The site's problems counted by kind, a line for each severity.
    counts: Vec<String>,
    drafts: Vec<Link>,
    waiting: Vec<Link>,
    /// The site as that round read it, and its folder.
    read: Option<Read>,
    site: PathBuf,
    /// Every problem of that round, and the kinds quietened.
    problems: Vec<Problem>,
    quiet: Quiet,
    /// The problems counted by kind, as JSON.
    kinds: String,
    /// Where the pages that round built are published, once asked.
    addresses: OnceLock<Addresses>,
}

struct Page {
    draft: bool,
    waiting: Option<String>,
    /// Its problems, drawn as `check` draws them; empty where it has none.
    drawn: String,
    /// Its problems, quietened ones among them.
    problems: Vec<Problem>,
}

struct Link {
    text: String,
    href: String,
    /// What it names, and the day it waits for, apart.
    named: String,
    day: Option<String>,
}

/// The time of day on this machine's clock.
fn now() -> String {
    jiff::Zoned::now().strftime("%H:%M:%S").to_string()
}

impl Panel {
    /// The panel after `round` of the site at `site`, published under
    /// `base`. `before` is the panel of the round before, whose pages are
    /// still the ones answered where this round stopped.
    pub fn after(
        mut round: Round,
        site: &Path,
        base: &str,
        besides: Besides,
        before: &Panel,
    ) -> Panel {
        let at = now();
        let stopped = round.outcome.stopped();
        let outcome = &round.outcome;
        let report = Report::new(outcome.sources(), site, false);
        let unquietened = |problem: &&Problem| !outcome.quiet.quietens(&problem.kind);
        let written: Vec<String> = report
            .stamps(&round.stamps)
            .lines()
            .map(str::to_owned)
            .collect();
        let counts = problems::counts(&outcome.problems, &outcome.quiet);
        let state = json_string(match stopped {
            true => "stopped",
            false => "built",
        });
        let built_at = match stopped {
            true => Some(&before.built.at).filter(|at| !at.is_empty()),
            false => Some(&at),
        };
        let changed: Vec<String> = (besides.changed.iter())
            .map(|file| json_string(file))
            .collect();
        let this = format!(
            "{{\"state\":{state},\"at\":{},\"builtAt\":{},\"changed\":[{}],\"built\":{},\
             \"outputs\":{},\"took\":{},\"counts\":{counts}}}",
            json_string(&at),
            built_at.map_or_else(|| "null".to_owned(), |at| json_string(at)),
            changed.join(","),
            round.built,
            round.outputs,
            problems::took(&besides.took),
        );
        let mut rounds = vec![format!(
            "{{\"state\":{state},\"at\":{},\"said\":{},\"counts\":{counts}}}",
            json_string(&at),
            pieced(besides.said),
        )];
        rounds.extend(before.rounds.iter().take(ROUNDS - 1).cloned());
        let mut panel = Panel {
            stopped: None,
            built: Arc::clone(&before.built),
            written,
            round: Some(this),
            rounds,
            gone: besides.gone.map(str::to_owned),
            root: Some(shown(&whole(site))),
        };
        if stopped {
            let errors: Vec<&Problem> = (outcome.problems.iter())
                .filter(|problem| problem.kind.severity() == Severity::Error)
                .collect();
            let kept = !before.built.line.is_empty();
            let ordered = report.ordered(errors.iter().copied());
            let addresses = OnceLock::new();
            let parts = Parts {
                report: &report,
                quiet: &outcome.quiet,
                read: round.read.as_ref(),
                addresses: &addresses,
            };
            let first = ordered.first().copied();
            panel.stopped = Some(Stopped {
                line: wording::panel_stopped(&at, errors.len(), kept),
                first: first.map(|first| First {
                    headline: shown(&first.headline()),
                    location: report.location(first).unwrap_or_default(),
                    suggestion: first.suggestion().map(|it| shown(&it)).unwrap_or_default(),
                }),
                more: errors.len().saturating_sub(1),
                drawn: report.problems(errors.iter().copied()),
                errors: (ordered.iter())
                    .map(|error| (error.kind.code(), parts.of(error)))
                    .collect(),
                at,
            });
            return panel;
        }
        let count = |severity| {
            let of = |problem: &&Problem| problem.kind.severity() == severity;
            outcome
                .problems
                .iter()
                .filter(unquietened)
                .filter(of)
                .count()
        };
        let line = wording::panel_built(
            &at,
            count(Severity::Error),
            count(Severity::Warning),
            count(Severity::Note),
        );
        let link = |noted: &super::session::Noted, day: Option<&str>| {
            let named = noted.title.as_deref().unwrap_or(&noted.address);
            let text = match day {
                Some(day) => format!("{named} ({day})"),
                None => named.to_owned(),
            };
            Link {
                text: shown(&text),
                href: on_the_wire(&format!("{base}{}", noted.address)),
                named: shown(named),
                day: day.map(str::to_owned),
            }
        };
        let mut built = Built {
            line,
            at,
            counts: report.counted(&outcome.problems, &outcome.quiet),
            site: site.to_owned(),
            kinds: problems::kinds(&outcome.problems, &outcome.quiet),
            ..Built::default()
        };
        for noted in std::mem::take(&mut round.noted) {
            if noted.draft {
                built.drafts.push(link(&noted, None));
            } else if let Some(day) = &noted.waiting {
                built.waiting.push(link(&noted, Some(day)));
            }
            let page = Page {
                draft: noted.draft,
                waiting: noted.waiting,
                drawn: report.problems(noted.problems.iter().filter(unquietened)),
                problems: noted.problems,
            };
            built.pages.insert(noted.file, page);
        }
        built.read = round.read.take();
        built.problems = std::mem::take(&mut round.outcome.problems);
        built.quiet = std::mem::take(&mut round.outcome.quiet);
        panel.built = Arc::new(built);
        panel
    }

    /// The last round to build the site's problems, of the kind named
    /// `kind` or of every kind, quietened ones among them, in their parts,
    /// as JSON.
    pub fn problems(&self, kind: Option<&str>) -> String {
        let built = &self.built;
        let Some(read) = &built.read else {
            return "{\"v\":2,\"problems\":[],\"more\":0,\"explanations\":{}}".to_owned();
        };
        let report = Report::new(read.renderer.sources(), &built.site, false);
        let of_kind = |problem: &&Problem| kind.is_none_or(|kind| problem.kind.name() == kind);
        let ordered = report.ordered(built.problems.iter().filter(of_kind));
        let more = ordered.len().saturating_sub(MOST);
        let told = &ordered[..ordered.len() - more];
        let parts = built.parts(&report, read);
        let problems: Vec<String> = told.iter().map(|problem| parts.of(problem)).collect();
        format!(
            "{{\"v\":2,\"problems\":[{}],\"more\":{more},\"explanations\":{}}}",
            problems.join(","),
            problems::explanations(told.iter().map(|problem| problem.kind.code())),
        )
    }

    /// The panel for the page published as `file`, or for an address with
    /// no page at it, as JSON.
    pub fn to_json(&self, file: Option<&str>) -> String {
        let built = &self.built;
        let page = file.and_then(|file| built.pages.get(file));
        let mut line = match &self.stopped {
            Some(_) => wording::PANEL_LINE_STOPPED.to_owned(),
            None => built.line.clone(),
        };
        match page {
            Some(page) if page.draft => line.push_str(&format!(" {}", wording::PANEL_DRAFT)),
            Some(Page {
                waiting: Some(day), ..
            }) => line.push_str(&format!(" {}", wording::panel_waiting(day))),
            _ => {}
        }
        let mut sections = Vec::new();
        let mut error = "null".to_owned();
        if let Some(stopped) = &self.stopped {
            let said = [stopped.line.clone()];
            sections.push(section(wording::PANEL_STOPPED, &said, &stopped.drawn, &[]));
            if let Some(first) = &stopped.first {
                let more = match stopped.more {
                    0 => String::new(),
                    more => wording::panel_more_errors(more),
                };
                error = format!(
                    "{{\"headline\":{},\"location\":{},\"suggestion\":{},\"more\":{}}}",
                    pieced(&first.headline),
                    json_string(&first.location),
                    pieced(&first.suggestion),
                    pieced(&more),
                );
            }
        }
        if file.is_some() && !built.line.is_empty() {
            let mut lines = Vec::new();
            let mut drawn = "";
            if let Some(page) = page {
                if page.draft {
                    lines.push(wording::PANEL_IS_DRAFT.to_owned());
                }
                if let Some(day) = &page.waiting {
                    lines.push(wording::panel_is_waiting(day));
                }
                drawn = &page.drawn;
            }
            if drawn.is_empty() {
                lines.push(wording::PANEL_PAGE_CLEAR.to_owned());
            }
            sections.push(section(wording::PANEL_PAGE, &lines, drawn, &[]));
        }
        let explained = built.read.as_ref().and_then(|read| {
            let &(page, number) = read.pages.get(file?)?;
            Some((read, read.renderer.explained(page, number)))
        });
        let mut facts = "null".to_owned();
        let mut names = "[]".to_owned();
        if let Some((read, explained)) = explained {
            let report = Report::new(read.renderer.sources(), &built.site, false);
            sections.push(comes_from(&report, &explained));
            sections.push(shows(&report, &explained));
            let output = (read.built.iter()).find(|output| Some(output.file.as_str()) == file);
            let on = page::On {
                report: &report,
                renderer: &read.renderer,
                explained: &explained,
                reads: output.map(|output| &output.reads),
                asked: output.map(|output| &output.asked),
                site: &built.site,
            };
            let waiting = page.and_then(|page| page.waiting.as_deref());
            facts = on.facts(page.is_some_and(|page| page.draft), waiting);
            names = on.names();
        }
        if !built.line.is_empty() {
            let lines = match built.counts.is_empty() {
                true => vec![wording::NO_PROBLEMS.to_owned()],
                false => built.counts.clone(),
            };
            let lists = [
                (wording::PANEL_DRAFTS, &built.drafts),
                (wording::PANEL_WAITING, &built.waiting),
            ];
            sections.push(section(wording::PANEL_SITE, &lines, "", &lists));
        }
        if !self.written.is_empty() {
            sections.push(section(wording::PANEL_WRITTEN, &self.written, "", &[]));
        }
        format!(
            "{{\"v\":2,\"line\":{},\"stopped\":{},\"error\":{error},\"show\":{},\"hide\":{},\"name\":{},\"sections\":[{}],{},\"page\":{facts},\"names\":{names}}}",
            pieced(&line),
            self.stopped.is_some(),
            json_string(wording::PANEL_SHOW),
            json_string(wording::PANEL_HIDE),
            json_string(wording::PANEL_NAME),
            sections.join(","),
            self.parts(page),
        )
    }

    /// The page's problems and the round's in their parts, the rounds,
    /// and the site, as the members of a JSON object.
    fn parts(&self, page: Option<&Page>) -> String {
        let built = &self.built;
        let mut codes: Vec<Code> = Vec::new();
        let mut problems = Vec::new();
        if let (Some(page), Some(read)) = (page, &built.read) {
            let report = Report::new(read.renderer.sources(), &built.site, false);
            let parts = built.parts(&report, read);
            for problem in report.ordered(&page.problems) {
                codes.push(problem.kind.code());
                problems.push(parts.of(problem));
            }
        }
        let stopped = match &self.stopped {
            Some(stopped) => {
                codes.extend(stopped.errors.iter().map(|(code, _)| *code));
                let errors: Vec<&str> = (stopped.errors.iter())
                    .map(|(_, error)| error.as_str())
                    .collect();
                format!(
                    "{{\"at\":{},\"errors\":[{}]}}",
                    json_string(&stopped.at),
                    errors.join(",")
                )
            }
            None => "null".to_owned(),
        };
        let link = |link: &Link| {
            let day = match &link.day {
                Some(day) => format!(",\"day\":{}", json_string(day)),
                None => String::new(),
            };
            format!(
                "{{\"text\":{},\"href\":{}{day}}}",
                json_string(&link.named),
                json_string(&link.href)
            )
        };
        let links = |links: &[Link]| links.iter().map(link).collect::<Vec<_>>().join(",");
        let written: Vec<String> = self.written.iter().map(|line| pieced(line)).collect();
        let site = format!(
            "{{\"kinds\":[{}],\"gone\":{},\"drafts\":[{}],\"waiting\":[{}],\"written\":[{}]}}",
            built.kinds,
            self.gone
                .as_deref()
                .map_or_else(|| "null".to_owned(), pieced),
            links(&built.drafts),
            links(&built.waiting),
            written.join(","),
        );
        let or_null = |json: Option<String>| json.unwrap_or_else(|| "null".to_owned());
        format!(
            "\"problems\":[{}],\"stoppedBy\":{stopped},\"explanations\":{},\"round\":{},\
             \"rounds\":[{}],\"site\":{site},\"root\":{}",
            problems.join(","),
            problems::explanations(codes),
            or_null(self.round.clone()),
            self.rounds.join(","),
            or_null(self.root.as_deref().map(json_string)),
        )
    }
}

impl Built {
    /// What turns this round's problems into their parts.
    fn parts<'a>(&'a self, report: &'a Report<'a>, read: &'a Read) -> Parts<'a> {
        Parts {
            report,
            quiet: &self.quiet,
            read: Some(read),
            addresses: &self.addresses,
        }
    }
}

/// The site's folder as a whole path, as an editor takes it.
fn whole(site: &Path) -> String {
    let whole = std::fs::canonicalize(site)
        .or_else(|_| std::path::absolute(site))
        .unwrap_or_else(|_| site.to_owned())
        .display()
        .to_string();
    // Windows gives a canonical path a prefix no editor's link takes.
    match whole.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with(r"UNC\") => rest.to_owned(),
        _ => whole,
    }
}

/// What builds the page and which lists hold it, a label and what it
/// names on each line.
fn comes_from(report: &Report, explained: &Explained) -> String {
    let file = match explained.source {
        Some(source) => format!("`{}`", report.file(source)),
        None => wording::PANEL_NO_FILE.to_owned(),
    };
    let layout = match &explained.layout {
        Some(chosen) => {
            let written = chosen.origin.place.map(|place| report.at(place));
            let handed_down = chosen.origin.arrival == Arrival::HandedDown;
            wording::panel_layout(&shown(&chosen.name), written.as_deref(), handed_down)
        }
        None => wording::PANEL_NO_LAYOUT.to_owned(),
    };
    let lists: Vec<String> = (explained.lists.iter())
        .map(|member| {
            wording::panel_member(member.word, &shown(&member.list), member.at, member.of)
        })
        .collect();
    let feeds: Vec<String> = (explained.feeds.iter())
        .map(|feed| format!("`{}`", shown(feed)))
        .collect();
    let or = |said: Vec<String>, none: &str| match said.is_empty() {
        true => vec![none.to_owned()],
        false => said,
    };
    let facts = [
        (wording::PANEL_FILE, vec![file]),
        (
            wording::PANEL_ADDRESS,
            vec![format!("`{}`", shown(&explained.address))],
        ),
        (wording::PANEL_LAYOUT, vec![layout]),
        (wording::PANEL_LISTS, or(lists, wording::PANEL_IN_NO_LIST)),
        (wording::PANEL_FEEDS, or(feeds, wording::PANEL_IN_NO_FEED)),
    ];
    let facts: Vec<String> = (facts.iter())
        .map(|(label, said)| {
            let lines: Vec<String> = said.iter().map(|line| pieced(line)).collect();
            format!(
                "{{\"label\":{},\"said\":[{}]}}",
                json_string(label),
                lines.join(",")
            )
        })
        .collect();
    format!(
        "{{\"heading\":{},\"lines\":[],\"drawn\":\"\",\"lists\":[],\"facts\":[{}]}}",
        json_string(wording::PANEL_FROM),
        facts.join(","),
    )
}

/// Every name a template can write on the page, a table for `page` and
/// one for `site`.
fn shows(report: &Report, explained: &Explained) -> String {
    let tables: Vec<String> = (explained.groups.iter())
        .map(|group| {
            let empty = |row: &&Row| row.held == Held::Nothing;
            let rows: Vec<String> = (group.rows.iter())
                .map(|row| {
                    let holds = page::holds(row.holds);
                    let value = page::value(row);
                    let has = match row.has.is_empty() {
                        true => String::new(),
                        false => wording::panel_has(&row.has),
                    };
                    let from = row.from.map(|place| report.at(place)).unwrap_or_default();
                    format!(
                        "{{\"name\":{},\"holds\":{},\"value\":{},\"empty\":{},\"has\":{},\"from\":{}}}",
                        json_string(&shown(&row.name)),
                        json_string(holds),
                        json_string(&value),
                        row.held == Held::Nothing,
                        pieced(&has),
                        json_string(&from),
                    )
                })
                .collect();
            let caption = wording::panel_group(
                group.of,
                group.rows.len(),
                group.rows.iter().filter(empty).count(),
            );
            format!(
                "{{\"caption\":{},\"rows\":[{}]}}",
                pieced(&caption),
                rows.join(",")
            )
        })
        .collect();
    let columns: Vec<String> = (wording::PANEL_COLUMNS.iter())
        .map(|column| json_string(column))
        .collect();
    format!(
        "{{\"heading\":{},\"lines\":[{}],\"drawn\":\"\",\"lists\":[],\"columns\":[{}],\"tables\":[{}]}}",
        json_string(wording::PANEL_SHOWS),
        pieced(wording::PANEL_SHOWS_HOW),
        columns.join(","),
        tables.join(","),
    )
}

/// One part of the panel: its heading, its lines, what is drawn beneath
/// them, and its lists of links, each under its own line.
fn section(heading: &str, lines: &[String], drawn: &str, lists: &[(&str, &Vec<Link>)]) -> String {
    let lines: Vec<String> = lines.iter().map(|line| pieced(line)).collect();
    let lists: Vec<String> = (lists.iter())
        .filter(|(_, links)| !links.is_empty())
        .map(|(label, links)| {
            let links: Vec<String> = (links.iter())
                .map(|link| {
                    format!(
                        "{{\"text\":{},\"href\":{}}}",
                        json_string(&link.text),
                        json_string(&link.href)
                    )
                })
                .collect();
            format!(
                "{{\"label\":{},\"links\":[{}]}}",
                pieced(label),
                links.join(",")
            )
        })
        .collect();
    format!(
        "{{\"heading\":{},\"lines\":[{}],\"drawn\":{},\"lists\":[{}]}}",
        json_string(heading),
        lines.join(","),
        json_string(drawn.trim_end()),
        lists.join(","),
    )
}

/// A sentence of Statac's own as JSON: its pieces in order, each a name,
/// `{"code": ...}`, or the words between, `{"text": ...}`.
fn pieced(sentence: &str) -> String {
    let pieces: Vec<String> = (pieces(sentence).into_iter())
        .map(|piece| match piece {
            Piece::Name(name) => format!("{{\"code\":{}}}", json_string(name)),
            Piece::Words(words) => format!("{{\"text\":{}}}", json_string(words)),
        })
        .collect();
    format!("[{}]", pieces.join(","))
}
