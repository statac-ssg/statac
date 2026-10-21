//! Reading a site and building it.

mod scripts;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rayon::prelude::*;

use crate::album::Progress;
use crate::assets::{self, Assets};
use crate::content;
use crate::date::Date;
use crate::feeds;
use crate::links;
use crate::new::update::Starter;
use crate::problem::{Carried, Published};
use crate::problem::{Code, Kind, Named, Problem, Reason, Severity};
use crate::publish::{self, Staging};
use crate::reads::{Asked, Key, Reads, Record};
use crate::report::Quiet;
use crate::site::{Id, ListId, OutputId, PageId, PictureId};
use crate::source::{Place, SourceId, Sources};
use crate::templates::{self, Renderer};
use scripts::{Finding, Found};
use std::sync::Mutex;

/// Where the built site goes in the site's folder, unless the author says.
pub const OUT: &str = "_site";

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Mode {
    /// Read the site and write it out.
    Build,
    /// Read the site and write nothing.
    Check,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Options {
    pub mode: Mode,
    /// The site's folder.
    pub site: PathBuf,
    /// Where the built site goes, where the author said.
    pub out: Option<PathBuf>,
    /// The build's date, `site.today`; today in the site's zone where none
    /// is given.
    pub today: Option<Date>,
    /// Whether drafts and posts waiting for their day are built as if
    /// published.
    pub drafts: bool,
    /// The site's address for this run, in place of `url`, where one was
    /// given.
    pub url: Option<String>,
}

impl Options {
    /// Where the built site goes: where the author said, or `_site` in the
    /// site's folder.
    pub fn out(&self) -> PathBuf {
        match &self.out {
            Some(out) => out.clone(),
            None if self.site == Path::new(".") => PathBuf::from(OUT),
            None => self.site.join(OUT),
        }
    }

    /// What reading the site is asked to do.
    pub fn run(&self) -> content::Run {
        content::Run {
            today: self.today,
            drafts: self.drafts,
            url: self.url.clone(),
        }
    }
}

/// What a build read and found, and what it published.
pub struct Outcome {
    held: Held,
    pub problems: Vec<Problem>,
    /// The kinds the site's `quiet:` setting quietens.
    pub quiet: Quiet,
    /// What was written, where the whole build succeeded and wrote the site.
    pub published: Option<Published>,
    /// The JavaScript the built pages carry, and where each came from.
    pub javascript: Vec<Javascript>,
    /// What each page read while it was built, by its output.
    pub record: Record,
    /// The drafts and posts waiting for their day built as if published.
    pub included: content::Included,
    /// What became of the copies of photographs, where the site was written.
    pub copies: Copies,
    /// What became of the sharing cards, where the site was written.
    pub cards: Cards,
    /// What became of the site's own commands.
    pub hooks: crate::hooks::Ran,
    /// How old what was fetched is, where the site was written.
    pub fetched: crate::fetched::Ages,
    /// The photographs beside the writing that a page shows and none
    /// links, so only their copies are published and not the files
    /// themselves: each by where its own file would be published.
    pub held_back: Vec<String>,
    /// How long the build waited for another build of the site to end.
    pub waited: Duration,
}

/// What a build did with the sharing cards.
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct Cards {
    /// The cards drawn, the rest being kept from an earlier build.
    pub drawn: usize,
    /// The cards kept from earlier builds and removed, no page being
    /// shared with them any more.
    pub removed: usize,
}

/// What a build did with the copies of photographs.
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct Copies {
    /// The copies made, the rest being kept from an earlier build.
    pub made: usize,
    /// The photographs those were made from.
    pub from: usize,
    /// The copies kept from earlier builds and removed, no page showing
    /// them any more.
    pub removed: usize,
}

/// What holds the files read, so any problem can be shown where it was
/// written.
enum Held {
    Sources(Sources),
    Renderer(Renderer),
}

impl Outcome {
    pub fn sources(&self) -> &Sources {
        match &self.held {
            Held::Sources(sources) => sources,
            Held::Renderer(renderer) => renderer.sources(),
        }
    }

    /// Whether any problem stops the build.
    pub fn stopped(&self) -> bool {
        stops(&self.problems)
    }

    /// The site as it was read, where it was.
    pub fn site(&self) -> Option<&crate::site::Site> {
        match &self.held {
            Held::Sources(_) => None,
            Held::Renderer(renderer) => Some(renderer.site()),
        }
    }

    /// The site's settings, where the site was read.
    pub fn settings(&self) -> Option<&crate::config::Settings> {
        match &self.held {
            Held::Sources(_) => None,
            Held::Renderer(renderer) => Some(renderer.settings()),
        }
    }

    /// The build's date, `site.today`, where the site was read.
    pub fn today(&self) -> Option<Date> {
        match &self.held {
            Held::Sources(_) => None,
            Held::Renderer(renderer) => Some(renderer.today()),
        }
    }

    pub(crate) fn unread(problem: Problem) -> Outcome {
        Outcome::stopped_at(Sources::default(), problem)
    }

    /// A build stopped by `problem` before the site was read, `sources`
    /// holding the file it is placed in.
    pub(crate) fn stopped_at(sources: Sources, problem: Problem) -> Outcome {
        Outcome {
            held: Held::Sources(sources),
            problems: vec![problem],
            quiet: Quiet::default(),
            published: None,
            javascript: Vec::new(),
            record: Record::default(),
            included: content::Included::default(),
            copies: Copies::default(),
            cards: Cards::default(),
            hooks: crate::hooks::Ran::default(),
            fetched: crate::fetched::Ages::default(),
            held_back: Vec::new(),
            waited: Duration::ZERO,
        }
    }
}

/// One kind of JavaScript the built pages carry, and on how many pages.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Javascript {
    pub carried: Carried,
    /// Whether a browser runs it; where not, it is somewhere a browser may
    /// or may not run it, such as a comment, and counted all the same.
    pub sure: bool,
    pub pages: usize,
}

/// Reads the site at `site`: its settings, its content and its assets,
/// with everything published at one address found once, whichever of the
/// three it comes from, and the feeds it publishes.
pub fn read(site: &Path, run: &content::Run) -> (content::Read, Assets, Vec<feeds::Feed>) {
    read_with(site, run, &mut content::Texts::default())
}

/// [`read`], taking the texts of the site's pages from `texts` as
/// [`content::read_with`] does.
pub(crate) fn read_with(
    site: &Path,
    run: &content::Run,
    texts: &mut content::Texts,
) -> (content::Read, Assets, Vec<feeds::Feed>) {
    let mut read = content::read_with(site, run, texts);
    let base = read.settings.base();
    let (assets, problems) = assets::gather(
        site,
        &base,
        read.settings.text("code_style").map(|style| style.value),
        &mut read.sources,
    );
    read.problems.extend(problems);
    let templates = template_names(site);
    let has = |name: &str| templates.contains(name);
    let (feeds, problems) = feeds::plan(&read.site, &read.settings, &read.sources, &has);
    read.problems.extend(problems);
    read.problems
        .extend(feeds::unusable(&read.site, &read.settings, &read.sources));
    read.problems
        .extend(feeds::undated(&read.site, &read.settings, &read.sources));
    read.problems.extend(robots_beneath_path(&read));
    let (fetched, found) = crate::fetched::read(site, &mut read.sources);
    read.site.fetched = fetched;
    read.problems.extend(found);
    let found = crate::album::read(site, &mut read, assets.statics());
    read.problems.extend(found);
    let (mentions, found) =
        crate::fetched::mentions::read(&read.site, &read.settings, &mut read.sources);
    read.site.mentions = mentions;
    read.problems.extend(found);
    let mut outputs = assets.claims();
    outputs.extend(read.claims());
    outputs.extend(read.site.pictures.claims());
    outputs.extend(feeds::claims(&feeds));
    read.problems.extend(assets::same_address(&outputs, &base));
    read.problems
        .extend(crate::hooks::claimed(&read.settings, &outputs, &base));
    read.problems
        .extend(content::sent_nowhere(&read.site, &outputs));
    read.problems
        .extend(content::taken(&read.site, &outputs, &base));
    (read, assets, feeds)
}

