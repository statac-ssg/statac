//! `statac serve`: the site built, answered on this machine, built again
//! on every save, and the browser told. After the first round with no
//! error, what `refresh` fetches is fetched once, beside the rounds.

pub mod http;
pub mod live;
pub mod panel;
mod server;
pub mod session;
pub mod watch;

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use crate::build::{self, Mode};
use crate::content::address::on_the_wire;
use crate::date::Date;
use crate::problem::{Kind, Problem, Severity, system_reason};
use crate::refresh::{self, Fetch, Token};
use crate::report::{Report, shown};
use crate::source::Sources;
use crate::{cli, config, wording};
use live::{Hub, Status, Told};
use panel::{Besides, Panel};
use session::{Round, Session};
use watch::{Batch, How, Settle, Watching};

/// The port `serve` answers on unless asked for another.
pub const PORT: u16 = 3000;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Options {
    /// The site's folder.
    pub site: PathBuf,
    /// The port to answer on, or the first free one after it; 0 for any.
    pub port: u16,
    pub watch: How,
    pub settle: Settle,
    /// The build's date, as `build` takes it.
    pub today: Option<Date>,
    /// Whether drafts and posts waiting for their day are shown.
    pub drafts: bool,
    /// What fetches what is new as the session starts; with none, nothing
    /// is.
    pub fetcher: Option<Fetcher>,
    /// The webmention relay's token, as `refresh` takes it.
    pub token: Option<Token>,
}

/// A fetcher handed to the thread that fetches.
#[derive(Clone)]
pub struct Fetcher(Arc<dyn Fetch + Send + Sync>);

impl Fetcher {
    pub fn new(fetcher: impl Fetch + Send + Sync + 'static) -> Fetcher {
        Fetcher(Arc::new(fetcher))
    }
}

impl PartialEq for Fetcher {
    fn eq(&self, other: &Fetcher) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Fetcher {}

impl std::fmt::Debug for Fetcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Fetcher")
    }
}

/// What a round waits for.
enum Heard {
    Saves(Batch),
    /// A line said by the fetching or the server.
    Said(String),
    /// The site's folder is no longer watched.
    Ended,
}

/// Where fetching stands.
enum Fetching {
    /// Waiting for the first round with no error, to fetch with this.
    Waits(Option<Fetcher>),
    /// Begun, or never to begin.
    Done,
}

/// A problem as it is known from one round to the next: its headline, and
/// its file and the text it points at. Its line isn't part of it, so
/// writing above a problem makes no new one of it.
type Identity = (String, Option<(PathBuf, String)>);

/// A problem of an earlier round.
struct Known {
    identity: Identity,
    /// Its kind's name.
    name: &'static str,
    error: bool,
}

/// A site being served.
pub struct Serving {
    options: Options,
    session: Session,
    hub: Arc<Hub>,
    site: Arc<http::Site>,
    heard: Receiver<Heard>,
    /// What the thread that fetches hands its lines to.
    tell: Sender<Heard>,
    /// Whether the site's folder is no longer watched.
    ended: bool,
    fetching: Fetching,
    watching: Watching,
    port: u16,
    colour: bool,
    /// The problems the author has been shown and that haven't gone.
    last: Vec<Known>,
    /// What each of the site's own files held as a round last left it,
    /// by the file: one Statac wrote a date or an id into, and one the
    /// site's own command wrote or, with nothing held, removed.
    left: HashMap<String, Option<blake3::Hash>>,
    /// Whether it has been said that no ids are written yet.
    said_unrecorded: bool,
    /// What the last round said of the problems gone since the one before.
    gone: Option<String>,
}

