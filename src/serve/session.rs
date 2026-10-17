//! A site kept built while its author works on it.
//!
//! A session holds its last round: every output with what it added to the
//! build. Each round reads the site whole, as a build does, and asks of
//! everything an output of the last round read whether the site read now
//! holds the same thing, saying the same, written in the same place. An
//! output is kept only where every answer is yes; every other output is
//! built by the code a build uses, and the round is concluded over all of
//! them by the code a build uses, so the folder a session leaves is the
//! folder a build of the same files leaves.
//!
//! An id is an index into one reading's arenas, so nothing is compared by
//! id: each thing read is known by what it names, and a kept output's ids
//! and places are carried over to the new reading by those names. Whatever
//! can't be carried over, or the session has no way to account for, is
//! built again.

mod print;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rayon::prelude::*;

use crate::album::{KEPT, Progress};
use crate::build::{self, Built, Mode, Options, Output, Plan, Publishing, Remembered, Writes};
use crate::content::words::PageWord;
use crate::hooks::Before;
use crate::new::update::Starter;
use crate::problem::{Kind, Problem};
use crate::publish::{self, Left, Occupant, Staging};
use crate::reads::{Key, Reads, Translation};
use crate::stamp::{self, Pass};
use print::{Prints, Reading, Taken};

/// The session's folder in the site's `.statac`, where no other session
/// of the site holds it.
const FOLDER: &str = "serve";

/// A site kept built in a folder of its own.
pub struct Session {
    options: Options,
    folder: PathBuf,
    /// Held while the session runs, so that no other takes `folder`.
    _claim: Option<std::fs::File>,
    /// The last round that built the site; none before one has, and after
    /// one that stopped while writing, which the next round builds whole.
    last: Option<Last>,
    /// What each file published as it is was found to hold.
    remembered: Remembered,
    /// What became of the command the site runs before a build, as the
    /// session started.
    before: Before,
    /// Whether the site names files the command it runs before a build is
    /// run again for.
    watches: bool,
    /// Whether the site sets a command to run on a built site.
    sets_after: bool,
    /// The command run on the site the folder holds, where one was.
    after: Option<String>,
    /// The note on newer starter files, as worked out since the last
    /// save of one of them.
    newer: Option<Option<Problem>>,
    took: Took,
}

/// What one round did.
pub struct Round {
    /// What the round read and found. Its record of reads is left empty:
    /// the session holds what each output read.
    pub outcome: build::Outcome,
    /// Files written and removed, from the top of the built site.
    pub written: Vec<String>,
    pub removed: Vec<String>,
    /// Outputs built this round, and how many the site has.
    pub built: usize,
    pub outputs: usize,
    /// Whether every page written differs from the one it replaces only
    /// by the names of stylesheets the round published under new names.
    pub restyled: bool,
    /// The stamps and the ids written into the site's own files before it
    /// was built.
    pub stamps: Pass,
    /// Each file of the site's own that the command it runs before a
    /// build wrote or removed this round, with what it now holds.
    pub hooked: Vec<(String, Option<blake3::Hash>)>,
    /// The pages there is something to say of.
    pub noted: Vec<Noted>,
    /// The site as the round read it, where it built.
    pub read: Option<Read>,
}

/// A site as a round read it, to be asked about a page afterwards.
pub struct Read {
    pub renderer: crate::templates::Renderer,
    /// The page each built file is, and which of its list's pages, counted
    /// from 1.
    pub pages: HashMap<String, (crate::site::PageId, usize)>,
}

/// A page there is something to say of to whoever is looking at it: one
/// a build leaves out, or one with problems of its own.
pub struct Noted {
    /// The file it is published as, from the top of the built site.
    pub file: String,
    /// Its address, without the path the site is published under.
    pub address: String,
    pub title: Option<String>,
    pub draft: bool,
    /// The day it waits for, where that is after today.
    pub waiting: Option<String>,
    /// The problems placed in its file, and those building it told.
    pub problems: Vec<Problem>,
}

