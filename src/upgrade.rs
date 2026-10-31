//! `statac upgrade`, the upgrade `statac serve` makes as it starts, and
//! `statac uninstall`.
//!
//! Statac replaces or removes itself only where its install script
//! installed it,
//! which it knows by the marker the script leaves beside the program,
//! `.statac-install`. Anything else is another tool's to upgrade or
//! uninstall, and Statac names that tool's command, guessing the tool from
//! where the program is: the guess picks a command to name, never what is
//! replaced or removed.
//!
//! # The marker
//!
//! The marker is the contract between the install scripts and Statac.
//! It stands in the folder of the program itself, beside `statac`
//! (`statac.exe` on Windows), the program's path being found with every
//! link on the way resolved: where `~/.local/bin/statac` is a link, the
//! marker is beside the file the link leads to. It is UTF-8 text, a
//! setting a line, `name: value`, with spaces round either taken off. A
//! blank line, and a line starting `#`, says nothing. A name Statac
//! doesn't know is passed over and kept, so a later script can add one.
//! A name given twice counts the first time. The names:
//!
//! - `updates`: `on` or `off`, nothing else: whether `statac serve`
//!   installs a fix by itself as it starts, a release with the same major
//!   and minor version as the one running. Missing, it is `off`. `statac
//!   upgrade --updates=on` and `--updates=off` rewrite this line alone,
//!   adding it where it is missing.
//! - `licences`: the folder the script put the licences in, as a whole
//!   path: `LICENSE-MIT`, `LICENSE-APACHE`, `THIRD-PARTY.md` and
//!   `THIRD-PARTY-GRAMMARS.md`, the four files a package carries beside
//!   the program. An upgrade writes the new release's over them, making
//!   the folder where it is gone; an uninstall removes them by name, and
//!   the folder where it then holds nothing. Missing, or not a whole
//!   path, and neither touches the licences.
//! - `path`: on Windows, the entry the script added to the user's PATH,
//!   the program's folder as a whole path. An uninstall takes the entry
//!   off the user's PATH where it is still there. Missing, and the PATH is
//!   left alone, as it is where the folder was on it before the script
//!   ran. Elsewhere the script changes no PATH and writes no `path`.
//!
//! The script writes, with its own answer and folder:
//!
//! ```text
//! # Written by Statac's install script. statac upgrade and statac uninstall read it.
//! updates: off
//! licences: /usr/local/share/statac
//! ```
//!
//! The script for Windows, `tools/install.ps1`, writes the same, with
//! Windows paths, and `path` where it added the folder to the PATH.
//!
//! # What an upgrade leaves beside the program
//!
//! Each name ends with the number of the process that wrote it, and an
//! uninstall removes them with the program and the marker:
//!
//! - `.statac-new-<number>`: a program being written, which takes the
//!   program's name once whole. One is left only where Statac was stopped
//!   while writing it, and the next upgrade removes it once it is an hour
//!   old.
//! - `.statac-old-<number>`: on Windows, the program an upgrade replaced
//!   while it was running, moved aside since Windows won't replace it.
//!   The upgrade removes it where nothing runs it, and otherwise the next
//!   upgrade does once nothing runs it.
//! - `.statac-install-<number>`: the marker being rewritten, left and
//!   removed as a new program is.
//!
//! In the licences' folder a file is written under a hidden name beside
//! it, [`crate::platform::beside`]'s, before it takes the file's own.
//!
//! # A release
//!
//! get.statac.dev holds each release in `v<version>/`: a package for each
//! platform, `statac-<version>-<target>.tar.gz` (`.zip` on Windows), and
//! `SHA256SUMS`, a line for each package, its SHA-256 in hex, two spaces
//! (or a space and an asterisk) and its name. From 0.8.0 `SHA256SUMS.sig`
//! beside it is the Ed25519 signature of `SHA256SUMS`, its 64 bytes as
//! they are, made with the key whose public half is [`KEY`].
//! `SHA256SUMS.sshsig` is the same key's SSH signature of it, which the
//! install script checks with `ssh-keygen` and an upgrade never reads.
//! `latest/SHA256SUMS` is the newest release's, and
//! `latest-<major>.<minor>/SHA256SUMS` the newest of that series'. An
//! upgrade refuses a release whose `SHA256SUMS` isn't signed with
//! [`KEY`], and a package whose SHA-256 isn't the one it lists. The
//! signature is always fetched from `v<version>/`, which is never
//! replaced, so a cache holding an older `latest/` can't pair it with a
//! newer signature.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use rustls::crypto::aws_lc_rs;
use rustls::pki_types::alg_id;

