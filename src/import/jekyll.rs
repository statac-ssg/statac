//! The reader for sites Jekyll made: a site with `_config.yml`,
//! `_config.yaml` or `_config.toml` at its top.
//!
//! Jekyll's rules are followed as its documentation gives them: which
//! files it publishes and which it keeps to itself, where it publishes
//! each page and post, how it reads front matter (YAML's older rules, by
//! which `yes` is true), and what its template tags do in the writing.

mod liquid;
mod url;

use std::collections::HashMap;
use std::path::Path;

use super::common::{dated_name, text_of};
use super::inside;
use super::report::code;
use super::{
    Failed, Field, Finding, FindingKind, HomePages, Import, Left, Page, PageKind, Value,
    one_home_page, yaml,
};
use crate::date::{self, Date};
use crate::origin::Style as Written;
use crate::problem::{Kind, Problem, Reason};
use crate::source::{Place, Sources};
use crate::yaml::{Node, Value as Yaml};
use url::{Moment, Style};

/// The settings files Jekyll reads, the first found.
const SETTINGS_FILES: [&str; 3] = ["_config.yml", "_config.yaml", "_config.toml"];

/// The endings of the files Jekyll reads as writing.
const WRITING: &[&str] = &["md", "markdown", "mkdown", "mkdn", "mkd", "html", "htm"];

/// What Jekyll leaves out of every site, whatever its settings say.
const LEFT_OUT: &[&str] = &[
    ".sass-cache",
    ".jekyll-cache",
    "gemfiles",
    "Gemfile",
    "Gemfile.lock",
    "node_modules",
    "vendor/bundle",
    "vendor/cache",
    "vendor/gems",
    "vendor/ruby",
];

/// Jekyll's own settings, besides those read below: none has an equal in
/// Statac.
const JEKYLLS_OWN: &[&str] = &[
    "cache_dir",
    "sass_dir",
    "safe",
    "keep_files",
    "encoding",
    "strict_front_matter",
    "show_drafts",
    "limit_posts",
    "unpublished",
    "whitelist",
    "markdown",
    "highlighter",
    "lsi",
    "incremental",
    "detach",
    "port",
    "host",
    "show_dir_listing",
    "quiet",
    "verbose",
    "liquid",
    "kramdown",
    "rdiscount",
    "redcarpet",
    "webrick",
    "theme",
    "remote_theme",
    "sass",
    "jekyll-archives",
    "livereload",
    "livereload_port",
    "profile",
    "disable_disk_cache",
    "relative_permalinks",
    "pygments",
    "gist",
    "compress_html",
];

/// The plugins whose work Statac does.
const PLUGINS_WITH_AN_EQUAL: &[&str] = &["jekyll-feed", "jekyll-redirect-from", "jekyll-paginate"];

/// The names a template's loop over the site's writing reads, which mark a
/// page as the templates' work rather than writing.
const LISTING: &[&str] = &[
    "site.posts",
    "paginator",
    "site.categories",
    "site.tags",
    "site.pages",
    "site.html_pages",
    "site.documents",
    "site.collections",
    "site.static_files",
];

pub(super) static TEMPLATE_PAGE: FindingKind = FindingKind {
    heading: "Pages made by templates",
    sentence: "Each of these pages was mostly the old site's templates at work, such as a home \
               page or an archive listing the posts, so Statac didn't bring it across: Statac \
               makes the home page and a page for each folder of posts and each tag itself, as \
               https://statac.dev/lists/ describes.",
};

pub(super) static HTML_WRITING: FindingKind = FindingKind {
    heading: "Pages written in HTML",
    sentence: "Each of these was written in HTML and came across as a Markdown file holding that \
               HTML, which Statac publishes as it is: read each, since Markdown reads a line \
               indented by four spaces after a blank line as code.",
};

pub(super) static TAG_LEFT: FindingKind = FindingKind {
    heading: "Template tags left as written",
    sentence: "Statac's Markdown runs no template tags and these have no equal in it, so each is \
               left as written and shows on the page as it stands: write by hand what it made, \
               or call a snippet, as https://statac.dev/snippets/ describes.",
};

pub(super) static TAG_IN_CODE: FindingKind = FindingKind {
    heading: "Template tags inside code",
    sentence: "Jekyll filled these in even inside code, and Statac shows code exactly as written, \
               so each now shows as it stands: write by hand what it made, where that was meant.",
};

pub(super) static LINK_NOWHERE: FindingKind = FindingKind {
    heading: "Links to pages that didn't come across",
    sentence: "Each of these names a page Statac couldn't find in the old site, or one Jekyll \
               didn't publish, so it is left as written: link to the page's URL instead.",
};

pub(super) static INCLUDE: FindingKind = FindingKind {
    heading: "Includes now snippet calls",
    sentence: "Each include is now a call to a snippet of its name, and the snippet holds only a \
               comment saying it wasn't converted: write the snippet, as \
               https://statac.dev/snippets/ describes, or take the calls out.",
};

pub(super) static EXCERPT_LEFT: FindingKind = FindingKind {
    heading: "Excerpt markers inside a line",
    sentence: "Statac ends an excerpt only at `<!--more-->` on a line of its own, so each of \
               these was left as written: put it on a line of its own, as `<!--more-->`.",
};

pub(super) static ATTRIBUTE_LINES: FindingKind = FindingKind {
    heading: "Attribute lists",
    sentence: "Statac's Markdown reads `{: ...}` as words, so each shows on the page as written: \
               read each page that has one, and take it out or write the HTML it stood for.",
};

pub(super) static TOC: FindingKind = FindingKind {
    heading: "Tables of contents written in Markdown",
    sentence: "Statac's Markdown reads `{:toc}` as words: take it and the list above it out. A \
               template can show the page's headings from `page.toc`, as \
               https://statac.dev/templates/#what-a-template-sees describes.",
};

pub(super) static EXTENSIONS: FindingKind = FindingKind {
    heading: "Extension blocks",
    sentence: "Statac's Markdown reads `{::name}` as words, so each shows on the page as written: \
               read each page that has one, and take it out or write what it stood for, such \
               as an HTML comment for `{::comment}`.",
};

pub(super) static ABBREVIATIONS: FindingKind = FindingKind {
    heading: "Abbreviations",
    sentence: "Statac's Markdown has no abbreviations, so each of these lines shows on the page \
               as words: take it out, or write each abbreviation as an `<abbr>`.",
};

pub(super) static MARKDOWN_IN_HTML: FindingKind = FindingKind {
    heading: "Markdown inside HTML",
    sentence: "Jekyll read the Markdown inside each of these tags, as `markdown=\"1\"` asks, and \
               Statac's Markdown leaves what is inside HTML as it is, so the Markdown there shows \
               as written: take off the HTML round it, or write what is inside as HTML.",
};

pub(super) static INLINE_MATHS: FindingKind = FindingKind {
    heading: "Maths between two dollar signs inside a line",
    sentence: "Jekyll set these inside the line, and Statac sets maths between `$$` apart on a \
               line of its own: read each page that has one, and write the maths with one `$` \
               either side to keep it inside the line.",
};

pub(super) static NOT_A_POST: FindingKind = FindingKind {
    heading: "Files in `_posts` that aren't posts",
    sentence: "Jekyll publishes a post only from a file with front matter named as \
               `2026-10-04-title.md`, so it didn't publish these, and Statac left them: bring \
               each across by hand if it is wanted.",
};

pub(super) static FRONT_MATTER_UNREAD: FindingKind = FindingKind {
    heading: "Front matter Statac can't read",
    sentence: "Statac's YAML reader stopped in the front matter of each of these, so the page \
               didn't come across: put the YAML right and import again.",
};

pub(super) static DATA_LEFT: FindingKind = FindingKind {
    heading: "Data files left behind",
    sentence: "Statac reads data files written in YAML or JSON only, so these weren't copied: \
               write each as YAML or JSON in `data/`.",
};

