//! How a built page's source keeps the indentation its templates were
//! written with. What a template puts on a line of its own follows that
//! line's indent on each of its lines, a body the engine held back in
//! place of the indent its lines share, a line that came to nothing is not
//! written, and each line is written where `landing` says.
//!
//! The engine can't say what column a value is printed at, so each render
//! writes to a [`Sink`] that keeps account of the line being written, and
//! the formatter asks it. A body the engine holds back, such as a macro's,
//! is written to a sink of its own, laid out as the template rendering it
//! lays out its own lines, and is then a value written whole.

use std::cell::RefCell;
use std::ops::Range;
use std::sync::Arc;
use std::{fmt, io};

use minijinja::{Captured, Error, Held, Output, Template, Value};

use super::dedent::Head;
use super::landing::{Deeper, Landing, Place, Placing};
use crate::html::{Markup, PREFORMATTED};
use crate::source::Place as SourcePlace;

/// The line a template, or a body the engine holds back, is writing.
struct Line {
    /// Where it starts in the output.
    start: usize,
    /// The spaces and tabs it opens with.
    indent: String,
    /// Whether nothing but those has been written on it.
    open: bool,
    /// Whether a value that came to nothing was put on it.
    emptied: bool,
    /// Whether the value put before it closed with its own line ending.
    ended: bool,
    /// Whether what is being written is a value, not the template's own
    /// text.
    value: bool,
    /// Whether a value written next is a body the engine held back, alone
    /// on an indented line of the template outside `<pre>` and
    /// `<textarea>`, so that its lines line up at the line.
    placed: bool,
    /// Where the value being written was written, where that is known.
    printing: Option<Printing>,
}

thread_local! {
    /// One for each sink being written, the innermost last: a snippet's
    /// above its caller's, a body's above the template's rendering it.
    static LINES: RefCell<Vec<Line>> = const { RefCell::new(Vec::new()) };
    /// Where the lines of each template being rendered are written, a
    /// snippet's above its caller's.
    static PLACES: RefCell<Vec<Place>> = const { RefCell::new(Vec::new()) };
}

/// A run of what a template wrote that wasn't a value's: where it lies in
/// the output, and where in memory it was written from. The engine writes
/// a template's own text from the text it holds, so that says where in the
/// template the run is written.
#[derive(Copy, Clone, Debug)]
struct Run {
    at: usize,
    from: usize,
    len: usize,
}

/// A run of what a template wrote as a value: where it lies in the output,
/// where the value was written, and where in the value's text the run
/// starts, where it is that text's and not an indent put before a line.
#[derive(Clone, Debug)]
struct Print {
    at: usize,
    len: usize,
    within: Option<usize>,
    from: Printed,
}

/// Where a value a template printed was written.
#[derive(Clone, Debug)]
pub(crate) enum Printed {
    /// Markup a file of the site holds, as a field or data does: the text
    /// written at this place; or what a snippet call written there wrote,
    /// with where in that each part was written.
    Text(SourcePlace, Option<Arc<Own>>),
    /// What the template made of anything else, printed at this place:
    /// what `{{ ... }}` holds; with where its text was written, where a
    /// body the engine held back or a snippet wrote it.
    Made(SourcePlace, Option<Arc<Own>>),
}

/// Where a byte of an output was written.
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum Origin {
    /// A template's own text, written from this address in memory.
    Own(usize),
    /// Markup a file of the site holds, written at this place, with where
    /// in its text the byte is, where that is known.
    Text(SourcePlace, Option<usize>),
    /// What a template made, printed at this place.
    Made(SourcePlace),
}

/// Where what a template wrote is its own text, and where each value it
/// printed was written, in the order written.
#[derive(Clone, Default, Debug)]
pub(crate) struct Own {
    runs: Vec<Run>,
    prints: Vec<Print>,
}

impl Own {
    /// Where in memory the byte at `at` of the output was written from;
    /// none for a byte of a value's.
    pub(super) fn written_from(&self, at: usize) -> Option<usize> {
        let after = self.runs.partition_point(|run| run.at <= at);
        let run = self.runs[..after].last()?;
        (at - run.at < run.len).then(|| run.from + (at - run.at))
    }

    /// Where the byte at `at` of the output was written: the template's own
    /// text, or else the value it is part of, followed into the text of a
    /// body or snippet that wrote that value where it can be.
    pub(super) fn origin(&self, at: usize) -> Option<Origin> {
        if let Some(from) = self.written_from(at) {
            return Some(Origin::Own(from));
        }
        let (print, within) = self.print_at(at)?;
        Some(match &print.from {
            Printed::Text(place, None) => Origin::Text(*place, within),
            // What a snippet call wrote, at the call where nothing in the
            // snippet places it.
            Printed::Text(place, Some(inner)) => (within.and_then(|within| inner.origin(within)))
                .unwrap_or(Origin::Text(*place, None)),
            Printed::Made(place, inner) => (inner.as_ref().zip(within))
                .and_then(|(inner, within)| inner.origin(within))
                .unwrap_or(Origin::Made(*place)),
        })
    }

    /// The value printed holding the byte at `at`, and where in its text
    /// the byte is, where that is known.
    fn print_at(&self, at: usize) -> Option<(&Print, Option<usize>)> {
        let after = self.prints.partition_point(|print| print.at <= at);
        let print = (self.prints[..after].last()).filter(|print| at - print.at < print.len)?;
        Some((print, print.within.map(|within| within + (at - print.at))))
    }

