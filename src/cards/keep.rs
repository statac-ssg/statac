//! Drawing the cards the built pages asked for, keeping them between
//! builds, and publishing them.
//!
//! A card is kept in `.statac/cards/` under its key, so the same card is
//! never drawn twice. It is kept as a photograph's copies are, behind the
//! same checks.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use super::draw::{Failure, Format, Key, Kit, Plan, Text};
use crate::album::{Counted, Kept, Making, Progress};
use crate::disk::KEPT;
use crate::problem::{Kind, Problem, Reason};
use crate::publish::Staging;

/// The folder of `.statac` cards are kept in.
const CARDS: &str = "cards";

/// A card some built page asked for.
pub struct Wanted<'a> {
    pub kit: &'a Kit,
    pub texts: Vec<Text<'a>>,
    pub key: Key,
    /// Its texts as placed, where no card kept showed it can be drawn.
    pub plan: Option<&'a Plan>,
    /// Where it is published, as a file from the top of the built site.
    pub file: String,
    /// The description it is drawn from, by its path in the site.
    pub from: &'a str,
}

impl Wanted<'_> {
    fn kept_as(&self) -> String {
        kept_as(&self.key, self.kit.format())
    }
}

/// The name a card is kept under.
pub fn kept_as(key: &Key, format: Format) -> String {
    format!("{}.{}", &key.hex()[..32], format.extension())
}

/// The cards a site keeps, for asking which are there.
pub struct Store(Kept);

impl Store {
    pub fn open(site: &Path) -> Store {
        Store(Kept::new(site, CARDS))
    }

    /// Whether the card of `key` is kept, drawn by an earlier build.
    pub fn has(&self, key: &Key, format: Format) -> bool {
        self.0.has(&kept_as(key, format))
    }
}

/// What drawing and publishing the cards came to.
#[derive(Default, Debug)]
pub struct Drew {
    /// The cards drawn, the rest being kept from an earlier build.
    pub drawn: usize,
    /// The cards written into the built site.
    pub published: usize,
    pub problems: Vec<Problem>,
}

/// Draws each of `wanted` that no earlier build kept, and publishes them
/// all, each beside its page.
pub fn publish(site: &Path, wanted: &[Wanted], staging: &Staging, progress: Progress) -> Drew {
    let kept = Kept::new(site, CARDS);
    // Two pages in one folder sharing a card share its file.
    let mut files = HashSet::new();
    let wanted: Vec<&Wanted> = wanted
        .iter()
        .filter(|wanted| files.insert(wanted.file.as_str()))
        .collect();
    // One card may be published beside pages in several folders; it is
    // drawn once.
    let mut missing: BTreeMap<String, Vec<&Wanted>> = BTreeMap::new();
    for wanted in &wanted {
        let name = wanted.kept_as();
        if !kept.has(&name) {
            missing.entry(name).or_default().push(wanted);
        }
    }
    if !missing.is_empty() {
        kept.make_ready(site);
    }
    let counted = Counted::new(progress, Making::Cards, missing.len());
    let problems: Vec<Problem> = missing
        .par_iter()
        .flat_map_iter(|(name, files)| {
            let first = files[0];
            let drawn = match first.plan {
                Some(plan) => Ok(first.kit.paint(plan)),
                // Kept when its page was built, and gone since.
                None => first.kit.draw(&first.texts),
            };
            let encoded = drawn.and_then(|drawn| first.kit.encode(&drawn.pixels));
            let problems: Vec<Problem> = match encoded {
                // Where it can't be kept, each file is written as it is.
                Ok(bytes) => files
                    .iter()
                    .enumerate()
                    .filter(|(nth, _)| *nth == 0 || !kept.has(name))
                    .filter_map(|(_, wanted)| kept.keep(name, &bytes, &wanted.file, staging).err())
                    .collect(),
                Err(Failure::Encode(said)) => vec![Problem::nowhere(Kind::CantWrite {
                    path: PathBuf::from(&first.file),
                    reason: Reason::Said(said),
                })],
                Err(_) => vec![Problem::nowhere(Kind::CantRead {
                    path: PathBuf::from(KEPT).join(CARDS).join(name),
                    reason: Reason::Missing,
                })],
            };
            counted.one_done();
            problems
        })
        .collect();
    counted.finish();
    if !problems.is_empty() {
        return Drew {
            problems,
            ..Drew::default()
        };
    }
    let files = wanted
        .iter()
        .map(|wanted| (wanted.kept_as(), wanted.file.clone()))
        .collect();
    let (published, problems) = kept.publish(files, staging);
    Drew {
        drawn: missing.len(),
        published,
        problems,
    }
}

/// Removes every card kept that is none of `asked`, each by the name it is
/// kept under, and says how many.
pub fn sweep(site: &Path, asked: &HashSet<String>) -> usize {
    Kept::new(site, CARDS).sweep(asked, &["png", "jpg"])
}
