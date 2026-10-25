//! Making a photograph's copies, keeping them between builds, and
//! publishing them.
//!
//! A copy is kept in `.statac/pictures/` under its key, so the same
//! photograph at the same settings is never made twice. It is written
//! under another name and renamed, so a file there is always whole.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use rayon::prelude::*;

use super::progress::{Counted, Making};
use super::{KEPT, Picture, Pictures, Progress, kept_in, make_kept_in, quietly};
use crate::pictures::{self, Error, Rendition, Unreadable};
use crate::platform;
use crate::problem::{Kind, PictureFault, Problem, Reason};
use crate::publish::Staging;
use crate::site::PictureId;
use crate::source::Place;

/// What making and publishing the copies came to.
#[derive(Default, Debug)]
pub struct Developed {
    /// The copies made, the rest being kept from an earlier build.
    pub made: usize,
    /// The photographs those were made from.
    pub from: usize,
    /// The copies written into the built site.
    pub published: usize,
    pub problems: Vec<Problem>,
    /// Each photograph whose copies were made, with how long making them
    /// took and how many there were, in the order of their paths.
    pub timed: Vec<(String, Duration, usize)>,
}

/// A photograph some page shows.
#[derive(Copy, Clone, Debug)]
pub struct Shown {
    pub id: PictureId,
    /// Where the writing first shows it; none where only a template does.
    pub place: Option<Place>,
    /// Whether its copy at its own width is asked for.
    pub full: bool,
}

/// The folder of `.statac` copies are kept in.
const PICTURES: &str = "pictures";

/// The name a copy is kept under.
fn kept_as(copy: &Rendition) -> String {
    format!("{}.{}", &copy.key.hex()[..32], copy.format.extension())
}

/// Where copies are kept, and what became of keeping them.
pub(crate) struct Kept {
    /// The folder's name in `.statac`.
    sub: &'static str,
    /// The site's `.statac`; none where nothing may be kept or taken from
    /// what is kept.
    kept: Option<PathBuf>,
    /// Why nothing may be, where the folder is refused.
    refused: Option<Reason>,
    /// Why nothing more is kept, once a copy couldn't be.
    unwritten: Mutex<Option<Problem>>,
    /// The copies written straight into the built site instead, each by
    /// its file there.
    direct: Mutex<HashSet<String>>,
    /// Counted into the name a copy is first written under, so two of the
    /// same file made at once, from two photographs alike, never share one.
    fresh: AtomicUsize,
}

impl Kept {
    /// What the site at `site` keeps in the folder `sub` of `.statac`.
    pub(crate) fn new(site: &Path, sub: &'static str) -> Kept {
        let (kept, refused) = match kept_in(site, sub) {
            Ok(_) => (Some(site.join(KEPT)), None),
            Err(error) => (None, Some(Reason::of(&error))),
        };
        Kept {
            sub,
            kept,
            refused,
            unwritten: Mutex::new(None),
            direct: Mutex::new(HashSet::new()),
            fresh: AtomicUsize::new(0),
        }
    }

    /// Whether `name` is kept. Only a file itself is: a link standing in
    /// for one is never read, and what it stood for is made again.
    pub(crate) fn has(&self, name: &str) -> bool {
        self.kept
            .as_ref()
            .is_some_and(|kept| platform::is_file_itself(&kept.join(self.sub).join(name)))
    }

    /// `name`'s path in `.statac`.
    fn rel(&self, name: &str) -> String {
        format!("{}/{name}", self.sub)
    }

    /// What is kept as `name`, opened to read through no link.
    fn open(&self, name: &str) -> std::io::Result<fs::File> {
        let gone = || std::io::Error::from(std::io::ErrorKind::NotFound);
        let kept = self.kept.as_ref().ok_or_else(gone)?;
        platform::open_file(kept, &self.rel(name))
    }

    /// Makes the folder ready to keep something new in.
    pub(crate) fn make_ready(&self, site: &Path) {
        // Asked again once made: either may have been put there as a link.
        let made = match (&self.kept, &self.refused) {
            (Some(_), _) => make_kept_in(site, self.sub)
                .map(drop)
                .map_err(|error| Reason::of(&error)),
            (None, Some(reason)) => Err(reason.clone()),
            (None, None) => Ok(()),
        };
        if let Err(reason) = made {
            self.gave_up(reason);
        }
    }

    fn gave_up(&self, reason: Reason) {
        let mut unwritten = self.unwritten.lock().expect("never poisoned");
        unwritten.get_or_insert_with(|| {
            Problem::nowhere(Kind::CacheNotWritten {
                path: PathBuf::from(KEPT).join(self.sub),
                reason,
            })
        });
    }