    /// The outermost markup of a file's holding the byte at `at`, followed
    /// through bodies and snippets as [`Own::origin`] follows them: for a
    /// byte a snippet called in the writing wrote, the call.
    pub(super) fn in_text(&self, at: usize) -> Option<SourcePlace> {
        if self.written_from(at).is_some() {
            return None;
        }
        match self.print_at(at)? {
            (
                Print {
                    from: Printed::Text(place, _),
                    ..
                },
                _,
            ) => Some(*place),
            (
                Print {
                    from: Printed::Made(_, inner),
                    ..
                },
                within,
            ) => inner.as_ref()?.in_text(within?),
        }
    }

    /// Forgets what was written from `len` on, which the output no longer
    /// holds.
    fn truncate(&mut self, len: usize) {
        while let Some(run) = self.runs.last_mut() {
            if run.at + run.len <= len {
                break;
            }
            if run.at < len {
                run.len = len - run.at;
                break;
            }
            self.runs.pop();
        }
        while let Some(print) = self.prints.last_mut() {
            if print.at + print.len <= len {
                break;
            }
            if print.at < len {
                print.len = len - print.at;
                break;
            }
            self.prints.pop();
        }
    }

    /// Where a text `len` long was written: as written at `place`.
    pub(super) fn text(place: SourcePlace, len: usize) -> Own {
        let print = Print {
            at: 0,
            len,
            within: Some(0),
            from: Printed::Text(place, None),
        };
        Own {
            runs: Vec::new(),
            prints: vec![print],
        }
    }

    /// Where what is copied from the output, piece by piece, was written:
    /// each piece where it is in the copy, and where it starts in the
    /// output.
    pub(super) fn copied(&self, pieces: &[(Range<usize>, usize)]) -> Own {
        let mut own = Own::default();
        for (to, from) in pieces {
            self.copy(&mut own, to.start, *from..from + to.len());
        }
        own
    }

    /// Where the output, with each address in `respelt` spelled again, was
    /// written: an address as the one it replaces, byte for byte as far as
    /// both go, and the rest of it where the last byte replaced was.
    pub(super) fn respelt(&self, respelt: &crate::elsewhere::Respelt, len: usize) -> Own {
        let mut own = Own::default();
        let (mut old, mut new) = (0, 0);
        for (was, now) in respelt {
            self.copy(&mut own, new, old..was.start);
            let both = was.len().min(now.len());
            self.copy(&mut own, now.start, was.start..was.start + both);
            let last = self.prints.partition_point(|print| print.at < was.end);
            let last = (self.prints[..last].last()).filter(|print| print.at + print.len >= was.end);
            if let Some(print) = last.filter(|_| now.len() > both) {
                own.prints.push(Print {
                    at: now.start + both,
                    len: now.len() - both,
                    within: None,
                    from: print.from.clone(),
                });
            }
            (old, new) = (was.end, now.end);
        }
        self.copy(&mut own, new, old..len);
        own
    }

    /// Copies into `own`, at `to`, what is known of where bytes `from` of
    /// the output were written.
    fn copy(&self, own: &mut Own, to: usize, from: Range<usize>) {
        let moved = |at: usize| to + at - from.start;
        let first = self
            .runs
            .partition_point(|run| run.at + run.len <= from.start);
        for run in self.runs[first..]
            .iter()
            .take_while(|run| run.at < from.end)
        {
            let (start, end) = (run.at.max(from.start), (run.at + run.len).min(from.end));
            own.runs.push(Run {
                at: moved(start),
                from: run.from + (start - run.at),
                len: end - start,
            });
        }
        let first = self
            .prints
            .partition_point(|print| print.at + print.len <= from.start);
        for print in self.prints[first..]
            .iter()
            .take_while(|print| print.at < from.end)
        {
            let (start, end) = (
                print.at.max(from.start),
                (print.at + print.len).min(from.end),
            );
            own.prints.push(Print {
                at: moved(start),
                len: end - start,
                within: print.within.map(|within| within + (start - print.at)),
                from: print.from.clone(),
            });
        }
    }

    /// Whether nothing is known of where any of the output was written.
    pub(super) fn is_empty(&self) -> bool {
        self.runs.is_empty() && self.prints.is_empty()
    }