/// The names of the templates at the top of the site's `templates/`, as
/// the template reader names them: `feed.xml`, and `home` for `home.html`.
fn template_names(site: &Path) -> HashSet<String> {
    let Ok(entries) = fs::read_dir(site.join("templates")) else {
        return HashSet::new();
    };
    let mut names = HashSet::new();
    for entry in entries.flatten() {
        if !entry.path().is_file() {
            continue;
        }
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        match name.strip_suffix(".html") {
            Some(bare) => names.insert(bare.to_owned()),
            None => names.insert(name),
        };
    }
    names
}

/// The note for a `robots.txt` page published beneath the site's path,
/// where crawlers never look for one.
fn robots_beneath_path(read: &content::Read) -> Option<Problem> {
    let base = read.settings.base();
    let url = read.settings.text("url")?;
    if base.is_empty() {
        return None;
    }
    let (id, page) = read
        .site
        .pages
        .iter()
        .find(|(_, page)| page.lands() && page.address.as_str() == "/robots.txt")?;
    let origin = site_origin(url.value);
    let place = page
        .words
        .get(crate::content::words::PageWord::Permalink)
        .and_then(|permalink| permalink.origin.place)
        .or_else(|| read.site.head(id, &read.sources));
    let kind = Kind::RobotsBeneathPath {
        top: format!("{origin}/robots.txt"),
    };
    Some(Problem::placed(kind, place))
}

/// Reads the site at `site` and its templates, ready to build its pages,
/// with every problem reading them found. `today` fixes `site.today`,
/// so a build can be repeated.
pub fn renderer(site: &Path, today: Option<Date>) -> (Renderer, Vec<Problem>) {
    let run = content::Run {
        today,
        drafts: false,
        url: None,
    };
    let prepared = prepared(site, &run);
    (prepared.renderer, prepared.problems)
}

/// A site read and ready to build.
pub(crate) struct Prepared {
    pub renderer: Renderer,
    /// Every problem reading it.
    pub problems: Vec<Problem>,
    /// The kinds its `quiet:` setting quietens.
    pub quiet: Quiet,
    /// What `--drafts` brought in.
    pub included: content::Included,
    /// The pictures that pages left out of the build name.
    pub left_out: Vec<PictureId>,
    /// The posts feed readers know, as the last build to publish kept
    /// them.
    pub known: feeds::Known,
}

pub(crate) fn prepared(site: &Path, run: &content::Run) -> Prepared {
    prepared_with(site, run, &mut content::Texts::default())
}

/// [`prepared`], taking the texts of the site's pages from `texts` as
/// [`content::read_with`] does.
pub(crate) fn prepared_with(
    site: &Path,
    run: &content::Run,
    texts: &mut content::Texts,
) -> Prepared {
    let (mut read, assets, feeds) = read_with(site, run, texts);
    let mut left_out: Vec<PictureId> = read
        .drafts
        .iter()
        .chain(&read.waiting)
        .flat_map(|&source| read.site.pictures.named_in(read.sources.text(source)))
        .collect();
    left_out.sort_unstable();
    left_out.dedup();
    let (quiet, unquietened) = Quiet::new(&read.settings.quiet());
    let (mut cards, found) = crate::cards::read(site, &mut read.sources);
    // A card only pages left out pick is still picked.
    cards.picked_elsewhere = read
        .drafts
        .iter()
        .chain(&read.waiting)
        .flat_map(|&source| crate::cards::picked_in(read.sources.text(source)))
        .collect();
    let (templates, told) =
        templates::read(site, &mut read.sources, &read.site, &read.settings, cards);
    let (known, moved) = feeds::Known::read(
        site,
        &read.site,
        &read.settings,
        &read.dates,
        &read.sources,
        &feeds,
    );
    let mut problems = read.problems;
    problems.extend(moved);
    problems.extend(told);
    problems.extend(found);
    problems.extend(unquietened);
    let renderer = Renderer::new(
        read.sources,
        read.settings,
        read.site,
        assets,
        templates,
        read.dates,
        feeds,
    );
    Prepared {
        renderer,
        problems,
        quiet,
        included: read.included,
        left_out,
        known,
    }
}

/// Something the build writes from a template.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) enum Output {
    Page(PageId),
    /// One of a list's pages after its first, by its number, counted from
    /// 2, built from the page that speaks for the list.
    Later(ListId, usize),
    /// The page left at one of the site's old addresses, by its place
    /// among them.
    Redirect(usize),
    /// A list's feed, by its place among the site's feeds.
    Feed(usize),
}

/// One output built, and everything building it adds to the build.
pub(crate) struct Built {
    pub output: Output,
    /// The file it is published as, from the top of the built site.
    pub file: String,
    /// Its text, where it was built whole and is held to be written later.
    pub html: Option<String>,
    pub reads: Reads,
    /// What it asked of its own page and of the site, by name.
    pub asked: Asked,
    pub problems: Vec<Problem>,
    pub scripts: Vec<Finding>,
    /// Each piece of JavaScript written into it where a browser runs it.
    pub pieces: Vec<scripts::Print>,
    /// What its links came to, for a page written as HTML.
    pub links: Option<links::Checked>,
    /// The photographs whose copies it names, as each was named.
    pub developed: Vec<crate::album::Shown>,
    /// The sharing cards it asked for.
    pub cards: Vec<templates::Asked>,
    /// Whether it is a page showing a picture with its width and height.
    pub sized: bool,
    /// Where the first block of code coloured names its language, in each
    /// page's writing it shows, whole or as its excerpt.
    pub coloured: Vec<Place>,
    /// How many pages of lists of the whole site it reached.
    pub walked: u64,
    /// Whether the built site holds it.
    pub written: bool,
    /// The languages whose search index it names.
    pub indexed: Vec<crate::site::LanguageId>,
}

/// Where an output goes once it is built.
#[derive(Copy, Clone)]
pub(crate) enum Writes<'a> {
    Nowhere,
    Into(&'a Staging),
    /// Held with the output, to be written once the whole site is known
    /// to build.
    Later,
}

/// What a site read is about to build, and what its links are held to.
pub(crate) struct Plan {
    pub outputs: Vec<Output>,
    pub linked: links::Site,
    /// The pictures beside the writing published only where a built page
    /// links them, each by its place among the files beside the writing.
    pub held: HashMap<usize, PictureId>,
}

impl Output {
    /// The file the output is published as, from the top of the built
    /// site.
    pub(crate) fn file(self, renderer: &Renderer) -> String {
        let site = renderer.site();
        match self {
            Output::Page(page) => site.pages.get(page).address.file(),
            Output::Later(list, number) => site
                .lists
                .get(list)
                .later_address(number)
                .expect("a later page is built where it stands")
                .file(),
            Output::Redirect(at) => site.redirects[at].from.file(),
            Output::Feed(at) => renderer.feeds()[at].address.file(),
        }
    }
}

impl Plan {
    /// The same plan, each page checked noting what its links ask the
    /// site for.
    pub(crate) fn noting(self) -> Plan {
        Plan {
            linked: self.linked.noting(),
            ..self
        }
    }

    /// Every output of the site `renderer` holds, adding to `problems`
    /// what leaves any out.
    pub(crate) fn of(renderer: &Renderer, problems: &mut Vec<Problem>) -> Plan {
        let site = renderer.site();
        let mut outputs: Vec<Output> = site
            .pages
            .iter()
            .filter(|(_, page)| page.lands())
            .map(|(id, _)| Output::Page(id))
            .collect();
        for (id, list) in site.lists.iter() {
            outputs.extend((2..list.later.len() + 2).map(|number| Output::Later(id, number)));
        }
        match renderer.without_redirect_template() {
            Some(problem) => problems.push(problem),
            None => outputs.extend((0..site.redirects.len()).map(Output::Redirect)),
        }
        let feeds = renderer.feeds();
        outputs.extend((0..feeds.len()).map(Output::Feed));
        let mut published: Vec<String> = renderer
            .assets()
            .claims()
            .into_iter()
            .map(|(file, _)| file)
            .collect();
        // Sass has said why a stylesheet is missing, and the build stops for
        // it, so a link to it isn't broken as well.
        published.extend(renderer.assets().unwritten().map(str::to_owned));
        published.extend(
            content::claims(site, renderer.sources(), renderer.settings())
                .into_iter()
                .map(|(file, _)| file),
        );
        published.extend(feeds::claims(feeds).into_iter().map(|(file, _)| file));
        let pictures = &site.pictures;
        published.extend(pictures.claims().into_iter().map(|(file, _)| file));
        let held: HashMap<usize, PictureId> = pictures.held().map(|(id, at)| (at, id)).collect();
        let lists = site
            .pages
            .iter()
            .filter(|(_, page)| page.source.is_none() || page.speaks_for.is_some());
        let linked = links::Site::new(renderer.settings(), published)
            .listing(lists.map(|(_, page)| page.address.file()))
            .watching(held.keys().map(|&at| site.files[at].address.file()));
        Plan {
            outputs,
            linked,
            held,
        }
    }
}

