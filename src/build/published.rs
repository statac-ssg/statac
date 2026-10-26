//! The addresses a build publishes, which `published.txt` keeps.

use std::collections::HashSet;

use crate::address::Address;
use crate::assets::Assets;
use crate::config::words::PageWord;
use crate::dates::Dates;
use crate::feeds::Feed;
use crate::ids::PageId;
use crate::site::{ListKey, Page, Site};

/// Every address of the author's `site` publishes, besides
/// the files beside the writing in `held_back`, each its path in the built
/// site, which no page links: each page from a file, each old address
/// sending readers on, the feed of each folder carrying a page of theirs,
/// and each file published as it is. A list's page Statac made, its pages
/// after the first, and a tag's or a series' feed come and go with what is
/// listed, and the site's assets are named by what they hold, so none of
/// them is kept.
pub(crate) fn now(
    site: &Site,
    dates: &Dates,
    assets: &Assets,
    feeds: &[Feed],
    held_back: &[String],
) -> Vec<Address> {
    // Built where the run brings them in, as serve's does, they are no
    // address of the site's yet.
    let left_out = |page: &Page| {
        let draft = (page.words.get(PageWord::Draft))
            .is_some_and(|draft| draft.value.as_bool() == Some(true));
        let waiting = page.speaks_for.is_none()
            && (page.date.as_ref()).is_some_and(|date| dates.after_today(&date.value));
        draft || waiting
    };
    let theirs: HashSet<PageId> = (site.pages.iter())
        .filter(|(_, page)| page.lands() && !left_out(page) && page.source.is_some())
        .map(|(id, _)| id)
        .collect();
    let pages = theirs.iter().map(|&id| site.pages.get(id).address.clone());
    let redirects = (site.redirects.iter())
        .filter(|redirect| !left_out(site.pages.get(redirect.page)))
        .map(|redirect| redirect.from.clone());
    let feeds = (feeds.iter())
        .filter(|feed| {
            let list = site.lists.get(feed.list);
            matches!(list.key, ListKey::Folder(_) | ListKey::Declared(_))
                && (list.members.iter().chain(&list.undated)).any(|id| theirs.contains(id))
        })
        .map(|feed| feed.address.clone());
    let files = (site.files.iter())
        .filter(|file| !held_back.contains(&file.address.file()))
        .filter(|file| !file.page.is_some_and(|page| left_out(site.pages.get(page))))
        .map(|file| file.address.clone());
    let statics = (assets.statics().iter())
        .filter_map(|file| Address::read_file(&format!("/{}", file.name)).ok());
    pages
        .chain(redirects)
        .chain(feeds)
        .chain(files)
        .chain(statics)
        .collect()
}
