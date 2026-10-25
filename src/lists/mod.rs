//! Folders, tags, series, the lists pages declare and the pages made from
//! a table, filled and put in order.
//!
//! A post is a published page with a date that speaks for no list. Each
//! list holds its posts in its order, and apart from them the pages it
//! would hold were they dated. A folder lists every post beneath it,
//! however deeply filed. A folder put in order by hand, by the numbers its
//! pages' names open with or by an `order:` list in its `index.md`, lists
//! every page in it in that order, then the posts in the folders inside.
//!
//! Every list is in one language. A language's pages stand at its top:
//! its folder at the top of `content/`, or for the main language with
//! none, `content/` itself. Folder lists stop there, so a folder holds only
//! its own language's pages, and the folder at a language's top is its own
//! list, what `site.posts` holds. Tags and series are a language's own, and
//! stand beneath its top.
//!
//! Whatever page stands at a list's address speaks for it. A tag or a
//! series no page speaks for has a page made for it where the site says
//! `tags.pages: true` or `series.pages: true`, and a language's own list
//! where the site has `templates/home.html`. A folder has a page only
//! where it has an `index.md`. A list whose page sets `posts_per_page`, or
//! the site's, goes on onto pages of its own beneath it.

mod chain;
mod filter;
mod paging;
#[cfg(test)]
mod tests;

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};

pub(crate) use filter::{DATE_PARTS, Filter, in_site_order, related, tags_of};

use crate::config::Settings;
use crate::content::address::{Address, Top};
use crate::content::slug;
use crate::content::words::{PageWord, Words};
use crate::dates::Dates;
use crate::origin::{Arrival, Sourced};
use crate::problem::{Kind, Problem, Word};
use crate::site::{Id, List, ListId, ListKey, Page, PageId, PagePlace, Site};
use crate::source::{Place, Sources, Span};
use crate::value::Value;

/// Which way a list's posts run.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Order {
    NewestFirst,
    OldestFirst,
}

impl Order {
    /// The order as `order:` writes it.
    pub fn word(self) -> &'static str {
        match self {
            Order::NewestFirst => "newest first",
            Order::OldestFirst => "oldest first",
        }
    }
}

/// The two kinds of list a page names by writing them: tags and series.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Named {
    Tag,
    Series,
}

impl Named {
    fn key(self, name: String) -> ListKey {
        match self {
            Named::Tag => ListKey::Tag(name),
            Named::Series => ListKey::Series(name),
        }
    }

    fn word(self) -> Word {
        match self {
            Named::Tag => Word::Tag,
            Named::Series => Word::Series,
        }
    }

    fn setting(self, setting: &str) -> String {
        match self {
            Named::Tag => format!("tags.{setting}"),
            Named::Series => format!("series.{setting}"),
        }
    }

    /// The names a page writes for lists of this kind, each with where it
    /// is written.
    fn written(self, page: &Page) -> Vec<(&str, Option<Place>)> {
        match self {
            Named::Tag => page
                .words
                .get(PageWord::Tags)
                .and_then(|tags| tags.value.as_texts())
                .unwrap_or_default()
                .iter()
                .map(|tag| (tag.value.as_str(), tag.origin.place))
                .collect(),
            Named::Series => page
                .words
                .text(PageWord::Series)
                .map(|series| (series.value, series.origin.place))
                .into_iter()
                .collect(),
        }
    }
}

/// Fills and orders every list of the site, and makes the list pages the
/// site asks for that no page of its own is: as `has` says which templates
/// the site has.
pub(crate) fn build(
    site: &mut Site,
    settings: &Settings,
    dates: &Dates,
    sources: &Sources,
    has: &dyn Fn(&str) -> bool,
) -> Vec<Problem> {
    let mut problems = Vec::new();
    let declared = declare(site);
    let mut tags_and_series = Vec::new();
    for named in [Named::Tag, Named::Series] {
        tags_and_series.extend(named_lists(site, settings, named, &mut problems));
    }
    speakers_at_addresses(site, sources, &mut problems);
    made_pages(site, settings, &tags_and_series, has);
    fill(site);
    let moments: Vec<Option<i64>> = site
        .pages
        .iter()
        .map(|(_, page)| moment(dates, page))
        .collect();
    let ids: Vec<ListId> = site.lists.iter().map(|(id, _)| id).collect();
    for &id in ids.iter().filter(|id| !declared.contains(id)) {
        order(site, &moments, id, &mut problems);
    }
    for id in declared {
        run(site, sources, id, &mut problems);
        order(site, &moments, id, &mut problems);
    }
    problems.extend(paging::settle(site, settings, sources));
    problems
}