/// How long each part of a round took.
#[derive(Copy, Clone, Default, Debug)]
pub struct Took {
    /// Reading the settings to find whether the site's command runs
    /// before this round, and running it where it does.
    pub before: Duration,
    /// Reading the site.
    pub reading: Duration,
    /// Finding which outputs of the last round still stand.
    pub comparing: Duration,
    /// Building the others.
    pub building: Duration,
    /// Concluding over them all.
    pub concluding: Duration,
    /// Writing what changed.
    pub writing: Duration,
}

/// A round that built the site, as the next round finds it.
struct Last {
    reading: Reading,
    /// The fingerprint of everything an output read, and of what each is
    /// built from besides, as taken that round.
    prints: Prints,
    /// What the session can't account for output by output: where this
    /// differs, every output is built.
    whole: u64,
    built: Vec<Built>,
    /// What each output is built from besides what it read.
    own: Vec<u64>,
    /// What each output's file holds, by the file.
    hashes: HashMap<String, blake3::Hash>,
    /// Every file the folder holds that is no output's.
    others: Vec<String>,
    /// What the files there were copied from.
    left: Left,
}

impl Session {
    /// Builds the whole site into its folder, having run the command the
    /// site runs before a build.
    pub fn start(options: &Options) -> (Session, Round) {
        Session::start_saying(options, &mut || {})
    }

    /// [`Session::start`], calling `waiting` where the round waits for
    /// another build of the site to end.
    pub fn start_saying(options: &Options, waiting: &mut dyn FnMut()) -> (Session, Round) {
        // Two sessions building into one folder would each serve what the
        // other wrote.
        let (name, claim) = match build::a_folder(&options.site) {
            Ok(()) => publish::claim(&options.site, FOLDER),
            Err(_) => (FOLDER.to_owned(), None),
        };
        let folder = match options.site == Path::new(".") {
            true => Path::new(KEPT).join(name),
            false => options.site.join(KEPT).join(name),
        };
        let mut session = Session {
            options: Options {
                mode: Mode::Build,
                out: Some(folder.clone()),
                ..options.clone()
            },
            folder,
            _claim: claim,
            last: None,
            remembered: Remembered::default(),
            before: Before::Unset,
            watches: false,
            sets_after: false,
            after: None,
            newer: None,
            took: Took::default(),
        };
        let mut round = session.round(None, waiting);
        session.before = round.outcome.hooks.before;
        // Said by whoever says how the session began, not as a build's.
        if session.before == Before::Ran {
            round.outcome.hooks.before = Before::Unset;
        }
        (session, round)
    }

    /// Whether the command the site runs before a build was run as the
    /// session started.
    pub fn ran_before(&self) -> bool {
        self.before == Before::Ran
    }

    /// Whether the site names files that command is run again for.
    pub fn watches(&self) -> bool {
        self.watches
    }

    /// Whether the site sets a command to run on a built site.
    pub fn sets_after(&self) -> bool {
        self.sets_after
    }

    /// Whether that command has run on the site the folder holds.
    pub fn ran_after(&self) -> bool {
        self.after.is_some()
    }

    /// Builds the site as it is on `today`, from the next round on.
    pub fn set_today(&mut self, today: Option<crate::date::Date>) {
        self.options.today = today;
    }

    /// The folder the session's site is in.
    pub fn folder(&self) -> &Path {
        &self.folder
    }

    /// How long each part of the last round took.
    pub fn took(&self) -> Took {
        self.took
    }

    /// Reads the site again, writes into its own files the dates and ids
    /// they are due, and builds what changed.
    pub fn rebuild(&mut self) -> Round {
        self.round(Some(&[]), &mut || {})
    }

    /// [`Session::rebuild`], for a save of `changed`, files of the site by
    /// their paths from its folder: the command the site runs before a
    /// build is run first where it is to be run again for any of them.
    pub fn rebuild_for(&mut self, changed: &[String]) -> Round {
        self.round(Some(changed), &mut || {})
    }

    /// [`Session::rebuild_for`], calling `waiting` where the round waits
    /// for another build of the site to end.
    pub fn rebuild_for_saying(&mut self, changed: &[String], waiting: &mut dyn FnMut()) -> Round {
        self.round(Some(changed), waiting)
    }

