//! Reading a site and building it.

pub mod incremental;
mod javascript;
mod scripts;
mod timings;

use std::collections::{HashMap, HashSet};
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
use crate::problem::{Code, Kind, Named, Problem, Published, Reason, Severity};
use crate::publish::{self, Staging};
use crate::reads::{Asked, Key, Reads, Record};
use crate::report::Quiet;
use crate::site::{Id, ListId, OutputId, PageId, PictureId};
use crate::source::{Place, SourceId, Sources};
use crate::templates::{self, Renderer};
use scripts::{Finding, Found};

pub use javascript::Javascript;
pub(crate) use javascript::Remembered;
pub(crate) use timings::Clock;
pub use timings::{Stage, Timings, Work};

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
            renamed: Vec::new(),
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
    /// The files of the pages left out as drafts and until their day:
    /// each page's own, or the data file a page was made from.
    pub pages_left_out: Vec<SourceId>,
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
    /// What the build added to `published.txt`.
    pub kept: crate::published::Kept,
    /// How long the build waited for another build of the site to end.
    pub waited: Duration,
    /// Where the build spent its time.
    pub timings: Timings,
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
            pages_left_out: Vec::new(),
            copies: Copies::default(),
            cards: Cards::default(),
            hooks: crate::hooks::Ran::default(),
            fetched: crate::fetched::Ages::default(),
            held_back: Vec::new(),
            kept: crate::published::Kept::default(),
            waited: Duration::ZERO,
            timings: Timings::default(),
        }
    }
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
    let has = |name: &str| read.templates.contains(name);
    let (feeds, problems) = feeds::plan(&read.site, &read.settings, &read.sources, &has);
    read.problems.extend(problems);
    read.problems
        .extend(feeds::unusable(&read.site, &read.settings, &read.sources));
    read.problems
        .extend(feeds::undated(&read.site, &read.settings, &read.sources));
    read.problems
        .extend(feeds::unasked(&read.site, &read.settings, &read.templates));
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
    let outputs = claims(&read, &assets, &feeds);
    read.problems.extend(assets::same_address(&outputs, &base));
    read.problems
        .extend(crate::hooks::claimed(&read.settings, &outputs, &base));
    read.problems
        .extend(content::sent_nowhere(&read.site, &outputs));
    read.problems
        .extend(content::taken(&read.site, &outputs, &base));
    (read, assets, feeds)
}

