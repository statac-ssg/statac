//! Search over a site's writing: one record for each part of a page that
//! a heading with an id starts, and one for the part before, ranked by
//! BM25 in each place a word stands: the page's title, the part's heading,
//! the page's tags and summary, the part's picture descriptions and its
//! text. How often a word stands in a place counts for less each time,
//! and the place's weight scales what that comes to, so a part holding a
//! word often in its text gains little on one holding it in its title or
//! heading. The title's length counts against a word in it, and the
//! text's against a word there.

mod command;
mod files;
mod kept;
mod meant;
mod runs;
mod why;
mod words;

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::sync::{Arc, OnceLock};

use rayon::prelude::*;

use crate::config::words::PageWord;
use crate::html::{Markup, markup};
use crate::ids::{LanguageId, PageId};
use crate::site::{ListKey, Page, Site};
use crate::templates::Renderer;

pub use command::{Options, Results, SHOWN, Shown, code, excerpt, folder_for_words, results};
pub(crate) use files::{File, claims};
pub(crate) use kept::Kept;
#[cfg(test)]
pub(crate) use why::{Counted, Stands};
pub(crate) use words::{Stemmer, fold, spans};

/// How much a word counts where it stands, at most. A page's title counts
/// in every part of the page, most in its opening.
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
    pub(crate) page: PageId,
    /// The heading the part starts with, by its id and its words; none for
    /// the part before the first heading.
    pub(crate) heading: Option<(String, String)>,
    /// Whether it opens the page: the part before the first heading, or
    /// the first heading's part where nothing stands before it.
    pub(crate) opening: bool,
    /// Its words as a reader reads them, the heading's left out.
    pub(crate) text: String,
    /// The descriptions of the pictures it shows.
    pub(crate) described: Vec<String>,
}

impl Record {
    /// This record, of the page `page`.
    fn of(&self, page: PageId) -> Record {
        Record {
            page,
            heading: self.heading.clone(),
            opening: self.opening,
            text: self.text.clone(),
            described: self.described.clone(),
        }
    }
}

