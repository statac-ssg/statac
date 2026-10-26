//! The posts feed readers know: where each post in a feed was last
//! published, and the id it had there.
//!
//! Kept as `.statac/feeds`, a post a line in the order of their files,
//! and written only by a build that publishes. A post in a feed standing
//! at another address than it is known at, with no `id:` of its own, has a
//! new id, so feed readers show it again as new: the build says so, with
//! the `id:` line that gives it back the id they know. Until the line is
//! written the post stays known as it was, so every build says it again.
//!
//! A post is the one known at its file's path, or one made from a named
//! item of a table at its page's path, made of the page that made it and
//! the item's name. Where no page is at a known path any more, the post it
//! was is the one sending readers on from the address known, or the one
//! post whose file holds the same bytes, or for an item, whose fields are
//! written the same. An item in a list has no path of its own, its place in the
//! list changing as items are added, so it is known only those two ways.
//! Nothing is guessed from a title or a day. Losing the file loses only
//! what it would have offered.
//!
//! An item takes its `id:` in its record, by hand: nothing here writes
//! into a data file.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use rayon::prelude::*;

use super::Feed;
use super::ids::{entry_id, is_a_feed_id, kept_id};
use crate::album::{KEPT, escaped, make_kept, unescaped};
use crate::config::Settings;
use crate::config::words::PageWord;
use crate::dates::Dates;
use crate::ids::PageId;
use crate::problem::{Kind, Problem};
use crate::site::{Item, Made, Page, SentTo, Site};
use crate::source::Sources;

/// The file's name in what Statac keeps.
pub(crate) const FILE: &str = "feeds";

const OPENING: &str = "statac feeds 1";

/// What opens every id Statac makes.
const MADE: &str = "urn:uuid:";

/// A post as feed readers know it.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Line {
    /// Its path, as [`path_of`] gives it; empty for an item in a list.
    path: String,
    /// Where it was published, without the site's own path.
    address: String,
    /// The id its entry had.
    id: String,
    /// What its file held, or its item's fields.
    hash: String,
}

impl Line {
    fn written(&self) -> String {
        let fields = [&self.path, &self.address, &self.id, &self.hash];
        let fields: Vec<String> = fields.into_iter().map(|field| escaped(field)).collect();
        fields.join("\t")
    }

    fn read(line: &str) -> Option<Line> {
        let mut fields = line.split('\t').map(unescaped);
        let mut next = || fields.next().flatten();
        let line = Line {
            path: next()?,
            address: next()?,
            id: next()?,
            hash: next()?,
        };
        fields.next().is_none().then_some(line)
    }
}

/// What the last build to publish kept, held to the site as it is now.
#[derive(Default, Debug)]
pub(crate) struct Known {
    /// The file as it was read, so one saying the same is left alone.
    text: Option<String>,
    /// Whether a file was read that a build kept.
    recorded: bool,
    lines: Vec<Line>,
    /// The posts the site's feeds carry now.
    posts: Vec<PageId>,
    /// The line each of them is known by.
    by: HashMap<PageId, usize>,
}

fn hash_of(text: &str) -> String {
    blake3::hash(text.as_bytes()).to_hex()[..32].to_owned()
}

/// The path a post is known at: its file's under `content/`, or for a
/// post made from a named item, its page's. None for an item in a list.
fn path_of(page: &Page) -> Option<&str> {
    match &page.made {
        None => Some(&page.path),
        Some(Made {
            item: Item::Key(_), ..
        }) => Some(&page.path),
        Some(_) => None,
    }
}

/// The hash of what a post holds: its file's text, or where it is made
/// from an item, its item's fields as written.
fn hash_held(page: &Page, sources: &Sources) -> Option<String> {
    let text = match page.made.as_ref().map(|made| made.fields) {
        None => sources.text(page.source?),
        Some(fields) => sources.spelling(fields?),
    };
    Some(hash_of(text))
}

/// Whether a run that builds drafts brought `page` in: a draft, or a post
/// waiting for its day.
fn brought_in(page: &Page, dates: &Dates) -> bool {
    let waits = page.speaks_for.is_none()
        && page
            .date
            .as_ref()
            .is_some_and(|date| dates.after_today(&date.value));
    waits || page.words.truth(PageWord::Draft) == Some(true)
}

