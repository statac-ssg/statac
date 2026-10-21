//! Removing a folder through open handles to the folders inside it, so a
//! name is only ever looked up in a folder already open, and a link put in
//! place of a folder fails to open rather than being followed.

use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::io;
use std::os::fd::{AsFd, BorrowedFd, OwnedFd};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, UNIX_EPOCH};

use rayon::prelude::*;
use rustix::fs::{
    AtFlags, CWD, Dir, FileType, Mode, OFlags, fchmod, fstat, openat, renameat, statat, unlinkat,
};

use super::{Over, Refused, Unwritten};
use rustix::io::Errno;

/// How many threads remove files at once. A disk removes them no faster
/// for more asking; the rest would only spin while they wait.
const REMOVERS: usize = 4;

/// `work`, run on the few threads that remove files.
fn removing<T: Send>(work: impl FnOnce() -> T + Send) -> T {
    static POOL: OnceLock<Option<rayon::ThreadPool>> = OnceLock::new();
    let pool = POOL.get_or_init(|| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(REMOVERS)
            .build()
            .ok()
    });
    match pool {
        Some(pool) => pool.install(work),
        None => work(),
    }
}

/// [`super::exchange`], by the system's one call for it.
#[cfg(any(target_vendor = "apple", target_os = "linux", target_os = "android"))]
pub(super) fn exchange(a: &Path, b: &Path) -> io::Result<bool> {
    use rustix::fs::{RenameFlags, renameat_with};
    match renameat_with(CWD, a, CWD, b, RenameFlags::EXCHANGE) {
        Ok(()) => Ok(true),
        // A system too old to have the call, or a disk that can't do it.
        Err(Errno::NOSYS | Errno::INVAL | Errno::NOTSUP) => Ok(false),
        Err(error) => Err(error.into()),
    }
}

/// Removes the folder at `path` and everything in it, many entries at once:
/// one at a time, a large site takes longer to remove than to build.
pub(super) fn remove_folder(path: &Path) -> io::Result<()> {
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::from(Refused::NamesNoFolder))?;
    let parent = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => Some(openat(
            CWD,
            parent,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
        )?),
        _ => None,
    };
    let at = parent.as_ref().map_or(CWD, AsFd::as_fd);
    let folder = open_folder(at, name)?;
    removing(|| empty(folder))?;
    Ok(unlinkat(at, name, AtFlags::REMOVEDIR)?)
}

/// Removes everything in the open folder `folder`.
fn empty(folder: OwnedFd) -> io::Result<()> {
    let mut listing = Dir::new(folder)?;
    let mut entries = Vec::new();
    while let Some(entry) = listing.read() {
        let entry = entry?;
        let name = entry.file_name();
        if name != c"." && name != c".." {
            entries.push((CString::from(name), entry.file_type()));
        }
    }
    let at = listing.fd()?;
    entries
        .par_iter()
        .try_for_each(|(name, kind)| remove_entry(at, name, *kind))
}

