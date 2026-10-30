//! What each page's writing links to, read apart from building it, and
//! the pages linking to each page from their writing.
//!
//! A page's writing is otherwise built only where an output shows it, and
//! in a round that keeps most outputs most writing is never built at all.
//! So the links are found in the writing as Markdown reads it, without
//! its snippets, colouring or templates, by the same finder building it
//! uses, the first time anything asks, and only then. A page's links are
//! carried to a later reading of the site where its file, and all reading
//! it turned on, are unchanged, so a round reads again only what changed.
//!
//! A backlink is a link a reader follows from the writing: a Markdown
//! link, or `href` on `<a>` or `<area>` in the writing's HTML, `@url(...)`
//! in either, to the page or to an old address that sends readers on to
//! it, followed as the link check follows links. Pictures, links a
//! template writes, as a menu's, and values handed to a snippet call
//! don't count, nor does a page's link to itself.

#[cfg(test)]
mod tests;

use std::cell::RefCell;
use std::collections::HashMap;

use super::render::Shared;
use crate::ids::{Id, PageId};
use crate::linked::Writing;
use crate::links;
use crate::site::{Order, SentTo};

/// Fewer pages than this are read, or have their links followed, on the
/// thread asking.
const TOGETHER: usize = 32;

/// What every page's writing links to, and the pages linking to each.
#[derive(Debug)]
pub(crate) struct Linking {
    /// What each page's file links to, by page; none for a page with no
    /// file of its own.
    read: Vec<Option<Read>>,
    /// The pages linking to each page, each once, newest first, by page.
    to: Vec<Vec<PageId>>,
    /// How many pages' files were read, the rest carried from the reading
    /// before.
    #[cfg_attr(not(test), expect(dead_code, reason = "told in tests"))]
    fresh: usize,
}

/// What one page's file links to, and what reading it turned on besides
/// its text.
#[derive(Clone, Debug)]
pub(crate) struct Read {
    writing: Writing,
    /// Each snippet name asked whether it wraps the writing below a call,
    /// with the answer.
    wraps: Vec<(String, Option<bool>)>,
}

/// What a later reading of a site takes from an earlier one: each page's
/// links, by the later reading's page, where its file reads the same.
pub(crate) type Carried = Vec<Option<Read>>;

impl Linking {
    /// What `page`'s writing links to; nothing for a page with no file.
    pub(crate) fn writing(&self, page: PageId) -> Option<&Writing> {
        let read = self.read.get(page.index())?.as_ref()?;
        Some(&read.writing)
    }

    /// The pages linking to `page`, each once, newest first.
    pub(crate) fn to(&self, page: PageId) -> &[PageId] {
        self.to.get(page.index()).map_or(&[], Vec::as_slice)
    }

    #[cfg(test)]
    pub(crate) fn fresh(&self) -> usize {
        self.fresh
    }

    /// The links of every page of the site `shared` holds, each read from
    /// its file or taken from what `carried` holds of it.
    pub(super) fn of(shared: &Shared, mut carried: Carried) -> Linking {
        let site = &shared.site;
        let mut read: Vec<Option<Read>> = vec![None; site.pages.len()];
        let mut unread = Vec::new();
        for (page, held) in site.pages.iter() {
            if held.source.is_none() || held.made.is_some() {
                continue;
            }
            match carried.get_mut(page.index()).and_then(Option::take) {
                Some(kept) => read[page.index()] = Some(kept),
                None => unread.push(page),
            }
        }
        let fresh = unread.len();
        let found = each(&unread, |page| read_one(shared, page));
        for (page, found) in unread.into_iter().zip(found) {
            read[page.index()] = Some(found);
        }
        let to = linking_to(shared, &read);
        Linking { read, to, fresh }
    }

    /// What `now`, a later reading of the site this one is of, `was`, can
    /// carry of each page's links: those of each page whose file is read
    /// as it was here.
    pub(super) fn carried(&self, was: &Shared, now: &Shared) -> Carried {
        let mut carried: Carried = vec![None; now.site.pages.len()];
        if was.base != now.base {
            return carried;
        }
        let by_path: HashMap<&std::path::Path, PageId> = (was.site.pages.iter())
            .filter(|(page, _)| self.read.get(page.index()).is_some_and(Option::is_some))
            .filter_map(|(page, held)| Some((was.sources.path(held.source?), page)))
            .collect();
        for (page, held) in now.site.pages.iter() {
            let Some(source) = held.source.filter(|_| held.made.is_none()) else {
                continue;
            };
            let Some(&before) = by_path.get(now.sources.path(source)) else {
                continue;
            };
            let (kept, old) = (&self.read[before.index()], was.site.pages.get(before));
            let Some((kept, old_source)) = kept.as_ref().zip(old.source) else {
                continue;
            };
            let same = old.body == held.body
                && old.path == held.path
                && was.sources.text(old_source) == now.sources.text(source)
                && (kept.wraps.iter())
                    .all(|(name, answer)| super::render::wraps(now, name) == *answer);
            if same {
                carried[page.index()] = moved(kept, old_source, source);
            }
        }
        carried
    }
}

