//! Control characters in the writing. Each is left out of the page and
//! said at its place: a reader sees nothing where one stands, and HTML
//! reads one as a mistake in the page. The tab and the line endings are
//! the writing's own and stay.

use std::borrow::Cow;

use crate::source::Span;

/// Whether `c` is a control character the page never carries: C0 but for
/// the tab, line feed and carriage return; DEL; and C1.
pub(super) fn left_out(c: char) -> bool {
    matches!(c, '\0'..='\x08' | '\x0b' | '\x0c' | '\x0e'..='\x1f' | '\x7f' | '\u{80}'..='\u{9f}')
}

/// `text` without its control characters.
pub(super) fn without(text: &str) -> Cow<'_, str> {
    match text.contains(left_out) {
        true => Cow::Owned(text.chars().filter(|c| !left_out(*c)).collect()),
        false => Cow::Borrowed(text),
    }
}

/// Each control character in `text` from `from` to `to`, placed, with the
/// character.
pub(super) fn found(text: &str, from: usize, to: usize) -> impl Iterator<Item = (Span, char)> {
    text[from..to]
        .char_indices()
        .filter(|(_, c)| left_out(*c))
        .map(move |(at, c)| (Span::new(from + at, from + at + c.len_utf8()), c))
}

#[cfg(test)]
mod tests;
