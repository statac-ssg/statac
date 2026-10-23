//! `content/` read into the site: every page with its words, handed down
//! nearest first, and the address it lands at.
//!
//! A folder holding pages is a list, and its `index.md` is the folder's own
//! page. A folder holding only an `index.md` and its files is one page,
//! listed in the folder above, and its files travel with it. Other files
//! are published beside the writing.

pub mod address;
mod made;
pub mod slug;
#[cfg(test)]
mod tests;
mod texts;
mod translations;
pub mod words;

use std::collections::{HashMap, HashSet};
use std::path::Path;

use rayon::prelude::*;

use crate::config::{self, Settings};
use crate::date::{self, Date};
use crate::dates::Dates;
use crate::origin::{Arrival, Origin, Sourced, Style};
use crate::problem::{
    AddressFault, Held, Kind, Named, Problem, PublishedAsText, Requirer, Root, SetAside,
};
use crate::site::{
    Id, Language, LanguageId, List, ListId, ListKey, Page, PageId, PagePlace, Redirect, SentTo,
    Site, Travelling,
};
use crate::source::{Place, SourceId, Sources, Span};
use crate::value::Value;
use crate::yaml::Node;
use address::{Address, Pieces, Top};
pub(crate) use texts::Texts;
use words::{Holder, PageWord, Words};

/// What a run asks of reading the site.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct Run {
    /// The build's day, `site.today`; today in the site's zone where none
    /// is given.
    pub today: Option<Date>,
    /// Whether drafts and posts waiting for their day are built as if
    /// published.
    pub drafts: bool,
    /// The site's address for this run, in place of `url`, where one was
    /// given.
    pub url: Option<String>,
    /// Files and folders under `content/` read as if renamed, each from its
    /// path there to the path it is read at.
    pub renamed: Vec<(String, String)>,
}

/// What reading a site found.
pub struct Read {
    pub sources: Sources,
    pub settings: Settings,
    pub site: Site,
    pub problems: Vec<Problem>,
    /// Pages left out of the build as drafts.
    pub drafts: Vec<SourceId>,
    /// Posts left out of the build until their day.
    pub waiting: Vec<SourceId>,
    /// The drafts and posts waiting for their day built as if published.
    pub included: Included,
    /// How the site shows its dates.
    pub dates: Dates,
    /// The files describing pictures, by their paths under `content/`,
    /// which are never published.
    pub sidecars: Vec<String>,
    /// The files beside pages left out of the build, by their paths under
    /// `content/`, left out with them.
    pub left_out: Vec<String>,
}

/// How many drafts and posts waiting for their day `--drafts` built.
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct Included {
    pub drafts: usize,
    pub waiting: usize,
}

impl Read {
    /// What the site publishes from `content/`: each file's path in the
    /// published site, and what it comes from, for finding two at one
    /// address alongside the assets'.
    pub fn claims(&self) -> Vec<(String, Named)> {
        claims(&self.site, &self.sources, &self.settings)
    }
}

/// What `site` publishes from `content/`: each file's path in the
/// published site, and what it comes from.
pub fn claims(site: &Site, sources: &Sources, settings: &Settings) -> Vec<(String, Named)> {
    let files = site.files.iter().map(|file| {
        let path = format!("content/{}", file.path);
        (file.address.file(), Named::File(path))
    });
    let pages = site
        .pages
        .iter()
        .filter(|(_, page)| page.lands())
        .map(|(id, page)| {
            let named = site.named(id, sources);
            // Only a page's own `permalink`: one its folder gives all its
            // pages is changed for them all.
            let own = (page.words.get(PageWord::Permalink))
                .filter(|permalink| permalink.origin.arrival == Arrival::Written)
                .and_then(|permalink| permalink.origin.place)
                .filter(|at| Some(at.source) == page.source);
            let named = match own {
                Some(at) => Named::Permalink {
                    page: Box::new(named),
                    at,
                },
                None => named,
            };
            (page.address.file(), named)
        });
    let redirects = site.redirects.iter().map(|redirect| {
        let named = site.named(redirect.page, sources);
        // A page sending readers on stands at its own address; any other
        // is one of its old ones.
        let named = match redirect.from == site.pages.get(redirect.page).address {
            true => named,
            false => Named::Redirect {
                page: Box::new(named),
                at: redirect.place,
            },
        };
        (redirect.from.file(), named)
    });
    // A page of the author's standing where a later page goes is told
    // with the lists, as that.
    let landed: HashSet<String> = site
        .pages
        .iter()
        .filter(|(_, page)| page.lands())
        .map(|(_, page)| page.address.file())
        .collect();
    let base = settings.base();
    let later = site.lists.iter().flat_map(|(_, list)| {
        let first = list
            .page
            .map(|page| format!("{base}{}", site.pages.get(page).address.as_str()));
        (2..)
            .zip(&list.later)
            .map(|(number, address)| (number, address.file()))
            .filter(|(_, file)| !landed.contains(file))
            .filter_map(move |(number, file)| {
                let list = first.clone()?;
                Some((file, Named::later(number, list)))
            })
    });
    files.chain(pages).chain(redirects).chain(later).collect()
}

/// Reads the settings and content of the site at `root`, and its lists.
pub fn read(root: &Path, run: &Run) -> Read {
    read_with(root, run, &mut Texts::default())
}

/// [`read`], taking from `texts` the text of each file that may be a
/// page as an earlier reading kept it, where it can, and keeping there
/// what this reading read.
pub(crate) fn read_with(root: &Path, run: &Run, texts: &mut Texts) -> Read {
    texts.begin(root);
    let mut sources = Sources::over(texts.unsaved().clone());
    let (mut settings, mut problems) = config::read(root, &mut sources);
    if let Some(address) = &run.url {
        settings.at(address);
    }
    if settings.languages().is_empty() && !settings.language_told {
        problems.push(Problem::nowhere(Kind::NoLanguage));
    }
    let (dates, found) = Dates::read(root, &settings, &mut sources);
    let dates = match run.today {
        Some(today) => dates.with_today(today),
        None => dates,
    };
    problems.extend(found);
    let mut site = Site::default();
    let (data, found) = crate::data::read(root, &mut sources);
    problems.extend(found);
    site.data = data;
    for language in dates.languages() {
        site.languages.push(Language {
            tag: language.tag.clone(),
            folder: None,
        });
    }
    let real = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let left_behind: HashSet<&str> = settings
        .texts("left_behind")
        .iter()
        .map(|name| name.value.as_str())
        .collect();
    // A folder with no `content/` has no pages; whether it is a site at
    // all is for the command to say.
    let (folders, found) = scan(root, &real, &left_behind, &run.renamed, &mut problems);
    let language_folders = language_folders(&site, &settings, &folders, &mut problems);
    for (rel, id) in &language_folders {
        site.languages.get_mut(*id).folder = Some(rel.trim_end_matches('/').to_owned());
    }
    let mut reader = Reader {
        settings: &settings,
        dates: &dates,
        run: run.clone(),
        language_folders,
        folders,
        docs: Vec::new(),
        problems,
        drafts: Vec::new(),
        waiting: Vec::new(),
        included: Included::default(),
        for_posts: Vec::new(),
        keys_left_out: Vec::new(),
        sidecars: Vec::new(),
        files_left_out: Vec::new(),
        held_back: Vec::new(),
        layouts_needing_url: Vec::new(),
    };
    reader.read_docs(&real, &found, &mut sources, texts);
    reader.build(&sources, &mut site);
    let page_names = match settings.declarations.naming().next() {
        Some(_) => reader.page_names(&sources),
        None => HashMap::new(),
    };
    let languages_of: Vec<LanguageId> = (reader.docs.iter())
        .map(|doc| reader.language_of(doc.folder))
        .collect();
    site.real = real;
    site.found = found;
    let (mut problems, drafts, waiting, included, for_posts) = (
        reader.problems,
        reader.drafts,
        reader.waiting,
        reader.included,
        reader.for_posts,
    );
    let keys_left_out = reader.keys_left_out;
    site.held_back = reader.held_back;
    site.layouts_needing_url = reader.layouts_needing_url;
    let (sidecars, left_out) = (reader.sidecars, reader.files_left_out);
    problems.extend(crate::lists::build(&mut site, &settings, &dates, &sources));
    made::make_for_posts(&sources, &mut site, for_posts, &mut problems);
    let written = (reader.docs.iter())
        .flat_map(|doc| [&doc.words.own, &doc.words.layer])
        .chain([&settings.pages])
        .chain(site.pages.iter().map(|(_, page)| &page.words));
    problems.extend(words::type_slips(
        written,
        &settings.declarations,
        &root.join("templates"),
    ));
    let written = (reader.docs.iter().zip(&languages_of))
        .flat_map(|(doc, &language)| {
            [&doc.words.own, &doc.words.layer].map(|w| (Some(language), w))
        })
        .chain([(None, &settings.pages)])
        .chain((site.pages.iter()).filter_map(|(_, page)| {
            (page.made.as_ref()).map(|_| (Some(page.language), &page.words))
        }));
    problems.extend(names_held(
        &sources,
        &settings.declarations,
        &site,
        &page_names,
        written,
    ));
    site.page_names = page_names;
    problems.extend(translations::link(&mut site, &sources, &keys_left_out));
    let on_pages = site.pages.iter().flat_map(|(_, page)| {
        let written = page.words.words.iter().map(|(_, value)| &value.value);
        let dates = written.filter_map(Value::as_date);
        page.date
            .as_ref()
            .map(|date| &date.value)
            .into_iter()
            .chain(dates)
    });
    let in_settings = settings
        .get("feed_ids_before")
        .and_then(|value| value.value.as_date());
    problems.extend(crate::dates::unzoned(
        &settings,
        on_pages.chain(in_settings),
    ));
    Read {
        sources,
        settings,
        site,
        problems,
        drafts,
        waiting,
        included,
        dates,
        sidecars,
        left_out,
    }
}

