//! Calls that wrap writing: a call to a snippet that shows `body`, on a
//! line of its own, wraps the lines below it up to a line that is `@end`.
//!
//! ```markdown
//! @note('warning')
//! The writing it wraps, *read as Markdown*.
//! @end
//! ```
//!
//! The call, its writing and its `@end` are found in the text before
//! Markdown reads it, and masked as one. The writing is read as Markdown
//! of its own, with the margin that puts the call where it is taken off
//! each line: the quotation markers of a quotation, and the indentation of
//! a list item. Its links and footnote markers find definitions written
//! anywhere on the page, as the page finds those written inside it. Calls
//! nest; each `@end` closes the innermost call open.

use super::call;
use super::node::Node;
use crate::origin::Origin;
use crate::problem::Problem;
use crate::source::{SourceId, Span};

/// Past this many calls inside one another, a call is read as wrapping
/// nothing, so a page can't nest its way into exhausting the stack.
pub(super) const DEEPEST: usize = 32;

/// Whether only a line's margin comes before `at` on its line, and only
/// space after `end`.
pub(super) fn stands_alone(b: &[u8], line: usize, at: usize, end: usize) -> bool {
    b[line..at].iter().all(|c| matches!(c, b' ' | b'\t' | b'>')) && call::blank_from(b, end)
}

/// Whether the `@` at `at` begins an `@end` standing alone on its line.
pub(super) fn is_end(b: &[u8], line: usize, at: usize) -> bool {
    b[at..].starts_with(b"@end") && stands_alone(b, line, at, at + "@end".len())
}

/// Whether the line holding `at` is inside a fenced block of code that
/// opens after `from`.
pub(super) fn fenced(b: &[u8], from: usize, at: usize) -> bool {
    let mut fences = Fences::default();
    let mut line = from;
    let last = line_start(b, at);
    while line < last {
        fences.read(b, line);
        let Some(next) = next_line(b, line) else {
            break;
        };
        line = next;
    }
    fences.open.is_some()
}

/// Fences opened and closed, a line at a time.
#[derive(Default)]
struct Fences {
    /// The fence's character and length, while one is open.
    open: Option<(u8, usize)>,
}

impl Fences {
    /// Reads the line starting at `line`, and says whether it is a fence.
    fn read(&mut self, b: &[u8], line: usize) -> bool {
        let at = past_margin(b, line);
        let Some(&fence @ (b'`' | b'~')) = b.get(at) else {
            return false;
        };
        let run = b[at..].iter().take_while(|&&c| c == fence).count();
        if run < 3 {
            return false;
        }
        match self.open {
            Some((open, length)) if open == fence && run >= length => {
                if call::blank_from(b, at + run) {
                    self.open = None;
                    return true;
                }
                false
            }
            Some(_) => false,
            None => {
                let info = &b[at + run..next_line(b, at).unwrap_or(b.len())];
                if fence == b'`' && info.contains(&b'`') {
                    return false;
                }
                self.open = Some((fence, run));
                true
            }
        }
    }
}

/// Past the spaces and quotation markers opening the line at `line`.
fn past_margin(b: &[u8], line: usize) -> usize {
    line + b[line..]
        .iter()
        .take_while(|c| matches!(c, b' ' | b'\t' | b'>'))
        .count()
}

/// Where the `@end` closing the call that ends at `from` is, counting the
/// calls that wrap writing between, or none where the writing runs out
/// first. An `@end` or a call in a fenced block of code is the code's.
pub(super) fn end_of(
    text: &str,
    source: SourceId,
    from: usize,
    wraps: super::Wraps<'_>,
) -> Option<usize> {
    let b = text.as_bytes();
    let mut open = 1;
    let mut fences = Fences::default();
    let mut line = next_line(b, from)?;
    loop {
        let at = past_margin(b, line);
        if fences.read(b, line) || fences.open.is_some() {
            line = next_line(b, at)?;
            continue;
        }
        if b.get(at) == Some(&b'@') {
            if is_end(b, line, at) {
                open -= 1;
                if open == 0 {
                    return Some(at);
                }
            } else if let Some(Ok(read)) = call::read(text, source, at, call::depth(b, line))
                && wraps(&read.call.name.value) == Some(true)
                && stands_alone(b, line, at, read.end)
            {
                open += 1;
                line = next_line(b, read.end)?;
                continue;
            }
        }
        line = next_line(b, at)?;
    }
}

