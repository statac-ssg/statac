//! `statac new --update`: each starter file a site holds brought up to
//! date where the author never changed it, and what is new shown where
//! they did. Nothing is kept in the site: a file is told by its bytes,
//! held to every version the starter has had.
//!
//! What a site's file is, is worked out against a [`Starter`] handed in,
//! as `new` takes its `Machine`, so that it can be held to starters of any
//! history.

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use super::differ::{self, Line};
use super::starter::{self, File, Past};
use super::{past, words};
use crate::config;
use crate::platform;
use crate::problem::{Kind, Problem, Reason};
use crate::report::{Report, shown};
use crate::rewrite::{self, Edit};
use crate::source::{Sources, Span};
use crate::wording;
use crate::yaml::{self, Node, Value};

/// Where a site's own card is described.
pub(super) const CARD: &str = "templates/og.yaml";

/// Where the words the starter's files read are kept in a site.
pub(crate) const WORDS_FILE: &str = "data/words.yaml";

/// How many lines both versions have are shown around each change.
const AROUND: usize = 3;

/// How many starter files not in the site are named, the rest counted.
const NAMED: usize = 6;

/// What `statac new --update` is asked besides the folder.
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct Update {
    /// Whether the starter files not in the site are written.
    pub missing: bool,
}

/// The starter a site is held to: the files `statac new` writes, the
/// sharing card's, which it writes only where asked, every version either
/// had in a release, oldest first, and the words they read.
#[derive(Copy, Clone)]
pub(crate) struct Starter {
    pub files: &'static [File],
    pub cards: &'static [File],
    pub past: &'static [Past],
    pub words: &'static [(&'static str, &'static str)],
}

impl Starter {
    /// The starter this Statac carries.
    pub(crate) const OWN: Starter = Starter {
        files: starter::FILES,
        cards: starter::CARDS,
        past: past::PAST,
        words: words::WORDS,
    };

    /// Whether `path`, from the top of a site, is one of its files' or a
    /// folder on the way to one.
    pub(crate) fn reaches(&self, path: &str) -> bool {
        (self.files.iter().chain(self.cards)).any(|file| {
            let below = file.path.strip_prefix(path);
            below.is_some_and(|below| below.is_empty() || below.starts_with('/'))
        })
    }

    /// Whether `path` is one of the sharing card's files.
    pub(super) fn is_a_cards(&self, path: &str) -> bool {
        self.cards.iter().any(|file| file.path == path)
    }
}

/// A starter file's path in a site, and what the site holds there.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) struct Found {
    pub path: &'static str,
    pub is: Is,
}

/// What a site holds at a starter file's path.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) enum Is {
    /// The current version's bytes.
    Newest,
    /// A past version's bytes, as read, and the current ones to put in
    /// their place: nothing in it is the author's.
    Older {
        read: Vec<u8>,
        current: &'static [u8],
    },
    /// Bytes of the author's own: whether they are a copy of the version
    /// nearest them changed, and what is new since that version: it and the
    /// current one, where they differ and the bytes are a copy.
    Yours {
        copy: bool,
        new: Option<(&'static [u8], &'static [u8])>,
    },
    /// Bytes of the author's own that aren't text, and whether the
    /// starter's file here has changed in some release. Bytes that aren't
    /// text have no version nearest them, so no more is known.
    NotText { changed: bool },
    /// Nothing, and the current version.
    NotHere { current: &'static [u8] },
    /// A link stands at the path, or on the way to it: the link's path.
    Linked(String),
    /// The file couldn't be read, for this reason.
    Unread(Reason),
}

/// Every version of a starter file.
struct Versions {
    /// Oldest first.
    past: Vec<&'static [u8]>,
    current: &'static [u8],
}

impl Versions {
    /// Oldest first, the current one last.
    fn all(&self) -> impl Iterator<Item = &'static [u8]> + '_ {
        self.past.iter().copied().chain([self.current])
    }
}