    /// Keeps as `name` what was just made, or where it can't be kept,
    /// writes it into the built site at `file`.
    pub(crate) fn keep(
        &self,
        name: &str,
        bytes: &[u8],
        file: &str,
        staging: &Staging,
    ) -> Result<(), Problem> {
        let usable = self.unwritten.lock().expect("never poisoned").is_none();
        if let Some(kept) = self.kept.as_ref().filter(|_| usable) {
            let nth = self.fresh.fetch_add(1, Ordering::Relaxed);
            let fresh = format!("{name}.{}-{nth}", std::process::id());
            match platform::keep_file(kept, &self.rel(name), &fresh, bytes) {
                Ok(()) => return Ok(()),
                Err(error) => self.gave_up(Reason::of(&error)),
            }
        }
        staging.write(file, bytes)?;
        self.direct
            .lock()
            .expect("never poisoned")
            .insert(file.to_owned());
        Ok(())
    }

    /// Publishes at each file what is kept under the name beside it, but
    /// for those written there already, and says how many files the built
    /// site has of them and what went wrong.
    pub(crate) fn publish(
        self,
        wanted: Vec<(String, String)>,
        staging: &Staging,
    ) -> (usize, Vec<Problem>) {
        let direct = std::mem::take(&mut *self.direct.lock().expect("never poisoned"));
        let copied: Vec<Result<(), Problem>> = wanted
            .par_iter()
            .filter(|(_, file)| !direct.contains(file))
            .map(|(name, file)| {
                let shown = format!("{KEPT}/{}/{name}", self.sub);
                staging.copy_whole(self.open(name), &shown, file)
            })
            .collect();
        let mut published = direct.len();
        let mut problems = Vec::new();
        for copied in copied {
            match copied {
                Ok(()) => published += 1,
                Err(problem) => problems.push(problem),
            }
        }
        problems.extend(self.unwritten.into_inner().expect("never poisoned"));
        (published, problems)
    }

    /// Removes every file kept under a name not among `wanted` that ends
    /// as one of `endings`, and says how many. Only files themselves,
    /// directly in the folder, are removed: a link or a folder found
    /// there is left alone, and nothing is removed where the folder is
    /// not a folder of the site's own.
    pub(crate) fn sweep(&self, wanted: &HashSet<String>, endings: &[&str]) -> usize {
        let Some(kept) = &self.kept else {
            return 0;
        };
        let Ok(entries) = fs::read_dir(kept.join(self.sub)) else {
            return 0;
        };
        let mut removed = 0;
        for entry in entries.flatten() {
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            // A directory entry's own kind, which follows no link.
            let file = entry.file_type().is_ok_and(|kind| kind.is_file());
            if !file || name == "index" || wanted.contains(&name) {
                continue;
            }
            let ours = endings
                .iter()
                .any(|ending| name.rsplit_once('.').is_some_and(|(_, e)| e == *ending));
            if platform::remove_file(kept, &self.rel(&name)).is_ok() && ours {
                removed += 1;
            }
        }
        removed
    }
}

pub(crate) fn fault(error: Error) -> PictureFault {
    match error {
        Error::Unreadable(Unreadable::Unsupported(what)) => PictureFault::Unsupported(what.into()),
        Error::Unreadable(_) => PictureFault::CutShort,
        Error::NotAPhotograph(_) => {
            PictureFault::Unsupported("picture of another kind than when the site was read".into())
        }
        Error::TooLarge(size) => PictureFault::TooLarge {
            width: size.width,
            height: size.height,
        },
        Error::NotOffered { width, format } => PictureFault::CantMake(format!(
            "it has no {} copy {width} wide",
            format.extension()
        )),
        Error::Encode { message, .. } => PictureFault::CantMake(message),
    }
}

