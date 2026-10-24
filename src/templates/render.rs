//! Building a page: its writing to HTML, its snippet calls through the
//! site's snippets, then its layout and the templates that layout extends.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ops::Range;
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use minijinja::value::{Rest, Value, ValueOrKwargs};
use minijinja::{Environment, Error, State};

use super::card::Gathered;
use super::files::{Snippet, Unnamed, unnamed_fills};
use super::frame::{self, Frame, Handed, Stop};
use super::indent::{Origin, Own};
use super::layout::{self, Chosen};
use super::values::{ListValue, PageValue, SiteValue};
use super::{Templates, explain};
use crate::assets::Assets;
use crate::config::Settings;
use crate::content::words::PageWord;
use crate::dates::Dates;
use crate::highlight::{self, Highlighter, Language};
use crate::markdown::{self, Piece, PieceValue, Placement, Within};
use crate::problem::{
    ArgumentFault, Chooser, Kind, Named, NamedFeed, Problem, Relation, ShownBy, ShownWords,
    Stopped, Word,
};
use crate::reads::{Asked, Key, Reads};
use crate::site::{
    Id, LanguageId, ListId, NameId, Names, PageId, PictureId, Redirect, SentTo, Site, TemplateId,
};
use crate::source::{Place, SourceId, Sources, Span};

/// Everything a page is built from, shared by every page built.
pub(super) struct Shared {
    pub sources: Sources,
    pub settings: Settings,
    pub site: Site,
    pub assets: Assets,
    pub templates: Templates,
    pub dates: Dates,
    /// The path the site is published under, such as `/blog`; empty at the
    /// top of its domain.
    pub base: String,
    outlines: Mutex<HashMap<PageId, Arc<Vec<Section>>>>,
    /// The names known before any page is built, read by every thread
    /// without waiting on another.
    known: Names,
    /// Names first met while building, numbered on from `known`.
    later: RwLock<Names>,
    /// Each page's writing once built, by page.
    pub bodies: Vec<OnceLock<Arc<Body>>>,
    /// Each data file's value once a template has reached it, by file,
    /// with where each text in it is written, by the text's address.
    pub data_values: Vec<OnceLock<(Value, frame::Origins)>>,
    /// Every feed the site publishes.
    feeds: Vec<crate::feeds::Feed>,
    /// Each list's feed among `feeds`, by list.
    feed_of: Vec<Option<usize>>,
    /// The page published at each address, by the address's key, a list's
    /// later pages under their first page; made when first asked.
    standing: OnceLock<HashMap<String, PageId>>,
    /// Every page the site publishes, see [`Shared::every_page`].
    every_page: OnceLock<Vec<(PageId, usize)>>,
    /// Each file beside the writing's place in `site.files`, by path.
    files_by_path: OnceLock<HashMap<String, usize>>,
    /// Each static file's place among the assets' statics, by name.
    statics_by_name: OnceLock<HashMap<String, usize>>,
    /// The photographs beside each page, in the order of their paths.
    photographs: OnceLock<HashMap<PageId, Vec<PictureId>>>,
    /// The pages chosen from every page by what each writes under one
    /// name, each choice worked out once: by the filter, the name and the
    /// test, and the values the test was handed.
    pub chosen: Mutex<HashMap<(String, Vec<Value>), Known>>,
    /// What there is to tell of the pages a sort worked out once put
    /// last, by what the sort is known by.
    pub put_last: Mutex<HashMap<(String, Vec<Value>), PutLast>>,
    /// What colours the code in the writing, in the site's `code_style`.
    pub highlighter: Option<Highlighter>,
    /// What the site's cards come to, each worked out once.
    pub cards: super::card::Room,
}

impl std::fmt::Debug for Shared {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Shared")
    }
}

/// A choice of pages, once some page has worked it out; held by the page
/// working it out while it does.
pub(super) type Known = Arc<Mutex<Option<SomePages>>>;

/// The pages a sort put last for lacking the field sorted by.
#[derive(Clone, Debug)]
pub(super) struct PutLast {
    /// The field they lack, or every field sorted by where they differ.
    pub field: String,
    pub count: usize,
    /// The first of them, and where it stood before the sort.
    pub first: (PageId, usize),
    pub index: usize,
}

/// Some of the site's pages, each with which of its list's pages it is.
pub(super) type SomePages = Arc<Vec<(PageId, usize)>>;

impl Shared {
    /// Every page the site publishes, a list's later pages among them
    /// with their numbers, in the order of their addresses.
    pub(super) fn every_page(&self) -> &[(PageId, usize)] {
        self.every_page.get_or_init(|| {
            let site = &self.site;
            let mut pages: Vec<(&str, PageId, usize)> = site
                .pages
                .iter()
                .filter(|(_, page)| page.lands())
                .map(|(id, page)| (page.address.as_str(), id, 1))
                .collect();
            for (_, list) in site.lists.iter() {
                if let Some(page) = list.page {
                    pages.extend(
                        (2..)
                            .zip(&list.later)
                            .map(|(number, address)| (address.as_str(), page, number)),
                    );
                }
            }
            pages.sort_unstable();
            pages
                .into_iter()
                .map(|(_, id, number)| (id, number))
                .collect()
        })
    }

    /// The place in `site.files` of the file at `path`, the first where
    /// two share it.
    pub(super) fn file_at(&self, path: &str) -> Option<usize> {
        let files = self.files_by_path.get_or_init(|| {
            let mut files = HashMap::with_capacity(self.site.files.len());
            for (at, file) in self.site.files.iter().enumerate() {
                files.entry(file.path.clone()).or_insert(at);
            }
            files
        });
        files.get(path).copied()
    }

    /// The place among the assets' statics of the static file called
    /// `name`, the first where two share it.
    pub(super) fn static_at(&self, name: &str) -> Option<usize> {
        let statics = self.statics_by_name.get_or_init(|| {
            let mut statics = HashMap::new();
            for (at, file) in self.assets.statics().iter().enumerate() {
                statics.entry(file.name.clone()).or_insert(at);
            }
            statics
        });
        statics.get(name).copied()
    }

    /// The photographs beside `page`, in the order of their paths.
    pub(super) fn photographs_beside(&self, page: PageId) -> &[PictureId] {
        let photographs = self.photographs.get_or_init(|| {
            let site = &self.site;
            let mut beside: Vec<(&str, PictureId, PageId)> = site
                .pictures
                .iter()
                .filter(|(_, picture)| {
                    matches!(picture.seen, crate::album::Seen::Photograph { .. })
                })
                .filter_map(|(id, picture)| {
                    let page = site.files[picture.travelling?].page?;
                    Some((picture.path.as_str(), id, page))
                })
                .collect();
            beside.sort_unstable();
            let mut photographs: HashMap<PageId, Vec<PictureId>> = HashMap::new();
            for (_, id, page) in beside {
                photographs.entry(page).or_default().push(id);
            }
            photographs
        });
        photographs.get(&page).map_or(&[], Vec::as_slice)
    }

    /// The id a field or setting is recorded by.
    pub(super) fn name_id(&self, name: &str) -> NameId {
        if let Some(id) = self.known.get(name) {
            return id;
        }
        let after_known = |id: NameId| NameId::from_index(self.known.len() + id.index());
        if let Some(id) = self.later.read().expect("never poisoned").get(name) {
            return after_known(id);
        }
        after_known(self.later.write().expect("never poisoned").intern(name))
    }

    /// The feed `list` publishes, where it publishes one.
    pub(super) fn feed(&self, list: ListId) -> Option<&crate::feeds::Feed> {
        self.feed_of[list.index()].map(|at| &self.feeds[at])
    }

    /// The page published at `address`, or the page whose list's later
    /// page is published there. Which page stands where is known from the
    /// site's pages, which is recorded as read.
    pub(super) fn standing_at(&self, address: &crate::content::address::Address) -> Option<PageId> {
        frame::read(Key::Pages);
        let standing = self.standing.get_or_init(|| {
            let mut standing = HashMap::new();
            for (id, page) in self.site.pages.iter().filter(|(_, page)| page.lands()) {
                standing.insert(page.address.key(), id);
                if let Some(list) = page.speaks_for {
                    for later in &self.site.lists.get(list).later {
                        standing.insert(later.key(), id);
                    }
                }
            }
            standing
        });
        standing.get(&address.key()).copied()
    }

    /// The name a field or setting is recorded under as `id`.
    pub(super) fn name_of(&self, id: NameId) -> String {
        match id.index().checked_sub(self.known.len()) {
            None => self.known.name(id).to_owned(),
            Some(later) => {
                let names = self.later.read().expect("never poisoned");
                names.name(NameId::from_index(later)).to_owned()
            }
        }
    }

    /// The page's address, the site's own path included.
    pub(super) fn address_of(&self, page: PageId) -> String {
        format!(
            "{}{}",
            self.base,
            self.site.pages.get(page).address.as_str()
        )
    }
}

/// A page's writing as built.
#[derive(Debug)]
pub(super) struct Body {
    written: markdown::Written,
    /// Where each part of `written`'s HTML is written in the page's file.
    traced: Arc<Own>,
    /// The same for its excerpt.
    excerpt_traced: Arc<Own>,
    reads: Reads,
    /// Those of `reads` reading the writing as Markdown read, which its
    /// headings turn on.
    reading: Vec<Key>,
    asked: Asked,
    problems: Vec<Problem>,
    /// The pictures the writing was told to show with no description.
    undescribed: Vec<String>,
    /// The photographs whose copies the writing names.
    developed: Vec<crate::album::Shown>,
    /// Where the first block of code coloured in the writing names its
    /// language, and the same for each page whose writing it shows, whole
    /// or as its excerpt.
    coloured: Vec<Place>,
    /// The same for the writing its excerpt shows.
    excerpt_coloured: Vec<Place>,
    /// Whether the writing shows any code coloured, its own or another
    /// page's.
    colours: bool,
    /// What the writing links to, for checking once the site is built.
    writing: crate::links::Writing,
    /// The steps the engine took building it, and whether they ran out.
    steps: u64,
    spent: bool,
    /// How many search boxes the writing asks for.
    search_boxes: usize,
    /// The languages whose search index it names.
    indexed: Vec<LanguageId>,
    /// The writing as its own page shows it whole after search boxes of
    /// the layout's, by how many, its own numbered after them.
    after_boxes: Mutex<Vec<(usize, String, Arc<Own>)>>,
    /// The writing as shown on other pages, made when first shown there.
    elsewhere: OnceLock<Elsewhere>,
    /// The writing as a feed shows it, built again without colour when
    /// first shown there.
    in_feeds: OnceLock<Elsewhere>,
}

/// A page's writing as shown other than on its own page, its HTML and its
/// excerpt: with the addresses read from the page spelled from the top of
/// the site, or in a feed without colour too, each with where its parts
/// were written, which a feed doesn't keep; none where that changes
/// nothing.
#[derive(Debug)]
struct Elsewhere {
    html: Option<(String, Arc<Own>)>,
    excerpt: Option<(String, Arc<Own>)>,
}

