//! Everything that differs between the systems Statac writes to: which
//! names a disk can hold, which names it holds as one, and how a folder is
//! moved. Every rule here is applied on every machine, the strictest
//! system's way, so a site that builds anywhere builds everywhere.

use std::io;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use unicode_normalization::UnicodeNormalization;

#[cfg(unix)]
mod unix;

/// Why some common disk can't hold a name.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Unwritable {
    /// A name Windows keeps for a device, with or without an extension.
    Device(String),
    /// One of the characters Windows refuses in a name.
    Character(char),
    Control,
    /// A dot at the end, which Windows drops.
    TrailingDot,
    /// A space at the end, which Windows drops.
    TrailingSpace,
    /// Bytes that aren't text, which no address can carry.
    NotText,
}

/// The digits Windows reads in a device name include three superscripts.
const DEVICES: &[&str] = &[
    "CON",
    "PRN",
    "AUX",
    "NUL",
    "COM1",
    "COM2",
    "COM3",
    "COM4",
    "COM5",
    "COM6",
    "COM7",
    "COM8",
    "COM9",
    "COM\u{b9}",
    "COM\u{b2}",
    "COM\u{b3}",
    "LPT1",
    "LPT2",
    "LPT3",
    "LPT4",
    "LPT5",
    "LPT6",
    "LPT7",
    "LPT8",
    "LPT9",
    "LPT\u{b9}",
    "LPT\u{b2}",
    "LPT\u{b3}",
];

const REFUSED: [char; 8] = ['<', '>', ':', '"', '|', '?', '*', '\\'];

/// Why `path`, a relative path written with forward slashes, can't be
/// written on some common disk, if it can't.
pub fn unwritable(path: &str) -> Option<Unwritable> {
    path.split('/').find_map(unwritable_piece)
}

fn unwritable_piece(piece: &str) -> Option<Unwritable> {
    if let Some(refused) = piece.chars().find(|c| REFUSED.contains(c)) {
        return Some(Unwritable::Character(refused));
    }
    if piece.chars().any(char::is_control) {
        return Some(Unwritable::Control);
    }
    if piece.ends_with('.') && piece != "." && piece != ".." {
        return Some(Unwritable::TrailingDot);
    }
    if piece.ends_with(' ') {
        return Some(Unwritable::TrailingSpace);
    }
    let stem = piece.split('.').next().unwrap_or(piece);
    DEVICES
        .contains(&stem.to_uppercase().as_str())
        .then(|| Unwritable::Device(stem.to_owned()))
}

/// How two names a disk holds as one differ.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Alike {
    Exactly,
    /// Only in how their accents are stored, which no one can see.
    ByNormalisation,
    ByCase,
}

/// `path` as common disks compare names: macOS and Windows disks ignore
/// capitals, and macOS disks ignore how an accented letter is stored. Two
/// paths with the same key are one file there.
pub fn folded(path: &str) -> String {
    // ASCII is its own decomposition, and most paths are nothing else.
    if path.is_ascii() {
        return path.to_ascii_lowercase();
    }
    let lowered: String = decomposed(path)
        .chars()
        .flat_map(char::to_lowercase)
        .collect();
    decomposed(&lowered)
}

/// How `a` and `b` differ, given that they fold alike.
pub fn alike(a: &str, b: &str) -> Alike {
    if a == b {
        Alike::Exactly
    } else if decomposed(a) == decomposed(b) {
        Alike::ByNormalisation
    } else {
        Alike::ByCase
    }
}

/// Whether the disk holding `folder`, a whole path that is there, holds
/// names differing only in capitals as one: asked of the nearest folder
/// whose name has a letter, by that name in other capitals, which such a
/// disk finds without listing both.
pub fn ignores_capitals(folder: &Path) -> bool {
    for at in folder.ancestors() {
        let (Some(above), Some(name)) = (at.parent(), at.file_name().and_then(|n| n.to_str()))
        else {
            return false;
        };
        let other: String = name
            .chars()
            .map(|c| match c.is_ascii_lowercase() {
                true => c.to_ascii_uppercase(),
                false => c.to_ascii_lowercase(),
            })
            .collect();
        if other == name {
            continue;
        }
        let both = std::fs::read_dir(above).map(|entries| {
            let names: Vec<_> = entries.flatten().map(|entry| entry.file_name()).collect();
            [name, other.as_str()]
                .iter()
                .all(|spelling| names.iter().any(|listed| listed == spelling))
        });
        return matches!(both, Ok(false)) && std::fs::symlink_metadata(above.join(&other)).is_ok();
    }
    false
}

/// Canonical decomposition, Unicode's NFD.
pub fn decomposed(text: &str) -> String {
    text.nfd().collect()
}

/// `rel`, written with forward slashes, as a path under `root`.
pub fn join(root: &Path, rel: &str) -> PathBuf {
    let mut path = root.to_path_buf();
    path.extend(rel.split('/').filter(|piece| !piece.is_empty()));
    path
}

