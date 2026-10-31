//! The install script for a Mac or Linux, `tools/install.sh`, run as its
//! instructions run it, `curl -fsSL <address> | sh -s -- <options>`, with
//! nothing in its environment but `PATH` and what each case gives it, as
//! in a container, a Dockerfile's `RUN` or a CI job: against a server on
//! this machine standing in for get.statac.dev. The release it serves is
//! one before 0.8.0, which has no signature, so what this machine could
//! check a signature with makes no difference.
#![cfg(any(target_os = "macos", target_os = "linux"))]

mod scratch;

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock};

use rustls::crypto::aws_lc_rs;
use scratch::Scratch;

const VERSION: &str = "0.7.0";
const LICENCES: [&str; 4] = [
    "LICENSE-MIT",
    "LICENSE-APACHE",
    "THIRD-PARTY.md",
    "THIRD-PARTY-GRAMMARS.md",
];

/// Every package the script may choose on this system: a Mac's shell under
/// Rosetta says x86_64, and the script takes the Apple silicon package.
const TARGETS: [&str; 2] = if cfg!(target_os = "macos") {
    ["aarch64-apple-darwin", "x86_64-apple-darwin"]
} else {
    ["aarch64-unknown-linux-musl", "x86_64-unknown-linux-musl"]
};

