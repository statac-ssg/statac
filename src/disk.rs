//! Telling whether a file has changed since it was read.
//!
//! A file is known by its stamp, and two stamps of it differ where it
//! changed between them, except that a disk's times move in steps, of a
//! few thousandths of a second on some disks and of seconds on others, and
//! a file changed again within the step it was last changed in keeps its
//! times. So a stamp is trusted only where the disk's clock, read before
//! the file was, settles it; a file whose stamp isn't settled is known by
//! its bytes, or not at all.
//!
//! It also finds `.statac`, where Statac keeps what it makes between
//! builds in the site's folder, and the folders in it, never through a
//! link.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use rayon::prelude::*;

use crate::platform;

/// Where Statac keeps what it makes between builds, in the site's folder.
pub(crate) const KEPT: &str = ".statac";

/// The folder `name` of what Statac keeps in the site at `site`, whether
/// or not it is there yet. Refused where `.statac` or the folder in it
/// is a link or anything else than a folder: nothing is ever read,
/// written or removed through a link, which may lead out of the site.
pub(crate) fn kept_in(site: &Path, name: &str) -> std::io::Result<PathBuf> {
    kept_at(site, &[KEPT, name])
}

/// [`kept_in`], for the folder everything Statac keeps is in.
pub(crate) fn kept(site: &Path) -> std::io::Result<PathBuf> {
    kept_at(site, &[KEPT])
}