/// What one folder under `content/` holds.
struct Folder {
    /// Its path under `content/`, ending with `/`; empty at the top.
    rel: String,
    /// Each folder inside, by its index in the scan.
    children: Vec<usize>,
    parent: Option<usize>,
    /// Files that may be pages, by name.
    writing: Vec<String>,
    /// Everything else, by name.
    files: Vec<String>,
    reading: Reading,
    /// The document that is its `index`, where there is one.
    index: Option<usize>,
    list: Option<ListId>,
    /// Whether its pages are ordered by the numbers their names open with.
    numbered: bool,
}

/// What a folder turns out to be.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Reading {
    /// A list of pages.
    Pages,
    /// One page.
    Page,
    /// Files and nothing else.
    Files,
}

impl Folder {
    fn new(rel: String, parent: Option<usize>) -> Folder {
        Folder {
            rel,
            children: Vec::new(),
            parent,
            writing: Vec::new(),
            files: Vec::new(),
            reading: Reading::Files,
            index: None,
            list: None,
            numbered: false,
        }
    }
}

/// A file read that may be a page.
struct Doc {
    folder: usize,
    name: String,
    source: SourceId,
    html: bool,
    front: Option<Node>,
    body: Span,
    /// Whether its front matter could be read, so a page can be made of it.
    read: bool,
    words: words::Read,
}

impl Doc {
    fn stem(&self) -> &str {
        self.name
            .rsplit_once('.')
            .map_or(&self.name, |(stem, _)| stem)
    }

    fn is_index(&self) -> bool {
        self.stem().eq_ignore_ascii_case("index")
    }
}

fn extension(name: &str) -> String {
    name.rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase())
        .unwrap_or_default()
}

fn is_writing(name: &str) -> bool {
    matches!(extension(name).as_str(), "md" | "markdown")
}

fn is_html(name: &str) -> bool {
    matches!(extension(name).as_str(), "html" | "htm")
}

/// The folders under `content/` and what each holds, each folder after the
/// one it is in, and where each file really is under the site's real path
/// `real`, by its path under `content/`. The walk is the assets' own:
/// hidden names are passed over, and a link is followed only where it
/// stays inside the site.
fn scan(
    root: &Path,
    real: &Path,
    left_behind: &HashSet<&str>,
    renamed: &[(String, String)],
    problems: &mut Vec<Problem>,
) -> (Vec<Folder>, HashMap<String, String>) {
    let files = crate::assets::walk(root, real, "content", false, problems);
    let mut folders = vec![Folder::new(String::new(), None)];
    let mut at_path: HashMap<String, usize> = HashMap::from([(String::new(), 0)]);
    let mut found = HashMap::new();
    for (rel, at) in files {
        let rel = renamed_to(rel, renamed);
        let pieces: Vec<&str> = rel.split('/').collect();
        if pieces.iter().any(|piece| left_behind.contains(piece)) {
            continue;
        }
        let (name, inside) = pieces.split_last().expect("a file has a name");
        let mut folder = 0;
        let mut path = String::new();
        for piece in inside {
            path.push_str(piece);
            path.push('/');
            folder = match at_path.get(&path) {
                Some(&at) => at,
                None => {
                    let at = folders.len();
                    folders.push(Folder::new(path.clone(), Some(folder)));
                    folders[folder].children.push(at);
                    at_path.insert(path.clone(), at);
                    at
                }
            };
        }
        let name = (*name).to_owned();
        match is_writing(&name) || is_html(&name) {
            true => folders[folder].writing.push(name),
            false => folders[folder].files.push(name),
        }
        found.insert(rel, at);
    }
    (folders, found)
}

/// The path under `content/` the file at `rel` there is read at: as
/// `renamed` has it, where it names the file or a folder it is in.
pub(crate) fn renamed_to(rel: String, renamed: &[(String, String)]) -> String {
    for (from, to) in renamed {
        if rel == *from {
            return to.clone();
        }
        if let Some(rest) = rel.strip_prefix(from.as_str())
            && rest.starts_with('/')
        {
            return format!("{to}{rest}");
        }
    }
    rel
}

/// The folders at the top of `content/` that hold a language's pages, on a
/// site in several languages: each named exactly by a declared tag. A
/// language other than the main one with no folder is warned of, at its tag.
fn language_folders(
    site: &Site,
    settings: &Settings,
    folders: &[Folder],
    problems: &mut Vec<Problem>,
) -> Vec<(String, LanguageId)> {
    if site.languages.len() < 2 {
        return Vec::new();
    }
    let mut found = Vec::new();
    for ((id, language), written) in site.languages.iter().zip(settings.languages()) {
        let Some(tag) = &language.tag else {
            continue;
        };
        let rel = format!("{tag}/");
        if folders.iter().any(|folder| folder.rel == rel) {
            found.push((rel, id));
        } else if id.index() > 0 {
            let kind = Kind::NoPagesInLanguage { tag: tag.clone() };
            problems.push(match written.origin.place {
                Some(place) => Problem::at(kind, place),
                None => Problem::nowhere(kind),
            });
        }
    }
    found
}

/// Whether a file of HTML is a page: it opens with front matter, as a
/// Markdown page's is found. Any other HTML travels as the file it is.
fn opens_with_front_matter(text: &str) -> bool {
    // Front matter left open is a page's, so the problem with it is told.
    crate::yaml::split(text).map_or(true, |split| split.front_matter.is_some())
}

