//! A folder of a test's own, shared by the tests that make whole sites.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

/// How many tests hold a folder in each suite's folder, which is made and
/// removed only while this is held, so no test sees it vanish.
static HELD: Mutex<Option<HashMap<PathBuf, usize>>> = Mutex::new(None);

fn held() -> MutexGuard<'static, Option<HashMap<PathBuf, usize>>> {
    HELD.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// A path of a test's own in the system's temporary folder, with nothing
/// there yet, removed when dropped with whatever was made there and the
/// site built from it beside it, at the same name ending `.built`.
pub struct Scratch(PathBuf);

impl Scratch {
    pub fn new(suite: &str, label: &str) -> Scratch {
        let path = std::env::temp_dir()
            .join(format!("statac-{suite}-{}", std::process::id()))
            .join(label);
        let _ = std::fs::remove_dir_all(&path);
        let _ = std::fs::remove_file(&path);
        let suite = path.parent().unwrap().to_path_buf();
        let mut held = held();
        std::fs::create_dir_all(&suite).unwrap();
        *held.get_or_insert_default().entry(suite).or_default() += 1;
        Scratch(path)
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
        let _ = std::fs::remove_dir_all(self.0.with_extension("built"));
        let suite = self.0.parent().unwrap();
        let mut held = held();
        let count = held
            .get_or_insert_default()
            .entry(suite.to_path_buf())
            .or_default();
        *count = count.saturating_sub(1);
        if *count == 0 {
            let _ = std::fs::remove_dir(suite);
        }
    }
}
