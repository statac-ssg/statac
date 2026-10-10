//! The site's pictures: every picture file under `content/`, `static/`
//! and `fetched/`, what its header says, the file describing it beside it,
//! and for a photograph the copies it is published as.
//!
//! Each is known once the site is read, before any page is built, so a
//! page names a copy that is yet to be made. What a header says, and what
//! the pixels say of a photograph whose header says it carries alpha, is
//! kept in `.statac/pictures/index` against the file's size and time, so a
//! build reads no picture it has read before; the copies are kept beside
//! it, each named by its key.

mod develop;
mod index;
mod progress;
mod sidecar;
#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::panic::PanicHookInfo;
use std::path::{Path, PathBuf};
use std::sync::Once;

use rayon::prelude::*;

use crate::content;
use crate::pictures::{self, Header, Kind, Photograph, Rendition, Size, Unreadable};
use crate::problem::{Kind as Told, Named, PictureFault, Problem};
use crate::site::{Arena, Id, PictureId};

pub use develop::{Developed, Shown, develop, sweep};
pub(crate) use develop::{Kept, fault};
pub(crate) use index::{escaped, unescaped};
pub use progress::Progress;
pub(crate) use progress::{Counted, Making};
pub use sidecar::Sidecar;

/// Where Statac keeps what it makes between builds, in the site's folder.
pub const KEPT: &str = ".statac";

/// The folder in the site at `site` where copies and the index are kept,
/// whether or not it is there yet. Refused where `.statac` or the folder
/// in it is a link or anything else than a folder: nothing is ever read,
/// written or removed through a link, which may lead out of the site.
fn kept_folder(site: &Path) -> std::io::Result<PathBuf> {
    kept_in(site, "pictures")
}

/// [`kept_folder`], for the folder `name` of what Statac keeps.
pub(crate) fn kept_in(site: &Path, name: &str) -> std::io::Result<PathBuf> {
    kept_at(site, &[KEPT, name])
}

/// [`kept_folder`], for the folder everything Statac keeps is in.
pub(crate) fn kept(site: &Path) -> std::io::Result<PathBuf> {
    kept_at(site, &[KEPT])
}

fn kept_at(site: &Path, names: &[&str]) -> std::io::Result<PathBuf> {
    let mut folder = site.to_path_buf();
    for name in names {
        folder.push(name);
        match std::fs::symlink_metadata(&folder) {
            Ok(found) if found.file_type().is_symlink() => {
                return Err(std::io::Error::other("it is a link"));
            }
            Ok(found) if !found.is_dir() => {
                return Err(std::io::Error::from(std::io::ErrorKind::NotADirectory));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(folder)
}

/// Whether a file itself stands at `path`: not a link to one, nor a
/// folder, nor on Unix a file with another name somewhere, which may be a
/// file outside the site.
pub(crate) fn is_file_itself(path: &Path) -> bool {
    std::fs::symlink_metadata(path)
        .is_ok_and(|found| found.file_type().is_file() && crate::platform::named_once(&found))
}

/// Writes `bytes` as `name` in `folder`, one of Statac's own: under a name
/// nothing has first, then renamed, so a file there is always whole. A
/// link or a folder standing at either name is never written through.
pub(crate) fn keep_file(
    folder: &Path,
    name: &str,
    fresh: &str,
    bytes: &[u8],
) -> std::io::Result<()> {
    let to = folder.join(name);
    if std::fs::symlink_metadata(&to).is_ok() && !is_file_itself(&to) {
        return Err(std::io::Error::other("something else has its name"));
    }
    let fresh = folder.join(fresh);
    let written = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&fresh)
        .and_then(|mut file| std::io::Write::write_all(&mut file, bytes))
        .and_then(|()| crate::platform::rename(&fresh, &to));
    written.inspect_err(|_| {
        if is_file_itself(&fresh) {
            let _ = std::fs::remove_file(&fresh);
        }
    })
}

/// The folder a picture file is under.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum Root {
    Content,
    Static,
    /// What Statac fetched, published only as copies, and never named
    /// from the writing.
    Fetched,
}

impl Root {
    pub fn folder(self) -> &'static str {
        match self {
            Root::Content => "content",
            Root::Static => crate::assets::STATIC,
            Root::Fetched => crate::fetched::FOLDER,
        }
    }
}

