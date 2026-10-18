//! What there is to say of one page to whoever is writing its template:
//! what builds it and which lists hold it, and every name a template can
//! read on it, with what each holds and where that is written.
//!
//! The names are the ones templates are checked against when they are
//! read, and each value is asked of the page as a template asks it, so
//! what is said here is what a template is handed.

use std::sync::Arc;

use minijinja::Value;
use minijinja::value::ValueKind;

use super::check::{Rec, Ty};
use super::data::FolderValue;
use super::frame::{self, Frame};
use super::values::{self, Lists, PageValue, SiteValue};
use super::{Chosen, Renderer};
use crate::content::words::PageWord;
use crate::language::Direction;
use crate::site::{DataId, NameId, PageId};
use crate::source::{Place, SourceId};

/// How deep the author's own nested fields are followed.
const DEEPEST: usize = 4;
/// How many fields of one nested value are shown.
const WIDEST: usize = 40;
/// How much of a text is shown, in characters.
const LONGEST: usize = 120;

/// Where a template reads what the site keeps under `data/`.
const DATA: &str = "site.data";

/// One page, as its template meets it.
pub struct Explained {
    /// The file it is written in; none for a page Statac made.
    pub source: Option<SourceId>,
    pub title: Option<String>,
    pub address: String,
    /// Its language's tag, as a template is handed it, and which way its
    /// lines run.
    pub language: String,
    pub direction: Direction,
    /// The template that builds it; none where there isn't one to.
    pub layout: Option<Chosen>,
    /// The files of that template and of each it extends, in turn.
    pub templates: Vec<SourceId>,
    pub lists: Vec<Member>,
    /// Where each feed carrying it is published.
    pub feeds: Vec<String>,
    /// `page`, `site`, then `site.data` where the site has any.
    pub groups: Vec<Group>,
}

/// A list the page is in, and its place among the list's posts.
pub struct Member {
    /// The kind of list it is, and its name.
    pub word: crate::problem::Word,
    pub list: String,
    pub at: usize,
    pub of: usize,
    /// The posts before and after it in the list.
    pub previous: Option<Neighbour>,
    pub next: Option<Neighbour>,
}

/// A post beside the page in a list: what it is called, a title or the
/// name Statac gives a page without one, and where it stands.
pub struct Neighbour {
    pub called: String,
    pub address: String,
}

pub struct Group {
    pub of: &'static str,
    pub rows: Vec<Row>,
}

/// One name a template can write.
pub struct Row {
    /// As a template writes it: `page.book.author`.
    pub name: String,
    pub holds: Holds,
    pub held: Held,
    /// Where the value is written, where it is written anywhere.
    pub from: Option<Place>,
    /// The names it answers in turn, as `page.date` answers `year`.
    pub has: Vec<String>,
    pub reached: Reached,
}

/// What a template asks for on its way to a name.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Reached {
    /// A name of the page's at the top, as `page.book` is for
    /// `page.book.author`.
    Page(NameId),
    /// A name of the site's at the top.
    Site(NameId),
    /// Any of these files under `data/`.
    Data(Vec<DataId>),
}

/// The kind of thing a name holds.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Holds {
    Text,
    Html,
    Number,
    Truth,
    Date,
    Page,
    Pages,
    List,
    Lists,
    Items,
    Fields,
    Mentions,
    Call,
    Unknown,
}

/// What a name holds on this page.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Held {
    /// As a template would print it.
    Shown(String),
    /// A list, by how many it holds.
    Count(usize),
    /// Fields of its own, each on a row beneath.
    Fields,
    /// Something a template calls rather than prints.
    Call,
    /// How many more fields the value has than are shown beneath it.
    More(usize),
    Nothing,
}

