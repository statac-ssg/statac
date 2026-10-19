//! Why a record scores what it does for what was asked: for each word,
//! the forms of it the record holds, where they stand and what each
//! counts there, and how rare the word is among the records.

use std::collections::{BTreeMap, HashMap, HashSet};

use super::{Asked, Index, Place, in_utf16, placed};
use crate::site::Site;

/// Why a record scores what it does.
#[derive(Debug, PartialEq)]
pub struct Why {
    /// Its length, its words counted by weight, and the records' average.
    pub length: u32,
    pub average: f64,
    /// What each word asked counted, in the order of the words.
    pub words: Vec<Counted>,
}

/// What one word asked counted in a record.
#[derive(Debug, PartialEq)]
pub struct Counted {
    pub word: String,
    /// The forms of it the record holds, folded, each once, in the order
    /// of their UTF-16 code units; none where it holds none.
    pub forms: Vec<String>,
    /// Whether they matched as the start of a longer word, which counts
    /// less than a whole one.
    pub starting: bool,
    /// Where the forms stand, in the order of the places.
    pub places: Vec<Stands>,
    /// How many records hold the forms matched; where the record holds
    /// none, how many hold any form the word finds.
    pub holding: usize,
    /// What it adds to the record's score.
    pub score: f64,
}

/// How many times a word's forms stand in one place in a record, and what
/// they count there.
#[derive(Debug, PartialEq)]
pub struct Stands {
    pub place: Place,
    pub times: u32,
    /// The times by the place's weight.
    pub counts: u32,
}

impl Index {
    /// Why the record at `record`, of a page of `site`, scores what
    /// [`Index::search`] gives it for `asked`: what each word adds sums to
    /// its score.
    pub fn why(&self, site: &Site, record: usize, asked: &Asked) -> Why {
        // The record's words by their entries, each where it stands.
        let mut held: HashMap<u32, Vec<(String, Place, u32)>> = HashMap::new();
        let mut length = 0;
        placed(site, &self.records[record], |word, place, weight| {
            length += weight;
            if let Some(entry) = self.entry_of(&word) {
                held.entry(entry).or_default().push((word, place, weight));
            }
        });
        let at = u32::try_from(record).expect("fewer records than u32 holds");
        let looked = self.looked_in(asked);
        let words = (asked.words.iter().zip(looked))
            .map(|(word, looked)| {
                // The entry scoring most here, as search keeps it.
                let mut best: Option<(u32, f64, f64)> = None;
                for &(entry, counts) in &looked {
                    let records = &self.entries[entry as usize];
                    let Ok(found) = records.binary_search_by_key(&at, |&(record, _)| record) else {
                        continue;
                    };
                    let rarity = self.rarity(records.len());
                    let score = self.scored(rarity, records[found].1, at) * counts;
                    if best.is_none_or(|(_, _, kept)| score > kept) {
                        best = Some((entry, counts, score));
                    }
                }
                let Some((entry, counts, score)) = best else {
                    let holding: HashSet<u32> = (looked.iter())
                        .flat_map(|&(entry, _)| &self.entries[entry as usize])
                        .map(|&(record, _)| record)
                        .collect();
                    return Counted {
                        word: word.clone(),
                        forms: Vec::new(),
                        starting: false,
                        places: Vec::new(),
                        holding: holding.len(),
                        score: 0.0,
                    };
                };
                let standing = held.get(&entry).map(Vec::as_slice).unwrap_or_default();
                let mut forms: Vec<String> =
                    standing.iter().map(|(form, ..)| form.clone()).collect();
                forms.sort_unstable_by(|a, b| in_utf16(a, b));
                forms.dedup();
                let mut places: BTreeMap<Place, (u32, u32)> = BTreeMap::new();
                for &(_, place, weight) in standing {
                    let (times, counted) = places.entry(place).or_default();
                    *times += 1;
                    *counted += weight;
                }
                Counted {
                    word: word.clone(),
                    forms,
                    starting: counts < 1.0,
                    places: (places.into_iter())
                        .map(|(place, (times, counts))| Stands {
                            place,
                            times,
                            counts,
                        })
                        .collect(),
                    holding: self.entries[entry as usize].len(),
                    score,
                }
            })
            .collect();
        Why {
            length,
            average: self.average,
            words,
        }
    }
}
