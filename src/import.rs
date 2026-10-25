//! `statac import`: a new Statac site written beside a site another tool
//! made, which is only ever read.
//!
//! # What is shared, and what is a reader's
//!
//! Each tool has a reader, a module of its own beside this one. A reader
//! knows one tool and nothing of Statac's files: it says whether a folder
//! is its tool's ([`Tool::owns`]), and turns the folder into one
//! [`Import`]. The shared part knows Statac and nothing of any tool: it
//! writes the [`Import`] out as a site, keeps its addresses, checks them,
//! and reports.
//!
//! A reader must:
//!
//! - read only, and never write, into the old folder;
//! - read nothing outside the old folder and follow no link: leave each
//!   link a walk meets, and ask `inside` before opening anything a
//!   setting or the writing names;
//! - give every page its place under `content/`, its front matter in
//!   Statac's words where the tool has an equal (`draft`, `updated`,
//!   `summary`, `tags` as a list, `redirect_to`), its writing with the
//!   tool's template tags converted where they have an equal, and the
//!   address the tool published it at, worked out by the tool's rules;
//! - say where in a page's writing it wrote a fenced code block or a
//!   snippet call, in [`Page::fences`] and [`Page::calls`]: the shared
//!   part sees that Statac reads each fence as one, and escapes
//!   everything else in the writing that Statac would read as a call;
//! - take a page's names in each list of terms the tool keeps, other than
//!   its tags, out of its front matter and into [`Page::terms`], and give
//!   where the tool published a page for each in [`Import::term_pages`]:
//!   the shared part makes each a tag and sends readers on;
//! - take a page's own address and its old addresses out of its front
//!   matter, into [`Page::address`], [`Page::own_address`] and
//!   [`Page::other_addresses`]: the shared part writes `permalink` and
//!   `redirect_from` itself;
//! - give dates as the tool wrote them, as text: the shared part writes
//!   `date` and `updated` as ISO 8601, one with its day in English words
//!   among them, and tells of one it can't;
//! - say of everything it leaves behind or can't convert: a [`Finding`]
//!   of a kind of its own, with the old file, the line and the text, or a
//!   [`Left`] for a whole file or folder it never read;
//! - word its findings and notes by Statac's wording rules, naming the
//!   tool where the author needs it.
//!
//! A reader must not:
//!
//! - rename a field that collides with one of Statac's words, write
//!   `permalink` or `redirect_from` on a page that came across, or turn a
//!   date into ISO 8601: those are the shared part's, so every tool's
//!   sites come out alike;
//! - escape a call in the writing, or put blank lines round HTML: how
//!   Statac's Markdown reads writing is the shared part's to know;
//! - copy the tool's templates or theme: the new site has none;
//! - guess silently: where the tool's rule is unclear, it says so as a
//!   finding or a note.
//!
//! The shared part makes the site with `new::make`, makes each page's
//! writing read in Statac as it showed in the
//! old site where a blank line or a backslash does it (see `writing`),
//! writes every page, file and setting, keeps each old address
//! by a `pages.permalink` pattern in a folder's `index.md`, a page's own
//! `permalink`, or `redirect_from`, then reads the new site with Statac's
//! own code and tells of each old address nothing answers, of each one
//! the tool's own built output (where the reader names one) doesn't
//! hold, and of each page that output holds at any other address that
//! nothing answers. It checks the new site as `statac check` does, and each error
//! found is a finding. Last it writes `IMPORT.md`. Anything that stops it part way
//! leaves the new folder as it was found.

mod addresses;
mod common;
mod eleventy;
mod hugo;
mod inside;
mod jekyll;
mod liquid;
mod mend;
mod report;
#[cfg(test)]
mod tests;
pub mod toml;
mod write;
mod writing;
pub mod yaml;
mod zola;

use std::io::Write;
use std::path::{Path, PathBuf};

use crate::date::Date;
use crate::new::{Asking, Input};
use crate::problem::{Kind, Maker, Problem, Reason};
use crate::report::Report;
use crate::source::Sources;
use crate::wording;
pub use writing::Maths;
pub use yaml::Value;

