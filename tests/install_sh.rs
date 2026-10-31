//! The install script for a Mac or Linux, `tools/install.sh`, run as its
//! instructions run it, `curl -fsSL <address> | sh -s -- <options>`, with
//! nothing in its environment but `PATH` and what each case gives it, as
//! in a container, a Dockerfile's `RUN` or a CI job: against a server on
//! this machine standing in for get.statac.dev, with a key of the tests'
//! own standing in for Statac's. Each package holds a script standing in
//! for the program, which says its version as Statac does. A machine that
//! can't check a signature is played by tools standing in for a stock
//! Ubuntu 18.04's, OpenSSH 7.6's `ssh-keygen` and OpenSSL 1.1.1, put first
//! on the PATH; the cases that check one run where this machine has the
//! tool.
#![cfg(any(target_os = "macos", target_os = "linux"))]

mod releases;
mod scratch;

use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;

use releases::{KEY, Releases, TEST_KEY, bytes_of_hex, sha256, signature};
use scratch::Scratch;

/// A release that is signed, and one from before releases were.
const VERSION: &str = "0.8.0";
const BEFORE: &str = "0.7.3";
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
/// Where the script looks for Homebrew's OpenSSL 3 off the PATH.
const HOMEBREW: [&str; 2] = [
    "/opt/homebrew/opt/openssl@3/bin/openssl",
    "/usr/local/opt/openssl@3/bin/openssl",
];

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

/// `bytes` with its length first, as OpenSSH writes a string.
fn ssh_string(bytes: &[u8]) -> Vec<u8> {
    let length = u32::try_from(bytes.len()).unwrap().to_be_bytes();
    [&length, bytes].concat()
}

/// A public key in OpenSSH's form, the key type and the key.
fn ssh_key(hex: &str) -> Vec<u8> {
    [ssh_string(b"ssh-ed25519"), ssh_string(&bytes_of_hex(hex))].concat()
}

/// The tests' key's SSH signature of `message` in `namespace`, laid out
/// as OpenSSH's PROTOCOL.sshsig gives. ssh-keygen hashes the message with
/// SHA-512 and accepts SHA-256 too, which is what these tests have.
fn ssh_signature(message: &[u8], namespace: &str) -> String {
    let fields = [
        ssh_string(namespace.as_bytes()),
        ssh_string(b""),
        ssh_string(b"sha256"),
    ]
    .concat();
    let signed = [
        b"SSHSIG".as_slice(),
        &fields,
        &ssh_string(&bytes_of_hex(&sha256(message))),
    ]
    .concat();
    let made = [ssh_string(b"ssh-ed25519"), ssh_string(&signature(&signed))].concat();
    let blob = [
        b"SSHSIG".as_slice(),
        &1u32.to_be_bytes(),
        &ssh_string(&ssh_key(TEST_KEY)),
        &fields,
        &ssh_string(&made),
    ]
    .concat();
    let text = base64_of(&blob);
    let lines: Vec<&str> = (0..text.len())
        .step_by(70)
        .map(|at| &text[at..text.len().min(at + 70)])
        .collect();
    format!(
        "-----BEGIN SSH SIGNATURE-----\n{}\n-----END SSH SIGNATURE-----\n",
        lines.join("\n")
    )
}

/// The namespace the script checks a release's SSH signature in.
fn namespace() -> &'static str {
    let script = include_str!("../tools/install.sh");
    let found: Vec<&str> = (script.lines())
        .filter_map(|line| line.strip_prefix("namespace="))
        .collect();
    assert_eq!(found.len(), 1);
    found[0]
}

/// What this machine checks a signature with, or plays at having: its
/// own tools, `ssh-keygen` alone, OpenSSL 3 alone, or neither.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Tools {
    SshKeygen,
    SshKeygenAlone,
    OpenSsl,
    Neither,
}

impl Tools {
    /// Whether this machine has the tool, where these tools need one.
    fn here(self) -> bool {
        let probe = match self {
            Tools::SshKeygen | Tools::SshKeygenAlone => {
                "ssh-keygen '-?' 2>&1 | grep -q -e '-Y verify'".to_owned()
            }
            Tools::OpenSsl => format!(
                "for openssl in openssl {}; do \
                 command -v \"$openssl\" >/dev/null 2>&1 && \
                 \"$openssl\" version 2>/dev/null | grep -q '^OpenSSL [3-9]' && \
                 \"$openssl\" pkeyutl -help 2>&1 | grep -q -e -rawin && exit 0; \
                 done; exit 1",
                HOMEBREW.join(" ")
            ),
            Tools::Neither => return true,
        };
        let here = Command::new("sh").args(["-c", &probe]).status().unwrap();
        if !here.success() {
            eprintln!("Not tried with {self:?}: this machine doesn't have it.");
        }
        here.success()
    }
}

