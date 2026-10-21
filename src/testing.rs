//! What the unit tests share: folders of their own to write into.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A folder of a test's own in the system's temporary folder, empty when
/// made and removed when dropped.
pub(crate) struct Scratch(pub(crate) PathBuf, Option<PathBuf>);

impl Scratch {
    /// A folder named for `name`, apart from every other one made.
    pub(crate) fn new(name: &str) -> Scratch {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "statac-{name}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Scratch(path, None)
    }

    /// A path named `label`, not yet made, in a folder of the run's own
    /// for `suite`, which goes once the last such path in it has.
    pub(crate) fn named(suite: &str, label: &str) -> Scratch {
        let run = std::env::temp_dir().join(format!("statac-{suite}-{}", std::process::id()));
        let path = run.join(label);
        let _ = std::fs::remove_dir_all(&path);
        let _ = std::fs::remove_file(&path);
        std::fs::create_dir_all(&run).unwrap();
        Scratch(path, Some(run))
    }

    /// Writes `bytes` to `rel`, with the folders above it, and gives its
    /// path.
    pub(crate) fn write(&self, rel: &str, bytes: impl AsRef<[u8]>) -> PathBuf {
        let path = crate::platform::join(&self.0, rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, bytes).unwrap();
        path
    }
}

impl std::ops::Deref for Scratch {
    type Target = PathBuf;

    fn deref(&self) -> &PathBuf {
        &self.0
    }
}

impl AsRef<std::path::Path> for Scratch {
    fn as_ref(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
        let _ = std::fs::remove_file(&self.0);
        if let Some(run) = &self.1 {
            let _ = std::fs::remove_dir(run);
        }
    }
}

/// A site's files, each a path and its text, written to a folder of their
/// own, removed when dropped.
pub(crate) struct TempSite {
    pub(crate) root: PathBuf,
    _folder: Scratch,
}

impl TempSite {
    /// `files` and nothing else.
    pub(crate) fn new(files: &[(&str, &str)]) -> TempSite {
        let folder = Scratch::new("site");
        for (rel, text) in files {
            folder.write(rel, text);
        }
        TempSite {
            root: folder.0.clone(),
            _folder: folder,
        }
    }

    /// `files` in a site that has a `content/` folder, and a `statac.yaml`
    /// setting its language where `files` gives none.
    pub(crate) fn site(files: &[(&str, &str)]) -> TempSite {
        let site = TempSite::new(files);
        std::fs::create_dir_all(site.root.join("content")).unwrap();
        if !files.iter().any(|(rel, _)| *rel == "statac.yaml") {
            std::fs::write(site.root.join("statac.yaml"), "language: en\n").unwrap();
        }
        site
    }
}