/// Whether `bytes` at `path`, from the top of a site, are what `statac
/// new` writes there, in this release or one before, the sharing card's
/// files among them, or a sample page as it wrote it: nothing in them is
/// the author's.
pub(crate) fn untouched(path: &str, bytes: &[u8]) -> bool {
    let path = each_languages(path).unwrap_or(path);
    let starter = Starter::OWN;
    let mut written = (starter.files.iter().chain(starter.cards))
        .map(|file| (file.path, file.bytes))
        .chain(starter.past.iter().map(|past| (past.path, past.bytes)));
    written.any(|(at, was)| at == path && was == bytes)
        || std::str::from_utf8(bytes).is_ok_and(|text| super::samples::untouched(path, text))
}

/// Whether `path`, from the top of a site, is where `statac new` writes a
/// file, a sample page among them.
pub(crate) fn writes(path: &str) -> bool {
    let path = each_languages(path).unwrap_or(path);
    let starter = Starter::OWN;
    (starter.files.iter().chain(starter.cards)).any(|file| file.path == path)
        || starter.past.iter().any(|past| past.path == path)
        || super::samples::writes(path)
}

/// The starter's page that `path` is a language's copy of, where it is
/// one: `content/fr/tags.md` of `content/tags.md`.
fn each_languages(path: &str) -> Option<&'static str> {
    let (folder, name) = path.strip_prefix("content/")?.split_once('/')?;
    (!folder.is_empty() && !name.contains('/'))
        .then(|| {
            (super::starter::EACH_LANGUAGE.iter())
                .find(|page| page.strip_prefix("content/") == Some(name))
                .copied()
        })
        .flatten()
}

/// Every starter file's versions, the sharing card's among them, by its
/// path in a site.
fn versions(starter: &Starter) -> BTreeMap<&'static str, Versions> {
    (starter.files.iter().chain(starter.cards))
        .map(|file| {
            let past = (starter.past.iter())
                .filter(|past| past.path == file.path)
                .map(|past| past.bytes)
                .collect();
            let current = file.bytes;
            (file.path, Versions { past, current })
        })
        .collect()
}

/// What the site at `root` holds at each starter file's path, by path,
/// held to `starter`. Paths only past versions have are left out.
pub(super) fn survey(root: &Path, starter: &Starter) -> Vec<Found> {
    (versions(starter).iter())
        .map(|(&path, versions)| {
            let is = match platform::read_file(root, path) {
                Ok(Some(bytes)) => told(bytes, versions),
                Ok(None) => Is::NotHere {
                    current: versions.current,
                },
                Err(error) => match link_on_the_way(root, path) {
                    Some(link) => Is::Linked(link),
                    None => Is::Unread(Reason::of(&error)),
                },
            };
            Found { path, is }
        })
        .collect()
}

