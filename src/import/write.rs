//! The new site written from an [`Import`]: `statac.yaml`, then what came
//! across, then the addresses kept and checked, then the report. Where anything stops it part way, the folder
//! is left as it was found.

use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};

use super::report::{self, Shown};
use super::writing::{self, Settled};
use super::{
    Field, File, Finding, FindingKind, Import, Left, Page, PageKind, Spot, Value, addresses, yaml,
};
use crate::config;
use crate::content::address;
use crate::content::words::{self, OTHER_SPELLINGS, STATAC_FILLS};
use crate::date::Date;
use crate::new::{self, Answers};
use crate::platform;
use crate::problem::{Kind, Maker, Problem, Reason, Word};

/// The file the report is written to, at the top of the new site.
pub(super) const REPORT: &str = "IMPORT.md";

/// Statac's words that mean on a page what the same word means to the
/// tools a site comes from, so a field of that name is kept as it is.
const SAME_MEANING: &[&str] = &[
    "title",
    "date",
    "updated",
    "summary",
    "tags",
    "series",
    "permalink",
    "redirect_from",
    "redirect_to",
    "draft",
];

pub(super) static FIELD_RENAMED: FindingKind = FindingKind {
    heading: "Fields renamed",
    sentence: "Each of these fields has the name of one of Statac's built-in fields, which means \
               something else there or holds another kind of value, so Statac put `old_` before \
               its name: show it from a template under that name, or take it out.",
};

pub(super) static DATE_LEFT: FindingKind = FindingKind {
    heading: "Dates left as written",
    sentence: "Statac couldn't be sure which moment each of these dates means, so it left it as \
               written, and the page won't build until it is written year first, as \
               `2026-10-04` or `2026-10-04T09:30+01:00`.",
};

pub(super) static NAME_TAKEN: FindingKind = FindingKind {
    heading: "Pages given another name",
    sentence: "Two pages of the old site would have had the same file in the new one, so the \
               later has a number after its name; its URL is kept all the same.",
};

pub(super) static FILE_RENAMED: FindingKind = FindingKind {
    heading: "Files given another name",
    sentence: "Statac can't publish a file under the name each of these had, so each has one it \
               can: a `%` and the two figures after it became the character they spell, as a \
               browser reads them, and any other character a URL or a disk won't take became \
               `-`. Links in the content that named the file reach it under its new name. Its \
               old URL is listed under URLs, and nothing is published there now: a file can't \
               redirect.",
};

pub(super) static MORE_OUT: FindingKind = FindingKind {
    heading: "Summary markers taken out",
    sentence: "Statac ends a summary only at `<!--more-->` on a line of its own between two \
               blocks of Markdown, and one anywhere else stops the build. Each of these stood \
               inside a line, a list, a quotation or an HTML element, so it was taken out, and \
               the page's summary is its first paragraph: put `<!--more-->` on a line of its \
               own where the summary should end.",
};

pub(super) static LINKS: FindingKind = FindingKind {
    heading: "Links Statac didn't follow",
    sentence: "Statac follows a link in the old site only to something inside it that the old \
               site published. Each of these it didn't follow, for the reason beside it, so \
               what the old site read through it didn't come across: put the folder or file \
               itself where the link is, in a copy of the old site, and import again.",
};

pub(super) static NOT_IN_BUILT: FindingKind = FindingKind {
    heading: "URLs the old built site doesn't have",
    sentence: "Statac worked these URLs out by the old tool's rules, but the old site's built \
               output has nothing there, so the rule was wrong for the page or the output is \
               older than the content: find where the old site published the page, and add that \
               URL to the page's `redirect_from`.",
};

pub(super) static FEED_LEFT: FindingKind = FindingKind {
    heading: "Feeds made by templates",
    sentence: "Readers subscribed to the old site's feed at each of these URLs, and the \
               template that made it didn't come across, so nothing is there now. A list's \
               `feed:` line can put that list's feed at a URL of its own, as `feed: \
               /index.rss`: see https://statac.dev/feeds/.",
};

pub(super) static NEW_SITE_ERRORS: FindingKind = FindingKind {
    heading: "Errors in the new site",
    sentence: "Statac found these when it read the new site, and each stops it building: \
               `statac check` shows each one with what to do.",
};

/// The lists of terms the old site kept that Statac has no equal for,
/// each name of which is now a tag with the list's name before it.
#[derive(Default, Debug)]
pub(super) struct Terms {
    /// Each list by its name, with each of its names in the order met and
    /// the tag it became.
    pub lists: Vec<(String, Vec<(String, String)>)>,
    /// Each tag the pages had of their own, in the order met.
    pub tags: Vec<String>,
    /// How many pages named more than one series.
    pub several_series: usize,
}

impl Terms {
    /// What a list's names are marked with: `category` for `categories`,
    /// and the list's own name for any other.
    fn mark(list: &str) -> &str {
        match list.eq_ignore_ascii_case("categories") {
            true => "category",
            false => list,
        }
    }

