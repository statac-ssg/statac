//! The reader for sites Eleventy made: a site with `.eleventy.js`,
//! `eleventy.config.js`, `.mjs` or `.cjs`, or a `package.json` that names
//! `@11ty/eleventy` among what it needs.
//!
//! Eleventy's settings are a program, which Statac never runs: what it
//! takes from them it reads in their text, in the plain forms most sites
//! write, and it says of each thing it took and each it assumed. Eleventy's
//! other rules are followed as its documentation gives them: which files
//! are pages, where each is published, what the data beside a page or a
//! folder hands it, and what its template tags do in the writing.

mod permalink;
mod settings;
mod writing;

use std::collections::HashSet;
use std::path::Path;

use super::common::{
    counted, feed_templates, feed_templates_said, glob_matches, joined, last, listed, mapping,
    names_in, parent, text_of,
};
use super::inside;
use super::report::code;
use super::{
    Failed, Field, File, Finding, FindingKind, Import, Left, Page, PageKind, Spot, Unread, Value,
};
use crate::address::Address;
use crate::date::{self, Date};
use crate::platform;
use crate::problem::{FeedFormat, Kind, Problem, Reason};
use crate::source::{Place, Sources};
use settings::{Folder, Settings};
use writing::Engine;

pub(super) static TEMPLATE_PAGE: FindingKind = FindingKind {
    heading: "Pages made by templates",
    sentence: "Each of these was the old site's templates at work, a page written in a template \
               language such as Nunjucks or one listing the old site's pages, so Statac didn't \
               bring it across: Statac makes a list page where the site asks for one, with an \
               `index.md` in a folder, `templates/home.html` for the home page, or \
               `tags.pages: true`, as https://statac.dev/lists/ describes.",
};

pub(super) static COLLECTIONS: FindingKind = FindingKind {
    heading: "Collections left out of tags",
    sentence: "Eleventy reads the `tags` in a folder's data file as the collections the folder's \
               pages belong to, as `\"tags\": \"posts\"` puts them in `collections.posts`, so \
               these didn't become tags. Statac needs no collection to list a folder's pages: \
               where one is also a tag readers should see, add it to the `tags` of each page.",
};

pub(super) static PARTS: FindingKind = FindingKind {
    heading: "Files that were parts of other pages",
    sentence: "Eleventy made no page of a file with `permalink: false`, and these had no title \
               and filled whole folders: content a template showed inside other pages, as \
               comments are, and not drafts. Statac left them behind, each folder listed under \
               Not converted, and they are counted here by the `tags` a template found them by. \
               To show them again, put what they say in a data file for a template to read: see \
               https://statac.dev/data/.",
};

pub(super) static HTML_WRITING: FindingKind = FindingKind {
    heading: "Pages written in HTML",
    sentence: "Each of these was written in HTML and came across as a Markdown file holding that \
               HTML, which Statac publishes unchanged: read each, since Markdown reads a line \
               indented by four spaces after a blank line as code.",
};

pub(super) static TAG_LEFT: FindingKind = FindingKind {
    heading: "Template tags left as written",
    sentence: "Statac's Markdown runs no template tags and these have no equal in it, so each appears on the page as written: write by hand what it made, \
               or call a snippet, as https://statac.dev/snippets/ describes.",
};

pub(super) static TAG_IN_CODE: FindingKind = FindingKind {
    heading: "Template tags inside code",
    sentence: "Eleventy ran these even inside code, and Statac shows code exactly as written, so \
               each now shows as written: write by hand what it made, where that was meant.",
};

pub(super) static INCLUDE: FindingKind = FindingKind {
    heading: "Includes now snippet calls",
    sentence: "Each include is now a call to a snippet of its name, and the snippet holds only a \
               comment saying it wasn't converted: write the snippet, as \
               https://statac.dev/snippets/ describes, or take the calls out.",
};

pub(super) static SNIPPET_CALLS: FindingKind = FindingKind {
    heading: "Shortcodes now snippet calls",
    sentence: "Each of these shortcodes of the old site's own is now a call to a snippet of its name, \
               and the snippet holds only a comment saying it wasn't converted: write the \
               snippet, as https://statac.dev/snippets/ describes, or take the calls out.",
};

pub(super) static EXCERPT_LEFT: FindingKind = FindingKind {
    heading: "Excerpt markers inside a line",
    sentence: "Statac ends an excerpt only at `<!--more-->` on a line of its own, so each of \
               these was left as written: put it on a line of its own, as `<!--more-->`.",
};

pub(super) static ATTRIBUTES: FindingKind = FindingKind {
    heading: "Attributes in braces",
    sentence: "The old site's Markdown read each of these as attributes of what stands before \
               it, and Statac reads them only at the end of a heading, so each shows as written: \
               take it out, or write the HTML it stood for.",
};

pub(super) static LINK_NOT_MADE: FindingKind = FindingKind {
    heading: "URLs no longer linked",
    sentence: "The old site's Markdown linked each of these, written without `www.` or `http`, \
               and Statac doesn't, so each shows as words: write it as a link, as \
               `<https://example.org>`, where one is meant.",
};

pub(super) static HTML_SHOWN: FindingKind = FindingKind {
    heading: "HTML the old site showed as text",
    sentence: "The old site's Markdown had HTML turned off and showed each of these as text, and \
               Statac publishes HTML in Markdown unchanged: write each as you mean it, or put a \
               backslash before its `<` to show it as text.",
};

pub(super) static FRONT_MATTER_UNREAD: FindingKind = FindingKind {
    heading: "Front matter Statac can't read",
    sentence: "Statac's reader stopped at this line in the front matter of each of these, so the \
               page didn't come across: put it right and import again.",
};

pub(super) static FRONT_MATTER_PROGRAM: FindingKind = FindingKind {
    heading: "Front matter written as a program",
    sentence: "Eleventy ran the front matter of each of these as JavaScript, and Statac runs none, \
               so the page came across with none: write its fields in YAML between `---` lines.",
};

pub(super) static DATE_FROM_FILE: FindingKind = FindingKind {
    heading: "Dates Eleventy took from the file",
    sentence: "Eleventy dated each of these pages by its file, as the words say, and Statac can't \
               know that day, so `date` was taken off and the page is dated by its name or not at \
               all: write the day it was published, as `2026-10-04`.",
};

pub(super) static ELEVENTYS_WORD: FindingKind = FindingKind {
    heading: "Eleventy's fields kept as custom fields",
    sentence: "Statac has no equal for each of these fields of Eleventy's own, so each came across \
               as a custom field and does nothing: take it out, or show it from a template.",
};

pub(super) static DATA_PROGRAM: FindingKind = FindingKind {
    heading: "Data written as a program",
    sentence: "Eleventy ran each of these JavaScript files for data, and Statac runs none, so what \
               it gave didn't come across: write it as JSON or YAML, in `data/` for the whole \
               site, or in the front matter of the pages it was for.",
};

pub(super) static DATA_UNREAD: FindingKind = FindingKind {
    heading: "Data Statac can't read",
    sentence: "Statac's reader stopped at this line of each of these, so what it gave didn't come \
               across: put it right and import again.",
};

pub(super) static DATA_LEFT: FindingKind = FindingKind {
    heading: "Data files left behind",
    sentence: "Statac reads data files written in YAML or JSON only, so these weren't copied: \
               write each as YAML or JSON in `data/`.",
};

pub(super) static ADDRESS_UNKNOWN: FindingKind = FindingKind {
    heading: "URLs Statac couldn't work out",
    sentence: "Each of these pages has a `permalink` holding a template tag Statac can't work out, \
               so its old URL wasn't kept and the page is at Statac's own: give it a \
               `permalink` of the URL the old site published it at.",
};

pub(super) static ADDRESS_A_FILE: FindingKind = FindingKind {
    heading: "URLs with no ending",
    sentence: "Eleventy wrote each of these pages as a file of exactly that name, and Statac \
               publishes a page whose URL has no ending in a folder of that name, as `/about/` \
               for `/about`: most hosts redirect from one to the other, but check that yours \
               does.",
};

pub(super) static REDIRECT_LEFT: FindingKind = FindingKind {
    heading: "Lines of `_redirects` not kept",
    sentence: "Each of these lines redirects to a URL no page of the old site has, holds a \
               pattern, or does more than redirect, so it isn't kept: keep it with your host, or \
               add its first URL to the `redirect_from` of the page it meant.",
};

pub(super) static COPY_UNREAD: FindingKind = FindingKind {
    heading: "Copies Statac doesn't make",
    sentence: "Each of these names a file or folder outside the old site's own folder, one \
               reached through a link, or one that isn't there, or a place outside the new site \
               to copy it to. Statac reads nothing outside the old site and follows no link in \
               it, so it copied nothing for it: bring across by hand what it named, if it is \
               wanted.",
};

pub(super) static SETTINGS_UNREAD: FindingKind = FindingKind {
    heading: "Settings Statac didn't read",
    sentence: "Eleventy's settings file is a program, and Statac ran none of it, so it doesn't \
               know what each of these did: read each, and write what the new site needs of it, \
               in its templates or snippets, as https://statac.dev/templates/ describes.",
};

/// Whether `folder` holds an Eleventy site.
pub(super) fn owns(folder: &Path) -> bool {
    settings::owns(folder)
}

/// The endings of the files Eleventy reads as templates, other than
/// Markdown and HTML.
const TEMPLATES: &[&str] = &[
    ".njk",
    ".liquid",
    ".webc",
    ".11ty.js",
    ".11ty.cjs",
    ".11ty.mjs",
    ".11ty.ts",
    ".11ty.jsx",
    ".11ty.tsx",
    ".hbs",
    ".mustache",
    ".ejs",
    ".haml",
    ".pug",
];

/// The words Eleventy reads in front matter that Statac has no equal for.
const ELEVENTYS_WORDS: &[&str] = &[
    "eleventyExcludeFromCollections",
    "eleventyComputed",
    "eleventyNavigation",
    "eleventyImport",
    "eleventyDataSchema",
    "eleventyAllowMissingExtension",
    "dynamicPermalink",
    "permalinkBypassOutputDir",
    "renderData",
    "pagination",
];

/// The words `date` takes that date a page by its file.
const DATES_FROM_FILE: &[&str] = &[
    "Last Modified",
    "Created",
    "git Last Modified",
    "git Created",
];

/// Reads the site in `folder`.
pub(super) fn read(folder: &Path, today: Date, given: Option<&str>) -> Result<Import, Box<Failed>> {
    let found = settings::FILES
        .iter()
        .find(|name| inside::a_file(folder, name));
    let s = match found {
        Some(name) => settings::read(folder, name).map_err(|error| {
            Box::<Failed>::from(Problem::nowhere(Kind::CantRead {
                path: folder.join(name),
                reason: Reason::of(&error),
            }))
        })?,
        None => Settings::default(),
    };
    let mut reader = Reader {
        folder,
        given,
        import: Import::default(),
        s,
        today,
        input: String::new(),
        output: "_site".into(),
        includes: "_includes".into(),
        data: "_data".into(),
        layouts: None,
        folder_notes: Vec::new(),
        ignores: Vec::new(),
        negated: 0,
        copied: HashSet::new(),
        copied_dirs: Vec::new(),
        copied_count: 0,
        redirects: None,
        docs: Vec::new(),
        dir_data: Vec::new(),
        file_data: Vec::new(),
        others: Vec::new(),
        words: Vec::new(),
        layouts_taken: 0,
        collections: Vec::new(),
        overrides: 0,
        unpublished: 0,
        undated: 0,
        html_pages: 0,
        addressed_by_name: Vec::new(),
        controls: (0, 0),
        plugin_feeds: Vec::new(),
        unread_feed: None,
    };
    reader.import.settings_file = reader.s.file.clone().unwrap_or_default();
    reader.folders();
    reader.ignore_files();
    reader.redirects_file();
    reader.copies()?;
    reader.outside("")?;
    let input = reader.input.clone();
    reader.walk(&input)?;
    reader.plugin_feeds()?;
    reader.finish();
    Ok(reader.import)
}