/// What `statac import` is asked.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Options {
    /// The old site.
    pub old: PathBuf,
    /// Where the new site is made; beside the old, its name with
    /// `-statac`, where none is given.
    pub new: Option<PathBuf>,
    /// The tool the author says made the old site.
    pub from: Option<Tool>,
    /// The day the import is made on; today on this machine's clock where
    /// none is given.
    pub today: Option<Date>,
}

/// A tool Statac has a reader for.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Tool {
    Jekyll,
    Hugo,
    Zola,
    Eleventy,
}

impl Tool {
    pub const ALL: &[Tool] = &[Tool::Jekyll, Tool::Hugo, Tool::Zola, Tool::Eleventy];

    /// The tool's name in `--from`.
    pub fn word(self) -> &'static str {
        match self {
            Tool::Jekyll => "jekyll",
            Tool::Hugo => "hugo",
            Tool::Zola => "zola",
            Tool::Eleventy => "eleventy",
        }
    }

    /// The tool's name in a sentence.
    pub fn name(self) -> &'static str {
        match self {
            Tool::Jekyll => "Jekyll",
            Tool::Hugo => "Hugo",
            Tool::Zola => "Zola",
            Tool::Eleventy => "Eleventy",
        }
    }

    pub fn from_word(word: &str) -> Option<Tool> {
        Tool::ALL.iter().copied().find(|tool| tool.word() == word)
    }

    /// Whether `folder` holds a site this tool made, by its files.
    pub fn owns(self, folder: &Path) -> bool {
        match self {
            Tool::Jekyll => jekyll::owns(folder),
            Tool::Hugo => hugo::owns(folder),
            Tool::Zola => zola::owns(folder),
            Tool::Eleventy => eleventy::owns(folder),
        }
    }

    /// Reads the old site in `folder`, taking `given` for its address
    /// where its settings give none.
    fn read(self, folder: &Path, today: Date, given: Option<&str>) -> Result<Import, Box<Failed>> {
        match self {
            Tool::Jekyll => jekyll::read(folder, today, given),
            Tool::Hugo => hugo::read(folder, today, given),
            Tool::Zola => zola::read(folder, today, given),
            Tool::Eleventy => eleventy::read(folder, today, given),
        }
    }
}

/// What a reader makes of an old site: everything the new site should
/// hold, and everything that didn't come across.
#[derive(Clone, Default, Debug)]
pub struct Import {
    /// The tool's settings file, by its path in the old site.
    pub settings_file: String,
    pub title: Option<String>,
    /// Where the site is published, whole, with any path it is under, and
    /// no `/` at its end.
    pub address: Option<String>,
    /// The site's languages, the main one first; `en` where none is given.
    pub languages: Vec<String>,
    /// A name from the time zone database.
    pub timezone: Option<String>,
    /// Whether the tool set maths written between dollar signs: where it
    /// didn't, the shared part escapes each dollar sign Statac would read
    /// as opening maths.
    pub maths: Maths,
    /// Lines for `statac.yaml` besides the title, address, languages and
    /// time zone, in order: Statac's own settings.
    pub settings: Vec<(String, Value)>,
    /// The author's own values in the settings, for `statac.yaml` as
    /// values of the author's own, in order.
    pub values: Vec<Field>,
    pub pages: Vec<Page>,
    pub files: Vec<File>,
    /// Files the reader writes whole, by their paths in the new site,
    /// such as a snippet standing for something that didn't convert.
    pub written: Vec<(String, String)>,
    /// Old addresses that belong to no one page, such as a list's later
    /// pages, from the top of the site.
    pub addresses: Vec<String>,
    /// The later pages of the old home page, where it showed the posts
    /// now in `content/posts/` a few at a time.
    pub home_pages: Option<HomePages>,
    /// The addresses of the pages left behind as the work of the old
    /// site's templates, each with the old file it was made from.
    pub templated: Vec<(String, String)>,
    /// The address of the page the old site made for each name in a list
    /// of terms, with the list's name and the name: `categories`, `linux`
    /// and `/categories/linux/`; and for a tag, as `tags`, where it isn't
    /// where Statac's would be.
    pub term_pages: Vec<(String, String, String)>,
    /// The address of each page the old site made to list every tag, or
    /// every name of another list of that kind, with the list's name:
    /// `/tags/` and `tags`.
    pub term_lists: Vec<(String, String)>,
    /// Addresses the old site published a feed at that its templates
    /// made, each with the old file it was made from.
    pub feeds: Vec<(String, String)>,
    /// The tool's own built output, by its path in the old site, where
    /// it is there.
    pub built: Option<String>,
    pub findings: Vec<Finding>,
    pub left: Vec<Left>,
    /// Sentences for the report on what the reader did with the settings
    /// and the front matter, in Markdown.
    pub notes: Vec<String>,
    /// Lines of front matter in Statac's own words, meant as Statac means
    /// them, such as a list's `feed` or `order`, by the path under
    /// `content/` of the page they go on: one that came across, or else one
    /// written to hold them. They are written as they are, after the
    /// page's own.
    pub words: Vec<(String, Field)>,
}

