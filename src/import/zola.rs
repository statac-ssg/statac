//! The reader for sites Zola made: a site with `zola.toml` at its top, or
//! a `config.toml` that sets `base_url`.
//!
//! Zola's rules are followed as its documentation gives them: which
//! folders it reads, where it publishes each page, list and term, how
//! `[slugify]` makes a name ready for an address, which dates it reads
//! from a file's name, and what its own tags do in the writing.

mod settings;
mod slug;
mod writing;

use std::collections::HashSet;
use std::path::Path;

use super::common::{
    counted, fields, front, glob_matches, joined, last, listed, names_in, parent, text_of, texts_of,
};
use super::inside;
use super::report::code;
use super::{
    Failed, Field, File, Finding, FindingKind, Import, Left, Page, PageKind, Value, left_behind,
    one_home_page,
};
use crate::content::address::{Address, name_fault};
use crate::content::slug as statac_slug;
use crate::date::{self, Date};
use crate::problem::{Kind, Problem, system_reason};
use settings::{Settings, Taxonomy};
use slug::Strategy;
use writing::Reading;

pub static TAG_LEFT: FindingKind = FindingKind {
    heading: "Tags in the writing left as written",
    sentence: "Statac's writing runs none of Zola's tags, and these have no equal in it, so each \
               is left as written and shows on the page as it stands: write by hand what it \
               made, or call a snippet, as https://statac.dev/snippets/ describes.",
};

pub static TEMPLATE_CODE: FindingKind = FindingKind {
    heading: "Template code Zola may have run",
    sentence: "Nothing in the old site says which version of Zola made it: 0.23 and later run \
               these as template code, and earlier versions show them as written. Each is left \
               as written and shows on the page as it stands: where the old site ran it, write \
               by hand what it made.",
};

pub static TAG_IN_CODE: FindingKind = FindingKind {
    heading: "Tags inside code",
    sentence: "Zola ran these even inside code, and Statac shows code exactly as written, so each \
               now shows as it stands: write by hand what it made, where that was meant.",
};

pub static LINK_NOWHERE: FindingKind = FindingKind {
    heading: "Links to pages that didn't come across",
    sentence: "Each of these names a page by its file, and Statac couldn't find that file in the \
               old site, or Zola didn't publish it, so it is left as written: link to the page's \
               address instead.",
};

pub static SNIPPET_CALLS: FindingKind = FindingKind {
    heading: "Tags now snippet calls",
    sentence: "Each of these tags is now a call to a snippet of its name, and the snippet holds \
               only a comment saying it wasn't converted: write the snippet, as \
               https://statac.dev/snippets/ describes, or take the calls out.",
};

pub static CODE_OPTIONS: FindingKind = FindingKind {
    heading: "Options on code taken off",
    sentence: "Statac reads the rest of a code block's first line as its caption, so Zola's \
               options after a comma, such as `,linenos`, were taken off each of these: the code \
               is shown without them.",
};

pub static EMOJI: FindingKind = FindingKind {
    heading: "Emoji written by name",
    sentence: "Zola showed each of these as its emoji, as the settings' `render_emoji` asked, and \
               Statac shows the name as written: read each page that has one, and write the \
               emoji itself in its place.",
};

pub static FRONT_MATTER_UNREAD: FindingKind = FindingKind {
    heading: "Front matter Statac can't read",
    sentence: "Statac's reader stopped at this line in the front matter of each of these, so the \
               page didn't come across: put it right and import again.",
};

pub static SETTINGS_LEFT: FindingKind = FindingKind {
    heading: "Settings with no equal",
    sentence: "Statac has no equal for each of these settings, so nothing came across for them: \
               https://statac.dev/settings/ says what Statac offers.",
};

pub static ADDRESS_UNKNOWN: FindingKind = FindingKind {
    heading: "Addresses Statac couldn't work out",
    sentence: "Zola wrote each of these names in ASCII for its address, and Statac doesn't know \
               how it writes a letter of this script, so the page's old address wasn't worked \
               out or kept: give the page a `permalink` of the address the old site published it \
               at.",
};

pub static ADDRESS_UNUSABLE: FindingKind = FindingKind {
    heading: "Addresses Statac can't publish at",
    sentence: "Zola published each of these pages at an address holding a character Statac's \
               addresses can't hold, such as `%` or `?`, so the page is at Statac's own address, \
               and the old one isn't kept.",
};

pub static ORDER_UNKNOWN: FindingKind = FindingKind {
    heading: "Orders Statac doesn't know",
    sentence: "Zola ordered the pages of each of these lists by a `sort_by` Statac doesn't know, \
               so the list is newest first: write the order you want as `order:`, as \
               https://statac.dev/lists/#order describes.",
};

/// Whether `folder` holds a Zola site.
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
    let (file, entries) = settings::found(folder)?;
    let s = settings::settle(&entries);
    let mut reader = Reader {
        folder,
        given,
        starter,
        import: Import::default(),
        s,
        today,
        languages: Vec::new(),
        docs: Vec::new(),
        own: Vec::new(),
        later: false,
        earlier: false,
        dropped: Vec::new(),
    };
    reader.import.settings_file = file;
    reader.settled();
    reader.walk()?;
    reader.finish();
    Ok(reader.import)
}

/// What a page is to Zola.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Shape {
    /// A page that is one file.
    Single,
    /// A page kept as a folder, `index.md` with its files beside it.
    Bundle,
    /// A folder's own page, `_index.md`.
    List,
}

/// What a folder's own page says of its list.
#[derive(Clone, Default, Debug)]
struct ListWords {
    sort_by: Option<(String, Option<usize>)>,
    paginate_by: Option<u64>,
    paginate_path: Option<String>,
    reversed: bool,
    transparent: bool,
    feeds: Option<bool>,
    render: bool,
}