/// Whether writing at `body` opens with front matter written without its
/// `---` lines: a field named by one of Statac's page words, and up to the
/// first blank line nothing but fields and the lines that continue them.
fn fields_without_lines(text: &str, body: Span) -> bool {
    fn field(line: &str) -> Option<&str> {
        let (name, value) = line.split_once(':')?;
        let named = !name.is_empty()
            && (name.bytes()).all(|c| c.is_ascii_alphanumeric() || b"_-.".contains(&c));
        (named && (value.is_empty() || value.starts_with([' ', '\t']))).then_some(name)
    }
    let continues = |line: &str| line.starts_with([' ', '\t']) || line.starts_with("- ");
    let mut lines = text[body.range()].lines();
    lines
        .next()
        .and_then(field)
        .is_some_and(|name| words::PageWord::named(name).is_some())
        && lines
            .take_while(|line| !line.trim().is_empty())
            .all(|line| field(line).is_some() || continues(line))
}

struct Reader<'a> {
    settings: &'a Settings,
    dates: &'a Dates,
    run: Run,
    /// Each language's folder at the top of `content/`, as its path there,
    /// on a site in several languages.
    language_folders: Vec<(String, LanguageId)>,
    folders: Vec<Folder>,
    docs: Vec<Doc>,
    problems: Vec<Problem>,
    drafts: Vec<SourceId>,
    waiting: Vec<SourceId>,
    included: Included,
    /// The pages making a page per post, made once lists are filled.
    for_posts: Vec<made::ForPosts>,
    /// The `translation_key` of each page left out as a draft or until its
    /// day, by its language.
    keys_left_out: Vec<(LanguageId, String)>,
    /// The files describing pictures, by their paths under `content/`.
    sidecars: Vec<String>,
    /// The files beside pages left out of the build.
    files_left_out: Vec<String>,
    /// Where the pages left out as drafts or until their day would stand.
    held_back: Vec<crate::site::HeldBack>,
    layouts_needing_url: Vec<String>,
}