/// Every file the site publishes from what is read, by its path from the
/// top of the built site, with what it comes from.
fn claims(read: &content::Read, assets: &Assets, feeds: &[feeds::Feed]) -> Vec<(String, Named)> {
    let mut outputs = assets.claims();
    outputs.extend(read.claims());
    outputs.extend(read.site.pictures.claims());
    outputs.extend(feeds::claims(feeds));
    outputs
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
        renamed: Vec::new(),
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
    /// The files of the pages left out of the build, as
    /// [`Outcome::pages_left_out`] has them.
    pub pages_left_out: Vec<SourceId>,
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
    // Read last, so whether the site has the file moves no other file's
    // place among those read.
    let (published, unread) = crate::published::Record::read(site, &mut read.sources);
    read.problems.extend(unread);
    let outputs = claims(&read, &assets, &feeds);
    read.problems
        .extend(published.gone(&outputs, &read.settings));
    read.site.published = published;
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
        pages_left_out: (read.drafts.iter().chain(&read.waiting).copied()).collect(),
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
    /// How long building it took, and what in.
    pub timed: timings::Output,
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

    /// Every output of the site `renderer` holds.
    pub(crate) fn of(renderer: &Renderer) -> Plan {
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
        outputs.extend((0..site.redirects.len()).map(Output::Redirect));
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
    let mut clock = Clock::start();
    let before = match options.mode {
        Mode::Build => match crate::hooks::before(options, progress, false) {
            Ok(before) => before,
            Err(stopped) => {
                clock.lap(Stage::Before);
                let (sources, problem) = *stopped;
                let mut outcome = Outcome::stopped_at(sources, problem);
                outcome.problems.extend(unheld);
                outcome.waited = waited;
                outcome.timings.stages = clock.stages();
                return outcome;
            }
        },
        Mode::Check => crate::hooks::Before::Unset,
    };
    if before == crate::hooks::Before::Ran {
        clock.lap(Stage::Before);
    }
    let mut outcome = built(options, progress, starter, unheld, clock);
    outcome.hooks.before = before;
    outcome.waited = waited;
    told_once_built(&mut outcome, options);
    outcome
}

/// Adds to `outcome` what is told once the site is built as `options`
/// asks: of the site's own commands, and in a check, of the list pages
/// nothing asks for and of the pictures published with where they were
/// taken.
pub(crate) fn told_once_built(outcome: &mut Outcome, options: &Options) {
    let Held::Renderer(renderer) = &outcome.held else {
        return;
    };
    let named = crate::hooks::names_output(renderer.settings(), renderer.sources(), options);
    outcome.problems.extend(named);
    if options.mode == Mode::Check {
        let unrun = crate::hooks::unrun_by_check(renderer.settings());
        outcome.problems.extend(unrun);
        let has = |name: &str| renderer.has_layout(name);
        let unasked = crate::lists::unasked(renderer.site(), renderer.settings(), &has);
        outcome.problems.extend(unasked);
        // What is held back is only known of a site that could be built.
        if !stops(&outcome.problems) {
            let located = locations_published(renderer.site(), &outcome.held_back);
            outcome.problems.extend(located);
        }
    }
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
    mut clock: Clock,
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
    let plan = Plan::of(&prepared.renderer);
    clock.lap(Stage::Reading);
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
    clock.lap(Stage::Building);
    let saving = staging.as_ref().map_or(Duration::ZERO, Staging::saving);
    let timings = Timings::of_outputs(renderer, &built, saving);
    let problems = std::mem::take(&mut prepared.problems);
    let search = &mut crate::search::Kept::default();
    let concluded = conclude(renderer, &plan, &built, problems, None, search);
    clock.lap(Stage::Concluding);
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
        clock,
    };
    let mut outcome = publish(options, prepared, &plan, &built, concluded, published);
    outcome.record = record;
    outcome.timings = Timings {
        stages: std::mem::take(&mut outcome.timings.stages),
        photographs: std::mem::take(&mut outcome.timings.photographs),
        ..timings
    };
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

    let javascript = javascript::carried(renderer, built, remembered);
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
    // Where in `problems` the outputs built with a template other than
    // their own, or with none, are told, by the kind each page told.
    let mut templateless: Vec<(Kind, usize)> = Vec::new();
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
                    if matches!(
                        problem.kind.code(),
                        Code::NoThumbnail | Code::WebmentionsOff
                    ) && problems.iter().any(|told| told.kind == problem.kind)
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
                    if let Kind::ContentNeverShown { .. } = problem.kind {
                        match hollow.get(&problem.place) {
                            Some(&at) => {
                                problems[at].met_building_too(problem);
                                if let Kind::ContentNeverShown { pages, .. } =
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
                    // Each page, or redirect, says what it was built with in
                    // place of its template; that is told once, counting
                    // them.
                    if matches!(
                        problem.kind,
                        Kind::BuiltWithPageTemplate { .. } | Kind::BuiltWithoutTemplate { .. }
                    ) {
                        match templateless.iter().find(|(kind, _)| *kind == problem.kind) {
                            Some(&(_, at)) => {
                                problems[at].met_building_too(problem);
                                match &mut problems[at].kind {
                                    Kind::BuiltWithPageTemplate { pages: count, .. }
                                    | Kind::BuiltWithoutTemplate { count, .. } => *count += 1,
                                    _ => unreachable!("matched above"),
                                }
                            }
                            None => {
                                templateless.push((problem.kind.clone(), problems.len()));
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
    problems.extend(no_language(renderer, built));
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

/// The note for a site that sets no `language` where something `built`
/// read the language of the site or of a page, which is then English.
fn no_language(renderer: &Renderer, built: &[Built]) -> Option<Problem> {
    let settings = renderer.settings();
    if !settings.languages().is_empty() || settings.language_told {
        return None;
    }
    let language = Key::Setting(renderer.name_id("language"));
    let read = built
        .iter()
        .any(|built| built.reads.keys().contains(&language));
    read.then(|| Problem::nowhere(Kind::NoLanguage))
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
    /// know, and the addresses published added to `published.txt`.
    pub remembers: bool,
    /// Why the build doesn't hold the site, where it doesn't.
    pub unheld: Option<Problem>,
    /// The build's stages timed so far.
    pub clock: Clock,
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
        pages_left_out,
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
        mut clock,
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
    clock.lap(Stage::Writing);
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
    let mut photographs = Vec::new();
    if let Some(staging) = &staging
        && !stops(&problems)
    {
        let made = crate::album::develop(&options.site, pictures, &developed, staging, progress);
        (copies.made, copies.from) = (made.made, made.from);
        files += made.published;
        problems.extend(made.problems);
        photographs = made.timed;
        timings::slowest_first(&mut photographs);
        clock.lap(match developed.is_empty() {
            true => Stage::Writing,
            false => Stage::Copies,
        });
    }
    if let Some(staging) = &staging
        && !stops(&problems)
    {
        let wanted = renderer.cards_wanted(&asked);
        let drew = crate::cards::keep::publish(&options.site, &wanted, staging, progress);
        cards.drawn = drew.drawn;
        files += drew.published;
        problems.extend(drew.problems);
        clock.lap(match wanted.is_empty() {
            true => Stage::Writing,
            false => Stage::Cards,
        });
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
    let mut kept_published = crate::published::Kept::default();
    let published = match staging {
        Some(mut staging) if !stops(&problems) => {
            clock.lap(Stage::Writing);
            // A command that doesn't end well publishes nothing.
            let ran = match after {
                true => {
                    let settings = renderer.settings();
                    let ran =
                        crate::hooks::after(settings, options, &mut staging, progress, serving);
                    if settings.text(crate::hooks::AFTER).is_some() {
                        clock.lap(Stage::After);
                    }
                    ran
                }
                false => Ok((None, Vec::new())),
            };
            let put = ran.and_then(|ran| staging.finish().map(|()| ran));
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
        // A run with the drafts in publishes them only to be looked at.
        if remembers && !options.drafts {
            let now = crate::published::now(
                &options.site,
                site,
                renderer.sources(),
                renderer.dates(),
                renderer.assets(),
                renderer.feeds(),
                &held_back,
            );
            match site.published.keep(&options.site, &now, None) {
                Ok(written) => {
                    written.re_place(&mut problems, renderer.sources());
                    kept_published = written.kept;
                }
                Err(error) => problems.push(Problem::nowhere(Kind::CantWrite {
                    path: crate::published::FILE.into(),
                    reason: Reason::of(&error),
                })),
            }
        }
    }
    drop(asked);
    let fetched = match published {
        Some(_) => crate::fetched::Ages::of(site, &renderer.today()),
        None => crate::fetched::Ages::default(),
    };
    clock.lap(Stage::Writing);
    Outcome {
        held: Held::Renderer(renderer),
        problems,
        quiet,
        published,
        javascript,
        record: Record::default(),
        included,
        pages_left_out,
        copies,
        cards,
        hooks,
        fetched,
        held_back,
        kept: kept_published,
        waited: Duration::ZERO,
        timings: Timings {
            stages: clock.stages(),
            photographs,
            ..Timings::default()
        },
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
    let whole = crate::platform::whole_to_be(&publish::lexically_normal(&whole));
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
    let began = std::time::Instant::now();
    let site = renderer.site();
    let feeds = renderer.feeds();
    templates::timed::open();
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
    let mut timed = timings::Output {
        address: address.as_str().to_owned(),
        tally: templates::timed::close(),
        layout: rendered.layout,
        ..timings::Output::default()
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
            };
            let checking = std::time::Instant::now();
            checked = Some(linked.check(renderer, page, &html));
            timed.links = checking.elapsed();
            sized = sizes_a_picture(&html);
        }
        match writes {
            Writes::Nowhere => {}
            Writes::Into(staging) => {
                let saving = std::time::Instant::now();
                match staging.write(&file, html.as_bytes()) {
                    Ok(()) => written = true,
                    Err(problem) => problems.push(problem),
                }
                timed.saving = saving.elapsed();
            }
            Writes::Later => held = Some(html),
        }
    }
    timed.whole = began.elapsed();
    Built {
        output,
        timed,
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

/// `https://example.org` from `https://example.org/blog/`.
fn site_origin(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return String::new();
    };
    let host = rest.split('/').next().unwrap_or_default();
    format!("{scheme}://{host}")
}