/// Reads the site, builds every page, and for `Mode::Build` publishes the
/// whole site, or nothing where any problem stops it.
pub fn run(options: &Options) -> Outcome {
    run_saying(options, Progress::Silent)
}

/// [`run`], saying as `progress` has it how far a long build has got.
/// A build runs the site's own commands around it, and `check` says once
/// that it runs neither.
pub fn run_saying(options: &Options, progress: Progress) -> Outcome {
    run_holding(options, progress, &Starter::OWN)
}

/// [`run_saying`], with the site's starter files held to `starter`.
pub(crate) fn run_holding(options: &Options, progress: Progress, starter: &Starter) -> Outcome {
    if let Err(problem) = a_folder(&options.site) {
        return Outcome::unread(problem);
    }
    if options.mode == Mode::Build
        && let Some(out) = &options.out
        && let Some(problem) = in_the_way(&options.site, out)
    {
        return Outcome::unread(problem);
    }
    // A check writes nothing, so it has nothing to keep apart.
    let (hold, unheld) = match options.mode {
        Mode::Build => {
            let (hold, unheld) =
                publish::hold(&options.site, || progress.say(crate::wording::WAITING));
            (Some(hold), unheld)
        }
        Mode::Check => (None, None),
    };
    let waited = hold.as_ref().map_or(Duration::ZERO, |hold| hold.waited);
    let before = match options.mode {
        Mode::Build => match crate::hooks::before(options, progress, false) {
            Ok(before) => before,
            Err(stopped) => {
                let (sources, problem) = *stopped;
                let mut outcome = Outcome::stopped_at(sources, problem);
                outcome.problems.extend(unheld);
                outcome.waited = waited;
                return outcome;
            }
        },
        Mode::Check => crate::hooks::Before::Unset,
    };
    let mut outcome = built(options, progress, starter, unheld);
    outcome.hooks.before = before;
    outcome.waited = waited;
    if let Held::Renderer(renderer) = &outcome.held {
        let named = crate::hooks::names_output(renderer.settings(), renderer.sources(), options);
        outcome.problems.extend(named);
        if options.mode == Mode::Check {
            let unrun = crate::hooks::unrun_by_check(renderer.settings());
            outcome.problems.extend(unrun);
            // What is held back is only known of a site that could be built.
            if !stops(&outcome.problems) {
                let located = locations_published(renderer.site(), &outcome.held_back);
                outcome.problems.extend(located);
            }
        }
    }
    outcome
}

/// The note `check` gives of each picture beside the writing that holds
/// where it was taken and is published as the file it is, in the order of
/// their paths. `held_back` are the files no page links, which aren't
/// published.
fn locations_published(site: &crate::site::Site, held_back: &[String]) -> Vec<Problem> {
    let mut located: Vec<String> = (site.pictures.iter())
        .filter(|(_, picture)| picture.located)
        .filter_map(|(_, picture)| Some(&site.files[picture.travelling?]))
        .filter(|file| !held_back.contains(&file.address.file()))
        .map(|file| format!("content/{}", file.path))
        .collect();
    located.sort_unstable();
    let located = located
        .into_iter()
        .map(|path| Kind::LocationPublished { path });
    located.map(Problem::nowhere).collect()
}

/// Reads the site and builds it, as [`run_saying`] does between the
/// site's own commands; `unheld` says why the build doesn't hold the site.
fn built(
    options: &Options,
    progress: Progress,
    starter: &Starter,
    unheld: Option<Problem>,
) -> Outcome {
    let mut prepared = prepared(&options.site, &options.run());
    let note = crate::new::update::newer_starter(&options.site, starter);
    prepared.problems.extend(note);
    let staging = match options.mode {
        Mode::Build if !stops(&prepared.problems) => {
            match publish::begin(&options.site, &options.out()) {
                Ok(staging) => Some(staging),
                Err(problem) => {
                    prepared.problems.push(problem);
                    None
                }
            }
        }
        _ => None,
    };
    let plan = Plan::of(&prepared.renderer, &mut prepared.problems);
    let renderer = &prepared.renderer;
    let writes = match &staging {
        Some(staging) => Writes::Into(staging),
        None => Writes::Nowhere,
    };
    let (mut built, (files, file_problems)) = rayon::join(
        || {
            plan.outputs
                .par_iter()
                .map(|&output| build_output(renderer, &plan.linked, output, writes))
                .collect::<Vec<Built>>()
        },
        || match &staging {
            Some(staging) => publish_files(renderer, staging, &plan.held),
            None => (0, Vec::new()),
        },
    );
    prepared.problems.extend(file_problems);
    let problems = std::mem::take(&mut prepared.problems);
    let search = &mut crate::search::Kept::default();
    let concluded = conclude(renderer, &plan, &built, problems, None, search);
    let mut record = Record::default();
    for (i, built) in built.iter_mut().enumerate() {
        record.set(OutputId::from_index(i), std::mem::take(&mut built.reads));
    }
    // A run given another address leaves what Statac keeps for the site
    // at its own: nothing is removed from it, and no post is kept as
    // feed readers knowing it.
    let own_address = options.url.is_none();
    let published = Publishing {
        staging,
        files,
        progress,
        tidies: own_address,
        after: true,
        serving: false,
        remembers: own_address,
        unheld,
    };
    let mut outcome = publish(options, prepared, &plan, &built, concluded, published);
    outcome.record = record;
    outcome
}

/// What the outputs built come to together, before anything more is
/// written.
pub(crate) struct Concluded {
    /// Every problem, reading the site and building it.
    pub problems: Vec<Problem>,
    javascript: Vec<Javascript>,
    /// The photographs whose copies some output names.
    developed: Vec<crate::album::Shown>,
    /// What the outputs asked of the site's cards.
    cards: templates::Gathered,
    /// The held pictures no page links, by their place among the files
    /// beside the writing.
    held_back: HashSet<usize>,
    /// The search index of each language some page names it for.
    search: Vec<crate::search::File>,
}

impl Concluded {
    /// Whether any problem stops the build.
    pub(crate) fn stopped(&self) -> bool {
        stops(&self.problems)
    }
}