/// Removes `name` in the open folder `at`, which its listing gave as `kind`.
fn remove_entry(at: BorrowedFd<'_>, name: &CStr, kind: FileType) -> io::Result<()> {
    if matches!(kind, FileType::Directory | FileType::Unknown) {
        match open_folder(at, name) {
            Ok(folder) => {
                empty(folder)?;
                return Ok(unlinkat(at, name, AtFlags::REMOVEDIR)?);
            }
            // A link, or no longer a folder: removed as what it is.
            Err(Errno::LOOP | Errno::NOTDIR) => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(unlinkat(at, name, AtFlags::empty())?)
}

/// What stands at `name` in `at`, where it would not open: a link is said
/// to be one, and anything else gives the system's own `error`.
fn refusal(at: BorrowedFd<'_>, name: &Path, error: Errno) -> io::Error {
    match statat(at, name, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(found) if FileType::from_raw_mode(found.st_mode) == FileType::Symlink => {
            io::Error::from(Refused::Link)
        }
        _ => error.into(),
    }
}

/// The folder holding `rel`, a path with forward slashes under `root`,
/// and the last name in `rel`: each folder opened by its name in the
/// folder above it, already open, and never through a link. None where
/// `root` or a folder on the way isn't there.
fn holder<'a>(root: &Path, rel: &'a str) -> io::Result<Option<(OwnedFd, &'a Path)>> {
    let mut at = match open_folder(CWD, root) {
        Ok(folder) => folder,
        Err(Errno::NOENT) => return Ok(None),
        Err(error) => return Err(refusal(CWD, root, error)),
    };
    let mut pieces = rel.split('/').peekable();
    while let Some(piece) = pieces.next() {
        let name = Path::new(piece);
        if pieces.peek().is_none() {
            return Ok(Some((at, name)));
        }
        at = match open_folder(at.as_fd(), name) {
            Ok(folder) => folder,
            Err(Errno::NOENT) => return Ok(None),
            Err(error) => return Err(refusal(at.as_fd(), name, error)),
        };
    }
    Ok(None)
}

/// [`super::no_link_to`], by the folder [`holder`] opens.
pub(super) fn no_link_to(root: &Path, rel: &str) -> io::Result<()> {
    // Nothing is there to be a link.
    let Some((at, name)) = holder(root, rel)? else {
        return Ok(());
    };
    match statat(at.as_fd(), name, AtFlags::SYMLINK_NOFOLLOW) {
        Err(Errno::NOENT) => Ok(()),
        Err(error) => Err(error.into()),
        Ok(found) => match FileType::from_raw_mode(found.st_mode) {
            FileType::RegularFile => Ok(()),
            FileType::Symlink => Err(io::Error::from(Refused::Link)),
            _ => Err(io::Error::from(Refused::NotAFile)),
        },
    }
}

/// [`super::remove_file`], by the folder [`holder`] opens.
pub(super) fn remove_file(root: &Path, rel: &str) -> io::Result<()> {
    match holder(root, rel)? {
        Some((at, name)) => remove_file_at(at.as_fd(), name),
        None => Ok(()),
    }
}

/// Removes the file `name` in the open folder `at`, refusing a link or
/// anything else than a file; nothing there is no error.
fn remove_file_at(at: BorrowedFd<'_>, name: &Path) -> io::Result<()> {
    match statat(at, name, AtFlags::SYMLINK_NOFOLLOW) {
        Err(Errno::NOENT) => return Ok(()),
        Err(error) => return Err(error.into()),
        Ok(found) => match FileType::from_raw_mode(found.st_mode) {
            FileType::RegularFile => {}
            FileType::Symlink => return Err(io::Error::from(Refused::Link)),
            _ => return Err(io::Error::from(Refused::NotAFile)),
        },
    }
    match unlinkat(at, name, AtFlags::empty()) {
        Err(Errno::NOENT) => Ok(()),
        done => Ok(done?),
    }
}

/// The folder [`holder`] opens for `rel`, which must be there.
fn holder_there<'a>(root: &Path, rel: &'a str) -> io::Result<(OwnedFd, &'a Path)> {
    holder(root, rel)?.ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
}

/// [`super::make_folder`], by the folder [`holder`] opens.
pub(super) fn make_folder(root: &Path, rel: &str) -> io::Result<()> {
    let (at, name) = holder_there(root, rel)?;
    Ok(rustix::fs::mkdirat(
        at.as_fd(),
        name,
        Mode::from_raw_mode(0o777),
    )?)
}

/// [`super::make_file`], by the folder [`holder`] opens.
pub(super) fn make_file(root: &Path, rel: &str, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write;
    let (at, name) = holder_there(root, rel)?;
    // Made only where nothing has the name, a link included.
    let made = OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let file = openat(at.as_fd(), name, made, Mode::from_raw_mode(0o666))?;
    std::fs::File::from(file).write_all(bytes)
}

/// [`super::read_file`], by the folder [`holder`] opens.
pub(super) fn read_file(root: &Path, rel: &str) -> io::Result<Option<Vec<u8>>> {
    use std::io::Read;
    let Some(mut file) = opened(root, rel)? else {
        return Ok(None);
    };
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(Some(bytes))
}

/// [`super::open_file`], by the folder [`holder`] opens.
pub(super) fn open_file(root: &Path, rel: &str) -> io::Result<std::fs::File> {
    opened(root, rel)?.ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
}

/// [`super::keep_file`], by the folder [`holder`] opens.
pub(super) fn keep_file(root: &Path, rel: &str, fresh: &str, bytes: &[u8]) -> io::Result<()> {
    use std::io::Write;
    let gone = || io::Error::from(io::ErrorKind::NotFound);
    let (at, name) = holder(root, rel)?.ok_or_else(gone)?;
    let at = at.as_fd();
    match statat(at, name, AtFlags::SYMLINK_NOFOLLOW) {
        Err(Errno::NOENT) => {}
        Ok(found)
            if FileType::from_raw_mode(found.st_mode) == FileType::RegularFile
                && found.st_nlink == 1 => {}
        Ok(_) => return Err(io::Error::from(Refused::Taken)),
        Err(error) => return Err(error.into()),
    }
    // Made only where nothing has the name, a link included.
    let made = OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let file = openat(at, fresh, made, Mode::from_raw_mode(0o666))?;
    let written = std::fs::File::from(file)
        .write_all(bytes)
        .and_then(|()| Ok(renameat(at, fresh, at, name)?));
    if written.is_err() {
        let _ = unlinkat(at, fresh, AtFlags::empty());
    }
    written
}