pub(super) static SETTINGS_LEFT: FindingKind = FindingKind {
    heading: "Settings with no equal",
    sentence: "Statac has no equal for each of these settings or plugins, so nothing came across \
               for them: https://statac.dev/settings/ says what Statac offers.",
};

pub(super) static UNPUBLISHED: FindingKind = FindingKind {
    heading: "Pages Jekyll didn't publish",
    sentence: "Each of these is in a folder of pages Jekyll's settings kept from being published, \
               so it came across with `draft: true`: take that line out of each you want \
               published.",
};

pub(super) static NUMBER_LEFT: FindingKind = FindingKind {
    heading: "Values Jekyll read as numbers",
    sentence: "Jekyll read each of these as a number other than its figures say, as an older \
               reading of YAML does, and Statac kept it as the text it is: write the number you \
               mean.",
};

pub(super) static ADDRESS_UNKNOWN: FindingKind = FindingKind {
    heading: "URLs Statac couldn't work out",
    sentence: "Each of these pages has a `permalink` holding a placeholder Statac doesn't know, so \
               its old URL wasn't worked out or kept: give the page a `permalink` of the URL \
               the old site published it at.",
};

/// Whether `folder` holds a Jekyll site: a settings file Jekyll reads at
/// its top.
pub(super) fn owns(folder: &Path) -> bool {
    SETTINGS_FILES
        .iter()
        .any(|name| inside::a_settings_file(folder, name))
}

/// A folder of pages Jekyll's settings name.
#[derive(Clone, Debug)]
struct Gathering {
    label: String,
    output: bool,
    permalink: Option<String>,
}

/// A line of the settings' `defaults`.
#[derive(Clone, Debug)]
struct Default {
    path: String,
    kind: Option<String>,
    values: Vec<(String, Value)>,
}

/// What the settings say.
#[derive(Default)]
struct Settings {
    title: Option<String>,
    url: Option<String>,
    base: String,
    language: Option<String>,
    timezone: Option<String>,
    permalink: Option<String>,
    paginate: Option<u64>,
    paginate_path: Option<String>,
    gatherings: Vec<Gathering>,
    posts_permalink: Option<String>,
    defaults: Vec<Default>,
    exclude: Vec<String>,
    include: Vec<String>,
    separator: Option<String>,
    future: bool,
    feed: bool,
    feed_path: Option<String>,
    /// The folder the site is read from, by its path in the old folder;
    /// empty where it is the old folder itself.
    source: String,
    destination: String,
    gatherings_dir: String,
    data_dir: String,
    layouts_dir: String,
    includes_dir: String,
    plugins_dir: String,
}

/// A value as a list of texts: a list's scalars, or one text parted by
/// spaces.
fn texts_of(value: &Value) -> Vec<String> {
    match value {
        Value::List(items) => items.iter().filter_map(text_of).collect(),
        Value::Nothing => Vec::new(),
        other => text_of(other)
            .map(|text| text.split_whitespace().map(str::to_owned).collect())
            .unwrap_or_default(),
    }
}

/// A plain scalar as Jekyll reads it, by YAML 1.1: `yes`, `no`, `on` and
/// `off` true or false whatever their capitals, a number with `_` in it a
/// number, and dates left as text.
fn plain(text: &str) -> Value {
    let lower = text.to_ascii_lowercase();
    match lower.as_str() {
        "" | "~" | "null" => return Value::Nothing,
        "yes" | "true" | "on" => return Value::Truth(true),
        "no" | "false" | "off" => return Value::Truth(false),
        ".inf" | "+.inf" | "-.inf" | ".nan" => return Value::Number(text.to_owned()),
        _ => {}
    }
    let unsigned = text.strip_prefix(['-', '+']).unwrap_or(text);
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit() || b == b'_');
    if digits(unsigned) && (unsigned == "0" || !unsigned.starts_with('0')) {
        return Value::Number(text.replace('_', ""));
    }
    if let Some((whole, fraction)) = unsigned.split_once('.') {
        let (fraction, exponent) = match fraction.split_once(['e', 'E']) {
            Some((fraction, exponent)) => (fraction, Some(exponent)),
            None => (fraction, None),
        };
        let exponent_reads = exponent.is_none_or(|e| {
            e.strip_prefix(['-', '+'])
                .is_some_and(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
        });
        if (whole.is_empty() || digits(whole))
            && (fraction.is_empty() || digits(fraction))
            && !(whole.is_empty() && fraction.is_empty())
            && exponent_reads
        {
            return Value::Number(text.replace('_', ""));
        }
    }
    Value::Text(text.to_owned())
}

/// Whether YAML 1.1 reads `text`, written plain, as a number its figures
/// don't say: octal, hexadecimal, binary, base 60, or with commas.
fn older_number(text: &str) -> bool {
    let unsigned = text.strip_prefix(['-', '+']).unwrap_or(text);
    let all = |s: &str, f: fn(u8) -> bool| {
        !s.is_empty() && s.bytes().all(|b| f(b) || b == b'_' || b == b',')
    };
    if let Some(hex) = unsigned.strip_prefix("0x") {
        return all(hex, |b| b.is_ascii_hexdigit());
    }
    if let Some(binary) = unsigned.strip_prefix("0b") {
        return all(binary, |b| b == b'0' || b == b'1');
    }
    if unsigned.len() > 1
        && unsigned.starts_with('0')
        && all(unsigned, |b| (b'0'..=b'7').contains(&b))
    {
        return true;
    }
    if unsigned.contains(',')
        && all(unsigned, |b| b.is_ascii_digit())
        && unsigned.starts_with(|c: char| c.is_ascii_digit())
    {
        return true;
    }
    let mut pieces = unsigned.split(':');
    let first = pieces.next().unwrap_or_default();
    let rest: Vec<&str> = pieces.collect();
    !rest.is_empty()
        && all(first, |b| b.is_ascii_digit())
        && !first.contains(',')
        && rest.iter().all(|piece| {
            (1..=2).contains(&piece.len())
                && piece.bytes().all(|b| b.is_ascii_digit())
                && piece.parse::<u8>().is_ok_and(|n| n < 60)
        })
}

/// Whether any plain scalar within `node` is one [`older_number`] tells of.
fn holds_older_number(node: &Node) -> bool {
    match &node.value {
        Yaml::Scalar(text) => {
            node.style == Written::Plain && node.tag.is_none() && older_number(text)
        }
        Yaml::Sequence(items) => items.iter().any(holds_older_number),
        Yaml::Mapping(entries) => entries.iter().any(|entry| holds_older_number(&entry.value)),
    }
}

/// A date Jekyll read as UTC, with `Z` after it: a plain scalar holding a
/// day, a time with its seconds and no offset, which is a timestamp to
/// YAML 1.1, and UTC there. One with no seconds, or in quotes, is text,
/// which Jekyll read in the site's time zone as Statac does.
fn read_as_utc(node: &Node) -> Option<String> {
    let Yaml::Scalar(text) = &node.value else {
        return None;
    };
    if node.style != Written::Plain || node.tag.is_some() {
        return None;
    }
    let text = text.trim();
    let (day, time) = text.split_once(['T', 't', ' '])?;
    let digits = |s: &str, most: usize| {
        (1..=most).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit())
    };
    let mut day = day.split('-');
    let dated = day
        .next()
        .is_some_and(|year| digits(year, 4) && year.len() == 4)
        && day.next().is_some_and(|month| digits(month, 2))
        && day.next().is_some_and(|d| digits(d, 2))
        && day.next().is_none();
    let (clock, fraction) = match time.trim_start().split_once('.') {
        Some((clock, fraction)) => (clock, fraction),
        None => (time.trim_start(), ""),
    };
    let mut clock = clock.split(':');
    let timed = clock.next().is_some_and(|hour| digits(hour, 2))
        && clock
            .next()
            .is_some_and(|minute| digits(minute, 2) && minute.len() == 2)
        && clock
            .next()
            .is_some_and(|second| digits(second, 2) && second.len() == 2)
        && clock.next().is_none()
        && fraction.bytes().all(|b| b.is_ascii_digit());
    (dated && timed).then(|| format!("{text}Z"))
}