/// Concludes over every output `built` of the site, in the order
/// of `plan`, adding to the `problems` reading it: what is told once
/// across them, what their links came to, the JavaScript they carry, and
/// the copies and cards they ask for, and the search index, taking from
/// `search` what an earlier build made of it from the same. Nothing a
/// build of them left anywhere else is read.
pub(crate) fn conclude(
    renderer: &Renderer,
    plan: &Plan,
    built: &[Built],
    mut problems: Vec<Problem>,
    remembered: Option<&Remembered>,
    search: &mut crate::search::Kept,
) -> Concluded {
    let site = renderer.site();
    let pictures = &site.pictures;
    let linked = &plan.linked;

    let javascript = javascript(renderer, built, remembered);
    let reached: HashSet<&str> = built
        .iter()
        .filter_map(|built| built.links.as_ref())
        .flat_map(|checked| checked.reached())
        .map(String::as_str)
        .collect();
    let developed = templates::developed(built.iter().flat_map(|built| &built.developed).copied());
    let cards = templates::Gathered::of(built.iter().flat_map(|built| &built.cards));
    // A picture a page is shared with is published as it is.
    let own_cards = renderer.own_cards(&cards);
    let mut held_back = HashSet::new();
    for (&at, id) in &plan.held {
        let file = &site.files[at];
        let shown = developed.binary_search_by_key(id, |shown| shown.id).is_ok();
        let linked = reached.contains(file.address.file().as_str()) || own_cards.contains_key(id);
        if shown && !linked {
            held_back.insert(at);
        }
    }
    let converted: Vec<Problem> = developed
        .iter()
        .filter_map(|shown| {
            let picture = pictures.get(shown.id);
            match &picture.seen {
                crate::album::Seen::Photograph {
                    photograph,
                    converts: true,
                    ..
                } => Some(Problem::nowhere(Kind::ColourConverted {
                    path: picture.shown(),
                    profile: photograph.profile.clone().flatten(),
                })),
                _ => None,
            }
        })
        .collect();
    // The languages some page names the search index of.
    let indexed = built.iter().flat_map(|built| &built.indexed);
    let mut indexed: Vec<crate::site::LanguageId> = indexed.copied().collect();
    indexed.sort_unstable();
    indexed.dedup();
    let entries: Vec<String> = (indexed.iter())
        .map(|&language| crate::search::entry(site, language))
        .collect();
    let made: HashSet<&str> = cards
        .files()
        .chain(entries.iter().map(String::as_str))
        .collect();
    let broken = linked.told(
        renderer,
        built.iter().filter_map(|built| built.links.as_ref()),
        &made,
    );
    // What each list's pages told, by list.
    let mut on_list: HashMap<ListId, Vec<Problem>> = HashMap::new();
    // Where in `problems` each problem a page's build met is, by its place.
    let mut met: HashMap<Option<Place>, Vec<usize>> = HashMap::new();
    // Where in `problems` each layout showing none of its pages' writing
    // is told, by its place.
    let mut hollow: HashMap<Option<Place>, usize> = HashMap::new();
    // Where in `problems` the words a language lacks are told, by the
    // language and the words.
    let mut wordless: HashMap<(String, crate::problem::ShownWords), usize> = HashMap::new();
    for built in built {
        if let Output::Page(page) = built.output
            && let Some(list) = site.pages.get(page).speaks_for
        {
            on_list.insert(list, built.problems.clone());
        }
        match built.output {
            // Built after the pages, through their first page's templates:
            // what went wrong there is told once, naming the first page
            // that met it.
            Output::Later(list, _) => {
                let told = on_list.entry(list).or_default();
                let new: Vec<Problem> = built
                    .problems
                    .iter()
                    .filter(|problem| {
                        !problems.contains(problem)
                            && !told.iter().any(|t| t.alike_but_for_page(problem))
                    })
                    .cloned()
                    .collect();
                told.extend(new.iter().cloned());
                problems.extend(new);
            }
            _ => {
                for problem in &built.problems {
                    // Told at the template, alike for every page built
                    // through it handed the same items, or told of a page
                    // every output naming it names: told once.
                    let once = matches!(problem.kind.code(), Code::SortedLast | Code::UntitledPage);
                    if once && problems.contains(problem) {
                        continue;
                    }
                    // A video with no still is told once across the site,
                    // where the first page to read its still asks for it,
                    // and so is a call to `webmention()` on a site without
                    // webmentions.
                    if matches!(problem.kind.code(), Code::NoStill | Code::WebmentionsOff)
                        && problems.iter().any(|told| told.kind == problem.kind)
                    {
                        continue;
                    }
                    // A value the `markdown` filter reads on many pages, or
                    // through many templates, is told at its place once,
                    // with the first filter and page that met it. The
                    // writing itself is told once already.
                    let read_as_markdown = crate::markdown::told_of_a_value(&problem.kind);
                    if read_as_markdown
                        && problems
                            .iter()
                            .any(|t| t.kind == problem.kind && t.place == problem.place)
                    {
                        continue;
                    }
                    // Each page says its layout never shows its writing;
                    // the layout is told once, counting them.
                    if let Kind::WritingNeverShown { .. } = problem.kind {
                        match hollow.get(&problem.place) {
                            Some(&at) => {
                                problems[at].met_building_too(problem);
                                if let Kind::WritingNeverShown { pages, .. } =
                                    &mut problems[at].kind
                                {
                                    *pages += 1;
                                }
                            }
                            None => {
                                hollow.insert(problem.place, problems.len());
                                problems.push(problem.clone());
                            }
                        }
                        continue;
                    }
                    // Each page says its language lacks the words it is
                    // shown with; they are told once, counting the pages.
                    if let Kind::MissingWords { tag, words, .. } = &problem.kind {
                        match wordless.get(&(tag.clone(), *words)) {
                            Some(&at) => {
                                problems[at].met_building_too(problem);
                                if let Kind::MissingWords { pages, .. } = &mut problems[at].kind {
                                    *pages += 1;
                                }
                            }
                            None => {
                                wordless.insert((tag.clone(), *words), problems.len());
                                problems.push(problem.clone());
                            }
                        }
                        continue;
                    }
                    // What many pages' builds met at one place is told once,
                    // with each page that met it.
                    if problem.built().next().is_some() {
                        let alike = met.entry(problem.place).or_default();
                        let told = alike
                            .iter()
                            .find(|&&at| problems[at].alike_but_for_page(problem));
                        match told {
                            Some(&at) => {
                                problems[at].met_building_too(problem);
                                continue;
                            }
                            None => alike.push(problems.len()),
                        }
                    }
                    problems.push(problem.clone());
                }
            }
        }
    }
    // Each page says its first call inside HTML; the site says only the
    // first page's.
    let mut said = false;
    problems.retain(|problem| {
        problem.kind.code() != Code::CallInHtml || !std::mem::replace(&mut said, true)
    });
    problems.extend(broken);
    problems.extend(converted);
    if built.iter().any(|built| built.sized) {
        problems.extend(renderer.assets().narrowed().cloned());
    }
    problems.extend(colours_not_loaded(renderer, built));
    problems.extend(
        renderer.loops_over_every_page(built.iter().map(|built| (built.walked, &built.reads))),
    );
    // What a card's texts met may be what its page's layout met too.
    let of_cards: Vec<Problem> = renderer
        .cards_told(&cards)
        .into_iter()
        .filter(|problem| !problems.contains(problem))
        .collect();
    problems.extend(of_cards);
    let wanted = renderer.cards_wanted(&cards);
    // Made only for the languages some page names it for, and only of a
    // site that builds.
    let search = match stops(&problems) {
        false => crate::search::files(renderer, &indexed, search),
        true => Vec::new(),
    };
    if !wanted.is_empty() || !search.is_empty() {
        // A card's address is known only once its page is built, and the
        // search index's once every page is, so they are held to the
        // addresses already taken here.
        // Pages sharing one card share its file.
        let mut claimed = HashSet::new();
        let mut new = Vec::new();
        for card in &wanted {
            if claimed.insert(card.file.as_str()) {
                new.push((card.file.clone(), Named::File(card.from.to_owned())));
            }
        }
        new.extend(crate::search::claims(&search));
        // The rest were held to each other as the site was read. Two
        // addresses meet only where they start with one folder or file.
        let top = |rel: &str| crate::platform::folded(rel.split('/').next().unwrap_or(rel));
        let tops: HashSet<String> = new.iter().map(|(rel, _)| top(rel)).collect();
        let mut outputs = renderer.assets().claims();
        outputs.extend(content::claims(
            site,
            renderer.sources(),
            renderer.settings(),
        ));
        outputs.extend(pictures.claims());
        outputs.extend(feeds::claims(renderer.feeds()));
        outputs.retain(|(rel, _)| tops.contains(&top(rel)));
        outputs.extend(new);
        let same = assets::same_address(&outputs, &renderer.settings().base());
        let new: Vec<Problem> = same
            .into_iter()
            .filter(|problem| !problems.contains(problem))
            .collect();
        problems.extend(new);
    }
    drop(wanted);
    Concluded {
        problems,
        javascript,
        developed,
        cards,
        held_back,
        search,
    }
}

/// The warning for pages showing coloured code where nothing `built` read
/// the stylesheet holding the colours, however it used it: told once, at
/// the first block coloured on the first such page by its file's path.
fn colours_not_loaded(renderer: &Renderer, built: &[Built]) -> Option<Problem> {
    let stylesheet = Key::Asset(renderer.assets().id(crate::highlight::STYLESHEET)?);
    let pages = built
        .iter()
        .filter(|built| built.file.ends_with(".html") || built.file.ends_with(".htm"));
    let mut blocks: Vec<Place> = pages.flat_map(|built| &built.coloured).copied().collect();
    if blocks.is_empty()
        || built
            .iter()
            .any(|built| built.reads.keys().contains(&stylesheet))
    {
        return None;
    }
    let sources = renderer.sources();
    blocks.sort_unstable_by_key(|block| (sources.path(block.source), block.span.start));
    Some(Problem::at(Kind::CodeColoursNotLoaded, *blocks.first()?))
}

