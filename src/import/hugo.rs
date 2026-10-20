//! The reader for sites Hugo made: a site with `hugo.toml`, `hugo.yaml` or
//! `hugo.json` at its top, a `config.toml` or the like that sets
//! `baseURL`, or a `config/_default/` folder.
//!
//! Hugo's rules are followed as its documentation gives them: which
//! folders it reads, where it publishes each page, list and term, how it
//! makes a name ready for an address, which dates it reads from where, and
//! what its own tags do in the writing.

mod settings;
mod url;
mod writing;

use std::collections::HashSet;
use std::path::Path;

use super::common::{dated_name, front, joined, last, names_in, parent, text_of, texts_of};
use super::inside;
use super::report::code;
use super::{
    Failed, Field, File, Finding, FindingKind, Import, Left, Page, PageKind, Value, left_behind,
    one_home_page,
};
use crate::content::slug;
use crate::date::{self, Date};
use crate::problem::{Kind, Problem, system_reason};
use settings::{Settings, get, truth_of};
use url::{Making, Pieces};

/// The endings of the files Hugo reads as Markdown.
const MARKDOWN: &[&str] = &["md", "markdown", "mdown", "mkdn", "mkd", "mdwn"];

/// The endings of the files Hugo reads as HTML.
const HTML: &[&str] = &["html", "htm"];

/// The endings of the files Hugo reads as writing in a format Statac
/// doesn't.
const OTHER_FORMATS: &[&str] = &["adoc", "asciidoc", "ad", "org", "pandoc", "pdc", "rst"];

/// The names Hugo gives a meaning to in front matter, which it reads in
/// any capitals.
const HUGOS_WORDS: &[&str] = &[
    "aliases",
    "audio",
    "build",
    "cascade",
    "date",
    "description",
    "draft",
    "expirydate",
    "headless",
    "images",
    "iscjklanguage",
    "keywords",
    "lastmod",
    "layout",
    "linktitle",
    "markup",
    "menu",
    "menus",
    "modified",
    "outputs",
    "params",
    "pubdate",
    "publishdate",
    "published",
    "resources",
    "sitemap",
    "slug",
    "summary",
    "title",
    "translationkey",
    "type",
    "unpublishdate",
    "url",
    "videos",
    "weight",
];

/// The names a page's dates are read from.
const DATE_NAMES: &[&str] = &[
    "date",
    "publishdate",
    "pubdate",
    "published",
    "lastmod",
    "modified",
    "expirydate",
    "unpublishdate",
];

pub static TAG_LEFT: FindingKind = FindingKind {
    heading: "Tags in the writing left as written",
    sentence: "Statac's writing runs none of Hugo's tags, and these have no equal in it, so each \
               is left as written and shows on the page as it stands: write by hand what it \
               made, or call a snippet, as https://statac.dev/snippets/ describes.",
};

pub static TAG_IN_CODE: FindingKind = FindingKind {
    heading: "Tags inside code",
    sentence: "Hugo ran these even inside code, and Statac shows code exactly as written, so each \
               now shows as it stands: write by hand what it made, where that was meant.",
};

pub static TAG_IN_ADDRESS: FindingKind = FindingKind {
    heading: "Tags inside an address",
    sentence: "Each of these tags stands inside a link's address, where Statac runs no snippet, \
               so it is left as written and the link leads nowhere: write the address the tag \
               made in its place.",
};

pub static LINK_NOWHERE: FindingKind = FindingKind {
    heading: "Links to pages that didn't come across",
    sentence: "Each of these names a page Statac couldn't find in the old site, so it is left as \
               written: link to the page's address instead.",
};

pub static SNIPPET_CALLS: FindingKind = FindingKind {
    heading: "Tags now snippet calls",
    sentence: "Each of these tags is now a call to a snippet of its name, and the snippet holds \
               only a comment saying it wasn't converted: write the snippet, as \
               https://statac.dev/snippets/ describes, or take the calls out.",
};

pub static FIGURE_HTML: FindingKind = FindingKind {
    heading: "Figures written as HTML",
    sentence: "Each of these figures shows a picture kept elsewhere or one the starter's `figure` \
               snippet can't show, such as a GIF, stands inside HTML, or asks for what the \
               snippet has no equal for, such as a link, so Statac wrote the HTML Hugo made for \
               it.",
};

pub static CODE_OPTIONS: FindingKind = FindingKind {
    heading: "Options on code taken off",
    sentence: "Statac reads the rest of a code block's first line as its caption, so options such \
               as `{linenos=true}`, and those handed to `highlight`, were taken off each of these: \
               the code is shown without them.",
};

pub static RAW_HTML: FindingKind = FindingKind {
    heading: "HTML Hugo left out",
    sentence: "Hugo's settings left HTML written in the writing out of the page, and Statac \
               publishes it as it is, so each of these now shows: read each page that has one, \
               and take out what wasn't meant to show.",
};

pub static EMOJI: FindingKind = FindingKind {
    heading: "Emoji written by name",
    sentence: "Hugo showed each of these names as its emoji where it knew one by the name, as the \
               settings' `enableEmoji` asked, and Statac shows the name as written: write the \
               emoji itself in its place, wherever it stands.",
};

pub static ATTRIBUTE_LINES: FindingKind = FindingKind {
    heading: "Attribute lists",
    sentence: "Hugo's settings read `{.class}` below a block as the block's own, and Statac's \
               Markdown reads it as words, so each shows on the page as written: read each page \
               that has one, and take it out or write the HTML it stood for.",
};

pub static SUMMARY_CUT: FindingKind = FindingKind {
    heading: "Summaries Hugo cut by length",
    sentence: "Hugo summed up each of these pages with its opening paragraphs, as many as made \
               its `summaryLength` of words, and Statac takes the first paragraph alone: read \
               each, and put `<!--more-->` where its summary should end.",
};

pub static PUBLISH_DATE: FindingKind = FindingKind {
    heading: "Pages published on a day of their own",
    sentence: "Hugo published each of these pages from its `publishDate`, a day other than its \
               `date`, and Statac publishes a page on its `date`, so the line was taken off: \
               where the page should wait, write that day as its `date`.",
};

pub static EXPIRY_DATE: FindingKind = FindingKind {
    heading: "Pages published until a day",
    sentence: "Hugo stopped publishing each of these pages after its `expiryDate`, which Statac \
               has no equal for, so the line was taken off: those already past that day came \
               across with `draft: true`, and the others stay published until you give them \
               the same.",
};

pub static DATE_ELSEWHERE: FindingKind = FindingKind {
    heading: "Dates Hugo took from the file or from Git",
    sentence: "Hugo's settings took the date of each of these pages from the time its file was \
               saved or from Git, which Statac doesn't read, so none was written: write its \
               `date`.",
};

pub static UNPUBLISHED: FindingKind = FindingKind {
    heading: "Pages Hugo didn't publish",
    sentence: "Each of these pages was `headless`, or its `build` kept it from being published, \
               so it came across with `draft: true`: take that line out of each you want \
               published.",
};

pub static ADDRESS_UNKNOWN: FindingKind = FindingKind {
    heading: "Addresses Statac couldn't work out",
    sentence: "Each of these pages is published by a pattern in `[permalinks]` holding a \
               placeholder Statac doesn't know, so its old address wasn't worked out or kept: \
               give the page a `permalink` of the address the old site published it at.",
};

pub static FRONT_MATTER_UNREAD: FindingKind = FindingKind {
    heading: "Front matter Statac can't read",
    sentence: "Statac's reader stopped at this line in the front matter of each of these, so the \
               page didn't come across: put it right and import again.",
};

pub static DATA_LEFT: FindingKind = FindingKind {
    heading: "Data files left behind",
    sentence: "Statac reads data files written in YAML or JSON, and writes those in TOML as YAML, \
               so these weren't copied: each is in another format, or its TOML stops reading at \
               the line given. Write each as YAML or JSON in `data/`.",
};

pub static SETTINGS_LEFT: FindingKind = FindingKind {
    heading: "Settings with no equal",
    sentence: "Statac has no equal for each of these settings, so nothing came across for them: \
               https://statac.dev/settings/ says what Statac offers.",
};

pub static FORMAT_LEFT: FindingKind = FindingKind {
    heading: "Pages in formats Statac doesn't read",
    sentence: "Hugo read each of these pages in a format other than Markdown or HTML, such as \
               AsciiDoc or Org, which Statac doesn't, so it didn't come across: write it in \
               Markdown.",
};

/// Whether `folder` holds a Hugo site.
pub fn owns(folder: &Path) -> bool {
    settings::owns(folder)
}

/// Reads the site in `folder`, for a new site with the starter where
/// `starter` says.
pub fn read(
    folder: &Path,
    today: Date,
    given: Option<&str>,
    starter: bool,
) -> Result<Import, Box<Failed>> {
    let found = settings::found(folder)?;
    let s = settings::settle(folder, &found.entries);
    let mut reader = Reader {
        folder,
        given,
        starter,
        import: Import::default(),
        making: Making {
            lower: s.lower,
            accents_removed: s.accents_removed,
        },
        s,
        today,
        main: String::new(),
        languages: Vec::new(),
        docs: Vec::new(),
        own: Vec::new(),
        own_files: Vec::new(),
        taken_off: 0,
        updated_elsewhere: 0,
        older_lists: None,
        files_later: None,
    };
    reader.import.settings_file = found.file;
    reader.import.left = found.left;
    reader.settled();
    reader.walk()?;
    reader.finish();
    Ok(reader.import)
}

/// What a page is to Hugo.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Shape {
    /// A page that is one file.
    Single,
    /// A page kept as a folder, `index.md` with its files beside it.
    Bundle,
    /// A folder's own page, `_index.md`.
    List,
    /// A folder's own page Hugo made, which Statac writes to keep what
    /// Hugo gave the list.
    Made,
}

/// A page as found, before its writing is converted.
struct Doc {
    /// The folder of writing it was found in.
    root: String,
    from: String,
    to: String,
    language: String,
    shape: Shape,
    kind: PageKind,
    fields: Vec<Field>,
    words: Vec<Field>,
    body: String,
    body_line: usize,
    html: bool,
    /// The folder the page stands in, under its language's content: for a
    /// folder's own page, that folder.
    dir: String,
    /// The file's name without its ending or language, or the folder's
    /// for a page kept as a folder.
    name: String,
    slug: Option<String>,
    url: Option<String>,
    date: Option<Date>,
    title: String,
    aliases: Vec<String>,
    weight: i64,
    /// Every field of its front matter, Hugo's names in small letters.
    params: Vec<(String, Value)>,
    /// Whether Hugo cut the page's summary, rather than the page giving
    /// one.
    summary_cut: bool,
    terms: Vec<(String, Vec<String>)>,
    draft: bool,
    address: Option<String>,
    own_address: bool,
    others: Vec<String>,
}

