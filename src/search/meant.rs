//! What a search finding nothing may have meant: its words the index
//! doesn't list respelt as the nearest it does, as a reader's browser
//! respells them from the piece of the word list it reads for each.

use std::collections::HashMap;

use super::{Asked, Index, files, fold, in_utf16, spans, unspaced};
use crate::suggest::distance;

/// The fewest characters a word respelt has.
const SHORTEST: usize = 3;

/// The fewest characters a word respelt has to be two edits from another;
/// a shorter word may be one.
const TWO_EDITS: usize = 5;

impl Index {
    /// `query` with each of its words the index doesn't list respelt as
    /// the nearest word in the piece of the word list it falls in; none
    /// where some such word has none near, or `query` respelt still finds
    /// nothing. A word being typed, where `typing`, is listed where it
    /// starts a word listed. A word of fewer than three characters, or of
    /// Chinese, Japanese or Korean, is kept as written.
    pub(super) fn meant(&self, query: &str, typing: bool) -> Option<String> {
        let starts = self.starts.get_or_init(|| files::starts(self));
        self.meant_among(query, typing, starts)
    }

    /// [`Index::meant`], the word list cut into pieces at `starts`.
    pub(super) fn meant_among(
        &self,
        query: &str,
        typing: bool,
        starts: &[usize],
    ) -> Option<String> {
        let asked = Asked::of(query, typing);
        let looked = self.looked_in(&asked);
        let mut meant: HashMap<&str, &str> = HashMap::new();
        for (word, looked) in asked.words.iter().zip(&looked) {
            let kept = word.chars().nth(SHORTEST - 1).is_none() || word.chars().any(unspaced);
            if looked.is_empty() && !kept {
                meant.insert(word, self.nearest(word, starts)?);
            }
        }
        let mut respelt = String::new();
        let mut written = 0;
        for span in spans(query) {
            if let Some(nearest) = meant.get(fold(&query[span.clone()]).as_str()) {
                respelt.push_str(&query[written..span.start]);
                respelt.push_str(nearest);
                written = span.end;
            }
        }
        respelt.push_str(&query[written..]);
        let finds = !self.search(&Asked::of(&respelt, false)).is_empty();
        finds.then_some(respelt)
    }

    /// The word nearest `word` in the piece of the word list cut at
    /// `starts` that it falls in: the fewest edits away, then the one held
    /// by the most records, then the first. A word of fewer than five
    /// characters is near a word one edit away, a longer one two.
    fn nearest(&self, word: &str, starts: &[usize]) -> Option<&str> {
        let length = word.chars().count();
        let reach = if length < TWO_EDITS { 1 } else { 2 };
        let at = (starts.iter())
            .rposition(|&start| start == 0 || in_utf16(&self.words[start].0, word).is_le())?;
        let end = starts.get(at + 1).copied().unwrap_or(self.words.len());
        let mut nearest: Option<(usize, usize, &str)> = None;
        for (listed, entry) in &self.words[starts[at]..end] {
            if listed == word || listed.chars().count().abs_diff(length) > reach {
                continue;
            }
            let edits = distance(word, listed);
            let held = self.entries[*entry as usize].len();
            let nearer = nearest
                .is_none_or(|(least, most, _)| edits < least || (edits == least && held > most));
            if edits <= reach && nearer {
                nearest = Some((edits, held, listed));
            }
        }
        nearest.map(|(_, _, listed)| listed)
    }
}