/// [`super::Folders`]: the folders under `root` opened so far, each kept
/// by its path there until [`OPEN_AT_MOST`] are, when the one least
/// lately asked for is let go.
#[derive(Debug)]
pub(super) struct Folders {
    root: PathBuf,
    open: Mutex<Open>,
}

#[derive(Debug, Default)]
struct Open {
    /// Each folder, with when it was last asked for.
    by_path: HashMap<String, (Arc<OwnedFd>, u64)>,
    asked: u64,
}

/// How many folders [`Folders`] keeps open at most: those near the top,
/// which nearly every file is under, and those the files being written
/// now are in, well short of the 256 files a process on macOS may have
/// open unless it asks for more.
const OPEN_AT_MOST: usize = 64;

impl Open {
    fn get(&mut self, rel: &str) -> Option<Arc<OwnedFd>> {
        self.asked += 1;
        let asked = self.asked;
        let (folder, last) = self.by_path.get_mut(rel)?;
        *last = asked;
        Some(Arc::clone(folder))
    }

    fn keep(&mut self, rel: &str, folder: &Arc<OwnedFd>) {
        if self.by_path.len() >= OPEN_AT_MOST {
            let least = (self.by_path.iter()).min_by_key(|(_, (_, last))| *last);
            if let Some(least) = least.map(|(rel, _)| rel.clone()) {
                self.by_path.remove(&least);
            }
        }
        self.asked += 1;
        let kept = (Arc::clone(folder), self.asked);
        self.by_path.insert(rel.to_owned(), kept);
    }
}

impl Folders {
    pub(super) fn new(root: &Path) -> Folders {
        Folders {
            root: root.to_path_buf(),
            open: Mutex::default(),
        }
    }

    /// The folder `rel`, the top itself where it is empty: opened by name
    /// in the nearest folder above it already open, and never through a
    /// link. A folder on the way that isn't there is made where `making`;
    /// none otherwise.
    fn folder(&self, rel: &str, making: bool) -> io::Result<Option<Arc<OwnedFd>>> {
        let kept = {
            let mut open = self.open.lock().expect("never poisoned");
            let mut end = rel.len();
            loop {
                if let Some(folder) = open.get(&rel[..end]) {
                    break Some((folder, end));
                }
                if end == 0 {
                    break None;
                }
                end = rel[..end].rfind('/').unwrap_or(0);
            }
        };
        let (mut at, done) = match kept {
            Some(kept) => kept,
            None => {
                let top = open_folder(CWD, &self.root);
                let top = Arc::new(top.map_err(|error| refusal(CWD, &self.root, error))?);
                self.open.lock().expect("never poisoned").keep("", &top);
                (top, 0)
            }
        };
        let mut end = done;
        for piece in rel[done..].split('/').filter(|piece| !piece.is_empty()) {
            end = rel[end..]
                .find(piece)
                .map_or(rel.len(), |at| end + at + piece.len());
            let name = Path::new(piece);
            let opened = match open_folder(at.as_fd(), name) {
                Err(Errno::NOENT) if !making => return Ok(None),
                Err(Errno::NOENT) => {
                    match rustix::fs::mkdirat(at.as_fd(), name, Mode::from_raw_mode(0o777)) {
                        // Made by another meanwhile.
                        Ok(()) | Err(Errno::EXIST) => open_folder(at.as_fd(), name),
                        Err(error) => Err(error),
                    }
                }
                opened => opened,
            };
            at = Arc::new(opened.map_err(|error| refusal(at.as_fd(), name, error))?);
            let mut open = self.open.lock().expect("never poisoned");
            open.keep(&rel[..end], &at);
        }
        Ok(Some(at))
    }