    /// Makes each of `page`'s names in a list of terms a tag after its own
    /// tags, the first of several series its `series`, and empties
    /// [`Page::terms`].
    fn tag(&mut self, page: &mut Page) {
        let own: Vec<String> = match page.front.iter().find(|field| field.name == "tags") {
            Some(Field {
                value: Value::List(items),
                ..
            }) => (items.iter())
                .filter_map(|item| match item {
                    Value::Text(text) | Value::Number(text) => Some(text.clone()),
                    _ => None,
                })
                .collect(),
            Some(Field {
                value: Value::Text(text) | Value::Number(text),
                ..
            }) => vec![text.clone()],
            _ => Vec::new(),
        };
        for tag in &own {
            if !self.tags.contains(tag) {
                self.tags.push(tag.clone());
            }
        }
        let mut added: Vec<String> = Vec::new();
        for (list, names) in std::mem::take(&mut page.terms) {
            let mut names = names.into_iter().filter(|name| !name.trim().is_empty());
            if list == "series" && !page.front.iter().any(|field| field.name == "series") {
                let Some(first) = names.next() else { continue };
                page.front
                    .push(Field::new("series", Value::Text(first), None));
            }
            let names: Vec<String> = names.collect();
            if list == "series" && !names.is_empty() {
                self.several_series += 1;
            }
            for name in names {
                let tag = format!("{}_{name}", Terms::mark(&list));
                let known = match self.lists.iter_mut().find(|(known, _)| *known == list) {
                    Some((_, known)) => known,
                    None => {
                        self.lists.push((list.clone(), Vec::new()));
                        &mut self.lists.last_mut().expect("the list put in").1
                    }
                };
                if !known.iter().any(|(known, _)| *known == name) {
                    known.push((name, tag.clone()));
                }
                if !own.contains(&tag) && !added.contains(&tag) {
                    added.push(tag);
                }
            }
        }
        if added.is_empty() {
            return;
        }
        let all = own.into_iter().chain(added).map(Value::Text).collect();
        match page.front.iter_mut().find(|field| field.name == "tags") {
            Some(field) => field.value = Value::List(all),
            None => page.front.push(Field::new("tags", Value::List(all), None)),
        }
    }

    /// Each old page of a name in a list, or of a tag, as the tag the
    /// name became and the page's address. A series' page comes first as
    /// the series, and then as its tag where the name was a later series
    /// of some page: whichever of the two the new site has is sent on to.
    fn old_pages(&self, pages: &[(String, String, String)]) -> Vec<(Word, String, String)> {
        let mut old = Vec::new();
        for (list, name, address) in pages {
            if list == "tags" {
                if self.tags.contains(name) {
                    old.push((Word::Tag, name.clone(), address.clone()));
                }
                continue;
            }
            if list == "series" {
                old.push((Word::Series, name.clone(), address.clone()));
            }
            let tag = (self.lists.iter())
                .find(|(known, _)| known == list)
                .and_then(|(_, names)| names.iter().find(|(known, _)| known == name));
            if let Some((_, tag)) = tag {
                old.push((Word::Tag, tag.clone(), address.clone()));
            }
        }
        old
    }

    /// Every tag a page has, with the name it had, its list, and the
    /// folders a tool may have published its old page in, for finding one in the old built output: a list's
    /// own name, and the name without its `s` or with `y` for `ies`.
    fn names(&self) -> Vec<(String, String, String, Vec<String>)> {
        let folders = |list: &str| {
            let lower = list.to_lowercase();
            let mut folders = vec![lower.clone()];
            if let Some(stem) = lower.strip_suffix("ies") {
                folders.push(format!("{stem}y"));
            } else if let Some(stem) = lower.strip_suffix('s') {
                folders.push(stem.to_owned());
            }
            folders
        };
        let lists = self.lists.iter().flat_map(|(list, names)| {
            (names.iter())
                .map(move |(name, tag)| (tag.clone(), name.clone(), list.clone(), folders(list)))
        });
        let tags = (self.tags.iter())
            .map(|tag| (tag.clone(), tag.clone(), "tags".to_owned(), folders("tags")));
        lists.chain(tags).collect()
    }
}

/// What was written, for the report.
#[derive(Default, Debug)]
pub(super) struct Summary {
    pub posts: usize,
    pub drafts: usize,
    pub pages: usize,
    pub files: usize,
    /// Where the posts went, by their folder under `content/`.
    pub post_folders: Vec<String>,
    pub kept: addresses::Kept,
    pub findings: Vec<Finding>,
    pub left: Vec<Left>,
    pub notes: Vec<String>,
    /// `statac.yaml` as written.
    pub settings: String,
    /// Whether the old site's home page came across.
    pub home: bool,
    /// The path the site is under, where it is under one.
    pub base: String,
    pub built: Option<String>,
    /// How many errors `statac check` finds in the new site.
    pub errors: usize,
    /// The templates the new site's pages and feeds are built with that it
    /// doesn't have, by their paths.
    pub templates: Vec<String>,
    /// The lists of terms whose names became tags.
    pub terms: Terms,
    /// What was changed in the pages' writing so Statac reads it as the
    /// old site showed it.
    pub settled: Settled,
}

