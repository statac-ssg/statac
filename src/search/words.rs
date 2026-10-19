//! The words search reads in a text.

use std::ops::Range;

/// Where each word of `text` is: runs of letters and digits, an
/// apostrophe between a word and a letter kept inside it.
pub fn spans(text: &str) -> impl Iterator<Item = Range<usize>> + '_ {
    let mut chars = text.char_indices().peekable();
    std::iter::from_fn(move || {
        let (start, _) = chars.by_ref().find(|(_, c)| c.is_alphanumeric())?;
        let mut end = text.len();
        while let Some(&(at, c)) = chars.peek() {
            let inner = matches!(c, '\'' | '\u{2019}')
                && (text[at + c.len_utf8()..].chars().next()).is_some_and(char::is_alphabetic);
            if !(c.is_alphanumeric() || inner) {
                end = at;
                break;
            }
            chars.next();
        }
        Some(start..end)
    })
}

/// The words of `text`, in small letters.
pub fn words(text: &str) -> impl Iterator<Item = String> + '_ {
    spans(text).map(|span| text[span].to_lowercase())
}
