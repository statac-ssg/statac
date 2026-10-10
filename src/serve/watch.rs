//! Watching a site's folder: the system's own reports of changes through
//! `notify`, or polling, settled into batches.
//!
//! A report is a hint, never the truth: once a burst goes quiet, each path
//! hinted at is looked at on disk. So an editor that saves by writing a
//! temporary file and renaming it makes one change to the real file, and a
//! file that came and went inside the burst makes none.

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant, SystemTime};

use notify::{Config, Event, EventKind, PollWatcher, RecommendedWatcher, RecursiveMode, Watcher};

use crate::platform;
use crate::problem::system_reason;
use crate::publish::MARKER;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum How {
    /// The system's reports where the folder's disk is the machine's own,
    /// polling otherwise.
    Auto,
    Native,
    Poll {
        every: Duration,
    },
}

/// How often a polled folder is looked at, unless asked otherwise.
pub const POLL: Duration = Duration::from_millis(500);

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Settle {
    /// A burst has settled once nothing has arrived for this long.
    pub quiet: Duration,
    /// A burst that keeps going is cut into batches no longer than this.
    pub longest: Duration,
}

impl Default for Settle {
    fn default() -> Self {
        Settle {
            quiet: Duration::from_millis(50),
            longest: Duration::from_secs(1),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Batch {
    /// Files written, made or removed, from the site's folder, with
    /// forward slashes, in order.
    pub changed: Vec<String>,
    /// The system lost reports, so anything may have changed.
    pub rescan: bool,
    pub first_event: Instant,
    pub settled: Instant,
}

/// Why a folder is polled when polling wasn't asked for.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Because {
    /// The folder is on a disk of this kind, which may not report changes.
    Remote(String),
    /// The system's reports couldn't be had, for this reason.
    NoReports(String),
}

/// The watcher in use. Watching ends when this is dropped.
pub struct Watching {
    _native: Option<RecommendedWatcher>,
    /// How often the folder is looked at, where it is polled.
    pub polling: Option<Duration>,
    pub because: Option<Because>,
}

enum Source {
    Native(RecommendedWatcher),
    Poll(PollWatcher, Duration),
}

/// Watches the folder at `root`, sending each settled batch to `batches`.
/// Fails only where the folder can't be watched even by polling, with the
/// system's reason.
pub fn start(
    root: &Path,
    how: How,
    settle: Settle,
    batches: mpsc::Sender<Batch>,
) -> Result<Watching, String> {
    let io = |error: io::Error| system_reason(&error);
    let root = std::path::absolute(root).map_err(io)?;
    // Reports name files by the folder's real path, which a link in the
    // path as given makes another.
    let canonical = std::fs::canonicalize(&root).map_err(io)?;
    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let (source, because) = open(&root, how, tx).map_err(|error| reason(&error))?;

    let mut known = BTreeSet::new();
    walk(&root, &root, &mut known);
    let as_found = as_found(&root, &known, SystemTime::now());
    let settler = Settler {
        roots: [root, canonical],
        known,
        as_found,
        hints: BTreeSet::new(),
        rescan: false,
        batches,
    };
    let thread = std::thread::Builder::new().name("watch".into());
    let (native, polling) = match source {
        Source::Native(watcher) => {
            thread
                .spawn(move || settler.native(rx, settle))
                .map_err(io)?;
            (Some(watcher), None)
        }
        Source::Poll(watcher, every) => {
            thread
                .spawn(move || settler.poll(watcher, every, rx, settle))
                .map_err(io)?;
            (None, Some(every))
        }
    };
    Ok(Watching {
        _native: native,
        polling,
        because,
    })
}

fn open(
    root: &Path,
    how: How,
    tx: mpsc::Sender<notify::Result<Event>>,
) -> notify::Result<(Source, Option<Because>)> {
    let poll = |every: Duration, tx, because| -> notify::Result<(Source, Option<Because>)> {
        let mut watcher = PollWatcher::new(tx, Config::default().with_manual_polling())?;
        watcher.watch(root, RecursiveMode::Recursive)?;
        Ok((Source::Poll(watcher, every), because))
    };
    match how {
        How::Poll { every } => poll(every, tx, None),
        How::Native | How::Auto => {
            if how == How::Auto
                && let Some(kind) = platform::remote_disk(root)
            {
                return poll(POLL, tx, Some(Because::Remote(kind)));
            }
            let native =
                RecommendedWatcher::new(tx.clone(), Config::default()).and_then(|mut watcher| {
                    watcher.watch(root, RecursiveMode::Recursive)?;
                    Ok(watcher)
                });
            match native {
                Ok(watcher) => Ok((Source::Native(watcher), None)),
                Err(error) => poll(POLL, tx, Some(Because::NoReports(reason(&error)))),
            }
        }
    }
}

fn reason(error: &notify::Error) -> String {
    match &error.kind {
        notify::ErrorKind::Io(error) => system_reason(error),
        notify::ErrorKind::PathNotFound => "it doesn't exist".into(),
        notify::ErrorKind::MaxFilesWatch => {
            "the system is watching as many files as it allows".into()
        }
        _ => system_reason(&io::Error::other(error.to_string())),
    }
}

struct Settler {
    roots: [PathBuf; 2],
    known: BTreeSet<String>,
    /// The files not yet seen to change, as they were when watching began.
    as_found: Found,
    hints: BTreeSet<String>,
    rescan: bool,
    batches: mpsc::Sender<Batch>,
}

impl Settler {
    /// Reports arrive as changes happen; a burst has settled when they
    /// stop for `quiet`.
    fn native(mut self, rx: mpsc::Receiver<notify::Result<Event>>, settle: Settle) {
        while let Ok(first) = rx.recv() {
            if !self.absorb(first) {
                continue;
            }
            let first_event = Instant::now();
            loop {
                let left = settle.longest.saturating_sub(first_event.elapsed());
                if left.is_zero() {
                    break;
                }
                match rx.recv_timeout(settle.quiet.min(left)) {
                    Ok(event) => {
                        self.absorb(event);
                    }
                    Err(RecvTimeoutError::Timeout) => break,
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            }
            if !self.emit(first_event) {
                return;
            }
        }
    }

    /// Polling sees a burst only scan by scan, its changes a whole
    /// interval apart, so no quiet period can settle it: a scan that finds
    /// changes is followed after `quiet` by another, and the burst has
    /// settled when one finds nothing.
    fn poll(
        mut self,
        watcher: PollWatcher,
        every: Duration,
        rx: mpsc::Receiver<notify::Result<Event>>,
        settle: Settle,
    ) {
        let mut first_event: Option<Instant> = None;
        loop {
            if watcher.poll_blocking().is_err() {
                return;
            }
            let mut found = false;
            while let Ok(event) = rx.try_recv() {
                found |= self.absorb(event);
            }
            match first_event {
                None if found => {
                    first_event = Some(Instant::now());
                    std::thread::sleep(settle.quiet);
                }
                Some(began) if found && began.elapsed() < settle.longest => {
                    std::thread::sleep(settle.quiet);
                }
                Some(began) => {
                    if !self.emit(began) {
                        return;
                    }
                    first_event = None;
                    std::thread::sleep(every);
                }
                None => std::thread::sleep(every),
            }
        }
    }

    /// Takes what a report says may have changed, and gives whether it
    /// said anything of the site's.
    fn absorb(&mut self, event: notify::Result<Event>) -> bool {
        let Ok(event) = event else {
            self.rescan = true;
            return true;
        };
        // Reading a file is reported on Linux, and counting it would make
        // every page built or served a save.
        if matches!(event.kind, EventKind::Access(_)) {
            return false;
        }
        let mut said = false;
        if event.need_rescan() {
            self.rescan = true;
            said = true;
        }
        for path in &event.paths {
            if let Some(relative) = relative(&self.roots, path)
                && !relative.is_empty()
                && !is_ignored(&relative)
                && !in_built_folder(&self.roots[0], &relative)
            {
                self.hints.insert(relative);
                said = true;
            }
        }
        said
    }

    /// Sends what the burst changed, if anything; gives whether anyone is
    /// still listening.
    fn emit(&mut self, first_event: Instant) -> bool {
        let rescan = std::mem::take(&mut self.rescan);
        let hints = std::mem::take(&mut self.hints);
        let changed = match rescan {
            true => rescan_all(&self.roots[0], &mut self.known),
            false => look_at(&self.roots[0], hints, &mut self.known, &mut self.as_found),
        };
        if changed.is_empty() && !rescan {
            return true;
        }
        let batch = Batch {
            changed,
            rescan,
            first_event,
            settled: Instant::now(),
        };
        self.batches.send(batch).is_ok()
    }
}

fn slashed(rest: &Path) -> String {
    let parts: Vec<_> = rest
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect();
    parts.join("/")
}

fn relative(roots: &[PathBuf; 2], path: &Path) -> Option<String> {
    roots
        .iter()
        .find_map(|root| path.strip_prefix(root).ok())
        .map(slashed)
}

/// What an editor or the system leaves beside a file while it works, and
/// anything named with a dot, which also hides what Statac keeps in the
/// site's folder.
pub fn is_ignored(relative: &str) -> bool {
    relative.split('/').any(|name| {
        name.starts_with('.')
            || name.ends_with('~')
            || (name.starts_with('#') && name.ends_with('#'))
            || name == "4913"
            || name.ends_with(".swp")
            || name.ends_with(".swx")
            || name.ends_with(".tmp")
            || name.ends_with(".crswap")
            || name.ends_with("___jb_tmp___")
            || name.ends_with("___jb_old___")
    })
}

/// Whether `relative` is in a folder Statac built, such as `_site`: a
/// build into one is no change to the site.
fn in_built_folder(root: &Path, relative: &str) -> bool {
    let mut folder = root.to_path_buf();
    relative.split('/').any(|name| {
        folder.push(name);
        folder.join(MARKER).is_file()
    })
}

fn within<'a>(known: &'a BTreeSet<String>, folder: &str) -> impl Iterator<Item = &'a String> {
    let prefix = format!("{folder}/");
    known
        .range(prefix.clone()..)
        .take_while(move |path| path.starts_with(&prefix))
}

/// A file's size and the time it was last written.
type Stamp = (u64, SystemTime);
type Found = BTreeMap<String, Stamp>;

fn stamp(found: &std::fs::Metadata) -> Option<Stamp> {
    Some((found.len(), found.modified().ok()?))
}

/// The stamp of each file in `known` as it is at `now`, for telling a
/// report of a change made before watching began, which can arrive after
/// it, from a save. A time with no fraction of a second may be all the
/// disk keeps, and a save within the same second or two would leave such
/// a stamp as it was, so one that recent is left out.
fn as_found(root: &Path, known: &BTreeSet<String>, now: SystemTime) -> Found {
    let settled = |written: SystemTime| {
        let whole = written
            .duration_since(SystemTime::UNIX_EPOCH)
            .is_ok_and(|since| since.subsec_nanos() == 0);
        !whole || now.duration_since(written).is_ok_and(|ago| ago > COARSEST)
    };
    known
        .iter()
        .filter_map(|path| {
            let found = std::fs::metadata(platform::join(root, path)).ok()?;
            let stamp = stamp(&found).filter(|(_, written)| settled(*written))?;
            Some((path.clone(), stamp))
        })
        .collect()
}

/// The longest step between the times a disk in use keeps for a file.
const COARSEST: Duration = Duration::from_secs(2);

fn look_at(
    root: &Path,
    hints: BTreeSet<String>,
    known: &mut BTreeSet<String>,
    as_found: &mut Found,
) -> Vec<String> {
    let mut changed = BTreeSet::new();
    for hint in hints {
        let at = platform::join(root, &hint);
        match std::fs::metadata(&at) {
            Ok(found) if found.is_file() => {
                if let Some(was) = as_found.get(&hint)
                    && stamp(&found).as_ref() == Some(was)
                {
                    continue;
                }
                as_found.remove(&hint);
                known.insert(hint.clone());
                changed.insert(hint);
            }
            Ok(found) if found.is_dir() => {
                let mut inside = BTreeSet::new();
                walk(root, &at, &mut inside);
                let gone: Vec<String> = within(known, &hint)
                    .filter(|path| !inside.contains(*path))
                    .cloned()
                    .collect();
                for path in gone {
                    known.remove(&path);
                    changed.insert(path);
                }
                // A folder's own report says something inside it moved;
                // only the files new to the watcher are sure to be changes.
                for path in inside {
                    if known.insert(path.clone()) {
                        changed.insert(path);
                    }
                }
            }
            _ => {
                let mut gone: Vec<String> = within(known, &hint).cloned().collect();
                gone.push(hint);
                for path in gone {
                    if known.remove(&path) {
                        changed.insert(path);
                    }
                }
            }
        }
    }
    changed.into_iter().collect()
}

/// Every file in the site at `root` that is watched, by its path from
/// there with forward slashes.
pub fn files(root: &Path) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    walk(root, root, &mut found);
    found
}