/// A part of a page's writing as built: the heading it starts with, by
/// its id and its words, none for the part before the first heading; and
/// its HTML, the heading's own included.
#[derive(Debug)]
pub struct Cut {
    pub heading: Option<(String, String)>,
    pub html: String,
}

/// The words of the heading `html` starts with.
fn heading_words(html: &str) -> String {
    let mut end = html.len();
    for (at, piece) in crate::html::markup(html) {
        if let crate::html::Markup::End(tag) = piece
            && matches!(tag.name.as_str(), "h1" | "h2" | "h3" | "h4" | "h5" | "h6")
        {
            end = at.end;
            break;
        }
    }
    markdown::words_of(&html[..end]).trim().to_owned()
}

/// One page built: its HTML, what it read, its problems, and what it asks
/// the build to make beside it. No HTML where a problem stopped it.
#[derive(Debug)]
pub struct Rendered {
    pub html: Option<String>,
    pub reads: Reads,
    pub asked: Asked,
    pub problems: Vec<Problem>,
    /// The photographs whose copies it names, as each was named.
    pub developed: Vec<crate::album::Shown>,
    /// The sharing cards it asked for, each page's once.
    pub cards: Vec<super::card::Asked>,
    /// Where the first block of code coloured names its language, in each
    /// page's writing it shows, whole or as its excerpt.
    pub coloured: Vec<Place>,
    /// How many pages of lists of the whole site it reached.
    pub walked: u64,
    /// Where its HTML is the own text of the templates it is built with.
    pub own: Own,
    /// The languages whose search index it names.
    pub indexed: Vec<LanguageId>,
    /// The template it is built with, by its index among the files.
    pub layout: Option<usize>,
}

impl Rendered {
    fn of(html: Option<String>, frame: Frame, mut problems: Vec<Problem>) -> Rendered {
        let shared = &frame.shared;
        if html.is_some()
            && !frame.shows_writing
            && let Some(page) = frame.home()
            && has_writing(shared, page)
            && let Some(layout) = frame.layout
        {
            let source = shared.templates.files[layout].source;
            let path = shared.sources.path(source).to_string_lossy();
            let kind = Kind::WritingNeverShown {
                layout: path.replace('\\', "/"),
                pages: 1,
            };
            let head = crate::content::words::head(&shared.sources, source);
            problems.push(met_building(Problem::at(kind, head), &frame));
        }
        let mut pages: Vec<PageId> = frame.reads.cards().collect();
        pages.sort_unstable();
        pages.dedup();
        let cards = pages.into_iter();
        Rendered {
            html,
            cards: cards
                .map(|page| super::card::asked(&frame.shared, page))
                .collect(),
            reads: frame.reads,
            asked: frame.asked,
            problems,
            developed: frame.developed,
            coloured: frame.coloured,
            walked: frame.walked,
            own: Own::default(),
            indexed: frame.indexed,
            layout: frame.layout,
        }
    }
}

/// The photographs `shown` on any output, each once in the order of their
/// ids: with the first place in the writing naming it, and whether its
/// copy at its own width is named anywhere.
pub(crate) fn developed(
    shown: impl IntoIterator<Item = crate::album::Shown>,
) -> Vec<crate::album::Shown> {
    let mut developed: BTreeMap<PictureId, (Option<Place>, bool)> = BTreeMap::new();
    for shown in shown {
        let (first, fuller) = developed
            .entry(shown.id)
            .or_insert((shown.place, shown.full));
        *fuller |= shown.full;
        let earlier = |a: Place, b: Place| (a.source, a.span.start) < (b.source, b.span.start);
        *first = match (*first, shown.place) {
            (Some(first), Some(at)) if earlier(at, first) => Some(at),
            (None, at) => at,
            (first, _) => first,
        };
    }
    developed
        .into_iter()
        .map(|(id, (place, full))| crate::album::Shown { id, place, full })
        .collect()
}

/// Builds the pages of one site.
#[derive(Clone)]
pub struct Renderer {
    pub(super) shared: Arc<Shared>,
}

impl Renderer {
    pub fn new(
        sources: Sources,
        settings: Settings,
        site: Site,
        assets: Assets,
        templates: Templates,
        dates: Dates,
        feeds: Vec<crate::feeds::Feed>,
    ) -> Renderer {
        let base = settings.base();
        let mut feed_of = vec![None; site.lists.len()];
        for (at, feed) in feeds.iter().enumerate() {
            feed_of[feed.list.index()] = Some(at);
        }
        let known = known_names(&settings, &site);
        let highlighter = settings
            .text("code_style")
            .and_then(|style| Highlighter::new(style.value));
        let bodies = std::iter::repeat_with(OnceLock::new)
            .take(site.pages.len())
            .collect();
        let data_values = std::iter::repeat_with(OnceLock::new)
            .take(site.data.files.len())
            .collect();
        let cards = super::card::Room::new(&templates.cards, site.pages.len());
        Renderer {
            shared: Arc::new(Shared {
                sources,
                settings,
                site,
                assets,
                templates,
                dates,
                base,
                outlines: Mutex::new(HashMap::new()),
                known,
                later: RwLock::new(Names::default()),
                bodies,
                data_values,
                feeds,
                feed_of,
                standing: OnceLock::new(),
                every_page: OnceLock::new(),
                files_by_path: OnceLock::new(),
                statics_by_name: OnceLock::new(),
                photographs: OnceLock::new(),
                chosen: Mutex::new(HashMap::new()),
                put_last: Mutex::new(HashMap::new()),
                highlighter,
                cards,
            }),
        }
    }

    pub fn sources(&self) -> &Sources {
        &self.shared.sources
    }

    pub fn site(&self) -> &Site {
        &self.shared.site
    }

    pub fn settings(&self) -> &Settings {
        &self.shared.settings
    }

    pub fn assets(&self) -> &Assets {
        &self.shared.assets
    }

    pub(crate) fn dates(&self) -> &Dates {
        &self.shared.dates
    }

    /// The same site, held twice.
    pub(crate) fn share(&self) -> Renderer {
        Renderer {
            shared: Arc::clone(&self.shared),
        }
    }

    /// Everything to say of the site's cards, once its outputs are built
    /// and have `asked` what they did of them: what stopped one, the pages
    /// left without, and what no page uses.
    pub fn cards_told(&self, asked: &Gathered) -> Vec<Problem> {
        super::card::told(&self.shared, asked)
    }

    /// The cards the outputs built `asked` for, to draw.
    pub fn cards_wanted<'a>(&'a self, asked: &'a Gathered) -> Vec<crate::cards::keep::Wanted<'a>> {
        super::card::wanted(&self.shared, asked)
    }

    /// The names those cards are kept under between builds.
    pub fn cards_kept(&self, asked: &Gathered) -> std::collections::HashSet<String> {
        super::card::kept(&self.shared, asked)
    }

    /// The card of `page`, as an output that read it asked for it.
    pub(crate) fn card_asked(&self, page: PageId) -> super::card::Asked {
        super::card::asked(&self.shared, page)
    }

    /// The pictures that pages are shared with as `og_image` names them,
    /// each with how many pages asked for it.
    pub fn own_cards(&self, asked: &Gathered) -> BTreeMap<PictureId, usize> {
        super::card::own(asked)
    }

    /// The name a field or setting's reads are recorded under as `id`.
    pub(crate) fn name_of(&self, id: NameId) -> String {
        self.shared.name_of(id)
    }

    /// How many names reads have been recorded under so far.
    pub(crate) fn names(&self) -> usize {
        self.shared.known.len() + self.shared.later.read().expect("never poisoned").len()
    }

    /// The path in the site of the template, snippet or text of a card
    /// at `at` among the files.
    pub(crate) fn template_path(&self, at: usize) -> String {
        let source = self.shared.templates.files[at].source;
        let path = self.shared.sources.path(source).to_string_lossy();
        path.replace('\\', "/")
    }

    /// The path in the site of the file `page` is written in.
    pub(crate) fn page_path(&self, page: PageId) -> String {
        format!("content/{}", self.shared.site.pages.get(page).path)
    }