use crate::fetch::{Fetched, Refusal};
use crate::problem::Reason;
use crate::wording;

mod package;

/// Where the releases are published.
const RELEASES: &str = "https://get.statac.dev";

/// The public half of the key each release's `SHA256SUMS` is signed with.
/// Its private half is kept away from the repository by whoever
/// publishes the releases; a release signed with another can't be
/// installed by an upgrade.
const KEY: [u8; 32] = hex("9ba90e0b6ec11a149baad9a54d6cf33d15a872e75c08d915f7361225d32febde");

/// The marker's name.
const MARKER: &str = ".statac-install";

/// The start of the name a new program is written under.
const NEW_PROGRAM: &str = ".statac-new-";

/// The start of the name a replaced program is moved aside to.
const OLD_PROGRAM: &str = ".statac-old-";

/// The start of the name a marker is rewritten under.
const NEW_MARKER: &str = ".statac-install-";

/// How old a file an upgrade left part written is before the next removes
/// it, being surely no upgrade's still running.
const LEFT_FOR: Duration = Duration::from_secs(60 * 60);

/// The licences a package carries beside the program.
const LICENCES: [&str; 4] = [
    "LICENSE-MIT",
    "LICENSE-APACHE",
    "THIRD-PARTY.md",
    "THIRD-PARTY-GRAMMARS.md",
];

/// The most bytes a `SHA256SUMS` may hold, and its signature.
const SUMS_LIMIT: u64 = 64 * 1024;
const SIGNATURE_LIMIT: u64 = 1024;

/// The most bytes a package may hold, and the time its download may take.
const PACKAGE_LIMIT: u64 = 200_000_000;
const PACKAGE_TIME: Duration = Duration::from_secs(10 * 60);

/// The time the smaller files may take.
const SMALL_TIME: Duration = Duration::from_secs(30);

/// The platform this program was built for, as a package names it, or
/// none where no package is published for it.
const TARGET: Option<&str> = if cfg!(all(target_arch = "aarch64", target_os = "macos")) {
    Some("aarch64-apple-darwin")
} else if cfg!(all(target_arch = "x86_64", target_os = "macos")) {
    Some("x86_64-apple-darwin")
} else if cfg!(all(
    target_arch = "aarch64",
    target_os = "linux",
    target_env = "musl"
)) {
    Some("aarch64-unknown-linux-musl")
} else if cfg!(all(
    target_arch = "aarch64",
    target_os = "linux",
    target_env = "gnu"
)) {
    Some("aarch64-unknown-linux-gnu")
} else if cfg!(all(
    target_arch = "x86_64",
    target_os = "linux",
    target_env = "musl"
)) {
    Some("x86_64-unknown-linux-musl")
} else if cfg!(all(
    target_arch = "x86_64",
    target_os = "linux",
    target_env = "gnu"
)) {
    Some("x86_64-unknown-linux-gnu")
} else if cfg!(all(
    target_arch = "x86_64",
    target_os = "windows",
    target_env = "msvc"
)) {
    Some("x86_64-pc-windows-msvc")
} else {
    None
};

/// A release's version, `major.minor.patch`.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct Version {
    major: u64,
    minor: u64,
    patch: u64,
}

impl Version {
    /// `written` read as three whole numbers with full stops between, or
    /// none.
    fn read(written: &str) -> Option<Version> {
        let mut numbers = written.split('.').map(|number| match number {
            "" => None,
            _ if !number.bytes().all(|byte| byte.is_ascii_digit()) => None,
            _ => number.parse().ok(),
        });
        let version = Version {
            major: numbers.next()??,
            minor: numbers.next()??,
            patch: numbers.next()??,
        };
        numbers.next().is_none().then_some(version)
    }

    /// The version running.
    fn running() -> Version {
        Version::read(env!("CARGO_PKG_VERSION")).expect("Cargo.toml's version is tested")
    }

    /// Whether `self` is a fix to `running`: newer, in the same series.
    fn fixes(self, running: Version) -> bool {
        self.series() == running.series() && self > running
    }

    fn series(self) -> (u64, u64) {
        (self.major, self.minor)
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// The marker's lines, as the file holds them.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Marker {
    lines: Vec<String>,
}

impl Marker {
    fn read(text: &str) -> Marker {
        Marker {
            lines: text.lines().map(str::to_owned).collect(),
        }
    }

