//! Publishing a built site. A build writes into a folder of its own, and
//! only a build that succeeded puts that folder in the output's place, so
//! a failed build leaves the last published site exactly as it was.
//!
//! The folder written into is the spare: the output before last, kept in
//! the site's `.statac/`. A build makes it equal to the new site, writing
//! only what differs, then exchanges it with the output, and what was the
//! output is the next build's spare. Nothing in the spare is taken on
//! trust but a copy of a photograph or a card that still has the length
//! and the time of the copy it was made from; everything else is compared
//! byte for byte. Where no spare can be kept, each build writes a fresh
//! folder beside the output and removes the old one.
//!
//! An output folder is replaced only when it is empty or carries the marker
//! Statac leaves in every folder it builds, so a mistyped output folder can
//! never cost anyone their files.
//!
//! Only one build of a site runs at a time. Each holds a lock on a file in
//! the site's `.statac` while it writes there and into its output, and
//! another waits for it.

use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Read, Seek};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

use rayon::prelude::*;

use crate::album::{KEPT, kept, kept_in};
use crate::disk::{CLOCK, Clock, Inventory, Stamp};
use crate::platform::{self, Over};
use crate::problem::{Kind, Problem, system_reason};

thread_local! {
    /// Whether a build on this thread has put its site in the output's
    /// place since [`nothing_in_place`] was last called.
    static IN_PLACE: Cell<bool> = const { Cell::new(false) };
}

/// Forgets any site put in place on this thread, as a command begins.
pub(crate) fn nothing_in_place() {
    IN_PLACE.set(false);
}

pub(crate) fn mark_in_place() {
    IN_PLACE.set(true);
}

/// Whether a site was put in place on this thread since the command began.
pub(crate) fn in_place() -> bool {
    IN_PLACE.get()
}

/// The file at the top of every folder Statac builds, which is how it
/// knows the folder is its own to replace.
pub const MARKER: &str = ".statac-built";

const MARKER_TEXT: &str = "Statac built this folder and replaces all of it on every build.\n\
                           Keep files you want published in the site's static/ folder.\n";

/// The spare's name in `.statac`.
const SPARE: &str = "spare";

/// The name in `.statac` of the file a build holds a lock on.
const LOCK: &str = "lock";

/// How many threads make files at once. A disk makes them no faster for
/// more asking; the rest would only spin while they wait.
const MAKERS: usize = 4;

/// What stands where an output folder should be, when it isn't Statac's.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Occupant {
    File,
    Link,
    /// A folder without the marker, holding this much at its top.
    Entries {
        files: usize,
        folders: usize,
    },
}

/// A build in progress: a folder written into until [`Staging::finish`]
/// puts it in the output's place. Dropped unfinished, it leaves the output
/// alone.
#[derive(Debug)]
pub struct Staging {
    /// The output as the author named it, for messages.
    shown: PathBuf,
    out: PathBuf,
    /// The folder written into: the spare, or a fresh one beside the output.
    fresh: PathBuf,
    /// The folders opened in `fresh`, kept open while the build writes.
    folders: platform::Folders,
    /// Where the output waits for the moment the two can't be exchanged
    /// in one step.
    aside: PathBuf,
    /// What the spare held when the build began; none for a fresh folder.
    spare: Option<Spare>,
    /// Whether the folder written into is kept whatever becomes of the
    /// build, as the spare is.
    keeps: bool,
    /// Whether a fresh folder is this build's to remove when dropped.
    owns_fresh: bool,
    /// Why no spare is kept, where one can't be.
    unkept: Option<Problem>,
    makers: Turns,
    /// Whether the folder written into is the output itself, made equal
    /// to the new site where it stands.
    in_place: bool,
    /// What the folder held of this build's asking when it was settled.
    settled: Option<Left>,
    /// The folders under one written where it stands that this build has
    /// walked to from its top, through no link.
    sure: Mutex<HashSet<String>>,
    /// What the build notes for whoever asked what it leaves.
    notes: Option<Notes>,
}

/// What a build that finished left in the folder it wrote, for a later
/// one that writes into the same folder to start from.
#[derive(Clone, Default, Debug)]
pub struct Left {
    /// Every file the folder holds, each a path from its top.
    pub files: Vec<String>,
    /// The files made or made anew.
    pub written: Vec<String>,
    /// The files the folder held before that it holds no more.
    pub removed: Vec<String>,
    /// Each file copied from one of the site's as it is, with what it
    /// was copied from, where a later change to that file would change
    /// its stamp.
    copied: HashMap<String, Stamp>,
    /// Each file whose name says what it holds.
    named: HashSet<String>,
}

#[derive(Debug)]
struct Notes {
    /// What the files the folder holds were copied from.
    before: HashMap<String, Stamp>,
    written: Mutex<Vec<String>>,
    copied: Mutex<HashMap<String, Stamp>>,
    /// The files the folder holds whose names say what they hold.
    named_before: HashSet<String>,
    named: Mutex<Vec<String>>,
    /// What the site's disk gave a file written as the build began. A
    /// file whose stamp it doesn't settle may be written again and keep
    /// its stamp.
    clock: Option<Clock>,
    /// Where what the build left is put once it has finished.
    left: Arc<Mutex<Option<Left>>>,
}

/// Lets only so many threads through at once.
#[derive(Debug)]
struct Turns {
    free: Mutex<usize>,
    freed: Condvar,
}

struct Turn<'a>(&'a Turns);

impl Turns {
    fn new(free: usize) -> Turns {
        Turns {
            free: Mutex::new(free),
            freed: Condvar::new(),
        }
    }

    fn take(&self) -> Turn<'_> {
        let mut free = self.free.lock().expect("never poisoned");
        while *free == 0 {
            free = self.freed.wait(free).expect("never poisoned");
        }
        *free -= 1;
        Turn(self)
    }
}

impl Drop for Turn<'_> {
    fn drop(&mut self) {
        *self.0.free.lock().expect("never poisoned") += 1;
        self.0.freed.notify_one();
    }
}

/// A file the spare held that this build has asked for.
const WANTED: u8 = 1;
/// A file the spare held, removed to make room for another name.
const GONE: u8 = 2;

/// What the spare held when the build began, and what has become of it.
#[derive(Debug, Default)]
struct Spare {
    /// Each file by its path, with what this build made of it.
    files: HashMap<String, AtomicU8>,
    folders: HashSet<String>,
    /// The same names as a disk that ignores capitals holds them, made
    /// when the build first writes a name the spare didn't hold.
    alike: OnceLock<Alike>,
    /// Held while something is removed to make room for another name.
    reshaping: Mutex<()>,
    /// The files this build wrote that the spare didn't hold.
    added: Mutex<Vec<String>>,
    /// The folders removed whole to make room for a file.
    gone: Mutex<Vec<String>>,
    /// Whether some folder the build wants is held under another
    /// spelling of its name.
    respelt: AtomicBool,
}

/// The spare's files and folders by their folded paths.
#[derive(Debug, Default)]
struct Alike {
    files: HashMap<String, Vec<String>>,
    folders: HashMap<String, Vec<String>>,
}

/// A folder's entries: its files, its folders, and whatever else it holds.
type Listing = (Vec<String>, Vec<String>, Vec<PathBuf>);

/// Lists the entries of `folder` by kind, which follows no link; a name
/// that isn't text is among whatever else it holds.
fn list(folder: &Path) -> io::Result<Listing> {
    let (mut files, mut folders, mut others) = (Vec::new(), Vec::new(), Vec::new());
    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        match entry.file_name().into_string() {
            Ok(name) if kind.is_file() => files.push(name),
            Ok(name) if kind.is_dir() => folders.push(name),
            _ => others.push(entry.path()),
        }
    }
    Ok((files, folders, others))
}

/// Everything under a folder: its files and its folders, each by its path
/// with forward slashes, and whatever else is there.
#[derive(Debug, Default)]
struct Held {
    files: Vec<String>,
    folders: Vec<String>,
    others: Vec<PathBuf>,
}

impl Held {
    /// Lists everything under `folder`, which is `rel` from the top,
    /// changing nothing.
    fn under(folder: &Path, rel: &str) -> io::Result<Held> {
        let (names, below, others) = list(folder)?;
        let mut held = Held {
            files: names.iter().map(|name| joined(rel, name)).collect(),
            folders: below.iter().map(|name| joined(rel, name)).collect(),
            others,
        };
        let deeper: Vec<Held> = below
            .par_iter()
            .zip(held.folders.par_iter())
            .map(|(name, rel)| Held::under(&folder.join(name), rel))
            .collect::<io::Result<_>>()?;
        for more in deeper {
            held.files.extend(more.files);
            held.folders.extend(more.folders);
            held.others.extend(more.others);
        }
        Ok(held)
    }
}

