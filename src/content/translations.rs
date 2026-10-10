//! Which pages are translations of one another.
//!
//! On a site in several languages, pages in two languages are translations
//! where they stand at the same path under their languages' tops, read
//! without the file's extension, so `about.md` matches `about.html`. A page
//! with a `translation_key` is linked by its key alone, on any site. A key
//! on a folder's own page pairs that folder with the folders in other
//! languages that share it, and the pages beneath paired folders match by
//! their path below them.
//!
//! Pages Statac makes match too: a folder's page as the folder's `index`
//! would, a tag's or a series' page by its address from its language's
//! top, and a page made by `each:` by the page that made it and what it
//! was made for. A page published nowhere is no page's translation.

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};

use crate::content::words::PageWord;
use crate::problem::{Kind, Problem, Translated};
use crate::site::{Id, Item, LanguageId, ListKey, PageId, PagePlace, Site};
use crate::source::{Place, Sources};

/// What a page is matched by in every language.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
enum Match {
    /// Its `translation_key`.
    Key(String),
    /// Its path under its language's top, without the file's extension.
    Place(String),
    /// Its path below the nearest folder above it whose own page has a
    /// key, without the file's extension, with that key.
    Paired(String, String),
    /// A tag's or a series' page Statac made, by its address from its
    /// language's top.
    Listed(&'static str, String),
    /// A page made by `each:`: the group of the page that made it, and what
    /// it was made for.
    Made(usize, Made),
}

/// What a page made by `each:` was made for, alike in every language.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
enum Made {
    Key(String),
    At(usize),
    /// A post, by its group.
    Post(usize),
}

/// The pages matched by one [`Match`], in the order they were found.
struct Group {
    by: Match,
    pages: Vec<PageId>,
    /// Whether two pages in one language were found in it and told.
    told: bool,
}

/// Links each page to its translations in `site.translations`, telling
/// what stops a page being linked: `keys_left_out` are the keys of pages
/// left out of this build.
pub(super) fn link(
    site: &mut Site,
    sources: &Sources,
    keys_left_out: &[(LanguageId, String)],
) -> Vec<Problem> {
    let several = site.languages.len() > 1;
    let keyed = |page: &crate::site::Page| page.words.text(PageWord::TranslationKey).is_some();
    if !several && !site.pages.iter().any(|(_, page)| keyed(page)) {
        return Vec::new();
    }
    // Each language's top as the path its pages' paths open with.
    let tops: Vec<String> = site
        .languages
        .iter()
        .map(|(id, _)| match site.top(id) {
            "" => String::new(),
            top => format!("{top}/"),
        })
        .collect();
    let paired = paired_folders(site);
    let places: Vec<Option<Match>> = site
        .pages
        .iter()
        .map(|(_, page)| {
            let top = &tops[page.language.index()];
            several
                .then(|| place_of(site, page, top, &paired))
                .flatten()
        })
        .collect();

    let mut groups: Vec<Group> = Vec::new();
    let mut found: HashMap<Match, usize> = HashMap::new();
    let mut group_of: Vec<Option<usize>> = vec![None; site.pages.len()];
    let mut join = |by: Match, page: PageId, group_of: &mut Vec<Option<usize>>| {
        let at = *found.entry(by.clone()).or_insert_with(|| {
            groups.push(Group {
                by,
                pages: Vec::new(),
                told: false,
            });
            groups.len() - 1
        });
        groups[at].pages.push(page);
        group_of[page.index()] = Some(at);
    };
    // A page made by `each:` is matched once the page that made it is.
    let mut made = Vec::new();
    for (id, page) in site.pages.iter() {
        if !page.lands() {
            continue;
        }
        if let Some(key) = page.words.text(PageWord::TranslationKey) {
            join(Match::Key(key.value.to_owned()), id, &mut group_of);
        } else if page.made.is_some() {
            made.push(id);
        } else if let Some(place) = &places[id.index()] {
            join(place.clone(), id, &mut group_of);
        }
    }
    for id in made {
        let Some(by) = &site.pages.get(id).made else {
            continue;
        };
        let item = match &by.item {
            Item::Key(key) => Some(Made::Key(key.clone())),
            Item::At(at) => Some(Made::At(*at)),
            Item::Post { page, .. } => group_of[page.index()].map(Made::Post),
        };
        if let (Some(maker), Some(item)) = (group_of[by.by.index()], item) {
            join(Match::Made(maker, item), id, &mut group_of);
        }
    }

    let mut problems = Vec::new();
    for group in &mut groups {
        problems.extend(one_each(site, sources, group));
    }
    problems.extend(unmatched(site, &groups, keys_left_out));
    problems.extend(not_linked(site, sources, &groups, &places));

    let mut translations = vec![Vec::new(); site.pages.len()];
    for group in &groups {
        if group.pages.len() < 2 {
            continue;
        }
        let mut pages = group.pages.clone();
        pages.sort_by_key(|&page| site.pages.get(page).language);
        for &page in &pages {
            translations[page.index()] = pages.iter().copied().filter(|&p| p != page).collect();
        }
    }
    if translations.iter().any(|pages| !pages.is_empty()) {
        site.translations = translations;
    }
    problems
}

/// Each folder whose own page has a `translation_key`, by its language and
/// its path under `content/`, with the key. Of two folders in one language
/// with one key, the first is paired.
fn paired_folders(site: &Site) -> HashMap<(LanguageId, String), String> {
    let mut paired = HashMap::new();
    let mut seen = HashSet::new();
    for (_, page) in site.pages.iter() {
        if page.place != PagePlace::Folder {
            continue;
        }
        let Some(key) = page.words.text(PageWord::TranslationKey) else {
            continue;
        };
        if seen.insert((page.language, key.value)) {
            let folder = folder_of(&page.path).to_owned();
            paired.insert((page.language, folder), key.value.to_owned());
        }
    }
    paired
}

/// The folder a path under `content/` is in, ending with `/`; empty at the
/// top.
fn folder_of(path: &str) -> &str {
    path.rfind('/').map_or("", |at| &path[..=at])
}

/// Where a page stands in its language, as its translations stand: by its
/// path under `top`, or for a list's page Statac made, by its list. None
/// for a page outside its language's top, which matches only by key.
fn place_of(
    site: &Site,
    page: &crate::site::Page,
    top: &str,
    paired: &HashMap<(LanguageId, String), String>,
) -> Option<Match> {
    let path = match page.place {
        PagePlace::Made => match &site.lists.get(page.speaks_for?).key {
            ListKey::Folder(folder) if folder.is_empty() => "index".to_owned(),
            ListKey::Folder(folder) => format!("{folder}/index"),
            ListKey::Tag(_) | ListKey::Series(_) => {
                let word = site.lists.get(page.speaks_for?).key.word();
                let address = page.address.as_str();
                let from_top = match address[1..].starts_with(top) {
                    // Kept from the `/` the top's own name ends with.
                    true => &address[top.len()..],
                    false => address,
                };
                return Some(Match::Listed(word, from_top.to_owned()));
            }
            ListKey::Declared(_) => return None,
        },
        PagePlace::Folder | PagePlace::Ordinary | PagePlace::Bundle => page.path.clone(),
    };
    let rel = path.strip_prefix(top)?;
    let name_at = rel.rfind('/').map_or(0, |at| at + 1);
    let stem = match rel[name_at..].rfind('.') {
        Some(dot) if dot > 0 => &rel[..name_at + dot],
        _ => rel,
    };
    // A folder's `index` stands where the folder does, as a page named for
    // the folder would: `blog/index.md` and `blog.md` are both `blog`, and
    // the top's own page is the empty place.
    let stem = match stem[name_at..].eq_ignore_ascii_case("index") {
        true => stem[..name_at].trim_end_matches('/'),
        false => stem,
    };
    // A folder's own page is paired with its folder by its own key, so the
    // folders above it pair it.
    let mut folder = folder_of(stem);
    // The top's own page pairs nothing: its key is the home page's.
    while !folder.is_empty() {
        if let Some(key) = paired.get(&(page.language, format!("{top}{folder}"))) {
            return Some(Match::Paired(key.clone(), stem[folder.len()..].to_owned()));
        }
        folder = folder_of(folder.trim_end_matches('/'));
    }
    Some(Match::Place(stem.to_owned()))
}

/// How a problem names a page: by its file, or a page Statac made by its
/// address.
fn named(site: &Site, sources: &Sources, page: PageId) -> String {
    let p = site.pages.get(page);
    p.file_named(sources)
        .unwrap_or_else(|| p.address.as_str().to_owned())
}

/// Where a page's `translation_key` is written, or else its first line.
fn key_place(site: &Site, sources: &Sources, page: PageId) -> Option<Place> {
    let key = site.pages.get(page).words.get(PageWord::TranslationKey);
    key.and_then(|key| key.origin.place)
        .or_else(|| site.head(page, sources))
}

/// Keeps the first page of each language in `group`, telling each other
/// one: always for a key, and for a place where a page in another language
/// stands there to be matched.
fn one_each(site: &Site, sources: &Sources, group: &mut Group) -> Vec<Problem> {
    let mut problems = Vec::new();
    let mut first_of: HashMap<LanguageId, PageId> = HashMap::new();
    let mut kept = Vec::new();
    let mut second = Vec::new();
    for &page in &group.pages {
        let language = site.pages.get(page).language;
        match first_of.get(&language) {
            Some(&first) => second.push((first, page)),
            None => {
                first_of.insert(language, page);
                kept.push(page);
            }
        }
    }
    let matched = first_of.len() > 1;
    for (first, page) in second {
        let by = match &group.by {
            Match::Key(key) => Translated::Key(key.clone()),
            _ if !matched => continue,
            // Already told as two pages at one address.
            _ if site.pages.get(first).address == site.pages.get(page).address => continue,
            _ => {
                let language = site.pages.get(page).language;
                let other = kept
                    .iter()
                    .copied()
                    .find(|&other| site.pages.get(other).language != language)
                    .expect("another language has a page here");
                Translated::Place(named(site, sources, other))
            }
        };
        let tag = site
            .languages
            .get(site.pages.get(page).language)
            .tag
            .clone();
        let kind = Kind::SameTranslation {
            tag: tag.unwrap_or_default(),
            by,
            first: key_place(site, sources, first),
        };
        problems.push(match key_place(site, sources, page) {
            Some(place) => Problem::at(kind, place),
            None => Problem::nowhere(kind),
        });
    }
    group.pages = kept;
    group.told = !problems.is_empty();
    problems
}

/// A warning for each `translation_key` no page in another language has,
/// nor a page in another language left out of this build, offering the
/// nearest such key in another language.
fn unmatched(
    site: &Site,
    groups: &[Group],
    keys_left_out: &[(LanguageId, String)],
) -> Vec<Problem> {
    let alone: Vec<(&str, PageId)> = groups
        .iter()
        .filter(|group| !group.told)
        .filter_map(|group| match (&group.by, group.pages.as_slice()) {
            (Match::Key(key), &[page]) => Some((key.as_str(), page)),
            _ => None,
        })
        .collect();
    let mut problems = Vec::new();
    for &(key, page) in &alone {
        let p = site.pages.get(page);
        if keys_left_out
            .iter()
            .any(|(language, left)| *language != p.language && left == key)
        {
            continue;
        }
        let Some(place) = p
            .words
            .get(PageWord::TranslationKey)
            .and_then(|key| key.origin.place)
        else {
            continue;
        };
        let others = alone
            .iter()
            .filter(|&&(_, other)| site.pages.get(other).language != p.language)
            .map(|&(other, _)| other);
        let nearest = crate::suggest::nearest(key, others).map(str::to_owned);
        let kind = Kind::UnmatchedTranslationKey {
            key: key.to_owned(),
            nearest,
        };
        problems.push(Problem::at(kind, place));
    }
    problems
}

/// A warning for each page of the author's with no key standing where a
/// page in another language with a key stands, that key linking no page
/// in its language.
fn not_linked(
    site: &Site,
    sources: &Sources,
    groups: &[Group],
    places: &[Option<Match>],
) -> Vec<Problem> {
    let mut at_place: HashMap<&Match, Vec<PageId>> = HashMap::new();
    for group in groups {
        if !matches!(group.by, Match::Key(_)) {
            at_place.entry(&group.by).or_default().extend(&group.pages);
        }
    }
    let mut problems = Vec::new();
    let mut told = HashSet::new();
    for group in groups {
        let Match::Key(key) = &group.by else {
            continue;
        };
        let languages: HashSet<LanguageId> = group
            .pages
            .iter()
            .map(|&page| site.pages.get(page).language)
            .collect();
        for &keyed in &group.pages {
            let Some(place) = &places[keyed.index()] else {
                continue;
            };
            let standing = at_place.get(place).into_iter().flatten();
            for &page in standing {
                let p = site.pages.get(page);
                if languages.contains(&p.language) || p.source.is_none() || !told.insert(page) {
                    continue;
                }
                let kind = Kind::NotLinkedByPlace {
                    keyed: named(site, sources, keyed),
                    key: key.clone(),
                };
                problems.push(match site.head(page, sources) {
                    Some(head) => Problem::at(kind, head),
                    None => Problem::nowhere(kind),
                });
            }
        }
    }
    problems
}