/// Where a concluded build is written, and how.
pub(crate) struct Publishing {
    /// The folder written into, holding every output already; none for a
    /// build that writes nothing.
    pub staging: Option<Staging>,
    /// How many files published as they are the folder holds, the held
    /// pictures counted.
    pub files: usize,
    pub progress: Progress,
    /// Whether what earlier builds kept and no page asks for any more is
    /// removed.
    pub tidies: bool,
    /// Whether the command the site runs on a built site is run.
    pub after: bool,
    /// Whether the site is built to be previewed, which a command it runs
    /// is told.
    pub serving: bool,
    /// Whether the posts published are kept as what feed readers now
    /// know.
    pub remembers: bool,
    /// Why the build doesn't hold the site, where it doesn't.
    pub unheld: Option<Problem>,
}

/// Writes what the outputs `built` asked for beside them, the pictures a
/// page links, the copies of photographs and the cards, and puts the
/// folder written in the output's place, where nothing stops the build.
pub(crate) fn publish(
    options: &Options,
    prepared: Prepared,
    plan: &Plan,
    built: &[Built],
    concluded: Concluded,
    publishing: Publishing,
) -> Outcome {
    let Prepared {
        renderer,
        problems: _,
        quiet,
        included,
        left_out,
        known,
    } = prepared;
    let Concluded {
        mut problems,
        javascript,
        developed,
        cards: asked,
        held_back,
        search,
    } = concluded;
    let Publishing {
        staging,
        mut files,
        progress,
        tidies,
        after,
        serving,
        remembers,
        unheld,
    } = publishing;
    let site = renderer.site();
    let pictures = &site.pictures;
    if let Some(staging) = &staging {
        for &at in plan.held.keys().filter(|at| !held_back.contains(at)) {
            let file = &site.files[at];
            let named = format!("content/{}", file.path);
            let source = site.open_content(&file.path);
            problems.extend(staging.copy(source, &named, &file.address.file()).err());
        }
    }
    files = files.saturating_sub(held_back.len());
    let mut held_back: Vec<String> = (held_back.iter())
        .map(|&at| site.files[at].address.file())
        .collect();
    held_back.sort();
    let (mut pages, mut redirects, mut feeds) = (0, 0, 0);
    for built in built.iter().filter(|built| built.written) {
        match built.output {
            Output::Page(_) | Output::Later(..) => pages += 1,
            Output::Redirect(_) => redirects += 1,
            Output::Feed(_) => feeds += 1,
        }
    }

    let mut copies = Copies::default();
    let mut cards = Cards::default();
    if let Some(staging) = &staging
        && !stops(&problems)
    {
        let made = crate::album::develop(&options.site, pictures, &developed, staging, progress);
        (copies.made, copies.from) = (made.made, made.from);
        files += made.published;
        problems.extend(made.problems);
    }
    if let Some(staging) = &staging
        && !stops(&problems)
    {
        let wanted = renderer.cards_wanted(&asked);
        let drew = crate::cards::keep::publish(&options.site, &wanted, staging, progress);
        cards.drawn = drew.drawn;
        files += drew.published;
        problems.extend(drew.problems);
    }
    if let Some(staging) = &staging
        && !stops(&problems)
    {
        let written = (search.par_iter()).map(|file| match file.named {
            true => staging.write_named(&file.path, &file.bytes),
            false => staging.write(&file.path, &file.bytes),
        });
        let unwritten: Vec<Problem> = written.filter_map(Result::err).collect();
        files += search.len();
        problems.extend(unwritten);
    }
    // Whatever else couldn't be kept has said so already, and once is
    // enough for the one cause they mostly share.
    for unkept in [unheld, staging.as_ref().and_then(Staging::unkept)]
        .into_iter()
        .flatten()
    {
        if !problems
            .iter()
            .any(|problem| problem.kind.code() == Code::CacheNotWritten)
        {
            problems.push(unkept);
        }
    }
    let mut hooks = crate::hooks::Ran::default();
    let published = match staging {
        Some(staging) if !stops(&problems) => {
            let put = match after {
                true => {
                    let settings = renderer.settings();
                    crate::hooks::publish(settings, options, staging, progress, serving)
                }
                false => staging.finish().map(|()| (None, Vec::new())),
            };
            match put {
                Ok((changed, told)) => {
                    hooks.after = changed;
                    problems.extend(told);
                    Some(Published {
                        pages,
                        redirects,
                        feeds,
                        files,
                    })
                }
                Err(problem) => {
                    problems.push(problem);
                    None
                }
            }
        }
        _ => None,
    };
    if published.is_some() {
        if tidies {
            let shown = developed.iter().map(|shown| (shown.id, shown.full));
            // What a page left out may show, its copy at its own width too.
            let asked_for = shown.chain(left_out.into_iter().map(|id| (id, true)));
            copies.removed = crate::album::sweep(&options.site, pictures, asked_for);
            cards.removed = crate::cards::keep::sweep(&options.site, &renderer.cards_kept(&asked));
        }
        // The site is built; a build that can't keep the index reads the
        // pictures again next time, and says so where nothing has yet.
        if let Err(error) = pictures.keep_index(&options.site)
            && !problems
                .iter()
                .any(|problem| problem.kind.code() == Code::CacheNotWritten)
        {
            problems.push(Problem::nowhere(Kind::CacheNotWritten {
                path: Path::new(crate::album::KEPT).join("pictures"),
                reason: Reason::of(&error),
            }));
        }
        let kept = match remembers {
            true => known.keep(
                &options.site,
                site,
                renderer.settings(),
                renderer.dates(),
                renderer.sources(),
                renderer.feeds(),
            ),
            false => Ok(()),
        };
        if let Err(error) = kept
            && !problems
                .iter()
                .any(|problem| problem.kind.code() == Code::CacheNotWritten)
        {
            problems.push(Problem::nowhere(Kind::CacheNotWritten {
                path: feeds::known::shown(),
                reason: Reason::of(&error),
            }));
        }
    }
    drop(asked);
    let fetched = match published {
        Some(_) => crate::fetched::Ages::of(site, &renderer.today()),
        None => crate::fetched::Ages::default(),
    };
    Outcome {
        held: Held::Renderer(renderer),
        problems,
        quiet,
        published,
        javascript,
        record: Record::default(),
        included,
        copies,
        cards,
        hooks,
        fetched,
        held_back,
        waited: Duration::ZERO,
    }
}

pub(crate) fn stops(problems: &[Problem]) -> bool {
    problems
        .iter()
        .any(|p| p.kind.severity() == Severity::Error)
}

/// Whether `site` is a folder that can be read, or the problem with it.
pub(crate) fn a_folder(site: &Path) -> Result<(), Problem> {
    let cant_read = |reason: Reason| {
        Problem::nowhere(Kind::CantRead {
            path: site.to_path_buf(),
            reason,
        })
    };
    match fs::metadata(site) {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => Err(cant_read(Reason::FileInTheWay)),
        Err(error) => Err(cant_read(Reason::of(&error))),
    }
}

/// The folders a site is read from.
const READ_FROM: [&str; 8] = [
    "content",
    templates::TEMPLATES,
    templates::SNIPPETS,
    assets::ASSETS,
    assets::STATIC,
    crate::data::FOLDER,
    crate::language::FOLDER,
    crate::fetched::FOLDER,
];

/// The problem with building the site at `site` into `out`, where `out`
/// is one of the folders the site is read from or inside one, so the next
/// build would read what this one wrote.
fn in_the_way(site: &Path, out: &Path) -> Option<Problem> {
    let site = fs::canonicalize(site).ok()?;
    let whole = std::path::absolute(out).ok()?;
    let whole = crate::import::whole_to_be(&publish::lexically_normal(&whole));
    let folds = crate::platform::ignores_capitals(&site);
    let same = |a: &std::ffi::OsStr, b: &std::ffi::OsStr| {
        a == b
            || folds
                && match (a.to_str(), b.to_str()) {
                    (Some(a), Some(b)) => crate::platform::folded(a) == crate::platform::folded(b),
                    _ => false,
                }
    };
    let mut pieces = whole.components();
    for piece in site.components() {
        if !pieces
            .next()
            .is_some_and(|other| same(piece.as_os_str(), other.as_os_str()))
        {
            return None;
        }
    }
    let first = pieces.next()?;
    let folder = READ_FROM
        .into_iter()
        .find(|folder| same(first.as_os_str(), folder.as_ref()))?;
    Some(Problem::nowhere(Kind::OutputInTheWay {
        out: out.to_path_buf(),
        folder: folder.to_owned(),
        whole: pieces.next().is_none(),
    }))
}