impl Summary {
    /// How many things of how many kinds are left for the author.
    pub(super) fn left_for_you(&self) -> (usize, usize) {
        let mut kinds: Vec<&FindingKind> = Vec::new();
        for finding in &self.findings {
            if !kinds.iter().any(|kind| std::ptr::eq(*kind, finding.kind)) {
                kinds.push(finding.kind);
            }
        }
        (self.findings.len(), kinds.len())
    }
}

/// Writes the site `import` describes into `folder`, which must be empty
/// or not there, and the report. Where anything stops it, the folder is
/// left as it was found.
pub(super) fn site(
    old: &Path,
    folder: &Path,
    import: Import,
    today: Date,
    shown: &Shown,
) -> Result<Summary, Problem> {
    let undo = Undo::before(folder);
    let written = write(old, folder, import, today, shown, &mut platform::make_file);
    if written.is_err() {
        undo.undo(folder);
    }
    written
}

/// Writes a file of the new site, by its path in it.
pub(super) type Writer<'a> = dyn FnMut(&Path, &str, &[u8]) -> io::Result<()> + 'a;

/// What a run will have to undo: whether the folder was there, and the
/// folders above it that weren't, outermost first.
pub(super) struct Undo {
    there: bool,
    outer: Vec<PathBuf>,
}

impl Undo {
    pub(super) fn before(folder: &Path) -> Undo {
        let mut outer = Vec::new();
        let mut at = Some(folder);
        while let Some(path) = at.filter(|path| !path.as_os_str().is_empty()) {
            if std::fs::symlink_metadata(path).is_ok() {
                break;
            }
            outer.push(path.to_path_buf());
            at = path.parent();
        }
        outer.reverse();
        Undo {
            there: outer.is_empty(),
            outer,
        }
    }

    /// Removes everything in `folder`, which held nothing before, and the
    /// folders made on the way to it.
    pub(super) fn undo(&self, folder: &Path) {
        match self.there {
            true => {
                let Ok(entries) = std::fs::read_dir(folder) else {
                    return;
                };
                for entry in entries.flatten() {
                    let path = entry.path();
                    let _ = match std::fs::symlink_metadata(&path) {
                        Ok(found) if found.is_dir() => platform::remove_folder(&path),
                        _ => std::fs::remove_file(&path),
                    };
                }
            }
            false => {
                let _ = platform::remove_folder(folder);
                for path in self.outer.iter().rev() {
                    let _ = std::fs::remove_dir(path);
                }
            }
        }
    }
}

/// The new site's folder, written into a file at a time. Everything in it
/// is the import's own, so a file is written over by removing it first.
pub(super) struct Out<'a> {
    pub folder: &'a Path,
    write: &'a mut Writer<'a>,
    made: HashSet<String>,
}

impl Out<'_> {
    pub(super) fn put(&mut self, path: &str, bytes: &[u8]) -> Result<(), Problem> {
        let at = || self.folder.join(path);
        let cant = |error: io::Error, path: PathBuf| {
            Problem::nowhere(Kind::CantWrite {
                path,
                reason: Reason::of(&error),
            })
        };
        if !platform::stays_inside(path) {
            return Err(cant(io::Error::from(Reason::OutsideNew), at()));
        }
        if let Some((above, _)) = path.rsplit_once('/') {
            let mut end = 0;
            for piece in above.split('/') {
                end += piece.len();
                let on_the_way = &above[..end];
                end += 1;
                if self.made.contains(on_the_way) {
                    continue;
                }
                match std::fs::symlink_metadata(platform::join(self.folder, on_the_way)) {
                    Ok(found) if found.is_dir() => {}
                    _ => platform::make_folder(self.folder, on_the_way)
                        .map_err(|error| cant(error, self.folder.join(on_the_way)))?,
                }
                self.made.insert(on_the_way.to_owned());
            }
        }
        platform::remove_file(self.folder, path).map_err(|error| cant(error, at()))?;
        (self.write)(self.folder, path, bytes).map_err(|error| cant(error, at()))
    }

    pub(super) fn remove(&mut self, path: &str) -> Result<(), Problem> {
        platform::remove_file(self.folder, path).map_err(|error| {
            Problem::nowhere(Kind::CantWrite {
                path: self.folder.join(path),
                reason: Reason::of(&error),
            })
        })
    }

    /// The text of a file of the new site, where it is there.
    pub(super) fn text(&self, path: &str) -> Option<String> {
        std::fs::read_to_string(platform::join(self.folder, path)).ok()
    }
}