struct Reader<'a> {
    folder: &'a Path,
    /// The site's address as the author gave it, for settings that give
    /// none.
    given: Option<&'a str>,
    /// Whether the new site has the starter.
    starter: bool,
    import: Import,
    s: Settings,
    making: Making,
    today: Date,
    /// The main language.
    main: String,
    /// Every language the settings declare, the main one first; empty on
    /// a site in one language.
    languages: Vec<String>,
    docs: Vec<Doc>,
    /// The names of the site's own tags.
    own: Vec<String>,
    /// What the file of each of the site's own tags holds, by the tag's
    /// name.
    own_files: Vec<(String, String)>,
    /// How many pages had `layout` or `type` taken off.
    taken_off: usize,
    /// How many pages had their last change from the file or Git.
    updated_elsewhere: usize,
    /// A list Hugo before 0.148 may have published as a file, and one of
    /// its later pages, where some such list had them.
    older_lists: Option<(String, String)>,
    /// One of the later pages of a tag's page that Hugo published as
    /// files, where some tag had them.
    files_later: Option<String>,
}

/// A file's name split into its stem and its ending, in small letters.
fn split_name(name: &str) -> (&str, String) {
    match name.rsplit_once('.') {
        Some((stem, ending)) if !stem.is_empty() => (stem, ending.to_ascii_lowercase()),
        _ => (name, String::new()),
    }
}