/// What a picture file says of itself.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Seen {
    Photograph {
        photograph: Photograph,
        /// Whether its copies are converted to sRGB from the colour
        /// profile it carries.
        converts: bool,
        /// Whether some pixel of it is not fully opaque, told from its
        /// pixels where its header says it carries alpha.
        transparent: bool,
    },
    Plain {
        kind: Kind,
        size: Option<Size>,
    },
    /// It begins as a kind of picture and can't be read as one.
    Unreadable(PictureFault),
}

pub struct Picture {
    pub root: Root,
    /// Its path under its root, names parted by `/`.
    pub path: String,
    file: PathBuf,
    /// Where it is published, from the top of the site; none for a file
    /// beside a page left out of the build.
    pub address: Option<String>,
    pub seen: Seen,
    /// The BLAKE3 of its bytes.
    pub hash: blake3::Hash,
    /// Whether a file under `content/` holds where it was taken.
    pub located: bool,
    pub sidecar: Option<Sidecar>,
    /// A photograph's copies; none for any other picture, and none for a
    /// photograph left alone by `optimise: false`.
    pub copies: Vec<Rendition>,
    /// The copy at the photograph's own width, made only where a template
    /// asks for it; none where the widest of `copies` is that wide.
    pub full: Option<Rendition>,
    /// Which of the files beside the writing it is, where it is one.
    pub travelling: Option<usize>,
}

impl Picture {
    /// The file, from the site's folder, as a problem names it.
    pub fn shown(&self) -> String {
        format!("{}/{}", self.root.folder(), self.path)
    }

    /// The size its tag gives: a photograph's once turned.
    pub fn size(&self) -> Option<Size> {
        match &self.seen {
            Seen::Photograph { photograph, .. } => Some(photograph.shown),
            Seen::Plain { size, .. } => *size,
            Seen::Unreadable(_) => None,
        }
    }

    /// Whether the file is published only where a built page links it: a
    /// photograph beside the writing that has copies to show in its place.
    pub fn held(&self) -> bool {
        self.root == Root::Content && !self.copies.is_empty()
    }

    /// Whether the file describing it leaves a photograph as it is.
    pub fn left_alone(&self) -> bool {
        self.sidecar
            .as_ref()
            .and_then(|sidecar| sidecar.optimise.as_ref())
            .is_some_and(|optimise| !optimise.value)
    }

    /// Whether the file describing it marks it as decoration.
    pub fn decorative(&self) -> bool {
        self.sidecar
            .as_ref()
            .and_then(|sidecar| sidecar.decorative.as_ref())
            .is_some_and(|decorative| decorative.value)
    }

    /// The copies published for it: its ladder, and with `full` the copy
    /// at its own width.
    pub fn wanted(&self, full: bool) -> impl Iterator<Item = &Rendition> {
        self.copies
            .iter()
            .chain(self.full.as_ref().filter(|_| full))
    }

    /// Where a copy named `name` is published, as a file from the top of
    /// the built site.
    fn copy_file(&self, name: &str) -> Option<String> {
        let address = self.address.as_deref()?;
        let folder = &address[1..=address.rfind('/')?];
        Some(format!("{folder}{name}"))
    }
}

/// Every picture of the site.
#[derive(Default)]
pub struct Pictures {
    all: Arena<PictureId, Picture>,
    by_source: HashMap<(Root, String), PictureId>,
    by_address: HashMap<String, PictureId>,
    by_name: HashMap<String, Vec<PictureId>>,
    /// How many picture files were read, the index not knowing them.
    pub opened: usize,
    index: index::Index,
}

/// A picture the writing names, and the address that reaches it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Found {
    pub id: PictureId,
    /// As written where that reaches the file from the page, else the
    /// file's own address, the site's path included.
    pub address: String,
}

/// Where a picture is named from.
#[derive(Copy, Clone, Debug)]
pub struct From<'a> {
    /// The folder under `content/` of the file the name is written in,
    /// ending with `/`, or empty at the top; none where no file holds it.
    pub folder: Option<&'a str>,
    /// Where the page is published, the site's path included.
    pub page: &'a str,
    /// The path the site is published under, or empty.
    pub base: &'a str,
}