/// [`site`], writing each file with `write`, and not undoing.
pub(super) fn write<'a>(
    old: &Path,
    folder: &'a Path,
    import: Import,
    today: Date,
    shown: &Shown,
    write: &'a mut Writer<'a>,
) -> Result<Summary, Problem> {
    let Import {
        title,
        settings_file,
        address,
        languages,
        timezone,
        maths,
        settings,
        values,
        mut pages,
        mut files,
        written,
        addresses: other_addresses,
        home_pages,
        templated,
        term_pages,
        term_lists,
        feeds,
        built,
        mut findings,
        left,
        notes,
        mut words,
    } = import;
    let languages = match languages.is_empty() {
        true => vec!["en".to_owned()],
        false => languages,
    };
    let other_languages: Vec<String> = languages.iter().skip(1).cloned().collect();
    let old_name = old
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let answers = Answers {
        title: title.unwrap_or_else(|| new::as_words(&old_name)),
        address: address.clone(),
        date_format: new::dates_offered(&languages[0])[0].1.clone(),
        languages,
    };
    let made = new::make(Maker::Import, folder, &answers, timezone.as_deref())?;
    let mut out = Out {
        folder,
        write,
        made: HashSet::new(),
    };
    for link in left
        .iter()
        .filter(|left| super::inside::is_link(&left.what))
    {
        findings.push(Finding {
            kind: &LINKS,
            about: None,
            spots: vec![super::Spot {
                file: String::new(),
                line: None,
                text: format!("{}: {}", super::report::code(&link.path), link.what),
                times: None,
            }],
        });
    }
    let mut summary = Summary {
        left,
        notes,
        built: built.clone(),
        base: address.as_deref().map(base_of).unwrap_or_default(),
        ..Summary::default()
    };
    let followed = super::inside::followed();
    if let Some(first) = followed.first() {
        summary.notes.push(format!(
            "Statac followed {}, {}, which {} to somewhere inside the old site, and read what {} \
             to as standing where the link is.",
            match followed.len() {
                1 => "1 link".to_owned(),
                n => format!("{n} links"),
            },
            match followed.len() {
                1 => super::report::code(first),
                _ => format!("such as {}", super::report::code(first)),
            },
            if followed.len() == 1 { "leads" } else { "lead" },
            if followed.len() == 1 {
                "it leads"
            } else {
                "each leads"
            },
        ));
    }
    summary.home = pages.iter().any(|page| page.to == "index.md");
    let mut text = made.settings.clone();
    text.push_str(&yaml::fields(
        settings.iter().map(|(name, value)| (name.as_str(), value)),
    ));
    let mut own = Vec::with_capacity(values.len());
    for mut value in values {
        if !own_value_reads(&value) {
            let renamed = format!("old_{}", value.name);
            let text = format!("{}: {}", value.name, shown_value(&value.value));
            findings.push(Finding::at(
                &FIELD_RENAMED,
                &settings_file,
                value.line,
                &text,
            ));
            value.name = renamed;
        }
        own.push(value);
    }
    text.push_str(&yaml::fields(
        own.iter().map(|field| (field.name.as_str(), &field.value)),
    ));
    out.put(config::FILE_NAME, text.as_bytes())?;
    summary.settings = text;

    // The home page, and each other language's, shows the newest of each
    // folder itself, so its own list is kept to one page, as a new site's
    // is, where the settings page the lists.
    let paged = settings.iter().any(|(name, value)| {
        name == "posts_per_page" && !matches!(value, Value::Number(n) if n == "0")
    });
    let homes = std::iter::once("").chain(other_languages.iter().map(String::as_str));
    for language in homes.filter(|_| paged) {
        let path = match language {
            "" => "index.md".to_owned(),
            language => format!("{language}/index.md"),
        };
        words.push((
            path,
            Field::new("posts_per_page", Value::Number("0".into()), None),
        ));
    }
    let unanswered_files = writable_names(&mut files, &mut pages, &mut findings);
    let mut taken: HashSet<String> = files
        .iter()
        .map(|file| platform::folded(&file.to))
        .collect();
    let mut terms = Terms::default();
    let came: HashSet<String> = pages.iter().map(|page| page.to.clone()).collect();
    let mut placed = Vec::with_capacity(pages.len());
    for mut page in pages {
        let own_words = words.iter().filter(|(path, _)| *path == page.to);
        let own_words: Vec<Field> = own_words.map(|(_, field)| field.clone()).collect();
        let to = free_name(&page.to, &mut taken);
        if to != page.to {
            findings.push(Finding::at(
                &NAME_TAKEN,
                &page.from,
                None,
                &format!("content/{to}"),
            ));
            page.to = to;
        }
        terms.tag(&mut page);
        page.front = shared_front(&page, &mut findings);
        page.front.extend(own_words);
        let settled = writing::settle(&mut page.writing, &mut page.fences, &mut page.calls, maths);
        for _ in 0..settled.markers_out {
            findings.push(Finding::at(&MORE_OUT, &page.from, None, "<!--more-->"));
        }
        summary.settled.add(&settled);
        match page.kind {
            PageKind::Post => summary.posts += 1,
            PageKind::Draft => summary.drafts += 1,
            PageKind::Page => summary.pages += 1,
        }
        // Empty for a post at the top of `content/`.
        let folder = match page.to.rsplit_once('/') {
            Some((folder, "index.md")) => folder.rsplit_once('/').map_or("", |(above, _)| above),
            Some((folder, _)) => folder,
            None => "",
        };
        if page.kind == PageKind::Post && !summary.post_folders.iter().any(|f| f == folder) {
            summary.post_folders.push(folder.to_owned());
        }
        out.put(&format!("content/{}", page.to), page_text(&page).as_bytes())?;
        placed.push(page);
    }
    for file in &files {
        let bytes = std::fs::read(super::inside::path(old, &file.from)).map_err(|error| {
            Problem::nowhere(Kind::CantRead {
                path: old.join(&file.from),
                reason: Reason::of(&error),
            })
        })?;
        out.put(&file.to, &bytes)?;
    }
    summary.files = files.len();
    for (path, text) in &written {
        out.put(path, text.as_bytes())?;
    }
    // The home page of each language that no page came across to is
    // written with no content, so the site has one.
    let homes = std::iter::once(String::new()).chain(other_languages.iter().cloned());
    for language in homes {
        let path = match language.is_empty() {
            true => "index.md".to_owned(),
            false => format!("{language}/index.md"),
        };
        if !came.contains(&path) && out.text(&format!("content/{path}")).is_none() {
            out.put(&format!("content/{path}"), b"")?;
        }
    }
    // A page the words are for that isn't there is written to hold them.
    for (path, field) in &words {
        if came.contains(path) {
            continue;
        }
        let text = out.text(&format!("content/{path}")).unwrap_or_default();
        let text = with_word(&text, field);
        out.put(&format!("content/{path}"), text.as_bytes())?;
    }

    // The old home page's later pages are sent on to those of the posts'
    // list page, which is published only where the folder has a page.
    if home_pages.is_some() && out.text("content/posts/index.md").is_none() {
        out.put("content/posts/index.md", b"")?;
    }
    summary.kept = addresses::keep(&mut out, &mut placed, today, &other_languages)?;
    // `keep` counts only the pages that came across; a page written to
    // hold words carries the `redirect_from` they give it.
    summary.kept.redirected += (words.iter())
        .filter(|(path, field)| field.name == "redirect_from" && !came.contains(path))
        .filter(|(path, _)| out.text(&format!("content/{path}")).is_some())
        .count();
    let mut worked_out: Vec<(String, String)> = Vec::new();
    for page in &placed {
        let addresses = page.address.iter().chain(&page.other_addresses);
        for address in addresses {
            if !worked_out.iter().any(|(known, _)| known == address) {
                worked_out.push((address.clone(), page.from.clone()));
            }
        }
    }
    for address in &other_addresses {
        if !worked_out.iter().any(|(known, _)| known == address) {
            worked_out.push((address.clone(), String::new()));
        }
    }
    for (address, from) in &templated {
        if !worked_out.iter().any(|(known, _)| known == address) {
            worked_out.push((address.clone(), from.clone()));
        }
    }
    let built_at = built.as_ref().map(|built| super::inside::path(old, built));
    let sent_on = addresses::send_on_to_tags_and_series(
        &mut out,
        today,
        &terms.old_pages(&term_pages),
        &terms.names(),
        built_at.as_deref(),
        &other_languages,
    )?;
    for (address, _) in &sent_on.pages {
        if !worked_out.iter().any(|(known, _)| known == address) {
            worked_out.push((address.clone(), String::new()));
        }
    }
    let mut term_lists = term_lists;
    for list in sent_on.lists {
        if !term_lists.iter().any(|(known, _)| *known == list.0) {
            term_lists.push(list);
        }
    }
    summary.kept.sent_on = sent_on.pages;
    if let Some(home) = &home_pages {
        summary.kept.later = addresses::send_on_to_later_pages(&mut out, today, &placed, home)?;
    }
    if let Some(built) = &built_at {
        let pages = addresses::send_on_to_list_pages(
            &mut out,
            today,
            built,
            &other_languages,
            &sent_on.moved,
        )?;
        for (address, _) in &pages {
            if !worked_out.iter().any(|(known, _)| known == address) {
                worked_out.push((address.clone(), String::new()));
            }
        }
        summary.kept.list_pages = pages;
    }
    summary.terms = terms;
    // A page listing every tag, where one came across, stands where the
    // old site's did.
    for (address, name) in &term_lists {
        if !name.eq_ignore_ascii_case("tags") {
            continue;
        }
        let language =
            (other_languages.iter()).find(|language| address.starts_with(&format!("/{language}/")));
        let (path, at, rest) = match language {
            Some(language) => (
                format!("content/{language}/tags.md"),
                format!("/{language}/tags/"),
                &address[language.len() + 1..],
            ),
            None => (
                "content/tags.md".to_owned(),
                "/tags/".to_owned(),
                address.as_str(),
            ),
        };
        if *address == at {
            continue;
        }
        if let Some((front, _)) = out
            .text(&path)
            .as_deref()
            .and_then(|t| t.rsplit_once("---\n"))
        {
            let line = yaml::fields([("permalink", &Value::Text(rest.to_owned()))]);
            out.put(&path, format!("{front}{line}---\n").as_bytes())?;
        }
    }
    let checked = addresses::check(
        folder,
        today,
        &worked_out,
        &feeds,
        &term_lists,
        built_at.as_deref(),
    );
    summary.kept.pictures = checked.pictures;
    for (address, from) in &checked.feeds {
        findings.push(Finding::at(&FEED_LEFT, from, None, address));
    }
    summary.kept.answered = checked.answered;
    summary.kept.unanswered = checked.unanswered;
    summary.kept.lists = checked.lists;
    summary.kept.files = unanswered_files;
    // An old address that can't be kept is told of once, as that.
    let unkept = &summary.kept.unkept;
    summary.kept.unworked = (checked.unworked.into_iter())
        .filter(|page| {
            !unkept
                .iter()
                .any(|(address, _, _)| address.trim_end_matches('/') == page.trim_end_matches('/'))
        })
        .collect();
    if let Some(built) = &built {
        for (address, from) in &worked_out {
            let file = built_file(address);
            if !platform::stays_inside(&file)
                || !super::inside::a_file(old, &format!("{built}/{file}"))
            {
                findings.push(Finding::at(&NOT_IN_BUILT, from, None, address));
            }
        }
    }
    summary.errors = checked.errors.len();
    summary.templates = checked.templates;
    findings.extend(
        checked
            .errors
            .into_iter()
            .map(|(file, line, text)| Finding {
                kind: &NEW_SITE_ERRORS,
                about: None,
                spots: vec![Spot {
                    file,
                    line,
                    text,
                    times: None,
                }],
            }),
    );
    summary.findings = grouped(findings);
    let document = report::document(shown, &summary);
    out.put(REPORT, document.as_bytes())?;
    Ok(summary)
}