/// A file read for its front matter: each field with its line, and the
/// writing after it with the line it starts on.
struct Front {
    fields: Vec<(String, Value, usize, Node)>,
    body: String,
    body_line: usize,
}

/// Reads `text`'s front matter, or tells why it can't be.
fn front(text: &str, path: &str) -> Result<Front, String> {
    let mut sources = Sources::default();
    let mended = super::mend::mended(text, false).text;
    let text = mended.as_str();
    let source = sources.add(path, text.to_owned());
    let split = crate::yaml::split(text)
        .map_err(|_| "Unclosed front matter: no `---` line ends it".to_owned())?;
    let mut fields = Vec::new();
    if let Some(span) = split.front_matter {
        let node = crate::yaml::read_at(&sources, Place { source, span })
            .map_err(|problem| problem.headline())?;
        match &node.value {
            Yaml::Mapping(entries) => {
                for entry in entries {
                    let line = sources.line_number(source, entry.key.place.span.start as usize);
                    let value = yaml::value_of(&entry.value, &plain);
                    fields.push((yaml::key_text(&entry.key), value, line, entry.value.clone()));
                }
            }
            Yaml::Scalar(text) if text.trim().is_empty() => {}
            _ => return Err("Front matter: it holds no fields".to_owned()),
        }
    }
    let body_line = sources.line_number(source, (split.body.start as usize).min(text.len()));
    Ok(Front {
        fields,
        body: text[split.body.range()].to_owned(),
        body_line,
    })
}

/// Whether `bytes` open with front matter as Jekyll tells it: a first line
/// of `---` and nothing else.
fn has_front_matter(bytes: &[u8]) -> bool {
    let bytes = bytes.strip_prefix("\u{feff}".as_bytes()).unwrap_or(bytes);
    let line = bytes.split(|b| *b == b'\n').next().unwrap_or_default();
    bytes.starts_with(b"---") && line.trim_ascii_end() == b"---" && bytes.len() > line.len()
}

/// Reads the site in `folder`.
pub(super) fn read(folder: &Path, today: Date, given: Option<&str>) -> Result<Import, Box<Failed>> {
    let mut import = Import::default();
    let mut reader = Reader {
        folder,
        given,
        import: &mut import,
        settings: Settings::default(),
        docs: Vec::new(),
        today,
        layouts: 0,
        utc_dates: 0,
    };
    reader.settings()?;
    reader.walk("")?;
    reader.finish();
    Ok(import)
}

/// A page, post or document as found, before its writing is converted.
struct Doc {
    /// Its path in the old folder.
    from: String,
    /// Its path in the site's source, which a `link` tag names it by.
    rel: String,
    to: String,
    kind: PageKind,
    fields: Vec<Field>,
    body: String,
    body_line: usize,
    address: Option<String>,
    own_address: bool,
    others: Vec<String>,
    separator: Option<String>,
    /// What `post_url` names it by, for a post.
    post_names: Vec<String>,
    /// Its names in each list of terms but its tags: its categories.
    terms: Vec<(String, Vec<String>)>,
    /// Its fields that hold plain text or a number, as Jekyll showed each
    /// in the writing.
    plain: Vec<(String, String)>,
}

/// Where a file found in the walk belongs.
#[derive(Clone, Debug, PartialEq)]
enum Found {
    Post { dirs: Vec<String> },
    Draft { dirs: Vec<String> },
    Page,
    Doc { label: String, within: String },
}

struct Reader<'a> {
    folder: &'a Path,
    /// The site's address as the author gave it, for settings that give
    /// none.
    given: Option<&'a str>,
    import: &'a mut Import,
    settings: Settings,
    docs: Vec<Doc>,
    today: Date,
    /// How many pages had `layout` taken off.
    layouts: usize,
    /// How many dates were written with a time and no offset.
    utc_dates: usize,
}

