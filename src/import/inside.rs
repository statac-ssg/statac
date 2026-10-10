//! The guard every reader keeps: nothing outside the old site's folder is
//! read, and nothing the old site didn't publish is published by way of a
//! link.
//!
//! A link in the old site is followed only where it leads to a place
//! inside the same old site that the reader itself brought across, as a
//! page or a file to publish, when it read the site with no link followed:
//! that first reading is the reader's own word on what the site
//! published, by every rule it keeps, so none of those rules is written
//! again here. What is read through a link is taken as standing where the
//! link stands, and each file and folder beneath is held to the same
//! test. A folder a setting names, and a settings file, are not
//! published, and may be reached through a link that leads anywhere
//! inside the old site that isn't hidden.
//!
//! Every path a reader opens is resolved here first, piece by piece, to
//! the real file or folder it leads to, and it is that real path the
//! reader opens, not the path by its name again: a link put in its place
//! after the check is not read through. A link changed between its check
//! and the read of what it led to is the gap the build has too, and isn't
//! closed here.

use std::cell::RefCell;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use super::FindingKind;
use crate::platform;

pub static FOLDER_UNREAD: FindingKind = FindingKind {
    heading: "Settings naming a folder Statac doesn't read",
    sentence: "Each of these settings names a folder outside the old site's own folder, or one \
               reached through a link that leads outside it. Statac reads nothing outside the \
               old site, so each was read as if it weren't set: bring across by hand what the \
               folder holds, if it is wanted.",
};

/// How many links one path is followed through.
const DEEPEST: usize = 8;

/// How many links are followed in one import, each counted once by the
/// path it is met at. What a link leads to is read again as standing at
/// the link, so the most one import reads is `FOLLOWED + 1` times what
/// the old folder holds, however its links are laid.
const FOLLOWED: usize = 64;

/// Why a link isn't followed, as the end of a sentence about it.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Refused {
    Outside,
    Back,
    Missing,
    TooMany,
    Unpublished,
}

impl Refused {
    pub fn what(self) -> &'static str {
        match self {
            Refused::Outside => {
                "a link that leads outside the old site, which Statac doesn't follow"
            }
            Refused::Back => {
                "a link that leads back to a folder it is inside, which Statac doesn't follow"
            }
            Refused::Missing => "a link to something that isn't there",
            Refused::Unpublished => {
                "a link that leads to a place the old site didn't publish, which Statac doesn't \
                 follow"
            }
            Refused::TooMany => "a link past the number Statac follows in one site",
        }
    }
}

/// Whether `what` says a file or folder left behind is a link that wasn't
/// followed.
pub fn is_link(what: &str) -> bool {
    [
        Refused::Outside,
        Refused::Back,
        Refused::Missing,
        Refused::TooMany,
        Refused::Unpublished,
    ]
    .iter()
    .any(|refused| refused.what() == what)
}

/// What the guard knows of the import this thread is making.
#[derive(Default)]
struct Import {
    /// The links followed, each by the path it was met at.
    met: Vec<String>,
    /// The links a setting or a settings file's name led through, each
    /// by the path it was met at.
    named: Vec<String>,
    /// Whether the site has been read once with no other link followed.
    read_once: bool,
    /// Whether that reading met a link that may be followed.
    waiting: bool,
    /// Each file that reading brought across to be published, by its
    /// real path, and each folder holding one.
    published: HashSet<PathBuf>,
    holding: HashSet<PathBuf>,
    /// Each place the reader said lies outside the folder it reads, and
    /// is left out by no other rule of the tool's.
    beyond: Vec<PathBuf>,
    /// The new site's folder, where it is there.
    new: Option<PathBuf>,
}

thread_local! {
    static IMPORT: RefCell<Import> = RefCell::new(Import::default());
}

/// Begins an import into `new`: the old site is first read with no link
/// followed but one a setting names.
pub fn begin(new: &Path) {
    IMPORT.with(|import| {
        *import.borrow_mut() = Import {
            new: std::fs::canonicalize(new).ok(),
            ..Import::default()
        };
    });
}

/// Whether the reading so far met a link that leads inside the old site
/// and would be followed if what it leads to was published.
pub fn waiting() -> bool {
    IMPORT.with(|import| import.borrow().waiting)
}

/// Ends the first reading: `published` is each file of the old folder
/// it brought across to be published. From here a link is followed where
/// it leads to one of them, or to a folder holding one.
pub fn read_once(folder: &Path, published: &[&str]) {
    let Ok(top) = std::fs::canonicalize(folder) else {
        return;
    };
    let real: Vec<PathBuf> = (published.iter())
        .filter_map(|path| real(folder, path, false).ok())
        .collect();
    IMPORT.with(|import| {
        let mut import = import.borrow_mut();
        for file in real {
            let mut above = file.parent();
            while let Some(folder) = above.filter(|folder| folder.starts_with(&top)) {
                if !import.holding.insert(folder.to_path_buf()) {
                    break;
                }
                above = folder.parent();
            }
            import.published.insert(file);
        }
        import.read_once = true;
        import.waiting = false;
        import.met.clear();
    });
}