/// Removes what stands at `path`, whatever it is, following no link.
fn remove(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(found) if found.is_dir() => platform::remove_folder(path),
        // A link to a folder is removed as a folder on Windows.
        Ok(_) => fs::remove_file(path).or_else(|_| fs::remove_dir(path)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn joined(above: &str, name: &str) -> String {
    match above {
        "" => name.to_owned(),
        above => format!("{above}/{name}"),
    }
}

/// The folders above `rel`, nearest first.
fn above(rel: &str) -> impl Iterator<Item = &str> {
    std::iter::successors(rel.rsplit_once('/'), |(above, _)| above.rsplit_once('/'))
        .map(|(above, _)| above)
}

impl Spare {
    /// What the folder that `held` lists holds, with whatever in it is
    /// neither a file nor a folder removed, so nothing is later read or
    /// written through a link.
    fn of(held: Held) -> io::Result<Spare> {
        for other in &held.others {
            remove(other)?;
        }
        let files = held.files.into_iter();
        Ok(Spare {
            files: files.map(|file| (file, AtomicU8::new(0))).collect(),
            folders: held.folders.into_iter().collect(),
            ..Spare::default()
        })
    }

    fn alike(&self) -> &Alike {
        self.alike.get_or_init(|| {
            let mut alike = Alike::default();
            for file in self.files.keys() {
                let named = alike.files.entry(platform::folded(file)).or_default();
                named.push(file.clone());
            }
            for folder in &self.folders {
                let named = alike.folders.entry(platform::folded(folder)).or_default();
                named.push(folder.clone());
            }
            alike
        })
    }

    /// Makes room in the folder at `root` for a file at `rel`, a name the
    /// spare didn't hold: removes a file it held under another spelling of
    /// the name, a folder it held under the name, and a file it held
    /// where a folder above `rel` goes. A disk that ignores capitals holds
    /// each of those in the new file's way.
    fn make_room(&self, root: &platform::Folders, rel: &str) -> io::Result<()> {
        if self.files.is_empty() && self.folders.is_empty() {
            return Ok(());
        }
        let alike = self.alike();
        let folded = platform::folded(rel);
        let mut files: Vec<&String> = alike.files.get(&folded).into_iter().flatten().collect();
        let folders = alike.folders.get(&folded);
        for (spelt, folded) in above(rel).zip(above(&folded)) {
            files.extend(alike.files.get(folded).into_iter().flatten());
            if !self.folders.contains(spelt) && alike.folders.contains_key(folded) {
                self.respelt.store(true, Ordering::Relaxed);
            }
        }
        if files.is_empty() && folders.is_none() {
            return Ok(());
        }
        let _reshaping = self.reshaping.lock().expect("never poisoned");
        for file in files {
            let unasked =
                self.files[file].compare_exchange(0, GONE, Ordering::Relaxed, Ordering::Relaxed);
            if unasked.is_ok() {
                root.remove(file, true)?;
            }
        }
        let mut gone = self.gone.lock().expect("never poisoned");
        for folder in folders.into_iter().flatten() {
            if gone.contains(folder) {
                continue;
            }
            let within = format!("{folder}/");
            for (file, state) in &self.files {
                if file.starts_with(&within) {
                    state.store(GONE, Ordering::Relaxed);
                }
            }
            root.remove(folder, true)?;
            gone.push(folder.clone());
        }
        Ok(())
    }

    /// Removes from the folder at `root` each file held that this build
    /// didn't ask for, and each folder that leaves empty. Whatever else
    /// the folder holds is no part of the spare, and stays.
    ///
    /// Only a file itself is removed, reached through no link: a link or
    /// a folder found where a file was written, or on the way to it,
    /// stops the removal there.
    fn clear(self, root: &platform::Folders) -> io::Result<()> {
        for (file, state) in &self.files {
            if state.load(Ordering::Relaxed) != 0 {
                continue;
            }
            if !platform::stays_inside(file) {
                return Err(io::Error::other("it is outside the built site"));
            }
            root.remove_file(file)?;
            for folder in above(file) {
                // One still holding something stays, as do those above it.
                if root.remove_empty_folder(folder).is_err() {
                    break;
                }
            }
        }
        Ok(())
    }

    /// Removes from the folder at `root` everything this build didn't ask
    /// for, files and folders, so it holds the new site and nothing else.
    fn settle(self, top: &Path, root: &platform::Folders, makers: &Turns) -> io::Result<()> {
        let added = self.added.into_inner().expect("never poisoned");
        let kept = || {
            let held = self.files.iter();
            let held = held.filter(|(_, state)| state.load(Ordering::Relaxed) == WANTED);
            held.map(|(file, _)| file.as_str())
                .chain(added.iter().map(String::as_str))
        };
        let mut wanted: HashSet<&str> = HashSet::new();
        for file in kept() {
            for folder in above(file) {
                if !wanted.insert(folder) {
                    break;
                }
            }
        }
        if self.respelt.load(Ordering::Relaxed) {
            let files: HashSet<&str> = kept().collect();
            let spelt: HashMap<String, &str> = wanted
                .iter()
                .map(|&folder| (platform::folded(folder), folder))
                .collect();
            return tidy(top, root, "", &files, &wanted, &spelt);
        }
        let stands = |rel: &str| {
            above(rel)
                .next()
                .is_none_or(|folder| wanted.contains(folder))
        };
        let gone = self.gone.into_inner().expect("never poisoned");
        let folders = self.folders.par_iter().filter(|folder| {
            !wanted.contains(folder.as_str()) && stands(folder) && !gone.contains(folder)
        });
        folders.try_for_each(|folder| root.remove(folder, false))?;
        let files = self
            .files
            .par_iter()
            .filter(|(file, state)| state.load(Ordering::Relaxed) == 0 && stands(file));
        files.try_for_each(|(file, _)| {
            let _turn = makers.take();
            root.remove(file, false)
        })
    }
}

/// Makes the folder `rel` in the spare at `top`, open as `root`, hold the
/// `files` and `folders` wanted and nothing else, by what the disk says
/// it holds now. A folder held under another spelling of a wanted name is
/// given the wanted one; `spelt` has each wanted folder by its folded
/// path.
fn tidy(
    top: &Path,
    root: &platform::Folders,
    rel: &str,
    files: &HashSet<&str>,
    folders: &HashSet<&str>,
    spelt: &HashMap<String, &str>,
) -> io::Result<()> {
    let (held, below, others) = list(&platform::join(top, rel))?;
    for other in others {
        match other.file_name().and_then(|name| name.to_str()) {
            Some(name) => root.remove(&joined(rel, name), false)?,
            None => remove(&other)?,
        }
    }
    for name in held {
        let at = joined(rel, &name);
        if !files.contains(at.as_str()) {
            root.remove(&at, false)?;
        }
    }
    for name in &below {
        let at = joined(rel, name);
        if folders.contains(at.as_str()) {
            tidy(top, root, &at, files, folders, spelt)?;
            continue;
        }
        let respelt = spelt.get(&platform::folded(&at)).and_then(|wanted| {
            let (_, last) = wanted.rsplit_once('/').unwrap_or(("", wanted));
            // Where the wanted spelling is here too, the disk holds the
            // two apart, and this one is nothing the build asked for.
            let here = joined(rel, last) == *wanted && !below.iter().any(|name| name == last);
            here.then_some((*wanted, last))
        });
        match respelt {
            Some((wanted, last)) => {
                root.rename_in(rel, name, last)?;
                tidy(top, root, wanted, files, folders, spelt)?;
            }
            None => root.remove(&at, false)?,
        }
    }
    Ok(())
}

/// A site's builds held to one at a time: while it is held, no other build
/// of the site, in this process or another, holds it. It is let go of when
/// dropped, and by the system when the process ends, however it ends.
#[derive(Debug)]
#[must_use = "the hold is let go of when dropped"]
pub struct Hold {
    _locked: Option<fs::File>,
    /// How long it took to have, waiting for another build included.
    pub waited: Duration,
}

/// Takes the hold on the site at `site` for a build, calling `waiting`
/// once and waiting where another build has it. Where it can't be had, as
/// where `.statac` can't be written or its disk can't lock a file, the
/// build goes ahead without it and the problem says why.
pub fn hold(site: &Path, waiting: impl FnOnce()) -> (Hold, Option<Problem>) {
    let asked = Instant::now();
    let locked = locked(site, waiting);
    let unheld = locked.as_ref().err().map(|error| {
        Problem::nowhere(Kind::CacheNotWritten {
            path: Path::new(KEPT).join(LOCK),
            reason: system_reason(error),
        })
    });
    let hold = Hold {
        _locked: locked.ok(),
        waited: asked.elapsed(),
    };
    (hold, unheld)
}

/// The file in the site's `.statac` whose lock is the hold, locked.
fn locked(site: &Path, waiting: impl FnOnce()) -> io::Result<fs::File> {
    let file = lock_file(site, LOCK)?;
    match file.try_lock() {
        Ok(()) => return Ok(file),
        Err(fs::TryLockError::WouldBlock) => waiting(),
        Err(fs::TryLockError::Error(error)) => return Err(error),
    }
    loop {
        match file.lock() {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            locked => return locked.map(|()| file),
        }
    }
}

/// A folder in the site's `.statac` for one process at a time: `name`,
/// or where another holds that, `name-2`, `name-3` and so on, the first
/// no other holds. It is held by a lock on a file beside it, let go of as
/// a [`Hold`] is. Where no file can be locked it is `name`, unheld, and
/// the hold each build takes says why.
pub fn claim(site: &Path, name: &str) -> (String, Option<fs::File>) {
    for n in 1.. {
        let folder = match n {
            1 => name.to_owned(),
            n => format!("{name}-{n}"),
        };
        let Ok(file) = lock_file(site, &format!("{folder}.lock")) else {
            break;
        };
        match file.try_lock() {
            Ok(()) => return (folder, Some(file)),
            Err(fs::TryLockError::WouldBlock) => {}
            Err(fs::TryLockError::Error(_)) => break,
        }
    }
    (name.to_owned(), None)
}

/// The file `name` in the site's `.statac`, opened to lock, with the
/// folder made where it isn't there.
fn lock_file(site: &Path, name: &str) -> io::Result<fs::File> {
    kept(site)?;
    match fs::create_dir(site.join(KEPT)) {
        Err(error) if error.kind() != io::ErrorKind::AlreadyExists => return Err(error),
        _ => {}
    }
    // Asked again once made: it may have been put there as a link.
    platform::lock_file(&kept(site)?, name)
}

/// Starts a build of the site at `site` into `out`: checks the output is
/// Statac's to replace, clears what an interrupted build left beside it,
/// and readies the folder to write into, with the marker in it.
pub fn begin(site: &Path, out: &Path) -> Result<Staging, Problem> {
    let cant_write = |error: &io::Error| {
        Problem::nowhere(Kind::CantWrite {
            path: out.to_path_buf(),
            reason: system_reason(error),
        })
    };
    let absolute = std::path::absolute(out).map_err(|error| cant_write(&error))?;
    let absolute = lexically_normal(&absolute);
    let (Some(parent), Some(name)) = (absolute.parent(), absolute.file_name()) else {
        return Err(cant_write(&io::Error::other("it is the top of the disk")));
    };
    let beside = |what: &str| {
        let mut named = std::ffi::OsString::from(".");
        named.push(name);
        named.push(what);
        parent.join(named)
    };
    let new = beside(".statac-new");
    let mut staging = Staging {
        shown: out.to_path_buf(),
        out: absolute.clone(),
        fresh: new.clone(),
        folders: platform::Folders::new(&new),
        aside: beside(".statac-old"),
        spare: None,
        keeps: false,
        owns_fresh: false,
        unkept: None,
        makers: Turns::new(MAKERS),
        in_place: false,
        settled: None,
        sure: Mutex::default(),
        notes: None,
    };

    // A build interrupted between moving the old output aside and moving
    // the new one in leaves no output, and the last published site beside it.
    if !exists(&staging.out) && ours(&staging.aside) {
        platform::rename(&staging.aside, &staging.out).map_err(|error| cant_write(&error))?;
    }
    staging.check_output()?;
    staging.clear(&new)?;
    fs::create_dir_all(parent).map_err(|error| cant_write(&error))?;

    let place = spare_place(site, &absolute, &new, || beside(".statac-spare"));
    // In `.statac` it is named as everything kept there is, from the site.
    let named = match &place {
        Ok(place) if place.parent() == Some(parent) => place.clone(),
        _ => Path::new(KEPT).join(SPARE),
    };
    let spare = place.and_then(|place| {
        // One interrupted after moving the new output in leaves the old
        // one aside, which is what the spare would have been.
        if ours(&staging.aside) && !exists(&place) {
            platform::rename(&staging.aside, &place)?;
        }
        if !exists(&place) {
            fs::create_dir(&place)?;
            return Ok((place, None));
        }
        // The spare beside an output is made sure of as the output is; in
        // `.statac` nothing else has a name of Statac's own.
        match occupant(&place)? {
            None => Spare::of(Held::under(&place, "")?).map(|held| (place, Some(held))),
            Some(_) => Err(io::Error::other("something else has its name")),
        }
    });
    staging.clear(&staging.aside)?;
    match spare {
        Ok((place, held)) => {
            staging.folders = platform::Folders::new(&place);
            (staging.fresh, staging.spare, staging.keeps) = (place, held, true);
        }
        Err(error) => {
            staging.unkept = Some(Problem::nowhere(Kind::CacheNotWritten {
                path: named,
                reason: system_reason(&error),
            }));
            fs::create_dir(&staging.fresh).map_err(|error| cant_write(&error))?;
            staging.owns_fresh = true;
        }
    }
    staging.write(MARKER, MARKER_TEXT.as_bytes())?;
    Ok(staging)
}

/// Starts a build into `folder` where it stands, a folder an earlier
/// build wrote and nothing else has changed since. Of what it holds, only
/// `files` are this build's to write anew or to remove where it doesn't
/// ask for them; whatever else is there stays as it is. The build leaves
/// in `into` what has become of `files`, and of any it added. A file
/// copied from one of the site at `site` is taken as still equal to it
/// where that file has the stamp it had when the build that `left` it
/// copied it. A build dropped unfinished leaves the folder as far as it
/// got.
pub fn resume(
    site: &Path,
    folder: &Path,
    files: impl IntoIterator<Item = String>,
    left: &Left,
    into: Arc<Mutex<Option<Left>>>,
) -> Staging {
    let mut spare = Spare::default();
    for file in files.into_iter().filter(|file| file != MARKER) {
        spare.files.insert(file, AtomicU8::new(0));
    }
    Staging {
        shown: folder.to_path_buf(),
        out: folder.to_path_buf(),
        fresh: folder.to_path_buf(),
        folders: platform::Folders::new(folder),
        aside: folder.to_path_buf(),
        spare: Some(spare),
        keeps: true,
        owns_fresh: false,
        unkept: None,
        makers: Turns::new(MAKERS),
        in_place: true,
        settled: None,
        sure: Mutex::default(),
        notes: Some(Notes {
            before: left.copied.clone(),
            written: Mutex::default(),
            copied: Mutex::default(),
            named_before: left.named.clone(),
            named: Mutex::default(),
            clock: Clock::of_site(site),
            left: into,
        }),
    }
}

/// Where the spare of the site at `site` is kept for a build into `out`:
/// in the site's `.statac` where a folder there can be exchanged with the
/// output, which takes the two on one disk, and under the hidden name
/// `beside` gives, next to the output, where not. `landing` is a name
/// beside the output that nothing has. Refused where `.statac` or the
/// spare in it is a link, as everything Statac keeps is.
fn spare_place(
    site: &Path,
    out: &Path,
    landing: &Path,
    beside: impl Fn() -> PathBuf,
) -> io::Result<PathBuf> {
    let site = lexically_normal(&std::path::absolute(site)?);
    let kept = site.join(KEPT);
    if out.starts_with(&kept) {
        return Ok(beside());
    }
    kept_in(&site, SPARE)?;
    match fs::create_dir(&kept) {
        Err(error) if error.kind() != io::ErrorKind::AlreadyExists => return Err(error),
        _ => {}
    }
    // Asked again once made: either may have been put there as a link.
    let place = kept_in(&site, SPARE)?;
    // An output in the site's own folder is on the site's disk.
    if out.parent() == Some(site.as_path())
        || platform::moves_between(&kept, "spare-probe", landing)?
    {
        Ok(place)
    } else {
        Ok(beside())
    }
}

/// Whether `a` and `b`, files open to read, hold the same bytes. `a` is
/// left wherever the comparison stopped.
fn same_bytes(a: &mut fs::File, mut b: fs::File) -> io::Result<bool> {
    if a.metadata()?.len() != b.metadata()?.len() {
        return Ok(false);
    }
    let (mut left, mut right) = (vec![0; 64 * 1024], vec![0; 64 * 1024]);
    loop {
        let read = a.read(&mut left)?;
        if read == 0 {
            return Ok(b.read(&mut right)? == 0);
        }
        if b.read_exact(&mut right[..read]).is_err() || left[..read] != right[..read] {
            return Ok(false);
        }
    }
}

impl Staging {
    /// The folder the build writes into.
    pub fn root(&self) -> &Path {
        &self.fresh
    }

    /// Why no spare is kept for the next build, where one can't be.
    pub fn unkept(&self) -> Option<Problem> {
        self.unkept.clone()
    }

    /// The same build of the site at `site`, leaving in `into` once it has
    /// finished what the output holds, for a build that [`resume`]s there.
    pub fn leaving(mut self, site: &Path, into: Arc<Mutex<Option<Left>>>) -> Staging {
        self.notes = Some(Notes {
            before: HashMap::new(),
            written: Mutex::default(),
            copied: Mutex::default(),
            named_before: HashSet::new(),
            named: Mutex::default(),
            clock: Clock::of_site(site),
            left: into,
        });
        self
    }

    /// Refuses `rel` where writing it would go through a link. The folders
    /// above the one it is in are walked from the top once in a build, by
    /// the first file written beneath them; after that the folder it is
    /// in, and what stands at `rel` itself, are asked again at every write.
    fn no_link_to(&self, rel: &str) -> io::Result<()> {
        let (above, name) = self.walked_to(rel)?;
        platform::no_link_to(&platform::join(&self.fresh, above), name)
    }

    /// The folder `rel` is in and its name there, once the folders above
    /// that folder have been walked from the top in this build.
    fn walked_to<'a>(&self, rel: &'a str) -> io::Result<(&'a str, &'a str)> {
        let (above, name) = rel.rsplit_once('/').unwrap_or(("", rel));
        let holding = |folder: &'a str| folder.rsplit_once('/').map_or("", |(holding, _)| holding);
        let walked = {
            let sure = self.sure.lock().expect("never poisoned");
            sure.contains(above) || (!above.is_empty() && sure.contains(holding(above)))
        };
        if !walked {
            platform::no_link_to(&self.fresh, rel)?;
            let mut sure = self.sure.lock().expect("never poisoned");
            let mut walked = above;
            while sure.insert(walked.to_owned()) && !walked.is_empty() {
                walked = holding(walked);
            }
        }
        Ok((above, name))
    }

    fn wrote(&self, rel: &str) {
        if let Some(notes) = &self.notes {
            let mut written = notes.written.lock().expect("never poisoned");
            written.push(rel.to_owned());
        }
    }

    /// Notes `rel` as a copy of a file with the stamp `from`, for the next
    /// build to trust by that stamp alone where a later change to the file
    /// would change it. Any other is known by its bytes.
    fn copied(&self, rel: &str, from: Option<Stamp>) {
        if let (Some(notes), Some(from)) = (&self.notes, from)
            && notes.clock.is_some_and(|clock| from.settled(&clock))
        {
            let mut copied = notes.copied.lock().expect("never poisoned");
            copied.insert(rel.to_owned(), from);
        }
    }

    /// What the folder written into holds as the build finishes, before
    /// what it didn't ask for is removed.
    fn left(&self) -> Option<Left> {
        let notes = self.notes.as_ref()?;
        let written = notes.written.lock().expect("never poisoned").clone();
        let copied = notes.copied.lock().expect("never poisoned").clone();
        let named = notes.named.lock().expect("never poisoned");
        let named = named.iter().cloned().collect();
        let (mut files, mut removed) = (Vec::new(), Vec::new());
        match &self.spare {
            // A fresh folder holds what was written into it.
            None => {
                files.clone_from(&written);
                files.push(MARKER.to_owned());
            }
            Some(spare) => {
                for (file, state) in &spare.files {
                    match state.load(Ordering::Relaxed) == WANTED {
                        true => files.push(file.clone()),
                        false => removed.push(file.clone()),
                    }
                }
                let added = spare.added.lock().expect("never poisoned");
                files.extend(added.iter().cloned());
            }
        }
        Some(Left {
            files,
            written,
            removed,
            copied,
            named,
        })
    }

    /// Writes `bytes` at `rel`, a path with forward slashes, making the
    /// folders above it.
    pub fn write(&self, rel: &str, bytes: &[u8]) -> Result<(), Problem> {
        let held = self.claimed(rel, false)?;
        if self.in_place {
            let cant = |error: io::Error| self.cant_write(rel, &error);
            // A file held is written over where it stands: making it anew
            // takes several times as long, and a change that reaches
            // every page writes every page.
            if held {
                let (above, name) = self.walked_to(rel).map_err(cant)?;
                let folder = platform::join(&self.fresh, above);
                // Only so many at once: written quicker, a system asked
                // to say what changes under the site's folder loses count,
                // and whoever asked must look at all of it again.
                let _turn = self.makers.take();
                match platform::write_over(&folder, name, bytes).map_err(cant)? {
                    Over::Same => return Ok(()),
                    Over::Written => {
                        self.wrote(rel);
                        return Ok(());
                    }
                    Over::Refused => {}
                }
            }
            self.no_link_to(rel).map_err(cant)?;
        }
        if held {
            let same = self.folders.open_file(rel).and_then(|mut file| {
                let mut there = Vec::new();
                let same = file.metadata()?.len() == bytes.len() as u64;
                Ok(same && file.read_to_end(&mut there)? == bytes.len() && there == bytes)
            });
            if same.unwrap_or(false) {
                return Ok(());
            }
        }
        let _turn = self.makers.take();
        let mut file = self.create(rel, held)?;
        io::Write::write_all(&mut file, bytes).map_err(|error| self.cant_write(rel, &error))
    }

    /// Writes `bytes` at `rel`, a name that says what they are, as
    /// [`Staging::write`] does. One the build before left at `rel` in the
    /// folder, written where it stands, holds them already: nothing of it
    /// is read, written or removed.
    pub fn write_named(&self, rel: &str, bytes: &[u8]) -> Result<(), Problem> {
        if !self.stands_named(rel) {
            self.write(rel, bytes)?;
        }
        if let Some(notes) = &self.notes {
            let mut named = notes.named.lock().expect("never poisoned");
            named.push(rel.to_owned());
        }
        Ok(())
    }

    /// Whether the folder, written where it stands, holds at `rel` a file
    /// the build before left there under a name that says what it holds,
    /// which is this build's from now on.
    fn stands_named(&self, rel: &str) -> bool {
        let (true, Some(notes), Some(spare)) = (self.in_place, &self.notes, &self.spare) else {
            return false;
        };
        notes.named_before.contains(rel)
            && spare.files.get(rel).is_some_and(|state| {
                let asked = state.compare_exchange(0, WANTED, Ordering::Relaxed, Ordering::Relaxed);
                asked.is_ok()
            })
    }

    /// Copies `source`, the file `shown` names as opened to read, to
    /// `rel`. Where it couldn't be opened, the problem says why.
    pub fn copy(
        &self,
        source: io::Result<fs::File>,
        shown: &str,
        rel: &str,
    ) -> Result<(), Problem> {
        let cant_read = |error: &io::Error| {
            Problem::nowhere(Kind::CantRead {
                path: shown.into(),
                reason: system_reason(error),
            })
        };
        let mut source = source.map_err(|error| cant_read(&error))?;
        let found = self.notes.as_ref().and_then(|_| {
            let found = source.metadata().ok()?;
            Stamp::of(&found)
        });
        if self.stands_as_left(rel, found) {
            return Ok(());
        }
        let held = self.claim(rel)?;
        if held {
            let before = self.notes.as_ref().and_then(|notes| notes.before.get(rel));
            let as_copied = found.is_some() && before == found.as_ref();
            let there = || self.folders.open_file(rel);
            if as_copied
                || there()
                    .and_then(|b| same_bytes(&mut source, b))
                    .unwrap_or(false)
            {
                self.copied(rel, found);
                return Ok(());
            }
            source.rewind().map_err(|error| cant_read(&error))?;
        }
        let _turn = self.makers.take();
        let mut file = self.create(rel, held)?;
        io::copy(&mut source, &mut file).map_err(|error| self.cant_write(rel, &error))?;
        self.copied(rel, found);
        Ok(())
    }

    /// Copies `source`, a file Statac keeps under a name that says what it
    /// holds, opened to read, to `rel` as the disk copies a file, which
    /// shares its bytes where the disk can; `shown` names `source` in a
    /// message. Where it couldn't be opened, the problem says why.
    ///
    /// The copy is given the time `source` was last changed, and one the
    /// spare holds is left as it is where it is a file itself with that
    /// time and the length of `source`, without its bytes being read.
    pub fn copy_whole(
        &self,
        source: io::Result<fs::File>,
        shown: &str,
        rel: &str,
    ) -> Result<(), Problem> {
        let cant_read = |error: io::Error| {
            Problem::nowhere(Kind::CantRead {
                path: shown.into(),
                reason: system_reason(&error),
            })
        };
        let source = source.map_err(cant_read)?;
        let found = source.metadata().map_err(cant_read)?;
        let changed = found.modified().ok();
        let from_stamp = Stamp::of(&found);
        if self.stands_as_left(rel, from_stamp) {
            return Ok(());
        }
        let held = self.claim(rel)?;
        let as_copied = |there: platform::Measure| {
            there.named_once && there.length == found.len() && Some(there.modified) == changed
        };
        if held && self.folders.measure(rel).is_ok_and(as_copied) {
            self.copied(rel, from_stamp);
            return Ok(());
        }
        let _turn = self.makers.take();
        self.make_folders_for(rel, held)?;
        let made = self
            .folders
            .copy_file(&source, rel, held)
            .map_err(|error| self.cant_write(rel, &error))?;
        self.wrote(rel);
        // Some disks copy a file's time with it and some don't. One that
        // can't be given the time is copied again by the next build.
        if let (Some(changed), Some(made)) = (changed, made)
            && made
                .metadata()
                .is_ok_and(|made| made.modified().ok() != Some(changed))
        {
            let _ = made.set_modified(changed);
        }
        self.copied(rel, from_stamp);
        Ok(())
    }

    /// Whether the folder, written where it stands, holds at `rel` the
    /// copy the build before left of a file that had the stamp `from` is.
    /// Such a copy is this build's as it stands: nothing of it is read,
    /// written or removed.
    fn stands_as_left(&self, rel: &str, from: Option<Stamp>) -> bool {
        let (true, Some(notes), Some(spare), Some(from)) =
            (self.in_place, &self.notes, &self.spare, from)
        else {
            return false;
        };
        if notes.before.get(rel) != Some(&from) {
            return false;
        }
        let unasked = spare.files.get(rel).is_some_and(|state| {
            let asked = state.compare_exchange(0, WANTED, Ordering::Relaxed, Ordering::Relaxed);
            asked.is_ok()
        });
        if unasked {
            self.copied(rel, Some(from));
        }
        unasked
    }

    /// Where `rel` goes in the folder written into, and whether the spare
    /// held a file there, which is this build's from now on. Room is made
    /// for a name the spare didn't hold.
    fn claim(&self, rel: &str) -> Result<bool, Problem> {
        self.claimed(rel, true)
    }

    /// [`Staging::claim`], which `asks` whether the way to `rel` goes
    /// through a link; the caller does where it doesn't.
    fn claimed(&self, rel: &str, asks: bool) -> Result<bool, Problem> {
        if !platform::stays_inside(rel) {
            return Err(self.cant_write(rel, &io::Error::other("it is outside the built site")));
        }
        // A folder written where it stands is not listed first, as a
        // spare is, so nothing in it is known to be no link until asked.
        if self.in_place && asks {
            self.no_link_to(rel)
                .map_err(|error| self.cant_write(rel, &error))?;
        }
        let Some(spare) = &self.spare else {
            return Ok(false);
        };
        match spare
            .files
            .get(rel)
            .map(|state| state.swap(WANTED, Ordering::Relaxed))
        {
            Some(0) => Ok(true),
            Some(GONE) => Ok(false),
            Some(_) => {
                let taken = io::Error::from(io::ErrorKind::AlreadyExists);
                Err(self.cant_write(rel, &taken))
            }
            None => {
                // Where the folder is written as it stands, nothing is
                // removed for a new name's sake: a name the disk holds as
                // another's fails to be made, and says so.
                if !self.in_place {
                    spare
                        .make_room(&self.folders, rel)
                        .map_err(|error| self.cant_write(rel, &error))?;
                }
                let mut added = spare.added.lock().expect("never poisoned");
                added.push(rel.to_owned());
                Ok(false)
            }
        }
    }

    /// Makes the folders above `rel`, where the spare didn't hold a file
    /// there.
    fn make_folders_for(&self, rel: &str, held: bool) -> Result<(), Problem> {
        match (held, rel.rsplit_once('/')) {
            (false, Some((above, _))) => self
                .folders
                .make_folders(above)
                .map_err(|error| self.cant_write(above, &error)),
            _ => Ok(()),
        }
    }

    /// Makes the file `rel` anew, as [`platform::Folders::make_new_file`]
    /// makes it, removing the spare's file there first where it `held`
    /// one: nothing already in its place, or in place of a folder on the
    /// way, a link included, is ever written through.
    fn create(&self, rel: &str, held: bool) -> Result<fs::File, Problem> {
        self.make_folders_for(rel, held)?;
        let made = self.folders.make_new_file(rel, held);
        let made = made.map_err(|error| self.cant_write(rel, &error))?;
        self.wrote(rel);
        Ok(made)
    }

    /// Leaves the folder written into holding what this build wrote and
    /// nothing else, once the build has written every file. What is
    /// written there afterwards without this build's hand is published
    /// with the rest.
    pub fn settle(&mut self) -> Result<(), Problem> {
        // What the build itself asked for is told only while the spare
        // still says so.
        if self.settled.is_none() {
            self.settled = self.left();
        }
        match self.spare.take() {
            Some(spare) => spare
                .settle(&self.fresh, &self.folders, &self.makers)
                .map_err(|error| self.cant_write("", &error)),
            None => Ok(()),
        }
    }

    /// Every file the folder written into holds now, each by its path
    /// with forward slashes, with its stamp where the system gives one.
    pub fn files(&self) -> Result<Inventory, Problem> {
        let found = |path: &Path| fs::symlink_metadata(path);
        Ok(Inventory::of(&self.fresh, self.held()?, found))
    }

    /// Every file the folder written into holds now, as [`Staging::files`]
    /// gives them, known well enough for a later look to tell one written
    /// again at its length and time, as [`Inventory::to_compare`] has it.
    pub fn files_to_compare(&self) -> Result<Inventory, Problem> {
        let clock = self.disk_clock();
        let found = |path: &Path| fs::symlink_metadata(path);
        Ok(Inventory::to_compare(
            &self.fresh,
            self.held()?,
            found,
            clock.as_ref(),
        ))
    }

    /// Every file the folder written into holds, by its path from the top.
    fn held(&self) -> Result<Vec<String>, Problem> {
        let held = Held::under(&self.fresh, "").map_err(|error| {
            Problem::nowhere(Kind::CantRead {
                path: self.shown.clone(),
                reason: system_reason(&error),
            })
        })?;
        Ok(held.files)
    }

    /// The clock of the disk the output is on, read from a file written
    /// beside the output under a name of Statac's own and removed: the
    /// folder written into may hold a file of the site's by any name. None
    /// where the disk keeps no time or the file can't be written there.
    fn disk_clock(&self) -> Option<Clock> {
        let mut name = std::ffi::OsString::from(".");
        name.push(self.out.file_name()?);
        name.push(CLOCK);
        Clock::at(&self.out.with_file_name(name)).ok()?
    }

    /// Puts the folder written into in the output's place.
    pub fn finish(self) -> Result<(), Problem> {
        self.finish_by(platform::exchange)
    }

    /// [`Staging::finish`], exchanging the two folders as `exchange` does.
    fn finish_by(
        mut self,
        exchange: impl FnOnce(&Path, &Path) -> io::Result<bool>,
    ) -> Result<(), Problem> {
        let left = self.settled.take().or_else(|| self.left());
        let into = self.notes.take().map(|notes| notes.left);
        self.put_in_place(exchange)?;
        mark_in_place();
        if let Some(into) = into {
            *into.lock().expect("never poisoned") = left;
        }
        Ok(())
    }

    fn put_in_place(
        mut self,
        exchange: impl FnOnce(&Path, &Path) -> io::Result<bool>,
    ) -> Result<(), Problem> {
        if self.in_place {
            let spare = self.spare.take().unwrap_or_default();
            return spare
                .clear(&self.folders)
                .map_err(|error| self.cant_write("", &error));
        }
        self.settle()?;
        if !exists(&self.out) {
            platform::rename(&self.fresh, &self.out)
                .map_err(|error| self.cant_write("", &error))?;
            self.owns_fresh = false;
            return Ok(());
        }
        // Asked again: something may have arrived since the build began.
        self.check_output()?;
        let exchanged = match self.keeps {
            true => exchange(&self.fresh, &self.out),
            false => Ok(false),
        };
        if exchanged.map_err(|error| self.cant_write("", &error))? {
            return Ok(());
        }
        platform::rename(&self.out, &self.aside).map_err(|error| self.cant_write("", &error))?;
        if let Err(error) = platform::rename(&self.fresh, &self.out) {
            // Should this fail too, the next build finds the old output
            // beside the place it belongs and puts it back.
            let _ = platform::rename(&self.aside, &self.out);
            return Err(self.cant_write("", &error));
        }
        self.owns_fresh = false;
        // The new site is published; what is left is tidying. An old
        // output that can't be kept as the spare is removed.
        if self.keeps && platform::rename(&self.aside, &self.fresh).is_ok() {
            return Ok(());
        }
        platform::remove_folder(&self.aside).map_err(|error| {
            Problem::nowhere(Kind::CantRemove {
                path: self.aside.clone(),
                reason: system_reason(&error),
            })
        })
    }

    fn check_output(&self) -> Result<(), Problem> {
        match occupant(&self.out) {
            Ok(None) => Ok(()),
            Ok(Some(found)) => Err(Problem::nowhere(Kind::NotStatacsFolder {
                path: self.shown.clone(),
                found,
            })),
            Err(error) => Err(Problem::nowhere(Kind::CantRead {
                path: self.shown.clone(),
                reason: system_reason(&error),
            })),
        }
    }

    /// Removes a folder an earlier build left beside the output, if it is
    /// one Statac made.
    fn clear(&self, leftover: &Path) -> Result<(), Problem> {
        match occupant(leftover) {
            Ok(None) if exists(leftover) => {}
            Ok(None) => return Ok(()),
            Ok(Some(found)) => {
                return Err(Problem::nowhere(Kind::NotStatacsFolder {
                    path: leftover.to_path_buf(),
                    found,
                }));
            }
            Err(error) => {
                return Err(Problem::nowhere(Kind::CantRead {
                    path: leftover.to_path_buf(),
                    reason: system_reason(&error),
                }));
            }
        }
        platform::remove_folder(leftover).map_err(|error| {
            Problem::nowhere(Kind::CantRemove {
                path: leftover.to_path_buf(),
                reason: system_reason(&error),
            })
        })
    }

    /// A failure to write `rel`, named where the author will find it.
    fn cant_write(&self, rel: &str, error: &io::Error) -> Problem {
        Problem::nowhere(Kind::CantWrite {
            path: platform::join(&self.shown, rel),
            reason: system_reason(error),
        })
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        // The spare stays as far as the build got with it: nothing in it
        // is taken on trust, so the next build puts right what differs.
        if self.owns_fresh {
            let _ = platform::remove_folder(&self.fresh);
        }
    }
}

/// What stands at `path` that makes it not Statac's to replace: `None` for
/// nothing at all, an empty folder, or a folder with the marker.
fn occupant(path: &Path) -> io::Result<Option<Occupant>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    if metadata.is_symlink() {
        return Ok(Some(Occupant::Link));
    }
    if !metadata.is_dir() {
        return Ok(Some(Occupant::File));
    }
    if ours(path) {
        return Ok(None);
    }
    let (mut files, mut folders) = (0, 0);
    for entry in fs::read_dir(path)? {
        match entry?.file_type()?.is_dir() {
            true => folders += 1,
            false => files += 1,
        }
    }
    Ok((files + folders > 0).then_some(Occupant::Entries { files, folders }))
}