    /// The folder holding `rel`, as [`Folders::folder`] opens it, and the
    /// last name in `rel`; none where a folder on the way isn't there.
    fn holder<'a>(&self, rel: &'a str) -> io::Result<Option<(Arc<OwnedFd>, &'a Path)>> {
        let (above, name) = rel.rsplit_once('/').unwrap_or(("", rel));
        Ok(self.folder(above, false)?.map(|at| (at, Path::new(name))))
    }

    /// [`Folders::holder`], for `rel` whose folders must be there.
    fn holder_there<'a>(&self, rel: &'a str) -> io::Result<(Arc<OwnedFd>, &'a Path)> {
        self.holder(rel)?
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
    }

    /// Lets go of every folder kept open, once one is removed or renamed:
    /// another may stand at its path now.
    fn forget(&self) {
        self.open.lock().expect("never poisoned").by_path.clear();
    }

    pub(super) fn make_folders(&self, rel: &str) -> io::Result<()> {
        self.folder(rel, true).map(drop)
    }

    pub(super) fn make_new_file(&self, rel: &str, replacing: bool) -> io::Result<std::fs::File> {
        let (at, name) = self.holder_there(rel)?;
        cleared(at.as_fd(), name, replacing)?;
        let file = openat(at.as_fd(), name, MADE, Mode::from_raw_mode(0o666))?;
        Ok(std::fs::File::from(file))
    }

    pub(super) fn copy_file(
        &self,
        source: &std::fs::File,
        rel: &str,
        replacing: bool,
    ) -> io::Result<Option<std::fs::File>> {
        let (at, name) = self.holder_there(rel)?;
        let at = at.as_fd();
        cleared(at, name, replacing)?;
        #[cfg(target_vendor = "apple")]
        {
            use rustix::fs::{CloneFlags, fclonefileat};
            match fclonefileat(source, at, name, CloneFlags::empty()) {
                Ok(()) => {
                    let opened = OFlags::WRONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
                    let made = openat(at, name, opened, Mode::empty()).ok();
                    return Ok(made.map(std::fs::File::from));
                }
                // A disk that shares no bytes between files, or another disk.
                Err(Errno::NOTSUP | Errno::XDEV) => {}
                Err(error) => return Err(error.into()),
            }
        }
        let made = openat(at, name, MADE, Mode::from_raw_mode(0o666))?;
        let mut made = std::fs::File::from(made);
        super::copy_whole(source, &mut made)?;
        made.set_permissions(source.metadata()?.permissions())?;
        Ok(Some(made))
    }

    pub(super) fn open_file(&self, rel: &str) -> io::Result<std::fs::File> {
        let opened = match self.holder(rel)? {
            Some((at, name)) => opened_at(at.as_fd(), name)?,
            None => None,
        };
        opened.ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
    }

    pub(super) fn measure(&self, rel: &str) -> io::Result<super::Measure> {
        let (at, name) = self.holder_there(rel)?;
        measure_at(at.as_fd(), name)
    }

    pub(super) fn remove(&self, rel: &str, alone: bool) -> io::Result<()> {
        let Some((at, name)) = self.holder(rel)? else {
            return Ok(());
        };
        let at = at.as_fd();
        let found = match statat(at, name, AtFlags::SYMLINK_NOFOLLOW) {
            Ok(found) => found,
            Err(Errno::NOENT) => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        if FileType::from_raw_mode(found.st_mode) == FileType::Directory {
            self.forget();
            match open_folder(at, name) {
                Ok(folder) => {
                    match alone {
                        true => empty_alone(folder)?,
                        false => removing(|| empty(folder))?,
                    }
                    return Ok(unlinkat(at, name, AtFlags::REMOVEDIR)?);
                }
                // A link, or no longer a folder: removed as what it is.
                Err(Errno::LOOP | Errno::NOTDIR) => {}
                Err(error) => return Err(error.into()),
            }
        }
        match unlinkat(at, name, AtFlags::empty()) {
            Ok(()) | Err(Errno::NOENT) => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    pub(super) fn remove_file(&self, rel: &str) -> io::Result<()> {
        match self.holder(rel)? {
            Some((at, name)) => remove_file_at(at.as_fd(), name),
            None => Ok(()),
        }
    }

    pub(super) fn remove_empty_folder(&self, rel: &str) -> io::Result<()> {
        let (at, name) = self.holder_there(rel)?;
        self.forget();
        Ok(unlinkat(at.as_fd(), name, AtFlags::REMOVEDIR)?)
    }

    pub(super) fn rename_in(&self, folder: &str, from: &str, to: &str) -> io::Result<()> {
        let gone = || io::Error::from(io::ErrorKind::NotFound);
        let at = self.folder(folder, false)?.ok_or_else(gone)?;
        self.forget();
        Ok(renameat(at.as_fd(), from, at.as_fd(), to)?)
    }
}

/// Removes what stands at `name` in `at`, where `replacing`, as a link
/// itself where it is one; nothing there is no error.
fn cleared(at: BorrowedFd<'_>, name: &Path, replacing: bool) -> io::Result<()> {
    match replacing {
        true => match unlinkat(at, name, AtFlags::empty()) {
            Ok(()) | Err(Errno::NOENT) => Ok(()),
            Err(error) => Err(error.into()),
        },
        false => Ok(()),
    }
}

/// Made only where nothing has the name, a link included.
const MADE: OFlags = OFlags::WRONLY
    .union(OFlags::CREATE)
    .union(OFlags::EXCL)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC);

/// Removes everything in the open folder `folder`, one entry at a time on
/// the calling thread.
fn empty_alone(folder: OwnedFd) -> io::Result<()> {
    let mut listing = Dir::new(folder)?;
    let mut entries = Vec::new();
    while let Some(entry) = listing.read() {
        let entry = entry?;
        let name = entry.file_name();
        if name != c"." && name != c".." {
            entries.push((CString::from(name), entry.file_type()));
        }
    }
    let at = listing.fd()?;
    for (name, kind) in entries {
        if matches!(kind, FileType::Directory | FileType::Unknown) {
            match open_folder(at, name.as_c_str()) {
                Ok(inner) => {
                    empty_alone(inner)?;
                    unlinkat(at, name.as_c_str(), AtFlags::REMOVEDIR)?;
                    continue;
                }
                Err(Errno::LOOP | Errno::NOTDIR) => {}
                Err(error) => return Err(error.into()),
            }
        }
        unlinkat(at, name.as_c_str(), AtFlags::empty())?;
    }
    Ok(())
}

/// [`super::measure`], by the folder [`holder`] opens.
pub(super) fn measure(root: &Path, rel: &str) -> io::Result<super::Measure> {
    let gone = || io::Error::from(io::ErrorKind::NotFound);
    let (at, name) = holder(root, rel)?.ok_or_else(gone)?;
    measure_at(at.as_fd(), name)
}

/// What the file `name` in the open folder `at` is found to be, refusing
/// a link or anything else than a file.
fn measure_at(at: BorrowedFd<'_>, name: &Path) -> io::Result<super::Measure> {
    let found = statat(at, name, AtFlags::SYMLINK_NOFOLLOW)?;
    match FileType::from_raw_mode(found.st_mode) {
        FileType::RegularFile => {}
        FileType::Symlink => return Err(io::Error::from(Refused::Link)),
        _ => return Err(io::Error::from(Refused::NotAFile)),
    }
    // The fields' types differ from one kind of machine to another.
    #[allow(
        clippy::unnecessary_cast,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_possible_wrap
    )]
    let (length, seconds, nanos) = (
        found.st_size as u64,
        found.st_mtime as i64,
        found.st_mtime_nsec as u32,
    );
    // Before 1970 the seconds count back and the nanoseconds still forward.
    let modified = match u64::try_from(seconds) {
        Ok(seconds) => UNIX_EPOCH + Duration::new(seconds, nanos),
        Err(_) => {
            UNIX_EPOCH - Duration::from_secs(seconds.unsigned_abs())
                + Duration::from_nanos(nanos.into())
        }
    };
    Ok(super::Measure {
        length,
        modified,
        named_once: found.st_nlink == 1,
    })
}