/// The later pages of an old home page that showed the site's posts a
/// few at a time, the newest first.
#[derive(Clone, Debug)]
pub struct HomePages {
    /// How many posts each page showed.
    pub per_page: usize,
    /// The address of each page after the first, page 2 first.
    pub addresses: Vec<String>,
    /// Whether posts of one moment stood in the order of their old files'
    /// paths turned round, as Jekyll puts them.
    pub ties_reversed: bool,
}

/// What a page is, for the counts.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum PageKind {
    Post,
    Draft,
    Page,
}

/// A page of the old site, as it goes into the new.
#[derive(Clone, Debug)]
pub struct Page {
    /// Its path in the old site.
    pub from: String,
    /// Its path under `content/`.
    pub to: String,
    pub kind: PageKind,
    /// Its front matter, in the author's order.
    pub front: Vec<Field>,
    pub writing: String,
    /// The page's names in each list of terms the tool kept that Statac has
    /// no equal for, such as `categories`, by the list's name as the site
    /// wrote it; and in `series`, where the page names more than one. The
    /// shared part makes each a tag.
    pub terms: Vec<(String, Vec<String>)>,
    /// Where the reader wrote a fenced code block into the writing, by the
    /// offset each fence starts at, in order.
    pub fences: Vec<usize>,
    /// Where the reader wrote a snippet call into the writing, and the
    /// `@end` of one that wraps writing, by the offset of each one's `@`,
    /// in order.
    pub calls: Vec<usize>,
    /// Where the old tool published it, from the top of the site; none
    /// where it wasn't published.
    pub address: Option<String>,
    /// Whether that address is the page's own choice, as a permalink in
    /// its front matter is, rather than its folder's rule.
    pub own_address: bool,
    /// Other addresses that led to it.
    pub other_addresses: Vec<String>,
}

/// A field of front matter.
#[derive(Clone, PartialEq, Debug)]
pub struct Field {
    pub name: String,
    pub value: Value,
    /// Its line in the old file, where it was written there.
    pub line: Option<usize>,
}

impl Field {
    pub fn new(name: &str, value: Value, line: Option<usize>) -> Field {
        Field {
            name: name.to_owned(),
            value,
            line,
        }
    }
}

/// A file copied as it is.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct File {
    /// Its path in the old site.
    pub from: String,
    /// Its path in the new site, such as `static/photo-1.jpg`.
    pub to: String,
}

/// A kind of thing that didn't convert: its heading in the report, and
/// one sentence on what it means and what to do.
#[derive(Debug, PartialEq, Eq)]
pub struct FindingKind {
    pub heading: &'static str,
    pub sentence: &'static str,
}