/// A list for each page naming its posts with `posts:`, which that page
/// speaks for.
fn declare(site: &mut Site) -> Vec<ListId> {
    let declaring: Vec<PageId> = site
        .pages
        .iter()
        .filter(|(_, page)| {
            !page.sends_on()
                && page
                    .words
                    .text(PageWord::Posts)
                    .is_some_and(|posts| posts.origin.arrival == Arrival::Written)
        })
        .map(|(id, _)| id)
        .collect();
    declaring
        .into_iter()
        .map(|page| {
            let language = site.pages.get(page).language;
            let list = site.lists.push(List {
                page: Some(page),
                ..List::new(ListKey::Declared(page), language)
            });
            site.pages.get_mut(page).speaks_for = Some(list);
            list
        })
        .collect()
}

/// One way a tag or a series is spelt, and the pages that spell it so.
struct Spelling<'a> {
    name: &'a str,
    pages: Vec<PageId>,
    /// Where it is first written.
    first: Option<Place>,
}

/// A list for each tag or series, spellings whose address parts come out
/// alike made one under the commoner, with a warning naming each.
fn named_lists(
    site: &mut Site,
    settings: &Settings,
    named: Named,
    problems: &mut Vec<Problem>,
) -> Vec<ListId> {
    let slugs = settings.get(&named.setting("slugs"));
    let mut groups: BTreeMap<(usize, String), Vec<Spelling>> = BTreeMap::new();
    for (id, page) in site.pages.iter() {
        if page.speaks_for_a_list() || page.sends_on() {
            continue;
        }
        for (name, place) in named.written(page) {
            let group = groups
                .entry((page.language.index(), address_part(name, slugs)))
                .or_default();
            match group.iter_mut().find(|spelling| spelling.name == name) {
                Some(spelling) if spelling.pages.last() == Some(&id) => {}
                Some(spelling) => spelling.pages.push(id),
                None => group.push(Spelling {
                    name,
                    pages: vec![id],
                    first: place,
                }),
            }
        }
    }
    let pattern = settings
        .text(&named.setting("permalink"))
        .map(|pattern| pattern.map(str::to_owned));
    let mut made = Vec::new();
    let mut lists = Vec::new();
    for ((language, part), mut spellings) in groups {
        let top = Top::new(site.top(Id::from_index(language)));
        // The commoner spelling; of two as common, the first in the order
        // of their characters, which puts capitals first.
        spellings.sort_by(|a, b| {
            b.pages
                .len()
                .cmp(&a.pages.len())
                .then_with(|| a.name.cmp(b.name))
        });
        let chosen = &spellings[0];
        if spellings.len() > 1 {
            let at = spellings[1..]
                .iter()
                .min_by_key(|spelling| spelling.pages[0])
                .and_then(|spelling| spelling.first);
            let kind = Kind::SpelledTwoWays {
                word: named.word(),
                spellings: spellings
                    .iter()
                    .map(|spelling| (spelling.name.to_owned(), spelling.pages.len()))
                    .collect(),
                chosen: chosen.name.to_owned(),
            };
            problems.push(match at {
                Some(place) => Problem::at(kind, place),
                None => Problem::nowhere(kind),
            });
        }
        let address = pattern
            .as_ref()
            .filter(|_| !part.is_empty())
            .and_then(|pattern| {
                let written = top.pattern(&pattern.value, pattern.origin.place, problems);
                let address = Address::read(&top.address(&written.replace("{slug}", &part)));
                Some(Sourced::new(address.ok()?, pattern.origin.clone()))
            });
        lists.push((
            language,
            chosen.name.to_owned(),
            address,
            spellings
                .iter()
                .map(|spelling| spelling.name.to_owned())
                .collect::<Vec<_>>(),
        ));
    }
    for (language, name, address, spellings) in lists {
        let language = Id::from_index(language);
        let list = site.lists.push(List {
            address,
            ..List::new(named.key(name), language)
        });
        for spelling in spellings {
            site.spellings.insert((language, named.key(spelling)), list);
        }
        made.push(list);
    }
    made
}

