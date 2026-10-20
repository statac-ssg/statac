//! Search over a site's writing: one record for each part of a page that
//! a heading with an id starts, and one for the part before, ranked by
//! BM25 over words weighted by where they stand.

mod command;
mod files;
mod kept;
mod meant;
mod why;
mod words;

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};

use rayon::prelude::*;

use crate::content::words::PageWord;
use crate::html::{Markup, markup};
use crate::site::{LanguageId, ListKey, Page, PageId, Site};
use crate::templates::Renderer;

pub use command::{Options, Outcome, Results, SHOWN, Shown, code, excerpt, results, run};
pub use files::{File, claims, entry, folder};
pub use kept::Kept;
pub use why::{Counted, Stands, Why};
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

impl Record {
    /// This record, of the page `page`.
    fn of(&self, page: PageId) -> Record {
        Record {
            page,
            heading: self.heading.clone(),
            text: self.text.clone(),
            described: self.described.clone(),
        }
    }
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

/// The records of every searched page in `language`, the pages in the
/// order [`pages`] gives.
pub fn records(renderer: &Renderer, language: LanguageId) -> Vec<Record> {
    let pages = pages(renderer.site(), language);
    let records = pages
        .par_iter()
        .flat_map_iter(|&page| records_of(renderer, page));
    records.collect()
}

/// Every searched page in `language`: the pages with no date by their
/// paths, then the dated from the oldest, so a new post's records come
/// last and leave every other record where it was.
fn pages(site: &Site, language: LanguageId) -> Vec<PageId> {
    let pages = (site.pages.iter()).filter(|(_, page)| page.language == language && searched(page));
    let mut pages: Vec<PageId> = pages.map(|(id, _)| id).collect();
    pages.sort_by_cached_key(|&id| {
        let page = site.pages.get(id);
        let day = (page.date.as_ref()).map(|date| date.value.iso_day());
        (day, page.path.clone())
    });
    pages
}

/// The records of `page`, in order.
fn records_of(renderer: &Renderer, page: PageId) -> impl Iterator<Item = Record> {
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
}

/// The files a reader's browser searches the site `renderer` read with,
/// each of `languages` its own, taking from `kept` what an earlier build
/// made from the same.
pub fn files(renderer: &Renderer, languages: &[LanguageId], kept: &mut Kept) -> Vec<File> {
    let standing = kept.standing();
    let taking = &*kept;
    let made: Vec<(Vec<File>, Vec<PageSearched>)> = (languages.par_iter())
        .map(|&language| language_files(renderer, language, &standing, taking))
        .collect();
    let mut files = Vec::new();
    let mut cut = HashMap::new();
    for (made, pages) in made {
        files.extend(made);
        cut.extend((pages.into_iter()).filter_map(|page| Some((page.page, page.kept?))));
    }
    kept.end(cut);
    files
}

/// One page as a build searches it.
struct PageSearched {
    page: PageId,
    records: Vec<Record>,
    weighed: Arc<[Weighed]>,
    /// Its records, where they are kept for the next build.
    kept: Option<Arc<[Record]>>,
}

/// The files of `language`'s index, and its pages as they were searched,
/// taking the records of each page `standing` holds from there, and from
/// `kept` what an earlier build made from the same.
fn language_files(
    renderer: &Renderer,
    language: LanguageId,
    standing: &HashMap<PageId, Arc<[Record]>>,
    kept: &Kept,
) -> (Vec<File>, Vec<PageSearched>) {
    let site = renderer.site();
    let pages = pages(site, language);
    let mut pages: Vec<PageSearched> = (pages.par_iter())
        .map(|&page| page_searched(renderer, page, standing.get(&page), kept))
        .collect();
    let weighed: Vec<&Weighed> = pages.iter().flat_map(|page| &*page.weighed).collect();
    let assembled = Index::assembled(site, language, &weighed);
    let records = pages
        .iter_mut()
        .flat_map(|page| std::mem::take(&mut page.records));
    let index = assembled.with(records.collect());
    let files = files::files(renderer, &folder(site, language), &index, kept);
    (files, pages)
}

/// `page` as a build searches it: its records as `standing` holds them,
/// where its writing stands, or else as they are cut from it, and weighed
/// as `kept` keeps them, where an earlier build weighed the same.
fn page_searched(
    renderer: &Renderer,
    page: PageId,
    standing: Option<&Arc<[Record]>>,
    kept: &Kept,
) -> PageSearched {
    let (records, cut): (Vec<Record>, Option<Arc<[Record]>>) = match standing {
        Some(cut) => {
            let records = cut.iter().map(|record| record.of(page)).collect();
            (records, Some(Arc::clone(cut)))
        }
        None => {
            let records: Vec<Record> = records_of(renderer, page).collect();
            let cut =
                (kept.keeps()).then(|| records.iter().map(|record| record.of(page)).collect());
            (records, cut)
        }
    };
    let said = Said::of(renderer.site(), page);
    let weighed = kept.weighed(
        |hasher| said.hash(&records, hasher),
        || {
            records
                .iter()
                .map(|record| weighed(&said, record))
                .collect()
        },
    );
    PageSearched {
        page,
        records,
        weighed,
        kept: cut,
    }
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

/// What weighing a page's records reads of the page besides them.
struct Said<'a> {
    title: Option<&'a str>,
    /// The names of its tags, as they are shown.
    tags: Vec<String>,
    summary: Option<&'a str>,
}

impl<'a> Said<'a> {
    fn of(site: &'a Site, page: PageId) -> Said<'a> {
        let held = site.pages.get(page);
        Said {
            title: held.title.as_ref().map(|title| title.value.as_str()),
            tags: tag_names(site, page),
            summary: (held.words.text(PageWord::Summary)).map(|summary| summary.value),
        }
    }

    /// Feeds `hasher` everything weighing `records` reads, this said of
    /// their page.
    fn hash(&self, records: &[Record], hasher: &mut blake3::Hasher) {
        let maybe = |hasher: &mut blake3::Hasher, text: Option<&str>| {
            hasher.update(&[u8::from(text.is_some())]);
            kept::hashed(hasher, text.unwrap_or_default());
        };
        maybe(hasher, self.title);
        maybe(hasher, self.summary);
        hasher.update(&(self.tags.len() as u64).to_le_bytes());
        for tag in &self.tags {
            kept::hashed(hasher, tag);
        }
        hasher.update(&(records.len() as u64).to_le_bytes());
        for record in records {
            maybe(
                hasher,
                record.heading.as_ref().map(|(_, words)| words.as_str()),
            );
            kept::hashed(hasher, &record.text);
            hasher.update(&(record.described.len() as u64).to_le_bytes());
            for description in &record.described {
                kept::hashed(hasher, description);
            }
        }
    }
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

/// Where in a record a word stands, as search weighs it.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Place {
    Title,
    Heading,
    Tags,
    Summary,
    /// The descriptions of the pictures it shows.
    Pictures,
    Text,
}

impl Place {
    pub fn name(self) -> &'static str {
        match self {
            Place::Title => "title",
            Place::Heading => "heading",
            Place::Tags => "tags",
            Place::Summary => "summary",
            Place::Pictures => "pictures",
            Place::Text => "text",
        }
    }
}

/// Each word of `record`, of a page `said` is said of, folded, in turn,
/// with where it stands and its weight there.
fn placed(said: &Said, record: &Record, mut each: impl FnMut(&str, Place, u32)) {
    // Each page's records begin with the part before its first heading.
    let opening = record.heading.is_none();
    let mut folded = String::new();
    let mut add = |text: &str, place: Place, weight: u32| {
        for span in spans(text) {
            words::fold_into(&text[span], &mut folded);
            each(&folded, place, weight);
        }
    };
    if let Some(title) = said.title {
        let weight = if opening { TITLE_OPENING } else { TITLE };
        add(title, Place::Title, weight);
    }
    if opening {
        for tag in &said.tags {
            add(tag, Place::Tags, TAG);
        }
        if let Some(summary) = said.summary {
            add(summary, Place::Summary, SUMMARY);
        }
    }
    if let Some((_, heading)) = &record.heading {
        add(heading, Place::Heading, HEADING);
    }
    for description in &record.described {
        add(description, Place::Pictures, DESCRIBED);
    }
    add(&record.text, Place::Text, TEXT);
}

/// A record's words, folded, each once with its weight there, and its
/// length: its words counted by weight.
struct Weighed {
    words: Vec<(String, u32)>,
    length: u32,
}

/// `record`, of a page `said` is said of, weighed.
fn weighed(said: &Said, record: &Record) -> Weighed {
    let mut words: HashMap<String, u32> = HashMap::new();
    let mut length = 0;
    placed(said, record, |word, _, weight| {
        match words.get_mut(word) {
            Some(held) => *held += weight,
            None => {
                words.insert(word.to_owned(), weight);
            }
        }
        length += weight;
    });
    Weighed {
        words: words.into_iter().collect(),
        length,
    }
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
    /// Where each piece of the word list a reader's browser is given
    /// starts, once asked for.
    starts: OnceLock<Vec<usize>>,
}

/// An index but for its records.
struct Assembled {
    lengths: Vec<u32>,
    average: f64,
    words: Vec<(String, u32)>,
    entries: Vec<Vec<(u32, u32)>>,
}

impl Assembled {
    /// The index of `records`, the records this was assembled of.
    fn with(self, records: Vec<Record>) -> Index {
        Index {
            records,
            lengths: self.lengths,
            average: self.average,
            words: self.words,
            entries: self.entries,
            starts: OnceLock::new(),
        }
    }
}

/// A record found, and how well it matched.
#[derive(Copy, Clone, PartialEq, Debug)]
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
        let weighed: Vec<Weighed> = (records.par_iter())
            .map(|record| weighed(&Said::of(site, record.page), record))
            .collect();
        let weighed: Vec<&Weighed> = weighed.iter().collect();
        Index::assembled(site, language, &weighed).with(records)
    }

    /// The index of the records `weighed` are, of pages in `language`, but
    /// for the records themselves.
    fn assembled(site: &Site, language: LanguageId, weighed: &[&Weighed]) -> Assembled {
        let stemmer = Stemmer::of(site.languages.get(language).tag.as_deref());
        // Each word once before sorting, as most records hold words many
        // others hold.
        let words = weighed.par_iter().fold(HashSet::new, |mut words, weighed| {
            words.extend(weighed.words.iter().map(|(word, _)| word.as_str()));
            words
        });
        let words = words.reduce(HashSet::new, |mut a, mut b| {
            if a.len() < b.len() {
                std::mem::swap(&mut a, &mut b);
            }
            a.extend(b);
            a
        });
        let mut words: Vec<&str> = words.into_iter().collect();
        words.par_sort_unstable_by(|a, b| in_utf16(a, b));
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
            .map(|weighed| {
                let mut held: HashMap<u32, u32> = HashMap::new();
                for (word, weight) in &weighed.words {
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
        let lengths: Vec<u32> = weighed.iter().map(|weighed| weighed.length).collect();
        let average = match lengths.len() {
            0 => 1.0,
            n => (lengths.iter().map(|&l| f64::from(l)).sum::<f64>() / n as f64).max(1.0),
        };
        Assembled {
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
        let mut found: HashMap<u32, (usize, f64)> = HashMap::new();
        for entries in self.looked_in(asked) {
            let mut best: HashMap<u32, f64> = HashMap::new();
            for (entry, counts) in entries {
                let held = &self.entries[entry as usize];
                let rarity = self.rarity(held.len());
                for &(record, weight) in held {
                    let kept = best.entry(record).or_default();
                    *kept = kept.max(self.scored(rarity, weight, record) * counts);
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

    /// How rare a word held by `holding` of the records is among them.
    fn rarity(&self, holding: usize) -> f64 {
        let count = self.records.len() as f64;
        let holding = holding as f64;
        (1.0 + (count - holding + 0.5) / (holding + 0.5)).ln()
    }

    /// What a word as rare as `rarity` scores in `record`, held there at
    /// `weight`.
    fn scored(&self, rarity: f64, weight: u32, record: u32) -> f64 {
        let weight = f64::from(weight);
        let length = f64::from(self.lengths[record as usize]) / self.average;
        rarity * weight * (K1 + 1.0) / (weight + K1 * (1.0 - B + B * length))
    }

    /// The records' average length, their words counted by weight.
    pub fn average(&self) -> f64 {
        self.average
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
