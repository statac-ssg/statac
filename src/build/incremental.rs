//! Building a site again after a change, building only what the change
//! touched.
//!
//! A round kept holds every output with what it added to the build. Each
//! round reads the site whole, as a build does, and asks of everything an
//! output of the kept round read whether the site read now holds the same
//! thing, saying the same, written in the same place. An output is kept
//! only where every answer is yes; every other output is built by the code
//! a build uses, and the round is concluded over all of them by the code a
//! build uses, so a round finds what a build of the same files finds.
//!
//! An id is an index into one reading's arenas, so nothing is compared by
//! id: each thing read is known by what it names, and a kept output's ids
//! and places are carried over to the new reading by those names. Whatever
//! can't be carried over, or a round has no way to account for, is built
//! again.

mod print;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use rayon::prelude::*;

use crate::album::Progress;
use crate::build::{
    self, Built, Concluded, Mode, Options, Outcome, Output, Plan, Prepared, Publishing, Remembered,
    Writes,
};
use crate::content::Texts;
use crate::disk::Clock;
use crate::problem::Problem;
use crate::reads::{Key, Reads, Translation};
use crate::search::Record;
use crate::site::PageId;
use crate::unsaved::Unsaved;
use print::{Prints, Taken};

pub(crate) use print::Reading;

/// A round kept for the next to build on.
pub(crate) struct Kept {
    reading: Reading,
    /// The fingerprint of everything an output read, and of what each is
    /// built from besides, as taken that round.
    prints: Prints,
    /// What a round can't account for output by output: where this
    /// differs, every output is built.
    whole: u64,
    built: Arc<Vec<Built>>,
    /// What each output is built from besides what it read.
    own: Vec<u64>,
    /// The records search cut from the writing of each page whose output
    /// read everything the writing read, so that where the output stands
    /// the writing does.
    searched: HashMap<PageId, Arc<[Record]>>,
}

impl Kept {
    /// A round to build on: the site as `reading` has it, `built` its
    /// outputs, and `keep` what [`rebuild`] left to keep of it.
    pub(crate) fn new(reading: Reading, built: Arc<Vec<Built>>, keep: Keep) -> Kept {
        Kept {
            reading,
            prints: keep.prints,
            whole: keep.whole,
            built,
            own: keep.own,
            searched: keep.searched,
        }
    }

    pub(crate) fn reading(&self) -> &Reading {
        &self.reading
    }

    pub(crate) fn built(&self) -> &[Built] {
        &self.built
    }
}

/// A site checked again and again as `statac check` checks it, each
/// check building again only what changed since the last.
pub struct Checking {
    texts: Texts,
    search: crate::search::Kept,
    last: Option<Kept>,
    /// How many outputs the last check built, and how many the site has.
    built: (usize, usize),
}

impl Default for Checking {
    fn default() -> Self {
        Checking::new()
    }
}

impl Checking {
    pub fn new() -> Checking {
        Checking {
            texts: Texts::keeping(),
            search: crate::search::Kept::keeping(),
            last: None,
            built: (0, 0),
        }
    }

    /// How many outputs the last check built, and how many the site has.
    pub fn built(&self) -> (usize, usize) {
        self.built
    }

    /// The site as the last check read it, where one has.
    pub(crate) fn renderer(&self) -> Option<&crate::templates::Renderer> {
        self.last.as_ref().map(|kept| &kept.reading().renderer)
    }

    /// Has each check from the next on read `unsaved` in place of the
    /// files it holds.
    pub(crate) fn set_unsaved(&mut self, unsaved: Unsaved) {
        self.texts.set_unsaved(unsaved);
    }