impl Reader<'_> {
    /// A file by its path in the old folder, where the settings file and
    /// the built output are.
    fn root(&self, path: &str) -> std::path::PathBuf {
        inside::path(self.folder, path)
    }

    /// The path in the old folder of `rel`, a path in the site's source.
    fn from(&self, rel: &str) -> String {
        match self.settings.source.is_empty() {
            true => rel.to_owned(),
            false => format!("{}/{rel}", self.settings.source),
        }
    }

    /// A file by its path in the site's source.
    fn path(&self, rel: &str) -> std::path::PathBuf {
        self.root(&self.from(rel))
    }

    fn left(&mut self, rel: &str, what: &str) {
        self.import.left.push(Left {
            path: self.from(rel),
            what: what.to_owned(),
        });
    }

    fn find(&mut self, kind: &'static FindingKind, rel: &str, line: Option<usize>, text: &str) {
        let file = self.from(rel);
        self.import
            .findings
            .push(Finding::at(kind, &file, line, text));
    }

    /// Tells of something in the settings file, which is in the old
    /// folder whatever the source.
    fn find_in_settings(&mut self, kind: &'static FindingKind, line: Option<usize>, text: &str) {
        let file = self.import.settings_file.clone();
        self.import
            .findings
            .push(Finding::at(kind, &file, line, text));
    }

    fn note(&mut self, text: String) {
        self.import.notes.push(text);
    }

    /// Reads the settings: those Statac has an equal for, and the
    /// author's own values.
    fn settings(&mut self) -> Result<(), Box<Failed>> {
        let s = &mut self.settings;
        s.destination = "_site".into();
        s.data_dir = "_data".into();
        s.layouts_dir = "_layouts".into();
        s.includes_dir = "_includes".into();
        s.plugins_dir = "_plugins".into();
        let found = SETTINGS_FILES
            .iter()
            .find(|name| inside::a_settings_file(self.folder, name));
        if let Some(name) = found {
            self.settings_file(name)?;
        }
        self.settled();
        Ok(())
    }

    /// Reads the settings file `name`.
    fn settings_file(&mut self, name: &str) -> Result<(), Box<Failed>> {
        let text = std::fs::read_to_string(self.root(name)).map_err(|error| {
            Box::<Failed>::from(Problem::nowhere(Kind::CantRead {
                path: self.folder.join(name),
                reason: Reason::of(&error),
            }))
        })?;
        let entries = match name.ends_with(".toml") {
            true => super::toml::read(&text)
                .map_err(|stopped| super::toml::failed(&self.folder.join(name), text, &stopped))?,
            false => {
                let mut sources = Sources::default();
                let source = sources.add(self.folder.join(name), text);
                let node = match crate::yaml::read(&sources, source) {
                    Ok(node) => node,
                    Err(problem) => return Err(Box::new(Failed { sources, problem })),
                };
                match &node.value {
                    Yaml::Mapping(entries) => entries
                        .iter()
                        .map(|entry| {
                            let line =
                                sources.line_number(source, entry.key.place.span.start as usize);
                            let value = yaml::value_of(&entry.value, &plain);
                            Field::new(&yaml::key_text(&entry.key), value, Some(line))
                        })
                        .collect(),
                    _ => Vec::new(),
                }
            }
        };
        let s = &mut self.settings;
        let mut values = Vec::new();
        let mut left = Vec::new();
        let mut unread = Vec::new();
        for field in entries {
            let (key, value) = (field.name, field.value);
            let line = field.line.unwrap_or_default();
            let text = text_of(&value);
            match key.as_str() {
                "title" => s.title = text,
                "url" => {
                    s.url = text
                        .map(|url| url.trim_end_matches('/').to_owned())
                        .filter(|url| !url.is_empty())
                }
                "baseurl" => {
                    let base = text.unwrap_or_default();
                    let base = base.trim_matches('/');
                    s.base = match base.is_empty() {
                        true => String::new(),
                        false => format!("/{base}"),
                    };
                }
                "lang" => s.language = text.or(s.language.take()),
                "locale" => {
                    if s.language.is_none() {
                        s.language = text.map(|code| {
                            code.split('.').next().unwrap_or_default().replace('_', "-")
                        });
                    }
                }
                "timezone" => s.timezone = text,
                "permalink" => s.permalink = text,
                "paginate" => s.paginate = text.and_then(|n| n.parse().ok()).filter(|n| *n > 0),
                "paginate_path" => s.paginate_path = text,
                "excerpt_separator" => s.separator = text,
                "future" => s.future = value == Value::Truth(true),
                "source" | "destination" | "collections_dir" | "data_dir" | "layouts_dir"
                | "includes_dir" | "plugins_dir" => {
                    let written = text.unwrap_or_default();
                    let Some(folder) = inside::named_folder(self.folder, &written) else {
                        unread.push((format!("{key}: {written}"), line));
                        continue;
                    };
                    match key.as_str() {
                        "source" => s.source = folder,
                        "destination" => s.destination = folder,
                        "collections_dir" => s.gatherings_dir = folder,
                        "data_dir" => s.data_dir = folder,
                        "layouts_dir" => s.layouts_dir = folder,
                        "includes_dir" => s.includes_dir = folder,
                        _ => s.plugins_dir = folder,
                    }
                }
                "exclude" => s.exclude = texts_of(&value),
                "include" => s.include = texts_of(&value),
                "markdown_ext" => {}
                "feed" => {
                    if let Value::Fields(fields) = &value {
                        s.feed_path = fields
                            .iter()
                            .find(|(name, _)| name == "path")
                            .and_then(|(_, v)| text_of(v));
                    }
                }
                "plugins" | "gems" => {
                    for plugin in texts_of(&value) {
                        if plugin == "jekyll-feed" {
                            s.feed = true;
                        }
                        if !PLUGINS_WITH_AN_EQUAL.contains(&plugin.as_str()) {
                            left.push((format!("{key}: {plugin}"), line));
                        }
                    }
                }
                "collections" => match &value {
                    Value::List(labels) => {
                        for label in labels.iter().filter_map(text_of) {
                            s.gatherings.push(Gathering {
                                label,
                                output: false,
                                permalink: None,
                            });
                        }
                    }
                    Value::Fields(labels) => {
                        for (label, metadata) in labels {
                            let field = |name: &str| match metadata {
                                Value::Fields(fields) => fields
                                    .iter()
                                    .find(|(n, _)| n == name)
                                    .map(|(_, v)| v.clone()),
                                _ => None,
                            };
                            let permalink = field("permalink").as_ref().and_then(text_of);
                            if label == "posts" {
                                s.posts_permalink = permalink;
                                continue;
                            }
                            s.gatherings.push(Gathering {
                                label: label.clone(),
                                output: field("output") == Some(Value::Truth(true)),
                                permalink,
                            });
                        }
                    }
                    _ => {}
                },
                "defaults" => {
                    if let Value::List(items) = &value {
                        for item in items {
                            let Value::Fields(item) = item else { continue };
                            let get =
                                |name: &str| item.iter().find(|(n, _)| n == name).map(|(_, v)| v);
                            let scope = |name: &str| match get("scope") {
                                Some(Value::Fields(scope)) => scope
                                    .iter()
                                    .find(|(n, _)| n == name)
                                    .and_then(|(_, v)| text_of(v)),
                                _ => None,
                            };
                            let values = match get("values") {
                                Some(Value::Fields(values)) => values.clone(),
                                _ => Vec::new(),
                            };
                            s.defaults.push(Default {
                                path: scope("path")
                                    .unwrap_or_default()
                                    .trim_matches('/')
                                    .to_owned(),
                                kind: scope("type"),
                                values,
                            });
                        }
                    }
                }
                _ if JEKYLLS_OWN.contains(&key.as_str()) => {
                    left.push((
                        format!("{key}: {}", super::write::shown_value(&value)),
                        line,
                    ));
                }
                _ => values.push(Field::new(&key, value, Some(line))),
            }
        }
        self.import.settings_file = name.to_owned();
        for (text, line) in left {
            self.find_in_settings(&SETTINGS_LEFT, Some(line), &text);
        }
        for (text, line) in unread {
            self.find_in_settings(&inside::FOLDER_UNREAD, Some(line), &text);
        }
        self.import.values = values;
        Ok(())
    }

    /// What the settings, or their absence, give the import: its title,
    /// address, language and zone, and what it says of each.
    fn settled(&mut self) {
        let s = &self.settings;
        self.import.title = s.title.clone();
        // Jekyll's Markdown sets maths between `$$` and `$$`, and none
        // between single dollar signs.
        self.import.maths = super::Maths {
            inline: false,
            display: true,
        };
        let mut notes = Vec::new();
        match (&s.url, self.given) {
            (Some(url), _) if url.starts_with("https://") || url.starts_with("http://") => {
                self.import.address = Some(format!("{url}{}", s.base));
            }
            (_, Some(given)) => self.import.address = Some(super::under(given, &s.base)),
            (Some(url), None) => notes.push(format!(
                "The settings' `url`, {}, isn't an `https://` or `http://` URL, so the new site \
                 has no `url` yet: add it to `statac.yaml` once it is known.",
                code(url)
            )),
            (None, None) if !s.base.is_empty() => notes.push(format!(
                "The settings give a `baseurl`, {}, but no `url`, and Statac puts a site under a \
                 path only by its `url`: the links that named it were written without it, so add \
                 the full URL as `url` in `statac.yaml`, and put the path back in those links.",
                code(&s.base)
            )),
            (None, None) => {}
        }
        match &s.language {
            Some(code_) if crate::new::a_language_code(code_) => {
                self.import.languages = vec![code_.clone()]
            }
            Some(written) => notes.push(format!(
                "The settings name the language {}, which isn't a code Statac reads, so the site \
                 is in English, `en`: write its code in `language` in `statac.yaml`.",
                code(written)
            )),
            None => notes.push(
                "The settings name no language, so the site is in English, `en`: change \
                 `language` in `statac.yaml` if it isn't."
                    .to_owned(),
            ),
        }
        match &s.timezone {
            Some(zone) if jiff::tz::TimeZone::get(zone).is_ok() => {
                self.import.timezone = Some(zone.clone())
            }
            Some(zone) => notes.push(format!(
                "The settings name the time zone {}, which Statac doesn't know, so none is \
                 written: add `timezone` to `statac.yaml`, as `Europe/London`.",
                code(zone)
            )),
            None => notes.push(
                "The settings name no time zone, so none is written, and Statac reads a date \
                 written without an offset as UTC, and says so: add `timezone` to \
                 `statac.yaml`, as `Europe/London`."
                    .to_owned(),
            ),
        }
        if s.future {
            notes.push(
                "The settings had `future: true`, by which Jekyll published posts dated after \
                 the day it built the site; Statac publishes each on its date, as a scheduled \
                 post."
                    .to_owned(),
            );
        }
        for note in notes {
            self.note(note);
        }
        let destination = self.settings.destination.clone();
        if !destination.is_empty() && inside::a_folder(self.folder, &destination) {
            self.import.built = Some(destination);
        }
        if !self.settings.source.is_empty() {
            self.note(format!(
                "The settings' `source` is {}, so the site was read from there, and nothing else \
                 in the old folder is the site's.",
                code(&format!("{}/", self.settings.source))
            ));
        }
    }

    /// Whether Jekyll leaves `rel` out of the site: by the settings'
    /// `exclude`, by its own list, or by a name starting `.`, `_`, `#` or
    /// `~` or ending `~`, unless `include` names it.
    fn excluded(&self, rel: &str) -> bool {
        let name = rel.rsplit('/').next().unwrap_or(rel);
        let names = |list: &[String]| {
            list.iter().any(|entry| {
                let entry = entry.trim_matches('/');
                entry == rel
                    || entry == name
                    || rel.starts_with(&format!("{entry}/"))
                    || glob(entry, rel)
            })
        };
        if names(&self.settings.include) {
            return false;
        }
        names(&self.settings.exclude)
            || LEFT_OUT
                .iter()
                .any(|entry| *entry == rel || rel.starts_with(&format!("{entry}/")))
            || name.starts_with(['.', '_', '#', '~'])
            || name.ends_with('~')
    }

    /// Reads the folder `rel` of the old site, `""` for its top.
    fn walk(&mut self, rel: &str) -> Result<(), Box<Failed>> {
        let mut names: Vec<String> = match std::fs::read_dir(self.path(rel)) {
            Ok(entries) => entries
                .flatten()
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect(),
            Err(error) => {
                return Err(Box::<Failed>::from(Problem::nowhere(Kind::CantRead {
                    path: self.path(rel),
                    reason: Reason::of(&error),
                })));
            }
        };
        names.sort();
        let gatherings_dir = self.settings.gatherings_dir.clone();
        for name in names {
            let path = match rel.is_empty() {
                true => name.clone(),
                false => format!("{rel}/{name}"),
            };
            let found = inside::metadata(self.folder, &self.from(&path));
            let Ok(found) = found else { continue };
            if found.is_symlink() {
                self.left(&path, inside::why(self.folder, &self.from(&path)));
                continue;
            }
            let dir = found.is_dir();
            let shown = if dir {
                format!("{path}/")
            } else {
                path.clone()
            };
            if self.from(&path) == self.settings.destination {
                self.left(
                    &shown,
                    "the old site's built output, read only to check the URLs",
                );
                continue;
            }
            if rel.is_empty() {
                if self.settings.source.is_empty() && SETTINGS_FILES.contains(&name.as_str()) {
                    if !self.read_settings_file(&name) {
                        self.left(
                            &shown,
                            "another settings file, which Jekyll reads only where the one \
                             before it isn't there",
                        );
                    }
                    continue;
                }
                let s = &self.settings;
                let special = [
                    (s.layouts_dir.as_str(), "the old site's layouts".to_owned()),
                    (
                        s.includes_dir.as_str(),
                        "the old site's includes; each one the content calls has a snippet of its \
                         name standing for it"
                            .to_owned(),
                    ),
                    ("_sass", "the old site's Sass".to_owned()),
                    (
                        s.plugins_dir.as_str(),
                        "the old site's plugins, which Statac doesn't run".to_owned(),
                    ),
                ];
                if let Some((_, what)) = special.iter().find(|(special, _)| *special == path) {
                    let what = (*what).to_owned();
                    self.left(&shown, &what);
                    continue;
                }
                if path == self.settings.data_dir && dir {
                    self.data(&path)?;
                    continue;
                }
            }
            if dir && name == "_posts" {
                self.special(&path, &path, true)?;
                continue;
            }
            if dir && name == "_drafts" {
                self.special(&path, &path, false)?;
                continue;
            }
            let gathering = self.settings.gatherings.iter().find(|g| {
                let at = match gatherings_dir.is_empty() {
                    true => format!("_{}", g.label),
                    false => format!("{gatherings_dir}/_{}", g.label),
                };
                at == path
            });
            if dir && let Some(gathering) = gathering.cloned() {
                self.gathering(&path, &path, &gathering)?;
                continue;
            }
            if self.excluded(&path) {
                let what = match dir {
                    true => "a folder Jekyll leaves out of the site",
                    false => "a file Jekyll leaves out of the site",
                };
                self.left(&shown, what);
                continue;
            }
            match dir {
                true => self.walk(&path)?,
                false => self.file(&path, Found::Page)?,
            }
        }
        Ok(())
    }

    /// Whether `name` is the settings file read.
    fn read_settings_file(&self, name: &str) -> bool {
        SETTINGS_FILES
            .iter()
            .find(|n| inside::a_file(self.folder, n))
            == Some(&name)
    }

    /// Reads `_posts` or `_drafts` at `rel`, and the folders in it.
    fn special(&mut self, top: &str, rel: &str, posts: bool) -> Result<(), Box<Failed>> {
        let mut names: Vec<String> = std::fs::read_dir(self.path(rel))
            .map(|entries| {
                entries
                    .flatten()
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        for name in names {
            let path = format!("{rel}/{name}");
            let Ok(found) = inside::metadata(self.folder, &self.from(&path)) else {
                continue;
            };
            if found.is_symlink() {
                self.left(&path, inside::why(self.folder, &self.from(&path)));
            } else if found.is_dir() {
                self.special(top, &path, posts)?;
            } else if name.starts_with('.') {
                self.left(&path, "a file Jekyll leaves out of the site");
            } else {
                let above = top.rsplit_once('/').map_or("", |(above, _)| above);
                let gatherings_dir = &self.settings.gatherings_dir;
                let above = match above.strip_prefix(gatherings_dir.as_str()) {
                    Some(rest) if !gatherings_dir.is_empty() => rest,
                    _ => above,
                };
                let mut dirs: Vec<String> = above
                    .split('/')
                    .filter(|d| !d.is_empty())
                    .map(str::to_owned)
                    .collect();
                let within = rel[top.len()..].trim_start_matches('/');
                dirs.extend(
                    within
                        .split('/')
                        .filter(|d| !d.is_empty())
                        .map(str::to_owned),
                );
                let found = match posts {
                    true => Found::Post { dirs },
                    false => Found::Draft { dirs },
                };
                self.file(&path, found)?;
            }
        }
        Ok(())
    }

    /// Reads a folder of pages the settings name, at `rel`.
    fn gathering(
        &mut self,
        top: &str,
        rel: &str,
        gathering: &Gathering,
    ) -> Result<(), Box<Failed>> {
        let mut names: Vec<String> = std::fs::read_dir(self.path(rel))
            .map(|entries| {
                entries
                    .flatten()
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        for name in names {
            let path = format!("{rel}/{name}");
            let Ok(found) = inside::metadata(self.folder, &self.from(&path)) else {
                continue;
            };
            if found.is_symlink() {
                self.left(&path, inside::why(self.folder, &self.from(&path)));
            } else if found.is_dir() {
                self.gathering(top, &path, gathering)?;
            } else {
                let within = path[top.len()..].trim_start_matches('/').to_owned();
                self.file(
                    &path,
                    Found::Doc {
                        label: gathering.label.clone(),
                        within,
                    },
                )?;
            }
        }
        Ok(())
    }

    /// Reads `_data/` at `rel`: YAML and JSON copied to `data/`.
    fn data(&mut self, rel: &str) -> Result<(), Box<Failed>> {
        let mut names: Vec<String> = std::fs::read_dir(self.path(rel))
            .map(|entries| {
                entries
                    .flatten()
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        let top = self.settings.data_dir.clone();
        for name in names {
            let path = format!("{rel}/{name}");
            let Ok(found) = inside::metadata(self.folder, &self.from(&path)) else {
                continue;
            };
            if found.is_symlink() {
                self.left(&path, inside::why(self.folder, &self.from(&path)));
            } else if found.is_dir() {
                self.data(&path)?;
            } else {
                let ending = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase());
                match ending.as_deref() {
                    Some("yml" | "yaml" | "json") => {
                        let to = format!("data/{}", path[top.len()..].trim_start_matches('/'));
                        let from = self.from(&path);
                        self.import.files.push(super::File { from, to });
                    }
                    _ => self.find(&DATA_LEFT, &path, None, ""),
                }
            }
        }
        Ok(())
    }

    /// Reads one file found as `found`.
    fn file(&mut self, rel: &str, found: Found) -> Result<(), Box<Failed>> {
        let bytes = std::fs::read(self.path(rel)).map_err(|error| {
            Box::<Failed>::from(Problem::nowhere(Kind::CantRead {
                path: self.path(rel),
                reason: Reason::of(&error),
            }))
        })?;
        let name = rel.rsplit('/').next().unwrap_or(rel);
        let ending = name
            .rsplit_once('.')
            .map(|(_, e)| e.to_ascii_lowercase())
            .unwrap_or_default();
        let writing = WRITING.contains(&ending.as_str());
        let fronted = has_front_matter(&bytes);
        if let Found::Post { .. } | Found::Draft { .. } = found {
            let is_post = matches!(found, Found::Post { .. });
            if !(writing && fronted) || (is_post && dated_name(name).is_none()) {
                match (is_post, writing) {
                    (false, _) => self.left(
                        rel,
                        "a file among the drafts that isn't one, which Jekyll doesn't publish",
                    ),
                    (true, true) => self.find(&NOT_A_POST, rel, None, ""),
                    (true, false) => self.left(
                        rel,
                        "a file among the posts that isn't one, which Jekyll doesn't publish there",
                    ),
                }
                return Ok(());
            }
        }
        if !fronted || !writing {
            match (fronted, &found) {
                (true, _) => {
                    if found == Found::Page && self.built_a_feed(rel) {
                        let from = self.from(rel);
                        self.import.feeds.push((format!("/{rel}"), from));
                    }
                    self.left(
                        rel,
                        "a file with front matter that isn't a page, such as a stylesheet or a \
                         feed, which Jekyll built with its templates",
                    )
                }
                (false, Found::Page) => self.import.files.push(super::File {
                    from: self.from(rel),
                    to: format!("static/{rel}"),
                }),
                (false, _) => {
                    self.left(rel, "a file among the pages of a folder that isn't a page")
                }
            }
            return Ok(());
        }
        let Ok(text) = String::from_utf8(bytes) else {
            self.left(rel, "a file that isn't UTF-8 text");
            return Ok(());
        };
        let front = match front(&text, rel) {
            Ok(front) => front,
            Err(headline) => {
                self.find(&FRONT_MATTER_UNREAD, rel, None, &headline);
                return Ok(());
            }
        };
        for (name, _, line, node) in &front.fields {
            if holds_older_number(node) {
                let shown = crate::import::write::shown_value(&yaml::value_of(node, &yaml::core));
                self.find(&NUMBER_LEFT, rel, Some(*line), &format!("{name}: {shown}"));
            }
        }
        self.doc(rel, found, front, matches!(ending.as_str(), "html" | "htm"));
        Ok(())
    }

    /// Whether the old built output shows what Jekyll made of `rel`, a
    /// file of front matter and no writing, to be a feed: it ends `.rss`
    /// or `.atom`, or opens as RSS or Atom.
    fn built_a_feed(&self, rel: &str) -> bool {
        let Some(built) = &self.import.built else {
            return false;
        };
        let path = format!("{built}/{rel}");
        if !inside::a_file(self.folder, &path) {
            return false;
        }
        let ending = rel.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase());
        if matches!(ending.as_deref(), Some("rss" | "atom")) {
            return true;
        }
        let Ok(bytes) = std::fs::read(self.root(&path)) else {
            return false;
        };
        let text = String::from_utf8_lossy(&bytes);
        let mut rest = text.trim_start_matches('\u{feff}').trim_start();
        // The XML declaration and any comment come before the root.
        while let Some(close) = ["<?", "<!--"]
            .iter()
            .zip(["?>", "-->"])
            .find_map(|(open, close)| rest.starts_with(open).then_some(close))
        {
            match rest.find(close) {
                Some(end) => rest = rest[end + close.len()..].trim_start(),
                None => return false,
            }
        }
        ["<rss", "<feed", "<rdf:RDF"].iter().any(|root| {
            rest.strip_prefix(root)
                .is_some_and(|after| after.starts_with([' ', '\t', '\r', '\n', '>']))
        })
    }

    /// The defaults that reach `rel`, of the type `kind`.
    fn defaults(&self, rel: &str, kind: &str) -> Vec<(String, Value)> {
        let mut values: Vec<(String, Value)> = Vec::new();
        for default in &self.settings.defaults {
            let path_fits = default.path.is_empty()
                || rel == default.path
                || rel.starts_with(&format!("{}/", default.path))
                || glob(&default.path, rel);
            let kind_fits = default
                .kind
                .as_deref()
                .is_none_or(|k| k == kind || (kind == "drafts" && k == "posts"));
            if path_fits && kind_fits {
                for (name, value) in &default.values {
                    match values.iter_mut().find(|(n, _)| n == name) {
                        Some(slot) => slot.1 = value.clone(),
                        None => values.push((name.clone(), value.clone())),
                    }
                }
            }
        }
        values
    }

    /// Makes a page, post or document of a file with front matter.
    fn doc(&mut self, rel: &str, found: Found, front: Front, html: bool) {
        let name = rel.rsplit('/').next().unwrap_or(rel);
        let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
        let type_name = match &found {
            Found::Post { .. } => "posts".to_owned(),
            Found::Draft { .. } => "drafts".to_owned(),
            Found::Page => "pages".to_owned(),
            Found::Doc { label, .. } => label.clone(),
        };
        let mut fields: Vec<(String, Value, Option<usize>)> = front
            .fields
            .iter()
            .map(|(n, v, line, _)| (n.clone(), v.clone(), Some(*line)))
            .collect();
        for ((name, value, _), (_, _, _, node)) in fields.iter_mut().zip(&front.fields) {
            if matches!(name.as_str(), "date" | "last_modified_at")
                && let Some(moment) = read_as_utc(node)
            {
                *value = Value::Text(moment);
                self.utc_dates += 1;
            }
        }
        let defaulted = self.defaults(rel, &type_name);
        let mut from_defaults = false;
        for (name, value) in defaulted {
            if !fields.iter().any(|(n, _, _)| *n == name) {
                fields.push((name, value, None));
                from_defaults = true;
            }
        }
        if from_defaults && !self.import.notes.iter().any(|n| n.contains("`defaults`")) {
            self.note(
                "The settings' `defaults` were written into each page they reached, as fields of \
                 its own."
                    .to_owned(),
            );
        }
        // A moment Jekyll showed in a form of its own, so it is no plain
        // text.
        let plain: Vec<(String, String)> = fields
            .iter()
            .filter(|(name, _, _)| name != "date")
            .filter_map(|(name, value, _)| match value {
                Value::Text(text) | Value::Number(text) => Some((name.clone(), text.clone())),
                _ => None,
            })
            .filter(|(_, text)| !date::read(text).is_ok_and(|date| date.time.is_some()))
            .collect();
        let get = |name: &str| {
            fields
                .iter()
                .find(|(n, _, _)| n == name)
                .map(|(_, v, _)| v.clone())
        };
        let (name_date, title_part) = match (&found, dated_name(stem)) {
            (Found::Post { .. } | Found::Draft { .. }, Some((date, rest))) => {
                (Some(date), rest.to_owned())
            }
            _ => (None, stem.to_owned()),
        };
        let title_part = title_part.trim_end_matches('.').to_owned();
        let written_date = get("date").as_ref().and_then(text_of);
        let date = written_date
            .as_deref()
            .and_then(super::write::date_of)
            .or(name_date);
        let published = get("published") != Some(Value::Truth(false));
        let slug_field = get("slug").as_ref().and_then(text_of);
        let mut categories: Vec<String> = Vec::new();
        for value in [get("categories"), get("category")].into_iter().flatten() {
            categories.extend(texts_of(&value));
        }
        if let Found::Post { dirs } | Found::Draft { dirs } = &found {
            categories.extend(dirs.iter().cloned());
        }
        let mut seen = Vec::new();
        categories.retain(|c| {
            let lower = c.to_lowercase();
            let new = !seen.contains(&lower);
            seen.push(lower);
            new
        });
        let own_permalink = get("permalink").as_ref().and_then(text_of);
        let separator = get("excerpt_separator")
            .as_ref()
            .and_then(text_of)
            .or(self.settings.separator.clone());

        let s = &self.settings;
        let style = Style::of(s.permalink.as_deref());
        let output_ext = ".html";
        let moment = date
            .as_ref()
            .and_then(|date| Moment::of(date, s.timezone.as_deref()));
        let gathering = match &found {
            Found::Doc { label, .. } => s.gatherings.iter().find(|g| g.label == *label).cloned(),
            _ => None,
        };
        let doc_values = |name: &str, path: &str, basename: &str| -> Option<String> {
            let slug_source = slug_field.clone().unwrap_or_else(|| title_part.clone());
            Some(match name {
                "title" => url::slugify(&slug_source, true, true),
                "slug" => url::slugify(&slug_source, false, false),
                "name" => url::slugify(basename, false, false),
                "categories" | "slugified_categories" => categories
                    .iter()
                    .map(|c| match name {
                        "categories" => c.to_lowercase(),
                        _ => url::slugify(c, false, false),
                    })
                    .collect::<Vec<_>>()
                    .join("/"),
                "output_ext" => output_ext.to_owned(),
                "path" => path.to_owned(),
                "basename" => basename.to_owned(),
                "collection" => match &found {
                    Found::Doc { label, .. } => label.clone(),
                    _ => "posts".to_owned(),
                },
                other => moment.as_ref()?.piece(other)?,
            })
        };
        let unpublished_gathering = gathering.as_ref().is_some_and(|g| !g.output);
        let mut address = None;
        if published && !matches!(found, Found::Draft { .. }) && !unpublished_gathering {
            let dir = rel.rsplit_once('/').map_or("", |(dir, _)| dir);
            let filled = match &found {
                Found::Post { .. } => {
                    let template = own_permalink
                        .clone()
                        .or(s.posts_permalink.clone())
                        .unwrap_or_else(|| style.post_template());
                    url::fill(&template, &|n| doc_values(n, "", stem))
                }
                Found::Doc { within, .. } => {
                    let path = format!(
                        "/{}",
                        within.rsplit_once('.').map_or(within.as_str(), |(p, _)| p)
                    );
                    let template = own_permalink
                        .clone()
                        .or(gathering.as_ref().and_then(|g| g.permalink.clone()))
                        .unwrap_or_else(|| style.suffixed("/:collection/:path"));
                    url::fill(&template, &|n| doc_values(n, &path, stem))
                }
                _ => {
                    let template = own_permalink
                        .clone()
                        .unwrap_or_else(|| match stem == "index" {
                            true => "/:path/".to_owned(),
                            false => style.suffixed("/:path/:basename"),
                        });
                    let path = format!("/{dir}");
                    url::fill(&template, &|n| match n {
                        "path" => Some(path.clone()),
                        "basename" => Some(stem.to_owned()),
                        "output_ext" => Some(output_ext.to_owned()),
                        _ => None,
                    })
                }
            };
            match filled {
                Ok(filled) => address = Some(filled),
                Err(piece) => {
                    let line = front
                        .fields
                        .iter()
                        .find(|(name, _, _, _)| name == "permalink")
                        .map(|(_, _, line, _)| *line);
                    self.find(&ADDRESS_UNKNOWN, rel, line, &piece);
                }
            }
        }

        let to = match &found {
            Found::Post { .. } | Found::Draft { .. } => format!("posts/{title_part}.md"),
            Found::Page => format!("{}.md", rel.rsplit_once('.').map_or(rel, |(p, _)| p)),
            Found::Doc { label, within } => {
                format!(
                    "{label}/{}.md",
                    within.rsplit_once('.').map_or(within.as_str(), |(p, _)| p)
                )
            }
        };
        if found == Found::Page && is_template(&front.body, &fields, &self.settings.gatherings) {
            let at = template_mark(&front.body);
            let (line, text) = match at {
                Some((offset, text)) => (
                    Some(front.body_line + front.body[..offset].matches('\n').count()),
                    text,
                ),
                None => (None, String::new()),
            };
            self.find(&TEMPLATE_PAGE, rel, line, &text);
            if let Some(address) = address {
                let from = self.from(rel);
                self.import.templated.push((address, from));
            }
            return;
        }

        let mut out: Vec<Field> = Vec::new();
        let mut others = Vec::new();
        let mut has_summary = fields.iter().any(|(n, _, _)| n == "summary");
        let has_description = fields.iter().any(|(n, _, _)| n == "description");
        let has_updated = fields.iter().any(|(n, _, _)| n == "updated");
        let mut layout = false;
        let mut draft_written = false;
        for (name, value, line) in fields {
            match name.as_str() {
                "layout" => layout = true,
                // Its work is done once the separators are converted.
                "excerpt_separator" => {}
                "permalink" => {}
                "redirect_from" => others.extend(texts_of(&value)),
                "published" => {
                    if value == Value::Truth(false) {
                        out.push(Field::new("draft", Value::Truth(true), line));
                        draft_written = true;
                    }
                }
                "last_modified_at" if !has_updated => out.push(Field::new("updated", value, line)),
                "description" if !has_summary => {
                    out.push(Field::new("summary", value, line));
                    has_summary = true;
                }
                "excerpt" if !has_summary && !has_description => {
                    out.push(Field::new("summary", value, line));
                    has_summary = true;
                }
                // The shared part makes each category a tag.
                "categories" | "category" => {}
                "tags" => {
                    let value = match value {
                        Value::Text(text) => Value::List(
                            text.split_whitespace()
                                .map(|t| Value::Text(t.to_owned()))
                                .collect(),
                        ),
                        other => other,
                    };
                    out.push(Field::new(&name, value, line));
                }
                _ => out.push(Field::new(&name, value, line)),
            }
        }
        if layout {
            self.layouts += 1;
        }
        let is_post = matches!(found, Found::Post { .. } | Found::Draft { .. });
        if is_post && !out.iter().any(|f| f.name == "title") {
            out.insert(
                0,
                Field::new("title", Value::Text(titleize(&title_part)), None),
            );
        }
        if is_post
            && !out.iter().any(|f| f.name == "date")
            && let Some(day) = name_date
        {
            let at = usize::from(out.first().is_some_and(|f| f.name == "title"));
            out.insert(at, Field::new("date", Value::Text(day.iso_day()), None));
        }
        let draft = matches!(found, Found::Draft { .. }) || unpublished_gathering;
        if draft && !draft_written {
            out.push(Field::new("draft", Value::Truth(true), None));
        }
        if unpublished_gathering {
            self.find(&UNPUBLISHED, rel, None, "");
        }
        if html {
            self.find(&HTML_WRITING, rel, None, "");
        }
        let kind = match (draft || draft_written, date.is_some() || is_post) {
            (true, _) => PageKind::Draft,
            (false, true) if is_post || matches!(found, Found::Doc { .. }) => PageKind::Post,
            _ => PageKind::Page,
        };
        let mut post_names = Vec::new();
        if let Found::Post { dirs } = &found {
            post_names.push(stem.to_owned());
            if !dirs.is_empty() {
                post_names.push(format!("{}/{stem}", dirs.join("/")));
            }
        }
        self.docs.push(Doc {
            from: self.from(rel),
            rel: rel.to_owned(),
            to,
            kind,
            fields: out,
            body: front.body,
            body_line: front.body_line,
            address,
            own_address: own_permalink.is_some(),
            others,
            separator,
            post_names,
            plain,
            terms: match categories.is_empty() {
                true => Vec::new(),
                false => vec![("categories".to_owned(), categories)],
            },
        });
    }

    /// Converts each page's writing, once every page's address is known,
    /// and hands the pages, the snippets standing for includes, and the
    /// settings to the import.
    fn finish(&mut self) {
        let mut posts = HashMap::new();
        let mut links = HashMap::new();
        for doc in &self.docs {
            if let Some(address) = &doc.address {
                for name in &doc.post_names {
                    posts.insert(name.clone(), address.clone());
                }
                links.insert(doc.rel.clone(), address.clone());
            }
        }
        for file in &self.import.files {
            if let Some(rel) = file.to.strip_prefix("static/") {
                links.insert(rel.to_owned(), format!("/{rel}"));
            }
        }
        let host = self.settings.url.clone();
        let base = match self.import.address.is_some() {
            true => self.settings.base.clone(),
            false => String::new(),
        };
        let mut calls: Vec<liquid::Call> = Vec::new();
        let docs = std::mem::take(&mut self.docs);
        let mut set_apart = 0;
        for doc in docs {
            let cx = liquid::Context {
                base: &base,
                host: host.as_deref(),
                posts: &posts,
                links: &links,
                separator: doc.separator.as_deref(),
                page: &doc.plain,
            };
            let converted = liquid::convert(&doc.body, doc.body_line, &doc.from, &cx);
            self.import.findings.extend(converted.findings);
            set_apart += converted.set_apart;
            calls.extend(converted.calls);
            self.import.pages.push(Page {
                from: doc.from,
                to: doc.to,
                kind: doc.kind,
                front: doc.fields,
                writing: converted.writing,
                terms: doc.terms,
                fences: converted.fences,
                calls: converted.called,
                address: doc.address,
                own_address: doc.own_address,
                other_addresses: doc.others,
            });
        }
        let mut snippets: Vec<String> = Vec::new();
        for call in &calls {
            if !snippets.contains(&call.snippet) {
                snippets.push(call.snippet.clone());
            }
        }
        for snippet in snippets {
            let of: Vec<&liquid::Call> = calls.iter().filter(|c| c.snippet == snippet).collect();
            let mut names: Vec<String> = Vec::new();
            for call in &of {
                for name in &call.names {
                    if !names.contains(name) {
                        names.push(name.clone());
                    }
                }
            }
            let path = format!("snippets/{snippet}.html");
            let text =
                super::liquid::stand_in(&format!("include `{}`", of[0].include), &names, false);
            self.import.written.push((path.clone(), text));
            self.import.findings.push(Finding {
                kind: &INCLUDE,
                about: Some(path),
                spots: of.iter().map(|call| call.spot.clone()).collect(),
            });
        }
        if set_apart > 0 {
            self.note(format!(
                "Statac put a blank line after {} that had a link definition on the next line: \
                 Jekyll read such a line as a link definition, and Statac's Markdown reads it as \
                 more of the footnote.",
                match set_apart {
                    1 => "1 footnote definition".to_owned(),
                    n => format!("{n} footnote definitions"),
                }
            ));
        }
        if self.utc_dates > 0 {
            self.note(format!(
                "{} a time and no offset, which Jekyll read as UTC whatever the site's time zone: \
                 {} written with `Z`, so it means the same moment.",
                match self.utc_dates {
                    1 => "1 date had".to_owned(),
                    n => format!("{n} dates had"),
                },
                match self.utc_dates {
                    1 => "it is",
                    _ => "each is",
                }
            ));
        }
        let layouts = self.layouts;
        if layouts > 0 {
            self.note(format!(
                "`layout` was taken off {}: each is built with the template for its kind, as \
                 https://statac.dev/templates/ describes.",
                match layouts {
                    1 => "1 page".to_owned(),
                    n => format!("{n} pages"),
                }
            ));
        }
        self.paging();
        if self.settings.feed {
            self.import
                .settings
                .push(("feed_ids_before".into(), Value::Text(self.today.iso_day())));
            self.import
                .settings
                .push(("feed_ids_from".into(), Value::Text("jekyll".into())));
            let path = self
                .settings
                .feed_path
                .clone()
                .unwrap_or_else(|| "feed.xml".into());
            let address = format!("/{}", path.trim_start_matches('/'));
            let feed = match address == format!("/{}", crate::feeds::FILE_NAME) {
                true => Value::Truth(true),
                false => Value::Text(address.clone()),
            };
            self.import
                .settings
                .push((crate::feeds::SETTING.into(), feed));
            self.import.addresses.push(address);
            self.note(
                "The old site's feed came from Jekyll's feed plugin, so `feed_ids_before` and \
                 `feed_ids_from: jekyll` keep each post's id in it as it was, and feed readers \
                 see nothing new: see https://statac.dev/feeds/#entry-ids."
                    .to_owned(),
            );
        }
    }

    /// How many posts a page of a list shows, and the old home page's
    /// later pages, which the posts' own list now stands in for.
    fn paging(&mut self) {
        let Some(per_page) = self.settings.paginate else {
            return;
        };
        self.import
            .settings
            .push(("posts_per_page".into(), Value::Number(per_page.to_string())));
        let home = ["index.html", "index.md", "index.markdown"]
            .iter()
            .any(|name| self.path(name).is_file());
        if !home {
            return;
        }
        let path = self
            .settings
            .paginate_path
            .clone()
            .unwrap_or_else(|| "/page:num".into());
        let posts = self
            .import
            .pages
            .iter()
            .filter(|page| page.kind == PageKind::Post && page.to.starts_with("posts/"))
            .count();
        if posts == 0 {
            return;
        }
        let per_page = usize::try_from(per_page).unwrap_or(usize::MAX);
        let addresses: Vec<String> = (2..=posts.div_ceil(per_page.max(1)))
            .map(|number| {
                let filled = path.replace(":num", &number.to_string());
                format!("/{}/", filled.trim_matches('/'))
            })
            .collect();
        self.import.addresses.extend(addresses.iter().cloned());
        self.note(format!(
            "The old site's home page showed its posts {per_page} to a page. {}, and the posts' \
             list page shows them {per_page} to a page, as `posts_per_page` in `statac.yaml` says \
             for every list: see https://statac.dev/lists/#pages-of-a-list.",
            one_home_page(true)
        ));
        self.import.home_pages = Some(HomePages {
            per_page,
            addresses,
            ties_reversed: true,
        });
    }
}

/// A post's title made from its name as Jekyll makes one: each word
/// parted by a hyphen, with a capital and the rest small.
fn titleize(slug: &str) -> String {
    slug.split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => {
                    first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase()
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether a page is the templates' work: its writing loops over the
/// site's pages, or it has no writing and sends no reader on.
fn is_template(
    body: &str,
    fields: &[(String, Value, Option<usize>)],
    gatherings: &[Gathering],
) -> bool {
    if body.trim().is_empty() {
        return !fields.iter().any(|(name, _, _)| name == "redirect_to");
    }
    template_mark(body).is_some()
        || gatherings
            .iter()
            .any(|g| body.contains(&format!("site.{}", g.label)) && body.contains("{%"))
}

/// The first tag in `body` that reads the site's pages, by its offset.
fn template_mark(body: &str) -> Option<(usize, String)> {
    let mut at = 0;
    while let Some(found) = body[at..].find("{%").or_else(|| body[at..].find("{{")) {
        let start = at + found;
        let close = if body[start..].starts_with("{%") {
            "%}"
        } else {
            "}}"
        };
        let end = body[start..]
            .find(close)
            .map_or(body.len(), |i| start + i + 2);
        let tag = &body[start..end];
        if LISTING.iter().any(|name| tag.contains(name)) {
            return Some((start, tag.to_owned()));
        }
        at = end;
    }
    None
}

/// Whether `rel` fits `pattern` written with `*`, any run of characters
/// within one folder's name.
fn glob(pattern: &str, rel: &str) -> bool {
    if !pattern.contains('*') {
        return false;
    }
    fn fits(p: &[u8], t: &[u8]) -> bool {
        match (p.first(), t.first()) {
            (None, None) => true,
            (Some(b'*'), _) => {
                fits(&p[1..], t) || (t.first().is_some_and(|c| *c != b'/') && fits(p, &t[1..]))
            }
            (Some(a), Some(b)) if a == b => fits(&p[1..], &t[1..]),
            _ => false,
        }
    }
    fits(pattern.as_bytes(), rel.as_bytes())
}
