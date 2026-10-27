//! The addresses the site has published, kept in `published.txt` at the
//! top of the site and committed with it, so a build on any machine knows
//! them. A build that publishes adds each address of the author's it
//! published that the file lacks: a page from a file, an old address
//! sending readers on, a feed, and a file published as it is. Nothing but
//! the author takes one out. An address the file holds that nothing
//! answers at any more is told at its line.
//!
//! Lines opening `#` say what the file is, and are written again as
//! Statac words them. Every other line is an address from the top of the
//! site, without the site's own path: a folder ends with `/`, and anything
//! else is a file. They are sorted, so two people adding pages at once
//! seldom touch the same line.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::io;
use std::path::Path;

use crate::address::Address;
use crate::config::Settings;
use crate::problem::{Kind, Named, Problem, Reason, RecordFault};
use crate::source::{Place, Sources, Span};

/// The file's name at the top of the site.
pub(crate) const FILE: &str = "published.txt";

/// What opens the file, as Statac writes it.
const ABOUT: &str = "\
# Every URL this site has published, so Statac can say when one is no
# longer published. Statac adds to it each time it builds the site.
# Take out the line of a URL that is gone for good.
";

/// The addresses the file holds, each with where its line is.
#[derive(Clone, Default, Debug)]
pub(crate) struct Record {
    /// Whether the site has the file.
    held: bool,
    addresses: Vec<(Address, Place)>,
}