/// What `bytes` are, held to the versions of their path.
fn told(bytes: Vec<u8>, versions: &Versions) -> Is {
    let current = versions.current;
    if current == bytes {
        return Is::Newest;
    }
    if versions.past.contains(&bytes.as_slice()) {
        return Is::Older {
            read: bytes,
            current,
        };
    }
    let Some(text) = text(&bytes) else {
        let changed = versions.past.iter().any(|past| *past != current);
        return Is::NotText { changed };
    };
    let read = differ::lines(text);
    // The nearest version, the newer of two as near, and whether the file
    // is a copy of it changed.
    let mut nearest: Option<(usize, &'static [u8], bool)> = None;
    for version in versions.all() {
        let Some(version_text) = self::text(version) else {
            continue;
        };
        let lines = differ::lines(version_text);
        let distance = differ::distance(&read, &lines);
        if nearest.is_none_or(|(least, ..)| distance <= least) {
            let shared = (read.len() + lines.len() - distance) / 2;
            nearest = Some((
                distance,
                version,
                is_a_copy(shared, read.len(), lines.len()),
            ));
        }
    }
    let copy = nearest.is_some_and(|(_, _, copy)| copy);
    let new = nearest
        .filter(|(_, version, copy)| *copy && *version != current)
        .map(|(_, version, _)| (version, current));
    Is::Yours { copy, new }
}

/// Whether a file of `own` lines is a version of `theirs` lines changed,
/// sharing `shared` of them: half of either's at least. A file less like
/// every version is the author's own, which the starter's news isn't for.
fn is_a_copy(shared: usize, own: usize, theirs: usize) -> bool {
    shared > 0 && (shared * 2 >= own || shared * 2 >= theirs)
}

/// `bytes` as text, where they are.
fn text(bytes: &[u8]) -> Option<&str> {
    std::str::from_utf8(bytes)
        .ok()
        .filter(|text| !text.contains('\0'))
}

/// The first link on the way to `rel` under `root`, `rel` itself
/// included, by its path from `root`.
fn link_on_the_way(root: &Path, rel: &str) -> Option<String> {
    let mut at = String::new();
    for piece in rel.split('/') {
        if !at.is_empty() {
            at.push('/');
        }
        at.push_str(piece);
        let found = std::fs::symlink_metadata(platform::join(root, &at)).ok()?;
        if found.is_symlink() {
            return Some(at);
        }
    }
    None
}

/// The note for a site at `site` with starter files that have a newer
/// version, where it has any: those whose bytes are a past version's, and
/// the author's where the version nearest them isn't the current one. A
/// file of the author's that isn't text is never counted. Nothing is read
/// where the starter has no past.
pub(crate) fn newer_starter(site: &Path, starter: &Starter) -> Option<Problem> {
    if starter.past.is_empty() {
        return None;
    }
    let root = std::fs::canonicalize(site).ok()?;
    let files = (survey(&root, starter).iter())
        .filter(|found| matches!(found.is, Is::Older { .. } | Is::Yours { new: Some(_), .. }))
        .count();
    (files > 0).then(|| Problem::nowhere(Kind::NewerStarter { files }))
}

/// The sharing card's font that `statac new --update --missing` would
/// write into the site at `root` and that draws every one of `characters`,
/// by its name in `templates/og/`: one the site lacks, where it describes
/// its own card.
pub(crate) fn font_to_bring(
    root: &Path,
    starter: &Starter,
    characters: &[char],
) -> Option<&'static str> {
    let here = |path: &str| std::fs::symlink_metadata(platform::join(root, path)).is_ok();
    if !here(CARD) {
        return None;
    }
    starter.cards.iter().find_map(|file| {
        let name = file.path.strip_prefix(crate::cards::FOLDER)?;
        let name = name.strip_prefix('/')?;
        let font = crate::cards::is_a_font(name)
            && !here(file.path)
            && crate::cards::draw::draws_all(file.bytes, characters);
        font.then_some(name)
    })
}

/// Brings the starter files of the site in `folder` up to date and says
/// what it did on `out`. Gives the exit code: 0, or 1 where a file
/// couldn't be read or written.
pub fn run(folder: &Path, update: Update, out: &mut dyn Write, colour: bool) -> u8 {
    run_holding(folder, update, &Starter::OWN, out, colour)
}