fn kept_at(site: &Path, names: &[&str]) -> std::io::Result<PathBuf> {
    let mut folder = site.to_path_buf();
    for name in names {
        folder.push(name);
        match std::fs::symlink_metadata(&folder) {
            Ok(found) if found.file_type().is_symlink() => {
                return Err(std::io::Error::from(crate::problem::Reason::Link));
            }
            Ok(found) if !found.is_dir() => {
                return Err(std::io::Error::from(std::io::ErrorKind::NotADirectory));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(folder)
}

/// Makes the site's `.statac` where it isn't there, and gives its path.
/// Refused where it is a link or anything else than a folder.
pub(crate) fn make_kept(site: &Path) -> std::io::Result<PathBuf> {
    let kept = site.join(KEPT);
    there_already(std::fs::create_dir(&kept))?;
    // Asked again once made: it may have been put there as a link.
    self::kept(site)?;
    Ok(kept)
}

/// [`make_kept`], and the folder `name` in it, made by name in `.statac`
/// opened and never through a link.
pub(crate) fn make_kept_in(site: &Path, name: &str) -> std::io::Result<PathBuf> {
    let kept = make_kept(site)?;
    there_already(crate::platform::make_folder(&kept, name))?;
    kept_in(site, name)?;
    Ok(kept)
}

/// `made`, where it failed only for something having the name already.
fn there_already(made: std::io::Result<()>) -> std::io::Result<()> {
    match made {
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        made => made,
    }
}

/// The file written and removed in the site's `.statac`, and the end of
/// the name of one written and removed beside the output, to read the time
/// their disk gives a file written now.
pub(crate) const CLOCK: &str = ".statac-clock";

/// What a file was found to be: its length, when its bytes last changed
/// and, where the system says, when anything of it last changed and which
/// file on which disk it is. Any change to a file after its stamp was
/// taken, or another file put in its place, is found as another stamp,
/// where the stamp is [`Stamp::settled`] and [`Stamp::placed`].
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct Stamp {
    len: u64,
    modified: SystemTime,
    placed: Option<Placed>,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
struct Placed {
    changed: (i64, i64),
    disk: u64,
    file: u64,
}

impl Stamp {
    /// The stamp of what `found` describes, where the disk keeps the time
    /// its bytes last changed.
    pub(crate) fn of(found: &fs::Metadata) -> Option<Stamp> {
        let placed = platform::changed_and_placed(found);
        Some(Stamp {
            len: found.len(),
            modified: found.modified().ok()?,
            placed: placed.map(|(changed, disk, file)| Placed {
                changed,
                disk,
                file,
            }),
        })
    }

    /// The stamp of the file at `path`, a link followed, where it is
    /// there.
    pub(crate) fn at(path: &Path) -> Option<Stamp> {
        Stamp::of(&fs::metadata(path).ok()?)
    }

    /// Whether the stamp says when anything of the file last changed and
    /// which file it is. Without them, a file written again and given back
    /// its time, or another put in its place, may be found as the same.
    /// The system gives neither off Unix.
    pub(crate) fn placed(&self) -> bool {
        self.placed.is_some()
    }

    /// Whether a file stamped as this one, any time after `clock` was
    /// read, was last changed before then: it is as it was when the clock
    /// was read, and any change to it since gives another stamp. A change
    /// after the clock was read takes a time no earlier than the clock's.
    pub(crate) fn settled(&self, clock: &Clock) -> bool {
        let placed = match (self.placed, clock.0.placed) {
            (Some(placed), Some(now)) => placed.disk == now.disk && placed.changed < now.changed,
            (None, None) => true,
            _ => false,
        };
        placed && self.modified < clock.0.modified
    }
}

/// What a disk gives a file written now.
#[derive(Copy, Clone, Debug)]
pub(crate) struct Clock(Stamp);

impl Clock {
    /// Read from a file written at `at` and removed. None where the disk
    /// keeps no such time.
    pub(crate) fn at(at: &Path) -> io::Result<Option<Clock>> {
        // Made anew, so that it takes the time whatever was there before.
        if fs::symlink_metadata(at).is_ok() {
            fs::remove_file(at)?;
        }
        let mut file = (fs::OpenOptions::new().write(true).create_new(true)).open(at)?;
        io::Write::write_all(&mut file, b"\n")?;
        let found = file.metadata().ok();
        drop(file);
        fs::remove_file(at)?;
        Ok(found.as_ref().and_then(Stamp::of).map(Clock))
    }

    /// The clock of the disk the site at `site` is on, read in its
    /// `.statac`, which is made where it isn't there. None where it can't
    /// be read.
    pub(crate) fn of_site(site: &Path) -> Option<Clock> {
        kept(site).ok()?;
        match fs::create_dir(site.join(KEPT)) {
            Err(error) if error.kind() != io::ErrorKind::AlreadyExists => return None,
            _ => {}
        }
        // Asked again once made: it may have been put there as a link.
        Clock::at(&kept(site).ok()?.join(CLOCK)).ok()?
    }

    /// The clock as it would be read `by` later, both its times moved on.
    #[cfg(test)]
    pub(crate) fn later(self, by: std::time::Duration) -> Clock {
        let Clock(mut stamp) = self;
        stamp.modified += by;
        if let Some(placed) = &mut stamp.placed {
            placed.changed.0 += i64::try_from(by.as_secs()).expect("a short time");
        }
        Clock(stamp)
    }
}

/// Waits until a clock read in the site at `site` settles the file at
/// `path`, which on a disk whose times move in steps it may not do straight
/// after the file changed.
#[cfg(test)]
pub(crate) fn wait_until_settled(site: &Path, path: &Path) {
    let stamp = Stamp::at(path).expect("a file there");
    for _ in 0..1000 {
        if Clock::of_site(site).is_some_and(|clock| stamp.settled(&clock)) {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    panic!("the disk's time never moved");
}

/// The files a folder held at one moment, for a later look at it to say
/// which were added, changed or removed.
pub(crate) struct Inventory {
    root: PathBuf,
    /// Each file by its path from the top, with its stamp where the
    /// system gives one.
    stamps: HashMap<String, Option<Stamp>>,
    /// The hash of the bytes of each file whose stamp may not show that
    /// it was written again.
    hashes: HashMap<String, blake3::Hash>,
}

impl Inventory {
    /// Each of `files`, by its path from `root` with forward slashes, with
    /// its stamp as `found` reads it.
    pub(crate) fn of(
        root: &Path,
        files: Vec<String>,
        found: impl Fn(&Path) -> io::Result<fs::Metadata> + Sync,
    ) -> Inventory {
        let stamped = files.into_par_iter().map(|rel| {
            let stamp = found(&platform::join(root, &rel)).ok();
            (rel, stamp.as_ref().and_then(Stamp::of))
        });
        Inventory {
            root: root.to_path_buf(),
            stamps: stamped.collect(),
            hashes: HashMap::new(),
        }
    }

    /// [`Inventory::of`], known well enough for a later look to tell a
    /// file written again at its length and time: each whose stamp
    /// `clock`, read before any of them was looked at, doesn't settle is
    /// known by its bytes as well.
    pub(crate) fn to_compare(
        root: &Path,
        files: Vec<String>,
        found: impl Fn(&Path) -> io::Result<fs::Metadata> + Sync,
        clock: Option<&Clock>,
    ) -> Inventory {
        let mut inventory = Inventory::of(root, files, found);
        let unsure = (inventory.stamps.par_iter()).filter(|(_, stamp)| match (stamp, clock) {
            (Some(stamp), Some(clock)) => !stamp.settled(clock),
            _ => true,
        });
        inventory.hashes = unsure
            .filter_map(|(rel, _)| Some((rel.clone(), hashed(&platform::join(root, rel))?)))
            .collect();
        inventory
    }

    pub(crate) fn holds(&self, rel: &str) -> bool {
        self.stamps.contains_key(rel)
    }

    pub(crate) fn names(&self) -> impl Iterator<Item = &str> {
        self.stamps.keys().map(String::as_str)
    }

    /// The files `before` held, and this holds, that have changed.
    pub(crate) fn changed_since<'a>(&'a self, before: &Inventory) -> Vec<&'a str> {
        (self.stamps.par_iter())
            .filter(|(rel, stamp)| {
                let Some(was) = before.stamps.get(*rel) else {
                    return false;
                };
                was != *stamp
                    || (before.hashes.get(*rel)).is_some_and(|hash| {
                        hashed(&platform::join(&self.root, rel)).as_ref() != Some(hash)
                    })
            })
            .map(|(rel, _)| rel.as_str())
            .collect()
    }
}

/// The hash of the bytes of the file at `path`, where it can be read.
pub(crate) fn hashed(path: &Path) -> Option<blake3::Hash> {
    let mut hasher = blake3::Hasher::new();
    hasher.update_reader(fs::File::open(path).ok()?).ok()?;
    Some(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn stamp(modified: SystemTime, changed: i64, disk: u64) -> Stamp {
        Stamp {
            len: 5,
            modified,
            placed: Some(Placed {
                changed: (changed, 0),
                disk,
                file: 1,
            }),
        }
    }

    #[test]
    fn a_stamp_is_settled_only_where_both_times_are_before_the_clock_on_its_disk() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000);
        let clock = Clock(stamp(now, 1_800_000_000, 7));
        let earlier = now - Duration::from_secs(3600);
        assert!(stamp(earlier, 1_799_999_999, 7).settled(&clock));
        // Within the clock's step, or after it.
        assert!(!stamp(now, 1_799_999_999, 7).settled(&clock));
        assert!(!stamp(now + Duration::from_secs(3600), 1_799_999_999, 7).settled(&clock));
        assert!(!stamp(earlier, 1_800_000_000, 7).settled(&clock));
        // A time of its bytes set back says nothing of a change since.
        assert!(!stamp(earlier, 1_800_000_001, 7).settled(&clock));
        // Another disk keeps its own time.
        assert!(!stamp(earlier, 1_799_999_999, 8).settled(&clock));
        // Where the system says neither, the time of its bytes tells.
        let unplaced = |modified| Stamp {
            len: 5,
            modified,
            placed: None,
        };
        let clock = Clock(unplaced(now));
        assert!(unplaced(earlier).settled(&clock));
        assert!(!unplaced(now).settled(&clock));
        assert!(!stamp(earlier, 1_799_999_999, 7).settled(&clock));
    }

    #[test]
    fn the_sites_clock_is_read_in_its_kept_folder_and_leaves_nothing() {
        let site = std::env::temp_dir().join(format!("statac-disk-{}", std::process::id()));
        let _ = fs::remove_dir_all(&site);
        fs::create_dir_all(&site).unwrap();
        let clock = Clock::of_site(&site);
        let left: Vec<_> = fs::read_dir(site.join(KEPT)).unwrap().collect();
        let _ = fs::remove_dir_all(&site);
        assert!(clock.is_some());
        assert!(left.is_empty());
    }
}
