//! Where each line of the templates is written in the built page.
//!
//! A line is written at its own indent, less what `dedent` says it has
//! only for the template's sake. A block's body written in another
//! template's place lines up where it lands: its lines give up the indent
//! they have in common and take the place's. The place is the line its
//! tags stand on, where they share it with nothing else, or where the body
//! it replaces has its lines, where its tags stand alone round that body.
//! Anywhere else the body is written as typed. A recursive loop's deeper
//! levels go in as far as its call stands inside its `for` tag.

use std::collections::HashMap;
use std::ops::Range;

use super::dedent::{Facts, Site};

/// Every line of the templates, and the overrides for each template whose
/// chain writes a block's body in another's place.
#[derive(Default)]
pub(super) struct Landings {
    lines: Vec<Landing>,
    calls: Vec<Deeper>,
    chains: HashMap<usize, Vec<(usize, Box<str>)>>,
}

/// A line of a template, by where in memory it starts.
pub(super) struct Landing {
    pub(super) at: usize,
    /// Its bytes, line ending and all.
    len: usize,
    /// The bytes of its indent.
    pub(super) indent: usize,
    /// What its indent is written as, where not as typed.
    written: Option<Box<str>>,
    /// It holds more than spaces.
    pub(super) holds: bool,
    /// Its indent may be written otherwise.
    pub(super) movable: bool,
    /// It holds tags and nothing else.
    pub(super) bare: bool,
    /// Its indent is followed by a value.
    pub(super) valued: bool,
    /// Its indent is one the engine is told to keep, before a tag.
    pub(super) kept: bool,
    /// Where in memory its line ending is, where the engine is told to
    /// keep it, after a tag.
    pub(super) ending: Option<usize>,
}

/// A recursive loop's call on a line of its own.
pub(super) struct Deeper {
    /// Where in memory the call starts and ends.
    pub(super) at: usize,
    pub(super) end: usize,
    /// What each level it runs is indented by beyond the level that ran it.
    pub(super) by: Box<str>,
}

/// A template the engine holds, with what is known of its lines.
pub(super) struct Held<'a> {
    /// Where in memory the engine holds its text.
    pub(super) base: usize,
    /// Its text as read, as long as the engine's and with the same indents.
    pub(super) source: &'a str,
    pub(super) facts: &'a Facts,
}

impl Held<'_> {
    /// The indent line `number` is written at in its own template.
    fn target(&self, number: usize) -> &str {
        let line = &self.facts.lines[number];
        &self.source[line.start + line.cut..line.start + line.indent]
    }
}

impl Landings {
    /// Adds the lines of a template. Only where `markup` do they move: in
    /// any other kind of file an indent may mean something.
    pub(super) fn add(&mut self, held: &Held<'_>, markup: bool) {
        let base = held.base;
        let lines = &held.facts.lines;
        for (number, line) in lines.iter().enumerate() {
            let next = lines
                .get(number + 1)
                .map_or(held.source.len(), |next| next.start);
            self.lines.push(Landing {
                at: base + line.start,
                len: next - line.start,
                indent: line.indent,
                written: (markup && line.cut > 0).then(|| held.target(number).into()),
                holds: line.holds,
                movable: line.movable,
                bare: line.bare,
                valued: line.valued,
                kept: line.kept,
                ending: line.ending.map(|ending| base + ending),
            });
        }
        if markup {
            for call in &held.facts.calls {
                let outer = held.target(call.opening);
                let inner = held.target(call.line);
                if let Some(by) = inner.strip_prefix(outer).filter(|by| !by.is_empty()) {
                    self.calls.push(Deeper {
                        at: base + call.span.start,
                        end: base + call.span.end,
                        by: by.into(),
                    });
                }
            }
        }
    }

    /// Puts what has been added in order.
    pub(super) fn sort(&mut self) {
        self.lines.sort_unstable_by_key(|line| line.at);
        self.calls.sort_unstable_by_key(|call| call.at);
    }

