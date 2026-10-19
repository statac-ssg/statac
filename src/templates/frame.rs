//! What the build of one output has gathered so far, kept for the thread
//! building it. The engine calls Statac's objects, filters and functions
//! with no way to hand them anything of Statac's, so they find the output
//! they are part of here. A page's writing built while a template reads it
//! is an output of its own, with a frame of its own on top.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::Arc;

use minijinja::{Captured, State, Value};

use super::guard::{Failed, Refusal};
use super::render::Shared;
use crate::assets::PageScripts;
use crate::problem::{Kind, Problem, Stopped};
use crate::reads::{Asked, Key, Reads};
use crate::site::{DataId, Id, LanguageId, PageId};
use crate::source::Place;

pub(super) struct Frame {
    pub shared: Arc<Shared>,
    /// The page being built, or whose writing is, or the page speaking for
    /// the list whose feed is; none for the feed of a list no page speaks
    /// for.
    pub page: Option<PageId>,
    /// The language of what is built: its page's, or for a feed its list's.
    /// Dates, words and `site` answer in it.
    pub language: LanguageId,
    /// Which of the page's list's pages is being built, counted from 1.
    pub number: usize,
    /// The feed being built, where it is one.
    pub feed: Option<crate::feeds::Feed>,
    /// Whether code is shown without colour: in a feed, as a feed reader
    /// loads no stylesheet, and in writing built for one.
    pub colourless: bool,
    /// Whether what is built is the page left at one of the page's old
    /// addresses.
    pub redirect: bool,
    pub reads: Reads,
    /// What it asked of its own page and of the site, by name.
    pub asked: Asked,
    /// Problems met along the way that didn't stop the template, such as a
    /// snippet call in the writing that couldn't be built.
    pub problems: Vec<Problem>,
    /// Lookups that found nothing, most recent last.
    pub misses: Vec<Miss>,
    /// What the last guard to refuse a value found.
    pub refusal: Option<Refusal>,
    /// The innermost of the engine's own filters, tests and functions to
    /// stop, and what it was handed.
    pub failed: Option<Failed>,
    /// A problem Statac's own code raised, to be told in place of the
    /// engine's error it stopped the template with.
    pub raised: Option<Kind>,
    /// What the innermost snippet that failed was handed.
    pub handed: Option<Handed>,
    /// The snippets being built, outermost first.
    pub snippets: Vec<String>,
    pub scripts: PageScripts,
    /// Where each text the output was handed is written, by its bytes'
    /// address and length, the text kept so no other takes its address.
    pub origins: HashMap<(usize, usize), (Value, Place)>,
    /// Where the writing makes the call, or shows the picture, whose
    /// snippet or piece is being built.
    pub calling: Option<Place>,
    /// The pictures told as shown with no description, each told once.
    pub undescribed: Vec<String>,
    /// How many search boxes what is built has asked for, those of its
    /// own page's writing counted once it shows that whole.
    pub search_boxes: usize,
    /// How many search boxes the output had asked for when it first showed
    /// its own page's writing whole.
    pub boxes_before_writing: Option<usize>,
    /// The photographs whose copies what is built names, as each was
    /// named.
    pub developed: Vec<crate::album::Shown>,
    /// Where the first block of code coloured names its language, in each
    /// page's writing what is built shows, whole or as its excerpt.
    pub coloured: Vec<Place>,
    /// Whether what is built shows any code coloured, its own or in the
    /// writing of a page it shows, whole or as its excerpt.
    pub colours: bool,
    pub making: Making,
    /// The template the output is built with, by its index among the
    /// files; none for a page's writing, which is built for its page's.
    pub layout: Option<usize>,
    /// The steps the engine took in the runs for the output that have
    /// ended, its page's writing counted.
    pub steps: u64,
    /// The steps each run still going had taken when it called the run
    /// inside it, outermost first.
    pub running: Vec<u64>,
    /// Whether the output's steps were told spent.
    pub spent: bool,
    /// Whether what is built asked the page published where it is for its
    /// writing, whole or as its excerpt.
    pub shows_writing: bool,
    /// How many pages of lists of the whole site what is built reached.
    pub walked: u64,
    /// The links the snippets built for it wrote in their own text.
    pub snipped: Vec<crate::links::Snipped>,
}

