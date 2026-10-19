//! Search over a site's writing: one record for each part of a page that
//! a heading with an id starts, and one for the part before, ranked by
//! BM25 over words weighted by where they stand.

mod command;
mod words;

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, HashMap};

use crate::content::words::PageWord;
use crate::html::{Markup, markup};
use crate::site::{ListKey, Page, PageId, Site};
use crate::templates::Renderer;

pub use command::{Options, Outcome, SHOWN, Shown, excerpt, run};
pub use words::{spans, words};

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
    let records = pages.iter().flat_map(|&page| {
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

/// The records of a site, and which hold each word.
pub struct Index {
    pub records: Vec<Record>,
    /// Each record's length, its words counted by weight.
    lengths: Vec<f32>,
    average: f32,
    /// Each word, and the records holding it with its weight in each.
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
        let mut postings: BTreeMap<String, HashMap<u32, f32>> = BTreeMap::new();
        let mut lengths = Vec::with_capacity(records.len());
        for (at, record) in records.iter().enumerate() {
            let at = u32::try_from(at).expect("fewer records than u32 holds");
            let page = site.pages.get(record.page);
            // Each page's records begin with the part before its first heading.
            let opening = record.heading.is_none();
            let mut length = 0.0;
            let mut add = |text: &str, weight: f32| {
                for word in words(text) {
                    *postings.entry(word).or_default().entry(at).or_default() += weight;
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
            lengths.push(length);
        }
        let average = match lengths.len() {
            0 => 1.0,
            n => (lengths.iter().sum::<f32>() / n as f32).max(1.0),
        };
        let postings = postings.into_iter().map(|(word, held)| {
            let mut held: Vec<(u32, f32)> = held.into_iter().collect();
            held.sort_unstable_by_key(|(record, _)| *record);
            (word, held)
        });
        Index {
            records,
            lengths,
            average,
            postings: postings.collect(),
        }
    }

    /// The records holding any word of `query`: those holding more of its
    /// words first, then the best matched.
    pub fn search(&self, query: &str) -> Vec<Found> {
        let mut asked: Vec<String> = words(query).collect();
        asked.sort_unstable();
        asked.dedup();
        let count = self.records.len() as f32;
        let mut found: HashMap<u32, (usize, f32)> = HashMap::new();
        for word in &asked {
            let Some(held) = self.postings.get(word) else {
                continue;
            };
            let holding = held.len() as f32;
            let rarity = (1.0 + (count - holding + 0.5) / (holding + 0.5)).ln();
            for &(record, weight) in held {
                let length = self.lengths[record as usize] / self.average;
                let score = rarity * weight * (K1 + 1.0) / (weight + K1 * (1.0 - B + B * length));
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

    /// Each word the index holds, in order.
    pub fn words(&self) -> impl Iterator<Item = &str> {
        self.postings.keys().map(String::as_str)
    }
}