    /// Where a page's writing, `html`, is written, from where each part of
    /// it is in `html`, nested, and where in the page's file that part is:
    /// each byte the innermost part's, and known within the part where the
    /// file spells the part as `html` has it. Each line a snippet call
    /// wrote, in `called`, is followed into what the snippet wrote where
    /// `calls` says, by the call's place.
    pub(super) fn of_writing(
        traced: &[(Range<usize>, SourcePlace)],
        called: &[crate::markdown::Called],
        calls: &[(SourcePlace, Arc<Own>)],
        html: &str,
        sources: &crate::source::Sources,
    ) -> Own {
        // A call's lines after the nodes, so a line as long as its call is
        // taken as inside it.
        let mut parts: Vec<Part> = (traced.iter())
            .map(|(range, place)| (range.clone(), *place, None))
            .collect();
        parts.extend(called.iter().filter_map(|line| {
            let (_, own) = calls.iter().find(|(place, _)| *place == line.place)?;
            Some((line.at.clone(), line.place, Some((line.within, own))))
        }));
        let mut order: Vec<usize> = (0..parts.len()).collect();
        order.sort_by_key(|&at| (parts[at].0.start, std::cmp::Reverse(parts[at].0.end)));
        let mut spelt: Vec<Option<bool>> = vec![None; parts.len()];
        let mut prints = Vec::new();
        let mut put = |from: usize, to: usize, part: usize| {
            if from >= to {
                return;
            }
            let (range, place, call) = &parts[part];
            let (within, inner) = match call {
                Some((within, own)) => (Some(within + from - range.start), Some(Arc::clone(own))),
                None => {
                    let same = *spelt[part].get_or_insert_with(|| {
                        html.get(range.clone()) == Some(sources.spelling(*place))
                    });
                    (same.then(|| from - range.start), None)
                }
            };
            prints.push(Print {
                at: from,
                len: to - from,
                within,
                from: Printed::Text(*place, inner),
            });
        };
        // The parts open around the byte reached, innermost last, each with
        // where it ends.
        let mut open: Vec<(usize, usize)> = Vec::new();
        let mut reached = 0;
        for at in order {
            let range = &parts[at].0;
            while let Some(&(end, part)) = open.last() {
                if end > range.start {
                    break;
                }
                put(reached, end, part);
                reached = reached.max(end);
                open.pop();
            }
            if let Some(&(_, part)) = open.last() {
                put(reached, range.start, part);
            }
            reached = reached.max(range.start);
            let end = open
                .last()
                .map_or(range.end, |&(end, _)| range.end.min(end));
            open.push((end, at));
        }
        while let Some((end, part)) = open.pop() {
            put(reached, end, part);
            reached = reached.max(end);
        }
        Own {
            runs: Vec::new(),
            prints,
        }
    }
}

/// A part of a page's writing: where it is in the writing's HTML, where it
/// is written, and for a line a snippet call wrote, where the line starts
/// in what the call wrote and where that was written.
type Part<'a> = (Range<usize>, SourcePlace, Option<(usize, &'a Arc<Own>)>);

/// A value being written: where it was written, and where its text is in
/// memory, by address and length.
struct Printing {
    from: Printed,
    text: (usize, usize),
}

/// What one template writes, or one body the engine holds back as it
/// renders one, and where its lines are written: nowhere in particular
/// where nothing is known of them.
struct Sink {
    place: Option<Place>,
    layout: Layout,
}

/// What a sink has written, and the account it keeps of it.
struct Layout {
    out: Vec<u8>,
    own: Own,
    /// Whether what it writes starts a line where it is put.
    begins: bool,
    /// Where in memory the template's own text last written ends, until
    /// anything else is written.
    last: Option<usize>,
    /// The recursive loops' calls being run, the deepest last.
    deeper: Vec<Deeper>,
    /// How long the output was as the last line whose line ending is kept
    /// began to write what it holds, until a kept line ending is reached.
    /// Where that line's own is left out, as in a branch not taken, the
    /// next kept one ends what it wrote.
    began: Option<usize>,
}

impl Sink {
    fn new(place: Option<Place>, begins: bool) -> Self {
        LINES.with_borrow_mut(|lines| {
            lines.push(Line {
                start: 0,
                indent: String::new(),
                open: true,
                emptied: false,
                ended: false,
                value: false,
                placed: false,
                printing: None,
            });
        });
        Self {
            place,
            layout: Layout {
                out: Vec::new(),
                own: Own::default(),
                begins,
                last: None,
                deeper: Vec::new(),
                began: None,
            },
        }
    }

    fn write(&mut self, text: &[u8]) {
        let placing = self.place.as_ref().map_or(Placing::NONE, Place::placing);
        self.layout.write(placing, text);
    }
}

impl Layout {
    /// Writes the template's own text, each line that starts a line of the
    /// output at the indent [`Placing`] gives it. A line that came to
    /// nothing, spaces before its line ending and no more, isn't written.
    fn own(&mut self, placing: Placing<'_>, mut text: &[u8]) {
        let mut from = text.as_ptr() as usize;
        // An owed line ending is not written where nothing has been written
        // for it to end.
        if let Some(line) = placing.holding(from)
            && line.owed() == Some(from)
        {
            let wrote = self.began.take().is_some_and(|len| self.out.len() > len);
            if !wrote {
                let ending = if text.starts_with(b"\r\n") { 2 } else { 1 };
                text = &text[ending.min(text.len())..];
                from += ending;
            }
        }
        let mut done = 0;
        for line in placing.starting(from..from + text.len()) {
            let at = line.at - from;
            let starts = if at > 0 {
                self.ending(placing, &text[done..at]);
                done = at;
                true
            } else {
                self.begins || !self.out.is_empty()
            };
            let end = at + line.indent;
            if end > text.len() || blank(&text[at..end]) < line.indent {
                continue;
            }
            if !starts || !self.open() {
                // An indent kept for a line of the output is no use inside one.
                if line.head == Head::AtLineStart {
                    self.put(&text[done..at]);
                    done = end;
                }
                continue;
            }
            if line.movable && self.land(placing, line, Some(&text[at..end])) {
                done = end;
            }
        }
        if done < text.len() {
            self.ending(placing, &text[done..]);
        }
    }