/// A page as found, before its writing is converted.
struct Doc {
    from: String,
    /// Its path in `content/`, as an `@/` link names it.
    rel: String,
    to: String,
    language: String,
    shape: Shape,
    kind: PageKind,
    fields: Vec<Field>,
    words: Vec<Field>,
    body: String,
    body_line: usize,
    /// The folder the page stands in, under `content/`: for a folder's own
    /// page, that folder.
    dir: String,
    date: Option<Date>,
    updated: Option<Date>,
    title: String,
    weight: Option<i64>,
    /// The name in its address, as Zola made it; none where Statac can't
    /// tell.
    made: Option<String>,
    draft: bool,
    list: ListWords,
    terms: Vec<(String, Vec<String>)>,
    address: Option<String>,
    own_address: bool,
    others: Vec<String>,
    /// Whether it takes nothing but words for the page the starter has
    /// there.
    words_only: bool,
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
    today: Date,
    /// Every language the settings declare, the main one first.
    languages: Vec<String>,
    docs: Vec<Doc>,
    /// The names of the site's own tags.
    own: Vec<String>,
    /// Whether the templates define a component, which only Zola 0.23 and
    /// later read.
    later: bool,
    /// Whether the templates hold a shortcode or a macro, which only
    /// versions before 0.23 read.
    earlier: bool,
    /// Zola's words taken off pages, each with the files it was on.
    dropped: Vec<(&'static str, Vec<String>)>,
}

/// The date a name opens with, `2026-01-02-` or `2026-01-02_`, and the
/// rest of the name.
fn dated_name(name: &str) -> Option<(Date, &str)> {
    let date = date::read(name.get(..10)?).ok()?;
    let rest = name[10..].strip_prefix(' ').unwrap_or(&name[10..]);
    let rest = rest.strip_prefix(['-', '_'])?;
    (!rest.is_empty()).then_some((date, rest))
}

/// A text field, where it isn't empty.
fn text_field(fields: &[Field], name: &str) -> Option<String> {
    fields
        .iter()
        .find(|f| f.name == name)
        .and_then(|f| text_of(&f.value))
        .map(|t| t.trim().to_owned())
        .filter(|t| !t.is_empty())
}

/// Two names compared as people read them: small letters and accents
/// alike, and figures by the number they make.
fn natural(a: &str, b: &str) -> std::cmp::Ordering {
    use unicode_normalization::UnicodeNormalization;
    use unicode_normalization::char::is_combining_mark;
    let fold = |s: &str| -> Vec<char> {
        s.nfd()
            .filter(|c| !is_combining_mark(*c))
            .flat_map(char::to_lowercase)
            .collect()
    };
    let (a, b) = (fold(a), fold(b));
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i].is_ascii_digit() && b[j].is_ascii_digit() {
            let (si, sj) = (i, j);
            while i < a.len() && a[i].is_ascii_digit() {
                i += 1;
            }
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            let x: String = a[si..i].iter().collect();
            let y: String = b[sj..j].iter().collect();
            let (x, y) = (x.trim_start_matches('0'), y.trim_start_matches('0'));
            let order = x.len().cmp(&y.len()).then_with(|| x.cmp(y));
            if order.is_ne() {
                return order;
            }
        } else {
            let order = a[i].cmp(&b[j]);
            if order.is_ne() {
                return order;
            }
            i += 1;
            j += 1;
        }
    }
    (a.len() - i).cmp(&(b.len() - j))
}

