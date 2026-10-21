//! What the panel on a served page shows: how the last round went, what
//! there is to say of the page it is on, and how the site stands.
//!
//! Everything is worked out here and handed to the page as text for it to
//! place. Whatever comes from the site is text a page sets as text, never
//! as markup, and is shown as a terminal is shown it. A sentence of
//! Statac's own goes as its pieces, each name apart from the words round
//! it, where a terminal is shown it in backticks. The panel's own words
//! are handed to the page once, in the script that draws it.

mod page;
mod problems;
mod search;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use super::session::{Read, Round, Took};
use crate::build::Outcome;
use crate::content::address::on_the_wire;
use crate::json;
use crate::problem::Holds;
use crate::problem::{Code, Kind, Problem, Severity};
use crate::report::{Piece, Quiet, Report, pieces, shown};
use crate::wording::{self, PanelWord};
use problems::{Addresses, Parts};

/// The most problems the panel is handed at once.
const MOST: usize = 200;
/// How many rounds the panel keeps.
const ROUNDS: usize = 10;
/// The version of every answer the panel's script reads. A page loaded
/// from one Statac can be answered by another, whose answers its script
/// may not read.
const VERSION: u32 = 4;

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
    /// Whether serve shows drafts and posts waiting for their day.
    drafts: bool,
}

/// What serve knows of a round besides the round itself.
pub struct Besides<'a> {
    pub took: Took,
    /// The files it was for, as they are shown; none for the first.
    pub changed: &'a [String],
    /// What the terminal said of how it ended.
    pub said: &'a str,
    pub gone: Option<&'a str>,
    /// Whether serve shows drafts and posts waiting for their day.
    pub drafts: bool,
}

struct Stopped {
    /// When it stopped, and each error in its parts, with its kind.
    at: String,
    errors: Vec<(Code, String)>,
}

#[derive(Default)]
struct Built {
    /// When, by this machine's clock; empty before any round has built.
    at: String,
    /// What there is to say of a page, by the file it is published as.
    pages: HashMap<String, Page>,
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
    /// The path the site is published under, as `/blog`, or empty.
    base: String,
    /// Each language's search index, by the language, once asked.
    indexes: OnceLock<Vec<crate::search::Index>>,
}

struct Page {
    draft: bool,
    waiting: Option<String>,
    /// Its problems, quietened ones among them.
    problems: Vec<Problem>,
}

struct Link {
    /// What it names, and the day it waits for.
    named: String,
    to: To,
    day: Option<String>,
}