/// The file `rel` under `root` opened to read, by name in the folder
/// [`holder`] opens and never through a link, or none where nothing is
/// there.
fn opened(root: &Path, rel: &str) -> io::Result<Option<std::fs::File>> {
    match holder(root, rel)? {
        Some((at, name)) => opened_at(at.as_fd(), name),
        None => Ok(None),
    }
}

/// The file `name` in the open folder `at`, opened to read and never
/// through a link, or none where nothing is there.
fn opened_at(at: BorrowedFd<'_>, name: &Path) -> io::Result<Option<std::fs::File>> {
    // Not waiting for a writer, where a pipe stands at the name.
    let opened = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
    let file = match openat(at, name, opened, Mode::empty()) {
        Ok(file) => file,
        Err(Errno::NOENT) => return Ok(None),
        Err(error) => return Err(refusal(at, name, error)),
    };
    let found = fstat(&file).map_err(io::Error::from)?;
    if FileType::from_raw_mode(found.st_mode) != FileType::RegularFile {
        return Err(io::Error::from(Refused::NotAFile));
    }
    Ok(Some(std::fs::File::from(file)))
}

/// [`super::lock_file`], by name in the folder, opened first and never
/// through a link.
pub(super) fn lock_file(folder: &Path, name: &str) -> io::Result<std::fs::File> {
    let at = open_folder(CWD, folder).map_err(|error| refusal(CWD, folder, error))?;
    let at = at.as_fd();
    // Not waiting for a reader, where a pipe stands at the name.
    let opened = OFlags::WRONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
    let made = opened | OFlags::CREATE | OFlags::EXCL;
    // Opened where it is there and made where it isn't, never in one step:
    // where two ask at once to open a file in a folder already open,
    // making it if need be, macOS can tell one that it isn't there.
    let mut tries = 0;
    let file = loop {
        let found = match openat(at, name, opened, Mode::empty()) {
            Err(Errno::NOENT) => openat(at, name, made, Mode::from_raw_mode(0o666)),
            found => found,
        };
        match found {
            Ok(file) => break file,
            // Made by another meanwhile.
            Err(Errno::EXIST) if tries < 8 => tries += 1,
            Err(error) => return Err(refusal(at, Path::new(name), error)),
        }
    };
    let found = fstat(&file).map_err(io::Error::from)?;
    if FileType::from_raw_mode(found.st_mode) != FileType::RegularFile {
        return Err(io::Error::from(Refused::NotAFile));
    }
    Ok(std::fs::File::from(file))
}

