//! Writing into a file of the author's: the one place Statac changes what
//! an author wrote. Only the spans named are replaced, and only where the
//! file still holds what was read; every other byte, line endings
//! included, stays as it was.

use std::path::Path;

use crate::platform::{self, Unwritten};
use crate::problem::system_reason;
use crate::source::Span;
use crate::wording;

/// One span of a file and what is written in its place.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Edit {
    pub span: Span,
    pub with: String,
}

/// Replaces each of `edits` in the file at `rel`, a path with forward
/// slashes under the folder `root`, and gives what the file holds
/// afterwards. `read` is the file as it was read, which the spans are
/// places in.
///
/// Nothing is written, and the reason is given, where the file no longer
/// holds `read`, where `root`, a folder on the way or the file is a link,
/// or where the file has a second name somewhere, as
/// [`platform::rewrite`] has it.
pub fn rewrite(root: &Path, rel: &str, read: &str, edits: &[Edit]) -> Result<Vec<u8>, String> {
    let text = replaced(read, edits);
    replace(root, rel, read.as_bytes(), text.as_bytes())?;
    Ok(text.into_bytes())
}

/// Puts `with` in place of the whole file at `rel`, refused as
/// [`rewrite`] is. Where it is refused or fails, the file is as it was.
pub fn replace(root: &Path, rel: &str, read: &[u8], with: &[u8]) -> Result<(), String> {
    match platform::rewrite(root, rel, read, with) {
        Ok(()) => Ok(()),
        Err(Unwritten::Changed) => Err(wording::CHANGED_SINCE_READ.into()),
        Err(Unwritten::NamedTwice) => Err(wording::NAMED_TWICE.into()),
        Err(Unwritten::Io(error)) => Err(system_reason(&error)),
    }
}