/// The part of an address a tag or a series stands at: its own from the
/// `slugs` table, or else its name made a slug.
fn address_part(name: &str, slugs: Option<&Sourced<Value>>) -> String {
    if let Some(Sourced {
        value: Value::Table(rows),
        ..
    }) = slugs
        && let Some((_, given)) = rows
            .iter()
            .find(|(written, _)| written.value == name)
            .or_else(|| {
                rows.iter()
                    .find(|(written, _)| written.value.eq_ignore_ascii_case(name))
            })
    {
        return given.value.clone();
    }
    slug::slugify(name)
}

/// Where a folder's own page stands unless a `permalink` written on it
/// moves it.
fn folder_address(path: &str) -> Option<Address> {
    Address::read(&format!("/{}", slug::tidied_path(&format!("{path}/")))).ok()
}

/// Gives each list no page speaks for yet the page standing at its
/// address, where one does. A post standing there speaks for the list
/// rather than being one of its posts, which is said at its date. A page
/// sending readers on speaks for the list too, leaving its redirect there.
fn speakers_at_addresses(site: &mut Site, sources: &Sources, problems: &mut Vec<Problem>) {
    let mut at: HashMap<String, PageId> = HashMap::new();
    for (id, page) in site.pages.iter() {
        if !page.speaks_for_a_list() && (page.lands() || page.sends_on()) {
            at.entry(page.address.key()).or_insert(id);
        }
    }
    let wanted: Vec<(ListId, PageId)> = site
        .lists
        .iter()
        .filter(|(_, list)| list.page.is_none())
        .filter_map(|(id, list)| {
            let address = match &list.key {
                ListKey::Folder(path) => folder_address(path),
                ListKey::Tag(_) | ListKey::Series(_) => {
                    list.address.as_ref().map(|address| address.value.clone())
                }
                ListKey::Declared(_) => None,
            }?;
            at.get(&address.key()).map(|&page| (id, page))
        })
        .collect();
    for (list, page) in wanted {
        let p = site.pages.get(page);
        if p.speaks_for.is_some() {
            continue;
        }
        // Only the author's pages stand here yet: Statac makes its own after.
        if let Some(date) = p.date.as_ref().filter(|_| !p.sends_on())
            && let (Some(path), Some(head)) = (p.file_named(sources), p.head(sources))
        {
            let (word, name) = match &site.lists.get(list).key {
                ListKey::Tag(name) => (Word::Tag, name.clone()),
                ListKey::Series(name) => (Word::Series, name.clone()),
                ListKey::Folder(path) => (Word::Folder, path.clone()),
                ListKey::Declared(_) => continue,
            };
            let kind = Kind::NotAPost { path, word, name };
            // A date in the file's name is written nowhere in the file.
            let at = date.origin.place.unwrap_or(head);
            problems.push(Problem::at(kind, at));
        }
        site.lists.get_mut(list).page = Some(page);
        site.pages.get_mut(page).speaks_for = Some(list);
    }
}

/// A page for each tag and series no page speaks for, where `tags.pages:
/// true` or `series.pages: true` asks, filed in its language's own list;
/// and for each language's own list no page speaks for, where the site has
/// `templates/home.html`, as `has` says. It has no title, having no front
/// matter and no writing: templates call it by its list's name. It carries
/// what `pages.` lines hand every page.
fn made_pages(site: &mut Site, settings: &Settings, named: &[ListId], has: &dyn Fn(&str) -> bool) {
    let homes: Vec<(ListId, String)> = site
        .lists
        .iter()
        .filter(|_| has("home"))
        .filter(|(_, list)| list.page.is_none())
        .filter_map(|(id, list)| match &list.key {
            ListKey::Folder(path) if path == site.top(list.language) => Some((id, path.clone())),
            _ => None,
        })
        .collect();
    for (id, path) in homes {
        let inside = match path.is_empty() {
            true => String::new(),
            false => format!("{path}/"),
        };
        let holds_pages = site
            .pages
            .iter()
            .any(|(_, page)| page.path.starts_with(&inside));
        let (true, Some(address)) = (holds_pages, folder_address(&path)) else {
            continue;
        };
        made_page(site, settings, id, id, address);
    }
    for &id in named {
        let list = site.lists.get(id);
        let setting = match list.key {
            ListKey::Tag(_) => "tags.pages",
            ListKey::Series(_) => "series.pages",
            _ => continue,
        };
        let (Some(address), None) = (&list.address, list.page) else {
            continue;
        };
        if settings.truth(setting) != Some(true) {
            continue;
        }
        let Some(top) = site.own_list(list.language) else {
            continue;
        };
        let address = address.value.clone();
        made_page(site, settings, id, top, address);
    }
}

