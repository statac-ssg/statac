//! The files a build reads, and places within them.

use std::path::{Path, PathBuf};

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub struct SourceId(u32);

impl SourceId {
    pub(crate) fn index(self) -> u32 {
        self.0
    }

    pub(crate) fn from_index(index: u32) -> Self {
        SourceId(index)
    }
}

/// A byte range within one source.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        assert!(start <= end, "span runs backwards: {start}..{end}");
        Span {
            start: u32::try_from(start).expect("source over 4 GiB"),
            end: u32::try_from(end).expect("source over 4 GiB"),
        }
    }

    pub fn range(self) -> std::ops::Range<usize> {
        self.start as usize..self.end as usize
    }
}

/// Where something was written: a span in a source.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Place {
    pub source: SourceId,
    pub span: Span,
}

/// A line and column, both counted from 1. The column counts characters, not bytes.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct LineColumn {
    pub line: usize,
    pub column: usize,
}

struct Source {
    path: PathBuf,
    text: String,
    line_starts: Vec<usize>,
    /// Spans holding a secret, never shown when a line is drawn.
    hidden: Vec<Span>,
}

/// Where each line of `text` starts.
pub fn line_starts(text: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(text.match_indices('\n').map(|(i, _)| i + 1))
        .collect()
}

/// Every source the build has read, kept whole so any place can be shown later.
#[derive(Default)]
pub struct Sources {
    sources: Vec<Source>,
}

impl Sources {
    pub fn add(&mut self, path: impl Into<PathBuf>, text: String) -> SourceId {
        let lines = line_starts(&text);
        self.add_with_lines(path, text, lines)
    }

    /// [`Sources::add`], for a text whose [`line_starts`] are found
    /// already, as many files read at once find them.
    pub fn add_with_lines(
        &mut self,
        path: impl Into<PathBuf>,
        text: String,
        line_starts: Vec<usize>,
    ) -> SourceId {
        let id = SourceId(u32::try_from(self.sources.len()).expect("too many sources"));
        self.sources.push(Source {
            path: path.into(),
            text,
            line_starts,
            hidden: Vec::new(),
        });
        id
    }

    /// Marks `place` as holding a secret, such as a token, so that no line
    /// drawn from its source shows it.
    /// Overlapping spans are merged, so a secret is masked once.
    pub fn hide(&mut self, place: Place) {
        let hidden = &mut self.sources[place.source.0 as usize].hidden;
        let mut span = place.span;
        hidden.retain(|other| {
            let overlaps = other.start < span.end && span.start < other.end;
            if overlaps {
                span.start = span.start.min(other.start);
                span.end = span.end.max(other.end);
            }
            !overlaps
        });
        hidden.push(span);
        hidden.sort_by_key(|s| s.start);
    }

    /// The spans of a source that hold secrets.
    pub fn hidden(&self, id: SourceId) -> &[Span] {
        &self.get(id).hidden
    }

    /// Every source, in the order read.
    pub fn ids(&self) -> impl Iterator<Item = SourceId> {
        (0..self.sources.len() as u32).map(SourceId)
    }

    pub fn path(&self, id: SourceId) -> &Path {
        &self.get(id).path
    }

    pub fn text(&self, id: SourceId) -> &str {
        &self.get(id).text
    }

    /// The text exactly as the author wrote it.
    pub fn spelling(&self, place: Place) -> &str {
        &self.text(place.source)[place.span.range()]
    }

    pub fn line_column(&self, id: SourceId, offset: usize) -> LineColumn {
        let source = self.get(id);
        let line = source.line_starts.partition_point(|&s| s <= offset) - 1;
        let mut start = source.line_starts[line];
        // A byte-order mark is no character the author can see.
        let mark = '\u{feff}'.len_utf8();
        if line == 0 && source.text.starts_with('\u{feff}') && offset >= mark {
            start = mark;
        }
        LineColumn {
            line: line + 1,
            column: source.text[start..offset].chars().count() + 1,
        }
    }

    /// The line `offset` falls on, counted from 1. An offset past the end is
    /// on the last line.
    pub fn line_number(&self, id: SourceId, offset: usize) -> usize {
        self.get(id).line_starts.partition_point(|&s| s <= offset)
    }

    /// Where line `line` (counted from 1) starts.
    pub fn line_start(&self, id: SourceId, line: usize) -> usize {
        self.get(id).line_starts[line - 1]
    }

    /// The whole of line `line` (counted from 1), without its line ending.
    pub fn line(&self, id: SourceId, line: usize) -> &str {
        let source = self.get(id);
        let start = source.line_starts[line - 1];
        let end = source
            .line_starts
            .get(line)
            .copied()
            .unwrap_or(source.text.len());
        source.text[start..end].trim_end_matches(['\n', '\r'])
    }

    fn get(&self, id: SourceId) -> &Source {
        &self.sources[id.0 as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(text: &str) -> (Sources, SourceId) {
        let mut sources = Sources::default();
        let id = sources.add("content/post-1.md", text.to_owned());
        (sources, id)
    }

    #[test]
    fn line_and_column_count_from_one() {
        let (sources, id) = one("title: Post 1\ndate: 2026-10-04\n");
        assert_eq!(
            sources.line_column(id, 0),
            LineColumn { line: 1, column: 1 }
        );
        assert_eq!(
            sources.line_column(id, 20),
            LineColumn { line: 2, column: 7 }
        );
    }

    #[test]
    fn columns_count_characters_not_bytes() {
        let (sources, id) = one("title: Café Ölçü\n");
        let offset = "title: Café Ö".len();
        assert_eq!(sources.line_column(id, offset).column, 14);
    }

    #[test]
    fn a_byte_order_mark_takes_no_column() {
        let (sources, id) = one("\u{feff}title: Post 1\n");
        assert_eq!(
            sources.line_column(id, 3),
            LineColumn { line: 1, column: 1 }
        );
        assert_eq!(sources.line_column(id, 10).column, 8);
    }

    #[test]
    fn a_place_at_the_very_end_is_on_the_last_line() {
        let (sources, id) = one("a\nb");
        assert_eq!(
            sources.line_column(id, 3),
            LineColumn { line: 2, column: 2 }
        );
    }

    #[test]
    fn spelling_is_the_written_text() {
        let (sources, id) = one("version: 3.10\n");
        let place = Place {
            source: id,
            span: Span::new(9, 13),
        };
        assert_eq!(sources.spelling(place), "3.10");
    }

    #[test]
    fn a_line_is_given_without_its_ending() {
        let (sources, id) = one("one\r\ntwo\nthree");
        assert_eq!(sources.line(id, 1), "one");
        assert_eq!(sources.line(id, 2), "two");
        assert_eq!(sources.line(id, 3), "three");
    }
}
