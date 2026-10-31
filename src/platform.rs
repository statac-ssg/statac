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

/// Why a file or folder is refused here rather than by the system. It
/// travels inside an `io::Error` and is worded as a `problem::Reason`.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum Refused {
    /// It is a link, never followed.
    Link,
    /// It is a folder or something else, not a file.
    NotAFile,
    /// Something other than what is wanted there has its name.
    Taken,
    /// A path that ends in no folder's name.
    NamesNoFolder,
}

impl Refused {
    /// The refusal `error` carries, where it is one.
    pub(crate) fn of(error: &io::Error) -> Option<Refused> {
        error.get_ref()?.downcast_ref().copied()
    }
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl std::error::Error for Refused {}

impl From<Refused> for io::Error {
    fn from(refused: Refused) -> io::Error {
        io::Error::other(refused)
    }
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
pub(crate) fn unwritable(path: &str) -> Option<Unwritable> {
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
pub(crate) fn folded(path: &str) -> String {
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
pub(crate) fn alike(a: &str, b: &str) -> Alike {
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
pub(crate) fn ignores_capitals(folder: &Path) -> bool {
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
pub(crate) fn decomposed(text: &str) -> String {
    text.nfd().collect()
}

/// `rel`, written with forward slashes, as a path under `root`.
pub(crate) fn join(root: &Path, rel: &str) -> PathBuf {
    let mut path = root.to_path_buf();
    path.extend(rel.split('/').filter(|piece| !piece.is_empty()));
    path
}

/// `path` made whole, as far as it is there, with the rest as written.
pub(crate) fn whole_to_be(path: &Path) -> PathBuf {
    let mut missing = Vec::new();
    let mut at = path.to_path_buf();
    loop {
        if let Ok(whole) = std::fs::canonicalize(&at) {
            return missing
                .iter()
                .rev()
                .fold(whole, |whole, piece| whole.join(piece));
        }
        match (at.file_name(), at.parent()) {
            (Some(name), Some(parent)) => {
                missing.push(name.to_os_string());
                at = match parent.as_os_str().is_empty() {
                    true => PathBuf::from("."),
                    false => parent.to_path_buf(),
                };
            }
            _ => return path.to_path_buf(),
        }
    }
}

/// A relative path written with forward slashes whatever the system, or
/// `Err` with a lossy spelling where a piece isn't text.
pub(crate) fn slashed(rel: &Path) -> Result<String, String> {
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
pub(crate) fn rename(from: &Path, to: &Path) -> io::Result<()> {
    retrying(|| std::fs::rename(from, to), transient, &PAUSES)
}

/// Puts `bytes` in place of the program at `path`, which may be running,
/// with the program's permissions: written whole at `new`, beside it,
/// which nothing may have yet, and moved to the program's name in one
/// step, so the program there is always whole and one running from it
/// carries on. Windows refuses to replace a program that is running but
/// lets it be renamed, so there the program is first moved to `aside`,
/// and removed from there where nothing runs it; where something does, it
/// stays until it can be. Where any step fails, the program is left as it
/// was and `new` is removed.
pub(crate) fn replace_program(
    path: &Path,
    bytes: &[u8],
    new: &Path,
    aside: &Path,
) -> io::Result<()> {
    use std::io::Write;
    let found = std::fs::metadata(path)?;
    let written = (|| -> io::Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(new)?;
        file.write_all(bytes)?;
        file.set_permissions(found.permissions())?;
        file.sync_all()?;
        drop(file);
        #[cfg(unix)]
        {
            let _ = aside;
            std::fs::rename(new, path)
        }
        #[cfg(not(unix))]
        {
            moved_aside_and_placed(new, path, aside)?;
            let _ = std::fs::remove_file(aside);
            Ok(())
        }
    })();
    if written.is_err() {
        let _ = std::fs::remove_file(new);
    }
    written
}

/// Moves the program at `path` to `aside`, then `new` to `path`, moving
/// the program back where the second move fails.
#[cfg(any(not(unix), test))]
fn moved_aside_and_placed(new: &Path, path: &Path, aside: &Path) -> io::Result<()> {
    retrying(|| std::fs::rename(path, aside), transient, &PAUSES)?;
    retrying(|| std::fs::rename(new, path), transient, &PAUSES).inspect_err(|_| {
        let _ = retrying(|| std::fs::rename(aside, path), transient, &PAUSES);
    })
}

/// Removes the program at `path`, which may be this one, running, and
/// gives whether it goes only once this process has ended. Windows
/// refuses to remove a program that is running but lets it be renamed,
/// so there the program is moved to `aside` and a hidden PowerShell,
/// started here, removes it once this process has ended. Where PowerShell
/// can't be started, the program is moved back.
pub(crate) fn remove_program(path: &Path, aside: &Path) -> io::Result<bool> {
    #[cfg(unix)]
    {
        let _ = aside;
        std::fs::remove_file(path).map(|()| false)
    }
    #[cfg(not(unix))]
    {
        use std::os::windows::process::CommandExt;
        // Its own hidden console, which outlives the author's, and its own
        // group, which Ctrl+C there doesn't reach.
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        retrying(|| std::fs::rename(path, aside), transient, &PAUSES)?;
        let script = removal_once_ended(aside, std::process::id());
        std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-EncodedCommand"])
            .arg(encoded_command(&script))
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW)
            .spawn()
            .map(|_| true)
            .inspect_err(|_| {
                let _ = retrying(|| std::fs::rename(aside, path), transient, &PAUSES);
            })
    }
}

/// The PowerShell that removes the file at `path` once the process `id`
/// has ended, trying for a few seconds after, since Windows may hold the
/// program a moment longer than the process.
#[cfg(any(not(unix), test))]
fn removal_once_ended(path: &Path, id: u32) -> String {
    let path = path.display().to_string();
    // Written as the author would, without the prefix a path made whole
    // carries on Windows.
    let path = match path.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with(r"UNC\") => rest,
        _ => &path,
    };
    // In single quotes PowerShell reads every character as written but a
    // quote, which is doubled; it takes curved quotes for straight ones.
    let path: String = (path.chars())
        .flat_map(|c| match c {
            '\'' | '\u{2018}' | '\u{2019}' | '\u{201a}' | '\u{201b}' => vec![c, c],
            _ => vec![c],
        })
        .collect();
    format!(
        "Wait-Process -Id {id} -ErrorAction SilentlyContinue\n\
         foreach ($try in 1..40) {{\n\
         \x20   try {{ Remove-Item -LiteralPath '{path}' -Force -ErrorAction Stop; break }}\n\
         \x20   catch {{ Start-Sleep -Milliseconds 250 }}\n\
         }}\n"
    )
}

/// `script` as PowerShell's `-EncodedCommand` takes it: UTF-16LE in
/// base64, which no quoting on the command line can disturb.
#[cfg(any(not(unix), test))]
fn encoded_command(script: &str) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let mut text = String::new();
    for chunk in bytes.chunks(3) {
        let joined = chunk.iter().enumerate().fold(0u32, |joined, (at, &byte)| {
            joined | u32::from(byte) << (16 - 8 * at)
        });
        for at in 0..4 {
            text.push(match at <= chunk.len() {
                true => char::from(ALPHABET[(joined >> (18 - 6 * at) & 63) as usize]),
                false => '=',
            });
        }
    }
    text
}

/// Why a file wasn't written over.
#[derive(Debug)]
pub(crate) enum Unwritten {
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
/// elsewhere each path is asked what it is before it is used, and the
/// file is asked and read through one handle.
pub(crate) fn rewrite(root: &Path, rel: &str, read: &[u8], text: &[u8]) -> Result<(), Unwritten> {
    #[cfg(unix)]
    {
        unix::rewrite(root, rel, read, text)
    }
    #[cfg(not(unix))]
    {
        use std::io::{Read, Write};
        no_link_to(root, rel)?;
        let path = join(root, rel);
        let mut file = std::fs::File::open(&path)?;
        let found = itself(&file)?;
        if !named_once(&file) {
            return Err(Unwritten::NamedTwice);
        }
        let mut held = Vec::with_capacity(read.len());
        file.read_to_end(&mut held)?;
        drop(file);
        if held != read {
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
pub(crate) enum Over {
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
pub(crate) fn write_over(folder: &Path, name: &str, bytes: &[u8]) -> io::Result<Over> {
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
pub(crate) fn exchange(a: &Path, b: &Path) -> io::Result<bool> {
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
pub(crate) fn no_link_to(root: &Path, rel: &str) -> io::Result<()> {
    #[cfg(unix)]
    {
        unix::no_link_to(root, rel)
    }
    #[cfg(not(unix))]
    {
        let link = || io::Error::from(Refused::Link);
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
                    return Err(io::Error::from(Refused::NotAFile));
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
pub(crate) fn remove_file(root: &Path, rel: &str) -> io::Result<()> {
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
pub(crate) fn make_folder(root: &Path, rel: &str) -> io::Result<()> {
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
pub(crate) fn make_file(root: &Path, rel: &str, bytes: &[u8]) -> io::Result<()> {
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
pub(crate) fn read_file(root: &Path, rel: &str) -> io::Result<Option<Vec<u8>>> {
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
pub(crate) fn open_file(root: &Path, rel: &str) -> io::Result<std::fs::File> {
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
pub(crate) fn keep_file(root: &Path, rel: &str, fresh: &str, bytes: &[u8]) -> io::Result<()> {
    #[cfg(unix)]
    {
        unix::keep_file(root, rel, fresh, bytes)
    }
    #[cfg(not(unix))]
    {
        use std::io::Write;
        let to = join(root, rel);
        if std::fs::symlink_metadata(&to).is_ok() && !is_file_itself(&to) {
            return Err(io::Error::from(Refused::Taken));
        }
        let fresh = to.with_file_name(fresh);
        let written = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&fresh)
            .and_then(|mut file| file.write_all(bytes))
            .and_then(|()| rename(&fresh, &to));
        written.inspect_err(|_| {
            if is_file_itself(&fresh) {
                let _ = std::fs::remove_file(&fresh);
            }
        })
    }
}

/// A folder written into, which keeps open each folder under it it opens,
/// up to a few dozen, for the work that follows: a file written is
/// reached from the nearest folder above it already open rather than from
/// the top. Each path given is a path with forward slashes under the
/// folder. On Unix each folder is opened by name in the folder above it,
/// already open, and never through a link, so a folder swapped for a link
/// once looked at is refused rather than gone through; one kept open stays
/// the folder it was, wherever it is moved. Elsewhere each is reached by
/// its path.
#[derive(Debug)]
pub(crate) struct Folders {
    #[cfg(unix)]
    open: unix::Folders,
    #[cfg(not(unix))]
    root: PathBuf,
}

impl Folders {
    /// The folder at `root`, nothing in it open yet.
    pub(crate) fn new(root: &Path) -> Folders {
        Folders {
            #[cfg(unix)]
            open: unix::Folders::new(root),
            #[cfg(not(unix))]
            root: root.to_path_buf(),
        }
    }

    /// Makes the folder `rel`, and each folder on the way to it, where
    /// they aren't there.
    pub(crate) fn make_folders(&self, rel: &str) -> io::Result<()> {
        #[cfg(unix)]
        {
            self.open.make_folders(rel)
        }
        #[cfg(not(unix))]
        {
            std::fs::create_dir_all(join(&self.root, rel))
        }
    }

    /// Makes the file `rel` anew, and gives it open to write. Where
    /// `replacing`, what stands at `rel` is removed first, a link as a
    /// link; nothing else already there is ever written through. The
    /// folders on the way must be there already.
    pub(crate) fn make_new_file(&self, rel: &str, replacing: bool) -> io::Result<std::fs::File> {
        #[cfg(unix)]
        {
            self.open.make_new_file(rel, replacing)
        }
        #[cfg(not(unix))]
        {
            let to = join(&self.root, rel);
            if replacing {
                match std::fs::remove_file(&to) {
                    Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
                    _ => {}
                }
            }
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(to)
        }
    }

    /// Copies all `source`, a file open to read, holds, wherever it was
    /// last read to, to the file `rel`, made anew as
    /// [`Folders::make_new_file`] makes it, and given the permissions of
    /// `source`. On Apple's systems the two share their
    /// bytes where the disk can, as a copy made by the system's own
    /// copying does. Gives the copy open to write where it can be opened
    /// so, for its time to be set.
    pub(crate) fn copy_file(
        &self,
        source: &std::fs::File,
        rel: &str,
        replacing: bool,
    ) -> io::Result<Option<std::fs::File>> {
        #[cfg(unix)]
        {
            self.open.copy_file(source, rel, replacing)
        }
        #[cfg(not(unix))]
        {
            let mut made = self.make_new_file(rel, replacing)?;
            copy_whole(source, &mut made)?;
            made.set_permissions(source.metadata()?.permissions())?;
            Ok(Some(made))
        }
    }

    /// The file `rel` opened to read, as [`open_file`] opens it.
    pub(crate) fn open_file(&self, rel: &str) -> io::Result<std::fs::File> {
        #[cfg(unix)]
        {
            self.open.open_file(rel)
        }
        #[cfg(not(unix))]
        {
            open_file(&self.root, rel)
        }
    }

    /// Gives the file `rel` `modified` as the time its bytes last changed,
    /// refused as [`open_file`] refuses.
    pub(crate) fn set_modified(
        &self,
        rel: &str,
        modified: std::time::SystemTime,
    ) -> io::Result<()> {
        #[cfg(unix)]
        {
            // The owner may set a file's times through any handle on it.
            self.open.open_file(rel)?.set_modified(modified)
        }
        #[cfg(not(unix))]
        {
            no_link_to(&self.root, rel)?;
            let file = std::fs::OpenOptions::new()
                .write(true)
                .open(join(&self.root, rel))?;
            file.set_modified(modified)
        }
    }

    /// What the file `rel` is found to be, as [`measure`] finds it.
    pub(crate) fn measure(&self, rel: &str) -> io::Result<Measure> {
        #[cfg(unix)]
        {
            self.open.measure(rel)
        }
        #[cfg(not(unix))]
        {
            measure(&self.root, rel)
        }
    }

    /// Removes what stands at `rel`, whatever it is: a folder with
    /// everything in it, a file, or a link as a link. Nothing there is no
    /// error. `alone` keeps the work on the calling thread: one waiting on
    /// others takes up other work meanwhile, which may ask for a lock it
    /// holds.
    pub(crate) fn remove(&self, rel: &str, alone: bool) -> io::Result<()> {
        #[cfg(unix)]
        {
            self.open.remove(rel, alone)
        }
        #[cfg(not(unix))]
        {
            let path = join(&self.root, rel);
            match std::fs::symlink_metadata(&path) {
                Ok(found) if found.is_dir() => match alone {
                    true => std::fs::remove_dir_all(&path),
                    false => remove_folder(&path),
                },
                // A link to a folder is removed as a folder on Windows.
                Ok(_) => std::fs::remove_file(&path).or_else(|_| std::fs::remove_dir(&path)),
                Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(error),
            }
        }
    }

    /// Removes the file `rel` as [`remove_file`] does.
    pub(crate) fn remove_file(&self, rel: &str) -> io::Result<()> {
        #[cfg(unix)]
        {
            self.open.remove_file(rel)
        }
        #[cfg(not(unix))]
        {
            remove_file(&self.root, rel)
        }
    }

    /// Removes the folder `rel` where it is empty, and fails where it
    /// isn't.
    pub(crate) fn remove_empty_folder(&self, rel: &str) -> io::Result<()> {
        #[cfg(unix)]
        {
            self.open.remove_empty_folder(rel)
        }
        #[cfg(not(unix))]
        {
            std::fs::remove_dir(join(&self.root, rel))
        }
    }

    /// Gives the entry `from` in the folder `folder` the name `to` there.
    pub(crate) fn rename_in(&self, folder: &str, from: &str, to: &str) -> io::Result<()> {
        #[cfg(unix)]
        {
            self.open.rename_in(folder, from, to)
        }
        #[cfg(not(unix))]
        {
            let folder = join(&self.root, folder);
            rename(&folder.join(from), &folder.join(to))
        }
    }
}

/// What [`measure`] finds of a file.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) struct Measure {
    pub(crate) length: u64,
    /// When its bytes last changed.
    pub(crate) modified: std::time::SystemTime,
    /// Whether it has one name only, as [`named_once`] says.
    pub(crate) named_once: bool,
}

/// What the file `rel`, a path with forward slashes under the folder
/// `root`, is found to be, refused as [`open_file`] refuses. The file is
/// asked without being opened to read, so one that can't be read is asked
/// too. On Unix it is asked by name in the folder above it, already open,
/// and never through a link; elsewhere through one handle, opened as
/// itself.
pub(crate) fn measure(root: &Path, rel: &str) -> io::Result<Measure> {
    #[cfg(unix)]
    {
        unix::measure(root, rel)
    }
    #[cfg(not(unix))]
    {
        no_link_to(root, rel)?;
        let file = opened_as_itself(&join(root, rel))?;
        let found = itself(&file)?;
        Ok(Measure {
            length: found.len(),
            modified: found.modified()?,
            named_once: named_once(&file),
        })
    }
}

/// Opens the file `name` in the folder `folder` for a lock to be taken on
/// it, making it where nothing has the name. Only a file itself is
/// opened, never through a link, and `folder` must be no link. On Unix
/// the file is opened by name in the folder, already open; elsewhere each
/// is asked what it is first, and a file Windows refuses for a moment
/// while another program has it open is asked for again.
pub(crate) fn lock_file(folder: &Path, name: &str) -> io::Result<std::fs::File> {
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
            return Err(io::Error::from(Refused::Link));
        }
        if !found.is_dir() {
            return Err(io::Error::from(io::ErrorKind::NotADirectory));
        }
    }
    Ok(())
}

/// `path` opened only to be asked what it is, which a file that can't be
/// read allows, and as itself: a link there is opened as the link, never
/// followed, so what the handle is asked is what stands at `path`. A
/// folder doesn't open.
#[cfg(not(unix))]
fn opened_as_itself(path: &Path) -> io::Result<std::fs::File> {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    std::fs::OpenOptions::new()
        .access_mode(0)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)
}

/// What the file open as `file` is found to be, refusing a link or
/// anything else than a file.
#[cfg(not(unix))]
fn itself(file: &std::fs::File) -> io::Result<std::fs::Metadata> {
    let found = file.metadata()?;
    match found.file_type() {
        kind if kind.is_file() => Ok(found),
        kind if kind.is_symlink() => Err(io::Error::from(Refused::Link)),
        _ => Err(io::Error::from(Refused::NotAFile)),
    }
}

/// Whether the file open as `file` has one name only. A file with a
/// second name somewhere may be a file outside the site. The count is
/// asked of the open file, so it is of the file read through `file`, and
/// where it can't be asked the file doesn't pass.
pub(crate) fn named_once(file: &std::fs::File) -> bool {
    #[cfg(unix)]
    {
        file.metadata()
            .is_ok_and(|found| std::os::unix::fs::MetadataExt::nlink(&found) == 1)
    }
    #[cfg(not(unix))]
    {
        winapi_util::file::information(file).is_ok_and(|found| found.number_of_links() == 1)
    }
}

/// Whether a file itself stands at `path`: not a link to one, nor a
/// folder, nor a file with another name somewhere, which may be a file
/// outside the site.
pub(crate) fn is_file_itself(path: &Path) -> bool {
    #[cfg(unix)]
    {
        std::fs::symlink_metadata(path).is_ok_and(|found| {
            found.file_type().is_file() && std::os::unix::fs::MetadataExt::nlink(&found) == 1
        })
    }
    #[cfg(not(unix))]
    {
        opened_as_itself(path).is_ok_and(|file| itself(&file).is_ok() && named_once(&file))
    }
}

/// When anything of the file `found` describes last changed, in seconds
/// and nanoseconds, which no program sets back as it can the time of its
/// bytes, and which disk and which file on it it is. Stable Rust reads
/// none of them on Windows.
pub(crate) fn changed_and_placed(found: &std::fs::Metadata) -> Option<((i64, i64), u64, u64)> {
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
pub(crate) fn moves_between(from: &Path, probe: &str, landing: &Path) -> io::Result<bool> {
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
pub(crate) fn stays_inside(rel: &str) -> bool {
    !rel.starts_with('/')
        && rel
            .split('/')
            .all(|piece| piece != "." && piece != ".." && !piece.contains(['\\', ':']))
}

/// Removes a folder and everything in it. A link, whether `path` itself
/// or anywhere inside, is never followed: `path` being one is refused, and
/// a link inside is removed as a link, even one put there while it runs.
pub(crate) fn remove_folder(path: &Path) -> io::Result<()> {
    if std::fs::symlink_metadata(path)?.is_symlink() {
        return Err(io::Error::from(Refused::Link));
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
pub(crate) fn remote_disk(path: &Path) -> Option<String> {
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
pub(crate) const KEEPS_REPORTS_PROMPT: Option<Duration> = match cfg!(target_os = "macos") {
    true => Some(Duration::from_secs(10)),
    false => None,
};

/// Whether a disk macOS calls `kind` is remote, `local` being whether the
/// system counts it as one of the machine's own.
#[cfg(any(target_vendor = "apple", test))]
pub(crate) fn remote_by_name(kind: &str, local: bool) -> bool {
    const REMOTE: [&str; 8] = [
        "nfs", "smbfs", "afpfs", "webdav", "cifs", "macfuse", "osxfuse", "fuse",
    ];
    !local || REMOTE.iter().any(|remote| kind.starts_with(remote))
}

/// The name of the kind of disk Linux numbers `magic`, where it is remote.
#[cfg(any(target_os = "linux", target_os = "android", test))]
pub(crate) fn remote_by_number(magic: u32) -> Option<&'static str> {
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
pub(crate) fn shell(line: &str) -> std::process::Command {
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

/// How the system's shell spells the folder a command is to act on.
pub(crate) const SHELL_OUT: &str = match cfg!(windows) {
    true => "%STATAC_OUT%",
    false => "$STATAC_OUT",
};

/// Whether the console Statac writes to shows escape codes as colour.
/// Older Windows consoles print them as text unless a program asks them
/// not to, which takes a system call Statac doesn't make.
pub(crate) fn console_reads_escapes() -> bool {
    let set = |name| std::env::var_os(name).is_some_and(|value| !value.is_empty());
    !cfg!(windows) || set("WT_SESSION") || set("TERM")
}

/// Whether `error`, from looking up a host's name, says no name server
/// knows the name, rather than that none could be asked.
pub(crate) fn no_such_name(error: &io::Error) -> bool {
    match cfg!(windows) {
        true => no_such_name_on_windows(error),
        false => no_such_name_elsewhere(error),
    }
}

/// Windows says so by its code alone.
fn no_such_name_on_windows(error: &io::Error) -> bool {
    error.raw_os_error() == Some(11001)
}

/// The other systems say so only in words: glibc's, musl's and macOS's
/// for a name nobody knows, and glibc's for a name with no address.
fn no_such_name_elsewhere(error: &io::Error) -> bool {
    let said = error.to_string();
    [
        "Name or service not known",
        "Name does not resolve",
        "nodename nor servname provided, or not known",
        "No address associated with hostname",
    ]
    .iter()
    .any(|words| said.contains(words))
}

/// Whether `code` is the exit code the system's shell gives for a command
/// it couldn't find.
pub(crate) fn command_not_found(code: i32) -> bool {
    code == if cfg!(windows) { 9009 } else { 127 }
}

/// Copies all `source` holds to `to`, from its start, as a clone does.
fn copy_whole(source: &std::fs::File, to: &mut std::fs::File) -> io::Result<u64> {
    use std::io::Seek;
    let mut from = source;
    from.rewind()?;
    io::copy(&mut from, to)
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_name_nobody_knows_is_told_from_a_lookup_that_couldnt_be_made() {
        let said = |words: &str| {
            io::Error::other(format!("failed to lookup address information: {words}"))
        };
        for words in [
            "Name or service not known",
            "Name does not resolve",
            "nodename nor servname provided, or not known",
            "No address associated with hostname",
        ] {
            assert!(no_such_name_elsewhere(&said(words)), "{words}");
            assert!(!no_such_name_on_windows(&said(words)), "{words}");
        }
        for words in ["Temporary failure in name resolution", "Try again"] {
            assert!(!no_such_name_elsewhere(&said(words)), "{words}");
        }
        assert!(no_such_name_on_windows(&io::Error::from_raw_os_error(
            11001
        )));
        assert!(!no_such_name_on_windows(&io::Error::from_raw_os_error(
            11002
        )));
    }

    use super::*;

    /// Held by a test starting a program and by one locking a file: a
    /// program being started holds a copy of every open file until it
    /// runs, which keeps a lock held after its file is closed.
    static STARTING: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
        let opened = || std::fs::File::open(root.join("a.txt")).unwrap();
        assert!(named_once(&opened()));
        assert!(is_file_itself(&root.join("a.txt")));
        assert!(measure(&root, "a.txt").unwrap().named_once);
        std::fs::hard_link(root.join("a.txt"), root.join("b.txt")).unwrap();
        assert!(!named_once(&opened()));
        assert!(!is_file_itself(&root.join("a.txt")));
        assert!(!measure(&root, "a.txt").unwrap().named_once);
        // Neither a folder nor nothing is a file itself.
        assert!(!is_file_itself(&root));
        assert!(!is_file_itself(&root.join("c.txt")));
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
    fn a_file_is_measured_as_the_system_gives_it() {
        let root =
            std::env::temp_dir().join(format!("statac-platform-times-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("a")).unwrap();
        std::fs::write(root.join("a").join("b.txt"), "bytes").unwrap();
        let found = std::fs::metadata(root.join("a").join("b.txt")).unwrap();
        assert_eq!(
            measure(&root, "a/b.txt").unwrap(),
            Measure {
                length: 5,
                modified: found.modified().unwrap(),
                named_once: true,
            }
        );
        let missing = measure(&root, "a/c.txt").unwrap_err();
        assert_eq!(missing.kind(), io::ErrorKind::NotFound);
        assert!(measure(&root, "a").is_err());
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
        assert_eq!(Refused::of(&refused), Some(Refused::Taken));
        assert!(!root.join("a").join("c.fresh").exists());
        remove_folder(&root).unwrap();
    }

    #[test]
    fn a_file_read_to_its_end_is_copied_whole() {
        use std::io::Read;
        let root =
            std::env::temp_dir().join(format!("statac-platform-whole-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.txt"), "bytes").unwrap();
        let mut source = std::fs::File::open(root.join("a.txt")).unwrap();
        source.read_to_end(&mut Vec::new()).unwrap();
        let mut made = std::fs::File::create(root.join("b.txt")).unwrap();
        copy_whole(&source, &mut made).unwrap();
        assert_eq!(std::fs::read(root.join("b.txt")).unwrap(), b"bytes");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_file_open_to_read_is_copied_to_a_file_made_anew() {
        let root =
            std::env::temp_dir().join(format!("statac-platform-copy-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.txt"), "bytes").unwrap();
        let source = std::fs::File::open(root.join("a.txt")).unwrap();
        let folders = Folders::new(&root);
        folders.make_folders("c/d").unwrap();
        folders.copy_file(&source, "c/d/b.txt", false).unwrap();
        let copied = root.join("c").join("d").join("b.txt");
        assert_eq!(std::fs::read(&copied).unwrap(), b"bytes");
        let taken = folders.copy_file(&source, "c/d/b.txt", false).unwrap_err();
        assert_eq!(taken.kind(), io::ErrorKind::AlreadyExists);
        std::fs::write(&copied, "other").unwrap();
        folders.copy_file(&source, "c/d/b.txt", true).unwrap();
        assert_eq!(std::fs::read(&copied).unwrap(), b"bytes");
        drop(folders);
        remove_folder(&root).unwrap();
    }

    #[test]
    fn a_lock_file_is_made_once_and_locked_by_one_handle_at_a_time() {
        let _starting = STARTING
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
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
    fn a_running_program_is_replaced_and_runs_on() {
        // The test runs a copy of itself as the program, which says it is
        // running and waits until its input closes.
        if std::env::var_os("STATAC_RUNNING_PROGRAM_TEST").is_some() {
            let mut out = std::io::stdout();
            std::io::Write::write_all(&mut out, b"running\n").unwrap();
            std::io::Write::flush(&mut out).unwrap();
            let _ = std::io::Read::read_to_end(&mut std::io::stdin(), &mut Vec::new());
            return;
        }
        let root =
            std::env::temp_dir().join(format!("statac-platform-program-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let program = root.join(format!("statac{}", std::env::consts::EXE_SUFFIX));
        std::fs::copy(std::env::current_exe().unwrap(), &program).unwrap();
        let starting = STARTING
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut running = std::process::Command::new(&program)
            .arg("platform::tests::a_running_program_is_replaced_and_runs_on")
            .args(["--exact", "--nocapture", "--test-threads=1"])
            .env("STATAC_RUNNING_PROGRAM_TEST", "1")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        drop(starting);
        // Replaced once it runs, as a serve is: macOS can stall a program
        // replaced while it is still being started.
        let mut said = std::io::BufReader::new(running.stdout.take().unwrap());
        let mut line = String::new();
        while !line.contains("running") {
            line.clear();
            let read = std::io::BufRead::read_line(&mut said, &mut line).unwrap();
            assert!(read > 0, "the program ended before it said it was running");
        }
        let (new, aside) = (root.join(".statac-new-1"), root.join(".statac-old-1"));
        replace_program(&program, b"the program after", &new, &aside).unwrap();
        assert_eq!(std::fs::read(&program).unwrap(), b"the program after");
        assert!(!new.exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&program).unwrap().permissions().mode();
            assert_eq!(mode & 0o111, 0o111, "{mode:o}");
        }
        assert!(running.try_wait().unwrap().is_none(), "the program stopped");
        drop(running.stdin.take());
        assert!(running.wait().unwrap().success());
        // Only Windows moves a running program aside.
        if cfg!(unix) {
            assert!(!aside.exists());
        }
        remove_folder(&root).unwrap();
    }

    #[test]
    fn a_program_moved_aside_comes_back_where_its_replacement_cant_take_its_place() {
        let root =
            std::env::temp_dir().join(format!("statac-platform-aside-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let (program, new, aside) = (root.join("statac"), root.join("new"), root.join("aside"));
        std::fs::write(&program, "before").unwrap();
        std::fs::write(&new, "after").unwrap();
        moved_aside_and_placed(&new, &program, &aside).unwrap();
        assert_eq!(std::fs::read(&program).unwrap(), b"after");
        assert_eq!(std::fs::read(&aside).unwrap(), b"before");
        assert!(!new.exists());

        std::fs::remove_file(&aside).unwrap();
        assert!(moved_aside_and_placed(&new, &program, &aside).is_err());
        assert_eq!(std::fs::read(&program).unwrap(), b"after");
        assert!(!aside.exists());
        remove_folder(&root).unwrap();
    }

    #[test]
    fn a_program_is_removed_or_moved_aside_until_this_process_ends() {
        let root =
            std::env::temp_dir().join(format!("statac-platform-remove-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let (program, aside) = (root.join("statac"), root.join(".statac-old-1"));
        std::fs::write(&program, "the program").unwrap();
        let once_ended = remove_program(&program, &aside).unwrap();
        assert!(!program.exists());
        // On Windows it waits beside it for this process, the test's, to end.
        assert_eq!(once_ended, cfg!(windows));
        assert_eq!(aside.exists(), cfg!(windows));
        if !cfg!(windows) {
            remove_folder(&root).unwrap();
        }
    }

    #[test]
    fn powershell_is_handed_the_removal_whole() {
        let script = removal_once_ended(
            Path::new("\\\\?\\C:\\Tools\\O'Neil \u{2019}s\\statac\\.statac-old-7"),
            7,
        );
        assert_eq!(
            script,
            "Wait-Process -Id 7 -ErrorAction SilentlyContinue\n\
             foreach ($try in 1..40) {\n\
             \x20   try { Remove-Item -LiteralPath 'C:\\Tools\\O''Neil \u{2019}\u{2019}s\\statac\\.statac-old-7' -Force -ErrorAction Stop; break }\n\
             \x20   catch { Start-Sleep -Milliseconds 250 }\n\
             }\n"
        );
        // PowerShell's own example of -EncodedCommand, and each length of
        // the last group of three bytes.
        assert_eq!(
            encoded_command("dir \"c:\\program files\" "),
            "ZABpAHIAIAAiAGMAOgBcAHAAcgBvAGcAcgBhAG0AIABmAGkAbABlAHMAIgAgAA=="
        );
        assert_eq!(encoded_command(""), "");
        assert_eq!(encoded_command("a"), "YQA=");
        assert_eq!(encoded_command("ab"), "YQBiAA==");
        assert_eq!(encoded_command("é"), "6QA=");
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
