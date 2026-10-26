//! The filters that choose posts: `tagged`, `in` and `where`. A template
//! and a page's `posts:` line both come here, so the two can never read a
//! filter differently. `related` chooses by a page's tags.

use std::cell::{OnceCell, RefCell};
use std::collections::{BTreeSet, HashSet};

use crate::config::words::PageWord;
use crate::ids::{Id, LanguageId, ListId, PageId};
use crate::problem::{Kind, Word};
use crate::site::{ListKey, Page, Site};
use crate::value::Value;
use crate::yaml::{Node, Value as Yaml};

/// The filters a `posts:` line can use, and how many values each takes.
pub(super) const CHAINABLE: &[(&str, usize)] = &[("tagged", 1), ("in", 1), ("where", 2)];

/// The parts of a page's date `where` answers for by name, where no page
/// writes a field of its own by that name.
pub(crate) const DATE_PARTS: &[&str] = &["year", "month", "day"];

/// One filter, its values read against the site: which posts it keeps.
#[derive(Debug)]
pub(crate) enum Filter {
    /// The pages carrying a tag, however it is spelt.
    Tagged(ListId),
    /// The pages filed in a folder, however deeply, by the folder's list,
    /// gathered when a page is first asked of.
    In(ListId, OnceCell<HashSet<PageId>>),
    /// The pages whose field answers to a value.
    Where { field: String, wanted: String },
}

impl Filter {
    /// `tagged(name)`: the tag's list, or the problem with the name.
    pub(crate) fn tagged(site: &Site, language: LanguageId, name: &str) -> Result<Filter, Kind> {
        let exact = site.spelt(language, &ListKey::Tag(name.to_owned()));
        let found = exact.or_else(|| {
            site.spellings
                .iter()
                .find(|((lang, key), _)| {
                    *lang == language
                        && matches!(key, ListKey::Tag(spelt) if spelt.eq_ignore_ascii_case(name))
                })
                .map(|(_, &list)| list)
        });
        match found {
            Some(list) => Ok(Filter::Tagged(list)),
            None => {
                let known = names_of(site, language, |key| matches!(key, ListKey::Tag(_)));
                Err(Kind::UnknownWord {
                    word: Word::Tag,
                    nearest: crate::suggest::nearest(name, known.iter().map(String::as_str))
                        .map(str::to_owned),
                    written: name.to_owned(),
                })
            }
        }
    }

    /// `in(folder)`: one of a language's folders, by its path under its
    /// top or under `content/`, or by its own name.
    pub(crate) fn in_folder(site: &Site, language: LanguageId, name: &str) -> Result<Filter, Kind> {
        let wanted = name.trim_matches('/');
        let top = site.top(language);
        let below = |path: &str| match top {
            "" => Some(path.to_owned()),
            top => path
                .strip_prefix(top)
                .and_then(|rest| rest.strip_prefix('/'))
                .map(str::to_owned),
        };
        let found = site.lists.iter().find(|(_, list)| {
            list.language == language
                && matches!(&list.key, ListKey::Folder(path)
                    if !path.is_empty()
                        && path != top
                        && (path == wanted
                            || below(path).as_deref() == Some(wanted)
                            || path.rsplit('/').next() == Some(wanted)))
        });
        match found {
            Some((id, _)) => Ok(Filter::In(id, OnceCell::new())),
            None => {
                let folders: Vec<&str> = site
                    .lists
                    .iter()
                    .filter(|(_, list)| list.language == language)
                    .filter_map(|(_, list)| match &list.key {
                        ListKey::Folder(path) if !path.is_empty() && path != top => {
                            Some(path.as_str())
                        }
                        _ => None,
                    })
                    .collect();
                Err(Kind::UnknownWord {
                    word: Word::Folder,
                    nearest: crate::suggest::nearest(wanted, folders.iter().copied())
                        .map(str::to_owned),
                    written: name.to_owned(),
                })
            }
        }
    }

    /// `where(field, value)`: a field some page writes, or a part of a
    /// page's date.
    pub(crate) fn where_field(site: &Site, field: &str, wanted: &str) -> Result<Filter, Kind> {
        let written = |page: &Page| match PageWord::named(field) {
            Some(PageWord::Title) => page.title.is_some(),
            Some(PageWord::Date) => page.date.is_some(),
            Some(word) => page.words.get(word).is_some(),
            None => page.words.field(field).is_some(),
        };
        if DATE_PARTS.contains(&field) || site.pages.iter().any(|(_, page)| written(page)) {
            return Ok(Filter::Where {
                field: field.to_owned(),
                wanted: wanted.to_owned(),
            });
        }
        let known: BTreeSet<&str> = site
            .pages
            .iter()
            .flat_map(|(_, page)| page.words.fields.iter().map(|f| f.name.as_str()))
            .chain(DATE_PARTS.iter().copied())
            .collect();
        Err(Kind::UnknownWord {
            word: Word::Field,
            nearest: crate::suggest::nearest(field, known.iter().copied()).map(str::to_owned),
            written: field.to_owned(),
        })
    }

    /// The field a page is read for when this filter asks of it.
    pub(crate) fn reads(&self) -> Option<&str> {
        match self {
            Filter::Tagged(_) => Some("tags"),
            Filter::In(..) => None,
            Filter::Where { field, .. } => Some(field),
        }
    }

