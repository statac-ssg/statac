//! Search over a site's writing: one record for each part of a page that
//! a heading with an id starts, and one for the part before, ranked by
//! BM25 over words weighted by where they stand.

mod command;
mod files;
mod words;

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use rayon::prelude::*;

use crate::content::words::PageWord;
use crate::html::{Markup, markup};
use crate::site::{Id, ListKey, Page, PageId, Site};
use crate::templates::Renderer;

pub use command::{Options, Outcome, SHOWN, Shown, excerpt, run};
pub use files::{ENTRY, FOLDER, File, claims};
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

/// Every searched page's records: the pages with no date by their paths,
/// then the dated from the oldest, so a new post's records come last and
/// leave every other record where it was.
pub fn records(renderer: &Renderer) -> Vec<Record> {
    let site = renderer.site();
    let pages = site.pages.iter().filter(|(_, page)| searched(page));
    let mut pages: Vec<PageId> = pages.map(|(id, _)| id).collect();
    pages.sort_by_cached_key(|&id| {
        let page = site.pages.get(id);
        let day = (page.date.as_ref()).map(|date| date.value.iso_day());
        (day, page.path.clone())
    });
    let records = pages.par_iter().flat_map_iter(|&page| {
        renderer.cut_at_headings(page).into_iter().map(move |cut| {
            let mut text = crate::markdown::words_of(&cut.html);
            if let Some((_, heading)) = &cut.heading {
                text = text
                    .trim_start()
                    .strip_prefix(heading.as_str())
                    .map_or(text.clone(), str::to_owned);
            }
            Record {
                page,
                heading: cut.heading,
                text: closed_up(&text),
                described: described(&cut.html),
            }
        })
    });
    records.collect()
}

/// The files a reader's browser searches the site `renderer` read with.
pub fn files(renderer: &Renderer) -> Vec<File> {
    let index = Index::of(renderer.site(), records(renderer));
    files::files(renderer, &index)
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

/// The records of a site, and which hold each word.
///
/// Each word's stem in one language is an entry, listing the records of
/// that language holding a word with that stem. Every word the site holds
/// is listed with the entries of its stem in each of the site's languages,
/// so a word is looked up as it is typed, with no stemming: what a reader's
/// browser does with the index files.
pub struct Index {
    pub records: Vec<Record>,
    /// Each record's length, its words counted by weight.
    lengths: Vec<u32>,
    average: f64,
    /// Every word the site holds, folded, in the order of their UTF-16 code
    /// units, with the entries of its stems.
    words: Vec<(String, Vec<u32>)>,
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
    pub fn of(site: &Site, records: Vec<Record>) -> Index {
        let stemmers: Vec<Stemmer> = (site.languages.iter())
            .map(|(_, language)| Stemmer::of(language.tag.as_deref()))
            .collect();
        let languages: Vec<usize> = (records.iter())
            .map(|record| site.pages.get(record.page).language.index())
            .collect();
        let weighed: Vec<(HashMap<String, u32>, u32)> = (records.par_iter())
            .map(|record| weighed(site, record))
            .collect();
        let mut words: Vec<&str> = (weighed.iter())
            .flat_map(|(words, _)| words.keys().map(String::as_str))
            .collect();
        words.par_sort_unstable_by(|a, b| in_utf16(a, b));
        words.dedup();
        // Each word's stem in each language, and every stem numbered in
        // order of its language and itself.
        let stems: Vec<Vec<String>> = (words.par_iter())
            .map(|word| stemmers.iter().map(|stemmer| stemmer.stem(word)).collect())
            .collect();
        let mut keys: Vec<(usize, &str)> = (stems.iter())
            .flat_map(|stems| stems.iter().map(String::as_str).enumerate())
            .collect();
        keys.par_sort_unstable();
        keys.dedup();
        let numbered: HashMap<(usize, &str), u32> = (keys.iter().enumerate())
            .map(|(id, &key)| (key, u32::try_from(id).expect("fewer stems than u32 holds")))
            .collect();
        let numbers: Vec<Vec<u32>> = (stems.par_iter())
            .map(|stems| {
                let stems = stems.iter().enumerate();
                stems
                    .map(|(language, stem)| numbered[&(language, stem.as_str())])
                    .collect()
            })
            .collect();
        let at: HashMap<&str, usize> = words.iter().enumerate().map(|(i, &w)| (w, i)).collect();
        // Each record's stems, with their weights there.
        let held: Vec<HashMap<u32, u32>> = (weighed.par_iter().zip(&languages))
            .map(|((words, _), &language)| {
                let mut held: HashMap<u32, u32> = HashMap::new();
                for (word, weight) in words {
                    *held
                        .entry(numbers[at[word.as_str()]][language])
                        .or_default() += weight;
                }
                held
            })
            .collect();
        let mut listed: Vec<Vec<(u32, u32)>> = vec![Vec::new(); keys.len()];
        for (record, held) in held.into_iter().enumerate() {
            let record = u32::try_from(record).expect("fewer records than u32 holds");
            for (stem, weight) in held {
                listed[stem as usize].push((record, weight));
            }
        }
        // An entry for each stem some record holds, in the same order.
        let mut entry_of = vec![None; keys.len()];
        let mut entries: Vec<Vec<(u32, u32)>> = Vec::new();
        for (stem, records) in listed.into_iter().enumerate() {
            if !records.is_empty() {
                entry_of[stem] =
                    Some(u32::try_from(entries.len()).expect("fewer stems than u32 holds"));
                entries.push(records);
            }
        }
        let words: Vec<(String, Vec<u32>)> = (words.iter().zip(&numbers))
            .map(|(&word, numbers)| {
                let entries = numbers.iter().filter_map(|&stem| entry_of[stem as usize]);
                (word.to_owned(), entries.collect())
            })
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

    /// The entries `word`, folded, is listed with; none for a word the
    /// site never holds.
    pub fn entries_of(&self, word: &str) -> &[u32] {
        match (self.words).binary_search_by(|(listed, _)| in_utf16(listed, word)) {
            Ok(at) => &self.words[at].1,
            Err(_) => &[],
        }
    }

    /// The records holding any word of `query`: those holding more of its
    /// words first, then the best matched, then in order.
    pub fn search(&self, query: &str) -> Vec<Found> {
        let count = self.records.len() as f64;
        let mut found: HashMap<u32, (usize, f64)> = HashMap::new();
        for word in asked(query) {
            let mut best: HashMap<u32, f64> = HashMap::new();
            for &entry in self.entries_of(&word) {
                let held = &self.entries[entry as usize];
                let holding = held.len() as f64;
                let rarity = (1.0 + (count - holding + 0.5) / (holding + 0.5)).ln();
                for &(record, weight) in held {
                    let weight = f64::from(weight);
                    let length = f64::from(self.lengths[record as usize]) / self.average;
                    let score =
                        rarity * weight * (K1 + 1.0) / (weight + K1 * (1.0 - B + B * length));
                    let kept = best.entry(record).or_default();
                    *kept = kept.max(score);
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

    /// Every word the site holds, folded, in the order of their UTF-16
    /// code units.
    pub fn words(&self) -> impl Iterator<Item = &str> {
        self.words.iter().map(|(word, _)| word.as_str())
    }
}

/// The words of `query`, folded, each once, in order.
pub fn asked(query: &str) -> Vec<String> {
    let mut asked: Vec<String> = spans(query).map(|span| fold(&query[span])).collect();
    asked.sort_unstable();
    asked.dedup();
    asked
}

/// `a` against `b` by their UTF-16 code units, as a browser orders strings.
pub fn in_utf16(a: &str, b: &str) -> std::cmp::Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}
