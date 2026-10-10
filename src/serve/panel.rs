//! What the panel on a served page shows: how the last round went, what
//! there is to say of the page it is on, and how the site stands.
//!
//! Everything is worked out here and handed to the page as text for it to
//! place. Whatever comes from the site is text a page sets as text, never
//! as markup, and is shown as a terminal is shown it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::live::json_string;
use super::session::{Read, Round};
use crate::content::address::on_the_wire;
use crate::origin::Arrival;
use crate::problem::{Problem, Severity};
use crate::report::{Report, shown};
use crate::templates::{Explained, Held, Holds, Row};
use crate::wording;

/// What the panel shows after one round.
#[derive(Default)]
pub struct Panel {
    /// The errors of a round that stopped.
    stopped: Option<Stopped>,
    /// The site as the last round to build it left it.
    built: Arc<Built>,
    /// What the round wrote into the site's own files, a line for each.
    written: Vec<String>,
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
    /// What there is to say of a page, by the file it is published as.
    pages: HashMap<String, Page>,
    /// The site's problems counted by kind, a line for each severity.
    counts: Vec<String>,
    drafts: Vec<Link>,
    waiting: Vec<Link>,
    /// The site as that round read it, and its folder.
    read: Option<Read>,
    site: PathBuf,
}

struct Page {
    draft: bool,
    waiting: Option<String>,
    /// Its problems, drawn as `check` draws them; empty where it has none.
    drawn: String,
}

struct Link {
    text: String,
    href: String,
}

/// The time of day on this machine's clock.
fn now() -> String {
    jiff::Zoned::now().strftime("%H:%M:%S").to_string()
}

impl Panel {
    /// The panel after `round` of the site at `site`, published under
    /// `base`. `before` is the panel of the round before, whose pages are
    /// still the ones answered where this round stopped.
    pub fn after(round: &Round, site: &Path, base: &str, before: &Panel) -> Panel {
        let outcome = &round.outcome;
        let report = Report::new(outcome.sources(), site, false);
        let unquietened = |problem: &&Problem| !outcome.quiet.quietens(&problem.kind);
        let written: Vec<String> = report
            .stamps(&round.stamps)
            .lines()
            .map(str::to_owned)
            .collect();
        if round.outcome.stopped() {
            let errors: Vec<&Problem> = (outcome.problems.iter())
                .filter(|problem| problem.kind.severity() == Severity::Error)
                .collect();
            let kept = !before.built.line.is_empty();
            let first = report.ordered(errors.iter().copied()).into_iter().next();
            return Panel {
                stopped: Some(Stopped {
                    line: wording::panel_stopped(&now(), errors.len(), kept),
                    first: first.map(|first| First {
                        headline: shown(&first.headline()),
                        location: report.location(first).unwrap_or_default(),
                        suggestion: first.suggestion().map(|it| shown(&it)).unwrap_or_default(),
                    }),
                    more: errors.len().saturating_sub(1),
                    drawn: report.problems(errors),
                }),
                built: Arc::clone(&before.built),
                written,
            };
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
            &now(),
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
            }
        };
        let read = round.read.as_ref().map(|read| Read {
            renderer: read.renderer.clone(),
            pages: read.pages.clone(),
        });
        let mut built = Built {
            line,
            counts: report.counted(&outcome.problems, &outcome.quiet),
            read,
            site: site.to_owned(),
            ..Built::default()
        };
        for noted in &round.noted {
            if noted.draft {
                built.drafts.push(link(noted, None));
            } else if let Some(day) = &noted.waiting {
                built.waiting.push(link(noted, Some(day)));
            }
            let page = Page {
                draft: noted.draft,
                waiting: noted.waiting.clone(),
                drawn: report.problems(noted.problems.iter().filter(unquietened)),
            };
            built.pages.insert(noted.file.clone(), page);
        }
        Panel {
            stopped: None,
            built: Arc::new(built),
            written,
        }
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
                    json_string(&first.headline),
                    json_string(&first.location),
                    json_string(&first.suggestion),
                    json_string(&more),
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
        if let Some((read, explained)) = explained {
            let report = Report::new(read.renderer.sources(), &built.site, false);
            sections.push(comes_from(&report, &explained));
            sections.push(shows(&report, &explained));
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
            "{{\"line\":{},\"stopped\":{},\"error\":{error},\"show\":{},\"hide\":{},\"name\":{},\"sections\":[{}]}}",
            json_string(&line),
            self.stopped.is_some(),
            json_string(wording::PANEL_SHOW),
            json_string(wording::PANEL_HIDE),
            json_string(wording::PANEL_NAME),
            sections.join(","),
        )
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
            let said: Vec<String> = said.iter().map(|line| json_string(line)).collect();
            format!(
                "{{\"label\":{},\"said\":[{}]}}",
                json_string(label),
                said.join(",")
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
    let count = |row: &Row, one: &str, many: &str| match row.held {
        Held::Count(count) => wording::panel_count(count, one, many),
        _ => String::new(),
    };
    let tables: Vec<String> = (explained.groups.iter())
        .map(|group| {
            let empty = |row: &&Row| row.held == Held::Nothing;
            let rows: Vec<String> = (group.rows.iter())
                .map(|row| {
                    let holds = match row.holds {
                        Holds::Text => "text",
                        Holds::Html => "HTML",
                        Holds::Number => "number",
                        Holds::Truth => "true or false",
                        Holds::Date => "date",
                        Holds::Page => "page",
                        Holds::Pages => "pages",
                        Holds::List => "list",
                        Holds::Lists => "lists",
                        Holds::Items => "list",
                        Holds::Fields => "fields",
                        Holds::Mentions => "webmentions",
                        Holds::Call => "call",
                        Holds::Unknown => "",
                    };
                    let value = match &row.held {
                        Held::Shown(text) => shown(text),
                        Held::Count(_) => match row.holds {
                            Holds::Pages => count(row, "page", "pages"),
                            Holds::Lists => count(row, "list", "lists"),
                            Holds::Mentions => count(row, "mention", "mentions"),
                            _ => count(row, "item", "items"),
                        },
                        Held::Fields => wording::PANEL_FIELDS_BELOW.to_owned(),
                        Held::Call => wording::PANEL_A_CALL.to_owned(),
                        Held::More(more) => wording::panel_more_fields(*more),
                        Held::Nothing => wording::PANEL_NO_VALUE.to_owned(),
                    };
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
                        json_string(&has),
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
                json_string(&caption),
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
        json_string(wording::PANEL_SHOWS_HOW),
        columns.join(","),
        tables.join(","),
    )
}

/// One part of the panel: its heading, its lines, what is drawn beneath
/// them, and its lists of links, each under its own line.
fn section(heading: &str, lines: &[String], drawn: &str, lists: &[(&str, &Vec<Link>)]) -> String {
    let lines: Vec<String> = lines.iter().map(|line| json_string(line)).collect();
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
                json_string(label),
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