/// Something that didn't convert, at the places it is.
#[derive(Clone, Debug)]
pub struct Finding {
    pub kind: &'static FindingKind,
    /// What the places share, such as the snippet every call names.
    pub about: Option<String>,
    pub spots: Vec<Spot>,
}

/// A place in the old site.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Spot {
    pub file: String,
    pub line: Option<usize>,
    /// The text there, exactly; or, where the spot names no file and
    /// counts no places, a line of the report as it stands.
    pub text: String,
    /// How many places hold the text, where the spot stands for all of
    /// them and names no file.
    pub times: Option<usize>,
}

impl Finding {
    pub fn at(kind: &'static FindingKind, file: &str, line: Option<usize>, text: &str) -> Finding {
        Finding {
            kind,
            about: None,
            spots: vec![Spot {
                file: file.to_owned(),
                line,
                text: text.to_owned(),
                times: None,
            }],
        }
    }
}

/// Why a reader couldn't read the old site, with the files the problem
/// is placed in.
pub struct Failed {
    pub sources: Sources,
    pub problem: Problem,
}

impl From<Problem> for Box<Failed> {
    fn from(problem: Problem) -> Box<Failed> {
        Box::new(Failed {
            sources: Sources::default(),
            problem,
        })
    }
}

/// A file or folder of the old site that wasn't converted at all.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Left {
    /// Its path in the old site, a folder's ending with `/`.
    pub path: String,
    /// What it is, as a sentence's end: `the old site's templates`.
    pub what: String,
}

/// What a reader says of the new home page's own list, kept to one page.
/// Where the settings give the lists `posts_per_page`, the home page says
/// `0`, and otherwise nothing.
fn one_home_page(paged: bool) -> &'static str {
    match paged {
        true => "`content/index.md` keeps its own list to one page with `posts_per_page: 0`",
        false => "`content/index.md` gives no `posts_per_page`, so its own list stays on one page",
    }
}

/// Imports the site `options` names, saying what it did on `out`, and
/// asking on `input` for the site's address where the old settings give
/// none. Gives the exit code: 0 where the site was made, 1 where it wasn't.
pub fn run(options: &Options, input: Option<Input>, out: &mut dyn Write, colour: bool) -> u8 {
    let mut say = |text: &str| {
        let _ = out.write_all(text.as_bytes());
        let _ = out.flush();
    };
    let sources = Sources::default();
    let report = Report::new(&sources, Path::new(""), false);
    let old_shown = options.old.to_string_lossy().into_owned();
    let new = options.new.clone().unwrap_or_else(|| beside(&options.old));
    let new_shown = new.to_string_lossy().into_owned();
    let today = options
        .today
        .unwrap_or_else(crate::new::today_on_this_machine);
    inside::begin(&new);
    let tool = match refusal(&options.old, &new, &new_shown).and_then(|()| tool(options)) {
        Ok(tool) => tool,
        Err(problem) => {
            say(&report.problem(&problem));
            return 1;
        }
    };
    // The old site is read once with no link followed, which says what
    // it published; where a link waits on that, it is read again.
    let mut read = tool.read(&options.old, today, None);
    if let Ok(first) = &read
        && inside::waiting()
    {
        let pages = (first.pages.iter())
            .filter(|page| page.kind != PageKind::Draft)
            .map(|page| page.from.as_str());
        let files = (first.files.iter())
            .filter(|file| file.to.starts_with("content/") || file.to.starts_with("static/"))
            .map(|file| file.from.as_str());
        let published: Vec<&str> = pages.chain(files).collect();
        inside::read_once(&options.old, &published);
        read = tool.read(&options.old, today, None);
    }
    if let (Ok(import), Some(input)) = (&read, input)
        && import.address.is_none()
    {
        let mut asking = Asking {
            reader: input.reader,
            say: &mut say,
            colour,
            terminal: input.terminal,
        };
        let hints = [
            wording::ASK_OLD_ADDRESS_HINT.to_owned(),
            wording::ASK_ADDRESS_HINT.to_owned(),
        ];
        match asking.address(wording::ASK_OLD_ADDRESS, &hints) {
            Ok(Some(address)) => read = tool.read(&options.old, today, Some(&address)),
            Ok(None) => {}
            Err(told) => {
                say(&format!("{}\n", crate::report::shown(&told)));
                return 1;
            }
        }
    }
    let import = match read {
        Ok(import) => import,
        Err(failed) => {
            let Failed { sources, problem } = *failed;
            let report = Report::new(&sources, Path::new(""), false);
            say(&report.problem(&problem));
            return 1;
        }
    };
    let shown = report::Shown {
        tool: tool.name(),
        old: &old_shown,
        new: &new_shown,
    };
    match write::site(&options.old, &new, import, today, &shown) {
        Ok(summary) => {
            for line in report::said(&shown, &summary).lines() {
                say(&format!("{}\n", crate::report::shown(line)));
            }
            0
        }
        Err(problem) => {
            say(&report.problem(&problem));
            1
        }
    }
}