    /// One round, for a save of `changed`, or for none as the session
    /// starts, when the command the site runs before a build always runs.
    /// The round holds the site as a build does, for as long as it runs.
    fn round(&mut self, changed: Option<&[String]>, waiting: &mut dyn FnMut()) -> Round {
        let at = self.options.site.clone();
        if let Err(problem) = build::a_folder(&at) {
            return Round::of_nothing(build::Outcome::unread(problem));
        }
        let (hold, unheld) = publish::hold(&at, waiting);
        let mut round = self.held(changed, &at, unheld);
        round.outcome.waited = hold.waited;
        round
    }

    /// [`Session::round`] once it holds the site, or knows why it can't.
    fn held(&mut self, changed: Option<&[String]>, at: &Path, unheld: Option<Problem>) -> Round {
        let began = Instant::now();
        // A starter file changed, or what changed isn't known.
        if changed.is_none_or(|changed| {
            changed.is_empty() || changed.iter().any(|path| Starter::OWN.reaches(path))
        }) {
            self.newer = None;
        }
        let (settings, _) = crate::config::read(at, &mut crate::source::Sources::default());
        let sets = settings.text(crate::hooks::BEFORE).is_some();
        let runs = match changed {
            Some(changed) => sets && crate::hooks::watched(&settings, changed),
            None => true,
        };
        if !runs {
            return self.built(began.elapsed(), unheld);
        }
        // What the command writes into the site is this round's to build,
        // and no save of the author's.
        let was = if sets { marks(at) } else { Marks::new() };
        let ran = crate::hooks::before(&self.options, Progress::Silent, true);
        let hooked = if sets {
            written_between(at, &was, &marks(at))
        } else {
            Vec::new()
        };
        let mut round = match ran {
            Ok(before) => {
                let mut round = self.built(began.elapsed(), unheld);
                round.outcome.hooks.before = before;
                round
            }
            Err(stopped) => {
                let (sources, problem) = *stopped;
                let mut outcome = build::Outcome::stopped_at(sources, problem);
                outcome.problems.extend(unheld);
                Round::of_nothing(outcome)
            }
        };
        round.hooked = hooked;
        round
    }

