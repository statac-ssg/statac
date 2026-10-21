//! Hugo's settings: one file at the top of the site, or split over
//! `config/_default/`, a file for each setting beside the main one, read
//! together. Hugo reads its settings' names in any capitals.

use std::path::Path;

use super::super::common::{fields, text_of, texts_of};
use super::super::{Failed, Field, Left, Value, inside, toml, yaml};
use crate::problem::{Kind, Problem, Reason};
use crate::source::Sources;
use crate::yaml::Value as Yaml;

/// The settings files Hugo reads at the top of a site, the first found.
pub(super) const FILES: &[&str] = &[
    "hugo.toml",
    "hugo.yaml",
    "hugo.yml",
    "hugo.json",
    "config.toml",
    "config.yaml",
    "config.yml",
    "config.json",
];

/// Those only Hugo names so.
const HUGOS_OWN_FILES: &[&str] = &["hugo.toml", "hugo.yaml", "hugo.yml", "hugo.json"];

/// The folder of settings every environment starts from.
pub(super) const DEFAULT_FOLDER: &str = "config/_default";

/// Whether `folder` holds a Hugo site, by its settings.
pub(super) fn owns(folder: &Path) -> bool {
    HUGOS_OWN_FILES
        .iter()
        .any(|n| inside::a_settings_file(folder, n))
        || inside::a_settings_folder(folder, DEFAULT_FOLDER)
        || FILES
            .iter()
            .filter(|n| !HUGOS_OWN_FILES.contains(n) && inside::a_settings_file(folder, n))
            .any(|n| {
                fields_of(folder, n).is_ok_and(|fields| {
                    fields
                        .iter()
                        .any(|f| f.name.eq_ignore_ascii_case("baseurl"))
                })
            })
}

/// The fields of the settings or data file `rel` of `folder`, by its
/// ending: TOML, YAML or JSON.
pub(super) fn fields_of(folder: &Path, rel: &str) -> Result<Vec<Field>, Box<Failed>> {
    let path = inside::path(folder, rel);
    let text = std::fs::read_to_string(&path).map_err(|error| {
        Box::<Failed>::from(Problem::nowhere(Kind::CantRead {
            path: folder.join(rel),
            reason: Reason::of(&error),
        }))
    })?;
    if rel.ends_with(".toml") {
        return toml::read(&text)
            .map_err(|stopped| toml::failed(&folder.join(rel), text, &stopped));
    }
    let mut sources = Sources::default();
    let source = sources.add(folder.join(rel), text);
    let read = match rel.ends_with(".json") {
        true => crate::yaml::read_json(&sources, source),
        false => crate::yaml::read(&sources, source),
    };
    let node = match read {
        Ok(node) => node,
        Err(problem) => return Err(Box::new(Failed { sources, problem })),
    };
    Ok(match &node.value {
        Yaml::Mapping(entries) => entries
            .iter()
            .map(|entry| {
                let line = sources.line_number(source, entry.key.place.span.start as usize);
                let value = yaml::value_of(&entry.value, &yaml::core);
                Field::new(&yaml::key_text(&entry.key), value, Some(line))
            })
            .collect(),
        _ => Vec::new(),
    })
}

/// A setting as found: its name, its value, and the file and line it is
/// written at.
#[derive(Clone, Debug)]
pub(super) struct Entry {
    pub field: Field,
    pub file: String,
}

/// Every setting of the site, the main file's first, and the files and
/// folders of settings Hugo doesn't read for the site as built.
pub(super) struct Found {
    pub entries: Vec<Entry>,
    pub file: String,
    pub left: Vec<Left>,
}