enum To {
    /// Its address.
    Page(String),
    /// A page left out, which has none: the file it is written in, and
    /// the line where the file holds more than the one page.
    File(Option<String>, Option<usize>),
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
        let written: Vec<String> = report
            .stamps(&round.stamps)
            .lines()
            .map(str::to_owned)
            .collect();
        let counts = problems::counts(&outcome.problems, &outcome.quiet);
        let state = json::string(match stopped {
            true => "stopped",
            false => "built",
        });
        let built_at = match stopped {
            true => Some(&before.built.at).filter(|at| !at.is_empty()),
            false => Some(&at),
        };
        let changed: Vec<String> = (besides.changed.iter())
            .map(|file| json::string(file))
            .collect();
        let this = format!(
            "{{\"state\":{state},\"at\":{},\"builtAt\":{},\"changed\":[{}],\"built\":{},\
             \"outputs\":{},\"took\":{},\"counts\":{counts}}}",
            json::string(&at),
            built_at.map_or_else(|| "null".to_owned(), |at| json::string(at)),
            changed.join(","),
            round.built,
            round.outputs,
            problems::took(&besides.took),
        );
        let mut rounds = vec![format!(
            "{{\"state\":{state},\"at\":{},\"said\":{},\"counts\":{counts},\"first\":{}}}",
            json::string(&at),
            pieced(besides.said),
            before.round.is_none(),
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
            drafts: besides.drafts,
        };
        if stopped {
            let errors = (outcome.problems.iter())
                .filter(|problem| problem.kind.severity() == Severity::Error);
            let addresses = OnceLock::new();
            let parts = Parts {
                report: &report,
                quiet: &outcome.quiet,
                read: round.read.as_ref(),
                addresses: &addresses,
            };
            panel.stopped = Some(Stopped {
                errors: (report.ordered(errors).iter())
                    .map(|error| (error.kind.code(), parts.of(error)))
                    .collect(),
                at,
            });
            return panel;
        }
        let link = |noted: &super::session::Noted, day: Option<&str>| Link {
            named: shown(noted.title.as_deref().unwrap_or(&noted.address)),
            to: To::Page(on_the_wire(&format!("{base}{}", noted.address))),
            day: day.map(str::to_owned),
        };
        let mut built = Built {
            at,
            site: site.to_owned(),
            kinds: problems::kinds(&outcome.problems, &outcome.quiet),
            base: base.to_owned(),
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
                problems: noted.problems,
            };
            built.pages.insert(noted.file, page);
        }
        left_out(&mut built, outcome, &report);
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
            return format!("{{\"v\":{VERSION},\"problems\":[],\"more\":0,\"explanations\":{{}}}}");
        };
        let report = Report::new(read.renderer.sources(), &built.site, false);
        let of_kind = |problem: &&Problem| kind.is_none_or(|kind| problem.kind.name() == kind);
        let ordered = report.ordered(built.problems.iter().filter(of_kind));
        let more = ordered.len().saturating_sub(MOST);
        let told = &ordered[..ordered.len() - more];
        let parts = built.parts(&report, read);
        let problems: Vec<String> = told.iter().map(|problem| parts.of(problem)).collect();
        format!(
            "{{\"v\":{VERSION},\"problems\":[{}],\"more\":{more},\"explanations\":{}}}",
            problems.join(","),
            problems::explanations(told.iter().map(|problem| problem.kind.code())),
        )
    }

    /// The panel for the page published as `file`, or for an address with
    /// no page at it, as JSON.
    pub fn to_json(&self, file: Option<&str>) -> String {
        let built = &self.built;
        let page = file.and_then(|file| built.pages.get(file));
        let explained = built.read.as_ref().and_then(|read| {
            let &(page, number) = read.pages.get(file?)?;
            Some((read, read.renderer.explained(page, number)))
        });
        let mut facts = "null".to_owned();
        let mut names = "[]".to_owned();
        if let Some((read, explained)) = explained {
            let report = Report::new(read.renderer.sources(), &built.site, false);
            let output = (read.built.iter()).find(|output| Some(output.file.as_str()) == file);
            let on = page::On {
                report: &report,
                renderer: &read.renderer,
                explained: &explained,
                reads: output.map(|output| &output.reads),
                asked: output.map(|output| &output.asked),
            };
            let waiting = page.and_then(|page| page.waiting.as_deref());
            facts = on.facts(page.is_some_and(|page| page.draft), waiting);
            names = on.names();
        }
        format!(
            "{{\"v\":{VERSION},{},\"page\":{facts},\"names\":{names}}}",
            self.parts(page),
        )
    }

    /// What a reader would find for `words`, its last word being typed
    /// where `typing`, in the site as the last round to build it read it,
    /// the language of the page published as `file` first, as JSON.
    pub fn search(&self, words: &str, typing: bool, file: Option<&str>) -> String {
        search::found(&self.built, self.drafts, words, typing, file)
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
                    json::string(&stopped.at),
                    errors.join(",")
                )
            }
            None => "null".to_owned(),
        };
        let link = |link: &Link| {
            let day = match &link.day {
                Some(day) => format!(",\"day\":{}", json::string(day)),
                None => String::new(),
            };
            let to = match &link.to {
                To::Page(href) => format!("\"href\":{}", json::string(href)),
                To::File(file, line) => format!(
                    "\"file\":{},\"line\":{}",
                    file.as_deref()
                        .map_or_else(|| "null".to_owned(), json::string),
                    line.map_or_else(|| "null".to_owned(), |line| line.to_string()),
                ),
            };
            format!("{{\"text\":{},{to}{day}}}", json::string(&link.named))
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
            "\"problems\":[{}],\"stopped\":{stopped},\"explanations\":{},\"round\":{},\
             \"rounds\":[{}],\"site\":{site},\"root\":{}",
            problems.join(","),
            problems::explanations(codes),
            or_null(self.round.clone()),
            self.rounds.join(","),
            or_null(self.root.as_deref().map(json::string)),
        )
    }
}

