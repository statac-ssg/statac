//! Links checked across the built site.
//!
//! As each page is built, every address a reader can follow on it is
//! followed as a browser follows it, from where the page is published, and
//! held to what the built site publishes: a page, a feed, a file from
//! `assets/`, `static/` or beside the writing, or an old address that
//! sends readers on. An `#anchor` is held to the ids on the page it lands
//! on, once every page is built. Links to other sites are never fetched,
//! nor are links under a path `not_ours` names or into what
//! `after_build_makes` names. The pages built from templates are the ones
//! read: a feed's links are its posts', told on their pages, and a file
//! published as it is was written by the author's other tools.
//!
//! A link is told where it was written. The writing's own, Markdown and
//! HTML alike, are told at the link, once, however many pages show the
//! writing; where only another page showing the writing breaks it, it is
//! told at the link, naming that page. One a template or a snippet wrote
//! is told at the attribute in that file, once, naming the first page it
//! breaks on.
//!
//! An address is offered back as the link was written: one written with
//! `@url(...)`, or with a template's `url(...)`, as another of the same.
//! On a site published under a path, a link from the top of the domain
//! that the site would publish under its own path is offered as one.

mod blame;
mod resolve;
mod scan;
#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};

pub use resolve::Site;
pub use scan::{Attribute, Linked, handed, written, written_in_html};

use crate::content::address::{self, Address};
use crate::content::slug;
use crate::problem::{Kind, Problem};
use crate::reads::{Key, Reads, Translation};
use crate::site::{PageId, TemplateId};
use crate::source::Place;
use crate::templates::Renderer;
use resolve::{Arrival, Leads};
use scan::{Found, scan};

/// What a page's writing links to, and what it hands its snippet calls.
#[derive(Default, Debug)]
pub struct Writing {
    pub written: Vec<Linked>,
    pub handed: Vec<Linked>,
}

/// One built page, as the check reads it.
pub struct Page<'a> {
    pub id: PageId,
    /// Which of its list's pages it is, counted from 1.
    pub number: usize,
    pub address: &'a Address,
    pub reads: &'a Reads,
    /// The files its own sharing cards are published as, from the top of
    /// the built site, which the site holds once the page is built.
    pub cards: Vec<String>,
}

/// What a built page's links came to, found as it was built.
#[derive(Clone, Debug)]
pub struct Checked {
    page: PageId,
    number: usize,
    /// The file it is published as, from the top of the built site.
    file: String,
    /// Sorted, each once.
    ids: Vec<String>,
    misses: Vec<Miss>,
    /// Links to an id on another page, held until every page is built.
    anchors: Vec<Miss>,
    /// The templates and snippets that built it, in the order read.
    templates: Vec<TemplateId>,
    /// The files it links that are published only where a page links them.
    reached: Vec<String>,
    /// The files its own sharing cards are published as.
    cards: Vec<String>,
    /// Each path on the site a link of its led to, sorted, with the file
    /// the site held there, where the site notes what its pages ask of it.
    asked: Vec<(String, Option<String>)>,
}

impl Checked {
    /// The files the page links that are published only where a page
    /// links them.
    pub fn reached(&self) -> &[String] {
        &self.reached
    }
}

impl Checked {
    /// The same findings as another reading of the site has them, the
    /// page, the templates and each place as `to` gives them there; none
    /// where that reading has no such page, template or place.
    pub fn moved(&self, to: &Translation) -> Option<Checked> {
        let moved = |misses: &[Miss]| {
            let misses = misses.iter().map(|miss| {
                let blame = match &miss.blame {
                    Blame::Writing(place, handed) => Blame::Writing(to.place(*place)?, *handed),
                    Blame::Shown(place, handed) => Blame::Shown(to.place(*place)?, *handed),
                    Blame::Template(found) => Blame::Template(found.clone()),
                };
                Some(Miss {
                    blame,
                    ..miss.clone()
                })
            });
            misses.collect::<Option<Vec<Miss>>>()
        };
        let templates = self.templates.iter().map(|&id| to.template(id));
        Some(Checked {
            page: to.page(self.page)?,
            misses: moved(&self.misses)?,
            anchors: moved(&self.anchors)?,
            templates: templates.collect::<Option<_>>()?,
            ..self.clone()
        })
    }
}

