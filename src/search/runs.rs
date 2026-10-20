//! How a search reads a run of words of Chinese, Japanese or Korean: by
//! the words the index lists, not only where the words were cut, since
//! a browser's `Intl.Segmenter` may cut such a run otherwise than the
//! index did. A reader's browser reads a run the same way.

use std::ops::Range;

use super::fold;

/// The most code points a word a run is read as has.
pub(super) const LONGEST: usize = 16;

/// The run of words at `words` in `text`, each next to the one before,
/// read as the fewest words that `listed` holds, folded, cut where the
/// words are cut or anywhere in a word `listed` doesn't hold; between
/// readings of as many, the one whose first word is longest, then whose
/// second is, and so on. Where `typing`, its last word may be one that
/// only `starts` a word listed. None where no reading covers the run.
pub(super) fn read(
    text: &str,
    words: &[Range<usize>],
    typing: bool,
    listed: impl Fn(&str) -> bool,
    starts: impl Fn(&str) -> bool,
) -> Option<Vec<String>> {
    let mut cuts = Vec::new();
    for word in words {
        cuts.push(word.start);
        if !listed(&fold(&text[word.clone()])) {
            let within = text[word.clone()].char_indices().skip(1);
            cuts.extend(within.map(|(at, _)| word.start + at));
        }
    }
    cuts.push(words.last()?.end);
    let last = cuts.len() - 1;
    // From each cut, the fewest words reading the rest, and where the
    // first of them ends.
    let mut best: Vec<Option<(usize, usize)>> = vec![None; cuts.len()];
    best[last] = Some((0, last));
    for from in (0..last).rev() {
        for to in from + 1..=last {
            let piece = &text[cuts[from]..cuts[to]];
            if piece.chars().nth(LONGEST).is_some() {
                break;
            }
            let Some((after, _)) = best[to] else {
                continue;
            };
            let word = fold(piece);
            let fits = listed(&word) || (typing && to == last && starts(&word));
            // Of readings of as many words, the one whose first word ends
            // later wins, and `to` only rises.
            if fits && best[from].is_none_or(|(fewest, _)| after < fewest) {
                best[from] = Some((after + 1, to));
            }
        }
    }
    let mut read = Vec::new();
    let mut from = 0;
    while from < last {
        let (_, to) = best[from]?;
        read.push(fold(&text[cuts[from]..cuts[to]]));
        from = to;
    }
    Some(read)
}