/// Builds the site, starts answering for it and watching it, and says on
/// `out` where. Nothing where it couldn't, having said why.
pub fn start(options: &Options, out: &mut dyn Write, colour: bool) -> Option<Serving> {
    let building = build::Options {
        mode: Mode::Build,
        site: options.site.clone(),
        out: None,
        today: options.today,
        drafts: options.drafts,
        url: None,
    };
    // Watching starts before the first build, so a save made while it runs
    // is built after it.
    let (sender, batches) = mpsc::channel();
    let watching = watch::start(&options.site, options.watch, options.settle, sender);
    let started = Instant::now();
    let (session, round) = Session::start_saying(&building, &mut || {
        say(out, &format!("{}\n", wording::WAITING));
    });
    let report = Report::new(round.outcome.sources(), &options.site, colour);
    let took = started.elapsed().saturating_sub(round.outcome.waited);
    say(
        out,
        &cli::said_of_build(&report, &round.outcome, session.folder(), took),
    );
    // What the terminal was told of how the round ended.
    let ended = match round.outcome.published {
        Some(published) => {
            let folder = session.folder().display().to_string();
            wording::wrote(&published, &folder, took.as_secs_f64())
        }
        None => wording::wrote_nothing(
            (round.outcome.problems.iter())
                .filter(|problem| problem.kind.severity() == Severity::Error)
                .count(),
        ),
    };
    let stop = |out: &mut dyn Write, problem: Problem| {
        let sources = Sources::default();
        let report = Report::new(&sources, &options.site, colour);
        say(out, &format!("\n{}", report.problem(&problem)));
    };
    let watching = match watching {
        Ok(watching) => watching,
        // A folder that can't be read at all has been said by the build.
        Err(_) if round.outcome.stopped() => return None,
        Err(reason) => {
            let path = options.site.clone();
            stop(out, Problem::nowhere(Kind::CantRead { path, reason }));
            return None;
        }
    };

    let hub = Arc::new(Hub::new());
    let folder = std::path::absolute(session.folder()).unwrap_or(session.folder().to_path_buf());
    let site = Arc::new(http::Site::new(
        folder,
        hub.clone(),
        base(&options.site).unwrap_or_default(),
    ));
    // One receiver holds the saves and what the fetching and the server
    // say, so only the thread running rounds writes, and no line is torn.
    let (tell, heard) = mpsc::channel();
    let unanswered = {
        let tell = tell.clone();
        move |error: &std::io::Error| {
            let line = wording::cant_answer(&system_reason(error));
            let _ = tell.send(Heard::Said(format!("{line}\n")));
        }
    };
    let port = match http::listen(&site, options.port, unanswered) {
        Ok(port) => port,
        Err(problem) => {
            stop(out, problem);
            return None;
        }
    };
    {
        let tell = tell.clone();
        std::thread::spawn(move || {
            for batch in batches {
                if tell.send(Heard::Saves(batch)).is_err() {
                    return;
                }
            }
            let _ = tell.send(Heard::Ended);
        });
    }
    let mut serving = Serving {
        options: options.clone(),
        session,
        hub,
        site,
        heard,
        tell,
        ended: false,
        fetching: Fetching::Waits(options.fetcher.clone()),
        watching,
        port,
        colour,
        last: Vec::new(),
        left: HashMap::new(),
        said_unrecorded: false,
        gone: None,
    };
    serving.last = serving.identities(&round);
    serving.left.extend(round.hooked.iter().cloned());
    serving.hub.set_status(serving.status(&round, None));

    let mut said = serving.stamps(&round);
    let included = round.outcome.included;
    if included.drafts + included.waiting > 0 && !round.outcome.stopped() {
        let showing = wording::showing(included.drafts, included.waiting);
        said.push_str(&format!("{showing}\n"));
    }
    if options.port != 0 && port != options.port {
        said.push_str(&format!("{}\n", wording::port_moved(options.port, port)));
    }
    said.push_str(&format!(
        "{}\n",
        report.said(&wording::serving(
            &serving.session.folder().display().to_string(),
            &serving.address()
        ))
    ));
    if serving.session.ran_before() {
        let ran = match serving.session.watches() {
            true => wording::HOOK_RUNS_AGAIN,
            false => wording::HOOK_RAN_ONCE,
        };
        said.push_str(&format!("{ran}\n"));
    }
    if serving.session.sets_after() {
        let after = match serving.session.ran_after() {
            true => wording::AFTER_RAN_ONCE,
            false => wording::AFTER_WAITS,
        };
        said.push_str(&format!("{after}\n"));
    }
    said.push_str(&serving.fetch(&round));
    if let Some(every) = serving.watching.polling {
        let because = serving.watching.because.as_ref();
        said.push_str(&format!("{}\n", wording::polling(every, because)));
    }
    said.push_str(&format!("{}\n", wording::STOP_SERVING));
    serving.set_panel(round, &shown(&ended), &[]);
    say(out, &said);
    Some(serving)
}