/// The start of the line after the one holding `at`.
pub(super) fn next_line(b: &[u8], at: usize) -> Option<usize> {
    let newline = b[at..].iter().position(|&c| c == b'\n')?;
    Some(at + newline + 1)
}

pub(super) fn line_start(b: &[u8], at: usize) -> usize {
    b[..at]
        .iter()
        .rposition(|&c| c == b'\n')
        .map_or(0, |p| p + 1)
}

/// The writing a call wraps, with the margin of the call's own line taken
/// off each of its lines, and where each line's writing is in the file.
pub(super) struct Wrapped {
    pub text: String,
    /// Each line's start in `text`, and where that is in the file.
    lines: Vec<(usize, usize)>,
}

impl Wrapped {
    /// The lines from `from` up to the line starting at `to`, below the
    /// call at `at`.
    pub(super) fn new(text: &str, at: usize, from: usize, to: usize) -> Wrapped {
        let b = text.as_bytes();
        let margin = Margin::of(b, line_start(b, at), at);
        let mut wrapped = Wrapped {
            text: String::with_capacity(to.saturating_sub(from)),
            lines: Vec::new(),
        };
        let mut line = from;
        while line < to {
            let end = next_line(b, line).unwrap_or(text.len()).min(to);
            let kept = margin.past(b, line, end);
            wrapped.lines.push((wrapped.text.len(), kept));
            wrapped.text.push_str(&text[kept..end]);
            line = end;
        }
        wrapped
    }

    /// Where an offset in the wrapped writing is in the file.
    fn offset(&self, at: u32) -> u32 {
        let at = at as usize;
        let line = self.lines.partition_point(|&(start, _)| start <= at);
        let found = match line {
            0 => at,
            n => {
                let (start, file) = self.lines[n - 1];
                file + (at - start)
            }
        };
        u32::try_from(found).expect("a file under 4 GiB")
    }

    pub(super) fn span(&self, span: &mut Span) {
        span.start = self.offset(span.start);
        span.end = self.offset(span.end).max(span.start);
    }

    fn origin(&self, origin: &mut Origin) {
        if let Some(place) = &mut origin.place {
            self.span(&mut place.span);
        }
    }

    /// Places what was read from the wrapped writing in the file.
    pub(super) fn place(&self, nodes: &mut [Node], problems: &mut [Problem]) {
        for node in nodes {
            node.walk_mut(&mut |node| {
                self.span(&mut node.place.span);
                if let super::Kind::Call(call) = &mut node.kind {
                    if let Some(head) = &mut call.head {
                        self.span(head);
                    }
                    self.origin(&mut call.name.origin);
                    for argument in &mut call.arguments {
                        if let Some(name) = &mut argument.name {
                            self.origin(&mut name.origin);
                        }
                        self.origin(&mut argument.value.origin);
                    }
                }
            });
        }
        for problem in problems {
            if let Some(place) = &mut problem.place {
                self.span(&mut place.span);
            }
        }
    }
}

/// What puts a call where it is: the quotation markers before it, and the
/// indentation after them.
struct Margin {
    markers: usize,
    columns: usize,
}

impl Margin {
    fn of(b: &[u8], line: usize, at: usize) -> Margin {
        let markers = b[line..at].iter().filter(|&&c| c == b'>').count();
        let past = Margin {
            markers,
            columns: 0,
        }
        .past(b, line, at);
        Margin {
            markers,
            columns: columns(&b[past..at]),
        }
    }

    /// Where the writing of the line from `line` to `end` starts once this
    /// margin is taken off: each quotation marker with the space after it,
    /// then up to as much indentation. A line written with less keeps
    /// what it has.
    fn past(&self, b: &[u8], line: usize, end: usize) -> usize {
        let mut i = line;
        for _ in 0..self.markers {
            let mut j = i;
            while j < end && j - i < 3 && b[j] == b' ' {
                j += 1;
            }
            if j >= end || b[j] != b'>' {
                break;
            }
            i = j + 1;
            if i < end && matches!(b[i], b' ' | b'\t') {
                i += 1;
            }
        }
        let mut column = 0;
        while i < end && column < self.columns {
            match b[i] {
                b' ' => column += 1,
                b'\t' => column += 4 - column % 4,
                _ => break,
            }
            i += 1;
        }
        i
    }
}

fn columns(space: &[u8]) -> usize {
    space.iter().fold(0, |column, c| match c {
        b'\t' => column + 4 - column % 4,
        _ => column + 1,
    })
}
