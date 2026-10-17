//! What a template sees: `page` and `site`, and everything reached from
//! them, as objects over the site rather than copies of it. Every name
//! looked up records what it read before answering, so the record of an
//! output is complete; a lookup that finds nothing records that too, since
//! adding the field later changes the output.
//!
//! Folders, tags, series and declared lists all answer as one kind of
//! list. A list's posts are read a stretch at a time, as each of its pages
//! shows them, and recorded so: `Key::ListPage(list, n)`. Reading the whole
//! list reads every stretch.

use std::cmp::Ordering;
use std::fmt;
use std::sync::Arc;

use minijinja::value::{DynObject, Enumerator, Object, ObjectRepr, Value};
use minijinja::{Error, State};

use super::frame;
use super::render::{Part, Shared, SomePages};
use crate::config::declarations::FieldKind;
use crate::content::words::PageWord;
use crate::date::{Date, When};
use crate::dates::ShowDates;
use crate::lists::Order;
use crate::problem::{ArgumentFault, Kind, Needed, Unsupported};
use crate::reads::Key;
use crate::site::{Id, LanguageId, ListId, ListKey, NameId, PageId};
use crate::source::Place;
use crate::value::Value as Statacs;
use crate::yaml::{Node, Resolved, Tag, Value as Yaml};

/// Names Statac answers on a page whose work isn't built yet.
pub(super) const PAGE_NOT_YET: &[&str] = &["key"];

/// The names a page answers to besides its words and fields.
pub(super) const PAGE_ANSWERS: &[&str] = &[
    "title",
    "url",
    "slug",
    "date",
    "language",
    "direction",
    "tags",
    "series",
    "posts",
    "list",
    "folder",
    "source",
    "content",
    "words",
    "excerpt",
    "excerpt_marked",
    "maths",
    "opening",
    "number",
    "next",
    "previous",
    "translations",
    "post",
];

/// What a page is asked for a search box by.
pub(super) const SEARCH_BOX: &str = "search_box";

/// The names a page answers with more than its front matter writes under
/// them, or with something else: every name [`PageValue::answer`] has an
/// answer of its own for.
const WORKED_OUT: &[&str] = &[
    "key",
    "post",
    SEARCH_BOX,
    "title",
    "url",
    "slug",
    "date",
    "content",
    "words",
    "toc",
    "excerpt",
    "opening",
    "excerpt_marked",
    "maths",
    "number",
    "next",
    "previous",
    "translations",
    "language",
    "direction",
    "tags",
    "series",
    "posts",
    "list",
    "folder",
    "source",
    "card",
    "webmentions",
    "og_title",
];

/// Whether every page answers `key` with what its front matter writes
/// under it and nothing else, so the answer turns on that alone.
pub(super) fn as_written(key: &str) -> bool {
    !WORKED_OUT.contains(&key) && !super::media::CALLS.contains(&key)
}

/// Whether `page` writes `key` in its front matter, or is handed it, as
/// the text `wanted` where it is given: a page that doesn't answers
/// nothing to `key` where it [`as_written`], or nothing equal to that
/// text, since a text a page writes is answered as itself and nothing
/// else it writes is answered as a text.
pub(crate) fn writes(page: &crate::site::Page, key: &str, wanted: Option<&str>) -> bool {
    match PageWord::named(key) {
        Some(word) => match (page.words.get(word), wanted) {
            (Some(written), Some(wanted)) => {
                matches!(&written.value, Statacs::Text(said) if said == wanted)
            }
            (written, _) => written.is_some(),
        },
        None => match (page.words.field(key), wanted) {
            (Some(written), Some(wanted)) => {
                matches!(&written.value.value, Yaml::Scalar(said) if said == wanted)
            }
            (written, _) => written.is_some(),
        },
    }
}

/// The names a page answers from what it says of itself before its
/// writing and from where it stands, the same whichever page asks.
const OF_THE_FRONT: &[&str] = &["title", "url", "slug", "date", "language"];

/// Whether what every page answers to `key` turns on what the pages say
/// of themselves before their writing, and on nothing of the page asking.
pub(crate) fn of_the_front(key: &str) -> bool {
    as_written(key) || OF_THE_FRONT.contains(&key)
}

/// Whether `value` is one of Statac's own objects, which never hold a
/// missing value and are too large to search.
pub(super) fn opaque(value: &Value) -> bool {
    value.downcast_object_ref::<PageValue>().is_some()
        || value.downcast_object_ref::<SiteValue>().is_some()
        || value.downcast_object_ref::<Pages>().is_some()
        || value.downcast_object_ref::<ListValue>().is_some()
        || value.downcast_object_ref::<Lists>().is_some()
        || value.downcast_object_ref::<Languages>().is_some()
        || value.downcast_object_ref::<DateValue>().is_some()
        || value.downcast_object_ref::<MentionValue>().is_some()
        || value
            .downcast_object_ref::<super::data::FolderValue>()
            .is_some()
}

/// A value refused because Statac hasn't built what it names yet. Looking
/// it up stops the template there.
pub(super) fn not_yet(name: String) -> Value {
    Value::from(frame::raise(Kind::NotSupported {
        what: Unsupported::NotYet(name),
    }))
}

fn field_key(shared: &Shared, page: PageId, name: &str) -> Key {
    Key::Field(page, shared.name_id(name))
}

fn setting_key(shared: &Shared, name: &str) -> Key {
    Key::Setting(shared.name_id(name))
}

/// What a list's later pages leave out of what their first page says of
/// itself: what dates it, names it or places it.
const FIRST_PAGES_OWN: &[PageWord] = &[
    PageWord::Date,
    PageWord::Updated,
    PageWord::Permalink,
    PageWord::RedirectFrom,
    PageWord::RedirectTo,
    PageWord::Id,
    PageWord::Draft,
];

/// The page the template is about.
#[derive(Debug)]
pub(super) struct PageValue {
    pub shared: Arc<Shared>,
    pub id: PageId,
    /// Which of its list's pages it is, counted from 1. A list's later
    /// pages are its page continued, carrying what that page says of itself
    /// but none of its writing.
    pub number: usize,
}

impl PageValue {
    pub fn value(shared: &Arc<Shared>, id: PageId) -> Value {
        PageValue::numbered(shared, id, 1)
    }

    pub fn numbered(shared: &Arc<Shared>, id: PageId, number: usize) -> Value {
        Value::from_object(PageValue {
            shared: Arc::clone(shared),
            id,
            number,
        })
    }

    /// How a problem names the page.
    pub fn named(&self) -> crate::problem::Named {
        let shared = &self.shared;
        match self.number {
            1 => shared.site.named(self.id, &shared.sources),
            number => crate::problem::Named::later(number, shared.address_of(self.id)),
        }
    }

