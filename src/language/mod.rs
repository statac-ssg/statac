//! The words and marks a language is written with: its months and
//! weekdays for dates, its quotation marks, its digits and which way its
//! lines run.
//!
//! Statac's own words for a language are kept in the binary. A site's own
//! `languages/<tag>.yaml` adds to them key by key: a key it writes takes
//! the place of Statac's, and a key it leaves out keeps Statac's. For a
//! language Statac has no words for, the file is the language whole, on
//! lines running the way its script does. A tag neither has is looked for
//! again without its last part, `en-US` as `en`, as the Unicode CLDR's own
//! data inherits.

mod direction;
mod read;
#[cfg(test)]
pub(crate) mod tests;

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use crate::origin::Sourced;
use crate::problem::{Kind, Problem};
use crate::source::Sources;

pub use read::{KEYS, read, read_onto};

/// Where a site keeps its languages.
pub const FOLDER: &str = "languages";

/// Which way a language's lines run.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum Direction {
    #[default]
    LeftToRight,
    RightToLeft,
}

impl Direction {
    /// As a language file writes it, and as HTML's `dir` reads it.
    pub fn as_str(self) -> &'static str {
        match self {
            Direction::LeftToRight => "ltr",
            Direction::RightToLeft => "rtl",
        }
    }
}

/// The marks straight quotes are set as: the outer pair, then the inner.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Quotes {
    pub open: char,
    pub close: char,
    pub inner_open: char,
    pub inner_close: char,
}

/// The words and marks of one language, each `None` or empty where the
/// language gives none.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Words {
    /// As a month is written inside a date, January first.
    pub months: Option<[String; 12]>,
    /// As a month is written with no day beside it, where that differs.
    pub months_alone: Option<[String; 12]>,
    pub months_short: Option<[String; 12]>,
    pub months_short_alone: Option<[String; 12]>,
    /// Monday first.
    pub weekdays: Option<[String; 7]>,
    pub weekdays_short: Option<[String; 7]>,
    /// A day of the month as a date writes it, the first first.
    pub days: Option<[String; 31]>,
    /// The morning's word, then the afternoon's.
    pub am_pm: Option<[String; 2]>,
    /// The ten digits numbers are written in, zero first.
    pub digits: Option<[char; 10]>,
    pub quotes: Option<Quotes>,
    /// Marks a typed space before them can't break at.
    pub no_break_before: Vec<char>,
    /// Marks a typed space after them can't break at.
    pub no_break_after: Vec<char>,
    /// The space written just inside the outer quotes.
    pub inside_quotes: Option<char>,
    pub direction: Direction,
    /// What a screen reader says for a note's way back to the text.
    pub back_to_the_text: Option<String>,
    /// The words for each kind of callout, in the order of `CALLOUTS`.
    pub callouts: Option<[String; 5]>,
}

/// The kinds of callout, in the order a language file gives their words.
pub const CALLOUTS: [&str; 5] = ["note", "tip", "important", "warning", "caution"];

impl Words {
    /// No words, on lines running the way the language `tag` is written.
    pub fn wordless(tag: &str) -> Words {
        Words {
            direction: Direction::of(tag),
            ..Words::default()
        }
    }

    /// Months with no day beside them: `months_alone`, or `months` where
    /// the language writes the two alike.
    pub fn months_with_no_day(&self) -> Option<&[String; 12]> {
        self.months_alone.as_ref().or(self.months.as_ref())
    }

    /// The words for the callout of the kind `kind`, where the language
    /// has them.
    pub fn callout(&self, kind: &str) -> Option<&str> {
        let at = CALLOUTS.iter().position(|known| *known == kind)?;
        self.callouts.as_ref().map(|words| words[at].as_str())
    }

    pub fn months_short_with_no_day(&self) -> Option<&[String; 12]> {
        self.months_short_alone
            .as_ref()
            .or(self.months_short.as_ref())
    }
}

/// Statac's own languages, by tag.
const BUNDLED: &[(&str, &str)] = &[
    ("ar", include_str!("bundled/ar.yaml")),
    ("de", include_str!("bundled/de.yaml")),
    ("en", include_str!("bundled/en.yaml")),
    ("es", include_str!("bundled/es.yaml")),
    ("fa", include_str!("bundled/fa.yaml")),
    ("fr", include_str!("bundled/fr.yaml")),
    ("ja", include_str!("bundled/ja.yaml")),
    ("ko", include_str!("bundled/ko.yaml")),
];

/// Statac's own words for `tag`, matched whole and ignoring case.
pub fn bundled(tag: &str) -> Option<(&'static str, Arc<Words>)> {
    static READ: OnceLock<Vec<Arc<Words>>> = OnceLock::new();
    let read = READ.get_or_init(|| {
        BUNDLED
            .iter()
            .map(|(tag, text)| {
                let mut sources = Sources::default();
                let id = sources.add(format!("{FOLDER}/{tag}.yaml"), (*text).to_owned());
                let (words, problems) = read(&sources, id);
                debug_assert!(problems.is_empty(), "{tag}: {problems:?}");
                Arc::new(words)
            })
            .collect()
    });
    BUNDLED
        .iter()
        .position(|(known, _)| known.eq_ignore_ascii_case(tag))
        .map(|at| (BUNDLED[at].0, Arc::clone(&read[at])))
}