/// What a frame is the build of.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(super) enum Making {
    /// A page, a feed or a redirect, from its template.
    Output,
    /// A page's writing.
    Writing,
    /// The texts of a page's card.
    Card,
}

/// Where each text in a value is written, by the text's address and
/// length (see [`address`]).
pub(super) type Origins = HashMap<(usize, usize), Place>;

/// A snippet's name and what one call handed it.
#[derive(Clone, Debug)]
pub(super) struct Handed {
    pub snippet: String,
    pub values: Vec<(String, Value)>,
}

/// A lookup that found nothing: the name, and the page or the part of
/// `data/` it was asked of where it was asked of one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Miss {
    pub page: Option<PageId>,
    pub data: Option<InData>,
    pub name: String,
}

/// Where in `data/` a lookup found nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum InData {
    File(DataId),
    /// A folder, by its path in the site.
    Folder(String),
}

impl Frame {
    pub fn new(shared: Arc<Shared>, page: Option<PageId>) -> Frame {
        let language = page.map_or(LanguageId::from_index(0), |page| {
            shared.site.pages.get(page).language
        });
        Frame {
            shared,
            page,
            language,
            number: 1,
            feed: None,
            colourless: false,
            redirect: false,
            reads: Reads::default(),
            asked: Asked::default(),
            problems: Vec::new(),
            misses: Vec::new(),
            refusal: None,
            failed: None,
            raised: None,
            handed: None,
            snippets: Vec::new(),
            scripts: PageScripts::default(),
            origins: HashMap::new(),
            calling: None,
            undescribed: Vec::new(),
            search_boxes: 0,
            boxes_before_writing: None,
            developed: Vec::new(),
            coloured: Vec::new(),
            colours: false,
            making: Making::Output,
            layout: None,
            steps: 0,
            running: Vec::new(),
            spent: false,
            shows_writing: false,
            walked: 0,
            snipped: Vec::new(),
        }
    }

    /// The steps the output has taken so far.
    fn taken(&self) -> u64 {
        self.steps + self.running.iter().sum::<u64>()
    }

    /// The page whose own address what is built is published at: none for
    /// a feed, a list's later page or the page left at an old address.
    pub fn home(&self) -> Option<PageId> {
        self.page
            .filter(|_| self.number == 1 && self.feed.is_none() && !self.redirect)
    }
}

thread_local! {
    static FRAMES: RefCell<Vec<Frame>> = const { RefCell::new(Vec::new()) };
}

pub(super) fn push(frame: Frame) {
    FRAMES.with_borrow_mut(|frames| frames.push(frame));
}

pub(super) fn pop() -> Frame {
    FRAMES
        .with_borrow_mut(Vec::pop)
        .expect("a frame pushed for every one popped")
}

/// Runs `f` on the frame of the output being built. None outside a build,
/// where nothing is being recorded.
pub(super) fn with<R>(f: impl FnOnce(&mut Frame) -> R) -> Option<R> {
    FRAMES.with_borrow_mut(|frames| frames.last_mut().map(f))
}

thread_local! {
    /// Whether what is read is being worked out for more than the output
    /// being built, see [`unrecorded`].
    static UNRECORDED: Cell<bool> = const { Cell::new(false) };
}

/// Runs `f` with nothing it reads or looks up recorded for the output
/// being built: for what is worked out once and answered to every output
/// that asks, each of which records the whole of it under one key.
pub(super) fn unrecorded<R>(f: impl FnOnce() -> R) -> R {
    let before = UNRECORDED.replace(true);
    let made = f();
    UNRECORDED.set(before);
    made
}

