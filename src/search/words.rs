//! The words search reads in a text, and the form each is indexed by.
//!
//! Words are found as Unicode's word boundaries find them, Chinese, Japanese
//! and the other languages written without spaces by dictionary, so they are
//! the words a browser's `Intl.Segmenter` finds in what a reader types. Each
//! is then folded, so `Café` and `cafe` are one, and cut to its stem by the
//! rules of its page's language, so `running` and `runs` are `run`.

use std::ops::Range;
use std::sync::LazyLock;

use icu_segmenter::options::WordBreakInvariantOptions;
use icu_segmenter::{WordSegmenter, WordSegmenterBorrowed};
use pagefind_stem::Algorithm;
use unicode_normalization::UnicodeNormalization;

static SEGMENTER: LazyLock<WordSegmenterBorrowed<'static>> =
    LazyLock::new(|| WordSegmenter::new_dictionary(WordBreakInvariantOptions::default()));

/// Where each word of `text` is.
pub fn spans(text: &str) -> impl Iterator<Item = Range<usize>> + '_ {
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
/// Marks other scripts need, such as the kana's voicing, are kept.
pub fn fold(word: &str) -> String {
    let unmarked: String = word.nfkd().filter(|&c| !diacritic(c)).collect();
    unmarked.nfc().collect::<String>().to_lowercase()
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
pub struct Stemmer(Option<pagefind_stem::Stemmer>);

impl Stemmer {
    /// The rules for the language `tag` names, by its first part; English
    /// where there is no tag, as a site that names no language is.
    pub fn of(tag: Option<&str>) -> Stemmer {
        let tag = tag.unwrap_or("en");
        let language = tag
            .split(['-', '_'])
            .next()
            .unwrap_or(tag)
            .to_ascii_lowercase();
        Stemmer(algorithm(&language).map(pagefind_stem::Stemmer::create))
    }

    /// The form `word` is indexed and looked up by.
    pub fn term(&self, word: &str) -> String {
        let folded = fold(word);
        match &self.0 {
            Some(stemmer) => stemmer.stem(&folded).into_owned(),
            None => folded,
        }
    }

    /// The form of each word of `text`.
    pub fn terms<'a>(&'a self, text: &'a str) -> impl Iterator<Item = String> + 'a {
        spans(text).map(|span| self.term(&text[span]))
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