    /// Where the page stands, the site's own path included.
    fn url(&self) -> String {
        let shared = &self.shared;
        let later = shared
            .site
            .pages
            .get(self.id)
            .speaks_for
            .and_then(|list| shared.site.lists.get(list).later_address(self.number));
        match later {
            Some(address) => format!("{}{}", shared.base, address.as_str()),
            None => shared.address_of(self.id),
        }
    }

    /// The list's page `number`, beside this one, where it has one.
    fn beside(&self, list: ListId, number: usize) -> Option<Value> {
        let shared = &self.shared;
        frame::read(Key::ListPage(list, stretch_key(self.number)));
        let pages = 1 + shared.site.lists.get(list).later.len();
        (1..=pages)
            .contains(&number)
            .then(|| PageValue::numbered(shared, self.id, number))
    }

    /// The page `other`, a translation of this one, as this one is shown:
    /// for a list's later page, the same page of its list, where it has one.
    fn translation(&self, other: PageId) -> Option<Value> {
        let shared = &self.shared;
        if self.number == 1 {
            return Some(PageValue::value(shared, other));
        }
        let list = shared.site.pages.get(other).speaks_for?;
        frame::read(Key::ListPage(list, stretch_key(self.number)));
        let pages = 1 + shared.site.lists.get(list).later.len();
        (self.number <= pages).then(|| PageValue::numbered(shared, other, self.number))
    }

    fn answer(&self, key: &str) -> Option<Value> {
        if as_written(key) {
            return self.written(key);
        }
        let shared = &self.shared;
        let page = shared.site.pages.get(self.id);
        let field = |name: &str| frame::read(field_key(shared, self.id, name));
        if key == "key"
            && let Some(made) = &page.made
        {
            field(key);
            return match &made.item {
                crate::site::Item::Key(name) => Some(Value::from(name.as_str())),
                crate::site::Item::At(_) | crate::site::Item::Post { .. } => None,
            };
        }
        if key == "post"
            && let Some(post) = made_for(page)
        {
            field(key);
            return Some(PageValue::value(shared, post));
        }
        if PAGE_NOT_YET.contains(&key) {
            return Some(not_yet(format!("page.{key}")));
        }
        if key == SEARCH_BOX {
            return Some(Value::from(search_box_given(false)));
        }
        if super::media::CALLS.contains(&key) {
            return Some(Value::from(super::media::uncalled(key)));
        }
        let later = self.number > 1;
        match key {
            "title" => {
                field(key);
                page.title.as_ref().map(|t| match t.origin.place {
                    Some(place) => frame::sourced(&t.value, place),
                    None => Value::from(t.value.as_str()),
                })
            }
            "url" => {
                field(key);
                Some(Value::from(self.url()))
            }
            "slug" => {
                field(key);
                let url = self.url();
                let address = url.trim_end_matches('/');
                Some(Value::from(address.rsplit('/').next().unwrap_or_default()))
            }
            "date" => {
                field(key);
                page.date
                    .as_ref()
                    .filter(|_| !later)
                    .map(|d| DateValue::value(shared, d.value))
            }
            "content" if later => Some(Value::from_safe_string(String::new())),
            "words" if later => Some(Value::from(0)),
            "toc" if later => Some(Value::from(Vec::<Value>::new())),
            "excerpt" | "opening" if later => None,
            "excerpt_marked" | "maths" if later => Some(Value::from(false)),
            "content" => super::render::html_of(shared, self.id, Part::Content),
            "words" => {
                super::render::of_body(shared, self.id, |written| Some(Value::from(written.words)))
            }
            "excerpt" => super::render::html_of(shared, self.id, Part::Excerpt),
            "excerpt_marked" => super::render::of_body(shared, self.id, |written| {
                Some(Value::from(written.excerpt_marked))
            }),
            "maths" => {
                super::render::of_body(shared, self.id, |written| Some(Value::from(written.maths)))
            }
            "opening" => super::render::of_body(shared, self.id, |written| {
                written.opening.as_deref().map(Value::from)
            }),
            "toc" => Some(contents(&super::render::outline(shared, self.id))),
            "number" => {
                field("list");
                page.speaks_for.map(|_| Value::from(self.number))
            }
            "next" | "previous" => {
                let step = |n: usize| match key {
                    "next" => n.checked_add(1),
                    _ => n.checked_sub(1),
                };
                if let Some(list) = page.speaks_for {
                    return self.beside(list, step(self.number)?);
                }
                frame::read(Key::Neighbours(self.id));
                let [previous, next] = shared.site.neighbours.get(self.id.index())?;
                let neighbour = match key {
                    "next" => next,
                    _ => previous,
                };
                neighbour.map(|id| PageValue::value(shared, id))
            }
            "translations" => {
                frame::read(Key::Translations(self.id));
                let others = shared.site.translations.get(self.id.index());
                let others = others.map_or(&[][..], Vec::as_slice);
                Some(Value::from(
                    others
                        .iter()
                        .filter_map(|&other| self.translation(other))
                        .collect::<Vec<_>>(),
                ))
            }
            "language" => {
                frame::read(Key::Language(page.language));
                Some(Value::from(code_of(shared, page.language)))
            }
            "direction" => {
                let words = super::render::words_of(shared, page.language);
                Some(Value::from(words.direction.as_str()))
            }
            "tags" => {
                field(key);
                Some(Lists::value(
                    shared,
                    crate::lists::tags_of(&shared.site, self.id),
                ))
            }
            "series" => {
                field(key);
                let series = page.words.text(PageWord::Series)?;
                let list = shared
                    .site
                    .spelt(page.language, &ListKey::Series(series.value.to_owned()))?;
                Some(ListValue::value(shared, list))
            }
            "posts" => {
                field("list");
                match page.speaks_for {
                    Some(list) => {
                        frame::read(Key::ListPage(list, stretch_key(self.number)));
                        Some(Pages::value(shared, Which::Stretch(list, self.number)))
                    }
                    // A page making a page per item lists nothing of its own.
                    None if page.speaks_for_a_list() => Some(all_pages(shared)),
                    None => None,
                }
            }
            "list" => {
                field(key);
                page.speaks_for.map(|list| ListValue::value(shared, list))
            }
            "folder" => {
                field(key);
                Some(ListValue::value(shared, page.folder))
            }
            "source" => {
                field(key);
                let source = page.source?;
                Some(Value::from(
                    shared.sources.path(source).to_string_lossy().as_ref(),
                ))
            }
            "card" => super::card::of(shared, self.id),
            "webmentions" => Some(webmentions(shared, self.id)),
            "og_title" => {
                field(key);
                Some(match page.words.text(PageWord::OgTitle) {
                    Some(title) => match title.origin.place {
                        Some(place) => frame::sourced(title.value, place),
                        None => Value::from(title.value),
                    },
                    None => match super::own::headline(shared, self.id) {
                        Ok(name) => Value::from(name),
                        Err(error) => Value::from(error),
                    },
                })
            }
            _ => self.written(key),
        }
    }