    /// Notes where the lines of `chain`, a template and those it extends in
    /// turn, are written when the first is rendered.
    pub(super) fn chain(&mut self, first: usize, chain: &[Held<'_>]) {
        if chain.len() < 2 {
            return;
        }
        let mut walk = Walk {
            chain,
            moved: Vec::new(),
        };
        let root = chain.len() - 1;
        walk.walk(root, 0..chain[root].facts.lines.len(), None, None);
        let mut moved = walk.moved;
        moved.sort_by_key(|(at, _)| *at);
        moved.dedup_by_key(|(at, _)| *at);
        if !moved.is_empty() {
            self.chains.insert(first, moved);
        }
    }

    /// Where the template `file` writes its lines when rendered.
    pub(super) fn placing(&self, file: Option<usize>) -> Placing<'_> {
        Placing {
            landings: self,
            chain: file
                .and_then(|file| self.chains.get(&file))
                .map_or(&[], Vec::as_slice),
        }
    }
}

/// Where the lines of one render are written.
#[derive(Clone, Copy)]
pub(super) struct Placing<'a> {
    landings: &'a Landings,
    chain: &'a [(usize, Box<str>)],
}

impl<'a> Placing<'a> {
    pub(super) fn is_empty(&self) -> bool {
        self.landings.lines.is_empty()
    }

    /// The lines starting in memory within `range`, in order.
    pub(super) fn starting(&self, range: Range<usize>) -> &'a [Landing] {
        let lines = &self.landings.lines;
        let first = lines.partition_point(|line| line.at < range.start);
        let after = lines.partition_point(|line| line.at < range.end);
        &lines[first..after]
    }

    /// The line a value written next stands on, the template's own text
    /// having been written up to `at` in memory: the first line from there
    /// that holds more than tags, where it opens with a value.
    pub(super) fn valued(&self, at: usize) -> Option<&'a Landing> {
        let lines = &self.landings.lines;
        let first = lines.partition_point(|line| line.at < at);
        if lines.get(first)?.at != at {
            return None;
        }
        let mut end = at;
        for line in &lines[first..] {
            // The next template's lines are not this one's.
            if line.at != end {
                return None;
            }
            if !line.bare {
                return line.valued.then_some(line);
            }
            end = line.at + line.len;
        }
        None
    }

    /// The line holding the byte at `at` in memory.
    pub(super) fn holding(&self, at: usize) -> Option<&'a Landing> {
        let lines = &self.landings.lines;
        let after = lines.partition_point(|line| line.at <= at);
        let line = lines[..after].last()?;
        (at < line.at + line.len).then_some(line)
    }

    /// What `line`'s indent is written as, where not as typed.
    pub(super) fn written(&self, line: &'a Landing) -> Option<&'a str> {
        match self.chain.binary_search_by_key(&line.at, |(at, _)| *at) {
            Ok(found) => Some(&self.chain[found].1),
            Err(_) => line.written.as_deref(),
        }
    }

    /// The recursive loop's call starting at `at` in memory.
    pub(super) fn call(&self, at: usize) -> Option<&'a Deeper> {
        let calls = &self.landings.calls;
        calls
            .binary_search_by_key(&at, |call| call.at)
            .ok()
            .map(|found| &calls[found])
    }
}

/// How a body's lines are moved where it lands: the indent they have in
/// common, and what is written for it.
type Moved = Option<(String, String)>;

struct Walk<'c, 'a> {
    chain: &'c [Held<'a>],
    moved: Vec<(usize, Box<str>)>,
}