    /// Writes the indent of `line`, which starts the line being written, as
    /// [`Placing`] and the recursive loops being run say, in place of any
    /// spaces already on it. `typed` is the indent as the template has it,
    /// where it is still to write. Whether anything was written.
    fn land(&mut self, placing: Placing<'_>, line: &Landing, typed: Option<&[u8]>) -> bool {
        let written = placing.written(line);
        let deeper: String = self.deeper.iter().map(|call| &*call.by).collect();
        let dangling =
            LINES.with_borrow(|lines| lines.last().is_some_and(|line| line.start < self.out.len()));
        if written.is_none() && deeper.is_empty() && !dangling {
            return false;
        }
        self.clear();
        self.made(&deeper);
        match (written, typed) {
            (Some(written), _) => self.made(written),
            (None, Some(typed)) => self.put(typed),
            (None, None) => {}
        }
        true
    }

    /// Whether nothing but spaces is written on the line being written.
    fn open(&self) -> bool {
        LINES.with_borrow(|lines| lines.last().is_some_and(|line| line.open))
    }

    /// Writes `text`, which ends a line of the template if it ends with a
    /// line ending, unless the line came to nothing.
    fn ending(&mut self, placing: Placing<'_>, text: &[u8]) {
        let nothing = text.ends_with(b"\n")
            && text.iter().all(u8::is_ascii_whitespace)
            && self.open()
            && placing
                .holding(text.as_ptr() as usize + text.len() - 1)
                .is_some_and(|line| line.holds);
        if nothing {
            self.clear();
        } else {
            self.put(text);
        }
    }

    /// Takes back the spaces written on the line being written.
    fn clear(&mut self) {
        LINES.with_borrow_mut(|lines| {
            let line = lines.last_mut().expect("a sink has its line");
            self.out.truncate(line.start);
            self.own.truncate(line.start);
            line.indent.clear();
            line.open = true;
            line.emptied = false;
            line.ended = false;
            line.placed = false;
        });
    }

    /// Writes an indent that is none of the template's text.
    fn made(&mut self, indent: &str) {
        self.out.extend_from_slice(indent.as_bytes());
        LINES.with_borrow_mut(|lines| {
            let line = lines.last_mut().expect("a sink has its line");
            line.indent.push_str(indent);
        });
    }

    /// Notes a recursive loop's call starting to run, from what was written
    /// before it and what is written now, or the call ending, from the
    /// template's own text written after it. `own` is where in memory the
    /// text now written is, where it is the template's, and `last` where its
    /// own text written before ends.
    fn track(&mut self, placing: Placing<'_>, own: Option<usize>, last: Option<usize>) {
        if let Some(own) = own
            && self.deeper.last().is_some_and(|call| call.end == own)
        {
            self.deeper.pop();
            return;
        }
        if let Some(call) = last.and_then(|last| placing.call(last))
            && own != Some(call.end)
        {
            self.deeper.push(call.clone());
        }
    }
}

impl Drop for Sink {
    fn drop(&mut self) {
        LINES.with_borrow_mut(Vec::pop);
    }
}