    /// Whether the filter keeps `page`.
    pub(crate) fn keeps(&self, site: &Site, page: PageId) -> bool {
        let p = site.pages.get(page);
        match self {
            Filter::Tagged(list) => p
                .words
                .get(PageWord::Tags)
                .and_then(|tags| tags.value.as_texts())
                .unwrap_or_default()
                .iter()
                .any(|tag| site.spelt(p.language, &ListKey::Tag(tag.value.clone())) == Some(*list)),
            Filter::In(list, members) => members
                .get_or_init(|| {
                    let list = site.lists.get(*list);
                    list.members.iter().chain(&list.undated).copied().collect()
                })
                .contains(&page),
            Filter::Where { field, wanted } => answers(p, field, wanted),
        }
    }
}

/// The tags a page carries, each once, however it spells them.
pub(crate) fn tags_of(site: &Site, page: PageId) -> Vec<ListId> {
    let p = site.pages.get(page);
    let mut lists = Vec::new();
    for tag in p
        .words
        .get(PageWord::Tags)
        .and_then(|tags| tags.value.as_texts())
        .unwrap_or_default()
    {
        if let Some(list) = site.spelt(p.language, &ListKey::Tag(tag.value.clone()))
            && !lists.contains(&list)
        {
            lists.push(list);
        }
    }
    lists
}

thread_local! {
    /// How many tags each page shares with the one in hand, by page; all
    /// nothing between one count and the next.
    static SHARED: RefCell<Vec<u16>> = const { RefCell::new(Vec::new()) };
}

/// The posts sharing at least one tag with `page`, never the page itself,
/// grouped by how many they share, those sharing most first. A group is
/// in no order: [`in_site_order`] puts one in the site's.
///
/// The posts of a tag grow with the site, so this is counted without
/// hashing or sorting, and the groups left in no order for whoever reads
/// only the first few posts.
pub(crate) fn related(site: &Site, page: PageId) -> Vec<Vec<PageId>> {
    SHARED.with_borrow_mut(|shared| {
        shared.resize(site.pages.len(), 0);
        let mut sharing = Vec::new();
        let mut most = 0;
        for tag in tags_of(site, page) {
            for &member in &site.lists.get(tag).members {
                let count = &mut shared[member.index()];
                if member != page {
                    if *count == 0 {
                        sharing.push(member);
                    }
                    *count = count.saturating_add(1);
                    most = most.max(*count);
                }
            }
        }
        let mut groups = vec![Vec::new(); usize::from(most)];
        for member in sharing {
            let count = std::mem::take(&mut shared[member.index()]);
            groups[usize::from(most - count)].push(member);
        }
        groups
    })
}

/// Posts put in the site's order, those in none after the rest.
pub(crate) fn in_site_order(site: &Site, posts: &mut [PageId]) {
    posts.sort_unstable_by_key(|&post| {
        let place = site.places.get(post.index()).copied().flatten();
        (place.unwrap_or(usize::MAX), post)
    });
}

/// The names of a language's lists of one kind, A to Z.
fn names_of(site: &Site, language: LanguageId, kind: impl Fn(&ListKey) -> bool) -> Vec<String> {
    let mut names: Vec<String> = site
        .lists
        .iter()
        .filter(|(_, list)| list.language == language && kind(&list.key))
        .filter_map(|(_, list)| match &list.key {
            ListKey::Tag(name) | ListKey::Series(name) => Some(name.clone()),
            _ => None,
        })
        .collect();
    names.sort();
    names
}

/// Whether a page's field answers to `wanted`: text as written, capitals
/// aside, as tags are; a list by holding it; a truth by its word; a date by
/// its day.
fn answers(page: &Page, field: &str, wanted: &str) -> bool {
    let wanted = wanted.trim();
    match PageWord::named(field) {
        Some(PageWord::Title) => page
            .title
            .as_ref()
            .is_some_and(|title| title.value.eq_ignore_ascii_case(wanted)),
        Some(PageWord::Date) => page
            .date
            .as_ref()
            .is_some_and(|date| date.value.iso_day() == wanted || date.value.to_string() == wanted),
        Some(word) => page
            .words
            .get(word)
            .is_some_and(|value| value_answers(&value.value, wanted)),
        None => match page.words.field(field) {
            Some(own) => node_answers(&own.value, wanted),
            None => date_part(page, field)
                .is_some_and(|part| wanted.parse::<i64>().is_ok_and(|asked| asked == part)),
        },
    }
}

fn date_part(page: &Page, field: &str) -> Option<i64> {
    let date = &page.date.as_ref()?.value;
    match field {
        "year" => Some(i64::from(date.year)),
        "month" => Some(i64::from(date.month)),
        "day" => Some(i64::from(date.day)),
        _ => None,
    }
}

fn value_answers(value: &Value, wanted: &str) -> bool {
    match value {
        Value::Text(text) => text.eq_ignore_ascii_case(wanted),
        Value::Texts(texts) => texts.iter().any(|t| t.value.eq_ignore_ascii_case(wanted)),
        Value::Bool(truth) => wanted == if *truth { "true" } else { "false" },
        Value::Whole(n) => wanted.parse::<u64>() == Ok(*n),
        Value::Date(date) => date.iso_day() == wanted || date.to_string() == wanted,
        Value::Table(_) | Value::Refused => false,
    }
}

fn node_answers(node: &Node, wanted: &str) -> bool {
    match &node.value {
        Yaml::Scalar(text) => text.trim().eq_ignore_ascii_case(wanted),
        Yaml::Sequence(items) => items.iter().any(|item| node_answers(item, wanted)),
        Yaml::Mapping(_) => false,
    }
}
