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
//! data inherits; a region is looked for first as the broader form of its
//! language the CLDR writes it as, `en-GB` as `en-001` and `es-MX` as
//! `es-419`. Statac's words for a language don't reach it written in a
//! script other than its usual one, `en-Dsrt`, whose CLDR data inherits
//! from no language.
//!
//! Statac's own words for a language come with the CLDR's patterns for
//! writing a date in it, which a site's file can't change.

mod direction;
mod parents;
mod read;
#[cfg(test)]
pub(crate) mod tests;

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use crate::origin::Sourced;
use crate::problem::{Kind, Problem};
use crate::source::Sources;

#[cfg(test)]
pub(crate) use read::read;
pub(crate) use read::{read_onto, read_statacs};

/// Where a site keeps its languages.
pub(crate) const FOLDER: &str = "languages";

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
    /// How the Unicode CLDR writes a date in the language, where Statac
    /// has the language's own.
    pub dates: Option<Patterns>,
}

/// The Unicode CLDR's patterns for a language's dates, in its letters:
/// `d MMMM y`.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Patterns {
    /// The CLDR's full date, with its weekday.
    pub full: String,
    /// The CLDR's long date.
    pub long: String,
    /// The CLDR's medium date.
    pub short: String,
    /// The CLDR's short date, in figures.
    pub figures: String,
    /// The day and the month, `MMMd`.
    pub day: String,
    /// The CLDR's short time.
    pub time: String,
    /// How a date and a time are put together, `{1}` the date and `{0}`
    /// the time: the CLDR's medium.
    pub date_and_time: String,
    /// `yMMMM`.
    pub month_and_year: String,
    /// `yMMM`.
    pub month_short_and_year: String,
    /// `y`.
    pub year: String,
}

/// The kinds of callout, in the order a language file gives their words.
pub(crate) const CALLOUTS: [&str; 5] = ["note", "tip", "important", "warning", "caution"];

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

