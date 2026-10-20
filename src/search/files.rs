//! The files a reader's browser searches the site with: for each language,
//! the pages of that language, under `search/` at the language's top.
//!
//! `index.js` is the script that reads the rest, written with their names,
//! so the script and the index always come from one build. The rest are
//! named by their content, so a browser may keep each for good and a build
//! leaves every file whose content is unchanged as it was. They are JSON
//! packed with zlib, which the browser's `DecompressionStream("deflate")`
//! unpacks, so no host has to be set to compress them; they end in `.bin`,
//! not `.gz`, which some hosts send marked as packed for the browser to
//! unpack itself.
//!
//! - `records.*.bin`: each page's text file and how many records it has,
//!   the records in order, and the length of each record's text.
//! - `words.*.bin`: a run of the language's words in the order of their
//!   UTF-16 code units, each with its stem's entry among the piece's, and
//!   each entry's records as gaps from the last, each with its weight in
//!   the record's text and its weight everywhere else: the gap times four,
//!   plus 1 where the weight in its text alone follows, 2 where the rest
//!   alone does, or 3 where both do, the text's first.
//!   `index.js` gives the first word of each.
//! - `pages/*.bin`: one page's address, its name as a list gives it, its
//!   date, the date as the site writes it where the page isn't called by
//!   it, and its parts, each its heading's id and words, none for the
//!   part before the first heading, and its text.

use std::collections::HashMap;
use std::fmt::Write as _;

use rayon::prelude::*;

use super::{Index, Kept, Weights};
use crate::content::address::Top;
use crate::problem::Named;
use crate::site::{LanguageId, Site};
use crate::templates::Renderer;

/// The folder each language's files are published in, at its top.
const FOLDER: &str = "search";

/// The script reading the index, by its path in the folder.
const ENTRY: &str = "index.js";

/// The folder `language`'s files are published in, by its path from the
/// top of the built site: `search`, or `fr/search` beneath a language's
/// folder.
pub fn folder(site: &Site, language: LanguageId) -> String {
    let top = Top::new(site.top(language)).address("/");
    format!("{}{FOLDER}", top.trim_start_matches('/'))
}

/// The script reading `language`'s index, by its path from the top of the
/// built site.
pub fn entry(site: &Site, language: LanguageId) -> String {
    format!("{}/{ENTRY}", folder(site, language))
}

/// The script, which the names of the index's files are written before.
const READER: &str = include_str!("reader.js");

/// The word list stays one file up to this size packed, and is cut into
/// pieces of about `PIECE` packed otherwise.
const WHOLE: usize = 120 * 1024;
const PIECE: usize = 22 * 1024;

/// How many hex digits of a file's hash name it: enough that thousands of
/// pages' files never share a name.
const MARK: usize = 16;

/// One file, by its path from the top of the built site.
#[derive(Clone, Debug, PartialEq)]
pub struct File {
    pub path: String,
    pub bytes: Vec<u8>,
    /// Whether its name says what it holds.
    pub named: bool,
}

