//! Where each list's feed is published, what builds it, and what is said
//! of a feed that isn't.
//!
//! A list has a feed only where a `feed:` line asks: on the page that
//! speaks for it, handed down to that page, or for each language's own
//! list, the site's `feed:` setting, which the page's own line wins over.
//! The feed sits beside that page, as `feed.xml`, unless the line gives it
//! another name, or an address of its own from the top of the site. A line
//! naming several has a feed at each, the first being the list's feed that
//! templates are handed to link. A feed needs the site's address, its
//! `feed_key` and its `title` in the list's language, and a list with no
//! dated pages has nothing to carry, so has none.
//!
//! Statac ships no feed of its own: each is built with the site's template
//! named as the feed's file, `templates/rss.xml` for `rss.xml`, or else the
//! one named by its extension, `templates/feed.json` for `/feed.json`, or
//! else `templates/feed.xml`. A page of the author's standing at a feed's
//! address is published in its place.

pub(crate) mod format;
mod ids;
pub(crate) mod known;
#[cfg(test)]
mod tests;

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::config::Settings;
use crate::content::address::Address;
use crate::content::words::PageWord;
use crate::origin::{Arrival, Origin, Sourced};
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

/// The names a `feed:` value gives a list's feeds, in order, each as
/// written with where it is written: `true` names `feed.xml`, written
/// nowhere, and `false` names none.
fn names(feed: &Sourced<Value>) -> Vec<(&str, Option<&Origin>)> {
    match &feed.value {
        Value::Bool(true) => vec![(FILE_NAME, None)],
        Value::Text(written) => vec![(written.as_str(), Some(&feed.origin))],
        Value::Texts(written) => (written.iter())
            .map(|name| (name.value.as_str(), Some(&name.origin)))
            .collect(),
        _ => Vec::new(),
    }
}