/// A relative path written with forward slashes whatever the system, or
/// `Err` with a lossy spelling where a piece isn't text.
pub fn slashed(rel: &Path) -> Result<String, String> {
    let mut out = String::new();
    let mut text = true;
    for component in rel.components() {
        if let Component::Normal(piece) = component {
            if !out.is_empty() {
                out.push('/');
            }
            match piece.to_str() {
                Some(piece) => out.push_str(piece),
                None => {
                    text = false;
                    out.push_str(&piece.to_string_lossy());
                }
            }
        }
    }
    if text { Ok(out) } else { Err(out) }
}

/// The pauses between attempts at a move or a removal that Windows refuses
/// for a moment while something else, such as a virus scanner or the search
/// indexer, has a file in the folder open.
const PAUSES: [Duration; 6] = [
    Duration::from_millis(10),
    Duration::from_millis(20),
    Duration::from_millis(50),
    Duration::from_millis(100),
    Duration::from_millis(200),
    Duration::from_millis(500),
];

/// Moves `from` to `to`, which must not exist: Windows refuses to move a
/// folder over anything, and other systems would move one over an empty
/// folder, so the two only agree when nothing is there.
pub fn rename(from: &Path, to: &Path) -> io::Result<()> {
    retrying(|| std::fs::rename(from, to), transient, &PAUSES)
}

/// Why a file wasn't written over.
#[derive(Debug)]
pub enum Unwritten {
    /// It no longer holds what was read.
    Changed,
    /// It has a second name somewhere, which writing it would part it from.
    NamedTwice,
    Io(io::Error),
}

impl From<io::Error> for Unwritten {
    fn from(error: io::Error) -> Self {
        Unwritten::Io(error)
    }
}

/// The hidden name a file's new text is written under, beside it, before
/// it takes the file's own.
pub fn beside(name: &str) -> String {
    format!(".{name}.{}.tmp", std::process::id())
}

/// Puts `text` in place of what the file at `rel`, a path with forward
/// slashes under the folder `root`, holds, where that is still `read`,
/// the file has one name, and neither `root`, a folder on the way nor the
/// file is a link. The text is written whole under [`beside`]'s name,
/// which nothing may have yet, given the file's permissions, and takes
/// the file's name in one step; where that fails it is removed. On Unix
/// no link is followed even where one is put in place while this runs;
/// elsewhere each path is asked what it is before it is used.
pub fn rewrite(root: &Path, rel: &str, read: &[u8], text: &[u8]) -> Result<(), Unwritten> {
    #[cfg(unix)]
    {
        unix::rewrite(root, rel, read, text)
    }
    #[cfg(not(unix))]
    {
        use std::io::Write;
        no_link_to(root, rel)?;
        let path = join(root, rel);
        let found = std::fs::symlink_metadata(&path)?;
        if !named_once(&found) {
            return Err(Unwritten::NamedTwice);
        }
        if std::fs::read(&path)? != read {
            return Err(Unwritten::Changed);
        }
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let new = path.with_file_name(beside(&name));
        let written = (|| -> io::Result<()> {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&new)?;
            file.write_all(text)?;
            file.set_permissions(found.permissions())?;
            file.sync_all()?;
            drop(file);
            retrying(|| std::fs::rename(&new, &path), transient, &PAUSES)
        })();
        if written.is_err() {
            let _ = std::fs::remove_file(&new);
        }
        Ok(written?)
    }
}

/// What became of a file asked to hold other bytes where it stands.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Over {
    /// It held them already, and is untouched.
    Same,
    Written,
    /// It can't be written where it stands: it is no file, it is a link,
    /// it has a second name, it can't be opened, or the system gives no
    /// way to be sure of any of that. Nothing was written.
    Refused,
}

/// Makes the file named `name` in the folder `folder` hold `bytes` by
/// writing over what it holds, which makes no new file and removes none:
/// several times quicker than making the file anew. Only a file with one
/// name is written, never through a link standing where it is, and only
/// where the folder is found to be a folder and no link; the folders
/// above `folder` are the caller's to be sure of. A failure part of the
/// way through leaves the file holding anything.
pub fn write_over(folder: &Path, name: &str, bytes: &[u8]) -> io::Result<Over> {
    #[cfg(unix)]
    {
        unix::write_over(folder, name, bytes)
    }
    #[cfg(not(unix))]
    {
        let _ = (folder, name, bytes);
        Ok(Over::Refused)
    }
}

/// Puts each of two folders on one disk under the other's name in one
/// step, so neither name is ever without its folder. `Ok(false)` where the
/// system or the disk has no such step, and nothing has moved: macOS and
/// Linux have one on most disks, Windows has none.
pub fn exchange(a: &Path, b: &Path) -> io::Result<bool> {
    #[cfg(any(target_vendor = "apple", target_os = "linux", target_os = "android"))]
    {
        unix::exchange(a, b)
    }
    #[cfg(not(any(target_vendor = "apple", target_os = "linux", target_os = "android")))]
    {
        let _ = (a, b);
        Ok(false)
    }
}