/// The code of English, the language of a site naming none.
pub const ENGLISH: &str = "en";

/// English, the words a site naming no language is written in.
pub fn english() -> Arc<Words> {
    bundled(ENGLISH).expect("English is bundled").1
}

/// Where a language's words came from.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Whence {
    /// The site's own file, by its path in the site.
    Site(String),
    /// Statac's own words for the tag.
    Statac(&'static str),
    /// No words at all.
    Nothing,
}

#[derive(Clone, Debug)]
pub struct Language {
    /// The tag as the site names it; none where it names no language.
    pub tag: Option<String>,
    pub words: Arc<Words>,
    pub whence: Whence,
}

impl Language {
    /// A site that names no language is written in English.
    pub fn unnamed() -> Language {
        Language {
            tag: None,
            words: english(),
            whence: Whence::Statac(ENGLISH),
        }
    }
}

/// The files in a site's `languages/` folder.
#[derive(Clone, Default, Debug)]
pub struct Languages {
    /// Each file's name, and where it is.
    files: Vec<(String, PathBuf)>,
}

impl Languages {
    /// Lists the language files of the site at `root`. A link is followed
    /// only where it stays inside the site.
    pub fn find(root: &Path, problems: &mut Vec<Problem>) -> Languages {
        let real = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
        let mut files: Vec<(String, PathBuf)> =
            crate::assets::walk(root, &real, FOLDER, false, problems)
                .into_iter()
                .filter(|(rel, _)| !rel.contains('/') && is_yaml(rel))
                .collect();
        files.sort();
        Languages { files }
    }

    /// The language a site names by `tag`, its file read into `sources`.
    /// A tag with no words at all is said once, at the tag.
    pub fn language(
        &self,
        tag: Option<Sourced<&str>>,
        sources: &mut Sources,
        problems: &mut Vec<Problem>,
    ) -> Language {
        let Some(tag) = tag else {
            return Language::unnamed();
        };
        for wanted in broader(tag.value) {
            let found = match self.file(wanted) {
                Some((name, path)) => {
                    let shown = format!("{FOLDER}/{name}");
                    match std::fs::read_to_string(path) {
                        Ok(text) => {
                            let id = sources.add(shown.clone(), text);
                            let statacs = broader(wanted)
                                .find_map(bundled)
                                .map(|(_, words)| (*words).clone())
                                .unwrap_or_else(|| Words::wordless(tag.value));
                            let (words, found) = read_onto(sources, id, statacs);
                            problems.extend(found);
                            Some((Arc::new(words), Whence::Site(shown)))
                        }
                        Err(error) => {
                            problems.push(Problem::nowhere(Kind::CantRead {
                                path: shown.clone().into(),
                                reason: crate::problem::system_reason(&error),
                            }));
                            Some((Arc::new(Words::wordless(tag.value)), Whence::Site(shown)))
                        }
                    }
                }
                None => bundled(wanted).map(|(known, words)| (words, Whence::Statac(known))),
            };
            if let Some((words, whence)) = found {
                return Language {
                    tag: Some(tag.value.to_owned()),
                    words,
                    whence,
                };
            }
        }
        let kind = Kind::UnknownLanguage {
            tag: tag.value.to_owned(),
        };
        problems.push(match tag.origin.place {
            Some(place) => Problem::at(kind, place),
            None => Problem::nowhere(kind),
        });
        Language {
            tag: Some(tag.value.to_owned()),
            words: Arc::new(Words::wordless(tag.value)),
            whence: Whence::Nothing,
        }
    }

    /// The file named `<tag>.yaml`, ignoring case, its own spelling first.
    fn file(&self, tag: &str) -> Option<(&str, &Path)> {
        let wanted = format!("{tag}.yaml");
        let alike = |(name, _): &&(String, PathBuf)| name.eq_ignore_ascii_case(&wanted);
        self.files
            .iter()
            .find(|(name, _)| *name == wanted)
            .or_else(|| self.files.iter().find(alike))
            .map(|(name, path)| (name.as_str(), path.as_path()))
    }
}

fn is_yaml(name: &str) -> bool {
    name.rsplit_once('.')
        .is_some_and(|(stem, ext)| !stem.is_empty() && ext.eq_ignore_ascii_case("yaml"))
}

/// The tag, then each broader tag left by taking off its last part:
/// `zh-Hant-TW`, `zh-Hant`, `zh`.
fn broader(tag: &str) -> impl Iterator<Item = &str> {
    std::iter::successors(Some(tag), |tag| tag.rsplit_once('-').map(|(less, _)| less))
}