/// Whether `list` asks for a feed, whether or not it can have one.
fn asks(site: &Site, settings: &Settings, list: ListId) -> bool {
    asked(site, settings, list)
        .and_then(|asked| asked.feed)
        .is_some_and(|feed| !names(feed).is_empty())
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

/// Each feed `list` would publish were the site to have everything a feed
/// needs, in the order its `feed:` names them: where, from the top of the
/// site and without the site's own path, and where the name giving that is
/// written, where one is. None for a name that gives no address, or one
/// given by a name before it.
fn wanting(site: &Site, settings: &Settings, list: ListId) -> Vec<(Address, Option<Place>)> {
    match pageless(site, list) {
        Some(_) => Vec::new(),
        None => asking(site, settings, list),
    }
}

/// Why `list` has no feed for want of a page: it has none, or its page
/// isn't published. A feed links to the list's page, and is known by it.
fn pageless(site: &Site, list: ListId) -> Option<Unwritten> {
    match site.lists.get(list).page.map(|page| site.pages.get(page)) {
        None => Some(Unwritten::NoPage),
        Some(page) if !page.lands() => Some(Unwritten::PageUnpublished),
        Some(_) => None,
    }
}

/// Each feed `list` asks for, as [`wanting`] gives them, whether or not
/// the list has a page published.
fn asking(site: &Site, settings: &Settings, list: ListId) -> Vec<(Address, Option<Place>)> {
    let Some(asked) = asked(site, settings, list) else {
        return Vec::new();
    };
    let carries =
        (site.lists.get(list).members.iter()).any(|&member| site.pages.get(member).date.is_some());
    let (Some(feed), true) = (asked.feed, carries) else {
        return Vec::new();
    };
    let Some(at) = standing(site, list, asked.page) else {
        return Vec::new();
    };
    let mut feeds: Vec<(Address, Option<Place>)> = Vec::new();
    for (written, origin) in names(feed) {
        let Ok(address) = written_address(&at, written) else {
            continue;
        };
        if feeds.iter().all(|(was, _)| was.key() != address.key()) {
            feeds.push((address, origin.and_then(|origin| origin.place)));
        }
    }
    feeds
}

/// Where each of `list`'s feeds would be published were the site to have
/// everything a feed needs, as [`wanting`] gives them.
pub(crate) fn wanted(site: &Site, settings: &Settings, list: ListId) -> Vec<Address> {
    (wanting(site, settings, list).into_iter())
        .map(|(address, _)| address)
        .collect()
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

/// Where `list`'s feed is published, the first its `feed:` names, from the
/// top of the site and without the site's own path; none where the list
/// has no feed, or the site lacks something every feed needs.
pub(crate) fn address(site: &Site, settings: &Settings, list: ListId) -> Option<Address> {
    if lacks(site, settings, list).is_some() {
        return None;
    }
    wanted(site, settings, list).into_iter().next()
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
        let Some(at) = standing(site, list, asked.page) else {
            continue;
        };
        for (written, origin) in names(feed) {
            let Some(origin) = origin else {
                continue;
            };
            let Err((fault, span)) = written_address(&at, written) else {
                continue;
            };
            let Some(place) = crate::value::place_within(sources, origin, written, span) else {
                continue;
            };
            if told.contains(&place) {
                continue;
            }
            told.push(place);
            problems.push(Problem::at(Kind::UnusableUrl { fault }, place));
        }
    }
    problems
}

/// Each name in a `feed:` line giving the address a name before it gives,
/// whose feed is published once, told once where it is written.
pub(crate) fn twice(site: &Site, settings: &Settings) -> Vec<Problem> {
    let mut told = Vec::new();
    let mut problems = Vec::new();
    for (list, _) in site.lists.iter() {
        let Some(asked) = asked(site, settings, list) else {
            continue;
        };
        let Some(feed) = asked.feed else {
            continue;
        };
        let Some(at) = standing(site, list, asked.page) else {
            continue;
        };
        let mut named: Vec<(Address, &str, Place)> = Vec::new();
        for (written, origin) in names(feed) {
            let (Ok(address), Some(place)) = (
                written_address(&at, written),
                origin.and_then(|origin| origin.place),
            ) else {
                continue;
            };
            let Some((_, first_written, first)) =
                named.iter().find(|(was, ..)| was.key() == address.key())
            else {
                named.push((address, written, place));
                continue;
            };
            if told.contains(&place) {
                continue;
            }
            told.push(place);
            let kind = Kind::FeedWrittenTwice {
                written: written.to_owned(),
                first_written: (*first_written).to_owned(),
                first: *first,
                address: format!("{}{}", settings.base(), address.as_str()),
            };
            problems.push(Problem::at(kind, place));
        }
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
        let at = standing(site, list, asked.page);
        let usable =
            |written: &str| (at.as_ref()).is_some_and(|at| written_address(at, written).is_ok());
        if !(names(feed).into_iter()).any(|(written, origin)| origin.is_none() || usable(written)) {
            continue;
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

/// The templates a feed at `address` may be built with, the first the site
/// has building it: the one named as its file, as `rss.xml`, then the one
/// named by its extension, as `feed.json`, then `feed.xml`. A file with no
/// extension, or ending `.html`, would name a page's template, so looks for
/// none of its own.
fn looked_for(address: &Address) -> Vec<String> {
    let last = address.as_str().rsplit('/').next().unwrap_or_default();
    let extension = (last.rsplit_once('.'))
        .map(|(_, extension)| extension)
        .filter(|extension| !matches!(*extension, "html" | "htm"));
    let mut names = Vec::new();
    names.extend(extension.map(|_| last.to_owned()));
    names.extend(
        extension
            .filter(|&extension| extension != "xml")
            .map(|extension| format!("feed.{extension}")),
    );
    names.push(FILE_NAME.to_owned());
    names.dedup();
    names
}

/// The template a feed at `address` is built with, as [`looked_for`]
/// gives them; none where the site has none of them. `has` says whether
/// the site has a template of a name.
pub(crate) fn template(address: &Address, has: &dyn Fn(&str) -> bool) -> Option<String> {
    looked_for(address).into_iter().find(|name| has(name))
}

/// The names of the templates the site's feeds are built with, among
/// those `has` says the site has, whether or not it has everything a feed
/// needs.
pub(crate) fn templates(
    site: &Site,
    settings: &Settings,
    has: &dyn Fn(&str) -> bool,
) -> BTreeSet<String> {
    (site.lists.iter())
        .flat_map(|(list, _)| wanted(site, settings, list))
        .filter_map(|address| template(&address, has))
        .collect()
}

/// Every feed the site publishes, in the order of its lists and each
/// list's in the order its `feed:` names them, and what is said of each
/// feed a list asks for that isn't: once for the site where the site lacks
/// what every feed needs, and once for each template it lacks; once for
/// each feed a page of the author's stands in the way of.
pub(crate) fn plan(
    site: &Site,
    settings: &Settings,
    sources: &Sources,
    has: &dyn Fn(&str) -> bool,
) -> (Vec<Feed>, Vec<Problem>) {
    let mut problems = Vec::new();
    // The templates looked for, and the lists whose feeds look for them.
    let mut untemplated: Vec<(Vec<String>, Vec<ListId>)> = Vec::new();
    // The lists asking for a feed with no page published, by why.
    let mut pageless_lists: Vec<(Unwritten, Vec<ListId>)> = Vec::new();
    let mut fed: Vec<(ListId, Address, String, Option<Place>)> = Vec::new();
    for (list, _) in site.lists.iter() {
        if let Some(why) = pageless(site, list)
            && !asking(site, settings, list).is_empty()
        {
            match pageless_lists.iter_mut().find(|(was, _)| *was == why) {
                Some((_, lists)) => lists.push(list),
                None => pageless_lists.push((why, vec![list])),
            }
            continue;
        }
        for (address, placed) in wanting(site, settings, list) {
            if let Some(template) = template(&address, has) {
                fed.push((list, address, template, placed));
                continue;
            }
            let looked = looked_for(&address);
            match untemplated.iter_mut().find(|(was, _)| *was == looked) {
                Some((_, lists)) if lists.last() == Some(&list) => {}
                Some((_, lists)) => lists.push(list),
                None => untemplated.push((looked, vec![list])),
            }
        }
    }
    for (looked, lists) in untemplated {
        let looked = looked.iter().map(|name| format!("templates/{name}"));
        let why = Unwritten::NoTemplate(looked.collect());
        problems.push(not_written(site, sources, &lists, why));
    }
    for (why, lists) in pageless_lists {
        problems.push(not_written(site, sources, &lists, why));
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
                more: lists_of(&fed).len() - 1,
                feeds: fed.len(),
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
        let untitled = lists_of(&untitled);
        problems.push(not_written(site, sources, &untitled, Unwritten::NoTitle));
    }
    let mut landed: HashMap<String, PageId> = HashMap::new();
    for (id, page) in site.pages.iter().filter(|(_, page)| page.lands()) {
        landed.entry(page.address.key()).or_insert(id);
    }
    let mut feeds = Vec::new();
    for (list, address, template, placed) in fed {
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
            placed,
        });
    }
    (feeds, problems)
}

/// The warning for `feed`, built as `text`, where its file name says one
/// format and it is written as another; none where they agree, or where
/// either says none.
pub(crate) fn mismatch(feed: &Feed, text: &str) -> Option<Problem> {
    let file = feed.address.as_str().rsplit('/').next().unwrap_or_default();
    let named = format::named(file)?;
    let built = format::of(text)?;
    if named == built {
        return None;
    }
    let own = looked_for(&feed.address).swap_remove(0);
    let kind = Kind::FeedFormatMismatch {
        file: file.to_owned(),
        template: format!("templates/{}", feed.template),
        own: format!("templates/{own}"),
        named,
        built,
    };
    Some(match feed.placed {
        Some(place) => Problem::at(kind, place),
        None => Problem::nowhere(kind),
    })
}

/// The lists of `fed`, each once, in order; a list's feeds are side by
/// side.
fn lists_of(fed: &[(ListId, Address, String, Option<Place>)]) -> Vec<ListId> {
    let mut lists: Vec<ListId> = Vec::new();
    for &(list, ..) in fed {
        if lists.last() != Some(&list) {
            lists.push(list);
        }
    }
    lists
}

/// The note for a site with a feed template where no list asks for a
/// feed, so that the template builds nothing: told once, of `feed.xml`
/// where the site has it, among `templates`, the names of the site's, and
/// `written_as_feeds`, those whose text is a feed whatever their names.
pub(crate) fn unasked(
    site: &Site,
    settings: &Settings,
    templates: &HashSet<String>,
    written_as_feeds: &BTreeSet<String>,
) -> Option<Problem> {
    let mut named: Vec<&String> = templates
        .iter()
        .filter(|name| {
            written_as_feeds.contains(*name)
                || name.strip_prefix("feed.").is_some_and(|extension| {
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