    /// Every template, snippet and text of a card the site has, in the
    /// order of their ids: what it is held as, the file it is read from,
    /// and whether it builds anything.
    pub(crate) fn templates(&self) -> impl Iterator<Item = (String, SourceId, bool)> + '_ {
        let templates = &self.shared.templates;
        templates.files.iter().enumerate().map(|(at, file)| {
            let held = format!("{:?} {}", file.role, file.name);
            (held, file.source, !templates.unusable.contains(&at))
        })
    }

    /// Every template a page can be built with: the name `layout:` gives
    /// it by, and the path of its file in the site.
    pub(crate) fn layouts(&self) -> Vec<(String, String)> {
        let templates = &self.shared.templates;
        (templates.files.iter())
            .filter(|file| file.role == super::files::Role::Template)
            .map(|file| {
                let path = self.shared.sources.path(file.source).to_string_lossy();
                (file.name.clone(), path.replace('\\', "/"))
            })
            .collect()
    }

    /// Every snippet a call can name, with the path of its file in the
    /// site.
    pub(crate) fn snippets(&self) -> Vec<(&super::Snippet, String)> {
        (self.shared.templates.snippets.values())
            .map(|snippet| (snippet, self.template_path(snippet.file)))
            .collect()
    }

    /// Whether the site's snippet `name` wraps the writing below a call to
    /// it, as the writing is read.
    pub(crate) fn wraps(&self, name: &str) -> Option<bool> {
        wraps(&self.shared, name)
    }

    /// The headings of `page`'s writing that have an id, in the order
    /// written.
    pub(crate) fn headings(&self, page: PageId) -> Vec<Headed> {
        let kept = self.shared.outlines.lock().expect("never poisoned");
        let sections = match kept.get(&page) {
            Some(kept) => Arc::clone(kept),
            None => {
                drop(kept);
                Arc::new(writing(&self.shared, page).map_or_else(Vec::new, |d| sections(&d)))
            }
        };
        (sections.iter())
            .filter(|section| !section.id.is_empty())
            .map(|section| Headed {
                id: section.id.clone(),
                words: section.text.clone(),
                place: section.place,
            })
            .collect()
    }

    /// The files the site's cards are drawn with, each by its path in the
    /// site.
    pub(crate) fn card_files(&self) -> Vec<String> {
        super::card::files(&self.shared.templates.cards)
    }

    /// The file of [`Renderer::card_files`] at `path`, opened where it
    /// really is and reached through no link.
    pub(crate) fn open_card_file(&self, path: &str) -> std::io::Result<std::fs::File> {
        super::card::open(&self.shared.templates.cards, path)
    }

    /// The build's date.
    pub(crate) fn today(&self) -> crate::date::Date {
        crate::dates::ShowDates::today(&self.shared.dates)
    }

    /// The feed `list` publishes, where it publishes one.
    pub(crate) fn feed_of(&self, list: ListId) -> Option<&crate::feeds::Feed> {
        self.shared.feed(list)
    }

    /// What `list` is known by but for its members.
    pub(crate) fn list_known(&self, list: ListId) -> super::values::ListKnown<'_> {
        super::values::list_known(&self.shared, list)
    }

    /// What a page's file alone puts on a page as HTML: its writing as
    /// Markdown makes it, so that what it shows as code or escapes is no
    /// markup, between its fields as written. A call is left as the file
    /// spells it, whatever it is handed with it: what its snippet writes
    /// is the snippet's. None for a page whose file isn't Markdown.
    pub(crate) fn written_by_its_file(&self, page: PageId) -> Option<String> {
        let shared = &*self.shared;
        let source = written_as_markdown(shared, page)?;
        let text = shared.sources.text(source);
        let body = shared.site.pages.get(page).body;
        let wraps = |name: &str| wraps(shared, name);
        let document = markdown::read(&shared.sources, source, body, &wraps, &shared.base);
        let options = markdown::Options {
            snippets: &AsSpelled(text),
            back_to_the_text: None,
            digits: None,
            right_to_left: false,
            highlighter: None,
        };
        let range = body.range();
        let mut out = text[..range.start].to_owned();
        out.push_str(&markdown::to_html(&document, &options));
        out.push_str(&text[range.end..]);
        Some(out)
    }

    /// The template whose own text the byte at `at` of an output is, and
    /// where in that template it is written; none for a byte of a value's,
    /// or of a snippet's.
    pub fn written_at(&self, own: &Own, at: usize) -> Option<(TemplateId, usize)> {
        let Origin::Own(from) = own.origin(at)? else {
            return None;
        };
        let (file, offset) = self.shared.templates.holds(from)?;
        Some((Templates::id(file), offset))
    }

    /// Where the value holding bytes `at..at + len` of an output was
    /// written: in a file of the site holding it as markup, those bytes
    /// where the file spells them as the output has them, at the same
    /// place in the text or the only place they are, else the whole text;
    /// for what a template made, the `{{ ... }}` printing it. None for the
    /// template's own text, or a value printed nowhere known.
    pub fn printed_at(&self, own: &Own, html: &str, at: usize, len: usize) -> Option<Place> {
        let sources = &self.shared.sources;
        match own.origin(at)? {
            Origin::Own(_) => None,
            Origin::Text(place, within) => {
                let spelling = sources.spelling(place);
                let wanted = html.get(at..at + len)?;
                let mut found = spelling.match_indices(wanted).map(|(at, _)| at);
                let first = found.next();
                let same = |within: &usize| spelling.get(*within..*within + len) == Some(wanted);
                let within = within
                    .filter(same)
                    .or_else(|| first.filter(|_| found.next().is_none()));
                Some(match within {
                    Some(within) => {
                        let start = place.span.start as usize + within;
                        Place {
                            source: place.source,
                            span: Span::new(start, start + len),
                        }
                    }
                    None => place,
                })
            }
            Origin::Made(place) => {
                let text = sources.text(place.source);
                let (start, end) = (place.span.start as usize, place.span.end as usize);
                let opens = text[..start].rfind("{{")?;
                let closes = end + text[end..].find("}}")? + 2;
                Some(Place {
                    source: place.source,
                    span: Span::new(opens, closes),
                })
            }
        }
    }

    /// The outermost markup of a file's that the byte at `at` of an output
    /// is in: for a byte a snippet called in a page's writing wrote, the
    /// call.
    pub fn in_text(&self, own: &Own, at: usize) -> Option<Place> {
        own.in_text(at)
    }

    /// The template or snippet read from `source`, where it is one.
    pub fn template_of(&self, source: SourceId) -> Option<TemplateId> {
        let files = &self.shared.templates.files;
        let at = files.iter().position(|file| file.source == source)?;
        Some(Templates::id(at))
    }

    /// The file a template or snippet is read from.
    pub fn template_source(&self, id: TemplateId) -> SourceId {
        self.shared.templates.files[id.index()].source
    }

    /// Whether the file is a snippet, built only where the writing or a
    /// template calls it by its name.
    pub fn called_by_name(&self, id: TemplateId) -> bool {
        matches!(
            self.shared.templates.files[id.index()].role,
            super::files::Role::Snippet(_)
        )
    }

    /// The id a field or setting's reads are recorded by.
    pub fn name_id(&self, name: &str) -> NameId {
        self.shared.name_id(name)
    }

    /// Everything `page`'s writing read, once it has been built.
    pub fn writing_read(&self, page: PageId) -> Option<&[Key]> {
        let body = self.shared.bodies[page.index()].get()?;
        Some(body.reads.keys())
    }

    /// What `page`'s writing links to and hands its snippet calls, once
    /// its writing has been built.
    pub fn writing(&self, page: PageId) -> Option<&crate::links::Writing> {
        let body = self.shared.bodies[page.index()].get()?;
        Some(&body.writing)
    }

    /// What `page`'s writing links to and hands its snippet calls, its
    /// writing built now where no output has built it, as where a round
    /// kept the page's output from the round before.
    pub fn writing_built(&self, page: PageId) -> &crate::links::Writing {
        kept_body(&self.shared, page);
        &(self.shared.bodies[page.index()].get())
            .expect("kept just now")
            .writing
    }

    /// `page`'s writing as built, cut where each heading with an id
    /// starts: the part before the first heading, then each heading's
    /// part. Built now where no output has built it.
    pub fn cut_at_headings(&self, page: PageId) -> Vec<Cut> {
        let shared = &self.shared;
        let body = kept_body(shared, page);
        let written = &body.written;
        let outline = shared
            .outlines
            .lock()
            .expect("never poisoned")
            .get(&page)
            .map(Arc::clone);
        let words_of = |id: &str, html: &str| {
            let section = outline
                .as_ref()
                .and_then(|sections| sections.iter().find(|s| s.id == id));
            match section {
                Some(section) => section.text.clone(),
                None => heading_words(html),
            }
        };
        let mut cuts = Vec::with_capacity(written.headings.len() + 1);
        let starts = written.headings.iter().map(|(at, _)| *at);
        let ends = starts.skip(1).chain(std::iter::once(written.html.len()));
        let first = written
            .headings
            .first()
            .map_or(written.html.len(), |(at, _)| *at);
        cuts.push(Cut {
            heading: None,
            html: written.html[..first].to_owned(),
        });
        for ((at, id), end) in written.headings.iter().zip(ends) {
            let html = &written.html[*at..end];
            cuts.push(Cut {
                heading: Some((id.clone(), words_of(id, html))),
                html: html.to_owned(),
            });
        }
        cuts
    }

    /// How a problem met building `page`, or its list's page `number`
    /// counted from 1, names the page, and where.
    pub fn building(&self, page: PageId, number: usize) -> Option<(Relation, Option<Place>)> {
        let frame = Frame {
            number,
            ..Frame::new(Arc::clone(&self.shared), Some(page))
        };
        building(&frame)
    }

    /// The template `page` is built with.
    pub fn layout_of(&self, page: PageId) -> Result<Chosen, Problem> {
        let shared = &self.shared;
        layout::choose(
            &shared.templates,
            &shared.sources,
            &shared.site,
            shared.site.pages.get(page),
        )
    }

    /// Builds `page`: its writing, then its layout.
    pub fn render(&self, page: PageId) -> Rendered {
        self.render_numbered(page, 1)
    }

    /// Builds the page `number`, counted from 2, of the list `page` speaks
    /// for: `page`'s layout, handed the page as it stands there, which
    /// carries none of its writing.
    pub fn render_later(&self, page: PageId, number: usize) -> Rendered {
        self.render_numbered(page, number)
    }

    fn render_numbered(&self, page: PageId, number: usize) -> Rendered {
        let shared = &self.shared;
        frame::push(Frame {
            number,
            ..Frame::new(Arc::clone(shared), Some(page))
        });
        let mut problems = match number {
            1 => match &body(shared, page) {
                Ok(body) => {
                    frame::with(|frame| {
                        frame.steps = body.steps;
                        frame.spent = body.spent;
                        // A picture the writing told isn't told again by the layout.
                        frame.undescribed.clone_from(&body.undescribed);
                    });
                    body.problems.clone()
                }
                Err(_) => Vec::new(),
            },
            _ => Vec::new(),
        };
        layout::read_choice(shared, page, frame::read);
        let chosen = match self.layout_of(page) {
            Ok(chosen) => chosen,
            Err(problem) => {
                let frame = frame::pop();
                // Placed in a file, it is one problem for every page built so.
                problems.push(match problem.place {
                    Some(_) => problem,
                    None => met_building(problem, &frame),
                });
                return Rendered::of(None, frame, problems);
            }
        };
        let context = minijinja::context! {
            page => PageValue::numbered(shared, page, number),
            site => SiteValue::value(shared, frame::language()),
        };
        self.finish(&chosen.name, context, problems)
    }

    /// The problem with a site that sends readers on from old addresses
    /// and has no template to build the pages left at them: one for the
    /// site, placed at the first old address.
    pub fn without_redirect_template(&self) -> Option<Problem> {
        let first = self.shared.site.redirects.first()?;
        if self.shared.templates.is_layout(REDIRECT) {
            return None;
        }
        Some(Problem::at(
            Kind::MissingTemplate {
                path: super::files::template_path(REDIRECT),
                chosen: Chooser::Written,
                nearest: None,
            },
            first.place,
        ))
    }

    /// Builds the page left at an address that sends a reader on, with the
    /// template `redirect`. It is handed `to`, where the reader is sent,
    /// whole where it is on the site and the site has a `url`; `title`,
    /// what the tab says meanwhile: the `title` of the page writing the
    /// redirect, or else what `headline` calls the page the reader is sent
    /// to, where one on the site is, or else `to`; and the page writing
    /// the redirect as `page`.
    pub fn render_redirect(&self, redirect: &Redirect) -> Rendered {
        let shared = &self.shared;
        let page = redirect.page;
        frame::push(Frame {
            redirect: true,
            ..Frame::new(Arc::clone(shared), Some(page))
        });
        frame::read(Key::Setting(shared.name_id("url")));
        frame::read(Key::Field(page, shared.name_id("url")));
        frame::read(Key::Field(page, shared.name_id("title")));
        let on_site = |address: String| match shared.settings.text("url") {
            Some(url) => crate::content::address::whole(&address, super::own::origin_of(url.value)),
            None => address,
        };
        let (to, sent_to) = match &redirect.to {
            SentTo::Page => (on_site(shared.address_of(page)), Some(page)),
            SentTo::Site {
                address, fragment, ..
            } => (
                on_site(format!("{}{}{fragment}", shared.base, address.as_str())),
                shared.standing_at(address),
            ),
            SentTo::Elsewhere(written) => (written.clone(), None),
        };
        let p = shared.site.pages.get(page);
        let title = match (&p.title, sent_to) {
            (Some(title), _) if p.words.get(PageWord::Title).is_some() => title.value.clone(),
            // Only writing asking for its own opening words is refused, and
            // none is being built here.
            (_, Some(sent_to)) => {
                super::own::headline(shared, sent_to).unwrap_or_else(|_| to.clone())
            }
            _ => to.clone(),
        };
        let context = minijinja::context! {
            to,
            title,
            page => PageValue::value(shared, page),
            site => SiteValue::value(shared, frame::language()),
        };
        self.finish(REDIRECT, context, Vec::new())
    }

    /// What a list calls the page `id`, as `headline` does, and its date as
    /// the site writes dates, where it has one it isn't already called by.
    /// What finding the name tells of the page is left to its own build.
    pub fn named(&self, id: PageId) -> (String, Option<String>) {
        let shared = &self.shared;
        frame::push(Frame::new(Arc::clone(shared), Some(id)));
        let called = super::own::called(shared, id);
        let page = shared.site.pages.get(id);
        let day = |date: &crate::date::Date| super::values::written_date(shared, date, None).ok();
        let named = match called {
            Ok((name, Some(crate::problem::Called::Date))) => (name, None),
            Ok((name, _)) => (name, page.date.as_ref().and_then(|date| day(&date.value))),
            Err(_) => (shared.address_of(id), None),
        };
        frame::pop();
        named
    }

    /// Every page the site publishes, a list's later pages among them
    /// with their numbers, in the order of their addresses.
    pub fn every_page(&self) -> &[(PageId, usize)] {
        self.shared.every_page()
    }

    /// Every feed the site publishes.
    pub fn feeds(&self) -> &[crate::feeds::Feed] {
        &self.shared.feeds
    }

    /// Builds a list's feed with its template, handed `list`, its dated
    /// `posts` newest first, and `page`, the page speaking for the list,
    /// where one does.
    pub fn render_feed(&self, feed: &crate::feeds::Feed) -> Rendered {
        let shared = &self.shared;
        let page = shared.site.lists.get(feed.list).page;
        frame::push(Frame {
            feed: Some(feed.clone()),
            colourless: true,
            language: shared.site.lists.get(feed.list).language,
            ..Frame::new(Arc::clone(shared), page)
        });
        frame::read(Key::ListHead(feed.list));
        let posts: Vec<Value> = crate::feeds::entries(&shared.site, &shared.dates, feed.list)
            .into_iter()
            .map(|post| PageValue::value(shared, post))
            .collect();
        let mut context: BTreeMap<&str, Value> = BTreeMap::new();
        context.insert("list", ListValue::value(shared, feed.list));
        context.insert("posts", Value::from(posts));
        context.insert("site", SiteValue::value(shared, frame::language()));
        if let Some(page) = page {
            context.insert("page", PageValue::value(shared, page));
        }
        self.finish(&feed.template, Value::from(context), Vec::new())
    }

    /// Builds the template `name` with `context`, on the frame pushed for
    /// the output, and pops it.
    fn finish(&self, name: &str, context: Value, mut problems: Vec<Problem>) -> Rendered {
        let shared = &self.shared;
        let chain = shared.templates.chain(name);
        for &at in &chain {
            frame::read(Key::Template(Templates::id(at)));
        }
        if chain
            .iter()
            .any(|at| shared.templates.unusable.contains(at))
        {
            let frame = frame::pop();
            return Rendered::of(None, frame, problems);
        }
        let layout = shared.templates.by_name.get(name).copied();
        frame::with(|frame| frame.layout = layout);
        let mut own = Own::default();
        let result = shared
            .templates
            .env
            .get_template(name)
            .and_then(|template| {
                let at = layout.expect("a template the engine holds");
                frame::begin(at, None)?;
                let built = super::indent::render_placed(
                    &template,
                    context,
                    shared.templates.placing(name),
                    true,
                );
                frame::end(
                    at,
                    built.map(|(captured, text, placed)| {
                        own = placed;
                        (captured, text)
                    }),
                )?
            });
        let mut frame = frame::pop();
        problems.append(&mut frame.problems);
        let html = match result {
            Ok(html) => Some(html),
            Err(error) => {
                problems.extend(explain::explain(&error, &mut frame, &[]));
                None
            }
        };
        let html = html.filter(|_| {
            !problems
                .iter()
                .any(|p| p.kind.severity() == crate::problem::Severity::Error)
        });
        Rendered {
            own,
            ..Rendered::of(html, frame, problems)
        }
    }
}

