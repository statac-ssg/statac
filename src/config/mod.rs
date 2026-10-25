//! The site's settings, read from `statac.yaml`.
//!
//! Statac's own settings are flat, one line each (`tags.permalink:`), and
//! typed; a setting nobody wrote takes Statac's default, marked as such.
//! `pages.<word>` lines are handed down to every page. On a site in
//! several languages, `<tag>.<setting>` writes a setting readers see for
//! one language. Every other key is the author's own value, reaching
//! templates as `site.<name>`, and may nest.

pub mod declarations;

use std::path::Path;

use crate::content::words::{self, Field, Words};
use crate::origin::{Origin, Sourced};
use crate::problem::{Expected, Kind, Own, Problem, Root, Word, WrongPlace};
use crate::source::{SourceId, Sources};
use crate::value::{Reader, Value};
use crate::yaml::{Node, Value as Yaml};
use declarations::Declarations;

pub const FILE_NAME: &str = "statac.yaml";

/// Names the settings were once read under, refused by name.
const FORMER_NAMES: &[&str] = &["statac.yml", "statac.conf"];

/// The names Statac gives meanings to on `site`, which the author's own
/// values can't take.
const RESERVED: &[&str] = &[
    "pages",
    "posts",
    "folders",
    "tags",
    "series",
    "pagination",
    "feed",
    "feeds",
    "host",
    "data",
    "types",
    "fields",
    "today",
    "languages",
];

/// The settings readers see that a site in several languages may write for
/// one of them, as `<tag>.<setting>`.
pub const PER_LANGUAGE: &[&str] = &["title", "date_format"];

/// The settings that hold a credential: never quoted in a problem, and
/// hidden wherever their line is drawn.
const SECRETS: &[&str] = &[TOKEN];

/// The token the webmention relay gives, which `statac refresh` takes from
/// the environment or the site's `.statac/token`, and the settings may
/// not hold.
const TOKEN: &str = "webmention_token";

/// Other spellings of Statac's settings, and the names some were once
/// read under.
const OTHER_SPELLINGS: &[(&str, &str)] = &[
    ("jpeg_max_width", "max_width"),
    ("pages.later", "pagination.permalink"),
    ("later_pages", "pagination.permalink"),
    ("left_behind", "exclude"),
    ("also_at", "other_hosts"),
    ("not_ours", "served_elsewhere"),
    ("after_build_makes", "after_build_outputs"),
];

#[derive(Copy, Clone, Debug)]
enum Shape {
    Text,
    Truth,
    Whole(u64),
    Texts,
    Date,
    SiteAddress,
    OneOf(&'static [&'static str]),
    Language,
    /// Full URLs, each told where it isn't one.
    WebAddresses,
    ListAddress,
    LaterAddress,
    Table,
    /// Files and folders, each of them what the test passes, told as
    /// what was expected where one isn't.
    Places(fn(&str) -> bool, PlaceIn),
}

/// Where a setting's files and folders are.
#[derive(Copy, Clone, Debug)]
enum PlaceIn {
    BuiltSite,
    Site,
}

#[derive(Copy, Clone, Debug)]
enum Preset {
    Text(&'static str),
    Truth(bool),
    Whole(u64),
    Texts(&'static [&'static str]),
}

struct Setting {
    name: &'static str,
    shape: Shape,
    default: Option<Preset>,
}

const fn setting(name: &'static str, shape: Shape, default: Option<Preset>) -> Setting {
    Setting {
        name,
        shape,
        default,
    }
}

