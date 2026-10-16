//! How far a build has got with the copies it is making, said while it
//! makes them: a long first build would otherwise look stopped.

use std::io::Write;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// How a build says how far it has got.
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub enum Progress {
    /// Not at all.
    #[default]
    Silent,
    /// On one line, written over as the count moves, and gone at the end:
    /// for a terminal.
    Line,
    /// On a line of its own every so often, where what is said is kept.
    Lines,
}

impl Progress {
    /// Says `line` on a line of its own that stays, where the build says
    /// anything at all.
    pub(crate) fn say(self, line: &str) {
        if self != Progress::Silent {
            let mut err = std::io::stderr().lock();
            let _ = writeln!(err, "{line}");
            let _ = err.flush();
        }
    }

    /// How long passes before the count is said, and then said again.
    fn pause(self) -> Duration {
        match self {
            Progress::Line => Duration::from_millis(250),
            _ => Duration::from_secs(10),
        }
    }
}

/// What a build is counting as it makes them.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum Making {
    Copies,
    Cards,
}

/// The copies or cards made so far of those to make, counted from every
/// thread making them.
pub(crate) struct Counted {
    how: Progress,
    what: Making,
    of: usize,
    done: AtomicUsize,
    said: Mutex<Said>,
}

struct Said {
    last: Instant,
    /// Whether a line stands on the terminal to be taken away.
    drawn: bool,
}

impl Counted {
    pub fn new(how: Progress, what: Making, of: usize) -> Counted {
        Counted {
            how,
            what,
            of,
            done: AtomicUsize::new(0),
            said: Mutex::new(Said {
                last: Instant::now(),
                drawn: false,
            }),
        }
    }

    pub fn one_done(&self) {
        let done = self.done.fetch_add(1, Ordering::Relaxed) + 1;
        if self.how == Progress::Silent {
            return;
        }
        // Another thread saying it is as good as this one.
        let Ok(mut said) = self.said.try_lock() else {
            return;
        };
        if said.last.elapsed() < self.how.pause() {
            return;
        }
        said.last = Instant::now();
        let line = match self.what {
            Making::Copies => crate::wording::making_copies(done, self.of),
            Making::Cards => crate::wording::drawing_cards(done, self.of),
        };
        let mut err = std::io::stderr().lock();
        let _ = match self.how {
            Progress::Line => {
                said.drawn = true;
                write!(err, "\r{line}\x1b[K")
            }
            _ => writeln!(err, "{line}"),
        };
        let _ = err.flush();
    }

    /// Takes away the line a terminal was shown.
    pub fn finish(&self) {
        let said = self.said.lock().expect("never poisoned");
        if said.drawn {
            let mut err = std::io::stderr().lock();
            let _ = write!(err, "\r\x1b[K");
            let _ = err.flush();
        }
    }
}