/// Lists the pages `outcome` leaves out, as its problems name them.
fn left_out(built: &mut Built, outcome: &Outcome, report: &Report) {
    let sources = outcome.sources();
    let mut files: Option<HashMap<_, _>> = None;
    let mut listed = HashSet::new();
    for problem in report.ordered(&outcome.problems) {
        let (links, path, day) = match &problem.kind {
            Kind::DraftLeftOut { path } => (&mut built.drafts, path, None),
            Kind::WaitingForItsDay { path, date } => (&mut built.waiting, path, Some(date)),
            _ => continue,
        };
        if !listed.insert(path) {
            continue;
        }
        let files =
            files.get_or_insert_with(|| (sources.ids()).map(|id| (sources.path(id), id)).collect());
        // A page made from an item is named by its file and the item, and
        // opened where its problem is.
        let to = match (files.get(Path::new(path)), problem.place) {
            (Some(&id), _) => To::File(Some(report.file(id)), None),
            (None, Some(place)) => To::File(
                Some(report.file(place.source)),
                Some(report.first_line(place)),
            ),
            (None, None) => To::File(None, None),
        };
        links.push(Link {
            named: shown(path),
            to,
            day: day.cloned(),
        });
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

/// A sentence of Statac's own as JSON: its pieces in order, each a name,
/// `{"code": ...}`, or the words between, `{"text": ...}`.
fn pieced(sentence: &str) -> String {
    let pieces: Vec<String> = (pieces(sentence).into_iter())
        .map(|piece| match piece {
            Piece::Name(name) => format!("{{\"code\":{}}}", json::string(name)),
            Piece::Words(words) => format!("{{\"text\":{}}}", json::string(words)),
        })
        .collect();
    format!("[{}]", pieces.join(","))
}

/// Every word the panel puts together for itself, as a JSON object of
/// each by its name: a sentence's pieces, or for a count, the pieces said
/// of one and of any other number.
pub fn words() -> &'static str {
    static WORDS: OnceLock<String> = OnceLock::new();
    WORDS.get_or_init(|| {
        let words: Vec<String> = (wording::panel_words().into_iter())
            .map(|(name, word)| {
                let said = match word {
                    PanelWord::Said(said) => pieced(&said),
                    PanelWord::Counted(one, many) => {
                        format!("{{\"one\":{},\"many\":{}}}", pieced(one), pieced(many))
                    }
                };
                format!("{}:{said}", json::string(name))
            })
            .collect();
        format!("{{{}}}", words.join(","))
    })
}

/// Every way the panel offers to write a name, as a JSON object of each
/// kind of thing a name holds, by the kind's name, and that kind's ways:
/// each the template and the name of the word for what it is for.
pub fn ways() -> &'static str {
    static WAYS: OnceLock<String> = OnceLock::new();
    WAYS.get_or_init(|| {
        let kinds: Vec<String> = (Holds::ALL.into_iter())
            .map(|holds| {
                let ways: Vec<String> = (page::ways(holds).iter())
                    .map(|(code, word)| format!("[{},{}]", json::string(code), json::string(word)))
                    .collect();
                format!("\"{}\":[{}]", page::kind(holds), ways.join(","))
            })
            .collect();
        format!("{{{}}}", kinds.join(","))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::templates::{Held, Renderer, Row};

    const CLIENT: &str = include_str!("client.js");

    #[test]
    fn every_kind_of_name_is_named_apart_and_has_its_ways_in_the_script() {
        let ways = ways();
        let mut kinds = HashSet::new();
        for holds in Holds::ALL {
            let kind = page::kind(holds);
            assert!(kinds.insert(kind), "{kind}");
            assert!(ways.contains(&format!("\"{kind}\":[")), "{kind}");
            for (_, word) in page::ways(holds) {
                assert!(words().contains(&format!("\"{word}\":")), "{word}");
            }
        }
        // Every kind the script asks after is one the panel sends.
        let asking = "row.kind === \"";
        assert!(CLIENT.contains(asking));
        for (at, _) in CLIENT.match_indices(asking) {
            let kind = &CLIENT[at + asking.len()..];
            let kind = &kind[..kind.find('"').unwrap()];
            assert!(kinds.contains(kind), "{kind}");
        }
        assert!(CLIENT.contains(&format!("const understood = {VERSION};")));
    }

    #[test]
    fn every_way_offered_is_one_a_template_can_write() {
        let root = std::env::temp_dir().join(format!("statac-panel-ways-{}", std::process::id()));
        let write = |path: &str, text: &str| {
            let path = root.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        };
        let _ = std::fs::remove_dir_all(&root);
        write(
            "statac.yaml",
            "language: en\ntitle: Site\ntimezone: UTC\npages.feed: false\n",
        );
        write(
            "content/blog/index.md",
            "---\ntitle: Blog\npages.layout: post\n---\n",
        );
        write(
            "content/blog/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-10-01\ntags: [Rust]\n---\nSome writing.\n",
        );
        write(
            "content/blog/post-2.md",
            "---\ntitle: Post 2\ndate: 2026-10-02\n---\n",
        );
        for name in ["home", "folder", "page", "post"] {
            write(&format!("templates/{name}.html"), "{{ page.title }}");
        }
        let today = Some(crate::date::Date::day_only(2026, 10, 4));
        let post = |renderer: &Renderer| {
            (renderer.site().pages.iter())
                .find(|(_, page)| page.address.as_str() == "/blog/post-1/")
                .map(|(id, _)| id)
                .unwrap()
        };

        // A name of each kind, one holding a value on the page where there
        // is one, written every way the panel offers for its kind.
        let (renderer, _) = crate::build::renderer(&root, today);
        let explained = renderer.explained(post(&renderer), 1);
        let rows: Vec<&Row> = (explained.groups.iter())
            .flat_map(|group| &group.rows)
            .collect();
        let mut template = String::new();
        for holds in Holds::ALL {
            let of_kind = || rows.iter().filter(|row| row.holds == holds);
            let row = (of_kind().find(|row| row.held != Held::Nothing))
                .or_else(|| of_kind().next())
                .unwrap_or_else(|| panic!("no name holds {holds:?}"));
            for (code, _) in page::ways(holds) {
                template.push_str(&code.replace("{name}", &row.name));
                template.push('\n');
            }
        }
        write("templates/post.html", &template);
        let (renderer, problems) = crate::build::renderer(&root, today);
        let rendered = renderer.render(post(&renderer));
        let _ = std::fs::remove_dir_all(&root);
        assert!(problems.is_empty(), "{problems:?}\n{template}");
        assert!(
            rendered.problems.is_empty(),
            "{:?}\n{template}",
            rendered.problems
        );
        assert!(rendered.html.is_some());
    }
}