/// The template that builds the page left at an old address.
pub(crate) const REDIRECT: &str = "redirect";

/// Every name a build can be told before it starts that a page or the site
/// may be asked for: Statac's own words and settings, and every field the
/// pages and settings write.
fn known_names(settings: &Settings, site: &Site) -> Names {
    use crate::content::words::PageWord;
    let mut names = Names::default();
    for word in PageWord::all() {
        names.intern(word.name());
    }
    for name in crate::config::setting_names() {
        names.intern(name);
    }
    for own in &settings.own {
        names.intern(&own.name);
    }
    for name in super::values::PAGE_ANSWERS {
        names.intern(name);
    }
    for (_, page) in site.pages.iter() {
        for field in &page.words.fields {
            names.intern(&field.name);
        }
    }
    names
}

thread_local! {
    /// The pages whose writing this thread is building, outermost first.
    static WRITING: RefCell<Vec<PageId>> = const { RefCell::new(Vec::new()) };
}

/// A page's writing, built once and kept. What building it read is
/// recorded on the output asking for it.
fn body(shared: &Arc<Shared>, page: PageId) -> Result<Arc<Body>, Error> {
    frame::read(Key::Body(page));
    // Built again, writing that shows itself stops as it did when built
    // first, though by then it is kept.
    if WRITING.with_borrow(|writing| writing.contains(&page)) {
        return Err(frame::raise(Kind::TemplateStopped {
            reason: Stopped::ContentInItself,
        }));
    }
    let body = kept_body(shared, page);
    frame::with(|frame| {
        for &key in body.reads.keys() {
            frame.reads.read(key);
        }
        // What the writing asked of its page, the output's own page asked.
        if frame.page == Some(page) && frame.number == 1 {
            frame.asked.extend(&body.asked);
        }
        frame.developed.extend(body.developed.iter().copied());
    });
    Ok(body)
}

/// A page's writing, built the first time it is asked for and kept.
fn kept_body(shared: &Arc<Shared>, page: PageId) -> Arc<Body> {
    let slot = &shared.bodies[page.index()];
    if let Some(body) = slot.get() {
        return Arc::clone(body);
    }
    WRITING.with_borrow_mut(|writing| writing.push(page));
    let built = Arc::new(build_body(shared, page, 0, false));
    WRITING.with_borrow_mut(Vec::pop);
    // Another thread may have built it meanwhile; the first kept wins.
    let _ = slot.set(built);
    Arc::clone(slot.get().expect("set just now"))
}

/// Whether a page's file holds writing: anything below its front matter
/// but space and HTML comments.
fn has_writing(shared: &Shared, page: PageId) -> bool {
    let p = shared.site.pages.get(page);
    let Some(source) = p.source else {
        return false;
    };
    let mut rest = &shared.sources.text(source)[p.body.range()];
    loop {
        rest = rest.trim_start_matches(|c: char| c.is_whitespace() || c == '\u{feff}');
        match rest.strip_prefix("<!--") {
            // A comment left open runs to the end, as Markdown reads it.
            Some(comment) => rest = comment.split_once("-->").map_or("", |(_, after)| after),
            None => return !rest.is_empty(),
        }
    }
}

/// Whether the site's snippet `name` wraps the writing below a call to it,
/// which it does where it shows the writing; none where the site has no
/// such snippet, or its snippet can't be read.
fn wraps(shared: &Shared, name: &str) -> Option<bool> {
    let snippet = shared.templates.snippets.get(name)?;
    snippet.readable.then_some(!snippet.wrapped.is_empty())
}

/// The file a page's writing is read from as Markdown; none for a page
/// written in HTML, or one Statac made with none.
fn written_as_markdown(shared: &Shared, page: PageId) -> Option<SourceId> {
    let p = shared.site.pages.get(page);
    let source = p.source?;
    (!(p.path.ends_with(".html") || p.path.ends_with(".htm"))).then_some(source)
}

/// Writes each call as the file spells it, before the writing it wraps.
struct AsSpelled<'a>(&'a str);

impl markdown::Snippets for AsSpelled<'_> {
    fn html(&self, _: &markdown::Call, place: Place, _: Placement, body: Option<&str>) -> String {
        let mut out = self.0[place.span.range()].to_owned();
        out.push_str(body.unwrap_or_default());
        out
    }
}

/// A page's writing read as Markdown, its headings given their ids; none
/// for a page written in HTML, or one Statac made with none.
fn writing(shared: &Shared, page: PageId) -> Option<markdown::Document> {
    let p = shared.site.pages.get(page);
    let source = written_as_markdown(shared, page)?;
    let wraps = |name: &str| wraps(shared, name);
    let mut document = markdown::read(&shared.sources, source, p.body, &wraps, &shared.base);
    // An address inside the site moves with the site.
    if !document.in_site.is_empty() {
        frame::read(Key::Setting(shared.name_id("url")));
    }
    let path = shared.sources.path(source).to_string_lossy();
    markdown::give_heading_ids(&mut document, &path, shared.sources.text(source));
    if let Some(words) = typography_of(shared, page) {
        let typography = markdown::Typography::of(words);
        markdown::set_typography(&mut document, &shared.sources, &typography);
    }
    Some(document)
}

/// The words of the language a page's writing is set in, where the site
/// sets its writing's punctuation at all.
pub(super) fn typography_of(shared: &Shared, page: PageId) -> Option<&crate::language::Words> {
    frame::read(Key::Setting(shared.name_id("typography")));
    if shared.settings.truth("typography") == Some(false) {
        return None;
    }
    Some(words_of(shared, shared.site.pages.get(page).language))
}

/// The words of `language`, recorded as read.
pub(super) fn words_of(shared: &Shared, language: LanguageId) -> &crate::language::Words {
    frame::read(Key::Language(language));
    &shared.dates.language(language).words
}

/// The headings of a page's writing, as its table of contents lists them.
#[derive(Debug)]
pub(super) struct Section {
    pub level: u8,
    pub id: String,
    pub text: String,
    pub place: Place,
}

/// A heading of a page's writing.
#[derive(Debug)]
pub(crate) struct Headed {
    /// What an address names it by after `#`.
    pub id: String,
    /// Its words, as plain text.
    pub words: String,
    /// Where it is written.
    pub place: Place,
}

fn sections(document: &markdown::Document) -> Vec<Section> {
    markdown::headings(document)
        .into_iter()
        .map(|heading| Section {
            level: heading.level,
            id: heading.id.unwrap_or_default().to_owned(),
            text: heading.text,
            place: heading.place,
        })
        .collect()
}