/// [`super::rewrite`], with everything after the walk to the file's
/// folder done by name in that folder, already open, and no link
/// followed: so a folder on the way swapped for a link while this runs
/// can't send the write anywhere else.
pub(super) fn rewrite(root: &Path, rel: &str, read: &[u8], text: &[u8]) -> Result<(), Unwritten> {
    use std::io::{Read, Write};
    let gone = || io::Error::from(io::ErrorKind::NotFound);
    let (at, name) = holder(root, rel)?.ok_or_else(gone)?;
    let at = at.as_fd();
    let opened = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let file = openat(at, name, opened, Mode::empty()).map_err(|error| refusal(at, name, error))?;
    let found = fstat(&file).map_err(io::Error::from)?;
    if FileType::from_raw_mode(found.st_mode) != FileType::RegularFile {
        return Err(io::Error::from(Refused::NotAFile).into());
    }
    if found.st_nlink != 1 {
        return Err(Unwritten::NamedTwice);
    }
    let mut held = Vec::with_capacity(read.len());
    std::fs::File::from(file).read_to_end(&mut held)?;
    if held != read {
        return Err(Unwritten::Changed);
    }
    let beside = super::beside(&name.to_string_lossy());
    // Made only where nothing has the name, a link included.
    let made = OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let new =
        openat(at, beside.as_str(), made, Mode::RUSR | Mode::WUSR).map_err(io::Error::from)?;
    let written = (|| -> io::Result<()> {
        let mut new = std::fs::File::from(new);
        new.write_all(text)?;
        fchmod(&new, Mode::from_raw_mode(found.st_mode))?;
        new.sync_all()?;
        Ok(renameat(at, beside.as_str(), at, name)?)
    })();
    if written.is_err() {
        let _ = unlinkat(at, beside.as_str(), AtFlags::empty());
    }
    Ok(written?)
}

/// [`super::write_over`]: the file is asked what it is through the handle
/// that is then written to. The folder is asked by its path and never
/// opened: opening is most of what writing a small file costs, and a site
/// that keeps each page in a folder of its own would open two for every
/// page.
pub(super) fn write_over(folder: &Path, name: &str, bytes: &[u8]) -> io::Result<Over> {
    use std::io::Read;
    use std::os::unix::fs::FileExt;
    let is = |found: rustix::fs::Stat, kind| FileType::from_raw_mode(found.st_mode) == kind;
    match statat(CWD, folder, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(found) if is(found, FileType::Directory) => {}
        _ => return Ok(Over::Refused),
    }
    let opened = OFlags::RDWR | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let Ok(file) = openat(CWD, folder.join(name), opened, Mode::empty()) else {
        return Ok(Over::Refused);
    };
    let found = fstat(&file).map_err(io::Error::from)?;
    if !is(found, FileType::RegularFile) || found.st_nlink != 1 {
        return Ok(Over::Refused);
    }
    let mut file = std::fs::File::from(file);
    let length = bytes.len() as u64;
    let same_length = u64::try_from(found.st_size) == Ok(length);
    if same_length {
        let mut held = Vec::with_capacity(bytes.len() + 1);
        file.read_to_end(&mut held)?;
        if held == bytes {
            return Ok(Over::Same);
        }
    }
    file.write_all_at(bytes, 0)?;
    if !same_length {
        file.set_len(length)?;
    }
    Ok(Over::Written)
}

