//! `statac search`: what a reader of the site would find for some words.

use std::collections::HashSet;
use std::path::PathBuf;

use super::{Index, Record, asked, fold, records, spans};
use crate::date::Date;
use crate::problem::{Problem, Severity};
use crate::site::PageId;
use crate::source::Sources;
use crate::templates::Renderer;

/// How many pages are shown, best first.
pub const SHOWN: usize = 10;

/// About how many characters of a record's text are shown around the
/// first word matched.
const BEFORE: usize = 60;
const AFTER: usize = 140;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Options {
    pub site: PathBuf,
    pub words: String,
    pub today: Option<Date>,
    /// Whether drafts and posts waiting for their day are searched too.
    pub drafts: bool,
}

/// One page found, at the part of it that matched best.
#[derive(Debug, PartialEq)]
pub struct Shown {
    pub title: Option<String>,
    pub heading: Option<String>,
    /// From the top of the site, with the heading's id where there is one.
    pub address: String,
    /// The text around the first word matched, in pieces, each saying
    /// whether it is a word matched.
    pub excerpt: Vec<(String, bool)>,
}

pub struct Outcome {
    renderer: Option<Renderer>,
    /// The errors that stopped the site being read.
    pub errors: Vec<Problem>,
    /// The pages found, best first, at most [`SHOWN`].
    pub shown: Vec<Shown>,
    /// How many pages were found in all.
    pub pages: usize,
    /// Whether no page held every word, and those found hold some.
    pub some: bool,
}

impl Outcome {
    pub fn sources(&self) -> Option<&Sources> {
        self.renderer.as_ref().map(Renderer::sources)
    }
}

pub fn run(options: &Options) -> Outcome {
    let run = crate::content::Run {
        today: options.today,
        drafts: options.drafts,
        url: None,
    };
    let prepared = crate::build::prepared(&options.site, &run);
    let errors: Vec<Problem> = (prepared.problems.into_iter())
        .filter(|problem| problem.kind.severity() == Severity::Error)
        .collect();
    if !errors.is_empty() {
        return Outcome {
            renderer: Some(prepared.renderer),
            errors,
            shown: Vec::new(),
            pages: 0,
            some: false,
        };
    }
    let renderer = prepared.renderer;
    let index = Index::of(renderer.site(), records(&renderer));
    let found = index.search(&options.words);
    let asked = asked(&options.words);
    // Records holding every word, or where none does, those holding most.
    let most = found.first().map_or(0, |found| found.matched);
    let some = most < asked.len();
    let mut seen: HashSet<PageId> = HashSet::new();
    let best = (found.iter())
        .take_while(|found| found.matched == most)
        .map(|found| found.record)
        .filter(|&record| seen.insert(index.records[record].page));
    let best: Vec<usize> = best.collect();
    // A word of the text is marked where it shares a stem with one asked.
    let matched: HashSet<u32> = (asked.iter())
        .flat_map(|word| index.entries_of(word))
        .copied()
        .collect();
    let marked =
        |word: &str| (index.entries_of(&fold(word)).iter()).any(|entry| matched.contains(entry));
    let shown = (best.iter().take(SHOWN))
        .map(|&record| shown(&renderer, &index.records[record], &marked))
        .collect();
    Outcome {
        pages: best.len(),
        some,
        shown,
        errors,
        renderer: Some(renderer),
    }
}

fn shown(renderer: &Renderer, record: &Record, marked: &impl Fn(&str) -> bool) -> Shown {
    let page = renderer.site().pages.get(record.page);
    let mut address = page.address.as_str().to_owned();
    if let Some((id, _)) = &record.heading {
        address.push('#');
        address.push_str(id);
    }
    Shown {
        title: page.title.as_ref().map(|title| title.value.clone()),
        heading: record.heading.as_ref().map(|(_, words)| words.clone()),
        address,
        excerpt: excerpt(&record.text, marked),
    }
}

/// The text around the first word in `text` that is `marked`, or its
/// start, cut at words, in pieces saying which are marked.
pub fn excerpt(text: &str, marked: impl Fn(&str) -> bool) -> Vec<(String, bool)> {
    let spans: Vec<_> = spans(text).collect();
    let matched = |span: &std::ops::Range<usize>| marked(&text[span.clone()]);
    let first = spans.iter().position(matched).unwrap_or(0);
    let Some(at) = spans.get(first) else {
        return Vec::new();
    };
    let from = (spans[..first].iter().rev())
        .take_while(|span| at.start - span.start <= BEFORE)
        .last()
        .map_or(at.start, |span| span.start);
    let last = (spans[first..].iter())
        .take_while(|span| span.end - from <= BEFORE + AFTER)
        .last()
        .map_or(at.end, |span| span.end);
    // On to the space after the last word, so its stop goes with it.
    let to = text[last..]
        .find(char::is_whitespace)
        .map_or(text.len(), |space| last + space);
    let mut pieces = Vec::new();
    if from > 0 {
        pieces.push(("\u{2026}".to_owned(), false));
    }
    let mut written = from;
    for span in spans
        .iter()
        .filter(|span| from <= span.start && span.end <= to)
    {
        if matched(span) {
            if written < span.start {
                pieces.push((text[written..span.start].to_owned(), false));
            }
            pieces.push((text[span.clone()].to_owned(), true));
            written = span.end;
        }
    }
    if written < to {
        pieces.push((text[written..to].to_owned(), false));
    }
    if to < text.len() {
        pieces.push(("\u{2026}".to_owned(), false));
    }
    pieces
}
