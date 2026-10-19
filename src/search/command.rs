//! `statac search`: what a reader of the site would find for some words.

use std::collections::HashSet;
use std::path::PathBuf;

use rayon::prelude::*;

use super::{Asked, Index, Record, fold, records, spans};
use crate::date::Date;
use crate::problem::{Expected, Found, Kind, Problem, Severity};
use crate::site::{Language, LanguageId, PageId};
use crate::source::Sources;
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
}

pub struct Outcome {
    renderer: Option<Renderer>,
    /// The errors that stopped the site being read.
    pub errors: Vec<Problem>,
    /// The language asked for, where the site declares none of its code.
    pub mistake: Option<Problem>,
    /// What each language searched found, by its code, in the site's
    /// order of languages, at most [`SHOWN`] pages shown of each.
    pub found: Vec<(String, Results)>,
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
    let renderer = prepared.renderer;
    let site = renderer.site();
    let mut languages: Vec<(String, LanguageId)> = (site.languages.iter())
        .map(|(id, language)| (code(language).to_owned(), id))
        .collect();
    let mut mistake = None;
    if let Some(asked) = &options.language {
        match languages.iter().position(|(code, _)| code == asked) {
            Some(at) => languages = vec![languages.swap_remove(at)],
            None => {
                let codes = languages.iter().map(|(code, _)| code.clone()).collect();
                mistake = Some(Problem::nowhere(Kind::WrongKind {
                    field: "--language".into(),
                    expected: Expected::OneOf(codes),
                    found: Found::Written(asked.clone()),
                    respelt: None,
                }));
            }
        }
    }
    if !errors.is_empty() || mistake.is_some() {
        return Outcome {
            renderer: Some(renderer),
            errors,
            mistake,
            found: Vec::new(),
        };
    }
    let found = (languages.into_par_iter())
        .map(|(code, language)| {
            let index = Index::of(site, language, records(&renderer, language));
            let results = results(&renderer, &index, &options.words, false, SHOWN);
            (code, results)
        })
        .collect();
    Outcome {
        renderer: Some(renderer),
        errors,
        mistake,
        found,
    }
}

/// A language's code, as the site declares it.
fn code(language: &Language) -> &str {
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
    let asked = Asked::of(query, typing);
    let found = index.search(&asked);
    // Records holding every word, or where none does, those holding most.
    let best_matched = found.first().map_or(0, |found| found.matched);
    let mut seen: HashSet<PageId> = HashSet::new();
    let best = (found.iter())
        .take_while(|found| found.matched == best_matched)
        .map(|found| found.record)
        .filter(|&record| seen.insert(index.records[record].page));
    let best: Vec<usize> = best.collect();
    // A word of the text is marked where it shares a stem with one asked.
    let matched = index.matched(&asked);
    let marked = |word: &str| (index.entry_of(&fold(word))).is_some_and(|e| matched.contains(&e));
    let shown = (best.iter().take(most))
        .map(|&record| shown(renderer, &index.records[record], &marked))
        .collect();
    let suggestion = match best.is_empty() {
        true => index.meant(query, typing),
        false => None,
    };
    Results {
        shown,
        pages: best.len(),
        some: best_matched < asked.words.len(),
        suggestion,
    }
}

fn shown(renderer: &Renderer, record: &Record, marked: &impl Fn(&str) -> bool) -> Shown {
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
    }
}

/// The text around the first word in `text` that is `marked`, or its
/// start, cut at words, in pieces saying which are marked. Lengths are
/// counted in UTF-16 code units, as a browser counts them.
pub fn excerpt(text: &str, marked: impl Fn(&str) -> bool) -> Vec<(String, bool)> {
    let spans: Vec<_> = spans(text).collect();
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
