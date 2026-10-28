//! The addresses the site has published, kept in `published.txt` at the
//! top of the site and committed with it, so a build on any machine knows
//! them. A build that publishes adds each address of the author's it
//! published that the file lacks: a page from a file, an old address
//! sending readers on, a feed, and a file published as it is. Nothing but
//! the author takes one out. An address the file holds that nothing
//! answers at any more is told at its line.
//!
//! A site with no file yet whose earlier build still stands in `_site`
//! takes what that build published in the file's place, so the first
//! build to keep the file tells each address it no longer publishes, and
//! writes them all into the file. Among them may be list pages and
//! paginated pages, which a build never adds itself.
//!
//! Lines opening `#` say what the file is, and are written again as
//! Statac words them. Every other line is an address from the top of the
//! site, without the site's own path: a folder ends with `/`, and anything
//! else is a file. They are sorted, so two people adding pages at once
//! seldom touch the same line.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::io;
use std::path::Path;

use crate::address::Address;
use crate::config::Settings;
use crate::problem::{Kind, Named, Problem, Reason, RecordFault, Relation};
use crate::source::{Place, Sources, Span};

/// The file's name at the top of the site.
pub(crate) const FILE: &str = "published.txt";

/// What opens the file, as Statac writes it.
const ABOUT: &str = "\
# Every URL this site has published, so Statac can say when one is no
# longer published. Statac adds to it each time it builds the site.
# Take out the line of a URL that is gone for good.
";

/// The addresses the file holds, each with where its line is; none for
/// an address found in the site's earlier build.
#[derive(Clone, Default, Debug)]
pub(crate) struct Record {
    /// Whether the site has the file.
    held: bool,
    /// Whether the addresses were found in the site's earlier build.
    found: bool,
    addresses: Vec<(Address, Option<Place>)>,
}