/// Builds `output`, follows its links where it is a page written as HTML,
/// and writes it where it lands as `writes` says.
pub(crate) fn build_output(
    renderer: &Renderer,
    linked: &links::Site,
    output: Output,
    writes: Writes<'_>,
) -> Built {
    let site = renderer.site();
    let feeds = renderer.feeds();
    // The page shown, and which of its list's pages; none for a redirect,
    // whose one link is told where `redirect_to` is written.
    let mut shown = None;
    let (rendered, address) = match output {
        Output::Page(page) => {
            shown = Some((page, 1));
            (renderer.render(page), &site.pages.get(page).address)
        }
        Output::Later(list, number) => {
            let list = site.lists.get(list);
            let page = list.page.expect("a list with later pages has a page");
            let address = list
                .later_address(number)
                .expect("a later page is built where it stands");
            shown = Some((page, number));
            (renderer.render_later(page, number), address)
        }
        Output::Redirect(at) => {
            let redirect = &site.redirects[at];
            (renderer.render_redirect(redirect), &redirect.from)
        }
        Output::Feed(at) => (renderer.render_feed(&feeds[at]), &feeds[at].address),
    };
    let file = address.file();
    let mut problems = rendered.problems;
    let mut scripts = Vec::new();
    let mut pieces = Vec::new();
    let mut checked = None;
    let mut written = false;
    let mut held = None;
    let mut sized = false;
    if let Some(html) = rendered.html {
        scripts = scripts::in_html(&html);
        if scripts.iter().any(|f| f.sure && f.found == Found::Written) {
            pieces = scripts::written(&html)
                .into_iter()
                .map(|piece| piece.print)
                .collect();
        }
        if let Some((page, number)) = shown
            && file.ends_with(".html")
        {
            let page = links::Page {
                id: page,
                number,
                address,
                reads: &rendered.reads,
                // A card is published once a page asking for it is built.
                cards: rendered
                    .cards
                    .iter()
                    .filter_map(|card| card.file().map(str::to_owned))
                    .collect(),
                own: &rendered.own,
                snipped: &rendered.snipped,
            };
            checked = Some(linked.check(renderer, page, &html));
            sized = sizes_a_picture(&html);
        }
        match writes {
            Writes::Nowhere => {}
            Writes::Into(staging) => match staging.write(&file, html.as_bytes()) {
                Ok(()) => written = true,
                Err(problem) => problems.push(problem),
            },
            Writes::Later => held = Some(html),
        }
    }
    Built {
        output,
        file,
        html: held,
        reads: rendered.reads,
        asked: rendered.asked,
        problems,
        scripts,
        pieces,
        links: checked,
        developed: rendered.developed,
        cards: rendered.cards,
        sized,
        coloured: rendered.coloured,
        walked: rendered.walked,
        written,
        indexed: rendered.indexed,
    }
}

/// Whether `html` holds an `<img>` with both a `width` and a `height`.
fn sizes_a_picture(html: &str) -> bool {
    let bytes = html.as_bytes();
    let mut at = 0;
    while let Some(found) = html[at..].find('<') {
        let name = at + found + 1;
        at = name;
        let is_img = bytes
            .get(name..name + 3)
            .is_some_and(|n| n.eq_ignore_ascii_case(b"img"))
            && bytes
                .get(name + 3)
                .is_none_or(|&after| crate::html::ends_name(after));
        if !is_img {
            continue;
        }
        let tag = crate::html::tag(html, name, usize::MAX);
        at = tag.end;
        let given = |side: &str| tag.get(side).is_some_and(|value| !value.is_empty());
        if given("width") && given("height") {
            return true;
        }
    }
    false
}

/// Writes the assets, the static files and the files beside the writing,
/// but for those `held` until the pages are built, and says how many the
/// site has, those held counted.
pub(crate) fn publish_files(
    renderer: &Renderer,
    staging: &Staging,
    held: &HashMap<usize, PictureId>,
) -> (usize, Vec<Problem>) {
    let assets = renderer.assets();
    let mut problems = assets.publish(staging);
    let travelling = &renderer.site().files;
    let sent = travelling
        .par_iter()
        .enumerate()
        .filter(|(at, _)| !held.contains_key(at));
    problems.par_extend(sent.filter_map(|(_, file)| {
        let source = renderer.site().open_content(&file.path);
        let shown = format!("content/{}", file.path);
        staging.copy(source, &shown, &file.address.file()).err()
    }));
    let files = assets.iter().count() + assets.statics().len() + travelling.len();
    (files, problems)
}

/// How much of a file is looked at to tell text from binary.
const SNIFFED: usize = 8 * 1024;

/// The endings of files a host sends as a picture, a recording, a font or
/// a PDF. A browser reads a file sent so as that and never as a document,
/// whatever it holds, so it runs no script from one. An SVG is a document.
const NEVER_DOCUMENTS: &[&str] = &[
    "jpg", "jpeg", "jfif", "png", "gif", "webp", "avif", "heic", "heif", "jxl", "bmp", "tif",
    "tiff", "ico", "mp3", "m4a", "aac", "ogg", "oga", "opus", "wav", "flac", "mp4", "m4v", "webm",
    "mov", "ogv", "woff", "woff2", "ttf", "otf", "eot", "pdf",
];

/// Whether the file `name` is one a browser never reads as a document.
fn never_a_document(name: &str) -> bool {
    let extension = Path::new(name).extension().and_then(|e| e.to_str());
    extension.is_some_and(|e| NEVER_DOCUMENTS.contains(&e.to_ascii_lowercase().as_str()))
}

/// A published file's text, where it is text: no NUL in its opening
/// stretch, as binary formats have and text doesn't. A browser may read any
/// text as a document, whatever the file is called, unless its name is one
/// [`never_a_document`] knows. A file that can't be read here can't be
/// published either, which publishing says.
fn text_of(bytes: &[u8]) -> Option<String> {
    if bytes[..bytes.len().min(SNIFFED)].contains(&0) {
        return None;
    }
    Some(String::from_utf8_lossy(bytes).into_owned())
}

/// [`text_of`] what `file`, open to read, holds, read no further than
/// its opening stretch where that is no text.
fn text_in(mut file: fs::File) -> Option<String> {
    let mut head = Vec::with_capacity(SNIFFED);
    std::io::Read::read_to_end(
        &mut std::io::Read::take(&mut file, SNIFFED as u64),
        &mut head,
    )
    .ok()?;
    if head.contains(&0) {
        return None;
    }
    std::io::Read::read_to_end(&mut file, &mut head).ok()?;
    text_of(&head)
}

/// Every file published as it is from `assets/`, `static/` or `content/`:
/// its address, the file it comes from, and what it is.
fn published_as_is(renderer: &Renderer) -> Vec<(String, String, AsIs<'_>)> {
    let assets = renderer.assets();
    let base = renderer.settings().base();
    let from_assets = assets.iter().map(|(_, asset)| {
        (
            format!("{base}/{}", asset.published),
            asset.source.clone(),
            AsIs::Bytes(asset.bytes()),
        )
    });
    let from_static = assets.statics().iter().map(|file| {
        (
            format!("{base}/{}", file.name),
            format!("{}/{}", assets::STATIC, file.name),
            AsIs::Static(file),
        )
    });
    let from_content = renderer.site().files.iter().map(|file| {
        (
            format!("{base}{}", file.address.as_str()),
            format!("content/{}", file.path),
            AsIs::Beside(&file.path),
        )
    });
    from_assets.chain(from_static).chain(from_content).collect()
}

