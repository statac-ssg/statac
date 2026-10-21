//! Shaping text and breaking it into lines.
//!
//! A shaped text doesn't depend on its size, so each is shaped once and
//! kept. A line is always measured by shaping it alone, both to decide
//! whether it fits and to answer its width, so that a line answered,
//! measured again by itself, comes back whole with the same width.

use std::collections::HashMap;
use std::ops::Range;

use cosmic_text::{FontSystem, Hinting, LayoutLine, ShapeLine, Shaping, SwashCache, Wrap};

use super::fonts::{self, Faces};

/// What drawing changes as it works: the text engine and what it has
/// shaped and drawn.
pub(super) struct Engine {
    pub system: FontSystem,
    pub faces: Faces,
    pub glyphs: SwashCache,
    shaped: HashMap<String, ShapeLine>,
}

/// The characters removed before shaping: the text and emoji presentation
/// selectors, which would only ask the font for a style it has or hasn't.
fn cleaned(text: &str) -> String {
    text.chars()
        .filter(|&c| c != '\u{FE0E}' && c != '\u{FE0F}')
        .collect()
}

/// Whether `c` ends a paragraph, as the bidirectional algorithm reads it.
fn ends_paragraph(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{1C}'..='\u{1E}' | '\u{85}' | '\u{2029}')
}

/// The paragraphs of `text` as they are set: none of them empty, and
/// without white space at their ends.
pub(super) fn paragraphs(text: &str) -> Vec<String> {
    cleaned(text)
        .split(ends_paragraph)
        .map(str::trim)
        .filter(|paragraph| !paragraph.is_empty())
        .map(str::to_owned)
        .collect()
}

impl Engine {
    pub(super) fn new(system: FontSystem, faces: Faces) -> Engine {
        Engine {
            system,
            faces,
            glyphs: SwashCache::new(),
            shaped: HashMap::new(),
        }
    }

    pub(super) fn shape(&mut self, text: &str) -> &ShapeLine {
        if !self.shaped.contains_key(text) {
            let attrs = self.faces.attrs(text);
            let line = ShapeLine::new(&mut self.system, text, &attrs, Shaping::Advanced, 4);
            self.shaped.insert(text.to_owned(), line);
        }
        &self.shaped[text]
    }

    /// `text` set on one line at `size`, unless it is empty.
    pub(super) fn set(&mut self, text: &str, size: f32) -> Option<LayoutLine> {
        let shaped = self.shape(text);
        let lines = shaped.layout(size, None, Wrap::None, None, None, Hinting::Disabled);
        lines.into_iter().next()
    }

    pub(super) fn width(&mut self, text: &str, size: f32) -> f32 {
        self.set(text, size).map_or(0.0, |line| line.w)
    }

    /// Where a line may break in `text`: before each word but the first,
    /// and at its end. A word with Hangul in it breaks only where a space
    /// is written, so no particle is parted from what it follows.
    fn opportunities(&mut self, text: &str) -> Vec<usize> {
        let shaped = self.shape(text);
        let mut starts: Vec<usize> = shaped
            .spans
            .iter()
            .flat_map(|span| &span.words)
            .filter(|word| !word.blank)
            .filter_map(|word| word.glyphs.iter().map(|glyph| glyph.start).min())
            .filter(|&start| start > 0)
            .filter(|&start| {
                let before = text[..start].chars().next_back();
                let after = text[start..].chars().next();
                let spaced = before.is_none_or(char::is_whitespace);
                spaced || !(before.is_some_and(fonts::hangul) || after.is_some_and(fonts::hangul))
            })
            .collect();
        starts.sort_unstable();
        starts.dedup();
        starts.push(text.len());
        starts
    }

    /// The lines `text` breaks into at `size` within `width`.
    pub(super) fn lines(&mut self, text: &str, size: f32, width: f32) -> Vec<String> {
        let mut lines = Vec::new();
        for paragraph in paragraphs(text) {
            let ranges = self.wrap(&paragraph, size, width);
            lines.extend(ranges.into_iter().map(|r| paragraph[r].to_owned()));
        }
        lines
    }

    fn fits(&mut self, text: &str, size: f32, width: f32) -> bool {
        self.width(text, size) <= width
    }

    /// The lines of one paragraph, without white space at their ends.
    fn wrap(&mut self, text: &str, size: f32, width: f32) -> Vec<Range<usize>> {
        if self.fits(text, size, width) {
            return std::iter::once(0..text.len()).collect();
        }
        let opportunities = self.opportunities(text);
        let mut lines = Vec::new();
        let mut start = 0;
        let mut fitted = None;
        let mut next = 0;
        while next < opportunities.len() {
            let end = opportunities[next];
            if end <= start {
                next += 1;
                continue;
            }
            if self.fits(text[start..end].trim(), size, width) {
                fitted = Some(end);
                next += 1;
                continue;
            }
            let at = match fitted.take() {
                Some(at) => at,
                None => self.split(text, start..end, size, width),
            };
            lines.push(trimmed(text, start..at));
            start = at;
        }
        if start < text.len() {
            lines.push(trimmed(text, start..text.len()));
        }
        lines.retain(|line| !line.is_empty());
        self.no_orphan(text, &opportunities, &mut lines, size, width);
        lines
    }

    /// Where to break a word wider than `width` by itself: after as many
    /// of its clusters as fit, and at least one.
    fn split(&mut self, text: &str, word: Range<usize>, size: f32, width: f32) -> usize {
        let clusters = fonts::clusters(&text[word.clone()]);
        let mut at = word.start + clusters[0].end;
        for cluster in &clusters[1..] {
            let end = word.start + cluster.end;
            if !self.fits(text[word.start..end].trim_end(), size, width) {
                break;
            }
            at = end;
        }
        at
    }

    /// Moves a character down where a spaceless script's last line would
    /// hold only one and the line before it three or more. Where the
    /// script forbids a break before the last character, as before a
    /// small kana, the line before gives up what follows its last break.
    fn no_orphan(
        &mut self,
        text: &str,
        opportunities: &[usize],
        lines: &mut [Range<usize>],
        size: f32,
        width: f32,
    ) {
        let [.., before, last] = lines else {
            return;
        };
        let alone = &text[last.clone()];
        let spaceless = |c: Option<char>| c.is_some_and(fonts::spaceless);
        if before.end != last.start
            || fonts::clusters(alone).len() != 1
            || !spaceless(alone.chars().next())
            || !spaceless(text[before.clone()].chars().next_back())
            || fonts::clusters(&text[before.clone()]).len() < 3
        {
            return;
        }
        let Some(&at) = opportunities
            .iter()
            .rev()
            .find(|&&at| before.start < at && at < before.end)
        else {
            return;
        };
        if fonts::clusters(&text[before.start..at]).len() < 2
            || !self.fits(text[before.start..at].trim_end(), size, width)
            || !self.fits(&text[at..last.end], size, width)
        {
            return;
        }
        before.end = at;
        last.start = at;
    }
}

fn trimmed(text: &str, range: Range<usize>) -> Range<usize> {
    let part = &text[range.clone()];
    let start = range.start + (part.len() - part.trim_start().len());
    let end = range.end - (part.len() - part.trim_end().len());
    start..end.max(start)
}