/// Data a folder's file or a page's file hands down, with the file it is
/// in.
struct Data {
    /// The folder, or the page's path without its ending, from the top of
    /// the input folder.
    at: String,
    fields: Vec<Field>,
    file: String,
}

/// A page as found, before its writing is converted.
struct Doc {
    /// Its path in the old folder.
    from: String,
    /// Its path from the top of the input folder.
    rel: String,
    html: bool,
    fields: Vec<Field>,
    body: String,
    body_line: usize,
}

/// A line of an ignore file, as a pattern from the top of the old folder.
struct Ignore {
    pattern: String,
    file: &'static str,
}

struct Reader<'a> {
    folder: &'a Path,
    /// The site's address as the author gave it, the settings giving none.
    given: Option<&'a str>,
    import: Import,
    s: Settings,
    today: Date,
    /// The folders, by their paths in the old folder, `""` for its top.
    input: String,
    output: String,
    includes: String,
    data: String,
    layouts: Option<String>,
    /// What is said of the folders.
    folder_notes: Vec<String>,
    ignores: Vec<Ignore>,
    /// How many lines of the ignore files bring back what another left
    /// out.
    negated: usize,
    /// The files copied as they are, by their paths in the old folder.
    copied: HashSet<String>,
    /// The folders copied whole, by their paths in the old folder.
    copied_dirs: Vec<String>,
    copied_count: usize,
    /// The `_redirects` file read.
    redirects: Option<String>,
    docs: Vec<Doc>,
    dir_data: Vec<Data>,
    file_data: Vec<Data>,
    /// The files of the input folder that are neither pages, data nor
    /// copied, by their paths in the old folder.
    others: Vec<String>,
    /// Each of Eleventy's words kept as a field, with where it is.
    words: Vec<(String, Vec<Spot>)>,
    layouts_taken: usize,
    /// The folders' data files whose `tags` were taken as collections.
    collections: Vec<String>,
    overrides: usize,
    unpublished: usize,
    undated: usize,
    html_pages: usize,
    /// The pages made by templates whose address is taken from the file's
    /// name, each by its path and its path in the input folder: a
    /// folder's data can give it another, or none.
    addressed_by_name: Vec<(String, String)>,
    /// The control characters taken out of front matter, and from how
    /// many pages.
    controls: (usize, usize),
    /// The address of each feed the settings show the RSS plugin made,
    /// with what it was written as, where the settings say.
    plugin_feeds: Vec<(String, Option<FeedFormat>)>,
    /// What the report says of the first `feedPlugin` call whose feed's
    /// URL Statac didn't take.
    unread_feed: Option<String>,
}

/// Whether `path` is `folder` or inside it.
fn under(path: &str, folder: &str) -> bool {
    folder.is_empty() || path == folder || path.starts_with(&format!("{folder}/"))
}

/// The date a name opens with, `2026-01-02-`, and the rest of the name.
fn dated_name(name: &str) -> Option<(Date, &str)> {
    let date = date::read(name.get(..10)?).ok()?;
    let rest = name[10..].strip_prefix('-')?;
    (!rest.is_empty()).then_some((date, rest))
}

/// The first day written in a file's path, as `2026-01-02`, in a folder's
/// name or the file's own, which is how Eleventy dates a page with no
/// `date`.
fn dated_path(rel: &str) -> Option<Date> {
    let bytes = rel.as_bytes();
    (0..bytes.len().saturating_sub(9)).find_map(|at| {
        let day = rel.get(at..at + 10)?;
        let shaped = day.bytes().enumerate().all(|(i, b)| match i {
            4 | 7 => b == b'-',
            _ => b.is_ascii_digit(),
        });
        match shaped {
            true => date::read(day).ok(),
            false => None,
        }
    })
}

/// A file read for its front matter.
struct Front {
    fields: Vec<Field>,
    body: String,
    body_line: usize,
    /// Whether the front matter was JavaScript, read as none.
    program: bool,
    /// How many control characters were taken out of it.
    controls: usize,
    /// The front matter, where Statac's YAML reader stopped at it: the
    /// page is then read as if it had none.
    unread: Option<Unread>,
}

/// Reads `text`'s front matter, written in YAML after `---` or `---yaml`,
/// up to a line of `---`; or gives the line it stopped at and that line's
/// text, where it isn't YAML.
fn front(text: &str, path: &str) -> Result<Front, (usize, String)> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let first = text.lines().next().unwrap_or_default().trim_end();
    let Some(language) = first.strip_prefix("---") else {
        return Ok(Front {
            fields: Vec::new(),
            body: text.to_owned(),
            body_line: 1,
            program: false,
            controls: 0,
            unread: None,
        });
    };
    let start = text.find('\n').map_or(text.len(), |n| n + 1);
    let mut at = start;
    let mut end = None;
    for line in text[start..].split_inclusive('\n') {
        if line.trim_end() == "---" {
            end = Some((at, at + line.len()));
            break;
        }
        at += line.len();
    }
    let (end, body) = end.ok_or_else(|| (1, first.to_owned()))?;
    // Eleventy publishes a control character in front matter as nothing,
    // so it is taken out here; the other readers leave one for Statac's
    // reader to refuse.
    let kept: String = (text[start..end].chars())
        .filter(|c| !c.is_control() || matches!(c, '\t' | '\n' | '\r'))
        .collect();
    let controls = text[start..end].chars().count() - kept.chars().count();
    let taken = end - start - kept.len();
    let text = format!("{}{kept}{}", &text[..start], &text[end..]);
    let (end, body) = (end - taken, body - taken);
    let line_of = |at: usize| text[..at.min(text.len())].matches('\n').count() + 1;
    let mut sources = Sources::default();
    let mut unread = None;
    let fields = match language.trim() {
        "" | "yaml" | "yml" => {
            let source = sources.add(path, text.clone());
            let span = crate::source::Span::new(start, end);
            match crate::yaml::read_at(&sources, Place { source, span }) {
                Ok(node) => mapping(&sources, &node),
                Err(problem) => {
                    let front = &text[start..end];
                    unread = Some(super::common::unread_at(&sources, &problem, path, front));
                    Vec::new()
                }
            }
        }
        "js" | "javascript" => {
            return Ok(Front {
                fields: Vec::new(),
                body: text[body..].to_owned(),
                body_line: line_of(body),
                program: true,
                controls,
                unread: None,
            });
        }
        _ => return Err((1, first.to_owned())),
    };
    Ok(Front {
        fields,
        body: text[body..].to_owned(),
        body_line: line_of(body),
        program: false,
        controls,
        unread,
    })
}

/// The first of a page's writing that reads the old site's lists of
/// pages, by its offset: a tag naming `collections.`, outside `raw`,
/// which holds tags written to be shown.
fn listing(body: &str) -> Option<(usize, String)> {
    let mut at = 0;
    while let Some(found) = body[at..].find("{%").or_else(|| body[at..].find("{{")) {
        let start = at + found;
        let close = match body[start..].starts_with("{%") {
            true => "%}",
            false => "}}",
        };
        let end = body[start..]
            .find(close)
            .map_or(body.len(), |i| start + i + 2);
        let tag = &body[start..end];
        let shown = super::liquid::tag_at(body, start)
            .filter(|(_, _, statement)| *statement)
            .and_then(|(_, inner, _)| match super::liquid::split_word(&inner).0 {
                "raw" => Some("endraw"),
                "verbatim" => Some("endverbatim"),
                _ => None,
            });
        if let Some(ends) = shown {
            at = super::liquid::find_statement(body, end, ends)
                .map_or(body.len(), |(start, length)| start + length);
            continue;
        }
        if tag.contains("collections.") || tag.contains("collections[") {
            return Some((start, tag.to_owned()));
        }
        at = end;
    }
    None
}

/// The engine a page's `templateEngineOverride` names: the first of the
/// languages it lists.
fn engine_named(name: &str) -> Engine {
    match name.split(',').next().unwrap_or_default().trim() {
        "liquid" => Engine::Liquid,
        "njk" => Engine::Nunjucks,
        _ => Engine::Not,
    }
}

/// Whether the writing names `name`, a file beside the page, as a link's
/// or a picture's address, or in HTML as a `src` or an `href`.
fn names_file(writing: &str, name: &str) -> bool {
    use pulldown_cmark::{Event, Parser, Tag};
    let fits = |address: &str| {
        let address = address.split(['#', '?']).next().unwrap_or_default();
        address.strip_prefix("./").unwrap_or(address) == name
    };
    let in_html = |html: &str| {
        ["src=", "href="].iter().any(|attribute| {
            html.match_indices(attribute).any(|(at, _)| {
                let value = &html[at + attribute.len()..];
                let quote = value.chars().next().filter(|c| matches!(c, '"' | '\''));
                let value = quote.map_or(value, |q| &value[q.len_utf8()..]);
                let end = value
                    .find(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '>'))
                    .unwrap_or(value.len());
                fits(&value[..end])
            })
        })
    };
    Parser::new_ext(writing, super::liquid::options()).any(|event| match event {
        Event::Start(Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. }) => fits(&dest_url),
        Event::Html(html) | Event::InlineHtml(html) => in_html(&html),
        _ => false,
    })
}

/// The format the RSS plugin's `type` names.
fn plugin_format(name: &str) -> Option<FeedFormat> {
    match name {
        "atom" => Some(FeedFormat::Atom),
        "rss" => Some(FeedFormat::Rss),
        "json" => Some(FeedFormat::Json),
        _ => None,
    }
}

/// A feed of `format`, in words.
fn a_feed(format: Option<FeedFormat>) -> &'static str {
    format.map_or("a feed", crate::wording::a_feed)
}