/// Refuses `rel`, a path with forward slashes under the folder `root`,
/// where writing or removing a file there would go through a link: where
/// `root`, a folder on the way, or what stands at `rel` itself is one, or
/// where what stands at `rel` is anything but a file. Whatever is not
/// there yet is no link. On Unix each folder is opened by its name in the
/// folder above it, already open, so none is looked up twice; elsewhere
/// each is asked what it is by its path.
pub fn no_link_to(root: &Path, rel: &str) -> io::Result<()> {
    #[cfg(unix)]
    {
        unix::no_link_to(root, rel)
    }
    #[cfg(not(unix))]
    {
        let link = || io::Error::other("it is a link");
        let mut at = root.to_path_buf();
        match std::fs::symlink_metadata(&at) {
            Ok(found) if found.is_symlink() => return Err(link()),
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        }
        let mut pieces = rel.split('/').peekable();
        while let Some(piece) = pieces.next() {
            at.push(piece);
            let found = match std::fs::symlink_metadata(&at) {
                Ok(found) => found,
                Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
                Err(error) => return Err(error),
            };
            let last = pieces.peek().is_none();
            match (found.is_symlink(), last) {
                (true, _) => return Err(link()),
                (false, false) if !found.is_dir() => {
                    return Err(io::Error::from(io::ErrorKind::NotADirectory));
                }
                (false, true) if !found.is_file() => {
                    return Err(io::Error::other("it is not a file"));
                }
                _ => {}
            }
        }
        Ok(())
    }
}

/// Removes the file at `rel`, a path with forward slashes under the
/// folder `root`, refusing as [`no_link_to`] does where `root`, a folder
/// on the way or what stands at `rel` is a link, or where that is no
/// file. Nothing there is no error. On Unix the name is removed from its
/// folder, already open, so a folder on the way swapped for a link while
/// this runs can't send the removal anywhere else.
pub fn remove_file(root: &Path, rel: &str) -> io::Result<()> {
    #[cfg(unix)]
    {
        unix::remove_file(root, rel)
    }
    #[cfg(not(unix))]
    {
        no_link_to(root, rel)?;
        match retrying(|| std::fs::remove_file(join(root, rel)), transient, &PAUSES) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            done => done,
        }
    }
}

/// Makes the folder `rel`, a path with forward slashes under the folder
/// `root`, where nothing has its name yet. The folders on the way must be
/// there already, and neither they nor `root` may be a link. On Unix the
/// folder is made by name in the folder above it, already open.
pub fn make_folder(root: &Path, rel: &str) -> io::Result<()> {
    #[cfg(unix)]
    {
        unix::make_folder(root, rel)
    }
    #[cfg(not(unix))]
    {
        no_link_on_the_way(root, rel)?;
        std::fs::create_dir(join(root, rel))
    }
}

/// Makes the file `rel`, a path with forward slashes under the folder
/// `root`, holding `bytes`, where nothing has its name yet, a link
/// included. The folders on the way must be there already, and neither
/// they nor `root` may be a link. On Unix the file is made by name in the
/// folder above it, already open.
pub fn make_file(root: &Path, rel: &str, bytes: &[u8]) -> io::Result<()> {
    #[cfg(unix)]
    {
        unix::make_file(root, rel, bytes)
    }
    #[cfg(not(unix))]
    {
        use std::io::Write;
        no_link_on_the_way(root, rel)?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(join(root, rel))?;
        file.write_all(bytes)
    }
}

/// The bytes of the file `rel`, a path with forward slashes under the
/// folder `root`, or none where nothing is there. Refused as
/// [`no_link_to`] refuses, where `root`, a folder on the way or what
/// stands at `rel` is a link, or where that is no file. On Unix the file
/// is opened by name in the folder above it, already open, and never
/// through a link.
pub fn read_file(root: &Path, rel: &str) -> io::Result<Option<Vec<u8>>> {
    #[cfg(unix)]
    {
        unix::read_file(root, rel)
    }
    #[cfg(not(unix))]
    {
        no_link_to(root, rel)?;
        match std::fs::read(join(root, rel)) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            read => read.map(Some),
        }
    }
}

/// The file `rel`, a path with forward slashes under the folder `root`,
/// opened to read, refused as [`read_file`] refuses. Nothing there is an
/// error. On Unix the file is opened by name in the folder above it,
/// already open, and never through a link, so a link put in place of the
/// file or a folder on the way after either was looked at is refused.
pub fn open_file(root: &Path, rel: &str) -> io::Result<std::fs::File> {
    #[cfg(unix)]
    {
        unix::open_file(root, rel)
    }
    #[cfg(not(unix))]
    {
        no_link_to(root, rel)?;
        std::fs::File::open(join(root, rel))
    }
}