    /// What the page's front matter writes as `key`, which is nothing
    /// where it [`writes`] nothing.
    fn written(&self, key: &str) -> Option<Value> {
        let shared = &self.shared;
        let page = shared.site.pages.get(self.id);
        frame::read(field_key(shared, self.id, key));
        if let Some(word) = PageWord::named(key) {
            if self.number > 1 && FIRST_PAGES_OWN.contains(&word) {
                return None;
            }
            return page
                .words
                .get(word)
                .and_then(|value| statacs_value(shared, &value.value, value.origin.place));
        }
        let found = page.words.field(key)?;
        Some(node_value(shared, &found.value, Some(key)))
    }

    /// The names the page answers to, for a template that walks them.
    fn names(&self) -> Vec<Value> {
        let page = self.shared.site.pages.get(self.id);
        let mut names: Vec<&str> = vec!["url", "slug", "content", "words", "tags", "folder"];
        if page.title.is_some() {
            names.push("title");
        }
        let later = self.number > 1;
        if page.date.is_some() && !later {
            names.push("date");
        }
        if made_for(page).is_some() {
            names.push("post");
        }
        for (word, _) in &page.words.words {
            if !(later && FIRST_PAGES_OWN.contains(word)) {
                names.push(word.name());
            }
        }
        for field in &page.words.fields {
            names.push(&field.name);
        }
        names.sort_unstable();
        names.dedup();
        names.into_iter().map(Value::from).collect()
    }
}

impl Object for PageValue {
    fn repr(self: &Arc<Self>) -> ObjectRepr {
        ObjectRepr::Map
    }

    /// A page is always something: counting its names to decide would
    /// read every one of them.
    fn is_true(self: &Arc<Self>) -> bool {
        true
    }

    fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
        let key = key.as_str()?;
        let answer = self.answer(key);
        if answer.is_none() {
            frame::missed(Some(self.id), key);
        }
        answer
    }

    fn enumerate(self: &Arc<Self>) -> Enumerator {
        Enumerator::Values(self.names())
    }

    fn call_method(
        self: &Arc<Self>,
        state: &mut State<'_, '_>,
        method: &str,
        args: &[Value],
    ) -> Result<Value, Error> {
        if method == SEARCH_BOX {
            return search_box(args);
        }
        if super::media::CALLS.contains(&method) {
            return super::media::call(&self.shared, self.id, state, method, args);
        }
        match self.get_value(&Value::from(method)) {
            Some(value) => value.call(state, args),
            None => Err(Error::from(minijinja::ErrorKind::UnknownMethod)),
        }
    }

    fn custom_cmp(self: &Arc<Self>, other: &DynObject) -> Option<Ordering> {
        let other = other.downcast_ref::<PageValue>()?;
        Some((self.id, self.number).cmp(&(other.id, other.number)))
    }

    fn render(self: &Arc<Self>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.url())
    }
}

/// A search box for what is being built: the id its element takes, a
/// second and later box each taking one of its own, and whether it is the
/// first, which is the one to carry what every box shares. A page's
/// writing counts its own boxes, since it is built once for every page
/// that shows it; the page's own output counts them with its layout's
/// where it shows the writing whole.
fn search_box(args: &[Value]) -> Result<Value, Error> {
    if !args.is_empty() {
        return Err(search_box_given(true));
    }
    let asked = frame::with(|frame| {
        frame.search_boxes += 1;
        frame.search_boxes
    });
    let id = match asked.unwrap_or(1) {
        0 | 1 => "statac-search".to_owned(),
        later => format!("statac-search-{later}"),
    };
    Ok(Value::from_object(Ordered(vec![
        ("id".into(), Value::from(id)),
        ("first".into(), Value::from(asked.unwrap_or(1) <= 1)),
    ])))
}

/// `page.search_box` named without being called, or `given` values it
/// has no use for.
fn search_box_given(given: bool) -> Error {
    frame::raise(Kind::WrongArgument {
        snippet: format!("page.{SEARCH_BOX}"),
        fault: match given {
            true => ArgumentFault::NeedsNames { wanted: Vec::new() },
            false => ArgumentFault::Needs(Needed::Brackets),
        },
    })
}

/// The post a page was made for by an `each:` choosing posts.
fn made_for(page: &crate::site::Page) -> Option<PageId> {
    match page.made.as_ref()?.item {
        crate::site::Item::Post { page, .. } => Some(page),
        _ => None,
    }
}

/// The code a template has for `language`: English's on a site naming none.
fn code(language: &crate::site::Language) -> &str {
    language.tag.as_deref().unwrap_or(crate::language::ENGLISH)
}

fn code_of(shared: &Shared, language: LanguageId) -> &str {
    code(shared.site.languages.get(language))
}

/// The tag of `language` on a site in several languages.
fn several(shared: &Shared, language: LanguageId) -> Option<&str> {
    let languages = &shared.site.languages;
    let tag = languages.get(language).tag.as_deref();
    tag.filter(|_| languages.len() > 1)
}

/// The text setting `name` as `language` reads it, recording its own line
/// on a site in several languages and, where it has none, the setting for
/// every language.
pub(super) fn setting_in<'a>(
    shared: &'a Shared,
    language: LanguageId,
    name: &str,
) -> Option<&'a str> {
    if let Some(tag) = several(shared, language) {
        frame::read(setting_key(shared, &format!("{tag}.{name}")));
        if shared.settings.for_language(tag, name).is_some() {
            return Some(shared.settings.text_for(Some(tag), name)?.value);
        }
    }
    frame::read(setting_key(shared, name));
    Some(shared.settings.text(name)?.value)
}

/// The site, the same everywhere, save that a site in several languages
/// answers in the language of the page being built.
#[derive(Debug)]
pub(super) struct SiteValue {
    pub shared: Arc<Shared>,
    pub language: LanguageId,
}

impl SiteValue {
    pub fn value(shared: &Arc<Shared>, language: LanguageId) -> Value {
        Value::from_object(SiteValue {
            shared: Arc::clone(shared),
            language,
        })
    }

    /// The tag of the language the site answers in, where it has several.
    fn several(&self) -> Option<&str> {
        several(&self.shared, self.language)
    }