impl Reader<'_> {
    fn path(&self, rel: &str) -> std::path::PathBuf {
        inside::path(self.folder, rel)
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

    fn main(&self) -> &str {
        &self.s.default_language
    }

    /// The folder a language's pages stand in under `content/` in the new
    /// site, and the path its addresses start with, ending `/` where there
    /// is one.
    fn prefix(&self, language: &str) -> String {
        match language == self.main() {
            true => String::new(),
            false => format!("{language}/"),
        }
    }

    /// Whether the folder `rel`, which the setting `setting` names as
    /// `written`, may be read: it is inside the old site and reached
    /// through no link that leads outside it. Where it may not, a finding
    /// says so.
    fn followed(&mut self, rel: &str, setting: &str, written: &str, line: Option<usize>) -> bool {
        if crate::platform::stays_inside(written)
            && inside::named_folder(self.folder, rel).is_some()
        {
            return true;
        }
        let file = self.import.settings_file.clone();
        self.find(
            &inside::FOLDER_UNREAD,
            &file,
            line,
            &format!("{setting}: {written}"),
        );
        false
    }

    /// The taxonomies a language declares.
    fn taxonomies_of(&self, language: &str) -> Vec<Taxonomy> {
        match language == self.main() {
            true => self.s.taxonomies.clone(),
            false => self
                .s
                .languages
                .iter()
                .find(|l| l.code == language)
                .and_then(|l| l.taxonomies.clone())
                .unwrap_or_else(|| self.s.taxonomies.clone()),
        }
    }

    /// Whether a language has a feed of its own, and its files' names.
    fn feeds_of(&self, language: &str) -> (bool, Vec<String>) {
        match self.s.languages.iter().find(|l| l.code == language) {
            Some(l) if language != self.main() => (
                l.feeds.unwrap_or(false),
                l.feed_filenames
                    .clone()
                    .filter(|names| !names.is_empty())
                    .unwrap_or_else(|| vec!["atom.xml".into()]),
            ),
            _ => (self.s.feeds, self.s.feed_filenames.clone()),
        }
    }

    /// What the settings give the import: its title, address, languages
    /// and zone, the author's own values, and what is said of each.
    fn settled(&mut self) {
        let s = &self.s;
        self.import.title = s.title.clone();
        let mut notes = Vec::new();
        match (&s.base_url, self.given) {
            (Some(url), _) if url.starts_with("https://") || url.starts_with("http://") => {
                self.import.address = Some(url.trim_end_matches('/').to_owned());
            }
            (_, Some(given)) => self.import.address = Some(given.to_owned()),
            (Some(url), None) => notes.push(format!(
                "The settings' `base_url`, {}, isn't a web address, so the new site has no `url` \
                 yet: add it to `statac.yaml` once it is known.",
                code(url)
            )),
            (None, None) => {}
        }
        let main = s.default_language.clone();
        self.languages = std::iter::once(main.clone())
            .chain(
                s.languages
                    .iter()
                    .map(|l| l.code.clone())
                    .filter(|c| *c != main),
            )
            .collect();
        if self.languages.len() > 1 {
            let unknown: Vec<String> = self
                .languages
                .iter()
                .filter(|c| !crate::new::a_language_code(c))
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
            for language in &s.languages {
                if let Some(title) = &language.title
                    && Some(title) != s.title.as_ref()
                    && language.code != main
                {
                    self.import.settings.push((
                        format!("{}.title", language.code),
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
                code(&main)
            ));
        } else {
            match crate::new::a_language_code(&main) {
                true => self.import.languages = vec![main.clone()],
                false => notes.push(format!(
                    "The settings name the language {}, which isn't a code Statac reads, so the \
                     site is in English, `en`: write its code in `language` in `statac.yaml`.",
                    code(&main)
                )),
            }
        }
        self.import.timezone = Some("UTC".into());
        notes.push(
            "Zola has no setting for a time zone, and reads a date written without an offset as \
             UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its \
             dates were meant there."
                .to_owned(),
        );
        if !s.smart_punctuation {
            self.import
                .settings
                .push(("typography".into(), Value::Truth(false)));
            notes.push(
                "Zola's settings left straight quotes, `--` and `...` as typed, so `typography: \
                 false` keeps them so: take the line out to have Statac set them as print does."
                    .to_owned(),
            );
        }
        if s.anchors != Strategy::On {
            notes.push(
                "The settings' `[slugify]` had Zola make each heading's id keeping its capitals \
                 and punctuation, and Statac makes it in small letters, its words joined by \
                 hyphens: a link to a heading by its old id lands at the top of the page, which \
                 `statac check` tells of."
                    .to_owned(),
            );
        }
        if s.target_blank || s.no_follow || s.no_referrer {
            let mut rel = Vec::new();
            if s.target_blank {
                rel.push("noopener");
            }
            if s.no_follow {
                rel.push("nofollow");
            }
            if s.no_referrer {
                rel.push("noreferrer");
            }
            let target = match s.target_blank {
                true => " target=\"_blank\"",
                false => "",
            };
            self.import.written.push((
                "snippets/markdown/link.html".into(),
                format!(
                    "{{# Each link in the writing, one to another site opened as the old site's \
                     settings asked: see /snippets/#pieces-of-the-writing -#}}\n\
                     <a href=\"{{{{ address }}}}\"{{% if title %}} title=\"{{{{ title }}}}\"\
                     {{% endif %}}{{% if outside %}}{target} rel=\"{}\"{{% endif %}}>\
                     {{{{ body }}}}</a>",
                    rel.join(" ")
                ),
            ));
            notes.push(
                "Zola's settings' `[markdown]` opened each link to another site with its own \
                 `target` or `rel`, so `snippets/markdown/link.html` writes each link the same."
                    .to_owned(),
            );
        }
        for note in notes {
            self.note(note);
        }
        self.import.values = std::mem::take(&mut self.s.values);
        let left = std::mem::take(&mut self.s.left);
        let file = self.import.settings_file.clone();
        for (line, text) in left {
            self.find(&SETTINGS_LEFT, &file, line, &text);
        }
        // An `output_dir` that may not be read is read as not set.
        let mut output = self.s.output_dir.clone();
        if output != "public" && !self.followed(&output, "output_dir", &output, self.s.output_line)
        {
            output = "public".to_owned();
            self.s.output_dir = output.clone();
        }
        if inside::a_folder(self.folder, &output) {
            self.import.built = Some(output);
        }
    }

    /// Reads the top of the old site and every folder Zola reads.
    fn walk(&mut self) -> Result<(), Box<Failed>> {
        if let Some(theme) = self.s.theme.clone() {
            let dir = format!("themes/{theme}");
            if self.followed(&dir, "theme", &theme, self.s.theme_line) {
                self.own_tags(&format!("{dir}/templates/shortcodes"));
                self.own_components(&format!("{dir}/templates"));
            }
        }
        self.own_tags("templates/shortcodes");
        self.own_components("templates");
        let output_top = self.s.output_dir.split('/').next().unwrap_or("").to_owned();
        for name in names_in(self.folder)? {
            let Ok(found) = inside::metadata(self.folder, &name) else {
                continue;
            };
            let dir = found.is_dir();
            let shown = match dir {
                true => format!("{name}/"),
                false => name.clone(),
            };
            if found.is_symlink() {
                self.left(&name, inside::why(self.folder, &name));
                continue;
            }
            if name == self.import.settings_file {
                continue;
            }
            let starter = self.starter;
            let what: String = match (name.as_str(), dir) {
                ("content", true) => {
                    self.content("")?;
                    continue;
                }
                ("static", true) => {
                    self.statics("")?;
                    continue;
                }
                (name, false) if settings::FILES.contains(&name) => {
                    "another settings file, which Zola reads only where `zola.toml` isn't there"
                        .into()
                }
                ("templates", true) => left_behind(
                    "the old site's templates",
                    "the starter's templates take",
                    starter,
                ),
                ("themes", true) => left_behind(
                    "the old site's themes",
                    "the starter's templates, snippets and stylesheet take",
                    starter,
                ),
                ("sass", true) => left_behind(
                    "the old site's Sass stylesheets",
                    "the starter's `assets/site.css` takes",
                    starter,
                ),
                (name, true) if name == output_top => {
                    "the old site's built output, read only to check the addresses".into()
                }
                (_, true) => "a folder Zola doesn't publish".into(),
                (_, false) => "a file Zola doesn't publish".into(),
            };
            self.left(&shown, &what);
        }
        Ok(())
    }

    /// Learns the names of the site's own tags from the folder `dir`.
    fn own_tags(&mut self, dir: &str) {
        if !inside::a_folder(self.folder, dir) {
            return;
        }
        let Ok(names) = names_in(&self.path(dir)) else {
            return;
        };
        for name in names {
            let stem = match name.rsplit_once('.') {
                Some((stem, "html" | "md")) => stem,
                _ => continue,
            };
            self.earlier = true;
            if !self.own.iter().any(|o| o == stem) {
                self.own.push(stem.to_owned());
            }
        }
    }

    /// Learns the names of the components the templates in the folder
    /// `dir`, and in every folder beneath it, define, and whether any
    /// holds a macro.
    fn own_components(&mut self, dir: &str) {
        let Ok(names) = names_in(&self.path(dir)) else {
            return;
        };
        for name in names {
            let rel = joined(dir, &name);
            let Ok(found) = inside::metadata(self.folder, &rel) else {
                continue;
            };
            if found.is_symlink() {
                continue;
            }
            if found.is_dir() {
                self.own_components(&rel);
                continue;
            }
            let Ok(template) = std::fs::read_to_string(self.path(&rel)) else {
                continue;
            };
            self.earlier |= writing::macros(&template);
            for component in writing::components(&template) {
                self.later = true;
                if !self.own.contains(&component) {
                    self.own.push(component);
                }
            }
        }
    }

    /// Whether `ignored_content` leaves out the file `rel` of `content/`.
    fn ignored(&self, globs: &[String], rel: &str) -> bool {
        globs
            .iter()
            .any(|glob| glob_matches(glob, rel) || glob_matches(glob, last(rel)))
    }

    /// Copies every file of `static/`, beneath `rel`, to `static/`.
    fn statics(&mut self, rel: &str) -> Result<(), Box<Failed>> {
        let top = joined("static", rel);
        for name in names_in(&self.path(&top))? {
            let within = joined(rel, &name);
            let path = joined("static", &within);
            let Ok(found) = inside::metadata(self.folder, &path) else {
                continue;
            };
            if found.is_symlink() {
                self.left(&path, inside::why(self.folder, &path));
            } else if self.ignored(&self.s.ignored_static.clone(), &within) {
                self.left(
                    &path,
                    "a file the settings' `ignored_static` names, which Zola doesn't publish",
                );
            } else if found.is_dir() {
                self.statics(&within)?;
            } else {
                self.import.files.push(File {
                    from: path,
                    to: format!("static/{within}"),
                });
            }
        }
        Ok(())
    }

    /// A Markdown file's name split into its stem and its language, where
    /// it is one.
    fn page_name<'n>(&self, name: &'n str) -> Option<(&'n str, String)> {
        let stem = name.strip_suffix(".md")?;
        Some(match stem.rsplit_once('.') {
            Some((rest, code_)) if self.languages.iter().any(|l| l == code_) => {
                (rest, code_.to_owned())
            }
            _ => (stem, self.main().to_owned()),
        })
    }

    /// Reads the folder `rel` of `content/`.
    fn content(&mut self, rel: &str) -> Result<(), Box<Failed>> {
        let names = names_in(&self.path(&joined("content", rel)))?;
        let globs = self.s.ignored_content.clone();
        let indexes: Vec<String> = names
            .iter()
            .filter_map(|n| self.page_name(n).filter(|(stem, _)| *stem == "index"))
            .map(|(_, language)| language)
            .collect();
        let bundle = !rel.is_empty() && !indexes.is_empty();
        for name in &names {
            let within = joined(rel, name);
            let path = joined("content", &within);
            let Ok(found) = inside::metadata(self.folder, &path) else {
                continue;
            };
            if found.is_symlink() {
                self.left(&path, inside::why(self.folder, &path));
                continue;
            }
            if self.ignored(&globs, &within) {
                self.left(
                    &path,
                    "a file the settings' `ignored_content` names, which Zola doesn't read",
                );
                continue;
            }
            let page = self.page_name(name);
            if bundle {
                match page {
                    Some(("index", language)) => self.page(&within, &language, Shape::Bundle)?,
                    Some(_) => self.left(
                        &path,
                        "writing kept among a page's files, which Zola doesn't publish as a page",
                    ),
                    None => {
                        let (dir, folder) = (parent(rel), last(rel));
                        let folder = self.bare_name(folder).to_owned();
                        for language in indexes.clone() {
                            let prefix = self.prefix(&language);
                            self.beside(&path, &format!("{prefix}{}", joined(dir, &folder)), name)?;
                        }
                    }
                }
                continue;
            }
            match page {
                _ if found.is_dir() => self.content(&within)?,
                Some(("_index", language)) => self.page(&within, &language, Shape::List)?,
                Some((_, language)) => self.page(&within, &language, Shape::Single)?,
                None => self.beside(&path, rel, name)?,
            }
        }
        Ok(())
    }

    /// Copies the file, or folder of files, `path`, named `name`, into the
    /// folder `folder` of the new site's `content/`.
    fn beside(&mut self, path: &str, folder: &str, name: &str) -> Result<(), Box<Failed>> {
        let Ok(found) = inside::metadata(self.folder, path) else {
            return Ok(());
        };
        if !found.is_dir() {
            self.import.files.push(File {
                from: path.to_owned(),
                to: format!("content/{}", joined(folder, name)),
            });
            return Ok(());
        }
        for inner in names_in(&self.path(path))? {
            let inner_path = joined(path, &inner);
            match inside::metadata(self.folder, &inner_path) {
                Ok(found) if found.is_symlink() => {
                    self.left(&inner_path, inside::why(self.folder, &inner_path));
                }
                Ok(_) => self.beside(&inner_path, &joined(folder, name), &inner)?,
                Err(_) => {}
            }
        }
        Ok(())
    }

    /// A page's or folder's name without the date it opens with.
    fn bare_name<'n>(&self, name: &'n str) -> &'n str {
        dated_name(name).map_or(name, |(_, rest)| rest)
    }

    /// Reads one page.
    fn page(&mut self, within: &str, language: &str, shape: Shape) -> Result<(), Box<Failed>> {
        let from = joined("content", within);
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
        let front = match front(&text, &from, false) {
            Ok(front) => front,
            Err((line, text)) => {
                self.find(&FRONT_MATTER_UNREAD, &from, Some(line), &text);
                return Ok(());
            }
        };
        let (dir, written_name) = match shape {
            Shape::Single => {
                let stem = self.page_name(last(within)).map_or("", |(stem, _)| stem);
                (parent(within).to_owned(), stem.to_owned())
            }
            Shape::Bundle => {
                let folder = parent(within);
                (parent(folder).to_owned(), last(folder).to_owned())
            }
            Shape::List => {
                let folder = parent(within);
                (folder.to_owned(), last(folder).to_owned())
            }
        };
        let name_date = dated_name(&written_name).filter(|_| shape != Shape::List);
        let name = name_date.map_or(written_name.clone(), |(_, rest)| rest.to_owned());

        let mut fields: Vec<Field> = Vec::new();
        let mut list = ListWords {
            render: true,
            ..ListWords::default()
        };
        let mut terms: Vec<(String, Vec<String>)> = Vec::new();
        let mut slug = None;
        let mut path = None;
        let mut aliases = Vec::new();
        let mut dropped: Vec<&'static str> = Vec::new();
        let mut words = Vec::new();
        let raw = front.fields;
        let has = |name: &str| raw.iter().any(|f| f.name == name);
        let summary_given = has("summary");
        let list_field = |name: &str| shape == Shape::List && LIST_WORDS.contains(&name);
        for field in &raw {
            let empty = text_of(&field.value).is_some_and(|t| t.trim().is_empty());
            match field.name.as_str() {
                "title" | "date" | "updated" if empty => {}
                "description" if empty || summary_given => {}
                "description" => {
                    fields.push(Field::new("summary", field.value.clone(), field.line))
                }
                "slug" => slug = text_of(&field.value).map(|s| s.trim().to_owned()),
                "path" => path = text_of(&field.value).map(|s| s.trim().to_owned()),
                "aliases" => aliases = texts_of(&field.value),
                "template" => dropped.push("template"),
                "in_search_index" => dropped.push("in_search_index"),
                "taxonomies" => {
                    for (plural, value) in fields_of(&field.value) {
                        let names = texts_of(value);
                        if plural.eq_ignore_ascii_case("tags") {
                            fields.push(Field::new(
                                "tags",
                                Value::List(names.iter().cloned().map(Value::Text).collect()),
                                field.line,
                            ));
                        }
                        terms.push((plural.clone(), names));
                    }
                }
                "extra" => {
                    let mut kept = Vec::new();
                    for (name, value) in fields_of(&field.value) {
                        match raw.iter().any(|f| f.name == *name)
                            || fields.iter().any(|f| f.name == *name)
                        {
                            true => kept.push((name.clone(), value.clone())),
                            false => fields.push(Field::new(name, value.clone(), field.line)),
                        }
                    }
                    if !kept.is_empty() {
                        fields.push(Field::new("extra", Value::Fields(kept), field.line));
                    }
                }
                name if list_field(name) => {
                    let text = text_of(&field.value).unwrap_or_default();
                    let truth = matches!(field.value, Value::Truth(true));
                    match name {
                        "sort_by" => list.sort_by = Some((text, field.line)),
                        "paginate_by" => list.paginate_by = text.parse().ok().filter(|n| *n > 0),
                        "paginate_path" => list.paginate_path = Some(text),
                        "paginate_reversed" => list.reversed = truth,
                        "transparent" => {
                            list.transparent = truth;
                            dropped.push("transparent");
                        }
                        "generate_feeds" | "generate_feed" => list.feeds = Some(truth),
                        "render" => {
                            list.render = !matches!(field.value, Value::Truth(false));
                            dropped.push("render");
                        }
                        "page_template" => dropped.push("page_template"),
                        "insert_anchor_links" => dropped.push("insert_anchor_links"),
                        "redirect_to" if !empty => {
                            let to = text.trim();
                            let to = match to.contains("://") || to.starts_with('/') {
                                true => to.to_owned(),
                                false => format!("/{to}"),
                            };
                            fields.push(Field::new("redirect_to", Value::Text(to), field.line));
                        }
                        _ => {}
                    }
                }
                _ => fields.push(field.clone()),
            }
        }
        if let Some((day, _)) = name_date
            && !fields.iter().any(|f| f.name == "date")
        {
            let at = usize::from(fields.first().is_some_and(|f| f.name == "title"));
            fields.insert(at, Field::new("date", Value::Text(day.iso_day()), None));
        }
        let date_of =
            |name: &str| text_field(&fields, name).and_then(|t| super::write::date_of(&t));
        let date = date_of("date");
        let updated = date_of("updated");
        let title = text_field(&fields, "title").unwrap_or_default();
        let weight = text_field(&fields, "weight").and_then(|w| w.parse().ok());
        let draft = fields
            .iter()
            .any(|f| f.name == "draft" && f.value == Value::Truth(true));
        for word in dropped {
            match self.dropped.iter_mut().find(|(w, _)| *w == word) {
                Some((_, files)) => files.push(from.clone()),
                None => self.dropped.push((word, vec![from.clone()])),
            }
        }
        if shape == Shape::List && list.paginate_by.is_some() && !dir.is_empty() {
            let n = list.paginate_by.unwrap_or_default();
            words.push(Field::new(
                "posts_per_page",
                Value::Number(n.to_string()),
                None,
            ));
        }

        let prefix = self.prefix(language);
        let made_name = match shape {
            Shape::List => None,
            _ => {
                let base = match (self.s.keep_dates, slug.as_deref().filter(|s| !s.is_empty())) {
                    (_, Some(slug)) => slug.to_owned(),
                    (true, None) => written_name.clone(),
                    (false, None) => name.clone(),
                };
                self.s.paths.made(&base)
            }
        };
        let mut own_address = false;
        let mut unknown = false;
        let address = match (shape, path.as_deref().filter(|p| !p.is_empty())) {
            (Shape::List, _) => Some(format!(
                "/{prefix}{}",
                match dir.is_empty() {
                    true => String::new(),
                    false => format!("{dir}/"),
                }
            )),
            (_, Some(path)) => {
                own_address = true;
                let path = match path.starts_with('/') {
                    true => path.to_owned(),
                    false => format!("/{path}"),
                };
                Some(match path.ends_with('/') {
                    true => path,
                    false => format!("{path}/"),
                })
            }
            (_, None) => match &made_name {
                Some(made) => Some(format!("/{prefix}{}/", joined(&dir, made))),
                None => {
                    unknown = true;
                    None
                }
            },
        };
        let address = match address {
            _ if draft => None,
            Some(address) if Address::read(&address).is_err() => {
                self.find(&ADDRESS_UNUSABLE, &from, None, &address);
                None
            }
            None if unknown && !draft => {
                self.find(&ADDRESS_UNKNOWN, &from, None, &written_name);
                None
            }
            other => other,
        };
        let others: Vec<String> = aliases
            .iter()
            .map(|alias| {
                let alias = format!("/{}", alias.trim().trim_start_matches('/'));
                match alias.ends_with(".html") || alias.ends_with('/') {
                    true => alias,
                    false => format!("{alias}/"),
                }
            })
            .collect();

        let to = match shape {
            Shape::Single => format!("{prefix}{}", joined(&dir, &format!("{name}.md"))),
            Shape::Bundle => format!("{prefix}{}/index.md", joined(&dir, &name)),
            Shape::List => format!("{prefix}{}", joined(&dir, "index.md")),
        };
        let kind = match (draft, date.is_some(), shape) {
            (true, _, _) => PageKind::Draft,
            (false, _, Shape::List) => PageKind::Page,
            (false, true, _) => PageKind::Post,
            (false, false, _) => PageKind::Page,
        };
        let words_only = shape == Shape::List
            && dir.is_empty()
            && front.body.trim().is_empty()
            && fields.is_empty()
            && others.is_empty();
        self.docs.push(Doc {
            from,
            rel: within.to_owned(),
            to,
            language: language.to_owned(),
            shape,
            kind,
            fields,
            words,
            body: front.body,
            body_line: front.body_line,
            dir,
            date,
            updated,
            title,
            weight,
            made: made_name,
            draft,
            list,
            terms,
            address,
            own_address,
            others,
            words_only,
        });
        Ok(())
    }
}