/// Statac's own languages, by tag, each after the tag it inherits from.
/// A region's file holds only where it differs from that tag's.
const BUNDLED: &[(&str, &str)] = &[
    ("ar", include_str!("bundled/ar.yaml")),
    ("ar-BH", include_str!("bundled/ar-BH.yaml")),
    ("ar-DJ", include_str!("bundled/ar-DJ.yaml")),
    ("ar-DZ", include_str!("bundled/ar-DZ.yaml")),
    ("ar-EG", include_str!("bundled/ar-EG.yaml")),
    ("ar-ER", include_str!("bundled/ar-ER.yaml")),
    ("ar-IL", include_str!("bundled/ar-IL.yaml")),
    ("ar-IQ", include_str!("bundled/ar-IQ.yaml")),
    ("ar-JO", include_str!("bundled/ar-JO.yaml")),
    ("ar-KM", include_str!("bundled/ar-KM.yaml")),
    ("ar-KW", include_str!("bundled/ar-KW.yaml")),
    ("ar-LB", include_str!("bundled/ar-LB.yaml")),
    ("ar-MA", include_str!("bundled/ar-MA.yaml")),
    ("ar-MR", include_str!("bundled/ar-MR.yaml")),
    ("ar-OM", include_str!("bundled/ar-OM.yaml")),
    ("ar-PS", include_str!("bundled/ar-PS.yaml")),
    ("ar-QA", include_str!("bundled/ar-QA.yaml")),
    ("ar-SA", include_str!("bundled/ar-SA.yaml")),
    ("ar-SD", include_str!("bundled/ar-SD.yaml")),
    ("ar-SO", include_str!("bundled/ar-SO.yaml")),
    ("ar-SS", include_str!("bundled/ar-SS.yaml")),
    ("ar-SY", include_str!("bundled/ar-SY.yaml")),
    ("ar-TD", include_str!("bundled/ar-TD.yaml")),
    ("ar-TN", include_str!("bundled/ar-TN.yaml")),
    ("ar-YE", include_str!("bundled/ar-YE.yaml")),
    ("de", include_str!("bundled/de.yaml")),
    ("de-AT", include_str!("bundled/de-AT.yaml")),
    ("de-IT", include_str!("bundled/de-IT.yaml")),
    ("en", include_str!("bundled/en.yaml")),
    ("en-001", include_str!("bundled/en-001.yaml")),
    ("en-150", include_str!("bundled/en-150.yaml")),
    ("en-AE", include_str!("bundled/en-AE.yaml")),
    ("en-AI", include_str!("bundled/en-AI.yaml")),
    ("en-AU", include_str!("bundled/en-AU.yaml")),
    ("en-BE", include_str!("bundled/en-BE.yaml")),
    ("en-BI", include_str!("bundled/en-BI.yaml")),
    ("en-BW", include_str!("bundled/en-BW.yaml")),
    ("en-BZ", include_str!("bundled/en-BZ.yaml")),
    ("en-CA", include_str!("bundled/en-CA.yaml")),
    ("en-CC", include_str!("bundled/en-CC.yaml")),
    ("en-CH", include_str!("bundled/en-CH.yaml")),
    ("en-CK", include_str!("bundled/en-CK.yaml")),
    ("en-CM", include_str!("bundled/en-CM.yaml")),
    ("en-CX", include_str!("bundled/en-CX.yaml")),
    ("en-DG", include_str!("bundled/en-DG.yaml")),
    ("en-DK", include_str!("bundled/en-DK.yaml")),
    ("en-FI", include_str!("bundled/en-FI.yaml")),
    ("en-FK", include_str!("bundled/en-FK.yaml")),
    ("en-GB", include_str!("bundled/en-GB.yaml")),
    ("en-GG", include_str!("bundled/en-GG.yaml")),
    ("en-GI", include_str!("bundled/en-GI.yaml")),
    ("en-GS", include_str!("bundled/en-GS.yaml")),
    ("en-HK", include_str!("bundled/en-HK.yaml")),
    ("en-ID", include_str!("bundled/en-ID.yaml")),
    ("en-IE", include_str!("bundled/en-IE.yaml")),
    ("en-IL", include_str!("bundled/en-IL.yaml")),
    ("en-IM", include_str!("bundled/en-IM.yaml")),
    ("en-IN", include_str!("bundled/en-IN.yaml")),
    ("en-IO", include_str!("bundled/en-IO.yaml")),
    ("en-JE", include_str!("bundled/en-JE.yaml")),
    ("en-JP", include_str!("bundled/en-JP.yaml")),
    ("en-KE", include_str!("bundled/en-KE.yaml")),
    ("en-MG", include_str!("bundled/en-MG.yaml")),
    ("en-MS", include_str!("bundled/en-MS.yaml")),
    ("en-MT", include_str!("bundled/en-MT.yaml")),
    ("en-MU", include_str!("bundled/en-MU.yaml")),
    ("en-MV", include_str!("bundled/en-MV.yaml")),
    ("en-NF", include_str!("bundled/en-NF.yaml")),
    ("en-NG", include_str!("bundled/en-NG.yaml")),
    ("en-NR", include_str!("bundled/en-NR.yaml")),
    ("en-NU", include_str!("bundled/en-NU.yaml")),
    ("en-PK", include_str!("bundled/en-PK.yaml")),
    ("en-PN", include_str!("bundled/en-PN.yaml")),
    ("en-RW", include_str!("bundled/en-RW.yaml")),
    ("en-SC", include_str!("bundled/en-SC.yaml")),
    ("en-SE", include_str!("bundled/en-SE.yaml")),
    ("en-SG", include_str!("bundled/en-SG.yaml")),
    ("en-SH", include_str!("bundled/en-SH.yaml")),
    ("en-SX", include_str!("bundled/en-SX.yaml")),
    ("en-TK", include_str!("bundled/en-TK.yaml")),
    ("en-TV", include_str!("bundled/en-TV.yaml")),
    ("en-TZ", include_str!("bundled/en-TZ.yaml")),
    ("en-UG", include_str!("bundled/en-UG.yaml")),
    ("en-ZA", include_str!("bundled/en-ZA.yaml")),
    ("en-ZW", include_str!("bundled/en-ZW.yaml")),
    ("es", include_str!("bundled/es.yaml")),
    ("es-419", include_str!("bundled/es-419.yaml")),
    ("es-AR", include_str!("bundled/es-AR.yaml")),
    ("es-BO", include_str!("bundled/es-BO.yaml")),
    ("es-BR", include_str!("bundled/es-BR.yaml")),
    ("es-BZ", include_str!("bundled/es-BZ.yaml")),
    ("es-CL", include_str!("bundled/es-CL.yaml")),
    ("es-CO", include_str!("bundled/es-CO.yaml")),
    ("es-CR", include_str!("bundled/es-CR.yaml")),
    ("es-DO", include_str!("bundled/es-DO.yaml")),
    ("es-EC", include_str!("bundled/es-EC.yaml")),
    ("es-GT", include_str!("bundled/es-GT.yaml")),
    ("es-HN", include_str!("bundled/es-HN.yaml")),
    ("es-MX", include_str!("bundled/es-MX.yaml")),
    ("es-NI", include_str!("bundled/es-NI.yaml")),
    ("es-PA", include_str!("bundled/es-PA.yaml")),
    ("es-PE", include_str!("bundled/es-PE.yaml")),
    ("es-PH", include_str!("bundled/es-PH.yaml")),
    ("es-PR", include_str!("bundled/es-PR.yaml")),
    ("es-PY", include_str!("bundled/es-PY.yaml")),
    ("es-SV", include_str!("bundled/es-SV.yaml")),
    ("es-US", include_str!("bundled/es-US.yaml")),
    ("es-UY", include_str!("bundled/es-UY.yaml")),
    ("es-VE", include_str!("bundled/es-VE.yaml")),
    ("fa", include_str!("bundled/fa.yaml")),
    ("fa-AF", include_str!("bundled/fa-AF.yaml")),
    ("fr", include_str!("bundled/fr.yaml")),
    ("fr-BE", include_str!("bundled/fr-BE.yaml")),
    ("fr-CA", include_str!("bundled/fr-CA.yaml")),
    ("fr-CH", include_str!("bundled/fr-CH.yaml")),
    ("fr-CM", include_str!("bundled/fr-CM.yaml")),
    ("fr-DJ", include_str!("bundled/fr-DJ.yaml")),
    ("fr-DZ", include_str!("bundled/fr-DZ.yaml")),
    ("fr-MA", include_str!("bundled/fr-MA.yaml")),
    ("fr-MR", include_str!("bundled/fr-MR.yaml")),
    ("fr-SY", include_str!("bundled/fr-SY.yaml")),
    ("fr-TD", include_str!("bundled/fr-TD.yaml")),
    ("fr-TN", include_str!("bundled/fr-TN.yaml")),
    ("fr-VU", include_str!("bundled/fr-VU.yaml")),
    ("ja", include_str!("bundled/ja.yaml")),
    ("ko", include_str!("bundled/ko.yaml")),
    ("ko-CN", include_str!("bundled/ko-CN.yaml")),
];

