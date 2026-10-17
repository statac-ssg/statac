//! The text of each file that may be a page as one reading of a site
//! found it, kept for the next reading to take in place of reading the
//! file again, where the file is found as it was then.

use std::collections::HashMap;
use std::fs;
use std::io::{self, Read as _};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

/// Texts kept from one reading of a site for the next. The default keeps
/// nothing, and every file is read.
#[derive(Default)]
pub struct Texts {
    keeps: bool,
    /// What a file written as this reading began was found to be, where
    /// that could be read.
    clock: Option<Stamp>,
    kept: HashMap<PathBuf, Arc<Kept>>,
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

/// What a file was found to be: its length, when its bytes and when
/// anything of it last changed, and which file on which disk it is. Any
/// change to a file after its stamp was taken, or another file put in its
/// place, is found as another stamp, where the stamp is [`Stamp::settled`].
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
struct Stamp {
    len: u64,
    modified: SystemTime,
    changed: (i64, i64),
    disk: u64,
    file: u64,
}

impl Stamp {
    fn of(found: &fs::Metadata) -> Option<Stamp> {
        if !found.is_file() {
            return None;
        }
        // Off Unix none of these can be read, and a stamp without them
        // could miss a file written again and given back its time, or
        // another put in its place: nothing is kept.
        let (changed, disk, file) = crate::platform::changed_and_placed(found)?;
        Some(Stamp {
            len: found.len(),
            modified: found.modified().ok()?,
            changed,
            disk,
            file,
        })
    }