    fn answer(&self, key: &str) -> Option<Value> {
        let shared = &self.shared;
        match key {
            "url" => {
                frame::read(setting_key(shared, key));
                let text = shared.settings.text(key)?;
                Some(Value::from(text.value.trim_end_matches('/')))
            }
            "host" => {
                frame::read(setting_key(shared, "url"));
                host_of(shared.settings.text("url")?.value).map(Value::from)
            }
            "title" => setting_in(shared, self.language, key).map(Value::from),
            "language" => {
                frame::read(setting_key(shared, key));
                Some(Value::from(code_of(shared, self.language)))
            }
            "languages" => {
                frame::read(setting_key(shared, "language"));
                Some(Value::from_object(Languages {
                    shared: Arc::clone(shared),
                }))
            }
            "pages" => Some(all_pages(shared)),
            "posts" => {
                let Some(list) = shared.site.own_list(self.language) else {
                    frame::read(Key::Home(self.language));
                    return None;
                };
                read_whole(list);
                Some(Pages::value(shared, Which::Posts(list)))
            }
            "tags" | "series" => {
                frame::read(Key::ListsIn(self.language));
                let tags = key == "tags";
                let mut lists: Vec<ListId> = shared
                    .site
                    .lists
                    .iter()
                    .filter(|(_, list)| list.language == self.language)
                    .filter(|(_, list)| match &list.key {
                        ListKey::Tag(_) => tags,
                        ListKey::Series(_) => !tags,
                        _ => false,
                    })
                    .map(|(id, _)| id)
                    .collect();
                lists.sort_by(|&a, &b| {
                    crate::lists::a_to_z(&list_name(shared, a), &list_name(shared, b))
                });
                Some(Lists::value(shared, lists))
            }
            "folders" => {
                frame::read(Key::Folders(self.language));
                Some(Lists::value(shared, shared.site.folders(self.language)))
            }
            "webmentions" => {
                frame::read(setting_key(shared, key));
                Some(Value::from(shared.settings.truth(key) == Some(true)))
            }
            "today" => {
                frame::read(Key::Today);
                Some(DateValue::value(shared, shared.dates.today()))
            }
            "data" => Some(super::data::FolderValue::top(shared)),
            "feed" => Some(match shared.site.own_list(self.language) {
                Some(list) => feed_value(shared, list),
                None => {
                    frame::read(Key::Home(self.language));
                    Value::from("")
                }
            }),
            _ => {
                frame::read(setting_key(shared, key));
                let own = shared.settings.own(key)?;
                Some(node_value(shared, &own.value, Some(key)))
            }
        }
    }
}

impl Object for SiteValue {
    fn repr(self: &Arc<Self>) -> ObjectRepr {
        ObjectRepr::Map
    }

    fn is_true(self: &Arc<Self>) -> bool {
        true
    }

    fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
        let key = key.as_str()?;
        let answer = self.answer(key);
        if answer.is_none() {
            frame::missed(None, key);
        }
        answer
    }

    fn enumerate(self: &Arc<Self>) -> Enumerator {
        let settings = &self.shared.settings;
        let mut names: Vec<Value> = [
            "data", "feed", "folders", "pages", "posts", "series", "tags", "today",
        ]
        .into_iter()
        .chain(
            ["title", "url", "host", "language"]
                .into_iter()
                .filter(|name| match *name {
                    "title" => settings.text_for(self.several(), name).is_some(),
                    "language" => true,
                    "host" => settings.text("url").is_some(),
                    _ => settings.text(name).is_some(),
                }),
        )
        .chain(["languages", "webmentions"])
        .map(Value::from)
        .collect();
        names.extend(settings.own.iter().map(|f| Value::from(f.name.as_str())));
        Enumerator::Values(names)
    }

    fn custom_cmp(self: &Arc<Self>, other: &DynObject) -> Option<Ordering> {
        other.downcast_ref::<SiteValue>().map(|_| Ordering::Equal)
    }

    fn render(self: &Arc<Self>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("site")
    }
}

/// The host a full address names, as the URL standard writes it:
/// `example.org` for `https://example.org/blog/`.
fn host_of(url: &str) -> Option<String> {
    url::Url::parse(url).ok()?.host_str().map(str::to_owned)
}

/// The key a stretch of a list is recorded by.
fn stretch_key(number: usize) -> u32 {
    u32::try_from(number).unwrap_or(u32::MAX)
}

/// Records reading every member of a list.
pub(super) fn read_whole(list: ListId) {
    frame::read(Key::Members(list));
}

/// Pages in an order, as a list that makes each page's value as it is
/// reached: most templates that ask for one want a few of them, or how
/// many there are.
#[derive(Debug)]
pub(super) struct Pages {
    pub shared: Arc<Shared>,
    pub which: Which,
}

/// Which pages a list of them holds.
#[derive(Debug)]
pub(super) enum Which {
    /// Every page the site publishes, a list's later pages among them, in
    /// the order of their addresses.
    Every,
    /// A list's posts.
    Posts(ListId),
    /// A list's posts, then its pages without a date.
    All(ListId),
    /// The posts on one of a list's pages, counted from 1.
    Stretch(ListId, usize),
    /// A list's page and each of its later pages, this many in all.
    Pager(PageId, usize),
    /// Posts in groups, one group after another, each put in the site's
    /// order when a post of it is first reached.
    Groups(Vec<Group>),
    /// Pages chosen from, or put in order from, one of the lists above
    /// that is the same whichever page asks, by `asked`.
    Chosen(SomePages, Asked, Over),
}

/// What pages chosen once for the site were chosen from, which decides
/// what reading them turns on.
#[derive(Clone, Copy, Debug)]
pub(super) enum Over {
    /// One list's pages, which reading the list recorded.
    List,
    /// Every page the site publishes.
    Every,
    /// Those of every page that write a name, as the text given where
    /// one is: a choice that keeps no page lacking it, or answering
    /// anything else to it, which turns on what those pages say and not
    /// on which other pages there are.
    Writing(NameId, Option<NameId>),
}

impl Over {
    /// Records reading what each page chosen from answers to `name`.
    pub fn read_field(self, name: NameId) {
        frame::read(match self {
            Over::Writing(writing, wanted) => Key::FieldOfWriters(writing, wanted, name),
            Over::List | Over::Every => Key::FieldOfEvery(name),
        });
    }
}

/// Posts in no order, and the same in the site's once it is asked for.
#[derive(Debug)]
pub(super) struct Group {
    posts: Vec<PageId>,
    ordered: std::sync::OnceLock<Vec<PageId>>,
}

impl Group {
    pub fn of(posts: Vec<PageId>) -> Group {
        Group {
            posts,
            ordered: std::sync::OnceLock::new(),
        }
    }
}

/// What a choice of pages worked out once for the site is known by: the
/// list and each filter it went through, and the values those were handed.
pub(super) type Asked = (String, Vec<Value>);