/// Whether `page` is searched: published as a page of the site's own,
/// from a file, and not left out with `search: false`.
pub(crate) fn searched(page: &Page) -> bool {
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

/// The records of `page`, in order. Where nothing stands before the
/// first heading, the first heading's part opens the page, so that a page
/// found by its title is shown with text.
fn records_of(renderer: &Renderer, page: PageId) -> impl Iterator<Item = Record> {
    let mut records: Vec<Record> = (renderer.cut_at_headings(page).into_iter())
        .map(|cut| {
            let mut text = crate::markdown::lines_of(&cut.html);
            if let Some((_, heading)) = &cut.heading
                && let Some(rest) = text.strip_prefix(heading.as_str())
            {
                text = rest.trim_start().to_owned();
            }
            Record {
                page,
                opening: cut.heading.is_none(),
                heading: cut.heading,
                text,
                described: described(&cut.html),
            }
        })
        .collect();
    if records.len() > 1 && records[0].text.is_empty() && records[0].described.is_empty() {
        records.remove(0);
        records[0].opening = true;
    }
    records.into_iter()
}

/// The files a reader's browser searches the site `renderer` read with,
/// each of `languages` its own, taking from `kept` what an earlier build
/// made from the same.
pub(crate) fn files(renderer: &Renderer, languages: &[LanguageId], kept: &mut Kept) -> Vec<File> {
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
    let files = files::files(renderer, &site.search_folder(language), &index, kept);
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
            hasher.update(&[u8::from(record.opening)]);
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
pub(crate) enum Place {
    Title,
    Heading,
    Tags,
    Summary,
    /// The descriptions of the pictures it shows.
    Pictures,
    Text,
}

impl Place {
    /// Every place, in order.
    const ALL: [Place; 6] = [
        Place::Title,
        Place::Heading,
        Place::Tags,
        Place::Summary,
        Place::Pictures,
        Place::Text,
    ];

    /// How much a word here counts at most, in an opening record or not.
    fn weight(self, opening: bool) -> u32 {
        match self {
            Place::Title if opening => TITLE_OPENING,
            Place::Title => TITLE,
            Place::Heading => HEADING,
            Place::Tags => TAG,
            Place::Summary => SUMMARY,
            Place::Pictures => DESCRIBED,
            Place::Text => TEXT,
        }
    }

    pub(crate) fn name(self) -> &'static str {
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
/// with where it stands.
fn placed(said: &Said, record: &Record, mut each: impl FnMut(&str, Place)) {
    let mut folded = String::new();
    let mut add = |text: &str, place: Place| {
        for span in spans(text) {
            words::fold_into(&text[span], &mut folded);
            each(&folded, place);
        }
    };
    if let Some(title) = said.title {
        add(title, Place::Title);
    }
    if record.opening {
        for tag in &said.tags {
            add(tag, Place::Tags);
        }
        if let Some(summary) = said.summary {
            add(summary, Place::Summary);
        }
    }
    if let Some((_, heading)) = &record.heading {
        add(heading, Place::Heading);
    }
    for description in &record.described {
        add(description, Place::Pictures);
    }
    add(&record.text, Place::Text);
}

/// A record's words, folded, each once with how many times it stands in
/// each place, and the length of its text and of its page's title, in
/// words.
struct Weighed {
    words: Vec<(String, Times)>,
    length: u32,
    titled: u32,
}

/// How many times a word stands in each place in a record, by the place.
#[derive(Copy, Clone, Default, PartialEq, Debug)]
struct Times([u32; Place::ALL.len()]);

impl Times {
    /// A word standing `times` in a record's text and nowhere else.
    fn of_text(times: u32) -> Times {
        let mut held = Times::default();
        held.0[Place::Text as usize] = times;
        held
    }

    fn of(self, place: Place) -> u32 {
        self.0[place as usize]
    }

    fn add(&mut self, other: Times) {
        for (times, more) in self.0.iter_mut().zip(other.0) {
            *times += more;
        }
    }
}

/// `record`, of a page `said` is said of, weighed.
fn weighed(said: &Said, record: &Record) -> Weighed {
    let mut words: HashMap<String, Times> = HashMap::new();
    let mut length = 0;
    let mut titled = 0;
    placed(said, record, |word, place| {
        let mut once = Times::default();
        once.0[place as usize] = 1;
        match words.get_mut(word) {
            Some(times) => times.add(once),
            None => {
                words.insert(word.to_owned(), once);
            }
        }
        match place {
            Place::Text => length += 1,
            Place::Title => titled += 1,
            _ => {}
        }
    });
    Weighed {
        words: words.into_iter().collect(),
        length,
        titled,
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
    pub(crate) records: Vec<Record>,
    /// The length of each record's text, in words.
    lengths: Vec<u32>,
    average: f64,
    /// The length of each record's page's title, in words, and the
    /// average of those with a title.
    titles: Vec<u32>,
    title_average: f64,
    /// Every word the records hold, folded, in the order of their UTF-16
    /// code units, with its stem's entry.
    words: Vec<(String, u32)>,
    /// Each entry's records, in order, with how many times the stem
    /// stands in each place there.
    entries: Vec<Vec<(u32, Times)>>,
    /// Where each piece of the word list a reader's browser is given
    /// starts, once asked for.
    starts: OnceLock<Vec<usize>>,
}

/// An index but for its records.
struct Assembled {
    lengths: Vec<u32>,
    average: f64,
    titles: Vec<u32>,
    title_average: f64,
    words: Vec<(String, u32)>,
    entries: Vec<Vec<(u32, Times)>>,
}

impl Assembled {
    /// The index of `records`, the records this was assembled of.
    fn with(self, records: Vec<Record>) -> Index {
        Index {
            records,
            lengths: self.lengths,
            average: self.average,
            titles: self.titles,
            title_average: self.title_average,
            words: self.words,
            entries: self.entries,
            starts: OnceLock::new(),
        }
    }
}

/// A record found, and how well it matched.
#[derive(Copy, Clone, PartialEq, Debug)]
pub(crate) struct Found {
    pub(crate) record: usize,
    /// How many of the words asked for it holds.
    pub(crate) matched: usize,
    pub(crate) score: f64,
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
        // Each record's stems, with how many times each stands where.
        let held: Vec<HashMap<u32, Times>> = (weighed.par_iter())
            .map(|weighed| {
                let mut held: HashMap<u32, Times> = HashMap::new();
                for (word, times) in &weighed.words {
                    held.entry(entry_of[word.as_str()]).or_default().add(*times);
                }
                held
            })
            .collect();
        let mut entries: Vec<Vec<(u32, Times)>> = vec![Vec::new(); keys.len()];
        for (record, held) in held.into_iter().enumerate() {
            let record = u32::try_from(record).expect("fewer records than u32 holds");
            for (entry, times) in held {
                entries[entry as usize].push((record, times));
            }
        }
        let words: Vec<(String, u32)> = (words.iter())
            .map(|&word| (word.to_owned(), entry_of[word]))
            .collect();
        let lengths: Vec<u32> = weighed.iter().map(|weighed| weighed.length).collect();
        let titles: Vec<u32> = weighed.iter().map(|weighed| weighed.titled).collect();
        let average = mean(lengths.iter().copied());
        let title_average = mean(titles.iter().copied().filter(|&length| length > 0));
        Assembled {
            lengths,
            average,
            titles,
            title_average,
            words,
            entries,
        }
    }

    /// The entry `word`, folded, is listed with; none for a word no record
    /// holds.
    pub(crate) fn entry_of(&self, word: &str) -> Option<u32> {
        let at = (self.words).binary_search_by(|(listed, _)| in_utf16(listed, word));
        at.ok().map(|at| self.words[at].1)
    }

    /// Whether `word`, folded, starts some word the records hold.
    fn starts_word(&self, word: &str) -> bool {
        let from = (self.words).partition_point(|(listed, _)| in_utf16(listed, word).is_lt());
        (self.words.get(from)).is_some_and(|(listed, _)| listed.starts_with(word))
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
    pub(crate) fn matched(&self, asked: &Asked) -> HashSet<u32> {
        let looked = self.looked_in(asked).into_iter().flatten();
        looked.map(|(entry, _)| entry).collect()
    }

    /// The records holding any word `asked`: those holding more of its
    /// words first, then the best matched, then in order. A word being
    /// typed counts as one word across the words it starts, as rare as
    /// the records holding any of them, in each at the best of them.
    pub(crate) fn search(&self, asked: &Asked) -> Vec<Found> {
        let mut found: HashMap<u32, (usize, f64)> = HashMap::new();
        for entries in self.looked_in(asked) {
            let rarity = self.rarity(self.holding(&entries));
            let mut best: HashMap<u32, f64> = HashMap::new();
            for (entry, counts) in entries {
                for &(record, times) in &self.entries[entry as usize] {
                    let kept = best.entry(record).or_default();
                    *kept = kept.max(self.scored(rarity, times, record) * counts);
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

    /// How many records hold a word of any of the entries `looked`.
    fn holding(&self, looked: &[(u32, f64)]) -> usize {
        match looked {
            [(entry, _)] => self.entries[*entry as usize].len(),
            _ => {
                let held = (looked.iter()).flat_map(|&(entry, _)| &self.entries[entry as usize]);
                let records: HashSet<u32> = held.map(|&(record, _)| record).collect();
                records.len()
            }
        }
    }

    /// How rare a word held by `holding` of the records is among them.
    fn rarity(&self, holding: usize) -> f64 {
        let count = self.records.len() as f64;
        let holding = holding as f64;
        (1.0 + (count - holding + 0.5) / (holding + 0.5)).ln()
    }

    /// What a word as rare as `rarity` scores in `record`, standing there
    /// `times` in each place: BM25 in each place, by the place's weight,
    /// added.
    fn scored(&self, rarity: f64, times: Times, record: u32) -> f64 {
        let places = Place::ALL.iter();
        rarity * places.fold(0.0, |sum, &place| sum + self.counted(times, record, place))
    }

    /// What a word standing `times` in each place in `record` counts in
    /// `place`, as rare words go: as BM25 counts how often a word stands in
    /// it, the text's and the title's times scaled by their length, by the
    /// place's weight.
    fn counted(&self, times: Times, record: u32, place: Place) -> f64 {
        let at = record as usize;
        let held = times.of(place);
        let held = match place {
            Place::Text => scaled(held, self.lengths[at], self.average),
            Place::Title => scaled(held, self.titles[at], self.title_average),
            _ => f64::from(held),
        };
        f64::from(place.weight(self.records[at].opening)) * saturated(held)
    }

    /// The average length of the records' text, in words.
    pub(crate) fn average(&self) -> f64 {
        self.average
    }

    /// The average length of the titles of the records' pages, in words,
    /// of the records whose page has one.
    pub(crate) fn title_average(&self) -> f64 {
        self.title_average
    }

    /// Every word the records hold, folded, in the order of their UTF-16
    /// code units.
    #[cfg(test)]
    pub(crate) fn words(&self) -> impl Iterator<Item = &str> {
        self.words.iter().map(|(word, _)| word.as_str())
    }
}

/// The average of `lengths`, at least 1.
fn mean(lengths: impl Iterator<Item = u32>) -> f64 {
    let (total, n) = lengths.fold((0.0, 0), |(total, n), length| {
        (total + f64::from(length), n + 1)
    });
    match n {
        0 => 1.0,
        n => (total / f64::from(n)).max(1.0),
    }
}

/// `times` a word stands in something `length` long where `average` is
/// the usual, scaled as BM25 scales it.
fn scaled(times: u32, length: u32, average: f64) -> f64 {
    let length = f64::from(length) / average;
    f64::from(times) / (1.0 - B + B * length)
}

/// What a word standing `times` counts, each time more counting for less:
/// 1 for once where the length is the usual, never `K1 + 1`.
fn saturated(times: f64) -> f64 {
    times * (K1 + 1.0) / (times + K1)
}

/// What a search asks for: its words, folded, each once, in the order of
/// their UTF-16 code units, and the one being typed, if any, which is
/// looked for as the start of longer words too.
#[derive(Debug, PartialEq)]
pub(crate) struct Asked {
    pub(crate) words: Vec<String>,
    pub(crate) typing: Option<String>,
}

impl Asked {
    /// The words of `query`, each run of words of Chinese, Japanese or
    /// Korean next to each other read by the words `index` lists, its last
    /// being typed where `typing` and nothing follows it, and it is two
    /// letters or more, or one of a script written without spaces.
    pub(crate) fn of(query: &str, typing: bool, index: &Index) -> Asked {
        let spans: Vec<_> = spans(query).collect();
        let ending = typing && spans.last().is_some_and(|span| span.end == query.len());
        let folded = |span: &Range<usize>| fold(&query[span.clone()]);
        let unspaced_word = |at: usize| query[spans[at].clone()].chars().all(unspaced);
        let mut words: Vec<String> = Vec::new();
        let mut at = 0;
        while at < spans.len() {
            if !unspaced_word(at) {
                words.push(folded(&spans[at]));
                at += 1;
                continue;
            }
            let mut end = at + 1;
            while end < spans.len() && spans[end].start == spans[end - 1].end && unspaced_word(end)
            {
                end += 1;
            }
            let run = &spans[at..end];
            let read = runs::read(
                query,
                run,
                ending && end == spans.len(),
                |word| index.entry_of(word).is_some(),
                |word| index.starts_word(word),
            );
            words.extend(read.unwrap_or_else(|| run.iter().map(folded).collect()));
            at = end;
        }
        let typed = (words.last())
            .filter(|_| ending)
            .filter(|word| word.chars().nth(1).is_some() || word.chars().any(unspaced))
            .cloned();
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
pub(crate) fn in_utf16(a: &str, b: &str) -> std::cmp::Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}