    /// The value of the first line naming `name`.
    fn value(&self, name: &str) -> Option<&str> {
        self.lines.iter().find_map(|line| {
            let (named, value) = setting(line)?;
            (named == name).then_some(value)
        })
    }

    /// Whether updates are on, or what `updates` is given that is neither
    /// `on` nor `off`.
    fn updates(&self) -> Result<bool, &str> {
        match self.value("updates") {
            None | Some("off") => Ok(false),
            Some("on") => Ok(true),
            Some(other) => Err(other),
        }
    }

    /// The licences' folder, where it is given as a whole path.
    fn licences(&self) -> Option<PathBuf> {
        let folder = PathBuf::from(self.value("licences")?);
        folder.is_absolute().then_some(folder)
    }

    /// The marker's text with `updates` set to `on`, its first line naming
    /// it rewritten and any other taken out, or a line added where none
    /// names it.
    fn with_updates(&self, on: bool) -> String {
        let line = format!("updates: {}", if on { "on" } else { "off" });
        let mut text = String::new();
        let mut written = false;
        for held in &self.lines {
            match setting(held) {
                Some(("updates", _)) if written => continue,
                Some(("updates", _)) => {
                    text.push_str(&line);
                    written = true;
                }
                _ => text.push_str(held),
            }
            text.push('\n');
        }
        if !written {
            text.push_str(&line);
            text.push('\n');
        }
        text
    }
}

/// A marker's line read as a name and a value.
fn setting(line: &str) -> Option<(&str, &str)> {
    let line = line.trim();
    if line.starts_with('#') {
        return None;
    }
    let (name, value) = line.split_once(':')?;
    Some((name.trim(), value.trim()))
}

/// Statac as its install script installed it.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Install {
    /// The program itself, every link on the way resolved.
    program: PathBuf,
    marker: Marker,
}

impl Install {
    fn folder(&self) -> &Path {
        self.program.parent().unwrap_or(Path::new("."))
    }

    fn marker_path(&self) -> PathBuf {
        self.folder().join(MARKER)
    }
}

/// What installed Statac.
#[derive(Clone, PartialEq, Eq, Debug)]
enum Installed {
    Script(Install),
    Other(Installer),
}

/// Another tool that may have installed Statac, guessed from where the
/// program is.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Installer {
    Homebrew,
    Scoop,
    Winget,
    Cargo,
    /// The install script, which can't be told without the marker, and
    /// whose command is named where no tool can be told.
    Script,
    /// None that can be told.
    Unknown,
}

/// What installed the program `program` names, which may be a link: its
/// install script where the marker stands beside the program itself.
fn installed(program: &Path) -> io::Result<Installed> {
    let program = std::fs::canonicalize(program)?;
    let folder = program.parent().unwrap_or(Path::new("."));
    match std::fs::read(folder.join(MARKER)) {
        Ok(bytes) => Ok(Installed::Script(Install {
            marker: Marker::read(&String::from_utf8_lossy(&bytes)),
            program,
        })),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Ok(Installed::Other(installer(&program)))
        }
        Err(error) => Err(error),
    }
}

/// The tool that installed the program at `program`, a path with no link
/// on the way, as far as each tool's own folder tells it.
fn installer(program: &Path) -> Installer {
    let pieces: Vec<String> = program
        .components()
        .map(|piece| piece.as_os_str().to_string_lossy().to_lowercase())
        .collect();
    let within = |names: &[&str]| pieces.windows(names.len()).any(|run| run == names);
    let cargo_root = program.parent().and_then(Path::parent);
    if within(&["cellar", "statac"]) {
        Installer::Homebrew
    } else if within(&["scoop", "apps", "statac"]) {
        Installer::Scoop
    } else if within(&["microsoft", "winget", "packages"]) {
        Installer::Winget
    } else if cargo_root.is_some_and(|root| {
        root.join(".crates2.json").is_file() || root.join(".crates.toml").is_file()
    }) {
        Installer::Cargo
    } else {
        Installer::Unknown
    }
}

/// A tool that may have installed Statac.
struct Tool {
    installer: Installer,
    name: &'static str,
    upgrade: &'static str,
    /// The command that uninstalls Statac: none for the install script,
    /// whose install `statac uninstall` removes.
    uninstall: Option<&'static str>,
    /// Whether it is a tool for Windows.
    windows: bool,
}