/// Makes the copies of `shown`, each a photograph with where the writing
/// first shows it, that no earlier build kept, and publishes every copy of
/// each beside where the photograph is published.
pub fn develop(
    site: &Path,
    all: &Pictures,
    shown: &[Shown],
    staging: &Staging,
    progress: Progress,
) -> Developed {
    let kept = Kept::new(site, PICTURES);
    let jobs: Vec<(&Picture, &Shown, Vec<&Rendition>)> = shown
        .par_iter()
        .map(|shown| {
            let picture = all.get(shown.id);
            let missing = picture
                .wanted(shown.full)
                .filter(|copy| !kept.has(&kept_as(copy)))
                .collect();
            (picture, shown, missing)
        })
        .collect();
    let to_make: usize = jobs.iter().map(|(_, _, missing)| missing.len()).sum();
    if to_make > 0 {
        kept.make_ready(site);
    }
    let counted = Counted::new(progress, Making::Copies, to_make);
    let settings = pictures::Settings::default();
    let made: Vec<(usize, Vec<Problem>, (String, Duration))> = jobs
        .par_iter()
        .filter(|(_, _, missing)| !missing.is_empty())
        .map(|(picture, shown, missing)| {
            // Each part is timed on its own: while this waits on its
            // widths, made at once, its thread may make another's.
            let took = AtomicU64::new(0);
            let timed = |began: Instant| {
                let nanos = u64::try_from(began.elapsed().as_nanos()).unwrap_or(u64::MAX);
                took.fetch_add(nanos, Ordering::Relaxed);
            };
            let timing = || {
                (
                    picture.shown(),
                    Duration::from_nanos(took.load(Ordering::Relaxed)),
                )
            };
            let began = Instant::now();
            let unreadable = |fault| {
                let kind = Kind::UnreadableImage {
                    path: picture.shown(),
                    fault,
                };
                Problem::placed(kind, shown.place)
            };
            let bytes = match picture.read(&all.real) {
                Ok(bytes) => bytes,
                Err(error) => {
                    let kind = Kind::CantRead {
                        path: picture.shown().into(),
                        reason: Reason::of(&error),
                    };
                    timed(began);
                    return (0, vec![Problem::nowhere(kind)], timing());
                }
            };
            let opened = match quietly(|| pictures::open(&bytes)) {
                Ok(opened) => opened,
                Err(error) => {
                    timed(began);
                    return (0, vec![unreadable(fault(error))], timing());
                }
            };
            drop(bytes);
            timed(began);
            let mut widths: BTreeMap<u32, Vec<&Rendition>> = BTreeMap::new();
            for copy in missing {
                widths.entry(copy.width).or_default().push(copy);
            }
            let problems: Vec<Problem> = widths
                .par_iter()
                .flat_map_iter(|(&width, copies)| {
                    let began = Instant::now();
                    let resized = opened.resize(width, &settings);
                    let mut problems = Vec::new();
                    for copy in copies {
                        let encoded = match &resized {
                            Ok(resized) => resized.encode(copy.format, &settings),
                            Err(error) => Err(error.clone()),
                        };
                        let file = picture.copy_file(&copy.name).unwrap_or_default();
                        match encoded {
                            Ok(bytes) => {
                                let name = kept_as(copy);
                                problems.extend(kept.keep(&name, &bytes, &file, staging).err())
                            }
                            Err(error) => problems.push(unreadable(fault(error))),
                        }
                        counted.one_done();
                    }
                    timed(began);
                    problems
                })
                .collect();
            (missing.len(), problems, timing())
        })
        .collect();
    counted.finish();

    let mut developed = Developed::default();
    // One photograph is told once, however many of its copies went wrong.
    let mut told: HashSet<String> = HashSet::new();
    for (copies, problems, (path, took)) in made {
        developed.timed.push((path, took, copies));
        if problems.is_empty() {
            developed.made += copies;
            developed.from += 1;
        }
        for problem in problems {
            let once = match &problem.kind {
                Kind::UnreadableImage { path, .. } => told.insert(path.clone()),
                _ => true,
            };
            if once {
                developed.problems.push(problem);
            }
        }
    }
    developed.timed.sort_by(|a, b| a.0.cmp(&b.0));
    if !developed.problems.is_empty() {
        return developed;
    }

    let wanted = jobs
        .iter()
        .flat_map(|(picture, shown, _)| {
            let copies = picture.wanted(shown.full);
            copies.map(|copy| {
                let file = picture.copy_file(&copy.name).unwrap_or_default();
                (kept_as(copy), file)
            })
        })
        .collect();
    let (published, problems) = kept.publish(wanted, staging);
    developed.published = published;
    developed.problems.extend(problems);
    developed
}

/// Removes every copy kept that is of none of `asked`, each a photograph
/// and whether its copy at its own width is wanted too, and says how many.
/// Only files themselves, directly in the folder of copies, are removed: a
/// link or a folder found there is left alone, and nothing is removed
/// where the folder of copies is not a folder of the site's own.
pub fn sweep(
    site: &Path,
    all: &Pictures,
    asked: impl IntoIterator<Item = (PictureId, bool)>,
) -> usize {
    let wanted: HashSet<String> = asked
        .into_iter()
        .flat_map(|(id, full)| all.get(id).wanted(full).map(kept_as))
        .collect();
    Kept::new(site, PICTURES).sweep(&wanted, &["avif", "jpg", "png"])
}
