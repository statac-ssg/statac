//! The settings about the site's own commands, and how they stand
//! together.

use std::path::Path;

use super::Settings;
use crate::platform;
use crate::problem::{Hint, Kind, Problem, WrongPlace};

pub(crate) const BEFORE: &str = "before_build";
pub(crate) const AFTER: &str = "after_build";
/// The setting naming what `after_build` makes in the built site.
pub(crate) const MAKES: &str = "after_build_outputs";
/// The setting naming what `before_build` is run again for.
pub(crate) const WATCHES: &str = "before_build_watches";

/// Whether `written` reads as a file or a folder below some folder: no
/// piece of it empty, climbing out, or naming a drive or a root.
pub(crate) fn a_place(written: &str) -> bool {
    let rel = written.strip_suffix('/').unwrap_or(written);
    !rel.is_empty() && platform::stays_inside(rel) && !rel.split('/').any(str::is_empty)
}

/// Whether `written` names a file or a folder in the built site, as an
/// entry of `after_build_outputs` does: from its top, with or without the
/// `/` an address opens with.
pub(crate) fn a_built_place(written: &str) -> bool {
    a_place(written.strip_prefix('/').unwrap_or(written))
}

/// What is wrong with how the settings about the site's commands stand
/// together, for the site at `site`: one describing a command the site
/// doesn't set, and something `before_build_watches` names that the site
/// doesn't have.
pub(crate) fn check(site: &Path, settings: &Settings) -> Vec<Problem> {
    let mut problems = Vec::new();
    for (about, command) in [(MAKES, AFTER), (WATCHES, BEFORE)] {
        if settings.text(command).is_none()
            && let Some(written) = settings.get(about)
        {
            let kind = Kind::WrongPlace {
                key: about.to_owned(),
                why: WrongPlace::NoCommand(command),
            };
            problems.push(Problem::placed(kind, written.origin.place));
        }
    }
    for watched in settings.texts(WATCHES) {
        let path = platform::join(site, &watched.value);
        let found = match watched.value.ends_with('/') {
            true => path.is_dir(),
            false => path.exists(),
        };
        if !found {
            let kind = Kind::MissingFile {
                path: watched.value.clone(),
                hint: None::<Hint>,
            };
            problems.push(Problem::placed(kind, watched.origin.place));
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_place_stays_below_its_folder() {
        for written in ["search/", "feed.json", "a/b/", "a/b.txt"] {
            assert!(a_place(written), "{written}");
            assert!(a_built_place(&format!("/{written}")), "{written}");
        }
        for written in [
            "", "/", "../a", "a/../b", "a//b", "/a", "C:/a", "a\\b", "./a",
        ] {
            assert!(!a_place(written), "{written}");
        }
        assert!(!a_built_place("//a"));
    }
}