impl Walk<'_, '_> {
    /// Notes where the lines `lines` of `chain[at]` are written, moved by
    /// `moved`, and the bodies of the blocks among them. `block` names the
    /// block whose body they are.
    fn walk(&mut self, at: usize, lines: Range<usize>, moved: Moved, block: Option<&str>) {
        let held = &self.chain[at];
        let facts = held.facts;
        let mut inner = Vec::new();
        let mut covered = lines.start;
        for named in &facts.blocks {
            if named.opening >= covered && named.opening < lines.end && named.end < lines.end {
                inner.push(named);
                covered = named.end;
            }
        }
        let outside = |number: usize| {
            !inner
                .iter()
                .any(|named| named.opening < number && number < named.end)
        };
        if let Some(moved) = &moved {
            for number in lines.clone().filter(|&number| outside(number)) {
                if facts.lines[number].movable {
                    let written = move_by(moved, held.target(number));
                    self.moved
                        .push((held.base + facts.lines[number].start, written.into()));
                }
            }
        }
        for &named in &inner {
            let body = named.opening + 1..named.end;
            let replacing = (0..=at)
                .find(|&other| {
                    self.chain[other]
                        .facts
                        .blocks
                        .iter()
                        .any(|theirs| theirs.name == named.name)
                })
                .unwrap_or(at);
            if replacing == at {
                self.walk(at, body, moved.clone(), Some(&named.name));
                continue;
            }
            let place = self.place(at, named.site, &body, &moved);
            self.land(replacing, &named.name, place);
        }
        let Some(name) = block else {
            return;
        };
        for &number in &facts.supers {
            if lines.contains(&number) && outside(number) {
                let place = Some(written(&moved, held.target(number)));
                if let Some(parent) = (at + 1..self.chain.len()).find(|&other| {
                    self.chain[other]
                        .facts
                        .blocks
                        .iter()
                        .any(|theirs| theirs.name == name)
                }) {
                    self.land(parent, name, place);
                }
            }
        }
    }

    /// Notes where the body of the block `name` in `chain[at]` is written,
    /// landing at `place`, or as typed where there is none.
    fn land(&mut self, at: usize, name: &str, place: Option<String>) {
        let held = &self.chain[at];
        let Some(theirs) = held.facts.blocks.iter().find(|theirs| theirs.name == name) else {
            return;
        };
        let mut body = theirs.opening + 1..theirs.end;
        // A body may run up to its end tag on the end tag's line.
        if theirs.end > theirs.opening && held.facts.lines[theirs.end].content {
            body.end += 1;
        }
        let moved = place.map(|place| (common(held, body.clone(), &None), place));
        self.walk(at, body, moved, Some(name));
    }

    /// Where a body written in place of the block standing at `site` in
    /// `chain[at]`, round `body`, lines up.
    fn place(&self, at: usize, site: Site, body: &Range<usize>, moved: &Moved) -> Option<String> {
        let held = &self.chain[at];
        match site {
            Site::Line => Some(written(moved, held.target(body.start - 1))),
            Site::Body => {
                let found = held.facts.lines[body.clone()]
                    .iter()
                    .any(|line| line.content);
                found.then(|| common(held, body.clone(), moved))
            }
            Site::Among => None,
        }
    }
}

/// The indent the lines of `lines` writing markup have in common, as
/// `moved` writes them.
fn common(held: &Held<'_>, lines: Range<usize>, moved: &Moved) -> String {
    let mut common: Option<String> = None;
    for number in lines.filter(|&number| held.facts.lines[number].content) {
        let target = written(moved, held.target(number));
        common = Some(match common {
            None => target,
            Some(common) => {
                let same = common
                    .bytes()
                    .zip(target.bytes())
                    .take_while(|(a, b)| a == b)
                    .count();
                common[..same].to_owned()
            }
        });
    }
    common.unwrap_or_default()
}

fn written(moved: &Moved, target: &str) -> String {
    match moved {
        Some(moved) => move_by(moved, target),
        None => target.to_owned(),
    }
}

/// `target` with the indent in common given up for the place's. A line
/// without it, such as an empty one, is written as typed.
fn move_by((common, place): &(String, String), target: &str) -> String {
    match target.strip_prefix(common.as_str()) {
        Some(rest) => format!("{place}{rest}"),
        None => target.to_owned(),
    }
}