impl io::Write for Sink {
    fn write(&mut self, text: &[u8]) -> io::Result<usize> {
        Sink::write(self, text);
        Ok(text.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// A body the engine holds back, such as a macro's or a `{% set %}`
/// block's, written as the template rendering it writes its own lines.
struct Body(Sink);

impl fmt::Write for Body {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.0.write(text.as_bytes());
        Ok(())
    }
}

impl Held for Body {
    fn finish(mut self: Box<Self>, safe: bool) -> Value {
        let out = std::mem::take(&mut self.0.layout.out);
        let text = String::from_utf8(out).expect("templates write text");
        super::frame::made(text, std::mem::take(&mut self.0.layout.own), safe)
    }
}

impl Layout {
    /// Writes `text`, a value's or the template's own, leaving out of the
    /// template's own what [`Placing`] says the page leaves out.
    fn write(&mut self, placing: Placing<'_>, text: &[u8]) {
        let value = LINES.with_borrow(|lines| lines.last().is_some_and(|line| line.value));
        if value {
            self.run(placing, text, true);
            return;
        }
        let from = text.as_ptr() as usize;
        let mut done = 0;
        for gap in placing.dropped(from..from + text.len()) {
            let start = gap.start.max(from) - from;
            if start > done {
                self.run(placing, &text[done..start], false);
            }
            done = done.max(gap.end.min(from + text.len()) - from);
        }
        self.run(placing, &text[done..], false);
    }

    /// Writes `text`, a value's or a run of the template's own text.
    fn run(&mut self, placing: Placing<'_>, text: &[u8], value: bool) {
        if text.is_empty() {
            return;
        }
        let from = text.as_ptr() as usize;
        let last = self.last.take();
        self.track(placing, (!value).then_some(from), last);
        if value || placing.is_empty() {
            self.put(text);
        } else {
            let len = self.out.len();
            self.own(placing, text);
            let end = from + text.len();
            // Text reaching past the indent of a line whose line ending is
            // owed is what the line holds being written.
            if let Some(line) = placing.holding(end - 1)
                && line.owed().is_some_and(|ending| end <= ending)
                && end > line.at + line.indent
            {
                self.began = Some(len);
            }
            // A value opening a line of the template with no indent before
            // it is placed by that line, before it is written, so that each
            // of its lines takes the indent.
            if let Some(line) = placing.valued(end)
                && line.indent == 0
                && line.movable
                && (self.begins || !self.out.is_empty())
                && self.open()
            {
                self.land(placing, line, None);
            }
            if let Some(line) = placing.holding(end)
                && line.owed().is_some()
                && line.at + line.indent == end
            {
                self.began = Some(self.out.len());
            }
            // A body held back, called from a line indented as typed.
            let placed = placing.holding(end).is_some_and(|line| {
                line.held && line.movable && line.indent > 0 && line.at + line.indent == end
            });
            if placed && self.open() {
                LINES.with_borrow_mut(|lines| {
                    if let Some(line) = lines.last_mut() {
                        line.placed = true;
                    }
                });
            }
        }
        if !value {
            self.last = Some(from + text.len());
        }
    }

    fn put(&mut self, mut text: &[u8]) {
        LINES.with_borrow_mut(|lines| {
            let line = lines.last_mut().expect("a sink has its line");
            line.placed &= text.is_empty();
            // A line ending a value writes is the value's to keep: only
            // the template's own takes the place of one already written,
            // or goes with a line left empty.
            if line.value {
                line.ended = false;
                line.emptied &= text.is_empty();
            }
            if std::mem::take(&mut line.ended) {
                text = match text {
                    [b'\r', b'\n', rest @ ..] | [b'\n', rest @ ..] => rest,
                    text => text,
                };
            }
            if line.emptied {
                let rest = &text[blank(text)..];
                let ending = match rest {
                    [b'\r', b'\n', ..] => 2,
                    [b'\n', ..] => 1,
                    _ => 0,
                };
                if ending > 0 {
                    self.out.truncate(line.start);
                    self.own.truncate(line.start);
                    line.indent.clear();
                    line.open = true;
                    line.emptied = false;
                    text = &rest[ending..];
                } else if !rest.is_empty() {
                    line.emptied = false;
                }
            }
            if !text.is_empty() {
                let from = text.as_ptr() as usize;
                match (line.value, &line.printing) {
                    (false, _) => self.own.runs.push(Run {
                        at: self.out.len(),
                        from,
                        len: text.len(),
                    }),
                    (true, Some(printing)) => {
                        let (start, len) = printing.text;
                        let inside = from >= start && from + text.len() <= start + len;
                        self.own.prints.push(Print {
                            at: self.out.len(),
                            len: text.len(),
                            within: inside.then(|| from - start),
                            from: printing.from.clone(),
                        });
                    }
                    (true, None) => {}
                }
            }
            self.out.extend_from_slice(text);
            let tail = match text.iter().rposition(|&byte| byte == b'\n') {
                Some(at) => {
                    let tail = &text[at + 1..];
                    line.start = self.out.len() - tail.len();
                    line.indent.clear();
                    line.open = true;
                    line.emptied = false;
                    tail
                }
                None => text,
            };
            if line.open {
                let blank = blank(tail);
                line.indent
                    .extend(tail[..blank].iter().map(|&byte| char::from(byte)));
                line.open = blank == tail.len();
            }
        });
    }
}

/// How many spaces and tabs `text` opens with.
fn blank(text: &[u8]) -> usize {
    text.iter()
        .take_while(|byte| matches!(byte, b' ' | b'\t'))
        .count()
}

/// Renders `template`, giving what it wrote beside the engine's own
/// account. `begins` says whether what it writes starts a line where it is
/// put, so that its first line may be written as [`Placing`] says.
pub(super) fn render<'source>(
    template: &Template<'_, 'source>,
    context: impl Into<Value>,
    place: Place,
    begins: bool,
) -> Result<(Captured<'source>, String), Error> {
    render_placed(template, context, place, begins).map(|(captured, text, _)| (captured, text))
}

/// Renders `template` as [`render`] does, giving as well where what it
/// wrote is its own text.
pub(super) fn render_placed<'source>(
    template: &Template<'_, 'source>,
    context: impl Into<Value>,
    place: Place,
    begins: bool,
) -> Result<(Captured<'source>, String, Own), Error> {
    let _rendering = Rendering::new(place.clone());
    let mut sink = Sink::new(Some(place), begins);
    let captured = template.render_captured_to(context, &mut sink)?;
    let layout = &mut sink.layout;
    let text = String::from_utf8(std::mem::take(&mut layout.out)).expect("templates write text");
    Ok((captured, text, std::mem::take(&mut layout.own)))
}

/// A template being rendered, whose place the bodies the engine holds back
/// as it renders take.
struct Rendering;

impl Rendering {
    fn new(place: Place) -> Self {
        PLACES.with_borrow_mut(|places| places.push(place));
        Self
    }
}

impl Drop for Rendering {
    fn drop(&mut self) {
        PLACES.with_borrow_mut(Vec::pop);
    }
}

/// The writer the engine writes a body it holds back to, laid out as the
/// template being rendered lays out its lines. Where the body lands is the
/// line writing it, so a block written in another template's place moves
/// that line and not the body's own.
pub(super) fn held() -> Box<dyn Held> {
    let place = PLACES.with_borrow(|places| places.last().map(Place::unchained));
    Box::new(Body(Sink::new(place, true)))
}

