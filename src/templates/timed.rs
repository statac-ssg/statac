//! How long each template and each page's writing took while one output
//! was built on this thread. A template's time leaves out the templates
//! and the writing built inside it, so the times add up; writing built
//! for another page an output shows is counted where it was built.

use std::cell::RefCell;
use std::time::{Duration, Instant};

use crate::ids::PageId;

/// What was timed while one output was built.
#[derive(Clone, Default, Debug)]
pub(crate) struct Tally {
    /// Each template run, by its index among the files: its time and how
    /// many times it ran.
    pub(crate) templates: Vec<(usize, Duration, u32)>,
    /// Each page's writing built, with its time.
    pub(crate) writing: Vec<(PageId, Duration)>,
}

impl Tally {
    /// The time of all the writing built.
    pub(crate) fn in_writing(&self) -> Duration {
        self.writing.iter().map(|(_, took)| *took).sum()
    }
}

struct Open {
    tally: Tally,
    /// When each run still going began, and the time of what ran inside
    /// it, outermost first.
    running: Vec<(Instant, Duration)>,
}

thread_local! {
    static OPEN: RefCell<Option<Open>> = const { RefCell::new(None) };
}

/// Starts timing what is built on this thread, until [`close`].
pub(crate) fn open() {
    OPEN.set(Some(Open {
        tally: Tally::default(),
        running: Vec::new(),
    }));
}

/// What was timed since [`open`]; nothing where nothing opened it.
pub(crate) fn close() -> Tally {
    OPEN.take().map(|open| open.tally).unwrap_or_default()
}

/// A template run or a page's writing starts.
pub(super) fn start() {
    OPEN.with_borrow_mut(|open| {
        if let Some(open) = open {
            open.running.push((Instant::now(), Duration::ZERO));
        }
    });
}

/// The run of the template `file` that [`start`] began has ended.
pub(super) fn template_ended(file: usize) {
    ended(
        |tally, took| match tally.templates.iter_mut().find(|t| t.0 == file) {
            Some((_, all, runs)) => {
                *all += took;
                *runs += 1;
            }
            None => tally.templates.push((file, took, 1)),
        },
    );
}

/// The writing of `page` that [`start`] began is built.
pub(super) fn writing_ended(page: PageId) {
    ended(|tally, took| tally.writing.push((page, took)));
}

fn ended(add: impl FnOnce(&mut Tally, Duration)) {
    OPEN.with_borrow_mut(|open| {
        let Some(open) = open else { return };
        let Some((began, inside)) = open.running.pop() else {
            return;
        };
        let whole = began.elapsed();
        if let Some((_, outer)) = open.running.last_mut() {
            *outer += whole;
        }
        add(&mut open.tally, whole.saturating_sub(inside));
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::Id;

    fn wait() {
        let began = Instant::now();
        while began.elapsed() < Duration::from_millis(2) {}
    }

    #[test]
    fn a_template_leaves_out_what_ran_inside_it() {
        let began = Instant::now();
        open();
        start();
        wait();
        start();
        wait();
        writing_ended(PageId::from_index(0));
        start();
        template_ended(2);
        template_ended(1);
        let tally = close();
        let whole = began.elapsed();
        let runs: Vec<usize> = tally.templates.iter().map(|t| t.0).collect();
        assert_eq!(runs, [2, 1]);
        assert!(tally.templates[1].1 >= Duration::from_millis(2));
        assert!(tally.in_writing() >= Duration::from_millis(2));
        let in_templates: Duration = tally.templates.iter().map(|t| t.1).sum();
        assert!(in_templates + tally.in_writing() <= whole);
    }

    #[test]
    fn runs_of_one_template_are_counted_together() {
        open();
        for _ in 0..3 {
            start();
            template_ended(4);
        }
        assert_eq!(
            close()
                .templates
                .iter()
                .map(|t| (t.0, t.2))
                .collect::<Vec<_>>(),
            [(4, 3)]
        );
    }

    #[test]
    fn nothing_is_timed_unless_opened() {
        start();
        template_ended(0);
        assert!(close().templates.is_empty());
    }
}