fn open_folder<P: rustix::path::Arg>(at: BorrowedFd<'_>, name: P) -> rustix::io::Result<OwnedFd> {
    openat(
        at,
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
}

/// [`super::remote_disk`], by the name the system gives the disk's kind.
#[cfg(target_vendor = "apple")]
pub(super) fn remote_disk(path: &Path) -> Option<String> {
    /// `MNT_LOCAL`.
    const LOCAL: u32 = 0x1000;
    let found = rustix::fs::statfs(path).ok()?;
    let kind: Vec<u8> = found
        .f_fstypename
        .iter()
        .take_while(|&&c| c != 0)
        .map(|&c| c.cast_unsigned())
        .collect();
    let kind = String::from_utf8_lossy(&kind).into_owned();
    super::remote_by_name(&kind, found.f_flags & LOCAL != 0).then_some(kind)
}

/// [`super::remote_disk`], by the number the system gives the disk's kind.
#[cfg(any(target_os = "linux", target_os = "android"))]
pub(super) fn remote_disk(path: &Path) -> Option<String> {
    let found = rustix::fs::statfs(path).ok()?;
    // The number's type differs from one kind of machine to another, and
    // every kind Statac knows fits in 32 bits.
    #[allow(clippy::unnecessary_cast, clippy::cast_possible_truncation)]
    let magic = found.f_type as u64 as u32;
    super::remote_by_number(magic).map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Scratch;
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::path::PathBuf;

    fn names(folder: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(folder)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    /// A folder holding notes, for links to lead to.
    fn kept(scratch: &Scratch) -> PathBuf {
        let kept = scratch.0.join("kept");
        fs::create_dir_all(kept.join("drafts")).unwrap();
        fs::write(kept.join("notes.txt"), "mine").unwrap();
        kept
    }

    #[test]
    fn a_folder_is_removed_whole_whatever_its_shape() {
        let scratch = Scratch::new("shapes");
        let built = scratch.0.join("built");
        for i in 0..200 {
            fs::create_dir_all(built.join(format!("posts/{i}"))).unwrap();
            fs::write(built.join(format!("posts/{i}/index.html")), "page").unwrap();
        }
        fs::create_dir_all(built.join("tags/a/b/c/d")).unwrap();
        fs::write(built.join("tags/a/b/c/index.html"), "page").unwrap();
        fs::create_dir(built.join("empty")).unwrap();
        fs::write(built.join("index.html"), "home").unwrap();
        remove_folder(&built).unwrap();
        assert_eq!(names(&scratch.0), [] as [&str; 0]);
    }

    #[test]
    fn links_at_every_depth_are_removed_as_links() {
        let scratch = Scratch::new("links");
        let kept = kept(&scratch);
        let built = scratch.0.join("built");
        for folder in ["", "posts/1", "tags/a/b"] {
            let folder = built.join(folder);
            fs::create_dir_all(&folder).unwrap();
            symlink(&kept, folder.join("folder")).unwrap();
            symlink(kept.join("notes.txt"), folder.join("file")).unwrap();
        }
        remove_folder(&built).unwrap();
        assert_eq!(names(&scratch.0), ["kept"]);
        assert_eq!(names(&kept), ["drafts", "notes.txt"]);
        assert_eq!(fs::read(kept.join("notes.txt")).unwrap(), b"mine");
    }

    #[test]
    fn a_folder_swapped_for_a_link_after_listing_loses_only_the_link() {
        let scratch = Scratch::new("swapped");
        let kept = kept(&scratch);
        let built = scratch.0.join("built");
        fs::create_dir(&built).unwrap();
        symlink(&kept, built.join("posts")).unwrap();
        let at = open_folder(CWD, &built).unwrap();
        // As the listing saw it before the swap.
        remove_entry(at.as_fd(), c"posts", FileType::Directory).unwrap();
        assert_eq!(names(&built), [] as [&str; 0]);
        assert_eq!(names(&kept), ["drafts", "notes.txt"]);
    }

    #[test]
    fn a_link_is_never_opened_as_the_folder_to_remove() {
        let scratch = Scratch::new("top-link");
        let kept = kept(&scratch);
        let link = scratch.0.join("out");
        symlink(&kept, &link).unwrap();
        assert!(remove_folder(&link).is_err());
        assert!(fs::symlink_metadata(&link).unwrap().is_symlink());
        assert_eq!(names(&kept), ["drafts", "notes.txt"]);
    }

    #[test]
    fn a_file_is_never_opened_to_read_through_a_link() {
        let scratch = Scratch::new("open-link");
        let kept = kept(&scratch);
        let folder = scratch.0.join("folder");
        fs::create_dir(&folder).unwrap();
        fs::write(folder.join("a.txt"), "a").unwrap();
        symlink(kept.join("notes.txt"), folder.join("notes.txt")).unwrap();
        symlink(&kept, folder.join("linked")).unwrap();
        let read = |rel: &str| io::read_to_string(open_file(&folder, rel)?);
        assert_eq!(read("a.txt").unwrap(), "a");
        for rel in ["notes.txt", "linked/notes.txt"] {
            assert_eq!(
                Refused::of(&read(rel).unwrap_err()),
                Some(Refused::Link),
                "{rel}"
            );
        }
        assert_eq!(read("missing").unwrap_err().kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn a_file_is_measured_through_no_link_and_without_reading_it() {
        use std::os::unix::fs::PermissionsExt;
        let scratch = Scratch::new("times-link");
        let kept = kept(&scratch);
        let folder = scratch.0.join("folder");
        fs::create_dir(&folder).unwrap();
        fs::write(folder.join("a.txt"), "a").unwrap();
        fs::set_permissions(folder.join("a.txt"), fs::Permissions::from_mode(0o000)).unwrap();
        symlink(kept.join("notes.txt"), folder.join("notes.txt")).unwrap();
        symlink(&kept, folder.join("linked")).unwrap();
        assert_eq!(measure(&folder, "a.txt").unwrap().length, 1);
        for rel in ["notes.txt", "linked/notes.txt"] {
            let refused = measure(&folder, rel).unwrap_err();
            assert_eq!(Refused::of(&refused), Some(Refused::Link), "{rel}");
        }
    }

    #[test]
    fn a_kept_file_is_never_written_through_a_link() {
        let scratch = Scratch::new("keep-link");
        let kept = kept(&scratch);
        let folder = scratch.0.join("folder");
        fs::create_dir(&folder).unwrap();
        symlink(&kept, folder.join("linked")).unwrap();
        symlink(kept.join("notes.txt"), folder.join("notes.txt")).unwrap();
        fs::write(folder.join("named.txt"), "named").unwrap();
        fs::hard_link(folder.join("named.txt"), kept.join("named.txt")).unwrap();
        let refused = keep_file(&folder, "linked/new.txt", "new.fresh", b"new").unwrap_err();
        assert_eq!(Refused::of(&refused), Some(Refused::Link));
        for rel in ["notes.txt", "named.txt"] {
            let refused = keep_file(&folder, rel, "fresh", b"new").unwrap_err();
            assert_eq!(Refused::of(&refused), Some(Refused::Taken), "{rel}");
        }
        assert_eq!(names(&kept), ["drafts", "named.txt", "notes.txt"]);
        assert_eq!(fs::read(kept.join("notes.txt")).unwrap(), b"mine");
        assert_eq!(fs::read(kept.join("named.txt")).unwrap(), b"named");
        assert_eq!(names(&folder), ["linked", "named.txt", "notes.txt"]);
    }

    #[test]
    fn a_folder_kept_open_is_never_a_link_put_in_its_place() {
        let scratch = Scratch::new("folders");
        let kept = kept(&scratch);
        let top = scratch.0.join("top");
        fs::create_dir_all(top.join("a")).unwrap();
        fs::create_dir_all(top.join("b")).unwrap();
        let folders = Folders::new(&top);
        folders.make_new_file("a/1.txt", false).unwrap();
        // Kept open, it is written into wherever it is moved.
        fs::rename(top.join("a"), scratch.0.join("moved")).unwrap();
        symlink(&kept, top.join("a")).unwrap();
        folders.make_new_file("a/2.txt", false).unwrap();
        assert_eq!(names(&scratch.0.join("moved")), ["1.txt", "2.txt"]);
        // Never opened, it is refused.
        fs::rename(top.join("b"), scratch.0.join("b")).unwrap();
        symlink(&kept, top.join("b")).unwrap();
        let refused = folders.make_new_file("b/1.txt", false).unwrap_err();
        assert_eq!(Refused::of(&refused), Some(Refused::Link));
        assert_eq!(names(&kept), ["drafts", "notes.txt"]);
        // Far more folders than are kept open, and one removed and made
        // again under its path.
        for nth in 0..OPEN_AT_MOST * 3 {
            folders.make_folders(&format!("c/{nth}")).unwrap();
            folders.make_new_file(&format!("c/{nth}/x"), false).unwrap();
        }
        assert_eq!(names(&top.join("c")).len(), OPEN_AT_MOST * 3);
        folders.remove("c", false).unwrap();
        folders.make_folders("c/0").unwrap();
        folders.make_new_file("c/0/x", false).unwrap();
        assert_eq!(names(&top.join("c")), ["0"]);
    }

    #[test]
    fn a_lock_file_is_never_opened_or_made_through_a_link() {
        let scratch = Scratch::new("lock-link");
        let kept = kept(&scratch);
        let folder = scratch.0.join("folder");
        fs::create_dir(&folder).unwrap();
        symlink(kept.join("notes.txt"), folder.join("lock")).unwrap();
        symlink(kept.join("absent"), folder.join("dangling")).unwrap();
        for name in ["lock", "dangling"] {
            let refused = lock_file(&folder, name).unwrap_err();
            assert_eq!(Refused::of(&refused), Some(Refused::Link), "{name}");
        }
        symlink(&kept, scratch.0.join("linked")).unwrap();
        let refused = lock_file(&scratch.0.join("linked"), "lock").unwrap_err();
        assert_eq!(Refused::of(&refused), Some(Refused::Link));
        assert_eq!(names(&kept), ["drafts", "notes.txt"]);
        assert_eq!(fs::read(kept.join("notes.txt")).unwrap(), b"mine");
    }
}