/// Makes the file `rel`, a path with forward slashes under the folder
/// `root`, hold `bytes`: written whole under the name `fresh` beside it,
/// which nothing may have yet, and given the file's name in one step, so
/// a file there is always whole. Refused where `root` or a folder on the
/// way is a link, or where something other than a file with one name
/// stands at `rel`; the file under `fresh` is removed where it fails. On
/// Unix each is done by name in the folder above, already open, and never
/// through a link.
pub fn keep_file(root: &Path, rel: &str, fresh: &str, bytes: &[u8]) -> io::Result<()> {
    #[cfg(unix)]
    {
        unix::keep_file(root, rel, fresh, bytes)
    }
    #[cfg(not(unix))]
    {
        use std::io::Write;
        let to = join(root, rel);
        let itself = |path: &Path| {
            std::fs::symlink_metadata(path)
                .is_ok_and(|found| found.file_type().is_file() && named_once(&found))
        };
        if std::fs::symlink_metadata(&to).is_ok() && !itself(&to) {
            return Err(io::Error::other("something else has its name"));
        }
        let fresh = to.with_file_name(fresh);
        let written = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&fresh)
            .and_then(|mut file| file.write_all(bytes))
            .and_then(|()| rename(&fresh, &to));
        written.inspect_err(|_| {
            if itself(&fresh) {
                let _ = std::fs::remove_file(&fresh);
            }
        })
    }
}

/// Copies what `source`, a file open to read, holds to a new file at `to`,
/// which nothing may have yet, given the permissions of `source`. On
/// Apple's systems the two share their bytes where the disk can, as a copy
/// made by the system's own copying does.
pub fn copy_file(source: &std::fs::File, to: &Path) -> io::Result<()> {
    #[cfg(target_vendor = "apple")]
    if unix::clone_file(source, to)? {
        return Ok(());
    }
    let mut made = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(to)?;
    io::copy(&mut &*source, &mut made)?;
    made.set_permissions(source.metadata()?.permissions())
}

/// The length of the file `rel`, a path with forward slashes under the
/// folder `root`, and when its bytes last changed, refused as
/// [`open_file`] refuses. The file is asked without being opened, so one
/// that can't be read is asked too. On Unix it is asked by name in the
/// folder above it, already open, and never through a link.
pub fn length_and_time(root: &Path, rel: &str) -> io::Result<(u64, std::time::SystemTime)> {
    #[cfg(unix)]
    {
        unix::length_and_time(root, rel)
    }
    #[cfg(not(unix))]
    {
        no_link_to(root, rel)?;
        let found = std::fs::metadata(join(root, rel))?;
        Ok((found.len(), found.modified()?))
    }
}

/// Opens the file `name` in the folder `folder` for a lock to be taken on
/// it, making it where nothing has the name. Only a file itself is
/// opened, never through a link, and `folder` must be no link. On Unix
/// the file is opened by name in the folder, already open; elsewhere each
/// is asked what it is first, and a file Windows refuses for a moment
/// while another program has it open is asked for again.
pub fn lock_file(folder: &Path, name: &str) -> io::Result<std::fs::File> {
    #[cfg(unix)]
    {
        unix::lock_file(folder, name)
    }
    #[cfg(not(unix))]
    {
        no_link_to(folder, name)?;
        let open = || {
            std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(false)
                .open(folder.join(name))
        };
        retrying(open, transient, &PAUSES)
    }
}

/// Refuses where `root`, or a folder on the way to `rel`, is a link or
/// isn't there.
#[cfg(not(unix))]
fn no_link_on_the_way(root: &Path, rel: &str) -> io::Result<()> {
    let mut at = root.to_path_buf();
    let mut folders = vec![at.clone()];
    if let Some((above, _)) = rel.rsplit_once('/') {
        for piece in above.split('/') {
            at.push(piece);
            folders.push(at.clone());
        }
    }
    for folder in folders {
        let found = std::fs::symlink_metadata(&folder)?;
        if found.is_symlink() {
            return Err(io::Error::other("it is a link"));
        }
        if !found.is_dir() {
            return Err(io::Error::from(io::ErrorKind::NotADirectory));
        }
    }
    Ok(())
}

/// Whether the file `found` describes has one name only. A file with a
/// second name somewhere may be a file outside the site. Windows keeps
/// the count where stable Rust can't read it, so every file passes there.
pub fn named_once(found: &std::fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        std::os::unix::fs::MetadataExt::nlink(found) == 1
    }
    #[cfg(not(unix))]
    {
        let _ = found;
        true
    }
}

/// When anything of the file `found` describes last changed, in seconds
/// and nanoseconds, which no program sets back as it can the time of its
/// bytes, and which disk and which file on it it is. Stable Rust reads
/// none of them on Windows.
pub fn changed_and_placed(found: &std::fs::Metadata) -> Option<((i64, i64), u64, u64)> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Some((
            (found.ctime(), found.ctime_nsec()),
            found.dev(),
            found.ino(),
        ))
    }
    #[cfg(not(unix))]
    {
        let _ = found;
        None
    }
}

