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

use std::collections::{BTreeSet, HashSet};
use std::io;
use std::path::Path;

use crate::assets::Assets;
use crate::config::Settings;
use crate::content::address::Address;
use crate::feeds::Feed;
use crate::problem::{Kind, Named, Problem, Reason, RecordFault};
use crate::site::{ListKey, PageId, Site};
use crate::source::{Place, Sources, Span};

/// The file's name at the top of the site.
pub(crate) const FILE: &str = "published.txt";

/// What opens the file, as Statac writes it.
const ABOUT: &str = "\
# Every address this site has published, so Statac can say when one
# stops answering. Statac adds to it each time it builds the site.
# Take out the line of an address that is gone for good.
";

/// The addresses the file holds, each with where its line is.
#[derive(Clone, Default, Debug)]
pub struct Record {
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
    /// site, stands at, nor anything the site says answers there.
    pub(crate) fn gone(&self, outputs: &[(String, Named)], settings: &Settings) -> Vec<Problem> {
        let published: BTreeSet<&str> = outputs.iter().map(|(file, _)| file.as_str()).collect();
        let base = settings.base();
        let made_after: Vec<String> = (settings.texts("after_build_makes").iter())
            .map(|place| place.value.trim_start_matches('/').to_owned())
            .collect();
        let not_ours: Vec<String> = (settings.texts("not_ours").iter())
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
            let elsewhere = not_ours.iter().any(|path| {
                (whole.strip_prefix(path.as_str()))
                    .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
            });
            published.contains(file.as_str()) || made || elsewhere
        };
        (self.addresses.iter())
            .filter(|(address, _)| !answered(address))
            .map(|(address, place)| {
                let kind = Kind::AddressGone {
                    address: format!("{base}{}", address.as_str()),
                    written: address.as_str().to_owned(),
                };
                Problem::at(kind, *place)
            })
            .collect()
    }

    /// Writes the file under the site at `site` with every address it
    /// holds and each of `now` it lacks, where it lacks any or the site
    /// has no file yet, and says what it added.
    pub(crate) fn keep(&self, site: &Path, now: &[Address]) -> io::Result<Kept> {
        let mut all: BTreeSet<&str> = (self.addresses.iter())
            .map(|(address, _)| address.as_str())
            .collect();
        let before = all.len();
        all.extend(now.iter().map(Address::as_str));
        let added = all.len() - before;
        if self.held && added == 0 {
            return Ok(Kept::default());
        }
        let mut text = ABOUT.to_owned();
        for address in all {
            text.push_str(address);
            text.push('\n');
        }
        crate::platform::keep_file(site, FILE, &crate::platform::beside(FILE), text.as_bytes())?;
        Ok(Kept {
            added,
            started: !self.held,
        })
    }
}

/// What a build added to the file.
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct Kept {
    /// How many addresses.
    pub added: usize,
    /// Whether the build made the file.
    pub started: bool,
}

/// Every address of the author's the site at `root` publishes, besides
/// the files beside the writing in `held_back`, each its path in the built
/// site, which no page links: each page from a file, each old address
/// sending readers on, the feed of each folder carrying a page of theirs,
/// and each file published as it is. A list's page Statac made, its pages
/// after the first, and a tag's or a series' feed come and go with what is
/// listed, and the site's assets are named by what they hold, so none of
/// them is kept; nor is a page or a file as `statac new` wrote it, which
/// the author may clear away before publishing anything.
pub(crate) fn now(
    root: &Path,
    site: &Site,
    sources: &Sources,
    assets: &Assets,
    feeds: &[Feed],
    held_back: &[String],
) -> Vec<Address> {
    let untouched = |path: &str, bytes: &dyn Fn() -> Option<Vec<u8>>| {
        crate::new::update::writes(path)
            && bytes().is_some_and(|bytes| crate::new::update::untouched(path, &bytes))
    };
    let on_disk = |path: &str| crate::platform::read_file(root, path).ok().flatten();
    let theirs: HashSet<PageId> = (site.pages.iter())
        .filter(|(_, page)| page.lands())
        .filter(|(_, page)| {
            let Some(source) = page.source else {
                return false;
            };
            let path = sources.path(source).to_string_lossy().replace('\\', "/");
            let text = || Some(sources.text(source).as_bytes().to_vec());
            page.made.is_some() || !untouched(&path, &text)
        })
        .map(|(id, _)| id)
        .collect();
    let pages = theirs.iter().map(|&id| site.pages.get(id).address.clone());
    let redirects = site.redirects.iter().map(|redirect| redirect.from.clone());
    let feeds = (feeds.iter())
        .filter(|feed| {
            let list = site.lists.get(feed.list);
            matches!(list.key, ListKey::Folder(_) | ListKey::Declared(_))
                && (list.members.iter().chain(&list.undated)).any(|id| theirs.contains(id))
        })
        .map(|feed| feed.address.clone());
    let files = (site.files.iter())
        .filter(|file| !held_back.contains(&file.address.file()))
        .filter(|file| {
            let path = format!("content/{}", file.path);
            !untouched(&path, &|| on_disk(&path))
        })
        .map(|file| file.address.clone());
    let statics = (assets.statics().iter())
        .filter(|file| {
            let path = format!("static/{}", file.name);
            !untouched(&path, &|| on_disk(&path))
        })
        .filter_map(|file| Address::read_file(&format!("/{}", file.name)).ok());
    pages
        .chain(redirects)
        .chain(feeds)
        .chain(files)
        .chain(statics)
        .collect()
}

#[cfg(test)]
mod tests;