/// A link that lands nowhere, or on a page that may not have its id.
#[derive(Clone, Debug)]
struct Miss {
    blame: Blame,
    /// The address as a browser follows it.
    written: String,
    arrival: Arrival,
    /// The file it lands on, where it lands on one.
    file: Option<String>,
}

/// Who wrote a link.
#[derive(Clone, Debug)]
enum Blame {
    /// The page's own writing, at this place, and whether as a value
    /// handed to a snippet call.
    Writing(Place, bool),
    /// Another page's writing, at this place, shown on the page, and
    /// whether as a value handed to a snippet call.
    Shown(Place, bool),
    /// A template or snippet, writing this link in its tag.
    Template(Box<Found>),
}

impl Site {
    /// Follows every link on `page`, built as `html`: what lands nowhere
    /// now, and what waits to be held to another page's ids.
    pub fn check(&self, renderer: &Renderer, page: Page<'_>, html: &str) -> Checked {
        let site = renderer.site();
        let scanned = scan(html);
        let published = format!("{}{}", self.base(), page.address.as_str());
        let from = match scanned
            .base
            .as_deref()
            .map(|base| self.leads(base, &published))
        {
            Some(Leads::Here(base)) => format!("{}{}", self.base(), base.path),
            _ => published,
        };
        let mut templates = Vec::new();
        let mut shown = Vec::new();
        for key in page.reads.keys() {
            match *key {
                Key::Template(id) if !templates.contains(&id) => templates.push(id),
                Key::Body(other)
                    if (other != page.id || page.number > 1) && !shown.contains(&other) =>
                {
                    shown.push(other);
                }
                _ => {}
            }
        }
        let mut checked = Checked {
            page: page.id,
            number: page.number,
            file: page.address.file(),
            ids: scanned.ids,
            misses: Vec::new(),
            anchors: Vec::new(),
            templates,
            reached: Vec::new(),
            cards: page.cards,
            asked: Vec::new(),
        };
        let own = (page.number == 1)
            .then(|| renderer.writing(page.id))
            .flatten();
        // The writing's own links, each once, and the same links on the
        // page passed over.
        let mut left: HashMap<String, usize> = HashMap::new();
        for written in own.iter().flat_map(|own| &own.written) {
            *left.entry(address::decoded(&written.address)).or_default() += 1;
            let blame = Blame::Writing(written.place, written.handed);
            self.follow(&written.address, &from, blame, &mut checked);
        }
        // Shown here, the writing's addresses read from its own page are
        // spelled from the top of the site.
        let shown: Vec<(PageId, &Writing, Vec<Option<String>>)> = shown
            .into_iter()
            .filter_map(|other| {
                let writing = renderer.writing(other)?;
                let home = format!("{}{}", self.base(), site.pages.get(other).address.as_str());
                let moved = writing
                    .written
                    .iter()
                    .chain(&writing.handed)
                    .map(|w| crate::elsewhere::address(&w.address, &home))
                    .map(|moved| moved.map(|moved| address::decoded(&moved)))
                    .collect();
                Some((other, writing, moved))
            })
            .collect();
        for link in &scanned.links {
            let written = link.address();
            if let Some((path, file)) = self.watched(&written, &from) {
                if self.notes() {
                    let own = |file: &str| checked.cards.iter().any(|card| card == file);
                    let held = self.file_among(&path, own);
                    checked.asked.push((path, held));
                }
                checked.reached.extend(file);
            }
            let key = address::decoded(&written);
            if let Some(count) = left.get_mut(&key).filter(|count| **count > 0) {
                *count -= 1;
                continue;
            }
            let same = |w: &&Linked| address::decoded(&w.address) == key;
            if let Some(handed) = own.and_then(|own| own.handed.iter().find(same)) {
                let blame = Blame::Writing(handed.place, handed.handed);
                self.follow(&written, &from, blame, &mut checked);
                continue;
            }
            let other = shown.iter().find_map(|(other, writing, moved)| {
                let (w, _) = writing
                    .written
                    .iter()
                    .chain(&writing.handed)
                    .zip(moved)
                    .find(|(w, moved)| same(w) || moved.as_deref() == Some(key.as_str()))?;
                Some((*other, w))
            });
            if let Some((other, w)) = other {
                let here = self.leads(&written, &from);
                let there = site.pages.get(other);
                // Where it leads the same from its own page, it is told
                // there.
                let at_home = format!("{}{}", self.base(), there.address.as_str());
                if there.lands() && self.leads(&written, &at_home) == here {
                    continue;
                }
                if let Leads::Here(arrival) = here {
                    let blame = Blame::Shown(w.place, w.handed);
                    self.arrive(arrival, &written, blame, &mut checked);
                }
                continue;
            }
            let blame = Blame::Template(Box::new(link.clone()));
            self.follow(&written, &from, blame, &mut checked);
        }
        checked.asked.sort_unstable();
        checked.asked.dedup();
        checked
    }