/// `page.toc`'s headings, read once and kept. The writing's own calls may
/// ask for them while it is being built.
pub(super) fn outline(shared: &Arc<Shared>, page: PageId) -> Arc<Vec<Section>> {
    frame::read(Key::Body(page));
    let kept = || {
        let outlines = shared.outlines.lock().expect("never poisoned");
        outlines.get(&page).map(Arc::clone)
    };
    if let Some(kept) = kept() {
        return kept;
    }
    // A page published at its own address has its writing built in every
    // build, which keeps the headings, so building it now reads it once.
    // Not while other writing is being built, which the page's writing
    // may show and would then find showing itself, nor where nothing read
    // is recorded, which would keep the writing with none of its reads.
    if written_as_markdown(shared, page).is_some()
        && shared.site.pages.get(page).lands()
        && WRITING.with_borrow(Vec::is_empty)
        && frame::recording()
    {
        let body = kept_body(shared, page);
        for &key in &body.reading {
            frame::read(key);
        }
        if let Some(kept) = kept() {
            return kept;
        }
    }
    let found = Arc::new(writing(shared, page).map_or_else(Vec::new, |d| sections(&d)));
    let mut outlines = shared.outlines.lock().expect("never poisoned");
    Arc::clone(outlines.entry(page).or_insert(found))
}

/// Builds a page's writing, its search boxes numbered after
/// `boxes_before` others, and its code without colour where `colourless`.
fn build_body(shared: &Arc<Shared>, page: PageId, boxes_before: usize, colourless: bool) -> Body {
    super::timed::start();
    let body = build_body_untimed(shared, page, boxes_before, colourless);
    super::timed::writing_ended(page);
    body
}

fn build_body_untimed(
    shared: &Arc<Shared>,
    page: PageId,
    boxes_before: usize,
    colourless: bool,
) -> Body {
    let p = shared.site.pages.get(page);
    frame::push(Frame {
        making: frame::Making::Writing,
        search_boxes: boxes_before,
        colourless,
        ..Frame::new(Arc::clone(shared), Some(page))
    });
    let mut problems = Vec::new();
    let mut wordless = Vec::new();
    let mut excerpt_coloured = Vec::new();
    let mut links = crate::links::Writing::default();
    let document = writing(shared, page);
    // The frame is new, so what it has read the reading read.
    let reading = frame::with(|frame| frame.reads.keys().to_vec()).unwrap_or_default();
    let mut calls = Vec::new();
    let written = match document {
        None => match p.source {
            Some(source) => {
                undescribed_in_html(shared, page, source, p.body);
                links.written = crate::links::written_in_html(&shared.sources, source, p.body);
                let mut written =
                    markdown::as_written(&shared.sources.text(source)[p.body.range()]);
                written.traced = vec![(
                    0..written.html.len(),
                    Place {
                        source,
                        span: p.body,
                    },
                )];
                written
            }
            None => markdown::Written::default(),
        },
        Some(document) => {
            links.written = crate::links::written(&document, &shared.sources);
            links.handed = crate::links::handed(&document);
            problems.extend(document.problems.iter().cloned());
            let blocks = code_languages(shared, &document, &mut problems);
            if let Some(&first) = blocks.first() {
                frame::with(|frame| {
                    frame.coloured.push(first);
                    frame.colours = true;
                });
            }
            for top in std::iter::once(&document.root).chain(&document.notes) {
                undescribed_in_node(shared, page, top);
            }
            shared
                .outlines
                .lock()
                .expect("never poisoned")
                .entry(page)
                .or_insert_with(|| Arc::new(sections(&document)));
            let prose = Prose {
                shared,
                page,
                document: &document,
                failed: RefCell::new(BTreeSet::new()),
                in_html_said: Cell::new(false),
                unreadable_said: RefCell::new(Vec::new()),
                moved: RefCell::new(Vec::new()),
                pieced: RefCell::new(Vec::new()),
                colouring: RefCell::new(Vec::new()),
                calls: RefCell::new(Vec::new()),
                wrapped: RefCell::new(Vec::new()),
            };
            let words = words_of(shared, p.language);
            let written = markdown::write(
                &document,
                &markdown::Options {
                    snippets: &prose,
                    back_to_the_text: words.back_to_the_text.as_deref(),
                    digits: words.digits.as_ref(),
                    right_to_left: words.direction == crate::language::Direction::RightToLeft,
                    highlighter: shared.highlighter.as_ref().filter(|_| !colourless),
                },
            );
            wordless = missing_words(shared, p.language, &document);
            if let Some(span) = written.excerpt_span {
                let inside = |place: &Place, span: crate::source::Span| {
                    place.source == document.root.place.source
                        && span.start <= place.span.start
                        && place.span.end <= span.end
                };
                // A note is written apart from the excerpt wherever it is defined.
                let shown = |place: &Place| {
                    inside(place, span)
                        && !document
                            .notes
                            .iter()
                            .any(|note| inside(place, note.place.span))
                };
                excerpt_coloured.extend(blocks.iter().find(|&place| shown(place)));
                frame::with(|frame| {
                    for (place, added) in prose.colouring.take() {
                        if shown(&place) {
                            excerpt_coloured.extend_from_slice(&frame.coloured[added]);
                        }
                    }
                });
            }
            // A picture's tag leads where it was written to, not where
            // the writing spells it.
            for (place, address) in prose.moved.into_inner() {
                for linked in &mut links.written {
                    if linked.place == place {
                        linked.address.clone_from(&address);
                    }
                }
            }
            // An address the site's own file was handed is followed only
            // where that file wrote it on the page.
            let pieced = prose.pieced.into_inner();
            let (handed, kept): (Vec<_>, Vec<_>) = std::mem::take(&mut links.written)
                .into_iter()
                .partition(|linked| pieced.contains(&linked.place));
            links.written = kept;
            links.handed.extend(handed);
            calls = prose.calls.into_inner();
            written
        }
    };
    let mut frame = frame::pop();
    problems.append(&mut frame.problems);
    let wordless = wordless.into_iter().map(Problem::nowhere);
    problems.extend(wordless.map(|problem| met_building(problem, &frame)));
    let traced = Own::of_writing(
        &written.traced,
        &written.called,
        &calls,
        &written.html,
        &shared.sources,
    );
    let excerpt_traced = Arc::new(traced.copied(&written.excerpt_from));
    let traced = Arc::new(traced);
    Body {
        written,
        traced,
        excerpt_traced,
        undescribed: frame.undescribed,
        reads: frame.reads,
        reading,
        asked: frame.asked,
        problems,
        developed: frame.developed,
        coloured: frame.coloured,
        excerpt_coloured,
        colours: frame.colours,
        writing: links,
        elsewhere: OnceLock::new(),
        in_feeds: OnceLock::new(),
        steps: frame.steps,
        spent: frame.spent,
        search_boxes: frame.search_boxes - boxes_before,
        indexed: frame.indexed,
        after_boxes: Mutex::new(Vec::new()),
    }
}

/// A page's writing whole as the page's own output shows it, where the
/// writing asks for search boxes: its boxes counted with the output's, so
/// the page has one first box and no two with one id. Shown after boxes
/// the layout asked for, the writing is built again with its own numbered
/// after them, since the writing kept numbers its own from the first.
fn among_the_pages_boxes(shared: &Arc<Shared>, page: PageId, body: &Body) -> Value {
    let before = frame::with(|frame| {
        *frame.boxes_before_writing.get_or_insert_with(|| {
            let before = frame.search_boxes;
            frame.search_boxes += body.search_boxes;
            before
        })
    });
    let before = before.unwrap_or_default();
    if before == 0 {
        return frame::kept(body.written.html.clone(), Arc::clone(&body.traced), true);
    }
    let kept = |body: &Body| {
        let kept = body.after_boxes.lock().expect("never poisoned");
        let found = kept.iter().find(|(after, ..)| *after == before);
        found.map(|(_, html, traced)| (html.clone(), Arc::clone(traced)))
    };
    if let Some((html, traced)) = kept(body) {
        return frame::kept(html, traced, true);
    }
    // What building it again tells and reads, the first building did.
    WRITING.with_borrow_mut(|writing| writing.push(page));
    let again = build_body(shared, page, before, false);
    WRITING.with_borrow_mut(Vec::pop);
    let mut kept = body.after_boxes.lock().expect("never poisoned");
    let html = again.written.html;
    kept.push((before, html.clone(), Arc::clone(&again.traced)));
    frame::kept(html, again.traced, true)
}

/// The words `document` is shown with that its page's language has none
/// of: what a screen reader says for a footnote's way back, and the names
/// of the callouts. Nothing where the language has no words at all, which
/// is told of the language, or where nothing shows the words: the site's
/// own file for the footnotes or a callout never reads them, or the site
/// has no file for a callout.
fn missing_words(
    shared: &Shared,
    language: LanguageId,
    document: &markdown::Document,
) -> Vec<Kind> {
    use crate::language::{FOLDER, Whence};
    let spoken = shared.dates.language(language);
    let (Some(tag), false) = (&spoken.tag, spoken.whence == Whence::Nothing) else {
        return Vec::new();
    };
    let file = match &spoken.whence {
        Whence::Site(path) => path.clone(),
        _ => format!("{FOLDER}/{tag}.yaml"),
    };
    let words = words_of(shared, language);
    let templates = &shared.templates;
    let footnotes = words.back_to_the_text.is_none()
        && !document.notes.is_empty()
        && templates.piece_shows(Piece::Footnotes, "label");
    let callouts = words.callouts.is_none()
        && markdown::has_callout(document)
        && templates.piece_shows(Piece::Callout, "title");
    [
        (footnotes, ShownWords::BackToTheText),
        (callouts, ShownWords::Callouts),
    ]
    .into_iter()
    .filter(|(missing, _)| *missing)
    .map(|(_, words)| Kind::MissingWords {
        tag: tag.clone(),
        file: file.clone(),
        words,
        pages: 1,
    })
    .collect()
}

/// Tells each `<img>` in the HTML at `span` of `page`'s writing that has
/// no `alt` at all, at the address it shows. One with an empty `alt` is
/// marked as decoration, as HTML marks one.
fn undescribed_in_html(shared: &Shared, page: PageId, source: SourceId, span: crate::source::Span) {
    let text = &shared.sources.text(source)[span.range()];
    crate::html::start_tags(text, |tag| {
        if tag.name != "img" || tag.get("alt").is_some() {
            return;
        }
        let shown = ["src", "srcset"].into_iter().find_map(|name| {
            let attribute = tag.attributes.iter().find(|a| a.name == name)?;
            let found =
                crate::html::addresses(&attribute.value, name == "srcset", attribute.at.start);
            found.first().copied()
        });
        let Some((written, at)) = shown else {
            return;
        };
        let start = span.start as usize + at;
        let at = Place {
            source,
            span: crate::source::Span::new(start, start + written.len()),
        };
        let picture = crate::html::decoded(written, false);
        let kind = Kind::UndescribedPicture {
            picture: picture.clone(),
            shown: ShownBy::Html,
        };
        let found =
            super::media::found(shared, page, &picture).map(|id| shared.site.pictures.get(id));
        if found.is_some_and(crate::album::Picture::decorative) {
            return;
        }
        let key = found.map_or(picture, crate::album::Picture::shown);
        super::media::undescribed(key, |_| Problem::at(kind, at));
    });
}