    /// Checks the site as `options` has it, finding what [`build::run`]
    /// finds checking it.
    pub fn check(&mut self, options: &Options) -> Outcome {
        let options = &Options {
            mode: Mode::Check,
            ..options.clone()
        };
        let at = options.site.as_path();
        if let Err(problem) = build::a_folder(at) {
            self.last = None;
            self.built = (0, 0);
            return Outcome::unread(problem);
        }
        // Read before the check reads any file of the site, so that a
        // stamp it settles is of the file as the check read it.
        let disk = Clock::of_site(at);
        let prepared = build::prepared_with(at, &options.run(), &mut self.texts);
        let plan = Plan::of(&prepared.renderer).noting();
        let rebuilding = Rebuilding {
            options,
            prepared,
            plan,
            disk,
            writes: Writes::Nowhere,
            remembered: None,
            notes: false,
        };
        let rebuilt = rebuild(self.last.as_ref(), rebuilding, &mut self.search);
        let publishing = Publishing {
            staging: None,
            files: 0,
            progress: Progress::Silent,
            tidies: false,
            after: false,
            serving: false,
            remembers: false,
            unheld: None,
            clock: build::Clock::start(),
        };
        let (prepared, plan, concluded) = (rebuilt.prepared, rebuilt.plan, rebuilt.concluded);
        let mut outcome = build::publish(
            options,
            prepared,
            &plan,
            &rebuilt.built,
            concluded,
            publishing,
        );
        build::told_once_built(&mut outcome, options);
        self.built = (rebuilt.fresh, rebuilt.built.len());
        let built = Arc::new(rebuilt.built);
        self.last = Some(Kept::new(rebuilt.reading, built, rebuilt.keep));
        outcome
    }
}

/// What a round to rebuild is handed.
pub(crate) struct Rebuilding<'a> {
    pub options: &'a Options,
    /// The site as the round read it.
    pub prepared: Prepared,
    /// What it is about to build, each page noting what its links ask the
    /// site for.
    pub plan: Plan,
    /// The clock of the site's disk, read before the round read any file
    /// of the site.
    pub disk: Option<Clock>,
    /// Where each output built goes: held to be written later, or nowhere.
    pub writes: Writes<'a>,
    /// What each file published as it is was found to hold, where that is
    /// known.
    pub remembered: Option<&'a Remembered>,
    /// Whether the round notes the keys it found changed.
    pub notes: bool,
}

/// A round rebuilt.
pub(crate) struct Rebuilt {
    pub reading: Reading,
    /// The site as the round read it, its problems taken into
    /// [`Rebuilt::concluded`].
    pub prepared: Prepared,
    pub plan: Plan,
    /// Every output, in the order `plan` has them.
    pub built: Vec<Built>,
    pub concluded: Concluded,
    /// Outputs built this round, the rest being kept.
    pub fresh: usize,
    /// Every key an output of the kept round read that this round found
    /// holding something else, as the kept round names it, in order;
    /// noted only where the round was asked to note them.
    pub changed: Vec<Key>,
    pub took: Took,
    /// What [`Kept::new`] keeps of the round.
    pub keep: Keep,
}

/// How long each part of [`rebuild`] took.
#[derive(Copy, Clone, Default, Debug)]
pub(crate) struct Took {
    /// Finding which outputs of the kept round still stand.
    pub comparing: Duration,
    /// Building the others.
    pub building: Duration,
    /// Concluding over them all.
    pub concluding: Duration,
}

/// What a round leaves to keep besides the site it read and its outputs.
pub(crate) struct Keep {
    prints: Prints,
    whole: u64,
    own: Vec<u64>,
    searched: HashMap<PageId, Arc<[Record]>>,
}