impl Pages {
    pub fn value(shared: &Arc<Shared>, which: Which) -> Value {
        Value::from_object(Pages {
            shared: Arc::clone(shared),
            which,
        })
    }

    /// What a choice from these pages is known by, where they are the
    /// same whichever page asks.
    pub fn known_by(&self) -> Option<Asked> {
        let of = |word: &str, list: &ListId| (format!("{word} {}", list.index()), Vec::new());
        match &self.which {
            Which::Every => Some(("every".to_owned(), Vec::new())),
            Which::Posts(list) => Some(of("posts", list)),
            Which::All(list) => Some(of("all", list)),
            Which::Chosen(_, asked, _) => Some(asked.clone()),
            Which::Stretch(..) | Which::Pager(..) | Which::Groups(_) => None,
        }
    }

    /// What these pages are chosen from, where they are the same
    /// whichever page asks.
    pub fn over(&self) -> Over {
        match &self.which {
            Which::Every => Over::Every,
            Which::Chosen(_, _, over) => *over,
            _ => Over::List,
        }
    }

    /// Each page, with which of its list's pages it is.
    pub fn each(&self) -> impl Iterator<Item = (PageId, usize)> + '_ {
        (0..self.len()).filter_map(|at| self.at(at))
    }

    /// The pages `choose` makes of these, known by `asked`, chosen `over`
    /// what is given: worked out the first time any page asks, with
    /// nothing it reads recorded, so the page asking records what the
    /// choice turns on itself, save which pages the site publishes. Pages
    /// asking the same at once wait for the first.
    pub fn once(
        &self,
        asked: Asked,
        over: Over,
        choose: impl FnOnce() -> Result<Vec<(PageId, usize)>, Error>,
    ) -> Result<Value, Error> {
        if let Over::Every = over {
            frame::read(Key::Pages);
        }
        let shared = &self.shared;
        let known = {
            let mut all = shared.chosen.lock().expect("never poisoned");
            Arc::clone(all.entry(asked.clone()).or_default())
        };
        let mut known = known.lock().expect("never poisoned");
        let chosen = match &*known {
            Some(chosen) => Arc::clone(chosen),
            None => {
                let chosen: SomePages = Arc::new(frame::unrecorded(choose)?);
                *known = Some(Arc::clone(&chosen));
                chosen
            }
        };
        Ok(Pages::value(shared, Which::Chosen(chosen, asked, over)))
    }

    fn len(&self) -> usize {
        let lists = &self.shared.site.lists;
        match &self.which {
            Which::Every => {
                frame::read(Key::Pages);
                self.shared.every_page().len()
            }
            Which::Posts(list) => lists.get(*list).members.len(),
            Which::All(list) => {
                let list = lists.get(*list);
                list.members.len() + list.undated.len()
            }
            Which::Stretch(list, number) => lists.get(*list).stretch(*number).len(),
            Which::Pager(_, pages) => *pages,
            Which::Groups(groups) => groups.iter().map(|group| group.posts.len()).sum(),
            Which::Chosen(pages, ..) => pages.len(),
        }
    }

    /// The page at `at`, and which of its list's pages it is.
    fn at(&self, at: usize) -> Option<(PageId, usize)> {
        let lists = &self.shared.site.lists;
        let first = |id: &PageId| (*id, 1);
        match &self.which {
            Which::Every => {
                frame::read(Key::Pages);
                self.shared.every_page().get(at).copied()
            }
            Which::Posts(list) => lists.get(*list).members.get(at).map(first),
            Which::All(list) => {
                let list = lists.get(*list);
                let undated = || list.undated.get(at - list.members.len());
                list.members.get(at).or_else(undated).map(first)
            }
            Which::Stretch(list, number) => lists.get(*list).stretch(*number).get(at).map(first),
            Which::Pager(page, pages) => (at < *pages).then_some((*page, at + 1)),
            Which::Groups(groups) => {
                let mut at = at;
                let group = groups.iter().find(|group| {
                    let within = at < group.posts.len();
                    if !within {
                        at -= group.posts.len();
                    }
                    within
                })?;
                let ordered = group.ordered.get_or_init(|| {
                    let mut posts = group.posts.clone();
                    crate::lists::in_site_order(&self.shared.site, &mut posts);
                    posts
                });
                ordered.get(at).map(first)
            }
            Which::Chosen(pages, ..) => pages.get(at).copied(),
        }
    }
}

impl Object for Pages {
    fn repr(self: &Arc<Self>) -> ObjectRepr {
        ObjectRepr::Seq
    }

    fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
        let (id, number) = self.at(key.as_usize()?)?;
        if !matches!(
            self.which,
            Which::Stretch(..) | Which::Pager(..) | Which::Groups(_)
        ) {
            frame::walked();
        }
        Some(PageValue::numbered(&self.shared, id, number))
    }

    fn enumerate(self: &Arc<Self>) -> Enumerator {
        Enumerator::Seq(self.len())
    }
}

/// Every page the site publishes, which records reading which pages
/// those are as they are looked at.
fn all_pages(shared: &Arc<Shared>) -> Value {
    Pages::value(shared, Which::Every)
}

/// A page's headings as its table of contents: each with its words, its
/// id, the link that lands on it, its level, and the headings beneath it
/// as `headings`.
fn contents(sections: &[super::render::Section]) -> Value {
    let mut listed = Vec::new();
    let mut at = 0;
    while let Some(section) = sections.get(at) {
        let end = sections[at + 1..]
            .iter()
            .position(|next| next.level <= section.level)
            .map_or(sections.len(), |beyond| at + 1 + beyond);
        listed.push(Value::from_object(Ordered(vec![
            ("title".into(), Value::from(section.text.as_str())),
            ("id".into(), Value::from(section.id.as_str())),
            ("url".into(), Value::from(fragment(&section.id))),
            ("level".into(), Value::from(section.level)),
            ("headings".into(), contents(&sections[at + 1..end])),
        ])));
        at = end;
    }
    Value::from(listed)
}

/// A link to an id on the same page: `#` and the id, with what an address
/// can't carry as itself written as `%` and its bytes.
fn fragment(id: &str) -> String {
    let mut out = String::with_capacity(id.len() + 1);
    out.push('#');
    for byte in id.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' => out.push(char::from(byte)),
            _ if b"-._~!$&'()*+,;=:@/?".contains(&byte) => out.push(char::from(byte)),
            _ => {
                use std::fmt::Write as _;
                write!(out, "%{byte:02X}").expect("writing to a string can't fail");
            }
        }
    }
    out
}