/// Tells the undescribed pictures in the HTML among `node`'s writing.
fn undescribed_in_node(shared: &Shared, page: PageId, node: &markdown::Node) {
    // A block of HTML is read whole, as a tag may run over its lines.
    if matches!(
        node.kind,
        markdown::Kind::HtmlBlock | markdown::Kind::Html(_)
    ) {
        undescribed_in_html(shared, page, node.place.source, node.place.span);
        return;
    }
    for child in &node.children {
        undescribed_in_node(shared, page, child);
    }
}

/// A page's opening words as built, or the refusal that stops the
/// template asking where building them stopped.
pub(super) fn opening(shared: &Arc<Shared>, page: PageId) -> Result<Option<String>, Error> {
    body(shared, page).map(|body| body.written.opening.clone())
}

/// Tells each block of code whose language Statac has no colours for, and
/// each left uncoloured for a line too long, at the word naming its
/// language, and records the style read where any is coloured. Gives
/// where the first block coloured names its language.
fn code_languages(
    shared: &Shared,
    document: &markdown::Document,
    problems: &mut Vec<Problem>,
) -> Vec<Place> {
    if shared.highlighter.is_none() {
        frame::read(Key::Setting(shared.name_id("code_style")));
        return Vec::new();
    }
    let mut coloured = Vec::new();
    let mut visit = |node: &markdown::Node| {
        let markdown::Kind::CodeBlock { info, text, .. } = &node.kind else {
            return;
        };
        let Some(word) = markdown::language_word(info) else {
            return;
        };
        match highlight::first_word(word).language() {
            Language::Plain => {}
            Language::Known(_) => {
                let place = fence_word(&shared.sources, node.place).unwrap_or(node.place);
                match highlight::too_long(text) {
                    None => coloured.push(place),
                    Some(characters) => {
                        problems.push(Problem::at(
                            Kind::CodeNotColoured {
                                characters,
                                block: true,
                            },
                            place,
                        ));
                    }
                }
            }
            Language::Unknown { nearest } => {
                let place = fence_word(&shared.sources, node.place);
                problems.push(Problem::at(
                    Kind::UnknownCodeLanguage {
                        written: place
                            .map_or(word, |place| shared.sources.spelling(place))
                            .to_owned(),
                        nearest: nearest.map(str::to_owned),
                    },
                    place.unwrap_or(node.place),
                ));
            }
        }
    };
    document.root.walk(&mut visit);
    for note in &document.notes {
        note.walk(&mut visit);
    }
    if !coloured.is_empty() {
        frame::read(Key::Setting(shared.name_id("code_style")));
    }
    coloured
}

/// The inside of a `<code>` holding `text`, which a template shows:
/// coloured as `language` where that names a grammar and no line is too
/// long, and as it is otherwise. A line too long is told at `at`, where
/// coloured code is recorded as shown.
pub(super) fn code_html(
    shared: &Shared,
    text: &str,
    language: Language,
    at: impl FnOnce() -> Option<Place>,
) -> String {
    let Language::Known(name) = language else {
        return highlight::plain(text);
    };
    if frame::with(|frame| frame.colourless).unwrap_or(false) {
        return highlight::plain(text);
    }
    if shared.highlighter.is_none() {
        frame::read(Key::Setting(shared.name_id("code_style")));
        return highlight::plain(text);
    }
    if let Some(characters) = highlight::too_long(text) {
        let kind = Kind::CodeNotColoured {
            characters,
            block: false,
        };
        super::media::told_at(at(), kind);
        return highlight::plain(text);
    }
    frame::read(Key::Setting(shared.name_id("code_style")));
    let at = at();
    frame::with(|frame| {
        frame.coloured.extend(at);
        frame.colours = true;
    });
    match &shared.highlighter {
        Some(highlighter) => highlighter.html(name, text).as_ref().to_owned(),
        None => highlight::plain(text),
    }
}

/// Whether the site has `code_style: none`, recorded as read.
pub(super) fn colouring_off(shared: &Shared) -> bool {
    frame::read(Key::Setting(shared.name_id("code_style")));
    shared.highlighter.is_none()
}

/// Where the first word after a code block's opening fence is written.
fn fence_word(sources: &Sources, block: Place) -> Option<Place> {
    let text = sources.text(block.source);
    let start = block.span.start as usize;
    let line = &text[start..];
    let line = &line[..line.find('\n').unwrap_or(line.len())];
    let fence = line.trim_start();
    let marker = fence.chars().next().filter(|c| matches!(c, '`' | '~'))?;
    let after = fence.trim_start_matches(marker);
    let word_start = line.len() - after.trim_start().len();
    let word = after.split_whitespace().next()?;
    let begin = start + word_start;
    Some(Place {
        source: block.source,
        span: crate::source::Span::new(begin, begin + word.len()),
    })
}

/// What a page's writing answers as built: `page.words`, `page.maths` and
/// the like. Where building it stopped, the refusal that stops the
/// template asking.
pub(super) fn of_body(
    shared: &Arc<Shared>,
    page: PageId,
    answer: impl FnOnce(&markdown::Written) -> Option<Value>,
) -> Option<Value> {
    match body(shared, page) {
        Ok(body) => answer(&body.written),
        Err(error) => Some(Value::from(error)),
    }
}

/// A part of a page's writing that is HTML.
#[derive(Copy, Clone)]
pub(super) enum Part {
    Content,
    Excerpt,
}

/// `page.content` or `page.excerpt`: as built on the page's own page, and
/// elsewhere with each address read from the page spelled from the top of
/// the site, so it leads where it leads there, and in a feed built without
/// colour. That depends on where the page is published, which is recorded
/// as read. Where building it stopped, the refusal that stops the template
/// asking.
pub(super) fn html_of(shared: &Arc<Shared>, page: PageId, part: Part) -> Option<Value> {
    frame::with(|frame| frame.shows_writing |= frame.home() == Some(page));
    let body = match body(shared, page) {
        Ok(body) => body,
        Err(error) => return Some(Value::from(error)),
    };
    frame::with(|frame| {
        for &language in &body.indexed {
            frame.names_index(language);
        }
    });
    let colourless = frame::with(|frame| frame.colourless).unwrap_or(false);
    if !colourless {
        frame::with(|frame| {
            frame.coloured.extend(match part {
                Part::Content => &body.coloured,
                Part::Excerpt => &body.excerpt_coloured,
            });
            frame.colours |= body.colours;
        });
    }
    let (own, traced) = match part {
        Part::Content => (&body.written.html, &body.traced),
        Part::Excerpt => (body.written.excerpt.as_ref()?, &body.excerpt_traced),
    };
    let (home, making) = frame::with(|frame| (frame.home(), frame.making)).unzip();
    if home.flatten() == Some(page) {
        let counted = matches!(part, Part::Content)
            && body.search_boxes > 0
            && making == Some(frame::Making::Output);
        return Some(match counted {
            true => among_the_pages_boxes(shared, page, &body),
            false => frame::kept(own.clone(), Arc::clone(traced), true),
        });
    }
    frame::read(Key::Field(page, shared.name_id("url")));
    let elsewhere = body.elsewhere.get_or_init(|| {
        let address = shared.address_of(page);
        let moved = |html: &str, traced: &Own| {
            let (moved, respelt) = crate::elsewhere::html(html, &address)?;
            let traced = traced.respelt(&respelt, html.len());
            Some((moved, Arc::new(traced)))
        };
        let written = &body.written;
        Elsewhere {
            html: moved(&written.html, &body.traced),
            excerpt: (written.excerpt.as_deref())
                .and_then(|excerpt| moved(excerpt, &body.excerpt_traced)),
        }
    });
    let moved = match part {
        Part::Content => &elsewhere.html,
        Part::Excerpt => &elsewhere.excerpt,
    };
    let (shown, traced) = moved
        .as_ref()
        .map_or((own, traced), |(html, traced)| (html, traced));
    if !colourless || !body.colours {
        return Some(frame::kept(shown.clone(), Arc::clone(traced), true));
    }
    let in_feeds = match body.in_feeds.get() {
        Some(in_feeds) => in_feeds,
        None => {
            // What building it again tells and reads, the first building did.
            WRITING.with_borrow_mut(|writing| writing.push(page));
            let again = build_body(shared, page, 0, true).written;
            WRITING.with_borrow_mut(Vec::pop);
            let address = shared.address_of(page);
            let moved = |html: String| {
                let moved = crate::elsewhere::html(&html, &address).map(|(moved, _)| moved);
                Some((moved.unwrap_or(html), Arc::default()))
            };
            // Another thread may have built it meanwhile; the first kept wins.
            let _ = body.in_feeds.set(Elsewhere {
                html: moved(again.html),
                excerpt: again.excerpt.and_then(moved),
            });
            body.in_feeds.get().expect("set just now")
        }
    };
    let uncoloured = match part {
        Part::Content => &in_feeds.html,
        Part::Excerpt => &in_feeds.excerpt,
    };
    let uncoloured = uncoloured.as_ref().map_or(shown, |(html, _)| html);
    Some(Value::from_safe_string(uncoloured.clone()))
}

/// Builds the snippet calls in a page's writing, and the pieces of it the
/// site writes itself.
/// Where each part of the writing a call wraps is written, as the
/// writing's HTML has it.
struct Wrapped {
    place: Place,
    traced: Vec<(Range<usize>, Place)>,
    called: Vec<markdown::Called>,
}

struct Prose<'a> {
    shared: &'a Arc<Shared>,
    page: PageId,
    document: &'a markdown::Document,
    /// Pieces whose file stopped once on this page, which are written as
    /// Statac writes them for the rest of it, so the one mistake is told
    /// once.
    failed: RefCell<BTreeSet<Piece>>,
    /// Whether a call inside HTML has been built on this page, which is
    /// said of the first.
    in_html_said: Cell<bool>,
    /// Where a picture that can't be read has been told of, a piece that
    /// stopped being written a second time as Statac writes it.
    unreadable_said: RefCell<Vec<Place>>,
    /// Each picture's address written other than the writing spells it,
    /// so it reaches the file, by where it is written.
    moved: RefCell<Vec<(Place, String)>>,
    /// Where each picture the site's own file wrote has its address
    /// written: the page carries what that file made of it.
    pieced: RefCell<Vec<Place>>,
    /// Where each call and piece is written, with the places in the
    /// frame's `coloured` that building it added.
    colouring: RefCell<Vec<(Place, Range<usize>)>>,
    /// Where what each call's snippet wrote was written, by the call's
    /// place.
    calls: RefCell<Vec<(Place, Arc<Own>)>>,
    /// Where each part of the writing a call wraps is written, by the
    /// call's place, until the call is built.
    wrapped: RefCell<Vec<Wrapped>>,
}

