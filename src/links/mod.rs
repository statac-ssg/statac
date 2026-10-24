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

pub(crate) use resolve::Site;
pub(crate) use scan::{Attribute, Linked, handed, written, written_in_html};

use crate::content::address::{self, Address};
use crate::content::slug;
use crate::problem::{Kind, Problem};
use crate::reads::{Key, Reads, Translation};
use crate::site::{PageId, TemplateId};
use crate::source::Place;
use crate::templates::{Own, Renderer};
use resolve::{Arrival, Leads};
use scan::{Found, scan};

/// What a page's writing links to, what it hands its snippet calls, and
/// what the snippets it calls write.
#[derive(Default, Debug)]
pub struct Writing {
    pub written: Vec<Linked>,
    pub handed: Vec<Linked>,
    pub snipped: Vec<Snipped>,
}

/// A link a snippet wrote in a tag of its own text: its address as a
/// browser follows it, and the snippet and where in it the attribute's
/// value starts.
#[derive(Clone, Debug)]
pub struct Snipped {
    pub address: String,
    pub template: TemplateId,
    pub at: usize,
}

/// Each link in `html`, what a snippet wrote, in a tag of a template's own
/// text. `written_at` gives the template the byte at an offset of `html`
/// is the own text of, and where in it; none for a byte of a value's.
pub(crate) fn snipped(
    html: &str,
    written_at: impl Fn(usize) -> Option<(TemplateId, usize)>,
) -> Vec<Snipped> {
    let scanned = scan(html);
    (scanned.links.iter())
        .filter_map(|link| {
            // What opens the value, a quote or the `=`, is the template's
            // own where the template wrote the attribute.
            let (template, opens) = written_at(link.value.checked_sub(1)?)?;
            Some(Snipped {
                address: link.address(),
                template,
                at: opens + 1,
            })
        })
        .collect()
}

/// Each link in `text`, a template's own text, whose address the template
/// writes whole, from the top of the host or with its host, so that it
/// leads to the same place from every page: the address as a browser
/// follows it, and where in `text` its value is.
pub(crate) fn fixed(text: &str) -> Vec<(String, std::ops::Range<usize>)> {
    (scan(text).links.into_iter())
        .filter(|link| {
            let raw = link.raw.trim_start();
            !(raw.contains("{{") || raw.contains("{%"))
                && (raw.starts_with('/') || raw.contains("://"))
        })
        .map(|link| (link.address(), link.at..link.at + link.raw.len()))
        .collect()
}

impl Site {
    /// The file the site publishes that `written`, a link on the page
    /// published at `from`, its path from the top of its host with the
    /// site's own path, lands on, and what follows its `#`; none where it
    /// leads off the site or to nothing published.
    pub(crate) fn lands_on(&self, written: &str, from: &str) -> Option<(String, Option<String>)> {
        let Leads::Here(arrival) = self.leads(written, from) else {
            return None;
        };
        if arrival.outside {
            return None;
        }
        Some((self.file_at(&arrival.path)?, arrival.fragment))
    }
}

/// One built page, as the check reads it.
pub(crate) struct Page<'a> {
    pub id: PageId,
    /// Which of its list's pages it is, counted from 1.
    pub number: usize,
    pub address: &'a Address,
    pub reads: &'a Reads,
    /// The files its own sharing cards are published as, from the top of
    /// the built site, which the site holds once the page is built.
    pub cards: Vec<String>,
    /// Where its HTML is the own text of the templates it is built with.
    pub own: &'a Own,
    /// What the snippets its templates call write.
    pub snipped: &'a [Snipped],
}

/// What a built page's links came to, found as it was built.
#[derive(Clone, Debug)]
pub(crate) struct Checked {
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
    pub(crate) fn reached(&self) -> &[String] {
        &self.reached
    }
}