impl Reader<'_> {
    fn path(&self, rel: &str) -> std::path::PathBuf {
        inside::path(self.folder, rel)
    }

    /// Whether `rel`, a path the writing names, is a file inside the old
    /// site, reached through no link that leads outside it.
    fn a_file_named(&self, rel: &str) -> bool {
        crate::platform::stays_inside(rel) && inside::a_file(self.folder, rel)
    }

    fn left(&mut self, path: &str, what: &str) {
        self.import.left.push(Left {
            path: path.to_owned(),
            what: what.to_owned(),
        });
    }

    fn find(&mut self, kind: &'static FindingKind, file: &str, line: Option<usize>, text: &str) {
        self.import
            .findings
            .push(Finding::at(kind, file, line, text));
    }

    fn note(&mut self, text: String) {
        self.import.notes.push(text);
    }

    fn several(&self) -> bool {
        self.languages.len() > 1
    }

    /// The folder a language's pages stand in under `content/`, and the
    /// path its addresses start with, each ending `/` where there is one.
    fn prefix(&self, language: &str) -> String {
        let own_folder = match self.several() {
            true => language != self.main || self.s.default_in_subdir,
            false => self.s.default_in_subdir,
        };
        match own_folder {
            true => format!("{language}/"),
            false => String::new(),
        }
    }

    /// What the settings give the import: its title, address, languages
    /// and zone, the author's own values, and what is said of each.
    fn settled(&mut self) {
        let s = &self.s;
        self.import.title = s.title.clone();
        self.import.maths = super::Maths {
            inline: s.maths.0,
            display: s.maths.1,
        };
        let mut notes = Vec::new();
        match (&s.base_url, self.given) {
            (Some(url), _) if url.starts_with("https://") || url.starts_with("http://") => {
                self.import.address = Some(url.trim_end_matches('/').to_owned());
            }
            (_, Some(given)) => self.import.address = Some(given.to_owned()),
            (Some(url), None) if url.trim_matches('/').is_empty() => {}
            (Some(url), None) => notes.push(format!(
                "The settings' `baseURL`, {}, isn't a web address, so the new site has no `url` \
                 yet: add it to `statac.yaml` once it is known.",
                code(url)
            )),
            (None, None) => {}
        }
        let mut declared = s.languages.clone();
        declared.sort_by(|a, b| a.weight.cmp(&b.weight).then_with(|| a.key.cmp(&b.key)));
        let default = s
            .default_language
            .clone()
            .or_else(|| declared.first().map(|l| l.key.clone()))
            .unwrap_or_else(|| "en".to_owned());
        if declared.len() > 1 {
            self.main = default.clone();
            self.languages = std::iter::once(default.clone())
                .chain(
                    declared
                        .iter()
                        .map(|l| l.key.clone())
                        .filter(|k| *k != default),
                )
                .collect();
            let unknown: Vec<String> = self
                .languages
                .iter()
                .filter(|code_| !crate::new::a_language_code(code_))
                .map(|c| code(c))
                .collect();
            if !unknown.is_empty() {
                notes.push(format!(
                    "The settings name the languages {}, which aren't codes Statac reads: write \
                     each language's code in `language` in `statac.yaml`, and name its folder \
                     in `content/` the same.",
                    unknown.join(", ")
                ));
            }
            self.import.languages = self.languages.clone();
            for language in &declared {
                if let Some(title) = &language.title
                    && Some(title) != s.title.as_ref()
                {
                    self.import.settings.push((
                        format!("{}.title", language.key),
                        Value::Text(title.clone()),
                    ));
                }
            }
            notes.push(format!(
                "The settings declare {} languages, so the new site is in each, the main one, \
                 {}, first: each other language's pages are in its own folder in `content/`, \
                 where they stand at the same place as the page they translate, as \
                 https://statac.dev/dates-and-languages/#a-site-in-several-languages \
                 describes.",
                self.languages.len(),
                code(&self.main)
            ));
        } else {
            self.main = declared.first().map_or(default.clone(), |l| l.key.clone());
            let named = s
                .language_code
                .clone()
                .or(s.default_language.clone())
                .or(declared.first().map(|l| l.key.clone()));
            match named {
                Some(code_) if crate::new::a_language_code(&code_) => {
                    self.import.languages = vec![code_]
                }
                Some(written) => notes.push(format!(
                    "The settings name the language {}, which isn't a code Statac reads, so the \
                     site is in English, `en`: write its code in `language` in `statac.yaml`.",
                    code(&written)
                )),
                None => notes.push(
                    "The settings name no language, so the site is in English, `en`: change \
                     `language` in `statac.yaml` if it isn't."
                        .to_owned(),
                ),
            }
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
        for note in notes {
            self.note(note);
        }
        if !self.s.params.is_empty() {
            self.import.settings_file = self.s.params_file.clone();
        }
        self.import.values = self.s.params.clone();
        let left = std::mem::take(&mut self.s.left);
        for (file, line, text) in left {
            self.find(&SETTINGS_LEFT, &file, line, &text);
        }
        let unread = std::mem::take(&mut self.s.unread);
        for (file, line, text) in unread {
            self.find(&inside::FOLDER_UNREAD, &file, line, &text);
        }
        if inside::a_folder(self.folder, &self.s.publish_dir) {
            self.import.built = Some(self.s.publish_dir.clone());
        }
    }

    /// The folders of writing, each with the language it holds where it
    /// holds one alone.
    fn content_roots(&self) -> Vec<(String, Option<String>)> {
        let mut roots = Vec::new();
        let by_folder: Vec<(String, String)> = match self.several() {
            true => self
                .s
                .languages
                .iter()
                .filter_map(|l| Some((l.content_dir.clone()?, l.key.clone())))
                .collect(),
            false => Vec::new(),
        };
        let shared = self
            .languages
            .iter()
            .any(|key| !by_folder.iter().any(|(_, k)| k == key));
        if shared || by_folder.is_empty() {
            roots.push((self.s.content_dir.clone(), None));
        }
        for (dir, key) in by_folder {
            if !roots.iter().any(|(d, _)| *d == dir) {
                roots.push((dir, Some(key)));
            }
        }
        roots
    }

    /// Reads the top of the old site and every folder Hugo reads.
    fn walk(&mut self) -> Result<(), Box<Failed>> {
        let roots = self.content_roots();
        let top_of = |path: &str| path.split('/').next().unwrap_or(path).to_owned();
        let mut content_tops: Vec<String> = roots.iter().map(|(dir, _)| top_of(dir)).collect();
        content_tops.dedup();
        for shortcodes in [
            format!("{}/shortcodes", self.s.layout_dir),
            format!("{}/_shortcodes", self.s.layout_dir),
        ] {
            self.own_tags(&shortcodes, "");
        }
        let themes = match inside::a_folder(self.folder, &self.s.themes_dir) {
            true => names_in(&self.path(&self.s.themes_dir)),
            false => Ok(Vec::new()),
        };
        if let Ok(themes) = themes {
            for theme in themes {
                let dir = format!("{}/{theme}/layouts/shortcodes", self.s.themes_dir);
                self.own_tags(&dir, "");
            }
        }
        for name in names_in(self.folder)? {
            let found = inside::metadata(self.folder, &name);
            let Ok(found) = found else { continue };
            let dir = found.is_dir();
            let shown = if dir {
                format!("{name}/")
            } else {
                name.clone()
            };
            if found.is_symlink() {
                self.left(&name, inside::why(self.folder, &name));
                continue;
            }
            if settings::FILES.contains(&name.as_str()) || (dir && name == "config") {
                continue;
            }
            if content_tops.contains(&name) {
                continue;
            }
            let s = &self.s;
            if s.static_dirs.iter().any(|d| top_of(d) == name) {
                continue;
            }
            if dir && name == top_of(&s.data_dir) {
                continue;
            }
            let starter = self.starter;
            let special = [
                (
                    s.publish_dir.as_str(),
                    "the old site's built output, read only to check the addresses".to_owned(),
                ),
                (
                    s.layout_dir.as_str(),
                    left_behind(
                        "the old site's layouts",
                        "the starter's templates take",
                        starter,
                    ),
                ),
                (
                    s.themes_dir.as_str(),
                    left_behind(
                        "the old site's themes",
                        "the starter's templates, snippets and stylesheet take",
                        starter,
                    ),
                ),
                (
                    s.asset_dir.as_str(),
                    left_behind(
                        "the files Hugo's templates worked on, such as stylesheets",
                        "the starter's `assets/site.css` takes",
                        starter,
                    ),
                ),
                (
                    s.archetype_dir.as_str(),
                    "the old site's patterns for new pages, which Statac has no equal for"
                        .to_owned(),
                ),
                (
                    s.i18n_dir.as_str(),
                    left_behind(
                        "the words the old site's templates showed in each language",
                        "the starter's `data/words.yaml` takes",
                        starter,
                    ),
                ),
                (
                    s.resource_dir.as_str(),
                    "the files Hugo kept from earlier builds".to_owned(),
                ),
            ];
            match special.iter().find(|(special, _)| *special == name) {
                Some((_, what)) if dir => {
                    let what = what.clone();
                    self.left(&shown, &what);
                }
                _ => {
                    let what = match dir {
                        true => "a folder Hugo doesn't publish",
                        false => "a file Hugo doesn't publish",
                    };
                    self.left(&shown, what);
                }
            }
        }
        for top in &content_tops {
            if roots.iter().any(|(dir, _)| dir == top) {
                continue;
            }
            for name in names_in(&self.path(top)).unwrap_or_default() {
                let path = joined(top, &name);
                let within_a_root = roots
                    .iter()
                    .any(|(dir, _)| *dir == path || dir.starts_with(&format!("{path}/")));
                if !within_a_root {
                    let shown = match self.path(&path).is_dir() {
                        true => format!("{path}/"),
                        false => path,
                    };
                    self.left(
                        &shown,
                        "a file in no language's folder, which Hugo doesn't read",
                    );
                }
            }
        }
        for (dir, language) in roots {
            if inside::a_folder(self.folder, &dir) {
                self.content(&dir, language.as_deref(), "")?;
            }
        }
        let statics = self.s.static_dirs.clone();
        for dir in statics {
            if inside::a_folder(self.folder, &dir) {
                self.copied(&dir, "static")?;
            }
        }
        let data = self.s.data_dir.clone();
        if inside::a_folder(self.folder, &data) {
            self.data(&data, "")?;
        }
        Ok(())
    }

    /// Learns the names of the site's own tags from the folder `dir` of
    /// its layouts, and those within it.
    fn own_tags(&mut self, dir: &str, within: &str) {
        if !inside::a_folder(self.folder, &joined(dir, within)) {
            return;
        }
        let Ok(names) = names_in(&self.path(&joined(dir, within))) else {
            return;
        };
        for name in names {
            let rel = joined(within, &name);
            if self.path(&joined(dir, &rel)).is_dir() {
                self.own_tags(dir, &rel);
                continue;
            }
            let stem = name.split('.').next().unwrap_or(&name);
            let tag = joined(within, stem);
            if !self.own.contains(&tag) {
                let file = joined(dir, &rel);
                if inside::a_file(self.folder, &file)
                    && let Ok(text) = std::fs::read_to_string(self.path(&file))
                {
                    self.own_files.push((tag.clone(), text));
                }
                self.own.push(tag);
            }
        }
    }

    /// Copies every file of the folder `top` to the same place beneath
    /// `to` in the new site.
    fn copied(&mut self, top: &str, to: &str) -> Result<(), Box<Failed>> {
        for within in self.files_under(top, "")? {
            self.import.files.push(File {
                from: joined(top, &within),
                to: format!("{to}/{within}"),
            });
        }
        Ok(())
    }

    /// Every file beneath the folder `top`/`rel`, by its path in `top`.
    fn files_under(&mut self, top: &str, rel: &str) -> Result<Vec<String>, Box<Failed>> {
        let mut files = Vec::new();
        for name in names_in(&self.path(&joined(top, rel)))? {
            let within = joined(rel, &name);
            let path = joined(top, &within);
            let Ok(found) = inside::metadata(self.folder, &path) else {
                continue;
            };
            if found.is_symlink() {
                self.left(&path, inside::why(self.folder, &path));
            } else if found.is_dir() {
                files.extend(self.files_under(top, &within)?);
            } else {
                files.push(within);
            }
        }
        Ok(files)
    }

    /// Reads the data folder at `rel`: YAML and JSON copied to `data/`,
    /// TOML written as YAML.
    fn data(&mut self, top: &str, rel: &str) -> Result<(), Box<Failed>> {
        for name in names_in(&self.path(&joined(top, rel)))? {
            let within = joined(rel, &name);
            let path = joined(top, &within);
            let Ok(found) = inside::metadata(self.folder, &path) else {
                continue;
            };
            if found.is_symlink() {
                self.left(&path, inside::why(self.folder, &path));
                continue;
            }
            if found.is_dir() {
                self.data(top, &within)?;
                continue;
            }
            let (stem, ending) = split_name(&name);
            match ending.as_str() {
                "yml" | "yaml" | "json" => self.import.files.push(File {
                    from: path,
                    to: format!("data/{within}"),
                }),
                "toml" => {
                    let text = std::fs::read_to_string(self.path(&path)).unwrap_or_default();
                    match super::toml::read(&text) {
                        Ok(fields) => {
                            let yaml = super::yaml::fields(
                                fields.iter().map(|f| (f.name.as_str(), &f.value)),
                            );
                            let to = format!(
                                "data/{}",
                                joined(parent(&within), &format!("{stem}.yaml"))
                            );
                            self.note(format!(
                                "{} was written in TOML, which Statac doesn't read in `data/`, so \
                                 it is written as YAML, holding the same values, in {}.",
                                code(&path),
                                code(&to)
                            ));
                            self.import.written.push((to, yaml));
                        }
                        Err(stopped) => {
                            let line = text.lines().nth(stopped.line - 1).unwrap_or_default();
                            self.find(&DATA_LEFT, &path, Some(stopped.line), line);
                        }
                    }
                }
                _ => self.find(&DATA_LEFT, &path, None, ""),
            }
        }
        Ok(())
    }

    /// The language a file's name gives it, and its name without it.
    fn language_of<'n>(&self, stem: &'n str) -> (Option<String>, &'n str) {
        match stem.rsplit_once('.') {
            Some((rest, code_))
                if self
                    .languages
                    .iter()
                    .chain(std::iter::once(&self.main))
                    .any(|l| l.eq_ignore_ascii_case(code_)) =>
            {
                (Some(code_.to_lowercase()), rest)
            }
            _ => (None, stem),
        }
    }

    /// Reads the folder `rel` of the writing in `root`.
    fn content(
        &mut self,
        root: &str,
        language: Option<&str>,
        rel: &str,
    ) -> Result<(), Box<Failed>> {
        let names = names_in(&self.path(&joined(root, rel)))?;
        let is_writing = |name: &str| {
            let (_, ending) = split_name(name);
            MARKDOWN.contains(&ending.as_str()) || HTML.contains(&ending.as_str())
        };
        let index_named = |name: &str, index: &str| {
            let (stem, _) = split_name(name);
            is_writing(name) && (stem == index || stem.split('.').next() == Some(index))
        };
        let bundle = !rel.is_empty() && names.iter().any(|n| index_named(n, "index"));
        for name in &names {
            let within = joined(rel, name);
            let path = joined(root, &within);
            let Ok(found) = inside::metadata(self.folder, &path) else {
                continue;
            };
            if found.is_symlink() {
                self.left(&path, inside::why(self.folder, &path));
                continue;
            }
            let (_, ending) = split_name(name);
            if bundle {
                if index_named(name, "index") {
                    self.page(root, language, &within, Shape::Bundle)?;
                } else if is_writing(name) {
                    self.left(
                        &path,
                        "writing kept among a page's files, which Hugo doesn't publish as a page",
                    );
                } else {
                    self.beside(root, language, rel, name, true)?;
                }
                continue;
            }
            if found.is_dir() {
                self.content(root, language, &within)?;
            } else if index_named(name, "_index") {
                self.page(root, language, &within, Shape::List)?;
            } else if is_writing(name) {
                self.page(root, language, &within, Shape::Single)?;
            } else if OTHER_FORMATS.contains(&ending.as_str()) {
                self.find(&FORMAT_LEFT, &path, None, "");
            } else {
                self.beside(root, language, rel, name, false)?;
            }
        }
        Ok(())
    }

    /// Copies a file kept beside writing, or a folder of them, to its
    /// place beside that writing in the new site: for a page kept as a
    /// folder, beside each language's page.
    fn beside(
        &mut self,
        root: &str,
        language: Option<&str>,
        rel: &str,
        name: &str,
        bundle: bool,
    ) -> Result<(), Box<Failed>> {
        let from = joined(root, &joined(rel, name));
        let languages: Vec<String> = match (language, bundle && self.several()) {
            (Some(language), _) => vec![language.to_owned()],
            (None, true) => self.languages.clone(),
            (None, false) => vec![self.main.clone()],
        };
        let files: Vec<(String, String)> = match self.path(&from).is_dir() {
            true => self
                .files_under(&from, "")?
                .into_iter()
                .map(|within| (joined(&from, &within), joined(name, &within)))
                .collect(),
            false => vec![(from, name.to_owned())],
        };
        let folder = self.new_folder(rel);
        for language in languages {
            let prefix = self.prefix(&language);
            for (from, within) in &files {
                self.import.files.push(File {
                    from: from.clone(),
                    to: format!("content/{prefix}{}", joined(&folder, within)),
                });
            }
        }
        Ok(())
    }

    /// The folder `rel` of the writing as the new site names it: a page
    /// kept as a folder whose name opens with the date Hugo read from it,
    /// without that date.
    fn new_folder(&self, rel: &str) -> String {
        let reads_names = self.s.date_sources("date").iter().any(|s| s == ":filename");
        rel.split('/')
            .filter(|piece| !piece.is_empty())
            .map(|piece| match (reads_names, dated_name(piece)) {
                (true, Some((_, rest))) => rest,
                _ => piece,
            })
            .collect::<Vec<_>>()
            .join("/")
    }

    /// Reads one page.
    fn page(
        &mut self,
        root: &str,
        root_language: Option<&str>,
        within: &str,
        shape: Shape,
    ) -> Result<(), Box<Failed>> {
        let from = joined(root, within);
        let bytes = std::fs::read(self.path(&from)).map_err(|error| {
            Box::<Failed>::from(Problem::nowhere(Kind::CantRead {
                path: self.path(&from),
                reason: system_reason(&error),
            }))
        })?;
        let Ok(text) = String::from_utf8(bytes) else {
            self.left(&from, "a file that isn't UTF-8 text");
            return Ok(());
        };
        let file = last(within);
        let (stem, ending) = split_name(file);
        let (named_language, stem) = self.language_of(stem);
        let language = named_language
            .or(root_language.map(str::to_owned))
            .unwrap_or_else(|| self.main.clone());
        let html = HTML.contains(&ending.as_str());
        let front = match front(&text, &from, true) {
            Ok(front) => front,
            Err((line, text)) => {
                self.find(&FRONT_MATTER_UNREAD, &from, Some(line), &text);
                return Ok(());
            }
        };
        let (dir, name) = match shape {
            Shape::Single => (parent(within).to_owned(), stem.to_owned()),
            Shape::Bundle => {
                let folder = parent(within);
                (parent(folder).to_owned(), last(folder).to_owned())
            }
            Shape::List | Shape::Made => {
                let folder = parent(within);
                (folder.to_owned(), last(folder).to_owned())
            }
        };
        let plurals: Vec<String> = self.s.plurals().iter().map(|p| (*p).to_owned()).collect();
        let mut fields: Vec<Field> = Vec::new();
        let mut spelled: Vec<(String, String)> = Vec::new();
        for mut field in front.fields {
            let lower = field.name.to_lowercase();
            spelled.push((lower.clone(), field.name.clone()));
            if HUGOS_WORDS.contains(&lower.as_str()) || plurals.contains(&lower) {
                field.name = lower;
            }
            if field.name == "params"
                && let Value::Fields(raised) = &field.value
            {
                let mut kept = Vec::new();
                for (name, value) in raised {
                    // Hugo reads none of its own words beneath `params` but
                    // the lists of terms, so `url` there is the author's.
                    let lower = name.to_lowercase();
                    let hugos = (HUGOS_WORDS.contains(&lower.as_str())
                        || DATE_NAMES.contains(&lower.as_str()))
                        && !plurals.contains(&lower);
                    match hugos || fields.iter().any(|f| f.name == *name) {
                        true => kept.push((name.clone(), value.clone())),
                        false => fields.push(Field::new(name, value.clone(), field.line)),
                    }
                }
                if !kept.is_empty() {
                    fields.push(Field::new("params", Value::Fields(kept), field.line));
                }
                continue;
            }
            fields.push(field);
        }
        let params: Vec<(String, Value)> = fields
            .iter()
            .map(|f| (f.name.to_lowercase(), f.value.clone()))
            .collect();
        let get = |name: &str| fields.iter().find(|f| f.name == name);
        let as_written = |field: &Field| {
            let name = spelled
                .iter()
                .find(|(lower, _)| *lower == field.name)
                .map_or(field.name.as_str(), |(_, written)| written.as_str());
            format!("{name}: {}", super::write::shown_value(&field.value))
        };
        let reads_names = self.s.date_sources("date").iter().any(|s| s == ":filename");
        let name_date = dated_name(&name).filter(|_| reads_names && shape != Shape::List);
        let date_got = self.date_from("date", &fields, name_date.map(|(d, _)| d));
        let mut date_field: Option<Field> = None;
        let mut date = None;
        match &date_got {
            Got::Field(field) => {
                date = text_of(&field.value).and_then(|t| date::read(&t).ok());
                date_field = Some(Field::new("date", field.value.clone(), field.line));
            }
            Got::Name(day) => {
                date = Some(*day);
                date_field = Some(Field::new("date", Value::Text(day.iso_day()), None));
            }
            Got::Elsewhere(source) => {
                self.find(&DATE_ELSEWHERE, &from, None, source);
            }
            Got::Nothing => {}
        }
        let updated = match self.date_from("lastmod", &fields, None) {
            Got::Field(field) if matches!(field.name.as_str(), "lastmod" | "modified") => {
                Some(Field::new("updated", field.value.clone(), field.line))
            }
            Got::Elsewhere(_) => {
                self.updated_elsewhere += 1;
                None
            }
            _ => None,
        };
        if let Got::Field(published) = self.date_from("publishdate", &fields, None)
            && published.name != "date"
            && date_field.as_ref().map(|f| &f.value) != Some(&published.value)
            && !matches!(&date_got, Got::Field(f) if f.name == published.name)
        {
            self.find(
                &PUBLISH_DATE,
                &from,
                published.line,
                &as_written(&published),
            );
        }
        let mut draft = get("draft").is_some_and(|f| truth_of(&f.value));
        if let Got::Field(expiry) = self.date_from("expirydate", &fields, None) {
            self.find(&EXPIRY_DATE, &from, expiry.line, &as_written(&expiry));
            let past = text_of(&expiry.value)
                .and_then(|t| date::read(&t).ok())
                .is_some_and(|d| {
                    (d.year, d.month, d.day) < (self.today.year, self.today.month, self.today.day)
                });
            draft |= past;
        }
        let headless = get("headless").is_some_and(|f| truth_of(&f.value));
        let build = get("build")
            .map(|f| super::common::fields(&f.value).to_vec())
            .unwrap_or_default();
        let kept_out = get("build").is_some_and(|_| {
            get_text(&build, "render").is_some_and(|r| matches!(r.as_str(), "never" | "false"))
        });
        if headless || kept_out {
            let field = get(if headless { "headless" } else { "build" }).cloned();
            if let Some(field) = field {
                self.find(&UNPUBLISHED, &from, field.line, &as_written(&field));
            }
            draft = true;
        }
        let slug = get("slug")
            .and_then(|f| text_of(&f.value))
            .map(|s| s.trim_matches('-').to_owned())
            .filter(|s| !s.is_empty())
            .or_else(|| match &date_got {
                Got::Name(_) => name_date.map(|(_, rest)| rest.to_owned()),
                _ => None,
            });
        let url = get("url")
            .and_then(|f| text_of(&f.value))
            .filter(|u| !u.is_empty());
        let title = get("title")
            .and_then(|f| text_of(&f.value))
            .unwrap_or_default();
        let aliases = get("aliases")
            .map(|f| texts_of(&f.value))
            .unwrap_or_default();
        let weight = get("weight")
            .and_then(|f| text_of(&f.value))
            .and_then(|w| w.parse().ok())
            .unwrap_or(0);
        let summary_given = get("summary").is_some() || front.body.contains("<!--more-->");
        let mut terms = Vec::new();
        for plural in &plurals {
            if let Some(field) = get(plural) {
                terms.push((plural.clone(), texts_of(&field.value)));
            }
        }

        let mut out: Vec<Field> = Vec::new();
        let mut words: Vec<Field> = Vec::new();
        let has_summary = fields.iter().any(|f| f.name == "summary");
        let mut taken_off = false;
        let mut date_written = false;
        for field in fields {
            let name = field.name.as_str();
            match name {
                _ if DATE_NAMES.contains(&name) => {
                    if !date_written && let Some(written) = date_field.take() {
                        out.push(written);
                        date_written = true;
                    }
                    if matches!(name, "lastmod" | "modified")
                        && let Some(updated) = updated.as_ref().filter(|u| u.line == field.line)
                    {
                        out.push(updated.clone());
                    }
                }
                "layout" | "type" => taken_off = true,
                "slug" | "url" | "aliases" | "headless" => {}
                "build" if kept_out => {}
                "draft" => {
                    if !draft {
                        out.push(field);
                    }
                }
                "description" if !has_summary => {
                    out.push(Field::new("summary", field.value, field.line))
                }
                "translationkey" if self.several() => {
                    words.push(Field::new("translation_key", field.value, field.line));
                }
                // Every list of terms but the tags, and a series of more
                // than one, the shared part makes tags of.
                _ if plurals.iter().any(|p| p == name)
                    && name != "tags"
                    && !(name == "series"
                        && terms.iter().any(|(p, names)| p == name && names.len() < 2)) => {}
                _ if plurals.iter().any(|p| p == name) => {
                    let value = match (name, field.value) {
                        ("series", Value::List(mut one)) if one.len() == 1 => one.remove(0),
                        ("series", text @ Value::Text(_)) => text,
                        (_, Value::Text(text)) => Value::List(vec![Value::Text(text)]),
                        (_, other) => other,
                    };
                    out.push(Field::new(name, value, field.line));
                }
                _ => out.push(field),
            }
        }
        if let Some(written) = date_field.take() {
            let at = usize::from(out.first().is_some_and(|f| f.name == "title"));
            out.insert(at, written);
        }
        if draft {
            out.push(Field::new("draft", Value::Truth(true), None));
        }
        if taken_off {
            self.taken_off += 1;
        }
        let prefix = self.prefix(&language);
        let folder = self.new_folder(&dir);
        let ending = if html && shape == Shape::Single {
            "html"
        } else {
            "md"
        };
        let to = match shape {
            Shape::Single => {
                let name = match (&date_got, name_date) {
                    (Got::Name(_), Some((_, rest))) => rest,
                    _ => name.as_str(),
                };
                format!("{prefix}{}", joined(&folder, &format!("{name}.{ending}")))
            }
            Shape::Bundle => {
                let bundle = self.new_folder(&joined(&dir, &name));
                format!("{prefix}{bundle}/index.md")
            }
            Shape::List | Shape::Made => format!("{prefix}{}", joined(&folder, "index.md")),
        };
        let kind = match (draft, date.is_some(), shape) {
            (true, _, _) => PageKind::Draft,
            (false, _, Shape::List | Shape::Made) => PageKind::Page,
            (false, true, _) => PageKind::Post,
            (false, false, _) => PageKind::Page,
        };
        self.docs.push(Doc {
            root: root.to_owned(),
            from,
            to,
            language,
            shape,
            kind,
            fields: out,
            words,
            body: front.body,
            body_line: front.body_line,
            html,
            dir,
            name,
            slug,
            url,
            date,
            title,
            aliases,
            weight,
            params,
            summary_cut: !summary_given && date.is_some(),
            terms,
            draft,
            address: None,
            own_address: false,
            others: Vec::new(),
        });
        Ok(())
    }

    /// Where the date `which` of a page comes from, by the settings.
    fn date_from(&self, which: &str, fields: &[Field], name_date: Option<Date>) -> Got {
        for source in self.s.date_sources(which) {
            match source.as_str() {
                ":filename" => {
                    if let Some(day) = name_date {
                        return Got::Name(day);
                    }
                }
                ":filemodtime" => return Got::Elsewhere(source),
                ":git" if self.s.git => return Got::Elsewhere(source),
                ":git" => {}
                name => {
                    if let Some(field) = fields.iter().find(|f| f.name == name)
                        && text_of(&field.value).is_some_and(|t| !t.is_empty())
                    {
                        return Got::Field(field.clone());
                    }
                }
            }
        }
        Got::Nothing
    }

    /// The folders that are lists to Hugo, by language: the top folders,
    /// and those within them with an `_index.md`.
    fn lists(&self) -> HashSet<(String, String)> {
        self.docs
            .iter()
            .filter(|d| matches!(d.shape, Shape::List | Shape::Made))
            .map(|d| (d.language.clone(), d.dir.clone()))
            .collect()
    }

    /// The folders a page in `dir` is in that are lists, outermost first.
    fn sections(
        &self,
        language: &str,
        dir: &str,
        lists: &HashSet<(String, String)>,
    ) -> Vec<String> {
        let mut sections = Vec::new();
        let mut path = String::new();
        for (at, piece) in dir.split('/').filter(|p| !p.is_empty()).enumerate() {
            path = joined(&path, piece);
            if at == 0 || lists.contains(&(language.to_owned(), path.clone())) {
                sections.push(piece.to_owned());
            } else {
                break;
            }
        }
        sections
    }

    /// Works out where Hugo published each page.
    fn addresses(&mut self) {
        let lists = self.lists();
        let mut unknown = Vec::new();
        for at in 0..self.docs.len() {
            let doc = &self.docs[at];
            let prefix = format!("/{}", self.prefix(&doc.language));
            let prefix = prefix.trim_end_matches('/');
            let sections = self.sections(&doc.language, &doc.dir, &lists);
            let section = sections.first().cloned().unwrap_or_default();
            let ugly = self.s.ugly.of(&section);
            let pieces = Pieces {
                date: doc.date.as_ref(),
                title: &doc.title,
                slug: doc.slug.as_deref(),
                file_name: &doc.name,
                sections: &sections,
            };
            let own_url = doc.url.as_ref().map(|url| {
                let whole = match url.starts_with('/') {
                    true => url.clone(),
                    false => format!("{prefix}/{url}"),
                };
                let file = last(whole.trim_end_matches('/')).contains('.') && !whole.ends_with('/');
                match file {
                    true => whole,
                    false => format!("{}/", whole.trim_end_matches('/')),
                }
            });
            let mut older = None;
            let made: Result<String, String> = match doc.shape {
                Shape::Single | Shape::Bundle => match self.s.permalinks.page(&section) {
                    Some(pattern) => url::expand(pattern, &pieces, &self.making)
                        .map(|path| url::finished(&path, ugly)),
                    None => {
                        let base = doc.slug.clone().unwrap_or_else(|| doc.name.clone());
                        let path = self.making.path(&joined(&doc.dir, &base));
                        Ok(url::finished(&path, ugly))
                    }
                },
                // Hugo before 0.148 published a folder's list under
                // `uglyURLs` as a file, `/posts.html`; since, as a folder.
                Shape::List | Shape::Made => {
                    let path = match self.s.permalinks.section(&section) {
                        Some(pattern) if !doc.dir.is_empty() => {
                            url::expand(pattern, &pieces, &self.making)
                        }
                        _ => Ok(self.making.path(&doc.dir)),
                    };
                    path.map(|path| {
                        if ugly && !doc.dir.is_empty() && own_url.is_none() && !doc.draft {
                            let file = format!("{prefix}{}", url::finished(&path, true));
                            // The old built output, where it is there,
                            // tells which Hugo built the site.
                            let built = (self.import.built.as_deref()).map(|built| {
                                inside::a_file(self.folder, &format!("{built}{file}"))
                            });
                            if built != Some(false) {
                                older = Some(file);
                            }
                        }
                        url::finished(&path, false)
                    })
                }
            };
            let hugos = match (&own_url, made) {
                (Some(url), _) => Some(url.clone()),
                (None, Ok(path)) => Some(match path.as_str() {
                    "/" => format!("{prefix}/"),
                    _ => format!("{prefix}{path}"),
                }),
                (None, Err(piece)) => {
                    unknown.push((doc.from.clone(), piece));
                    None
                }
            };
            let base = hugos.clone().unwrap_or_default();
            let mut others: Vec<String> = older.into_iter().collect();
            for alias in &doc.aliases {
                let alias = match alias.starts_with('/') {
                    true => alias.clone(),
                    false => format!("{}/{alias}", parent(base.trim_end_matches('/'))),
                };
                let alias = match (alias.ends_with(".html"), ugly) {
                    (true, _) => alias,
                    (false, true) => format!("{}.html", alias.trim_end_matches('/')),
                    (false, false) => format!("{}/", alias.trim_end_matches('/')),
                };
                others.push(alias);
            }
            let own_address = own_url.is_some();
            let draft = doc.draft;
            let doc = &mut self.docs[at];
            doc.address = hugos.filter(|_| !draft);
            doc.own_address = own_address;
            doc.others = others;
        }
        for (from, piece) in unknown {
            self.find(&ADDRESS_UNKNOWN, &from, None, &piece);
        }
    }

    /// Writes a folder's own page for each folder at the top of each
    /// language's writing that Hugo made a list of without one.
    fn made_lists(&mut self) {
        let mut wanted: Vec<(String, String, String)> = Vec::new();
        for doc in &self.docs {
            if !matches!(doc.shape, Shape::Single | Shape::Bundle) {
                continue;
            }
            let top = doc.dir.split('/').next().unwrap_or_default();
            if top.is_empty() {
                continue;
            }
            let has = self
                .docs
                .iter()
                .any(|d| d.shape == Shape::List && d.language == doc.language && d.dir == top);
            if !has
                && !wanted
                    .iter()
                    .any(|(l, t, _)| *l == doc.language && t == top)
            {
                let from = format!("{}/", joined(&doc.root, top));
                wanted.push((doc.language.clone(), top.to_owned(), from));
            }
        }
        for (language, top, from) in wanted {
            let prefix = self.prefix(&language);
            let title = crate::new::as_words(&top);
            self.docs.push(Doc {
                root: String::new(),
                from,
                to: format!("{prefix}{}/index.md", self.new_folder(&top)),
                language,
                shape: Shape::Made,
                kind: PageKind::Page,
                fields: vec![Field::new("title", Value::Text(title.clone()), None)],
                words: Vec::new(),
                body: String::new(),
                body_line: 1,
                html: false,
                dir: top.clone(),
                name: top,
                slug: None,
                url: None,
                date: None,
                title,
                aliases: Vec::new(),
                weight: 0,
                params: Vec::new(),
                summary_cut: false,
                terms: Vec::new(),
                draft: false,
                address: None,
                own_address: false,
                others: Vec::new(),
            });
        }
    }

    /// Gives each folder whose pages Hugo ordered by `weight` that order
    /// Statac's way, as `order:` on the folder's own page.
    fn weights(&mut self) {
        let mut ordered = Vec::new();
        for (at, list) in self.docs.iter().enumerate() {
            if !matches!(list.shape, Shape::List | Shape::Made) || list.dir.is_empty() {
                continue;
            }
            let folder = parent(&list.to).to_owned();
            let mut members: Vec<&Doc> = self
                .docs
                .iter()
                .filter(|d| {
                    let own = match d.shape {
                        Shape::Single => parent(&d.to),
                        Shape::Bundle => parent(parent(&d.to)),
                        _ => return false,
                    };
                    own == folder && d.address.is_some()
                })
                .collect();
            if !members.iter().any(|d| d.weight != 0) {
                continue;
            }
            members.sort_by(|a, b| {
                (a.weight == 0)
                    .cmp(&(b.weight == 0))
                    .then(a.weight.cmp(&b.weight))
                    .then_with(|| {
                        let day = |d: &Doc| {
                            d.date.map(|d| {
                                (
                                    d.year,
                                    d.month,
                                    d.day,
                                    d.time.map(|t| (t.hour, t.minute, t.second)),
                                )
                            })
                        };
                        day(b).cmp(&day(a))
                    })
                    .then_with(|| a.title.cmp(&b.title))
                    .then_with(|| a.from.cmp(&b.from))
            });
            let names: Vec<Value> = members
                .iter()
                .filter_map(|d| {
                    let address = d.address.as_deref()?;
                    Some(Value::Text(last(address.trim_end_matches('/')).to_owned()))
                })
                .collect();
            let froms: Vec<String> = members.iter().map(|d| d.from.clone()).collect();
            ordered.push((at, folder, names, froms));
        }
        for (at, folder, names, froms) in ordered {
            self.docs[at]
                .words
                .push(Field::new("order", Value::List(names), None));
            for doc in &mut self.docs {
                if froms.contains(&doc.from) {
                    doc.fields.retain(|f| f.name != "weight");
                }
            }
            self.note(format!(
                "Hugo ordered the pages of {} by their `weight`, so {} lists them in that order \
                 with `order:`, and `weight` was taken off them.",
                code(&format!("content/{folder}/")),
                code(&format!("content/{folder}/index.md"))
            ));
        }
    }

    /// Converts each page's writing, once every page's address is known,
    /// and hands the pages, the snippets standing for tags, and the
    /// settings to the import.
    fn finish(&mut self) {
        self.made_lists();
        self.addresses();
        self.weights();
        let firsts = self.later_pages();
        self.lists_and_feeds();
        self.terms();
        let wrapping: HashSet<String> = self
            .docs
            .iter()
            .flat_map(|d| writing::paired(&d.body))
            .collect();
        let mut calls: Vec<writing::Call> = Vec::new();
        let mut titles = 0;
        let mut figures = 0;
        let mut plainer = 0;
        let mut valued = 0;
        let site_params: Vec<(String, Value)> = (self.s.params.iter())
            .map(|f| (f.name.to_lowercase(), f.value.clone()))
            .collect();
        let site_param = |name: &str| value_at(&site_params, name);
        let values: Vec<(String, writing::SiteValue)> = self
            .own_files
            .iter()
            .filter_map(|(tag, file)| {
                let value = writing::site_value(file, self.s.title.as_deref(), &site_param)?;
                Some((tag.clone(), value))
            })
            .collect();
        let mut cut = 0;
        let mut emoji: Vec<(String, usize)> = Vec::new();
        let mut written = Vec::new();
        let posts = self.docs.iter().filter(|d| {
            d.date.is_some() && !d.draft && matches!(d.shape, Shape::Single | Shape::Bundle)
        });
        let posts = posts.count() as u64;
        let docs = std::mem::take(&mut self.docs);
        let whole = self.import.address.clone();
        let base = whole
            .as_deref()
            .map(|w| {
                let past = w.split_once("//").map_or(w, |(_, rest)| rest);
                past.split_once('/').map_or(String::new(), |(_, path)| {
                    format!("/{}", path.trim_matches('/'))
                })
            })
            .unwrap_or_default();
        let base = if base == "/" { String::new() } else { base };
        for doc in &docs {
            let refs = |path: &str, language: Option<&str>| refer(&docs, doc, path, language);
            let param = |name: &str| {
                let lower = name.to_lowercase();
                value_at(&doc.params, &lower).or_else(|| {
                    let site: Vec<(String, Value)> = self
                        .s
                        .params
                        .iter()
                        .map(|f| (f.name.to_lowercase(), f.value.clone()))
                        .collect();
                    value_at(&site, &lower)
                })
            };
            let picture = |src: &str| self.picture(doc, src);
            let cx = writing::Context {
                base: &base,
                whole: whole.as_deref(),
                refs: &refs,
                param: &param,
                picture: &picture,
                own: &self.own,
                values: &values,
                wrapping: &wrapping,
                html: doc.html,
                unsafe_html: self.s.unsafe_html,
                emoji: self.s.emoji,
                block_attributes: self.s.block_attributes,
                summary_length: doc.summary_cut.then_some(self.s.summary_length),
                starter: self.starter,
            };
            let converted = writing::convert(&doc.body, doc.body_line, &doc.from, &cx);
            for finding in converted.findings {
                if std::ptr::eq(finding.kind, &SUMMARY_CUT) {
                    cut += 1;
                } else if std::ptr::eq(finding.kind, &EMOJI) {
                    for spot in finding.spots {
                        match emoji.iter_mut().find(|(name, _)| *name == spot.text) {
                            Some((_, times)) => *times += 1,
                            None => emoji.push((spot.text, 1)),
                        }
                    }
                } else {
                    self.import.findings.push(finding);
                }
            }
            calls.extend(converted.calls);
            titles += converted.titles;
            figures += converted.figures;
            plainer += converted.plainer;
            valued += converted.valued;
            written.push((converted.writing, converted.fences, converted.called));
        }
        for (doc, (writing, fences, called)) in docs.into_iter().zip(written) {
            if doc.shape == Shape::Made {
                let permalink = (self.made_permalink(&doc))
                    .map(|within| Field::new("permalink", Value::Text(within), None));
                let fields = doc.fields.iter().chain(&doc.words).chain(&permalink);
                let front = super::yaml::fields(fields.map(|f| (f.name.as_str(), &f.value)));
                self.import
                    .written
                    .push((format!("content/{}", doc.to), format!("---\n{front}---\n")));
                if !doc.others.is_empty() {
                    let from = doc.others.iter().cloned().map(Value::Text).collect();
                    let from = Field::new("redirect_from", Value::List(from), None);
                    self.import.words.push((doc.to.clone(), from));
                }
                self.import
                    .addresses
                    .extend(doc.address.into_iter().chain(doc.others));
                continue;
            }
            for word in &doc.words {
                self.import.words.push((doc.to.clone(), word.clone()));
            }
            self.import.pages.push(Page {
                from: doc.from,
                to: doc.to,
                kind: doc.kind,
                front: doc.fields,
                writing,
                terms: (doc.terms.into_iter())
                    .filter(|(plural, names)| {
                        plural != "tags" && !(plural == "series" && names.len() < 2)
                    })
                    .collect(),
                address: doc.address,
                own_address: doc.own_address,
                other_addresses: doc.others,
                fences,
                calls: called,
            });
        }
        self.snippets(&calls);
        for (name, times) in emoji {
            self.import.findings.push(Finding {
                kind: &EMOJI,
                about: None,
                spots: vec![super::Spot {
                    file: String::new(),
                    line: None,
                    text: name,
                    times: Some(times),
                }],
            });
        }
        if cut > 0 {
            self.note(format!(
                "Hugo summed up {} opening paragraphs, as many as made its `summaryLength` of \
                 words, and Statac takes the first paragraph alone: where a summary should run \
                 further, put `<!--more-->` on a line of its own where it should end.",
                match cut {
                    1 => "1 page with its".to_owned(),
                    n => format!("{n} pages with their"),
                }
            ));
        }
        if valued > 0 {
            let example = values
                .first()
                .map(|(tag, _)| tag.as_str())
                .unwrap_or_default();
            self.note(format!(
                "{} a tag of the site's own that showed one of the site's values and nothing \
                 else, such as {}: {} the value itself now, the site's address as the path from \
                 its top.",
                match valued {
                    1 => "1 place held".to_owned(),
                    n => format!("{n} places held"),
                },
                code(&format!("{{{{< {example} >}}}}")),
                match valued {
                    1 => "it holds",
                    _ => "each holds",
                }
            ));
        }
        if figures > 0 {
            self.note(format!(
                "{} to the starter's `figure` snippet.{}",
                match figures {
                    1 => "1 figure is now a call".to_owned(),
                    n => format!("{n} figures are now calls"),
                },
                match plainer {
                    0 => String::new(),
                    n => format!(
                        " `class`, `width` and `height` were taken off {n} of them, which the \
                         snippet doesn't take: each shows at the size the starter gives a picture."
                    ),
                }
            ));
        }
        if titles > 0 {
            self.note(format!(
                "{} a `title`, which the starter's `figure` snippet doesn't take, so it opens the \
                 caption, in bold.",
                match titles {
                    1 => "1 figure had".to_owned(),
                    n => format!("{n} figures had"),
                }
            ));
        }
        if self.taken_off > 0 {
            self.note(format!(
                "`layout` and `type` were taken off {}: each is built with the {}template for its \
                 kind, as https://statac.dev/templates/ describes.",
                match self.taken_off {
                    1 => "1 page".to_owned(),
                    n => format!("{n} pages"),
                },
                match self.starter {
                    true => "starter's ",
                    false => "",
                }
            ));
        }
        if self.updated_elsewhere > 0 {
            self.note(format!(
                "Hugo took when {} last changed from the file or from Git, which Statac doesn't \
                 read, so no `updated` was written from there.",
                match self.updated_elsewhere {
                    1 => "1 page".to_owned(),
                    n => format!("{n} pages"),
                }
            ));
        }
        self.paging(posts, firsts);
    }
}