/// A file published as it is.
enum AsIs<'a> {
    /// A file in `static/`.
    Static(&'a assets::StaticFile),
    /// A file beside the writing, by its path under `content/`.
    Beside(&'a str),
    /// What is published, already read.
    Bytes(&'a [u8]),
}

impl AsIs<'_> {
    /// The file opened where it really is and reached through no link, or
    /// none where it is already read.
    fn open(&self, renderer: &Renderer) -> Option<std::io::Result<fs::File>> {
        let assets = renderer.assets();
        match self {
            AsIs::Static(file) => Some(crate::platform::open_file(assets.real(), file.at())),
            AsIs::Beside(path) => Some(renderer.site().open_content(path)),
            AsIs::Bytes(_) => None,
        }
    }
}

/// What was found in the files published as they are, kept from one
/// build of a site to the next by whoever builds it again and again: a
/// file with the length and the time it had is not read a second time.
#[derive(Default)]
pub(crate) struct Remembered {
    /// Each by the file it comes from, from the site's folder.
    scripts: Mutex<HashMap<String, Read>>,
}

/// A file's length and when it was last changed, and what it held then.
type Read = (u64, std::time::SystemTime, Vec<Finding>);

impl Remembered {
    /// What the file `from` names, opened as `opened`, writes that runs
    /// JavaScript or may.
    fn found_in(&self, from: &str, opened: std::io::Result<fs::File>) -> Vec<Finding> {
        let stamp = (opened.as_ref().ok())
            .and_then(|file| file.metadata().ok())
            .and_then(|found| Some((found.len(), found.modified().ok()?)));
        if let Some((length, changed)) = stamp
            && let Some((was, then, findings)) =
                self.scripts.lock().expect("never poisoned").get(from)
            && (*was, *then) == (length, changed)
        {
            return findings.clone();
        }
        let text = opened.ok().and_then(text_in);
        let findings = text.map_or_else(Vec::new, |text| scripts::in_html(&text));
        let mut scripts = self.scripts.lock().expect("never poisoned");
        match stamp {
            Some((length, changed)) => {
                scripts.insert(from.to_owned(), (length, changed, findings.clone()));
            }
            None => {
                scripts.remove(from);
            }
        }
        findings
    }
}

/// Who a page's JavaScript is blamed on.
enum Blame<'a> {
    /// A page built from templates, blamed on those that write it.
    Built(&'a Built),
    /// A file published as it is, blamed on itself.
    File(String),
}

/// What each file's own text writes that runs JavaScript or may, read once.
struct Texts<'a> {
    renderer: &'a Renderer,
    /// The page each file of writing is.
    pages: HashMap<SourceId, PageId>,
    found: HashMap<SourceId, Vec<Finding>>,
    pieces: HashMap<SourceId, Vec<scripts::Piece>>,
}

impl<'a> Texts<'a> {
    fn new(renderer: &'a Renderer) -> Self {
        let pages = renderer.site().pages.iter();
        Texts {
            renderer,
            pages: pages
                .filter_map(|(id, page)| Some((page.source?, id)))
                .collect(),
            found: HashMap::new(),
            pieces: HashMap::new(),
        }
    }

    /// What a page's file puts on a page, where that isn't its text as it
    /// stands: writing in Markdown shows as code, or escapes, what its
    /// text read as HTML would take for markup. Asked only of a file whose
    /// text holds something to take so.
    fn html(&self, source: SourceId) -> Option<String> {
        let page = *self.pages.get(&source)?;
        self.renderer.written_by_its_file(page)
    }

    fn of(&mut self, source: SourceId) -> &[Finding] {
        if !self.found.contains_key(&source) {
            let mut found = scripts::in_html(self.renderer.sources().text(source));
            if !found.is_empty()
                && let Some(html) = self.html(source)
            {
                found = scripts::in_html(&html);
            }
            self.found.insert(source, found);
        }
        &self.found[&source]
    }

    /// Each piece of JavaScript the file writes in where a browser runs it.
    fn pieces(&mut self, source: SourceId) -> &[scripts::Piece] {
        if !self.pieces.contains_key(&source) {
            let pieces = if self.of(source).is_empty() {
                Vec::new()
            } else {
                match self.html(source) {
                    Some(html) => scripts::written(&html),
                    None => scripts::written(self.renderer.sources().text(source)),
                }
            };
            self.pieces.insert(source, pieces);
        }
        &self.pieces[&source]
    }

    /// Whether the file writes JavaScript in: where a browser runs it for
    /// `sure`, anywhere one may otherwise.
    fn writes(&mut self, source: SourceId, sure: bool) -> bool {
        self.of(source)
            .iter()
            .any(|finding| finding.found == Found::Written && (finding.sure || !sure))
    }

    /// Whether the file writes JavaScript in only where a browser may not
    /// run it.
    fn may_write(&mut self, source: SourceId) -> bool {
        self.of(source)
            .iter()
            .any(|finding| finding.found == Found::Written && !finding.sure)
    }
}

/// The files a built page read that could have written its JavaScript.
struct Reader {
    /// Every template and snippet it was built from, every page whose
    /// writing it shows, and its own file.
    read: Vec<SourceId>,
    /// Those of them that are snippets called by name.
    called: Vec<SourceId>,
    /// The page's own file, where it has one.
    source: Option<SourceId>,
    /// For a page Statac made, the files of the pages whose fields it shows.
    fields: Vec<SourceId>,
}

impl Reader {
    fn of(renderer: &Renderer, page: &Built) -> Reader {
        let site = renderer.site();
        let shown = match page.output {
            Output::Page(page) => Some(page),
            Output::Later(list, _) => Some(
                site.lists
                    .get(list)
                    .page
                    .expect("a list with later pages has a page"),
            ),
            Output::Redirect(at) => Some(site.redirects[at].page),
            Output::Feed(_) => None,
        };
        // A page Statac made has no file of its own to have written one.
        let source = shown.and_then(|shown| site.pages.get(shown).source);
        let mut called = Vec::new();
        // Another page's writing shown here, as a feed or a front page
        // showing whole posts does, can carry its own scripts.
        let mut read: Vec<SourceId> = page
            .reads
            .keys()
            .iter()
            .filter_map(|key| match key {
                Key::Template(id) => {
                    let source = renderer.template_source(*id);
                    if renderer.called_by_name(*id) {
                        called.push(source);
                    }
                    Some(source)
                }
                Key::Body(other) => site.pages.get(*other).source,
                _ => None,
            })
            .chain(source)
            .collect();
        read.sort_unstable();
        read.dedup();
        called.sort_unstable();
        called.dedup();
        let mut fields = Vec::new();
        if shown.is_some() && source.is_none() {
            fields.extend(page.reads.keys().iter().filter_map(|key| match key {
                Key::Field(other, _) => site.pages.get(*other).source,
                _ => None,
            }));
            fields.sort_unstable();
            fields.dedup();
        }
        Reader {
            read,
            called,
            source,
            fields,
        }
    }

    fn calls(&self, source: SourceId) -> bool {
        self.called.binary_search(&source).is_ok()
    }

    /// Whether a snippet the page calls by name loaded or embedded `found`,
    /// as far as the files tell: one holds it as written, and no other
    /// file the page read holds it too, or holds one whose address is
    /// filled in as the page is built.
    fn chose(&self, found: &Found, texts: &mut Texts) -> bool {
        let filled = |address: &str| address.contains("{{") || address.contains("{%");
        let could_be = |held: &Found| match (held, found) {
            (Found::Loaded(address), Found::Loaded(_))
            | (Found::Embedded(address), Found::Embedded(_)) => filled(address),
            _ => false,
        };
        let mut chosen = false;
        for &source in &self.read {
            let held = texts.of(source);
            if self.calls(source) {
                chosen |= held.iter().any(|each| each.found == *found);
            } else if held
                .iter()
                .any(|each| each.found == *found || could_be(&each.found))
            {
                return false;
            }
        }
        chosen
    }

