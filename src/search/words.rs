//! The words search reads in a text, and the form each is indexed by.
//!
//! Words are found as Unicode's word boundaries find them, Chinese, Japanese
//! and the other languages written without spaces by dictionary, as a
//! browser's `Intl.Segmenter` finds them in what a reader types; its
//! dictionary may cut such a language otherwise, so a search reads a run of
//! its words by the words the index lists. Each is then folded, so `Café`
//! and `cafe` are one, and cut to its stem by the rules of its page's
//! language, so `running` and `runs` are `run`.

use std::ops::Range;
use std::sync::LazyLock;

use icu_segmenter::options::WordBreakInvariantOptions;
use icu_segmenter::{WordSegmenter, WordSegmenterBorrowed};
use pagefind_stem::Algorithm;
use unicode_normalization::UnicodeNormalization;

static SEGMENTER: LazyLock<WordSegmenterBorrowed<'static>> =
    LazyLock::new(|| WordSegmenter::new_dictionary(WordBreakInvariantOptions::default()));

/// Where each word of `text` is.
pub(crate) fn spans(text: &str) -> impl Iterator<Item = Range<usize>> + '_ {
    let mut from = 0;
    let boundaries = SEGMENTER.segment_str(text).iter_with_word_type();
    boundaries.filter_map(move |(at, kind)| {
        let span = from..at;
        from = at;
        (kind.is_word_like() && !span.is_empty()).then_some(span)
    })
}

/// `word` with the marks Latin, Greek and Cyrillic letters carry taken
/// off, in its compatibility form and small letters: `Ｃafé` is `cafe`.
/// Marks other scripts need, such as the kana's voicing, are kept. A
/// typographic apostrophe is the plain one, so `don’t` is `don't`.
pub(crate) fn fold(word: &str) -> String {
    let mut folded = String::new();
    fold_into(word, &mut folded);
    folded
}

/// [`fold`], written over `folded`.
pub(super) fn fold_into(word: &str, folded: &mut String) {
    folded.clear();
    // Nothing in ASCII decomposes or carries a mark.
    if word.is_ascii() {
        folded.push_str(word);
        folded.make_ascii_lowercase();
        return;
    }
    let unmarked: String = (word.nfkd())
        .filter(|&c| !diacritic(c))
        .map(|c| if c == '\u{2019}' { '\'' } else { c })
        .collect();
    folded.push_str(&unmarked.nfc().collect::<String>().to_lowercase());
}

/// Whether `c` is a combining mark of the blocks that hold the marks of
/// Latin, Greek and Cyrillic letters.
fn diacritic(c: char) -> bool {
    matches!(c,
        '\u{0300}'..='\u{036f}'
        | '\u{1ab0}'..='\u{1aff}'
        | '\u{1dc0}'..='\u{1dff}'
        | '\u{20d0}'..='\u{20ff}'
        | '\u{fe20}'..='\u{fe2f}')
}

/// How words of one language are cut to their stems; for a language
/// with no rules, not at all.
pub(crate) struct Stemmer(Option<pagefind_stem::Stemmer>);

impl Stemmer {
    /// The rules for the language `tag` names, by its first part; English
    /// where there is no tag, as a site that names no language is.
    pub(crate) fn of(tag: Option<&str>) -> Stemmer {
        let tag = tag.unwrap_or("en");
        let language = tag
            .split(['-', '_'])
            .next()
            .unwrap_or(tag)
            .to_ascii_lowercase();
        Stemmer(algorithm(&language).map(pagefind_stem::Stemmer::create))
    }

    /// The form `word` is indexed by.
    #[cfg(test)]
    pub(crate) fn term(&self, word: &str) -> String {
        self.stem(&fold(word))
    }

    /// The stem of `folded`, a word as [`fold`] leaves it.
    pub(crate) fn stem(&self, folded: &str) -> String {
        match &self.0 {
            Some(stemmer) => stemmer.stem(folded).into_owned(),
            None => folded.to_owned(),
        }
    }
}

/// The stemming rules for a language, by its ISO 639-1 code.
fn algorithm(language: &str) -> Option<Algorithm> {
    Some(match language {
        "ar" => Algorithm::Arabic,
        "hy" => Algorithm::Armenian,
        "eu" => Algorithm::Basque,
        "ca" => Algorithm::Catalan,
        "da" => Algorithm::Danish,
        "nl" => Algorithm::Dutch,
        "en" => Algorithm::English,
        "eo" => Algorithm::Esperanto,
        "et" => Algorithm::Estonian,
        "fi" => Algorithm::Finnish,
        "fr" => Algorithm::French,
        "de" => Algorithm::German,
        "el" => Algorithm::Greek,
        "hi" => Algorithm::Hindi,
        "hu" => Algorithm::Hungarian,
        "id" => Algorithm::Indonesian,
        "ga" => Algorithm::Irish,
        "it" => Algorithm::Italian,
        "lt" => Algorithm::Lithuanian,
        "ne" => Algorithm::Nepali,
        "no" | "nb" | "nn" => Algorithm::Norwegian,
        "pl" => Algorithm::Polish,
        "pt" => Algorithm::Portuguese,
        "ro" => Algorithm::Romanian,
        "ru" => Algorithm::Russian,
        "sr" => Algorithm::Serbian,
        "es" => Algorithm::Spanish,
        "sv" => Algorithm::Swedish,
        "ta" => Algorithm::Tamil,
        "tr" => Algorithm::Turkish,
        "yi" => Algorithm::Yiddish,
        _ => return None,
    })
}