/// Records that the output being built read `key`.
pub(super) fn read(key: Key) {
    if !UNRECORDED.get() {
        with(|frame| frame.reads.read(key));
    }
}

/// Notes that the output being built asked `page`, as its list's page
/// `number`, for `name`, where that is the page it is built for.
pub(super) fn ask_page(page: PageId, number: usize, name: &str) {
    if !UNRECORDED.get() {
        with(|frame| {
            if frame.page == Some(page) && frame.number == number {
                let name = frame.shared.name_id(name);
                frame.asked.ask_page(name);
            }
        });
    }
}

/// Notes that the output being built asked the site, as it answers in
/// `language`, for `name`, where that is the output's language.
pub(super) fn ask_site(language: LanguageId, name: &str) {
    if !UNRECORDED.get() {
        with(|frame| {
            if frame.language == language {
                let name = frame.shared.name_id(name);
                frame.asked.ask_site(name);
            }
        });
    }
}

/// Notes that the output being built reached `file` under `data/`.
pub(super) fn reach(file: DataId) {
    if !UNRECORDED.get() {
        with(|frame| frame.asked.reach(file));
    }
}

/// Notes that the output being built reached one more page of a list of
/// the whole site, where it is the output's own doing.
pub(super) fn walked() {
    if !UNRECORDED.get() {
        with(|frame| frame.walked += 1);
    }
}

/// Notes that the output being built names the copies of the photograph
/// `id`, at `place` where the writing does, with `full` its copy at its
/// own width, so they are made.
pub(super) fn develops(id: crate::site::PictureId, place: Option<Place>, full: bool) {
    with(|frame| {
        frame
            .developed
            .push(crate::album::Shown { id, place, full })
    });
}

pub(super) fn missed(page: Option<PageId>, name: &str) {
    if UNRECORDED.get() {
        return;
    }
    with(|frame| {
        frame.misses.push(Miss {
            page,
            data: None,
            name: name.to_owned(),
        })
    });
}

pub(super) fn missed_in_data(data: InData, name: &str) {
    with(|frame| {
        frame.misses.push(Miss {
            page: None,
            data: Some(data),
            name: name.to_owned(),
        })
    });
}

/// Text written at `place`, as a value that remembers where: a filter
/// handed it, or a copy of it, finds the place with [`origin`].
pub(super) fn sourced(text: &str, place: Place) -> Value {
    if text.is_empty() {
        return Value::from("");
    }
    let value = Value::from(Arc::<str>::from(text));
    with(|frame| {
        frame
            .origins
            .insert(address(&value), (value.clone(), place))
    });
    value
}

/// Where a text was written, where it is one handed out by [`sourced`] or
/// as a data file's.
pub(super) fn origin(value: &Value) -> Option<Place> {
    let key = value.as_str().map(|_| address(value))?;
    let found = FRAMES.with_borrow(|frames| {
        frames
            .iter()
            .rev()
            .find_map(|frame| frame.origins.get(&key).map(|(_, place)| *place))
    });
    found.or_else(|| {
        let shared = shared()?;
        shared
            .data_values
            .iter()
            .filter_map(|made| made.get())
            .find_map(|(_, origins)| origins.get(&key).copied())
    })
}

/// A text value's bytes, by address and length. A text kept apart, as
/// [`sourced`] keeps it, is never moved or freed while held, so only copies
/// of that value share its address.
pub(super) fn address(value: &Value) -> (usize, usize) {
    let text = value.as_str().unwrap_or_default();
    (text.as_ptr() as usize, text.len())
}

/// The site of the output being built.
pub(super) fn shared() -> Option<Arc<Shared>> {
    with(|frame| Arc::clone(&frame.shared))
}

/// The language of the output being built; the main language outside a
/// build.
pub(super) fn language() -> LanguageId {
    with(|frame| frame.language).unwrap_or(LanguageId::from_index(0))
}