    /// Whether the site still holds, at each path a link of the page led
    /// to, what it held when the page was checked.
    pub fn holds_as(&self, checked: &Checked) -> bool {
        let own = |file: &str| checked.cards.iter().any(|card| card == file);
        let mut asked = checked.asked.iter();
        asked.all(|(path, file)| self.file_among(path, own) == *file)
    }

    fn follow(&self, written: &str, from: &str, blame: Blame, checked: &mut Checked) {
        if let Leads::Here(arrival) = self.leads(written, from) {
            self.arrive(arrival, written, blame, checked);
        }
    }

    /// Holds a link arriving at `arrival` to what the site publishes there.
    fn arrive(&self, arrival: Arrival, written: &str, blame: Blame, checked: &mut Checked) {
        let own = |file: &str| checked.cards.iter().any(|card| card == file);
        let file = match arrival.outside {
            true => None,
            false => self.file_among(&arrival.path, own),
        };
        if self.notes() && !arrival.outside {
            checked.asked.push((arrival.path.clone(), file.clone()));
        }
        let anchored = file.as_ref().is_some_and(|file| file.ends_with(".html"))
            && arrival
                .fragment
                .as_deref()
                .is_some_and(|fragment| !lands_anywhere(fragment));
        if file.is_some() && !anchored {
            return;
        }
        let here = file.as_deref() == Some(checked.file.as_str());
        if here
            && has_id(
                &checked.ids,
                arrival.fragment.as_deref().unwrap_or_default(),
            )
        {
            return;
        }
        let miss = Miss {
            blame,
            written: written.to_owned(),
            arrival,
            file,
        };
        match miss.file.is_some() && !here {
            true => checked.anchors.push(miss),
            false => checked.misses.push(miss),
        }
    }