/// Whether a folder in `from` can be moved into `to` without copying it,
/// which only two folders on one disk allow. Told by moving an empty
/// folder called `probe` from one to `landing` in the other, neither of
/// which may exist, and removing it.
pub fn moves_between(from: &Path, probe: &str, landing: &Path) -> io::Result<bool> {
    let probe = from.join(probe);
    // One left by a build stopped half way.
    let _ = std::fs::remove_dir(&probe);
    std::fs::create_dir(&probe)?;
    match rename(&probe, landing) {
        Ok(()) => std::fs::remove_dir(landing).map(|()| true),
        Err(error) => {
            std::fs::remove_dir(&probe)?;
            match error.kind() {
                io::ErrorKind::CrossesDevices => Ok(false),
                _ => Err(error),
            }
        }
    }
}

/// Whether `rel`, a path with forward slashes, stays inside the folder it
/// is joined to: no piece climbs out or names a drive or a root.
pub fn stays_inside(rel: &str) -> bool {
    !rel.starts_with('/')
        && rel
            .split('/')
            .all(|piece| piece != "." && piece != ".." && !piece.contains(['\\', ':']))
}

/// Removes a folder and everything in it. A link, whether `path` itself
/// or anywhere inside, is never followed: `path` being one is refused, and
/// a link inside is removed as a link, even one put there while it runs.
pub fn remove_folder(path: &Path) -> io::Result<()> {
    if std::fs::symlink_metadata(path)?.is_symlink() {
        return Err(io::Error::other("it is a link"));
    }
    #[cfg(unix)]
    {
        // Whatever the quick removal leaves, say for want of handles,
        // `remove_dir_all` removes as safely, one entry at a time.
        unix::remove_folder(path).or_else(|_| std::fs::remove_dir_all(path))
    }
    #[cfg(not(unix))]
    retrying(|| std::fs::remove_dir_all(path), transient, &PAUSES)
}

/// The kind of disk holding `path`, where it is one that may not say when
/// a file on it changes: a network share, or a folder shared with a
/// virtual machine or a container. Nothing where the disk is the
/// machine's own, or where the system can't be asked: Windows can't be
/// from stable Rust without a call Statac doesn't make.
pub fn remote_disk(path: &Path) -> Option<String> {
    #[cfg(any(target_vendor = "apple", target_os = "linux", target_os = "android"))]
    {
        unix::remote_disk(path)
    }
    #[cfg(not(any(target_vendor = "apple", target_os = "linux", target_os = "android")))]
    {
        let _ = path;
        None
    }
}

/// How often a disk must be written for the system's reports of changes
/// on it to stay prompt. macOS's come up to a second and a half late
/// after fifteen seconds or more with nothing written. Nothing where they
/// come as promptly whatever the quiet.
pub const KEEPS_REPORTS_PROMPT: Option<Duration> = match cfg!(target_os = "macos") {
    true => Some(Duration::from_secs(10)),
    false => None,
};

/// Whether a disk macOS calls `kind` is remote, `local` being whether the
/// system counts it as one of the machine's own.
pub fn remote_by_name(kind: &str, local: bool) -> bool {
    const REMOTE: [&str; 8] = [
        "nfs", "smbfs", "afpfs", "webdav", "cifs", "macfuse", "osxfuse", "fuse",
    ];
    !local || REMOTE.iter().any(|remote| kind.starts_with(remote))
}

/// The name of the kind of disk Linux numbers `magic`, where it is remote.
pub fn remote_by_number(magic: u32) -> Option<&'static str> {
    Some(match magic {
        0x6969 => "NFS",
        0x517B => "SMB",
        0xFF53_4D42 => "CIFS",
        0xFE53_4D42 => "SMB2",
        // How Linux run inside Windows sees a Windows disk.
        0x0102_1997 => "9P",
        0x6573_5546 => "FUSE",
        0x786F_4256 => "VirtualBox shared folder",
        0x00C3_6400 => "Ceph",
        0x5346_414F => "AFS",
        _ => return None,
    })
}

fn transient(error: &io::Error) -> bool {
    cfg!(windows) && transient_on_windows(error)
}

/// Access denied, a sharing violation and a lock violation: what Windows
/// says while another program holds a file open.
fn transient_on_windows(error: &io::Error) -> bool {
    matches!(error.raw_os_error(), Some(5 | 32 | 33))
}

fn retrying<T>(
    mut attempt: impl FnMut() -> io::Result<T>,
    transient: impl Fn(&io::Error) -> bool,
    pauses: &[Duration],
) -> io::Result<T> {
    let mut pauses = pauses.iter();
    loop {
        match attempt() {
            Err(error) if transient(&error) => match pauses.next() {
                Some(pause) => std::thread::sleep(*pause),
                None => return Err(error),
            },
            done => return done,
        }
    }
}