impl Serving {
    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn hub(&self) -> &Arc<Hub> {
        &self.hub
    }

    /// How often the site's folder is looked at, where it is polled.
    pub fn polling(&self) -> Option<Duration> {
        self.watching.polling
    }

    /// The site's first page, as a browser on this machine asks for it.
    pub fn address(&self) -> String {
        format!(
            "http://localhost:{}{}",
            self.port,
            on_the_wire(&format!("{}/", self.site.base()))
        )
    }

    /// Waits for the site's files to change, builds what changed, tells the
    /// browser and says on `out` what happened, and each line the fetching
    /// or the server said meanwhile. False once the folder is no longer
    /// watched.
    pub fn round(&mut self, out: &mut dyn Write) -> bool {
        let mut batch = loop {
            if self.ended {
                say(out, &format!("{}\n", wording::WATCHING_ENDED));
                return false;
            }
            match self.heard.recv() {
                Ok(Heard::Saves(batch)) => break batch,
                Ok(Heard::Said(line)) => say(out, &line),
                Ok(Heard::Ended) | Err(_) => self.ended = true,
            }
        };
        // Saves that settled while the last round ran are one round more.
        while let Ok(more) = self.heard.try_recv() {
            match more {
                Heard::Saves(more) => {
                    batch.changed.extend(more.changed);
                    batch.rescan |= more.rescan;
                }
                Heard::Said(line) => say(out, &line),
                Heard::Ended => self.ended = true,
            }
        }
        batch.changed.sort();
        batch.changed.dedup();
        // A file as a round left it is built already, and its being
        // written was no save of the author's.
        batch.changed.retain(|file| !self.as_left(file));
        if batch.changed.is_empty() && !batch.rescan {
            return true;
        }

        let started = Instant::now();
        self.hub.set_building(true);
        let round = self.session.rebuild_for_saying(&batch.changed, &mut || {
            say(out, &format!("{}\n", wording::WAITING));
        });
        let took = started.elapsed().saturating_sub(round.outcome.waited);
        self.left.extend(round.hooked.iter().cloned());
        // The names are the site's own, shown as any text of its is.
        let changed: Vec<String> = batch.changed.iter().map(|file| shown(file)).collect();

        let stopped = round.outcome.stopped();
        let told = match stopped {
            true => Told::Nothing,
            false => {
                if let Some(base) = base(&self.options.site) {
                    self.site.set_base(base);
                }
                self.told(&round)
            }
        };
        let sources = Sources::default();
        let mut more =
            Report::new(&sources, &self.options.site, self.colour).hooks(round.outcome.hooks);
        more.push_str(&self.stamps(&round));
        more.push_str(&self.differences(&round));
        more.push_str(&self.fetch(&round));
        // The system lost reports and no file came or went, which serve's
        // own writing can cause. A round that changed nothing the author
        // can see found no save, and says nothing.
        if batch.changed.is_empty() && !stopped && told == Told::Nothing && more.is_empty() {
            self.hub.set_building(false);
            return true;
        }
        let report = Report::new(&sources, &self.options.site, self.colour);
        let result = self.result(&round, Some(took));
        let mut said = format!(
            "{} {}",
            report.said(&wording::files_changed(&changed)),
            match stopped {
                true => report.stopped(&result),
                false => result.clone(),
            }
        );
        said.push(' ');
        said.push_str(&match stopped {
            true => wording::BROWSER_KEEPS.to_owned(),
            false => wording::told(&told),
        });
        said.push('\n');
        said.push_str(&more);

        let status = self.status(&round, Some(took));
        self.set_panel(round, &result, &changed);
        self.hub.publish(told, changed, status);
        say(out, &said);
        true
    }