const TOOLS: [Tool; 6] = [
    Tool {
        installer: Installer::Homebrew,
        name: "Homebrew",
        upgrade: "brew upgrade statac",
        uninstall: Some("brew uninstall statac"),
        windows: false,
    },
    Tool {
        installer: Installer::Scoop,
        name: "Scoop",
        upgrade: "scoop update statac",
        uninstall: Some("scoop uninstall statac"),
        windows: true,
    },
    Tool {
        installer: Installer::Winget,
        name: "winget",
        upgrade: "winget upgrade Statac.Statac",
        uninstall: Some("winget uninstall Statac.Statac"),
        windows: true,
    },
    Tool {
        installer: Installer::Cargo,
        name: "Cargo",
        upgrade: "cargo install --locked statac",
        uninstall: Some("cargo uninstall statac"),
        windows: false,
    },
    Tool {
        installer: Installer::Script,
        name: "the install script",
        upgrade: "curl -fsSL https://get.statac.dev | sh",
        uninstall: None,
        windows: false,
    },
    Tool {
        installer: Installer::Script,
        name: "the install script",
        upgrade: "irm https://get.statac.dev/install.ps1 | iex",
        uninstall: None,
        windows: true,
    },
];

/// The tools this system has that could have installed Statac.
fn tools_here() -> impl Iterator<Item = &'static Tool> {
    (TOOLS.iter())
        .filter(|tool| tool.windows == cfg!(windows) || tool.installer == Installer::Cargo)
}

/// The line naming the command that upgrades Statac where `installer`
/// installed it, or where that can't be told, the commands of each tool
/// this system has that could have.
fn upgrade_with(installer: Installer) -> String {
    let tool = (TOOLS.iter())
        .find(|tool| tool.installer == installer)
        .map(|tool| (tool.name, tool.upgrade));
    let commands: Vec<&str> = tools_here().map(|tool| tool.upgrade).collect();
    wording::upgrade_with(tool, &commands)
}

/// The line naming the command that uninstalls Statac where `installer`
/// installed it, or where that can't be told, the commands of each tool
/// this system has that could have, and the program at `program` to
/// remove where none did.
fn uninstall_with(installer: Installer, program: &Path) -> String {
    let tool = (TOOLS.iter())
        .find(|tool| tool.installer == installer)
        .and_then(|tool| Some((tool.name, tool.uninstall?)));
    let commands: Vec<&str> = tools_here().filter_map(|tool| tool.uninstall).collect();
    wording::uninstall_with(tool, &commands, &shown(program))
}

/// Where the releases are fetched from, and how.
struct Source<'a> {
    /// The releases' address, with no closing slash.
    releases: &'a str,
    key: &'a [u8; 32],
    /// Fetches an address, given the most bytes and the time it may take.
    fetch: &'a dyn Fn(&str, u64, Duration) -> Result<Fetched, Refusal>,
    target: Option<&'a str>,
    running: Version,
}

impl Source<'static> {
    fn get_statac_dev() -> Source<'static> {
        Source {
            releases: RELEASES,
            key: &KEY,
            fetch: &crate::fetch::fetch_within,
            target: TARGET,
            running: Version::running(),
        }
    }
}

/// A release as its `SHA256SUMS` names it.
struct Named {
    version: Version,
    /// The `SHA256SUMS` as fetched, which its signature is of.
    sums: Vec<u8>,
    /// This platform's package, its name and SHA-256.
    package: Option<(String, [u8; 32])>,
}

/// Why no release could be read.
enum Unchecked {
    Refused(Refusal),
    /// The `SHA256SUMS` at this address doesn't read as one.
    Unreadable(String),
}

impl Unchecked {
    fn said(&self, source: &Source) -> String {
        match self {
            Unchecked::Refused(refusal) => wording::refusal(source.releases, refusal),
            Unchecked::Unreadable(address) => wording::unreadable_sums(address),
        }
    }
}

/// The release `SHA256SUMS` names in the folder `folder` of the releases.
fn named(source: &Source, folder: &str) -> Result<Named, Unchecked> {
    let address = format!("{}/{folder}/SHA256SUMS", source.releases);
    let fetched = (source.fetch)(&address, SUMS_LIMIT, SMALL_TIME).map_err(Unchecked::Refused)?;
    let unreadable = || Unchecked::Unreadable(address.clone());
    let text = std::str::from_utf8(&fetched.bytes).map_err(|_| unreadable())?;
    let mut version = None;
    let mut package = None;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let (hash, name) = sum(line).ok_or_else(unreadable)?;
        let (of, target) = package_name(name).ok_or_else(unreadable)?;
        if version.is_some_and(|version| version != of) {
            return Err(unreadable());
        }
        version = Some(of);
        if Some(target) == source.target {
            package = Some((name.to_owned(), hash));
        }
    }
    Ok(Named {
        version: version.ok_or_else(unreadable)?,
        sums: fetched.bytes,
        package,
    })
}