/// Where a list's feed is published, the site's own path included; empty
/// where none is, so `{% if list.feed %}` asks whether it has one.
fn feed_value(shared: &Shared, list: ListId) -> Value {
    let language = shared.site.lists.get(list).language;
    for name in crate::feeds::NEEDS {
        match *name {
            "title" => {
                setting_in(shared, language, name);
            }
            name => frame::read(setting_key(shared, name)),
        }
    }
    frame::read(Key::List(list));
    match shared.site.lists.get(list).page {
        Some(page) => {
            for name in ["feed", "index_page", "url", "redirect_to"] {
                frame::read(field_key(shared, page, name));
            }
        }
        None => frame::read(setting_key(shared, "pages.feed")),
    }
    Value::from(shared.feed(list).map_or_else(String::new, |feed| {
        format!("{}{}", shared.base, feed.address.as_str())
    }))
}

/// What a list is called: a folder by its own name, a tag or a series as
/// it is spelt, a declared list by its page's title or else the last part
/// of its address. The site's own list has no name of its own.
pub(super) fn list_name(shared: &Shared, id: ListId) -> String {
    match &shared.site.lists.get(id).key {
        ListKey::Folder(path) => path.rsplit('/').next().unwrap_or_default().to_owned(),
        ListKey::Tag(name) | ListKey::Series(name) => name.clone(),
        ListKey::Declared(page) => {
            frame::read(field_key(shared, *page, "title"));
            let page = shared.site.pages.get(*page);
            match &page.title {
                Some(title) => title.value.clone(),
                None => {
                    let address = page.address.as_str().trim_end_matches('/');
                    address.rsplit('/').next().unwrap_or_default().to_owned()
                }
            }
        }
    }
}

/// A list: a folder, a tag, a series or one a page declares, each
/// answering the same names.
#[derive(Debug)]
pub(super) struct ListValue {
    pub shared: Arc<Shared>,
    pub id: ListId,
}

impl ListValue {
    pub fn value(shared: &Arc<Shared>, id: ListId) -> Value {
        Value::from_object(ListValue {
            shared: Arc::clone(shared),
            id,
        })
    }

    fn answer(&self, key: &str) -> Option<Value> {
        let shared = &self.shared;
        let list = shared.site.lists.get(self.id);
        let own = Key::List(self.id);
        match key {
            "name" => {
                frame::read(own);
                Some(Value::from(list_name(shared, self.id)))
            }
            "title" => {
                frame::read(own);
                if let Some(page) = list.page {
                    frame::read(field_key(shared, page, "title"));
                    if let Some(title) = &shared.site.pages.get(page).title {
                        return Some(Value::from(title.value.as_str()));
                    }
                }
                match shared.site.is_home(self.id) {
                    true => setting_in(shared, list.language, "title").map(Value::from),
                    false => Some(Value::from(list_name(shared, self.id))),
                }
            }
            "page" => {
                frame::read(own);
                list.page.map(|page| PageValue::value(shared, page))
            }
            "url" => {
                frame::read(own);
                match (list.page, &list.key) {
                    (Some(page), _) => {
                        frame::read(field_key(shared, page, "url"));
                        Some(Value::from(shared.address_of(page)))
                    }
                    (None, ListKey::Folder(path)) => Some(Value::from(format!(
                        "{}/{}",
                        shared.base,
                        crate::content::slug::tidied_path(&format!("{path}/"))
                            .trim_start_matches('/')
                    ))),
                    // A tag or a series with no page stands at no address.
                    (None, _) => None,
                }
            }
            "posts" => {
                read_whole(self.id);
                Some(Pages::value(shared, Which::Posts(self.id)))
            }
            "pages" => {
                read_whole(self.id);
                Some(Pages::value(shared, Which::All(self.id)))
            }
            "pager" => {
                read_whole(self.id);
                Some(match list.page {
                    Some(page) if shared.site.pages.get(page).lands() => {
                        Pages::value(shared, Which::Pager(page, 1 + list.later.len()))
                    }
                    _ => Value::from(Vec::<Value>::new()),
                })
            }
            "feed" => Some(feed_value(shared, self.id)),
            "word" => Some(Value::from(match shared.site.is_home(self.id) {
                true => "site",
                false => list.key.word(),
            })),
            "order" => {
                frame::read(own);
                Some(Value::from(list.order.map_or("", Order::word)))
            }
            _ => None,
        }
    }
}

impl Object for ListValue {
    fn repr(self: &Arc<Self>) -> ObjectRepr {
        ObjectRepr::Map
    }

    fn is_true(self: &Arc<Self>) -> bool {
        true
    }

    fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
        let key = key.as_str()?;
        let answer = self.answer(key);
        if answer.is_none() {
            frame::missed(None, key);
        }
        answer
    }

    fn enumerate(self: &Arc<Self>) -> Enumerator {
        Enumerator::Str(&[
            "name", "title", "url", "page", "feed", "posts", "pages", "pager", "word", "order",
        ])
    }

    fn custom_cmp(self: &Arc<Self>, other: &DynObject) -> Option<Ordering> {
        let other = other.downcast_ref::<ListValue>()?;
        Some(self.id.index().cmp(&other.id.index()))
    }

    /// A list printed is its name, so `{{ tag }}` shows the tag.
    fn render(self: &Arc<Self>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        frame::read(Key::List(self.id));
        f.write_str(&list_name(&self.shared, self.id))
    }
}

/// The languages a site declares, main first: walked as their tags, and
/// asked by tag for the page at that language's top, written or made.
#[derive(Debug)]
pub(super) struct Languages {
    pub shared: Arc<Shared>,
}

impl Object for Languages {
    fn repr(self: &Arc<Self>) -> ObjectRepr {
        ObjectRepr::Map
    }

    fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
        let tag = key.as_str()?;
        let site = &self.shared.site;
        let language = site
            .languages
            .iter()
            .find(|(_, language)| code(language) == tag)
            .map(|(id, _)| id);
        let home = language.and_then(|language| {
            frame::read(Key::Home(language));
            site.home(language)
        });
        if home.is_none() {
            frame::missed(None, tag);
        }
        home.map(|page| PageValue::value(&self.shared, page))
    }

    fn enumerate(self: &Arc<Self>) -> Enumerator {
        let tags = self.shared.site.languages.iter();
        let tags = tags.map(|(_, language)| code(language));
        Enumerator::Values(tags.map(Value::from).collect())
    }

    fn render(self: &Arc<Self>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tags = self.shared.site.languages.iter();
        let tags: Vec<&str> = tags.map(|(_, language)| code(language)).collect();
        f.write_str(&tags.join(", "))
    }
}

/// Lists of one kind, such as a page's tags: walked, counted and joined as
/// a list of them is, and asked by name, so `"Rust" in page.tags` finds the
/// tag however its capitals are written.
#[derive(Debug)]
pub(super) struct Lists {
    pub shared: Arc<Shared>,
    pub lists: Vec<ListId>,
}