/// Whether an author's own value reads in `statac.yaml` as one of the
/// author's own, by its name, as Statac reads the file.
fn own_value_reads(value: &Field) -> bool {
    let mut sources = crate::source::Sources::default();
    let text = yaml::fields([(value.name.as_str(), &value.value)]);
    let source = sources.add(config::FILE_NAME, text);
    let (settings, problems) = config::read_source(&sources, source);
    problems.is_empty() && settings.own.iter().any(|field| field.name == value.name)
}

/// The path a site at `address` is under, as `/blog`; empty at the top of
/// its host.
fn base_of(address: &str) -> String {
    let past = address.split_once("//").map_or(address, |(_, rest)| rest);
    match past.split_once('/') {
        Some((_, path)) if !path.trim_matches('/').is_empty() => {
            format!("/{}", path.trim_matches('/'))
        }
        _ => String::new(),
    }
}

/// The file a tool writes for `address` in its built output: a folder's
/// `index.html`, a file by its name, and a name with no extension as an
/// HTML file.
fn built_file(address: &str) -> String {
    let path = address.trim_start_matches('/');
    if path.is_empty() || path.ends_with('/') {
        return format!("{path}index.html");
    }
    let last = path.rsplit('/').next().unwrap_or(path);
    match last.contains('.') {
        true => path.to_owned(),
        false => format!("{path}.html"),
    }
}