    /// Reads the site and builds what changed since the last round, the
    /// round having taken `before` so far; `unheld` says why the round
    /// doesn't hold the site.
    fn built(&mut self, before: Duration, unheld: Option<Problem>) -> Round {
        let options = &self.options;
        let at = options.site.as_path();
        let mut took = Took {
            before,
            ..Took::default()
        };
        let mut clock = Instant::now();
        let mut lap = |part: &mut Duration| {
            *part = clock.elapsed();
            clock = Instant::now();
        };
        let mut prepared = build::prepared(at, &options.run());
        let newer = (self.newer)
            .get_or_insert_with(|| crate::new::update::newer_starter(at, &Starter::OWN));
        prepared.problems.extend(newer.clone());
        let fixed = options.today.is_some();
        let (stamps, mut prepared) = stamp::write(at, &options.run(), prepared, fixed);
        let plan = Plan::of(&prepared.renderer, &mut prepared.problems).noting();
        lap(&mut took.reading);

        let reading = Reading::of(prepared.renderer.share());
        let renderer = &reading.renderer;
        let settings = renderer.settings();
        let after = settings.text(crate::hooks::AFTER);
        let after = after.map(|line| line.value.to_owned());
        self.sets_after = after.is_some();
        self.watches = !settings.texts(crate::hooks::WATCHES).is_empty();
        // The command ran on the folder as an earlier round built it whole,
        // and what it left there stays. Another command, or none, is for a
        // folder built whole again, once the settings read without error.
        if after != self.after && !build::stops(&prepared.problems) {
            self.last = None;
        }
        let prints = Prints::new(&reading);
        let taken = Taken {
            reading: &reading,
            at,
            prints: &prints,
        };
        let whole = taken.whole(&plan, options);
        let outputs = plan.outputs.par_iter();
        let own: Vec<u64> = outputs.map(|&output| taken.own(output)).collect();
        let kept: Vec<Option<Built>> = match &self.last {
            Some(last) if last.whole == whole => carry(last, taken, &plan, &own),
            _ => plan.outputs.iter().map(|_| None).collect(),
        };
        lap(&mut took.comparing);

        let fresh = kept.iter().filter(|kept| kept.is_none()).count();
        let mut built: Vec<Built> = plan
            .outputs
            .par_iter()
            .zip(kept)
            .map(|(&output, kept)| match kept {
                Some(kept) => kept,
                None => {
                    let built = build::build_output(renderer, &plan.linked, output, Writes::Later);
                    // Taken now, as what the output was built from.
                    for &key in built.reads.keys() {
                        taken.of(key);
                    }
                    built
                }
            })
            .collect();
        lap(&mut took.building);

        let problems = std::mem::take(&mut prepared.problems);
        let remembered = Some(&self.remembered);
        let mut concluded = build::conclude(renderer, at, &plan, &built, problems, remembered);
        lap(&mut took.concluding);

        // An output built to the bytes the folder holds already is left
        // where it is, as one kept is.
        let renamed = match &self.last {
            Some(last) => renamed_styles(&last.reading, &reading),
            None => Vec::new(),
        };
        let held = self.last.as_ref().map(|last| &last.hashes);
        let hashed: Vec<Option<(blake3::Hash, bool)>> = match concluded.stopped() {
            true => Vec::new(),
            false => (built.par_iter_mut())
                .map(|built| {
                    let html = built.html.as_ref()?;
                    let hash = blake3::hash(html.as_bytes());
                    let before = held.and_then(|held| held.get(&built.file));
                    if before == Some(&hash) {
                        built.html = None;
                        built.written = true;
                        return None;
                    }
                    let restyled = !is_page(&built.file)
                        || before.is_some_and(|before| {
                            !renamed.is_empty()
                                && *before == blake3::hash(&unstyled(html, &renamed))
                        });
                    Some((hash, restyled))
                })
                .collect(),
        };
        let restyled = hashed.iter().flatten().all(|(_, restyled)| *restyled);
        // What the folder holds that this round may write anew or remove:
        // every file but the outputs left where they are.
        let mine: Option<Vec<String>> = self.last.as_ref().map(|last| {
            let stay: HashSet<&str> = (built.iter())
                .filter(|built| built.html.is_none() && built.written)
                .map(|built| built.file.as_str())
                .collect();
            let outputs = last.built.iter().map(|built| &built.file);
            let outputs = outputs.filter(|file| !stay.contains(file.as_str()));
            last.others.iter().chain(outputs).cloned().collect()
        });

        let into = Arc::new(Mutex::new(None));
        // Asked before every round that writes: either may have been put
        // there as a link since the last.
        let linked = match concluded.stopped() {
            true => None,
            false => link_in_the_way(at, &self.folder),
        };
        if let Some(problem) = linked {
            concluded.problems.push(problem);
            // Whatever the folder holds now is none of the session's.
            self.last = None;
        }
        let staging = match (concluded.stopped(), &self.last, mine) {
            (true, ..) => None,
            (false, Some(last), Some(mine)) => {
                let into = Arc::clone(&into);
                Some(publish::resume(at, &self.folder, mine, &last.left, into))
            }
            (false, ..) => match publish::begin(at, &self.folder) {
                Ok(staging) => Some(staging.leaving(at, Arc::clone(&into))),
                Err(problem) => {
                    concluded.problems.push(problem);
                    None
                }
            },
        };
        let whole_site = self.last.is_none();
        let in_place = staging.is_some() && !whole_site;
        let mut files = 0;
        if let Some(staging) = &staging {
            let (told, (held, more)) = rayon::join(
                || write(&mut built, staging),
                || build::publish_files(renderer, at, staging, &plan.held),
            );
            files = held;
            concluded.problems.extend(told);
            concluded.problems.extend(more);
        }
        let publishing = Publishing {
            staging,
            files,
            progress: Progress::Silent,
            // What earlier builds kept is a build's to tidy, and the
            // first round is one.
            tidies: whole_site,
            after: whole_site,
            serving: true,
            remembers: false,
            unheld,
        };
        let mut outcome = build::publish(options, prepared, &plan, &built, concluded, publishing);
        let named = crate::hooks::names_output(renderer.settings(), renderer.sources(), options);
        outcome.problems.extend(named);
        if !whole_site && after.is_some() && !outcome.stopped() {
            let unmade = crate::hooks::still_unmade(renderer.settings(), &self.folder);
            outcome.problems.extend(unmade);
        }
        if whole_site && outcome.published.is_some() {
            self.after = after;
        }
        let noted = noted(renderer, &built, &outcome.problems);
        let pages = built.iter().filter_map(|built| {
            let page = match built.output {
                Output::Page(page) => (page, 1),
                Output::Later(list, number) => (renderer.site().lists.get(list).page?, number),
                _ => return None,
            };
            Some((built.file.clone(), page))
        });
        let pages = pages.collect();
        let left: Option<Left> = into.lock().expect("never poisoned").take();
        let mut round = Round {
            outcome,
            written: Vec::new(),
            removed: Vec::new(),
            built: fresh,
            outputs: built.len(),
            restyled,
            stamps,
            hooked: Vec::new(),
            noted,
            read: Some(Read {
                renderer: renderer.clone(),
                pages,
            }),
        };
        match left.filter(|_| round.outcome.published.is_some()) {
            Some(mut left) => {
                left.files.sort_unstable();
                left.written.sort_unstable();
                left.removed.sort_unstable();
                round.written = match whole_site {
                    true => left.files.clone(),
                    false => std::mem::take(&mut left.written),
                };
                // The marker says whose the folder is, and is no part of
                // the site.
                round.written.retain(|file| file != publish::MARKER);
                round.removed = std::mem::take(&mut left.removed);
                // What the round wrote holds the outputs it wrote, which
                // are none of the others.
                let mut hashes = match self.last.take() {
                    Some(mut last) => {
                        let hashes = std::mem::take(&mut last.hashes);
                        // Every file the site was read from, let go of
                        // while the round is told.
                        rayon::spawn(move || drop(last));
                        hashes
                    }
                    None => HashMap::new(),
                };
                for file in &round.removed {
                    hashes.remove(file);
                }
                let mut outputs: HashSet<&str> = HashSet::new();
                for (built, hashed) in built.iter().zip(&hashed) {
                    if let Some((hash, _)) = hashed {
                        outputs.insert(built.file.as_str());
                        hashes.insert(built.file.clone(), *hash);
                    }
                }
                let others = std::mem::take(&mut left.files).into_iter();
                let others = others.filter(|file| !outputs.contains(file.as_str()));
                let others = others.collect();
                self.last = Some(Last {
                    reading,
                    prints,
                    whole,
                    built,
                    own,
                    hashes,
                    others,
                    left,
                });
            }
            // A round that stops before it writes leaves the folder, and
            // what the session knows of it, as they were. One that stops
            // while writing where the site stands may leave anything.
            None if in_place => self.last = None,
            None => {}
        }
        lap(&mut took.writing);
        self.took = took;
        round
    }
}