/// Whether nothing but spaces has been written on the line being written,
/// by the template being rendered or a body the engine holds back.
pub(super) fn at_line_start() -> bool {
    LINES.with_borrow(|lines| lines.last().is_some_and(|line| line.open))
}

/// Runs `write`, which writes a value, of `text` where it is text, written
/// at `printed`, so that the line endings it writes are known for the
/// value's.
pub(super) fn value<T>(
    printed: Option<Printed>,
    text: Option<&str>,
    write: impl FnOnce() -> T,
) -> T {
    let printing = printed.map(|from| Printing {
        from,
        text: text.map_or((0, 0), |text| (text.as_ptr() as usize, text.len())),
    });
    let set = |value, printing| {
        LINES.with_borrow_mut(|lines| {
            if let Some(line) = lines.last_mut() {
                line.value = value;
                line.printing = printing;
            }
        });
    };
    set(true, printing);
    let written = write();
    set(false, None);
    written
}

/// Notes that a value which came to nothing was put on the line, so the
/// line is not written if nothing else is put on it.
pub(super) fn nothing() {
    LINES.with_borrow_mut(|lines| {
        if let Some(line) = lines.last_mut()
            && line.open
        {
            line.emptied = true;
        }
    });
}

/// Notes that the value just written closed with a line ending of its own,
/// so the one the template follows it with is not written as well: the two
/// would be a blank line neither file has.
pub(super) fn ended() {
    LINES.with_borrow_mut(|lines| {
        if let Some(line) = lines.last_mut()
            && line.open
            && line.indent.is_empty()
        {
            line.ended = true;
        }
    });
}

/// Writes markup of several lines with each line after its first indented
/// as the line it is put on, where it is the first thing on that line. A
/// body the engine held back gives up the indent its lines share, which
/// they have only for the template's sake, as a macro's body indented
/// under its tag. `None` where there is nothing to change and the markup is
/// still to write.
pub(super) fn write(out: &mut Output, markup: &str) -> Option<Result<(), Error>> {
    let (indent, placed) = LINES.with_borrow(|lines| {
        lines
            .last()
            .filter(|line| line.open && !line.indent.is_empty())
            .map(|line| (line.indent.clone(), line.placed))
    })?;
    if !markup.contains('\n') {
        return None;
    }
    let shared = if placed { shared(markup) } else { "" };
    // Written a piece at a time, so each line is written from the
    // markup's own text and found there.
    Some(
        indent_into(markup, shared, &indent, |piece| out.write_str(piece))
            .map_err(|_| Error::from(minijinja::ErrorKind::WriteFailure)),
    )
}

/// The spaces and tabs that every line of `markup` holding more opens
/// with, of the lines [`indented`] would indent.
fn shared(markup: &str) -> &str {
    let kept = kept_as_written(markup);
    let mut shared: Option<&str> = None;
    for (number, line) in markup.split_inclusive('\n').enumerate() {
        if !kept[number] && !line.trim().is_empty() {
            let opening = &line[..blank(line.as_bytes())];
            let len = shared.map_or(opening.len(), |shared| {
                shared
                    .bytes()
                    .zip(opening.bytes())
                    .take_while(|(a, b)| a == b)
                    .count()
            });
            if len == 0 {
                return "";
            }
            shared = Some(&opening[..len]);
        }
    }
    shared.unwrap_or("")
}

/// What [`indent_into`] puts, as one text.
#[cfg(test)]
fn indented(markup: &str, shared: &str, indent: &str) -> String {
    let mut out = String::with_capacity(markup.len() + markup.len() / 8);
    let _ = indent_into(markup, shared, indent, |piece| {
        out.push_str(piece);
        Ok(())
    });
    out
}

/// Puts `markup` with `indent` before each line after its first, in place
/// of `shared` before each line, a piece at a time, each piece the indent
/// or part of `markup` itself. An empty line stays empty, and so does a
/// line the spaces would change: one inside `<pre>` or `<textarea>`, where
/// they would show, or inside an attribute's value.
fn indent_into(
    markup: &str,
    shared: &str,
    indent: &str,
    mut put: impl FnMut(&str) -> fmt::Result,
) -> fmt::Result {
    let kept = kept_as_written(markup);
    for (number, line) in markup.split_inclusive('\n').enumerate() {
        let takes = !kept[number];
        let rest = match line.strip_prefix(shared) {
            Some(rest) if takes => rest,
            None if takes && line.trim().is_empty() => line.trim_start_matches([' ', '\t']),
            _ => line,
        };
        if number > 0 && takes && !matches!(rest, "\n" | "\r\n") {
            put(indent)?;
        }
        put(rest)?;
    }
    Ok(())
}