impl Renderer {
    /// What there is to say of the page `number`, counted from 1, of the
    /// list `page` speaks for, or of `page` itself.
    pub fn explained(&self, page: PageId, number: usize) -> Explained {
        let shared = &self.shared;
        let site = &shared.site;
        let held = site.pages.get(page);
        let schema = super::schema(site, &shared.settings);

        frame::push(Frame {
            number,
            ..Frame::new(Arc::clone(shared), Some(page))
        });
        let as_page = PageValue::numbered(shared, page, number);
        let address = as_page.to_string();
        let page_rows = self.rows("page", &as_page, &schema, Rec::Page, Some(page));
        let as_site = SiteValue::value(shared, held.language);
        let site_rows = self.rows("site", &as_site, &schema, Rec::Site, None);
        let neighbour = |id: PageId| Neighbour {
            called: super::own::headline(shared, id).unwrap_or_else(|_| shared.address_of(id)),
            address: shared.address_of(id),
        };
        let lists = site.lists.iter().filter_map(|(id, list)| {
            let at = list.members.iter().position(|&member| member == page)?;
            let (word, name) = crate::feeds::described(site, &shared.sources, id);
            let beside = |at: Option<usize>| list.members.get(at?).copied().map(neighbour);
            Some(Member {
                word,
                list: name,
                at: at + 1,
                of: list.members.len(),
                previous: beside(at.checked_sub(1)),
                next: beside(at.checked_add(1)),
            })
        });
        let lists: Vec<Member> = lists.collect();
        // Nothing asked here is a read of any output's.
        frame::pop();

        // What the site keeps under `data/` is the author's own, and can be
        // long.
        let (data, site_rows): (Vec<Row>, Vec<Row>) = (site_rows.into_iter())
            .partition(|row| row.name.len() > DATA.len() && row.name.starts_with(DATA));
        let mut groups = vec![
            Group {
                of: "page",
                rows: page_rows,
            },
            Group {
                of: "site",
                rows: site_rows,
            },
        ];
        if !data.is_empty() {
            groups.push(Group {
                of: DATA,
                rows: data,
            });
        }
        let feeds = (shared.site.lists.iter())
            .filter(|(_, list)| list.members.contains(&page))
            .filter_map(|(id, _)| self.feeds().iter().find(|feed| feed.list == id))
            .map(|feed| format!("{}{}", shared.base, feed.address.as_str()));
        let layout = self.layout_of(page).ok();
        let templates = &shared.templates;
        let chain = layout.as_ref().map(|chosen| templates.chain(&chosen.name));
        Explained {
            source: held.source,
            title: held.title.as_ref().map(|title| title.value.clone()),
            address,
            language: (site.languages.get(held.language).tag.clone())
                .unwrap_or_else(|| crate::language::ENGLISH.to_owned()),
            direction: shared.dates.language(held.language).words.direction,
            layout,
            templates: (chain.into_iter().flatten())
                .map(|at| templates.files[at].source)
                .collect(),
            lists,
            feeds: feeds.collect(),
            groups,
        }
    }

    /// A row for each name `record` answers, by name.
    fn rows(
        &self,
        prefix: &str,
        record: &Value,
        schema: &super::Schema,
        rec: Rec,
        page: Option<PageId>,
    ) -> Vec<Row> {
        let mut rows = Vec::new();
        for (name, field) in schema.record(rec) {
            if rec == Rec::Page && values::PAGE_NOT_YET.contains(&name.as_str()) {
                continue;
            }
            let value = record.get_attr(name).ok().filter(|value| !nothing(value));
            let written = match page {
                Some(page) => self.page_written(page, name),
                None => self.setting_written(name),
            };
            let from = value.as_ref().and_then(frame::origin).or(written);
            let full = format!("{prefix}.{name}");
            let has = match &field.ty {
                Ty::Record(rec @ (Rec::List | Rec::Date | Rec::When | Rec::Webmentions)) => {
                    schema.record(*rec).keys().cloned().collect()
                }
                _ => Vec::new(),
            };
            let from = from.filter(|_| value.is_some());
            let folder = value
                .as_ref()
                .and_then(Value::downcast_object_ref::<FolderValue>);
            let reached = match folder {
                Some(folder) => Reached::Data(folder.files()),
                None if rec == Rec::Page => Reached::Page(self.name_id(name)),
                None => Reached::Site(self.name_id(name)),
            };
            self.row(&mut rows, full, &field.ty, value, from, has, reached, 1);
        }
        rows
    }

    /// The row for one name, and beneath it a row for each field of the
    /// author's own it holds.
    #[allow(clippy::too_many_arguments)]
    fn row(
        &self,
        rows: &mut Vec<Row>,
        name: String,
        ty: &Ty,
        value: Option<Value>,
        from: Option<Place>,
        has: Vec<String>,
        reached: Reached,
        depth: usize,
    ) {
        let holds = holds(ty, value.as_ref());
        // A field is reached through what holds it, save under `data/`,
        // where each file is reached as itself.
        let folder = value
            .as_ref()
            .and_then(Value::downcast_object_ref::<FolderValue>);
        let reached_by = |key: &str| match folder {
            Some(folder) => Reached::Data(folder.files_of(key)),
            None => reached.clone(),
        };
        let fields = value.as_ref().filter(|value| {
            matches!(ty, Ty::Any)
                && value.kind() == ValueKind::Map
                && !statacs_own(value)
                && depth < DEEPEST
        });
        let held = match &value {
            _ if holds == Holds::Call => Held::Call,
            Some(value) if holds == Holds::Mentions => {
                let said = value.get_attr("said").ok().and_then(|said| said.len());
                Held::Count(said.unwrap_or(0))
            }
            None => Held::Nothing,
            Some(_) if fields.is_some() => Held::Fields,
            Some(value) => held(value),
        };
        let more = fields
            .and_then(|value| value.len())
            .map_or(0, |fields| fields.saturating_sub(WIDEST));
        let inner: Vec<(String, Value, Reached)> = fields
            .and_then(|value| value.try_iter().ok())
            .into_iter()
            .flatten()
            .take(WIDEST)
            .filter_map(|key| {
                let inner = value.as_ref()?.get_item(&key).ok()?;
                let key = key.as_str()?;
                Some((within(&name, key), inner, reached_by(key)))
            })
            .collect();
        let further = (more > 0).then(|| Row {
            name: name.clone(),
            holds,
            held: Held::More(more),
            from: None,
            has: Vec::new(),
            reached: reached.clone(),
        });
        rows.push(Row {
            name,
            holds,
            held,
            from,
            has,
            reached,
        });
        for (name, value, reached) in inner {
            let from = frame::origin(&value);
            let value = Some(value).filter(|value| !nothing(value));
            self.row(
                rows,
                name,
                &Ty::Any,
                value,
                from,
                Vec::new(),
                reached,
                depth + 1,
            );
        }
        rows.extend(further);
    }