impl Round {
    /// A round that built nothing, for what `outcome` tells.
    fn of_nothing(outcome: build::Outcome) -> Round {
        Round {
            outcome,
            written: Vec::new(),
            removed: Vec::new(),
            built: 0,
            outputs: 0,
            restyled: false,
            stamps: Pass::default(),
            hooked: Vec::new(),
            noted: Vec::new(),
            read: None,
        }
    }
}

/// The pages among `built` there is something to say of, `problems` being
/// every problem of the round.
fn noted(
    renderer: &crate::templates::Renderer,
    built: &[Built],
    problems: &[Problem],
) -> Vec<Noted> {
    let site = renderer.site();
    let dates = renderer.dates();
    let mut placed: HashMap<crate::source::SourceId, Vec<&Problem>> = HashMap::new();
    for problem in problems {
        if let Some(place) = problem.place {
            placed.entry(place.source).or_default().push(problem);
        }
    }
    let noted = built.iter().filter_map(|built| {
        let (id, address) = match built.output {
            Output::Page(page) => (page, &site.pages.get(page).address),
            Output::Later(list, number) => {
                let list = site.lists.get(list);
                (list.page?, list.later_address(number)?)
            }
            _ => return None,
        };
        let page = site.pages.get(id);
        let draft = page.words.truth(PageWord::Draft) == Some(true);
        let waiting = page.date.as_ref().filter(|date| {
            page.made.is_none() && !page.speaks_for_a_list() && dates.after_today(&date.value)
        });
        // What is written in a file many pages are made from is no one
        // page's.
        let own = page.source.filter(|_| page.made.is_none());
        let mut problems: Vec<Problem> = own
            .and_then(|source| placed.get(&source))
            .map(|placed| placed.iter().map(|&problem| problem.clone()).collect())
            .unwrap_or_default();
        for problem in &built.problems {
            if !problems.contains(problem) {
                problems.push(problem.clone());
            }
        }
        if !draft && waiting.is_none() && problems.is_empty() {
            return None;
        }
        Some(Noted {
            file: built.file.clone(),
            address: address.as_str().to_owned(),
            title: page.title.as_ref().map(|title| title.value.clone()),
            draft,
            waiting: waiting.map(|date| date.value.iso_day()),
            problems,
        })
    });
    noted.collect()
}

