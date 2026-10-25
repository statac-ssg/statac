//! Where each list's feed is published, what builds it, and what is said
//! of a feed that isn't.
//!
//! A list has a feed only where a `feed:` line asks: on the page that
//! speaks for it, handed down to that page, or for each language's own
//! list, the site's `feed:` setting, which the page's own line wins over.
//! The feed sits beside that page, as `feed.xml`, unless the line gives it
//! another name, or an address of its own from the top of the site. A feed
//! needs the site's address, its `feed_key` and its `title` in the list's
//! language, and a list with no dated pages has nothing to carry, so has
//! none.
//!
//! Statac ships no feed of its own: each is built with the site's template
//! named by the feed's extension, `templates/feed.json` for `/feed.json`,
//! or else `templates/feed.xml`. A page of the author's standing at a
//! feed's address is published in its place.

mod ids;
pub(crate) mod known;
#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};

use crate::config::Settings;
use crate::content::address::Address;
use crate::content::words::PageWord;
use crate::origin::{Arrival, Sourced};
use crate::problem::{AddressFault, Kind, Named, Problem, Unwritten, Waiting, Word};
use crate::site::{ListId, ListKey, Page, PageId, Site};
use crate::source::{Place, Sources, Span};
use crate::value::Value;

pub(crate) use ids::{entry_id, mint_key};
pub(crate) use known::{IdsToWrite, Known, ids_to_write};

pub(crate) const FILE_NAME: &str = "feed.xml";

/// The site setting asking for each language's own list's feed.
pub(crate) const SETTING: &str = "feed";

/// The settings every feed needs.
pub(crate) const NEEDS: &[&str] = &["url", "feed_key", "title"];

/// A feed the build writes.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Feed {
    pub list: ListId,
    /// Where it is published, from the top of the site and without the
    /// site's own path.
    pub address: Address,
    /// The template it is built with, by its name.
    pub template: String,
    /// Where the `feed:` giving its address is written, where one does.
    pub placed: Option<Place>,
}

/// What a list's feed is asked by: the page speaking for it, where it has
/// one, and the `feed:` line the list goes by, where one is written.
struct Asked<'a> {
    page: Option<&'a Page>,
    feed: Option<&'a Sourced<Value>>,
}

/// The `feed:` line a list goes by: its page's own, else the site's
/// setting for a language's own list, else one handed down to the page,
/// or for a list no page speaks for, what `pages.` lines hand every page.
fn asked<'a>(site: &'a Site, settings: &'a Settings, id: ListId) -> Option<Asked<'a>> {
    let list = site.lists.get(id);
    let page = list.page.map(|page| site.pages.get(page));
    if (page.is_none() && !site.outermost(id)) || page.is_some_and(Page::sends_on) {
        return None;
    }
    let words = page.map_or(&settings.pages, |page| &page.words);
    let line = words.get(PageWord::Feed);
    let own = line.filter(|line| page.is_some() && line.origin.arrival == Arrival::Written);
    let own_list = matches!(&list.key, ListKey::Folder(path) if path == site.top(list.language));
    let setting = settings.get(SETTING).filter(|_| own_list);
    Some(Asked {
        page,
        feed: own.or(setting).or(line),
    })
}

/// Whether `list` asks for a feed, whether or not it can have one.
fn asks(site: &Site, settings: &Settings, list: ListId) -> bool {
    asked(site, settings, list)
        .and_then(|asked| asked.feed)
        .is_some_and(|feed| matches!(feed.value, Value::Bool(true) | Value::Text(_)))
}

/// Where a list's page stands, or a folder's would.
fn standing(site: &Site, list: ListId, page: Option<&Page>) -> Option<Address> {
    match (page, &site.lists.get(list).key) {
        (Some(page), _) => Some(page.address.clone()),
        (None, ListKey::Folder(path)) => Address::read(&format!(
            "/{}",
            crate::content::slug::tidied_path(&format!("{path}/"))
        ))
        .ok(),
        (None, _) => None,
    }
}

