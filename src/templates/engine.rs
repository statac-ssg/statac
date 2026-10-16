//! What the template engine stopped at, in Statac's own terms: its errors
//! read into typed faults, each placed at the text it is about.
//!
//! The engine says what it stopped at in its own words, which are matched
//! here; the engine is pinned to one release, and a test runs each case
//! through it, so a change to its words fails a test rather than reaching
//! an author. What a message names is taken from the template as written.

mod stopped;
mod syntax;

pub(super) use stopped::{Site, missing_operand, read_stopped};
pub(super) use syntax::read_syntax;

use std::ops::Range;

/// The line `offset` is on, counted from 1.
fn line_of(text: &str, offset: usize) -> usize {
    text[..offset.min(text.len())].matches('\n').count() + 1
}

/// The engine's range, kept within the text.
fn within(text: &str, range: Option<Range<usize>>) -> Range<usize> {
    let range = range.unwrap_or(0..0);
    let start = range.start.min(text.len());
    start..range.end.clamp(start, text.len())
}
