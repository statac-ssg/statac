//! Why a record scores what it does for what was asked: for each word,
//! the forms of it the record holds, where they stand and what each
//! counts there, and how rare the word is among the records.

use std::collections::{BTreeMap, HashMap};

use super::{Asked, Index, Place, Said, Times, in_utf16, placed};
use crate::site::Site;

/// Why a record scores what it does.
#[derive(Debug, PartialEq)]
pub(crate) struct Why {
    /// The length of its text, in words, and the records' average.
    pub(crate) length: u32,
    pub(crate) average: f64,
    /// The length of its page's title, in words, and the average of the
    /// records whose page has one.
    pub(crate) title: u32,
    pub(crate) title_average: f64,
    /// What each word asked counted, in the order of the words.
    pub(crate) words: Vec<Counted>,
}

/// What one word asked counted in a record.
#[derive(Debug, PartialEq)]
pub(crate) struct Counted {
    pub(crate) word: String,
    /// The forms of it the record holds, folded, each once, in the order
    /// of their UTF-16 code units; none where it holds none.
    pub(crate) forms: Vec<String>,
    /// Whether they matched as the start of a longer word, which counts
    /// less than a whole one.
    pub(crate) starting: bool,
    /// Where the forms stand, in the order of the places.
    pub(crate) places: Vec<Stands>,
    /// How many records hold any form the word finds.
    pub(crate) holding: usize,
    /// What it adds to the record's score.
    pub(crate) score: f64,
}

/// How many times a word's forms stand in one place in a record, and what
/// they add to its score there.
#[derive(Debug, PartialEq)]
pub(crate) struct Stands {
    pub(crate) place: Place,
    pub(crate) times: u32,
    pub(crate) adds: f64,
}

impl Index {
    /// Why the record at `record`, of a page of `site`, scores what
    /// [`Index::search`] gives it for `asked`: what each word adds sums to
    /// its score.
    pub(crate) fn why(&self, site: &Site, record: usize, asked: &Asked) -> Why {
        // The record's words by their entries, each where it stands.
        let mut held: HashMap<u32, Vec<(String, Place)>> = HashMap::new();
        let mut length = 0;
        let mut title = 0;
        let said = Said::of(site, self.records[record].page);
        placed(&said, &self.records[record], |word, place| {
            match place {
                Place::Text => length += 1,
                Place::Title => title += 1,
                _ => {}
            }
            if let Some(entry) = self.entry_of(word) {
                held.entry(entry)
                    .or_default()
                    .push((word.to_owned(), place));
            }
        });
        let at = u32::try_from(record).expect("fewer records than u32 holds");
        let looked = self.looked_in(asked);
        let words = (asked.words.iter().zip(looked))
            .map(|(word, looked)| {
                let holding = self.holding(&looked);
                let rarity = self.rarity(holding);
                // The entry scoring most here, as search keeps it.
                let mut best: Option<(u32, Times, f64, f64)> = None;
                for &(entry, counts) in &looked {
                    let records = &self.entries[entry as usize];
                    let Ok(found) = records.binary_search_by_key(&at, |&(record, _)| record) else {
                        continue;
                    };
                    let times = records[found].1;
                    let score = self.scored(rarity, times, at) * counts;
                    if best.is_none_or(|(.., kept)| score > kept) {
                        best = Some((entry, times, counts, score));
                    }
                }
                let Some((entry, times, counts, score)) = best else {
                    return Counted {
                        word: word.clone(),
                        forms: Vec::new(),
                        starting: false,
                        places: Vec::new(),
                        holding,
                        score: 0.0,
                    };
                };
                let standing = held.get(&entry).map(Vec::as_slice).unwrap_or_default();
                let mut forms: Vec<String> =
                    standing.iter().map(|(form, ..)| form.clone()).collect();
                forms.sort_unstable_by(|a, b| in_utf16(a, b));
                forms.dedup();
                let mut places: BTreeMap<Place, u32> = BTreeMap::new();
                for &(_, place) in standing {
                    *places.entry(place).or_default() += 1;
                }
                Counted {
                    word: word.clone(),
                    forms,
                    starting: counts < 1.0,
                    places: (places.into_iter())
                        .map(|(place, held)| Stands {
                            place,
                            times: held,
                            adds: rarity * self.counted(times, at, place) * counts,
                        })
                        .collect(),
                    holding,
                    score,
                }
            })
            .collect();
        Why {
            length,
            average: self.average,
            title,
            title_average: self.title_average,
            words,
        }
    }
}