/// What a date was found as.
enum Got {
    Field(Field),
    Name(Date),
    Elsewhere(String),
    Nothing,
}

/// A text field of a table, in any capitals.
fn get_text(fields: &[(String, Value)], name: &str) -> Option<String> {
    get(fields, name).and_then(text_of)
}

/// The scalar at a dotted path among `fields`, as text.
fn value_at(fields: &[(String, Value)], path: &str) -> Option<String> {
    let (first, rest) = match path.split_once('.') {
        Some((first, rest)) => (first, Some(rest)),
        None => (path, None),
    };
    let value = get(fields, first)?;
    match rest {
        Some(rest) => value_at(super::common::fields(value), rest),
        None => text_of(value),
    }
}

/// The address, from the top of the site, of the page `path` names from
/// `doc`, as `ref` finds it: beside the page, then from the top of the
/// writing, then by its name alone where one page has it.
fn refer(docs: &[Doc], doc: &Doc, path: &str, language: Option<&str>) -> Option<String> {
    let (path, anchor) = match path.split_once('#') {
        Some((path, anchor)) => (path, format!("#{anchor}")),
        None => (path, String::new()),
    };
    if path.is_empty() {
        return Some(anchor);
    }
    let language = language.map_or(doc.language.clone(), str::to_lowercase);
    let keys = |d: &Doc| -> Vec<String> {
        let stemmed = match d.shape {
            Shape::Single => joined(&d.dir, &d.name),
            Shape::Bundle => joined(&d.dir, &d.name),
            Shape::List | Shape::Made => d.dir.clone(),
        };
        let file = last(&d.from).to_owned();
        let folder_of = match d.shape {
            Shape::Single => d.dir.clone(),
            _ => stemmed.clone(),
        };
        vec![stemmed.clone(), joined(&folder_of, &file)]
    };
    let tidy = |p: &str| {
        let mut parts: Vec<&str> = Vec::new();
        for piece in p.split('/') {
            match piece {
                "" | "." => {}
                ".." => {
                    parts.pop();
                }
                piece => parts.push(piece),
            }
        }
        parts.join("/")
    };
    let wanted = |p: &str| {
        let p = tidy(p);
        let bare = match p.rsplit_once('.') {
            Some((stem, ending)) if !ending.contains('/') => stem.to_owned(),
            _ => p.clone(),
        };
        let bare = bare
            .strip_suffix("/_index")
            .or(bare.strip_suffix("/index"))
            .map(str::to_owned)
            .unwrap_or(match bare.as_str() {
                "_index" | "index" => String::new(),
                _ => bare.clone(),
            });
        (p, bare)
    };
    let here = match doc.shape {
        Shape::Single => doc.dir.clone(),
        _ => joined(&doc.dir, &doc.name),
    };
    let tries: Vec<String> = match path.starts_with('/') {
        true => vec![path.to_owned()],
        false => vec![joined(&here, path), path.to_owned()],
    };
    let in_language = |d: &&Doc| d.language == language && d.address.is_some();
    for attempt in tries {
        let (whole, bare) = wanted(&attempt);
        if let Some(found) = docs
            .iter()
            .filter(in_language)
            .find(|d| keys(d).iter().any(|k| *k == whole || *k == bare))
        {
            return Some(format!("{}{anchor}", found.address.as_deref()?));
        }
    }
    if !path.contains('/') {
        let (_, bare) = wanted(path);
        let named: Vec<&Doc> = docs
            .iter()
            .filter(in_language)
            .filter(|d| d.name == bare && matches!(d.shape, Shape::Single | Shape::Bundle))
            .collect();
        if let [one] = named.as_slice() {
            return Some(format!("{}{anchor}", one.address.as_deref()?));
        }
    }
    None
}