/// Where `list`'s feed would be published were the site to have everything
/// a feed needs: from the top of the site and without the site's own path.
/// None where the list has no feed, or its `feed:` gives no address.
pub(crate) fn wanted(site: &Site, settings: &Settings, list: ListId) -> Option<Address> {
    let asked = asked(site, settings, list)?;
    let (fed, written) = match asked.feed.map(|feed| &feed.value) {
        Some(Value::Bool(fed)) => (*fed, None),
        Some(Value::Text(written)) => (true, Some(written.as_str())),
        _ => (false, None),
    };
    let carries = site
        .lists
        .get(list)
        .members
        .iter()
        .any(|&member| site.pages.get(member).date.is_some());
    if !fed || !carries {
        return None;
    }
    let at = standing(site, list, asked.page)?;
    written_address(&at, written.unwrap_or(FILE_NAME)).ok()
}

/// Where the `feed:` giving `list`'s feed its address is written, where
/// one does.
fn placed(site: &Site, settings: &Settings, list: ListId) -> Option<Place> {
    let feed = asked(site, settings, list)?.feed?;
    match feed.value {
        Value::Text(_) => feed.origin.place,
        _ => None,
    }
}

/// The first of what every feed needs that `list`'s feed lacks: the site's
/// `url` and `feed_key`, and the `title` of the list's language.
fn lacks(site: &Site, settings: &Settings, list: ListId) -> Option<&'static str> {
    let language = site.lists.get(list).language;
    let tag = site.languages.get(language).tag.as_deref();
    let several = site.languages.len() > 1;
    NEEDS.iter().copied().find(|&name| match name {
        "title" if several => settings.text_for(tag, name).is_none(),
        _ => settings.text(name).is_none(),
    })
}

/// Where `list`'s feed is published, from the top of the site and without
/// the site's own path; none where the list has no feed, or the site lacks
/// something every feed needs.
pub(crate) fn address(site: &Site, settings: &Settings, list: ListId) -> Option<Address> {
    if lacks(site, settings, list).is_some() {
        return None;
    }
    wanted(site, settings, list)
}

/// The address a feed written as `written` takes for a list whose page
/// stands at `at`: as written where it opens with `/`, and beside the page
/// otherwise. A fault for a value that names no file, its span within the
/// value.
fn written_address(at: &Address, written: &str) -> Result<Address, (AddressFault, Span)> {
    let whole = Span::new(0, written.len());
    if written.starts_with('/') {
        let address = Address::read(written)?;
        return match address.is_file() {
            true => Ok(address),
            false => Err((
                AddressFault::FeedAtFolder {
                    nearest: as_a_file(written),
                },
                whole,
            )),
        };
    }
    let beside = |name: &str| match at.is_file() {
        true => format!("{}.{name}", at.as_str()),
        false => format!("{}{name}", at.as_str()),
    };
    if written.contains('/') {
        return Err((
            AddressFault::SlashBesidePage {
                nearest: as_a_file(&beside(written)),
            },
            whole,
        ));
    }
    // Read from the top of the site, so a fault's span is one along.
    let address = Address::read(&format!("/{written}")).map_err(|(fault, span)| {
        let start = (span.start as usize).saturating_sub(1);
        let end = (span.end as usize).saturating_sub(1);
        (fault, Span::new(start, end))
    })?;
    if !address.is_file() {
        return Err((
            AddressFault::FeedAtFolder {
                nearest: as_a_file(written),
            },
            whole,
        ));
    }
    Address::read_file(&beside(written)).map_err(|(fault, _)| (fault, whole))
}

/// The nearest spelling of `written` that names a file: `feed.xml` inside
/// a folder ending in `/`, and `.xml` after a name with no extension.
fn as_a_file(written: &str) -> String {
    if written.is_empty() || written.ends_with('/') {
        return format!("{written}{FILE_NAME}");
    }
    let last = written.rsplit('/').next().unwrap_or(written);
    match last.contains('.') {
        true => written.to_owned(),
        false => format!("{written}.xml"),
    }
}

/// Each `feed:` value a list goes by that names no file a feed can be
/// published at, told once where it is written.
pub(crate) fn unusable(site: &Site, settings: &Settings, sources: &Sources) -> Vec<Problem> {
    let mut told = Vec::new();
    let mut problems = Vec::new();
    for (list, _) in site.lists.iter() {
        let Some(asked) = asked(site, settings, list) else {
            continue;
        };
        let Some(feed) = asked.feed else {
            continue;
        };
        let Value::Text(written) = &feed.value else {
            continue;
        };
        let Some(at) = standing(site, list, asked.page) else {
            continue;
        };
        let Err((fault, span)) = written_address(&at, written) else {
            continue;
        };
        let Some(place) = crate::value::place_within(sources, &feed.origin, written, span) else {
            continue;
        };
        if told.contains(&place) {
            continue;
        }
        told.push(place);
        problems.push(Problem::at(Kind::UnusableUrl { fault }, place));
    }
    problems
}