impl Checked {
    /// The same findings as another reading of the site has them, the
    /// page, the templates and each place as `to` gives them there; none
    /// where that reading has no such page, template or place.
    pub(crate) fn moved(&self, to: &Translation) -> Option<Checked> {
        let moved = |misses: &[Miss]| {
            let misses = misses.iter().map(|miss| {
                let blame = match &miss.blame {
                    Blame::Writing(place, handed) => Blame::Writing(to.place(*place)?, *handed),
                    Blame::Shown(place, handed) => Blame::Shown(to.place(*place)?, *handed),
                    Blame::Template(found, placed) => {
                        let placed = match placed {
                            Some(Placed::Attribute(template, at)) => {
                                Some(Placed::Attribute(to.template(*template)?, *at))
                            }
                            Some(Placed::Printed(place)) => {
                                Some(Placed::Printed(to.place(*place)?))
                            }
                            None => None,
                        };
                        Blame::Template(found.clone(), placed)
                    }
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
    /// A template or snippet, writing this link in its tag, placed where
    /// that is known.
    Template(Box<Found>, Option<Placed>),
}

/// Where a link a template or snippet wrote was written.
#[derive(Clone, Debug)]
enum Placed {
    /// The tag is a template's own text: the template, and where in it the
    /// attribute's value starts.
    Attribute(TemplateId, usize),
    /// The tag is in a value the template printed, written here.
    Printed(Place),
}

/// Another page's writing shown on the page being checked, with each
/// address it links to and each its snippets wrote, as spelled from the
/// top of the site where that differs, in order.
struct Shown<'a> {
    page: PageId,
    writing: &'a Writing,
    linked: Vec<Option<String>>,
    snipped: Vec<Option<String>>,
}

impl Site {
    /// Follows every link on `page`, built as `html`: what lands nowhere
    /// now, and what waits to be held to another page's ids.
    pub(crate) fn check(&self, renderer: &Renderer, page: Page<'_>, html: &str) -> Checked {
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
        let shown: Vec<Shown> = shown
            .into_iter()
            .filter_map(|other| {
                let writing = renderer.writing(other)?;
                let home = format!("{}{}", self.base(), site.pages.get(other).address.as_str());
                let moved = |address: &str| {
                    crate::elsewhere::address(address, &home).map(|moved| address::decoded(&moved))
                };
                let linked = (writing.written.iter())
                    .chain(&writing.handed)
                    .map(|w| moved(&w.address))
                    .collect();
                let snipped = writing.snipped.iter().map(|s| moved(&s.address)).collect();
                Some(Shown {
                    page: other,
                    writing,
                    linked,
                    snipped,
                })
            })
            .collect();
        // What the snippets built for the page and for its writing wrote,
        // and which of those links on the page have been placed.
        let snipped: Vec<&Snipped> = (page.snipped.iter())
            .chain(own.iter().flat_map(|own| &own.snipped))
            .collect();
        let mut placed_at: HashSet<usize> = HashSet::new();
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
            let same = |w: &&Linked| address::decoded(&w.address) == key;
            // Where its address was written, where that is known: a file of
            // the site's, or what a template made.
            let printed = renderer.printed_at(page.own, html, link.at, link.raw.len());
            let in_file = printed.filter(|at| renderer.template_of(at.source).is_none());
            // Else what opens the value, a quote or the `=`, is a template's
            // own where the template wrote the attribute.
            let mut placed = (in_file.is_none())
                .then(|| renderer.written_at(page.own, link.value.checked_sub(1)?))
                .flatten()
                .map(|(template, opens)| Placed::Attribute(template, opens + 1));
            // A template's link in another page's writing shown here, as a
            // snippet called there writes, is told on that page where it
            // leads the same from there.
            let called = in_file
                .is_none()
                .then(|| renderer.in_text(page.own, link.at));
            if let Some(call) = called.flatten() {
                let other = shown
                    .iter()
                    .find(|shown| site.pages.get(shown.page).source == Some(call.source));
                if let Some(other) = other {
                    let there = site.pages.get(other.page);
                    let at_home = format!("{}{}", self.base(), there.address.as_str());
                    if there.lands()
                        && self.leads(&written, &at_home) == self.leads(&written, &from)
                    {
                        continue;
                    }
                }
            }
            // In the writing of the page, or of another page shown here, it
            // is that writing's link at that place, or else of that address.
            let writer = in_file.and_then(|at| {
                let own = own.filter(|_| site.pages.get(page.id).source == Some(at.source));
                let own = own.map(|own| (page.id, own));
                own.or_else(|| {
                    let other =
                        |shown: &&Shown| site.pages.get(shown.page).source == Some(at.source);
                    shown
                        .iter()
                        .find(other)
                        .map(|shown| (shown.page, shown.writing))
                })
            });
            if let (Some(at), Some((writer, writing))) = (in_file, writer) {
                // Shown here, its address may be spelled from the top of the
                // site.
                let home = (writer != page.id)
                    .then(|| format!("{}{}", self.base(), site.pages.get(writer).address.as_str()));
                let alike = |w: &&Linked| {
                    let moved = || {
                        let home = home.as_deref()?;
                        crate::elsewhere::address(&w.address, home)
                    };
                    same(w) || moved().is_some_and(|moved| address::decoded(&moved) == key)
                };
                let there = |w: &&Linked| {
                    let (a, b) = (w.place.span, at.span);
                    alike(w) && w.place.source == at.source && a.start < b.end && b.start < a.end
                };
                let found = (writing.written.iter().find(there).map(|w| (w, false)))
                    .or_else(|| writing.handed.iter().find(there).map(|w| (w, true)))
                    .or_else(|| writing.written.iter().find(alike).map(|w| (w, false)));
                match found {
                    // The writing's own links were followed above.
                    Some((_, false)) if writer == page.id => continue,
                    Some((w, true)) if writer == page.id => {
                        let blame = Blame::Writing(w.place, w.handed);
                        self.follow(&written, &from, blame, &mut checked);
                        continue;
                    }
                    Some((w, _)) => {
                        self.shown(site, writer, w, &written, &from, &mut checked);
                        continue;
                    }
                    None => {}
                }
            }
            // Written in a file, it is told there.
            if let Some(at) = in_file {
                placed = Some(Placed::Printed(at));
            }
            // Where nothing says where it was written, a link of the
            // writing's own is one with its address, each taken once.
            let unknown = placed.is_none();
            if unknown && let Some(count) = left.get_mut(&key).filter(|count| **count > 0) {
                *count -= 1;
                continue;
            }
            if unknown && let Some(handed) = own.and_then(|own| own.handed.iter().find(same)) {
                let blame = Blame::Writing(handed.place, handed.handed);
                self.follow(&written, &from, blame, &mut checked);
                continue;
            }
            let other = unknown.then(|| {
                shown.iter().find_map(|shown| {
                    let writing = shown.writing;
                    let (w, _) = writing
                        .written
                        .iter()
                        .chain(&writing.handed)
                        .zip(&shown.linked)
                        .find(|(w, moved)| same(w) || moved.as_deref() == Some(key.as_str()))?;
                    Some((shown.page, w))
                })
            });
            if let Some((other, w)) = other.flatten() {
                self.shown(site, other, w, &written, &from, &mut checked);
                continue;
            }
            // Else a snippet wrote it in its own text: each of a snippet's
            // links with the same address is taken once, in order.
            if placed.is_none() {
                let same: Vec<usize> = (0..snipped.len())
                    .filter(|&at| address::decoded(&snipped[at].address) == key)
                    .collect();
                let at = (same.iter())
                    .find(|at| !placed_at.contains(at))
                    .or(same.first());
                if let Some(&at) = at {
                    placed_at.insert(at);
                    placed = Some(Placed::Attribute(snipped[at].template, snipped[at].at));
                }
            }
            // Or a snippet in another page's writing shown here, which is
            // told on that page where it leads the same from there.
            if unknown {
                let other = shown.iter().find_map(|shown| {
                    let snipped = shown.writing.snipped.iter().zip(&shown.snipped);
                    let (s, _) = snipped.into_iter().find(|(s, moved)| {
                        address::decoded(&s.address) == key
                            || moved.as_deref() == Some(key.as_str())
                    })?;
                    Some((shown.page, s))
                });
                if let Some((other, s)) = other {
                    let there = site.pages.get(other);
                    let at_home = format!("{}{}", self.base(), there.address.as_str());
                    if there.lands()
                        && self.leads(&written, &at_home) == self.leads(&written, &from)
                    {
                        continue;
                    }
                    placed = Some(Placed::Attribute(s.template, s.at));
                }
            }
            // Else it is in a value a template printed.
            if placed.is_none() {
                placed = printed.map(Placed::Printed);
            }
            let blame = Blame::Template(Box::new(link.clone()), placed);
            self.follow(&written, &from, blame, &mut checked);
        }
        checked.asked.sort_unstable();
        checked.asked.dedup();
        checked
    }

    /// Follows `w`, a link in the writing of `other`, shown on the page
    /// being checked as `written`: told there only where it leads elsewhere
    /// than from its own page.
    fn shown(
        &self,
        site: &crate::site::Site,
        other: PageId,
        w: &Linked,
        written: &str,
        from: &str,
        checked: &mut Checked,
    ) {
        let here = self.leads(written, from);
        let there = site.pages.get(other);
        let at_home = format!("{}{}", self.base(), there.address.as_str());
        if there.lands() && self.leads(written, &at_home) == here {
            return;
        }
        if let Leads::Here(arrival) = here {
            let blame = Blame::Shown(w.place, w.handed);
            self.arrive(arrival, written, blame, checked);
        }
    }

    /// Whether the site still holds, at each path a link of the page led
    /// to, what it held when the page was checked.
    pub(crate) fn holds_as(&self, checked: &Checked) -> bool {
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
    /// where it was written. `made` are files known only once the pages
    /// are built, which a link from any page lands on: the pages' sharing
    /// cards, and the scripts reading the search index.
    pub(crate) fn told<'a>(
        &self,
        renderer: &Renderer,
        checked: impl IntoIterator<Item = &'a Checked>,
        made: &HashSet<&str>,
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
                let made_later = |file: &str| made.contains(file);
                let found = miss.file.is_none() && !miss.arrival.outside && !made.is_empty();
                !(found && self.file_among(&miss.arrival.path, made_later).is_some())
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
                    let kind = self.kind(miss, page, there, wrote, named(*place), true);
                    Problem::at(kind, *place)
                }
                Blame::Shown(place, handed) => {
                    let wrote = in_writing(*place, *handed);
                    let kind = self.kind(miss, page, there, wrote, named(*place), false);
                    building(renderer, page, Problem::at(kind, *place))
                }
                Blame::Template(_, Some(Placed::Printed(place)))
                    if renderer.template_of(place.source).is_none() =>
                {
                    let wrote = Wrote {
                        helper: None,
                        used: false,
                    };
                    let kind = self.kind(miss, page, there, wrote, None, false);
                    building(renderer, page, Problem::at(kind, *place))
                }
                Blame::Template(found, placed) => {
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
                    let placed = placed.as_ref().and_then(|placed| match *placed {
                        Placed::Attribute(template, at) => {
                            Some((template, templates.get(&template)?.written_at(at)?))
                        }
                        Placed::Printed(place) => {
                            let template = renderer.template_of(place.source)?;
                            let at = place.span.start as usize..place.span.end as usize;
                            Some((template, at))
                        }
                    });
                    let best = placed.or_else(|| blame::best(candidates, found));
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
                    let kind = self.kind(miss, page, there, wrote, None, false);
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
    /// `own` says the link is written in `page`'s own writing, so that
    /// page isn't offered.
    fn kind(
        &self,
        miss: &Miss,
        page: &Checked,
        there: &[String],
        wrote: Wrote,
        named: Option<String>,
        own: bool,
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
            let leaves_out = own && self.file_at(path).as_ref() != Some(&page.file);
            let from = leaves_out.then_some(page.file.as_str());
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
