//! `statac search`: what a reader of the site would find for some words.

use std::collections::HashSet;
use std::ops::Range;
use std::path::PathBuf;

use super::{Asked, Index, Record, fold, runs, spans, unspaced};
use crate::date::Date;
use crate::ids::PageId;
use crate::problem::{Kind, Problem};
use crate::site::Language;
use crate::templates::Renderer;

/// How many pages are shown, best first.
pub const SHOWN: usize = 10;

/// About how many UTF-16 code units of a record's text are shown around
/// the first word matched.
const BEFORE: usize = 60;
const AFTER: usize = 140;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Options {
    pub site: PathBuf,
    pub words: String,
    pub today: Option<Date>,
    /// Whether drafts and posts waiting for their day are searched too.
    pub drafts: bool,
    /// The code of the one language whose pages are searched; none for
    /// every language's.
    pub language: Option<String>,
}

/// One page found, at the part of it that matched best.
#[derive(Debug, PartialEq)]
pub struct Shown {
    /// What a list calls the page.
    pub title: String,
    /// The page's date as the site writes dates, where it isn't already
    /// called by it.
    pub day: Option<String>,
    pub heading: Option<String>,
    /// From the top of the site, with the heading's id where there is one.
    pub address: String,
    /// The text around the first word matched, in pieces, each saying
    /// whether it is a word matched.
    pub excerpt: Vec<(String, bool)>,
    /// The part, by its place among the index's records, and how well it
    /// matched.
    pub found: super::Found,
}

/// Where `options` name no folder and the words are a site's folder, as
/// every other command takes it, what says the words go first.
pub fn folder_for_words(options: &Options) -> Option<Problem> {
    let words = options.words.trim();
    let site = std::path::Path::new(words).join(crate::config::FILE_NAME);
    (options.site.as_os_str().is_empty() && site.is_file()).then(|| {
        Problem::nowhere(Kind::NothingToSearch {
            site: Some(words.to_owned()),
        })
    })
}

/// A language's code, as the site declares it.
pub fn code(language: &Language) -> &str {
    language.tag.as_deref().unwrap_or(crate::language::ENGLISH)
}

/// What a search finds, as a reader's browser shows it.
#[derive(Debug, PartialEq)]
pub struct Results {
    /// The pages found, best first, at most as many as asked.
    pub shown: Vec<Shown>,
    /// How many pages were found in all.
    pub pages: usize,
    /// Whether no page held every word, and those found hold some.
    pub some: bool,
    /// Where no page was found, the words respelt as the nearest the
    /// index lists, where they find some.
    pub suggestion: Option<String>,
}

/// The pages of the site `renderer` read that `index` finds for `query`,
/// each once at its best part, the first `most` shown; with its last word
/// being typed where `typing`.
pub fn results(
    renderer: &Renderer,
    index: &Index,
    query: &str,
    typing: bool,
    most: usize,
) -> Results {
    let asked = Asked::of(query, typing, index);
    let found = index.search(&asked);
    // Records holding every word, or where none does, those holding most.
    let best_matched = found.first().map_or(0, |found| found.matched);
    let mut seen: HashSet<PageId> = HashSet::new();
    let best = (found.iter())
        .take_while(|found| found.matched == best_matched)
        .filter(|found| seen.insert(index.records[found.record].page));
    let best: Vec<&super::Found> = best.collect();
    // A word of the text is marked where it shares a stem with one asked.
    let matched = index.matched(&asked);
    let marked = |word: &str| (index.entry_of(&fold(word))).is_some_and(|e| matched.contains(&e));
    let shown = (best.iter().take(most))
        .map(|&&found| shown(renderer, &index.records[found.record], found, &marked))
        .collect();
    let suggestion = match best.is_empty() {
        true => index.meant(query, typing),
        false => None,
    };
    Results {
        shown,
        pages: best.len(),
        some: !best.is_empty() && best_matched < asked.words.len(),
        suggestion,
    }
}

fn shown(
    renderer: &Renderer,
    record: &Record,
    found: super::Found,
    marked: &impl Fn(&str) -> bool,
) -> Shown {
    let page = renderer.site().pages.get(record.page);
    let mut address = page.address.as_str().to_owned();
    if let Some((id, _)) = &record.heading {
        address.push('#');
        address.push_str(id);
    }
    let (title, day) = renderer.named(record.page);
    Shown {
        title,
        day,
        heading: record.heading.as_ref().map(|(_, words)| words.clone()),
        address,
        excerpt: excerpt(&record.text, marked),
        found,
    }
}

/// The text around the first word in `text` that is `marked`, or its
/// start, cut at words, in pieces saying which are marked. Lengths are
/// counted in UTF-16 code units, as a browser counts them.
pub fn excerpt(text: &str, marked: impl Fn(&str) -> bool) -> Vec<(String, bool)> {
    excerpt_of(text, spans(text).collect(), marked)
}

/// [`excerpt`], the words of `text` being at `spans`.
pub(super) fn excerpt_of(
    text: &str,
    spans: Vec<Range<usize>>,
    marked: impl Fn(&str) -> bool,
) -> Vec<(String, bool)> {
    let spans = joined(text, spans, &marked);
    let matched = |span: &std::ops::Range<usize>| marked(&text[span.clone()]);
    let first = spans.iter().position(matched).unwrap_or(0);
    let Some(at) = spans.get(first) else {
        return Vec::new();
    };
    let units = |from: usize, to: usize| text[from..to].encode_utf16().count();
    let from = (spans[..first].iter().rev())
        .take_while(|span| units(span.start, at.start) <= BEFORE)
        .last()
        .map_or(at.start, |span| span.start);
    let last = (spans[first..].iter())
        .take_while(|span| units(from, span.end) <= BEFORE + AFTER)
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

/// The words of `text` at `spans`, each run of Chinese, Japanese or
/// Korean next to each other that together make a word `marked` takes
/// taken as that one word, the longest first: a browser may cut such text
/// into smaller words than the index did, and the excerpt it shows reads
/// the text so too.
fn joined(
    text: &str,
    spans: Vec<Range<usize>>,
    marked: &impl Fn(&str) -> bool,
) -> Vec<Range<usize>> {
    let unspaced_at = |span: &Range<usize>| text[span.clone()].chars().all(unspaced);
    let mut words = Vec::new();
    let mut at = 0;
    while at < spans.len() {
        let mut last = at;
        if unspaced_at(&spans[at]) {
            let mut to = at + 1;
            while to < spans.len()
                && spans[to].start == spans[to - 1].end
                && unspaced_at(&spans[to])
            {
                let run = &text[spans[at].start..spans[to].end];
                if run.chars().nth(runs::LONGEST).is_some() {
                    break;
                }
                if marked(run) {
                    last = to;
                }
                to += 1;
            }
        }
        words.push(spans[at].start..spans[last].end);
        at = last + 1;
    }
    words
}