/// The notes `check` gives of the list pages nothing asks for, as `has`
/// says which templates the site has. Where the site has
/// `templates/folder.html`, which would build them: one for the folders
/// directly inside a language's top holding posts with no `index.md`, and
/// one each for the tags and the series where `tags.pages` or
/// `series.pages` isn't written. And one for the tops of languages holding
/// pages where nothing is published.
pub(crate) fn unasked(
    site: &Site,
    settings: &Settings,
    has: &dyn Fn(&str) -> bool,
) -> Vec<Problem> {
    let holds_pages = |path: &str| {
        let inside = match path.is_empty() {
            true => String::new(),
            false => format!("{path}/"),
        };
        site.pages
            .iter()
            .any(|(_, page)| page.path.starts_with(&inside))
    };
    let dated =
        |list: &List| (list.members.iter()).any(|&member| site.pages.get(member).date.is_some());
    let unspoken = site.lists.iter().filter(|(_, list)| list.page.is_none());
    let mut folders = Vec::new();
    let mut tops = Vec::new();
    let mut named: [Vec<String>; 2] = Default::default();
    for (id, list) in unspoken {
        match &list.key {
            ListKey::Folder(path) if !holds_pages(path) || folder_address(path).is_none() => {}
            ListKey::Folder(path) if path == site.top(list.language) => tops.push(path),
            ListKey::Folder(path) if site.outermost(id) && dated(list) => folders.push(path),
            ListKey::Tag(name) if list.address.is_some() => named[0].push(name.clone()),
            ListKey::Series(name) if list.address.is_some() => named[1].push(name.clone()),
            _ => {}
        }
    }
    let mut problems = Vec::new();
    let mut tell = |word: Word, mut names: Vec<String>| {
        if names.is_empty() {
            return;
        }
        let more = names.len() - 1;
        let list = (word, names.remove(0));
        problems.push(Problem::nowhere(Kind::NoListPage { list, more }));
    };
    if has("folder") {
        folders.sort_unstable();
        tell(Word::Folder, folders.into_iter().cloned().collect());
        for (named, names) in [Named::Tag, Named::Series].into_iter().zip(named) {
            let setting = settings.get(&named.setting("pages"));
            if setting.is_none_or(|setting| setting.origin.arrival == Arrival::Defaulted) {
                tell(named.word(), names);
            }
        }
    }
    if let Some(first) = tops.first() {
        let base = settings.base();
        let addresses = (tops.iter())
            .filter_map(|path| folder_address(path))
            .map(|address| format!("{base}{}", address.as_str()))
            .collect();
        let index = match first.is_empty() {
            true => "content/index.md".to_owned(),
            false => format!("content/{first}/index.md"),
        };
        problems.push(Problem::nowhere(Kind::NoHomePage { addresses, index }));
    }
    problems
}

/// Makes the page that speaks for `list`, filed in `folder`, with no
/// writing of its own.
fn made_page(site: &mut Site, settings: &Settings, list: ListId, folder: ListId, address: Address) {
    let mut words = Words::default();
    words.hand_down_from(&settings.pages);
    let page = site.pages.push(Page {
        source: None,
        language: site.lists.get(list).language,
        path: String::new(),
        place: PagePlace::Made,
        folder,
        words,
        title: None,
        date: None,
        address,
        body: Span::new(0, 0),
        speaks_for: Some(list),
        made: None,
    });
    site.lists.get_mut(list).page = Some(page);
}

/// The `order:` list a list's page writes itself, where it writes one.
fn order_list(site: &Site, list: &List) -> Option<Sourced<Vec<Sourced<String>>>> {
    let page = site.pages.get(list.page?);
    let order = page.words.get(PageWord::Order)?;
    if order.origin.arrival != Arrival::Written {
        return None;
    }
    let names = order.value.as_texts()?;
    Some(Sourced::new(names.to_vec(), order.origin.clone()))
}

