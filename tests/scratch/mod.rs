//! A folder of a test's own, shared by the tests that make whole sites.

use std::path::PathBuf;

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
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
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
        // The suite's folder, once its last test's is gone.
        let _ = std::fs::remove_dir(self.0.parent().unwrap());
    }
}