/// `read` with each of `edits` made. They may come in any order, and none
/// may overlap another.
fn replaced(read: &str, edits: &[Edit]) -> String {
    let mut edits: Vec<&Edit> = edits.iter().collect();
    edits.sort_by_key(|edit| edit.span.start);
    let mut text = String::with_capacity(read.len());
    let mut at = 0;
    for edit in edits {
        let range = edit.span.range();
        assert!(at <= range.start, "two edits overlap");
        text.push_str(&read[at..range.start]);
        text.push_str(&edit.with);
        at = range.end;
    }
    text.push_str(&read[at..]);
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Scratch {
            let path =
                std::env::temp_dir().join(format!("statac-rewrite-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(path.join("content").join("posts")).unwrap();
            Scratch(path)
        }

        fn content(&self) -> PathBuf {
            self.0.join("content")
        }

        fn put(&self, rel: &str, text: &str) -> PathBuf {
            let path = platform::join(&self.content(), rel);
            fs::write(&path, text).unwrap();
            path
        }

        /// The names in the folder a file of `content/posts` is in.
        fn names(&self) -> Vec<String> {
            let mut names: Vec<String> = fs::read_dir(self.content().join("posts"))
                .unwrap()
                .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
                .collect();
            names.sort();
            names
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn edit(text: &str, written: &str, with: &str) -> Edit {
        let start = text.find(written).unwrap();
        Edit {
            span: Span::new(start, start + written.len()),
            with: with.into(),
        }
    }

    #[test]
    fn only_the_spans_named_change() {
        let scratch = Scratch::new("spans");
        let read =
            "\u{feff}---\r\ntitle: Post 1\r\ndate: one\r\nupdated: two\r\n---\r\nBody one.\r\n";
        let path = scratch.put("posts/post-1.md", read);
        // Given last first: the order they are made in is the file's.
        let edits = [
            edit(read, "two", "2026-10-05"),
            edit(read, "one", "2026-10-04"),
        ];
        let written = rewrite(&scratch.content(), "posts/post-1.md", read, &edits).unwrap();
        let expected = "\u{feff}---\r\ntitle: Post 1\r\ndate: 2026-10-04\r\nupdated: 2026-10-05\r\n---\r\nBody one.\r\n";
        assert_eq!(fs::read_to_string(&path).unwrap(), expected);
        assert_eq!(written, expected.as_bytes());
        assert_eq!(scratch.names(), ["post-1.md"], "nothing is left beside it");
    }

    #[test]
    fn a_file_changed_since_it_was_read_is_left_as_it_is() {
        let scratch = Scratch::new("changed");
        let read = "---\ndate: one\n---\n";
        let path = scratch.put("posts/post-1.md", "---\ndate: one\n---\nA line more.\n");
        let refused = rewrite(
            &scratch.content(),
            "posts/post-1.md",
            read,
            &[edit(read, "one", "2026-10-04")],
        );
        assert_eq!(refused, Err(wording::CHANGED_SINCE_READ.to_owned()));
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "---\ndate: one\n---\nA line more.\n"
        );
        assert_eq!(scratch.names(), ["post-1.md"]);
    }

    #[test]
    fn a_file_that_has_gone_is_not_made() {
        let scratch = Scratch::new("gone");
        let read = "---\ndate: one\n---\n";
        let refused = rewrite(
            &scratch.content(),
            "posts/post-1.md",
            read,
            &[edit(read, "one", "2026-10-04")],
        );
        assert_eq!(refused, Err("it doesn't exist".to_owned()));
        assert!(scratch.names().is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn permissions_are_kept() {
        use std::os::unix::fs::PermissionsExt;
        let scratch = Scratch::new("permissions");
        let read = "---\ndate: one\n---\n";
        let path = scratch.put("posts/post-1.md", read);
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        rewrite(
            &scratch.content(),
            "posts/post-1.md",
            read,
            &[edit(read, "one", "2026-10-04")],
        )
        .unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o640);
    }

    #[cfg(unix)]
    #[test]
    fn nothing_is_written_through_a_link() {
        let scratch = Scratch::new("link");
        let read = "---\ndate: one\n---\n";
        let edits = [edit(read, "one", "2026-10-04")];
        let outside = scratch.0.join("outside");
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("post-2.md"), read).unwrap();

        // The file is a link.
        let link = scratch.content().join("posts").join("post-2.md");
        std::os::unix::fs::symlink(outside.join("post-2.md"), &link).unwrap();
        let refused = rewrite(&scratch.content(), "posts/post-2.md", read, &edits);
        assert_eq!(refused, Err("it is a link".to_owned()));

        // A folder on the way is one.
        std::os::unix::fs::symlink(&outside, scratch.content().join("linked")).unwrap();
        let refused = rewrite(&scratch.content(), "linked/post-2.md", read, &edits);
        assert_eq!(refused, Err("it is a link".to_owned()));

        // The folder itself is one.
        let root = scratch.0.join("root");
        std::os::unix::fs::symlink(&outside, &root).unwrap();
        let refused = rewrite(&root, "post-2.md", read, &edits);
        assert_eq!(refused, Err("it is a link".to_owned()));

        assert_eq!(fs::read_to_string(outside.join("post-2.md")).unwrap(), read);
        assert!(fs::symlink_metadata(&link).unwrap().is_symlink());
    }

    #[cfg(unix)]
    #[test]
    fn a_file_with_a_second_name_is_left_as_it_is() {
        let scratch = Scratch::new("names");
        let read = "---\ndate: one\n---\n";
        let path = scratch.put("posts/post-1.md", read);
        let other = scratch.0.join("other.md");
        fs::hard_link(&path, &other).unwrap();
        let refused = rewrite(
            &scratch.content(),
            "posts/post-1.md",
            read,
            &[edit(read, "one", "2026-10-04")],
        );
        assert_eq!(refused, Err(wording::NAMED_TWICE.to_owned()));
        assert_eq!(fs::read_to_string(&other).unwrap(), read);
    }

    #[cfg(unix)]
    #[test]
    fn a_link_where_the_new_text_would_be_written_is_never_written_through() {
        let scratch = Scratch::new("beside");
        let read = "---\ndate: one\n---\n";
        let path = scratch.put("posts/post-1.md", read);
        let outside = scratch.0.join("outside.md");
        fs::write(&outside, "Outside.\n").unwrap();
        let beside = scratch
            .content()
            .join("posts")
            .join(platform::beside("post-1.md"));
        std::os::unix::fs::symlink(&outside, &beside).unwrap();
        let refused = rewrite(
            &scratch.content(),
            "posts/post-1.md",
            read,
            &[edit(read, "one", "2026-10-04")],
        );
        assert_eq!(refused, Err("file exists".to_owned()));
        assert_eq!(fs::read_to_string(&outside).unwrap(), "Outside.\n");
        assert_eq!(fs::read_to_string(&path).unwrap(), read);
        // What was there is not this run's to remove.
        assert!(fs::symlink_metadata(&beside).unwrap().is_symlink());
    }

    #[cfg(unix)]
    #[test]
    fn a_folder_that_takes_no_new_file_leaves_the_file_and_nothing_beside_it() {
        use std::os::unix::fs::PermissionsExt;
        let scratch = Scratch::new("left");
        let read = "---\ndate: one\n---\n";
        let folder = scratch.content().join("posts");
        let path = scratch.put("posts/post-1.md", read);
        fs::set_permissions(&folder, fs::Permissions::from_mode(0o555)).unwrap();
        let refused = rewrite(
            &scratch.content(),
            "posts/post-1.md",
            read,
            &[edit(read, "one", "2026-10-04")],
        );
        fs::set_permissions(&folder, fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(refused, Err("permission denied".to_owned()));
        assert_eq!(fs::read_to_string(&path).unwrap(), read);
        assert_eq!(scratch.names(), ["post-1.md"]);
    }
}