/// One sentence naming `posts`, the old URLs of the posts whose ids the
/// RSS plugin's Atom and JSON feeds escape twice.
fn escaped_twice(posts: &[String]) -> String {
    let shown: Vec<String> = posts.iter().map(|post| code(post)).collect();
    let (posts, each) = match shown.as_slice() {
        [one] => (format!("1 post, {one}, an id"), "it"),
        _ => (
            format!("{} posts, {}, ids", shown.len(), listed(&shown)),
            "each",
        ),
    };
    format!(
        "The old site's feed may have given {posts} escaped twice, as Eleventy's RSS plugin does \
         with a `'` or `&` in an Atom or JSON feed, so feed readers may show {each} once as new \
         after the move."
    )
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

    /// The path in the old folder of `written`, a folder the settings name
    /// from `base`, where it may be followed: inside the old folder as
    /// written, and reached through no link.
    fn follow(&self, base: &str, written: &str) -> Option<String> {
        let written = written.strip_prefix("./").unwrap_or(written);
        let written = match written.trim_end_matches('/') {
            "." => "",
            "" if !written.is_empty() => return None,
            written => written,
        };
        if !platform::stays_inside(written) {
            return None;
        }
        let path = joined(base, written);
        inside::reached(self.folder, &path).then_some(path)
    }

    /// The folders the settings' `dir` names, each followed only where it
    /// may be, and what is said of them.
    fn folders(&mut self) {
        let mut took = Vec::new();
        let mut own = Vec::new();
        let mut unplain = Vec::new();
        let file = self.import.settings_file.clone();
        let names = [
            ("input", self.s.input.clone(), "."),
            ("output", self.s.output.clone(), "_site"),
            ("includes", self.s.includes.clone(), "_includes"),
            ("data", self.s.data.clone(), "_data"),
        ];
        for (name, given, default) in names {
            let base = match name {
                "input" | "output" => String::new(),
                _ => self.input.clone(),
            };
            let followed = match &given {
                Some(Folder::Plain(given)) => match self.follow(&base, &given.value) {
                    Some(path) => {
                        took.push(format!("`{name}` as {}", code(&given.value)));
                        Some(path)
                    }
                    None => {
                        self.find(
                            &inside::FOLDER_UNREAD,
                            &file,
                            Some(given.line),
                            &format!("{name}: {}", given.value),
                        );
                        None
                    }
                },
                Some(Folder::Unplain(_)) => {
                    unplain.push(format!("`{name}`"));
                    None
                }
                None => None,
            };
            let path = match followed {
                Some(path) => path,
                None => {
                    own.push(format!("`{name}` as `{default}`"));
                    match name {
                        "input" => String::new(),
                        _ => joined(&base, default),
                    }
                }
            };
            match name {
                "input" => self.input = path,
                "output" => self.output = path,
                "includes" => self.includes = path,
                _ => self.data = path,
            }
        }
        if let Some(given) = self.s.layouts.clone() {
            match given {
                Folder::Plain(given) => match self.follow(&self.input.clone(), &given.value) {
                    Some(path) => {
                        took.push(format!("`layouts` as {}", code(&given.value)));
                        self.layouts = Some(path);
                    }
                    None => self.find(
                        &inside::FOLDER_UNREAD,
                        &file,
                        Some(given.line),
                        &format!("layouts: {}", given.value),
                    ),
                },
                Folder::Unplain(_) => unplain.push("`layouts`".into()),
            }
        }
        if !self.input.is_empty() && !inside::a_folder(self.folder, &self.input) {
            self.folder_notes.push(format!(
                "The settings name {} as the input folder, which isn't there, so Statac read the \
                 old site from its top.",
                code(&self.input)
            ));
            self.input = String::new();
            self.includes = "_includes".into();
            self.data = "_data".into();
        }
        let note = match (took.is_empty(), own.is_empty()) {
            (true, _) => format!(
                "The settings name no folder in a `dir` Statac can read, so Statac took \
                 Eleventy's own: {}.",
                listed(&own)
            ),
            (false, true) => format!("The settings' `dir` names {}.", listed(&took)),
            (false, false) => format!(
                "The settings' `dir` names {}; for the rest Statac took Eleventy's own: {}.",
                listed(&took),
                listed(&own)
            ),
        };
        self.folder_notes.push(note);
        if !unplain.is_empty() {
            self.folder_notes.push(format!(
                "The settings' `dir` gives {} as something other than text in quotes, which \
                 Statac can't know without running the settings, so it took Eleventy's own.",
                listed(&unplain)
            ));
        }
        self.import.output = self.output.clone();
        if inside::a_folder(self.folder, &self.output) && !self.output.is_empty() {
            self.import.built = Some(self.output.clone());
        }
    }

    /// Reads `.eleventyignore`, at the top and in the input folder, and
    /// `.gitignore` at the top unless the settings turn it off.
    fn ignore_files(&mut self) {
        let mut files = vec![(String::new(), ".eleventyignore")];
        if !self.input.is_empty() {
            files.push((self.input.clone(), ".eleventyignore"));
        }
        if self.s.no_gitignore.is_none() {
            files.push((String::new(), ".gitignore"));
        }
        for (base, name) in files {
            let path = joined(&base, name);
            if !inside::a_file(self.folder, &path) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(self.path(&path)) else {
                continue;
            };
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if line.starts_with('!') {
                    self.negated += 1;
                    continue;
                }
                let line = line.strip_prefix("./").unwrap_or(line);
                let line = line.trim_start_matches('/');
                self.ignores.push(Ignore {
                    pattern: joined(&base, line.trim_end_matches('/')),
                    file: name,
                });
            }
        }
    }

    /// The ignore file that leaves `path` out, where one does.
    fn ignored(&self, path: &str) -> Option<&'static str> {
        self.ignores
            .iter()
            .find(|ignore| {
                path == ignore.pattern
                    || path.starts_with(&format!("{}/", ignore.pattern))
                    || glob_matches(&ignore.pattern, path)
                    || glob_matches(&format!("{}/**", ignore.pattern), path)
            })
            .map(|ignore| ignore.file)
    }

    /// Every file beneath `path` in the old folder, links left, and told
    /// of where `tell`; hidden names, the built output and the packages
    /// left out where not.
    fn files_beneath(
        &mut self,
        path: &str,
        tell: bool,
        out: &mut Vec<String>,
    ) -> Result<(), Box<Failed>> {
        for name in names_in(&self.path(path))? {
            let inner = joined(path, &name);
            if !tell && (name.starts_with('.') || name == "node_modules" || inner == self.output) {
                continue;
            }
            match inside::metadata(self.folder, &inner) {
                Ok(found) if found.is_symlink() => {
                    if tell {
                        self.left(&inner, inside::why(self.folder, &inner));
                    }
                }
                Ok(found) if found.is_dir() => self.files_beneath(&inner, tell, out)?,
                Ok(_) => out.push(inner),
                Err(_) => {}
            }
        }
        Ok(())
    }

    /// Copies what each `addPassthroughCopy` names into `static/`, at the
    /// address Eleventy gave it.
    fn copies(&mut self) -> Result<(), Box<Failed>> {
        let file = self.import.settings_file.clone();
        let mut every: Option<Vec<String>> = None;
        for copy in self.s.copies.clone() {
            let written = copy.from.trim_start_matches("./");
            let glob = written.contains(['*', '?', '{', '[']);
            let to = match &copy.to {
                Some(to) => {
                    let to = to.trim_start_matches("./").trim_matches('/');
                    match platform::stays_inside(to) || to.is_empty() {
                        true => Some(to.to_owned()),
                        false => {
                            self.find(&COPY_UNREAD, &file, Some(copy.line), &copy.written);
                            continue;
                        }
                    }
                }
                None => None,
            };
            // Each file copied, with its path in the built site.
            let mut found: Vec<(String, String)> = Vec::new();
            let beneath_input =
                |path: &str, input: &str| match under(path, input) && !input.is_empty() {
                    true => path[input.len()..].trim_start_matches('/').to_owned(),
                    false => path.to_owned(),
                };
            if glob {
                if !platform::stays_inside(written.trim_end_matches('/')) {
                    self.find(&COPY_UNREAD, &file, Some(copy.line), &copy.written);
                    continue;
                }
                if every.is_none() {
                    let mut all = Vec::new();
                    self.files_beneath("", false, &mut all)?;
                    every = Some(all);
                }
                for path in every.iter().flatten() {
                    if glob_matches(written, path) {
                        let target = match &to {
                            Some(to) => joined(to, last(path)),
                            None => beneath_input(path, &self.input),
                        };
                        found.push((path.clone(), target));
                    }
                }
            } else {
                let Some(from) = self.follow("", written) else {
                    self.find(&COPY_UNREAD, &file, Some(copy.line), &copy.written);
                    continue;
                };
                if inside::a_file(self.folder, &from) {
                    let target = match &to {
                        Some(to) => to.clone(),
                        None => beneath_input(&from, &self.input),
                    };
                    found.push((from.clone(), target));
                } else if inside::a_folder(self.folder, &from) {
                    let mut beneath = Vec::new();
                    self.files_beneath(&from, true, &mut beneath)?;
                    self.copied_dirs.push(from.clone());
                    for path in beneath {
                        let target = match &to {
                            Some(to) => joined(to, path[from.len()..].trim_start_matches('/')),
                            None => beneath_input(&path, &self.input),
                        };
                        found.push((path, target));
                    }
                } else {
                    self.find(&COPY_UNREAD, &file, Some(copy.line), &copy.written);
                    continue;
                }
            }
            for (path, target) in found {
                self.copy(path, &target);
            }
        }
        Ok(())
    }

    fn copy(&mut self, from: String, to: &str) {
        if from == self.import.settings_file
            || Some(&from) == self.redirects.as_ref()
            || !self.copied.insert(from.clone())
        {
            return;
        }
        self.copied_count += 1;
        self.import.files.push(File {
            from,
            to: format!("static/{to}"),
        });
    }

    /// Finds the `_redirects` file, in the input folder or at the top.
    fn redirects_file(&mut self) {
        let mut places = vec![joined(&self.input, "_redirects")];
        if !self.input.is_empty() {
            places.push("_redirects".into());
        }
        self.redirects = places
            .into_iter()
            .find(|path| inside::a_file(self.folder, path));
    }

    /// Whether `path`, in the old folder, is a file Statac read for what
    /// it says rather than as a page.
    fn read_as_settings(&self, path: &str) -> bool {
        Some(path) == self.s.file.as_deref()
            || Some(path) == self.redirects.as_deref()
            || path == "package.json"
            || path == ".gitignore"
            || path == ".eleventyignore"
            || path == joined(&self.input, ".eleventyignore")
    }

    /// Tells of what is in the folder `path`, outside the input folder.
    fn outside(&mut self, path: &str) -> Result<(), Box<Failed>> {
        if self.input.is_empty() {
            return Ok(());
        }
        for name in names_in(&self.path(path))? {
            let inner = joined(path, &name);
            let Ok(found) = inside::metadata(self.folder, &inner) else {
                continue;
            };
            let dir = found.is_dir();
            let shown = match dir {
                true => format!("{inner}/"),
                false => inner.clone(),
            };
            if found.is_symlink() {
                self.left(&inner, inside::why(self.folder, &inner));
                continue;
            }
            if inner == self.input {
                continue;
            }
            if dir && under(&self.input, &inner) {
                self.outside(&inner)?;
                continue;
            }
            if self.read_as_settings(&inner)
                || self.copied.contains(&inner)
                || (dir && self.copied.iter().any(|copied| under(copied, &inner)))
            {
                continue;
            }
            let what = if inner == self.output {
                "the old site's built output, read only to check the URLs".to_owned()
            } else if name == "node_modules" {
                "the packages Eleventy and its plugins were installed from".to_owned()
            } else if settings::FILES.contains(&inner.as_str()) {
                "another settings file, which Eleventy reads only where the one before it isn't \
                 there"
                    .to_owned()
            } else {
                format!(
                    "a {} outside the input folder, {}, which Eleventy doesn't publish",
                    if dir { "folder" } else { "file" },
                    code(&format!("{}/", self.input))
                )
            };
            self.left(&shown, &what);
        }
        Ok(())
    }

    /// Reads the folder `path` of the input folder.
    fn walk(&mut self, path: &str) -> Result<(), Box<Failed>> {
        for name in names_in(&self.path(path))? {
            let inner = joined(path, &name);
            let Ok(found) = inside::metadata(self.folder, &inner) else {
                continue;
            };
            let dir = found.is_dir();
            let shown = match dir {
                true => format!("{inner}/"),
                false => inner.clone(),
            };
            if found.is_symlink() {
                self.left(&inner, inside::why(self.folder, &inner));
                continue;
            }
            if self.read_as_settings(&inner) || (dir && self.copied_dirs.contains(&inner)) {
                continue;
            }
            if inner == self.output {
                self.left(
                    &shown,
                    "the old site's built output, read only to check the URLs",
                );
                continue;
            }
            if name == "node_modules" {
                self.left(
                    &shown,
                    "the packages Eleventy and its plugins were installed from",
                );
                continue;
            }
            if settings::FILES.contains(&inner.as_str()) {
                self.left(
                    &shown,
                    "another settings file, which Eleventy reads only where the one before it \
                     isn't there",
                );
                continue;
            }
            if name.starts_with('.') {
                let what = match dir {
                    true => "a hidden folder, which Eleventy doesn't read",
                    false => "a hidden file, which Eleventy doesn't read",
                };
                self.left(&shown, what);
                continue;
            }
            if let Some(file) = self.ignored(&inner) {
                let what = format!(
                    "a {} {} leaves out, which Eleventy doesn't read",
                    if dir { "folder" } else { "file" },
                    code(file)
                );
                self.left(&shown, &what);
                continue;
            }
            if dir && inner == self.includes {
                self.left(
                    &shown,
                    "the old site's includes and layouts; each include the content calls has a \
                     snippet of its name standing for it",
                );
                continue;
            }
            if dir && Some(&inner) == self.layouts.as_ref() {
                self.left(&shown, "the old site's layouts");
                continue;
            }
            if dir && inner == self.data {
                self.data_folder(&inner)?;
                continue;
            }
            match dir {
                true => self.walk(&inner)?,
                false => self.file(&inner)?,
            }
        }
        Ok(())
    }

    /// The path of `path`, in the old folder, from the top of the input
    /// folder.
    fn rel(&self, path: &str) -> String {
        match self.input.is_empty() {
            true => path.to_owned(),
            false => path[self.input.len()..].trim_start_matches('/').to_owned(),
        }
    }

    /// Reads the data folder at `path`: JSON and YAML copied to `data/`.
    fn data_folder(&mut self, path: &str) -> Result<(), Box<Failed>> {
        for name in names_in(&self.path(path))? {
            let inner = joined(path, &name);
            let Ok(found) = inside::metadata(self.folder, &inner) else {
                continue;
            };
            if found.is_symlink() {
                self.left(&inner, inside::why(self.folder, &inner));
            } else if found.is_dir() {
                self.data_folder(&inner)?;
            } else {
                let ending = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase());
                match ending.as_deref() {
                    Some("json" | "yaml" | "yml") => {
                        let to =
                            format!("data/{}", inner[self.data.len()..].trim_start_matches('/'));
                        self.import.files.push(File { from: inner, to });
                    }
                    Some("js" | "cjs" | "mjs") => self.find(&DATA_PROGRAM, &inner, None, ""),
                    _ => self.find(&DATA_LEFT, &inner, None, ""),
                }
            }
        }
        Ok(())
    }

    /// Reads one file of the input folder, at `path` in the old folder.
    fn file(&mut self, path: &str) -> Result<(), Box<Failed>> {
        if self.copied.contains(path) {
            return Ok(());
        }
        let rel = self.rel(path);
        let name = last(path);
        let folder = last(parent(&rel));
        if [".11tydata.js", ".11tydata.cjs", ".11tydata.mjs"]
            .iter()
            .any(|ending| name.ends_with(ending))
        {
            self.find(&DATA_PROGRAM, path, None, "");
            return Ok(());
        }
        let data_stem = name.strip_suffix(".11tydata.json").or_else(|| {
            let stem = name.strip_suffix(".json")?;
            let mut endings = ["md", "html"]
                .into_iter()
                .chain(TEMPLATES.iter().map(|t| &t[1..]));
            let beside = endings.any(|ending| {
                inside::a_file(
                    self.folder,
                    &joined(parent(path), &format!("{stem}.{ending}")),
                )
            });
            (stem == folder || beside).then_some(stem)
        });
        if let Some(stem) = data_stem {
            return self.data_file(path, &rel, stem);
        }
        if name.ends_with(".md") {
            return self.page(path, false);
        }
        if name.ends_with(".html") {
            return self.page(path, true);
        }
        if let Some(ending) = TEMPLATES.iter().find(|ending| name.ends_with(*ending)) {
            return self.template(path, &rel, ending);
        }
        self.others.push(path.to_owned());
        Ok(())
    }

    /// Reads a file of data beside a page, or a folder's own, as JSON.
    fn data_file(&mut self, path: &str, rel: &str, stem: &str) -> Result<(), Box<Failed>> {
        let text = std::fs::read_to_string(self.path(path)).map_err(|error| {
            Box::<Failed>::from(Problem::nowhere(Kind::CantRead {
                path: self.path(path),
                reason: Reason::of(&error),
            }))
        })?;
        let mut sources = Sources::default();
        let source = sources.add(path, text.clone());
        let node = match crate::yaml::read_json(&sources, source) {
            Ok(node) => node,
            Err(problem) => {
                let line = problem.place.map_or(1, |place| {
                    sources.line_number(place.source, place.span.start as usize)
                });
                let shown = text.lines().nth(line - 1).unwrap_or_default().to_owned();
                self.find(&DATA_UNREAD, path, Some(line), &shown);
                return Ok(());
            }
        };
        let fields = mapping(&sources, &node);
        let folder = parent(rel);
        let data = match stem == last(folder) && !folder.is_empty() {
            true => Data {
                at: folder.to_owned(),
                fields,
                file: path.to_owned(),
            },
            false => {
                self.file_data.push(Data {
                    at: joined(folder, stem),
                    fields,
                    file: path.to_owned(),
                });
                return Ok(());
            }
        };
        self.dir_data.push(data);
        Ok(())
    }

    /// The address Eleventy gives a file by its name, at `rel` from the
    /// top of the input folder, with its ending taken off as `stem`.
    fn default_address(rel: &str, stem: &str) -> String {
        let dir = parent(rel);
        let at = match stem == "index" || (!dir.is_empty() && stem == last(dir)) {
            true => dir.to_owned(),
            false => joined(dir, stem),
        };
        match at.is_empty() {
            true => "/".to_owned(),
            false => format!("/{at}/"),
        }
    }

    /// Tells of a page made by a template, and keeps where it was
    /// published; or keeps where a feed a template made was published.
    fn template(&mut self, path: &str, rel: &str, ending: &str) -> Result<(), Box<Failed>> {
        let stem = &last(rel)[..last(rel).len() - ending.len()];
        // A template written in JavaScript gives its address in code.
        let mut address = match ending.starts_with(".11ty.") {
            true => None,
            false => Some(Self::default_address(rel, stem)),
        };
        let mut by_name = address.is_some();
        if let Ok(text) = std::fs::read_to_string(self.path(path))
            && let Ok(front) = front(&text, path)
            && let Some(field) = front.fields.iter().find(|f| f.name == "permalink")
        {
            by_name = false;
            address = match &field.value {
                Value::Text(text) if !text.contains(['{', '}']) => {
                    Some(Self::settled_address(text).0)
                }
                _ => None,
            };
        }
        if by_name {
            self.addressed_by_name
                .push((path.to_owned(), rel.to_owned()));
        }
        match address {
            Some(address) if self.built_a_feed(&address) => {
                self.import.feeds.push((address, path.to_owned()));
            }
            address => {
                self.find(&TEMPLATE_PAGE, path, None, "");
                (self.import.templated).extend(address.map(|a| (a, path.to_owned())));
            }
        }
        Ok(())
    }

    /// Whether the old built output holds a feed at `address`: a file that
    /// is RSS, Atom or a JSON Feed.
    fn built_a_feed(&self, address: &str) -> bool {
        let Some(built) = &self.import.built else {
            return false;
        };
        let file = address.trim_start_matches('/');
        if file.is_empty() || file.ends_with('/') || !platform::stays_inside(file) {
            return false;
        }
        let path = joined(built, file);
        if !inside::a_file(self.folder, &path) {
            return false;
        }
        let Ok(bytes) = std::fs::read(self.path(&path)) else {
            return false;
        };
        crate::feeds::format::of(&String::from_utf8_lossy(&bytes)).is_some()
    }

    /// The feeds the settings show the RSS plugin made, and the feeds in
    /// the old built output that no file of the old site makes, the work
    /// of the plugin or of templates Statac didn't find.
    fn plugin_feeds(&mut self) -> Result<(), Box<Failed>> {
        for feed in self.s.feeds.clone() {
            let address = (feed.path.as_ref())
                .map(|path| format!("/{}", path.trim().trim_start_matches('/')))
                .filter(|address| Address::read(address).is_ok_and(|address| address.is_file()));
            let Some(address) = address else {
                let call = format!(
                    "The settings' {}, on line {},",
                    code(&feed.written),
                    feed.line
                );
                let why = match &feed.path {
                    Some(path) => {
                        self.s.unread.push((feed.line, feed.written.clone()));
                        format!(
                            "{call} gives its feed the URL {}, which Statac can't give a feed",
                            code(path)
                        )
                    }
                    None => format!("{call} doesn't give its feed's URL as text in quotes"),
                };
                self.unread_feed.get_or_insert(why);
                continue;
            };
            if self.plugin_feeds.iter().all(|(had, _)| *had != address) {
                let kind = feed.kind.as_deref().and_then(plugin_format);
                self.plugin_feeds.push((address, kind));
            }
        }
        let Some(built) = self.import.built.clone() else {
            return Ok(());
        };
        let mut files = Vec::new();
        self.files_beneath(&built, false, &mut files)?;
        for path in files {
            let rel = path[built.len()..].trim_start_matches('/');
            let ending = rel.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase());
            if !matches!(ending.as_deref(), Some("xml" | "rss" | "atom" | "json")) {
                continue;
            }
            let address = format!("/{rel}");
            let copied = format!("static/{rel}");
            let made = (self.import.feeds.iter())
                .chain(&self.import.templated)
                .any(|(made, _)| *made == address)
                || self.plugin_feeds.iter().any(|(made, _)| *made == address);
            if made || self.import.files.iter().any(|file| file.to == copied) {
                continue;
            }
            let Ok(bytes) = std::fs::read(self.path(&path)) else {
                continue;
            };
            if crate::feeds::format::of(&String::from_utf8_lossy(&bytes)).is_some() {
                self.import.feeds.push((address, path));
            }
        }
        Ok(())
    }

    /// A `permalink` written out as an address: opening with `/`, a
    /// folder's `index.html` as the folder; and whether Eleventy wrote it
    /// as a file with no ending, which Statac publishes as a folder.
    fn settled_address(written: &str) -> (String, bool) {
        let address = format!("/{}", written.trim().trim_start_matches('/'));
        let address = match address.strip_suffix("index.html") {
            Some(folder) if folder.ends_with('/') => folder.to_owned(),
            _ => address,
        };
        if address.ends_with('/') || last(&address).contains('.') {
            return (address, false);
        }
        (format!("{address}/"), true)
    }

    /// Reads one page, Markdown or HTML.
    fn page(&mut self, path: &str, html: bool) -> Result<(), Box<Failed>> {
        let bytes = std::fs::read(self.path(path)).map_err(|error| {
            Box::<Failed>::from(Problem::nowhere(Kind::CantRead {
                path: self.path(path),
                reason: Reason::of(&error),
            }))
        })?;
        let Ok(text) = String::from_utf8(bytes) else {
            self.left(path, "a file that isn't UTF-8 text");
            return Ok(());
        };
        let mut front = match front(&text, path) {
            Ok(front) => front,
            Err((line, text)) => {
                self.find(&FRONT_MATTER_UNREAD, path, Some(line), &text);
                return Ok(());
            }
        };
        let unread = front.unread.take();
        let as_written = unread.is_some();
        self.import.unread.extend(unread);
        if front.controls > 0 {
            self.controls.0 += front.controls;
            self.controls.1 += 1;
        }
        if front.program {
            let first = text
                .lines()
                .next()
                .unwrap_or_default()
                .trim_end()
                .to_owned();
            self.find(&FRONT_MATTER_PROGRAM, path, Some(1), &first);
        }
        let rel = self.rel(path);
        let paged = front.fields.iter().find(|f| f.name == "pagination");
        let listed = listing(&front.body).filter(|_| !as_written);
        if paged.is_some() || listed.is_some() {
            let (line, shown) = match (paged, listed) {
                (Some(field), _) => (field.line, "pagination".to_owned()),
                (None, Some((at, tag))) => (
                    Some(front.body_line + front.body[..at].matches('\n').count()),
                    tag,
                ),
                (None, None) => (None, String::new()),
            };
            self.find(&TEMPLATE_PAGE, path, line, &shown);
            let stem = last(&rel).rsplit_once('.').map_or(last(&rel), |(s, _)| s);
            let address = match front.fields.iter().find(|f| f.name == "permalink") {
                Some(Field {
                    value: Value::Text(text),
                    ..
                }) if !text.contains(['{', '}']) => Some(Self::settled_address(text).0),
                Some(_) => None,
                None => {
                    self.addressed_by_name.push((path.to_owned(), rel.clone()));
                    Some(Self::default_address(&rel, stem))
                }
            };
            if let Some(address) = address {
                self.import.templated.push((address, path.to_owned()));
            }
            return Ok(());
        }
        if html {
            self.html_pages += 1;
        }
        self.docs.push(Doc {
            from: path.to_owned(),
            rel,
            html,
            fields: front.fields,
            body: front.body,
            body_line: front.body_line,
        });
        Ok(())
    }
}