    /// Begins fetching, where `round` is the first with no error, and says
    /// so where the site has something to fetch.
    fn fetch(&mut self, round: &Round) -> String {
        if round.outcome.stopped() {
            return String::new();
        }
        let Fetching::Waits(fetcher) = std::mem::replace(&mut self.fetching, Fetching::Done) else {
            return String::new();
        };
        if !refresh::wanted(&round.outcome) {
            return String::new();
        }
        let Some(Fetcher(fetcher)) = fetcher else {
            return format!("{}\n", wording::OFFLINE);
        };
        let options = refresh::Options {
            site: self.options.site.clone(),
            today: self.options.today,
            token: self.options.token.clone(),
        };
        let (tell, colour) = (self.tell.clone(), self.colour);
        std::thread::spawn(move || {
            let mut say = |line: &str| {
                let _ = tell.send(Heard::Said(line.to_owned()));
            };
            // A fault is said in a line of serve's own, not as Rust prints
            // a panic.
            let fetched = crate::album::quietly(|| {
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    refresh::fetch_new(&options, &*fetcher, &mut say, colour)
                }))
            });
            if fetched.is_err() {
                say(&format!("{}\n", wording::FETCHING_STOPPED));
            }
        });
        format!("{}\n", wording::FETCHING)
    }

    /// Whether the file at `file`, from the site's folder, is as a round
    /// last left it.
    fn as_left(&mut self, file: &str) -> bool {
        let Some(left) = self.left.get(file) else {
            return false;
        };
        let path = crate::platform::join(&self.options.site, file);
        let same = match (std::fs::read(&path), left) {
            (Ok(held), Some(left)) => blake3::hash(&held) == *left,
            (Err(_), None) => !path.exists(),
            _ => false,
        };
        if !same {
            self.left.remove(file);
        }
        same
    }

    /// What the round wrote into the site's own files, a line for each
    /// date and each id, and each file it couldn't write, drawn.
    fn stamps(&mut self, round: &Round) -> String {
        let stamps = &round.stamps;
        let written = stamps.files.iter().cloned();
        self.left
            .extend(written.map(|(file, held)| (file, Some(held))));
        let sources = Sources::default();
        let report = Report::new(&sources, &self.options.site, self.colour);
        let mut said = report.stamps(stamps);
        if stamps.unrecorded && !std::mem::replace(&mut self.said_unrecorded, true) {
            said.push_str(&format!("{}\n", wording::IDS_NOT_WRITTEN));
        }
        if !stamps.refused.is_empty() {
            said.push_str(&format!("\n{}\n", report.problems(&stamps.refused)));
        }
        said
    }

    /// Hands the pages the panel for `round`, which was for a save of
    /// `changed` and ended as `said` says.
    fn set_panel(&self, round: Round, said: &str, changed: &[String]) {
        let before = self.hub.panel();
        let besides = Besides {
            took: self.session.took(),
            changed,
            said,
            gone: self.gone.as_deref(),
            drafts: self.options.drafts,
        };
        let panel = Panel::after(
            round,
            &self.options.site,
            &self.site.base(),
            besides,
            &before,
        );
        self.hub.set_panel(panel);
    }

    /// What the round built, or that it stopped.
    fn result(&self, round: &Round, took: Option<Duration>) -> String {
        let errors = round
            .outcome
            .problems
            .iter()
            .filter(|problem| problem.kind.severity() == Severity::Error)
            .count();
        match (round.outcome.stopped(), took) {
            (true, _) => wording::stopped_round(errors),
            (false, Some(took)) => {
                wording::built_round(round.built, round.outputs, took.as_secs_f64())
            }
            (false, None) => String::new(),
        }
    }

    fn status(&self, round: &Round, took: Option<Duration>) -> Status {
        let stopped = round.outcome.stopped();
        let mut line = self.result(round, took);
        if stopped {
            line.push(' ');
            line.push_str(wording::PAGE_KEPT);
        }
        let report = Report::new(round.outcome.sources(), &self.options.site, false);
        Status {
            stopped,
            line,
            problems: report.check(&round.outcome.problems, &round.outcome.quiet),
        }
    }

    fn identities(&self, round: &Round) -> Vec<Known> {
        let sources = round.outcome.sources();
        round
            .outcome
            .problems
            .iter()
            .filter(|problem| !round.outcome.quiet.quietens(&problem.kind))
            .map(|problem| Known {
                identity: identity(problem, sources),
                name: problem.kind.name(),
                error: problem.kind.severity() == Severity::Error,
            })
            .collect()
    }

    /// The problems new since the last round, drawn, with every error
    /// where the round stopped, and the ones that have gone, counted.
    fn differences(&mut self, round: &Round) -> String {
        let outcome = &round.outcome;
        let stopped = outcome.stopped();
        let now = self.identities(round);
        let mut before = std::mem::replace(&mut self.last, now);
        let shown: Vec<&Problem> = outcome
            .problems
            .iter()
            .filter(|problem| !outcome.quiet.quietens(&problem.kind))
            .filter(|problem| {
                let identity = identity(problem, outcome.sources());
                let known = before.iter().position(|known| known.identity == identity);
                let new = known.map(|at| before.swap_remove(at)).is_none();
                new || (stopped && problem.kind.severity() == Severity::Error)
            })
            .collect();
        // A round that stops finds only what stopped it. The warnings of
        // the round before are neither gone nor, later, new.
        let (unsaid, before): (Vec<Known>, Vec<Known>) = before
            .into_iter()
            .partition(|known| stopped && !known.error);
        self.last.extend(unsaid);
        let mut gone: Vec<(&str, usize)> = Vec::new();
        for Known { name, .. } in &before {
            match gone.iter_mut().find(|(known, _)| known == name) {
                Some((_, count)) => *count += 1,
                None => gone.push((name, 1)),
            }
        }
        gone.sort_by_key(|&(name, count)| (std::cmp::Reverse(count), name));

        let report = Report::new(outcome.sources(), &self.options.site, self.colour);
        let mut said = String::new();
        if !shown.is_empty() {
            said.push_str(&format!("\n{}\n", report.problems(shown)));
        }
        self.gone = (!gone.is_empty()).then(|| wording::gone(&gone));
        if let Some(gone) = &self.gone {
            said.push_str(&format!("{gone}\n"));
        }
        said
    }

    /// What the round asks of the pages open in a browser, from what it
    /// wrote. Stylesheets are swapped in only where nothing else changed:
    /// a stylesheet is published under a name made from what it holds, so
    /// a change to one also writes every page that links it, and those
    /// pages must differ from the ones before by that name alone.
    fn told(&self, round: &Round) -> Told {
        match moved_styles(&round.written, &round.removed) {
            Some(moved) if !moved.is_empty() => match round.restyled {
                true => {
                    let base = self.site.base();
                    let address = |path: &String| on_the_wire(&format!("{base}/{path}"));
                    Told::Styles(
                        moved
                            .iter()
                            .map(|(old, new)| (address(old), address(new)))
                            .collect(),
                    )
                }
                false => Told::Reload,
            },
            _ if round.written.is_empty() && round.removed.is_empty() => Told::Nothing,
            _ => Told::Reload,
        }
    }
}