/// Puts each post in its folders, tags and series, and each page with no
/// date beside them.
fn fill(site: &mut Site) {
    let folders: HashMap<String, ListId> = site
        .lists
        .iter()
        .filter_map(|(id, list)| match &list.key {
            ListKey::Folder(path) => Some((path.clone(), id)),
            _ => None,
        })
        .collect();
    let by_hand: HashSet<ListId> = site
        .lists
        .iter()
        .filter(|(_, list)| {
            matches!(list.key, ListKey::Folder(_))
                && (list.numbered || order_list(site, list).is_some())
        })
        .map(|(id, _)| id)
        .collect();
    let mut members: Vec<Vec<PageId>> = vec![Vec::new(); site.lists.len()];
    let mut undated: Vec<Vec<PageId>> = vec![Vec::new(); site.lists.len()];
    for (id, page) in site.pages.iter() {
        if page.speaks_for_a_list() || page.sends_on() {
            continue;
        }
        let post = page.date.is_some();
        let mut path = match &site.lists.get(page.folder).key {
            ListKey::Folder(path) => Some(path.as_str()),
            _ => None,
        };
        let top = path.map_or("", |path| site.top_above(path));
        while let Some(at) = path {
            if let Some(&list) = folders.get(at) {
                let own = list == page.folder && by_hand.contains(&list);
                match post || own {
                    true => members[list.index()].push(id),
                    false => undated[list.index()].push(id),
                }
            }
            path = match at {
                _ if at == top => None,
                _ => Some(at.rsplit_once('/').map_or("", |(parent, _)| parent)),
            };
        }
        // A page made from an item of a table is listed by the page that
        // made it, besides its folder.
        if let Some(made) = &page.made
            && let Some(list) = site.pages.get(made.by).speaks_for
        {
            match post {
                true => members[list.index()].push(id),
                false => undated[list.index()].push(id),
            }
        }
        for named in [Named::Tag, Named::Series] {
            for (name, _) in named.written(page) {
                let Some(list) = site.spelt(page.language, &named.key(name.to_owned())) else {
                    continue;
                };
                let into = match post {
                    true => &mut members[list.index()],
                    false => &mut undated[list.index()],
                };
                if into.last() != Some(&id) {
                    into.push(id);
                }
            }
        }
    }
    for ((list, members), undated) in members.into_iter().enumerate().zip(undated) {
        let list = site.lists.get_mut(Id::from_index(list));
        list.members = members;
        list.undated = undated;
    }
}

/// Fills a declared list with the posts its page's `posts:` line chooses.
fn run(site: &mut Site, sources: &Sources, id: ListId, problems: &mut Vec<Problem>) {
    let Some(page) = site.lists.get(id).page else {
        return;
    };
    let p = site.pages.get(page);
    let Some(written) = p.words.text(PageWord::Posts) else {
        return;
    };
    match chosen(site, sources, p, &written) {
        Ok(members) => site.lists.get_mut(id).members = members,
        Err(problem) => problems.push(problem),
    }
}

/// The posts a line `page` writes chooses from its language's own list, in
/// its order, once lists are filled; or the problem with the line, placed
/// in it.
pub(crate) fn chosen(
    site: &Site,
    sources: &Sources,
    page: &Page,
    written: &Sourced<&str>,
) -> Result<Vec<PageId>, Problem> {
    let posts = site
        .own_list(page.language)
        .map_or_else(Vec::new, |list| site.lists.get(list).members.clone());
    chain::members(site, page.language, written.value, &posts).map_err(|(kind, span)| {
        let place = crate::value::place_within(sources, &written.origin, written.value, span)
            .or_else(|| page.head(sources));
        Problem::placed(kind, place)
    })
}

/// The name a page goes by in a folder: its file's, or its folder's where
/// the folder is the page.
pub(crate) fn file_name(page: &Page) -> &str {
    let mut names = page.path.rsplit('/');
    let last = names.next().unwrap_or_default();
    match page.place {
        PagePlace::Bundle | PagePlace::Folder => names.next().unwrap_or(last),
        PagePlace::Ordinary | PagePlace::Made => last,
    }
}

/// The number a name opens with, as `03-day-one` does, with the rest of the
/// name. A name opening with a date opens with no number.
pub(crate) fn numbered(name: &str) -> Option<(u64, &str)> {
    let digits = name.len() - name.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    let rest = name[digits..].strip_prefix('-')?;
    if digits == 0 || rest.is_empty() || crate::date::in_name(name).is_some() {
        return None;
    }
    Some((name[..digits].parse().ok()?, rest))
}

/// Two pages A to Z by name, capitals aside until they are all that tells
/// them apart.
pub(crate) fn by_name(a: &Page, b: &Page) -> Ordering {
    a_to_z(file_name(a), file_name(b)).then_with(|| a.path.cmp(&b.path))
}