/// Reads every settings file Hugo reads in `folder`.
pub(super) fn found(folder: &Path) -> Result<Found, Box<Failed>> {
    let mut entries: Vec<Entry> = Vec::new();
    let mut left = Vec::new();
    let mut main = None;
    for name in FILES {
        if !inside::a_settings_file(folder, name) {
            continue;
        }
        if main.is_some() {
            left.push(Left {
                path: (*name).to_owned(),
                what: "another settings file, which Hugo reads only where the one before it \
                       isn't there"
                    .to_owned(),
            });
            continue;
        }
        main = Some((*name).to_owned());
        for field in fields_of(folder, name)? {
            merge(&mut entries, field, name);
        }
    }
    let split = inside::path(folder, DEFAULT_FOLDER);
    if inside::a_settings_folder(folder, DEFAULT_FOLDER) {
        let mut names = names_in(&split);
        // The main file first, so a file for one setting adds to it.
        names.sort_by_key(|name| !matches!(stem(name).as_str(), "hugo" | "config"));
        for name in names {
            let rel = format!("{DEFAULT_FOLDER}/{name}");
            if !inside::reached(folder, &rel) {
                left.push(Left {
                    what: inside::why(folder, &rel).to_owned(),
                    path: rel,
                });
                continue;
            }
            let ending = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase());
            if !matches!(ending.as_deref(), Some("toml" | "yaml" | "yml" | "json")) {
                left.push(Left {
                    path: rel,
                    what: "a file among the settings that Hugo doesn't read".to_owned(),
                });
                continue;
            }
            let fields = fields_of(folder, &rel)?;
            let stem = stem(&name);
            let (key, language) = match stem.split_once('.') {
                Some((key, language)) => (key.to_owned(), Some(language.to_owned())),
                None => (stem.clone(), None),
            };
            if main.is_none() || matches!(key.as_str(), "hugo" | "config") {
                main.get_or_insert_with(|| rel.clone());
            }
            match (key.as_str(), language) {
                ("hugo" | "config", None) => {
                    for field in fields {
                        merge(&mut entries, field, &rel);
                    }
                }
                (_, language) => {
                    let value =
                        Value::Fields(fields.into_iter().map(|f| (f.name, f.value)).collect());
                    let field = match language {
                        Some(language) => Field::new(
                            "languages",
                            Value::Fields(vec![(
                                language,
                                Value::Fields(vec![(key.clone(), value)]),
                            )]),
                            None,
                        ),
                        None => Field::new(&key, value, None),
                    };
                    merge(&mut entries, field, &rel);
                }
            }
        }
    }
    let config = inside::path(folder, "config");
    if inside::a_settings_folder(folder, "config") {
        for name in names_in(&config) {
            if name == "_default" {
                continue;
            }
            left.push(Left {
                path: format!("config/{name}/"),
                what: "the settings of another of the old site's environments, which Hugo \
                       reads over the default ones only when asked"
                    .to_owned(),
            });
        }
    }
    Ok(Found {
        entries,
        file: main.unwrap_or_default(),
        left,
    })
}

/// The names in a folder, A to Z.
fn names_in(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(folder)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// A file's name without its ending, in small letters.
fn stem(name: &str) -> String {
    name.rsplit_once('.')
        .map_or(name, |(stem, _)| stem)
        .to_ascii_lowercase()
}

/// Adds `field` to `entries`: a setting already there keeps its value,
/// and a table already there takes what it lacks of another.
fn merge(entries: &mut Vec<Entry>, field: Field, file: &str) {
    match entries
        .iter_mut()
        .find(|entry| entry.field.name.eq_ignore_ascii_case(&field.name))
    {
        Some(entry) => {
            if let (Value::Fields(held), Value::Fields(more)) =
                (&mut entry.field.value, field.value)
            {
                merge_fields(held, more);
            }
        }
        None => entries.push(Entry {
            field,
            file: file.to_owned(),
        }),
    }
}

fn merge_fields(held: &mut Vec<(String, Value)>, more: Vec<(String, Value)>) {
    for (name, value) in more {
        match held.iter_mut().find(|(n, _)| n.eq_ignore_ascii_case(&name)) {
            Some((_, Value::Fields(inner))) => {
                if let Value::Fields(more) = value {
                    merge_fields(inner, more);
                }
            }
            Some(_) => {}
            None => held.push((name, value)),
        }
    }
}

/// Whether a value is true, as Hugo reads one written as text too.
pub(super) fn truth_of(value: &Value) -> bool {
    match value {
        Value::Truth(truth) => *truth,
        Value::Text(text) => text.eq_ignore_ascii_case("true"),
        _ => false,
    }
}

/// The field `name` of a table, in any capitals.
pub(super) fn get<'a>(fields: &'a [(String, Value)], name: &str) -> Option<&'a Value> {
    fields
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v)
}