impl Prose<'_> {
    /// `build`, recording the coloured places it adds as those of the call
    /// or piece at `place`.
    fn counted<T>(&self, place: Place, build: impl FnOnce() -> T) -> T {
        let coloured = || frame::with(|frame| frame.coloured.len()).unwrap_or_default();
        let from = coloured();
        let built = build();
        let to = coloured();
        if to > from {
            self.colouring.borrow_mut().push((place, from..to));
        }
        built
    }

    /// Where `address` is written within the picture at `place`.
    fn address_place(&self, address: &str, place: Place) -> Place {
        if let Some(stands) = self.document.in_site_at(address, place) {
            return stands;
        }
        let spelling = self.shared.sources.spelling(place);
        match spelling.rfind(address).filter(|_| !address.is_empty()) {
            Some(at) => {
                let start = place.span.start as usize + at;
                Place {
                    source: place.source,
                    span: crate::source::Span::new(start, start + address.len()),
                }
            }
            None => place,
        }
    }
}

impl markdown::Snippets for Prose<'_> {
    fn wraps(&self, place: Place, traced: &[(Range<usize>, Place)], called: &[markdown::Called]) {
        self.wrapped.borrow_mut().push(Wrapped {
            place,
            traced: traced.to_vec(),
            called: called.to_vec(),
        });
    }

    fn writes(&self, piece: Piece) -> bool {
        self.shared.templates.pieces.contains_key(&piece) && !self.failed.borrow().contains(&piece)
    }

    fn reads(&self, piece: Piece, name: &str) -> bool {
        super::pieces::reads(self.shared, piece, name)
    }

    fn piece(
        &self,
        piece: Piece,
        values: Vec<(&'static str, PieceValue)>,
        place: Place,
    ) -> Option<String> {
        self.counted(place, || {
            // A picture is told where its address is written, as a picture
            // Statac writes is.
            let calling = match (piece, values.first()) {
                (Piece::Image, Some(("address", PieceValue::Text(address)))) => {
                    self.address_place(address, place)
                }
                _ => place,
            };
            let before = frame::with(|frame| frame.calling.replace(calling)).flatten();
            let built = super::pieces::build(self.shared, self.page, piece, values);
            frame::with(|frame| frame.calling = before);
            match built? {
                Ok(html) => {
                    if piece == Piece::Image {
                        self.pieced.borrow_mut().push(calling);
                    }
                    Some(html)
                }
                Err(error) => {
                    self.failed.borrow_mut().insert(piece);
                    let problems = frame::with(|frame| explain::explain(&error, frame, &[place]))
                        .unwrap_or_default();
                    frame::with(|frame| frame.problems.extend(problems));
                    None
                }
            }
        })
    }

    fn picture(&self, address: &str, place: Place, tagged: bool) -> Option<markdown::Picture> {
        let shared = self.shared;
        let page = shared.site.pages.get(self.page);
        // A page made from data is written in no folder of `content/`.
        let folder = page
            .made
            .is_none()
            .then(|| page.path.rfind('/').map_or("", |at| &page.path[..=at]));
        let published = shared.address_of(self.page);
        let from = crate::album::From {
            folder,
            page: &published,
            base: &shared.base,
        };
        let pictures = &shared.site.pictures;
        let Some(found) = pictures.find(address, &from) else {
            frame::read(Key::Pictures);
            return None;
        };
        // How its tag reaches it depends on where the page is published.
        frame::read(Key::Field(self.page, shared.name_id("url")));
        frame::read(Key::Picture(found.id));
        let picture = pictures.get(found.id);
        let at = self.address_place(address, place);
        let left_alone =
            picture.left_alone() || shared.settings.truth("optimise_images") == Some(false);
        frame::read(Key::Setting(shared.name_id("optimise_images")));
        let told = match &picture.seen {
            crate::album::Seen::Unreadable(fault) if !left_alone => Some(Kind::UnreadablePicture {
                path: picture.shown(),
                fault: fault.clone(),
            }),
            crate::album::Seen::Plain {
                kind: crate::pictures::Kind::Heic | crate::pictures::Kind::Heif,
                ..
            } => Some(Kind::HeicPicture {
                path: picture.shown(),
            }),
            _ => None,
        };
        if let Some(kind) = told
            && !self.unreadable_said.borrow().contains(&at)
        {
            self.unreadable_said.borrow_mut().push(at);
            let problem = Problem::at(kind, at);
            frame::with(|frame| {
                if !frame.problems.contains(&problem) {
                    frame.problems.push(problem);
                }
            });
        }
        if tagged && found.address != address {
            self.moved.borrow_mut().push((at, found.address.clone()));
        }
        let size = picture.size().map(|size| (size.width, size.height));
        let alt = picture.sidecar.as_ref().and_then(|said| said.alt.as_ref());
        let mut shown = markdown::Picture {
            address: found.address,
            size,
            copies: None,
            alt: alt.map(|alt| alt.value.clone()).unwrap_or_default(),
            decorative: picture.decorative(),
            file: Some(picture.shown()),
            described_beside: picture.root == crate::album::Root::Content,
        };
        if !tagged || picture.copies.is_empty() {
            return Some(shown);
        }
        frame::read(Key::Setting(shared.name_id("max_width")));
        frame::read(Key::Setting(shared.name_id("image_sizes")));
        let end = shown
            .address
            .find(['?', '#'])
            .unwrap_or(shown.address.len());
        let beside = &shown.address[..shown.address[..end].rfind('/').map_or(0, |at| at + 1)];
        let srcset = |format: crate::pictures::Format| {
            let mut srcset = String::new();
            for copy in picture.copies.iter().filter(|copy| copy.format == format) {
                if !srcset.is_empty() {
                    srcset.push_str(", ");
                }
                let mut address = String::new();
                markdown::escape_href(&format!("{beside}{}", copy.name), &mut address);
                // A comma parts the addresses of a `srcset`.
                srcset.push_str(&address.replace(',', "%2C"));
                srcset.push_str(&format!(" {}w", copy.width));
            }
            srcset
        };
        let widths: Vec<u32> = picture.copies.iter().map(|copy| copy.width).collect();
        let fallback = picture.copies.last()?.format;
        let src_width = crate::pictures::src_width(&widths)?;
        let mut src = String::new();
        let named = picture
            .copies
            .iter()
            .find(|copy| copy.format == fallback && copy.width == src_width)?;
        markdown::escape_href(&format!("{beside}{}", named.name), &mut src);
        shown.copies = Some(markdown::Copies {
            avif: srcset(crate::pictures::Format::Avif),
            fallback: srcset(fallback),
            src,
            sizes: shared
                .settings
                .text("image_sizes")
                .map(|sizes| sizes.value.to_owned())
                .unwrap_or_default(),
        });
        frame::develops(found.id, Some(at), false);
        Some(shown)
    }

    fn undescribed(&self, picture: &str, found: Option<&markdown::Picture>, place: Place) {
        let at = self.address_place(picture, place);
        let kind = Kind::UndescribedPicture {
            picture: picture.to_owned(),
            shown: ShownBy::Markdown {
                described_beside: found.is_some_and(|found| found.described_beside),
            },
        };
        let file = found.and_then(|found| found.file.clone());
        let key = file.unwrap_or_else(|| picture.to_owned());
        super::media::undescribed(key, |_| Problem::at(kind, at));
    }

    fn html(
        &self,
        call: &markdown::Call,
        place: Place,
        placement: Placement,
        body: Option<&str>,
    ) -> String {
        self.counted(place, || {
            let shared = self.shared;
            let name = &call.name.value;
            let Some(snippet) = shared.templates.snippets.get(name) else {
                // In HTML, only a name the site has is a call; anything
                // else is the author's HTML, written back as it is.
                if call.within != Within::Prose {
                    return shared.sources.spelling(place).to_owned();
                }
                let nearest = crate::suggest::nearest(
                    name,
                    shared.templates.snippets.keys().map(String::as_str),
                )
                .map(str::to_owned);
                let at = call.name.origin.place.unwrap_or(place);
                frame::with(|frame| {
                    frame.problems.push(Problem::at(
                        Kind::UnknownWord {
                            word: Word::Snippet,
                            written: name.clone(),
                            nearest,
                        },
                        at,
                    ))
                });
                return String::new();
            };
            if call.within != Within::Prose && !self.in_html_said.replace(true) {
                frame::with(|frame| {
                    frame
                        .problems
                        .push(Problem::at(Kind::CallInHtml { name: name.clone() }, place))
                });
            }
            if !snippet.readable {
                frame::read(Key::Template(Templates::id(snippet.file)));
                return String::new();
            }
            if !snippet.wrapped.is_empty() && !call.wraps() {
                frame::with(|frame| {
                    frame.problems.push(Problem::at(
                        Kind::MalformedCall {
                            found: crate::problem::CallFault::WrapsNothing(name.clone()),
                        },
                        place,
                    ))
                });
                return String::new();
            }
            let wrong = |fault, at: Place| {
                frame::with(|frame| {
                    frame.problems.push(Problem::at(
                        Kind::WrongArgument {
                            snippet: name.clone(),
                            fault,
                        },
                        at,
                    ))
                });
                String::new()
            };
            let mut handed: Vec<(String, Value)> = Vec::new();
            let mut unnamed = Vec::new();
            let words = typography_of(shared, self.page);
            let typography = words.map(markdown::Typography::of);
            for argument in &call.arguments {
                let value = prose_value(&argument.value.value, typography.as_ref());
                // Text handed to the snippet remembers where it is written.
                let value = match (argument.value.origin.place, value.as_str()) {
                    (Some(written), Some(text)) => frame::sourced(text, written),
                    _ => value,
                };
                let at = argument.value.origin.place.unwrap_or(place);
                match &argument.name {
                    Some(given) => {
                        let given_at = given.origin.place.unwrap_or(at);
                        let given_name = given.value.as_str();
                        if super::files::SUPPLIED.contains(&given_name)
                            || super::files::WRAPPED.contains(&given_name)
                        {
                            return wrong(ArgumentFault::Supplied(given.value.clone()), given_at);
                        }
                        if !snippet.params.contains(&given.value) {
                            let nearest = crate::suggest::nearest(
                                &given.value,
                                snippet.params.iter().map(String::as_str),
                            )
                            .map(str::to_owned);
                            return wrong(
                                ArgumentFault::NoSuchName {
                                    given: given.value.clone(),
                                    nearest,
                                },
                                given_at,
                            );
                        }
                        handed.push((given.value.clone(), value));
                    }
                    None => unnamed.push((value, at)),
                }
            }
            let first = unnamed.first().map_or(place, |(_, at)| *at);
            match unnamed_fills(
                snippet.takes.as_ref(),
                &snippet.params,
                unnamed.len(),
                |p| handed.iter().any(|(h, _)| h == p),
            ) {
                Unnamed::Nothing => {}
                Unnamed::Fills(names) => {
                    for (name, (value, _)) in names.into_iter().zip(unnamed) {
                        handed.push((name, value));
                    }
                }
                Unnamed::Gathers(name) => {
                    let values: Vec<Value> = unnamed.into_iter().map(|(v, _)| v).collect();
                    handed.push((name, Value::from(values)));
                }
                Unnamed::NeedsNames => {
                    return wrong(
                        ArgumentFault::NeedsNames {
                            wanted: snippet.params.clone(),
                        },
                        first,
                    );
                }
                Unnamed::TooMany { takes, past } => {
                    let at = unnamed.get(past).map_or(first, |(_, at)| *at);
                    let others = snippet
                        .params
                        .iter()
                        .filter(|p| !takes.contains(p))
                        .cloned()
                        .collect();
                    return wrong(ArgumentFault::TooMany { takes, others }, at);
                }
                Unnamed::Twice { name: twice, at } => {
                    let at = unnamed.get(at).map_or(first, |(_, at)| *at);
                    return wrong(ArgumentFault::Twice(twice), at);
                }
            }
            if let Some(body) = body
                && snippet.wrapped.iter().any(|name| name == "body")
            {
                let mut wrapped = self.wrapped.borrow_mut();
                let traced = (wrapped.iter().position(|wrapped| wrapped.place == place))
                    .map(|at| wrapped.swap_remove(at));
                drop(wrapped);
                let value = match traced {
                    Some(Wrapped { traced, called, .. }) => {
                        let calls = self.calls.borrow();
                        let own = Own::of_writing(&traced, &called, &calls, body, &shared.sources);
                        drop(calls);
                        frame::made(body.to_owned(), own, true)
                    }
                    None => Value::from_safe_string(body.to_owned()),
                };
                handed.push(("body".into(), value));
            }
            handed.push((
                "standalone".into(),
                Value::from(placement == Placement::Standalone),
            ));
            let page = PageValue::value(shared, self.page);
            let site = SiteValue::value(shared, frame::language());
            let before = frame::with(|frame| frame.calling.replace(place)).flatten();
            let standalone = placement == Placement::Standalone;
            let built = build_snippet(shared, snippet, handed, page, site, None, standalone).map(
                |(html, own)| {
                    self.calls.borrow_mut().push((place, Arc::new(own)));
                    html
                },
            );
            frame::with(|frame| frame.calling = before);
            match built {
                Ok(html) => html,
                Err(error) => {
                    let problems = frame::with(|frame| explain::explain(&error, frame, &[place]))
                        .unwrap_or_default();
                    frame::with(|frame| frame.problems.extend(problems));
                    String::new()
                }
            }
        })
    }
}