/// A line of `SHA256SUMS` read as a hash and a name.
fn sum(line: &str) -> Option<([u8; 32], &str)> {
    let (hash, rest) = line.split_once(' ')?;
    let name = rest.strip_prefix([' ', '*'])?;
    let mut bytes = [0; 32];
    if hash.len() != 64 || name.is_empty() {
        return None;
    }
    for (at, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(hash.get(at * 2..at * 2 + 2)?, 16).ok()?;
    }
    Some((bytes, name))
}

/// A package's name, `statac-<version>-<target>.tar.gz` or `.zip`, read
/// as its version and target.
fn package_name(name: &str) -> Option<(Version, &str)> {
    let rest = name.strip_prefix("statac-")?;
    let rest = (rest.strip_suffix(".tar.gz")).or_else(|| rest.strip_suffix(".zip"))?;
    let (version, target) = rest.split_once('-')?;
    Some((Version::read(version)?, target))
}

/// The SHA-256 of `bytes`, through the TLS library Statac fetches with.
fn sha256(bytes: &[u8]) -> [u8; 32] {
    let suite = aws_lc_rs::cipher_suite::TLS13_AES_128_GCM_SHA256
        .tls13()
        .expect("a TLS 1.3 suite");
    let hash = suite.common.hash_provider.hash(bytes);
    hash.as_ref().try_into().expect("a SHA-256 is 32 bytes")
}

/// Whether `signature` is `key`'s Ed25519 signature of `message`, checked
/// by the TLS library Statac fetches with.
fn signed(message: &[u8], signature: &[u8], key: &[u8; 32]) -> bool {
    let provider = aws_lc_rs::default_provider();
    let ed25519 = provider
        .signature_verification_algorithms
        .all
        .iter()
        .find(|algorithm| {
            algorithm.public_key_alg_id() == alg_id::ED25519
                && algorithm.signature_alg_id() == alg_id::ED25519
        });
    ed25519.is_some_and(|algorithm| algorithm.verify_signature(key, message, signature).is_ok())
}

/// `text`, 64 hex digits, as 32 bytes.
const fn hex(text: &str) -> [u8; 32] {
    const fn digit(byte: u8) -> u8 {
        match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            _ => panic!("not a hex digit"),
        }
    }
    let text = text.as_bytes();
    assert!(text.len() == 64);
    let mut bytes = [0; 32];
    let mut at = 0;
    while at < 32 {
        bytes[at] = digit(text[at * 2]) << 4 | digit(text[at * 2 + 1]);
        at += 1;
    }
    bytes
}