impl Reader<'_> {
    fn several_languages(&self) -> bool {
        self.dates.languages().nth(1).is_some()
    }

    /// The language of what the folder at `at` holds: the language whose
    /// folder it is in, or else the main language.
    fn language_of(&self, at: usize) -> LanguageId {
        let rel = &self.folders[at].rel;
        self.language_folders
            .iter()
            .find(|(folder, _)| rel.starts_with(folder.as_str()))
            .map_or(LanguageId::from_index(0), |&(_, id)| id)
    }

    /// The language folder the folder at `at` is in, as its path under
    /// `content/` ending with `/`; empty outside every language folder.
    fn top_of(&self, at: usize) -> &str {
        let rel = &self.folders[at].rel;
        self.language_folders
            .iter()
            .find(|(folder, _)| rel.starts_with(folder.as_str()))
            .map_or("", |(folder, _)| folder.as_str())
    }

    /// Reads every file that may be a page, in parallel, and their front
    /// matter; then settles what each folder is and reads each page's words.
    /// Each is read where it really is, as `found` has it under `real`.
    fn read_docs(
        &mut self,
        real: &Path,
        found: &HashMap<String, String>,
        sources: &mut Sources,
        kept: &mut Texts,
    ) {
        let wanted: Vec<(usize, String)> = self
            .folders
            .iter()
            .enumerate()
            .flat_map(|(at, folder)| folder.writing.iter().map(move |name| (at, name.clone())))
            .collect();
        let texts: Vec<_> = wanted
            .par_iter()
            .map(|(at, name)| {
                let rel = format!("{}{name}", self.folders[*at].rel);
                let at = found[&rel].as_str();
                (at, kept.read(real, at))
            })
            .collect();
        let read = texts
            .iter()
            .filter_map(|(at, text)| Some((*at, text.as_ref().ok()?)));
        kept.keep(read);
        let mut added = Vec::new();
        for ((at, name), (_, text)) in wanted.into_iter().zip(texts) {
            let path = format!("content/{}{name}", self.folders[at].rel);
            let (text, lines) = match text {
                Ok(read) => (read.text, read.lines),
                Err(error) => {
                    self.problems.push(Problem::nowhere(Kind::CantRead {
                        path: path.into(),
                        reason: crate::problem::Reason::of(&error),
                    }));
                    continue;
                }
            };
            let html = is_html(&name);
            if html && !opens_with_front_matter(&text) {
                self.folders[at].files.push(name);
                continue;
            }
            let source = sources.add_with_lines(path, text, lines);
            added.push((at, name, source, html));
        }
        for folder in &mut self.folders {
            folder.writing.clear();
        }
        let parsed: Vec<_> = added
            .par_iter()
            .map(|(_, _, source, _)| crate::yaml::front_matter(sources, *source))
            .collect();
        for ((folder, name, source, html), parsed) in added.into_iter().zip(parsed) {
            let (front, body, read) = match parsed {
                // A stamp is no value yet, so nothing is made of its page.
                Ok((Some(front), _)) if self.stamps(&front) => {
                    (None, Span::new(0, sources.text(source).len()), false)
                }
                Ok((front, body)) => {
                    let text = sources.text(source);
                    if front.is_none() && !html && fields_without_lines(text, body) {
                        let place = words::head(sources, source);
                        let kind = Kind::PublishedAsText {
                            written: sources.spelling(place).to_owned(),
                            why: PublishedAsText::FrontMatter,
                        };
                        self.problems.push(Problem::at(kind, place));
                    }
                    (front, body, true)
                }
                Err(problem) => {
                    self.problems.push(problem);
                    let len = sources.text(source).len();
                    (None, Span::new(0, len), false)
                }
            };
            self.folders[folder].writing.push(name.clone());
            self.docs.push(Doc {
                folder,
                name,
                source,
                html,
                front,
                body,
                read,
                words: words::Read::default(),
            });
        }
        self.settle_folders();
        let holders: Vec<Holder> = (0..self.docs.len()).map(|d| self.holder(d)).collect();
        let declarations = &self.settings.declarations;
        let several = self.several_languages();
        let read: Vec<words::Read> = self
            .docs
            .par_iter()
            .zip(holders)
            .map(|(doc, holder)| {
                words::read_front_matter(sources, doc.front.as_ref(), holder, declarations, several)
            })
            .collect();
        for (doc, read) in self.docs.iter_mut().zip(read) {
            doc.words = read;
        }
    }

    /// Tells of each stamp in a file's front matter, and gives whether
    /// there is one.
    fn stamps(&mut self, front: &Node) -> bool {
        let found = crate::stamp::problems(front);
        let any = !found.is_empty();
        self.problems.extend(found);
        any
    }

    /// Settles each folder's index and what each folder is, innermost first.
    fn settle_folders(&mut self) {
        for (at, doc) in self.docs.iter().enumerate() {
            let folder = &mut self.folders[doc.folder];
            if doc.is_index() && folder.index.is_none() {
                folder.index = Some(at);
            }
        }
        for at in (0..self.folders.len()).rev() {
            let folder = &self.folders[at];
            let others = folder.writing.len() - usize::from(folder.index.is_some());
            let holds = folder
                .children
                .iter()
                .any(|&child| self.folders[child].reading != Reading::Files);
            let speaks = folder.index.is_some_and(|index| {
                let front = self.docs[index].front.as_ref();
                matches!(front.map(|n| &n.value), Some(crate::yaml::Value::Mapping(entries))
                    if entries.iter().any(|e| words::key_name(&e.key).starts_with("pages.")))
            });
            // A language's folder holds its pages as the top of `content/`
            // holds the main language's.
            let language = self
                .language_folders
                .iter()
                .any(|(rel, _)| *rel == folder.rel);
            let reading = if folder.parent.is_none() || language || others > 0 || holds || speaks {
                Reading::Pages
            } else if folder.index.is_some() {
                Reading::Page
            } else {
                Reading::Files
            };
            self.folders[at].reading = reading;
        }
    }

    fn place(&self, doc: usize) -> PagePlace {
        let d = &self.docs[doc];
        let folder = &self.folders[d.folder];
        match (folder.index == Some(doc), folder.reading) {
            (true, Reading::Pages) => PagePlace::Folder,
            (true, _) => PagePlace::Bundle,
            _ => PagePlace::Ordinary,
        }
    }

    fn holder(&self, doc: usize) -> Holder {
        match self.place(doc) {
            PagePlace::Folder => Holder::FolderPage {
                top: self.folders[self.docs[doc].folder].parent.is_none(),
            },
            // A page making pages hands words down to them.
            _ if self.makes_pages(doc) => Holder::FolderPage { top: false },
            _ => Holder::Page,
        }
    }

    /// Whether a document's front matter has an `each:` line.
    fn makes_pages(&self, doc: usize) -> bool {
        match self.docs[doc].front.as_ref().map(|node| &node.value) {
            Some(crate::yaml::Value::Mapping(entries)) => entries
                .iter()
                .any(|entry| words::key_name(&entry.key) == PageWord::Each.name()),
            _ => false,
        }
    }

    /// The layers a page is handed, nearest first, from `folder` outwards.
    fn layers(&self, mut folder: Option<usize>) -> Vec<&Words> {
        let mut layers = Vec::new();
        while let Some(at) = folder {
            if let Some(index) = self.folders[at].index
                && self.folders[at].reading == Reading::Pages
            {
                layers.push(&self.docs[index].words.layer);
            }
            folder = self.folders[at].parent;
        }
        layers.push(&self.settings.pages);
        layers
    }

    /// Whether the folder at `at` is read as numbered: every page directly
    /// in it, files and folders that are one page alike, opens with a
    /// number, and its `index.md` doesn't say `numbered: false`.
    fn numbered(&self, at: usize) -> bool {
        let folder = &self.folders[at];
        let refused = folder.index.is_some_and(|index| {
            self.docs[index].words.own.truth(PageWord::Numbered) == Some(false)
        });
        let files = self
            .docs
            .iter()
            .enumerate()
            .filter(|&(d, doc)| doc.folder == at && folder.index != Some(d))
            .map(|(_, doc)| doc.stem());
        let bundles = folder
            .children
            .iter()
            .filter(|&&child| self.folders[child].reading == Reading::Page)
            .map(|&child| {
                let rel = self.folders[child].rel.trim_end_matches('/');
                rel.rsplit('/').next().unwrap_or(rel)
            });
        let mut names = files.chain(bundles).peekable();
        !refused
            && names.peek().is_some()
            && names.all(|name| crate::lists::numbered(name).is_some())
    }

    /// The names of the pages each folder under `content/` holds, by the
    /// folder's path there ending with `/`: each page in it or in a folder
    /// beneath it, as a folder's list holds them, by its name in its
    /// address. A page left out of the build has its name still; a
    /// folder's own page, and a page that lists posts, makes pages or
    /// sends readers on, has none, as it is in no folder's list.
    fn page_names(&self, sources: &Sources) -> HashMap<String, Vec<String>> {
        let mut names: Vec<Vec<String>> = vec![Vec::new(); self.folders.len()];
        for (at, doc) in self.docs.iter().enumerate() {
            let place = self.place(at);
            let own = &doc.words.own;
            let speaks = [PageWord::Posts, PageWord::Each, PageWord::RedirectTo];
            if place == PagePlace::Folder || speaks.into_iter().any(|w| own.get(w).is_some()) {
                continue;
            }
            let rel = self.folders[doc.folder].rel.trim_end_matches('/');
            let (name, listed_in) = match (place, self.folders[doc.folder].parent) {
                (PagePlace::Bundle, Some(parent)) => {
                    (rel.rsplit('/').next().unwrap_or(rel), parent)
                }
                _ => (doc.stem(), doc.folder),
            };
            let path = sources.path(doc.source).to_string_lossy();
            let name = name_in_address(name, self.folders[listed_in].numbered, &path);
            let top = self.top_of(listed_in);
            let mut folder = Some(listed_in);
            while let Some(at) = folder {
                names[at].push(name.clone());
                folder = self.folders[at]
                    .parent
                    .filter(|_| self.folders[at].rel != top);
            }
        }
        let folders = self.folders.iter().map(|folder| folder.rel.clone());
        folders.zip(names).collect()
    }

    fn build(&mut self, sources: &Sources, site: &mut Site) {
        for at in 0..self.folders.len() {
            if self.folders[at].reading == Reading::Pages {
                let key = self.folders[at].rel.trim_end_matches('/').to_owned();
                let numbered = self.numbered(at);
                let list = site.lists.push(List {
                    numbered,
                    ..List::new(ListKey::Folder(key), self.language_of(at))
                });
                self.folders[at].list = Some(list);
                self.folders[at].numbered = numbered;
                if numbered {
                    self.problems.push(Problem::nowhere(Kind::NumberedFolder {
                        folder: format!("content/{}", self.folders[at].rel),
                    }));
                }
            }
        }
        let mut problems = Vec::new();
        let (mut flat, mut nested) = (Vec::new(), Vec::new());
        let mut pages_of: HashMap<usize, PageId> = HashMap::new();
        for doc in 0..self.docs.len() {
            problems.append(&mut self.docs[doc].words.problems);
            words::shapes(&self.docs[doc].words.own.fields, &mut flat, &mut nested);
            words::shapes(&self.docs[doc].words.layer.fields, &mut flat, &mut nested);
            if !self.docs[doc].read {
                continue;
            }
            if let Some(page) = self.page(sources, doc, &mut problems) {
                let id = site.pages.push(page);
                pages_of.insert(doc, id);
                let page = site.pages.get_mut(id);
                if page.place == PagePlace::Folder {
                    page.speaks_for = Some(page.folder);
                    site.lists.get_mut(page.folder).page = Some(id);
                }
            }
        }
        self.make_pages(
            sources,
            site,
            &pages_of,
            (&mut flat, &mut nested),
            &mut problems,
        );
        problems.extend(words::written_two_ways(Root::Page, &flat, &nested));
        self.travelling(site, &pages_of, &mut problems);
        self.redirects(sources, site, &mut problems);
        follow(site, &mut problems);
        self.problems.append(&mut problems);
    }

    /// The page a document makes, or none for a draft.
    fn page(&mut self, sources: &Sources, doc: usize, problems: &mut Vec<Problem>) -> Option<Page> {
        let place = self.place(doc);
        let d = &self.docs[doc];
        let folder_at = d.folder;
        let mut words = d.words.own.clone();
        let start = match place {
            PagePlace::Ordinary | PagePlace::Made => Some(folder_at),
            PagePlace::Folder | PagePlace::Bundle => self.folders[folder_at].parent,
        };
        for layer in self.layers(start) {
            words.hand_down_from(layer);
        }
        let head = words::head(sources, d.source);
        let path = sources.path(d.source).to_string_lossy().into_owned();
        // Left out, a page is still placed, so what is kept for it waits
        // with it; nothing more is said of it.
        let mut held = None;
        let mut unsaid = Vec::new();
        if let Some(draft) = words.get(PageWord::Draft)
            && draft.value.as_bool() == Some(true)
        {
            if self.run.drafts {
                self.included.drafts += 1;
            } else {
                self.drafts.push(d.source);
                self.left_out(&words, folder_at);
                let kind = Kind::DraftLeftOut { path: path.clone() };
                let place = draft.origin.place.unwrap_or(head);
                problems.push(Problem::at(kind, place));
                held = Some((Held::Draft, place));
            }
        }
        let d = &self.docs[doc];
        if held.is_none()
            && let Some(needs) = words.text(PageWord::Needs)
            && needs.value == "url"
            && self.settings.text("url").is_none()
        {
            let kind = Kind::NoSiteAddress {
                waiting: crate::problem::Waiting::Page(path.clone()),
            };
            problems.push(Problem::at(kind, needs.origin.place.unwrap_or(head)));
            if let Some(layout) = words.text(PageWord::Layout) {
                self.layouts_needing_url.push(layout.value.to_owned());
            }
            return None;
        }
        let name = match place {
            PagePlace::Ordinary => d.stem().to_owned(),
            _ => self.folders[folder_at]
                .rel
                .trim_end_matches('/')
                .rsplit('/')
                .next()
                .unwrap_or_default()
                .to_owned(),
        };
        let listed_in = match place {
            PagePlace::Bundle => self.folders[folder_at].parent.unwrap_or(folder_at),
            _ => folder_at,
        };
        let date = match held {
            Some(_) => self.date(&words, &name, &mut unsaid),
            None => self.date(&words, &name, problems),
        };
        let speaks = place == PagePlace::Folder
            || [PageWord::Posts, PageWord::Each].into_iter().any(|word| {
                words
                    .get(word)
                    .is_some_and(|value| value.origin.arrival == Arrival::Written)
            });
        if let Some(date) = &date
            && held.is_none()
            && !speaks
            && self.dates.after_today(&date.value)
        {
            if self.run.drafts {
                self.included.waiting += 1;
            } else {
                self.waiting.push(d.source);
                self.left_out(&words, folder_at);
                let kind = Kind::WaitingForItsDay {
                    path: path.clone(),
                    date: date.value.iso_day(),
                };
                let place = date.origin.place.unwrap_or(head);
                problems.push(Problem::at(kind, place));
                held = Some((Held::Waiting, place));
            }
        }
        let d = &self.docs[doc];
        if let Some((why, at)) = held {
            let target = Target {
                place,
                own: &self.folders[folder_at].rel,
                listed: &self.folders[listed_in].rel,
                top: self.top_of(folder_at),
                name: &name,
                numbered: place != PagePlace::Folder && self.folders[listed_in].numbered,
                sends_on: words.text(PageWord::RedirectTo).is_some(),
                date: date.as_ref().map(|d| &d.value),
                path: &path,
                head,
            };
            let address = address_of(sources, &words, &target, &mut unsaid);
            let from = words.get(PageWord::RedirectFrom);
            let from = from
                .and_then(|from| from.value.as_texts())
                .unwrap_or_default();
            self.held_back.push(crate::site::HeldBack {
                address,
                redirect_from: (from.iter())
                    .filter_map(|written| Address::read(&written.value).ok())
                    .collect(),
                held: why,
                place: at,
            });
            return None;
        }
        for (field, of_type) in self
            .settings
            .declarations
            .required(words.text(PageWord::Type).map(|t| t.value))
        {
            if !words.has(field) {
                problems.push(Problem::at(
                    Kind::MissingField {
                        field: field.to_owned(),
                        required_by: of_type
                            .map_or(Requirer::EveryPage, |t| Requirer::Type(t.to_owned())),
                    },
                    head,
                ));
            }
        }
        let title = match words.get(PageWord::Title) {
            Some(title) => title
                .value
                .as_text()
                .map(|t| Sourced::new(t.to_owned(), title.origin.clone())),
            None if !d.html => opening_heading(sources, d.source, d.body),
            None => None,
        };
        let target = Target {
            place,
            own: &self.folders[folder_at].rel,
            listed: &self.folders[listed_in].rel,
            top: self.top_of(folder_at),
            name: &name,
            numbered: place != PagePlace::Folder && self.folders[listed_in].numbered,
            sends_on: words.text(PageWord::RedirectTo).is_some(),
            date: date.as_ref().map(|d| &d.value),
            path: &path,
            head,
        };
        let address = address_of(sources, &words, &target, problems);
        Some(Page {
            source: Some(d.source),
            language: self.language_of(folder_at),
            path: path.trim_start_matches("content/").to_owned(),
            place,
            folder: self.folders[listed_in]
                .list
                .expect("a page is listed in a folder of pages"),
            words,
            title,
            date,
            address,
            body: d.body,
            speaks_for: None,
            made: None,
        })
    }

    /// Keeps the `translation_key` of a page left out of the build, which
    /// its translations still share.
    fn left_out(&mut self, words: &Words, folder_at: usize) {
        if let Some(key) = words.text(PageWord::TranslationKey) {
            let language = self.language_of(folder_at);
            self.keys_left_out.push((language, key.value.to_owned()));
        }
    }

    /// The page's date: its `date` line, or else a date its name opens with.
    fn date(
        &self,
        words: &Words,
        name: &str,
        problems: &mut Vec<Problem>,
    ) -> Option<Sourced<Date>> {
        let written = words.get(PageWord::Date).and_then(|d| {
            d.value
                .as_date()
                .map(|date| Sourced::new(*date, d.origin.clone()))
        });
        let named = date::in_name(name).map(|(date, _)| date);
        match (written, named) {
            (Some(written), Some(named)) => {
                if !written.value.same_day(&named)
                    && let Some(place) = written.origin.place
                {
                    problems.push(Problem::at(
                        Kind::DatesDisagree {
                            named: named.iso_day(),
                            written: written.value.iso_day(),
                        },
                        place,
                    ));
                }
                Some(written)
            }
            (Some(written), None) => Some(written),
            (None, Some(named)) => Some(Sourced::new(
                named,
                Origin {
                    place: None,
                    style: Style::None,
                    arrival: Arrival::FileName,
                },
            )),
            (None, None) => None,
        }
    }

    /// The files under `content/` that are no page, each where it lands:
    /// beside the page whose folder it is in, or in its folder's place.
    fn travelling(
        &mut self,
        site: &mut Site,
        pages_of: &HashMap<usize, PageId>,
        problems: &mut Vec<Problem>,
    ) {
        let (mut sidecars, mut left_out) = (Vec::new(), Vec::new());
        for at in 0..self.folders.len() {
            let folder = &self.folders[at];
            // The nearest folder that is one page, at or above this one.
            let mut bundle = None;
            let mut up = Some(at);
            while let Some(u) = up {
                if self.folders[u].reading == Reading::Page {
                    bundle = Some(u);
                    break;
                }
                if self.folders[u].reading == Reading::Pages {
                    break;
                }
                up = self.folders[u].parent;
            }
            // None where the page whose folder it is was left out.
            let lands = match bundle {
                Some(b) => self.folders[b]
                    .index
                    .and_then(|i| pages_of.get(&i))
                    .map(|&page| {
                        let inside = &folder.rel[self.folders[b].rel.len()..];
                        let base = format!(
                            "{}{}",
                            site.pages.get(page).address.folder(),
                            slug::tidied_path(inside)
                        );
                        (base, Some(page))
                    }),
                None => Some((format!("/{}", slug::tidied_path(&folder.rel)), None)),
            };
            for name in &folder.files {
                let path = format!("{}{name}", folder.rel);
                if crate::album::is_sidecar(name) {
                    sidecars.push(path);
                    continue;
                }
                let Some((base, page)) = &lands else {
                    left_out.push(path);
                    continue;
                };
                match Address::read_file(&format!("{base}{name}")) {
                    Ok(address) => site.files.push(Travelling {
                        path,
                        address,
                        page: *page,
                    }),
                    Err((fault, _)) => problems.push(Problem::nowhere(Kind::CantWrite {
                        path: Path::new("content").join(&path),
                        reason: crate::problem::Reason::Address(Box::new(fault)),
                    })),
                }
            }
        }
        self.sidecars = sidecars;
        self.files_left_out = left_out;
    }

    /// The addresses each page sends readers on from: its old addresses,
    /// and its own where it sends them on with `redirect_to`.
    fn redirects(&self, sources: &Sources, site: &mut Site, problems: &mut Vec<Problem>) {
        for (id, page) in site.pages.iter() {
            // Every page is read from a file here: Statac makes its own later.
            let Some(head) = page.head(sources) else {
                continue;
            };
            if let Some(to) = page.words.text(PageWord::RedirectTo) {
                if page.made.is_none() {
                    problems.extend(unpublished(sources, page));
                }
                let place = to.origin.place.unwrap_or(head);
                match sent_to(to.value) {
                    Ok(SentTo::Site { address, .. }) if address.key() == page.address.key() => {
                        problems.push(Problem::at(
                            Kind::RedirectToItself {
                                written: to.value.to_owned(),
                            },
                            place,
                        ));
                    }
                    Ok(sent) => site.redirects.push(Redirect {
                        from: page.address.clone(),
                        page: id,
                        to: sent,
                        place,
                    }),
                    Err((fault, span)) => {
                        let place = crate::value::place_within(sources, &to.origin, to.value, span)
                            .unwrap_or(place);
                        problems.push(Problem::at(Kind::UnusableAddress { fault }, place));
                    }
                }
            }
            let Some(from) = page.words.get(PageWord::RedirectFrom) else {
                continue;
            };
            let Some(old) = from.value.as_texts() else {
                continue;
            };
            let mut seen = HashSet::new();
            for written in old {
                let place = written.origin.place.unwrap_or(head);
                match Address::read(&written.value) {
                    Ok(address) if address.key() == page.address.key() => {
                        problems.push(Problem::at(
                            Kind::RedirectFromItself {
                                written: written.value.clone(),
                            },
                            place,
                        ));
                    }
                    Ok(address) => {
                        if seen.insert(address.key()) {
                            site.redirects.push(Redirect {
                                from: address,
                                page: id,
                                to: SentTo::Page,
                                place,
                            });
                        }
                    }
                    Err((fault, span)) => {
                        let place = crate::value::place_within(
                            sources,
                            &written.origin,
                            &written.value,
                            span,
                        )
                        .unwrap_or(place);
                        problems.push(Problem::at(Kind::UnusableAddress { fault }, place));
                    }
                }
            }
        }
    }
}