impl Lists {
    pub fn value(shared: &Arc<Shared>, lists: Vec<ListId>) -> Value {
        Value::from_object(Lists {
            shared: Arc::clone(shared),
            lists,
        })
    }

    /// The list `name` is a spelling of, among these.
    fn named(&self, name: &str) -> Option<ListId> {
        frame::read(Key::Spellings);
        let site = &self.shared.site;
        self.lists.iter().copied().find(|&id| {
            let list = site.lists.get(id);
            let spelt = match &list.key {
                ListKey::Tag(_) => ListKey::Tag(name.to_owned()),
                ListKey::Series(_) => ListKey::Series(name.to_owned()),
                _ => return false,
            };
            site.spelt(list.language, &spelt) == Some(id)
                || list_name(&self.shared, id).eq_ignore_ascii_case(name)
        })
    }
}

impl Object for Lists {
    fn repr(self: &Arc<Self>) -> ObjectRepr {
        ObjectRepr::Map
    }

    fn is_true(self: &Arc<Self>) -> bool {
        !self.lists.is_empty()
    }

    fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
        let id = match key.as_i64() {
            Some(at) => *self.lists.get(usize::try_from(at).ok()?)?,
            None => self.named(&key.to_string())?,
        };
        Some(ListValue::value(&self.shared, id))
    }

    fn enumerate(self: &Arc<Self>) -> Enumerator {
        Enumerator::Values(
            self.lists
                .iter()
                .map(|&id| ListValue::value(&self.shared, id))
                .collect(),
        )
    }

    fn render(self: &Arc<Self>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names: Vec<String> = self
            .lists
            .iter()
            .map(|&id| list_name(&self.shared, id))
            .collect();
        f.write_str(&names.join(", "))
    }
}

/// A date, printed in the site's own way.
#[derive(Debug)]
pub(super) struct DateValue {
    pub shared: Arc<Shared>,
    /// The date, or the first day of the year or month written.
    pub date: Date,
    /// A year, or a year and month, where that is all that was written,
    /// with the language it is printed in.
    pub part: Option<(When, LanguageId)>,
}

impl DateValue {
    pub fn value(shared: &Arc<Shared>, date: Date) -> Value {
        Value::from_object(DateValue {
            shared: Arc::clone(shared),
            date,
            part: None,
        })
    }

    /// A date written as `when`, a whole date or a part of one.
    pub fn when(shared: &Arc<Shared>, when: When) -> Value {
        let date = match when {
            When::Date(date) => return DateValue::value(shared, date),
            When::Year(year) => Date::day_only(year, 1, 1),
            When::Month(year, month) => Date::day_only(year, month, 1),
        };
        let language = read_date_settings(shared);
        Value::from_object(DateValue {
            shared: Arc::clone(shared),
            date,
            part: Some((when, language)),
        })
    }

    /// The date as printed, for a part of one.
    pub fn part_printed(&self) -> Option<String> {
        let (when, language) = self.part.as_ref()?;
        Some(self.shared.dates.when(*language, when))
    }

    /// How much of the date was written: of two on one day, the one
    /// written with less comes first.
    fn written(&self) -> u8 {
        match self.part {
            Some((When::Year(_), _)) => 0,
            Some((When::Month(..), _)) => 1,
            _ => 2,
        }
    }
}

impl Object for DateValue {
    fn repr(self: &Arc<Self>) -> ObjectRepr {
        ObjectRepr::Plain
    }

    fn is_true(self: &Arc<Self>) -> bool {
        true
    }

    fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
        let has = |at_least: u8| self.written() >= at_least;
        match key.as_str()? {
            "year" => Some(Value::from(self.date.year)),
            "month" if has(1) => Some(Value::from(self.date.month)),
            "day" if has(2) => Some(Value::from(self.date.day)),
            "weekday" if has(2) => {
                let language = frame::language();
                frame::read(Key::Language(language));
                self.shared
                    .dates
                    .weekday(language, &self.date)
                    .map(Value::from)
            }
            other => {
                frame::missed(None, other);
                None
            }
        }
    }

    fn custom_cmp(self: &Arc<Self>, other: &DynObject) -> Option<Ordering> {
        let other = other.downcast_ref::<DateValue>()?;
        let order = self.shared.dates.order(&self.date, &other.date);
        Some(order.then(self.written().cmp(&other.written())))
    }

    fn render(self: &Arc<Self>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(printed) = self.part_printed() {
            return f.write_str(&printed);
        }
        let written = written_date(&self.shared, &self.date, None);
        f.write_str(written.as_deref().unwrap_or_default())
    }
}

/// A date as the page being built writes it, in the site's format or by a
/// template's `example`, recording what that read.
pub(super) fn written_date(
    shared: &Shared,
    date: &Date,
    example: Option<&str>,
) -> Result<String, crate::dates::Refusal> {
    let language = read_date_settings(shared);
    let written = shared.dates.format(language, date, example);
    if example.is_some() {
        read_others(shared, shared.dates.read_in(language, example));
    }
    written
}

/// Records reading every language, where an example the page's language
/// couldn't read was read in another's words: which reads it turns on
/// them all.
fn read_others(shared: &Shared, read_in: Option<LanguageId>) {
    if read_in.is_none() {
        return;
    }
    for (id, _) in shared.site.languages.iter() {
        frame::read(Key::Language(id));
    }
}

/// Records reading what a date is written by in the language of the page
/// being built, and returns that language: the site's format, the
/// language's words, and the zone.
pub(super) fn read_date_settings(shared: &Shared) -> LanguageId {
    for name in ["date_format", "date_zeros", "timezone"] {
        frame::read(setting_key(shared, name));
    }
    let language = frame::language();
    if let Some(tag) = several(shared, language) {
        frame::read(setting_key(shared, &format!("{tag}.date_format")));
    }
    frame::read(Key::Language(language));
    read_others(shared, shared.dates.read_in(language, None));
    language
}

/// One of Statac's own words, as a template sees it.
/// One of Statac's own values on a page, a text remembering `place`,
/// where it is written.
fn statacs_value(shared: &Arc<Shared>, value: &Statacs, place: Option<Place>) -> Option<Value> {
    Some(match value {
        Statacs::Text(text) => match place {
            Some(place) => frame::sourced(text, place),
            None => Value::from(text.as_str()),
        },
        Statacs::Bool(b) => Value::from(*b),
        Statacs::Whole(n) => Value::from(*n),
        Statacs::Texts(texts) => Value::from(
            texts
                .iter()
                .map(|t| match t.origin.place {
                    Some(place) => frame::sourced(&t.value, place),
                    None => Value::from(t.value.as_str()),
                })
                .collect::<Vec<_>>(),
        ),
        Statacs::Date(date) => DateValue::value(shared, *date),
        Statacs::Table(rows) => Value::from_object(Ordered(
            rows.iter()
                .map(|(name, given)| (name.value.clone(), Value::from(given.value.as_str())))
                .collect(),
        )),
        Statacs::Refused => return None,
    })
}