/// Statac's own words for `tag`, matched whole and ignoring case.
pub(crate) fn bundled(tag: &str) -> Option<(&'static str, Arc<Words>)> {
    static READ: OnceLock<Vec<Arc<Words>>> = OnceLock::new();
    let read = READ.get_or_init(|| {
        let mut read: Vec<Arc<Words>> = Vec::with_capacity(BUNDLED.len());
        for (tag, text) in BUNDLED {
            let onto = parents::inherited(tag)
                .skip(1)
                .find_map(|broader| BUNDLED.iter().position(|(known, _)| *known == broader))
                .map(|at| (*read[at]).clone())
                .unwrap_or_default();
            let mut sources = Sources::default();
            let id = sources.add(format!("{FOLDER}/{tag}.yaml"), (*text).to_owned());
            let (words, problems) = read_statacs(&sources, id, onto);
            debug_assert!(problems.is_empty(), "{tag}: {problems:?}");
            read.push(Arc::new(words));
        }
        read
    });
    BUNDLED
        .iter()
        .position(|(known, _)| known.eq_ignore_ascii_case(tag))
        .map(|at| (BUNDLED[at].0, Arc::clone(&read[at])))
}

/// Statac's own words for `tag`, or for the nearest tag it inherits from.
pub(crate) fn statacs(tag: &str) -> Option<(&'static str, Arc<Words>)> {
    parents::inherited(tag).find_map(|tag| bundled(&tag))
}