/// For each line of `markup`, whether spaces before it would change it:
/// it starts inside `<pre>`, `<textarea>` or the like, where they would
/// show, or inside an attribute's value. Markup escaped whole, as a feed
/// carries a post, is read as the markup it shows; in markup as written,
/// an escaped tag is words about one, such as `<pre>` named in a sentence.
fn kept_as_written(markup: &str) -> Vec<bool> {
    let unescaped;
    let text = if markup.contains('<') {
        markup
    } else {
        // As long as what it stands for, so every line starts where it did.
        unescaped = markup.replace("&lt;", "   <").replace("&gt;", ">   ");
        &unescaped
    };
    // Where a line starting is kept as written.
    let mut kept: Vec<Range<usize>> = Vec::new();
    // The element whose spaces show being read, and where its name starts.
    let mut shown: Option<(String, usize)> = None;
    for (range, piece) in crate::html::markup_where_closed(text) {
        match piece {
            Markup::Start(tag) => {
                for attribute in &tag.attributes {
                    if text[attribute.at.clone()].contains('\n') {
                        kept.push(attribute.at.start + 1..attribute.at.end + 1);
                    }
                }
                if shown.is_none() && PREFORMATTED.contains(&tag.name.as_str()) {
                    shown = Some((tag.name, range.start + 1));
                }
            }
            Markup::End(tag) => {
                if let Some((name, from)) = &shown
                    && *name == tag.name
                {
                    kept.push(*from..range.start + 1);
                    shown = None;
                }
            }
            Markup::Other => {}
        }
    }
    if let Some((_, from)) = shown {
        kept.push(from..usize::MAX);
    }
    std::iter::once(0)
        .chain(text.match_indices('\n').map(|(at, _)| at + 1))
        .map(|start| kept.iter().any(|range| range.contains(&start)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Origin, Own, Run, indented, shared};
    use crate::source::{Place, Sources, Span};

    #[test]
    fn each_byte_of_the_writing_is_the_innermost_parts_known_within_it_where_spelt_alike() {
        let mut sources = Sources::default();
        let text = "See [a](/a/) and <b>this</b>.";
        let source = sources.add("content/post-1.md", text.to_owned());
        let at = |start, end| Place {
            source,
            span: Span::new(start, end),
        };
        let html = "<p>See <a href=\"/a/\">a</a> and <b>this</b>.</p>";
        let link = html.find("<a").unwrap()..html.find(" and").unwrap();
        let tag = html.find("<b>").unwrap()..html.find("<b>").unwrap() + 3;
        let traced = [
            (link.clone(), at(4, 12)),
            (tag.clone(), at(17, 20)),
            (0..html.len(), at(0, text.len())),
        ];
        let own = Own::of_writing(&traced, &[], &[], html, &sources);
        assert_eq!(own.origin(0), Some(Origin::Text(at(0, text.len()), None)));
        assert_eq!(
            own.origin(link.start + 9),
            Some(Origin::Text(at(4, 12), None))
        );
        assert_eq!(
            own.origin(link.end),
            Some(Origin::Text(at(0, text.len()), None))
        );
        // The tag is written as typed, so each byte is known in it.
        assert_eq!(
            own.origin(tag.start + 1),
            Some(Origin::Text(at(17, 20), Some(1)))
        );
        assert_eq!(
            own.origin(html.len() - 1),
            Some(Origin::Text(at(0, text.len()), None))
        );
        assert_eq!(own.origin(html.len()), None);
    }

    #[test]
    fn what_is_copied_or_respelt_from_the_writing_is_known_where_the_writing_is() {
        let mut sources = Sources::default();
        let html = "<b>a</b> <i>b</i>";
        let source = sources.add("content/post-1.md", html.to_owned());
        let at = |start, end| Place {
            source,
            span: Span::new(start, end),
        };
        let traced = [(0..8, at(0, 8)), (9..17, at(9, 17)), (0..17, at(0, 17))];
        let own = Own::of_writing(&traced, &[], &[], html, &sources);
        let text = |place, within| Some(Origin::Text(place, within));
        // `a` spelt `xyz`: byte for byte as far as both go, then where the
        // last byte replaced was.
        let respelt = own.respelt(&vec![(3..4, 3..6)], html.len());
        assert_eq!(respelt.origin(2), text(at(0, 8), Some(2)));
        assert_eq!(respelt.origin(3), text(at(0, 8), Some(3)));
        assert_eq!(respelt.origin(5), text(at(0, 8), None));
        assert_eq!(respelt.origin(6), text(at(0, 8), Some(4)));
        assert_eq!(respelt.origin(10), text(at(0, 17), Some(8)));
        assert_eq!(respelt.origin(14), text(at(9, 17), Some(3)));
        assert_eq!(respelt.origin(19), None);
        let copied = own.copied(&[(0..3, 9), (3..4, 12)]);
        assert_eq!(copied.origin(0), text(at(9, 17), Some(0)));
        assert_eq!(copied.origin(3), text(at(9, 17), Some(3)));
        assert_eq!(copied.origin(4), None);
    }

    #[test]
    fn a_templates_own_text_is_found_where_it_was_written_from() {
        let run = |at, from, len| Run { at, from, len };
        let mut own = Own {
            runs: vec![run(0, 100, 4), run(6, 110, 3), run(9, 200, 5)],
            prints: Vec::new(),
        };
        assert_eq!(own.written_from(0), Some(100));
        assert_eq!(own.written_from(3), Some(103));
        // Between two runs a value was written.
        assert_eq!(own.written_from(4), None);
        assert_eq!(own.written_from(8), Some(112));
        assert_eq!(own.written_from(9), Some(200));
        assert_eq!(own.written_from(14), None);
        // A line taken back takes what was written on it.
        own.truncate(11);
        assert_eq!(own.written_from(10), Some(201));
        assert_eq!(own.written_from(11), None);
        own.truncate(5);
        assert_eq!(own.written_from(8), None);
        assert_eq!(own.written_from(3), Some(103));
    }

    #[test]
    fn each_further_line_takes_the_indent() {
        assert_eq!(
            indented("<ul>\n  <li>one</li>\n</ul>\n", "", "    "),
            "<ul>\n      <li>one</li>\n    </ul>\n"
        );
    }

    #[test]
    fn the_indent_every_line_shares_gives_way_to_the_lines() {
        let markup = "    <ul>\n      <li>one</li>\n\n  \n    </ul>\n";
        assert_eq!(shared(markup), "    ");
        assert_eq!(
            indented(markup, shared(markup), "  "),
            "<ul>\n    <li>one</li>\n\n\n  </ul>\n"
        );
        // Lines whose spaces would show neither count nor move.
        let markup = "  <pre>\none\n  two</pre>\n  <p>a</p>\n";
        assert_eq!(shared(markup), "  ");
        assert_eq!(
            indented(markup, shared(markup), "\t"),
            "<pre>\none\n  two</pre>\n\t<p>a</p>\n"
        );
        assert_eq!(shared("    <p>a</p>\n  <p>b</p>\n"), "  ");
        assert_eq!(shared("  <p>a</p>\n\t<p>b</p>\n"), "");
        assert_eq!(shared("<p>a</p>\n  <p>b</p>\n"), "");
    }

    #[test]
    fn an_empty_line_stays_empty() {
        assert_eq!(
            indented("<p>a</p>\n\n<p>b</p>", "", "\t"),
            "<p>a</p>\n\n\t<p>b</p>"
        );
        assert_eq!(indented("a\r\n\r\nb", "", "  "), "a\r\n\r\n  b");
    }

    #[test]
    fn lines_shown_as_written_are_left() {
        assert_eq!(
            indented(
                "<p>a</p>\n<pre><code>one\n  two</code></pre>\n<p>b</p>",
                "",
                "  "
            ),
            "<p>a</p>\n  <pre><code>one\n  two</code></pre>\n  <p>b</p>"
        );
        assert_eq!(
            indented("<TextArea>\none\n</TEXTAREA>\n<p>b</p>", "", "  "),
            "<TextArea>\none\n</TEXTAREA>\n  <p>b</p>"
        );
        assert_eq!(indented("<pretty>\none", "", "  "), "<pretty>\n  one");
    }

    #[test]
    fn an_attributes_value_is_left_and_its_tags_other_lines_are_not() {
        assert_eq!(
            indented(
                "<img alt=\"one\ntwo\"\n  src=\"a.png\">\n<p>b</p>",
                "",
                "  "
            ),
            "<img alt=\"one\ntwo\"\n    src=\"a.png\">\n  <p>b</p>"
        );
    }

    #[test]
    fn scripts_styles_and_comments_are_read_through() {
        assert_eq!(
            indented(
                "<script>\nif (a < b) {\n  c = \"\n}\n</script>\n<p>b</p>",
                "",
                "  "
            ),
            "<script>\n  if (a < b) {\n    c = \"\n  }\n  </script>\n  <p>b</p>"
        );
        assert_eq!(
            indented("<!-- it's\n<pre>\n-->\n<p>b</p>", "", "  "),
            "<!-- it's\n  <pre>\n  -->\n  <p>b</p>"
        );
        assert_eq!(indented("a < b and \"c\nd", "", "  "), "a < b and \"c\n  d");
    }

    #[test]
    fn a_tag_named_in_the_words_opens_nothing() {
        assert_eq!(
            indented(
                "<p>Inside <code>&lt;pre&gt;</code>, nothing is.</p>\n<p>b</p>\n<p>c</p>",
                "",
                "  "
            ),
            "<p>Inside <code>&lt;pre&gt;</code>, nothing is.</p>\n  <p>b</p>\n  <p>c</p>"
        );
    }

    #[test]
    fn escaped_markup_keeps_what_it_shows() {
        assert_eq!(
            indented(
                "&lt;p&gt;a&lt;/p&gt;\n&lt;pre&gt;one\n  two&lt;/pre&gt;\n&lt;p&gt;b",
                "",
                "  "
            ),
            "&lt;p&gt;a&lt;/p&gt;\n  &lt;pre&gt;one\n  two&lt;/pre&gt;\n  &lt;p&gt;b"
        );
    }

    #[test]
    fn the_markup_is_read_as_a_browser_reads_it() {
        // A comment closed by `--!>` hides nothing after it.
        assert_eq!(
            indented("<!-- a --!>\n<pre>\nx\n</pre>\n<p>y</p>", "", "  "),
            "<!-- a --!>\n  <pre>\nx\n</pre>\n  <p>y</p>"
        );
        // An apostrophe in an attribute's value written without quotes
        // opens no quotation.
        assert_eq!(
            indented("<p><a title=it's>a</a></p>\n<p>b</p>\n<p>c</p>", "", "  "),
            "<p><a title=it's>a</a></p>\n  <p>b</p>\n  <p>c</p>"
        );
        // An `<iframe>` named in the words and never closed is taken to
        // hold nothing, where a browser would read the rest as its text.
        assert_eq!(
            indented(
                "<code><iframe></code>\n<pre>\nx\n</pre>\n<p>y</p>",
                "",
                "  "
            ),
            "<code><iframe></code>\n  <pre>\nx\n</pre>\n  <p>y</p>"
        );
    }
}