/// Where each kind of page goes, by `[permalinks]`: patterns by the name
/// of a folder or a list of terms.
#[derive(Default, Debug)]
pub(super) struct Permalinks {
    /// Written straight under `[permalinks]`, for a folder's pages or a
    /// list of terms.
    pub any: Vec<(String, String)>,
    pub page: Vec<(String, String)>,
    pub section: Vec<(String, String)>,
    pub term: Vec<(String, String)>,
}

impl Permalinks {
    /// The pattern for a page in the folder `section`.
    pub(super) fn page(&self, section: &str) -> Option<&str> {
        find(&self.page, section).or_else(|| find(&self.any, section))
    }

    /// The pattern for a folder's own page.
    pub(super) fn section(&self, section: &str) -> Option<&str> {
        find(&self.section, section)
    }

    /// The pattern for a term of the list `plural`.
    pub(super) fn term(&self, plural: &str) -> Option<&str> {
        find(&self.term, plural).or_else(|| find(&self.any, plural))
    }
}

fn find<'a>(patterns: &'a [(String, String)], name: &str) -> Option<&'a str> {
    patterns
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, p)| p.as_str())
}

/// A language the settings declare.
#[derive(Clone, Debug)]
pub(super) struct Language {
    /// Its name among the settings, which its folder and addresses use.
    pub key: String,
    pub title: Option<String>,
    pub content_dir: Option<String>,
    pub weight: i64,
}

/// Which pages Hugo published as their own file rather than a folder.
#[derive(Default, Debug)]
pub(super) struct Ugly {
    pub all: bool,
    pub folders: Vec<(String, bool)>,
}

impl Ugly {
    pub(super) fn of(&self, section: &str) -> bool {
        self.folders
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(section))
            .map_or(self.all, |(_, ugly)| *ugly)
    }
}

/// What the settings say.
#[derive(Debug)]
pub(super) struct Settings {
    pub base_url: Option<String>,
    pub title: Option<String>,
    pub language_code: Option<String>,
    pub default_language: Option<String>,
    pub default_in_subdir: bool,
    pub timezone: Option<String>,
    pub permalinks: Permalinks,
    /// Each list of terms, by its name in one and in many.
    pub taxonomies: Vec<(String, String)>,
    pub pager_size: Option<u64>,
    pub pager_path: Option<String>,
    pub ugly: Ugly,
    pub lower: bool,
    pub accents_removed: bool,
    /// Where each date of a page is taken from, by the date's name.
    pub dates: Vec<(String, Vec<String>)>,
    pub languages: Vec<Language>,
    pub params: Vec<Field>,
    pub params_file: String,
    pub content_dir: String,
    pub static_dirs: Vec<String>,
    pub data_dir: String,
    pub publish_dir: String,
    pub layout_dir: String,
    pub asset_dir: String,
    pub i18n_dir: String,
    pub archetype_dir: String,
    pub themes_dir: String,
    pub resource_dir: String,
    pub summary_length: usize,
    pub emoji: bool,
    pub unsafe_html: bool,
    /// Whether the settings' `passthrough` hands maths between `$` and
    /// `$`, and between `$$` and `$$`, through to the page to be set.
    pub maths: (bool, bool),
    pub block_attributes: bool,
    pub feeds: bool,
    /// Whether `outputs` gives a feed to the home page, to each list, and
    /// to each tag.
    pub feeds_for: (bool, bool, bool),
    /// The name of a feed's file, which `outputFormats` can change.
    pub feed_name: String,
    pub terms: bool,
    pub git: bool,
    /// Settings Statac has no equal for, each with its file, line and text.
    pub left: Vec<(String, Option<usize>, String)>,
    /// Settings naming a folder outside the old site or through a link,
    /// each with its file, line and text.
    pub unread: Vec<(String, Option<usize>, String)>,
}

/// Hugo's lists of where each date is found, unless the settings say.
pub(super) const DEFAULT_DATES: &[(&str, &[&str])] = &[
    (
        "date",
        &[
            "date",
            "publishdate",
            "pubdate",
            "published",
            "lastmod",
            "modified",
        ],
    ),
    (
        "lastmod",
        &[
            ":git",
            "lastmod",
            "modified",
            "date",
            "publishdate",
            "pubdate",
            "published",
        ],
    ),
    (
        "publishdate",
        &["publishdate", "pubdate", "published", "date"],
    ),
    ("expirydate", &["expirydate", "unpublishdate"]),
];