/// `piece` with each `%` and two figures as the character they spell, as
/// a browser reads an address.
fn spelled_out(piece: &str) -> String {
    let bytes = piece.as_bytes();
    let mut spelled = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let figure = |i: usize| bytes.get(i).and_then(|b| char::from(*b).to_digit(16));
        match (bytes[at], figure(at + 1), figure(at + 2)) {
            (b'%', Some(high), Some(low)) => {
                spelled.push((high * 16 + low) as u8);
                at += 3;
            }
            (byte, _, _) => {
                spelled.push(byte);
                at += 1;
            }
        }
    }
    String::from_utf8(spelled).unwrap_or_else(|_| piece.to_owned())
}

/// `piece` as an address spells it in full: every byte but a letter, a
/// figure and `-._~` as `%` and two figures.
fn in_full_of(piece: &str) -> String {
    let mut out = String::with_capacity(piece.len());
    for byte in piece.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(char::from(byte))
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// `piece`, a file's or a folder's name, as one Statac will publish:
/// [`spelled_out`], then `-` for each character an address or a disk
/// won't take.
fn writable(piece: &str) -> String {
    let mut name: String = spelled_out(piece)
        .chars()
        .map(|c| match c {
            '/' | '%' | '#' => '-',
            c if c.is_control() => '-',
            c if address::name_fault(&c.to_string()).is_some() && c != '.' && c != ' ' => '-',
            c => c,
        })
        .collect();
    name.truncate(name.trim_end_matches(['.', ' ']).len());
    match address::name_fault(&name).is_some() || name.is_empty() {
        true => format!("_{name}"),
        false => name,
    }
}

/// Gives each file whose name Statac can't publish one it can, changes
/// the places in the writing that spell the old name some other way than
/// the new one is spelled, and tells of each. Gives each such file's old
/// address, which nothing answers now, with the old file.
fn writable_names(
    files: &mut [File],
    pages: &mut [Page],
    findings: &mut Vec<Finding>,
) -> Vec<(String, String)> {
    let mut unanswered = Vec::new();
    let mut taken: HashSet<String> = files
        .iter()
        .map(|file| platform::folded(&file.to))
        .collect();
    for file in files.iter_mut() {
        let to = file.to.clone();
        let faulty = |piece: &str| address::name_fault(piece).is_some();
        if !to.split('/').any(faulty) {
            continue;
        }
        let pieces: Vec<String> = to
            .split('/')
            .map(|piece| match faulty(piece) {
                true => writable(piece),
                false => piece.to_owned(),
            })
            .collect();
        let mut new = pieces.join("/");
        let (stem, ending) = match new.rsplit_once('.') {
            Some((stem, ending)) if !ending.contains('/') => {
                (stem.to_owned(), format!(".{ending}"))
            }
            _ => (new.clone(), String::new()),
        };
        let mut number = 1;
        while !taken.insert(platform::folded(&new)) {
            number += 1;
            new = format!("{stem}-{number}{ending}");
        }
        let (folder, old_name) = to.rsplit_once('/').unwrap_or(("", &to));
        let new_name = new.rsplit('/').next().unwrap_or(&new).to_owned();
        // A file beside a page is named by its name alone there; one
        // published as it is, by its address from anywhere.
        let beside = folder
            .strip_prefix("content/")
            .or(folder.strip_prefix("content"));
        let strict = old_name.replace('%', "%25");
        let in_full = in_full_of(old_name);
        for page in pages.iter_mut() {
            let page_folder = page.to.rsplit_once('/').map_or("", |(above, _)| above);
            if beside.is_some_and(|beside| beside != page_folder) {
                continue;
            }
            for spelling in [in_full.as_str(), strict.as_str(), old_name] {
                respell(page, spelling, &new_name);
            }
        }
        let old_address = match beside {
            Some(beside) => {
                format!("{}{in_full}", beside_address(pages, beside))
            }
            None => {
                let path = to.split_once('/').map_or(to.as_str(), |(_, path)| path);
                let pieces: Vec<String> = path.split('/').map(in_full_of).collect();
                format!("/{}", pieces.join("/"))
            }
        };
        unanswered.push((old_address, file.from.clone()));
        findings.push(Finding::at(&FILE_RENAMED, &file.from, None, &new));
        file.to = new;
    }
    unanswered
}

/// The address the old site published the files of `folder`, a folder
/// under `content/`, beneath: that of the folder's own page, where it has
/// one, and else the folder's path.
fn beside_address(pages: &[Page], folder: &str) -> String {
    let page = pages.iter().find(|page| {
        page.to.rsplit_once('/').map_or("", |(above, _)| above) == folder
            && page.to.ends_with("index.md")
    });
    match page.and_then(|page| page.address.as_deref()) {
        Some(address) => address[..=address.rfind('/').unwrap_or(0)].to_owned(),
        None => format!("/{folder}/"),
    }
}

/// Changes each place `page`'s writing spells a file's name as `spelling`
/// to the way an address spells `name`, unless a browser reads the
/// spelling as that name already. The page's marks move with it.
fn respell(page: &mut Page, spelling: &str, name: &str) {
    let wire = address::on_the_wire(name);
    if spelled_out(spelling) == name || spelling == wire {
        return;
    }
    let mut from = 0;
    while let Some(found) = page.writing[from..].find(spelling) {
        let at = from + found;
        page.writing.replace_range(at..at + spelling.len(), &wire);
        for mark in page.calls.iter_mut().chain(page.fences.iter_mut()) {
            if *mark > at {
                *mark = *mark + wire.len() - spelling.len();
            }
        }
        from = at + wire.len();
    }
}

/// `to`, or where a file has its name already, the same with a number
/// after it, which is then taken.
fn free_name(to: &str, taken: &mut HashSet<String>) -> String {
    let (stem, ending) = match to.rsplit_once('.') {
        Some((stem, ending)) if !ending.contains('/') => (stem, format!(".{ending}")),
        _ => (to, String::new()),
    };
    let mut name = to.to_owned();
    let mut number = 1;
    while !taken.insert(platform::folded(&format!("content/{name}"))) {
        number += 1;
        name = format!("{stem}-{number}{ending}");
    }
    name
}

/// The findings with those of a kind together, the kinds in the order
/// each first came.
fn grouped(findings: Vec<Finding>) -> Vec<Finding> {
    let mut kinds: Vec<&'static FindingKind> = Vec::new();
    for finding in &findings {
        if !kinds.iter().any(|kind| std::ptr::eq(*kind, finding.kind)) {
            kinds.push(finding.kind);
        }
    }
    let mut out = Vec::with_capacity(findings.len());
    for kind in kinds {
        out.extend(
            findings
                .iter()
                .filter(|finding| std::ptr::eq(finding.kind, kind))
                .cloned(),
        );
    }
    out
}

/// A file of the new site with `field` in its front matter, in place of
/// any line of that name, or as front matter of its own where it has none.
fn with_word(text: &str, field: &Field) -> String {
    let line = yaml::fields([(field.name.as_str(), &field.value)]);
    let Some((front, writing)) = text.strip_prefix("---\n").and_then(|rest| {
        let end = match rest.starts_with("---\n") {
            true => 0,
            false => rest.find("\n---\n")? + 1,
        };
        Some((&rest[..end], &rest[end + 4..]))
    }) else {
        return format!("---\n{line}---\n{text}");
    };
    let mut kept = String::new();
    let mut within = false;
    for written in front.lines() {
        if written.starts_with(&format!("{}:", field.name)) {
            within = true;
            continue;
        }
        if within && written.starts_with([' ', '-']) {
            continue;
        }
        within = false;
        kept.push_str(written);
        kept.push('\n');
    }
    format!("---\n{kept}{line}---\n{writing}")
}

/// A page's file: its front matter, where it has any, then its writing.
pub(super) fn page_text(page: &Page) -> String {
    if page.front.is_empty() {
        return page.writing.clone();
    }
    let fields = yaml::fields(
        page.front
            .iter()
            .map(|field| (field.name.as_str(), &field.value)),
    );
    format!("---\n{fields}---\n{}", page.writing)
}

/// A page's front matter with the shared rules kept: each field that
/// would collide with one of Statac's words renamed, each of Statac's
/// words that holds no text left out, and `date` and
/// `updated` written as ISO 8601 where they have one reading.
fn shared_front(page: &Page, findings: &mut Vec<Finding>) -> Vec<Field> {
    let names: Vec<String> = page.front.iter().map(|field| field.name.clone()).collect();
    let mut front = Vec::with_capacity(page.front.len());
    for field in &page.front {
        let mut field = field.clone();
        // One of Statac's words holding no text says nothing, and some,
        // as `summary`, then show nothing where Statac would have shown
        // what it works out itself.
        if SAME_MEANING.contains(&field.name.as_str())
            && matches!(&field.value, Value::Text(text) if text.trim().is_empty())
        {
            continue;
        }
        if collides(&field.name, &field.value) {
            let mut name = format!("old_{}", field.name);
            while names.contains(&name) {
                name = format!("old_{name}");
            }
            let text = format!("{}: {}", field.name, shown_value(&field.value));
            findings.push(Finding::at(&FIELD_RENAMED, &page.from, field.line, &text));
            field.name = name;
        } else if matches!(field.name.as_str(), "date" | "updated") {
            let written = match &field.value {
                Value::Text(text) | Value::Number(text) => text.clone(),
                other => shown_value(other),
            };
            match date_of(&written) {
                Some(date) => field.value = Value::Text(date.to_string()),
                None => {
                    let text = format!("{}: {written}", field.name);
                    findings.push(Finding::at(&DATE_LEFT, &page.from, field.line, &text));
                }
            }
        }
        front.push(field);
    }
    front
}

/// The moment a date means as a tool wrote it: ISO 8601, or with its day
/// in English words. None where it has no one reading.
pub(super) fn date_of(written: &str) -> Option<Date> {
    crate::date::read(written)
        .ok()
        .or_else(|| crate::date::read_words(written))
}

/// Whether a field named `name` holding `value` would mean something else
/// to Statac than it meant to the author.
pub(super) fn collides(name: &str, value: &Value) -> bool {
    let words_of_statacs = words::is_statacs(name) && !SAME_MEANING.contains(&name);
    let other_tools = OTHER_SPELLINGS.iter().any(|(other, _)| *other == name);
    let placed = name == "pages"
        || [
            "pages.",
            "site.",
            "default.",
            "tags.",
            "series.",
            "pagination.",
        ]
        .iter()
        .any(|start| name.starts_with(start));
    words_of_statacs
        || other_tools
        || placed
        || STATAC_FILLS.contains(&name)
        || (SAME_MEANING.contains(&name) && !fits(name, value))
}

/// Whether `value` is the kind of value Statac's word `name` holds.
fn fits(name: &str, value: &Value) -> bool {
    let text = |value: &Value| matches!(value, Value::Text(_) | Value::Number(_));
    match name {
        "tags" | "redirect_from" => match value {
            Value::List(items) => items.iter().all(text),
            other => text(other),
        },
        "draft" => matches!(value, Value::Truth(_)),
        "date" | "updated" => text(value),
        _ => text(value),
    }
}

/// A value as a report quotes it: as one line of YAML.
pub(super) fn shown_value(value: &Value) -> String {
    match value {
        Value::Text(text) => crate::yaml::write::text(text),
        Value::Number(number) => number.clone(),
        Value::Truth(truth) => truth.to_string(),
        Value::Nothing => "null".into(),
        Value::List(items) => {
            let items: Vec<String> = items.iter().map(shown_value).collect();
            format!("[{}]", items.join(", "))
        }
        Value::Fields(fields) => {
            let fields: Vec<String> = fields
                .iter()
                .map(|(name, value)| format!("{name}: {}", shown_value(value)))
                .collect();
            format!("{{{}}}", fields.join(", "))
        }
    }
}