fn ours(folder: &Path) -> bool {
    fs::symlink_metadata(folder.join(MARKER)).is_ok_and(|marker| marker.is_file())
}

fn exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

/// `path` with `.` and `..` walked, so `out/..` names the folder it means.
pub(crate) fn lexically_normal(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Scratch;
    use std::time::{Duration, SystemTime};

    /// A scratch folder holding an empty `site/`.
    fn scratch_with_site(name: &str) -> Scratch {
        let scratch = Scratch::new(name);
        fs::create_dir_all(scratch.site()).unwrap();
        scratch
    }

    /// A site in `site/` of a scratch folder, built into `out/`.
    trait Publishing {
        fn site(&self) -> PathBuf;
        fn out(&self) -> PathBuf;
        fn spare(&self) -> PathBuf;
        fn begin(&self) -> Staging;
        /// Builds a site of `files`, each a path and its text.
        fn build(&self, files: &[(&str, &str)]);
    }

    impl Publishing for Scratch {
        fn site(&self) -> PathBuf {
            self.0.join("site")
        }

        fn out(&self) -> PathBuf {
            self.0.join("out")
        }

        fn spare(&self) -> PathBuf {
            self.site().join(KEPT).join(SPARE)
        }

        fn begin(&self) -> Staging {
            begin(&self.site(), &self.out()).unwrap()
        }

        fn build(&self, files: &[(&str, &str)]) {
            let staging = self.begin();
            for (rel, text) in files {
                staging.write(rel, text.as_bytes()).unwrap();
            }
            staging.finish().unwrap();
        }
    }

    /// What a build that finished left, sorted.
    fn taken(into: &Arc<Mutex<Option<Left>>>) -> Left {
        let mut left = into.lock().unwrap().take().expect("a build that finished");
        left.files.sort();
        left.written.sort();
        left.removed.sort();
        left
    }

    #[test]
    fn a_build_resumed_where_the_last_stands_writes_only_what_differs() {
        let scratch = scratch_with_site("resumed");
        let into = Arc::new(Mutex::new(None));
        let staging = scratch.begin().leaving(&scratch.site(), Arc::clone(&into));
        for (rel, text) in [
            ("index.html", "home"),
            ("a/index.html", "a"),
            ("b/c.txt", "c"),
        ] {
            staging.write(rel, text.as_bytes()).unwrap();
        }
        staging.finish().unwrap();
        let left = taken(&into);
        assert_eq!(
            left.files,
            [MARKER, "a/index.html", "b/c.txt", "index.html"]
        );

        // The home page is no file of this build's, and stays unasked for.
        let mine = ["a/index.html", "b/c.txt"].map(str::to_owned);
        let staging = resume(
            &scratch.site(),
            &scratch.out(),
            mine,
            &left,
            Arc::clone(&into),
        );
        staging.write("a/index.html", b"a").unwrap();
        staging.write("d/index.html", b"d").unwrap();
        staging.finish().unwrap();
        let left = taken(&into);
        assert_eq!(left.files, ["a/index.html", "d/index.html"]);
        assert_eq!(left.written, ["d/index.html"]);
        assert_eq!(left.removed, ["b/c.txt"]);
        assert_eq!(
            tree(&scratch.out()),
            [
                "a/",
                "a/index.html=a",
                "d/",
                "d/index.html=d",
                "index.html=home"
            ]
        );
        assert!(scratch.out().join(MARKER).is_file());
    }

    #[test]
    fn a_build_resumed_leaves_a_file_named_by_what_it_holds_unread() {
        let scratch = scratch_with_site("resumed-named");
        let into = Arc::new(Mutex::new(None));
        let staging = scratch.begin().leaving(&scratch.site(), Arc::clone(&into));
        staging.write_named("search/a.bin", b"a").unwrap();
        staging.write("search/b.bin", b"b").unwrap();
        staging.finish().unwrap();
        let left = taken(&into);
        // Changed behind the builds' backs, only to show which they read.
        for rel in ["search/a.bin", "search/b.bin"] {
            fs::write(scratch.out().join(rel), "changed").unwrap();
        }

        let resumed = |left: &Left| {
            let mine = left.files.iter().filter(|file| *file != MARKER).cloned();
            let staging = resume(
                &scratch.site(),
                &scratch.out(),
                mine,
                left,
                Arc::clone(&into),
            );
            staging.write_named("search/a.bin", b"a").unwrap();
            staging.write_named("search/b.bin", b"b").unwrap();
            staging.finish().unwrap();
            taken(&into)
        };
        // Only the file the build before wrote by its name stands unread.
        let left = resumed(&left);
        assert_eq!(left.files, ["search/a.bin", "search/b.bin"]);
        assert_eq!(left.written, ["search/b.bin"]);
        assert!(left.removed.is_empty(), "{:?}", left.removed);
        assert_eq!(
            tree(&scratch.out().join("search")),
            ["a.bin=changed", "b.bin=b"]
        );

        // Both are known by their names now.
        fs::write(scratch.out().join("search/b.bin"), "changed").unwrap();
        let left = resumed(&left);
        assert!(left.written.is_empty(), "{:?}", left.written);
        assert_eq!(
            tree(&scratch.out().join("search")),
            ["a.bin=changed", "b.bin=changed"]
        );
    }

    /// A site of three pages built into the output, and what it left.
    fn three_pages(scratch: &Scratch, into: &Arc<Mutex<Option<Left>>>) -> Left {
        let staging = scratch.begin().leaving(&scratch.site(), Arc::clone(into));
        for (rel, text) in [
            ("index.html", "home"),
            ("a/index.html", "a page"),
            ("a/b/index.html", "b"),
        ] {
            staging.write(rel, text.as_bytes()).unwrap();
        }
        staging.finish().unwrap();
        taken(into)
    }

    #[test]
    fn a_build_resumed_writes_a_file_over_what_it_held_whatever_its_length() {
        let scratch = scratch_with_site("resumed-over");
        let into = Arc::new(Mutex::new(None));
        let left = three_pages(&scratch, &into);
        let mine = left.files.clone();
        let staging = resume(
            &scratch.site(),
            &scratch.out(),
            mine,
            &left,
            Arc::clone(&into),
        );
        staging.write("index.html", b"HOME").unwrap();
        staging.write("a/index.html", b"a").unwrap();
        staging.write("a/b/index.html", b"b, and more").unwrap();
        staging.finish().unwrap();
        let left = taken(&into);
        assert_eq!(
            left.written,
            ["a/b/index.html", "a/index.html", "index.html"]
        );
        assert!(left.removed.is_empty(), "{:?}", left.removed);
        assert_eq!(
            tree(&scratch.out()),
            [
                "a/",
                "a/b/",
                "a/b/index.html=b, and more",
                "a/index.html=a",
                "index.html=HOME"
            ]
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_build_resumed_writes_a_file_over_without_making_it_anew() {
        use std::os::unix::fs::MetadataExt;
        let scratch = scratch_with_site("resumed-same-file");
        let into = Arc::new(Mutex::new(None));
        let left = three_pages(&scratch, &into);
        let page = scratch.out().join("a/index.html");
        let was = fs::metadata(&page).unwrap().ino();
        let mine = left.files.clone();
        let staging = resume(
            &scratch.site(),
            &scratch.out(),
            mine,
            &left,
            Arc::clone(&into),
        );
        staging.write("a/index.html", b"another").unwrap();
        staging.finish().unwrap();
        assert_eq!(fs::read(&page).unwrap(), b"another");
        assert_eq!(fs::metadata(&page).unwrap().ino(), was);
    }

    #[cfg(unix)]
    #[test]
    fn a_build_resumed_writes_through_no_second_name_of_a_file() {
        let scratch = scratch_with_site("resumed-named-twice");
        let outside = scratch.0.join("outside.txt");
        fs::write(&outside, "mine").unwrap();
        let into = Arc::new(Mutex::new(None));
        let left = three_pages(&scratch, &into);
        let page = scratch.out().join("a/index.html");
        fs::remove_file(&page).unwrap();
        fs::hard_link(&outside, &page).unwrap();
        let mine = left.files.clone();
        let staging = resume(
            &scratch.site(),
            &scratch.out(),
            mine,
            &left,
            Arc::clone(&into),
        );
        staging.write("a/index.html", b"another").unwrap();
        staging.finish().unwrap();
        assert_eq!(fs::read(&page).unwrap(), b"another");
        assert_eq!(fs::read(&outside).unwrap(), b"mine");
    }

    #[cfg(unix)]
    #[test]
    fn a_build_resumed_writes_through_no_link_where_a_file_or_its_folder_was() {
        use std::os::unix::fs::symlink;
        let scratch = scratch_with_site("resumed-links");
        let outside = scratch.0.join("outside");
        fs::create_dir_all(outside.join("b")).unwrap();
        fs::write(outside.join("index.html"), "mine").unwrap();
        fs::write(outside.join("b/index.html"), "mine").unwrap();
        let into = Arc::new(Mutex::new(None));
        let left = three_pages(&scratch, &into);
        let out = scratch.out();
        fs::remove_file(out.join("index.html")).unwrap();
        symlink(outside.join("index.html"), out.join("index.html")).unwrap();
        fs::remove_dir_all(out.join("a/b")).unwrap();
        symlink(outside.join("b"), out.join("a/b")).unwrap();
        let mine = left.files.clone();
        let staging = resume(&scratch.site(), &out, mine, &left, Arc::clone(&into));
        assert!(staging.write("index.html", b"HOME").is_err());
        assert!(staging.write("a/b/index.html", b"B").is_err());
        assert_eq!(fs::read(outside.join("index.html")).unwrap(), b"mine");
        assert_eq!(fs::read(outside.join("b/index.html")).unwrap(), b"mine");
    }

    #[test]
    fn a_build_resumed_takes_a_copy_as_equal_by_what_it_was_copied_from() {
        let scratch = scratch_with_site("resumed-copy");
        let from = scratch.site().join("file-1.txt");
        fs::write(&from, "one").unwrap();
        change(&from, long_ago());
        crate::disk::wait_until_settled(&scratch.site(), &from);
        let into = Arc::new(Mutex::new(None));
        let staging = scratch.begin().leaving(&scratch.site(), Arc::clone(&into));
        staging
            .copy(fs::File::open(&from), "file-1.txt", "file-1.txt")
            .unwrap();
        staging.finish().unwrap();
        let left = taken(&into);

        // Changed where only its bytes could say so: the copy is trusted.
        let copied = scratch.out().join("file-1.txt");
        fs::write(&copied, "ONE").unwrap();
        let mine = ["file-1.txt".to_owned()];
        let staging = resume(
            &scratch.site(),
            &scratch.out(),
            mine.clone(),
            &left,
            Arc::clone(&into),
        );
        staging
            .copy(fs::File::open(&from), "file-1.txt", "file-1.txt")
            .unwrap();
        staging.finish().unwrap();
        let left = taken(&into);
        assert!(left.written.is_empty());
        assert_eq!(fs::read_to_string(&copied).unwrap(), "ONE");

        fs::write(&from, "one and more").unwrap();
        let staging = resume(
            &scratch.site(),
            &scratch.out(),
            mine,
            &left,
            Arc::clone(&into),
        );
        staging
            .copy(fs::File::open(&from), "file-1.txt", "file-1.txt")
            .unwrap();
        staging.finish().unwrap();
        assert_eq!(taken(&into).written, ["file-1.txt"]);
        assert_eq!(fs::read_to_string(&copied).unwrap(), "one and more");
    }

    #[test]
    fn a_build_resumed_reads_a_copy_whose_file_changed_as_the_last_build_began() {
        let scratch = scratch_with_site("resumed-copy-same-step");
        let from = scratch.site().join("file-1.txt");
        fs::write(&from, "one").unwrap();
        // Changed within the step the disk's times were in as the build
        // began, as a disk whose times move in steps sees it.
        let now = SystemTime::now() + Duration::from_secs(3600);
        change(&from, now);
        let into = Arc::new(Mutex::new(None));
        let staging = scratch.begin().leaving(&scratch.site(), Arc::clone(&into));
        staging
            .copy(fs::File::open(&from), "file-1.txt", "file-1.txt")
            .unwrap();
        staging.finish().unwrap();
        let left = taken(&into);
        assert!(!exists(&scratch.site().join(KEPT).join(CLOCK)));

        // Written again at the same length, within the same step.
        fs::write(&from, "two").unwrap();
        change(&from, now);
        let mine = ["file-1.txt".to_owned()];
        let staging = resume(
            &scratch.site(),
            &scratch.out(),
            mine,
            &left,
            Arc::clone(&into),
        );
        staging
            .copy(fs::File::open(&from), "file-1.txt", "file-1.txt")
            .unwrap();
        staging.finish().unwrap();
        assert_eq!(taken(&into).written, ["file-1.txt"]);
        let copied = scratch.out().join("file-1.txt");
        assert_eq!(fs::read_to_string(&copied).unwrap(), "two");
    }

    fn names(folder: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(folder)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    /// Everything under `folder` as the disk spells it: each file with its
    /// text, each folder with a slash, each link with an arrow.
    fn tree(folder: &Path) -> Vec<String> {
        fn inside(folder: &Path, rel: &str, found: &mut Vec<String>) {
            for name in names(folder) {
                let (path, at) = (folder.join(&name), joined(rel, &name));
                let kind = fs::symlink_metadata(&path).unwrap().file_type();
                if kind.is_dir() {
                    found.push(format!("{at}/"));
                    inside(&path, &at, found);
                } else if kind.is_file() {
                    found.push(format!("{at}={}", fs::read_to_string(&path).unwrap()));
                } else {
                    found.push(format!("{at}->"));
                }
            }
        }
        let mut found = Vec::new();
        inside(folder, "", &mut found);
        found.retain(|entry| !entry.starts_with(MARKER));
        found
    }

    fn changed(path: &Path) -> SystemTime {
        fs::symlink_metadata(path).unwrap().modified().unwrap()
    }

    fn change(path: &Path, to: SystemTime) {
        let file = fs::OpenOptions::new().write(true).open(path).unwrap();
        file.set_modified(to).unwrap();
    }

    fn long_ago() -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000)
    }

    #[test]
    fn a_file_written_again_is_told_changed_though_its_length_and_time_stay() {
        let scratch = scratch_with_site("compared");
        let staging = scratch.begin();
        let at = |rel: &str| staging.root().join(rel);
        for rel in ["kept.txt", "same.txt", "rewritten.txt", "old.txt"] {
            staging.write(rel, b"one\n").unwrap();
        }
        // A file of the site's by the name the disk's time is read from.
        staging.write(CLOCK, b"mine\n").unwrap();
        // Written within the step the disk's times are in, as a disk whose
        // times move in steps sees them, or long before.
        let now = SystemTime::now() + Duration::from_secs(3600);
        for rel in ["same.txt", "rewritten.txt"] {
            change(&at(rel), now);
        }
        for rel in ["kept.txt", "old.txt", CLOCK] {
            change(&at(rel), long_ago());
        }
        let before = staging.files_to_compare().unwrap();
        assert_eq!(fs::read(at(CLOCK)).unwrap(), b"mine\n");
        assert!(!exists(&scratch.0.join(format!(".out{CLOCK}"))));
        // Written again at the same length, within the same step, which a
        // file of that step left as it is is not.
        fs::write(at("rewritten.txt"), b"two\n").unwrap();
        change(&at("rewritten.txt"), now);
        fs::write(at("old.txt"), b"two\n").unwrap();
        let after = staging.files().unwrap();
        let mut changed = after.changed_since(&before);
        changed.sort_unstable();
        assert_eq!(changed, ["old.txt", "rewritten.txt"]);
    }

    #[test]
    fn a_first_build_makes_the_output_with_its_marker() {
        let scratch = scratch_with_site("first");
        let out = scratch.out();
        let staging = scratch.begin();
        staging.write("assets/site.css", b"body {}").unwrap();
        assert!(!out.exists());
        staging.finish().unwrap();
        assert_eq!(names(&out), [MARKER, "assets"]);
        assert_eq!(fs::read(out.join("assets/site.css")).unwrap(), b"body {}");
        assert_eq!(names(&scratch.0), ["out", "site"]);
        assert_eq!(names(&scratch.site().join(KEPT)), [] as [&str; 0]);
    }

    #[test]
    fn a_later_build_replaces_the_whole_output_and_keeps_the_old_as_the_spare() {
        let scratch = scratch_with_site("later");
        scratch.build(&[("post-1.html", "one")]);
        scratch.build(&[("post-2.html", "two")]);
        assert_eq!(tree(&scratch.out()), ["post-2.html=two"]);
        assert_eq!(tree(&scratch.spare()), ["post-1.html=one"]);
        assert!(scratch.spare().join(MARKER).is_file());
        assert_eq!(names(&scratch.0), ["out", "site"]);
    }

    #[test]
    fn a_build_into_the_spare_writes_only_what_differs() {
        let scratch = scratch_with_site("differs");
        let first = [("a.html", "a"), ("b/c.html", "c"), ("d/e.html", "e")];
        scratch.build(&first);
        for file in ["a.html", "b/c.html"] {
            change(&scratch.out().join(file), long_ago());
        }
        scratch.build(&first);
        // The first build's output is the spare now.
        scratch.build(&[("a.html", "a"), ("b/c.html", "C"), ("f.html", "f")]);
        let out = scratch.out();
        assert_eq!(tree(&out), ["a.html=a", "b/", "b/c.html=C", "f.html=f"]);
        assert_eq!(changed(&out.join("a.html")), long_ago());
        assert_ne!(changed(&out.join("b/c.html")), long_ago());
        assert_eq!(tree(&scratch.spare()).len(), first.len() + 2);
    }

    #[test]
    fn an_address_written_twice_is_refused_whether_or_not_the_spare_held_it() {
        let scratch = scratch_with_site("twice");
        scratch.build(&[("a.html", "a")]);
        scratch.build(&[("a.html", "a")]);
        let staging = scratch.begin();
        for rel in ["a.html", "b.html"] {
            staging.write(rel, b"once").unwrap();
            let Kind::CantWrite { path, .. } = staging.write(rel, b"twice").unwrap_err().kind
            else {
                panic!("{rel}");
            };
            assert_eq!(path, scratch.out().join(rel));
        }
    }

    #[test]
    fn a_failed_build_leaves_the_output_untouched_and_the_spare_for_the_next() {
        let scratch = scratch_with_site("failed");
        scratch.build(&[("post-1.html", "one")]);
        scratch.build(&[("post-1.html", "one")]);
        let failed = scratch.begin();
        failed.write("post-1.html", b"ONE").unwrap();
        failed.write("post-2.html", b"two").unwrap();
        drop(failed);
        assert_eq!(tree(&scratch.out()), ["post-1.html=one"]);
        assert_eq!(names(&scratch.0), ["out", "site"]);
        // As far as the failed build got with it.
        assert_eq!(
            tree(&scratch.spare()),
            ["post-1.html=ONE", "post-2.html=two"]
        );
        scratch.build(&[("post-1.html", "one"), ("post-3.html", "three")]);
        assert_eq!(
            tree(&scratch.out()),
            ["post-1.html=one", "post-3.html=three"]
        );
        assert_eq!(tree(&scratch.spare()), ["post-1.html=one"]);
    }

    #[test]
    fn a_folder_of_someone_elses_files_is_never_replaced() {
        let scratch = scratch_with_site("theirs");
        let out = scratch.0.join("docs");
        fs::create_dir_all(out.join("drafts")).unwrap();
        fs::write(out.join("post-1.md"), "one").unwrap();
        fs::write(out.join("post-2.md"), "two").unwrap();
        let refused = begin(&scratch.site(), &out).unwrap_err();
        assert_eq!(
            refused.kind,
            Kind::NotStatacsFolder {
                path: out.clone(),
                found: Occupant::Entries {
                    files: 2,
                    folders: 1
                },
            }
        );
        assert_eq!(names(&out), ["drafts", "post-1.md", "post-2.md"]);
        assert_eq!(names(&scratch.0), ["docs", "site"]);
        assert_eq!(names(&scratch.site()), [] as [&str; 0]);
    }

    #[test]
    fn a_file_where_the_output_should_be_is_refused() {
        let scratch = scratch_with_site("file");
        let out = scratch.out();
        fs::write(&out, "notes").unwrap();
        let refused = begin(&scratch.site(), &out).unwrap_err();
        assert!(matches!(
            refused.kind,
            Kind::NotStatacsFolder {
                found: Occupant::File,
                ..
            }
        ));
        assert_eq!(fs::read(&out).unwrap(), b"notes");
    }

    #[test]
    fn an_empty_folder_is_taken() {
        let scratch = scratch_with_site("empty");
        let out = scratch.out();
        fs::create_dir(&out).unwrap();
        scratch.build(&[("index.html", "home")]);
        assert_eq!(names(&out), [MARKER, "index.html"]);
    }

    #[test]
    fn what_an_interrupted_build_left_is_cleared_or_put_back() {
        let scratch = scratch_with_site("interrupted");
        let out = scratch.out();
        scratch.build(&[("post-1.html", "one")]);
        // As if a build stopped after moving the output aside.
        let old = scratch.0.join(".out.statac-old");
        fs::rename(&out, &old).unwrap();
        let fresh = scratch.0.join(".out.statac-new");
        fs::create_dir(&fresh).unwrap();
        fs::write(fresh.join(MARKER), "").unwrap();

        let staging = scratch.begin();
        assert_eq!(names(&out), [MARKER, "post-1.html"]);
        drop(staging);
        assert_eq!(names(&scratch.0), ["out", "site"]);
    }

    #[test]
    fn an_old_output_left_aside_becomes_the_spare() {
        let scratch = scratch_with_site("aside");
        scratch.build(&[("post-1.html", "one")]);
        // As if a build stopped after moving the new output in.
        let old = scratch.0.join(".out.statac-old");
        fs::create_dir(&old).unwrap();
        fs::write(old.join(MARKER), "").unwrap();
        fs::write(old.join("post-0.html"), "none").unwrap();
        scratch.build(&[("post-1.html", "one")]);
        assert_eq!(names(&scratch.0), ["out", "site"]);
        assert_eq!(tree(&scratch.out()), ["post-1.html=one"]);
        assert_eq!(tree(&scratch.spare()), ["post-1.html=one"]);
    }

    #[test]
    fn a_stranger_beside_the_output_is_left_alone() {
        let scratch = scratch_with_site("stranger");
        let fresh = scratch.0.join(".out.statac-new");
        fs::create_dir(&fresh).unwrap();
        fs::write(fresh.join("notes.txt"), "mine").unwrap();
        let refused = begin(&scratch.site(), &scratch.out()).unwrap_err();
        assert!(matches!(refused.kind, Kind::NotStatacsFolder { ref path, .. } if *path == fresh));
        assert_eq!(names(&fresh), ["notes.txt"]);
    }

    #[test]
    fn failures_name_the_output_as_the_author_wrote_it() {
        let scratch = scratch_with_site("named");
        let staging = scratch.begin();
        // A file where a folder is needed.
        staging.write("assets", b"").unwrap();
        let failed = staging.write("assets/site.css", b"").unwrap_err();
        match failed.kind {
            Kind::CantWrite { path, .. } => assert_eq!(path, scratch.out().join("assets")),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn dots_in_the_output_path_are_walked() {
        assert_eq!(
            lexically_normal(Path::new("/site/./out/../public")),
            Path::new("/site/public")
        );
    }

    #[test]
    fn where_the_folders_cant_be_exchanged_they_are_moved() {
        let scratch = scratch_with_site("moved");
        scratch.build(&[("post-1.html", "one")]);
        for (nth, text) in [(2, "two"), (3, "three")] {
            let staging = scratch.begin();
            staging
                .write(&format!("post-{nth}.html"), text.as_bytes())
                .unwrap();
            staging.finish_by(|_, _| Ok(false)).unwrap();
        }
        assert_eq!(tree(&scratch.out()), ["post-3.html=three"]);
        assert_eq!(tree(&scratch.spare()), ["post-2.html=two"]);
        assert_eq!(names(&scratch.0), ["out", "site"]);
    }

    #[test]
    fn an_exchange_that_fails_leaves_the_output_as_it_was() {
        let scratch = scratch_with_site("unexchanged");
        scratch.build(&[("post-1.html", "one")]);
        let staging = scratch.begin();
        staging.write("post-2.html", b"two").unwrap();
        let failed =
            staging.finish_by(|_, _| Err(io::Error::from(io::ErrorKind::PermissionDenied)));
        assert!(matches!(failed.unwrap_err().kind, Kind::CantWrite { .. }));
        assert_eq!(tree(&scratch.out()), ["post-1.html=one"]);
    }

    #[test]
    fn an_output_inside_what_statac_keeps_has_its_spare_beside_it() {
        let scratch = scratch_with_site("inside");
        let kept = scratch.site().join(KEPT);
        let out = kept.join("out");
        for text in ["one", "two"] {
            let staging = begin(&scratch.site(), &out).unwrap();
            staging.write("post.html", text.as_bytes()).unwrap();
            staging.finish().unwrap();
        }
        assert_eq!(names(&kept), [".out.statac-spare", "out"]);
        assert_eq!(tree(&out), ["post.html=two"]);
        assert_eq!(tree(&kept.join(".out.statac-spare")), ["post.html=one"]);
    }

    #[test]
    fn something_else_under_the_spares_name_is_left_alone_and_said() {
        let scratch = scratch_with_site("occupied");
        let spare = scratch.spare();
        fs::create_dir_all(&spare).unwrap();
        fs::write(spare.join("notes.txt"), "mine").unwrap();
        for _ in 0..2 {
            let staging = scratch.begin();
            assert_eq!(
                staging.unkept().map(|problem| problem.kind),
                Some(Kind::CacheNotWritten {
                    path: Path::new(KEPT).join(SPARE),
                    reason: "something else has its name".into(),
                })
            );
            staging.write("post-1.html", b"one").unwrap();
            staging.finish().unwrap();
        }
        assert_eq!(tree(&scratch.out()), ["post-1.html=one"]);
        assert_eq!(tree(&spare), ["notes.txt=mine"]);
        assert_eq!(names(&scratch.0), ["out", "site"]);
    }

    /// What `before` becomes when `after` is built into a spare holding
    /// it, and what a build of `after` with no spare gives.
    fn rebuilt(name: &str, before: &[(&str, &str)], after: &[(&str, &str)]) -> [Vec<String>; 2] {
        let scratch = scratch_with_site(name);
        scratch.build(before);
        scratch.build(before);
        scratch.build(after);
        let fresh = scratch_with_site(&format!("{name}-fresh"));
        fresh.build(after);
        [tree(&scratch.out()), tree(&fresh.out())]
    }

    #[test]
    fn a_name_respelt_or_become_a_folder_or_a_file_is_as_a_fresh_build_spells_it() {
        type Files<'a> = &'a [(&'a str, &'a str)];
        let cases: [(&str, Files, Files); 9] = [
            ("file", &[("Post.html", "a")], &[("post.html", "a")]),
            (
                "folder",
                &[
                    ("Blog/post.html", "a"),
                    ("Blog/Sub/b.html", "b"),
                    ("Blog/old.html", "c"),
                ],
                &[
                    ("blog/post.html", "a"),
                    ("blog/sub/b.html", "b"),
                    ("blog/new.html", "d"),
                ],
            ),
            (
                "above",
                &[("Blog/sub/a.html", "a")],
                &[("blog/sub/a.html", "a")],
            ),
            (
                "below",
                &[("blog/Sub/a.html", "a"), ("blog/b.html", "b")],
                &[("blog/sub/a.html", "a"), ("blog/b.html", "b")],
            ),
            (
                "to-folder",
                &[("about", "a")],
                &[("about/index.html", "a"), ("about/b/c.html", "c")],
            ),
            (
                "to-file",
                &[("about/index.html", "a"), ("about/b/c.html", "c")],
                &[("about", "a")],
            ),
            (
                "respelt-to-folder",
                &[("About", "a")],
                &[("about/index.html", "a")],
            ),
            (
                "respelt-to-file",
                &[("About/index.html", "a")],
                &[("about", "a")],
            ),
            (
                "accent",
                &[("caf\u{e9}/a.html", "a")],
                &[("cafe\u{301}/a.html", "a")],
            ),
        ];
        for (name, before, after) in cases {
            let [rebuilt, fresh] = rebuilt(&format!("respelt-{name}"), before, after);
            assert_eq!(rebuilt, fresh, "{name}");
        }
    }

    #[test]
    fn names_change_kind_while_many_files_are_written_at_once() {
        // Each of sixty names is a file in one build and a folder of
        // three in the next, among pages that stay as they are.
        let build = |scratch: &Scratch, kind: usize| {
            let staging = scratch.begin();
            let written = (0..600usize).into_par_iter().try_for_each(|nth| {
                let at = format!("{}/post-{nth}", nth % 7);
                match (nth.is_multiple_of(10), (nth / 10 + kind).is_multiple_of(2)) {
                    (true, true) => staging.write(&at, b"file"),
                    (true, false) => (0..3).try_for_each(|below| {
                        staging.write(&format!("{at}/{below}/index.html"), b"folder")
                    }),
                    (false, _) => staging.write(&format!("{at}/index.html"), b"page"),
                }
            });
            written.unwrap();
            staging.finish().unwrap();
        };
        let scratch = scratch_with_site("at-once");
        let fresh = [
            scratch_with_site("at-once-0"),
            scratch_with_site("at-once-1"),
        ];
        for (kind, fresh) in fresh.iter().enumerate() {
            build(fresh, kind);
        }
        for kind in [0, 0, 1, 0, 1, 1, 0] {
            build(&scratch, kind);
            assert_eq!(tree(&scratch.out()), tree(&fresh[kind].out()), "{kind}");
        }
    }

    #[test]
    fn what_was_put_in_the_output_by_hand_is_gone_two_builds_on() {
        let scratch = scratch_with_site("by-hand");
        let files = [("a.html", "a"), ("b/c.html", "c")];
        scratch.build(&files);
        let out = scratch.out();
        fs::write(out.join("a.html"), "edited").unwrap();
        fs::write(out.join("b/added.html"), "added").unwrap();
        fs::create_dir_all(out.join("empty/folder")).unwrap();
        fs::create_dir_all(out.join("full")).unwrap();
        fs::write(out.join("full/added.html"), "added").unwrap();
        fs::remove_file(out.join("b/c.html")).unwrap();
        let built = ["a.html=a", "b/", "b/c.html=c"];
        scratch.build(&files);
        assert_eq!(tree(&out), built);
        scratch.build(&files);
        assert_eq!(tree(&out), built);
        assert_eq!(tree(&scratch.spare()), built);
    }

    /// A scratch folder with a copy Statac keeps, built twice so the spare
    /// holds it published.
    fn with_a_copy(name: &str) -> (Scratch, PathBuf) {
        let scratch = scratch_with_site(name);
        let kept = scratch.0.join("kept.avif");
        fs::write(&kept, "copy").unwrap();
        change(&kept, long_ago());
        for _ in 0..2 {
            publish_copy(&scratch, &kept);
        }
        (scratch, kept)
    }

    fn publish_copy(scratch: &Scratch, kept: &Path) {
        let staging = scratch.begin();
        staging
            .copy_whole(fs::File::open(kept), "kept.avif", "a/copy.avif")
            .unwrap();
        staging.finish().unwrap();
    }

    #[test]
    fn a_copy_is_published_with_the_time_of_the_copy_kept() {
        let (scratch, kept) = with_a_copy("copy-time");
        assert_eq!(tree(&scratch.out()), ["a/", "a/copy.avif=copy"]);
        assert_eq!(changed(&scratch.out().join("a/copy.avif")), changed(&kept));
    }

    #[test]
    fn a_copy_in_the_spare_changed_in_length_or_time_is_copied_again() {
        let (scratch, kept) = with_a_copy("copy-changed");
        let published = scratch.spare().join("a/copy.avif");
        // Longer, at the same time.
        fs::write(&published, "edited").unwrap();
        change(&published, long_ago());
        publish_copy(&scratch, &kept);
        assert_eq!(tree(&scratch.out()), ["a/", "a/copy.avif=copy"]);
        // As long, at another time.
        fs::write(scratch.spare().join("a/copy.avif"), "COPY").unwrap();
        publish_copy(&scratch, &kept);
        assert_eq!(tree(&scratch.out()), ["a/", "a/copy.avif=copy"]);
        // The copy kept made again, as long, at another time.
        fs::write(&kept, "made").unwrap();
        publish_copy(&scratch, &kept);
        assert_eq!(tree(&scratch.out()), ["a/", "a/copy.avif=made"]);
        assert_eq!(changed(&scratch.out().join("a/copy.avif")), changed(&kept));
    }

    // The one thing in the spare taken on trust. Comparing every copy's
    // bytes would cost a build of a site with thousands of photographs
    // most of what the spare saves it.
    #[test]
    fn a_copy_in_the_spare_changed_with_its_length_and_time_put_back_is_published() {
        let (scratch, kept) = with_a_copy("copy-put-back");
        let published = scratch.spare().join("a/copy.avif");
        fs::write(&published, "COPY").unwrap();
        change(&published, long_ago());
        publish_copy(&scratch, &kept);
        assert_eq!(tree(&scratch.out()), ["a/", "a/copy.avif=COPY"]);
        // The output before it was never changed, and is the spare now.
        publish_copy(&scratch, &kept);
        assert_eq!(tree(&scratch.out()), ["a/", "a/copy.avif=copy"]);
        // With the changed one removed, every build publishes it right.
        remove(&scratch.spare()).unwrap();
        for _ in 0..3 {
            publish_copy(&scratch, &kept);
            assert_eq!(tree(&scratch.out()), ["a/", "a/copy.avif=copy"]);
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_copy_in_the_spare_that_is_another_name_for_a_file_is_copied_again() {
        let (scratch, kept) = with_a_copy("copy-named-twice");
        let outside = scratch.0.join("outside.avif");
        fs::write(&outside, "mine").unwrap();
        change(&outside, long_ago());
        let published = scratch.spare().join("a/copy.avif");
        fs::remove_file(&published).unwrap();
        fs::hard_link(&outside, &published).unwrap();
        publish_copy(&scratch, &kept);
        assert_eq!(tree(&scratch.out()), ["a/", "a/copy.avif=copy"]);
        assert_eq!(fs::read(&outside).unwrap(), b"mine");
    }

    #[test]
    fn a_file_copied_is_compared_by_its_bytes() {
        let scratch = scratch_with_site("copied");
        let from = scratch.0.join("from.txt");
        let publish = |text: &str| {
            fs::write(&from, text).unwrap();
            change(&from, long_ago());
            let staging = scratch.begin();
            staging
                .copy(fs::File::open(&from), "from.txt", "to.txt")
                .unwrap();
            staging.finish().unwrap();
        };
        publish("one");
        publish("one");
        let first = changed(&scratch.spare().join("to.txt"));
        publish("one");
        assert_eq!(changed(&scratch.out().join("to.txt")), first);
        // As long, changed at the same time: told apart by its bytes.
        publish("two");
        publish("two");
        assert_eq!(tree(&scratch.out()), ["to.txt=two"]);
        publish("three");
        assert_eq!(tree(&scratch.out()), ["to.txt=three"]);
    }

    #[test]
    fn two_files_are_the_same_only_byte_for_byte() {
        let scratch = scratch_with_site("bytes");
        let long = "a".repeat(200_000);
        let other = format!("{}b", &long[1..]);
        let texts = ["", "a", "b", "ab", long.as_str(), other.as_str()];
        for (nth, text) in texts.iter().enumerate() {
            fs::write(scratch.0.join(nth.to_string()), text).unwrap();
        }
        for (a, left) in texts.iter().enumerate() {
            for (b, right) in texts.iter().enumerate() {
                let mut file = fs::File::open(scratch.0.join(a.to_string())).unwrap();
                let other = fs::File::open(scratch.0.join(b.to_string())).unwrap();
                let same = same_bytes(&mut file, other).unwrap();
                assert_eq!(same, left == right, "{a} {b}");
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn links_put_in_the_output_are_removed_and_what_they_lead_to_kept() {
        use std::os::unix::fs::symlink;
        let scratch = scratch_with_site("link-inside");
        let kept = scratch.0.join("kept");
        fs::create_dir(&kept).unwrap();
        fs::write(kept.join("notes.txt"), "mine").unwrap();
        fs::write(kept.join("post-1.html"), "mine").unwrap();
        let out = scratch.out();
        let files = [("post-1.html", "one"), ("posts/post-2.html", "two")];
        scratch.build(&files);
        // Links beside the files, and in the place of a file and a folder.
        let plant = |out: &Path| {
            symlink(&kept, out.join("folder")).unwrap();
            symlink(kept.join("notes.txt"), out.join("file")).unwrap();
            fs::remove_file(out.join("post-1.html")).unwrap();
            symlink(kept.join("post-1.html"), out.join("post-1.html")).unwrap();
            fs::remove_dir_all(out.join("posts")).unwrap();
            symlink(&kept, out.join("posts")).unwrap();
        };
        plant(&out);
        let built = ["post-1.html=one", "posts/", "posts/post-2.html=two"];
        for _ in 0..3 {
            scratch.build(&files);
            assert_eq!(tree(&out), built);
            plant(&out);
        }
        scratch.build(&files);
        assert_eq!(tree(&scratch.spare()).len(), 4);
        scratch.build(&files);
        assert_eq!(tree(&scratch.spare()), built);
        assert_eq!(names(&kept), ["notes.txt", "post-1.html"]);
        assert_eq!(fs::read(kept.join("notes.txt")).unwrap(), b"mine");
        assert_eq!(fs::read(kept.join("post-1.html")).unwrap(), b"mine");
    }

    #[cfg(unix)]
    #[test]
    fn a_page_in_the_spare_that_is_another_name_for_a_file_is_not_written_through() {
        let scratch = scratch_with_site("named-twice");
        let outside = scratch.0.join("outside.txt");
        fs::write(&outside, "mine").unwrap();
        scratch.build(&[("post-1.html", "one")]);
        scratch.build(&[("post-1.html", "one")]);
        let page = scratch.spare().join("post-1.html");
        fs::remove_file(&page).unwrap();
        fs::hard_link(&outside, &page).unwrap();
        scratch.build(&[("post-1.html", "one")]);
        assert_eq!(tree(&scratch.out()), ["post-1.html=one"]);
        assert_eq!(fs::read(&outside).unwrap(), b"mine");
    }

    #[cfg(unix)]
    #[test]
    fn a_link_where_the_spare_is_kept_is_never_followed() {
        use std::os::unix::fs::symlink;
        for (name, link) in [("link-spare", SPARE), ("link-kept", "")] {
            let scratch = scratch_with_site(name);
            let outside = scratch.0.join("outside");
            fs::create_dir(&outside).unwrap();
            fs::write(outside.join(MARKER), "").unwrap();
            fs::write(outside.join("notes.txt"), "mine").unwrap();
            let kept = scratch.site().join(KEPT);
            match link {
                "" => symlink(&outside, &kept).unwrap(),
                link => {
                    fs::create_dir(&kept).unwrap();
                    symlink(&outside, kept.join(link)).unwrap();
                }
            }
            for nth in 1..4 {
                let staging = scratch.begin();
                assert_eq!(
                    staging.unkept().map(|problem| problem.kind),
                    Some(Kind::CacheNotWritten {
                        path: Path::new(KEPT).join(SPARE),
                        reason: "it is a link".into(),
                    })
                );
                let text = nth.to_string();
                staging.write("post.html", text.as_bytes()).unwrap();
                staging.finish().unwrap();
                assert_eq!(tree(&scratch.out()), [format!("post.html={text}")]);
                assert_eq!(names(&scratch.0), ["out", "outside", "site"]);
                assert_eq!(names(&outside), [MARKER, "notes.txt"]);
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_folder_of_the_spare_swapped_for_a_link_after_listing_is_never_gone_through() {
        let scratch = scratch_with_site("spare-swapped");
        let files = [("a/b.html", "b"), ("a/c.html", "c")];
        scratch.build(&files);
        scratch.build(&files);
        let outside = scratch.0.join("outside");
        fs::create_dir(&outside).unwrap();
        for name in ["b.html", "c.html"] {
            fs::write(outside.join(name), "mine").unwrap();
        }
        let staging = scratch.begin();
        let folder = scratch.spare().join("a");
        fs::rename(&folder, scratch.0.join("moved")).unwrap();
        std::os::unix::fs::symlink(&outside, &folder).unwrap();
        let refused = |at: &str| {
            Problem::nowhere(Kind::CantWrite {
                path: scratch.out().join(at),
                reason: "it is a link".into(),
            })
        };
        let written = staging.write("a/b.html", b"new");
        assert_eq!(written, Err(refused("a/b.html")));
        let written = staging.write("a/d.html", b"new");
        assert_eq!(written, Err(refused("a")));
        // What the build didn't ask for is removed as it settles.
        assert_eq!(staging.finish(), Err(refused("")));
        assert_eq!(names(&outside), ["b.html", "c.html"]);
        for name in ["b.html", "c.html"] {
            assert_eq!(fs::read(outside.join(name)).unwrap(), b"mine");
        }
    }

    #[cfg(unix)]
    #[test]
    fn an_output_that_is_a_link_is_refused_not_followed() {
        use std::os::unix::fs::symlink;
        let scratch = scratch_with_site("link-output");
        let real = scratch.0.join("real");
        let first = begin(&scratch.site(), &real).unwrap();
        first.write("post-1.html", b"one").unwrap();
        first.finish().unwrap();
        let out = scratch.out();
        symlink(&real, &out).unwrap();
        let refused = begin(&scratch.site(), &out).unwrap_err();
        assert!(matches!(
            refused.kind,
            Kind::NotStatacsFolder {
                found: Occupant::Link,
                ..
            }
        ));
        assert_eq!(names(&real), [MARKER, "post-1.html"]);
        assert!(platform::remove_folder(&out).is_err());
        assert_eq!(names(&real), [MARKER, "post-1.html"]);
    }

    #[cfg(unix)]
    #[test]
    fn nothing_is_written_through_a_link_already_in_its_place() {
        use std::os::unix::fs::symlink;
        let scratch = scratch_with_site("link-in-place");
        let kept = scratch.0.join("kept.txt");
        fs::write(&kept, "mine").unwrap();
        let staging = scratch.begin();
        symlink(&kept, staging.root().join("index.html")).unwrap();
        assert!(staging.write("index.html", b"page").is_err());
        assert_eq!(fs::read(&kept).unwrap(), b"mine");
    }

    #[test]
    fn a_path_that_climbs_out_is_never_written() {
        let scratch = scratch_with_site("climb");
        let staging = scratch.begin();
        for rel in [
            "../escaped.html",
            "a/../../escaped.html",
            "/escaped.html",
            "C:/x.html",
        ] {
            assert!(staging.write(rel, b"page").is_err(), "{rel}");
        }
        assert_eq!(names(&scratch.0), ["site"]);
        assert_eq!(names(&scratch.site()), [KEPT]);
        assert_eq!(names(&scratch.site().join(KEPT)), [SPARE]);
        assert_eq!(names(&scratch.spare()), [MARKER]);
    }

    #[test]
    fn a_second_hold_on_a_site_says_once_that_it_waits_and_waits_for_the_first() {
        let scratch = scratch_with_site("held");
        let (first, unheld) = hold(&scratch.site(), || panic!("nothing holds the site"));
        assert!(unheld.is_none());
        assert_eq!(names(&scratch.site().join(KEPT)), [LOCK]);
        let (said, heard) = std::sync::mpsc::channel();
        let (held, taken) = std::sync::mpsc::channel();
        std::thread::scope(|scope| {
            scope.spawn(|| {
                let (second, unheld) = hold(&scratch.site(), || said.send(()).unwrap());
                held.send(unheld).unwrap();
                drop(second);
            });
            heard.recv().unwrap();
            let waits = std::time::Duration::from_millis(200);
            assert!(taken.recv_timeout(waits).is_err());
            drop(first);
            assert!(taken.recv().unwrap().is_none());
        });
        assert!(heard.try_recv().is_err());
        let (_again, unheld) = hold(&scratch.site(), || panic!("nothing holds the site"));
        assert!(unheld.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn a_site_that_cant_be_held_goes_unheld_and_says_why() {
        use std::os::unix::fs::symlink;
        for (name, link) in [("link-lock", LOCK), ("link-kept-lock", "")] {
            let scratch = scratch_with_site(name);
            let outside = scratch.0.join("outside");
            fs::create_dir(&outside).unwrap();
            fs::write(outside.join("notes.txt"), "mine").unwrap();
            let kept = scratch.site().join(KEPT);
            match link {
                "" => symlink(&outside, &kept).unwrap(),
                link => {
                    fs::create_dir(&kept).unwrap();
                    symlink(outside.join("notes.txt"), kept.join(link)).unwrap();
                }
            }
            let (_hold, unheld) = hold(&scratch.site(), || panic!("nothing holds the site"));
            assert_eq!(
                unheld.map(|problem| problem.kind),
                Some(Kind::CacheNotWritten {
                    path: Path::new(KEPT).join(LOCK),
                    reason: "it is a link".into(),
                })
            );
            assert_eq!(names(&outside), ["notes.txt"]);
            assert_eq!(fs::read(outside.join("notes.txt")).unwrap(), b"mine");
        }
        let scratch = scratch_with_site("file-kept-lock");
        fs::write(scratch.site().join(KEPT), "").unwrap();
        let (_hold, unheld) = hold(&scratch.site(), || panic!("nothing holds the site"));
        assert_eq!(
            unheld.map(|problem| problem.kind),
            Some(Kind::CacheNotWritten {
                path: Path::new(KEPT).join(LOCK),
                reason: "a file is in the way of a folder".into(),
            })
        );
    }

    #[test]
    fn only_so_many_take_a_turn_at_once() {
        let turns = Turns::new(2);
        let (inside, most) = (Mutex::new(0), Mutex::new(0));
        std::thread::scope(|scope| {
            for _ in 0..8 {
                scope.spawn(|| {
                    for _ in 0..50 {
                        let _turn = turns.take();
                        let now = {
                            let mut inside = inside.lock().unwrap();
                            *inside += 1;
                            *inside
                        };
                        let mut most = most.lock().unwrap();
                        *most = (*most).max(now);
                        drop(most);
                        std::thread::yield_now();
                        *inside.lock().unwrap() -= 1;
                    }
                });
            }
        });
        assert!((1..=2).contains(&*most.lock().unwrap()));
        assert_eq!(*turns.free.lock().unwrap(), 2);
    }
}
