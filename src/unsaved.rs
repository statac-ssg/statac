//! The text an editor holds for files of a site and hasn't saved, which a
//! reading of the site takes in place of what each file holds.

use std::collections::HashMap;
use std::fs;
use std::io::{self, Read as _};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::platform;

/// Texts by the real path of the file each stands for, past every link.
/// The default holds none, and every file is read.
#[derive(Clone, Default, Debug)]
pub(crate) struct Unsaved(Arc<HashMap<PathBuf, Arc<str>>>);

impl Unsaved {
    pub(crate) fn new(texts: HashMap<PathBuf, Arc<str>>) -> Unsaved {
        Unsaved(Arc::new(texts))
    }

    /// The editor's text for the file at `at` under the folder `real`.
    pub(crate) fn held(&self, real: &Path, at: &str) -> Option<&str> {
        if self.0.is_empty() {
            return None;
        }
        self.0.get(&platform::join(real, at)).map(|text| &**text)
    }

    /// The text of the file at `at` under the folder `real`, opened as
    /// [`platform::open_file`] opens it: the editor's, where it holds one.
    pub(crate) fn read(&self, real: &Path, at: &str) -> io::Result<String> {
        let file = platform::open_file(real, at)?;
        match self.held(real, at) {
            Some(text) => Ok(text.to_owned()),
            None => io::read_to_string(file),
        }
    }

    /// [`Unsaved::read`], as bytes.
    pub(crate) fn read_bytes(&self, real: &Path, at: &str) -> io::Result<Vec<u8>> {
        let mut file = platform::open_file(real, at)?;
        if let Some(text) = self.held(real, at) {
            return Ok(text.as_bytes().to_vec());
        }
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Ok(bytes)
    }

    /// The text of the file at `path`, read through any link as
    /// [`fs::read_to_string`] reads it: the editor's, where it holds one.
    pub(crate) fn read_path(&self, path: &Path) -> io::Result<String> {
        let text = fs::read_to_string(path)?;
        if self.0.is_empty() {
            return Ok(text);
        }
        let held = fs::canonicalize(path)
            .ok()
            .and_then(|real| self.0.get(&real));
        Ok(held.map_or(text, |held| held.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Scratch;

    #[test]
    fn a_file_the_editor_holds_is_read_as_the_editor_has_it() {
        let scratch = Scratch::new("unsaved");
        let held = scratch.write("a/held.md", "saved");
        scratch.write("a/other.md", "other");
        let real = fs::canonicalize(&scratch.0).unwrap();
        let texts = [(fs::canonicalize(&held).unwrap(), Arc::from("typed"))];
        let unsaved = Unsaved::new(texts.into_iter().collect());
        assert_eq!(unsaved.read(&real, "a/held.md").unwrap(), "typed");
        assert_eq!(unsaved.read_bytes(&real, "a/held.md").unwrap(), b"typed");
        assert_eq!(unsaved.read_path(&held).unwrap(), "typed");
        assert_eq!(unsaved.read(&real, "a/other.md").unwrap(), "other");
        assert_eq!(Unsaved::default().read_path(&held).unwrap(), "saved");
    }

    #[test]
    fn a_file_gone_from_the_disk_is_gone_whatever_the_editor_holds() {
        let scratch = Scratch::new("unsaved-gone");
        let held = scratch.write("held.md", "saved");
        let real = fs::canonicalize(&scratch.0).unwrap();
        let texts = [(fs::canonicalize(&held).unwrap(), Arc::from("typed"))];
        let unsaved = Unsaved::new(texts.into_iter().collect());
        fs::remove_file(&held).unwrap();
        let gone = unsaved.read(&real, "held.md").unwrap_err();
        assert_eq!(gone.kind(), io::ErrorKind::NotFound);
        assert!(unsaved.read_path(&held).is_err());
    }
}
