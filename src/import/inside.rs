//! The guard every reader keeps: nothing outside the old site's folder is
//! read, and no link in it is followed.
//!
//! Every path a reader opens is checked here first, piece by piece: one
//! that climbs out of the old folder, or that passes through a link, is
//! never opened, and the link is told of as what it is. A link put in
//! place between the check and the read is the gap the build has too, and
//! isn't closed here.

use std::path::{Path, PathBuf};

use super::FindingKind;
use crate::platform;

pub(super) static FOLDER_UNREAD: FindingKind = FindingKind {
    heading: "Settings naming a folder Statac doesn't read",
    sentence: "Each of these settings names a folder outside the old site's own folder, or one \
               reached through a link. Statac reads nothing outside the old site and follows no \
               link in it, so each was read as if it weren't set: bring across by hand what the \
               folder holds, if it is wanted.",
};

/// Where a link that isn't followed leads, as the end of a sentence about
/// it.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(super) enum Refused {
    Inside,
    Outside,
    Missing,
}

impl Refused {
    pub(super) fn what(self) -> &'static str {
        match self {
            Refused::Inside => {
                "a link that leads somewhere inside the old site, which Statac doesn't follow"
            }
            Refused::Outside => {
                "a link that leads outside the old site, which Statac doesn't follow"
            }
            Refused::Missing => "a link to something that isn't there",
        }
    }
}

/// Whether `what` says a file or folder left behind is a link that wasn't
/// followed.
pub(super) fn is_link(what: &str) -> bool {
    [Refused::Inside, Refused::Outside, Refused::Missing]
        .iter()
        .any(|refused| refused.what() == what)
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

/// The real file or folder `path`, in the old folder, is, where no piece
/// of it climbs out or is a link.
fn real(folder: &Path, path: &str) -> Result<PathBuf, Stopped> {
    let top = std::fs::canonicalize(folder).map_err(|_| Stopped::Climbs)?;
    let mut at = top.clone();
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
        if found.is_symlink() {
            let leads = match std::fs::canonicalize(&next) {
                Ok(target) if target.starts_with(&top) => Refused::Inside,
                Ok(_) => Refused::Outside,
                Err(_) => Refused::Missing,
            };
            return Err(Stopped::Link(walked, leads));
        }
        at = next;
    }
    Ok(at)
}

/// The path to open for `path`, in the old folder. Where a link is on the
/// way, a path nothing can be opened at.
pub(super) fn path(folder: &Path, path: &str) -> PathBuf {
    match real(folder, path) {
        Ok(real) | Err(Stopped::Gone(real)) => real,
        Err(_) => PathBuf::new(),
    }
}

/// What stands at `path`, in the old folder: the file or folder, or the
/// link itself, for the reader to tell of with [`why`].
pub(super) fn metadata(folder: &Path, path: &str) -> std::io::Result<std::fs::Metadata> {
    match real(folder, path) {
        Ok(real) => std::fs::symlink_metadata(real),
        Err(Stopped::Link(link, _)) if link == path.trim_matches('/') => {
            let (above, name) = link.rsplit_once('/').unwrap_or(("", &link));
            let above = real(folder, above).map_err(|_| std::io::ErrorKind::NotFound)?;
            std::fs::symlink_metadata(above.join(name))
        }
        Err(_) => Err(std::io::ErrorKind::NotFound.into()),
    }
}

/// What to say of the link at `path`.
pub(super) fn why(folder: &Path, path: &str) -> &'static str {
    match real(folder, path) {
        Err(Stopped::Link(_, refused)) => refused.what(),
        _ => Refused::Missing.what(),
    }
}

/// Whether `path`, in the old folder, is reached through no link.
pub(super) fn reached(folder: &Path, path: &str) -> bool {
    !matches!(real(folder, path), Err(Stopped::Link(..)))
}

/// Whether `path`, in the old folder, is a file reached through no link.
pub(super) fn a_file(folder: &Path, path: &str) -> bool {
    real(folder, path).is_ok_and(|real| real.is_file())
}

/// Whether `path`, in the old folder, is a folder reached through no
/// link.
pub(super) fn a_folder(folder: &Path, path: &str) -> bool {
    real(folder, path).is_ok_and(|real| real.is_dir())
}

/// The folder a setting names, by its path in the old folder, empty for
/// the old folder itself. None where it isn't inside the old folder as
/// written, or is reached through a link: the setting is then read as not
/// set, and told of under [`FOLDER_UNREAD`].
pub(super) fn named_folder(folder: &Path, written: &str) -> Option<String> {
    let inside = written.strip_prefix("./").unwrap_or(written);
    let inside = match inside.trim_end_matches('/') {
        "." => "",
        // All slashes: the top of the disk.
        "" if !inside.is_empty() => return None,
        inside => inside,
    };
    (platform::stays_inside(inside) && reached(folder, inside)).then(|| inside.to_owned())
}