/// A value a call in the writing hands its snippet. The author's own words
/// are set as the writing around the call is, so a caption's dash is the
/// prose's; a value naming something reaches the snippet as typed.
fn prose_value(value: &markdown::Value, typography: Option<&markdown::Typography<'_>>) -> Value {
    match value {
        markdown::Value::Text(text) => match typography {
            Some(typography) if !names_something(text) => {
                Value::from(markdown::set_words(text, typography))
            }
            _ => Value::from(text.as_str()),
        },
        markdown::Value::Number(n) if n.fract() == 0.0 && n.abs() < 1e15 => Value::from(*n as i64),
        markdown::Value::Number(n) => Value::from(*n),
        markdown::Value::Bool(b) => Value::from(*b),
    }
}

/// Whether a value handed to a call names something rather than saying it:
/// a file, an address, the id of a video. A curled apostrophe or a dash in
/// a name is a file nothing finds. A snippet never says which of its
/// parameters are names, so this is asked of the value's shape: empty, an
/// address, a path, a last word with a file's extension, or a single word
/// with no apostrophe or ellipsis in it.
fn names_something(value: &str) -> bool {
    let value = value.trim();
    let one_word_of_writing =
        |word: &str| !word.contains('/') && (word.contains('\'') || word.contains("..."));
    value.is_empty()
        || value.contains("://")
        || value.starts_with('/')
        || ends_in_an_extension(value)
        || (!value.contains(char::is_whitespace) && !one_word_of_writing(value))
}

/// A last word with a short extension on it: `photo 1.jpg` names a file,
/// while a sentence ending in a full stop has nothing after the dot.
fn ends_in_an_extension(value: &str) -> bool {
    let last = value.rsplit(char::is_whitespace).next().unwrap_or(value);
    last.rsplit_once('.').is_some_and(|(before, extension)| {
        !before.is_empty()
            && (1..=5).contains(&extension.chars().count())
            && extension.chars().all(|c| c.is_ascii_alphanumeric())
    })
}

/// Builds a snippet with what a call handed it, and `page` and `site`,
/// called by the run `calling` where a template calls it. `begins` says
/// whether what it writes starts a line where it is put.
fn build_snippet(
    shared: &Arc<Shared>,
    snippet: &Snippet,
    handed: Vec<(String, Value)>,
    page: Value,
    site: Value,
    calling: Option<&State>,
    begins: bool,
) -> Result<(String, Own), Error> {
    let templates = &shared.templates;
    frame::read(Key::Template(Templates::id(snippet.file)));
    if templates.unusable.contains(&snippet.file) {
        return Err(frame::told());
    }
    let calls_itself = frame::with(|frame| frame.snippets.contains(&snippet.name)).unwrap_or(false);
    if calls_itself {
        return Err(frame::raise(Kind::TemplateStopped {
            reason: Stopped::CallsItself(snippet.name.clone()),
        }));
    }
    let mut context: BTreeMap<String, Value> = handed.iter().cloned().collect();
    context.insert("page".into(), page);
    context.insert("site".into(), site);
    let name = &templates.files[snippet.file].name;
    let mut written = Own::default();
    frame::begin(snippet.file, calling)?;
    frame::with(|frame| frame.snippets.push(snippet.name.clone()));
    let built = templates
        .env
        .get_template(name)
        .and_then(|template| {
            super::indent::render_placed(&template, context, templates.placing(name), begins)
        })
        .map(|(captured, text, own)| {
            written = own;
            (captured, text)
        });
    frame::with(|frame| frame.snippets.pop());
    let built = frame::end(snippet.file, built)?;
    built.map(|html| (html, written)).map_err(|inner| {
        let kind = inner.kind();
        let handed = Handed {
            snippet: snippet.name.clone(),
            values: handed,
        };
        Stop::InSnippet(handed, inner).into_error(kind)
    })
}

/// Registers a snippet as a function templates call by its name. A
/// missing value handed to it is refused at the call, and a failure inside
/// it carries the call's place.
pub(super) fn register_snippet(env: &mut Environment<'static>, snippet: &Snippet) {
    let snippet = snippet.clone();
    let name = snippet.name.clone();
    env.add_function(name, move |state: &State, args: Rest<ValueOrKwargs>| {
        let args = args.into_values();
        super::guard::check_args(
            &snippet.name,
            super::guard::Callee::Snippet,
            &args,
            super::guard::Input::Whole,
        )?;
        let Some(shared) = frame::shared() else {
            return Err(frame::told());
        };
        if !snippet.readable {
            frame::read(Key::Template(Templates::id(snippet.file)));
            return Err(frame::told());
        }
        let raise = |fault| {
            frame::raise(Kind::WrongArgument {
                snippet: snippet.name.clone(),
                fault,
            })
        };
        let mut handed: Vec<(String, Value)> = Vec::new();
        let mut unnamed = Vec::new();
        for arg in args.iter() {
            if !arg.is_kwargs() {
                unnamed.push(arg.clone());
                continue;
            }
            for key in arg.try_iter()? {
                let given = key.as_str().unwrap_or_default().to_owned();
                if super::files::SUPPLIED.contains(&given.as_str()) {
                    return Err(raise(ArgumentFault::Supplied(given)));
                }
                if !snippet.params.contains(&given) && !snippet.wrapped.contains(&given) {
                    let nearest =
                        crate::suggest::nearest(&given, snippet.params.iter().map(String::as_str))
                            .map(str::to_owned);
                    return Err(raise(ArgumentFault::NoSuchName { given, nearest }));
                }
                let value = arg.get_item(&key)?;
                handed.push((given, value));
            }
        }
        match unnamed_fills(
            snippet.takes.as_ref(),
            &snippet.params,
            unnamed.len(),
            |p| handed.iter().any(|(h, _)| h == p),
        ) {
            Unnamed::Nothing => {}
            Unnamed::Fills(names) => handed.extend(names.into_iter().zip(unnamed)),
            Unnamed::Gathers(name) => handed.push((name, Value::from(unnamed))),
            Unnamed::NeedsNames => {
                return Err(raise(ArgumentFault::NeedsNames {
                    wanted: snippet.params.clone(),
                }));
            }
            Unnamed::TooMany { takes, .. } => {
                let others = snippet
                    .params
                    .iter()
                    .filter(|p| !takes.contains(p))
                    .cloned()
                    .collect();
                return Err(raise(ArgumentFault::TooMany { takes, others }));
            }
            Unnamed::Twice { name, .. } => return Err(raise(ArgumentFault::Twice(name))),
        }
        let page = state.lookup("page").unwrap_or_default();
        let site = state.lookup("site").unwrap_or_default();
        let begins = super::indent::at_line_start();
        build_snippet(&shared, &snippet, handed, page, site, Some(state), begins)
            .map(|(html, own)| frame::made(html, own, true))
    });
}

/// How a problem met building the frame's page names it: a page of the
/// author's by its first line, or the item it was made from; a list's page
/// Statac made, one after its first, or a feed, by name, with the setting
/// placing it or the file of its list's page where there is one.
pub(super) fn building(frame: &Frame) -> Option<(Relation, Option<Place>)> {
    let shared = &frame.shared;
    if let Some(feed) = &frame.feed {
        let (word, name) = crate::feeds::described(&shared.site, &shared.sources, feed.list);
        let named = Named::Feed(Box::new(NamedFeed {
            word,
            name,
            address: format!("{}{}", shared.base, feed.address.as_str()),
        }));
        return Some((Relation::Building(Some(named)), feed.placed));
    }
    let page = frame.page?;
    let head = shared.site.head(page, &shared.sources);
    let named = match frame.number {
        1 if shared.site.pages.get(page).source.is_some() => None,
        1 => Some(shared.site.named(page, &shared.sources)),
        number => Some(Named::later(number, shared.address_of(page))),
    };
    Some((Relation::Building(named), head))
}

/// Marks a problem as met building the frame's page, where a page is
/// being built and the problem isn't in its own file.
pub(super) fn met_building(problem: Problem, frame: &Frame) -> Problem {
    let Some((relation, head)) = building(frame) else {
        return problem;
    };
    if relation == Relation::Building(None)
        && let (Some(at), Some(head)) = (problem.place, head)
        && at.source == head.source
    {
        return problem;
    }
    problem.met(relation, head)
}