/// The system's shell, ready to run `line` as it reads a line typed at
/// it: `sh -c` on Unix, `cmd /C` on Windows.
pub fn shell(line: &str) -> std::process::Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let mut command = std::process::Command::new("cmd");
        // `cmd` doesn't read the backslashes Rust would put before each
        // quote of an argument, so the line is handed over as written.
        command.arg("/C").raw_arg(line);
        command
    }
    #[cfg(not(windows))]
    {
        let mut command = std::process::Command::new("sh");
        command.arg("-c").arg(line);
        command
    }
}

/// Whether `code` is the exit code the system's shell gives for a command
/// it couldn't find.
pub fn command_not_found(code: i32) -> bool {
    code == if cfg!(windows) { 9009 } else { 127 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_disk_is_asked_about_capitals_by_a_folder_with_a_letter_in_its_name() {
        let top = std::env::temp_dir().join(format!("statac-capitals-{}", std::process::id()));
        let numbered = top.join("2026");
        let _ = std::fs::remove_dir_all(&top);
        std::fs::create_dir_all(&numbered).unwrap();
        let top = std::fs::canonicalize(&top).unwrap();
        std::fs::write(top.join("probe"), "").unwrap();
        let alike = top.join("PROBE").exists();
        assert_eq!(ignores_capitals(&top), alike);
        assert_eq!(ignores_capitals(&top.join("2026")), alike);
        // Asked by a spelling other than the disk's own, the answer is the same.
        std::fs::create_dir(top.join("Folder")).unwrap();
        assert_eq!(ignores_capitals(&top.join("Folder")), alike);
        if alike {
            assert!(ignores_capitals(&top.join("fOLDER")));
        }
        let _ = std::fs::remove_dir_all(&top);
    }

    #[test]
    fn shared_and_network_disks_are_remote() {
        for kind in ["nfs", "smbfs", "afpfs", "webdav", "macfuse", "fusefs"] {
            assert!(remote_by_name(kind, true), "{kind}");
        }
        assert!(!remote_by_name("apfs", true));
        assert!(!remote_by_name("hfs", true));
        assert!(remote_by_name("apfs", false));
        assert_eq!(remote_by_number(0x6969), Some("NFS"));
        assert_eq!(remote_by_number(0x0102_1997), Some("9P"));
        // ext4 and btrfs.
        assert_eq!(remote_by_number(0xEF53), None);
        assert_eq!(remote_by_number(0x9123_683E), None);
    }

    #[test]
    fn a_folder_that_isnt_there_is_on_no_remote_disk() {
        assert_eq!(remote_disk(Path::new("no/such/folder/anywhere")), None);
        // Whatever this machine answers, asking doesn't fail.
        let _ = remote_disk(&std::env::temp_dir());
    }

    #[test]
    fn device_names_are_unwritable_with_or_without_an_extension() {
        for name in [
            "CON",
            "con",
            "nul.txt",
            "NUL.tar.gz",
            "com1.css",
            "LPT\u{b9}",
        ] {
            assert!(
                matches!(unwritable(name), Some(Unwritable::Device(_))),
                "{name}"
            );
        }
        assert_eq!(
            unwritable("fonts/aux.woff2"),
            Some(Unwritable::Device("aux".into()))
        );
        for name in ["console.css", "null.js", "com10.txt", "icon.png"] {
            assert_eq!(unwritable(name), None, "{name}");
        }
    }

    #[test]
    fn characters_windows_refuses_are_unwritable() {
        assert_eq!(unwritable("a:b.css"), Some(Unwritable::Character(':')));
        assert_eq!(unwritable("what?.txt"), Some(Unwritable::Character('?')));
        assert_eq!(unwritable("a\\b.txt"), Some(Unwritable::Character('\\')));
        assert_eq!(unwritable("a\u{1}b.txt"), Some(Unwritable::Control));
        assert_eq!(unwritable("notes./a.txt"), Some(Unwritable::TrailingDot));
        assert_eq!(unwritable("a.txt "), Some(Unwritable::TrailingSpace));
        assert_eq!(unwritable(".well-known/security.txt"), None);
        assert_eq!(unwritable("Café ölçü/a b.txt"), None);
    }

    #[test]
    fn capitals_and_stored_accents_fold_alike() {
        assert_eq!(folded("Logo.PNG"), folded("logo.png"));
        // One precomposed, one a letter and a combining accent.
        assert_eq!(folded("caf\u{e9}.txt"), folded("cafe\u{301}.txt"));
        assert_eq!(folded("\u{c9}COLE"), folded("e\u{301}cole"));
        assert_ne!(folded("cafe.txt"), folded("caf\u{e9}.txt"));
        assert_eq!(alike("Logo.png", "logo.png"), Alike::ByCase);
        assert_eq!(
            alike("caf\u{e9}.txt", "cafe\u{301}.txt"),
            Alike::ByNormalisation
        );
        assert_eq!(alike("a", "a"), Alike::Exactly);
    }

    #[test]
    fn decomposition_puts_marks_in_canonical_order() {
        // A dot below (class 220) belongs before an acute (class 230),
        // whichever order they were typed in.
        assert_eq!(decomposed("a\u{301}\u{323}"), "a\u{323}\u{301}");
        assert_eq!(decomposed("\u{1ea1}\u{301}"), "a\u{323}\u{301}");
        // Hangul syllables decompose by arithmetic.
        assert_eq!(decomposed("\u{d55c}"), "\u{1112}\u{1161}\u{11ab}");
        assert_eq!(decomposed("\u{ac00}"), "\u{1100}\u{1161}");
        // The Kelvin and Angstrom signs are letters underneath.
        assert_eq!(decomposed("\u{212a}"), "K");
        assert_eq!(decomposed("\u{212b}"), "A\u{30a}");
    }

    #[test]
    fn relative_paths_join_and_come_back_slashed() {
        let joined = join(Path::new("out"), "assets/fonts/a.woff2");
        assert_eq!(
            joined,
            Path::new("out")
                .join("assets")
                .join("fonts")
                .join("a.woff2")
        );
        assert_eq!(
            slashed(Path::new("assets").join("fonts").join("a.woff2").as_path()),
            Ok("assets/fonts/a.woff2".into())
        );
    }

    #[test]
    fn windows_holding_a_file_open_is_worth_another_try() {
        assert!(transient_on_windows(&io::Error::from_raw_os_error(32)));
        assert!(transient_on_windows(&io::Error::from_raw_os_error(5)));
        assert!(!transient_on_windows(&io::Error::from_raw_os_error(2)));
    }

    #[test]
    fn a_transient_refusal_is_retried_until_the_pauses_run_out() {
        let pauses = [Duration::ZERO; 3];
        let busy = || io::Error::other("busy");
        let mut tries = 0;
        let done = retrying(
            || {
                tries += 1;
                if tries < 3 { Err(busy()) } else { Ok(tries) }
            },
            |_| true,
            &pauses,
        );
        assert_eq!(done.unwrap(), 3);

        let mut tries = 0;
        let failed: io::Result<()> = retrying(
            || {
                tries += 1;
                Err(busy())
            },
            |_| true,
            &pauses,
        );
        assert!(failed.is_err());
        assert_eq!(tries, 4);

        let mut tries = 0;
        let lasting: io::Result<()> = retrying(
            || {
                tries += 1;
                Err(busy())
            },
            |_| false,
            &pauses,
        );
        assert!(lasting.is_err());
        assert_eq!(tries, 1);
    }

    #[test]
    fn a_plain_path_folds_as_any_other_does() {
        let slow = |path: &str| {
            let lowered: String = decomposed(path)
                .chars()
                .flat_map(char::to_lowercase)
                .collect();
            decomposed(&lowered)
        };
        for path in ["", "Blog/Post-1.HTML", "a b/C_d.e~f", "\u{7f}Z/@[`{"] {
            assert_eq!(folded(path), slow(path), "{path}");
        }
    }

    #[test]
    fn two_folders_are_exchanged_whole_or_not_at_all() {
        let root =
            std::env::temp_dir().join(format!("statac-platform-swap-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (a, b) = (root.join("a"), root.join("b"));
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        std::fs::write(a.join("a.txt"), "a").unwrap();
        std::fs::write(b.join("b.txt"), "b").unwrap();
        let (in_a, in_b) = match exchange(&a, &b).unwrap() {
            true => ("b.txt", "a.txt"),
            false => ("a.txt", "b.txt"),
        };
        assert!(a.join(in_a).is_file());
        assert!(b.join(in_b).is_file());
        // A name with no folder is refused, not made.
        assert!(matches!(exchange(&a, &root.join("c")), Ok(false) | Err(_)));
        assert!(!root.join("c").exists());
        assert!(a.join(in_a).is_file());
        remove_folder(&root).unwrap();
    }

    #[test]
    fn a_folder_moves_between_two_folders_on_one_disk() {
        let root =
            std::env::temp_dir().join(format!("statac-platform-probe-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("from")).unwrap();
        std::fs::create_dir_all(root.join("to")).unwrap();
        let landing = root.join("to").join("landing");
        assert!(moves_between(&root.join("from"), "probe", &landing).unwrap());
        assert!(!landing.exists());
        assert!(!root.join("from").join("probe").exists());
        // Nowhere to land is an error, and the probe is taken away.
        let nowhere = root.join("missing").join("landing");
        assert!(moves_between(&root.join("from"), "probe", &nowhere).is_err());
        assert!(!root.join("from").join("probe").exists());
        remove_folder(&root).unwrap();
    }

    #[test]
    fn a_file_with_one_name_is_named_once() {
        let root =
            std::env::temp_dir().join(format!("statac-platform-names-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.txt"), "a").unwrap();
        assert!(named_once(&std::fs::metadata(root.join("a.txt")).unwrap()));
        std::fs::hard_link(root.join("a.txt"), root.join("b.txt")).unwrap();
        let twice = named_once(&std::fs::metadata(root.join("a.txt")).unwrap());
        assert_eq!(twice, !cfg!(unix));
        remove_folder(&root).unwrap();
    }

    #[test]
    fn a_file_opens_to_read_by_its_path_under_a_folder() {
        let root =
            std::env::temp_dir().join(format!("statac-platform-open-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("a")).unwrap();
        std::fs::write(root.join("a").join("b.txt"), "b").unwrap();
        let opened = open_file(&root, "a/b.txt").unwrap();
        assert_eq!(io::read_to_string(opened).unwrap(), "b");
        let missing = open_file(&root, "a/c.txt").unwrap_err();
        assert_eq!(missing.kind(), io::ErrorKind::NotFound);
        // A folder is no file to read.
        assert!(open_file(&root, "a").is_err());
        remove_folder(&root).unwrap();
    }

    #[test]
    fn a_files_length_and_time_are_as_the_system_gives_them() {
        let root =
            std::env::temp_dir().join(format!("statac-platform-times-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("a")).unwrap();
        std::fs::write(root.join("a").join("b.txt"), "bytes").unwrap();
        let found = std::fs::metadata(root.join("a").join("b.txt")).unwrap();
        assert_eq!(
            length_and_time(&root, "a/b.txt").unwrap(),
            (5, found.modified().unwrap())
        );
        let missing = length_and_time(&root, "a/c.txt").unwrap_err();
        assert_eq!(missing.kind(), io::ErrorKind::NotFound);
        assert!(length_and_time(&root, "a").is_err());
        remove_folder(&root).unwrap();
    }

    #[test]
    fn a_kept_file_is_replaced_whole_and_leaves_nothing_beside_it() {
        let root =
            std::env::temp_dir().join(format!("statac-platform-keep-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("a")).unwrap();
        keep_file(&root, "a/b.txt", "b.txt.fresh", b"one").unwrap();
        keep_file(&root, "a/b.txt", "b.txt.fresh", b"two").unwrap();
        assert_eq!(std::fs::read(root.join("a").join("b.txt")).unwrap(), b"two");
        let names: Vec<_> = std::fs::read_dir(root.join("a")).unwrap().collect();
        assert_eq!(names.len(), 1);
        // A folder with the name is not written over.
        std::fs::create_dir(root.join("a").join("c")).unwrap();
        let refused = keep_file(&root, "a/c", "c.fresh", b"three").unwrap_err();
        assert_eq!(refused.to_string(), "something else has its name");
        assert!(!root.join("a").join("c.fresh").exists());
        remove_folder(&root).unwrap();
    }

    #[test]
    fn a_file_open_to_read_is_copied_to_a_new_file_only() {
        let root =
            std::env::temp_dir().join(format!("statac-platform-copy-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.txt"), "bytes").unwrap();
        let source = std::fs::File::open(root.join("a.txt")).unwrap();
        copy_file(&source, &root.join("b.txt")).unwrap();
        assert_eq!(std::fs::read(root.join("b.txt")).unwrap(), b"bytes");
        let taken = copy_file(&source, &root.join("b.txt")).unwrap_err();
        assert_eq!(taken.kind(), io::ErrorKind::AlreadyExists);
        remove_folder(&root).unwrap();
    }

    #[test]
    fn a_lock_file_is_made_once_and_locked_by_one_handle_at_a_time() {
        let root =
            std::env::temp_dir().join(format!("statac-platform-lock-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let first = lock_file(&root, "lock").unwrap();
        first.lock().unwrap();
        // Opened again, as another build opens it, the file is the same.
        let second = lock_file(&root, "lock").unwrap();
        assert!(matches!(
            second.try_lock(),
            Err(std::fs::TryLockError::WouldBlock)
        ));
        drop(first);
        second.try_lock().unwrap();
        drop(second);
        assert_eq!(std::fs::read(root.join("lock")).unwrap(), b"");
        // A folder with the name is no file to lock.
        std::fs::create_dir(root.join("folder")).unwrap();
        assert!(lock_file(&root, "folder").is_err());
        remove_folder(&root).unwrap();
    }

    #[test]
    fn a_lock_file_made_by_several_at_once_opens_for_each() {
        let root =
            std::env::temp_dir().join(format!("statac-platform-locks-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for nth in 0..100 {
            let folder = root.join(nth.to_string());
            std::fs::create_dir_all(&folder).unwrap();
            let ready = std::sync::Barrier::new(4);
            std::thread::scope(|scope| {
                for _ in 0..4 {
                    scope.spawn(|| {
                        ready.wait();
                        lock_file(&folder, "lock").unwrap();
                    });
                }
            });
        }
        remove_folder(&root).unwrap();
    }

    #[test]
    fn a_folder_moves_only_where_nothing_stands() {
        let root = std::env::temp_dir().join(format!("statac-platform-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("from")).unwrap();
        std::fs::write(root.join("from").join("a.txt"), "a").unwrap();
        rename(&root.join("from"), &root.join("to")).unwrap();
        assert_eq!(std::fs::read(root.join("to").join("a.txt")).unwrap(), b"a");
        remove_folder(&root).unwrap();
        assert!(!root.exists());
    }
}