/// The code of English, the language of a site naming none.
pub(crate) const ENGLISH: &str = "en";

/// English, the words a site naming no language is written in.
pub(crate) fn english() -> Arc<Words> {
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
pub(crate) struct Languages {
    /// Each file's name, and where it really is under `real`.
    files: Vec<(String, String)>,
    /// The site's real path.
    real: PathBuf,
}

impl Languages {
    /// Lists the language files of the site at `root`. A link is followed
    /// only where it stays inside the site.
    pub(crate) fn find(root: &Path, problems: &mut Vec<Problem>) -> Languages {
        let real = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
        let mut files: Vec<(String, String)> =
            crate::assets::walk(root, &real, FOLDER, false, problems)
                .into_iter()
                .filter(|(rel, _)| !rel.contains('/') && is_yaml(rel))
                .collect();
        files.sort();
        Languages { files, real }
    }

    /// The language a site names by `tag`, its file read into `sources`:
    /// the site's file for the tag or a broader one, over Statac's words
    /// for the tag, or else Statac's alone. A tag with no words at all is
    /// said once, at the tag.
    pub(crate) fn language(
        &self,
        tag: Option<Sourced<&str>>,
        sources: &mut Sources,
        problems: &mut Vec<Problem>,
    ) -> Language {
        let Some(tag) = tag else {
            return Language::unnamed();
        };
        let file = broader(tag.value).find_map(|wanted| self.file(wanted));
        let found = match file {
            Some((name, at)) => {
                let shown = format!("{FOLDER}/{name}");
                match sources.unsaved().read(&self.real, at) {
                    Ok(text) => {
                        let id = sources.add(shown.clone(), text);
                        let statacs = statacs(tag.value)
                            .map(|(_, words)| (*words).clone())
                            .unwrap_or_else(|| Words::wordless(tag.value));
                        let (words, found) = read_onto(sources, id, statacs);
                        problems.extend(found);
                        Some((Arc::new(words), Whence::Site(shown)))
                    }
                    Err(error) => {
                        problems.push(Problem::nowhere(Kind::CantRead {
                            path: shown.clone().into(),
                            reason: crate::problem::Reason::of(&error),
                        }));
                        Some((Arc::new(Words::wordless(tag.value)), Whence::Site(shown)))
                    }
                }
            }
            None => statacs(tag.value).map(|(known, words)| (words, Whence::Statac(known))),
        };
        if let Some((words, whence)) = found {
            return Language {
                tag: Some(tag.value.to_owned()),
                words,
                whence,
            };
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
    fn file(&self, tag: &str) -> Option<(&str, &str)> {
        let wanted = format!("{tag}.yaml");
        let alike = |(name, _): &&(String, String)| name.eq_ignore_ascii_case(&wanted);
        self.files
            .iter()
            .find(|(name, _)| *name == wanted)
            .or_else(|| self.files.iter().find(alike))
            .map(|(name, at)| (name.as_str(), at.as_str()))
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
