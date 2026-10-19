//! Search over a site's writing: one record for each part of a page that
//! a heading with an id starts, and one for the part before, ranked by
//! BM25 over words weighted by where they stand.

mod command;
mod files;
mod words;

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};

use rayon::prelude::*;

use crate::content::words::PageWord;
use crate::html::{Markup, markup};
use crate::site::{LanguageId, ListKey, Page, PageId, Site};
use crate::templates::Renderer;

pub use command::{Options, Outcome, Results, SHOWN, Shown, excerpt, results, run};
pub use files::{File, claims, entry, folder};
pub use words::{Stemmer, fold, spans};

/// How much a word counts where it stands. A page's title counts in every
/// part of the page, most in its opening.
const TITLE_OPENING: u32 = 5;
const TITLE: u32 = 2;
const HEADING: u32 = 3;
const TAG: u32 = 3;
const SUMMARY: u32 = 2;
const DESCRIBED: u32 = 2;
const TEXT: u32 = 1;

/// How much a word being typed counts in a longer word it starts.
const STARTING: f64 = 0.8;

/// BM25's constants, as most search engines set them.
const K1: f64 = 1.2;
const B: f64 = 0.75;

/// One part of a page as search reads it.
#[derive(Debug)]
pub struct Record {
    pub page: PageId,
    /// The heading the part starts with, by its id and its words; none for
    /// the part before the first heading.
    pub heading: Option<(String, String)>,
    /// Its words as a reader reads them, the heading's left out.
    pub text: String,
    /// The descriptions of the pictures it shows.
    pub described: Vec<String>,
}

/// Whether `page` is searched: published as a page of the site's own,
/// from a file, and not left out with `search: false`.
pub fn searched(page: &Page) -> bool {
    let address = &page.address;
    let html = !address.is_file() || address.as_str().ends_with(".html");
    page.lands()
        && page.source.is_some()
        && html
        && page.words.truth(PageWord::Search) != Some(false)
}

/// The records of every searched page in `language`: the pages with no
/// date by their paths, then the dated from the oldest, so a new post's
/// records come last and leave every other record where it was.
pub fn records(renderer: &Renderer, language: LanguageId) -> Vec<Record> {
    let site = renderer.site();
    let pages = (site.pages.iter()).filter(|(_, page)| page.language == language && searched(page));
    let mut pages: Vec<PageId> = pages.map(|(id, _)| id).collect();
    pages.sort_by_cached_key(|&id| {
        let page = site.pages.get(id);
        let day = (page.date.as_ref()).map(|date| date.value.iso_day());
        (day, page.path.clone())
    });
    let records = pages.par_iter().flat_map_iter(|&page| {
        renderer.cut_at_headings(page).into_iter().map(move |cut| {
            let mut text = crate::markdown::lines_of(&cut.html);
            if let Some((_, heading)) = &cut.heading
                && let Some(rest) = text.strip_prefix(heading.as_str())
            {
                text = rest.trim_start().to_owned();
            }
            Record {
                page,
                heading: cut.heading,
                text,
                described: described(&cut.html),
            }
        })
    });
    records.collect()
}

/// The files a reader's browser searches the site `renderer` read with,
/// each of `languages` its own.
pub fn files(renderer: &Renderer, languages: &[LanguageId]) -> Vec<File> {
    let site = renderer.site();
    let files = languages.par_iter().flat_map_iter(|&language| {
        let index = Index::of(site, language, records(renderer, language));
        files::files(renderer, &folder(site, language), &index)
    });
    files.collect()
}

/// The descriptions of the pictures `html` shows.
fn described(html: &str) -> Vec<String> {
    let alts = markup(html).filter_map(|(_, piece)| match piece {
        Markup::Start(tag) if tag.name == "img" => tag.get("alt").map(crate::markdown::words_of),
        _ => None,
    });
    alts.map(|alt| closed_up(&alt))
        .filter(|alt| !alt.is_empty())
        .collect()
}