    /// Whether a file stamped as this one, on a disk where `clock` was
    /// read from a file written as the reading began, was last changed
    /// before then. A disk's times move in steps, and a file changed again
    /// within the step it was last changed in keeps its times; a change
    /// after the clock was read takes a time no earlier than the clock's.
    fn settled(&self, clock: &Stamp) -> bool {
        self.disk == clock.disk && self.modified < clock.modified && self.changed < clock.changed
    }
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
            true => crate::publish::written_now(site)
                .as_ref()
                .and_then(Stamp::of),
            false => None,
        };
    }

    /// The text of the file at `path`: as kept, where the file is found as
    /// it was when kept, or else as read now.
    pub(super) fn read(&self, path: &Path) -> io::Result<Text> {
        let Some(clock) = &self.clock else {
            let text = fs::read_to_string(path)?;
            let lines = crate::source::line_starts(&text);
            return Ok(Text {
                text,
                lines,
                kept: None,
            });
        };
        if let Some(kept) = self.kept.get(path)
            && fs::metadata(path).is_ok_and(|found| Stamp::of(&found) == Some(kept.stamp))
        {
            return Ok(Text {
                text: kept.text.clone(),
                lines: kept.lines.clone(),
                kept: Some(Arc::clone(kept)),
            });
        }
        // Stamped as opened, so the stamp is of the file read whatever is
        // put at `path` meanwhile.
        let mut file = fs::File::open(path)?;
        let stamp = file.metadata().ok().as_ref().and_then(Stamp::of);
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

    /// Keeps for the next reading the texts of `read`, each the file at
    /// its path as this reading read it, in place of all kept before.
    pub(super) fn keep<'a>(&mut self, read: impl Iterator<Item = (&'a Path, &'a Text)>) {
        self.kept = read
            .filter_map(|(path, text)| Some((path.to_path_buf(), Arc::clone(text.kept.as_ref()?))))
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const HOUR: Duration = Duration::from_secs(3600);

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Scratch {
            let root =
                std::env::temp_dir().join(format!("statac-texts-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(root.join(crate::album::KEPT)).unwrap();
            Scratch(root)
        }

        fn write(&self, name: &str, text: &str, modified: SystemTime) -> PathBuf {
            let path = self.0.join(name);
            fs::write(&path, text).unwrap();
            let file = fs::File::options().write(true).open(&path).unwrap();
            file.set_modified(modified).unwrap();
            path
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn stamp(len: u64, modified: SystemTime, changed: i64, disk: u64) -> Stamp {
        Stamp {
            len,
            modified,
            changed: (changed, 0),
            disk,
            file: 1,
        }
    }

    /// What a reading gives of each of `paths`, kept for the next.
    fn read(texts: &mut Texts, paths: &[&Path]) -> Vec<String> {
        let read: Vec<Text> = paths.iter().map(|path| texts.read(path).unwrap()).collect();
        texts.keep(paths.iter().copied().zip(&read));
        read.into_iter().map(|text| text.text).collect()
    }

    #[test]
    fn a_stamp_is_settled_only_where_both_times_are_before_the_clock_on_its_disk() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000);
        let clock = stamp(1, now, 1_800_000_000, 7);
        let earlier = now - HOUR;
        assert!(stamp(5, earlier, 1_799_999_999, 7).settled(&clock));
        // Within the clock's step, or after it.
        assert!(!stamp(5, now, 1_799_999_999, 7).settled(&clock));
        assert!(!stamp(5, now + HOUR, 1_799_999_999, 7).settled(&clock));
        assert!(!stamp(5, earlier, 1_800_000_000, 7).settled(&clock));
        // A time of its bytes set back says nothing of a change since.
        assert!(!stamp(5, earlier, 1_800_000_001, 7).settled(&clock));
        // Another disk keeps its own time.
        assert!(!stamp(5, earlier, 1_799_999_999, 8).settled(&clock));
    }

    #[test]
    fn nothing_is_kept_where_no_clock_is_read() {
        let scratch = Scratch::new("unkept");
        let path = scratch.write("post.md", "one", SystemTime::now() - HOUR);
        let mut texts = Texts::default();
        texts.begin(&scratch.0);
        assert_eq!(read(&mut texts, &[&path]), ["one"]);
        assert!(texts.kept.is_empty());

        let mut texts = Texts::keeping();
        texts.begin(&scratch.0.join("nowhere"));
        assert!(texts.clock.is_none());
        assert_eq!(read(&mut texts, &[&path]), ["one"]);
        assert!(texts.kept.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn a_kept_text_is_taken_only_for_the_file_found_as_kept() {
        let scratch = Scratch::new("kept");
        let earlier = SystemTime::now() - HOUR;
        let path = scratch.write("post.md", "one", earlier);
        let mut texts = Texts::keeping();
        texts.begin(&scratch.0);
        // A clock its times are surely before, however coarse the disk's.
        let mut clock = texts.clock.expect("a clock on this disk");
        clock.modified += 2 * HOUR;
        clock.changed.0 += 7200;
        texts.clock = Some(clock);
        assert_eq!(read(&mut texts, &[&path]), ["one"]);
        assert_eq!(texts.kept.len(), 1);

        // Taken as kept, as the file is found as it was.
        let kept = Arc::get_mut(texts.kept.get_mut(&path).unwrap()).unwrap();
        kept.text = "two".to_owned();
        assert_eq!(read(&mut texts, &[&path]), ["two"]);

        // Written again at the same length, and given its time back.
        scratch.write("post.md", "six", earlier);
        assert_eq!(read(&mut texts, &[&path]), ["six"]);

        // Another file, at the same length and time, put in its place.
        let other = scratch.write("other.md", "ten", earlier);
        fs::rename(&other, &path).unwrap();
        assert_eq!(read(&mut texts, &[&path]), ["ten"]);
    }

    #[cfg(unix)]
    #[test]
    fn a_file_changed_as_the_reading_began_is_read_again() {
        let scratch = Scratch::new("within");
        // A time the clock read now is before, as a file changed within
        // the clock's step on a disk whose times move in steps has.
        let later = SystemTime::now() + HOUR;
        let path = scratch.write("post.md", "one", later);
        let mut texts = Texts::keeping();
        texts.begin(&scratch.0);
        assert!(texts.clock.is_some());
        assert_eq!(read(&mut texts, &[&path]), ["one"]);
        assert!(texts.kept.is_empty());
    }
}