/// A home of a test's own: a folder to install into, one standing in for
/// `.local/share`, and one of tools standing in for older ones.
struct Home {
    scratch: Scratch,
}

impl Home {
    fn new(label: &str) -> Home {
        let scratch = Scratch::new("install_sh", label);
        std::fs::create_dir_all(scratch.join("old")).unwrap();
        // OpenSSH 7.6's ssh-keygen, which has no -Y, and OpenSSL 1.1.1,
        // which has no `pkeyutl -rawin`.
        let old = [
            (
                "ssh-keygen",
                "echo 'usage: ssh-keygen [-q] [-b bits] [-t dsa | ecdsa | ed25519 | rsa]' >&2\nexit 1\n",
            ),
            (
                "openssl",
                "case $1 in\n    version) echo 'OpenSSL 1.1.1  11 Sep 2018' ;;\n    *) exit 1 ;;\nesac\n",
            ),
        ];
        for (name, body) in old {
            let path = scratch.join("old").join(name);
            std::fs::write(&path, format!("#!/bin/sh\n{body}")).unwrap();
            make_runnable(&path);
        }
        Home { scratch }
    }

    fn to(&self) -> PathBuf {
        self.scratch.join("bin")
    }

    fn share(&self) -> PathBuf {
        self.scratch.join("share")
    }

    fn old(&self, name: &str) -> PathBuf {
        self.scratch.join("old").join(name)
    }

    /// Nothing put in place: no program, no folder for it, no licences.
    fn holds_nothing(&self, said: &str) {
        assert!(!self.to().exists(), "{said}");
        assert!(!self.share().exists(), "{said}");
    }
}

