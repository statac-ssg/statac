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
//!   the records in order, and each record's length.
//! - `words.*.bin`: a run of the language's words in the order of their
//!   UTF-16 code units, each with its stem's entry among the piece's, and
//!   each entry's records as gaps from the last, each with its weight.
//!   `index.js` gives the first word of each.
//! - `pages/*.bin`: one page's address, its name as a list gives it, its
//!   date, the date as the site writes it where the page isn't called by
//!   it, and its parts, each its heading's id and words, none for the
//!   opening, and its text.

use std::collections::HashMap;
use std::fmt::Write as _;

use rayon::prelude::*;

use super::{Index, Kept};
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
            packed("pages/", &json, kept)
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
    let records = packed("records.", &json, kept);

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
    // Cut first on how far such lists usually pack, and again on how far
    // this one did where that is far out.
    let mut packs = PACKS;
    for again in [false, true] {
        let (cuts, written) = cuts(index, (each as f64 * packs) as usize);
        let ranges: Vec<std::ops::Range<usize>> = cuts.windows(2).map(|w| w[0]..w[1]).collect();
        let pieces: Vec<(usize, File)> = (ranges.into_par_iter())
            .map(|range| (range.start, packed("words.", &piece(index, range), kept)))
            .collect();
        let size: usize = pieces.iter().map(|(_, file)| file.bytes.len()).sum();
        if size <= whole {
            let all = piece(index, 0..index.words.len());
            return vec![(0, packed("words.", &all, kept))];
        }
        let measured = written as f64 / size as f64;
        if again || (measured / packs - 1.0).abs() < 0.15 {
            return pieces;
        }
        packs = measured;
    }
    unreachable!("the second cut is kept")
}

/// How many times smaller a word list's text, as [`cuts`] counts it,
/// usually is packed.
const PACKS: f64 = 3.5;

/// Where to cut the word list for pieces of about `most` bytes of text,
/// as counted here, from the first word to past the last, and how long
/// the whole is counted.
fn cuts(index: &Index, most: usize) -> (Vec<usize>, usize) {
    let mut cuts: Vec<usize> = vec![0];
    let (mut size, mut written) = (0, 0);
    let mut held: std::collections::HashSet<u32> = std::collections::HashSet::new();
    for (at, (word, entry)) in index.words.iter().enumerate() {
        let listed = word.len() + 10;
        let records = written_length(&index.entries[*entry as usize]);
        let adds = listed + if held.contains(entry) { 0 } else { records };
        if size > 0 && size + adds > most {
            cuts.push(at);
            written += size;
            size = 0;
            held.clear();
        }
        size += listed;
        if held.insert(*entry) {
            size += records;
        }
    }
    cuts.push(index.words.len());
    (cuts, written + size)
}

/// About how long an entry's records are written.
fn written_length(records: &[(u32, u32)]) -> usize {
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
        let pairs = index.entries[entry as usize]
            .iter()
            .flat_map(|&(record, weight)| {
                let gap = record - last;
                last = record;
                [gap, weight]
            });
        numbers(pairs, &mut json);
    }
    json.push_str("]}");
    json
}

/// `json` packed, at `before` followed by its mark and `.bin`: as `kept`
/// keeps it, where an earlier build packed the same.
fn packed(before: &str, json: &str, kept: &Kept) -> File {
    kept.packed(before, json, || {
        let bytes = pack(json.as_bytes());
        let mark = &blake3::hash(&bytes).to_hex()[..MARK];
        File {
            path: format!("{before}{mark}.bin"),
            bytes,
            named: true,
        }
    })
}

/// `bytes` packed with zlib.
fn pack(bytes: &[u8]) -> Vec<u8> {
    miniz_oxide::deflate::compress_to_vec_zlib(bytes, 6)
}

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