/// [`run`], held to `starter`.
pub(super) fn run_holding(
    folder: &Path,
    update: Update,
    starter: &Starter,
    out: &mut dyn Write,
    colour: bool,
) -> u8 {
    let sources = Sources::default();
    let report = Report::new(&sources, folder, colour);
    let mut said = String::new();
    let mut say = |text: &str| {
        said.push_str(text);
        said.push('\n');
    };
    let root = match a_site(folder) {
        Ok(root) => root,
        Err(problem) => {
            let _ = out.write_all(report.problem(&problem).as_bytes());
            return 1;
        }
    };
    let surveyed = survey(&root, starter);
    let mut failed = false;
    let mut problems = String::new();
    let problem = |kind: Kind| report.problem(&Problem::nowhere(kind));
    let (mut updated, mut new) = (0, 0);

    for found in &surveyed {
        if let Is::Older { read, current } = &found.is {
            match rewrite::replace(&root, found.path, read, current) {
                Ok(()) => {
                    say(&wording::updated_file(found.path));
                    updated += 1;
                }
                Err(reason) => {
                    failed = true;
                    say(problem(Kind::CantWrite {
                        path: found.path.into(),
                        reason,
                    })
                    .trim_end());
                }
            }
        }
    }
    for found in &surveyed {
        if let Is::Yours {
            new: Some((nearest, current)),
            ..
        } = &found.is
        {
            say(&wording::yours_with_new(found.path));
            say(&changes(nearest, current));
            new += 1;
        }
    }
    for found in &surveyed {
        if found.is == (Is::NotText { changed: true }) {
            say(&wording::yours_not_text(found.path));
        }
    }

    // A site holding nothing of the starter's is told so once, in place of
    // every file it lacks.
    let without = !surveyed.iter().any(|found| {
        let asked_apart = starter.is_a_cards(found.path) || starter::is_a_samples(found.path);
        let starters = matches!(
            found.is,
            Is::Newest | Is::Older { .. } | Is::Yours { copy: true, .. }
        );
        starters && !asked_apart
    });
    let (settings, _) = config::read(&root, &mut Sources::default());
    let languages: Vec<&str> = settings.languages().iter().map(|tag| tag.value).collect();
    let mut words_missing = false;
    match add_words(&root, &languages, starter.words) {
        Words::Nothing => {}
        Words::NotHere => words_missing = true,
        Words::Told { lines, wrote } => {
            for line in lines {
                say(&line);
            }
            updated += usize::from(wrote);
        }
        Words::Failed(kind) => {
            failed = true;
            say(problem(kind).trim_end());
        }
    }

    // A site that describes no card never asked for the sharing card's
    // files, and one without a sample page's did away with it.
    let described = (surveyed.iter())
        .any(|found| found.path == CARD && !matches!(found.is, Is::NotHere { .. }));
    let missing: Vec<(&str, &[u8])> = (surveyed.iter())
        .filter_map(|found| match found.is {
            Is::NotHere { .. } if !described && starter.is_a_cards(found.path) => None,
            Is::NotHere { .. } if starter::is_a_samples(found.path) => None,
            Is::NotHere { current } => Some((found.path, current)),
            _ => None,
        })
        .collect();
    match update.missing {
        true => {
            if words_missing {
                let words = super::words_file(&languages, starter.words);
                match write_missing(&root, WORDS_FILE, words.as_bytes()) {
                    Ok(()) => {
                        say(&wording::wrote_file(WORDS_FILE));
                        updated += 1;
                    }
                    Err(error) => {
                        failed = true;
                        say(problem(Kind::CantWrite {
                            path: WORDS_FILE.into(),
                            reason: Reason::of(&error),
                        })
                        .trim_end());
                    }
                }
            }
            for (path, current) in &missing {
                match write_missing(&root, path, current) {
                    Ok(()) => {
                        say(&wording::wrote_file(path));
                        updated += 1;
                    }
                    Err(error) => {
                        failed = true;
                        say(problem(Kind::CantWrite {
                            path: (*path).into(),
                            reason: Reason::of(&error),
                        })
                        .trim_end());
                    }
                }
            }
        }
        false => {
            // The card's files are asked for apart from the starter, so
            // a site without it is still told each of theirs it lacks.
            let told_apart = without && (missing.iter()).any(|(path, _)| !starter.is_a_cards(path));
            let named: Vec<&str> = (missing.iter())
                .map(|(path, _)| *path)
                .filter(|path| !told_apart || starter.is_a_cards(path))
                .collect();
            match told_apart {
                true => say(wording::NONE_OF_THE_STARTER),
                false if words_missing => say(wording::NO_WORDS_FILE),
                false => {}
            }
            if !named.is_empty() {
                let more = named.len().saturating_sub(NAMED);
                say(&wording::not_here(&named[..named.len() - more], more));
            }
        }
    }
    for found in &surveyed {
        match &found.is {
            Is::Linked(link) => say(&shown(&wording::left_for_a_link(found.path, link))),
            Is::Unread(reason) => {
                failed = true;
                problems.push_str(&problem(Kind::CantRead {
                    path: found.path.into(),
                    reason: reason.clone(),
                }));
            }
            _ => {}
        }
    }
    said.push_str(&problems);
    match wording::updated_files(updated, new) {
        Some(closing) => said.push_str(&format!("{closing}\n")),
        None if said.is_empty() => said.push_str(&format!("{}\n", wording::NOTHING_TO_UPDATE)),
        None => {}
    }
    let _ = out.write_all(said.as_bytes());
    let _ = out.flush();
    u8::from(failed)
}