/// The stylesheets a round moved, each as where it was and where it is,
/// which are the same for one written again in place. Nothing where the
/// round wrote or removed anything but stylesheets and pages, or removed a
/// stylesheet it wrote no other for.
fn moved_styles(written: &[String], removed: &[String]) -> Option<Vec<(String, String)>> {
    let style = |path: &String| path.ends_with(".css");
    if !removed.iter().all(style) || !written.iter().all(|path| style(path) || is_page(path)) {
        return None;
    }
    let mut removed: Vec<&String> = removed.iter().collect();
    let mut moved = Vec::new();
    for new in written.iter().filter(|path| style(path)) {
        let old = removed
            .iter()
            .position(|old| unmarked(old) == unmarked(new))
            .map(|at| removed.swap_remove(at));
        moved.push((old.unwrap_or(new).clone(), new.clone()));
    }
    removed.is_empty().then_some(moved)
}

fn is_page(path: &str) -> bool {
    path.ends_with(".html") || path.ends_with(".htm")
}

/// `path` without the eight letters and figures a build puts before a
/// published file's extension: `site.1a2b3c4d.css` as `site.css`.
fn unmarked(path: &str) -> String {
    let pieces: Vec<&str> = path.rsplitn(3, '.').collect();
    match pieces[..] {
        [extension, mark, stem]
            if mark.len() == 8 && mark.bytes().all(|b| b.is_ascii_hexdigit()) =>
        {
            format!("{stem}.{extension}")
        }
        _ => path.to_owned(),
    }
}

