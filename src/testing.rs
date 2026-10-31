//! What the unit tests share: folders of their own to write into.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

/// How many paths [`Scratch::named`] has handed out in each run's folder
/// that are still held. The folder stays while any is: a test making its
/// path, or the folders above it, mustn't find it gone.
static HELD: Mutex<BTreeMap<PathBuf, usize>> = Mutex::new(BTreeMap::new());

/// Held by a test from before it starts a program until the program has
/// run code of its own or ended, and by a test locking a file it has just
/// let go of: on Unix a program being started keeps a copy of every file
/// the process has open until then, so a lock stays held after its file
/// is closed. Every program Statac starts in a test is started holding it.
pub(crate) fn starting() -> MutexGuard<'static, ()> {
    static STARTING: Mutex<()> = Mutex::new(());
    STARTING.lock().unwrap_or_else(PoisonError::into_inner)
}

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
        {
            let mut held = HELD.lock().unwrap_or_else(PoisonError::into_inner);
            std::fs::create_dir_all(&run).unwrap();
            *held.entry(run.clone()).or_default() += 1;
        }
        let _ = std::fs::remove_dir_all(&path);
        let _ = std::fs::remove_file(&path);
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
            let mut held = HELD.lock().unwrap_or_else(PoisonError::into_inner);
            let Some(count) = held.get_mut(run) else {
                return;
            };
            *count -= 1;
            if *count == 0 {
                held.remove(run);
                let _ = std::fs::remove_dir(run);
            }
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

#[test]
fn a_runs_folder_stays_while_a_path_in_it_is_held() {
    let run = std::env::temp_dir().join(format!("statac-held-{}", std::process::id()));
    let first = Scratch::named("held", "first");
    let second = Scratch::named("held", "second");
    drop(first);
    assert!(run.is_dir());
    std::fs::create_dir(&*second).unwrap();
    drop(second);
    assert!(!run.exists());
}