impl Pictures {
    pub fn get(&self, id: PictureId) -> &Picture {
        self.all.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (PictureId, &Picture)> {
        self.all.iter()
    }

    pub fn len(&self) -> usize {
        self.all.len()
    }

    /// The picture at `path` under `fetched/`.
    pub fn fetched(&self, path: &str) -> Option<PictureId> {
        self.by_source
            .get(&(Root::Fetched, path.to_owned()))
            .copied()
    }

    pub fn is_empty(&self) -> bool {
        self.all.is_empty()
    }

    /// The picture `written` names from `from`: a relative name from the
    /// folder of the file it is written in, a name from the top in
    /// `content/` and then in `static/`, and either as a browser reads it
    /// from the page. None for an address elsewhere, or one that reaches
    /// no picture.
    pub fn find(&self, written: &str, from: &From<'_>) -> Option<Found> {
        if written.is_empty() || content::address::is_full(written) || written.starts_with('#') {
            return None;
        }
        let end = written.find(['?', '#']).unwrap_or(written.len());
        let (path, rest) = written.split_at(end);
        let name = content::address::decoded(path);
        let source = match name.strip_prefix('/') {
            Some(_) => {
                let inside = match from.base.is_empty() {
                    true => None,
                    false => name
                        .strip_prefix(from.base)
                        .filter(|rest| rest.starts_with('/')),
                };
                [inside, Some(name.as_str())]
                    .into_iter()
                    .flatten()
                    .filter_map(|top| walked("", top))
                    .find_map(|rel| {
                        [Root::Content, Root::Static]
                            .into_iter()
                            .find_map(|root| self.by_source.get(&(root, rel.clone())))
                    })
            }
            None => from
                .folder
                .and_then(|folder| walked(folder, &name))
                .and_then(|rel| self.by_source.get(&(Root::Content, rel))),
        };
        // Where a browser lands, following the name from the page.
        let folder = &from.page[..=from.page.rfind('/')?];
        let landed = match name.starts_with('/') {
            true => walked("", &name),
            false => walked(folder, &name),
        }
        .map(|path| format!("/{path}"));
        let lands_on = |picture: &Picture| {
            let address = picture.address.as_deref()?;
            Some(format!("{}{address}", from.base))
        };
        let id = source.copied().or_else(|| {
            let inside = landed.as_deref()?.strip_prefix(from.base)?;
            self.by_address.get(inside).copied()
        })?;
        let own = lands_on(self.get(id))?;
        let address = match landed.as_deref() == Some(own.as_str()) {
            true => written.to_owned(),
            false => format!("{own}{rest}"),
        };
        Some(Found { id, address })
    }

    /// The pictures whose file names `text` holds, as writing naming them
    /// would: what a page left out of the build may show.
    pub fn named_in(&self, text: &str) -> Vec<PictureId> {
        let mut found = Vec::new();
        for word in text.split(|c: char| c.is_whitespace() || "()[]<>\"'=,".contains(c)) {
            if !is_picture(word) {
                continue;
            }
            let word = content::address::decoded(word);
            let name = word.rsplit('/').next().unwrap_or(&word);
            found.extend(self.by_name.get(name).into_iter().flatten());
        }
        found.sort_unstable();
        found.dedup();
        found
    }

    /// Every copy the site may publish: its file from the top of the built
    /// site, and the photograph it is made from.
    pub fn claims(&self) -> Vec<(String, Named)> {
        let mut claims = Vec::new();
        for (_, picture) in self.iter() {
            for copy in picture.wanted(true) {
                if let Some(file) = picture.copy_file(&copy.name) {
                    claims.push((file, Named::File(picture.shown())));
                }
            }
        }
        claims
    }

    /// The files beside the writing published only where a built page
    /// links them, each by its place among those files.
    pub fn held(&self) -> impl Iterator<Item = (PictureId, usize)> + '_ {
        self.iter()
            .filter(|(_, picture)| picture.held())
            .filter_map(|(id, picture)| Some((id, picture.travelling?)))
    }

    /// Keeps what the headers said for the next build. Nothing is written
    /// where nothing changed.
    pub fn keep_index(&self, site: &Path) -> std::io::Result<()> {
        self.index.save(site)
    }
}

/// `name` followed from `folder`, a path ending with `/` or empty, with its
/// `.` and `..` walked; none where it climbs out of the top.
pub(crate) fn walked(folder: &str, name: &str) -> Option<String> {
    let mut pieces: Vec<&str> = folder
        .split('/')
        .filter(|piece| !piece.is_empty())
        .collect();
    for piece in name.split('/') {
        match piece {
            "" | "." => {}
            ".." => {
                pieces.pop()?;
            }
            piece => pieces.push(piece),
        }
    }
    Some(pieces.join("/"))
}

/// The endings of the files read as pictures; what each is, its bytes say.
const ENDINGS: [&str; 9] = [
    "jpg", "jpeg", "webp", "png", "gif", "svg", "avif", "heic", "heif",
];