/// Installs `release` in place of the program `install` names, and its
/// licences in place of the script's, saying on `downloading` the line
/// that says what it downloads. The program is replaced only once the
/// release's `SHA256SUMS` is found signed with the source's key and the
/// package matches it; where anything stops it, the program is as it
/// was, and the words to follow a colon say why. A line saying the
/// licences couldn't be written is given back, for after the line saying
/// the program was.
fn install_release(
    source: &Source,
    install: &Install,
    release: &Named,
    downloading: &mut dyn FnMut(&str),
) -> Result<Option<String>, String> {
    let version = release.version;
    let target = source.target.unwrap_or("");
    let Some((name, hash)) = &release.package else {
        return Err(wording::no_package(target));
    };
    let signature = format!("{}/v{version}/SHA256SUMS.sig", source.releases);
    let signature = match (source.fetch)(&signature, SIGNATURE_LIMIT, SMALL_TIME) {
        Ok(fetched) => fetched.bytes,
        Err(Refusal::Missing { .. }) => return Err(wording::UNSIGNED.into()),
        Err(refusal) => return Err(wording::refusal(&signature, &refusal)),
    };
    if !signed(&release.sums, &signature, source.key) {
        return Err(wording::SIGNED_OTHERWISE.into());
    }
    remove_left_behind(install.folder());
    // A folder Statac can't write in is found before the download.
    let new = install
        .folder()
        .join(format!("{NEW_PROGRAM}{}", std::process::id()));
    let cant_write = |path: &Path, error: &io::Error| {
        wording::cant_write_program(&shown(path), &Reason::of(error))
    };
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&new)
        .and_then(|_| std::fs::remove_file(&new))
        .map_err(|error| cant_write(&new, &error))?;

    downloading(&wording::downloading(&version.to_string(), target));
    let address = format!("{}/v{version}/{name}", source.releases);
    let package = match (source.fetch)(&address, PACKAGE_LIMIT, PACKAGE_TIME) {
        Ok(fetched) => fetched.bytes,
        Err(Refusal::TimedOut) => return Err(wording::download_took(PACKAGE_TIME)),
        Err(refusal) => return Err(wording::refusal(&address, &refusal)),
    };
    if sha256(&package) != *hash {
        return Err(wording::PACKAGE_DIFFERS.into());
    }
    let folder = name
        .strip_suffix(".tar.gz")
        .or_else(|| name.strip_suffix(".zip"))
        .unwrap_or(name);
    let program_name = match target.contains("-windows-") {
        true => "statac.exe",
        false => "statac",
    };
    let files = package::files(&package, name.ends_with(".zip"))
        .map_err(|_| wording::UNREADABLE_PACKAGE.to_owned())?;
    let in_package = |file: &str| {
        let path = format!("{folder}/{file}");
        files
            .iter()
            .find(|(named, _)| *named == path)
            .map(|(_, bytes)| bytes.as_slice())
    };
    let program = in_package(program_name)
        .ok_or_else(|| wording::no_program(&format!("{folder}/{program_name}")))?;
    let aside = install
        .folder()
        .join(format!("{OLD_PROGRAM}{}", std::process::id()));
    crate::platform::replace_program(&install.program, program, &new, &aside)
        .map_err(|error| cant_write(&install.program, &error))?;

    let Some(licences) = install.marker.licences() else {
        return Ok(None);
    };
    let written = std::fs::create_dir_all(&licences).and_then(|()| {
        LICENCES
            .iter()
            .try_for_each(|licence| match in_package(licence) {
                Some(bytes) => {
                    let fresh = crate::platform::beside(licence);
                    crate::platform::keep_file(&licences, licence, &fresh, bytes)
                }
                None => Ok(()),
            })
    });
    Ok(written
        .err()
        .map(|error| wording::licences_unwritten(&shown(&licences), &Reason::of(&error))))
}

/// Removes what earlier upgrades left in the program's folder `folder`: a
/// program moved aside that nothing runs any more, which Windows refuses
/// to remove while something does, and a file left part written an hour
/// ago or more.
fn remove_left_behind(folder: &Path) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    let ours = std::process::id().to_string();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let left = |start: &str| {
            name.strip_prefix(start)
                .is_some_and(|number| number != ours && number.parse::<u32>().is_ok())
        };
        let old = || {
            entry
                .metadata()
                .and_then(|found| found.modified())
                .is_ok_and(|modified| {
                    SystemTime::now()
                        .duration_since(modified)
                        .is_ok_and(|age| age >= LEFT_FOR)
                })
        };
        if left(OLD_PROGRAM) || ((left(NEW_PROGRAM) || left(NEW_MARKER)) && old()) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// `path` as the author would write it: on Windows, without the prefix a
/// path made whole carries.
fn shown(path: &Path) -> String {
    let path = path.display().to_string();
    match path.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with(r"UNC\") => rest.to_owned(),
        _ => path,
    }
}

/// Does what `statac upgrade` asks, saying on `out` what happened, and
/// gives the exit code: with `updates`, turns updates on or off and
/// installs nothing; without, installs the latest release where it is
/// newer.
pub(crate) fn run(updates: Option<bool>, out: &mut dyn Write) -> u8 {
    let mut say = |text: &str| {
        let _ = out.write_all(text.as_bytes());
        let _ = out.flush();
    };
    let program = match std::env::current_exe() {
        Ok(program) => program,
        Err(error) => {
            say(&format!(
                "{}\n",
                wording::cant_find_program("upgrade", &Reason::of(&error))
            ));
            return 1;
        }
    };
    run_with(&Source::get_statac_dev(), &program, updates, &mut say)
}