/// Stops the template with `kind`: the error the engine is handed carries
/// no words of its own, and the problem told is `kind`, placed where the
/// engine says the template stopped.
pub(super) fn raise(kind: Kind) -> minijinja::Error {
    with(|frame| frame.raised = Some(kind));
    minijinja::Error::new(minijinja::ErrorKind::InvalidOperation, RAISED)
}

/// The words of an error Statac raised itself.
pub(super) const RAISED: &str = "raised by Statac";

/// The words of an error for a problem already told, when a template was
/// read, which is not told again for every page.
pub(super) const TOLD: &str = "told already";

pub(super) fn told() -> minijinja::Error {
    minijinja::Error::new(minijinja::ErrorKind::InvalidOperation, TOLD)
}

/// Starts a run of the template `file` for the output being built, called
/// by the run `calling` where one calls it. Stopped where the output has
/// no steps left.
///
/// The engine counts each run's steps afresh, so the output's are counted
/// here: a run is stopped by the engine where it alone takes them all,
/// and by Statac where it starts with none left or ends with too many.
pub(super) fn begin(file: usize, calling: Option<&State>) -> Result<(), minijinja::Error> {
    let called = calling
        .and_then(State::fuel_levels)
        .map_or(0, |(taken, _)| taken);
    let spent = with(|frame| {
        frame.running.push(called);
        frame.spent || frame.taken() >= frame.shared.templates.steps
    });
    if spent != Some(true) {
        return Ok(());
    }
    let stopped = stop(file);
    with(|frame| frame.running.pop());
    Err(stopped)
}

/// Ends the run of `file` that [`begin`] started, its steps added to the
/// output's: what it built, or why it stopped. Stopped itself where the
/// output has taken too many steps.
pub(super) fn end(
    file: usize,
    built: Result<(Captured, String), minijinja::Error>,
) -> Result<Result<String, minijinja::Error>, minijinja::Error> {
    let out_of_fuel = matches!(&built, Err(e) if e.kind() == minijinja::ErrorKind::OutOfFuel);
    let taken = match &built {
        Ok((captured, _)) => captured.state().fuel_levels().map_or(0, |(taken, _)| taken),
        Err(_) => 0,
    };
    let over = with(|frame| {
        frame.steps += taken;
        // Told where it stopped, as the engine places it.
        frame.spent |= out_of_fuel;
        let over = frame.taken() > frame.shared.templates.steps;
        frame.running.pop();
        over
    });
    match built {
        Ok(_) if over == Some(true) => Err(stop(file)),
        built => Ok(built.map(|(_, text)| text)),
    }
}

/// Stops the template for the output's steps running out, told once.
fn stop(file: usize) -> minijinja::Error {
    let told_already = with(|frame| std::mem::replace(&mut frame.spent, true));
    match told_already {
        Some(true) => told(),
        _ => {
            let kind = with(|frame| ran_on(frame, file)).expect("a frame to have run in");
            raise(kind)
        }
    }
}

/// The output's steps run out, while the template `file` was running.
/// Told of the template the output is built with, so alike for every page
/// built with it; a page's writing is told of its page's layout.
pub(super) fn ran_on(frame: &mut Frame, file: usize) -> Kind {
    let shared = Arc::clone(&frame.shared);
    let templates = &shared.templates;
    let layout = frame.layout.or_else(|| {
        let page = frame.page?;
        frame.reads.read(Key::Field(page, shared.name_id("layout")));
        frame.reads.read(Key::Field(page, shared.name_id("type")));
        let p = shared.site.pages.get(page);
        let chosen = super::layout::choose(templates, &shared.sources, &shared.site, p).ok()?;
        templates.by_name.get(&chosen.name).copied()
    });
    let path = shared
        .sources
        .path(templates.files[layout.unwrap_or(file)].source);
    Kind::TemplateStopped {
        reason: Stopped::RanOn {
            template: path.to_string_lossy().replace('\\', "/"),
            steps: templates.steps,
        },
    }
}