/// Sends each page's `redirect_to` naming an address on the site that
/// is itself a redirect straight to where following ends, and each old
/// address of a page that itself sends readers on likewise, and refuses
/// each `redirect_to` whose following comes back to its own page.
fn follow(site: &mut Site, problems: &mut Vec<Problem>) {
    let from = by_from(site);
    let ring: Vec<bool> = (0..site.redirects.len())
        .map(|at| followed(site, &from, at).is_none())
        .collect();
    for (redirect, _) in site.redirects.iter().zip(&ring).filter(|&(_, &ring)| ring) {
        if let SentTo::Site { written, .. } = &redirect.to {
            let kind = Kind::RedirectedBack {
                written: written.clone(),
            };
            problems.push(Problem::at(kind, redirect.place));
        }
    }
    let mut ring = ring.into_iter();
    site.redirects.retain(|_| !ring.next().unwrap_or_default());
    // Another redirect at an address a ring stood at, which is told as
    // the same address, may make a ring of its own: it is left as written.
    let from = by_from(site);
    let ends: Vec<Option<SentTo>> = (0..site.redirects.len())
        .map(|at| followed(site, &from, at))
        .collect();
    for (redirect, end) in site.redirects.iter_mut().zip(ends) {
        if let Some(end) = end {
            redirect.to = end;
        }
    }
}

