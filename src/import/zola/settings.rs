//! Zola's settings: one TOML file at the top of the site, `zola.toml` or
//! `config.toml`. Zola reads its settings' names as written, in small
//! letters.

use std::path::Path;

use super::super::write::shown_value;
use super::super::{Failed, Field, Value, inside, toml};
use super::slug::Strategy;
use crate::problem::{Kind, Problem, system_reason};

/// The settings files Zola reads at the top of a site, the first found.
pub const FILES: &[&str] = &["zola.toml", "config.toml"];

/// Whether `folder` holds a Zola site, by its settings: a `zola.toml`, or
/// a `config.toml` that sets `base_url`, which Zola can't do without.
pub fn owns(folder: &Path) -> bool {
    inside::a_settings_file(folder, "zola.toml")
        || inside::a_settings_file(folder, "config.toml")
            .then(|| std::fs::read_to_string(inside::path(folder, "config.toml")).ok())
            .flatten()
            .and_then(|text| toml::read(&text).ok())
            .is_some_and(|fields| fields.iter().any(|f| f.name == "base_url"))
}

/// The settings file Zola reads in `folder`, with its fields.
pub fn found(folder: &Path) -> Result<(String, Vec<Field>), Box<Failed>> {
    let name = FILES
        .iter()
        .find(|name| inside::a_settings_file(folder, name))
        .copied()
        .unwrap_or(FILES[1]);
    let path = folder.join(name);
    if !inside::reached(folder, name) {
        return Err(Box::<Failed>::from(Problem::nowhere(Kind::CantRead {
            path,
            reason: "it is a link that leads outside the old site".to_owned(),
        })));
    }
    let text = std::fs::read_to_string(inside::path(folder, name)).map_err(|error| {
        Box::<Failed>::from(Problem::nowhere(Kind::CantRead {
            path: path.clone(),
            reason: system_reason(&error),
        }))
    })?;
    let fields = toml::read(&text).map_err(|stopped| toml::failed(&path, text, &stopped))?;
    Ok((name.to_owned(), fields))
}

/// A list of terms the settings declare.
#[derive(Clone, Debug)]
pub struct Taxonomy {
    pub name: String,
    pub paginate_by: Option<u64>,
    pub feed: bool,
    pub render: bool,
}

/// A language the settings declare besides the main one.
#[derive(Clone, Debug)]
pub struct Language {
    pub code: String,
    pub title: Option<String>,
    pub feeds: Option<bool>,
    pub feed_filenames: Option<Vec<String>>,
    pub taxonomies: Option<Vec<Taxonomy>>,
}

/// What the settings say.
#[derive(Debug)]
pub struct Settings {
    pub base_url: Option<String>,
    pub title: Option<String>,
    pub default_language: String,
    pub languages: Vec<Language>,
    pub taxonomies: Vec<Taxonomy>,
    pub paths: Strategy,
    pub terms: Strategy,
    pub anchors: Strategy,
    pub keep_dates: bool,
    pub feeds: bool,
    pub feed_filenames: Vec<String>,
    pub output_dir: String,
    pub ignored_content: Vec<String>,
    pub ignored_static: Vec<String>,
    pub smart_punctuation: bool,
    pub render_emoji: bool,
    pub target_blank: bool,
    pub no_follow: bool,
    pub no_referrer: bool,
    pub theme: Option<String>,
    /// The lines `output_dir` and `theme` are written on.
    pub output_line: Option<usize>,
    pub theme_line: Option<usize>,
    /// The author's own values, and the site's own words that are values
    /// in Statac, such as `description`.
    pub values: Vec<Field>,
    /// Settings Statac has no equal for, each with its line and text.
    pub left: Vec<(Option<usize>, String)>,
}

/// A value as text, where it is a scalar.
pub fn text_of(value: &Value) -> Option<String> {
    match value {
        Value::Text(text) | Value::Number(text) => Some(text.clone()),
        Value::Truth(truth) => Some(truth.to_string()),
        _ => None,
    }
}

/// A value as a list of texts: a list's scalars, or one scalar.
pub fn texts_of(value: &Value) -> Vec<String> {
    match value {
        Value::List(items) => items.iter().filter_map(text_of).collect(),
        Value::Nothing => Vec::new(),
        other => text_of(other).into_iter().collect(),
    }
}

/// A table's fields, or none.
pub fn fields(value: &Value) -> &[(String, Value)] {
    match value {
        Value::Fields(fields) => fields,
        _ => &[],
    }
}

/// The field `name` of a table.
pub fn get<'a>(fields: &'a [(String, Value)], name: &str) -> Option<&'a Value> {
    fields.iter().find(|(n, _)| n == name).map(|(_, v)| v)
}

fn truth(value: &Value) -> bool {
    matches!(value, Value::Truth(true))
}

fn number(value: &Value) -> Option<u64> {
    text_of(value).and_then(|n| n.parse().ok())
}

fn taxonomies(value: &Value) -> Vec<Taxonomy> {
    let Value::List(items) = value else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            let table = fields(item);
            Some(Taxonomy {
                name: get(table, "name").and_then(text_of)?,
                paginate_by: get(table, "paginate_by").and_then(number),
                feed: get(table, "feed").is_some_and(truth),
                render: get(table, "render").is_none_or(truth),
            })
        })
        .collect()
}