/// Two names A to Z, capitals aside until they are all that tells them
/// apart.
pub(crate) fn a_to_z(a: &str, b: &str) -> Ordering {
    a.to_lowercase()
        .cmp(&b.to_lowercase())
        .then_with(|| a.cmp(b))
}

/// Two posts in `order` by the moments their dates name, each given with
/// its moment in seconds; posts of one moment A to Z by name. A page with
/// no date goes after every page with one.
pub(crate) fn by_moment(
    order: Order,
    (a, at): (&Page, Option<i64>),
    (b, bt): (&Page, Option<i64>),
) -> Ordering {
    match (at, bt) {
        (Some(x), Some(y)) => match order {
            Order::OldestFirst => x.cmp(&y),
            Order::NewestFirst => y.cmp(&x),
        },
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
    .then_with(|| by_name(a, b))
}

/// The moment a page's date names, in seconds.
pub(crate) fn moment(dates: &Dates, page: &Page) -> Option<i64> {
    page.date.as_ref().map(|date| dates.instant(&date.value))
}

/// The last part of a page's address, which an `order:` list names it by.
fn address_name(page: &Page) -> &str {
    let path = page.address.as_str().trim_end_matches('/');
    path.rsplit('/').next().unwrap_or(path)
}

/// Puts a list's posts in its order, and its undated pages A to Z. Each
/// page's moment is given, by page.
fn order(site: &mut Site, moments: &[Option<i64>], id: ListId, problems: &mut Vec<Problem>) {
    let list = site.lists.get(id);
    let written = list
        .page
        .and_then(|page| site.pages.get(page).words.text(PageWord::Order));
    let direction = match written.as_ref().map(|order| order.value) {
        Some("oldest first") => Order::OldestFirst,
        Some("newest first") => Order::NewestFirst,
        _ if matches!(list.key, ListKey::Series(_)) => Order::OldestFirst,
        _ => Order::NewestFirst,
    };
    let page = |id: &PageId| site.pages.get(*id);
    let mut members = list.members.clone();
    members.sort_by(|&x, &y| {
        let (a, b) = (page(&x), page(&y));
        let number = |p: &Page| {
            (list.numbered && p.folder == id)
                .then(|| numbered(file_name(p)).map(|(n, _)| n))
                .flatten()
        };
        match (number(a), number(b)) {
            (Some(x), Some(y)) => x.cmp(&y).then_with(|| by_name(a, b)),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => by_moment(direction, (a, moments[x.index()]), (b, moments[y.index()])),
        }
    });
    let names = order_list(site, list);
    let dated = !list.numbered && names.is_none();
    if let Some(names) = names {
        members = by_hand(site, members, &names, problems);
    }
    let mut undated = list.undated.clone();
    undated.sort_by(|a, b| by_name(page(a), page(b)));
    let list = site.lists.get_mut(id);
    list.members = members;
    list.undated = undated;
    list.order = dated.then_some(direction);
}

/// `members` in the order an `order:` list names them, the pages it leaves
/// out after them in the order they had.
fn by_hand(
    site: &Site,
    mut members: Vec<PageId>,
    names: &Sourced<Vec<Sourced<String>>>,
    problems: &mut Vec<Problem>,
) -> Vec<PageId> {
    let mut placed = Vec::with_capacity(members.len());
    for name in &names.value {
        let found = members
            .iter()
            .position(|&member| address_name(site.pages.get(member)) == name.value);
        match found {
            Some(at) => placed.push(members.remove(at)),
            None => {
                let named_already = placed
                    .iter()
                    .any(|&member| address_name(site.pages.get(member)) == name.value);
                if named_already {
                    continue;
                }
                let known = members
                    .iter()
                    .map(|&member| address_name(site.pages.get(member)));
                let kind = Kind::UnknownWord {
                    word: Word::Page,
                    nearest: crate::suggest::nearest(&name.value, known).map(str::to_owned),
                    written: name.value.clone(),
                };
                let place = name.origin.place.expect("an order is written");
                problems.push(Problem::at(kind, place));
            }
        }
    }
    if !members.is_empty() {
        let kind = Kind::UnorderedPages {
            names: members
                .iter()
                .map(|&member| address_name(site.pages.get(member)).to_owned())
                .collect(),
        };
        let place = names.origin.place.expect("an order is written");
        problems.push(Problem::at(kind, place));
    }
    placed.extend(members);
    placed
}