/// Each redirect by where it sends readers on from, as a disk would hold
/// it; the first where two stand at one address.
fn by_from(site: &Site) -> HashMap<String, usize> {
    let mut from = HashMap::new();
    for (at, redirect) in site.redirects.iter().enumerate() {
        from.entry(redirect.from.key()).or_insert(at);
    }
    from
}

/// Where readers sent on by the redirect at `start` end up, following
/// every redirect on the site they reach, as `from` finds them; none
/// where following comes back to it. The fragment written at the start
/// is kept, and else the first met on the way. An old address of a page
/// is followed from the page's own, and is left as it is where the page
/// sends readers nowhere.
fn followed(site: &Site, from: &HashMap<String, usize>, start: usize) -> Option<SentTo> {
    let redirects = &site.redirects;
    let (written, mut address, mut fragment) = match &redirects[start].to {
        SentTo::Site {
            written,
            address,
            fragment,
        } => (written.clone(), address.clone(), fragment.clone()),
        SentTo::Page => {
            let own = &site.pages.get(redirects[start].page).address;
            if from.get(&own.key()).is_none_or(|&at| at == start) {
                return Some(SentTo::Page);
            }
            // The page's own address stands for what was written: a
            // redirect is published there.
            (own.as_str().to_owned(), own.clone(), String::new())
        }
        SentTo::Elsewhere(_) => return Some(redirects[start].to.clone()),
    };
    let mut seen = HashSet::from([start]);
    while let Some(&next) = from.get(&address.key()) {
        if next == start {
            return None;
        }
        // A ring that doesn't come back here ends the following.
        if !seen.insert(next) {
            break;
        }
        match &redirects[next].to {
            SentTo::Page => address = site.pages.get(redirects[next].page).address.clone(),
            SentTo::Site {
                address: on,
                fragment: met,
                ..
            } => {
                address = on.clone();
                if fragment.is_empty() {
                    fragment.clone_from(met);
                }
            }
            SentTo::Elsewhere(elsewhere) => {
                let elsewhere = match fragment.is_empty() {
                    true => elsewhere.clone(),
                    false => {
                        let bare = elsewhere.split('#').next().unwrap_or_default();
                        format!("{bare}{fragment}")
                    }
                };
                return Some(SentTo::Elsewhere(elsewhere));
            }
        }
    }
    Some(SentTo::Site {
        written,
        address,
        fragment,
    })
}

/// Statac's own words a page sending readers on reads: where it stands,
/// whether it is published, what its redirect is called, and where else
/// readers are sent from. A folder's order still orders its pages.
const READ_WHEN_SENDING: &[PageWord] = &[
    PageWord::RedirectTo,
    PageWord::RedirectFrom,
    PageWord::Title,
    PageWord::Date,
    PageWord::Permalink,
    PageWord::Draft,
    PageWord::Needs,
    PageWord::Order,
    PageWord::Numbered,
];

/// What a page sending readers on writes that is never published: each of
/// Statac's own words it doesn't read, and its writing, told at its first
/// line.
fn unpublished(sources: &Sources, page: &Page) -> Vec<Problem> {
    let mut problems: Vec<Problem> = page
        .words
        .words
        .iter()
        .filter(|(word, value)| {
            value.origin.arrival == Arrival::Written && !READ_WHEN_SENDING.contains(word)
        })
        .filter_map(|(word, value)| {
            let kind = Kind::SetAside {
                field: word.name().to_owned(),
                why: SetAside::SendsOn,
            };
            Some(Problem::at(kind, value.origin.place?))
        })
        .collect();
    let Some(source) = page.source else {
        return problems;
    };
    let text = sources.text(source);
    let body = &text[page.body.range()];
    if let Some(at) = body.find(|c: char| !c.is_whitespace()) {
        let start = page.body.start as usize + at;
        let end = text[start..]
            .find(['\n', '\r'])
            .map_or(text.len(), |len| start + len);
        let path = sources.path(source).to_string_lossy().into_owned();
        problems.push(Problem::at(
            Kind::WritingNotPublished { path },
            Place {
                source,
                span: Span::new(start, end),
            },
        ));
    }
    problems
}

