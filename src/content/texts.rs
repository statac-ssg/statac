//! The text of each file that may be a page as one reading of a site
//! found it, kept for the next reading to take in place of reading the
//! file again, where the file is found as it was then.

use std::collections::HashMap;
use std::fs;
use std::io::{self, Read as _};
use std::path::Path;
use std::sync::Arc;

use crate::disk::{Clock, Stamp};
use crate::platform;

/// Texts kept from one reading of a site for the next. The default keeps
/// nothing, and every file is read.
#[derive(Default)]
pub struct Texts {
    keeps: bool,
    /// What a file written as this reading began was found to be, where
    /// that could be read.
    clock: Option<Clock>,
    /// Each by where the file really is under the site's real path.
    kept: HashMap<String, Arc<Kept>>,
}

struct Kept {
    stamp: Stamp,
    text: String,
    lines: Vec<usize>,
}

/// A file read, with where its lines start.
pub(super) struct Text {
    pub text: String,
    pub lines: Vec<usize>,
    kept: Option<Arc<Kept>>,
}

/// The stamp of what `found` describes, where it is a file whose every
/// change since gives another stamp, as [`Stamp::placed`] says.
fn stamp_of(found: &fs::Metadata) -> Option<Stamp> {
    // Off Unix a stamp can't say which file it is, or when anything of it
    // last changed: nothing is kept.
    Stamp::of(found).filter(|stamp| found.is_file() && stamp.placed())
}

impl Texts {
    /// Texts that keep what each reading read for the next.
    pub fn keeping() -> Texts {
        Texts {
            keeps: true,
            ..Texts::default()
        }
    }

    /// Begins a reading of the site at `site`, before any file that may
    /// be a page is read.
    pub(super) fn begin(&mut self, site: &Path) {
        self.clock = match self.keeps {
            true => Clock::of_site(site),
            false => None,
        };
    }

    /// The text of the file at `at` under the folder `real`, opened as
    /// [`platform::open_file`] opens it: as kept, where the file opened is
    /// found as it was when kept, or else as read now.
    pub(super) fn read(&self, real: &Path, at: &str) -> io::Result<Text> {
        let mut file = platform::open_file(real, at)?;
        let Some(clock) = &self.clock else {
            let text = io::read_to_string(file)?;
            let lines = crate::source::line_starts(&text);
            return Ok(Text {
                text,
                lines,
                kept: None,
            });
        };
        // Stamped as opened, so the stamp is of the file read whatever is
        // put at its name meanwhile.
        let stamp = file.metadata().ok().as_ref().and_then(stamp_of);
        if let Some(kept) = self.kept.get(at)
            && stamp == Some(kept.stamp)
        {
            return Ok(Text {
                text: kept.text.clone(),
                lines: kept.lines.clone(),
                kept: Some(Arc::clone(kept)),
            });
        }
        let mut text = String::new();
        file.read_to_string(&mut text)?;
        let lines = crate::source::line_starts(&text);
        let kept = stamp.filter(|stamp| stamp.settled(clock)).map(|stamp| {
            Arc::new(Kept {
                stamp,
                text: text.clone(),
                lines: lines.clone(),
            })
        });
        Ok(Text { text, lines, kept })
    }