/// Whether a file's name says it may be a picture.
pub fn is_picture(name: &str) -> bool {
    name.rsplit_once('.').is_some_and(|(_, ending)| {
        ENDINGS
            .iter()
            .any(|known| ending.eq_ignore_ascii_case(known))
    })
}

/// Whether a file under `content/` describes the picture it is named
/// after: `photo-1.jpg.yaml` beside `photo-1.jpg`.
pub fn is_sidecar(name: &str) -> bool {
    name.strip_suffix(".yaml").is_some_and(is_picture)
}

/// A picture file to read.
struct Wanted {
    root: Root,
    path: String,
    file: PathBuf,
    address: Option<String>,
    travelling: Option<usize>,
}

/// Reads every picture under `content/`, `static/` and `fetched/` of the
/// site at `root` into `read`, with the files describing them, and says
/// what is wrong with those.
pub fn read(
    root: &Path,
    read: &mut content::Read,
    statics: &[crate::assets::StaticFile],
) -> Vec<Problem> {
    let content = root.join("content");
    let mut wanted = Vec::new();
    for (at, file) in read.site.files.iter().enumerate() {
        if is_picture(&file.path) {
            wanted.push(Wanted {
                root: Root::Content,
                path: file.path.clone(),
                file: crate::platform::join(&content, &file.path),
                address: Some(file.address.as_str().to_owned()),
                travelling: Some(at),
            });
        }
    }
    for path in read.left_out.iter().filter(|path| is_picture(path)) {
        wanted.push(Wanted {
            root: Root::Content,
            path: path.clone(),
            file: crate::platform::join(&content, path),
            address: None,
            travelling: None,
        });
    }
    for file in statics.iter().filter(|file| is_picture(&file.name)) {
        wanted.push(Wanted {
            root: Root::Static,
            path: file.name.clone(),
            file: file.path().to_path_buf(),
            address: Some(format!("/{}", file.name)),
            travelling: None,
        });
    }
    // Its copies are published from the top of the built site as the file
    // stands under `fetched/`.
    for (path, file) in read.site.fetched.files.iter() {
        if is_picture(path) {
            wanted.push(Wanted {
                root: Root::Fetched,
                path: path.clone(),
                file: file.clone(),
                address: Some(format!("/{path}")),
                travelling: None,
            });
        }
    }

    let known = index::Index::load(root);
    let entries: Vec<Option<(index::Entry, bool)>> = wanted
        .par_iter()
        .map(|wanted| {
            let metadata = std::fs::metadata(&wanted.file).ok()?;
            let stamp = index::Stamp::of(&metadata);
            if let Some(entry) = known.get(wanted.root, &wanted.path)
                && stamp.is_some_and(|stamp| stamp == entry.stamp)
            {
                return Some((entry.clone(), false));
            }
            let bytes = std::fs::read(&wanted.file).ok()?;
            let stamp = stamp.unwrap_or_default();
            Some((index::Entry::read(stamp, wanted.root, &bytes), true))
        })
        .collect();

    let settings = pictures::Settings::default();
    let max_width = read
        .settings
        .whole("max_width")
        .map_or(1600, |width| u32::try_from(width).unwrap_or(u32::MAX));
    let optimise = read.settings.truth("optimise_images") != Some(false);
    let mut gathered = Pictures::default();
    let mut index = index::Index::default();
    let mut changed = known.len() != entries.iter().flatten().count();
    for (wanted, entry) in wanted.into_iter().zip(entries) {
        // A file that can't be read is told of when it is published.
        let Some((entry, opened)) = entry else {
            continue;
        };
        gathered.opened += usize::from(opened);
        changed |= opened;
        index.insert(wanted.root, &wanted.path, entry.clone());
        // What is fetched is shown only as a photograph's copies.
        let seen = match (wanted.root, entry.seen) {
            (Root::Fetched, None | Some(Seen::Plain { .. })) => {
                Seen::Unreadable(PictureFault::NotAStill)
            }
            (_, Some(seen)) => seen,
            (_, None) => continue,
        };
        let id = gathered.all.push(Picture {
            root: wanted.root,
            path: wanted.path.clone(),
            file: wanted.file,
            address: wanted.address.clone(),
            seen,
            hash: entry.hash,
            located: entry.located,
            sidecar: None,
            copies: Vec::new(),
            full: None,
            travelling: wanted.travelling,
        });
        if wanted.root != Root::Fetched {
            let name = wanted.path.rsplit('/').next().unwrap_or(&wanted.path);
            gathered
                .by_name
                .entry(name.to_owned())
                .or_default()
                .push(id);
            if let Some(address) = wanted.address {
                gathered.by_address.insert(address, id);
            }
        }
        gathered.by_source.insert((wanted.root, wanted.path), id);
    }
    index.changed = changed;
    gathered.index = index;

    let mut problems = Vec::new();
    for path in std::mem::take(&mut read.sidecars) {
        let Ok(text) = std::fs::read_to_string(crate::platform::join(&content, &path)) else {
            continue;
        };
        let source = read.sources.add(format!("content/{path}"), text);
        let (sidecar, found) = sidecar::read(&read.sources, source);
        problems.extend(found);
        let picture = path.strip_suffix(".yaml").unwrap_or(&path);
        match gathered.by_source.get(&(Root::Content, picture.to_owned())) {
            Some(&id) => gathered.all.get_mut(id).sidecar = sidecar,
            None => {
                let (folder, name) = picture
                    .rsplit_once('/')
                    .map_or(("", picture), |(f, n)| (f, n));
                let beside = gathered
                    .iter()
                    .filter(|(_, other)| other.root == Root::Content)
                    .filter_map(|(_, other)| match other.path.rsplit_once('/') {
                        Some((f, n)) if f == folder => Some(n),
                        None if folder.is_empty() => Some(other.path.as_str()),
                        _ => None,
                    });
                let nearest = crate::suggest::nearest(name, beside).map(str::to_owned);
                problems.push(Problem::at(
                    Told::DescribesNoPicture {
                        picture: format!("content/{picture}"),
                        nearest,
                    },
                    content::words::head(&read.sources, source),
                ));
            }
        }
    }

    for at in 0..gathered.all.len() {
        let picture = gathered.all.get_mut(PictureId::from_index(at));
        let Seen::Photograph {
            photograph,
            transparent,
            ..
        } = &picture.seen
        else {
            continue;
        };
        let transparent = *transparent;
        if (!optimise || picture.left_alone()) && picture.root != Root::Fetched {
            continue;
        }
        let named = picture.address.as_deref().unwrap_or(&picture.path);
        let name = named.rsplit('/').next().unwrap_or(named);
        let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
        let source = &picture.hash;
        picture.copies =
            pictures::ladder(photograph, transparent, stem, source, max_width, &settings);
        // A cap as wide as the photograph gives a ladder ending at its
        // own width; of that ladder only the fallback is wanted.
        let own = photograph.shown.width;
        if picture.copies.last().is_some_and(|copy| copy.width < own) {
            picture.full =
                pictures::ladder(photograph, transparent, stem, source, own, &settings).pop();
        }
    }
    read.site.pictures = gathered;
    problems
}