/// Where a `redirect_to` sends readers: a full address elsewhere, opening
/// with a scheme such as `https:` or with `//`, or else an address on the
/// site, read by the part before any `#` with any `%XX` read as the
/// character it stands for.
fn sent_to(written: &str) -> Result<SentTo, address::Fault> {
    if address::is_full(written) {
        return Ok(SentTo::Elsewhere(written.to_owned()));
    }
    let (path, fragment) = written
        .find('#')
        .map_or((written, ""), |at| written.split_at(at));
    let decoded = address::decoded(path);
    let address = match Address::read(&decoded) {
        Ok(address) => address,
        // Placed in what was written, where nothing was decoded.
        Err((fault, span)) if decoded == path => return Err((fault, span)),
        Err((fault, _)) => return Err((fault, Span::new(0, path.len()))),
    };
    Ok(SentTo::Site {
        written: written.to_owned(),
        address,
        fragment: fragment.to_owned(),
    })
}

/// A warning for each redirect sending readers to an address on the site
/// that nothing in `outputs`, each published file's path from the top of
/// the built site, stands at, offering the nearest that something does.
pub fn sent_nowhere(site: &Site, outputs: &[(String, Named)]) -> Vec<Problem> {
    let published: HashSet<&str> = outputs.iter().map(|(file, _)| file.as_str()).collect();
    let mut addresses: Option<Vec<String>> = None;
    let mut problems = Vec::new();
    for redirect in &site.redirects {
        let SentTo::Site { written, .. } = &redirect.to else {
            continue;
        };
        // Judged as written: a redirect followed on from there is judged
        // where it is written itself.
        let Ok(SentTo::Site {
            address, fragment, ..
        }) = sent_to(written)
        else {
            continue;
        };
        if published.contains(address.file().as_str()) {
            continue;
        }
        let addresses = addresses.get_or_insert_with(|| {
            outputs
                .iter()
                .map(|(file, _)| match file.strip_suffix("index.html") {
                    Some(folder) if folder.is_empty() || folder.ends_with('/') => {
                        format!("/{folder}")
                    }
                    _ => format!("/{file}"),
                })
                .collect()
        });
        let suggestion =
            crate::suggest::nearest(address.as_str(), addresses.iter().map(String::as_str))
                .map(|nearest| format!("{nearest}{fragment}"));
        problems.push(Problem::at(
            Kind::BrokenLink {
                target: written.clone(),
                suggestion,
                inside: false,
            },
            redirect.place,
        ));
    }
    problems
}

/// A warning for each page left out of the build whose address something
/// in `outputs`, each published file's path from the top of the built
/// site, already stands at, compared as two published files are.
pub fn taken(site: &Site, outputs: &[(String, Named)], base: &str) -> Vec<Problem> {
    // Reversed, so the first to claim a file is the one named.
    let published: HashMap<String, &Named> = (outputs.iter().rev())
        .map(|(file, named)| (crate::platform::folded(file), named))
        .collect();
    let taken = site.held_back.iter().filter_map(|held| {
        let named = published.get(&held.address.key())?;
        let kind = Kind::AddressTaken {
            held: held.held,
            address: format!("{base}{}", held.address.as_str()),
            taken: Box::new((*named).clone()),
        };
        Some(Problem::at(kind, held.place))
    });
    taken.collect()
}

/// What a page's address is made from.
#[derive(Copy, Clone)]
struct Target<'a> {
    place: PagePlace,
    /// The path of the page's own folder under `content/`.
    own: &'a str,
    /// The path of the folder it is listed in.
    listed: &'a str,
    /// The path of the language folder it is in, which its `permalink` is
    /// read from; empty outside every language folder.
    top: &'a str,
    /// Its name as written: its file's, or its folder's where the folder is the page.
    name: &'a str,
    /// Whether it is in a numbered folder, whose numbers no address keeps.
    numbered: bool,
    /// Whether it sends readers on, standing where something stood before,
    /// so its address keeps its whole name.
    sends_on: bool,
    date: Option<&'a Date>,
    path: &'a str,
    head: Place,
}

/// Where a page lands: its `permalink` filled in, written or handed down,
/// though a folder's own page is moved only by one written on it; or its
/// folder and name. A piece naming a declared field is filled from the
/// page's own, as an item's is.
fn address_of(
    sources: &Sources,
    words: &Words,
    target: &Target,
    problems: &mut Vec<Problem>,
) -> Address {
    let Target {
        place,
        own,
        listed,
        top,
        name,
        numbered,
        sends_on,
        date,
        path,
        head,
    } = *target;
    let below = slug::tidied_path(&listed[top.len()..]);
    let listed = slug::tidied_path(listed);
    let slug = match sends_on {
        true => Some(slug::slugify(name))
            .filter(|slug| !slug.is_empty())
            .unwrap_or_else(|| slug::made_slug(path)),
        false => name_in_address(name, numbered, path),
    };
    let default = match place {
        PagePlace::Folder => format!("/{}", slug::tidied_path(own)),
        _ => format!("/{listed}{slug}/"),
    };
    let makes_pages = words
        .get(PageWord::Each)
        .is_some_and(|each| each.origin.arrival == Arrival::Written);
    let permalink = words.get(PageWord::Permalink).filter(|_| !makes_pages);
    let written_here = permalink.is_some_and(|p| p.origin.arrival == Arrival::Written);
    let at = |problems: &mut Vec<Problem>, fault: AddressFault| {
        let place = permalink
            .filter(|_| written_here)
            .and_then(|p| p.origin.place)
            .unwrap_or(head);
        problems.push(Problem::at(Kind::UnusableAddress { fault }, place));
        Address::made(path)
    };
    let Some(permalink) = permalink.filter(|_| place != PagePlace::Folder || written_here) else {
        return match Address::read(&default) {
            Ok(address) => address,
            Err((fault, _)) => at(problems, fault),
        };
    };
    let Value::Text(pattern) = &permalink.value else {
        return Address::made(path);
    };
    let folder = below
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or_default();
    let pieces = Pieces {
        date,
        slug: &slug,
        folder,
        name,
    };
    let top = Top::new(top);
    let written = (pattern.as_str(), &permalink.origin);
    let pattern = top.pattern(pattern, permalink.origin.place, problems);
    let filling = (path, made::Whose::Page, head);
    let Some(pattern) = made::own_fields(sources, words, pattern, written, filling, problems)
    else {
        return Address::made(path);
    };
    let expanded = match address::expand(&pattern, &pieces) {
        Ok(expanded) => top.address(&expanded),
        Err(fault) => {
            problems.push(Problem::at(Kind::UnusableAddress { fault }, head));
            return Address::made(path);
        }
    };
    let last = expanded.rsplit('/').next().unwrap_or_default();
    let read = if writes_other_than_html(words) && !expanded.ends_with('/') && !last.contains('.') {
        Address::read_file(&expanded)
    } else {
        Address::read(&expanded)
    };
    match read {
        Ok(address) => address,
        Err((fault, _)) => {
            problems.push(Problem::at(Kind::UnusableAddress { fault }, head));
            Address::made(path)
        }
    }
}