/// Every file of `index`, the site `renderer` read, in `folder`, the
/// script first, each taken from `kept` where an earlier build packed the
/// same.
pub(super) fn files(renderer: &Renderer, folder: &str, index: &Index, kept: &Kept) -> Vec<File> {
    let site = renderer.site();
    let base = renderer.settings().base();
    // Each page's records, which stand together.
    let mut pages: Vec<(usize, usize)> = Vec::new();
    for (at, record) in index.records.iter().enumerate() {
        match pages.last_mut() {
            Some((first, count)) if index.records[*first].page == record.page => *count += 1,
            _ => pages.push((at, 1)),
        }
    }
    let texts: Vec<File> = (pages.par_iter())
        .map(|&(first, count)| {
            let records = &index.records[first..first + count];
            let page = site.pages.get(records[0].page);
            let mut json = String::from("{\"url\":");
            string(&format!("{base}{}", page.address.as_str()), &mut json);
            let (title, day) = renderer.named(records[0].page);
            json.push_str(",\"title\":");
            string(&title, &mut json);
            json.push_str(",\"date\":");
            match &page.date {
                Some(date) => string(&date.value.iso_day(), &mut json),
                None => json.push_str("null"),
            }
            json.push_str(",\"day\":");
            match &day {
                Some(day) => string(day, &mut json),
                None => json.push_str("null"),
            }
            json.push_str(",\"parts\":[");
            for (i, record) in records.iter().enumerate() {
                if i > 0 {
                    json.push(',');
                }
                json.push('[');
                match &record.heading {
                    Some((id, words)) => {
                        string(id, &mut json);
                        json.push(',');
                        string(words, &mut json);
                    }
                    None => json.push_str("null,null"),
                }
                json.push(',');
                string(&record.text, &mut json);
                json.push(']');
            }
            json.push_str("]}");
            packed("pages/", &json, TEXT, kept)
        })
        .collect();

    let mut json = String::from("{\"pages\":[");
    for (i, (text, &(_, count))) in texts.iter().zip(&pages).enumerate() {
        if i > 0 {
            json.push(',');
        }
        let name = text.path.rsplit('/').next().unwrap_or_default();
        write!(json, "[\"{name}\",{count}]").expect("writing to a string can't fail");
    }
    json.push_str("],\"lengths\":");
    numbers(index.lengths.iter().copied(), &mut json);
    json.push('}');
    let records = packed("records.", &json, TEXT, kept);

    let pieces = pieces(index, WHOLE, PIECE, kept);
    let mut entry = String::from("const FILES = {\"records\":");
    string(&records.path, &mut entry);
    entry.push_str(",\"words\":[");
    for (i, (start, piece)) in pieces.iter().enumerate() {
        if i > 0 {
            entry.push(',');
        }
        // The first piece's first word is empty, so that it holds every
        // word that comes before the list's first.
        let first = match start {
            0 => "",
            &at => index.words[at].0.as_str(),
        };
        entry.push('[');
        string(first, &mut entry);
        entry.push(',');
        string(&piece.path, &mut entry);
        entry.push(']');
    }
    entry.push_str("]};\n\n");
    entry.push_str(READER);

    let mut files = vec![File {
        path: ENTRY.to_owned(),
        bytes: entry.into_bytes(),
        named: false,
    }];
    files.push(records);
    files.extend(pieces.into_iter().map(|(_, piece)| piece));
    files.extend(texts);
    for file in &mut files {
        file.path = format!("{folder}/{}", file.path);
    }
    files
}

/// What each file claims of the site's addresses.
pub fn claims(files: &[File]) -> Vec<(String, Named)> {
    (files.iter())
        .map(|file| (file.path.clone(), Named::Search))
        .collect()
}

/// Where each piece of the word list a reader's browser is given starts,
/// by its first word's place in the list.
pub(super) fn starts(index: &Index) -> Vec<usize> {
    let pieces = pieces(index, WHOLE, PIECE, &Kept::default()).into_iter();
    pieces.map(|(start, _)| start).collect()
}

/// The word list in pieces, each with its first word's place in the list.
/// A word falls in the last piece whose first word it doesn't come
/// before, or in the first. The list stays whole up to `whole` bytes
/// packed, and is otherwise cut into pieces of about `each` packed. Each
/// is taken from `kept` where an earlier build packed the same.
pub(super) fn pieces(index: &Index, whole: usize, each: usize, kept: &Kept) -> Vec<(usize, File)> {
    let pack = |range: std::ops::Range<usize>| {
        (
            range.start,
            packed("words.", &piece(index, range), WORDS, kept),
        )
    };
    // Cut first on how far such lists usually pack, and measure how far
    // this one does on a few of the pieces, spread through it.
    let (first, sizes) = cuts(index, (each as f64 * PACKS) as usize);
    let step = sizes.len().div_ceil(SAMPLED);
    let sampled: Vec<(usize, File)> = (first.par_windows(2).step_by(step))
        .map(|w| pack(w[0]..w[1]))
        .collect();
    let counted: usize = sizes.iter().step_by(step).sum();
    let packed_to: usize = sampled.iter().map(|(_, file)| file.bytes.len()).sum();
    let measured = counted as f64 / packed_to.max(1) as f64;
    let written: usize = sizes.iter().sum();
    // Packed whole where it likely fits, and kept so where it does.
    if written as f64 / measured <= whole as f64 * 1.25 {
        let all = packed("words.", &piece(index, 0..index.words.len()), WORDS, kept);
        if all.bytes.len() <= whole {
            return vec![(0, all)];
        }
    }
    if (measured / PACKS - 1.0).abs() < 0.15 {
        let rest: Vec<(usize, File)> = (first.par_windows(2).enumerate())
            .filter(|(i, _)| i % step != 0)
            .map(|(_, w)| pack(w[0]..w[1]))
            .collect();
        let (mut sampled, mut rest) = (sampled.into_iter(), rest.into_iter());
        let pieces = (0..sizes.len()).map(|i| match i % step {
            0 => sampled.next(),
            _ => rest.next(),
        });
        return pieces
            .map(|piece| piece.expect("every piece is packed once"))
            .collect();
    }
    let (again, _) = cuts(index, (each as f64 * measured) as usize);
    (again.par_windows(2)).map(|w| pack(w[0]..w[1])).collect()
}

