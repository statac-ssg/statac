//! Completion: what may be written where the cursor stands in the site's
//! settings or a page's front matter or writing, from what Statac takes
//! there and what the site holds.

mod spot;
#[cfg(test)]
mod tests;
mod writing;

use std::collections::BTreeMap;
use std::ops::Range;
use std::path::Path;

use spot::Spot;

use crate::config::declarations::{Declarations, FieldKind, KIND_WORDS, Names};
use crate::content::words::{Holder, PageWord};
use crate::problem::Code;
use crate::site::{LanguageId, ListKey, PageId, PagePlace};
use crate::source::{Place, Sources, Span};
use crate::templates::Renderer;
use crate::wording;

/// What may be written in place of what is typed.
#[derive(Debug)]
pub(super) struct Completion {
    /// What is typed so far, as offsets in the file's text.
    pub typed: Range<usize>,
    pub items: Vec<Item>,
    /// Whether the items are a part of what may be written there, so the
    /// editor asks again as more is typed.
    pub incomplete: bool,
}

impl Completion {
    fn of(typed: Range<usize>, items: Vec<Item>) -> Completion {
        Completion {
            typed,
            items,
            incomplete: false,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) struct Item {
    pub label: String,
    /// What the item is, said beside it.
    pub detail: Option<String>,
    pub kind: ItemKind,
    /// What is written in place of what is typed.
    pub insert: String,
    /// Where it is listed among the others, the lowest first: Statac's own
    /// before the site's.
    pub group: u8,
    /// Whether it is text of the site's, written to read back as that text,
    /// rather than one of Statac's own words, written as it is.
    pub text: bool,
    /// What the editor matches what is typed against, where it isn't the
    /// label.
    pub filter: Option<String>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(super) enum ItemKind {
    Field,
    Value,
    /// A call, written with places the editor fills in where it can.
    Snippet,
    File,
    /// An address or a heading.
    Reference,
}

impl ItemKind {
    /// The protocol's number for the kind.
    pub(super) fn number(self) -> u8 {
        match self {
            ItemKind::Field => 10,
            ItemKind::Value => 12,
            ItemKind::Snippet => 15,
            ItemKind::File => 17,
            ItemKind::Reference => 18,
        }
    }
}

/// What a file of the site holds, as completion reads it.
enum Holds {
    Settings,
    FrontMatter,
}

/// What may be written at `offset` in `text`, the text of `file` in the
/// site at `site`; none where nothing Statac knows of is written there.
/// `known` is the site as it was last read, where it has been, and
/// `snippets` whether the editor fills in places in what it writes.
pub(super) fn complete(
    site: &Path,
    file: &Path,
    text: &str,
    offset: usize,
    known: Option<&Renderer>,
    snippets: bool,
) -> Option<Completion> {
    let rel = file.strip_prefix(site).ok()?;
    let rel = rel.to_string_lossy().replace('\\', "/");
    let holds = match rel.as_str() {
        crate::config::FILE_NAME => Holds::Settings,
        _ if rel.starts_with("content/") && rel.ends_with(".md") => Holds::FrontMatter,
        _ => return None,
    };
    let mut sources = Sources::default();
    let source = sources.add(rel.as_str(), text.to_owned());
    let offset = text.floor_char_boundary(offset.min(text.len()));
    let span = match holds {
        Holds::Settings => Span::new(0, text.len()),
        Holds::FrontMatter => match crate::yaml::split(text) {
            Ok(split)
                if split
                    .front_matter
                    .is_none_or(|yaml| offset > yaml.end as usize) =>
            {
                let spot = writing::find(text, split.body, offset)?;
                let site = Known::of(known, &rel);
                let folder = rel.strip_prefix("content/").unwrap_or(&rel);
                let offer = writing::Offer {
                    text,
                    body: split.body,
                    renderer: known,
                    page: site.page,
                    folder: folder.rfind('/').map_or("", |at| &folder[..=at]),
                    snippets,
                };
                return offer.complete(spot, offset);
            }
            Ok(split) => split.front_matter?,
            Err(unclosed) => unclosed.rest,
        },
    };
    let spot = spot::find(&sources, Place { source, span }, offset)?;
    let site = Known::of(known, &rel);
    let (typed, items) = match spot {
        Spot::Name {
            path,
            typed,
            written,
            colon,
        } => {
            let names = match holds {
                Holds::Settings if path.is_empty() => site.setting_names(),
                Holds::Settings => Vec::new(),
                Holds::FrontMatter => site.field_names(&path),
            };
            (typed, names_offered(names, &written, colon))
        }
        Spot::Value {
            path,
            typed,
            flow,
            quote,
            written,
            spaced,
        } => {
            let name = path.join(".");
            let values = match holds {
                Holds::Settings => site.setting_values(&name, &text[typed.clone()]),
                Holds::FrontMatter => {
                    site.field_values(name.strip_prefix("pages.").unwrap_or(&name))
                }
            };
            let items = (values.into_iter())
                .filter(|value| !written.contains(&value.label))
                .map(|value| Item {
                    insert: value_written(&value.label, value.text, flow, quote, spaced),
                    ..value
                })
                .collect();
            (typed, items)
        }
    };
    Some(Completion::of(typed, items))
}

/// Names offered for a field's name, but those `written` already, each
/// with its colon where it has none yet.
fn names_offered(names: Vec<Item>, written: &[String], colon: bool) -> Vec<Item> {
    (names.into_iter())
        .filter(|name| !written.contains(&name.label))
        .map(|name| {
            let key = crate::yaml::write::key(&name.label);
            Item {
                insert: match colon {
                    true => key,
                    false => format!("{key}: "),
                },
                ..name
            }
        })
        .collect()
}

/// `value` as it is written where the cursor stands: inside the quote it
/// opens with, among the items of a list in brackets, or alone; as it is,
/// where it isn't `text`.
fn value_written(value: &str, text: bool, flow: bool, quote: Option<char>, spaced: bool) -> String {
    let written = match quote {
        None if !text => value.to_owned(),
        Some('\'') => value.replace('\'', "''"),
        Some(_) => {
            let quoted = crate::yaml::write::quoted(value);
            quoted[1..quoted.len() - 1].to_owned()
        }
        None if flow && value.contains([',', '[', ']', '{', '}']) => {
            crate::yaml::write::quoted(value)
        }
        None => crate::yaml::write::text(value),
    };
    match spaced {
        true => format!(" {written}"),
        false => written,
    }
}

/// What the site holds, as completion offers it.
struct Known<'a> {
    renderer: Option<&'a Renderer>,
    /// The page being written, where the site as last read holds it.
    page: Option<PageId>,
    /// The language of the page being written, where the site as last
    /// read tells it.
    language: Option<LanguageId>,
    /// What holds the front matter being written.
    holder: Holder,
}

impl<'a> Known<'a> {
    fn of(renderer: Option<&'a Renderer>, rel: &str) -> Known<'a> {
        let found = renderer.and_then(|renderer| {
            let (sources, site) = (renderer.sources(), renderer.site());
            (site.pages.iter()).find_map(|(id, page)| {
                let source = page.source.filter(|_| page.made.is_none())?;
                (sources.path(source) == Path::new(rel)).then_some((id, page))
            })
        });
        let page = found.map(|(_, page)| page);
        // A page whose front matter can't be read as it is typed is left
        // out of the site, but its folder is a list where it holds others.
        let folder = rel.strip_prefix("content/").unwrap_or(rel);
        let folder = folder.rsplit_once('/').map_or("", |(folder, _)| folder);
        let list = renderer.and_then(|renderer| {
            let key = ListKey::Folder(folder.to_owned());
            (renderer.site().lists.iter()).find_map(|(_, list)| (list.key == key).then_some(list))
        });
        let index = rel.ends_with("/index.md");
        let speaks = match page {
            Some(page) => page.place == PagePlace::Folder || page.makes_pages(),
            None if renderer.is_some() => index && list.is_some(),
            None => index,
        };
        let holder = match speaks {
            true => Holder::FolderPage {
                top: rel == "content/index.md",
            },
            false => Holder::Page,
        };
        let language = page.map(|page| page.language);
        Known {
            renderer,
            page: found.map(|(id, _)| id),
            language: language.or(list.map(|list| list.language)),
            holder,
        }
    }

    fn declarations(&self) -> Option<&'a Declarations> {
        Some(&self.renderer?.settings().declarations)
    }

    /// Statac's settings, the words a site hands down to every page, and
    /// each setting a language may give its own.
    fn setting_names(&self) -> Vec<Item> {
        let mut names: Vec<Item> = (crate::config::setting_names())
            .map(|name| own_field(name, wording::OFFERED_SETTING))
            .collect();
        let handed = (PageWord::all())
            .filter(|word| word.handed_down() && *word != PageWord::IndexPage)
            .map(|word| own_field(&format!("pages.{}", word.name()), wording::OFFERED_HANDED));
        names.extend(handed);
        names.extend(self.declared_handed());
        if let Some(renderer) = self.renderer {
            let languages = &renderer.site().languages;
            let tags = (languages.iter()).filter_map(|(_, language)| language.tag.as_deref());
            if languages.len() > 1 {
                for tag in tags {
                    let each = crate::config::PER_LANGUAGE.iter();
                    names.extend(each.map(|setting| {
                        own_field(&format!("{tag}.{setting}"), wording::OFFERED_SETTING)
                    }));
                }
            }
        }
        names
    }

    /// The fields that may be written in front matter, in the mapping under
    /// the fields `path` names.
    fn field_names(&self, path: &[String]) -> Vec<Item> {
        let mut names = Vec::new();
        if path.is_empty() {
            let words = PageWord::all().filter(|word| {
                *word != PageWord::Numbered || matches!(self.holder, Holder::FolderPage { .. })
            });
            names.extend(words.map(|word| own_field(word.name(), wording::OFFERED_FIELD)));
            if self.holder == (Holder::FolderPage { top: false }) {
                let handed = PageWord::all().filter(|word| word.handed_down());
                let handed = handed.map(|word| format!("pages.{}", word.name()));
                names.extend(handed.map(|name| own_field(&name, wording::OFFERED_HANDED)));
                names.extend(self.declared_handed());
            }
        }
        let Some(declarations) = self.declarations() else {
            return names;
        };
        let prefix: String = path.iter().map(|name| format!("{name}.")).collect();
        let mut nested = BTreeMap::new();
        for declared in declarations.names() {
            let Some(rest) = declared.strip_prefix(prefix.as_str()) else {
                continue;
            };
            let (name, deeper) = match rest.split_once('.') {
                Some((name, _)) => (name, true),
                None => (rest, false),
            };
            let detail = match deeper {
                true => None,
                false => self.kind_written(declared),
            };
            nested.entry(name.to_owned()).or_insert(detail);
        }
        names.extend(nested.into_iter().map(|(name, detail)| Item {
            label: name.clone(),
            detail,
            kind: ItemKind::Field,
            insert: name,
            group: 1,
            text: false,
            filter: None,
        }));
        names
    }

    /// `pages.` and each field the site declares that isn't nested.
    fn declared_handed(&self) -> Vec<Item> {
        let Some(declarations) = self.declarations() else {
            return Vec::new();
        };
        (declarations.names())
            .filter(|name| !name.contains('.'))
            .map(|name| Item {
                label: format!("pages.{name}"),
                detail: self.kind_written(name),
                kind: ItemKind::Field,
                insert: String::new(),
                group: 1,
                text: false,
                filter: None,
            })
            .collect()
    }

    /// The kind the field `name` is declared as, as written.
    fn kind_written(&self, name: &str) -> Option<String> {
        let renderer = self.renderer?;
        let declared = renderer.settings().declarations.declared(name)?;
        let spelling = renderer.sources().spelling(declared.place);
        Some(spelling.trim_matches(['"', '\'']).to_owned())
    }

    /// The values the setting `name` may take, `typed` being what is
    /// typed of its value so far.
    fn setting_values(&self, name: &str, typed: &str) -> Vec<Item> {
        if name == "quiet" {
            return (Code::ALL.iter())
                .filter(|code| code.quietenable())
                .map(|code| Item {
                    detail: Some(wording::severity(code.severity()).to_owned()),
                    ..own_value(code.name())
                })
                .collect();
        }
        if let Some(word) = name.strip_prefix("pages.") {
            return self.field_values(word);
        }
        if name.starts_with("fields.") || name.starts_with("types.") {
            return self.kinds(typed);
        }
        (crate::config::choices(name).into_iter())
            .map(own_value)
            .collect()
    }

    /// The kinds a field may be declared as, after `required` where
    /// `typed` begins with it.
    fn kinds(&self, typed: &str) -> Vec<Item> {
        let required = typed.starts_with("required ");
        let before = match required {
            true => "required ",
            false => "",
        };
        let one_of = format!("{before}one of");
        let mut kinds: Vec<Item> = match typed.starts_with(&one_of) {
            true => self.names_held(&one_of),
            false => (KIND_WORDS.iter())
                .map(|kind| own_value(&format!("{before}{kind}")))
                .collect(),
        };
        if typed.is_empty() {
            kinds.push(own_value("required"));
        }
        kinds
    }

    /// What `one of` may name: each table of the site's data and each
    /// folder of its pages, written after `one_of`.
    fn names_held(&self, one_of: &str) -> Vec<Item> {
        let Some(renderer) = self.renderer else {
            return Vec::new();
        };
        let site = renderer.site();
        let tables =
            (site.data.names().into_iter()).map(|name| own_value(&format!("{one_of} data.{name}")));
        let folders = (site.lists.iter()).filter_map(|(_, list)| match &list.key {
            ListKey::Folder(path) => {
                let below = path[site.top_above(path).len()..].trim_start_matches('/');
                (!below.is_empty()).then(|| format!("{one_of} folder.{below}"))
            }
            _ => None,
        });
        let mut folders: Vec<String> = folders.collect();
        folders.sort();
        folders.dedup();
        tables
            .chain(folders.iter().map(|folder| own_value(folder)))
            .collect()
    }

    /// The values the field `name` may take in front matter.
    fn field_values(&self, name: &str) -> Vec<Item> {
        if let Some(word) = PageWord::named(name) {
            let mut values: Vec<Item> = word.choices().into_iter().map(own_value).collect();
            values.extend(match word {
                PageWord::Tags => self.lists(
                    |key| match key {
                        ListKey::Tag(name) => Some(name),
                        _ => None,
                    },
                    wording::offered_tag,
                ),
                PageWord::Series => self.lists(
                    |key| match key {
                        ListKey::Series(name) => Some(name),
                        _ => None,
                    },
                    wording::offered_series,
                ),
                PageWord::Layout => self.layouts(),
                PageWord::Type => self.types(),
                _ => Vec::new(),
            });
            return values;
        }
        let Some(kind) = self.declarations().and_then(|d| d.kind(name)) else {
            return Vec::new();
        };
        match kind {
            FieldKind::TrueFalse => vec![own_value("true"), own_value("false")],
            FieldKind::OneOf(words) => (words.iter())
                .map(|word| Item {
                    text: true,
                    ..own_value(word)
                })
                .collect(),
            FieldKind::Named(names) => self.named(names),
            _ => Vec::new(),
        }
    }

    /// The site's tags or series, as `which` picks them from the lists,
    /// with how many pages carry each, as `said`, in the page's language
    /// where it is known.
    fn lists(
        &self,
        which: impl Fn(&ListKey) -> Option<&String>,
        said: fn(usize) -> String,
    ) -> Vec<Item> {
        let Some(renderer) = self.renderer else {
            return Vec::new();
        };
        let mut found: BTreeMap<&str, usize> = BTreeMap::new();
        for (_, list) in renderer.site().lists.iter() {
            if self
                .language
                .is_some_and(|language| language != list.language)
            {
                continue;
            }
            if let Some(name) = which(&list.key) {
                *found.entry(name).or_default() += list.members.len() + list.undated.len();
            }
        }
        (found.into_iter())
            .map(|(name, pages)| Item {
                detail: Some(said(pages)),
                group: 1,
                text: true,
                ..own_value(name)
            })
            .collect()
    }

    fn layouts(&self) -> Vec<Item> {
        let Some(renderer) = self.renderer else {
            return Vec::new();
        };
        (renderer.layouts().into_iter())
            .map(|(name, path)| Item {
                detail: Some(path),
                group: 1,
                text: true,
                ..own_value(&name)
            })
            .collect()
    }

    fn types(&self) -> Vec<Item> {
        let Some(declarations) = self.declarations() else {
            return Vec::new();
        };
        (declarations.types())
            .map(|name| Item {
                detail: Some(wording::OFFERED_TYPE.to_owned()),
                group: 1,
                text: true,
                ..own_value(name)
            })
            .collect()
    }

    /// The names a field declared `one of` a table or a folder may take.
    fn named(&self, names: &Names) -> Vec<Item> {
        let Some(renderer) = self.renderer else {
            return Vec::new();
        };
        let site = renderer.site();
        let (held, detail): (Vec<String>, String) = match names {
            Names::Data(table) => {
                use crate::data::Reached;
                use crate::yaml::Value as Yaml;
                match site.data.reach(&table.split('.').collect::<Vec<_>>()) {
                    Some(Reached::Value { file, node }) => {
                        let held = match &node.value {
                            Yaml::Mapping(entries) => (entries.iter())
                                .filter_map(|entry| match &entry.key.value {
                                    Yaml::Scalar(name) => Some(name.clone()),
                                    _ => None,
                                })
                                .collect(),
                            _ => Vec::new(),
                        };
                        (held, site.data.file(file).path.clone())
                    }
                    Some(Reached::Folder(folder)) => {
                        let held = folder.entries.keys().cloned().collect();
                        (held, folder.path.clone())
                    }
                    _ => return Vec::new(),
                }
            }
            Names::Folder(path) => {
                let mut held = Vec::new();
                for (language, _) in site.languages.iter() {
                    if self.language.is_some_and(|own| own != language) {
                        continue;
                    }
                    let rel = match site.top(language) {
                        "" => format!("{path}/"),
                        top => format!("{top}/{path}/"),
                    };
                    held.extend(site.page_names.get(&rel).into_iter().flatten().cloned());
                }
                held.sort();
                held.dedup();
                (held, format!("content/{path}/"))
            }
        };
        (held.iter())
            .map(|name| Item {
                detail: Some(detail.clone()),
                group: 1,
                text: true,
                ..own_value(name)
            })
            .collect()
    }
}

/// One of Statac's own fields or settings.
fn own_field(name: &str, detail: &str) -> Item {
    Item {
        label: name.to_owned(),
        detail: Some(detail.to_owned()),
        kind: ItemKind::Field,
        insert: String::new(),
        group: 0,
        text: false,
        filter: None,
    }
}

/// One of the values Statac takes.
fn own_value(value: &str) -> Item {
    Item {
        label: value.to_owned(),
        detail: None,
        kind: ItemKind::Value,
        insert: String::new(),
        group: 0,
        text: false,
        filter: None,
    }
}