/// The folder a site is in, as found by its whole path, or why `folder`
/// isn't one.
fn a_site(folder: &Path) -> Result<PathBuf, Problem> {
    let cant_read = |reason: Reason| {
        Problem::nowhere(Kind::CantRead {
            path: folder.to_path_buf(),
            reason,
        })
    };
    match std::fs::metadata(folder) {
        Ok(found) if found.is_dir() => {}
        Ok(_) => return Err(cant_read(Reason::FileInTheWay)),
        Err(error) => return Err(cant_read(Reason::of(&error))),
    }
    if !folder.join(config::FILE_NAME).is_file() {
        return Err(cant_read(Reason::NotASite));
    }
    std::fs::canonicalize(folder).map_err(|error| cant_read(Reason::of(&error)))
}

/// The change from `nearest` to `current`, as the lines that left and
/// came, with the lines around them.
fn changes(nearest: &[u8], current: &[u8]) -> String {
    let (old, new) = (
        String::from_utf8_lossy(nearest),
        String::from_utf8_lossy(current),
    );
    let (old, new) = (differ::lines(&old), differ::lines(&new));
    let mut lines = Vec::new();
    for hunk in differ::hunks(&differ::diff(&old, &new), AROUND) {
        lines.push(format!("  {}", wording::changed_at(hunk.line)));
        for line in hunk.lines {
            let (mark, text) = match line {
                Line::Same(text) => (' ', text),
                Line::Left(text) => ('-', text),
                Line::Came(text) => ('+', text),
            };
            let bare = text.strip_suffix('\n').unwrap_or(text);
            let bare = bare.strip_suffix('\r').unwrap_or(bare);
            lines.push(format!("  {mark}{}", shown(bare)));
            if !text.ends_with('\n') {
                lines.push(format!("  {}", wording::NO_LINE_BREAK));
            }
        }
    }
    lines.join("\n")
}

/// Writes `bytes` as the file `path` in the site at `root`, making each
/// folder on the way that isn't there.
pub(super) fn write_missing(root: &Path, path: &str, bytes: &[u8]) -> std::io::Result<()> {
    if let Some((above, _)) = path.rsplit_once('/') {
        let mut end = 0;
        for piece in above.split('/') {
            end += piece.len();
            match platform::make_folder(root, &above[..end]) {
                Err(error) if error.kind() != std::io::ErrorKind::AlreadyExists => {
                    return Err(error);
                }
                _ => {}
            }
            end += 1;
        }
    }
    platform::make_file(root, path, bytes)
}

/// What became of the words the starter reads.
enum Words {
    /// The site names no language, or every block has every word.
    Nothing,
    /// The site has no `data/words.yaml`.
    NotHere,
    /// What to say, and whether the file was written.
    Told { lines: Vec<String>, wrote: bool },
    /// The file couldn't be read or written.
    Failed(Kind),
}