impl Record {
    /// Reads the site's file, the editor's text where it holds one, with a
    /// problem at each line that isn't an address; such a line is left
    /// out. A site without the file has published what its earlier build
    /// in `_site` holds, where there is one, and otherwise nothing anyone
    /// knows of.
    pub(crate) fn read(
        site: &Path,
        sources: &mut Sources,
        settings: &Settings,
    ) -> (Record, Vec<Problem>) {
        let real = std::fs::canonicalize(site).unwrap_or_else(|_| site.to_path_buf());
        let text = match sources.unsaved().read(&real, FILE) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return (Record::from_built(site, settings), Vec::new());
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
            found: false,
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
            record.addresses.push((address, Some(place)));
        }
        (record, problems)
    }

    /// Whether its addresses were found in the site's earlier build, the
    /// site having no file.
    pub(crate) fn found(&self) -> bool {
        self.found
    }

    /// The addresses the site's earlier build in `_site` published, where
    /// Statac built the folder and it can be read whole: a folder's
    /// `index.html` as the folder, and every other file as itself. Left
    /// out are the files Statac names by what they hold, which a reader
    /// can't keep a link to, the script that reads a search index, and
    /// any file whose name the site's `exclude` lists, such as a
    /// `.DS_Store` a file browser left there.
    fn from_built(site: &Path, settings: &Settings) -> Record {
        let top = site.join(crate::build::OUT);
        let marked = fs::symlink_metadata(top.join(crate::publish::MARKER))
            .is_ok_and(|marker| marker.is_file());
        if !fs::symlink_metadata(&top).is_ok_and(|top| top.is_dir()) || !marked {
            return Record::default();
        }
        let excluded = settings.texts("exclude");
        let excluded: HashSet<&str> = (excluded.iter())
            .map(|name| name.value.as_str())
            .filter(|name| !name.contains('/'))
            .collect();
        let mut found = BTreeSet::new();
        if walk(&top, "/", &excluded, &mut found).is_err() {
            return Record::default();
        }
        found.remove(&format!("/{}", crate::publish::MARKER));
        // In the order the file keeps them.
        let addresses: BTreeMap<String, Address> = (found.iter())
            .filter_map(|written| match written.strip_suffix("index.html") {
                Some(folder) if folder.ends_with('/') => Address::read(folder).ok(),
                _ => Address::read_file(written).ok(),
            })
            .map(|address| (address.as_str().to_owned(), address))
            .collect();
        let addresses = (addresses.into_values())
            .map(|address| (address, None))
            .collect();
        Record {
            held: false,
            found: true,
            addresses,
        }
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
        let gone: Vec<&(Address, Option<Place>)> = (self.addresses.iter())
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
        let mut groups: BTreeMap<&str, Vec<&(Address, Option<Place>)>> = BTreeMap::new();
        for each in &gone {
            if let Some(folder) = whole(each.0.as_str(), &held) {
                groups.entry(folder).or_default().push(each);
            }
        }
        // With no line to place it at, a warning says where the address
        // was found.
        let told = |kind, place: Option<Place>| match self.found {
            true => Problem::nowhere(kind).met(Relation::FoundBuilt, None),
            false => Problem::placed(kind, place),
        };
        let mut warnings = Vec::new();
        for each @ (address, place) in &gone {
            let Some(folder) = whole(address.as_str(), &held) else {
                let kind = Kind::UrlGone {
                    address: format!("{base}{}", address.as_str()),
                    written: address.as_str().to_owned(),
                };
                warnings.push(told(kind, *place));
                continue;
            };
            let group = &groups[folder];
            // Told once, where the first of them is.
            if !std::ptr::eq(group[0], *each) {
                continue;
            }
            let places: Option<Vec<Place>> = group.iter().map(|(_, at)| *at).collect();
            let place = places.map(|places| {
                let start = places.iter().map(|at| at.span.start).min().unwrap_or(0);
                let end = places.iter().map(|at| at.span.end).max().unwrap_or(0);
                Place {
                    source: places[0].source,
                    span: Span::new(start as usize, end as usize),
                }
            });
            let kind = Kind::AddressesGone {
                under: format!("{base}{folder}"),
                written: folder.to_owned(),
                gone: group.iter().map(|(a, _)| a.as_str().to_owned()).collect(),
            };
            warnings.push(told(kind, place));
        }
        warnings
    }

    /// Writes the file under the site at `site` with every address it
    /// holds and each of `now` it lacks, where it lacks any, and says what
    /// it added. A site with no file yet gets one only once there is an
    /// address to write. Where `ours` holds the
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
            found: match self.found {
                true => before,
                false => 0,
            },
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

/// Adds to `found` the path from the top of the built site of each file
/// in `folder`, whose own path is `at`, and of each file in the folders
/// in it, but those [`Record::from_built`] leaves out, through no link.
fn walk(
    folder: &Path,
    at: &str,
    excluded: &HashSet<&str>,
    found: &mut BTreeSet<String>,
) -> io::Result<()> {
    let mut names = Vec::new();
    for entry in fs::read_dir(folder)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        // A name that isn't UTF-8 is no address.
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if kind.is_symlink() || excluded.contains(name.as_str()) {
            continue;
        }
        match kind.is_dir() {
            true => walk(&entry.path(), &format!("{at}{name}/"), excluded, found)?,
            false => names.push(name),
        }
    }
    let searched = (names.iter()).any(|name| name.starts_with("records.") && by_content(name));
    for name in names {
        if by_content(&name) || (searched && name == crate::site::SEARCH_ENTRY) {
            continue;
        }
        found.insert(format!("{at}{name}"));
    }
    Ok(())
}

/// Whether Statac named the file `name` by what it holds, with 8 or 16
/// hex digits before its extension or at its end: `site.1a2b3c4d.css`,
/// `photo-400.1a2b3c4d.jpg`, or a search index's `0123456789abcdef.bin`.
fn by_content(name: &str) -> bool {
    let mark = |piece: &str| {
        matches!(piece.len(), 8 | 16)
            && (piece.bytes()).all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    };
    let mut pieces = name.rsplit('.');
    match (pieces.next(), pieces.next()) {
        (Some(last), Some(before)) => mark(before) || mark(last),
        _ => false,
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
    /// How many addresses of the site's earlier build it made the file
    /// with.
    pub(crate) found: usize,
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