/// `text` with each run of space closed up to one, and none at the ends.
fn closed_up(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The names of a page's tags, as they are shown.
fn tag_names(site: &Site, page: PageId) -> Vec<String> {
    let tags = crate::lists::tags_of(site, page).into_iter();
    tags.filter_map(|list| match &site.lists.get(list).key {
        ListKey::Tag(name) => Some(name.clone()),
        _ => None,
    })
    .collect()
}

/// The words of `record`, folded, each with its weight there, and the
/// record's length: its words counted by weight.
fn weighed(site: &Site, record: &Record) -> (HashMap<String, u32>, u32) {
    let page = site.pages.get(record.page);
    // Each page's records begin with the part before its first heading.
    let opening = record.heading.is_none();
    let mut words: HashMap<String, u32> = HashMap::new();
    let mut length = 0;
    let mut add = |text: &str, weight: u32| {
        for span in spans(text) {
            *words.entry(fold(&text[span])).or_default() += weight;
            length += weight;
        }
    };
    if let Some(title) = &page.title {
        add(&title.value, if opening { TITLE_OPENING } else { TITLE });
    }
    if opening {
        for tag in tag_names(site, record.page) {
            add(&tag, TAG);
        }
        if let Some(summary) = page.words.text(PageWord::Summary) {
            add(summary.value, SUMMARY);
        }
    }
    if let Some((_, heading)) = &record.heading {
        add(heading, HEADING);
    }
    for description in &record.described {
        add(description, DESCRIBED);
    }
    add(&record.text, TEXT);
    (words, length)
}

/// The records of a site's pages in one language, and which hold each
/// word.
///
/// Each word's stem in the language is an entry, listing the records
/// holding a word with that stem. Every word the records hold is listed
/// with its stem's entry, so a word is looked up as it is typed, with no
/// stemming: what a reader's browser does with the index files.
pub struct Index {
    pub records: Vec<Record>,
    /// Each record's length, its words counted by weight.
    lengths: Vec<u32>,
    average: f64,
    /// Every word the records hold, folded, in the order of their UTF-16
    /// code units, with its stem's entry.
    words: Vec<(String, u32)>,
    /// Each entry's records, in order, with the stem's weight in each.
    entries: Vec<Vec<(u32, u32)>>,
}

/// A record found, and how well it matched.
#[derive(Debug)]
pub struct Found {
    pub record: usize,
    /// How many of the words asked for it holds.
    pub matched: usize,
    pub score: f64,
}

impl Index {
    /// The index of `records`, of pages in `language`, each word cut to
    /// its stem by that language's rules.
    pub fn of(site: &Site, language: LanguageId, records: Vec<Record>) -> Index {
        let stemmer = Stemmer::of(site.languages.get(language).tag.as_deref());
        let weighed: Vec<(HashMap<String, u32>, u32)> = (records.par_iter())
            .map(|record| weighed(site, record))
            .collect();
        let mut words: Vec<&str> = (weighed.iter())
            .flat_map(|(words, _)| words.keys().map(String::as_str))
            .collect();
        words.par_sort_unstable_by(|a, b| in_utf16(a, b));
        words.dedup();
        let stems: Vec<String> = words.par_iter().map(|word| stemmer.stem(word)).collect();
        // Every stem numbered in its own order, each an entry, since some
        // record holds each word.
        let mut keys: Vec<&str> = stems.iter().map(String::as_str).collect();
        keys.par_sort_unstable();
        keys.dedup();
        let numbered: HashMap<&str, u32> = (keys.iter().enumerate())
            .map(|(id, &key)| (key, u32::try_from(id).expect("fewer stems than u32 holds")))
            .collect();
        let entry_of: HashMap<&str, u32> = (words.iter().zip(&stems))
            .map(|(&word, stem)| (word, numbered[stem.as_str()]))
            .collect();
        // Each record's stems, with their weights there.
        let held: Vec<HashMap<u32, u32>> = (weighed.par_iter())
            .map(|(words, _)| {
                let mut held: HashMap<u32, u32> = HashMap::new();
                for (word, weight) in words {
                    *held.entry(entry_of[word.as_str()]).or_default() += weight;
                }
                held
            })
            .collect();
        let mut entries: Vec<Vec<(u32, u32)>> = vec![Vec::new(); keys.len()];
        for (record, held) in held.into_iter().enumerate() {
            let record = u32::try_from(record).expect("fewer records than u32 holds");
            for (entry, weight) in held {
                entries[entry as usize].push((record, weight));
            }
        }
        let words: Vec<(String, u32)> = (words.iter())
            .map(|&word| (word.to_owned(), entry_of[word]))
            .collect();
        let lengths: Vec<u32> = weighed.iter().map(|(_, length)| *length).collect();
        let average = match lengths.len() {
            0 => 1.0,
            n => (lengths.iter().map(|&l| f64::from(l)).sum::<f64>() / n as f64).max(1.0),
        };
        Index {
            records,
            lengths,
            average,
            words,
            entries,
        }
    }

    /// The entry `word`, folded, is listed with; none for a word no record
    /// holds.
    pub fn entry_of(&self, word: &str) -> Option<u32> {
        let at = (self.words).binary_search_by(|(listed, _)| in_utf16(listed, word));
        at.ok().map(|at| self.words[at].1)
    }

    /// The entries `asked` looks in for each of its words, each with how
    /// much a match there counts.
    fn looked_in(&self, asked: &Asked) -> Vec<Vec<(u32, f64)>> {
        let words = asked.words.iter();
        words
            .map(|word| {
                if asked.typing.as_ref() != Some(word) {
                    let entry = self.entry_of(word).into_iter();
                    return entry.map(|entry| (entry, 1.0)).collect();
                }
                let from =
                    (self.words).partition_point(|(listed, _)| in_utf16(listed, word).is_lt());
                let starting = self.words[from..].iter();
                let starting = starting.take_while(|(listed, _)| listed.starts_with(word.as_str()));
                let mut entries: Vec<(u32, f64)> = Vec::new();
                for (listed, entry) in starting {
                    let counts = if listed == word { 1.0 } else { STARTING };
                    match entries.iter_mut().find(|(held, _)| held == entry) {
                        Some((_, kept)) => *kept = kept.max(counts),
                        None => entries.push((*entry, counts)),
                    }
                }
                entries
            })
            .collect()
    }

    /// Every entry `asked` looks in.
    pub fn matched(&self, asked: &Asked) -> HashSet<u32> {
        let looked = self.looked_in(asked).into_iter().flatten();
        looked.map(|(entry, _)| entry).collect()
    }

    /// The records holding any word `asked`: those holding more of its
    /// words first, then the best matched, then in order.
    pub fn search(&self, asked: &Asked) -> Vec<Found> {
        let count = self.records.len() as f64;
        let mut found: HashMap<u32, (usize, f64)> = HashMap::new();
        for entries in self.looked_in(asked) {
            let mut best: HashMap<u32, f64> = HashMap::new();
            for (entry, counts) in entries {
                let held = &self.entries[entry as usize];
                let holding = held.len() as f64;
                let rarity = (1.0 + (count - holding + 0.5) / (holding + 0.5)).ln();
                for &(record, weight) in held {
                    let weight = f64::from(weight);
                    let length = f64::from(self.lengths[record as usize]) / self.average;
                    let score =
                        rarity * weight * (K1 + 1.0) / (weight + K1 * (1.0 - B + B * length));
                    let kept = best.entry(record).or_default();
                    *kept = kept.max(score * counts);
                }
            }
            for (record, score) in best {
                let entry = found.entry(record).or_default();
                entry.0 += 1;
                entry.1 += score;
            }
        }
        let mut found: Vec<Found> = found
            .into_iter()
            .map(|(record, (matched, score))| Found {
                record: record as usize,
                matched,
                score,
            })
            .collect();
        found.sort_by(|a, b| {
            (b.matched.cmp(&a.matched))
                .then(b.score.total_cmp(&a.score))
                .then(a.record.cmp(&b.record))
        });
        found
    }

    /// Every word the records hold, folded, in the order of their UTF-16
    /// code units.
    pub fn words(&self) -> impl Iterator<Item = &str> {
        self.words.iter().map(|(word, _)| word.as_str())
    }
}

/// What a search asks for: its words, folded, each once, in the order of
/// their UTF-16 code units, and the one being typed, if any, which is
/// looked for as the start of longer words too.
#[derive(Debug, PartialEq)]
pub struct Asked {
    pub words: Vec<String>,
    pub typing: Option<String>,
}

impl Asked {
    /// The words of `query`, its last being typed where `typing` and
    /// nothing follows it, and it is two letters or more, or one of a
    /// script written without spaces.
    pub fn of(query: &str, typing: bool) -> Asked {
        let spans: Vec<_> = spans(query).collect();
        let typed = (spans.last())
            .filter(|span| typing && span.end == query.len())
            .map(|span| fold(&query[span.clone()]))
            .filter(|word| word.chars().nth(1).is_some() || word.chars().any(unspaced));
        let mut words: Vec<String> = spans
            .iter()
            .map(|span| fold(&query[span.clone()]))
            .collect();
        words.sort_unstable_by(|a, b| in_utf16(a, b));
        words.dedup();
        Asked {
            words,
            typing: typed,
        }
    }
}

/// Whether `c` is of Chinese, Japanese or Korean, where one letter may
/// start many words.
fn unspaced(c: char) -> bool {
    matches!(c,
        '\u{1100}'..='\u{11ff}'
        | '\u{3040}'..='\u{30ff}'
        | '\u{3130}'..='\u{318f}'
        | '\u{31f0}'..='\u{31ff}'
        | '\u{3400}'..='\u{4dbf}'
        | '\u{4e00}'..='\u{9fff}'
        | '\u{ac00}'..='\u{d7af}'
        | '\u{f900}'..='\u{faff}'
        | '\u{ff66}'..='\u{ff9f}'
        | '\u{20000}'..='\u{2ffff}')
}

/// `a` against `b` by their UTF-16 code units, as a browser orders strings.
pub fn in_utf16(a: &str, b: &str) -> std::cmp::Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}