fn run_with(
    source: &Source,
    program: &Path,
    updates: Option<bool>,
    say: &mut dyn FnMut(&str),
) -> u8 {
    let mut line = |text: &str| say(&format!("{text}\n"));
    let install = match installed(program) {
        Ok(Installed::Script(install)) => install,
        Ok(Installed::Other(installer)) => {
            line(match updates {
                Some(_) => wording::UPDATES_NOT_SCRIPTED,
                None => wording::UPGRADE_NOT_SCRIPTED,
            });
            line(&upgrade_with(installer));
            return 1;
        }
        Err(error) => {
            let path = shown(&program.parent().unwrap_or(program).join(MARKER));
            line(&wording::cant_read_file(&path, &Reason::of(&error)));
            return 1;
        }
    };
    if let Some(on) = updates {
        return match set_updates(&install, on) {
            Ok(()) => {
                line(&wording::updates(on));
                0
            }
            Err(error) => {
                let path = shown(&install.marker_path());
                line(&wording::cant_write_file(&path, &Reason::of(&error)));
                1
            }
        };
    }
    let latest = match named(source, "latest") {
        Ok(latest) => latest,
        Err(unchecked) => {
            line(&wording::cant_check(&unchecked.said(source)));
            return 1;
        }
    };
    let (from, to) = (source.running.to_string(), latest.version.to_string());
    if latest.version <= source.running {
        line(&wording::nothing_to_upgrade(&from, &to));
        return 0;
    }
    match install_release(source, &install, &latest, &mut line) {
        Ok(licences) => {
            line(&wording::upgraded(&from, &to));
            if let Some(licences) = licences {
                line(&licences);
            }
            0
        }
        Err(reason) => {
            line(&wording::not_upgraded(&to, &reason));
            1
        }
    }
}

/// Rewrites the marker with updates `on` or off: written whole beside it
/// and moved over it.
fn set_updates(install: &Install, on: bool) -> io::Result<()> {
    remove_left_behind(install.folder());
    let fresh = format!("{NEW_MARKER}{}", std::process::id());
    let text = install.marker.with_updates(on);
    crate::platform::keep_file(install.folder(), MARKER, &fresh, text.as_bytes())
}

/// Does what `statac uninstall` asks, saying on `out` what happened, and
/// gives the exit code: where Statac's install script installed it,
/// removes the program, the marker and what upgrades left beside them,
/// the licences, and on Windows the entry the script added to the PATH.
pub(crate) fn uninstall(out: &mut dyn Write) -> u8 {
    let mut say = |text: &str| {
        let _ = out.write_all(text.as_bytes());
        let _ = out.flush();
    };
    let program = match std::env::current_exe() {
        Ok(program) => program,
        Err(error) => {
            say(&format!(
                "{}\n",
                wording::cant_find_program("uninstall", &Reason::of(&error))
            ));
            return 1;
        }
    };
    uninstall_at(&program, Version::running(), &mut say)
}

fn uninstall_at(program: &Path, running: Version, say: &mut dyn FnMut(&str)) -> u8 {
    let mut line = |text: &str| say(&format!("{text}\n"));
    let install = match installed(program) {
        Ok(Installed::Script(install)) => install,
        Ok(Installed::Other(installer)) => {
            let program = std::fs::canonicalize(program).unwrap_or_else(|_| program.into());
            line(wording::UNINSTALL_NOT_SCRIPTED);
            line(&uninstall_with(installer, &program));
            return 1;
        }
        Err(error) => {
            let path = shown(&program.parent().unwrap_or(program).join(MARKER));
            line(&wording::cant_read_file(&path, &Reason::of(&error)));
            return 1;
        }
    };
    let folder = install.folder();
    let aside = folder.join(format!("{OLD_PROGRAM}{}", std::process::id()));
    let once_ended = match crate::platform::remove_program(&install.program, &aside) {
        Ok(once_ended) => once_ended,
        Err(error) => {
            let path = shown(&install.program);
            line(&wording::not_uninstalled(&path, &Reason::of(&error)));
            return 1;
        }
    };
    let mut unremoved = Vec::new();
    let mut remove = |path: &Path| match std::fs::remove_file(path) {
        Ok(()) => true,
        Err(error) if error.kind() == io::ErrorKind::NotFound => false,
        Err(error) => {
            unremoved.push(wording::cant_remove_file(&shown(path), &Reason::of(&error)));
            false
        }
    };
    let numbered = |name: &str, start: &str| {
        name.strip_prefix(start)
            .is_some_and(|number| number.parse::<u32>().is_ok())
    };
    for name in names_in(folder) {
        let left = [OLD_PROGRAM, NEW_PROGRAM, NEW_MARKER]
            .iter()
            .any(|start| numbered(&name, start));
        if left && folder.join(&name) != aside {
            remove(&folder.join(&name));
        }
    }
    remove(&install.marker_path());
    line(&wording::uninstalled(
        &running.to_string(),
        &shown(folder),
        once_ended,
    ));

    if let Some(licences) = install.marker.licences() {
        let mut removed_any = false;
        for licence in LICENCES {
            removed_any |= remove(&licences.join(licence));
        }
        // A licence an upgrade was stopped while writing.
        for name in names_in(&licences) {
            let part_written = LICENCES.iter().any(|licence| {
                (name.strip_prefix(&format!(".{licence}.")))
                    .and_then(|rest| rest.strip_suffix(".tmp"))
                    .is_some_and(|number| number.parse::<u32>().is_ok())
            });
            if part_written {
                remove(&licences.join(&name));
            }
        }
        let holds_other = !names_in(&licences).is_empty();
        let folder_removed = licences.is_dir()
            && !holds_other
            && match std::fs::remove_dir(&licences) {
                Ok(()) => true,
                Err(error) => {
                    let path = shown(&licences);
                    unremoved.push(wording::cant_remove_file(&path, &Reason::of(&error)));
                    false
                }
            };
        if removed_any || folder_removed {
            line(&wording::licences_removed(
                &shown(&licences),
                folder_removed,
                holds_other,
            ));
        }
    }
    let added = (install.marker.value("path")).filter(|entry| cfg!(windows) && !entry.is_empty());
    if let Some(entry) = added {
        match crate::platform::leave_user_path(entry) {
            Ok(true) => line(&wording::path_left(entry)),
            Ok(false) => {}
            Err(error) => unremoved.push(wording::cant_leave_path(entry, &Reason::of(&error))),
        }
    }
    for said in &unremoved {
        line(said);
    }
    u8::from(!unremoved.is_empty())
}