/// What a picture file's bytes say of it; none where they begin as no
/// kind of picture known.
fn seen(bytes: &[u8]) -> Option<Seen> {
    match quietly(|| pictures::read_header(bytes)) {
        Ok(Header::Photograph(photograph)) => Some(Seen::Photograph {
            converts: photograph.profile.is_some() && quietly(|| pictures::converts(bytes)),
            transparent: photograph.alpha && quietly(|| pictures::transparent(bytes)),
            photograph,
        }),
        Ok(Header::Plain(plain)) => Some(Seen::Plain {
            kind: plain.kind,
            size: plain.size.ok(),
        }),
        Err(Unreadable::NotAPicture) => None,
        Err(Unreadable::CutShort) => Some(Seen::Unreadable(PictureFault::CutShort)),
        Err(Unreadable::Unsupported(what)) => {
            Some(Seen::Unreadable(PictureFault::Unsupported(what.to_owned())))
        }
    }
}

thread_local! {
    /// Whether this thread is reading a picture, whose reader may panic
    /// and be caught.
    static READING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

type PanicHook = Box<dyn Fn(&PanicHookInfo<'_>) + Sync + Send + 'static>;

/// `before`, but for the panics of a thread reading a picture.
fn silenced(before: PanicHook) -> PanicHook {
    Box::new(move |info| {
        if !READING.get() {
            before(info);
        }
    })
}

/// Runs `read`, which may catch a panic of a picture's reader, with
/// nothing printed of one: Rust otherwise prints every panic, caught or
/// not, into what the build says.
pub(crate) fn quietly<T>(read: impl FnOnce() -> T) -> T {
    static HOOKED: Once = Once::new();
    HOOKED.call_once(|| std::panic::set_hook(silenced(std::panic::take_hook())));
    let was = READING.replace(true);
    let answer = read();
    READING.set(was);
    answer
}