/// Tells the guard that `path`, in the old folder, lies outside the
/// folder the tool reads and is left out by no other rule of the tool's:
/// the old tool read such a place where a link led to it.
pub fn beyond(folder: &Path, path: &str) {
    if let Ok(real) = real(folder, path, false) {
        IMPORT.with(|import| import.borrow_mut().beyond.push(real));
    }
}

/// The links followed, each by its path in the old folder, in order.
pub fn followed() -> Vec<String> {
    IMPORT.with(|import| {
        let mut met = import.borrow().met.clone();
        met.sort();
        met
    })
}

/// Whether `real`, beneath `top`, is a place a link may lead to, or lies
/// beneath one: nothing hidden and no packages folder on the way to it,
/// not the new site, and, where `published` is asked, a file the first
/// reading brought across to publish, a folder holding one, or a place
/// beyond what the tool reads.
fn admitted(import: &Import, top: &Path, real: &Path, published: bool) -> bool {
    let Ok(within) = real.strip_prefix(top) else {
        return false;
    };
    let plain = within.components().all(|piece| {
        let name = piece.as_os_str().to_string_lossy();
        !name.starts_with('.') && name != "node_modules"
    });
    if !plain || import.new.as_ref().is_some_and(|new| real.starts_with(new)) {
        return false;
    }
    !published
        || import.published.contains(real)
        || import.holding.contains(real)
        || import.beyond.iter().any(|beyond| real.starts_with(beyond))
}

/// Why `path` can't be read.
#[derive(Clone, PartialEq, Eq, Debug)]
enum Stopped {
    /// A link at this path in the old folder isn't followed.
    Link(String, Refused),
    /// A piece of it isn't there: the real path it would have.
    Gone(PathBuf),
    /// A piece of it climbs out of the old folder, or the old folder
    /// itself can't be found.
    Climbs,
}

/// The real file or folder `path`, in the old folder, leads to, every
/// link on the way followed where it may be.
///
/// A link is followed where what it leads to, with every link in that
/// resolved in turn, is there and lies within the old folder's own real
/// path, by whole pieces of the path. One that leads to a folder already
/// on the way to it, or to one above such a folder, would be read without
/// end, and isn't followed. `named` says the path is a setting's or a
/// settings file's, which is read and not published: a link on it need
/// lead only to a place that isn't hidden. Any other link must lead to
/// what the site published, and so must each piece beneath it.
fn real(folder: &Path, path: &str, named: bool) -> Result<PathBuf, Stopped> {
    let top = std::fs::canonicalize(folder).map_err(|_| Stopped::Climbs)?;
    let mut at = top.clone();
    // Each real folder passed on the way.
    let mut passed = vec![top.clone()];
    let mut links = 0;
    // Whether a link no setting named has been followed on the way.
    let mut through = false;
    let mut walked = String::new();
    let mut pieces = path.split('/').filter(|piece| !matches!(*piece, "" | "."));
    while let Some(piece) = pieces.next() {
        if piece == ".." {
            return Err(Stopped::Climbs);
        }
        if !walked.is_empty() {
            walked.push('/');
        }
        walked.push_str(piece);
        let next = at.join(piece);
        let Ok(found) = std::fs::symlink_metadata(&next) else {
            // Nothing stands beyond a piece that isn't there, so the rest
            // holds no link.
            let rest: Vec<&str> = pieces.collect();
            return match rest.contains(&"..") {
                true => Err(Stopped::Climbs),
                false => Err(Stopped::Gone(
                    rest.iter().fold(next, |at, piece| at.join(piece)),
                )),
            };
        };
        if !found.is_symlink() {
            if through && !IMPORT.with(|import| admitted(&import.borrow(), &top, &next, true)) {
                return Err(Stopped::Climbs);
            }
            at = next;
            passed.push(at.clone());
            continue;
        }
        let refuse = |why| Err(Stopped::Link(walked.clone(), why));
        let Ok(target) = std::fs::canonicalize(&next) else {
            return refuse(Refused::Missing);
        };
        if !target.starts_with(&top) {
            return refuse(Refused::Outside);
        }
        if passed.iter().any(|folder| folder.starts_with(&target)) {
            return refuse(Refused::Back);
        }
        links += 1;
        let (follow, counted) = IMPORT.with(|import| {
            let mut import = import.borrow_mut();
            let by_name = named || import.named.contains(&walked);
            if !admitted(&import, &top, &target, false) {
                return (false, false);
            }
            if by_name {
                if !import.named.contains(&walked) {
                    import.named.push(walked.clone());
                }
            } else if !import.read_once {
                import.waiting = true;
                return (false, false);
            } else if !admitted(&import, &top, &target, true) {
                return (false, false);
            }
            through |= !by_name;
            let known = import.met.contains(&walked);
            if !known && import.met.len() < FOLLOWED {
                import.met.push(walked.clone());
                return (true, true);
            }
            (true, known)
        });
        if !follow {
            return refuse(Refused::Unpublished);
        }
        if links > DEEPEST || !counted {
            return refuse(Refused::TooMany);
        }
        at = target;
        passed.push(at.clone());
    }
    Ok(at)
}