/// Every file of a site's own, by its path from the site's folder, with
/// its length and when it was last changed.
type Marks = HashMap<String, (u64, Option<std::time::SystemTime>)>;

fn marks(site: &Path) -> Marks {
    let files = crate::serve::watch::files(site).into_iter();
    let marked = files.filter_map(|file| {
        let found = std::fs::metadata(crate::platform::join(site, &file)).ok()?;
        Some((file, (found.len(), found.modified().ok())))
    });
    marked.collect()
}

/// The files of the site at `site` that differ between `was` and `now`,
/// each with what it holds, or nothing where it has gone.
fn written_between(site: &Path, was: &Marks, now: &Marks) -> Vec<(String, Option<blake3::Hash>)> {
    let changed = now
        .iter()
        .filter(|(file, mark)| was.get(*file) != Some(mark));
    let mut written: Vec<(String, Option<blake3::Hash>)> = changed
        .map(|(file, _)| {
            let held = std::fs::read(crate::platform::join(site, file));
            (file.clone(), held.ok().map(|held| blake3::hash(&held)))
        })
        .collect();
    let gone = was.keys().filter(|file| !now.contains_key(*file));
    written.extend(gone.map(|file| (file.clone(), None)));
    written.sort_by(|a, b| a.0.cmp(&b.0));
    written
}

fn is_page(file: &str) -> bool {
    file.ends_with(".html") || file.ends_with(".htm")
}

/// The stylesheets `now` publishes under another name than `was` did,
/// each as the name it had and the name it has.
fn renamed_styles(was: &Reading, now: &Reading) -> Vec<(String, String)> {
    let name = |published: &str| published.rsplit('/').next().unwrap_or(published).to_owned();
    let now: HashMap<&str, &str> = (now.renderer.assets().iter())
        .map(|(_, asset)| (asset.name.as_str(), asset.published.as_str()))
        .collect();
    let was = was.renderer.assets().iter();
    was.filter(|(_, asset)| asset.published.ends_with(".css"))
        .filter_map(|(_, asset)| {
            let published = *now.get(asset.name.as_str())?;
            (published != asset.published).then(|| (name(&asset.published), name(published)))
        })
        .collect()
}

/// `html` with each stylesheet in `renamed` under the name it had.
fn unstyled(html: &str, renamed: &[(String, String)]) -> Vec<u8> {
    let mut html = html.to_owned();
    for (was, now) in renamed {
        html = html.replace(now.as_str(), was);
    }
    html.into_bytes()
}

/// The problem with a session's `folder` in the site at `site` where it,
/// or the folder of the site's it is kept in, is a link: nothing is
/// written or removed through one, which may lead out of the site.
fn link_in_the_way(site: &Path, folder: &Path) -> Option<Problem> {
    let is_link = |path: &PathBuf| std::fs::symlink_metadata(path).is_ok_and(|at| at.is_symlink());
    let link = [site.join(KEPT), folder.to_path_buf()]
        .into_iter()
        .find(is_link)?;
    Some(Problem::nowhere(Kind::NotStatacsFolder {
        path: link,
        found: Occupant::Link,
    }))
}

