//! Where a build spent its time, which `statac build --timings` shows.
//! Every build times itself: the cost is too small to measure.

use std::time::{Duration, Instant};

use super::Built;
use crate::templates::Renderer;
use crate::templates::timed::Tally;

/// A part of a build, in the order a build goes through them.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Stage {
    /// Running the site's `before_build`.
    Before,
    Reading,
    /// Building every output, and copying the files published as they
    /// are meanwhile.
    Building,
    /// What is worked out across every output: links, the JavaScript
    /// pages carry, the search index.
    Concluding,
    Copies,
    Cards,
    /// Running the site's `after_build`.
    After,
    /// Everything else written, and the built site put in place.
    Writing,
}

const STAGES: [Stage; 8] = [
    Stage::Before,
    Stage::Reading,
    Stage::Building,
    Stage::Concluding,
    Stage::Copies,
    Stage::Cards,
    Stage::After,
    Stage::Writing,
];

/// Where a build spent its time.
#[derive(Clone, Default, Debug)]
pub struct Timings {
    /// How long each stage the build went through took, in the order a
    /// build goes through them.
    pub stages: Vec<(Stage, Duration)>,
    /// What building the outputs and copying the files came to, on every
    /// thread together.
    pub work: Work,
    /// How many threads that work was shared among.
    pub threads: usize,
    /// Each template that ran, by its path in the site: its time, leaving
    /// out the templates and writing built inside it, and how many times it
    /// ran. The slowest first.
    pub templates: Vec<(String, Duration, u32)>,
    /// Each page's writing built, by its file's path in the site. The
    /// slowest first.
    pub writing: Vec<(String, Duration)>,
    /// Each output built, by its address: its time, leaving out the writing
    /// built for it, and the template it is built with. The slowest first.
    pub pages: Vec<(String, Duration, Option<String>)>,
    /// Each photograph whose copies were made, by its path in the site:
    /// how long making them took, and how many were made. The slowest
    /// first.
    pub photographs: Vec<(String, Duration, usize)>,
}

/// What building the outputs and copying the files came to, on every
/// thread together.
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct Work {
    pub whole: Duration,
    /// Writing files into the built site and copying files there.
    pub saving: Duration,
    /// Building the writing of pages.
    pub writing: Duration,
    pub templates: Duration,
    pub links: Duration,
}

impl Work {
    /// What the parts named leave of the whole.
    pub fn rest(&self) -> Duration {
        let named = self.saving + self.writing + self.templates + self.links;
        self.whole.saturating_sub(named)
    }
}

/// What was timed building one output.
#[derive(Clone, Default, Debug)]
pub(crate) struct Output {
    /// Its address.
    pub address: String,
    pub whole: Duration,
    pub links: Duration,
    pub saving: Duration,
    pub tally: Tally,
    /// The template it is built with, by its index among the files.
    pub layout: Option<usize>,
}

/// Times each stage of a build as it ends.
#[derive(Debug)]
pub(crate) struct Clock {
    lap: Instant,
    took: [Option<Duration>; STAGES.len()],
}

impl Clock {
    pub(crate) fn start() -> Clock {
        Clock {
            lap: Instant::now(),
            took: [None; STAGES.len()],
        }
    }

    /// Adds the time since the last lap to `stage`.
    pub(crate) fn lap(&mut self, stage: Stage) {
        let now = Instant::now();
        let at = STAGES
            .iter()
            .position(|s| *s == stage)
            .expect("every stage listed");
        let took = self.took[at].get_or_insert(Duration::ZERO);
        *took += now - self.lap;
        self.lap = now;
    }

    /// Each stage lapped, with its time.
    pub(crate) fn stages(&self) -> Vec<(Stage, Duration)> {
        let took = STAGES.iter().zip(self.took);
        took.filter_map(|(stage, took)| Some((*stage, took?)))
            .collect()
    }
}

impl Timings {
    /// The time of the outputs `built` and of the templates and writing
    /// built with them, `saving` being the time spent saving files while
    /// they were built, on every thread together.
    pub(crate) fn of_outputs(renderer: &Renderer, built: &[Built], saving: Duration) -> Timings {
        let mut work = Work {
            saving,
            ..Work::default()
        };
        let mut templates: Vec<(String, Duration, u32)> = Vec::new();
        let mut by_template: Vec<Option<usize>> = Vec::new();
        let mut writing: Vec<(String, Duration)> = Vec::new();
        let mut pages = Vec::with_capacity(built.len());
        let mut saved_by_outputs = Duration::ZERO;
        for timed in built.iter().map(|built| &built.timed) {
            work.whole += timed.whole;
            work.links += timed.links;
            saved_by_outputs += timed.saving;
            for &(file, took, runs) in &timed.tally.templates {
                work.templates += took;
                if by_template.len() <= file {
                    by_template.resize(file + 1, None);
                }
                let at = *by_template[file].get_or_insert_with(|| {
                    templates.push((renderer.template_path(file), Duration::ZERO, 0));
                    templates.len() - 1
                });
                templates[at].1 += took;
                templates[at].2 += runs;
            }
            for &(page, took) in &timed.tally.writing {
                work.writing += took;
                writing.push((renderer.page_path(page), took));
            }
            let own = timed.whole.saturating_sub(timed.tally.in_writing());
            let layout = timed.layout.map(|at| renderer.template_path(at));
            pages.push((timed.address.clone(), own, layout));
        }
        // The files published as they are were saved alongside.
        work.whole += saving.saturating_sub(saved_by_outputs);
        // Writing built again, as for a feed, is counted with the first.
        writing.sort_by(|a, b| a.0.cmp(&b.0));
        writing.dedup_by(|later, first| {
            let same = later.0 == first.0;
            if same {
                first.1 += later.1;
            }
            same
        });
        slowest_first(&mut templates);
        writing.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        slowest_first(&mut pages);
        Timings {
            work,
            threads: rayon::current_num_threads(),
            templates,
            writing,
            pages,
            ..Timings::default()
        }
    }
}

/// Puts `timed` in order, the slowest first, then by name.
pub(crate) fn slowest_first<T>(timed: &mut [(String, Duration, T)]) {
    timed.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stages_come_in_the_order_a_build_goes_through_them() {
        let mut clock = Clock::start();
        clock.lap(Stage::Reading);
        clock.lap(Stage::Writing);
        clock.lap(Stage::Building);
        clock.lap(Stage::Writing);
        let stages: Vec<Stage> = clock.stages().into_iter().map(|(s, _)| s).collect();
        assert_eq!(stages, [Stage::Reading, Stage::Building, Stage::Writing]);
    }

    #[test]
    fn the_rest_is_what_the_named_parts_leave() {
        let ms = Duration::from_millis;
        let work = Work {
            whole: ms(10),
            saving: ms(4),
            writing: ms(3),
            templates: ms(2),
            links: ms(2),
        };
        assert_eq!(work.rest(), Duration::ZERO);
        assert_eq!(
            Work {
                links: ms(0),
                ..work
            }
            .rest(),
            ms(1)
        );
    }
}