    /// The files JavaScript written into the page is told as written in,
    /// and whether a browser runs it there, for a page with `runs` findings
    /// of it where a browser runs it, each of them one of `pieces`, and
    /// with some where a browser may not for `may_run`.
    ///
    /// What a snippet called by name writes is the author's choice and is
    /// not told, but only what the snippet's own file can be shown to have
    /// written: a piece on the page that the file holds whole, to the
    /// letter, and that no other file the page read holds. Every other
    /// piece is told of some file, as it would be were no snippet called:
    /// of each file that writes some, or where the files hold every piece
    /// on the page to the letter, of those holding one.
    fn wrote(
        &self,
        runs: usize,
        pieces: &[scripts::Print],
        may_run: bool,
        texts: &mut Texts,
    ) -> Vec<(SourceId, bool)> {
        // The called snippets that write nothing a template fills in, and
        // what they write.
        let mut cleared = Vec::new();
        let mut chosen = HashSet::new();
        for &snippet in &self.called {
            let held = texts.pieces(snippet);
            if held.iter().all(|piece| !piece.filled) {
                cleared.push(snippet);
            }
            chosen.extend(
                held.iter()
                    .filter(|piece| !piece.filled)
                    .map(|piece| piece.print),
            );
        }
        let mut all_chosen =
            runs > 0 && runs == pieces.len() && pieces.iter().all(|piece| chosen.contains(piece));
        if all_chosen {
            all_chosen = !self.read.iter().any(|&source| {
                !self.calls(source)
                    && texts
                        .pieces(source)
                        .iter()
                        .any(|piece| pieces.contains(&piece.print))
            });
        }
        let sure = runs > 0 && !all_chosen;
        if !sure && !may_run {
            return Vec::new();
        }
        let mut from: Vec<SourceId> = Vec::new();
        if sure {
            from.extend(
                self.read
                    .iter()
                    .copied()
                    .filter(|&source| !cleared.contains(&source) && texts.writes(source, true)),
            );
            // Where every piece on the page is one some file holds to the
            // letter, a file holding none of them wrote none of them: its
            // own went unshown, or was shown as text.
            let holds = |texts: &mut Texts, source: SourceId, piece: &scripts::Print| {
                texts.pieces(source).iter().any(|held| held.print == *piece)
            };
            let all_held = pieces
                .iter()
                .all(|piece| self.read.iter().any(|&source| holds(texts, source, piece)));
            if all_held {
                let wrote: Vec<SourceId> = from
                    .iter()
                    .copied()
                    .filter(|&source| pieces.iter().any(|piece| holds(texts, source, piece)))
                    .collect();
                if !wrote.is_empty() {
                    from = wrote;
                }
            }
        }
        if from.is_empty() {
            // A cleared snippet's own pieces are run wherever it is called,
            // so only one writing some a browser may not run could have
            // written such a one.
            from.extend(self.read.iter().copied().filter(
                |&source| match cleared.contains(&source) {
                    true => !sure && texts.may_write(source),
                    false => texts.writes(source, false),
                },
            ));
        }
        // Written by a value no file holds as markup, such as a field
        // passed through `safe`: the page's own file holds the value,
        // or for a page Statac made, a file whose fields it shows.
        if from.is_empty() {
            from.extend(
                self.fields
                    .iter()
                    .copied()
                    .filter(|&field| texts.writes(field, false)),
            );
        }
        if from.is_empty() {
            from.extend(self.source.or(self.read.first().copied()));
        }
        from.into_iter().map(|source| (source, sure)).collect()
    }
}

/// The JavaScript the built site carries: each script it publishes, from
/// any folder, whether or not a page loads it; each address elsewhere a
/// page loads one from; each file that writes JavaScript into a page,
/// pages built and every text file published as it is that a browser may
/// read as a document alike; and
/// each thing embedded that may run its own.
fn javascript(
    renderer: &Renderer,
    built: &[Built],
    remembered: Option<&Remembered>,
) -> Vec<Javascript> {
    let assets = renderer.assets();
    let as_is = published_as_is(renderer);
    let mut published: Vec<assets::Travelled> = assets.travelled();
    published.extend(
        as_is
            .iter()
            .filter(|(_, from, _)| from.starts_with("content/") && assets::is_script(from))
            .map(|(address, from, _)| assets::Travelled {
                address: address.clone(),
                from: from.clone(),
            }),
    );
    let documents: Vec<(Vec<Finding>, String)> = as_is
        .par_iter()
        // A script is reported whole already, as the script it is.
        .filter(|(_, from, _)| !assets::is_script(from) && !never_a_document(from))
        .filter_map(|(_, from, file)| {
            let findings = match (file.open(renderer), remembered) {
                (Some(opened), Some(remembered)) => remembered.found_in(from, opened),
                (Some(opened), None) => scripts::in_html(&text_in(opened.ok()?)?),
                (None, _) => {
                    let AsIs::Bytes(bytes) = file else {
                        return None;
                    };
                    scripts::in_html(&text_of(bytes)?)
                }
            };
            (!findings.is_empty()).then(|| (findings, from.clone()))
        })
        .collect();
    let origin = renderer
        .settings()
        .text("url")
        .map(|url| site_origin(url.value))
        .unwrap_or_default();
    let mut on: BTreeMap<(Carried, bool), usize> = published
        .iter()
        .map(|script| {
            let carried = Carried::Published {
                address: script.address.clone(),
                from: script.from.clone(),
            };
            ((carried, true), 0)
        })
        .collect();
    let by_address: HashMap<&str, Carried> = published
        .iter()
        .map(|script| {
            let carried = Carried::Published {
                address: script.address.clone(),
                from: script.from.clone(),
            };
            (script.address.as_str(), carried)
        })
        .collect();
    let sources = renderer.sources();
    let mut texts = Texts::new(renderer);
    let every = built
        .iter()
        .map(|page| (&page.scripts, Blame::Built(page)))
        .chain(
            documents
                .iter()
                .map(|(findings, from)| (findings, Blame::File(from.clone()))),
        );
    for (findings, blame) in every {
        if findings.is_empty() {
            continue;
        }
        let reader = match &blame {
            Blame::Built(page) => Some(Reader::of(renderer, page)),
            Blame::File(_) => None,
        };
        let mut chosen = |found: &Found| {
            reader
                .as_ref()
                .is_some_and(|reader| reader.chose(found, &mut texts))
        };
        let mut carried: Vec<(Carried, bool)> = Vec::new();
        // How often JavaScript is written in where a browser runs it, and
        // whether it is where one may.
        let (mut runs, mut may_run) = (0, false);
        for finding in findings {
            let sure = finding.sure;
            match &finding.found {
                Found::Written if sure => runs += 1,
                Found::Written => may_run = true,
                Found::Embedded(address) => {
                    if !chosen(&finding.found) {
                        carried.push((Carried::Embedded(address.clone()), sure))
                    }
                }
                Found::Loaded(src) => {
                    let local = (!origin.is_empty())
                        .then(|| src.strip_prefix(origin.as_str()))
                        .flatten()
                        .filter(|path| path.starts_with('/'))
                        .unwrap_or(src.as_str());
                    let path = local.split(['?', '#']).next().unwrap_or_default();
                    match by_address.get(path) {
                        Some(script) => carried.push((script.clone(), sure)),
                        None if chosen(&finding.found) => {}
                        None => carried.push((Carried::Loaded(src.clone()), sure)),
                    }
                }
            }
        }
        match (blame, reader) {
            (Blame::File(from), _) if runs > 0 || may_run => {
                carried.push((Carried::Written(from), runs > 0))
            }
            (Blame::Built(page), Some(reader)) if runs > 0 || may_run => carried.extend(
                reader
                    .wrote(runs, &page.pieces, may_run, &mut texts)
                    .into_iter()
                    .map(|(source, sure)| (Carried::Written(slashed(sources.path(source))), sure)),
            ),
            _ => {}
        }
        // Sure before unsure, so a page carrying one both ways counts as sure.
        carried.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
        carried.dedup_by(|later, first| later.0 == first.0);
        for each in carried {
            *on.entry(each).or_default() += 1;
        }
    }
    let unsure: Vec<Carried> = on
        .keys()
        .filter(|(_, sure)| !sure)
        .map(|(carried, _)| carried.clone())
        .collect();
    on.into_iter()
        .filter(|((carried, sure), pages)| !(*sure && *pages == 0 && unsure.contains(carried)))
        .map(|((carried, sure), pages)| Javascript {
            carried,
            sure,
            pages,
        })
        .collect()
}

/// `https://example.org` from `https://example.org/blog/`.
fn site_origin(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return String::new();
    };
    let host = rest.split('/').next().unwrap_or_default();
    format!("{scheme}://{host}")
}

fn slashed(path: &Path) -> String {
    crate::platform::slashed(path).unwrap_or_else(|lossy| lossy)
}