fn sha256(bytes: &[u8]) -> String {
    let suite = aws_lc_rs::cipher_suite::TLS13_AES_128_GCM_SHA256
        .tls13()
        .unwrap();
    let hash = suite.common.hash_provider.hash(bytes);
    hash.as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// A server on this machine standing in for get.statac.dev, answering
/// each path it holds with its bytes and any other with `404`.
struct Releases {
    addr: SocketAddr,
    files: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl Releases {
    fn start() -> Releases {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let files: Arc<Mutex<HashMap<String, Vec<u8>>>> = Arc::default();
        let held = files.clone();
        std::thread::spawn(move || {
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
        Releases { addr, files }
    }

    fn address(&self) -> String {
        format!("http://{}", self.addr)
    }

    fn put(&self, path: &str, bytes: impl Into<Vec<u8>>) {
        let mut files = self.files.lock().unwrap();
        files.insert(path.to_owned(), bytes.into());
    }

    /// A release of the packages, which latest/ names.
    fn publish() -> Releases {
        let releases = Releases::start();
        let mut sums = String::new();
        for (target, package) in TARGETS.iter().zip(packages()) {
            let name = format!("statac-{VERSION}-{target}.tar.gz");
            sums.push_str(&format!("{}  {name}\n", sha256(package)));
            releases.put(&format!("/v{VERSION}/{name}"), package.clone());
        }
        releases.put(&format!("/v{VERSION}/SHA256SUMS"), sums.clone());
        releases.put("/latest/SHA256SUMS", sums);
        releases
    }
}

/// A package for each of `TARGETS`, made as the release makes it, by
/// `tar`, holding the licences and a program that says its version as
/// Statac does.
fn packages() -> &'static [Vec<u8>] {
    static PACKAGES: OnceLock<Vec<Vec<u8>>> = OnceLock::new();
    PACKAGES.get_or_init(|| {
        let staging = Scratch::new("install_sh", "packages");
        TARGETS
            .iter()
            .map(|target| {
                let name = format!("statac-{VERSION}-{target}");
                let folder = staging.join(&name);
                std::fs::create_dir_all(&folder).unwrap();
                let program = folder.join("statac");
                std::fs::write(&program, format!("#!/bin/sh\necho 'Statac {VERSION}'\n")).unwrap();
                std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
                for licence in LICENCES {
                    let from = Path::new(env!("CARGO_MANIFEST_DIR")).join(licence);
                    std::fs::copy(from, folder.join(licence)).unwrap();
                }
                let made = Command::new("tar")
                    .args(["-czf", &format!("{name}.tar.gz"), &name])
                    .current_dir(&*staging)
                    .env("COPYFILE_DISABLE", "1")
                    .stdin(Stdio::null())
                    .output()
                    .unwrap();
                assert!(made.status.success(), "{made:?}");
                std::fs::read(staging.join(format!("{name}.tar.gz"))).unwrap()
            })
            .collect()
    })
}

/// The shell to run the script with: dash where the machine has it, as
/// Debian and Ubuntu's sh is, and sh otherwise, busybox's on Alpine. A
/// Mac's sh is bash, which sets `SHELL` itself where it is unset.
fn shell() -> &'static str {
    let dash = Command::new("dash")
        .args(["-c", ":"])
        .stdin(Stdio::null())
        .status();
    match dash.is_ok_and(|status| status.success()) {
        true => "dash",
        false => "sh",
    }
}

/// What the script said and its exit code, given `options` and an
/// environment holding only `PATH`, the address of `releases` and
/// `variables`.
fn install(
    releases: &Releases,
    options: &[String],
    variables: &[(&str, String)],
) -> (String, Option<i32>) {
    let mut child = Command::new(shell())
        .args(["-s", "--"])
        .args(options)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap())
        .env("STATAC_RELEASES", releases.address())
        .envs(variables.iter().map(|(name, value)| (name, value)))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let script = include_str!("../tools/install.sh");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(script.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (said, output.status.code())
}

fn holds(said: &str, lines: &[String]) {
    for line in lines {
        assert!(said.contains(&format!("{line}\n")), "{line}\n\n{said}");
    }
}

#[test]
fn an_install_says_how_to_add_its_folder_to_the_path_by_the_shell_it_finds() {
    let releases = Releases::publish();
    for (label, shell, hint) in [
        (
            "unset",
            None,
            "Add this line to your shell's startup file: export PATH=\"{to}:$PATH\"",
        ),
        (
            "zsh",
            Some("/bin/zsh"),
            "Add this line to ~/.zshrc: export PATH=\"{to}:$PATH\"",
        ),
        (
            "bash",
            Some("/usr/local/bin/bash"),
            "Add this line to ~/.bashrc: export PATH=\"{to}:$PATH\"",
        ),
        (
            "fish",
            Some("/opt/homebrew/bin/fish"),
            "Add it: fish_add_path \"{to}\"",
        ),
        (
            "another",
            Some("/bin/ksh"),
            "Add this line to your shell's startup file: export PATH=\"{to}:$PATH\"",
        ),
    ] {
        let scratch = Scratch::new("install_sh", &format!("shell-{label}"));
        let home = scratch.join("home");
        let to = scratch.join("bin");
        std::fs::create_dir_all(&home).unwrap();
        let to_shown = to.display().to_string();
        let mut variables = vec![("HOME", home.display().to_string())];
        if let Some(shell) = shell {
            variables.push(("SHELL", shell.to_owned()));
        }
        let options = [format!("--to={to_shown}")];
        let (said, code) = install(&releases, &options, &variables);
        assert_eq!(code, Some(0), "{label}\n{said}");
        holds(
            &said,
            &[
                format!("Installed Statac {VERSION} at {to_shown}/statac"),
                format!("{to_shown} isn't on your PATH, so `statac` alone won't be found."),
                format!("  {}", hint.replace("{to}", &to_shown)),
            ],
        );
    }
}

#[test]
fn an_install_needs_home_only_for_what_it_puts_there() {
    let releases = Releases::publish();
    let hint = "  Add this line to your shell's startup file: export PATH=\"{to}:$PATH\"";
    for (label, to, data, code, lines) in [
        // Where HOME is unset, the licences go in XDG_DATA_HOME.
        (
            "data-given",
            Some("{scratch}/bin"),
            true,
            0,
            vec![
                "Installed Statac {version} at {scratch}/bin/statac",
                "Its licences are in {scratch}/data/statac",
                hint,
            ],
        ),
        (
            "no-data",
            Some("{scratch}/bin"),
            false,
            1,
            vec![
                "Can't install Statac: `HOME` isn't set, so there's nowhere for its licences.",
                "  Set `HOME`, then run this again.",
            ],
        ),
        (
            "no-folder",
            None,
            true,
            1,
            vec![
                "Can't tell where to install Statac: `HOME` isn't set.",
                "  Give the folder with `--to=<folder>`.",
            ],
        ),
        (
            "folder-in-home",
            Some("~/bin"),
            true,
            1,
            vec![
                "Can't install Statac in `~/bin`: `HOME` isn't set.",
                "  Write the folder's whole path.",
            ],
        ),
    ] {
        let scratch = Scratch::new("install_sh", &format!("home-{label}"));
        std::fs::create_dir_all(&*scratch).unwrap();
        let shown = |text: &str| {
            text.replace("{version}", VERSION)
                .replace("{scratch}", &scratch.display().to_string())
                .replace("{to}", &scratch.join("bin").display().to_string())
        };
        let options: Vec<String> = to
            .map(|to| format!("--to={}", shown(to)))
            .into_iter()
            .collect();
        let variables = match data {
            true => vec![("XDG_DATA_HOME", shown("{scratch}/data"))],
            false => vec![],
        };
        let (said, found) = install(&releases, &options, &variables);
        assert_eq!(found, Some(code), "{label}\n{said}");
        let lines: Vec<String> = lines.iter().map(|line| shown(line)).collect();
        holds(&said, &lines);
        let installed = code == 0;
        assert_eq!(scratch.join("bin/statac").is_file(), installed, "{label}");
        assert_eq!(scratch.join("data/statac").is_dir(), installed, "{label}");
        if installed {
            assert_eq!(
                std::fs::read_to_string(scratch.join("bin/.statac-install")).unwrap(),
                shown(
                    "# Written by Statac's install script. statac upgrade and statac uninstall read it.\n\
                     updates: off\nlicences: {scratch}/data/statac\n"
                )
            );
            for licence in LICENCES {
                assert!(
                    scratch.join("data/statac").join(licence).is_file(),
                    "{licence}"
                );
            }
        }
    }
}