/// Each `feed:` line asking for a feed of a list with nothing dated, which
/// has none, told once where it is written for the first such list.
pub(crate) fn undated(site: &Site, settings: &Settings, sources: &Sources) -> Vec<Problem> {
    let mut told = Vec::new();
    let mut problems = Vec::new();
    for (list, _) in site.lists.iter() {
        let Some(asked) = asked(site, settings, list) else {
            continue;
        };
        let Some(feed) = asked.feed else {
            continue;
        };
        let Some(place) = feed.origin.place else {
            continue;
        };
        if told.contains(&place) {
            continue;
        }
        // A value naming no file is told as that.
        match &feed.value {
            Value::Bool(true) => {}
            Value::Text(written)
                if standing(site, list, asked.page)
                    .is_some_and(|at| written_address(&at, written).is_ok()) => {}
            _ => continue,
        }
        let members = &site.lists.get(list).members;
        if members
            .iter()
            .any(|&page| site.pages.get(page).date.is_some())
        {
            continue;
        }
        told.push(place);
        let kind = Kind::FeedNotWritten {
            list: described(site, sources, list),
            more: 0,
            why: Unwritten::Undated,
        };
        problems.push(Problem::at(kind, place));
    }
    problems
}

/// The template a feed at `address` is built with: the one named by its
/// extension, as `feed.json`, where the site has it, or else `feed.xml`.
/// None where the site has neither. `has` says whether the site has a
/// template of a name.
pub(crate) fn template(address: &Address, has: &dyn Fn(&str) -> bool) -> Option<String> {
    let last = address.as_str().rsplit('/').next().unwrap_or_default();
    let own = last
        .rsplit_once('.')
        .map(|(_, extension)| extension)
        .filter(|extension| !matches!(*extension, "html" | "htm" | "xml"))
        .map(|extension| format!("feed.{extension}"));
    own.into_iter()
        .chain([FILE_NAME.to_owned()])
        .find(|name| has(name))
}

/// Every feed the site publishes, in the order of its lists, and what is
/// said of each feed a list asks for that isn't: once for the site where
/// the site lacks the template, or what every feed needs; once for each
/// feed a page of the author's stands in the way of.
pub(crate) fn plan(
    site: &Site,
    settings: &Settings,
    sources: &Sources,
    has: &dyn Fn(&str) -> bool,
) -> (Vec<Feed>, Vec<Problem>) {
    let mut problems = Vec::new();
    let mut untemplated = Vec::new();
    let mut fed: Vec<(ListId, Address, String)> = Vec::new();
    for (list, _) in site.lists.iter() {
        let Some(address) = wanted(site, settings, list) else {
            continue;
        };
        match template(&address, has) {
            Some(template) => fed.push((list, address, template)),
            None => untemplated.push(list),
        }
    }
    if !untemplated.is_empty() {
        let why = Unwritten::NoTemplate(format!("templates/{FILE_NAME}"));
        problems.push(not_written(site, sources, &untemplated, why));
    }
    let Some((first, ..)) = fed.first() else {
        return (Vec::new(), problems);
    };
    match NEEDS
        .iter()
        .find(|&&name| name != "title" && settings.text(name).is_none())
    {
        Some(&"url") => {
            let waiting = Waiting::Feeds {
                list: described(site, sources, *first),
                more: fed.len() - 1,
            };
            problems.push(Problem::nowhere(Kind::NoSiteUrl { waiting }));
            return (Vec::new(), problems);
        }
        Some(_) => {
            problems.push(Problem::nowhere(Kind::NoFeedKey { key: mint_key() }));
            return (Vec::new(), problems);
        }
        None => {}
    }
    // A site in several languages may title some of them alone.
    let (untitled, fed): (Vec<_>, Vec<_>) = fed
        .into_iter()
        .partition(|(list, ..)| lacks(site, settings, *list).is_some());
    if !untitled.is_empty() {
        let untitled: Vec<ListId> = untitled.into_iter().map(|(list, ..)| list).collect();
        problems.push(not_written(site, sources, &untitled, Unwritten::NoTitle));
    }
    let mut landed: HashMap<String, PageId> = HashMap::new();
    for (id, page) in site.pages.iter().filter(|(_, page)| page.lands()) {
        landed.entry(page.address.key()).or_insert(id);
    }
    let mut feeds = Vec::new();
    for (list, address, template) in fed {
        if let Some(&page) = landed.get(&address.key()) {
            let why = Unwritten::PageThere {
                page: Box::new(site.named(page, sources)),
                address: format!("{}{}", settings.base(), address.as_str()),
            };
            problems.push(not_written(site, sources, &[list], why));
            continue;
        }
        feeds.push(Feed {
            list,
            address,
            template,
            placed: placed(site, settings, list),
        });
    }
    (feeds, problems)
}