/// Adds each of `words` missing from a language's block in the site's
/// `data/words.yaml`, in English, at the block's end, for each of
/// `languages`. Nothing else in the file changes.
fn add_words(root: &Path, languages: &[&str], words: &[(&str, &str)]) -> Words {
    if languages.is_empty() || words.is_empty() {
        return Words::Nothing;
    }
    let told = |line: String| Words::Told {
        lines: vec![line],
        wrote: false,
    };
    let cant_read = |reason| {
        Words::Failed(Kind::CantRead {
            path: WORDS_FILE.into(),
            reason,
        })
    };
    let bytes = match platform::read_file(root, WORDS_FILE) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => return Words::NotHere,
        Err(error) => {
            return match link_on_the_way(root, WORDS_FILE) {
                Some(link) => told(shown(&wording::left_for_a_link(WORDS_FILE, &link))),
                None => cant_read(Reason::of(&error)),
            };
        }
    };
    let Ok(text) = String::from_utf8(bytes) else {
        return cant_read(Reason::NotUtf8);
    };
    let Some((edits, lines)) = words_added(&text, languages, words) else {
        return told(wording::WORDS_UNREADABLE.into());
    };
    if edits.is_empty() {
        return match lines.is_empty() {
            true => Words::Nothing,
            false => Words::Told {
                lines,
                wrote: false,
            },
        };
    }
    match rewrite::rewrite(root, WORDS_FILE, &text, &edits) {
        Ok(_) => Words::Told { lines, wrote: true },
        Err(reason) => Words::Failed(Kind::CantWrite {
            path: WORDS_FILE.into(),
            reason,
        }),
    }
}

/// The edits adding each word missing from each language's block of
/// `text`, and what to say of each block; none where `text` can't be read
/// as YAML.
fn words_added(
    text: &str,
    languages: &[&str],
    words: &[(&str, &str)],
) -> Option<(Vec<Edit>, Vec<String>)> {
    let mut sources = Sources::default();
    let source = sources.add(WORDS_FILE, text.to_owned());
    let read = yaml::read(&sources, source).ok()?;
    let mut edits = Vec::new();
    let mut lines = Vec::new();
    for tag in languages {
        let Some(block) = read.get(tag) else {
            lines.push(wording::no_words_under(tag));
            continue;
        };
        let Value::Mapping(entries) = &block.value else {
            lines.push(wording::no_words_under(tag));
            continue;
        };
        let missing: Vec<&(&str, &str)> = (words.iter())
            .filter(|(name, _)| block.get(name).is_none())
            .collect();
        if missing.is_empty() {
            continue;
        }
        let Some(edit) = at_the_end(text, block, entries.last(), &missing) else {
            lines.push(wording::words_not_one_to_a_line(tag));
            continue;
        };
        edits.push(edit);
        let names: Vec<&str> = missing.iter().map(|(name, _)| *name).collect();
        lines.push(shown(&wording::added_words(&names, tag)));
    }
    Some((edits, lines))
}

/// The edit putting `missing` at the end of the block `block`, after its
/// last entry, `last`, in the block's own indentation and line breaks;
/// none where the block isn't written one entry to a line.
fn at_the_end(
    text: &str,
    block: &Node,
    last: Option<&yaml::Entry>,
    missing: &[&(&str, &str)],
) -> Option<Edit> {
    let last = last?;
    if text[block.place.span.range().start..].starts_with('{') {
        return None;
    }
    let key = last.key.place.span.range().start;
    let line_start = text[..key].rfind('\n').map_or(0, |at| at + 1);
    let indent = &text[line_start..key];
    if indent.is_empty() || !indent.chars().all(|c| c == ' ') {
        return None;
    }
    let end = (last.value.place.span.range().end).max(last.key.place.span.range().end);
    let (at, ending) = match text[..end].ends_with('\n') {
        true => (end, text[..end].ends_with("\r\n")),
        false => match text[end..].find('\n') {
            Some(found) => (end + found + 1, text[..end + found].ends_with('\r')),
            None => (text.len(), text[line_start..].contains('\r')),
        },
    };
    let eol = if ending { "\r\n" } else { "\n" };
    let added: Vec<String> = (missing.iter())
        .map(|(name, english)| format!("{indent}{name}: {}", yaml::write::text(english)))
        .collect();
    let with = match at == text.len() && !text.ends_with('\n') {
        true => format!("{eol}{}", added.join(eol)),
        false => format!("{}{eol}", added.join(eol)),
    };
    Some(Edit {
        span: Span::new(at, at),
        with,
    })
}