/// The words of a folder's own page that say how Zola makes its list.
const LIST_WORDS: &[&str] = &[
    "sort_by",
    "paginate_by",
    "paginate_path",
    "paginate_reversed",
    "transparent",
    "generate_feeds",
    "generate_feed",
    "render",
    "page_template",
    "insert_anchor_links",
    "redirect_to",
];

fn fields_of(value: &Value) -> &[(String, Value)] {
    fields(value)
}

impl Reader<'_> {
    /// The folder's own page that lists a page in `dir`, in `language`:
    /// the nearest folder above with one, or the top.
    fn list_of(&self, language: &str, dir: &str) -> Option<usize> {
        let mut at = dir.to_owned();
        loop {
            if let Some(found) = self
                .docs
                .iter()
                .position(|d| d.shape == Shape::List && d.language == language && d.dir == at)
            {
                return Some(found);
            }
            if at.is_empty() {
                return None;
            }
            at = parent(&at).to_owned();
        }
    }

    /// The pages Zola listed on the folder's own page `list`: those it is
    /// the nearest list of, and those of each list below that passes its
    /// pages up.
    fn members(&self, list: usize) -> Vec<usize> {
        let mut found = Vec::new();
        for (at, doc) in self.docs.iter().enumerate() {
            if doc.draft {
                continue;
            }
            let lists = |dir: &str| self.list_of(&doc.language, dir);
            match doc.shape {
                Shape::List if at != list => {
                    if doc.list.transparent && lists(parent(&doc.dir)) == Some(list) {
                        found.extend(self.members(at));
                    }
                }
                Shape::List => {}
                _ => {
                    if lists(&doc.dir) == Some(list) {
                        found.push(at);
                    }
                }
            }
        }
        found
    }

    /// Takes the address off each folder's own page Zola published none
    /// for.
    fn lists(&mut self) {
        for doc in &mut self.docs {
            if doc.shape == Shape::List && (doc.draft || !doc.list.render) {
                doc.address = None;
            }
        }
    }

    /// The name of a page in its list's `order:`: the last name of its
    /// address.
    fn order_name(doc: &Doc) -> Option<Value> {
        let address = doc.address.as_deref()?;
        Some(Value::Text(last(address.trim_end_matches('/')).to_owned()))
    }

    /// Gives each list Zola ordered other than by date that order
    /// Statac's way, as `order:` on its own page.
    fn orders(&mut self) {
        let mut unordered = Vec::new();
        for list in 0..self.docs.len() {
            if self.docs[list].shape != Shape::List || self.docs[list].draft {
                continue;
            }
            let words = self.docs[list].list.clone();
            let members = self.members(list);
            let (by, line) = words.sort_by.clone().unwrap_or(("none".into(), None));
            let folder = code(&format!("{}/", parent(&self.docs[list].from)));
            let index = code(&format!("content/{}", self.docs[list].to));
            let mut keyed: Vec<usize> = members.clone();
            let what = match by.as_str() {
                "date" => {
                    if words.reversed {
                        self.docs[list].words.push(Field::new(
                            "order",
                            Value::Text("oldest first".into()),
                            None,
                        ));
                        self.note(format!(
                            "Zola listed the pages of {folder} oldest first, as its \
                             `paginate_reversed` asked, so {index} says `order: oldest first`."
                        ));
                    }
                    continue;
                }
                "none" => {
                    if members.len() > 1 {
                        unordered.push(folder);
                    }
                    continue;
                }
                "update_date" => {
                    keyed.retain(|at| self.docs[*at].updated.is_some());
                    let moment = |at: &usize| {
                        self.docs[*at].updated.map(|d| {
                            (
                                d.year,
                                d.month,
                                d.day,
                                d.time.map(|t| (t.hour, t.minute, t.second)),
                            )
                        })
                    };
                    keyed.sort_by_key(|at| std::cmp::Reverse(moment(at)));
                    "their `updated` dates, newest first"
                }
                "title" => {
                    keyed.retain(|at| !self.docs[*at].title.is_empty());
                    keyed.sort_by(|a, b| natural(&self.docs[*a].title, &self.docs[*b].title));
                    "their titles, A to Z"
                }
                "title_bytes" => {
                    keyed.retain(|at| !self.docs[*at].title.is_empty());
                    keyed.sort_by(|a, b| self.docs[*a].title.cmp(&self.docs[*b].title));
                    "their titles, A to Z"
                }
                "weight" => {
                    keyed.retain(|at| self.docs[*at].weight.is_some());
                    keyed.sort_by_key(|at| self.docs[*at].weight);
                    "their `weight`, lightest first"
                }
                "slug" => {
                    keyed.retain(|at| self.docs[*at].made.is_some());
                    keyed.sort_by(|a, b| {
                        natural(
                            self.docs[*a].made.as_deref().unwrap_or_default(),
                            self.docs[*b].made.as_deref().unwrap_or_default(),
                        )
                    });
                    "the names in their addresses, A to Z"
                }
                _ => {
                    let from = self.docs[list].from.clone();
                    self.find(
                        &ORDER_UNKNOWN,
                        &from,
                        line,
                        &format!("sort_by = {}", crate::yaml::write::text(&by)),
                    );
                    continue;
                }
            };
            if words.reversed {
                keyed.reverse();
            }
            let names: Vec<Value> = keyed
                .iter()
                .filter_map(|at| Self::order_name(&self.docs[*at]))
                .collect();
            if names.is_empty() {
                continue;
            }
            self.docs[list]
                .words
                .push(Field::new("order", Value::List(names), None));
            if by == "weight" {
                for at in &keyed {
                    self.docs[*at].fields.retain(|f| f.name != "weight");
                }
            }
            let reversed = match words.reversed {
                true => ", the other way round, as its `paginate_reversed` asked",
                false => "",
            };
            let weight = match by.as_str() {
                "weight" => ", and `weight` was taken off them",
                _ => "",
            };
            self.note(format!(
                "Zola ordered the pages of {folder} by {what}{reversed}, so {index} lists them in \
                 that order with `order:`{weight}."
            ));
        }
        if !unordered.is_empty() {
            self.note(format!(
                "Zola gave the pages of {} no order of its own, as no `sort_by` asked for one, \
                 and Statac lists them newest first.",
                listed(&unordered)
            ));
        }
    }

    /// The later pages of each list Zola showed some pages of at a time.
    fn paging(&mut self) {
        let mut later_paths: Vec<String> = Vec::new();
        let mut any = false;
        let mut home_paged = None;
        for list in 0..self.docs.len() {
            let doc = &self.docs[list];
            let Some(size) = doc.list.paginate_by else {
                continue;
            };
            if doc.shape != Shape::List || doc.draft {
                continue;
            }
            // Only a list below the home page says `posts_per_page`: the
            // home page keeps its own list to one page.
            any |= !doc.dir.is_empty();
            let path = doc
                .list
                .paginate_path
                .clone()
                .unwrap_or_else(|| "page".into());
            let path = path.trim_matches('/').to_owned();
            let Some(base) = doc.address.clone() else {
                continue;
            };
            let (by, _) = doc.list.sort_by.clone().unwrap_or(("none".into(), None));
            let members: Vec<usize> = self
                .members(list)
                .into_iter()
                .filter(|at| {
                    let d = &self.docs[*at];
                    match by.as_str() {
                        "date" => d.date.is_some(),
                        "update_date" => d.updated.is_some(),
                        "title" | "title_bytes" => !d.title.is_empty(),
                        "weight" => d.weight.is_some(),
                        _ => true,
                    }
                })
                .collect();
            let pages = members.len().div_ceil(usize::try_from(size).unwrap_or(1));
            let home = doc.dir.is_empty();
            if home {
                home_paged = Some((size, pages > 1));
            } else if pages > 1 && !later_paths.contains(&path) {
                later_paths.push(path.clone());
            }
            let at = |n: usize| match path.is_empty() {
                true => format!("{base}{n}/"),
                false => format!("{base}{path}/{n}/"),
            };
            let first = at(1);
            match self.docs[list].words_only {
                true => {
                    self.import.addresses.push(first.clone());
                    let from = Value::List(vec![Value::Text(first)]);
                    (self.docs[list].words).push(Field::new("redirect_from", from, None));
                }
                false => self.docs[list].others.push(first),
            }
            for n in 2..=pages {
                self.import.addresses.push(at(n));
            }
        }
        self.import
            .settings
            .push(("posts_per_page".into(), Value::Number("0".into())));
        if let Some(path) = later_paths.first()
            && path != "page"
        {
            let later = match path.is_empty() {
                true => "{list}{n}/".to_owned(),
                false => format!("{{list}}{path}/{{n}}/"),
            };
            self.import
                .settings
                .push(("later_pages".into(), Value::Text(later)));
        }
        let mut note = "Zola shows all of a list's pages on one page, unless the list's own page \
                        gives `paginate_by`, so `posts_per_page: 0` in `statac.yaml` does the \
                        same"
            .to_owned();
        if any {
            note.push_str(
                ", and each list's page that gave it says `posts_per_page`; the first of its \
                 later pages, which Zola sent on to the list, is in the list's `redirect_from`",
            );
        }
        note.push('.');
        self.note(note);
        if let Some((size, later)) = home_paged {
            let mut note = format!(
                "The old site's home page showed its posts {size} to a page. {}",
                one_home_page(self.starter)
            );
            if later {
                note.push_str(
                    ", and nothing answers the old home page's later pages: they are listed \
                     under Addresses",
                );
            }
            note.push('.');
            self.note(note);
        }
        if later_paths.len() > 1 {
            self.note(format!(
                "The lists' `paginate_path` put their later pages at {}, and Statac puts every \
                 list's in one place, so only the first are kept: the others are listed under \
                 Addresses.",
                listed(&later_paths.iter().map(|p| code(p)).collect::<Vec<_>>())
            ));
        }
    }

    /// The feed each list and each language keeps, and the layout each
    /// list of posts hands its pages.
    fn feeds(&mut self) {
        let mut feed_addresses: Vec<String> = Vec::new();
        let mut kept: Vec<String> = Vec::new();
        let mut laid_out = Vec::new();
        let mut extra_names: Vec<String> = Vec::new();
        for at in 0..self.docs.len() {
            let doc = &self.docs[at];
            if doc.shape != Shape::List || doc.draft {
                continue;
            }
            let language = doc.language.clone();
            let top = !doc.dir.is_empty() && !doc.dir.contains('/');
            let members = self.members(at);
            let dated = members.iter().any(|m| self.docs[*m].date.is_some());
            let undated = members.iter().any(|m| self.docs[*m].date.is_none());
            let mut words = Vec::new();
            if self.starter && top && dated && !undated {
                words.push(Field::new("pages.layout", Value::Text("post".into()), None));
                laid_out.push(parent(&doc.to).to_owned());
            }
            let (_, names) = self.feeds_of(&language);
            if doc.list.feeds == Some(true)
                && !doc.dir.is_empty()
                && let Some(base) = doc.address.clone()
            {
                words.push(Field::new("feed", Value::Text(names[0].clone()), None));
                kept.push(format!("{base}{}", names[0]));
                for name in &names {
                    feed_addresses.push(format!("{base}{name}"));
                }
                extra_names.extend(names.iter().skip(1).cloned());
            }
            self.docs[at].words.extend(words);
        }
        for language in self.languages.clone() {
            let (on, names) = self.feeds_of(&language);
            if !on {
                continue;
            }
            let prefix = self.prefix(&language);
            let index = format!("{prefix}index.md");
            let word = Field::new("feed", Value::Text(names[0].clone()), None);
            match self.docs.iter_mut().find(|d| d.to == index) {
                Some(doc) => doc.words.push(word),
                None if prefix.is_empty() => self.import.words.push((index, word)),
                None => {
                    let front = super::yaml::fields([(word.name.as_str(), &word.value)]);
                    self.import
                        .written
                        .push((format!("content/{index}"), format!("---\n{front}---\n")));
                    self.import.addresses.push(format!("/{prefix}"));
                }
            }
            kept.push(format!("/{prefix}{}", names[0]));
            for name in &names {
                feed_addresses.push(format!("/{prefix}{name}"));
            }
            extra_names.extend(names.iter().skip(1).cloned());
        }
        if !feed_addresses.is_empty() {
            self.import
                .settings
                .push(("feed_ids_before".into(), Value::Text(self.today.iso_day())));
            let shown: Vec<String> = kept.iter().map(|a| code(a)).collect();
            let (feeds, are) = match (shown.len(), self.starter) {
                (1, true) => ("feed", "is now Statac's Atom feed"),
                (_, true) => ("feeds", "are each now Statac's Atom feed"),
                (1, false) => ("feed", "stays"),
                (_, false) => ("feeds", "each stay"),
            };
            let written = match self.starter {
                true => "written by `templates/feed.xml`, as `feed:` on each list's page says",
                false => {
                    "as `feed:` on each list's page says, built with `templates/feed.xml` once \
                     you add it"
                }
            };
            self.note(format!(
                "Zola's {feeds}, at {}, {are} at the same address, {written}, and \
                 `feed_ids_before` keeps each post's id the address Zola gave it, so feed \
                 readers see nothing new: see https://statac.dev/feeds/#entry-ids.",
                listed(&shown)
            ));
            self.import.addresses.extend(feed_addresses);
        }
        extra_names.sort();
        extra_names.dedup();
        if !extra_names.is_empty() {
            let shown: Vec<String> = extra_names.iter().map(|n| code(n)).collect();
            self.note(format!(
                "The settings' `feed_filenames` gave each list more than one feed, and Statac \
                 writes one: those named {} aren't made, and are listed under Addresses.",
                listed(&shown)
            ));
        }
        if !laid_out.is_empty() {
            let folders: Vec<String> = laid_out
                .iter()
                .map(|f| code(&format!("content/{f}/index.md")))
                .collect();
            self.note(format!(
                "{} {} `pages.layout: post`, so the pages beneath {}, every one dated, are built \
                 with the starter's template for posts.",
                listed(&folders),
                if folders.len() == 1 {
                    "hands"
                } else {
                    "each hand"
                },
                if folders.len() == 1 { "it" } else { "them" }
            ));
        }
    }

    /// The addresses of the pages Zola made for each term, the settings
    /// that keep those of tags, and what is said of the other lists of
    /// terms.
    fn terms(&mut self) {
        let mut addresses: Vec<String> = Vec::new();
        let mut slugs: Vec<(String, Value)> = Vec::new();
        let mut permalink: Option<String> = None;
        let mut term_pages: Vec<(String, String, String)> = Vec::new();
        let mut term_lists: Vec<(String, String)> = Vec::new();
        let mut feeds = false;
        let mut paged = false;
        for doc in self.docs.iter().filter(|d| !d.draft) {
            let declared = self.taxonomies_of(&doc.language);
            let prefix = self.prefix(&doc.language);
            for (plural, terms) in &doc.terms {
                if terms.is_empty() {
                    continue;
                }
                let Some(taxonomy) = declared.iter().find(|t| t.name == *plural) else {
                    continue;
                };
                // The page Zola made for each name in any other list, and
                // the one listing them all: the shared part makes each
                // name a tag.
                if !plural.eq_ignore_ascii_case("tags") {
                    let Some(list) = self
                        .s
                        .terms
                        .made(&taxonomy.name)
                        .filter(|_| taxonomy.render)
                    else {
                        continue;
                    };
                    let whole = format!("/{prefix}{list}/");
                    for term in terms {
                        let Some(zolas) = self.s.terms.made(term) else {
                            continue;
                        };
                        let page = (plural.clone(), term.clone(), format!("{whole}{zolas}/"));
                        if !term_pages.contains(&page) {
                            term_pages.push(page);
                        }
                    }
                    if !term_lists.iter().any(|(at, _)| *at == whole) {
                        term_lists.push((whole, plural.clone()));
                    }
                    continue;
                }
                feeds |= taxonomy.feed;
                paged |= taxonomy.paginate_by.is_some();
                if !taxonomy.render {
                    continue;
                }
                let Some(list) = self.s.terms.made(&taxonomy.name) else {
                    continue;
                };
                if list != "tags" && permalink.is_none() {
                    permalink = Some(format!("/{list}/{{slug}}/"));
                }
                let whole = format!("/{prefix}{list}/");
                if !term_lists.iter().any(|(at, _)| *at == whole) {
                    term_lists.push((whole.clone(), plural.clone()));
                }
                for term in terms {
                    let Some(zolas) = self.s.terms.made(term) else {
                        continue;
                    };
                    let address = format!("{whole}{zolas}/");
                    if !addresses.contains(&address) {
                        addresses.push(address);
                    }
                    if zolas != statac_slug::slugify(term)
                        && name_fault(&zolas).is_none()
                        && !slugs.iter().any(|(t, _)| t == term)
                    {
                        slugs.push((term.clone(), Value::Text(zolas)));
                    }
                }
            }
        }
        self.import.addresses.extend(addresses);
        if !slugs.is_empty() {
            self.import
                .settings
                .push(("tags.slugs".into(), Value::Fields(slugs)));
        }
        if let Some(pattern) = permalink {
            self.import
                .settings
                .push(("tags.permalink".into(), Value::Text(pattern)));
        }
        if feeds {
            self.note(
                "Zola made a feed for each tag, as the settings' `taxonomies` asked, which has no \
                 equal in Statac: a tag has a feed only where a page of yours speaks for it."
                    .to_owned(),
            );
        }
        if paged {
            self.note(
                "Zola showed the posts of each tag some at a time, as the settings' `taxonomies` \
                 asked, and Statac shows them all on one page, so the later pages of a tag aren't \
                 made."
                    .to_owned(),
            );
        }
        self.import.term_pages.extend(term_pages);
        self.import.term_lists.extend(term_lists);
    }

    /// What is said of each of Zola's words taken off pages.
    fn dropped_notes(&mut self) {
        let dropped = std::mem::take(&mut self.dropped);
        for (word, files) in dropped {
            let what: String = match word {
                "template" => left_behind(
                    "it named the old site's template for the page",
                    "the starter's templates take",
                    self.starter,
                ),
                "page_template" => left_behind(
                    "it named the old site's template for the list's pages",
                    "the starter's templates take",
                    self.starter,
                ),
                "transparent" => "Zola listed the folder's pages among those of the folder above \
                                  it, as Statac's list of a folder holds every post beneath it \
                                  already"
                    .into(),
                "render" => "with `render = false` Zola published the list's pages but no page \
                             for the list itself; `index_page: false` on the list's page does \
                             that in Statac"
                    .into(),
                "in_search_index" => "it said whether Zola's search index held the page, and \
                                      Statac makes no search index"
                    .into(),
                _ => "it put a link beside each heading, which a snippet of yours in \
                      `snippets/markdown/heading.html` can do, as \
                      https://statac.dev/snippets/#pieces-of-the-writing describes"
                    .into(),
            };
            let shown: Vec<String> = files.iter().map(|f| code(f)).collect();
            self.note(format!(
                "`{word}` was taken off {}, {}: {what}.",
                counted(files.len(), "page", "pages"),
                listed(&shown)
            ));
        }
    }

    /// How Zola read the writing, by what the settings, the templates and
    /// the writing show of the version that made the site. A sign of 0.23
    /// or later is taken first, since what only it reads would have
    /// stopped an earlier version; `raw` in the writing is the weakest,
    /// since an earlier version shows it as written, where nobody would
    /// have left it.
    fn reading(&self) -> Reading {
        let signs: Vec<writing::Signs> = (self.docs.iter())
            .map(|doc| writing::signs(&doc.body, &self.own))
            .collect();
        if self.s.skip_content_templating.is_some()
            || self.later
            || signs.iter().any(|signs| signs.component || signs.function)
        {
            Reading::Template
        } else if self.earlier || signs.iter().any(|signs| signs.shown) {
            Reading::Tags
        } else if signs.iter().any(|signs| signs.raw) {
            Reading::Template
        } else {
            Reading::Unknown
        }
    }

    /// Converts each page's writing, once every page's address is known,
    /// and hands the pages, the snippets standing for tags, and the
    /// settings to the import.
    fn finish(&mut self) {
        self.lists();
        self.orders();
        self.paging();
        self.feeds();
        self.terms();
        self.dropped_notes();
        let wrapping: HashSet<String> = self
            .docs
            .iter()
            .flat_map(|d| writing::paired(&d.body))
            .collect();
        let reading = self.reading();
        let base = self
            .import
            .address
            .as_deref()
            .map(|w| {
                let past = w.split_once("//").map_or(w, |(_, rest)| rest);
                past.split_once('/').map_or(String::new(), |(_, path)| {
                    format!("/{}", path.trim_matches('/'))
                })
            })
            .unwrap_or_default();
        let base = if base == "/" { String::new() } else { base };
        let docs = std::mem::take(&mut self.docs);
        let refs = |path: &str| {
            let doc = docs.iter().find(|d| d.rel == path && !d.draft)?;
            match doc.shape {
                Shape::List if doc.address.is_none() => {
                    let folder = parent(&doc.to);
                    let address = format!("/{}", statac_slug::tidied_path(folder));
                    Some(match address.ends_with('/') {
                        true => address,
                        false => format!("{address}/"),
                    })
                }
                _ => doc.address.clone(),
            }
        };
        let came: HashSet<String> = (self.import.files.iter())
            .filter_map(|file| file.from.strip_prefix("content/"))
            .map(str::to_owned)
            .collect();
        // As Zola reads it: a folder's `index.md` owns every file beneath
        // its folder, and a list's `_index.md` the files in its folder
        // itself, the nearest first.
        let beside = |path: &str| {
            if !came.contains(path) {
                return None;
            }
            let owns = |dir: &str, shape| {
                docs.iter()
                    .any(|d| d.shape == shape && parent(&d.rel) == dir)
            };
            let mut dir = parent(path);
            let mut nearest = true;
            while !dir.is_empty() {
                let rest = path[dir.len() + 1..].to_owned();
                if owns(dir, Shape::Bundle) {
                    return Some((format!("{dir}/index.md"), rest));
                }
                if nearest && owns(dir, Shape::List) {
                    return Some((format!("{dir}/_index.md"), rest));
                }
                nearest = false;
                dir = parent(dir);
            }
            None
        };
        let skip = self.s.skip_content_templating.clone().unwrap_or_default();
        let cx = writing::Context {
            reading,
            language: &self.s.default_language,
            main_language: &self.s.default_language,
            feeds: &self.s.feed_filenames,
            base: &base,
            refs: &refs,
            beside: &beside,
            own: &self.own,
            wrapping: &wrapping,
            emoji: self.s.render_emoji,
            starter: self.starter,
        };
        let mut calls = Vec::new();
        let mut written = Vec::new();
        for doc in &docs {
            let reading = match reading {
                Reading::Template if self.ignored(&skip, &doc.rel) => Reading::AsWritten,
                reading => reading,
            };
            let cx = writing::Context {
                reading,
                language: &doc.language,
                ..cx
            };
            let converted = writing::convert(&doc.body, doc.body_line, &doc.from, &cx);
            self.import.findings.extend(converted.findings);
            calls.extend(converted.calls);
            written.push((converted.writing, converted.called));
        }
        for (doc, (writing, called)) in docs.into_iter().zip(written) {
            if doc.words_only {
                for word in doc.words {
                    self.import.words.push((doc.to.clone(), word));
                }
                if let Some(address) = doc.address {
                    self.import.addresses.push(address);
                }
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
                    .filter(|(plural, _)| !plural.eq_ignore_ascii_case("tags"))
                    .collect(),
                address: doc.address,
                own_address: doc.own_address,
                other_addresses: doc.others,
                fences: Vec::new(),
                calls: called,
            });
        }
        self.snippets(&calls);
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
            let shown: String = names
                .iter()
                .map(|name| format!(" {name}: {{{{ {name} | default(\"\") }}}}"))
                .collect();
            let mut text = format!(
                "<!-- Not converted: this snippet stands for the old site's `{}`. Write it as \
                 https://statac.dev/snippets/ describes, or take its calls out: see \
                 IMPORT.md.{shown} -->\n",
                of[0].tag
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_dated_by_either_mark() {
        let day = Date::day_only(2026, 1, 2);
        assert_eq!(dated_name("2026-01-02-post-1"), Some((day, "post-1")));
        assert_eq!(dated_name("2026-01-02_post-1"), Some((day, "post-1")));
        assert_eq!(dated_name("2026-01-02"), None);
        assert_eq!(dated_name("2026-13-02-post-1"), None);
    }

    #[test]
    fn a_glob_matches_as_zola_reads_one() {
        assert!(glob_matches("*.xlsx", "data.xlsx"));
        assert!(!glob_matches("*.xlsx", "posts/data.xlsx"));
        assert!(glob_matches("**/drafts/*", "posts/drafts/post-1.md"));
        assert!(glob_matches("*.{xlsx,csv}", "data.csv"));
        assert!(glob_matches("post-?.md", "post-1.md"));
    }

    #[test]
    fn names_are_compared_as_read() {
        let mut names = vec!["Post 10", "post 2", "Éclair", "Post 1"];
        names.sort_by(|a, b| natural(a, b));
        assert_eq!(names, vec!["Éclair", "Post 1", "post 2", "Post 10"]);
    }
}