/// An author's value from YAML: a scalar as YAML's standard reading makes
/// it, or as the declaration of the field at `path` says where it has one,
/// `book.author` being `author:` beneath `book:`.
pub(super) fn node_value(shared: &Arc<Shared>, node: &Node, path: Option<&str>) -> Value {
    let declarations = &shared.settings.declarations;
    let kind = path.and_then(|path| declarations.kind(path));
    match &node.value {
        Yaml::Scalar(_) if node.tag == Some(Tag::Bytes) => Value::from_object(BytesValue),
        Yaml::Scalar(text) => {
            if matches!(kind, Some(FieldKind::Date | FieldKind::When))
                && let Ok(when) = crate::date::read_when(text)
            {
                return DateValue::when(shared, when);
            }
            match node.resolved() {
                Some(Resolved::Null) => Value::from(()),
                Some(Resolved::Bool(b)) => Value::from(b),
                Some(Resolved::Int(Some(n))) => match i64::try_from(n) {
                    Ok(n) => Value::from(n),
                    Err(_) => frame::sourced(text, node.place),
                },
                Some(Resolved::Float(f)) => Value::from(f),
                _ => frame::sourced(text, node.place),
            }
        }
        Yaml::Sequence(items) => Value::from(
            items
                .iter()
                .map(|item| node_value(shared, item, None))
                .collect::<Vec<_>>(),
        ),
        Yaml::Mapping(entries) => {
            let nests = path.filter(|path| declarations.nests(path));
            Value::from_object(Ordered(
                entries
                    .iter()
                    .map(|entry| {
                        let key = crate::content::words::key_name(&entry.key);
                        let inner = nests.map(|path| format!("{path}.{key}"));
                        let value = node_value(shared, &entry.value, inner.as_deref());
                        (key, value)
                    })
                    .collect(),
            ))
        }
    }
}

/// A map that keeps the order it was written in.
#[derive(Debug)]
pub(super) struct Ordered(pub Vec<(String, Value)>);

impl Object for Ordered {
    fn repr(self: &Arc<Self>) -> ObjectRepr {
        ObjectRepr::Map
    }

    fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
        let key = key.as_str()?;
        let found = self.0.iter().find(|(name, _)| name == key);
        if found.is_none() {
            frame::missed(None, key);
        }
        found.map(|(_, value)| value.clone())
    }

    fn enumerate(self: &Arc<Self>) -> Enumerator {
        Enumerator::Values(
            self.0
                .iter()
                .map(|(k, _)| Value::from(k.as_str()))
                .collect(),
        )
    }
}

/// The mentions of the page `id`: those that say something, and those
/// shown as a face alone.
fn webmentions(shared: &Arc<Shared>, id: PageId) -> Value {
    frame::read(Key::Mentions(id));
    let (faces, said): (Vec<_>, Vec<_>) =
        (shared.site.mentions.of(id).iter()).partition(|mention| mention.kind.is_a_face());
    let listed = |mentions: Vec<&crate::fetched::mentions::Mention>| {
        Value::from(
            (mentions.into_iter())
                .map(|mention| {
                    Value::from_object(MentionValue {
                        shared: Arc::clone(shared),
                        mention: mention.clone(),
                    })
                })
                .collect::<Vec<_>>(),
        )
    };
    Value::from_object(Ordered(vec![
        ("said".into(), listed(said)),
        ("faces".into(), listed(faces)),
    ]))
}

/// The names a mention answers to.
const MENTION_ANSWERS: &[&str] = &[
    "kind",
    "author",
    "author_url",
    "url",
    "published",
    "words",
    "face",
];

/// One mention of a page, every text of it as plain text. Its face is
/// looked for when first asked.
#[derive(Debug)]
struct MentionValue {
    shared: Arc<Shared>,
    mention: crate::fetched::mentions::Mention,
}

impl Object for MentionValue {
    fn repr(self: &Arc<Self>) -> ObjectRepr {
        ObjectRepr::Map
    }

    fn is_true(self: &Arc<Self>) -> bool {
        true
    }

    fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
        let key = key.as_str()?;
        let mention = &self.mention;
        let text = |text: &Option<String>| text.as_deref().map(Value::from);
        let answer = match key {
            "kind" => Some(Value::from(mention.kind.word())),
            "author" => text(&mention.author),
            "author_url" => text(&mention.author_url),
            "url" => Some(Value::from(mention.url.as_str())),
            "published" => (mention.published).map(|day| DateValue::value(&self.shared, day)),
            "words" => text(&mention.words),
            "face" => (mention.face.as_deref())
                .map(|name| super::media::face(&self.shared, name))
                .filter(|face| !face.is_none()),
            _ => None,
        };
        if answer.is_none() {
            frame::missed(None, key);
        }
        answer
    }

    fn enumerate(self: &Arc<Self>) -> Enumerator {
        let names = MENTION_ANSWERS.iter().copied();
        Enumerator::Values(names.map(Value::from).collect())
    }
}

/// A `!!binary` value whose bytes aren't text. A template may hold it and
/// pass it on; showing it, or making words of it, stops the template.
#[derive(Debug)]
pub(super) struct BytesValue;

impl Object for BytesValue {
    fn repr(self: &Arc<Self>) -> ObjectRepr {
        ObjectRepr::Plain
    }
}

/// Whether `value` is, or holds, bytes that aren't text.
pub(super) fn holds_bytes(value: &Value) -> bool {
    if value.downcast_object_ref::<BytesValue>().is_some() {
        return true;
    }
    if opaque(value) {
        return false;
    }
    if let Some(own) = super::media::own_fields(value) {
        return own.iter().any(|(_, item)| holds_bytes(item));
    }
    match value.kind() {
        minijinja::value::ValueKind::Seq | minijinja::value::ValueKind::Iterable => value
            .try_iter()
            .is_ok_and(|mut items| items.any(|item| holds_bytes(&item))),
        minijinja::value::ValueKind::Map => value.try_iter().is_ok_and(|mut keys| {
            keys.any(|key| value.get_item(&key).is_ok_and(|item| holds_bytes(&item)))
        }),
        _ => false,
    }
}

/// The problem a template stops with where it shows bytes that aren't text.
pub(super) fn not_text() -> Error {
    frame::raise(Kind::TemplateStopped {
        reason: crate::problem::Stopped::NotText,
    })
}