/// The path to open for `path`, in the old folder: the real file or
/// folder it leads to. Where a link on the way isn't followed, a path
/// nothing can be opened at.
pub fn path(folder: &Path, path: &str) -> PathBuf {
    match real(folder, path, false) {
        Ok(real) | Err(Stopped::Gone(real)) => real,
        Err(_) => PathBuf::new(),
    }
}

/// What stands at `path`, in the old folder: the file or folder it leads
/// to where it is one, or a link followed. A link that isn't followed is
/// given as the link itself, for the reader to tell of with [`why`].
pub fn metadata(folder: &Path, path: &str) -> std::io::Result<std::fs::Metadata> {
    match real(folder, path, false) {
        Ok(real) => std::fs::symlink_metadata(real),
        Err(Stopped::Link(link, _)) if link == path.trim_matches('/') => {
            let (above, name) = link.rsplit_once('/').unwrap_or(("", &link));
            let above = real(folder, above, false).map_err(|_| std::io::ErrorKind::NotFound)?;
            std::fs::symlink_metadata(above.join(name))
        }
        Err(_) => Err(std::io::ErrorKind::NotFound.into()),
    }
}

/// What to say of the link at `path` that isn't followed.
pub fn why(folder: &Path, path: &str) -> &'static str {
    match real(folder, path, false) {
        Err(Stopped::Link(_, refused)) => refused.what(),
        _ => Refused::Missing.what(),
    }
}

/// Whether `path`, a settings file or a file among the settings by its
/// path in the old folder, is reached through no link that isn't
/// followed.
pub fn reached(folder: &Path, path: &str) -> bool {
    !matches!(real(folder, path, true), Err(Stopped::Link(..)))
}

/// Where `path`, in the old folder, really stands, by its path there:
/// another place than `path` where a link was followed on the way.
pub fn stands_at(folder: &Path, path: &str) -> Option<String> {
    let top = std::fs::canonicalize(folder).ok()?;
    let real = real(folder, path, false).ok()?;
    let within = real.strip_prefix(&top).ok()?;
    let pieces: Vec<String> = (within.components())
        .map(|piece| piece.as_os_str().to_string_lossy().into_owned())
        .collect();
    Some(pieces.join("/"))
}

/// Whether `path`, in the old folder, is a file, or leads to one the old
/// site published.
pub fn a_file(folder: &Path, path: &str) -> bool {
    real(folder, path, false).is_ok_and(|real| real.is_file())
}

/// Whether `path`, in the old folder, is a folder, or leads to one
/// holding what the old site published.
pub fn a_folder(folder: &Path, path: &str) -> bool {
    real(folder, path, false).is_ok_and(|real| real.is_dir())
}

/// Whether `path`, a settings file by its fixed name in the old folder,
/// is a file, or leads to one inside the old folder.
pub fn a_settings_file(folder: &Path, path: &str) -> bool {
    real(folder, path, true).is_ok_and(|real| real.is_file())
}

/// Whether `path`, a folder of settings by its fixed name in the old
/// folder, is a folder, or leads to one inside the old folder.
pub fn a_settings_folder(folder: &Path, path: &str) -> bool {
    real(folder, path, true).is_ok_and(|real| real.is_dir())
}

/// The folder a setting names, by its path in the old folder, empty for
/// the old folder itself. None where it isn't inside the old folder as
/// written, or is reached through a link that isn't followed: the setting
/// is then read as not set, and told of under [`FOLDER_UNREAD`].
pub fn named_folder(folder: &Path, written: &str) -> Option<String> {
    let inside = written.strip_prefix("./").unwrap_or(written);
    let inside = match inside.trim_end_matches('/') {
        "." => "",
        // All slashes: the top of the disk.
        "" if !inside.is_empty() => return None,
        inside => inside,
    };
    let reached = !matches!(real(folder, inside, true), Err(Stopped::Link(..)));
    (platform::stays_inside(inside) && reached).then(|| inside.to_owned())
}