    /// Where the page writes `name`, or is handed it.
    fn page_written(&self, page: PageId, name: &str) -> Option<Place> {
        let page = self.shared.site.pages.get(page);
        match name {
            "title" => page.title.as_ref()?.origin.place,
            "date" => page.date.as_ref()?.origin.place,
            _ => match PageWord::named(name) {
                Some(word) => page.words.get(word)?.origin.place,
                None => Some(page.words.field(name)?.key),
            },
        }
    }

    /// Where `statac.yaml` writes `name`.
    fn setting_written(&self, name: &str) -> Option<Place> {
        let settings = &self.shared.settings;
        let own = settings.own.iter().find(|own| own.name == name);
        match own {
            Some(own) => Some(own.key),
            None => settings.get(name)?.origin.place,
        }
    }
}

/// Whether a value is one of Statac's own things, which prints as itself
/// and is not walked field by field.
fn statacs_own(value: &Value) -> bool {
    value.downcast_object_ref::<PageValue>().is_some()
        || value.downcast_object_ref::<SiteValue>().is_some()
        || value.downcast_object_ref::<values::ListValue>().is_some()
        || value.downcast_object_ref::<values::Languages>().is_some()
}

/// The field `key` of `name`, as a template writes it: after a dot where
/// the key can be, and in brackets where it can't.
fn within(name: &str, key: &str) -> String {
    let mut letters = key.chars();
    let plain = letters
        .next()
        .is_some_and(|first| first.is_alphabetic() || first == '_')
        && letters.all(|letter| letter.is_alphanumeric() || letter == '_');
    match plain {
        true => format!("{name}.{key}"),
        false => format!("{name}[\"{key}\"]"),
    }
}

fn nothing(value: &Value) -> bool {
    value.is_undefined() || value.is_none()
}

/// What a name holds, by what templates are checked against, or where
/// that doesn't say, by the value itself.
fn holds(ty: &Ty, value: Option<&Value>) -> Holds {
    match ty {
        Ty::Text => Holds::Text,
        Ty::Html => Holds::Html,
        Ty::Number => Holds::Number,
        Ty::Bool => Holds::Truth,
        Ty::Date | Ty::Record(Rec::Date | Rec::When) => Holds::Date,
        Ty::List(of) => match **of {
            Ty::Record(Rec::Page) => Holds::Pages,
            Ty::Record(Rec::List) => Holds::Lists,
            _ => Holds::Items,
        },
        Ty::Record(Rec::Page) => Holds::Page,
        Ty::Record(Rec::List) => Holds::List,
        Ty::Record(Rec::Webmentions) => Holds::Mentions,
        Ty::Record(_) => Holds::Fields,
        Ty::Callable => Holds::Call,
        Ty::Any => match value.map(Value::kind) {
            Some(ValueKind::String) => Holds::Text,
            Some(ValueKind::Number) => Holds::Number,
            Some(ValueKind::Bool) => Holds::Truth,
            Some(ValueKind::Seq | ValueKind::Iterable) => Holds::Items,
            Some(ValueKind::Map) => Holds::Fields,
            _ => Holds::Unknown,
        },
    }
}

/// A value as a row shows it: a list by its count, save a page's tags,
/// which are few and print as their names, and anything else as a
/// template would print it.
fn held(value: &Value) -> Held {
    if let Some(lists) = value.downcast_object_ref::<Lists>() {
        return match lists.lists.len() {
            0 => Held::Count(0),
            _ => Held::Shown(value.to_string()),
        };
    }
    if matches!(value.kind(), ValueKind::Seq | ValueKind::Iterable) {
        return Held::Count(value.len().unwrap_or(0));
    }
    if value.kind() == ValueKind::Bool {
        return Held::Shown(value.is_true().to_string());
    }
    // On one line, however many it was written on.
    let printed = value.to_string();
    let printed = printed.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut shown: String = printed.chars().take(LONGEST).collect();
    if shown.len() < printed.len() {
        shown.push('\u{2026}');
    }
    match shown.is_empty() {
        true => Held::Nothing,
        false => Held::Shown(shown),
    }
}