impl Record {
    /// Reads the site's file, the editor's text where it holds one, with a
    /// problem at each line that isn't an address; such a line is left
    /// out. A site without the file has published nothing anyone knows of.
    pub(crate) fn read(site: &Path, sources: &mut Sources) -> (Record, Vec<Problem>) {
        let real = std::fs::canonicalize(site).unwrap_or_else(|_| site.to_path_buf());
        let text = match sources.unsaved().read(&real, FILE) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return (Record::default(), Vec::new());
            }
            Err(error) => {
                let kind = Kind::CantRead {
                    path: FILE.into(),
                    reason: Reason::of(&error),
                };
                return (Record::default(), vec![Problem::nowhere(kind)]);
            }
        };
        let source = sources.add(FILE, text);
        let text = sources.text(source);
        let mut record = Record {
            held: true,
            addresses: Vec::new(),
        };
        let mut problems = Vec::new();
        let mut seen = BTreeSet::new();
        let mut start = 0;
        for line in text.split_inclusive('\n') {
            let at = start;
            start += line.len();
            let line = line.trim_end_matches('\n').trim_end_matches('\r');
            let written = line.trim();
            if written.is_empty() || written.starts_with('#') {
                continue;
            }
            let from = at + (line.len() - line.trim_start().len());
            let place = Place {
                source,
                span: Span::new(from, from + written.len()),
            };
            let malformed = |fault| Problem::at(Kind::MalformedRecord { fault }, place);
            let read = match written.ends_with('/') {
                true => Address::read(written),
                false => Address::read_file(written),
            };
            let address = match read {
                Ok(address) if written.starts_with('/') => address,
                _ => {
                    problems.push(malformed(RecordFault::NotAnAddress(written.into())));
                    continue;
                }
            };
            if !seen.insert(address.as_str().to_owned()) {
                problems.push(malformed(RecordFault::Twice(written.into())));
                continue;
            }
            record.addresses.push((address, place));
        }
        (record, problems)
    }

    /// A warning at the line of each address it holds that nothing in
    /// `outputs`, each published file's path from the top of the built
    /// site, stands at, nor anything the site says answers there. Where
    /// every address it holds under a folder is gone, two or more, they are
    /// one warning across their lines, under the outermost such folder: a
    /// part of the site taken away whole.
    pub(crate) fn gone(&self, outputs: &[(String, Named)], settings: &Settings) -> Vec<Problem> {
        let published: BTreeSet<&str> = outputs.iter().map(|(file, _)| file.as_str()).collect();
        let base = settings.base();
        let made_after: Vec<String> = (settings.texts("after_build_outputs").iter())
            .map(|place| place.value.trim_start_matches('/').to_owned())
            .collect();
        let served_elsewhere: Vec<String> = (settings.texts("served_elsewhere").iter())
            .map(|path| format!("/{}", path.value.trim_matches('/')))
            .collect();
        let answered = |address: &Address| {
            let file = address.file();
            let made = made_after
                .iter()
                .any(|place| match place.strip_suffix('/') {
                    Some(folder) => file == folder || file.starts_with(place.as_str()),
                    None => file == *place,
                });
            let whole = format!("{base}{}", address.as_str());
            let elsewhere = served_elsewhere.iter().any(|path| {
                (whole.strip_prefix(path.as_str()))
                    .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
            });
            published.contains(file.as_str()) || made || elsewhere
        };
        let gone: Vec<&(Address, Place)> = (self.addresses.iter())
            .filter(|(address, _)| !answered(address))
            .collect();
        let lost: HashSet<&str> = gone.iter().map(|(address, _)| address.as_str()).collect();
        let mut held: HashMap<&str, (usize, usize)> = HashMap::new();
        for (address, _) in &self.addresses {
            let lost = lost.contains(address.as_str());
            for folder in folders(address.as_str()) {
                let (all, lost_here) = held.entry(folder).or_default();
                *all += 1;
                *lost_here += usize::from(lost);
            }
        }
        let mut groups: BTreeMap<&str, Vec<&(Address, Place)>> = BTreeMap::new();
        for each in &gone {
            if let Some(folder) = whole(each.0.as_str(), &held) {
                groups.entry(folder).or_default().push(each);
            }
        }
        let mut told = Vec::new();
        for each @ (address, place) in &gone {
            let Some(folder) = whole(address.as_str(), &held) else {
                let kind = Kind::UrlGone {
                    address: format!("{base}{}", address.as_str()),
                    written: address.as_str().to_owned(),
                };
                told.push(Problem::at(kind, *place));
                continue;
            };
            let group = &groups[folder];
            // Told once, where the first of them is.
            if !std::ptr::eq(group[0], *each) {
                continue;
            }
            let start = group.iter().map(|(_, at)| at.span.start).min().unwrap_or(0);
            let end = group.iter().map(|(_, at)| at.span.end).max().unwrap_or(0);
            let kind = Kind::AddressesGone {
                under: format!("{base}{folder}"),
                written: folder.to_owned(),
                gone: group.iter().map(|(a, _)| a.as_str().to_owned()).collect(),
            };
            let place = Place {
                source: place.source,
                span: Span::new(start as usize, end as usize),
            };
            told.push(Problem::at(kind, place));
        }
        told
    }

    /// Writes the file under the site at `site` with every address it
    /// holds and each of `now` it lacks, where it lacks any, and says what
    /// it added. A site with no file yet gets one only once `now` holds
    /// an address. Where `ours` holds the
    /// addresses a serve added since its first round, one of those no
    /// longer in `now` was never published beyond it, and is taken out.
    pub(crate) fn keep(
        &self,
        site: &Path,
        now: &[Address],
        ours: Option<&mut BTreeSet<String>>,
    ) -> io::Result<Written> {
        let mut all: BTreeSet<&str> = (self.addresses.iter())
            .map(|(address, _)| address.as_str())
            .collect();
        let now: BTreeSet<&str> = now.iter().map(Address::as_str).collect();
        let mut written = Written::default();
        if let Some(ours) = ours {
            let dropped: Vec<String> = (ours.iter())
                .filter(|address| all.contains(address.as_str()) && !now.contains(address.as_str()))
                .cloned()
                .collect();
            for address in &dropped {
                all.remove(address.as_str());
                ours.remove(address);
            }
            ours.extend(
                (now.iter())
                    .filter(|address| !all.contains(*address))
                    .map(|a| (*a).to_owned()),
            );
            written.dropped = dropped;
        }
        let before = all.len();
        all.extend(now);
        if !self.held && all.is_empty() {
            return Ok(written);
        }
        written.kept = Kept {
            added: all.len() - before,
            started: !self.held,
        };
        if self.held && written.kept.added == 0 && written.dropped.is_empty() {
            return Ok(written);
        }
        let mut text = ABOUT.to_owned();
        let opening = ABOUT.lines().count();
        for (at, address) in all.into_iter().enumerate() {
            text.push_str(address);
            text.push('\n');
            written.lines.insert(address.to_owned(), opening + at + 1);
        }
        crate::platform::keep_file(site, FILE, &crate::platform::beside(FILE), text.as_bytes())?;
        written.hash = Some(blake3::hash(text.as_bytes()));
        Ok(written)
    }
}