fn identity(problem: &Problem, sources: &Sources) -> Identity {
    let written = problem.place.map(|place| {
        (
            sources.path(place.source).to_path_buf(),
            sources.spelling(place).to_owned(),
        )
    });
    (problem.headline(), written)
}

/// The path the site at `site` is published under, where its settings can
/// be read.
fn base(site: &Path) -> Option<String> {
    let (settings, problems) = config::read(site, &mut Sources::default());
    let stops = problems
        .iter()
        .any(|problem| problem.kind.severity() == Severity::Error);
    (!stops).then(|| settings.base())
}

/// A stream that can't be written to is let go, as `build`'s is.
fn say(out: &mut dyn Write, text: &str) {
    let _ = out.write_all(text.as_bytes());
    let _ = out.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(paths: &[&str]) -> Vec<String> {
        paths.iter().map(|path| path.to_string()).collect()
    }

    fn pair(old: &str, new: &str) -> (String, String) {
        (old.into(), new.into())
    }

    /// A small site served from the system's temporary folder, polled so
    /// seldom that only what a test hands it starts a round.
    struct Served {
        folder: PathBuf,
        serving: Serving,
    }

    impl Served {
        fn new(label: &str) -> Served {
            let folder = std::env::temp_dir()
                .join(format!("statac-serve-unit-{label}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&folder);
            let write = |relative: &str, text: &str| {
                let path = folder.join(relative);
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(path, text).unwrap();
            };
            write("statac.yaml", "title: Site\nlanguage: en\n");
            write(
                "templates/page.html",
                "<!doctype html>\n<title>{{ page.title }}</title>\n{{ page.content }}\n",
            );
            write("content/index.md", "---\nlayout: page\n---\n# Home\n");
            write("content/post-1.md", "# Post 1\n\nThe first post.\n");
            let options = Options {
                site: folder.clone(),
                port: 0,
                watch: How::Poll {
                    every: Duration::from_secs(3600),
                },
                settle: Settle::default(),
                today: None,
                drafts: true,
                fetcher: None,
                token: None,
            };
            let mut opened = Vec::new();
            let serving = start(&options, &mut opened, false).unwrap_or_else(|| {
                panic!("serve didn't start:\n{}", String::from_utf8_lossy(&opened))
            });
            Served { folder, serving }
        }

        /// What serve says of a round for a batch the system's reports were
        /// lost from, naming `changed`.
        fn rescan(&mut self, changed: &[&str]) -> String {
            let now = Instant::now();
            let batch = Batch {
                changed: paths(changed),
                rescan: true,
                first_event: now,
                settled: now,
            };
            self.serving.tell.send(Heard::Saves(batch)).unwrap();
            let mut said = Vec::new();
            assert!(self.serving.round(&mut said));
            String::from_utf8(said).unwrap()
        }
    }

    impl Drop for Served {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.folder);
        }
    }

    #[test]
    fn reports_lost_with_nothing_changed_say_nothing() {
        let mut served = Served::new("lost");
        let generation = served.serving.hub.generation();
        assert_eq!(served.rescan(&[]), "");
        assert_eq!(served.serving.hub.generation(), generation);
    }

    #[test]
    fn reports_lost_over_a_save_say_what_it_built() {
        let mut served = Served::new("lost-save");
        std::fs::write(
            served.folder.join("content/post-1.md"),
            "# Post 1\n\nThe first post, saved.\n",
        )
        .unwrap();
        let generation = served.serving.hub.generation();
        let said = served.rescan(&[]);
        assert!(said.starts_with("Files changed. Built 1 of "), "{said}");
        assert_ne!(served.serving.hub.generation(), generation);
    }

    #[test]
    fn reports_lost_naming_a_file_say_it_even_where_nothing_changed() {
        let mut served = Served::new("lost-named");
        std::fs::write(served.folder.join("content/post-2.md"), "# Post 2\n").unwrap();
        std::fs::remove_file(served.folder.join("content/post-2.md")).unwrap();
        let said = served.rescan(&["content/post-2.md"]);
        assert!(said.starts_with("`content/post-2.md` changed."), "{said}");
    }

    #[test]
    fn a_published_name_is_known_without_its_mark() {
        assert_eq!(unmarked("site.1a2b3c4d.css"), "site.css");
        assert_eq!(unmarked("assets/print.0F0F0F0F.css"), "assets/print.css");
        assert_eq!(unmarked("code.css"), "code.css");
        assert_eq!(unmarked("site.notamark.css"), "site.notamark.css");
        assert_eq!(unmarked("site.1a2b3c.css"), "site.1a2b3c.css");
    }

    #[test]
    fn a_stylesheet_under_a_new_name_is_paired_with_its_old_one() {
        assert_eq!(
            moved_styles(
                &paths(&["index.html", "site.22222222.css"]),
                &paths(&["site.11111111.css"])
            ),
            Some(vec![pair("site.11111111.css", "site.22222222.css")])
        );
        // Written again where it was.
        assert_eq!(
            moved_styles(&paths(&["code.css"]), &[]),
            Some(vec![pair("code.css", "code.css")])
        );
        // Two at once, each with its own.
        assert_eq!(
            moved_styles(
                &paths(&["print.44444444.css", "site.22222222.css"]),
                &paths(&["print.33333333.css", "site.11111111.css"])
            ),
            Some(vec![
                pair("print.33333333.css", "print.44444444.css"),
                pair("site.11111111.css", "site.22222222.css")
            ])
        );
    }

    #[test]
    fn anything_but_stylesheets_and_pages_is_no_swap() {
        assert_eq!(moved_styles(&paths(&["index.html"]), &[]), Some(vec![]));
        assert_eq!(
            moved_styles(&paths(&["site.22222222.css", "app.33333333.js"]), &[]),
            None
        );
        assert_eq!(
            moved_styles(
                &paths(&["site.22222222.css"]),
                &paths(&["post-1/index.html"])
            ),
            None
        );
        // A stylesheet that went, with none in its place.
        assert_eq!(
            moved_styles(&paths(&["index.html"]), &paths(&["print.11111111.css"])),
            None
        );
    }
}
