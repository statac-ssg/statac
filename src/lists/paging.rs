//! A list's posts shown a page at a time, where the pages after its first
//! stand, and the run each page's previous and next walk.
//!
//! A list shows `posts_per_page` posts on each page, as its page says or
//! else the site, `0` showing them all on one. Page 1 is the list's own
//! page; the rest stand where `pagination.permalink` puts them beneath it, built
//! through the same template.

use std::collections::HashMap;

use crate::address::Address;
use crate::config::Settings;
use crate::config::words::PageWord;
use crate::problem::{Kind, Problem};
use crate::site::{Id, ListId, ListKey, Page, PageId, Site};
use crate::source::Sources;

/// Where a list's later pages stand where the site doesn't say.
const LATER: &str = "{list}page/{n}/";

/// Settles how many posts each list's pages show, where its later pages
/// stand, and each page's previous and next. Gives the problem with each
/// later page an author's page already stands at, told at `pagination.permalink`
/// where the site sets it, or else at the page.
pub(super) fn settle(site: &mut Site, settings: &Settings, sources: &Sources) -> Vec<Problem> {
    let problems = pages(site, settings, sources);
    walks(site);
    problems
}

fn pages(site: &mut Site, settings: &Settings, sources: &Sources) -> Vec<Problem> {
    let per_site = settings
        .whole("posts_per_page")
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or_default();
    let written = settings.text("pagination.permalink");
    let setting = written.as_ref().and_then(|later| later.origin.place);
    let pattern = written.map_or_else(|| LATER.to_owned(), |later| later.value.to_owned());
    let after = pattern.strip_prefix("{list}").unwrap_or(&pattern);
    let base = settings.base();
    let mut landed: HashMap<String, PageId> = HashMap::new();
    for (id, page) in site.pages.iter() {
        if page.lands() {
            landed.entry(page.address.file()).or_insert(id);
        }
    }
    let ids: Vec<ListId> = site.lists.iter().map(|(id, _)| id).collect();
    for &id in &ids {
        let per_page = match site.lists.get(id).page.map(|page| site.pages.get(page)) {
            Some(page) if !writes_html(page) => 0,
            Some(page) => page
                .words
                .get(PageWord::PostsPerPage)
                .and_then(|per| per.value.as_whole())
                .and_then(|n| usize::try_from(n).ok())
                .unwrap_or(per_site),
            None => per_site,
        };
        site.lists.get_mut(id).per_page = per_page;
    }
    let mut problems = Vec::new();
    for id in ids {
        let list = site.lists.get(id);
        let Some(speaker) = list.page.filter(|&page| site.pages.get(page).lands()) else {
            continue;
        };
        let page = site.pages.get(speaker);
        let beneath = folder_of(&page.address);
        let mut later = Vec::new();
        for number in 2..=list.stretches() {
            let written = format!("{beneath}{}", after.replacen("{n}", &number.to_string(), 1));
            let address = Address::read(&written)
                .expect("a later page's address is read whole once the pattern is");
            if let Some(&other) = landed.get(&address.file())
                && other != speaker
            {
                let head = site.head(other, sources);
                let kind = Kind::LaterPageTaken {
                    list: format!("{base}{}", page.address.as_str()),
                    number,
                    page: Box::new(site.named(other, sources)),
                    page_at: setting.and(head),
                    address: format!("{base}{}", address.as_str()),
                };
                problems.push(Problem::placed(kind, setting.or(head)));
            }
            later.push(address);
        }
        site.lists.get_mut(id).later = later;
    }
    problems
}

/// Whether a page is published as HTML. A list whose page is published as
/// something else, such as a feed, shows every post on that one page.
fn writes_html(page: &Page) -> bool {
    if !page.address.is_file() {
        return true;
    }
    let path = page.address.as_str();
    let last = path.rsplit('/').next().unwrap_or(path);
    last.rsplit_once('.').is_some_and(|(_, extension)| {
        extension.eq_ignore_ascii_case("html") || extension.eq_ignore_ascii_case("htm")
    })
}

/// The folder a list's later pages stand beneath: its page's address, or
/// for a page at a file, the file's name as a folder, so `/journal.html`
/// pages beneath `/journal/`.
fn folder_of(address: &Address) -> String {
    let path = address.as_str();
    if !address.is_file() {
        return path.to_owned();
    }
    let (folder, last) = path.rsplit_once('/').unwrap_or(("", path));
    let stem = last.rsplit_once('.').map_or(last, |(stem, _)| stem);
    match stem.eq_ignore_ascii_case("index") {
        true => format!("{folder}/"),
        false => format!("{folder}/{stem}/"),
    }
}

/// Each page's previous and next in the run it walks, and each post's
/// place in its language's own list.
fn walks(site: &mut Site) {
    let folders: HashMap<&str, ListId> = site
        .lists
        .iter()
        .filter_map(|(id, list)| match &list.key {
            ListKey::Folder(path) => Some((path.as_str(), id)),
            _ => None,
        })
        .collect();
    let mut places: HashMap<(ListId, PageId), usize> = HashMap::new();
    for (id, list) in site.lists.iter() {
        if matches!(list.key, ListKey::Folder(_) | ListKey::Series(_)) {
            for (at, &member) in list.members.iter().enumerate() {
                places.insert((id, member), at);
            }
        }
    }
    let mut neighbours = vec![[None, None]; site.pages.len()];
    for (id, page) in site.pages.iter() {
        let Some((run, at)) = run_of(site, &folders, &places, id, page) else {
            continue;
        };
        let members = &site.lists.get(run).members;
        neighbours[id.index()] = [
            at.checked_sub(1).map(|before| members[before]),
            members.get(at + 1).copied(),
        ];
    }
    let mut in_site = vec![None; site.pages.len()];
    for (language, _) in site.languages.iter() {
        let Some(own) = site.own_list(language) else {
            continue;
        };
        for (at, &member) in site.lists.get(own).members.iter().enumerate() {
            in_site[member.index()] = Some(at);
        }
    }
    site.neighbours = neighbours;
    site.places = in_site;
}

/// The run a page's previous and next walk, and its place in it: the one
/// its `neighbours:` names, or else its series, or else the outermost
/// folder holding it beneath its language's top. A page speaking for a
/// list walks the list's pages instead, and a page in no run has none.
fn run_of(
    site: &Site,
    folders: &HashMap<&str, ListId>,
    places: &HashMap<(ListId, PageId), usize>,
    id: PageId,
    page: &Page,
) -> Option<(ListId, usize)> {
    if page.speaks_for_a_list() {
        return None;
    }
    let placed = |list: ListId| places.get(&(list, id)).map(|&at| (list, at));
    let series = || {
        let name = page.words.text(PageWord::Series)?;
        let list = site.spelt(page.language, &ListKey::Series(name.value.to_owned()))?;
        placed(list)
    };
    let folder = || {
        let path = match &site.lists.get(page.folder).key {
            ListKey::Folder(path) => path.as_str(),
            _ => return None,
        };
        let top = site.top_above(path);
        let mut inward: Vec<&str> = path
            .match_indices('/')
            .map(|(at, _)| &path[..at])
            .filter(|folder| folder.len() > top.len())
            .collect();
        inward.push(path);
        inward
            .into_iter()
            .find_map(|path| folders.get(path).and_then(|&list| placed(list)))
    };
    if page.words.truth(PageWord::Neighbours) == Some(false) {
        return None;
    }
    match page.words.text(PageWord::Neighbours).map(|n| n.value) {
        Some("site") => site.own_list(page.language).and_then(placed),
        Some("series") => series(),
        Some("folder") => folder(),
        _ => series().or_else(folder),
    }
}