impl Settings {
    /// Where the date `name` is found, in order.
    pub(super) fn date_sources(&self, name: &str) -> Vec<String> {
        let default: Vec<String> = DEFAULT_DATES
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, list)| list.iter().map(|s| (*s).to_owned()).collect())
            .unwrap_or_default();
        match self.dates.iter().find(|(n, _)| n == name) {
            None => default,
            Some((_, list)) => list
                .iter()
                .flat_map(|source| match source.as_str() {
                    ":default" => default.clone(),
                    other => vec![other.to_owned()],
                })
                .collect(),
        }
    }

    /// The plural names of the lists of terms.
    pub(super) fn plurals(&self) -> Vec<&str> {
        self.taxonomies.iter().map(|(_, p)| p.as_str()).collect()
    }
}

/// What `entries` say.
pub(super) fn settle(folder: &Path, entries: &[Entry]) -> Settings {
    let mut s = Settings {
        base_url: None,
        title: None,
        language_code: None,
        default_language: None,
        default_in_subdir: false,
        timezone: None,
        permalinks: Permalinks::default(),
        taxonomies: vec![
            ("tag".into(), "tags".into()),
            ("category".into(), "categories".into()),
        ],
        pager_size: None,
        pager_path: None,
        ugly: Ugly::default(),
        lower: true,
        accents_removed: false,
        dates: Vec::new(),
        languages: Vec::new(),
        params: Vec::new(),
        params_file: String::new(),
        content_dir: "content".into(),
        static_dirs: vec!["static".into()],
        data_dir: "data".into(),
        publish_dir: "public".into(),
        layout_dir: "layouts".into(),
        asset_dir: "assets".into(),
        i18n_dir: "i18n".into(),
        archetype_dir: "archetypes".into(),
        themes_dir: "themes".into(),
        resource_dir: "resources".into(),
        summary_length: 70,
        emoji: false,
        unsafe_html: false,
        maths: (false, false),
        block_attributes: false,
        feeds: true,
        feeds_for: (true, true, true),
        feed_name: "index.xml".into(),
        terms: true,
        git: false,
        left: Vec::new(),
        unread: Vec::new(),
    };
    let old = folder;
    let unread = std::cell::RefCell::new(Vec::new());
    for Entry { field, file } in entries {
        let value = &field.value;
        let text = text_of(value);
        // A folder a setting names, where it is inside the old site and
        // no link; else the setting is told of, and read as not set.
        let folder = |name: &str, value: &Value| {
            let written = text_of(value)?;
            let named = inside::named_folder(old, &written);
            if named.is_none() {
                let said = format!("{name}: {written}");
                unread.borrow_mut().push((file.clone(), field.line, said));
            }
            named
        };
        let mut leave = |text: String| s.left.push((file.clone(), field.line, text));
        match field.name.to_ascii_lowercase().as_str() {
            "baseurl" => s.base_url = text.filter(|t| !t.is_empty()),
            "title" => s.title = text,
            "languagecode" => s.language_code = text,
            "defaultcontentlanguage" => s.default_language = text.map(|t| t.to_lowercase()),
            "defaultcontentlanguageinsubdir" => s.default_in_subdir = truth_of(value),
            "timezone" => s.timezone = text,
            "permalinks" => {
                for (name, pattern) in fields(value) {
                    let list = match name.to_ascii_lowercase().as_str() {
                        "page" => &mut s.permalinks.page,
                        "section" => &mut s.permalinks.section,
                        "term" => &mut s.permalinks.term,
                        "taxonomy" => {
                            leave(format!(
                                "permalinks.taxonomy: {}",
                                super::super::write::shown_value(pattern)
                            ));
                            continue;
                        }
                        _ => {
                            if let Some(pattern) = text_of(pattern) {
                                s.permalinks.any.push((name.clone(), pattern));
                            }
                            continue;
                        }
                    };
                    for (name, pattern) in fields(pattern) {
                        if let Some(pattern) = text_of(pattern) {
                            list.push((name.clone(), pattern));
                        }
                    }
                }
            }
            "taxonomies" => {
                s.taxonomies = fields(value)
                    .iter()
                    .filter_map(|(one, many)| {
                        Some((one.to_lowercase(), text_of(many)?.to_lowercase()))
                    })
                    .collect();
            }
            "paginate" => s.pager_size = text.and_then(|n| n.parse().ok()),
            "paginatepath" => s.pager_path = text,
            "pagination" => {
                let table = fields(value);
                if let Some(size) = get(table, "pagerSize").and_then(text_of) {
                    s.pager_size = size.parse().ok();
                }
                if let Some(path) = get(table, "path").and_then(text_of) {
                    s.pager_path = Some(path);
                }
            }
            "uglyurls" => match value {
                Value::Fields(table) => {
                    s.ugly.folders = table
                        .iter()
                        .map(|(name, ugly)| (name.clone(), truth_of(ugly)))
                        .collect();
                }
                other => s.ugly.all = truth_of(other),
            },
            "disablepathtolower" => s.lower = !truth_of(value),
            "removepathaccents" => s.accents_removed = truth_of(value),
            "frontmatter" => {
                s.dates = fields(value)
                    .iter()
                    .map(|(name, list)| {
                        let list = texts_of(list).iter().map(|t| t.to_lowercase()).collect();
                        (name.to_lowercase(), list)
                    })
                    .collect();
            }
            "languages" => {
                for (key, table) in fields(value) {
                    let table = fields(table);
                    let mut language = Language {
                        key: key.to_lowercase(),
                        title: None,
                        content_dir: None,
                        weight: 0,
                    };
                    for (name, value) in table {
                        match name.to_ascii_lowercase().as_str() {
                            "title" => language.title = text_of(value),
                            "contentdir" => {
                                language.content_dir =
                                    folder(&format!("languages.{key}.{name}"), value)
                            }
                            "weight" => {
                                language.weight =
                                    text_of(value).and_then(|w| w.parse().ok()).unwrap_or(0)
                            }
                            "languagename" | "languagecode" | "languagedirection" | "label"
                            | "direction" | "locale" => {}
                            _ => leave(format!(
                                "languages.{key}.{name}: {}",
                                super::super::write::shown_value(value)
                            )),
                        }
                    }
                    s.languages.push(language);
                }
            }
            "params" => {
                s.params_file = file.clone();
                s.params = fields(value)
                    .iter()
                    .map(|(name, value)| Field::new(name, value.clone(), field.line))
                    .collect();
            }
            "contentdir" => {
                s.content_dir = folder(&field.name, value).unwrap_or(s.content_dir.clone())
            }
            "staticdir" => {
                s.static_dirs = texts_of(value)
                    .iter()
                    .filter_map(|dir| folder(&field.name, &Value::Text(dir.clone())))
                    .collect()
            }
            "datadir" => s.data_dir = folder(&field.name, value).unwrap_or(s.data_dir.clone()),
            "publishdir" => {
                s.publish_dir = folder(&field.name, value).unwrap_or(s.publish_dir.clone())
            }
            "layoutdir" => {
                s.layout_dir = folder(&field.name, value).unwrap_or(s.layout_dir.clone())
            }
            "assetdir" => s.asset_dir = folder(&field.name, value).unwrap_or(s.asset_dir.clone()),
            "i18ndir" => s.i18n_dir = folder(&field.name, value).unwrap_or(s.i18n_dir.clone()),
            "archetypedir" => {
                s.archetype_dir = folder(&field.name, value).unwrap_or(s.archetype_dir.clone())
            }
            "themesdir" => {
                s.themes_dir = folder(&field.name, value).unwrap_or(s.themes_dir.clone())
            }
            "resourcedir" => {
                s.resource_dir = folder(&field.name, value).unwrap_or(s.resource_dir.clone())
            }
            "summarylength" => s.summary_length = text.and_then(|n| n.parse().ok()).unwrap_or(70),
            "enableemoji" => s.emoji = truth_of(value),
            "enablegitinfo" => s.git = truth_of(value),
            "disablekinds" => {
                for kind in texts_of(value) {
                    match kind.to_ascii_lowercase().as_str() {
                        "rss" => s.feeds = false,
                        "term" | "taxonomyterm" => s.terms = false,
                        _ => {}
                    }
                }
                leave(format!(
                    "{}: {}",
                    field.name,
                    super::super::write::shown_value(value)
                ));
            }
            "outputs" => {
                let mut rest = Vec::new();
                for (kind, formats) in fields(value) {
                    let formats = texts_of(formats);
                    let feed = formats.iter().any(|f| f.eq_ignore_ascii_case("rss"));
                    match kind.to_ascii_lowercase().as_str() {
                        "home" => s.feeds_for.0 = feed,
                        "section" => s.feeds_for.1 = feed,
                        "term" => s.feeds_for.2 = feed,
                        _ => {}
                    }
                    let others: Vec<Value> = (formats.iter())
                        .filter(|f| !["html", "rss"].contains(&f.to_lowercase().as_str()))
                        .map(|f| Value::Text(f.clone()))
                        .collect();
                    if !others.is_empty() {
                        rest.push((kind.clone(), Value::List(others)));
                    }
                }
                if !rest.is_empty() {
                    leave(format!(
                        "{}: {}",
                        field.name,
                        super::super::write::shown_value(&Value::Fields(rest))
                    ));
                }
            }
            "outputformats" => {
                let mut rest = value.clone();
                let rss = get(fields(value), "rss").map(fields).unwrap_or_default();
                // Only a plain name is kept: it is written into the new
                // site as part of an address.
                let plain = |name: &String| {
                    !name.is_empty()
                        && !name.starts_with('.')
                        && (name.chars()).all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
                };
                if let Some(name) = get(rss, "basename").and_then(text_of).filter(plain) {
                    s.feed_name = format!("{name}.xml");
                    take(&mut rest, &["rss", "basename"]);
                }
                if rest != Value::Fields(Vec::new()) {
                    leave(format!(
                        "{}: {}",
                        field.name,
                        super::super::write::shown_value(&rest)
                    ));
                }
            }
            "markup" => {
                let mut rest = value.clone();
                let goldmark = get(fields(value), "goldmark")
                    .map(fields)
                    .unwrap_or_default();
                let renderer = get(goldmark, "renderer").map(fields).unwrap_or_default();
                let parser = get(goldmark, "parser").map(fields).unwrap_or_default();
                let attribute = get(parser, "attribute").map(fields).unwrap_or_default();
                s.unsafe_html = get(renderer, "unsafe").is_some_and(truth_of);
                let extensions = get(goldmark, "extensions").map(fields).unwrap_or_default();
                let passthrough = get(extensions, "passthrough")
                    .map(fields)
                    .unwrap_or_default();
                if get(passthrough, "enable").is_some_and(truth_of) {
                    let delimiters = get(passthrough, "delimiters")
                        .map(fields)
                        .unwrap_or_default();
                    let opened_by = |kind: &str, opening: &str| {
                        get(delimiters, kind).is_some_and(|pairs| match pairs {
                            Value::List(pairs) => pairs.iter().any(|pair| {
                                matches!(pair, Value::List(pair)
                                    if pair.first().and_then(text_of).as_deref() == Some(opening))
                            }),
                            _ => false,
                        })
                    };
                    s.maths = (opened_by("inline", "$"), opened_by("block", "$$"));
                }
                s.block_attributes = get(attribute, "block").is_some_and(truth_of);
                take(&mut rest, &["goldmark", "renderer", "unsafe"]);
                if s.maths != (false, false) {
                    take(&mut rest, &["goldmark", "extensions", "passthrough"]);
                }
                take(&mut rest, &["goldmark", "parser", "attribute", "block"]);
                if rest != Value::Fields(Vec::new()) {
                    leave(format!(
                        "{}: {}",
                        field.name,
                        super::super::write::shown_value(&rest)
                    ));
                }
            }
            _ => leave(format!(
                "{}: {}",
                field.name,
                super::super::write::shown_value(value)
            )),
        }
    }
    s.unread = unread.into_inner();
    s
}

/// Takes the field at `path` out of `value`, and every table it leaves
/// empty.
fn take(value: &mut Value, path: &[&str]) {
    let Value::Fields(fields) = value else { return };
    let Some((first, rest)) = path.split_first() else {
        return;
    };
    let Some(at) = fields
        .iter()
        .position(|(n, _)| n.eq_ignore_ascii_case(first))
    else {
        return;
    };
    if rest.is_empty() {
        fields.remove(at);
        return;
    }
    take(&mut fields[at].1, rest);
    if fields[at].1 == Value::Fields(Vec::new()) {
        fields.remove(at);
    }
}