/// `read`, of the file read as `from`, as a reading reading it as `to`
/// has it; none where it holds a place in another file.
fn moved(read: &Read, from: crate::source::SourceId, to: crate::source::SourceId) -> Option<Read> {
    let mut read = read.clone();
    let writing = &mut read.writing;
    for linked in writing.written.iter_mut().chain(&mut writing.handed) {
        if linked.place.source != from {
            return None;
        }
        linked.place.source = to;
    }
    Some(read)
}

/// `f` of each of `pages`, in order: on threads of their own where there
/// are many, not the build's pool, since this runs inside a cell that a
/// worker of that pool, left waiting here, could otherwise ask again.
fn each<T: Send>(pages: &[PageId], f: impl Fn(PageId) -> T + Sync) -> Vec<T> {
    if pages.len() < TOGETHER {
        return pages.iter().map(|&page| f(page)).collect();
    }
    let threads = std::thread::available_parallelism().map_or(1, usize::from);
    let chunk = pages.len().div_ceil(threads).max(TOGETHER);
    let f = &f;
    std::thread::scope(|scope| {
        let parts: Vec<_> = (pages.chunks(chunk))
            .map(|part| scope.spawn(move || part.iter().map(|&page| f(page)).collect::<Vec<_>>()))
            .collect();
        (parts.into_iter())
            .flat_map(|part| {
                part.join()
                    .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
            })
            .collect()
    })
}

/// What `page`'s file links to, as Markdown reads its writing, or as HTML
/// for a page written in HTML.
fn read_one(shared: &Shared, page: PageId) -> Read {
    let held = shared.site.pages.get(page);
    let source = held.source.expect("a page with a file");
    let asked: RefCell<Vec<(String, Option<bool>)>> = RefCell::new(Vec::new());
    let writing = match super::render::written_as_markdown(shared, page) {
        Some(source) => {
            let wraps = |name: &str| {
                let answer = super::render::wraps(shared, name);
                asked.borrow_mut().push((name.to_owned(), answer));
                answer
            };
            let document =
                crate::markdown::read(&shared.sources, source, held.body, &wraps, &shared.base);
            Writing::of(&document, &shared.sources)
        }
        None => Writing::of_html(&shared.sources, source, held.body),
    };
    let mut wraps = asked.into_inner();
    wraps.sort_unstable();
    wraps.dedup();
    Read { writing, wraps }
}

/// The pages linking to each page from their writing, each once, newest
/// first, by page.
fn linking_to(shared: &Shared, read: &[Option<Read>]) -> Vec<Vec<PageId>> {
    let site = &shared.site;
    // The page each file the site publishes stands for: its own, or for an
    // old address, the page it sends readers on to, however many steps on.
    let mut standing: HashMap<String, PageId> = (site.pages.iter())
        .filter(|(_, held)| held.lands())
        .map(|(page, held)| (held.address.file(), page))
        .collect();
    let mut waiting: Vec<_> = (site.redirects.iter())
        .filter(|redirect| !standing.contains_key(&redirect.from.file()))
        .collect();
    loop {
        let before = waiting.len();
        waiting.retain(|redirect| {
            let to = match &redirect.to {
                SentTo::Page => Some(redirect.page),
                SentTo::Site { address, .. } => standing.get(&address.file()).copied(),
                SentTo::Elsewhere(_) => return false,
            };
            match to {
                Some(to) => {
                    standing.insert(redirect.from.file(), to);
                    false
                }
                None => true,
            }
        });
        if waiting.len() == before {
            break;
        }
    }
    let follow = links::Site::new(&shared.settings, standing.keys().cloned());
    let linking: Vec<PageId> = (site.pages.iter())
        .filter(|&(page, held)| held.lands() && read.get(page.index()).is_some_and(Option::is_some))
        .map(|(page, _)| page)
        .collect();
    let linked = each(&linking, |page| {
        let from = shared.address_of(page);
        let written = read[page.index()]
            .as_ref()
            .map_or(&[][..], |read| &read.writing.written);
        let followed = written.iter().filter(|linked| linked.followed);
        let mut targets: Vec<PageId> = (followed)
            .filter_map(|linked| standing.get(&follow.lands_on(&linked.address, &from)?.0))
            .copied()
            .filter(|&target| target != page)
            .collect();
        targets.sort_unstable();
        targets.dedup();
        targets
    });
    let mut to: Vec<Vec<PageId>> = vec![Vec::new(); site.pages.len()];
    for (&page, targets) in linking.iter().zip(linked) {
        for target in targets {
            to[target.index()].push(page);
        }
    }
    let moment = |page: PageId| crate::lists::moment(&shared.dates, site.pages.get(page));
    for pages in &mut to {
        pages.sort_by(|&a, &b| {
            let (x, y) = (site.pages.get(a), site.pages.get(b));
            crate::lists::by_moment(Order::NewestFirst, (x, moment(a)), (y, moment(b)))
        });
    }
    to
}