/// The note for a site with a feed template where no list asks for a
/// feed, so that the template builds nothing: told once, of `feed.xml`
/// where the site has it, among `templates`, the names of the site's.
pub(crate) fn unasked(
    site: &Site,
    settings: &Settings,
    templates: &HashSet<String>,
) -> Option<Problem> {
    let mut named: Vec<&String> = templates
        .iter()
        .filter(|name| {
            name.strip_prefix("feed.").is_some_and(|extension| {
                !extension.is_empty()
                    && !extension.contains('.')
                    && !matches!(extension, "html" | "htm")
            })
        })
        .collect();
    named.sort_by_key(|name| (name.as_str() != FILE_NAME, name.as_str()));
    let template = named.first()?;
    if site
        .lists
        .iter()
        .any(|(list, _)| asks(site, settings, list))
    {
        return None;
    }
    let line = match template.as_str() {
        FILE_NAME => "true".to_owned(),
        other => other.to_owned(),
    };
    Some(Problem::nowhere(Kind::UnusedFeedTemplate {
        template: format!("templates/{template}"),
        line: format!("{SETTING}: {line}"),
    }))
}

/// The feeds of `lists` not written, for `why`, named by the first list
/// and counting the rest.
fn not_written(site: &Site, sources: &Sources, lists: &[ListId], why: Unwritten) -> Problem {
    Problem::nowhere(Kind::FeedNotWritten {
        list: described(site, sources, lists[0]),
        more: lists.len() - 1,
        why,
    })
}

/// A list as a problem names it: a folder by its path, the site's own
/// where that is empty, a tag or a series by its name, a list a page
/// chooses by that page's file.
pub(crate) fn described(site: &Site, sources: &Sources, list: ListId) -> (Word, String) {
    match &site.lists.get(list).key {
        ListKey::Folder(path) => (Word::Folder, path.clone()),
        ListKey::Tag(name) => (Word::Tag, name.clone()),
        ListKey::Series(name) => (Word::Series, name.clone()),
        ListKey::Declared(page) => (
            Word::Page,
            site.pages
                .get(*page)
                .file_named(sources)
                .expect("a page declaring a list is the author's"),
        ),
    }
}

/// The claims each feed makes on an address, for finding two things
/// published at one: the file it writes, and the template it comes from.
pub(crate) fn claims(feeds: &[Feed]) -> Vec<(String, Named)> {
    feeds
        .iter()
        .map(|feed| {
            let template = format!("templates/{}", feed.template);
            (feed.address.file(), Named::File(template))
        })
        .collect()
}

/// The posts a feed carries, newest first: the list's dated posts, put in
/// order by the moments their dates name, whatever order the list keeps.
pub(crate) fn entries(site: &Site, dates: &crate::dates::Dates, list: ListId) -> Vec<PageId> {
    let mut posts: Vec<(PageId, Option<i64>)> = site
        .lists
        .get(list)
        .members
        .iter()
        .filter(|&&member| site.pages.get(member).date.is_some())
        .map(|&member| (member, crate::lists::moment(dates, site.pages.get(member))))
        .collect();
    posts.sort_by(|a, b| {
        crate::lists::by_moment(
            crate::lists::Order::NewestFirst,
            (site.pages.get(a.0), a.1),
            (site.pages.get(b.0), b.1),
        )
    });
    posts.into_iter().map(|(page, _)| page).collect()
}
