//! Code coloured as the site is built, where `code_colours: true` asks,
//! by `statac_highlight`: each piece of code of a kind carries one class,
//! `hl-` and its kind, which the site's own stylesheet colours. A block
//! that asks for its lines numbered or marked is written a line at a time
//! (see `lines`).

pub(crate) mod lines;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub(crate) use statac_highlight::{FirstWord, Language, file_language, first_word, too_long};

use crate::html::escape;

/// The language nearest a word that names none, where one is near. Short
/// names lie a slip apart from other languages altogether (`glsl` and
/// `hlsl`, `obj` and `objc`), so only a word of five characters or more is
/// offered one.
pub(crate) fn nearest(word: &str) -> Option<&'static str> {
    let word = word.to_lowercase();
    if word.chars().count() < 5 {
        return None;
    }
    crate::suggest::nearest(&word, statac_highlight::names())
}

/// Blocks of code coloured, by their language and their code.
type Done = HashMap<(&'static str, Box<str>), Arc<str>>;

/// Colours code.
#[derive(Default)]
pub(crate) struct Highlighter {
    /// Each block once coloured, so a block written twice is coloured once.
    done: Mutex<Done>,
}

impl Highlighter {
    /// The inside of a `<code>` holding `code`, coloured as `language`, or
    /// as it is where its grammar fails on it.
    pub(crate) fn html(&self, language: &'static str, code: &str) -> Arc<str> {
        let key = (language, Box::from(code));
        if let Some(done) = self.done.lock().expect("never poisoned").get(&key) {
            return Arc::clone(done);
        }
        let html: Arc<str> = statac_highlight::html(language, code)
            .unwrap_or_else(|| plain(code))
            .into();
        let mut done = self.done.lock().expect("never poisoned");
        Arc::clone(done.entry(key).or_insert(html))
    }
}

/// The inside of a `<code>` holding `code` as it is, without colour.
pub(crate) fn plain(code: &str) -> String {
    let mut out = String::with_capacity(code.len());
    escape(code, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_word_of_five_characters_or_more_is_offered_the_nearest_language() {
        assert_eq!(nearest("cosole"), Some("console"));
        assert_eq!(nearest("Pyhton"), Some("python"));
        for word in ["qbasic", "glsl", "obj", "pc", "ssh", "gas"] {
            assert_eq!(nearest(word), None, "{word}");
        }
    }

    #[test]
    fn a_block_written_twice_is_coloured_once() {
        let highlighter = Highlighter::default();
        let code = "fn main() {}";
        let first = highlighter.html("rust", code);
        let second = highlighter.html("rust", code);
        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn code_of_no_grammar_carried_is_shown_as_it_is() {
        let html = Highlighter::default().html("no such language", "a < b");
        assert_eq!(&*html, "a &lt; b");
    }
}