    /// Keeps for the next reading the texts of `read`, each the file
    /// where it really is as this reading read it, in place of all kept
    /// before.
    pub(super) fn keep<'a>(&mut self, read: impl Iterator<Item = (&'a str, &'a Text)>) {
        self.kept = read
            .filter_map(|(at, text)| Some((at.to_owned(), Arc::clone(text.kept.as_ref()?))))
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Scratch;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};

    const HOUR: Duration = Duration::from_secs(3600);

    /// A scratch folder holding the folder texts are kept in.
    fn scratch_with_kept(name: &str) -> Scratch {
        let scratch = Scratch::new(name);
        fs::create_dir_all(scratch.0.join(crate::album::KEPT)).unwrap();
        scratch
    }

    /// Writes `text` to `name`, last changed at `modified`.
    fn write_dated(scratch: &Scratch, name: &str, text: &str, modified: SystemTime) -> PathBuf {
        let path = scratch.write(name, text);
        let file = fs::File::options().write(true).open(&path).unwrap();
        file.set_modified(modified).unwrap();
        path
    }

    /// What a reading of the site at `site` gives of each of `names`,
    /// kept for the next.
    fn read(texts: &mut Texts, site: &Path, names: &[&str]) -> Vec<String> {
        let read: Vec<Text> = (names.iter())
            .map(|name| texts.read(site, name).unwrap())
            .collect();
        texts.keep(names.iter().copied().zip(&read));
        read.into_iter().map(|text| text.text).collect()
    }

    #[test]
    fn nothing_is_kept_where_no_clock_is_read() {
        let scratch = scratch_with_kept("unkept");
        write_dated(&scratch, "post.md", "one", SystemTime::now() - HOUR);
        let mut texts = Texts::default();
        texts.begin(&scratch.0);
        assert_eq!(read(&mut texts, &scratch.0, &["post.md"]), ["one"]);
        assert!(texts.kept.is_empty());

        let mut texts = Texts::keeping();
        texts.begin(&scratch.0.join("nowhere"));
        assert!(texts.clock.is_none());
        assert_eq!(read(&mut texts, &scratch.0, &["post.md"]), ["one"]);
        assert!(texts.kept.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn a_kept_text_is_taken_only_for_the_file_found_as_kept() {
        let scratch = scratch_with_kept("kept");
        let earlier = SystemTime::now() - HOUR;
        let path = write_dated(&scratch, "post.md", "one", earlier);
        let mut texts = Texts::keeping();
        texts.begin(&scratch.0);
        // A clock its times are surely before, however coarse the disk's.
        let clock = texts.clock.expect("a clock on this disk");
        texts.clock = Some(clock.later(2 * HOUR));
        assert_eq!(read(&mut texts, &scratch.0, &["post.md"]), ["one"]);
        assert_eq!(texts.kept.len(), 1);

        // Taken as kept, as the file is found as it was.
        let kept = Arc::get_mut(texts.kept.get_mut("post.md").unwrap()).unwrap();
        kept.text = "two".to_owned();
        assert_eq!(read(&mut texts, &scratch.0, &["post.md"]), ["two"]);

        // Written again at the same length, and given its time back. The
        // clock moved on says the first writing's step is over, so the
        // disk's own time must have left that step too, which a coarse
        // disk's may not have done this soon after it.
        let first = Stamp::at(&path).unwrap();
        let mut tries = 0;
        loop {
            write_dated(&scratch, "post.md", "six", earlier);
            if Stamp::at(&path).unwrap() != first {
                break;
            }
            tries += 1;
            assert!(tries < 1000, "the disk's time never moved");
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(read(&mut texts, &scratch.0, &["post.md"]), ["six"]);

        // Another file, at the same length and time, put in its place.
        let other = write_dated(&scratch, "other.md", "ten", earlier);
        fs::rename(&other, &path).unwrap();
        assert_eq!(read(&mut texts, &scratch.0, &["post.md"]), ["ten"]);
    }

    #[cfg(unix)]
    #[test]
    fn a_kept_text_is_never_taken_for_a_link_put_in_its_place() {
        let scratch = scratch_with_kept("linked");
        let earlier = SystemTime::now() - HOUR;
        let path = write_dated(&scratch, "post.md", "one", earlier);
        let mut texts = Texts::keeping();
        texts.begin(&scratch.0);
        let clock = texts.clock.expect("a clock on this disk");
        texts.clock = Some(clock.later(2 * HOUR));
        assert_eq!(read(&mut texts, &scratch.0, &["post.md"]), ["one"]);
        assert_eq!(texts.kept.len(), 1);
        let moved = scratch.0.join("moved.md");
        fs::rename(&path, &moved).unwrap();
        std::os::unix::fs::symlink(&moved, &path).unwrap();
        let refused = texts.read(&scratch.0, "post.md").err().unwrap();
        assert_eq!(refused.to_string(), "it is a link");
    }

    #[cfg(unix)]
    #[test]
    fn a_file_changed_as_the_reading_began_is_read_again() {
        let scratch = scratch_with_kept("within");
        // A time the clock read now is before, as a file changed within
        // the clock's step on a disk whose times move in steps has.
        let later = SystemTime::now() + HOUR;
        write_dated(&scratch, "post.md", "one", later);
        let mut texts = Texts::keeping();
        texts.begin(&scratch.0);
        assert!(texts.clock.is_some());
        assert_eq!(read(&mut texts, &scratch.0, &["post.md"]), ["one"]);
        assert!(texts.kept.is_empty());
    }
}