fn make_runnable(path: &Path) {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// The script, carrying the tests' key in place of Statac's, and where
/// `tools` plays a machine with no OpenSSL 3, looking for OpenSSL off the
/// PATH only where `home`'s old one is.
fn script(tools: Tools, home: &Home) -> String {
    let mut script = include_str!("../tools/install.sh").to_owned();
    let swaps = [
        (KEY.to_owned(), TEST_KEY.to_owned()),
        (base64_of(&ssh_key(KEY)), base64_of(&ssh_key(TEST_KEY))),
    ];
    for (from, to) in swaps {
        assert_eq!(script.matches(&from).count(), 1, "{from}");
        script = script.replace(&from, &to);
    }
    if matches!(tools, Tools::SshKeygenAlone | Tools::Neither) {
        for from in HOMEBREW {
            assert_eq!(script.matches(from).count(), 1, "{from}");
            script = script.replace(from, &home.old("openssl").display().to_string());
        }
    }
    script
}

/// A package for each of `TARGETS`, named as release `version`'s, made as
/// the release makes it, by `tar`, holding the licences and a program that
/// says its version as Statac does.
fn packages(version: &str) -> Vec<Vec<u8>> {
    static PACKAGES: Mutex<Vec<(String, Vec<Vec<u8>>)>> = Mutex::new(Vec::new());
    let mut made = PACKAGES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some((_, packages)) = made.iter().find(|(made, _)| made == version) {
        return packages.clone();
    }
    let staging = Scratch::new("install_sh", &format!("packages-{version}"));
    let packages: Vec<Vec<u8>> = (TARGETS.iter())
        .map(|target| {
            let name = format!("statac-{version}-{target}");
            let folder = staging.join(&name);
            std::fs::create_dir_all(&folder).unwrap();
            let program = folder.join("statac");
            std::fs::write(&program, format!("#!/bin/sh\necho 'Statac {version}'\n")).unwrap();
            make_runnable(&program);
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
        .collect();
    made.push((version.to_owned(), packages.clone()));
    packages
}

/// A release `version`, which latest/ names, signed both ways from 0.8.0
/// on, as every release since is.
fn publish(version: &str) -> Releases {
    let releases = Releases::start();
    let mut sums = String::new();
    for (target, package) in TARGETS.iter().zip(packages(version)) {
        let name = format!("statac-{version}-{target}.tar.gz");
        sums.push_str(&format!("{}  {name}\n", sha256(&package)));
        releases.put(&format!("/v{version}/{name}"), package);
    }
    sums.push_str(&format!(
        "{}  statac-{version}-x86_64-pc-windows-msvc.zip\n",
        sha256(b"another platform's")
    ));
    releases.put(&format!("/v{version}/SHA256SUMS"), sums.clone());
    let unsigned = (version.strip_prefix("0."))
        .and_then(|rest| rest.split('.').next()?.parse::<u32>().ok())
        .is_some_and(|minor| minor < 8);
    if !unsigned {
        releases.put(
            &format!("/v{version}/SHA256SUMS.sig"),
            signature(sums.as_bytes()),
        );
        releases.put(
            &format!("/v{version}/SHA256SUMS.sshsig"),
            ssh_signature(sums.as_bytes(), namespace()),
        );
    }
    releases.put("/latest/SHA256SUMS", sums);
    releases
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

/// What `script` said and its exit code, given `options` and an
/// environment holding only `PATH`, the address of `releases` and
/// `variables`, any of which may give `PATH` in its place.
fn run(
    releases: &Releases,
    script: &str,
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

/// What the script said and its exit code, carrying the tests' key, run
/// unattended in `home` with `options` after `--to` and `--updates=off`,
/// with `tools` standing in for this machine's where they play older ones.
fn install(
    releases: &Releases,
    home: &Home,
    tools: Tools,
    options: &[&str],
) -> (String, Option<i32>) {
    let mut path = std::env::var_os("PATH").unwrap_or_default();
    let mut shadowed = Vec::new();
    match tools {
        Tools::SshKeygen => {}
        Tools::SshKeygenAlone => shadowed.push("openssl"),
        Tools::OpenSsl => shadowed.push("ssh-keygen"),
        Tools::Neither => shadowed.extend(["ssh-keygen", "openssl"]),
    }
    if !shadowed.is_empty() {
        let first = home.scratch.join("first");
        std::fs::create_dir_all(&first).unwrap();
        for name in shadowed {
            let _ = std::fs::remove_file(first.join(name));
            std::os::unix::fs::symlink(home.old(name), first.join(name)).unwrap();
        }
        let mut joined = first.into_os_string();
        joined.push(":");
        joined.push(&path);
        path = joined;
    }
    let mut given = vec![
        format!("--to={}", home.to().display()),
        "--updates=off".to_owned(),
    ];
    given.extend(options.iter().map(|option| (*option).to_owned()));
    let variables = [
        ("PATH", path.to_string_lossy().into_owned()),
        ("HOME", home.scratch.display().to_string()),
        ("XDG_DATA_HOME", home.share().display().to_string()),
    ];
    run(releases, &script(tools, home), &given, &variables)
}

/// The script as published, for the cases a release before 0.8.0 serves,
/// where nothing is signed and the machine's tools make no difference.
const SCRIPT: &str = include_str!("../tools/install.sh");

fn holds(said: &str, lines: &[String]) {
    for line in lines {
        assert!(said.contains(&format!("{line}\n")), "{line}\n\n{said}");
    }
}

/// The lines saying what checked the package, as each of `tools` has it.
fn checked(tools: Tools) -> Vec<String> {
    let lines: &[&str] = match tools {
        Tools::SshKeygen | Tools::SshKeygenAlone => &[
            "Checked the package against the release's SHA256SUMS, and SHA256SUMS against",
            "  its signature with `ssh-keygen`.",
        ],
        Tools::OpenSsl => &[
            "Checked the package against the release's SHA256SUMS, and SHA256SUMS against",
            "  its signature with OpenSSL 3.",
        ],
        Tools::Neither => &[
            "Checked the package against the release's SHA256SUMS, but not its signature:",
            "  that needs `ssh-keygen` from OpenSSH 8.1 or later, or OpenSSL 3, and this",
            "  machine has neither.",
        ],
    };
    lines.iter().map(|line| (*line).to_owned()).collect()
}

const EVERY: [Tools; 4] = [
    Tools::SshKeygen,
    Tools::SshKeygenAlone,
    Tools::OpenSsl,
    Tools::Neither,
];

#[test]
fn an_install_says_how_to_add_its_folder_to_the_path_by_the_shell_it_finds() {
    let releases = publish(BEFORE);
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
        let options = [format!("--to={to_shown}"), format!("--version={BEFORE}")];
        let (said, code) = run(&releases, SCRIPT, &options, &variables);
        assert_eq!(code, Some(0), "{label}\n{said}");
        holds(
            &said,
            &[
                format!("Installed Statac {BEFORE} at {to_shown}/statac"),
                format!("{to_shown} isn't on your PATH, so `statac` alone won't be found."),
                format!("  {}", hint.replace("{to}", &to_shown)),
            ],
        );
    }
}

#[test]
fn an_install_needs_home_only_for_what_it_puts_there() {
    let releases = publish(BEFORE);
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
            text.replace("{version}", BEFORE)
                .replace("{scratch}", &scratch.display().to_string())
                .replace("{to}", &scratch.join("bin").display().to_string())
        };
        let mut options: Vec<String> = to
            .map(|to| format!("--to={}", shown(to)))
            .into_iter()
            .collect();
        options.push(format!("--version={BEFORE}"));
        let variables = match data {
            true => vec![("XDG_DATA_HOME", shown("{scratch}/data"))],
            false => vec![],
        };
        let (said, found) = run(&releases, SCRIPT, &options, &variables);
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

#[test]
fn a_signed_release_is_checked_with_the_tool_this_machine_has() {
    for tools in EVERY.into_iter().filter(|tools| tools.here()) {
        let home = Home::new(&format!("signed-{tools:?}"));
        let releases = publish(VERSION);
        let (said, code) = install(&releases, &home, tools, &[]);
        assert_eq!(code, Some(0), "{tools:?}\n{said}");
        let mut lines = checked(tools);
        lines.push(format!(
            "Installed Statac {VERSION} at {}/statac",
            home.to().display()
        ));
        holds(&said, &lines);
        assert_eq!(
            std::fs::read_to_string(home.to().join(".statac-install")).unwrap(),
            format!(
                "# Written by Statac's install script. statac upgrade and statac uninstall read it.\n\
                 updates: off\nlicences: {}\n",
                home.share().join("statac").display()
            )
        );
    }
}

#[test]
fn a_release_that_doesnt_match_its_signature_installs_nothing() {
    let checking = [Tools::SshKeygen, Tools::OpenSsl];
    for tools in checking.into_iter().filter(|tools| tools.here()) {
        let home = Home::new(&format!("tampered-{tools:?}"));
        let releases = publish(VERSION);
        let sums: String = (TARGETS.iter())
            .map(|target| {
                let hash = sha256(b"another package");
                format!("{hash}  statac-{VERSION}-{target}.tar.gz\n")
            })
            .collect();
        releases.put(&format!("/v{VERSION}/SHA256SUMS"), sums.clone());
        releases.put("/latest/SHA256SUMS", sums);
        let (said, code) = install(&releases, &home, tools, &[]);
        assert_eq!(code, Some(1), "{tools:?}\n{said}");
        holds(
            &said,
            &["The release's SHA256SUMS isn't signed with Statac's key, so nothing was installed.".into()],
        );
        home.holds_nothing(&said);
    }
}

#[test]
fn a_release_from_0_8_0_on_with_no_signature_installs_nothing() {
    for tools in EVERY {
        let home = Home::new(&format!("unsigned-{tools:?}"));
        let releases = publish(VERSION);
        releases.remove(&format!("/v{VERSION}/SHA256SUMS.sig"));
        releases.remove(&format!("/v{VERSION}/SHA256SUMS.sshsig"));
        let (said, code) = install(&releases, &home, tools, &[]);
        assert_eq!(code, Some(1), "{tools:?}\n{said}");
        holds(
            &said,
            &[
                "The release's SHA256SUMS has no signature, so nothing was installed.".into(),
                format!(
                    "  Every release from 0.8.0 on is signed, and {}/v{VERSION}/ holds neither SHA256SUMS.sshsig nor SHA256SUMS.sig.",
                    releases.address()
                ),
            ],
        );
        assert!(!said.contains("Installing"), "{said}");
        home.holds_nothing(&said);
    }
}

#[test]
fn a_machine_that_can_check_a_signature_never_installs_on_the_hash_alone() {
    // The signature this machine can check taken away, leaving the other.
    for (tools, missing, line) in [
        (
            Tools::SshKeygenAlone,
            "SHA256SUMS.sshsig",
            "has no SHA256SUMS.sshsig, which `ssh-keygen` checks, and SHA256SUMS.sig needs OpenSSL 3.",
        ),
        (
            Tools::OpenSsl,
            "SHA256SUMS.sig",
            "has no SHA256SUMS.sig, which OpenSSL 3 checks, and SHA256SUMS.sshsig needs `ssh-keygen` from OpenSSH 8.1 or later.",
        ),
    ] {
        if !tools.here() {
            continue;
        }
        let home = Home::new(&format!("uncheckable-{tools:?}"));
        let releases = publish(VERSION);
        releases.remove(&format!("/v{VERSION}/{missing}"));
        let (said, code) = install(&releases, &home, tools, &[]);
        assert_eq!(code, Some(1), "{tools:?}\n{said}");
        holds(
            &said,
            &[
                "The release's SHA256SUMS has no signature this machine can check, so nothing was installed.".into(),
                format!("  {}/v{VERSION}/ {line}", releases.address()),
            ],
        );
        home.holds_nothing(&said);
    }

    // With both, the other one is checked.
    if Tools::SshKeygen.here() && Tools::OpenSsl.here() {
        let home = Home::new("both");
        let releases = publish(VERSION);
        releases.remove(&format!("/v{VERSION}/SHA256SUMS.sshsig"));
        let (said, code) = install(&releases, &home, Tools::SshKeygen, &[]);
        assert_eq!(code, Some(0), "{said}");
        holds(&said, &checked(Tools::OpenSsl));
    }
}

#[test]
fn a_release_that_isnt_the_one_asked_for_installs_nothing() {
    for asked in [None, Some(VERSION)] {
        let home = Home::new(&format!("not-asked-{}", asked.unwrap_or("latest")));
        let releases = publish(BEFORE);
        let (sums, option, lines) = match asked {
            None => (
                format!("{}/latest/SHA256SUMS", releases.address()),
                Vec::new(),
                vec![
                    "{sums} names Statac 0.7.3, from before 0.8.0, so nothing was installed.",
                    "  This script comes with Statac 0.8.0 and later, so the newest release is 0.8.0 or later.",
                ],
            ),
            Some(asked) => {
                let older = releases.get(&format!("/v{BEFORE}/SHA256SUMS"));
                releases.put(&format!("/v{asked}/SHA256SUMS"), older);
                (
                    format!("{}/v{asked}/SHA256SUMS", releases.address()),
                    vec![format!("--version={asked}")],
                    vec!["{sums} names Statac 0.7.3, not 0.8.0, so nothing was installed."],
                )
            }
        };
        let options: Vec<&str> = option.iter().map(String::as_str).collect();
        let (said, code) = install(&releases, &home, Tools::SshKeygen, &options);
        assert_eq!(code, Some(1), "{asked:?}\n{said}");
        let lines: Vec<String> = (lines.iter())
            .map(|line| line.replace("{sums}", &sums))
            .collect();
        holds(&said, &lines);
        home.holds_nothing(&said);
    }
}

#[test]
fn a_signature_that_cant_be_downloaded_installs_nothing() {
    // With this machine's own tools: the SSH signature failing, and the
    // other failing where there is no SSH signature.
    for (failing, missing) in [
        ("SHA256SUMS.sshsig", None),
        ("SHA256SUMS.sig", Some("SHA256SUMS.sshsig")),
    ] {
        let home = Home::new(&format!("failing-{failing}"));
        let releases = publish(VERSION);
        releases.fail(&format!("/v{VERSION}/{failing}"));
        if let Some(missing) = missing {
            releases.remove(&format!("/v{VERSION}/{missing}"));
        }
        let (said, code) = install(&releases, &home, Tools::SshKeygen, &[]);
        assert_eq!(code, Some(1), "{failing}\n{said}");
        holds(
            &said,
            &[format!(
                "Can't download {}/v{VERSION}/{failing}",
                releases.address()
            )],
        );
        // curl's own reason, under it.
        assert!(said.contains("  curl: (22) "), "{said}");
        assert!(!said.contains("Installing"), "{said}");
        home.holds_nothing(&said);
    }
}

#[test]
fn a_release_before_0_8_0_is_checked_against_its_sha256sums_alone() {
    let version = BEFORE;
    for tools in EVERY {
        let home = Home::new(&format!("before-{tools:?}"));
        let releases = publish(version);
        let option = format!("--version={version}");
        let (said, code) = install(&releases, &home, tools, &[&option]);
        assert_eq!(code, Some(0), "{tools:?}\n{said}");
        holds(
            &said,
            &[
                "Checked the package against the release's SHA256SUMS. Releases before 0.8.0 aren't signed.".into(),
                format!(
                    "Installed Statac {version} at {}/statac",
                    home.to().display()
                ),
            ],
        );
    }
}