/// Names a declared field's value can be: what holds them, and the
/// language whose pages they are where a folder does.
type NamesHeld<'a> = (Option<LanguageId>, crate::problem::NamedIn, Vec<&'a str>);

/// Holds each field declared `one of` a table's names or a folder's pages
/// to them, wherever a value for it is `written`, with the language it is
/// written in where it has one. A declaration naming nothing a value can
/// be held to is told where it is written, and its field's values pass.
///
/// A folder is the one at its path in each language's own folder, and a
/// page there in any language is a name: a problem shows the folder of
/// the language the value is written in.
fn names_held<'a>(
    sources: &Sources,
    declarations: &config::declarations::Declarations,
    site: &Site,
    page_names: &HashMap<String, Vec<String>>,
    written: impl Iterator<Item = (Option<LanguageId>, &'a Words)>,
) -> Vec<Problem> {
    use crate::data::Reached;
    use crate::problem::NamedIn;
    use crate::yaml::Value as Yaml;
    use config::declarations::Names;

    let mut problems = Vec::new();
    let mut held: HashMap<&str, Vec<NamesHeld>> = HashMap::new();
    for (field, declared, names) in declarations.naming() {
        let told = |kind| Problem::at(kind, declared.place);
        let declared = declared.written.clone();
        match names {
            Names::Data(table) => {
                let reached = site.data.reach(&table.split('.').collect::<Vec<_>>());
                let (path, names) = match reached {
                    Some(Reached::Value { file, node }) => {
                        let path = site.data.file(file).path.clone();
                        let names = match &node.value {
                            Yaml::Mapping(entries) => (entries.iter())
                                .filter_map(|entry| match &entry.key.value {
                                    Yaml::Scalar(name) => Some(name.as_str()),
                                    _ => None,
                                })
                                .collect(),
                            Yaml::Sequence(_) => {
                                problems.push(told(Kind::NoNamesToCheck { path, declared }));
                                continue;
                            }
                            Yaml::Scalar(_) => Vec::new(),
                        };
                        (path, names)
                    }
                    Some(Reached::Folder(folder)) => {
                        let names = folder.entries.keys().map(String::as_str).collect();
                        (folder.path.clone(), names)
                    }
                    // Told already, where the file was read.
                    Some(Reached::Unread(_)) => continue,
                    None => {
                        let known = site.data.names();
                        let nearest =
                            crate::suggest::nearest(table, known.iter().map(String::as_str));
                        problems.push(told(Kind::MissingData {
                            table: format!("data.{table}"),
                            declared,
                            nearest: nearest.map(|name| format!("data.{name}")),
                        }));
                        continue;
                    }
                };
                held.insert(field, vec![(None, NamedIn::Data(path), names)]);
            }
            Names::Folder(path) => {
                let mut folders: Vec<NamesHeld> = Vec::new();
                for (language, _) in site.languages.iter() {
                    let rel = match site.top(language) {
                        "" => format!("{path}/"),
                        top => format!("{top}/{path}/"),
                    };
                    if let Some(names) = page_names.get(&rel) {
                        let names = names.iter().map(String::as_str).collect();
                        let folder = NamedIn::Folder(format!("content/{rel}"));
                        folders.push((Some(language), folder, names));
                    }
                }
                if folders.is_empty() {
                    let known = page_names.keys().filter_map(|rel| {
                        let rel = rel.trim_end_matches('/');
                        let below = rel[site.top_above(rel).len()..].trim_start_matches('/');
                        (!below.is_empty()).then_some(below)
                    });
                    let mut known: Vec<&str> = known.collect();
                    known.sort_unstable();
                    let nearest = crate::suggest::nearest(path, known);
                    problems.push(told(Kind::MissingContentFolder {
                        folder: format!("content/{path}/"),
                        declared,
                        nearest: nearest.map(|name| format!("folder.{name}")),
                    }));
                    continue;
                }
                held.insert(field, folders);
            }
        }
    }
    if held.is_empty() {
        return problems;
    }
    let mut seen = HashSet::new();
    for (language, words) in written {
        for field in &words.fields {
            for (name, node) in declarations.naming_in(&field.name, &field.value) {
                let Some(among) = held.get(name) else {
                    continue;
                };
                // Anything but text is told as the wrong kind of value.
                let reader = crate::value::Reader::new(sources, name);
                let Ok(Some(text)) = reader.text(node) else {
                    continue;
                };
                let written = text.value.as_str();
                if !seen.insert(node.place)
                    || among.iter().any(|(_, _, names)| names.contains(&written))
                {
                    continue;
                }
                let own = among
                    .iter()
                    .position(|(of, ..)| of.is_some() && *of == language);
                let own = own.unwrap_or(0);
                let nearer = among[own].2.iter().chain(
                    (among.iter().enumerate())
                        .filter(|(at, _)| *at != own)
                        .flat_map(|(_, (_, _, names))| names),
                );
                let nearest = crate::suggest::nearest(written, nearer.copied());
                problems.push(Problem::at(
                    Kind::NamesNothing {
                        written: written.to_owned(),
                        among: among[own].1.clone(),
                        nearest: nearest.map(str::to_owned),
                    },
                    node.place,
                ));
            }
        }
    }
    problems
}

/// A page's name in its address, from its name as written: the date or
/// number it opens with taken off, or where that leaves no letter or figure,
/// a name made from its path.
fn name_in_address(name: &str, numbered: bool, path: &str) -> String {
    let unnamed = match numbered {
        true => crate::lists::numbered(name).map_or(name, |(_, rest)| rest),
        false => {
            date::in_name(name).map_or(name, |(_, rest)| if rest.is_empty() { name } else { rest })
        }
    };
    let slug = slug::slugify(unnamed);
    match slug.is_empty() {
        true => slug::made_slug(path),
        false => slug,
    }
}

/// Whether the page's layout writes something other than HTML, being named
/// in full with another ending, such as `feed.xml`.
fn writes_other_than_html(words: &Words) -> bool {
    words
        .text(PageWord::Layout)
        .and_then(|layout| {
            layout
                .value
                .rsplit_once('.')
                .map(|(_, ext)| ext.to_ascii_lowercase())
        })
        .is_some_and(|ext| ext != "html" && ext != "htm")
}

/// The words of a Markdown page's opening `# heading`, placed where they are.
fn opening_heading(sources: &Sources, source: SourceId, body: Span) -> Option<Sourced<String>> {
    let text = sources.text(source);
    let mut at = body.start as usize;
    for line in text[body.range()].split_inclusive('\n') {
        let start = at;
        at += line.len();
        let trimmed = line.trim_end_matches(['\n', '\r']);
        if trimmed.trim().is_empty() {
            continue;
        }
        let indent = trimmed.len() - trimmed.trim_start_matches(' ').len();
        let rest = &trimmed[indent..];
        if indent > 3 {
            return None;
        }
        let words = match rest.strip_prefix('#') {
            Some(words) if words.is_empty() || words.starts_with([' ', '\t']) => words,
            _ => return None,
        };
        let mut words = words.trim();
        // A closing run of `#` after a space is no part of the heading.
        let closed = words.trim_end_matches('#');
        if closed.len() < words.len() && (closed.is_empty() || closed.ends_with([' ', '\t'])) {
            words = closed.trim_end();
        }
        if words.is_empty() {
            return None;
        }
        let offset = start + indent + 1 + (rest[1..].len() - rest[1..].trim_start().len());
        let place = Place {
            source,
            span: Span::new(offset, offset + words.len()),
        };
        return Some(Sourced::new(
            words.to_owned(),
            Origin::written(place, Style::None),
        ));
    }
    None
}