impl Reader<'_> {
    /// Whether the picture `src` a figure on `doc` names is one of the
    /// site's that Statac finds from the page, and a photograph.
    fn picture(&self, doc: &Doc, src: &str) -> bool {
        if src.contains("://") || src.starts_with("//") {
            return false;
        }
        match src.strip_prefix('/') {
            Some(rest) => self
                .s
                .static_dirs
                .iter()
                .chain(std::iter::once(&self.s.content_dir))
                .map(|dir| joined(dir, rest))
                .find(|file| self.a_file_named(file))
                .is_some_and(|file| self.a_photograph(&file)),
            None => {
                let file = joined(parent(&doc.from), src);
                doc.shape == Shape::Bundle && self.a_file_named(&file) && self.a_photograph(&file)
            }
        }
    }

    /// Whether the file at `rel` in the old site is a photograph, by what
    /// it holds: a JPEG, a PNG or a WebP that doesn't move. One that
    /// begins as a picture and doesn't read is told by its name's ending,
    /// so the new site's build says what is wrong with it.
    fn a_photograph(&self, rel: &str) -> bool {
        use crate::pictures::{Header, Unreadable, read_header};
        let Ok(bytes) = std::fs::read(inside::path(self.folder, rel)) else {
            return false;
        };
        match read_header(&bytes) {
            Ok(Header::Photograph(_)) => true,
            Ok(Header::Plain(_)) | Err(Unreadable::NotAPicture) => false,
            Err(Unreadable::CutShort | Unreadable::Unsupported(_)) => {
                let ending = rel.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase());
                matches!(ending.as_deref(), Some("jpg" | "jpeg" | "png" | "webp"))
            }
        }
    }

    /// The feed and layout each list's own page keeps.
    fn lists_and_feeds(&mut self) {
        let (home, lists, tags) = match self.s.feeds {
            true => self.s.feeds_for,
            false => (false, false, false),
        };
        let name = self.s.feed_name.clone();
        let off = || Field::new("feed", Value::Truth(false), None);
        let mut laid_out = Vec::new();
        let mut homes: Vec<String> = Vec::new();
        let mut feed_addresses = Vec::new();
        let mut whole: Vec<(String, String, String)> = Vec::new();
        let mut unfed = false;
        for at in 0..self.docs.len() {
            let doc = &self.docs[at];
            if !matches!(doc.shape, Shape::List | Shape::Made) || doc.draft {
                continue;
            }
            let folder = parent(&doc.to).to_owned();
            let top = !doc.dir.is_empty() && !doc.dir.contains('/');
            let dated = self.docs.iter().any(|d| {
                d.date.is_some()
                    && !d.draft
                    && matches!(d.shape, Shape::Single | Shape::Bundle)
                    && d.language == doc.language
                    && (doc.dir.is_empty()
                        || d.dir == doc.dir
                        || d.dir.starts_with(&format!("{}/", doc.dir)))
            });
            let undated = self.docs.iter().any(|d| {
                d.date.is_none()
                    && matches!(d.shape, Shape::Single | Shape::Bundle)
                    && d.language == doc.language
                    && (d.dir == doc.dir || d.dir.starts_with(&format!("{}/", doc.dir)))
            });
            let mut words = Vec::new();
            if self.starter && top && dated && !undated {
                words.push(Field::new("pages.layout", Value::Text("post".into()), None));
                laid_out.push(folder.clone());
            }
            if doc.dir.is_empty() {
                homes.push(doc.language.clone());
            }
            let fed = match doc.dir.is_empty() {
                true => home,
                false => lists,
            };
            if fed && dated {
                let prefix = self.prefix(&doc.language);
                let page = (doc.address.clone())
                    .unwrap_or_else(|| format!("/{prefix}{}", slug::tidied_path(&doc.dir)));
                // Hugo's feeds take no ugly address: a list at `/posts.html`
                // has its feed at `/posts/index.xml`, not beside its page.
                let (feed, word) = match page.ends_with('/') {
                    true => (format!("{page}{name}"), name.clone()),
                    false => {
                        let folder = page.rsplit_once('.').map_or(page.as_str(), |(f, _)| f);
                        let feed = format!("{folder}/{name}");
                        whole.push((page.clone(), doc.to.clone(), feed.clone()));
                        (feed.clone(), feed)
                    }
                };
                feed_addresses.push(feed);
                words.push(Field::new("feed", Value::Text(word), None));
            } else if dated && !doc.dir.contains('/') {
                words.push(off());
                unfed = true;
            }
            self.docs[at].words.extend(words);
        }
        let languages: Vec<String> = match self.several() {
            true => self.languages.clone(),
            false => vec![self.main.clone()],
        };
        for language in languages {
            let dated = self.docs.iter().any(|d| {
                d.date.is_some()
                    && !d.draft
                    && d.language == language
                    && matches!(d.shape, Shape::Single | Shape::Bundle)
            });
            if homes.contains(&language) || !dated {
                continue;
            }
            let prefix = self.prefix(&language);
            let word = match home {
                true => Field::new("feed", Value::Text(name.clone()), None),
                false => off(),
            };
            self.import.words.push((format!("{prefix}index.md"), word));
            match home {
                true => feed_addresses.push(format!("/{prefix}{name}")),
                false => unfed = true,
            }
        }
        let fed = !feed_addresses.is_empty();
        if fed {
            self.import.addresses.extend(feed_addresses);
            self.import
                .settings
                .push(("feed_ids_before".into(), Value::Text(self.today.iso_day())));
        }
        let on = match (home, lists) {
            (true, false) => "the home page",
            _ => "each list's page",
        };
        let (now, kept) = match self.starter {
            true => (
                "is now an Atom feed",
                format!(
                    "at the same address, written by `templates/feed.xml`, as `feed: {name}` on \
                     {on} says, and `feed_ids_before` keeps each post's id the address Hugo gave \
                     it, so feed readers see nothing new: see https://statac.dev/feeds/#entry-ids."
                ),
            ),
            false => (
                "stays",
                format!(
                    "at the same address, as `feed: {name}` on {on} says, built with \
                     `templates/feed.xml` once you add it, and `feed_ids_before` keeps each \
                     post's id the address Hugo gave it, so feed readers see nothing new: see \
                     https://statac.dev/feeds/#entry-ids."
                ),
            ),
        };
        let mut said = match (fed, home, lists) {
            (false, ..) => String::new(),
            (true, true, true) => format!(
                "Hugo's feeds were RSS, at `{name}` beside the home page and each list's own \
                 page. Each {now} {kept}"
            ),
            (true, true, false) => {
                format!("Hugo's feed was RSS, at `{name}` beside the home page. It {now} {kept}")
            }
            (true, false, _) => format!(
                "Hugo's feeds were RSS, at `{name}` beside each list's own page. Each {now} {kept}"
            ),
        };
        match whole.as_slice() {
            [] => {}
            [(page, to, feed)] => {
                said += &format!(
                    " A list published as its own file, {}, had its feed in the folder of the \
                     same name, so {} gives the feed's whole address: {}.",
                    code(page),
                    code(&format!("content/{to}")),
                    code(&format!("feed: {feed}"))
                );
            }
            [(page, to, feed), ..] => {
                said += &format!(
                    " The lists published as their own files, such as {}, had their feeds in \
                     the folders of the same names, so the page of each gives its feed's whole \
                     address, as {} does: {}.",
                    code(page),
                    code(&format!("content/{to}")),
                    code(&format!("feed: {feed}"))
                );
            }
        }
        if unfed {
            said += match fed {
                true => {
                    " Hugo made no feed for the other lists, so the page of each says \
                     `feed: false`."
                }
                false => {
                    "Hugo made no feeds, so the page of each list says `feed: false`: see \
                     https://statac.dev/feeds/."
                }
            };
        }
        if fed && tags && self.s.terms {
            said += " The feed Hugo made for each tag has no equal.";
        }
        if !said.is_empty() {
            self.note(said);
        }
        if !laid_out.is_empty() {
            let folders: Vec<String> = laid_out
                .iter()
                .map(|f| code(&format!("content/{f}/index.md")))
                .collect();
            self.note(format!(
                "{} {} `pages.layout: post`, so the pages beneath {}, every one dated, are built \
                 with the starter's template for posts.",
                folders.join(", "),
                if folders.len() == 1 {
                    "hands"
                } else {
                    "each hand"
                },
                if folders.len() == 1 { "it" } else { "them" }
            ));
        }
        let made: Vec<String> = self
            .docs
            .iter()
            .filter(|d| d.shape == Shape::Made)
            .map(|d| {
                format!(
                    "{} for {}",
                    code(&format!("content/{}", d.to)),
                    code(&d.from)
                )
            })
            .collect();
        let moved: Vec<(String, String)> = (self.docs.iter())
            .filter(|d| d.shape == Shape::Made)
            .filter_map(|d| Some((d.to.clone(), self.made_permalink(d)?)))
            .collect();
        if !made.is_empty() {
            let mut said = format!(
                "Hugo made a list of each folder at the top of the writing, and these had no \
                 `_index.md` of their own, so Statac wrote one to keep what the list was given: \
                 {}.",
                made.join(", ")
            );
            match moved.as_slice() {
                [] => {}
                [(to, permalink)] => {
                    said += &format!(
                        " {} keeps its list where Hugo published it, with {}.",
                        code(&format!("content/{to}")),
                        code(&format!("permalink: {permalink}"))
                    );
                }
                [(to, permalink), ..] => {
                    said += &format!(
                        " {} of them keep their lists where Hugo published them, each with a \
                         `permalink`, as {} does with {}.",
                        moved.len(),
                        code(&format!("content/{to}")),
                        code(&format!("permalink: {permalink}"))
                    );
                }
            }
            self.note(said);
        }
    }

    /// The `permalink` that keeps a list Hugo made at the address Hugo
    /// published it at, where that isn't its folder's. No page of it came
    /// across, so the shared part keeps no address of it.
    fn made_permalink(&self, doc: &Doc) -> Option<String> {
        let folder = format!("/{}", slug::tidied_path(parent(&doc.to)));
        let address = doc
            .address
            .as_deref()
            .filter(|address| *address != folder)?;
        let others = self.import.languages.get(1..).unwrap_or_default();
        super::addresses::from_its_language(&doc.to, address, others)
    }

    /// The addresses of the pages Hugo made for each term, and the
    /// settings that keep those of tags and of a series.
    fn terms(&mut self) {
        let mut addresses: Vec<String> = Vec::new();
        // How many pages each tag's page listed, by its address, where
        // Statac's tag pages follow Hugo's, and whether it is a file.
        let mut held: Vec<(String, usize, bool)> = Vec::new();
        for plural in ["tags", "series"] {
            let pattern = self.s.permalinks.term(plural).map(str::to_owned);
            let statacs = pattern.as_ref().map(|pattern| {
                let statacs = pattern
                    .replace(":slug", "{slug}")
                    .replace(":title", "{slug}");
                format!("/{}/", statacs.trim_matches('/'))
            });
            let kept = (statacs.as_ref())
                .is_none_or(|statacs| !statacs.contains(':') && statacs.ends_with("{slug}/"));
            let ugly = self.s.ugly.of(plural);
            // How many pages each of this list's pages listed, by its
            // address.
            let mut counts: Vec<(String, usize)> = Vec::new();
            let mut slugs: Vec<(String, Value)> = Vec::new();
            for doc in self.docs.iter().filter(|d| !d.draft) {
                let prefix = format!("/{}", self.prefix(&doc.language));
                let prefix = prefix.trim_end_matches('/');
                let Some((_, terms)) = doc.terms.iter().find(|(p, _)| p == plural) else {
                    continue;
                };
                if terms.is_empty() || !self.s.terms || (plural == "series" && terms.len() > 1) {
                    continue;
                }
                for term in terms {
                    let hugos = self.making.made(term);
                    let path = match &pattern {
                        Some(pattern) => pattern.replace(":slug", &hugos).replace(":title", &hugos),
                        None => format!("/{plural}/{hugos}/"),
                    };
                    let path = url::finished(&self.making.path(&path), ugly);
                    let address = format!("{prefix}{path}");
                    match counts.iter_mut().find(|(at, _)| *at == address) {
                        Some((_, pages)) => *pages += 1,
                        None => counts.push((address.clone(), 1)),
                    }
                    // A page that isn't where Statac's stands sends readers
                    // on.
                    let page = (plural.to_owned(), term.clone(), address.clone());
                    if (ugly || !kept) && !self.import.term_pages.contains(&page) {
                        self.import.term_pages.push(page);
                    }
                    if !addresses.contains(&address) {
                        addresses.push(address);
                    }
                    if hugos != slug::slugify(term)
                        && crate::content::address::name_fault(&hugos).is_none()
                        && !slugs.iter().any(|(t, _)| t == term)
                    {
                        slugs.push((term.clone(), Value::Text(hugos)));
                    }
                }
                let list = format!("{prefix}/{plural}/");
                if pattern.is_none() && !self.import.term_lists.iter().any(|(at, _)| *at == list) {
                    self.import.term_lists.push((list, plural.to_owned()));
                }
            }
            if !slugs.is_empty() {
                self.import
                    .settings
                    .push((format!("{plural}.slugs"), Value::Fields(slugs)));
            }
            if kept {
                held.extend(
                    counts
                        .iter()
                        .map(|(address, pages)| (address.clone(), *pages, ugly)),
                );
            }
            // Hugo put the later pages of a tag's page or a series' beneath
            // it, whatever the pattern; the old built output, where it is there, shows
            // which there were, and the shared part sends readers on.
            let size = self.pager_size();
            let path = self.pager_path();
            let later = (counts.iter())
                .find(|(_, pages)| *pages > size)
                .filter(|_| self.import.built.is_none())
                .map(|(address, _)| match address.strip_suffix(".html") {
                    Some(folder) => format!("{folder}/{path}/2.html"),
                    None => format!("{address}{path}/2/"),
                });
            if let (Some(pattern), Some(statacs)) = (pattern, statacs) {
                match kept {
                    false if self.starter => {
                        let one = match plural {
                            "tags" => "A tag's",
                            _ => "A series'",
                        };
                        let later = match later {
                            Some(later) => format!(
                                " {one} later pages stood beneath its page, such as {}: \
                                 nothing answers those. With the old site's built output in {}, \
                                 `statac import` sends readers on from each it finds there.",
                                code(&later),
                                code(&format!("{}/", self.s.publish_dir))
                            ),
                            None => String::new(),
                        };
                        self.note(format!(
                            "The settings' `[permalinks]` put the page of each of the `{plural}` \
                             at {}, which Statac has no pattern for, so each is at Statac's own \
                             address, and a page at the old one sends readers on to it.{later}",
                            code(&pattern)
                        ))
                    }
                    false => self.note(format!(
                        "The settings' `[permalinks]` put the page of each of the `{plural}` at \
                         {}, which Statac has no pattern for, so each is at Statac's own \
                         address: those Hugo used are listed under Addresses.",
                        code(&pattern)
                    )),
                    true => self
                        .import
                        .settings
                        .push((format!("{plural}.permalink"), Value::Text(statacs))),
                }
            }
        }
        // Hugo wrote a feed beside the page of each tag and of each list
        // of that kind; Statac gives one only where a page asks.
        if self.s.feeds && self.s.feeds_for.2 && self.s.terms {
            let mut feeds: Vec<String> = Vec::new();
            for doc in self.docs.iter().filter(|d| !d.draft) {
                let prefix = format!("/{}", self.prefix(&doc.language));
                let prefix = prefix.trim_end_matches('/');
                for (plural, terms) in &doc.terms {
                    for term in terms {
                        let feed = format!(
                            "{prefix}/{plural}/{}/{}",
                            self.making.made(term),
                            self.s.feed_name
                        );
                        if !feeds.contains(&feed) {
                            feeds.push(feed);
                        }
                    }
                }
            }
            if let Some(example) = feeds.first() {
                let said = format!(
                    "Hugo's feed for each tag and each list of that kind, {} of them, such as {}: \
                     Statac gives such a list a feed only where a page of yours at its address \
                     asks for one with `feed:`.",
                    feeds.len(),
                    code(example)
                );
                self.import.findings.push(Finding {
                    kind: &super::write::FEED_LEFT,
                    about: None,
                    spots: vec![super::Spot {
                        file: String::new(),
                        line: None,
                        text: said,
                        times: None,
                    }],
                });
            }
        }
        self.import.addresses.extend(addresses);
        let path = self.pager_path();
        for (address, pages, file) in held {
            let pages = pages.div_ceil(self.pager_size());
            match file {
                // Hugo published a tag's page under `uglyURLs` as a file,
                // `/tags/rust.html`, and its later pages as files beneath
                // `/tags/rust/`, in every version.
                true if pages > 1 && self.files_later.is_none() => {
                    let folder = address.strip_suffix(".html").unwrap_or(&address);
                    self.files_later = Some(format!("{folder}/{path}/2.html"));
                }
                true => {}
                false => {
                    for n in 2..=pages {
                        (self.import.addresses).push(format!("{address}{path}/{n}/"));
                    }
                }
            }
        }
        // Hugo before 0.148 published the page listing every tag under
        // `uglyURLs` as a file; since, as a folder. The old built output,
        // where it is there, tells which, and the shared part keeps it.
        if self.s.ugly.of("tags") && self.import.built.is_none() && self.starter {
            for (list, plural) in &self.import.term_lists {
                if plural != "tags" {
                    continue;
                }
                let within = list.trim_start_matches('/').trim_end_matches('/');
                let path = match within.strip_suffix("tags") {
                    Some(language) => format!("{language}tags.md"),
                    None => continue,
                };
                let from = Value::List(vec![Value::Text(format!("/{within}.html"))]);
                (self.import.words).push((path, Field::new("redirect_from", from, None)));
            }
        }
        // The page Hugo made for each name in any other list, and the one
        // listing them all, which for tags under `uglyURLs` was a file.
        if self.s.terms {
            for doc in self.docs.iter().filter(|d| !d.draft) {
                let prefix = format!("/{}", self.prefix(&doc.language));
                let prefix = prefix.trim_end_matches('/');
                for (plural, terms) in &doc.terms {
                    if (plural == "tags" && !self.s.ugly.of(plural))
                        || (plural == "series" && terms.len() < 2)
                    {
                        continue;
                    }
                    let ugly = self.s.ugly.of(plural);
                    for term in terms.iter().filter(|_| plural != "tags") {
                        let path = format!("/{plural}/{}/", self.making.made(term));
                        let path = url::finished(&self.making.path(&path), ugly);
                        let page = (plural.clone(), term.clone(), format!("{prefix}{path}"));
                        if !self.import.term_pages.contains(&page) {
                            self.import.term_pages.push(page);
                        }
                    }
                    let list = (format!("{prefix}/{plural}/"), plural.clone());
                    if !self.import.term_lists.contains(&list) {
                        self.import.term_lists.push(list);
                    }
                }
            }
        }
    }

    /// The snippets standing for tags that became calls, each with one
    /// finding listing its calls.
    fn snippets(&mut self, calls: &[writing::Call]) {
        let mut snippets: Vec<&str> = Vec::new();
        for call in calls {
            if !snippets.contains(&call.snippet.as_str()) {
                snippets.push(&call.snippet);
            }
        }
        for snippet in snippets {
            let of: Vec<&writing::Call> = calls.iter().filter(|c| c.snippet == snippet).collect();
            let mut names: Vec<&str> = Vec::new();
            for call in &of {
                for name in &call.names {
                    if !names.contains(&name.as_str()) {
                        names.push(name);
                    }
                }
            }
            let path = format!("snippets/{snippet}.html");
            let tag = of[0].tag.replace("--", "- -").replace(['{', '}'], "");
            let shown: String = names
                .iter()
                .map(|name| format!(" {name}: {{{{ {name} | default(\"\") }}}}"))
                .collect();
            let mut text = format!(
                "<!-- Not converted: this snippet stands for the old site's `{tag}`. Write it as \
                 https://statac.dev/snippets/ describes, or take its calls out: see \
                 IMPORT.md.{shown} -->\n"
            );
            if of.iter().any(|c| c.wraps) {
                text.push_str("{{ body }}\n");
            }
            self.import.written.push((path.clone(), text));
            self.import.findings.push(Finding {
                kind: &SNIPPET_CALLS,
                about: Some(path),
                spots: of.iter().map(|call| call.spot.clone()).collect(),
            });
        }
    }

    /// The addresses Hugo gave the later pages of each list below the home
    /// page, where its template showed the list a few pages at a time, to
    /// be looked for in the new site; the first, which Hugo sent on to the
    /// list, goes among the list's other addresses. Whether any list had
    /// later pages.
    fn later_pages(&mut self) -> bool {
        let lists = self.lists();
        let size = self.pager_size();
        let path = self.pager_path();
        let mut any = false;
        for at in 0..self.docs.len() {
            let list = &self.docs[at];
            if !matches!(list.shape, Shape::List | Shape::Made) || list.dir.is_empty() {
                continue;
            }
            // A list Hugo published as a file had no later pages.
            let Some(address) = list.address.clone().filter(|a| a.ends_with('/')) else {
                continue;
            };
            // Hugo's list holds the pages in its folder and in folders
            // beneath that are no lists, and each list beneath it.
            let held = (self.docs.iter())
                .filter(|d| !d.draft && d.language == list.language)
                .filter(|d| match d.shape {
                    Shape::Single | Shape::Bundle => {
                        self.sections(&d.language, &d.dir, &lists).join("/") == list.dir
                    }
                    Shape::List => parent(&d.dir) == list.dir,
                    Shape::Made => false,
                })
                .count();
            let pages = held.div_ceil(size);
            if pages < 2 {
                continue;
            }
            any = true;
            let file = format!("{}.html", address.trim_end_matches('/'));
            if self.older_lists.is_none() && list.others.contains(&file) {
                self.older_lists = Some((file, format!("{address}{path}/2.html")));
            }
            self.docs[at].others.push(format!("{address}{path}/1/"));
            for n in 2..=pages {
                self.import.addresses.push(format!("{address}{path}/{n}/"));
            }
        }
        any
    }

    /// How many pages each of a list's pages showed.
    fn pager_size(&self) -> usize {
        (self.s.pager_size)
            .and_then(|size| usize::try_from(size).ok())
            .filter(|size| *size > 0)
            .unwrap_or(10)
    }

    /// The folder a list's later pages stood in, by their numbers.
    fn pager_path(&self) -> String {
        (self.s.pager_path.as_deref())
            .map(|path| path.trim_matches('/'))
            .filter(|path| !path.is_empty())
            .unwrap_or("page")
            .to_owned()
    }

    /// The settings for a list's later pages, in a site of so many posts.
    /// `firsts` is whether a list's first later page was sent on.
    fn paging(&mut self, posts: u64, firsts: bool) {
        if let Some(size) = self.s.pager_size
            && size != 10
        {
            self.import
                .settings
                .push(("posts_per_page".into(), Value::Number(size.to_string())));
        }
        if let Some(path) = self.s.pager_path.clone() {
            let path = path.trim_matches('/');
            if path != "page" && !path.is_empty() {
                self.import.settings.push((
                    "later_pages".into(),
                    Value::Text(format!("{{list}}{path}/{{n}}/")),
                ));
            }
        }
        // Later pages are the layouts' doing, which nothing here reads: a
        // site with more posts than a page shows may have had them.
        let more = posts > self.s.pager_size.unwrap_or(10);
        if self.s.pager_size.is_some() || self.s.pager_path.is_some() || more || firsts {
            let firsts = match firsts {
                true => {
                    " The first of a list's later pages, which Hugo sent on to the list, is in \
                     the list's `redirect_from`."
                }
                false => "",
            };
            self.note(format!(
                "Hugo made later pages only for the lists whose templates asked for them, which \
                 Statac can't tell from the writing; Statac makes them for every list with more \
                 posts than a page shows: see https://statac.dev/lists/#pages-of-a-list.{firsts}"
            ));
        }
        // The old built output, where it is there, shows which of these
        // there were, and the shared part sends readers on from them.
        let files = match (&self.older_lists, &self.files_later) {
            _ if self.import.built.is_some() => None,
            (Some((list, later)), None) => Some(format!(
                "Hugo before version 0.148 published a list under `uglyURLs` as a file, such as \
                 {}, which now sends readers on to the list, and its later pages as files too, \
                 such as {}: nothing answers those.",
                code(list),
                code(later)
            )),
            (Some((list, later)), Some(tags)) => Some(format!(
                "Hugo before version 0.148 published a list under `uglyURLs` as a file, such as \
                 {}, which now sends readers on to the list, and its later pages as files too, \
                 such as {}; and every version published a tag's later pages as files, such as \
                 {}: nothing answers those.",
                code(list),
                code(later),
                code(tags)
            )),
            (None, Some(tags)) => Some(format!(
                "Hugo published a tag's later pages under `uglyURLs` as files, such as {}: \
                 nothing answers those.",
                code(tags)
            )),
            (None, None) => None,
        };
        if let Some(files) = files {
            self.note(format!(
                "{files} With the old site's built output in {}, `statac import` sends readers on \
                 from each it finds there.",
                code(&format!("{}/", self.s.publish_dir))
            ));
        }
        if self.s.pager_size.is_some() || more {
            self.note(format!(
                "Where the old home page had later pages, nothing answers them: {}, and Statac \
                 can't tell from the writing which later pages Hugo made.",
                one_home_page(self.starter)
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn front_matter_is_read_in_each_format() {
        let yaml = front(
            "---\nTitle: Post 1\ntags: [a]\n---\nWriting.\n",
            "a.md",
            true,
        )
        .unwrap();
        let toml = front(
            "+++\nTitle = \"Post 1\"\ntags = [\"a\"]\n+++\nWriting.\n",
            "a.md",
            true,
        )
        .unwrap();
        let json = front(
            "{\n  \"Title\": \"Post 1\",\n  \"tags\": [\"a\"]\n}\nWriting.\n",
            "a.md",
            true,
        )
        .unwrap();
        for read in [&yaml, &toml, &json] {
            let fields: Vec<(&str, &Value, Option<usize>)> = read
                .fields
                .iter()
                .map(|f| (f.name.as_str(), &f.value, f.line))
                .collect();
            assert_eq!(
                fields,
                vec![
                    ("Title", &Value::Text("Post 1".into()), Some(2)),
                    ("tags", &Value::List(vec![Value::Text("a".into())]), Some(3)),
                ]
            );
            assert_eq!(read.body, "Writing.\n");
            assert_eq!(read.body_line, 5);
        }
        assert_eq!(
            front("+++\ntitle = \"Post 1\"\ndate = \n+++\n", "a.md", true).err(),
            Some((3, "date = ".to_owned()))
        );
    }

    /// A tag's later pages are looked for where Statac's tag pages stand
    /// where Hugo's did; under `uglyURLs` Hugo's were files, and are
    /// spoken of instead.
    #[test]
    fn the_later_pages_of_a_tag_are_looked_for_where_statac_publishes_them() {
        let top = std::env::temp_dir().join(format!("statac-hugo-{}", std::process::id()));
        // The settings, and whether a tag's second page is looked for.
        for (at, (settings, looked_for)) in [("", true), ("uglyURLs = true\n", false)]
            .into_iter()
            .enumerate()
        {
            let folder = top.join(at.to_string());
            let _ = std::fs::remove_dir_all(&folder);
            std::fs::create_dir_all(folder.join("content/posts")).unwrap();
            let paging = "[pagination]\npagerSize = 1\n";
            let toml =
                format!("baseURL = 'https://example.org/'\ntitle = 'Tags'\n{settings}{paging}");
            std::fs::write(folder.join("hugo.toml"), toml).unwrap();
            for n in 1..=2 {
                let post = format!("---\ntitle: Post {n}\ndate: 2026-01-0{n}\ntags: [a]\n---\n");
                std::fs::write(folder.join(format!("content/posts/post-{n}.md")), post).unwrap();
            }
            let Ok(import) = read(&folder, Date::day_only(2026, 10, 6), None, true) else {
                panic!("{at}: the site wasn't read");
            };
            let has = |address: &str| import.addresses.iter().any(|a| a == address);
            assert_eq!(
                has("/tags/a/page/2/"),
                looked_for,
                "{at}: {:?}",
                import.addresses
            );
            assert!(!has("/tags/a/page/3/"), "{at}: {:?}", import.addresses);
            assert_eq!(
                import
                    .notes
                    .iter()
                    .any(|note| note.contains("`/tags/a/page/2.html`")),
                !looked_for,
                "{at}: {:?}",
                import.notes
            );
        }
    }
}