const SETTINGS: &[Setting] = &[
    setting("title", Shape::Text, None),
    setting("url", Shape::SiteAddress, None),
    setting("other_hosts", Shape::Texts, None),
    setting("served_elsewhere", Shape::Texts, None),
    setting("language", Shape::Language, None),
    setting("timezone", Shape::Text, None),
    setting("date_format", Shape::Text, None),
    setting("date_zeros", Shape::Truth, Some(Preset::Truth(true))),
    setting(
        "code_style",
        Shape::OneOf(crate::highlight::SETTING),
        Some(Preset::Text("ocean")),
    ),
    setting("typography", Shape::Truth, Some(Preset::Truth(true))),
    setting("optimise_images", Shape::Truth, Some(Preset::Truth(true))),
    setting("before_build", Shape::Text, None),
    setting("after_build", Shape::Text, None),
    setting(
        "after_build_outputs",
        Shape::Places(crate::hooks::a_built_place, PlaceIn::BuiltSite),
        None,
    ),
    setting(
        "before_build_watches",
        Shape::Places(crate::hooks::a_place, PlaceIn::Site),
        None,
    ),
    setting(
        "search_without_javascript",
        Shape::OneOf(crate::search::elsewhere::SETTING),
        None,
    ),
    setting("webmentions", Shape::Truth, Some(Preset::Truth(false))),
    setting("refuse_webmentions_from", Shape::WebAddresses, None),
    setting("feed_key", Shape::Text, None),
    setting("feed_ids_before", Shape::Date, None),
    setting("feed_ids_from", Shape::OneOf(&["jekyll"]), None),
    setting("write_feed_ids", Shape::Truth, Some(Preset::Truth(false))),
    setting(
        "tags.permalink",
        Shape::ListAddress,
        Some(Preset::Text("/tags/{slug}/")),
    ),
    setting("tags.pages", Shape::Truth, Some(Preset::Truth(true))),
    setting("tags.slugs", Shape::Table, None),
    setting(
        "series.permalink",
        Shape::ListAddress,
        Some(Preset::Text("/series/{slug}/")),
    ),
    setting("series.pages", Shape::Truth, Some(Preset::Truth(true))),
    setting("series.slugs", Shape::Table, None),
    setting("posts_per_page", Shape::Whole(0), Some(Preset::Whole(10))),
    setting(
        "pagination.permalink",
        Shape::LaterAddress,
        Some(Preset::Text("{list}page/{n}/")),
    ),
    setting(
        "image_sizes",
        Shape::Text,
        Some(Preset::Text("(max-width: 46rem) 100vw, 46rem")),
    ),
    setting(
        "exclude",
        Shape::Texts,
        Some(Preset::Texts(&[".DS_Store", "Thumbs.db", "desktop.ini"])),
    ),
    setting("max_width", Shape::Whole(1), Some(Preset::Whole(1600))),
    setting("quiet", Shape::Texts, None),
];

/// The names of Statac's own site settings.
pub fn setting_names() -> impl Iterator<Item = &'static str> {
    SETTINGS.iter().map(|s| s.name)
}

/// The values the setting `name` takes where they are a few words to
/// choose from.
pub fn choices(name: &str) -> Vec<&'static str> {
    match SETTINGS.iter().find(|s| s.name == name).map(|s| s.shape) {
        Some(Shape::Truth) => vec!["true", "false"],
        Some(Shape::OneOf(words)) => words.to_vec(),
        _ => Vec::new(),
    }
}

/// Whether `name` is one of Statac's own site settings.
pub fn is_setting(name: &str) -> bool {
    SETTINGS.iter().any(|s| s.name == name)
}