    /// Every link on the built pages that lands nowhere, each told once,
    /// where it was written. `cards` are the files the pages' sharing
    /// cards are published as, which a link from any page lands on.
    pub fn told<'a>(
        &self,
        renderer: &Renderer,
        checked: impl IntoIterator<Item = &'a Checked>,
        cards: &HashSet<&str>,
    ) -> Vec<Problem> {
        let checked: Vec<&Checked> = checked.into_iter().collect();
        let ids: HashMap<&str, &[String]> = checked
            .iter()
            .map(|page| (page.file.as_str(), page.ids.as_slice()))
            .collect();
        let mut templates: HashMap<TemplateId, blame::Read> = HashMap::new();
        let mut misses: Vec<(&Checked, &Miss)> = Vec::new();
        for page in &checked {
            let anchors = page.anchors.iter().filter(|miss| {
                let fragment = miss.arrival.fragment.as_deref().unwrap_or_default();
                miss.file
                    .as_deref()
                    .and_then(|file| ids.get(file))
                    .is_some_and(|ids| !has_id(ids, fragment))
            });
            let nowhere = page.misses.iter().filter(|miss| {
                let on_a_card = |file: &str| cards.contains(file);
                let found = miss.file.is_none() && !miss.arrival.outside && !cards.is_empty();
                !(found && self.file_among(&miss.arrival.path, on_a_card).is_some())
            });
            misses.extend(nowhere.chain(anchors).map(|miss| (*page, miss)));
        }
        // The writing's own first, so a link it breaks on its own page is
        // told there, and not again where another page shows it.
        misses.sort_by_key(|(_, miss)| !matches!(miss.blame, Blame::Writing(..)));
        let mut told = Vec::new();
        let mut seen: HashSet<(Option<Place>, &str)> = HashSet::new();
        for (page, miss) in misses {
            let there = miss
                .file
                .as_deref()
                .map_or(&[][..], |file| ids.get(file).copied().unwrap_or_default());
            let sources = renderer.sources();
            let in_writing = |place: Place, handed: bool| {
                let path = sources.path(place.source).to_string_lossy();
                let helper = !(handed || path.ends_with(".html") || path.ends_with(".htm"));
                let written = sources.spelling(place);
                Wrote {
                    helper: helper.then_some(Helper::Call),
                    used: helper && written.starts_with("@url(") && written.ends_with(')'),
                }
            };
            let named = |place: Place| named_by_file(renderer.site(), place, &miss.written);
            let problem = match &miss.blame {
                Blame::Writing(place, handed) => {
                    let wrote = in_writing(*place, *handed);
                    let kind = self.kind(miss, page, there, wrote, named(*place));
                    Problem::at(kind, *place)
                }
                Blame::Shown(place, handed) => {
                    let wrote = in_writing(*place, *handed);
                    let kind = self.kind(miss, page, there, wrote, named(*place));
                    building(renderer, page, Problem::at(kind, *place))
                }
                Blame::Template(found) => {
                    for &template in &page.templates {
                        templates.entry(template).or_insert_with(|| {
                            let source = renderer.template_source(template);
                            blame::read(renderer.sources().text(source))
                        });
                    }
                    let candidates = page
                        .templates
                        .iter()
                        .map(|template| (*template, &templates[template]));
                    let best = blame::best(candidates, found);
                    let wrote = match &best {
                        Some((template, at)) => {
                            let source = renderer.template_source(*template);
                            let written = &sources.text(source)[at.clone()];
                            // HTML printed whole is the writing's only
                            // where it was read as Markdown.
                            let helper = match templates[template].prints(at) {
                                false => Some(Helper::Function),
                                true if written.contains("markdown") => Some(Helper::Call),
                                true => None,
                            };
                            let inside = written.trim_start_matches(['{', '-', ' ']);
                            Wrote {
                                helper,
                                used: helper == Some(Helper::Function)
                                    && written.starts_with("{{")
                                    && inside.starts_with("url("),
                            }
                        }
                        None => Wrote {
                            helper: None,
                            used: false,
                        },
                    };
                    let kind = self.kind(miss, page, there, wrote, None);
                    let place = best.map(|(template, at)| Place {
                        source: renderer.template_source(template),
                        span: crate::source::Span::new(at.start, at.end),
                    });
                    building(renderer, page, Problem::placed(kind, place))
                }
            };
            // Once for each place it is written, naming the first page it
            // breaks on.
            if seen.insert((problem.place, miss.written.as_str())) {
                told.push(problem);
            }
        }
        told
    }

    /// What a link that lands nowhere is told as, with the nearest address
    /// the site publishes, or the nearest id on `there`, the page it lands
    /// on. `wrote` says how the link was written, and `named` is the
    /// address of the page it names by its file, which is offered first.
    fn kind(
        &self,
        miss: &Miss,
        page: &Checked,
        there: &[String],
        wrote: Wrote,
        named: Option<String>,
    ) -> Kind {
        let fragment = miss.arrival.fragment.as_deref();
        let Some(file) = &miss.file else {
            let written = miss.written.trim_matches(|c: char| c.is_ascii_whitespace());
            let inside = wrote.helper.filter(|_| {
                miss.arrival.outside
                    && crate::markdown::from_the_top(written)
                    && self.file_at(&miss.arrival.path).is_some()
            });
            if let Some(helper) = inside {
                return Kind::BrokenLink {
                    target: miss.written.clone(),
                    suggestion: Some(helper.spelled(written)),
                    inside: true,
                };
            }
            // A link naming its own page from outside the site's path
            // means that page.
            let path = &miss.arrival.path;
            let own = self.file_at(path).as_ref() != Some(&page.file);
            let from = own.then_some(page.file.as_str());
            let named = named.filter(|address| self.file_at(address).is_some());
            let nearest = named.as_deref().or_else(|| self.nearest(path, from));
            let suggestion = nearest.map(|nearest| {
                let origin = miss.arrival.origin.as_deref().unwrap_or_default();
                let fragment = fragment.map(|f| format!("#{f}")).unwrap_or_default();
                match wrote.helper.filter(|_| wrote.used) {
                    Some(helper) => helper.spelled(&format!("{nearest}{fragment}")),
                    None => {
                        let base = self.base_of(&miss.arrival);
                        format!("{origin}{base}{nearest}{fragment}")
                    }
                }
            });
            return Kind::BrokenLink {
                target: miss.written.clone(),
                suggestion,
                inside: false,
            };
        };
        let id = address::decoded(fragment.unwrap_or_default());
        // An id that looks the same, with its marks put together another
        // way; then the id a heading of these words is given; then the
        // nearest spelling.
        let composed = slug::nfc(&id);
        let made = slug::anchor(&id);
        let suggestion = there
            .iter()
            .find(|other| slug::nfc(other) == composed)
            .or_else(|| there.iter().find(|other| **other == made))
            .map(String::as_str)
            .or_else(|| crate::suggest::nearest(&id, there.iter().map(String::as_str)))
            .map(|id| format!("#{id}"));
        let page = (*file != page.file).then(|| {
            let address = match file.strip_suffix("index.html") {
                Some(folder) if folder.is_empty() || folder.ends_with('/') => folder,
                _ => file.as_str(),
            };
            format!("{}/{address}", self.base())
        });
        Kind::MissingAnchor {
            id,
            page,
            suggestion,
        }
    }
}