/// Writes every output held to be written.
fn write(built: &mut [Built], staging: &Staging) -> Vec<Problem> {
    let told = built.par_iter_mut().filter_map(|built| {
        let html = built.html.take()?;
        match staging.write(&built.file, html.as_bytes()) {
            Ok(()) => built.written = true,
            Err(problem) => return Some(problem),
        }
        None
    });
    told.collect()
}

/// `output` as the reading `to` leads to has it.
fn carried(output: Output, to: &Translation) -> Option<Output> {
    Some(match output {
        Output::Page(page) => Output::Page(to.page(page)?),
        Output::Later(list, number) => Output::Later(to.list(list)?, number),
        Output::Redirect(at) => Output::Redirect((*to.redirects.get(at)?)?),
        Output::Feed(at) => Output::Feed((*to.feeds.get(at)?)?),
    })
}

/// Each output of the reading `now` as the last round built it, where it
/// still stands.
fn carry(last: &Last, now: Taken<'_>, plan: &Plan, own: &[u64]) -> Vec<Option<Built>> {
    let renderer = &now.reading.renderer;
    let was = Taken {
        reading: &last.reading,
        at: now.at,
        prints: &last.prints,
    };
    let to = last.reading.to(now.reading);
    let built = last.built.par_iter().enumerate();
    let stood: HashMap<Output, usize> = built
        .filter_map(|(at, old)| Some((carried(old.output, &to)?, at)))
        .collect();
    let stands = |at: usize, output: Output| -> Option<Built> {
        let old_at = *stood.get(&output)?;
        let old = &last.built[old_at];
        if last.own[old_at] != own[at] || !old.written || old.file != output.file(renderer) {
            return None;
        }
        let keys = old.reads.keys().iter();
        let same = keys.map(|&key| {
            let carried = to.key(key)?;
            (was.of(key) == now.of(carried)).then_some(carried)
        });
        let reads: Vec<Key> = same.collect::<Option<_>>()?;
        let problems = old.problems.iter().map(|problem| moved(problem, &to));
        let problems: Vec<Problem> = problems.collect::<Option<_>>()?;
        let links = match &old.links {
            Some(checked) => {
                let checked = checked.moved(&to)?;
                if !plan.linked.holds_as(&checked) {
                    return None;
                }
                Some(checked)
            }
            None => None,
        };
        let developed = old.developed.iter().map(|shown| {
            Some(crate::album::Shown {
                id: to.picture(shown.id)?,
                place: match shown.place {
                    Some(place) => Some(to.place(place)?),
                    None => None,
                },
                full: shown.full,
            })
        });
        let cards = old.cards.iter().map(|asked| {
            // Worked out again from the site as it is read now, which
            // gives what was asked for wherever what it read is the same.
            Some(renderer.card_asked(to.page(asked.page())?))
        });
        let coloured = old.coloured.iter().map(|&place| to.place(place));
        Some(Built {
            output,
            file: old.file.clone(),
            html: None,
            reads: Reads::of(reads),
            problems,
            scripts: old.scripts.clone(),
            pieces: old.pieces.clone(),
            links,
            developed: developed.collect::<Option<_>>()?,
            cards: cards.collect::<Option<_>>()?,
            sized: old.sized,
            coloured: coloured.collect::<Option<_>>()?,
            walked: old.walked,
            written: true,
        })
    };
    let outputs = plan.outputs.par_iter().enumerate();
    outputs.map(|(at, &output)| stands(at, output)).collect()
}

/// `problem` as the reading `to` leads to places it; none where a place
/// it holds is in a file that reading holds changed, or not at all.
fn moved(problem: &Problem, to: &Translation) -> Option<Problem> {
    // Places inside what is told are not carried over.
    if !problem.kind.related().is_empty() {
        return None;
    }
    let mut places = problem.context.iter().map(|(_, place)| *place);
    let carried = |place| match place {
        Some(place) => to.place(place).is_some(),
        None => true,
    };
    if !carried(problem.place) || !places.all(carried) {
        return None;
    }
    let to = |place| to.place(place).expect("carried over, as found above");
    Some(problem.clone().moved(to))
}