/// How many times smaller a word list's text, as [`cuts`] counts it,
/// usually is packed.
const PACKS: f64 = 3.7;

/// How many pieces of the word list are packed to measure how far it
/// packs.
const SAMPLED: usize = 8;

/// Where to cut the word list for pieces of about `most` bytes of text,
/// as counted here, from the first word to past the last, and how long
/// each piece is counted.
fn cuts(index: &Index, most: usize) -> (Vec<usize>, Vec<usize>) {
    let mut cuts: Vec<usize> = vec![0];
    let (mut size, mut sizes) = (0, Vec::new());
    let mut held: std::collections::HashSet<u32> = std::collections::HashSet::new();
    for (at, (word, entry)) in index.words.iter().enumerate() {
        let listed = word.len() + 10;
        let records = written_length(&index.entries[*entry as usize]);
        let adds = listed + if held.contains(entry) { 0 } else { records };
        if size > 0 && size + adds > most {
            cuts.push(at);
            sizes.push(size);
            size = 0;
            held.clear();
        }
        size += listed;
        if held.insert(*entry) {
            size += records;
        }
    }
    cuts.push(index.words.len());
    sizes.push(size);
    (cuts, sizes)
}

/// About how long an entry's records are written.
fn written_length(records: &[(u32, Weights)]) -> usize {
    records.len() * 6 + 2
}

/// The words of `index` in `range`, as JSON: the words, each word's
/// entry as its place among the piece's, and each entry's records.
fn piece(index: &Index, range: std::ops::Range<usize>) -> String {
    let words = &index.words[range];
    let mut local: HashMap<u32, usize> = HashMap::new();
    let mut order: Vec<u32> = Vec::new();
    let mut json = String::from("{\"words\":[");
    for (i, (word, _)) in words.iter().enumerate() {
        if i > 0 {
            json.push(',');
        }
        string(word, &mut json);
    }
    json.push_str("],\"stems\":");
    let places = words.iter().map(|&(_, entry)| {
        let next = order.len();
        let place = *local.entry(entry).or_insert(next);
        if place == next {
            order.push(entry);
        }
        u32::try_from(place).expect("fewer stems than u32 holds")
    });
    numbers(places.collect::<Vec<_>>().into_iter(), &mut json);
    json.push_str(",\"records\":[");
    for (i, &entry) in order.iter().enumerate() {
        if i > 0 {
            json.push(',');
        }
        let mut last = 0;
        let records = index.entries[entry as usize]
            .iter()
            .flat_map(|&(record, weights)| {
                let gap = record - last;
                last = record;
                let gap = gap
                    .checked_mul(4)
                    .expect("fewer records than u32 holds a quarter of");
                match (weights.text, weights.rest) {
                    (text, 0) => vec![gap + 1, text],
                    (0, rest) => vec![gap + 2, rest],
                    (text, rest) => vec![gap + 3, text, rest],
                }
            });
        numbers(records, &mut json);
    }
    json.push_str("]}");
    json
}

/// `json` packed at `level`, at `before` followed by its mark and `.bin`:
/// as `kept` keeps it, where an earlier build packed the same.
fn packed(before: &str, json: &str, level: u8, kept: &Kept) -> File {
    kept.packed(before, json, || {
        let bytes = miniz_oxide::deflate::compress_to_vec_zlib(json.as_bytes(), level);
        let mark = &blake3::hash(&bytes).to_hex()[..MARK];
        File {
            path: format!("{before}{mark}.bin"),
            bytes,
            named: true,
        }
    })
}

/// How hard zlib packs a page's text and the records, and the word list.
/// The word list is mostly numbers, which pack within a few per cent as
/// far for a quarter of the work, and a reader fetches it in pieces of
/// one size however far it packs.
const TEXT: u8 = 6;
const WORDS: u8 = 3;

/// `numbers` as a JSON array written to `out`.
fn numbers(numbers: impl Iterator<Item = u32>, out: &mut String) {
    out.push('[');
    for (i, number) in numbers.enumerate() {
        if i > 0 {
            out.push(',');
        }
        write!(out, "{number}").expect("writing to a string can't fail");
    }
    out.push(']');
}

/// `text` as a JSON string written to `out`, escaping only what JSON
/// needs: these files are read only as JSON, never inside a page.
fn string(text: &str, out: &mut String) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if u32::from(c) < 0x20 => {
                write!(out, "\\u{:04x}", u32::from(c)).expect("writing to a string can't fail")
            }
            c => out.push(c),
        }
    }
    out.push('"');
}
