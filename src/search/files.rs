//! The files a reader's browser searches the site with, under `search/`.
//!
//! `index.json` is read first and leads to the rest, which are named by
//! their content, so a browser may keep each for good and a build leaves
//! every file whose content is unchanged as it was. Those are JSON packed
//! with zlib, which the browser's `DecompressionStream("deflate")`
//! unpacks, so no host has to be set to compress them; they end in `.bin`,
//! not `.gz`, which some hosts send marked as packed for the browser to
//! unpack itself.
//!
//! - `records.*.bin`: each page's text file and how many records it has,
//!   the records in order, and each record's length.
//! - `words.*.bin`: a run of the site's words in the order of their UTF-16
//!   code units, each with the entries of its stems among the piece's,
//!   and each entry's records as gaps from the last, each with its weight.
//!   `index.json` gives the first word of each.
//! - `pages/*.bin`: one page's address, title, date and parts, each part
//!   its heading's id and words, none for the opening, and its text.

use std::collections::HashMap;
use std::fmt::Write as _;

use rayon::prelude::*;

use super::Index;
use crate::problem::Named;
use crate::templates::Renderer;

/// The folder the files are published in.
pub const FOLDER: &str = "search";

/// The file read first, by its path from the top of the built site.
pub const ENTRY: &str = "search/index.json";

/// The version of the files' format, for the script reading them.
const FORMAT: u32 = 1;

/// The word list stays one file up to this size packed, and is cut into
/// pieces of about `PIECE` packed otherwise.
const WHOLE: usize = 120 * 1024;
const PIECE: usize = 22 * 1024;

/// How many hex digits of a file's hash name it: enough that thousands of
/// pages' files never share a name.
const MARK: usize = 16;

/// One file, by its path from the top of the built site.
#[derive(Debug, PartialEq)]
pub struct File {
    pub path: String,
    pub bytes: Vec<u8>,
}

/// Every file of `index`, the site `renderer` read, `index.json` first.
pub fn files(renderer: &Renderer, index: &Index) -> Vec<File> {
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
            json.push_str(",\"title\":");
            match &page.title {
                Some(title) => string(&title.value, &mut json),
                None => json.push_str("null"),
            }
            json.push_str(",\"date\":");
            match &page.date {
                Some(date) => string(&date.value.iso_day(), &mut json),
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
            packed(&format!("{FOLDER}/pages/"), &json)
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
    let records = packed(&format!("{FOLDER}/records."), &json);

    let pieces = pieces(index, WHOLE, PIECE);
    let mut entry = format!("{{\"format\":{FORMAT},\"records\":");
    string(name_in_folder(&records.path), &mut entry);
    entry.push_str(",\"words\":[");
    for (i, (first, piece)) in pieces.iter().enumerate() {
        if i > 0 {
            entry.push(',');
        }
        entry.push('[');
        string(first, &mut entry);
        entry.push(',');
        string(name_in_folder(&piece.path), &mut entry);
        entry.push(']');
    }
    entry.push_str("]}\n");

    let mut files = vec![File {
        path: ENTRY.to_owned(),
        bytes: entry.into_bytes(),
    }];
    files.push(records);
    files.extend(pieces.into_iter().map(|(_, piece)| piece));
    files.extend(texts);
    files
}

/// What each file claims of the site's addresses.
pub fn claims(files: &[File]) -> Vec<(String, Named)> {
    (files.iter())
        .map(|file| (file.path.clone(), Named::Search))
        .collect()
}

/// The path of a file under `search/` from there.
fn name_in_folder(path: &str) -> &str {
    &path[FOLDER.len() + 1..]
}

/// The word list in pieces, each with its first word, the first piece's
/// empty, so that every word falls in the last piece whose first word it
/// doesn't come before. The list stays whole up to `whole` bytes packed,
/// and is otherwise cut into pieces of about `each` packed.
pub(super) fn pieces(index: &Index, whole: usize, each: usize) -> Vec<(String, File)> {
    // Cut first on how far such lists usually pack, and again on how far
    // this one did where that is far out.
    let mut packs = PACKS;
    for again in [false, true] {
        let (cuts, written) = cuts(index, (each as f64 * packs) as usize);
        let ranges: Vec<std::ops::Range<usize>> = cuts.windows(2).map(|w| w[0]..w[1]).collect();
        let pieces: Vec<(String, File)> = (ranges.into_par_iter())
            .map(|range| {
                let first = match range.start {
                    0 => String::new(),
                    at => index.words[at].0.clone(),
                };
                (first, named("words.", &piece(index, range)))
            })
            .collect();
        let packed: usize = pieces.iter().map(|(_, file)| file.bytes.len()).sum();
        if packed <= whole {
            let all = piece(index, 0..index.words.len());
            return vec![(String::new(), named("words.", &all))];
        }
        let measured = written as f64 / packed as f64;
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
    for (at, (word, entries)) in index.words.iter().enumerate() {
        let listed = word.len() + 4 + 6 * entries.len();
        let new = entries.iter().filter(|entry| !held.contains(entry));
        let adds = listed
            + new
                .map(|&entry| written_length(&index.entries[entry as usize]))
                .sum::<usize>();
        if size > 0 && size + adds > most {
            cuts.push(at);
            written += size;
            size = 0;
            held.clear();
        }
        size += listed;
        for &entry in entries {
            if held.insert(entry) {
                size += written_length(&index.entries[entry as usize]);
            }
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
/// entries as their places among the piece's, and each entry's records.
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
    json.push_str("],\"stems\":[");
    for (i, (_, entries)) in words.iter().enumerate() {
        if i > 0 {
            json.push(',');
        }
        let places = entries.iter().map(|&entry| {
            let next = order.len();
            let place = *local.entry(entry).or_insert(next);
            if place == next {
                order.push(entry);
            }
            u32::try_from(place).expect("fewer stems than u32 holds")
        });
        numbers(places.collect::<Vec<_>>().into_iter(), &mut json);
    }
    json.push_str("],\"records\":[");
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

/// `json` packed and named by its packed content under `search/`, after
/// `stem`.
fn named(stem: &str, json: &str) -> File {
    packed(&format!("{FOLDER}/{stem}"), json)
}

/// `json` packed, at `before` followed by its mark and `.bin`.
fn packed(before: &str, json: &str) -> File {
    let bytes = pack(json.as_bytes());
    let mark = &blake3::hash(&bytes).to_hex()[..MARK];
    File {
        path: format!("{before}{mark}.bin"),
        bytes,
    }
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