/// The posts the site's feeds carry, each once, in the order of their
/// pages' paths: those written in files of their own or made from items,
/// and published whatever the run brings in.
fn posts(site: &Site, dates: &Dates, feeds: &[Feed]) -> Vec<PageId> {
    let mut posts: Vec<PageId> = feeds
        .iter()
        .flat_map(|feed| site.lists.get(feed.list).members.iter().copied())
        .filter(|&post| {
            let page = site.pages.get(post);
            page.date.is_some() && page.source.is_some() && page.lands() && !brought_in(page, dates)
        })
        .collect();
    posts.sort_unstable();
    posts.dedup();
    posts.sort_by(|a, b| {
        let path = |post: &PageId| site.pages.get(*post).path.as_str();
        path(a).cmp(path(b))
    });
    posts
}

impl Known {
    /// Reads what the site at `at` keeps, and says of each post it knows
    /// at another address what feed readers will make of it. Nothing is
    /// read for a site with no feed.
    pub(crate) fn read(
        at: &Path,
        site: &Site,
        settings: &Settings,
        dates: &Dates,
        sources: &Sources,
        feeds: &[Feed],
    ) -> (Known, Vec<Problem>) {
        if feeds.is_empty() {
            return (Known::default(), Vec::new());
        }
        let mut known = Known::load(at);
        known.posts = posts(site, dates, feeds);
        known.hold_to(site, sources);
        // Feed readers know a post where the site is at its own address,
        // whatever address the run was given.
        let base = settings
            .home()
            .map(crate::config::path_of)
            .unwrap_or_default();
        let problems = (known.posts.iter())
            .filter_map(|&post| {
                let (line, id) = known.moved(site, settings, dates, post)?;
                let kind = Kind::FeedIdChanged {
                    was: format!("{base}{}", line.address),
                    id: id.to_owned(),
                    item: site.pages.get(post).made.is_some(),
                };
                Some(Problem::placed(kind, site.head(post, sources)))
            })
            .collect();
        (known, problems)
    }

    fn load(at: &Path) -> Known {
        let mut known = Known::default();
        // Asked what it is through the handle it is read by, so no link
        // put in its place, or in place of `.statac`, is read.
        let Ok(file) = crate::platform::open_file(&at.join(KEPT), FILE) else {
            return known;
        };
        if !crate::platform::named_once(&file) {
            return known;
        }
        let Ok(text) = std::io::read_to_string(file) else {
            return known;
        };
        let mut lines = text.lines();
        if lines.next() == Some(OPENING) {
            known.recorded = true;
            known.lines = lines.filter_map(Line::read).collect();
        }
        known.text = Some(text);
        known
    }

    /// Finds the line each post is known by.
    fn hold_to(&mut self, site: &Site, sources: &Sources) {
        let lines = &self.lines;
        let at_path: HashMap<&str, usize> = (lines.iter().enumerate())
            .filter(|(_, line)| !line.path.is_empty())
            .map(|(at, line)| (line.path.as_str(), at))
            .collect();
        let mut by = HashMap::new();
        let mut unknown = Vec::new();
        for &post in &self.posts {
            let path = path_of(site.pages.get(post));
            match path.and_then(|path| at_path.get(path)) {
                Some(&line) => {
                    by.insert(post, line);
                }
                None => unknown.push(post),
            }
        }
        if !unknown.is_empty() && by.len() < lines.len() {
            let standing: HashSet<&str> = (site.pages.iter())
                .filter(|(_, page)| page.source.is_some())
                .filter_map(|(_, page)| path_of(page))
                .collect();
            let mut gone: Vec<usize> = (0..lines.len())
                .filter(|&line| !standing.contains(lines[line].path.as_str()))
                .collect();
            for redirect in &site.redirects {
                let post = redirect.page;
                if !matches!(redirect.to, SentTo::Page)
                    || by.contains_key(&post)
                    || !unknown.contains(&post)
                {
                    continue;
                }
                let from = redirect.from.as_str();
                if let Some(found) = gone.iter().position(|&line| lines[line].address == from) {
                    by.insert(post, gone.remove(found));
                }
            }
            let mut held: HashMap<&str, Vec<usize>> = HashMap::new();
            for line in gone {
                held.entry(lines[line].hash.as_str())
                    .or_default()
                    .push(line);
            }
            let mut holding: HashMap<String, Vec<PageId>> = HashMap::new();
            for &post in unknown.iter().filter(|post| !by.contains_key(post)) {
                if let Some(hash) = hash_held(site.pages.get(post), sources) {
                    holding.entry(hash).or_default().push(post);
                }
            }
            for (hash, posts) in holding {
                if let ([post], Some([line])) =
                    (posts.as_slice(), held.get(hash.as_str()).map(Vec::as_slice))
                {
                    by.insert(*post, *line);
                }
            }
        }
        self.by = by;
    }