/// The names in the folder `folder`, none where it can't be read.
fn names_in(folder: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    (entries.flatten())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect()
}

/// What `statac serve` does as it starts, on a thread of its own, saying
/// each line on `say`: where Statac's install script installed it and
/// updates are on, installs a fix to the running release, and names a
/// later series, which it leaves to `statac upgrade`. Offline, it checks
/// nothing and says so where updates are on. Anywhere else it says
/// nothing.
pub(crate) fn as_serve_starts(offline: bool, say: &mut dyn FnMut(&str)) {
    let Ok(program) = std::env::current_exe() else {
        return;
    };
    as_serve_starts_with(&Source::get_statac_dev(), &program, offline, say);
}

fn as_serve_starts_with(source: &Source, program: &Path, offline: bool, say: &mut dyn FnMut(&str)) {
    let mut line = |text: &str| say(&format!("{text}\n"));
    let install = match installed(program) {
        Ok(Installed::Script(install)) => install,
        Ok(Installed::Other(_)) => return,
        Err(error) => {
            let path = shown(&program.parent().unwrap_or(program).join(MARKER));
            line(&wording::cant_read_file(&path, &Reason::of(&error)));
            return;
        }
    };
    match install.marker.updates() {
        Ok(true) => {}
        Ok(false) => return,
        Err(written) => {
            let path = shown(&install.marker_path());
            line(&wording::unclear_updates(&path, written));
            return;
        }
    }
    if offline {
        line(wording::UPGRADE_OFFLINE);
        return;
    }
    let running = source.running;
    let latest = match named(source, "latest") {
        Ok(latest) => latest,
        Err(unchecked) => {
            line(&wording::cant_check(&unchecked.said(source)));
            return;
        }
    };
    let (fix, later) = if latest.version.fixes(running) {
        (Some(latest), None)
    } else if latest.version > running {
        let series = format!("latest-{}.{}", running.major, running.minor);
        let fix = match named(source, &series) {
            Ok(fix) if fix.version.fixes(running) => Some(fix),
            Ok(_) | Err(Unchecked::Refused(Refusal::Missing { .. })) => None,
            Err(unchecked) => {
                line(&wording::cant_check(&unchecked.said(source)));
                None
            }
        };
        (fix, Some(latest.version))
    } else {
        (None, None)
    };
    if let Some(fix) = fix {
        let (from, to) = (running.to_string(), fix.version.to_string());
        // Serve says one line of an upgrade, after it.
        match install_release(source, &install, &fix, &mut |_| {}) {
            Ok(licences) => {
                line(&wording::upgraded_while_serving(&from, &to));
                if let Some(licences) = licences {
                    line(&licences);
                }
            }
            Err(reason) => line(&wording::not_upgraded(&to, &reason)),
        }
    }
    if let Some(later) = later {
        line(&wording::later_release(&later.to_string()));
    }
}

#[cfg(test)]
mod tests;