/// The site's settings, each with where it came from.
#[derive(Clone, Default, Debug)]
pub struct Settings {
    values: Vec<(&'static str, Sourced<Value>)>,
    /// Settings written for one language, by its tag as declared.
    for_languages: Vec<(String, &'static str, Sourced<Value>)>,
    /// What `pages.` lines hand to every page.
    pub pages: Words,
    pub declarations: Declarations,
    /// The author's own values, as `site.<name>`.
    pub own: Vec<Field>,
    /// `statac.yaml`, where there is one.
    pub source: Option<SourceId>,
    /// Whether something was told of the `language` line as written.
    pub language_told: bool,
    /// Where secrets are written, to be hidden from every drawn line.
    pub secrets: Vec<crate::source::Place>,
    /// The `url` the settings wrote, where the run was given another
    /// address; `None` within where they wrote none.
    replaced: Option<Option<Sourced<Value>>>,
}

impl Settings {
    pub fn get(&self, name: &str) -> Option<&Sourced<Value>> {
        self.values.iter().find(|(n, _)| *n == name).map(|(_, v)| v)
    }

    pub fn text(&self, name: &str) -> Option<Sourced<&str>> {
        let value = self.get(name)?;
        Some(Sourced::new(value.value.as_text()?, value.origin.clone()))
    }

    pub fn truth(&self, name: &str) -> Option<bool> {
        self.get(name)?.value.as_bool()
    }

    pub fn whole(&self, name: &str) -> Option<u64> {
        self.get(name)?.value.as_whole()
    }

    pub fn texts(&self, name: &str) -> &[Sourced<String>] {
        self.get(name)
            .and_then(|v| v.value.as_texts())
            .unwrap_or_default()
    }

    /// The languages the site declares, main first, each as written.
    pub fn languages(&self) -> Vec<Sourced<&str>> {
        let Some(language) = self.get("language") else {
            return Vec::new();
        };
        match &language.value {
            Value::Text(tag) => vec![Sourced::new(tag.as_str(), language.origin.clone())],
            Value::Texts(tags) => tags
                .iter()
                .map(|tag| Sourced::new(tag.value.as_str(), tag.origin.clone()))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// The setting `name` as written for the language `tag` alone, as
    /// `<tag>.<name>`.
    pub fn for_language(&self, tag: &str, name: &str) -> Option<&Sourced<Value>> {
        self.for_languages
            .iter()
            .find(|(t, n, _)| t == tag && *n == name)
            .map(|(_, _, value)| value)
    }

    /// The text setting `name` as the language `tag` reads it: its own
    /// line where it has one, else the setting for every language.
    pub fn text_for(&self, tag: Option<&str>, name: &str) -> Option<Sourced<&str>> {
        match tag.and_then(|tag| self.for_language(tag, name)) {
            Some(value) => Some(Sourced::new(value.value.as_text()?, value.origin.clone())),
            None => self.text(name),
        }
    }

    /// The names in `quiet:`, as written.
    pub fn quiet(&self) -> Vec<Sourced<String>> {
        self.texts("quiet").to_vec()
    }

    /// The path the site is published under, from `url`: `/blog` for
    /// `https://example.org/blog/`, and empty at the top of its domain.
    pub fn base(&self) -> String {
        self.text("url")
            .map(|url| path_of(url.value))
            .unwrap_or_default()
    }

    /// Takes `address`, given on the command line, as the site's address
    /// for this run, in place of `url`.
    pub fn at(&mut self, address: &str) {
        let given = Sourced::new(Value::Text(address.to_owned()), Origin::command_line());
        let written = match self.values.iter_mut().find(|(name, _)| *name == "url") {
            Some((_, url)) => Some(std::mem::replace(url, given)),
            None => {
                self.values.push(("url", given));
                None
            }
        };
        self.replaced.get_or_insert(written);
    }

    /// The address the site is known by beyond this run, which what other
    /// sites and feed readers hold of it names: `url` as the settings
    /// write it, whatever address the run was given, and the run's where
    /// they write none.
    pub fn home(&self) -> Option<&str> {
        match &self.replaced {
            Some(Some(written)) => written.value.as_text(),
            _ => Some(self.text("url")?.value),
        }
    }

    /// Whether the run was given its address on the command line.
    pub fn elsewhere(&self) -> bool {
        self.replaced.is_some()
    }

    pub fn own(&self, name: &str) -> Option<&Field> {
        self.own.iter().find(|field| field.name == name)
    }

    /// Fills every setting nobody wrote that has a default.
    fn defaults(&mut self) {
        for setting in SETTINGS {
            if self.get(setting.name).is_some() {
                continue;
            }
            let Some(default) = setting.default else {
                continue;
            };
            let value = match default {
                Preset::Text(text) => Value::Text(text.into()),
                Preset::Truth(b) => Value::Bool(b),
                Preset::Whole(n) => Value::Whole(n),
                Preset::Texts(texts) => Value::Texts(
                    texts
                        .iter()
                        .map(|t| Sourced::new((*t).to_owned(), Origin::defaulted()))
                        .collect(),
                ),
            };
            self.values
                .push((setting.name, Sourced::new(value, Origin::defaulted())));
        }
    }
}

/// Reads the settings of the site at `root`.
pub fn read(root: &Path, sources: &mut Sources) -> (Settings, Vec<Problem>) {
    let mut problems = Vec::new();
    for former in FORMER_NAMES {
        if root.join(former).exists() {
            problems.push(Problem::nowhere(Kind::OtherSpelling {
                word: Word::File,
                written: (*former).into(),
                ours: FILE_NAME.into(),
            }));
        }
    }
    let path = root.join(FILE_NAME);
    let text = match sources.unsaved().read_path(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let mut settings = Settings::default();
            settings.defaults();
            return (settings, problems);
        }
        Err(error) => {
            problems.push(Problem::nowhere(Kind::CantRead {
                path: FILE_NAME.into(),
                reason: crate::problem::Reason::of(&error),
            }));
            let mut settings = Settings::default();
            settings.defaults();
            return (settings, problems);
        }
    };
    let source = sources.add(FILE_NAME, text);
    let (settings, mut found) = read_source(sources, source);
    found.extend(crate::hooks::check(root, &settings));
    for &secret in &settings.secrets {
        sources.hide(secret);
    }
    // YAML that can't be read leaves no tree to find a secret in, so a
    // secret's line is also hidden by its key, whatever follows.
    for place in secret_lines(sources, source) {
        sources.hide(place);
    }
    problems.extend(found);
    (settings, problems)
}

/// What follows the key on every line that starts with a secret setting.
fn secret_lines(sources: &Sources, source: SourceId) -> Vec<crate::source::Place> {
    let text = sources.text(source);
    let mut places = Vec::new();
    let mut start = 0;
    for line in text.split_inclusive('\n') {
        let body = line.trim_end_matches(['\n', '\r']);
        let key = body.trim_start();
        let key = key.strip_prefix("pages.").unwrap_or(key);
        for secret in SECRETS {
            if let Some(rest) = key.strip_prefix(secret)
                && rest.trim_start().starts_with(':')
            {
                let value = rest.trim_start()[1..].trim();
                if !value.is_empty() {
                    let from = start + body.len() - rest.trim_start()[1..].trim_start().len();
                    places.push(crate::source::Place {
                        source,
                        span: crate::source::Span::new(from, from + value.len()),
                    });
                }
            }
        }
        start += line.len();
    }
    places
}

/// Reads settings from a source already added.
pub fn read_source(sources: &Sources, source: SourceId) -> (Settings, Vec<Problem>) {
    let mut settings = Settings {
        source: Some(source),
        ..Settings::default()
    };
    let mut problems = Vec::new();
    match crate::yaml::read(sources, source) {
        Ok(node) => read_node(sources, &node, &mut settings, &mut problems),
        Err(problem) => problems.push(problem),
    }
    settings.defaults();
    (settings, problems)
}

fn read_node(sources: &Sources, node: &Node, settings: &mut Settings, problems: &mut Vec<Problem>) {
    let entries = match &node.value {
        Yaml::Mapping(entries) => entries,
        Yaml::Scalar(_) if node.resolved() == Some(crate::yaml::Resolved::Null) => return,
        _ => {
            let reader = Reader::new(sources, FILE_NAME);
            problems.push(reader.wrong(node, Expected::Fields, None));
            return;
        }
    };
    let declared = declared(sources, entries);
    // Declarations first, so values handed down can be checked against them.
    for entry in entries {
        let name = words::key_name(&entry.key);
        let declares = name.starts_with("fields.") || name.starts_with("types.");
        if declares && matches!(entry.value.value, Yaml::Mapping(_)) {
            problems.extend(words::nested(sources, &name, &entry.value));
        } else if let Some(field) = name.strip_prefix("fields.") {
            problems.extend(settings.declarations.declare(
                sources,
                &entry.key,
                &name,
                field,
                None,
                &entry.value,
            ));
        } else if let Some(rest) = name.strip_prefix("types.") {
            match rest.split_once('.') {
                Some((page_type, field)) if !page_type.is_empty() => {
                    problems.extend(settings.declarations.declare(
                        sources,
                        &entry.key,
                        &name,
                        field,
                        Some(page_type),
                        &entry.value,
                    ))
                }
                _ => problems.push(unknown_setting(&name, &entry.key)),
            }
        }
    }
    for entry in entries {
        let name = words::key_name(&entry.key);
        let (key, value) = (&entry.key, &entry.value);
        let bare = ["pages.", "default."]
            .iter()
            .find_map(|prefix| name.strip_prefix(prefix))
            .unwrap_or(&name);
        if SECRETS.contains(&bare) {
            settings.secrets.push(value.place);
        }
        if name.starts_with("fields.") || name.starts_with("types.") {
            continue;
        }
        if name == TOKEN {
            problems.push(Problem::at(Kind::TokenInSettings, key.place));
            continue;
        }
        if let Some(setting) = SETTINGS.iter().find(|s| s.name == name) {
            let told = problems.len();
            problems.extend(words::empty(&name, Word::Setting, key, value));
            let read = match (setting.shape, &value.value) {
                (Shape::Language, Yaml::Sequence(items)) => {
                    Ok(read_languages(sources, value, items, problems))
                }
                (Shape::WebAddresses, _) => {
                    read_web_addresses(&Reader::new(sources, setting.name), value, problems)
                }
                _ => read_setting(sources, setting, value),
            };
            match read {
                Ok(Some(read)) => settings.values.push((setting.name, read)),
                Ok(None) => {}
                Err(problem) => problems.push(problem),
            }
            settings.language_told |= setting.name == "language" && problems.len() > told;
        } else if let Some((tag, rest)) = name.split_once('.')
            && declared.iter().any(|declared| declared == tag)
        {
            let setting = SETTINGS
                .iter()
                .find(|s| s.name == rest && PER_LANGUAGE.contains(&rest));
            if setting.is_some() {
                problems.extend(words::empty(&name, Word::Setting, key, value));
            }
            match setting.map(|setting| (setting, read_setting(sources, setting, value))) {
                Some((setting, Ok(Some(read)))) => {
                    settings
                        .for_languages
                        .push((tag.to_owned(), setting.name, read));
                }
                Some((_, Ok(None))) => {}
                Some((_, Err(problem))) => problems.push(problem),
                None => problems.push(unknown_for_a_language(&name, key, &declared)),
            }
        } else if declared.contains(&name) && matches!(value.value, Yaml::Mapping(_)) {
            problems.extend(words::nested(sources, &name, value));
        } else if let Some((tag, rest)) = name.split_once('.')
            && !declared.is_empty()
            && PER_LANGUAGE.contains(&rest)
            && names_a_language(tag, &declared)
        {
            problems.push(unknown_for_a_language(&name, key, &declared));
        } else if let Some(&(_, ours)) = OTHER_SPELLINGS.iter().find(|(other, _)| *other == name) {
            problems.push(Problem::at(
                Kind::OtherSpelling {
                    word: Word::Setting,
                    written: name.clone(),
                    ours: ours.into(),
                },
                key.place,
            ));
        } else if let Some(rest) = name.strip_prefix("default.") {
            problems.push(Problem::at(
                Kind::OtherSpelling {
                    word: Word::Setting,
                    written: name.clone(),
                    ours: format!("pages.{rest}"),
                },
                key.place,
            ));
        } else if name == "pages.index_page" {
            // Handed to every page, it would leave no list a page, the
            // site's own included.
            problems.push(Problem::at(
                Kind::WrongPlace {
                    key: name.clone(),
                    why: WrongPlace::WholeSite,
                },
                key.place,
            ));
        } else if let Some(rest) = name.strip_prefix("pages.") {
            words::read_handed(
                sources,
                key,
                rest,
                value,
                (&settings.declarations, !declared.is_empty()),
                &mut settings.pages,
                problems,
            );
        } else if RESERVED.contains(&name.as_str()) && matches!(value.value, Yaml::Mapping(_)) {
            problems.extend(words::nested(sources, &name, value));
        } else if let Some((head, _)) = name.split_once('.')
            && matches!(head, "tags" | "series" | "pagination")
        {
            problems.push(unknown_setting(&name, key));
        } else if RESERVED.contains(&name.split('.').next().unwrap_or(&name)) {
            let head = name.split('.').next().unwrap_or(&name);
            problems.push(Problem::at(
                Kind::ReservedName {
                    name: format!("site.{head}"),
                },
                key.place,
            ));
        } else {
            let known = setting_names().filter(|known| !known.contains('.'));
            if !name.contains('.')
                && let Some(nearest) = crate::suggest::one_slip(&name, known)
            {
                problems.push(Problem::at(
                    Kind::ReadAsCustom {
                        own: Own::Setting,
                        written: name.clone(),
                        nearest: nearest.into(),
                    },
                    key.place,
                ));
            }
            settings.own.push(Field {
                name,
                key: key.place,
                value: value.clone(),
                origin: value.origin(),
            });
        }
    }
    let (mut flat, mut nested) = (Vec::new(), Vec::new());
    words::shapes(&settings.own, &mut flat, &mut nested);
    problems.extend(words::written_two_ways(Root::Site, &flat, &nested));
}

/// The languages a site in several languages declares, as written; none
/// for a site in one language or none, which writes no setting for one.
fn declared(sources: &Sources, entries: &[crate::yaml::Entry]) -> Vec<String> {
    let Some(entry) = entries
        .iter()
        .find(|entry| words::key_name(&entry.key) == "language")
    else {
        return Vec::new();
    };
    let Yaml::Sequence(items) = &entry.value.value else {
        return Vec::new();
    };
    let read = read_languages(sources, &entry.value, items, &mut Vec::new());
    let tags: Vec<String> = read
        .as_ref()
        .and_then(|read| read.value.as_texts())
        .unwrap_or_default()
        .iter()
        .map(|tag| tag.value.clone())
        .collect();
    match tags.len() {
        0 | 1 => Vec::new(),
        _ => tags,
    }
}

/// `language:` written as a list, each tag checked, and a tag written again
/// in any capitals refused at the second.
fn read_languages(
    sources: &Sources,
    node: &Node,
    items: &[Node],
    problems: &mut Vec<Problem>,
) -> Option<Sourced<Value>> {
    let reader = Reader::new(sources, "language");
    let mut tags: Vec<Sourced<String>> = Vec::with_capacity(items.len());
    for item in items {
        let tag = match reader.text(item) {
            Ok(Some(tag)) if language_code(&tag.value) => tag,
            Ok(None) => continue,
            _ => {
                problems.push(reader.wrong(item, Expected::LanguageCode, None));
                continue;
            }
        };
        match tags
            .iter()
            .find(|first| first.value.eq_ignore_ascii_case(&tag.value))
        {
            Some(first) => problems.push(Problem::at(
                Kind::LanguageWrittenTwice {
                    written: tag.value.clone(),
                    first_written: first.value.clone(),
                    first: first.origin.place.unwrap_or(item.place),
                },
                item.place,
            )),
            None => tags.push(tag),
        }
    }
    (!tags.is_empty()).then(|| Sourced::new(Value::Texts(tags), node.origin()))
}

/// Full URLs, as a list or one standing for a list of one, each that
/// isn't one told and left out.
fn read_web_addresses(
    reader: &Reader,
    node: &Node,
    problems: &mut Vec<Problem>,
) -> Result<Option<Sourced<Value>>, Problem> {
    let items = match &node.value {
        Yaml::Sequence(items) => items.iter().collect(),
        Yaml::Mapping(_) => return Err(reader.wrong(node, Expected::List, None)),
        Yaml::Scalar(_) if node.resolved() == Some(crate::yaml::Resolved::Null) => return Ok(None),
        Yaml::Scalar(_) => vec![node],
    };
    let a_url = |text: &str| crate::fetched::refused::Spot::of(text).is_some();
    let mut urls = Vec::with_capacity(items.len());
    for item in items {
        match reader.text(item) {
            Ok(Some(url)) if a_url(&url.value) => urls.push(url),
            Ok(Some(url)) => {
                let https = format!("https://{}", url.value);
                let respelt = (!url.value.contains("://") && a_url(&https)).then_some(https);
                problems.push(reader.wrong(item, Expected::SiteAddress, respelt));
            }
            Ok(None) => {}
            Err(problem) => problems.push(problem),
        }
    }
    Ok(Some(Sourced::new(Value::Texts(urls), node.origin())))
}

/// Whether `tag` names a language, so that a setting written for it is
/// meant for a language rather than being one of the author's own: one
/// the site declares in other capitals, or one Statac has words for.
fn names_a_language(tag: &str, declared: &[String]) -> bool {
    declared
        .iter()
        .any(|declared| declared.eq_ignore_ascii_case(tag))
        || (language_code(tag) && crate::language::bundled(tag).is_some())
}

/// A setting written for a language the site doesn't declare, or one that
/// can't be written for a language. Another language is never offered for
/// the one written, which may be meant: only its capitals as declared.
fn unknown_for_a_language(name: &str, key: &Node, declared: &[String]) -> Problem {
    let (tag, rest) = name.split_once('.').unwrap_or((name, ""));
    let nearest = match declared.iter().find(|d| d.eq_ignore_ascii_case(tag)) {
        Some(declared) if declared == tag => {
            crate::suggest::nearest(rest, PER_LANGUAGE.iter().copied())
                .map(|setting| format!("{tag}.{setting}"))
        }
        Some(declared) if PER_LANGUAGE.contains(&rest) => Some(format!("{declared}.{rest}")),
        _ => None,
    };
    Problem::at(
        Kind::UnknownWord {
            word: Word::Setting,
            written: name.to_owned(),
            nearest,
        },
        key.place,
    )
}

fn unknown_setting(name: &str, key: &Node) -> Problem {
    let known = SETTINGS.iter().map(|s| s.name).filter(|n| n.contains('.'));
    Problem::at(
        Kind::UnknownWord {
            word: Word::Setting,
            written: name.to_owned(),
            nearest: crate::suggest::nearest(name, known).map(str::to_owned),
        },
        key.place,
    )
}

fn read_setting(
    sources: &Sources,
    setting: &Setting,
    node: &Node,
) -> Result<Option<Sourced<Value>>, Problem> {
    let reader = match SECRETS.contains(&setting.name) {
        true => Reader::secret(sources, setting.name),
        false => Reader::new(sources, setting.name),
    };
    let text = |shaped: fn(&str) -> bool, expected: Expected| match reader.text(node) {
        Ok(Some(text)) if shaped(&text.value) => Ok(Some(text.map(Value::Text))),
        Ok(None) => Ok(None),
        _ => Err(reader.wrong(node, expected, None)),
    };
    Ok(match setting.shape {
        Shape::Text => reader.text(node)?.map(|t| t.map(Value::Text)),
        Shape::Truth => reader.truth(node)?.map(|b| b.map(Value::Bool)),
        Shape::Whole(from) => reader.whole(node, from)?.map(|n| n.map(Value::Whole)),
        Shape::Texts => reader.texts(node)?.map(|t| t.map(Value::Texts)),
        Shape::Places(a_place, within) => {
            let items = match &node.value {
                Yaml::Sequence(items) => items.iter().collect(),
                _ => vec![node],
            };
            for item in items {
                if let Ok(Some(text)) = reader.text(item)
                    && !a_place(&text.value)
                {
                    let expected = match within {
                        PlaceIn::BuiltSite => Expected::BuiltPlace,
                        PlaceIn::Site => Expected::SitePlace,
                    };
                    return Err(reader.wrong(item, expected, None));
                }
            }
            reader.texts(node)?.map(|t| t.map(Value::Texts))
        }
        Shape::Date => reader.date(node)?.map(|d| d.map(Value::Date)),
        Shape::OneOf(words) => reader.one_of(node, words)?.map(|t| t.map(Value::Text)),
        Shape::SiteAddress => text(site_address, Expected::SiteAddress)?,
        Shape::Language => text(language_code, Expected::LanguageCode)?,
        Shape::WebAddresses => {
            let mut told = Vec::new();
            let read = read_web_addresses(&reader, node, &mut told)?;
            match told.into_iter().next() {
                Some(problem) => return Err(problem),
                None => read,
            }
        }
        Shape::ListAddress => text(list_address, Expected::ListAddress)?,
        Shape::LaterAddress => text(later_address, Expected::LaterAddress)?,
        Shape::Table => {
            let Yaml::Mapping(entries) = &node.value else {
                if node.resolved() == Some(crate::yaml::Resolved::Null) {
                    return Ok(None);
                }
                return Err(reader.wrong(node, Expected::Fields, None));
            };
            let mut table = Vec::with_capacity(entries.len());
            for entry in entries {
                let name = Reader::new(sources, setting.name).text(&entry.key)?;
                let given = reader.text(&entry.value)?;
                if let (Some(name), Some(given)) = (name, given) {
                    table.push((name, given));
                }
            }
            Some(Sourced::new(Value::Table(table), node.origin()))
        }
    })
}

/// The path a site at `address` is published under: `/blog` for
/// `https://example.org/blog/`, and empty at the top of its domain.
pub fn path_of(address: &str) -> String {
    let past = address.split_once("//").map_or(address, |(_, rest)| rest);
    match past.split_once('/') {
        Some((_, path)) if !path.trim_matches('/').is_empty() => {
            format!("/{}", path.trim_matches('/'))
        }
        _ => String::new(),
    }
}

/// `https://` or `http://`, then a host with a dot or `localhost`.
pub fn site_address(text: &str) -> bool {
    let Some(rest) = text
        .strip_prefix("https://")
        .or_else(|| text.strip_prefix("http://"))
    else {
        return false;
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = host.split(':').next().unwrap_or_default();
    host == "localhost"
        || (host.contains('.')
            && host.split('.').all(|part| {
                !part.is_empty() && part.chars().all(|c| c.is_alphanumeric() || c == '-')
            }))
}

/// Two or three letters, then any number of `-` and up to eight letters or
/// figures: `en`, `en-GB`, `zh-Hant-TW`.
fn language_code(text: &str) -> bool {
    let mut parts = text.split('-');
    let first = parts.next().unwrap_or_default();
    (2..=3).contains(&first.len())
        && first.bytes().all(|b| b.is_ascii_alphabetic())
        && parts.all(|p| (1..=8).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_alphanumeric()))
}

/// Folders, then `{slug}` as the last piece: `/topics/{slug}/`.
fn list_address(text: &str) -> bool {
    let Some(before) = text.trim_end_matches('/').strip_suffix("{slug}") else {
        return false;
    };
    text.ends_with('/')
        && before.ends_with('/')
        && !before.trim_matches('/').is_empty()
        && !before.contains(['{', '}'])
}

/// `{list}` first, `{n}` once, and a `/` at the end, with names every disk
/// will write between: `{list}page/{n}/`.
fn later_address(text: &str) -> bool {
    let Some(rest) = text.strip_prefix("{list}") else {
        return false;
    };
    rest.matches("{n}").count() == 1
        && rest.replace("{n}", "").find(['{', '}']).is_none()
        && text.ends_with('/')
        && crate::content::address::Address::read(&format!("/{}", rest.replace("{n}", "2"))).is_ok()
}

#[cfg(test)]
mod tests;