/// Builds every output of the site `rebuilding` holds that doesn't stand
/// as `last` built it, and concludes over them all, taking from `search`
/// what search made of the pages that stand.
pub(crate) fn rebuild(
    last: Option<&Kept>,
    rebuilding: Rebuilding<'_>,
    search: &mut crate::search::Kept,
) -> Rebuilt {
    let Rebuilding {
        options,
        mut prepared,
        plan,
        disk,
        writes,
        remembered,
        notes,
    } = rebuilding;
    let at = options.site.as_path();
    let mut took = Took::default();
    let mut clock = Instant::now();
    let mut lap = |part: &mut Duration| {
        *part = clock.elapsed();
        clock = Instant::now();
    };
    let reading = Reading::of(prepared.renderer.share());
    let renderer = &reading.renderer;
    let prints = Prints::new(&reading, disk);
    let taken = Taken {
        reading: &reading,
        at,
        prints: &prints,
    };
    let whole = taken.whole(&plan, options);
    let outputs = plan.outputs.par_iter();
    let own: Vec<u64> = outputs.map(|&output| taken.own(output)).collect();
    let changed = match last {
        Some(last) if notes => changed(last, taken),
        _ => Vec::new(),
    };
    let to = match last {
        Some(last) if last.whole == whole => Some(last.reading.to(&reading)),
        _ => None,
    };
    let held = matches!(writes, Writes::Later);
    let kept: Vec<Option<Built>> = match (last, &to) {
        (Some(last), Some(to)) => carry(last, taken, &plan, &own, to, held),
        _ => plan.outputs.iter().map(|_| None).collect(),
    };
    let standing: HashMap<PageId, Arc<[Record]>> = match (last, &to) {
        (Some(last), Some(to)) => {
            let outputs = plan.outputs.iter().zip(&kept);
            let stands: HashSet<PageId> = (outputs)
                .filter_map(|(output, kept)| match (output, kept) {
                    (Output::Page(page), Some(_)) => Some(*page),
                    _ => None,
                })
                .collect();
            let searched = last.searched.iter();
            let standing = searched.filter_map(|(&page, records)| {
                let page = to.page(page)?;
                stands.contains(&page).then(|| (page, Arc::clone(records)))
            });
            standing.collect()
        }
        _ => HashMap::new(),
    };
    let stood: HashSet<PageId> = standing.keys().copied().collect();
    search.stand(standing);
    lap(&mut took.comparing);

    let fresh = kept.iter().filter(|kept| kept.is_none()).count();
    let built: Vec<Built> = plan
        .outputs
        .par_iter()
        .zip(kept)
        .map(|(&output, kept)| match kept {
            Some(kept) => kept,
            None => {
                let built = build::build_output(renderer, &plan.linked, output, writes);
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
    let concluded = build::conclude(renderer, &plan, &built, problems, remembered, search);
    let pages: HashMap<PageId, usize> = (plan.outputs.iter().enumerate())
        .filter_map(|(at, output)| match output {
            Output::Page(page) => Some((*page, at)),
            _ => None,
        })
        .collect();
    let searched = search.cut().into_par_iter().filter(|(page, _)| {
        stood.contains(page)
            || (pages.get(page)).is_some_and(|&at| reads_writing(renderer, *page, &built[at]))
    });
    let searched = searched.collect();
    lap(&mut took.concluding);
    Rebuilt {
        reading,
        prepared,
        plan,
        built,
        concluded,
        fresh,
        changed,
        took,
        keep: Keep {
            prints,
            whole,
            own,
            searched,
        },
    }
}

/// Every key an output of `last` read that the reading `now` holds
/// otherwise, each once, in order.
fn changed(last: &Kept, now: Taken<'_>) -> Vec<Key> {
    let was = Taken {
        reading: &last.reading,
        at: now.at,
        prints: &last.prints,
    };
    let to = last.reading.to(now.reading);
    let keys = last.built.iter().flat_map(|built| built.reads.keys());
    let mut keys: Vec<Key> = keys.copied().collect();
    keys.sort_unstable();
    keys.dedup();
    now.changed(&was, &to, keys.into_iter())
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

/// Whether `built`, the output of `page`, read everything the page's
/// writing read, as `renderer` built it.
fn reads_writing(renderer: &crate::templates::Renderer, page: PageId, built: &Built) -> bool {
    let Some(writing) = renderer.writing_read(page) else {
        return false;
    };
    let read: HashSet<&Key> = built.reads.keys().iter().collect();
    writing.iter().all(|key| read.contains(key))
}

/// Each output of the reading `now` as `last` built it, where it still
/// stands, `to` carrying the last round's ids to `now`. Where outputs are
/// `held` to be written, only one that was written stands.
fn carry(
    last: &Kept,
    now: Taken<'_>,
    plan: &Plan,
    own: &[u64],
    to: &Translation,
    held: bool,
) -> Vec<Option<Built>> {
    let renderer = &now.reading.renderer;
    let was = Taken {
        reading: &last.reading,
        at: now.at,
        prints: &last.prints,
    };
    let built = last.built.par_iter().enumerate();
    let stood: HashMap<Output, usize> = built
        .filter_map(|(at, old)| Some((carried(old.output, to)?, at)))
        .collect();
    let stands = |at: usize, output: Output| -> Option<Built> {
        let old_at = *stood.get(&output)?;
        let old = &last.built[old_at];
        if last.own[old_at] != own[at]
            || (held && !old.written)
            || old.file != output.file(renderer)
        {
            return None;
        }
        let keys = old.reads.keys().iter();
        let same = keys.map(|&key| {
            let carried = to.key(key)?;
            (was.of(key) == now.of(carried)).then_some(carried)
        });
        let reads: Vec<Key> = same.collect::<Option<_>>()?;
        let asked = old.asked.carried(to)?;
        let problems = old.problems.iter().map(|problem| moved(problem, to));
        let problems: Vec<Problem> = problems.collect::<Option<_>>()?;
        let links = match &old.links {
            Some(checked) => {
                let checked = checked.moved(to)?;
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
            timed: Default::default(),
            file: old.file.clone(),
            html: None,
            reads: Reads::of(reads),
            asked,
            problems,
            scripts: old.scripts.clone(),
            pieces: old.pieces.clone(),
            links,
            developed: developed.collect::<Option<_>>()?,
            cards: cards.collect::<Option<_>>()?,
            sized: old.sized,
            coloured: coloured.collect::<Option<_>>()?,
            walked: old.walked,
            written: old.written,
            indexed: old.indexed.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::Report;
    use crate::testing::TempSite;
    use std::path::Path;

    /// Every problem drawn in full.
    fn said(outcome: &Outcome, root: &Path) -> String {
        let report = Report::new(outcome.sources(), root, false);
        report.check(&outcome.problems, &outcome.quiet)
    }

    fn options(root: &Path) -> Options {
        Options {
            mode: Mode::Check,
            site: root.to_path_buf(),
            out: None,
            today: crate::date::read("2026-10-04").ok(),
            drafts: false,
            url: None,
        }
    }

    /// A photograph of one colour, as a JPEG.
    fn jpeg() -> Vec<u8> {
        let picture = image::RgbImage::from_pixel(8, 8, image::Rgb([90, 90, 90]));
        let mut bytes = std::io::Cursor::new(Vec::new());
        picture
            .write_to(&mut bytes, image::ImageFormat::Jpeg)
            .unwrap();
        bytes.into_inner()
    }

    #[test]
    fn text_an_editor_holds_is_checked_as_if_it_were_saved() {
        let page = "{{ page.content }}\n{{ css('site.css') }}{{ css('plain.css') }}\n";
        let files = [
            ("statac.yaml", "language: en\ntitle: Site\n"),
            ("templates/home.html", page),
            ("templates/page.html", page),
            ("snippets/aside.html", "<aside>{{ text }}</aside>\n"),
            ("data/people.yaml", "- name: Person 1\n"),
            (
                "languages/en.yaml",
                "months_short: [Jan, Feb, Mar, Apr, May, Jun, Jul, Aug, Sep, Oct, Nov, Dec]\n",
            ),
            ("assets/site.scss", "@use 'part';\nbody { margin: 0; }\n"),
            ("assets/_part.scss", "p { margin: 0; }\n"),
            ("assets/plain.css", "p { margin: 0; }\n"),
            (
                "content/index.md",
                "---\ntitle: Home\n---\nThe home page.\n",
            ),
            (
                "content/post-1/index.md",
                "---\ntitle: Post 1\n---\n@aside(text='A.')\n\n![A photograph.](photo-1.jpg)\n",
            ),
            ("content/post-1/photo-1.jpg.yaml", "decorative: false\n"),
        ];
        // Each file, and what the editor holds for it.
        let typed = [
            ("statac.yaml", "language: en\ntitel: Site\n"),
            ("templates/page.html", "{{ page.content }\n"),
            ("snippets/aside.html", "<aside>{{ text }</aside>\n"),
            ("data/people.yaml", "- name: [Person 1\n"),
            ("languages/en.yaml", "months_short: [J\n"),
            ("assets/site.scss", "@use 'part';\nbody { margin: 0 }}\n"),
            ("assets/_part.scss", "p { margin: 0 }}\n"),
            ("assets/plain.css", "img { width: 50%; }\n"),
            (
                "content/post-1/index.md",
                "---\ntitle: Post 1\ndate: 3/4/2026\n---\n",
            ),
            ("content/post-1/photo-1.jpg.yaml", "decorative: [false\n"),
        ];
        for (path, text) in typed {
            let site = TempSite::site(&files);
            let root = &site.root;
            std::fs::write(root.join("content/post-1/photo-1.jpg"), jpeg()).unwrap();
            let options = options(root);
            let mut checking = Checking::new();
            let before = checking.check(&options);
            assert!(!before.stopped(), "{}", said(&before, root));
            let before = said(&before, root);
            let file = std::fs::canonicalize(root.join(path)).unwrap();
            let held = [(file, Arc::from(text))].into_iter().collect();
            checking.set_unsaved(Unsaved::new(held));
            let checked = said(&checking.check(&options), root);
            std::fs::write(root.join(path), text).unwrap();
            let saved = said(&crate::build::run(&options), root);
            assert_ne!(checked, before, "{path} as typed");
            assert_eq!(checked, saved, "{path} as typed");
        }
    }
}
