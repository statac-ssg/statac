//! The upgrade's tests, all on this machine: a server standing in for
//! get.statac.dev, a key of the tests' own standing in for Statac's, and
//! a folder standing in for where the install script put Statac.

use std::collections::HashMap;
use std::io::Read;
use std::net::{SocketAddr, TcpListener};
use std::sync::{Arc, Mutex};
use std::thread;

use rustls::SignatureScheme;
use rustls::pki_types::PrivatePkcs8KeyDer;

use super::package::tests::{tar_gz, zip};
use super::*;
use crate::testing::Scratch;

/// The tests' private key, in PKCS #8, and its public half.
const TEST_PRIVATE: &str = "302e020100300506032b657004220420c8e9544d25e30fee7798a7808d7fc8495fdec2be7890ab945763f9096ee66dcd";
const TEST_KEY: [u8; 32] = hex("530c1dad1c6081a3172af0e023067098707b8619b7725705cbd5bd3e7299cc9b");

/// A platform packaged as a gzipped tar file, and one as a zip file.
const UNIX: &str = "x86_64-unknown-linux-musl";
const WINDOWS: &str = "x86_64-pc-windows-msvc";

fn version(written: &str) -> Version {
    Version::read(written).unwrap()
}

fn bytes_of_hex(text: &str) -> Vec<u8> {
    (0..text.len() / 2)
        .map(|at| u8::from_str_radix(&text[at * 2..at * 2 + 2], 16).unwrap())
        .collect()
}

/// The tests' key's signature of `message`.
fn signature(message: &[u8]) -> Vec<u8> {
    let der = bytes_of_hex(TEST_PRIVATE);
    let key = aws_lc_rs::sign::any_eddsa_type(&PrivatePkcs8KeyDer::from(&der[..])).unwrap();
    let signer = key.choose_scheme(&[SignatureScheme::ED25519]).unwrap();
    signer.sign(message).unwrap()
}

fn hex_of(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// A server on this machine standing in for get.statac.dev, answering
/// each path it holds with its bytes and any other with `404`, and
/// remembering each path asked for.
struct Releases {
    addr: SocketAddr,
    files: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    asked: Arc<Mutex<Vec<String>>>,
}

impl Releases {
    fn start() -> Releases {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let files: Arc<Mutex<HashMap<String, Vec<u8>>>> = Arc::default();
        let asked: Arc<Mutex<Vec<String>>> = Arc::default();
        let (held, heard) = (files.clone(), asked.clone());
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else {
                    continue;
                };
                let mut request = Vec::new();
                let mut buffer = [0; 1024];
                while !request.windows(4).any(|end| end == b"\r\n\r\n") {
                    match stream.read(&mut buffer) {
                        Ok(0) | Err(_) => break,
                        Ok(read) => request.extend(&buffer[..read]),
                    }
                }
                let request = String::from_utf8_lossy(&request);
                let path = request.split(' ').nth(1).unwrap_or_default().to_owned();
                heard.lock().unwrap().push(path.clone());
                let body = held.lock().unwrap().get(&path).cloned();
                let (status, body) = match body {
                    Some(body) => ("200 OK", body),
                    None => ("404 Not Found", b"nothing here".to_vec()),
                };
                let head = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(&body);
            }
        });
        Releases { addr, files, asked }
    }

    fn address(&self) -> String {
        format!("http://{}", self.addr)
    }

    fn put(&self, path: &str, bytes: impl Into<Vec<u8>>) {
        self.files
            .lock()
            .unwrap()
            .insert(path.to_owned(), bytes.into());
    }

    fn remove(&self, path: &str) {
        self.files.lock().unwrap().remove(path);
    }

    fn asked(&self) -> Vec<String> {
        self.asked.lock().unwrap().clone()
    }

    /// Publishes `version` with a package for `target` holding `program`
    /// and the licences, and one for another platform, signed with the
    /// tests' key, and makes each of `folders` name it.
    fn publish(&self, version: &str, target: &str, program: &[u8], folders: &[&str]) {
        let (name, bytes) = package(version, target, program);
        let other = match target {
            UNIX => "aarch64-apple-darwin",
            _ => UNIX,
        };
        let (other_name, other_package) = package(version, other, b"another platform's");
        let sums = format!(
            "{}  {other_name}\n{} *{name}\n",
            hex_of(&sha256(&other_package)),
            hex_of(&sha256(&bytes)),
        );
        self.put(&format!("/v{version}/{name}"), bytes);
        self.put(&format!("/v{version}/{other_name}"), other_package);
        self.put(&format!("/v{version}/SHA256SUMS"), sums.clone());
        self.put(
            &format!("/v{version}/SHA256SUMS.sig"),
            signature(sums.as_bytes()),
        );
        for folder in folders {
            self.put(&format!("/{folder}/SHA256SUMS"), sums.clone());
        }
    }
}