fn rescan_all(root: &Path, known: &mut BTreeSet<String>) -> Vec<String> {
    let mut now = BTreeSet::new();
    walk(root, root, &mut now);
    let changed = known.symmetric_difference(&now).cloned().collect();
    *known = now;
    changed
}

fn walk(root: &Path, folder: &Path, into: &mut BTreeSet<String>) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(rest) = path.strip_prefix(root) else {
            continue;
        };
        let relative = slashed(rest);
        if is_ignored(&relative) {
            continue;
        }
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => {
                if !path.join(MARKER).is_file() {
                    walk(root, &path, into);
                }
            }
            Ok(kind) if kind.is_file() => {
                into.insert(relative);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Scratch {
            let path =
                std::env::temp_dir().join(format!("statac-watch-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).unwrap();
            Scratch(path)
        }

        fn write(&self, relative: &str) {
            let path = platform::join(&self.0, relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, relative).unwrap();
        }

        fn known(&self) -> BTreeSet<String> {
            let mut known = BTreeSet::new();
            walk(&self.0, &self.0, &mut known);
            known
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn hints(paths: &[&str]) -> BTreeSet<String> {
        paths.iter().map(|path| path.to_string()).collect()
    }

    #[test]
    fn what_editors_leave_is_ignored() {
        for name in [
            ".post-1.md.swp",
            "content/.#post-1.md",
            "post-1.md~",
            "#post-1.md#",
            "4913",
            "post-1.md.tmp",
            "post-1.md___jb_tmp___",
            ".statac/serve/index.html",
            ".git/HEAD",
        ] {
            assert!(is_ignored(name), "{name}");
        }
        for name in [
            "statac.yaml",
            "content/post-1.md",
            "assets/site.css",
            "notes.txt",
        ] {
            assert!(!is_ignored(name), "{name}");
        }
    }

    #[test]
    fn a_folder_statac_built_is_no_part_of_the_site() {
        let scratch = Scratch::new("built");
        scratch.write("content/post-1.md");
        scratch.write("_site/index.html");
        scratch.write(&format!("_site/{MARKER}"));
        scratch.write("public/index.html");
        assert_eq!(
            scratch.known(),
            hints(&["content/post-1.md", "public/index.html"])
        );
        assert!(in_built_folder(&scratch.0, "_site/posts/index.html"));
        assert!(in_built_folder(&scratch.0, "_site"));
        assert!(!in_built_folder(&scratch.0, "public/index.html"));
        assert!(!in_built_folder(&scratch.0, "content/post-1.md"));
    }

    #[test]
    fn what_is_fetched_is_watched_and_a_file_half_written_there_is_not() {
        let scratch = Scratch::new("fetched");
        scratch.write("content/post-1.md");
        scratch.write("fetched/record");
        scratch.write("fetched/stills/youtube-abcdefghijk.jpg");
        scratch.write("fetched/stills/.youtube-bcdefghijkl.jpg.1.tmp");
        assert_eq!(
            scratch.known(),
            hints(&[
                "content/post-1.md",
                "fetched/record",
                "fetched/stills/youtube-abcdefghijk.jpg"
            ])
        );
    }

    #[test]
    fn a_hint_is_a_change_only_as_the_disk_has_it() {
        let scratch = Scratch::new("hints");
        scratch.write("content/post-1.md");
        scratch.write("content/post-2.md");
        let mut known = scratch.known();

        // Saved, and a file that came and went.
        let changed = look_at(
            &scratch.0,
            hints(&["content/post-1.md", "content/post-9.md"]),
            &mut known,
            &mut Found::new(),
        );
        assert_eq!(changed, ["content/post-1.md"]);

        // Removed.
        std::fs::remove_file(scratch.0.join("content").join("post-2.md")).unwrap();
        let changed = look_at(
            &scratch.0,
            hints(&["content/post-2.md"]),
            &mut known,
            &mut Found::new(),
        );
        assert_eq!(changed, ["content/post-2.md"]);
        assert_eq!(known, hints(&["content/post-1.md"]));
    }

    #[test]
    fn a_report_of_a_file_as_it_was_when_watching_began_is_no_change() {
        let scratch = Scratch::new("as-found");
        scratch.write("content/post-1.md");
        scratch.write("content/post-2.md");
        let mut known = scratch.known();
        let later = SystemTime::now() + 2 * COARSEST;
        let mut found = as_found(&scratch.0, &known, later);
        assert_eq!(found.len(), 2);

        let both = || hints(&["content/post-1.md", "content/post-2.md"]);
        assert!(look_at(&scratch.0, both(), &mut known, &mut found).is_empty());

        // Saved: from then on every report of it is a change.
        std::fs::write(scratch.0.join("content").join("post-1.md"), "saved").unwrap();
        let changed = look_at(&scratch.0, both(), &mut known, &mut found);
        assert_eq!(changed, ["content/post-1.md"]);
        let changed = look_at(&scratch.0, both(), &mut known, &mut found);
        assert_eq!(changed, ["content/post-1.md"]);
    }

    #[test]
    fn a_folder_hinted_at_gives_the_files_that_came_and_went() {
        let scratch = Scratch::new("folders");
        scratch.write("content/post-1.md");
        scratch.write("content/album/photo-1.txt");
        let mut known = scratch.known();

        scratch.write("content/album/photo-2.txt");
        let changed = look_at(
            &scratch.0,
            hints(&["content/album"]),
            &mut known,
            &mut Found::new(),
        );
        assert_eq!(changed, ["content/album/photo-2.txt"]);

        std::fs::remove_dir_all(scratch.0.join("content").join("album")).unwrap();
        let changed = look_at(
            &scratch.0,
            hints(&["content/album"]),
            &mut known,
            &mut Found::new(),
        );
        assert_eq!(
            changed,
            ["content/album/photo-1.txt", "content/album/photo-2.txt"]
        );
        assert_eq!(known, hints(&["content/post-1.md"]));
    }

    #[test]
    fn a_rescan_finds_what_came_and_went() {
        let scratch = Scratch::new("rescan");
        scratch.write("content/post-1.md");
        scratch.write("content/post-2.md");
        let mut known = scratch.known();
        std::fs::remove_file(scratch.0.join("content").join("post-1.md")).unwrap();
        scratch.write("content/post-3.md");
        assert_eq!(
            rescan_all(&scratch.0, &mut known),
            ["content/post-1.md", "content/post-3.md"]
        );
    }
}