/// The address of the page a link names by its file: `written` read as a
/// path from the folder of the file at `place`, where a page has that
/// path under `content/`, or is the page kept as a folder of that name.
fn named_by_file(site: &crate::site::Site, place: Place, written: &str) -> Option<String> {
    let written = written.trim_matches(|c: char| c.is_ascii_whitespace());
    let path = written.split(['#', '?']).next().unwrap_or_default();
    if path.is_empty() || path.starts_with('/') || path.contains(':') {
        return None;
    }
    let own = |page: &&crate::site::Page| page.source.is_some() && page.made.is_none();
    let pages = || site.pages.iter().map(|(_, page)| page).filter(own);
    let from = pages().find(|page| page.source == Some(place.source))?;
    let mut names: Vec<&str> = from.path.split('/').collect();
    names.pop();
    let path = address::decoded(path);
    for name in path.split('/') {
        match name {
            "" | "." => {}
            ".." => {
                names.pop()?;
            }
            name => names.push(name),
        }
    }
    let file = names.join("/");
    let folder = file
        .rsplit_once('.')
        .filter(|(_, ending)| !ending.contains('/'))
        .map(|(stem, ending)| format!("{stem}/index.{ending}"));
    let page = pages().find(|page| page.path == file || Some(&page.path) == folder.as_ref())?;
    page.lands().then(|| page.address.as_str().to_owned())
}

/// How a link was written, as far as an address inside the site goes.
#[derive(Copy, Clone)]
struct Wrote {
    /// What the file it is written in reads as an address inside the
    /// site; none for one that reads nothing so.
    helper: Option<Helper>,
    /// Whether the link was written with it.
    used: bool,
}

/// What stands for an address inside the site, wherever the site is
/// published.
#[derive(Copy, Clone, PartialEq, Eq)]
enum Helper {
    /// `@url(...)`, in writing.
    Call,
    /// `{{ url("...") }}`, in a template or snippet.
    Function,
}

impl Helper {
    /// `address`, from the top of the site, as the helper is handed it.
    fn spelled(self, address: &str) -> String {
        let quoted = address.contains([',', '(', ')']) || address.contains(char::is_whitespace);
        match self {
            Helper::Call if quoted => format!("@url(\"{address}\")"),
            Helper::Call => format!("@url({address})"),
            Helper::Function => format!("{{{{ url(\"{address}\") }}}}"),
        }
    }
}

/// The problem, met building `page`.
fn building(renderer: &Renderer, page: &Checked, problem: Problem) -> Problem {
    match renderer.building(page.page, page.number) {
        Some((relation, head)) => problem.met(relation, head),
        None => problem,
    }
}

/// Whether a fragment lands on any page: none at all, `#top`, and a
/// fragment picking out words are a browser's own.
fn lands_anywhere(fragment: &str) -> bool {
    fragment.is_empty() || fragment.eq_ignore_ascii_case("top") || fragment.starts_with(":~:")
}

/// Whether `ids`, sorted, hold the id a fragment names, as written or with
/// its `%XX` read.
fn has_id(ids: &[String], fragment: &str) -> bool {
    let found = |id: &str| ids.binary_search_by(|other| other.as_str().cmp(id)).is_ok();
    lands_anywhere(fragment) || found(fragment) || found(&address::decoded(fragment))
}