    /// The line `post` is known by and the id to write as its `id:`, where
    /// it stands at another address than it is known at with an id made
    /// from that address, and an `id:` would give it the one known.
    fn moved(
        &self,
        site: &Site,
        settings: &Settings,
        dates: &Dates,
        post: PageId,
    ) -> Option<(&Line, &str)> {
        let line = &self.lines[*self.by.get(&post)?];
        let page = site.pages.get(post);
        if line.address == page.address.as_str()
            || page.words.get(PageWord::Id).is_some()
            || kept_id(site, settings, dates, post).is_some()
        {
            return None;
        }
        let id = line.id.strip_prefix(MADE).filter(|id| is_a_feed_id(id))?;
        Some((line, id))
    }

    /// Keeps the posts the site's feeds carry as feed readers now know
    /// them, for the build that has just published the site read. Nothing
    /// is written where the file says the same already, and what is kept
    /// is left alone while the site has no feed.
    pub(crate) fn keep(
        &self,
        at: &Path,
        site: &Site,
        settings: &Settings,
        dates: &Dates,
        sources: &Sources,
        feeds: &[Feed],
    ) -> std::io::Result<()> {
        if feeds.is_empty() {
            return Ok(());
        }
        let lines: Vec<String> = (self.posts)
            .par_iter()
            .filter_map(|&post| {
                let page = site.pages.get(post);
                let hash = hash_held(page, sources)?;
                let (address, id) = match self.moved(site, settings, dates, post) {
                    Some((line, _)) => (line.address.clone(), line.id.clone()),
                    None => (
                        page.address.as_str().to_owned(),
                        entry_id(site, settings, dates, post)?,
                    ),
                };
                let line = Line {
                    path: path_of(page).unwrap_or_default().to_owned(),
                    address,
                    id,
                    hash,
                };
                Some(line.written())
            })
            .collect();
        let mut text = format!("{OPENING}\n");
        for line in lines {
            text.push_str(&line);
            text.push('\n');
        }
        if self.text.as_deref() == Some(text.as_str()) {
            return Ok(());
        }
        let kept = make_kept(at)?;
        let fresh = format!("{FILE}.{}", std::process::id());
        crate::platform::keep_file(&kept, FILE, &fresh, text.as_bytes())
    }
}

/// The file [`Known::keep`] writes, as a problem names it.
pub(crate) fn shown() -> std::path::PathBuf {
    Path::new(KEPT).join(FILE)
}

/// What `write_feed_ids` asks for.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum IdsToWrite {
    /// Each post to give an `id:` line, in the order of their files, with
    /// the line.
    Lines(Vec<(PageId, String)>),
    /// Posts are without an `id:`, and no build has kept what feed readers
    /// know: which of them are new can't be told, so none is given one.
    NoRecord,
}

/// The posts `write_feed_ids` gives an `id:` line, held to what the last
/// build to publish kept: a post in a feed with no `id:` that feed
/// readers don't know yet, given the id it has now, and one they know at
/// another address, given the id it had there. A post made from an item
/// is given none, its line written in its record by hand. None on a site
/// that leaves the setting off.
pub(crate) fn ids_to_write(
    known: &Known,
    site: &Site,
    settings: &Settings,
    dates: &Dates,
) -> IdsToWrite {
    if settings.truth("write_feed_ids") != Some(true) {
        return IdsToWrite::Lines(Vec::new());
    }
    let lines: Vec<(PageId, String)> = (known.posts.iter())
        .filter(|&&post| site.pages.get(post).made.is_none())
        .filter_map(|&post| {
            let id = match known.by.contains_key(&post) {
                true => known.moved(site, settings, dates, post)?.1.to_owned(),
                false => {
                    if site.pages.get(post).words.get(PageWord::Id).is_some()
                        || kept_id(site, settings, dates, post).is_some()
                    {
                        return None;
                    }
                    let now = entry_id(site, settings, dates, post)?;
                    now.strip_prefix(MADE)?.to_owned()
                }
            };
            Some((post, format!("id: {id}")))
        })
        .collect();
    match known.recorded || lines.is_empty() {
        true => IdsToWrite::Lines(lines),
        false => IdsToWrite::NoRecord,
    }
}
