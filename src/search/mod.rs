//! Search over a site's writing: one record for each part of a page that
//! a heading with an id starts, and one for the part before, ranked by
//! BM25 over words weighted by where they stand.

mod command;
mod words;

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, HashMap};

use rayon::prelude::*;

use crate::content::words::PageWord;
use crate::html::{Markup, markup};
use crate::site::{Id, ListKey, Page, PageId, Site};
use crate::templates::Renderer;

pub use command::{Options, Outcome, SHOWN, Shown, excerpt, run};
pub use words::{Stemmer, fold, spans};

/// How much a word counts where it stands. A page's title counts in every
/// part of the page, most in its opening.
const TITLE_OPENING: f32 = 5.0;
const TITLE: f32 = 2.0;
const HEADING: f32 = 3.0;
const TAG: f32 = 3.0;
const SUMMARY: f32 = 2.0;
const DESCRIBED: f32 = 2.0;
const TEXT: f32 = 1.0;

/// BM25's constants, as most search engines set them.
const K1: f32 = 1.2;
const B: f32 = 0.75;

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

/// Every searched page's records, in the order of the site's pages.
pub fn records(renderer: &Renderer) -> Vec<Record> {
    let site = renderer.site();
    let pages = site.pages.iter().filter(|(_, page)| searched(page));
    let pages: Vec<PageId> = pages.map(|(id, _)| id).collect();
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

/// The form of each word `record` holds, by the rules of `stemmer`, with
/// its weight there, and the record's length: its words counted by weight.
fn weighed(site: &Site, record: &Record, stemmer: &Stemmer) -> (HashMap<String, f32>, f32) {
    let page = site.pages.get(record.page);
    // Each page's records begin with the part before its first heading.
    let opening = record.heading.is_none();
    let mut forms: HashMap<String, f32> = HashMap::new();
    let mut length = 0.0;
    let mut add = |text: &str, weight: f32| {
        for term in stemmer.terms(text) {
            *forms.entry(term).or_default() += weight;
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
    (forms, length)
}

/// The records of a site, and which hold each word.
pub struct Index {
    pub records: Vec<Record>,
    /// The rules each of the site's languages cuts words to stems by.
    stemmers: Vec<Stemmer>,
    /// Each record's language, by its place among the site's.
    languages: Vec<usize>,
    /// Each record's length, its words counted by weight.
    lengths: Vec<f32>,
    average: f32,
    /// Each word's form, and the records holding it with its weight in each.
    postings: BTreeMap<String, Vec<(u32, f32)>>,
}

/// A record found, and how well it matched.
#[derive(Debug)]
pub struct Found {
    pub record: usize,
    /// How many of the words asked for it holds.
    pub matched: usize,
    pub score: f32,
}

impl Index {
    pub fn of(site: &Site, records: Vec<Record>) -> Index {
        let stemmers: Vec<Stemmer> = (site.languages.iter())
            .map(|(_, language)| Stemmer::of(language.tag.as_deref()))
            .collect();
        let languages: Vec<usize> = (records.iter())
            .map(|record| site.pages.get(record.page).language.index())
            .collect();
        let weighed: Vec<(HashMap<String, f32>, f32)> = (records.par_iter().zip(&languages))
            .map(|(record, &language)| weighed(site, record, &stemmers[language]))
            .collect();
        let mut postings: BTreeMap<String, Vec<(u32, f32)>> = BTreeMap::new();
        let mut lengths = Vec::with_capacity(records.len());
        for (at, (forms, length)) in weighed.into_iter().enumerate() {
            let at = u32::try_from(at).expect("fewer records than u32 holds");
            for (term, weight) in forms {
                postings.entry(term).or_default().push((at, weight));
            }
            lengths.push(length);
        }
        let average = match lengths.len() {
            0 => 1.0,
            n => (lengths.iter().sum::<f32>() / n as f32).max(1.0),
        };
        Index {
            records,
            stemmers,
            languages,
            lengths,
            average,
            postings,
        }
    }

    /// The rules the words of `record` are cut to stems by.
    pub fn stemmer(&self, record: usize) -> &Stemmer {
        &self.stemmers[self.languages[record]]
    }

    /// The records holding any word of `query`, each word looked for in
    /// its form in each record's language: those holding more of its words
    /// first, then the best matched.
    pub fn search(&self, query: &str) -> Vec<Found> {
        let mut asked: Vec<String> = spans(query).map(|span| fold(&query[span])).collect();
        asked.sort_unstable();
        asked.dedup();
        let count = self.records.len() as f32;
        let mut found: HashMap<u32, (usize, f32)> = HashMap::new();
        for word in &asked {
            // The word's form in each language, and the languages giving it.
            let mut forms: BTreeMap<String, Vec<usize>> = BTreeMap::new();
            for (language, stemmer) in self.stemmers.iter().enumerate() {
                forms.entry(stemmer.term(word)).or_default().push(language);
            }
            let mut best: HashMap<u32, f32> = HashMap::new();
            for (term, languages) in &forms {
                let Some(held) = self.postings.get(term) else {
                    continue;
                };
                let holding = held.len() as f32;
                let rarity = (1.0 + (count - holding + 0.5) / (holding + 0.5)).ln();
                let held = held
                    .iter()
                    .filter(|(record, _)| languages.contains(&self.languages[*record as usize]));
                for &(record, weight) in held {
                    let length = self.lengths[record as usize] / self.average;
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

    /// Each word's form the index holds, in order.
    pub fn terms(&self) -> impl Iterator<Item = &str> {
        self.postings.keys().map(String::as_str)
    }
}