/// `feed_filenames`, or the older `feed_filename`, where `table` gives
/// either: each a file's name and no path, which is all Zola takes.
fn feed_filenames(table: &[(String, Value)]) -> Option<Vec<String>> {
    let names = get(table, "feed_filenames")
        .map(texts_of)
        .or_else(|| get(table, "feed_filename").map(texts_of))?;
    let named = |name: &String| !name.contains('/') && crate::platform::stays_inside(name);
    Some(names.into_iter().filter(named).collect())
}

/// `generate_feeds`, or the older `generate_feed`, where `table` gives
/// either.
fn generate_feeds(table: &[(String, Value)]) -> Option<bool> {
    get(table, "generate_feeds")
        .or_else(|| get(table, "generate_feed"))
        .map(truth)
}

/// What `entries` say.
pub fn settle(entries: &[Field]) -> Settings {
    let mut s = Settings {
        base_url: None,
        title: None,
        default_language: "en".into(),
        languages: Vec::new(),
        taxonomies: Vec::new(),
        paths: Strategy::On,
        terms: Strategy::On,
        anchors: Strategy::On,
        keep_dates: false,
        feeds: false,
        feed_filenames: vec!["atom.xml".into()],
        output_dir: "public".into(),
        ignored_content: Vec::new(),
        ignored_static: Vec::new(),
        smart_punctuation: false,
        render_emoji: false,
        target_blank: false,
        no_follow: false,
        no_referrer: false,
        theme: None,
        output_line: None,
        theme_line: None,
        values: Vec::new(),
        left: Vec::new(),
    };
    let top: Vec<(String, Value)> = entries
        .iter()
        .map(|f| (f.name.clone(), f.value.clone()))
        .collect();
    if let Some(generate) = generate_feeds(&top) {
        s.feeds = generate;
    }
    if let Some(names) = feed_filenames(&top).filter(|names| !names.is_empty()) {
        s.feed_filenames = names;
    }
    for field in entries {
        let value = &field.value;
        let text = text_of(value);
        let mut leave = |text: String| s.left.push((field.line, text));
        match field.name.as_str() {
            "base_url" => s.base_url = text.filter(|t| !t.is_empty()),
            "title" => s.title = text.filter(|t| !t.is_empty()),
            "description" | "author" => {
                if text.as_ref().is_some_and(|t| !t.is_empty()) {
                    s.values.push(field.clone());
                }
            }
            "default_language" => {
                if let Some(code) = text.filter(|t| !t.is_empty()) {
                    s.default_language = code;
                }
            }
            "generate_feeds" | "generate_feed" | "feed_filenames" | "feed_filename" => {}
            "taxonomies" => s.taxonomies = taxonomies(value),
            "output_dir" => {
                s.output_line = field.line;
                if let Some(dir) = text.filter(|t| !t.is_empty()) {
                    s.output_dir = dir.trim_end_matches('/').to_owned();
                }
            }
            "ignored_content" => s.ignored_content = texts_of(value),
            "ignored_static" => s.ignored_static = texts_of(value),
            "theme" => {
                s.theme_line = field.line;
                s.theme = text.filter(|t| !t.is_empty());
                leave(format!("{}: {}", field.name, shown_value(value)));
            }
            "slugify" => {
                for (name, inner) in fields(value) {
                    let strategy = text_of(inner).and_then(|t| Strategy::named(&t));
                    match (name.as_str(), strategy) {
                        ("paths", Some(strategy)) => s.paths = strategy,
                        ("taxonomies", Some(strategy)) => s.terms = strategy,
                        ("anchors", Some(strategy)) => s.anchors = strategy,
                        ("paths_keep_dates", _) => s.keep_dates = truth(inner),
                        _ => leave(format!("slugify.{name}: {}", shown_value(inner))),
                    }
                }
            }
            "markdown" => {
                for (name, inner) in fields(value) {
                    match name.as_str() {
                        "smart_punctuation" => s.smart_punctuation = truth(inner),
                        "render_emoji" => s.render_emoji = truth(inner),
                        "external_links_target_blank" => s.target_blank = truth(inner),
                        "external_links_no_follow" => s.no_follow = truth(inner),
                        "external_links_no_referrer" => s.no_referrer = truth(inner),
                        _ => leave(format!("markdown.{name}: {}", shown_value(inner))),
                    }
                }
            }
            "languages" => {
                for (code, table) in fields(value) {
                    let table = fields(table);
                    let mut language = Language {
                        code: code.clone(),
                        title: None,
                        feeds: generate_feeds(table),
                        feed_filenames: feed_filenames(table),
                        taxonomies: None,
                    };
                    for (name, inner) in table {
                        match name.as_str() {
                            "title" => language.title = text_of(inner).filter(|t| !t.is_empty()),
                            "taxonomies" => language.taxonomies = Some(taxonomies(inner)),
                            "generate_feeds" | "generate_feed" | "feed_filenames"
                            | "feed_filename" => {}
                            _ => leave(format!("languages.{code}.{name}: {}", shown_value(inner))),
                        }
                    }
                    s.languages.push(language);
                }
            }
            "extra" => {
                s.values.extend(
                    fields(value)
                        .iter()
                        .map(|(name, value)| Field::new(name, value.clone(), field.line)),
                );
            }
            _ => leave(format!("{}: {}", field.name, shown_value(value))),
        }
    }
    s
}