/// The outermost folder `address` is in, or is, under which `held` has
/// two addresses or more kept, all of them gone, by how many it keeps
/// and how many are gone.
fn whole<'a>(address: &'a str, held: &HashMap<&str, (usize, usize)>) -> Option<&'a str> {
    folders(address).find(|folder| {
        let (all, lost) = held[folder];
        all >= 2 && lost == all
    })
}

/// Each folder `address` is in, or is, from the outermost: `/blog/` and
/// `/blog/2026/` of `/blog/2026/post-1/`. None for a file at the top.
fn folders(address: &str) -> impl Iterator<Item = &str> {
    (address.match_indices('/').skip(1)).map(move |(at, _)| &address[..=at])
}

/// What a build added to the file.
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub(crate) struct Kept {
    /// How many addresses.
    pub(crate) added: usize,
    /// Whether the build made the file.
    pub(crate) started: bool,
}

/// What writing the file came to.
#[derive(Clone, Default, Debug)]
pub(crate) struct Written {
    pub(crate) kept: Kept,
    /// The addresses a serve session took out, as the file writes them.
    pub(crate) dropped: Vec<String>,
    /// The hash of what was written, where the file was written.
    pub(crate) hash: Option<blake3::Hash>,
    /// The line each address is on in what was written, counted from 1.
    pub(crate) lines: std::collections::HashMap<String, usize>,
}

impl Written {
    /// Takes its place from each warning in `problems` of an address gone
    /// that the file as written holds on another line than the one it was
    /// read at, where the file was written: the place told would be
    /// another address's.
    pub(crate) fn re_place(&self, problems: &mut [Problem], sources: &Sources) {
        if self.hash.is_none() {
            return;
        }
        for problem in problems {
            let Some(place) = problem.place else {
                continue;
            };
            let (first, last) = match &problem.kind {
                Kind::UrlGone { written, .. } => (written, written),
                Kind::AddressesGone { gone, .. } => match (gone.first(), gone.last()) {
                    (Some(first), Some(last)) => (first, last),
                    _ => continue,
                },
                _ => continue,
            };
            let line = |offset: u32| sources.line_number(place.source, offset as usize);
            if self.lines.get(first) != Some(&line(place.span.start))
                || self.lines.get(last) != Some(&line(place.span.end))
            {
                problem.place = None;
            }
        }
    }

    /// Takes out of `problems` the addresses gone that a serve let go of
    /// as it wrote the file: a warning of them alone, and each from a
    /// warning of several, which can be told then only by what remains.
    pub(crate) fn forget_dropped(&self, problems: &mut Vec<Problem>) {
        let dropped = |address: &String| self.dropped.contains(address);
        problems.retain_mut(|problem| match &mut problem.kind {
            Kind::UrlGone { written, .. } => !dropped(written),
            Kind::AddressesGone { gone, .. } => {
                let before = gone.len();
                gone.retain(|address| !dropped(address));
                if gone.len() != before {
                    problem.place = None;
                }
                !gone.is_empty()
            }
            _ => true,
        });
    }
}

#[cfg(test)]
mod tests;