/// `given`, an address the author gave for the site, with `path`, the path
/// the old settings put the site under, at its end where the address has
/// no path of its own.
fn under(given: &str, path: &str) -> String {
    let host = given.split_once("//").map_or(given, |(_, rest)| rest);
    match host.contains('/') {
        true => given.to_owned(),
        false => format!("{given}{path}"),
    }
}

/// The new site's folder where none is given: beside `old`, named as it
/// is with `-statac`.
fn beside(old: &Path) -> PathBuf {
    let name = match old.file_name() {
        Some(name) => name.to_string_lossy().into_owned(),
        None => std::fs::canonicalize(old)
            .ok()
            .and_then(|whole| whole.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "site".to_owned()),
    };
    let new = format!("{name}-statac");
    match old.file_name() {
        Some(_) => old.with_file_name(new),
        None => match std::fs::canonicalize(old)
            .ok()
            .and_then(|w| w.parent().map(Path::to_path_buf))
        {
            Some(parent) => parent.join(new),
            None => PathBuf::from(new),
        },
    }
}

/// Why nothing can be imported from `old` into `new`, if nothing can.
fn refusal(old: &Path, new: &Path, new_shown: &str) -> Result<(), Problem> {
    let cant_read = |reason: Reason| {
        Problem::nowhere(Kind::CantRead {
            path: old.to_path_buf(),
            reason,
        })
    };
    match std::fs::metadata(old) {
        Ok(found) if found.is_dir() => {}
        Ok(_) => return Err(cant_read(Reason::NotAFolder)),
        Err(error) => return Err(cant_read(Reason::of(&error))),
    }
    let whole_old = std::fs::canonicalize(old).map_err(|error| cant_read(Reason::of(&error)))?;
    let whole_new = crate::platform::whole_to_be(new);
    let inside = |reason: Reason| {
        Problem::nowhere(Kind::CantWrite {
            path: new.to_path_buf(),
            reason,
        })
    };
    if whole_new.starts_with(&whole_old) {
        return Err(inside(Reason::InsideOld(old.to_path_buf())));
    }
    if whole_old.starts_with(&whole_new) {
        return Err(inside(Reason::HoldsOld(old.to_path_buf())));
    }
    crate::new::refusal(new, new_shown, Maker::Import)
}

/// The tool the author named, or the one whose files the old site has.
fn tool(options: &Options) -> Result<Tool, Problem> {
    if let Some(tool) = options.from {
        return Ok(tool);
    }
    let owners: Vec<Tool> = Tool::ALL
        .iter()
        .copied()
        .filter(|tool| tool.owns(&options.old))
        .collect();
    match owners.as_slice() {
        [one] => Ok(*one),
        many => Err(Problem::nowhere(Kind::CantTellWhatMade {
            folder: options.old.to_string_lossy().into_owned(),
            several: !many.is_empty(),
            tools: Tool::ALL
                .iter()
                .map(|tool| tool.word().to_owned())
                .collect(),
        })),
    }
}