/// A release's package for `target`, its name and bytes, holding
/// `program` and the four licences, each saying which it is.
fn package(version: &str, target: &str, program: &[u8]) -> (String, Vec<u8>) {
    let folder = format!("statac-{version}-{target}");
    let licence = |name: &str| format!("{name} of {version}").into_bytes();
    let licences: Vec<(String, Vec<u8>)> = LICENCES
        .iter()
        .map(|name| (format!("{folder}/{name}"), licence(name)))
        .collect();
    match target.contains("-windows-") {
        true => {
            // As the Windows package is made, with backslashes.
            let program_path = format!("{folder}\\statac.exe");
            let licences: Vec<(String, &[u8])> = (licences.iter())
                .map(|(path, bytes)| (path.replace('/', "\\"), bytes.as_slice()))
                .collect();
            let mut files: Vec<(&str, &[u8])> = vec![(&program_path, program)];
            files.extend(licences.iter().map(|(path, bytes)| (path.as_str(), *bytes)));
            (format!("{folder}.zip"), zip(&files))
        }
        false => {
            let program_path = format!("{folder}/statac");
            let mut entries: Vec<(&str, u8, &[u8])> =
                vec![(&folder, b'5', b""), (&program_path, b'0', program)];
            entries.extend(
                (licences.iter()).map(|(path, bytes)| (path.as_str(), b'0', bytes.as_slice())),
            );
            (format!("{folder}.tar.gz"), tar_gz(&entries))
        }
    }
}

/// A folder standing in for an install by the script: the program, the
/// marker beside it, and a folder for the licences.
struct Installed {
    scratch: Scratch,
    program: PathBuf,
}

impl Installed {
    fn new(label: &str, marker: Option<&str>) -> Installed {
        let scratch = Scratch::new(&format!("upgrade-{label}"));
        let program = scratch.write("bin/statac", "the program before");
        if let Some(marker) = marker {
            let licences = scratch.join("share").display().to_string();
            scratch.write(
                &format!("bin/{MARKER}"),
                marker.replace("<licences>", &licences),
            );
        }
        Installed { scratch, program }
    }

    fn scripted(label: &str, updates: &str) -> Installed {
        Installed::new(
            label,
            Some(&format!(
                "# Written by Statac's install script. statac upgrade reads it.\n\
                 updates: {updates}\nlicences: <licences>\n"
            )),
        )
    }

    fn program_holds(&self) -> String {
        String::from_utf8(std::fs::read(&self.program).unwrap()).unwrap()
    }

    fn licence(&self, name: &str) -> Option<String> {
        std::fs::read_to_string(self.scratch.join("share").join(name)).ok()
    }

    fn marker(&self) -> String {
        std::fs::read_to_string(self.scratch.join("bin").join(MARKER)).unwrap()
    }