/// What a page comes to, once the data handed down to it is known.
struct Made {
    to: String,
    kind: PageKind,
    fields: Vec<Field>,
    address: Option<String>,
    own_address: bool,
    engine: Engine,
    /// Whether `permalink: false` kept Eleventy from publishing it.
    unpublished: bool,
    /// The page's `categories`.
    categories: Vec<String>,
}

/// Where a folder's data is written in the new site.
#[derive(Clone, PartialEq, Debug)]
enum Placed {
    /// In `pages.` lines of the folder's own page, by its path under
    /// `content/`; the page there came across where it is true.
    Index(String, bool),
    /// In each page's own front matter.
    EachPage,
}

/// The fields of a folder's data that can't be handed down as `pages.`
/// lines, being each page's own in Statac or Eleventy's.
fn each_pages_own(name: &str, value: &Value) -> bool {
    matches!(name, "title" | "date" | "redirect_from")
        || super::write::collides(statacs_name(name), value)
}

/// The name Statac gives a field of Eleventy's, where it has one.
fn statacs_name(name: &str) -> &str {
    match name {
        "description" => "summary",
        "modified" | "lastmod" => "updated",
        other => other,
    }
}

impl Reader<'_> {
    /// Where each folder's data goes.
    fn placed(&self, dir: &str, tos: &[String]) -> Placed {
        let index = joined(dir, "index.md");
        match tos.contains(&index) {
            true => Placed::Index(index, true),
            false if !dir.contains('/') => Placed::Index(index, false),
            false => Placed::EachPage,
        }
    }

    /// The folders' data that reaches a page at `rel`, nearest first.
    fn reaching(&self, rel: &str) -> Vec<&Data> {
        let mut found: Vec<&Data> = self
            .dir_data
            .iter()
            .filter(|data| under(parent(rel), &data.at))
            .collect();
        found.sort_by_key(|data| std::cmp::Reverse(data.at.len()));
        found
    }

    /// Tells of one of Eleventy's words kept as a field.
    fn eleventys_word(&mut self, name: &str, file: &str, line: Option<usize>, value: &Value) {
        let spot = Spot {
            file: file.to_owned(),
            line,
            text: format!("{name}: {}", super::write::shown_value(value)),
            times: None,
        };
        match self.words.iter_mut().find(|(word, _)| word == name) {
            Some((_, spots)) => spots.push(spot),
            None => self.words.push((name.to_owned(), vec![spot])),
        }
    }

    /// Makes the page `doc` of the new site: its place, its kind, its
    /// fields in Statac's words, and the address Eleventy gave it.
    fn made(&mut self, at: usize, placements: &[(String, Placed)]) -> Made {
        let doc = &self.docs[at];
        let rel = doc.rel.clone();
        let from = doc.from.clone();
        let html = doc.html;
        let dir = parent(&rel).to_owned();
        let name = last(&rel);
        let stem = name
            .rsplit_once('.')
            .map_or(name, |(stem, _)| stem)
            .to_owned();
        let index = stem == "index";
        let called = dated_name(&stem).map_or(stem.as_str(), |(_, rest)| rest);
        let called = called.to_owned();
        let name_date = dated_path(&rel);
        // A file named as its folder is published at the folder, as its
        // own page is.
        let to = match index || (!dir.is_empty() && stem == last(&dir)) {
            true => joined(&dir, "index.md"),
            false => joined(&dir, &format!("{called}.md")),
        };
        // Each field with the file it is written in: the page's own, then
        // the data beside it, then each folder's nearest first.
        let own: Vec<(Field, String)> = doc
            .fields
            .iter()
            .cloned()
            .map(|field| (field, from.clone()))
            .collect();
        let beside: Vec<(Field, String)> = self
            .file_data
            .iter()
            .filter(|data| data.at == joined(&dir, &stem))
            .flat_map(|data| data.fields.iter().cloned().map(|f| (f, data.file.clone())))
            .collect();
        let handed: Vec<(Field, String, String)> = self
            .reaching(&rel)
            .into_iter()
            .flat_map(|data| {
                (data.fields.iter().cloned()).map(|f| (f, data.file.clone(), data.at.clone()))
            })
            .collect();
        let all: Vec<(&Field, &str)> = (own.iter().map(|(f, file)| (f, file.as_str())))
            .chain(beside.iter().map(|(f, file)| (f, file.as_str())))
            .chain(handed.iter().map(|(f, file, _)| (f, file.as_str())))
            .collect();
        let get = |name: &str| {
            all.iter()
                .find(|(f, _)| f.name == name)
                .map(|(f, file)| ((*f).clone(), file.to_string()))
        };
        // A folder's data names collections with `tags`, which become no
        // tags; the page's own and the data beside it gather, the data's
        // first.
        for (field, file, _) in &handed {
            if field.name == "tags" && !self.collections.contains(file) {
                self.collections.push(file.clone());
                let shown = format!("tags: {}", super::write::shown_value(&field.value));
                self.find(&COLLECTIONS, file, field.line, &shown);
            }
        }
        let mut tags: Vec<String> = Vec::new();
        let mut categories: Vec<String> = Vec::new();
        for (field, _) in beside
            .iter()
            .map(|(f, file)| (f, file))
            .chain(own.iter().map(|(f, file)| (f, file)))
        {
            if field.name != "tags" {
                continue;
            }
            let names = match &field.value {
                Value::List(items) => items.iter().filter_map(text_of).collect(),
                other => text_of(other).into_iter().collect::<Vec<_>>(),
            };
            for name in names {
                if !tags.contains(&name) {
                    tags.push(name);
                }
            }
        }
        let engine = match get("templateEngineOverride").and_then(|(f, _)| text_of(&f.value)) {
            Some(named) => {
                self.overrides += 1;
                engine_named(&named)
            }
            None => {
                let given = match html {
                    true => &self.s.html_engine,
                    false => &self.s.engine,
                };
                match given.as_ref().map(|g| g.value.as_deref()) {
                    None | Some(Some("liquid")) => Engine::Liquid,
                    Some(Some("njk")) => Engine::Nunjucks,
                    _ => Engine::Not,
                }
            }
        };
        // What the page's own front matter and the folders that can't
        // hand down give it, in Statac's words.
        let mine = |data_at: &str, field: &Field| {
            placements
                .iter()
                .find(|(at, _)| at == data_at)
                .is_none_or(|(_, placed)| match placed {
                    Placed::EachPage => true,
                    Placed::Index(path, _) => {
                        *path == to || each_pages_own(&field.name, &field.value)
                    }
                })
        };
        let mut fields: Vec<Field> = Vec::new();
        let mut written: Vec<(Field, String)> = Vec::new();
        for (field, file) in own.iter().chain(beside.iter()) {
            if !written.iter().any(|(f, _)| f.name == field.name) {
                written.push((field.clone(), file.clone()));
            }
        }
        for (field, file, data_at) in &handed {
            if mine(data_at, field) && !written.iter().any(|(f, _)| f.name == field.name) {
                written.push((field.clone(), file.clone()));
            }
        }
        let has = |name: &str| written.iter().any(|(f, _)| f.name == name);
        let has_summary = has("summary");
        let has_updated = has("updated");
        let mut summary_written = has_summary;
        let mut updated_written = has_updated;
        let mut tags_written = false;
        let mut layout = false;
        for (field, file) in &written {
            let line = match *file == from {
                true => field.line,
                false => None,
            };
            match field.name.as_str() {
                "layout" => layout = true,
                "permalink" | "templateEngineOverride" => {}
                // The site's own list of terms, which the shared part
                // makes tags of.
                "categories" => {
                    let names = match &field.value {
                        Value::List(items) => items.iter().filter_map(text_of).collect(),
                        other => text_of(other).into_iter().collect::<Vec<_>>(),
                    };
                    for name in names {
                        if !name.trim().is_empty() && !categories.contains(&name) {
                            categories.push(name);
                        }
                    }
                }
                "tags" => {
                    if !tags_written && !tags.is_empty() {
                        let list = tags.iter().cloned().map(Value::Text).collect();
                        fields.push(Field::new("tags", Value::List(list), line));
                        tags_written = true;
                    }
                }
                "date" => match text_of(&field.value) {
                    Some(text) if DATES_FROM_FILE.contains(&text.trim()) => {
                        let shown = format!("date: {}", text.trim());
                        self.find(&DATE_FROM_FILE, file, field.line, &shown);
                    }
                    _ => fields.push(Field::new("date", field.value.clone(), line)),
                },
                "updated" | "modified" | "lastmod" if !updated_written => {
                    fields.push(Field::new("updated", field.value.clone(), line));
                    updated_written = true;
                }
                "description" if !summary_written => {
                    fields.push(Field::new("summary", field.value.clone(), line));
                    summary_written = true;
                }
                name => {
                    if ELEVENTYS_WORDS.contains(&name) {
                        let shown_line = field.line;
                        self.eleventys_word(name, file, shown_line, &field.value);
                    }
                    fields.push(Field::new(name, field.value.clone(), line));
                }
            }
        }
        if !tags_written && !tags.is_empty() {
            let list = tags.iter().cloned().map(Value::Text).collect();
            fields.push(Field::new("tags", Value::List(list), None));
        }
        if layout {
            self.layouts_taken += 1;
        }
        let written_date = fields
            .iter()
            .find(|f| f.name == "date")
            .and_then(|f| text_of(&f.value));
        let date = written_date
            .as_deref()
            .and_then(super::write::date_of)
            .or(name_date);
        if written_date.is_none()
            && let Some(day) = name_date
        {
            let at = usize::from(fields.first().is_some_and(|f| f.name == "title"));
            fields.insert(at, Field::new("date", Value::Text(day.iso_day()), None));
        }
        if date.is_none() {
            self.undated += 1;
        }
        // The address, by `permalink` wherever it is given.
        let permalink = get("permalink");
        let plain: Vec<(String, String)> = all
            .iter()
            .filter_map(|(f, _)| match &f.value {
                Value::Text(text) | Value::Number(text) => Some((f.name.clone(), text.clone())),
                _ => None,
            })
            .collect();
        // Eleventy takes the day off a file's name for both, and off the
        // folder's for the slug of a folder's own page.
        let file_slug = match index {
            true => dated_name(last(&dir)).map_or(last(&dir), |(_, rest)| rest),
            false => &called,
        }
        .to_owned();
        let file_path_stem = format!("/{}", joined(&dir, &called));
        let mut unpublished = false;
        let mut own_address = false;
        let address = match &permalink {
            Some((field, _)) if field.value == Value::Truth(false) => {
                unpublished = true;
                None
            }
            Some((field, file)) => match &field.value {
                Value::Text(text) => {
                    let facts = permalink::Facts {
                        file_slug: &file_slug,
                        file_path_stem: &file_path_stem,
                        date: date.as_ref(),
                        fields: &plain,
                    };
                    match permalink::fill(text, &facts) {
                        Ok(filled) => {
                            let (address, a_file) = Self::settled_address(&filled);
                            if a_file {
                                self.find(
                                    &ADDRESS_A_FILE,
                                    file,
                                    field.line,
                                    address.trim_end_matches('/'),
                                );
                            }
                            own_address = true;
                            Some(address)
                        }
                        Err(tag) => {
                            self.find(&ADDRESS_UNKNOWN, file, field.line, &tag);
                            None
                        }
                    }
                }
                other => {
                    let shown = format!("permalink: {}", super::write::shown_value(other));
                    self.find(&ADDRESS_UNKNOWN, file, field.line, &shown);
                    None
                }
            },
            None => Some(Self::default_address(&rel, &stem)),
        };
        let draft_field = fields
            .iter()
            .any(|f| f.name == "draft" && f.value == Value::Truth(true));
        if unpublished {
            self.unpublished += 1;
            if !draft_field {
                fields.push(Field::new("draft", Value::Truth(true), None));
            }
        }
        let kind = match (unpublished || draft_field, date.is_some()) {
            (true, _) => PageKind::Draft,
            (false, true) => PageKind::Post,
            (false, false) => PageKind::Page,
        };
        Made {
            to,
            kind,
            fields,
            address: address.filter(|_| !unpublished && !draft_field),
            own_address,
            engine,
            unpublished,
            categories,
        }
    }

    /// Converts each page's writing, once every page is known, and hands
    /// the pages, the snippets standing for tags, the data handed down
    /// and what is said to the import.
    fn finish(&mut self) {
        // A folder's data that gives its pages a `permalink` gives it to
        // the pages its templates make too: `false` publishes none, and
        // any other is filled in by Eleventy, so the name says nothing.
        let elsewhere: Vec<String> = std::mem::take(&mut self.addressed_by_name)
            .into_iter()
            .filter(|(_, rel)| {
                let stem = rel.rsplit_once('.').map_or(rel.as_str(), |(stem, _)| stem);
                self.dir_data.iter().any(|data| {
                    let above = data.at.is_empty()
                        || data.at == stem
                        || parent(rel) == data.at
                        || parent(rel).starts_with(&format!("{}/", data.at));
                    above && data.fields.iter().any(|f| f.name == "permalink")
                })
            })
            .map(|(path, _)| path)
            .collect();
        (self.import.templated).retain(|(_, path)| !elsewhere.contains(path));
        (self.import.feeds).retain(|(_, path)| !elsewhere.contains(path));
        let tos: Vec<String> = self
            .docs
            .iter()
            .map(|doc| {
                let stem = last(&doc.rel)
                    .rsplit_once('.')
                    .map_or(last(&doc.rel), |(s, _)| s);
                let dir = parent(&doc.rel);
                match (
                    stem == "index" || (!dir.is_empty() && stem == last(dir)),
                    dated_name(stem),
                ) {
                    (true, _) => joined(dir, "index.md"),
                    (false, Some((_, rest))) => joined(dir, &format!("{rest}.md")),
                    (false, None) => joined(dir, &format!("{stem}.md")),
                }
            })
            .collect();
        let placements: Vec<(String, Placed)> = self
            .dir_data
            .iter()
            .map(|data| (data.at.clone(), self.placed(&data.at, &tos)))
            .collect();
        self.hand_down(&placements, &tos);
        let mut made = Vec::with_capacity(self.docs.len());
        for at in 0..self.docs.len() {
            made.push(self.made(at, &placements));
        }
        let parts = self.parts(&made);
        let wrapping: HashSet<String> = self
            .docs
            .iter()
            .zip(&made)
            .zip(&parts)
            .filter(|((_, made), part)| made.engine != Engine::Not && !**part)
            .map(|(both, _)| both)
            .flat_map(|(doc, _)| writing::paired(&doc.body))
            .collect();
        let separator = self.s.separator.as_ref().map(|s| s.value.clone());
        let html_off = match (&self.s.html, self.s.library) {
            (Some(given), _) => !given.value,
            (None, Some(_)) => true,
            (None, None) => false,
        };
        let breaks_on = self.s.breaks.as_ref().is_some_and(|b| b.value);
        let linkify_on = self.s.linkify.as_ref().is_some_and(|b| b.value);
        let attributes_on = self.s.attributes.is_some();
        let base = self.s.base();
        let mut calls: Vec<writing::Call> = Vec::new();
        let mut breaks = 0;
        let mut separators = 0;
        let mut counts = [0usize; 3];
        let docs = std::mem::take(&mut self.docs);
        let mut pages = Vec::with_capacity(docs.len());
        for ((doc, made), part) in docs.into_iter().zip(made).zip(parts) {
            if part {
                continue;
            }
            let cx = writing::Context {
                engine: made.engine,
                markdown: !doc.html,
                separator: separator.as_deref(),
                wrapping: &wrapping,
                breaks: breaks_on,
                attributes: attributes_on,
                linkify: linkify_on,
                html_off,
                base: &base,
            };
            let converted = writing::convert(&doc.body, doc.body_line, &doc.from, &cx);
            breaks += converted.breaks;
            separators += converted.separators;
            for finding in &converted.findings {
                for (n, kind) in [&ATTRIBUTES, &LINK_NOT_MADE, &HTML_SHOWN]
                    .iter()
                    .enumerate()
                {
                    if std::ptr::eq(finding.kind, *kind) {
                        counts[n] += 1;
                    }
                }
            }
            self.import.findings.extend(converted.findings);
            calls.extend(converted.calls);
            if doc.html {
                self.find(&HTML_WRITING, &doc.from, None, "");
            }
            pages.push(Page {
                from: doc.from,
                to: made.to,
                kind: made.kind,
                front: made.fields,
                writing: converted.writing,
                terms: match made.categories.is_empty() {
                    true => Vec::new(),
                    false => vec![("categories".to_owned(), made.categories)],
                },
                fences: converted.fences,
                calls: converted.called,
                address: made.address,
                own_address: made.own_address,
                other_addresses: Vec::new(),
                unread: None,
            });
        }
        self.beside(&pages);
        self.redirects(&mut pages);
        self.import.pages = pages;
        self.snippets(&calls);
        for (name, spots) in std::mem::take(&mut self.words) {
            self.import.findings.push(Finding {
                kind: &ELEVENTYS_WORD,
                about: Some(name),
                spots,
            });
        }
        let file = self.import.settings_file.clone();
        for (line, text) in std::mem::take(&mut self.s.unread) {
            self.find(&SETTINGS_UNREAD, &file, Some(line), &text);
        }
        self.notes(breaks, separators, counts);
    }

    /// Which pages are parts of other pages and not drafts: those in a
    /// folder where every page has `permalink: false` and no title. Each
    /// such folder is left behind, and the files counted by their tags.
    fn parts(&mut self, made: &[Made]) -> Vec<bool> {
        let part = |made: &Made| made.unpublished && made.fields.iter().all(|f| f.name != "title");
        let mut whole: Vec<(&str, usize)> = Vec::new();
        let mut mixed: HashSet<&str> = HashSet::new();
        for (doc, made) in self.docs.iter().zip(made) {
            let dir = parent(&doc.rel);
            match part(made) {
                true => match whole.iter_mut().find(|(at, _)| *at == dir) {
                    Some((_, files)) => *files += 1,
                    None => whole.push((dir, 1)),
                },
                false => {
                    mixed.insert(dir);
                }
            }
        }
        whole.retain(|(dir, _)| !dir.is_empty() && !mixed.contains(dir));
        let parts: Vec<bool> = (self.docs.iter())
            .map(|doc| whole.iter().any(|(dir, _)| *dir == parent(&doc.rel)))
            .collect();
        let mut tagged: Vec<(String, usize)> = Vec::new();
        for ((_, made), _) in (self.docs.iter().zip(made).zip(&parts)).filter(|(_, part)| **part) {
            let tags = match made.fields.iter().find(|f| f.name == "tags") {
                Some(field) => format!("tags: {}", super::write::shown_value(&field.value)),
                None => "no tags".to_owned(),
            };
            match tagged.iter_mut().find(|(shown, _)| *shown == tags) {
                Some((_, files)) => *files += 1,
                None => tagged.push((tags, 1)),
            }
        }
        let folder_of = |doc: &Doc| {
            let from = doc.from.rsplit_once('/').map_or("", |(dir, _)| dir);
            format!("{from}/")
        };
        let mut left: Vec<(String, usize)> = Vec::new();
        for (doc, _) in self.docs.iter().zip(&parts).filter(|(_, part)| **part) {
            let folder = folder_of(doc);
            match left.iter_mut().find(|(at, _)| *at == folder) {
                Some((_, files)) => *files += 1,
                None => left.push((folder, 1)),
            }
        }
        self.unpublished -= left.iter().map(|(_, files)| files).sum::<usize>();
        for (folder, files) in left {
            let what = format!(
                "{} with `permalink: false` and no title, which Eleventy made no page of",
                counted(files, "file", "files")
            );
            self.left(&folder, &what);
        }
        for (tags, files) in tagged {
            self.import.findings.push(Finding {
                kind: &PARTS,
                about: None,
                spots: vec![Spot {
                    file: String::new(),
                    line: None,
                    text: format!("{}: {}", code(&tags), counted(files, "file", "files")),
                    times: None,
                }],
            });
        }
        parts
    }

    /// Writes each folder's data where it goes: `pages.` lines in the
    /// folder's own page, or else each page's own fields.
    fn hand_down(&mut self, placements: &[(String, Placed)], tos: &[String]) {
        let mut new_pages: Vec<(String, Vec<Field>)> = Vec::new();
        let mut layouts = 0;
        let mut told = Vec::new();
        for (data_at, placed) in placements {
            let Placed::Index(path, came) = placed else {
                continue;
            };
            if !tos.iter().any(|to| under(to, data_at) && to != path) {
                continue;
            }
            let Some(data) = self.dir_data.iter().find(|d| d.at == *data_at) else {
                continue;
            };
            let mut lines = Vec::new();
            let mut words = Vec::new();
            for field in &data.fields {
                match field.name.as_str() {
                    "layout" => layouts += 1,
                    "permalink" | "tags" | "templateEngineOverride" => {}
                    name if each_pages_own(name, &field.value) => {}
                    name => {
                        if ELEVENTYS_WORDS.contains(&name) {
                            words.push((
                                name.to_owned(),
                                data.file.clone(),
                                field.line,
                                field.value.clone(),
                            ));
                        }
                        let date_word = name == "date";
                        if !date_word {
                            lines.push(Field::new(
                                &format!("pages.{}", statacs_name(name)),
                                field.value.clone(),
                                None,
                            ));
                        }
                    }
                }
            }
            for (name, file, line, value) in words {
                self.eleventys_word(&name, &file, line, &value);
            }
            if lines.is_empty() {
                continue;
            }
            told.push(code(&format!("content/{path}")));
            if *came || path == "posts/index.md" {
                for line in lines {
                    self.import.words.push((path.clone(), line));
                }
            } else {
                new_pages.push((path.clone(), lines));
            }
        }
        // A page written only to hold the lines publishes nothing, as the
        // old site had no page there.
        for (path, mut lines) in new_pages {
            lines.push(Field::new("index_page", Value::Truth(false), None));
            let front = super::yaml::fields(lines.iter().map(|f| (f.name.as_str(), &f.value)));
            self.import
                .written
                .push((format!("content/{path}"), format!("---\n{front}---\n")));
        }
        self.layouts_taken += layouts;
        if !told.is_empty() {
            self.note(format!(
                "The data each folder's own file cascaded to the pages beneath it is in `pages.` \
                 lines of {}, as https://statac.dev/front-matter/#handing-down describes; what \
                 can't cascade that way, as a `title` or a field inside a folder with no \
                 page of its own, is in each page's own front matter.",
                listed(&told)
            ));
        }
    }

    /// Brings across each file beside a page that its writing names, and
    /// lists the rest by folder.
    fn beside(&mut self, pages: &[Page]) {
        let others = std::mem::take(&mut self.others);
        let mut unread: Vec<(String, usize)> = Vec::new();
        for path in others {
            let name = last(&path);
            let folder = parent(&path);
            let named = pages
                .iter()
                .any(|page| parent(&page.from) == folder && names_file(&page.writing, name));
            if named {
                let to = format!("content/{}", self.rel(&path));
                self.import.files.push(File { from: path, to });
                continue;
            }
            match folder == self.input {
                true => self.left(
                    &path,
                    "a file Eleventy didn't publish, being neither a page nor copied by the \
                     settings",
                ),
                false => match unread.iter_mut().find(|(f, _)| f == folder) {
                    Some((_, n)) => *n += 1,
                    None => unread.push((folder.to_owned(), 1)),
                },
            }
        }
        for (folder, n) in unread {
            let what = format!(
                "{} Eleventy didn't publish, being neither pages nor copied by the settings, nor \
                 named by a page beside them",
                counted(n, "file", "files")
            );
            self.left(&format!("{folder}/"), &what);
        }
    }

    /// Gives each page the old addresses `_redirects` sent on to it.
    fn redirects(&mut self, pages: &mut [Page]) {
        let Some(path) = self.redirects.clone() else {
            return;
        };
        let Ok(text) = std::fs::read_to_string(self.path(&path)) else {
            return;
        };
        let mut kept = 0;
        for (n, line) in text.lines().enumerate() {
            let written = line.trim();
            if written.is_empty() || written.starts_with('#') {
                continue;
            }
            let words: Vec<&str> = written.split_whitespace().collect();
            let status_ok = match words.get(2) {
                None => true,
                Some(status) => matches!(
                    status.trim_end_matches('!'),
                    "301" | "302" | "303" | "307" | "308"
                ),
            };
            let (Some(from), Some(to)) = (words.first(), words.get(1)) else {
                self.find(&REDIRECT_LEFT, &path, Some(n + 1), written);
                continue;
            };
            let plain =
                |address: &str| address.starts_with('/') && !address.contains(['*', ':', '?']);
            let target = |address: &str| {
                let address = address.split('#').next().unwrap_or_default();
                let address = address.strip_suffix("index.html").unwrap_or(address);
                match address.ends_with('/') || last(address).contains('.') {
                    true => address.to_owned(),
                    false => format!("{address}/"),
                }
            };
            let page = pages
                .iter_mut()
                .find(|page| page.address.as_deref() == Some(target(to).as_str()));
            match (
                page,
                status_ok && plain(from) && plain(to) && words.len() <= 3,
            ) {
                (Some(page), true) if page.address.as_deref() != Some(*from) => {
                    page.other_addresses.push((*from).to_owned());
                    kept += 1;
                }
                _ => self.find(&REDIRECT_LEFT, &path, Some(n + 1), written),
            }
        }
        if kept > 0 {
            self.note(format!(
                "The old site's {} redirected {} to a page, each now in that page's \
                 `redirect_from`.",
                code(&path),
                counted(kept, "old URL", "old URLs")
            ));
        }
    }

    /// The snippets standing for includes and tags of the site's own that
    /// became calls, each with one finding listing its calls.
    fn snippets(&mut self, calls: &[writing::Call]) {
        let mut snippets: Vec<&str> = Vec::new();
        for call in calls {
            if !snippets.contains(&call.snippet.as_str()) {
                snippets.push(&call.snippet);
            }
        }
        for snippet in snippets {
            let of: Vec<&writing::Call> = calls.iter().filter(|c| c.snippet == snippet).collect();
            let mut names: Vec<String> = Vec::new();
            for call in &of {
                for name in &call.names {
                    if !names.contains(name) {
                        names.push(name.clone());
                    }
                }
            }
            let path = format!("snippets/{snippet}.html");
            let positional = of.iter().any(|c| c.positional);
            let mut text = super::liquid::stand_in(&of[0].what, &names, positional);
            if of.iter().any(|c| c.wraps) {
                text.push_str("{{ body }}\n");
            }
            self.import.written.push((path.clone(), text));
            self.import.findings.push(Finding {
                kind: match of[0].include {
                    true => &INCLUDE,
                    false => &SNIPPET_CALLS,
                },
                about: Some(path),
                spots: of.iter().map(|call| call.spot.clone()).collect(),
            });
        }
    }

    /// What is said of the settings, each thing taken from them and each
    /// assumed, and of what the pages had taken off.
    fn notes(&mut self, breaks: usize, separators: usize, counts: [usize; 3]) {
        let file = self.import.settings_file.clone();
        let mut notes = Vec::new();
        notes.push(match file.is_empty() {
            true => "The old site has no settings file Eleventy reads, so Statac took Eleventy's \
                     own settings throughout."
                .to_owned(),
            false => format!(
                "Eleventy's settings are a program, and Statac runs none of it: it read {} as \
                 text, for the plain forms it knows, and says here what it took and what it \
                 assumed where it found none.",
                code(&file)
            ),
        });
        let base = self.s.base();
        let feed_url = self.feed_url(&base);
        self.import.address = match &feed_url {
            Some((url, _)) => Some(url.clone()),
            None => self.given.map(|given| super::under(given, &base)),
        };
        let title = self.feed_gives(|feed| &feed.title);
        let language = self.feed_gives(|feed| &feed.language);
        self.import.title = title.as_ref().map(|(title, _)| title.clone());
        if let Some((written, _)) = &language
            && crate::new::a_language_code(written)
        {
            self.import.languages = vec![written.clone()];
        }
        let mut missing = Vec::new();
        let mut so = Vec::new();
        if title.is_none() {
            missing.push("title");
            so.push("the new site's `title` is made from the old folder's name");
        }
        if feed_url.is_none() {
            missing.push("URL");
            so.push(match (self.given, so.is_empty()) {
                (Some(_), true) => "the new site's `url` is the URL you gave",
                (Some(_), false) => "its `url` is the URL you gave",
                (None, true) => "the new site has no `url` yet",
                (None, false) => "it has no `url` yet",
            });
        }
        if language.is_none() {
            missing.push("language");
            so.push(match so.is_empty() {
                true => "the new site is in English, `en`",
                false => "it is in English, `en`",
            });
        }
        if let ([rest @ .., last], [first_so @ .., last_so]) = (missing.as_slice(), so.as_slice()) {
            let (missing_shown, so) = match rest {
                [] => ((*last).to_owned(), (*last_so).to_owned()),
                _ => (
                    format!("{} or {last}", rest.join(", ")),
                    format!("{}, and {last_so}", first_so.join(", ")),
                ),
            };
            let then = match (missing.as_slice(), self.given) {
                (["URL"], Some(_)) => "",
                (["URL"], None) => ": add it to `statac.yaml` once it is known",
                ([_], _) => ": put it right in `statac.yaml`",
                _ => ": put each right in `statac.yaml`",
            };
            notes.push(format!(
                "Eleventy's settings hold no {missing_shown} for the site, so {so}{then}."
            ));
        }
        if let Some((written, feed)) = &title {
            notes.push(format!(
                "The new site's `title`, {}, is taken from `metadata.title` in the settings' \
                 `feedPlugin`, on line {}.",
                code(written),
                feed.line
            ));
        }
        match &language {
            Some((written, feed)) if crate::new::a_language_code(written) => notes.push(format!(
                "The new site's `language`, {}, is taken from `metadata.language` in the \
                 settings' `feedPlugin`, on line {}.",
                code(written),
                feed.line
            )),
            Some((written, feed)) => notes.push(format!(
                "The settings' `feedPlugin`, on line {}, gives `metadata.language` as {}, which \
                 isn't a code Statac reads, so the site is in English, `en`: write its code in \
                 `language` in `statac.yaml`.",
                feed.line,
                code(written)
            )),
            None => {}
        }
        if let Some((url, feed)) = &feed_url {
            notes.push(format!(
                "The new site's `url`, {}, is made from `metadata.base` in the settings' \
                 `feedPlugin`, on line {}, {}as Eleventy's RSS plugin made each post's full URL.",
                code(url),
                feed.line,
                match base.is_empty() {
                    true => "",
                    false => "and from `pathPrefix`, ",
                }
            ));
        }
        if !base.is_empty() && self.import.address.is_none() {
            notes.push(format!(
                "The settings' `pathPrefix` puts the old site under {path}, and each URL \
                 through the `url` filter is written with {path} in front, as Eleventy wrote it. \
                 Statac puts a site under a path only by its `url`, and the new site has none \
                 yet. Add the full URL as `url` in `statac.yaml`, such as {example}. Until then, \
                 `statac check` says nothing is published at those links: leave them as they \
                 are, even where it offers another URL.",
                path = code(&base),
                example = code(&format!("url: https://example.org{base}"))
            ));
        }
        self.import.timezone = Some("UTC".into());
        let maths = settings::sets_maths(self.folder);
        self.import.maths = super::Maths {
            inline: maths,
            display: maths,
        };
        notes.push(
            "Eleventy reads a date written without an offset as UTC, so `timezone` is `UTC`: \
             write the site's own, as `Europe/London`, where its dates were meant there."
                .to_owned(),
        );
        notes.extend(std::mem::take(&mut self.folder_notes));
        let copies: Vec<String> = self.s.copies.iter().map(|copy| code(&copy.from)).fold(
            Vec::new(),
            |mut shown, from| {
                if !shown.contains(&from) {
                    shown.push(from);
                }
                shown
            },
        );
        notes.push(match copies.is_empty() {
            true => "The settings have no `addPassthroughCopy` Statac can read, so Statac \
                     copied only the files beside a page that its content names."
                .to_owned(),
            false => format!(
                "The settings copy {} as {} with `addPassthroughCopy`, so Statac copied {} into \
                 `static/`, where {}.",
                listed(&copies),
                if copies.len() == 1 {
                    "it is"
                } else {
                    "they are"
                },
                counted(self.copied_count, "file", "files"),
                match self.copied_count {
                    1 => "it keeps its URL",
                    _ => "each keeps its URL",
                }
            ),
        });
        let engine_note = |given: &Option<settings::Given<Option<String>>>,
                           setting: &str,
                           files: &str| match given {
            None => format!(
                "The settings name no `{setting}`, so Statac read the template tags in the \
                 {files} as Liquid, as Eleventy does unless told otherwise."
            ),
            Some(given) => match given.value.as_deref() {
                Some("liquid") => format!(
                    "The settings' `{setting}` is `liquid`, so Statac read the template tags in \
                     the {files} as Liquid."
                ),
                Some("njk") => format!(
                    "The settings' `{setting}` is `njk`, so Statac read the template tags in the \
                     {files} as Nunjucks."
                ),
                None | Some("md" | "html") => format!(
                    "The settings' `{setting}` is {}, so Eleventy ran no template tags in the \
                     {files}, and Statac left `{{{{` and `{{%` in them as words, as Eleventy \
                     showed them.",
                    match &given.value {
                        Some(name) => code(name),
                        None => "`false`".to_owned(),
                    }
                ),
                Some(other) => format!(
                    "The settings' `{setting}` is {}, whose tags Statac doesn't read, so it left \
                     the {files} as written.",
                    code(other)
                ),
            },
        };
        notes.push(engine_note(
            &self.s.engine,
            "markdownTemplateEngine",
            "Markdown files",
        ));
        if self.html_pages > 0 {
            notes.push(engine_note(
                &self.s.html_engine,
                "htmlTemplateEngine",
                "HTML files",
            ));
        }
        notes.push(match self.s.breaks.as_ref().map(|b| b.value) {
            Some(true) => format!(
                "The settings' Markdown had `breaks: true`, which made each line end inside a \
                 paragraph a line break, so Statac ended {} with `\\`, which Statac reads as a \
                 line break.",
                counted(breaks, "line", "lines")
            ),
            _ => "The settings don't set `breaks: true`, so Statac read a line end inside a \
                  paragraph as a space, as Eleventy's Markdown does unless told otherwise."
                .to_owned(),
        });
        notes.push(match self.s.linkify.as_ref().map(|b| b.value) {
            Some(true) => format!(
                "The settings' Markdown had `linkify: true`, which linked bare URLs; Statac links \
                 those starting `www.`, `http://` or `https://` too, and {}, listed under Left \
                 for you.",
                match counts[1] {
                    0 => "the content has no other".to_owned(),
                    1 => "1 other is no longer linked".to_owned(),
                    n => format!("{n} others are no longer linked"),
                }
            ),
            _ => "The settings don't set `linkify: true`, so Eleventy left bare URLs as words, \
                  and Statac links those starting `www.`, `http://` or `https://`, as \
                  https://statac.dev/writing/#addresses-written-bare describes."
                .to_owned(),
        });
        if !self.s.markdown_in.is_empty() {
            let files: Vec<String> = self.s.markdown_in.iter().map(|file| code(file)).collect();
            notes.push(format!(
                "Markdown's settings were read in {}, which the settings file fetches.",
                listed(&files)
            ));
        }
        notes.push(match (&self.s.html, self.s.library) {
            (Some(given), _) if given.value => "The settings' Markdown had `html: true`, so \
                 Eleventy published HTML in Markdown unchanged, as Statac does."
                .to_owned(),
            (Some(_), _) => format!(
                "The settings' Markdown had `html: false`, so Eleventy showed HTML in Markdown as \
                 text, and Statac publishes it as HTML: {}.",
                match counts[2] {
                    0 => "the Markdown holds none".to_owned(),
                    n => format!(
                        "{} listed under Left for you",
                        counted(n, "place is", "places are")
                    ),
                }
            ),
            (None, Some(_)) => format!(
                "The settings give Markdown a library of their own and don't set `html: true`, \
                 so Eleventy showed HTML in Markdown as text, and Statac publishes it as HTML: {}.",
                match counts[2] {
                    0 => "the Markdown holds none".to_owned(),
                    n => format!(
                        "{} listed under Left for you",
                        counted(n, "place is", "places are")
                    ),
                }
            ),
            (None, None) => "The settings give Markdown no library of their own, so Statac took \
                 Eleventy's own, which publishes HTML in Markdown unchanged, as Statac does."
                .to_owned(),
        });
        notes.push(match &self.s.attributes {
            Some(plugin) => format!(
                "The settings use {}, which read attributes in braces, as `{{.class}}`; Statac \
                 reads them only at the end of a heading, so {}.",
                code(&plugin.value),
                match counts[0] {
                    0 => "the content has none elsewhere".to_owned(),
                    1 => "1 elsewhere shows as written, listed under Left for you".to_owned(),
                    n => format!("{n} elsewhere show as written, listed under Left for you"),
                }
            ),
            None => "The settings name no plugin for attributes in braces, so Statac took \
                     `{.class}` and the like for words, as Eleventy did."
                .to_owned(),
        });
        notes.push(match (&self.s.separator, self.s.excerpt) {
            (Some(separator), _) => format!(
                "The settings' `excerpt_separator`, {}, is now `<!--more-->`, Statac's excerpt \
                 marker, in {}.",
                code(&separator.value),
                counted(separators, "place", "places")
            ),
            (None, Some(_)) => "The settings ask for excerpts and name no `excerpt_separator`, \
                 so Eleventy ended each at the first line of `---`, which Statac leaves as written: \
                 put `<!--more-->` where an excerpt should end."
                .to_owned(),
            (None, None) => "The settings name no `excerpt_separator`, so Statac changed no \
                 excerpt marker."
                .to_owned(),
        });
        if self.s.no_gitignore.is_some() {
            notes.push(
                "The settings' `setUseGitIgnore(false)` had Eleventy pass over `.gitignore`, so \
                 Statac did too."
                    .to_owned(),
            );
        }
        if self.negated > 0 {
            notes.push(format!(
                "Statac passed over {} of `.eleventyignore` or `.gitignore` starting `!`, which \
                 brought back what another left out, so what {} brought back is left out too.",
                counted(self.negated, "line", "lines"),
                if self.negated == 1 { "it" } else { "they" }
            ));
        }
        if self.layouts_taken > 0 {
            notes.push(format!(
                "`layout` was taken off {}: once you add templates, each is built with the \
                 one for its kind, as https://statac.dev/templates/ describes.",
                counted(self.layouts_taken, "page or folder", "pages and folders"),
            ));
        }
        if self.controls.0 > 0 {
            notes.push(format!(
                "Statac took {} out of the front matter of {}: Eleventy published each as \
                 nothing, and Statac's reader stops at one.",
                counted(self.controls.0, "control character", "control characters"),
                counted(self.controls.1, "page", "pages")
            ));
        }
        if self.overrides > 0 {
            notes.push(format!(
                "`templateEngineOverride` was taken off {}, once the content was read in the \
                 language it named.",
                counted(self.overrides, "page", "pages")
            ));
        }
        if self.unpublished > 0 {
            notes.push(format!(
                "{} `permalink: false`, so Eleventy didn't publish {}, and {} across with \
                 `draft: true`.",
                match self.unpublished {
                    1 => "1 page had".to_owned(),
                    n => format!("{n} pages had"),
                },
                if self.unpublished == 1 { "it" } else { "them" },
                if self.unpublished == 1 {
                    "it came"
                } else {
                    "each came"
                }
            ));
        }
        if self.undated > 0 {
            notes.push(format!(
                "{} no date, which Eleventy gave the day {} file was made; Statac gives {} none, \
                 so {} a page and not a post.",
                match self.undated {
                    1 => "1 page had".to_owned(),
                    n => format!("{n} pages had"),
                },
                if self.undated == 1 {
                    "its"
                } else {
                    "each one's"
                },
                if self.undated == 1 { "it" } else { "each" },
                if self.undated == 1 {
                    "it is"
                } else {
                    "each is"
                }
            ));
        }
        self.feeds(&mut notes);
        let mut all = notes;
        all.append(&mut self.import.notes);
        self.import.notes = all;
    }

    /// The site's URL as the RSS plugin's feeds wrote each post's: the
    /// scheme and host of the first `metadata.base` the settings give as an
    /// `https://` or `http://` URL, with `base`, the path the site is
    /// under, after them; and the feed that gives it.
    fn feed_url(&self, base: &str) -> Option<(String, settings::PluginFeed)> {
        self.s.feeds.iter().find_map(|feed| {
            let url = url::Url::parse(feed.base.as_deref()?.trim()).ok()?;
            if !matches!(url.scheme(), "https" | "http") || url.host().is_none() {
                return None;
            }
            let origin = url.origin().ascii_serialization();
            Some((format!("{origin}{base}"), feed.clone()))
        })
    }

    /// The first value `read` finds in the RSS plugin's feeds' `metadata`,
    /// trimmed, that isn't empty; and the feed that gives it.
    fn feed_gives(
        &self,
        read: impl Fn(&settings::PluginFeed) -> &Option<String>,
    ) -> Option<(String, settings::PluginFeed)> {
        self.s.feeds.iter().find_map(|feed| {
            let written = read(feed).as_deref()?.trim();
            (!written.is_empty()).then(|| (written.to_owned(), feed.clone()))
        })
    }

    /// Asks for each feed the RSS plugin made at the URL it had, and keeps
    /// each post's id in it; where the old site had no plugin, says that
    /// the ids its own feeds gave aren't kept.
    fn feeds(&mut self, notes: &mut Vec<String>) {
        let named = settings::makes_feeds(self.folder);
        if !named && self.s.feeds.is_empty() {
            let feeds: Vec<String> = (self.import.feeds.iter())
                .map(|(address, _)| code(address))
                .collect();
            if !feeds.is_empty() {
                notes.push(format!(
                    "Each post's id in the old site's {} at {} changes after the move, so feed \
                     readers may show each post once as new: see \
                     https://statac.dev/feeds/#entry-ids.",
                    match feeds.len() {
                        1 => "feed",
                        _ => "feeds",
                    },
                    listed(&feeds)
                ));
            }
            return;
        }
        self.import
            .settings
            .push(("feed_ids_before".into(), Value::Text(self.today.iso_day())));
        self.import
            .settings
            .push(("feed_ids_from".into(), Value::Text("eleventy".into())));
        let base = self.s.base();
        let posts: Vec<String> = (self.import.pages.iter())
            .filter(|page| page.kind == PageKind::Post)
            .filter_map(|page| page.address.as_ref())
            .filter(|address| address.contains(['\'', '&']))
            .map(|address| format!("{base}{address}"))
            .collect();
        let twice = (!posts.is_empty()).then(|| escaped_twice(&posts));
        if self.plugin_feeds.is_empty() {
            let ids = match named {
                true => "The old site's `package.json` names Eleventy's RSS plugin",
                false => "The settings use Eleventy's RSS plugin",
            };
            let why = (self.unread_feed.clone())
                .unwrap_or_else(|| "The settings show no `feedPlugin`".to_owned());
            let see = "Feed readers see nothing new once a list's `feed:` line puts its feed at \
                       the old feed's URL: see https://statac.dev/feeds/#entry-ids.";
            let rest = match (&self.unread_feed, self.import.feeds.is_empty()) {
                (None, false) => see.to_owned(),
                (Some(_), false) => format!("{why}, so Statac asked for none. {see}"),
                (_, true) => format!(
                    "{why}, and {}, so Statac asked for none: a `feed:` line in `statac.yaml` \
                     puts the site's feed at the old feed's URL, as `feed: /feed.xml`, and feed \
                     readers see nothing new: see https://statac.dev/feeds/#entry-ids.",
                    match self.import.built {
                        Some(_) => "nothing in the old built output shows where its feed was",
                        None => "the old site has no built output to show where its feed was",
                    }
                ),
            };
            notes.push(format!(
                "{ids}, whose feed templates give each post its full URL as its id, so \
                 `feed_ids_before` and `feed_ids_from: eleventy` keep that id for each post. \
                 {rest}"
            ));
            notes.extend(twice);
            return;
        }
        let made: Vec<String> = (self.plugin_feeds.iter())
            .map(|(address, kind)| format!("{} at {}", a_feed(*kind), code(address)))
            .collect();
        let lead = format!(
            "The settings' `feedPlugin`, from Eleventy's RSS plugin, made {}.",
            listed(&made)
        );
        let asked = self.plugin_feeds.clone();
        let (shown, templates) = self.ask(&asked);
        let mut note = format!(
            "{lead} {} at the same URL, as {shown} in `statac.yaml` says, built with {} once you \
             add {}, and `feed_ids_before` and `feed_ids_from: eleventy` keep each post's id the \
             URL Eleventy gave it, so feed readers see nothing new: see \
             https://statac.dev/feeds/#entry-ids.",
            match asked.len() {
                1 => "It stays",
                _ => "Each stays",
            },
            listed(&templates),
            match templates.len() {
                1 => "it",
                _ => "them",
            }
        );
        let feeds: Vec<(String, Option<FeedFormat>)> = (self.plugin_feeds.iter())
            .map(|(address, kind)| (last(address).to_owned(), *kind))
            .collect();
        if let Some(alike) = feed_templates_said(&[feeds]) {
            note.push(' ');
            note.push_str(&alike);
        }
        notes.push(note);
        notes.extend(twice);
    }

    /// Asks for the site's feed at each address of `feeds`, the first being
    /// the one its pages link to, each with its format where known, giving
    /// the setting as shown and the templates the feeds are built with.
    fn ask(&mut self, feeds: &[(String, Option<FeedFormat>)]) -> (String, Vec<String>) {
        let addresses: Vec<String> = feeds.iter().map(|(address, _)| address.clone()).collect();
        let (value, shown) = match addresses.as_slice() {
            [only] if *only == format!("/{}", crate::feeds::FILE_NAME) => {
                (Value::Truth(true), code("feed: true"))
            }
            [only] => (Value::Text(only.clone()), code(&format!("feed: {only}"))),
            _ => (
                Value::List(addresses.iter().cloned().map(Value::Text).collect()),
                "`feed:`".to_owned(),
            ),
        };
        self.import
            .settings
            .push((crate::feeds::SETTING.into(), value));
        let named: Vec<(String, Option<FeedFormat>)> = (feeds.iter())
            .map(|(address, format)| (last(address).to_owned(), *format))
            .collect();
        let mut templates = Vec::new();
        for template in feed_templates(&named) {
            if !templates.contains(&template) {
                templates.push(template);
            }
        }
        self.import.feed_templates.extend(templates.iter().cloned());
        self.import.addresses.extend(addresses);
        (shown, templates.iter().map(|t| code(t)).collect())
    }
}