    /// The names in the program's folder.
    fn beside(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(self.scratch.join("bin"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}

/// What a run against `releases` said and its exit code, with `running`
/// the version running and `target` the platform.
fn upgrading(
    releases: &str,
    allowed: SocketAddr,
    running: &str,
    target: &str,
    program: &Path,
    updates: Option<bool>,
) -> (String, u8) {
    let fetch = move |address: &str, limit, time| {
        crate::fetch::fetch_allowing(&[allowed], address, limit, time)
    };
    let source = Source {
        releases,
        key: &TEST_KEY,
        fetch: &fetch,
        target: Some(target),
        running: version(running),
    };
    let mut said = String::new();
    let code = run_with(&source, program, updates, &mut |text| said.push_str(text));
    (said, code)
}

/// What serve's check against `releases` said.
fn serving(releases: &Releases, running: &str, program: &Path, offline: bool) -> String {
    let allowed = releases.addr;
    let fetch = move |address: &str, limit, time| {
        crate::fetch::fetch_allowing(&[allowed], address, limit, time)
    };
    let address = releases.address();
    let source = Source {
        releases: &address,
        key: &TEST_KEY,
        fetch: &fetch,
        target: Some(UNIX),
        running: version(running),
    };
    let mut said = String::new();
    as_serve_starts_with(&source, program, offline, &mut |text| said.push_str(text));
    said
}

fn upgrade(
    releases: &Releases,
    running: &str,
    target: &str,
    installed: &Installed,
) -> (String, u8) {
    upgrading(
        &releases.address(),
        releases.addr,
        running,
        target,
        &installed.program,
        None,
    )
}

#[test]
fn a_newer_release_replaces_the_program_and_the_licences() {
    for target in [UNIX, WINDOWS] {
        let releases = Releases::start();
        releases.publish("0.8.1", target, b"the program after", &["latest"]);
        let installed = Installed::scripted("newer", "off");
        assert_eq!(
            upgrade(&releases, "0.8.0", target, &installed),
            (
                format!(
                    "Downloading Statac 0.8.1 ({target}).\nUpgraded Statac from 0.8.0 to 0.8.1.\n"
                ),
                0
            )
        );
        assert_eq!(installed.program_holds(), "the program after");
        for name in LICENCES {
            assert_eq!(installed.licence(name), Some(format!("{name} of 0.8.1")));
        }
        // Nothing is left beside the program but the marker.
        assert_eq!(installed.beside(), [MARKER, "statac"]);
        let name = match target {
            UNIX => format!("statac-0.8.1-{UNIX}.tar.gz"),
            _ => format!("statac-0.8.1-{WINDOWS}.zip"),
        };
        assert_eq!(
            releases.asked(),
            [
                "/latest/SHA256SUMS".to_owned(),
                "/v0.8.1/SHA256SUMS.sig".to_owned(),
                format!("/v0.8.1/{name}"),
            ]
        );
    }
}

#[test]
fn a_marker_naming_no_licences_leaves_them_alone() {
    let releases = Releases::start();
    releases.publish("0.8.1", UNIX, b"the program after", &["latest"]);
    let installed = Installed::new("no-licences", Some("updates: off\n"));
    assert_eq!(upgrade(&releases, "0.8.0", UNIX, &installed).1, 0);
    assert_eq!(installed.program_holds(), "the program after");
    assert_eq!(installed.licence("LICENSE-MIT"), None);
}

#[test]
fn with_nothing_newer_nothing_changes() {
    let releases = Releases::start();
    releases.publish("0.8.0", UNIX, b"the program after", &["latest"]);
    let installed = Installed::scripted("nothing-newer", "off");
    assert_eq!(
        upgrade(&releases, "0.8.0", UNIX, &installed),
        (
            "Nothing to upgrade: Statac 0.8.0 is the latest release.\n".into(),
            0
        )
    );
    assert_eq!(
        upgrade(&releases, "0.9.0", UNIX, &installed),
        (
            "Nothing to upgrade: this is Statac 0.9.0, newer than the latest release, 0.8.0.\n"
                .into(),
            0
        )
    );
    assert_eq!(installed.program_holds(), "the program before");
    assert_eq!(
        releases.asked(),
        ["/latest/SHA256SUMS", "/latest/SHA256SUMS"]
    );
}

/// What an upgrade to 0.8.1 said once `tamper` had changed what is
/// published, with the program and the licences checked unchanged.
fn refused(label: &str, tamper: impl FnOnce(&Releases)) -> String {
    let releases = Releases::start();
    releases.publish("0.8.1", UNIX, b"the program after", &["latest"]);
    tamper(&releases);
    let installed = Installed::scripted(label, "off");
    let (said, code) = upgrade(&releases, "0.8.0", UNIX, &installed);
    assert_eq!(code, 1, "{said}");
    assert_eq!(installed.program_holds(), "the program before", "{said}");
    assert_eq!(installed.licence("LICENSE-MIT"), None, "{said}");
    assert_eq!(installed.beside(), [MARKER, "statac"], "{said}");
    said
}

#[test]
fn a_package_that_doesnt_match_its_sum_is_refused() {
    let name = format!("/v0.8.1/statac-0.8.1-{UNIX}.tar.gz");
    let said = refused("tampered-package", |releases| {
        let (_, other) = package("0.8.1", UNIX, b"a program of someone else's");
        releases.put(&name, other);
    });
    assert_eq!(
        said,
        format!(
            "Downloading Statac 0.8.1 ({UNIX}).\n\
             Didn't upgrade Statac to 0.8.1: the download doesn't match the release's SHA256SUMS.\n"
        )
    );
}

#[test]
fn sums_that_dont_match_their_signature_are_refused() {
    let said = refused("tampered-sums", |releases| {
        // The package changed, and its sum with it, but not the signature.
        let (name, other) = package("0.8.1", UNIX, b"a program of someone else's");
        let sums = format!("{}  {name}\n", hex_of(&sha256(&other)));
        releases.put(&format!("/v0.8.1/{name}"), other);
        releases.put("/v0.8.1/SHA256SUMS", sums.clone());
        releases.put("/latest/SHA256SUMS", sums);
    });
    assert_eq!(
        said,
        "Didn't upgrade Statac to 0.8.1: its SHA256SUMS isn't signed with Statac's key.\n"
    );
}

#[test]
fn sums_signed_with_another_key_are_refused() {
    let said = refused("other-key", |releases| {
        let sums = releases.files.lock().unwrap()["/latest/SHA256SUMS"].clone();
        let mut forged = signature(&sums);
        forged[0] ^= 1;
        releases.put("/v0.8.1/SHA256SUMS.sig", forged);
    });
    assert_eq!(
        said,
        "Didn't upgrade Statac to 0.8.1: its SHA256SUMS isn't signed with Statac's key.\n"
    );
}

#[test]
fn sums_with_no_signature_are_refused() {
    let said = refused("unsigned", |releases| {
        releases.remove("/v0.8.1/SHA256SUMS.sig");
    });
    assert_eq!(
        said,
        "Didn't upgrade Statac to 0.8.1: its SHA256SUMS has no signature.\n"
    );
}

#[test]
fn a_release_with_no_package_for_this_machine_is_refused() {
    let said = refused("no-package", |releases| {
        let sums = releases.files.lock().unwrap()["/latest/SHA256SUMS"].clone();
        let sums = String::from_utf8(sums).unwrap();
        let sums: String = sums
            .lines()
            .take(1)
            .map(|line| format!("{line}\n"))
            .collect();
        releases.put("/latest/SHA256SUMS", sums.clone());
        releases.put("/v0.8.1/SHA256SUMS.sig", signature(sums.as_bytes()));
    });
    assert_eq!(
        said,
        format!(
            "Didn't upgrade Statac to 0.8.1: the release has no package for this machine, `{UNIX}`.\n"
        )
    );
}

#[test]
fn a_package_holding_no_program_is_refused() {
    let said = refused("no-program", |releases| {
        let name = format!("statac-0.8.1-{UNIX}.tar.gz");
        let package = tar_gz(&[("statac-0.8.1-elsewhere/statac", b'0', b"misplaced")]);
        let sums = format!("{}  {name}\n", hex_of(&sha256(&package)));
        releases.put(&format!("/v0.8.1/{name}"), package);
        releases.put("/latest/SHA256SUMS", sums.clone());
        releases.put("/v0.8.1/SHA256SUMS.sig", signature(sums.as_bytes()));
    });
    assert_eq!(
        said,
        format!(
            "Downloading Statac 0.8.1 ({UNIX}).\n\
             Didn't upgrade Statac to 0.8.1: the package holds no `statac-0.8.1-{UNIX}/statac`.\n"
        )
    );
}

#[test]
fn with_no_answer_nothing_changes() {
    // A port nothing listens on.
    let closed = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap();
    let installed = Installed::scripted("offline", "off");
    let (said, code) = upgrading(
        &format!("http://{closed}"),
        closed,
        "0.8.0",
        UNIX,
        &installed.program,
        None,
    );
    assert!(
        said.starts_with("Can't check for a newer Statac: "),
        "{said}"
    );
    assert_eq!(code, 1);
    assert_eq!(installed.program_holds(), "the program before");

    let releases = Releases::start();
    let (said, code) = upgrade(&releases, "0.8.0", UNIX, &installed);
    assert_eq!(
        said,
        "Can't check for a newer Statac: the server has nothing at that URL.\n"
    );
    assert_eq!(code, 1);
    releases.put("/latest/SHA256SUMS", "not a list of sums\n");
    let (said, _) = upgrade(&releases, "0.8.0", UNIX, &installed);
    assert_eq!(
        said,
        format!(
            "Can't check for a newer Statac: `{}/latest/SHA256SUMS` isn't a list of hashes and package names.\n",
            releases.address()
        )
    );
    assert_eq!(installed.program_holds(), "the program before");
}

#[test]
fn another_tools_install_is_left_to_its_command() {
    let releases = Releases::start();
    let ways = match cfg!(windows) {
        true => {
            "`scoop update statac`, `winget upgrade Statac.Statac` or `cargo install --locked statac`"
        }
        false => {
            "`brew upgrade statac`, `cargo install --locked statac` or `curl -fsSL https://get.statac.dev | sh`"
        }
    };
    let cases = [
        (
            "bin/statac",
            None,
            format!("Upgrade it the way it was installed: {ways}."),
        ),
        (
            "Cellar/statac/0.8.0/bin/statac",
            None,
            "Homebrew installed it, so upgrade it with `brew upgrade statac`.".into(),
        ),
        (
            "scoop/apps/statac/current/statac.exe",
            None,
            "Scoop installed it, so upgrade it with `scoop update statac`.".into(),
        ),
        (
            "Microsoft/WinGet/Packages/Statac.Statac_Microsoft.Winget.Source/statac.exe",
            None,
            "winget installed it, so upgrade it with `winget upgrade Statac.Statac`.".into(),
        ),
        (
            ".cargo/bin/statac",
            Some(".cargo/.crates2.json"),
            "Cargo installed it, so upgrade it with `cargo install --locked statac`.".into(),
        ),
    ];
    for (path, beside, line) in cases {
        let scratch = Scratch::new("upgrade-other");
        let program = scratch.write(path, "another tool's program");
        if let Some(beside) = beside {
            scratch.write(beside, "{}");
        }
        for (updates, first) in [
            (None, wording::UPGRADE_NOT_SCRIPTED),
            (Some(true), wording::UPDATES_NOT_SCRIPTED),
        ] {
            let said = upgrading(
                &releases.address(),
                releases.addr,
                "0.8.0",
                UNIX,
                &program,
                updates,
            );
            assert_eq!(said, (format!("{first}\n{line}\n"), 1), "{path}");
        }
        assert_eq!(std::fs::read(&program).unwrap(), b"another tool's program");
    }
    assert!(releases.asked().is_empty());
}

#[cfg(unix)]
#[test]
fn the_marker_is_found_beside_the_program_a_link_leads_to() {
    let releases = Releases::start();
    releases.publish("0.8.1", UNIX, b"the program after", &["latest"]);
    let installed = Installed::scripted("link", "off");
    let link = installed.scratch.join("linked/statac");
    std::fs::create_dir_all(link.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&installed.program, &link).unwrap();
    let (said, code) = upgrading(
        &releases.address(),
        releases.addr,
        "0.8.0",
        UNIX,
        &link,
        None,
    );
    assert_eq!(code, 0, "{said}");
    assert_eq!(installed.program_holds(), "the program after");
    // The link still leads to the program.
    assert!(std::fs::symlink_metadata(&link).unwrap().is_symlink());
    assert_eq!(std::fs::read(&link).unwrap(), b"the program after");
}

#[test]
fn updates_are_turned_on_and_off_in_the_marker_alone() {
    let releases = Releases::start();
    let installed = Installed::new(
        "updates",
        Some("# A comment\nlicences: /somewhere\nlater: kept\nupdates:  off \nupdates: on\n"),
    );
    let set = |on| {
        upgrading(
            &releases.address(),
            releases.addr,
            "0.8.0",
            UNIX,
            &installed.program,
            Some(on),
        )
    };
    assert_eq!(
        set(true),
        (
            "Statac keeps itself up to date: `statac serve` installs each fix as it starts.\n\
             Turn this off with `statac upgrade --updates=off`.\n"
                .into(),
            0
        )
    );
    assert_eq!(
        installed.marker(),
        "# A comment\nlicences: /somewhere\nlater: kept\nupdates: on\n"
    );
    assert_eq!(
        set(false),
        (
            "Statac doesn't keep itself up to date.\n\
             Turn this on with `statac upgrade --updates=on`.\n"
                .into(),
            0
        )
    );
    assert_eq!(
        installed.marker(),
        "# A comment\nlicences: /somewhere\nlater: kept\nupdates: off\n"
    );
    assert_eq!(installed.beside(), [MARKER, "statac"]);
    assert_eq!(installed.program_holds(), "the program before");
    assert!(releases.asked().is_empty());

    let unanswered = Installed::new("updates-unanswered", Some("licences: /somewhere"));
    let (_, code) = upgrading(
        &releases.address(),
        releases.addr,
        "0.8.0",
        UNIX,
        &unanswered.program,
        Some(true),
    );
    assert_eq!(code, 0);
    assert_eq!(unanswered.marker(), "licences: /somewhere\nupdates: on\n");
}

#[test]
fn the_markers_lines_are_read_as_settings() {
    // A whole path on any system.
    let licences = std::env::temp_dir().join("statac");
    let marker = Marker::read(&format!(
        "# updates: on\n\n  updates :  on  \nupdates: off\nlicences: {}\nlater: kept\n",
        licences.display()
    ));
    assert_eq!(marker.updates(), Ok(true));
    assert_eq!(marker.licences(), Some(licences));
    assert_eq!(marker.value("later"), Some("kept"));
    assert_eq!(Marker::read("").updates(), Ok(false));
    assert_eq!(Marker::read("updates: off").updates(), Ok(false));
    assert_eq!(Marker::read("updates: yes").updates(), Err("yes"));
    assert_eq!(Marker::read("updates: On").updates(), Err("On"));
    assert_eq!(Marker::read("licences: share/statac").licences(), None);
}

#[test]
fn serve_installs_a_fix_where_updates_are_on() {
    let releases = Releases::start();
    releases.publish("0.8.1", UNIX, b"the program after", &["latest"]);
    let installed = Installed::scripted("serve-fix", "on");
    assert_eq!(
        serving(&releases, "0.8.0", &installed.program, false),
        "Upgraded Statac from 0.8.0 to 0.8.1. This serve carries on with 0.8.0 until you start it again.\n"
    );
    assert_eq!(installed.program_holds(), "the program after");
    assert_eq!(
        installed.licence("THIRD-PARTY.md"),
        Some("THIRD-PARTY.md of 0.8.1".into())
    );
}

#[test]
fn serve_names_a_later_series_and_installs_only_its_own_fix() {
    let releases = Releases::start();
    releases.publish("0.8.2", UNIX, b"the fix", &["latest-0.8"]);
    releases.publish(
        "0.9.0",
        UNIX,
        b"the later series",
        &["latest", "latest-0.9"],
    );
    let installed = Installed::scripted("serve-later", "on");
    assert_eq!(
        serving(&releases, "0.8.0", &installed.program, false),
        "Upgraded Statac from 0.8.0 to 0.8.2. This serve carries on with 0.8.0 until you start it again.\n\
         Statac 0.9.0 is released. Run `statac upgrade` to install it.\n"
    );
    assert_eq!(installed.program_holds(), "the fix");

    // With no fix in its own series, only the later one is named.
    let releases = Releases::start();
    releases.publish("0.9.0", UNIX, b"the later series", &["latest"]);
    let installed = Installed::scripted("serve-later-only", "on");
    assert_eq!(
        serving(&releases, "0.8.0", &installed.program, false),
        "Statac 0.9.0 is released. Run `statac upgrade` to install it.\n"
    );
    assert_eq!(installed.program_holds(), "the program before");
    assert_eq!(
        releases.asked(),
        ["/latest/SHA256SUMS", "/latest-0.8/SHA256SUMS"]
    );
}

#[test]
fn serve_says_nothing_and_asks_nothing_where_updates_are_off() {
    let releases = Releases::start();
    releases.publish("0.8.1", UNIX, b"the program after", &["latest"]);
    for updates in ["off", "on"] {
        let installed = Installed::scripted("serve-off", updates);
        let offline = updates == "on";
        let said = serving(&releases, "0.8.0", &installed.program, offline);
        let expected = match offline {
            true => "Offline: Statac doesn't check for a newer release of itself.\n",
            false => "",
        };
        assert_eq!(said, expected);
        assert_eq!(installed.program_holds(), "the program before");
    }
    let other = Installed::new("serve-other", None);
    assert_eq!(serving(&releases, "0.8.0", &other.program, false), "");
    assert!(releases.asked().is_empty());
}

#[test]
fn serve_says_in_one_line_why_it_didnt_upgrade() {
    let releases = Releases::start();
    releases.publish("0.8.1", UNIX, b"the program after", &["latest"]);
    releases.remove("/v0.8.1/SHA256SUMS.sig");
    let installed = Installed::scripted("serve-unsigned", "on");
    assert_eq!(
        serving(&releases, "0.8.0", &installed.program, false),
        "Didn't upgrade Statac to 0.8.1: its SHA256SUMS has no signature.\n"
    );
    assert_eq!(installed.program_holds(), "the program before");

    let unclear = Installed::scripted("serve-unclear", "yes");
    let said = serving(&releases, "0.8.0", &unclear.program, false);
    assert!(
        said.starts_with("Didn't check for a newer Statac: `updates` in `")
            && said.ends_with(&format!("{MARKER}` is `yes`, not `on` or `off`.\n")),
        "{said}"
    );
}

#[test]
fn what_an_upgrade_left_is_removed_once_nothing_needs_it() {
    let scratch = Scratch::new("upgrade-left");
    let ours = std::process::id();
    let other = ours.wrapping_add(1);
    let names = [
        format!("{OLD_PROGRAM}{other}"),
        format!("{NEW_PROGRAM}{other}"),
        format!("{NEW_MARKER}{other}"),
        format!("{NEW_PROGRAM}{ours}"),
        format!("{NEW_PROGRAM}not-a-number"),
        "statac".to_owned(),
    ];
    for name in &names {
        scratch.write(name, "left");
    }
    let aged = |name: &str| {
        let file = std::fs::File::options()
            .write(true)
            .open(scratch.join(name))
            .unwrap();
        file.set_modified(SystemTime::now() - LEFT_FOR - Duration::from_secs(1))
            .unwrap();
    };
    let present = || {
        let mut found: Vec<String> = std::fs::read_dir(&*scratch)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        found.sort();
        found
    };
    remove_left_behind(&scratch);
    let mut young = vec![
        names[1].clone(),
        names[2].clone(),
        names[3].clone(),
        names[4].clone(),
        "statac".to_owned(),
    ];
    young.sort();
    assert_eq!(present(), young);
    for name in &names[1..] {
        aged(name);
    }
    remove_left_behind(&scratch);
    let mut kept = vec![names[3].clone(), names[4].clone(), "statac".to_owned()];
    kept.sort();
    assert_eq!(present(), kept);
}

/// What an uninstall of the install at `installed` said and its exit
/// code, with 0.8.0 running.
fn uninstalling(installed: &Installed) -> (String, u8) {
    let mut said = String::new();
    let code = uninstall_at(&installed.program, version("0.8.0"), &mut |text| {
        said.push_str(text)
    });
    (said, code)
}

impl Installed {
    /// The program's folder as an uninstall names it, every link on the
    /// way resolved.
    fn folder_shown(&self) -> String {
        shown(&std::fs::canonicalize(self.scratch.join("bin")).unwrap())
    }

    fn licences_shown(&self) -> String {
        self.scratch.join("share").display().to_string()
    }

    fn write_licences(&self) {
        for licence in LICENCES {
            self.scratch.write(&format!("share/{licence}"), licence);
        }
    }

    /// What the line saying the program was removed reads.
    fn uninstalled(&self) -> String {
        wording::uninstalled("0.8.0", &self.folder_shown(), cfg!(windows))
    }
}

#[test]
fn an_uninstall_removes_the_program_the_marker_and_the_licences() {
    let installed = Installed::scripted("uninstall", "on");
    installed.write_licences();
    let other = std::process::id().wrapping_add(1);
    for left in [
        format!("bin/{NEW_PROGRAM}{other}"),
        format!("bin/{OLD_PROGRAM}{other}"),
        format!("bin/{NEW_MARKER}{other}"),
        format!("bin/{NEW_PROGRAM}not-a-number"),
        "bin/another-program".to_owned(),
        format!("share/{}", ".LICENSE-MIT.12.tmp"),
    ] {
        installed.scratch.write(&left, "left");
    }
    assert_eq!(
        uninstalling(&installed),
        (
            format!(
                "{}\nRemoved its licences, and their folder `{}`, which held nothing else.\n",
                installed.uninstalled(),
                installed.licences_shown()
            ),
            0
        )
    );
    let mut kept = vec![
        format!("{NEW_PROGRAM}not-a-number"),
        "another-program".to_owned(),
    ];
    // On Windows the program waits aside until this process ends.
    if cfg!(windows) {
        kept.push(format!("{OLD_PROGRAM}{}", std::process::id()));
    }
    kept.sort();
    assert_eq!(installed.beside(), kept);
    assert!(!installed.scratch.join("share").exists());
}

#[test]
fn an_uninstall_keeps_the_licences_folder_where_it_holds_other_files() {
    let installed = Installed::scripted("uninstall-kept", "off");
    installed.write_licences();
    installed
        .scratch
        .write("share/notes.txt", "the author's own");
    let (said, code) = uninstalling(&installed);
    assert_eq!(
        said,
        format!(
            "{}\nRemoved its licences from `{}`, and kept the folder, which holds other files.\n",
            installed.uninstalled(),
            installed.licences_shown()
        )
    );
    assert_eq!(code, 0);
    let names: Vec<String> = std::fs::read_dir(installed.scratch.join("share"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["notes.txt"]);
}

#[test]
fn an_uninstall_says_nothing_of_licences_it_found_none_of() {
    for (label, marker) in [
        ("uninstall-unnamed", "updates: off\n"),
        ("uninstall-gone", "updates: off\nlicences: <licences>\n"),
    ] {
        let installed = Installed::new(label, Some(marker));
        assert_eq!(
            uninstalling(&installed),
            (format!("{}\n", installed.uninstalled()), 0)
        );
        assert!(!installed.program.exists());
    }
}

#[cfg(unix)]
#[test]
fn an_uninstall_that_cant_remove_the_program_changes_nothing() {
    use std::os::unix::fs::PermissionsExt;
    let installed = Installed::scripted("uninstall-denied", "off");
    installed.write_licences();
    let bin = installed.scratch.join("bin");
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o555)).unwrap();
    let said = uninstalling(&installed);
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    let program = shown(&std::fs::canonicalize(&installed.program).unwrap());
    assert_eq!(
        said,
        (
            format!(
                "Didn't uninstall Statac: Statac can't remove `{program}` (permission denied).\n"
            ),
            1
        )
    );
    assert_eq!(installed.beside(), [MARKER, "statac"]);
    assert_eq!(
        installed.licence("LICENSE-MIT").as_deref(),
        Some("LICENSE-MIT")
    );
}

#[test]
fn another_tools_install_is_left_to_its_uninstall_command() {
    let scratch = Scratch::new("uninstall-other");
    let program = scratch.write("bin/statac", "another tool's program");
    let ways = match cfg!(windows) {
        true => {
            "`scoop uninstall statac`, `winget uninstall Statac.Statac` or `cargo uninstall statac`"
        }
        false => "`brew uninstall statac` or `cargo uninstall statac`",
    };
    let shown_program = shown(&std::fs::canonicalize(&program).unwrap());
    let cases = [
        (
            program.clone(),
            format!(
                "Uninstall it the way it was installed: {ways}, or delete `{shown_program}` if you put it there yourself."
            ),
        ),
        (
            scratch.write("Cellar/statac/0.8.0/bin/statac", "another tool's program"),
            "Homebrew installed it, so uninstall it with `brew uninstall statac`.".into(),
        ),
        (
            scratch.write(
                "scoop/apps/statac/current/statac.exe",
                "another tool's program",
            ),
            "Scoop installed it, so uninstall it with `scoop uninstall statac`.".into(),
        ),
        (
            scratch.write(
                "Microsoft/WinGet/Packages/Statac.Statac_Microsoft.Winget.Source/statac.exe",
                "another tool's program",
            ),
            "winget installed it, so uninstall it with `winget uninstall Statac.Statac`.".into(),
        ),
        (
            {
                scratch.write(".cargo/.crates2.json", "{}");
                scratch.write(".cargo/bin/statac", "another tool's program")
            },
            "Cargo installed it, so uninstall it with `cargo uninstall statac`.".into(),
        ),
    ];
    for (program, line) in cases {
        let mut said = String::new();
        let code = uninstall_at(&program, version("0.8.0"), &mut |text| said.push_str(text));
        assert_eq!(
            (said, code),
            (format!("{}\n{line}\n", wording::UNINSTALL_NOT_SCRIPTED), 1),
            "{}",
            program.display()
        );
        assert_eq!(std::fs::read(&program).unwrap(), b"another tool's program");
    }
}

#[test]
fn the_install_script_carries_statacs_key() {
    let script =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/install.sh"))
            .unwrap();
    let carried: Vec<&str> = (script.lines())
        .filter_map(|line| line.strip_prefix("key="))
        .collect();
    assert_eq!(carried, [hex_of(&KEY)]);
    // OpenSSH's form: the key type and the key, each with its length first,
    // in base64.
    let blob = [b"\0\0\0\x0bssh-ed25519\0\0\0\x20".as_slice(), &KEY].concat();
    let signers: Vec<&str> = (script.lines())
        .filter_map(|line| line.strip_prefix("signer="))
        .collect();
    assert_eq!(
        signers,
        [format!("'statac ssh-ed25519 {}'", base64_of(&blob))]
    );
}

fn base64_of(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut text = String::new();
    for chunk in bytes.chunks(3) {
        let joined = (chunk.iter().enumerate()).fold(0u32, |joined, (at, &byte)| {
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

#[test]
fn versions_are_read_and_ordered_as_numbers() {
    assert_eq!(
        Version::read("0.10.2"),
        Some(Version {
            major: 0,
            minor: 10,
            patch: 2
        })
    );
    for unread in [
        "",
        "0.8",
        "0.8.1.2",
        "0.8.x",
        "v0.8.1",
        "0.8.-1",
        "0.8.1-rc.1",
        "0..1",
    ] {
        assert_eq!(Version::read(unread), None, "{unread}");
    }
    assert!(version("0.10.0") > version("0.9.9"));
    assert!(version("0.8.2").fixes(version("0.8.1")));
    assert!(!version("0.8.1").fixes(version("0.8.1")));
    assert!(!version("0.9.0").fixes(version("0.8.1")));
    assert!(!version("1.8.2").fixes(version("0.8.1")));
    assert_eq!(Version::running().to_string(), env!("CARGO_PKG_VERSION"));
}

#[test]
fn sums_and_package_names_are_read_as_published() {
    let hash = "10d572e2c649180e4c6e411a1d58ce2d34d2be222a2d8146e99f7073b9cb4027";
    let line = format!("{hash}  statac-0.7.2-aarch64-apple-darwin.tar.gz");
    let (bytes, name) = sum(&line).unwrap();
    assert_eq!(hex_of(&bytes), hash);
    assert_eq!(
        package_name(name),
        Some((version("0.7.2"), "aarch64-apple-darwin"))
    );
    let windows = format!("{hash} *statac-0.7.2-x86_64-pc-windows-msvc.zip");
    let (_, name) = sum(&windows).unwrap();
    assert_eq!(
        package_name(name),
        Some((version("0.7.2"), "x86_64-pc-windows-msvc"))
    );
    for unread in [
        format!("{hash} statac-0.7.2-aarch64-apple-darwin.tar.gz"),
        format!("{}  statac-0.7.2-aarch64-apple-darwin.tar.gz", &hash[1..]),
        format!("{hash}  "),
        "not a sum".to_owned(),
    ] {
        assert_eq!(sum(&unread), None, "{unread}");
    }
    assert_eq!(package_name("statac-0.7.2-target.deb"), None);
}

#[test]
fn hashes_and_signatures_are_checked_by_the_tls_library() {
    assert_eq!(
        hex_of(&sha256(b"abc")),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let message = b"a SHA256SUMS";
    let signed_message = signature(message);
    assert!(signed(message, &signed_message, &TEST_KEY));
    assert!(!signed(b"another SHA256SUMS", &signed_message, &TEST_KEY));
    assert!(!signed(message, &signed_message, &KEY));
    assert!(!signed(message, &signed_message[1..], &TEST_KEY));
    assert!(!signed(message, b"", &TEST_KEY));
}

#[test]
fn this_machine_has_a_package_where_it_is_one_releases_are_built_for() {
    let released = cfg!(any(target_os = "macos", target_os = "windows"))
        || cfg!(all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        ));
    if released {
        let target = TARGET.unwrap();
        assert!(target.starts_with(std::env::consts::ARCH), "{target}");
    }
}
