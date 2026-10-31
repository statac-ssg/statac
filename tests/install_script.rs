//! The install script for Windows, `tools/install.ps1`, run as its
//! instructions run it, `irm <address> | iex`, by Windows PowerShell and,
//! where the machine has it, PowerShell 7: against a server on this
//! machine standing in for get.statac.dev, with a key of the tests' own
//! standing in for Statac's, and a folder standing in for
//! `%LOCALAPPDATA%`. The user's PATH, which the script changes, is set to
//! the PATH with entries of the tests' own added, and put back after. The
//! script's own check of a signature is held to RFC 8032's test vectors
//! and to changes of them that it must refuse.
#![cfg(windows)]

#[path = "ed25519/cases.rs"]
mod ed25519;
mod scratch;

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

use rustls::SignatureScheme;
use rustls::crypto::aws_lc_rs;
use rustls::pki_types::PrivatePkcs8KeyDer;
use scratch::Scratch;

/// Statac's key, as the script carries it, and the tests' own, its
/// private half in PKCS #8, as `src/upgrade/tests.rs` has them.
const KEY: &str = "9ba90e0b6ec11a149baad9a54d6cf33d15a872e75c08d915f7361225d32febde";
const TEST_KEY: &str = "530c1dad1c6081a3172af0e023067098707b8619b7725705cbd5bd3e7299cc9b";
const TEST_PRIVATE: &str = "302e020100300506032b657004220420c8e9544d25e30fee7798a7808d7fc8495fdec2be7890ab945763f9096ee66dcd";

const VERSION: &str = "0.8.0";
const TARGET: &str = "x86_64-pc-windows-msvc";
const LICENCES: [&str; 4] = [
    "LICENSE-MIT",
    "LICENSE-APACHE",
    "THIRD-PARTY.md",
    "THIRD-PARTY-GRAMMARS.md",
];

fn bytes_of_hex(text: &str) -> Vec<u8> {
    (0..text.len() / 2)
        .map(|at| u8::from_str_radix(&text[at * 2..at * 2 + 2], 16).unwrap())
        .collect()
}

fn hex_of(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The tests' key's signature of `message`.
fn signature(message: &[u8]) -> Vec<u8> {
    let der = bytes_of_hex(TEST_PRIVATE);
    let key = aws_lc_rs::sign::any_eddsa_type(&PrivatePkcs8KeyDer::from(&der[..])).unwrap();
    let signer = key.choose_scheme(&[SignatureScheme::ED25519]).unwrap();
    signer.sign(message).unwrap()
}

fn sha256(bytes: &[u8]) -> String {
    let suite = aws_lc_rs::cipher_suite::TLS13_AES_128_GCM_SHA256
        .tls13()
        .unwrap();
    hex_of(suite.common.hash_provider.hash(bytes).as_ref())
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
                // As get.statac.dev serves the scripts.
                let kind = match path.ends_with(".zip") {
                    true => "application/zip",
                    false => "text/plain; charset=utf-8",
                };
                let head = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
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

    /// The script, carrying the tests' key in place of Statac's, and a
    /// release of `package` signed with it, which latest/ names.
    fn publish(package: &[u8]) -> Releases {
        let releases = Releases::start();
        let script = include_str!("../tools/install.ps1");
        assert_eq!(script.matches(KEY).count(), 1);
        releases.put("/install.ps1", script.replace(KEY, TEST_KEY));
        let name = format!("statac-{VERSION}-{TARGET}.zip");
        let sums = format!(
            "{}  statac-{VERSION}-aarch64-apple-darwin.tar.gz\n{} *{name}\n",
            sha256(b"another platform's"),
            sha256(package),
        );
        releases.put(&format!("/v{VERSION}/{name}"), package);
        releases.put(&format!("/v{VERSION}/SHA256SUMS"), sums.clone());
        releases.put(
            &format!("/v{VERSION}/SHA256SUMS.sig"),
            signature(sums.as_bytes()),
        );
        releases.put("/latest/SHA256SUMS", sums);
        releases
    }
}

/// The Windows package of the Statac these tests are built with, made as
/// the release makes it, by Windows PowerShell's `Compress-Archive`.
fn package() -> &'static [u8] {
    static PACKAGE: OnceLock<Vec<u8>> = OnceLock::new();
    PACKAGE.get_or_init(|| {
        let staging = Scratch::new("install_script", "package");
        let name = format!("statac-{VERSION}-{TARGET}");
        let folder = staging.join(&name);
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::copy(env!("CARGO_BIN_EXE_statac"), folder.join("statac.exe")).unwrap();
        for licence in LICENCES {
            let from = Path::new(env!("CARGO_MANIFEST_DIR")).join(licence);
            std::fs::copy(from, folder.join(licence)).unwrap();
        }
        let made = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command"])
            .arg(format!(
                "Compress-Archive -Path '{name}' -DestinationPath '{name}.zip'"
            ))
            .current_dir(&*staging)
            .env_remove("PSModulePath")
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(made.status.success(), "{made:?}");
        std::fs::read(staging.join(format!("{name}.zip"))).unwrap()
    })
}

/// The PowerShells this machine has: Windows PowerShell, always, and
/// PowerShell 7 where it is installed.
fn shells() -> Vec<&'static str> {
    let mut shells = vec!["powershell.exe"];
    let seven = Command::new("pwsh.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", "exit 0"])
        .stdin(Stdio::null())
        .output();
    if seven.is_ok_and(|output| output.status.success()) {
        shells.push("pwsh.exe");
    }
    shells
}

/// What a PowerShell script `script` printed, run by Windows PowerShell,
/// stopping the test where it didn't run through.
fn powershell(script: &str) -> String {
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .env_remove("PSModulePath")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// `text` in PowerShell's single quotes.
fn quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

/// The user's PATH as the registry holds it, the kind of value and the
/// value, read through a file so no code page can change it.
fn user_path() -> Option<(String, String)> {
    let file = std::env::temp_dir().join(format!("statac-user-path-{}", std::process::id()));
    powershell(&format!(
        "$k = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment'); \
         $v = $null; \
         if ($k) {{ $v = $k.GetValue('Path', $null, 'DoNotExpandEnvironmentNames') }}; \
         $text = ''; \
         if ($null -ne $v) {{ $text = $k.GetValueKind('Path').ToString() + [char]10 + $v }}; \
         [IO.File]::WriteAllText({}, $text)",
        quoted(&file.display().to_string())
    ));
    let text = std::fs::read_to_string(&file).unwrap();
    std::fs::remove_file(&file).unwrap();
    let (kind, value) = text.split_once('\n')?;
    Some((kind.to_owned(), value.to_owned()))
}

fn set_user_path(path: Option<&(String, String)>) {
    let change = match path {
        Some((kind, value)) => format!("$k.SetValue('Path', {}, {})", quoted(value), quoted(kind)),
        None => "$k.DeleteValue('Path', $false)".to_owned(),
    };
    powershell(&format!(
        "$k = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey('Environment'); {change}"
    ));
}

/// The user's PATH, held by one test at a time, with entries of the
/// tests' own added that the script must keep as written: one naming a
/// variable, and enough long ones to take it past the 1,024 characters
/// `setx` would cut it to. Put back as it was when dropped.
struct UserPath {
    before: Option<(String, String)>,
    tests: String,
    _held: MutexGuard<'static, ()>,
}

impl UserPath {
    fn set_aside() -> UserPath {
        static HELD: Mutex<()> = Mutex::new(());
        let held = HELD.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let before = user_path();
        let mut entries = vec![r"%USERPROFILE%\statac-tests-entry".to_owned()];
        for at in 0..6 {
            entries.push(format!(r"C:\statac-tests-entry-{at}\{}", "a".repeat(180)));
        }
        let tests = match &before {
            Some((_, value)) if !value.trim_end_matches(';').is_empty() => {
                format!("{};{}", value.trim_end_matches(';'), entries.join(";"))
            }
            _ => entries.join(";"),
        };
        set_user_path(Some(&("ExpandString".to_owned(), tests.clone())));
        UserPath {
            before,
            tests,
            _held: held,
        }
    }

    /// The user's PATH now, which must be of the kind it was set to.
    fn now(&self) -> String {
        let (kind, value) = user_path().unwrap();
        assert_eq!(kind, "ExpandString");
        value
    }
}

impl Drop for UserPath {
    fn drop(&mut self) {
        set_user_path(self.before.as_ref());
    }
}

/// `path` as the author would write it, every link resolved, without the
/// prefix a path made whole carries on Windows.
fn whole(path: &Path) -> PathBuf {
    let path = std::fs::canonicalize(path).unwrap().display().to_string();
    PathBuf::from(path.strip_prefix(r"\\?\").unwrap_or(&path))
}

/// A folder of a test's own standing in for `%LOCALAPPDATA%`.
struct Local {
    _scratch: Scratch,
    folder: PathBuf,
}

impl Local {
    fn new(label: &str) -> Local {
        let scratch = Scratch::new("install_script", label);
        std::fs::create_dir_all(&*scratch).unwrap();
        let folder = whole(&scratch);
        Local {
            _scratch: scratch,
            folder,
        }
    }

    fn programs(&self) -> PathBuf {
        self.folder.join(r"Programs\Statac")
    }

    fn licences(&self) -> PathBuf {
        self.folder.join("Statac")
    }
}

/// What the script said and its exit code, run by `shell` as
/// `irm <address> | iex`, with `options` its environment variables.
fn install(
    shell: &str,
    releases: &Releases,
    local: &Local,
    options: &[(&str, &str)],
) -> (String, Option<i32>) {
    let mut command = Command::new(shell);
    command
        .args(["-NoProfile", "-NonInteractive", "-Command"])
        .arg(format!("irm {}/install.ps1 | iex", releases.address()))
        .env("STATAC_RELEASES", releases.address())
        .env("LOCALAPPDATA", &local.folder)
        .env_remove("STATAC_TO")
        .env_remove("STATAC_UPDATES")
        .env_remove("STATAC_VERSION")
        .env_remove("PSModulePath")
        .stdin(Stdio::null());
    for (name, value) in options {
        command.env(name, value);
    }
    let output = command.output().unwrap();
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (said.replace("\r\n", "\n"), output.status.code())
}

fn statac(program: &Path, args: &[&str]) -> (String, Option<i32>) {
    let output = Command::new(program)
        .args(args)
        .env("NO_COLOR", "1")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    (said, output.status.code())
}

fn holds(said: &str, lines: &[String]) {
    for line in lines {
        assert!(said.contains(&format!("{line}\n")), "{line}\n\n{said}");
    }
}

#[test]
fn a_fresh_install_takes_the_defaults_and_adds_its_folder_to_the_users_path() {
    for shell in shells() {
        let releases = Releases::publish(package());
        let local = Local::new(&format!("defaults-{shell}"));
        let user_path = UserPath::set_aside();
        let folder = local.programs();
        let licences = local.licences();
        let program = folder.join("statac.exe");
        let marker = folder.join(".statac-install");
        let (folder_shown, licences_shown) = (folder.display(), licences.display());

        let (said, code) = install(shell, &releases, &local, &[]);
        assert_eq!(code, Some(0), "{shell}\n{said}");
        holds(
            &said,
            &[
                format!("Installing Statac {VERSION} ({TARGET})."),
                format!(
                    "No terminal to ask in, so Statac goes in {folder_shown}, and doesn't keep itself up to date."
                ),
                "  To choose: $env:STATAC_TO = '<folder>'; $env:STATAC_UPDATES = 'on'; irm https://get.statac.dev/install.ps1 | iex".into(),
                format!("Downloading statac-{VERSION}-{TARGET}.zip"),
                "Checked the package against the release's SHA256SUMS, and SHA256SUMS against".into(),
                "  its signature.".into(),
                format!(
                    "Installed Statac {} at {}",
                    env!("CARGO_PKG_VERSION"),
                    program.display()
                ),
                format!("Its licences are in {licences_shown}"),
                "Statac doesn't keep itself up to date.".into(),
                format!(
                    "Added {folder_shown} to your PATH, so a terminal opened from now on finds `statac`."
                ),
            ],
        );
        let written = format!(
            "# Written by Statac's install script. statac upgrade and statac uninstall read it.\n\
             updates: off\nlicences: {licences_shown}\npath: {folder_shown}\n"
        );
        assert_eq!(std::fs::read_to_string(&marker).unwrap(), written);
        for licence in LICENCES {
            assert!(licences.join(licence).is_file(), "{licence}");
        }
        let joined = format!("{};{folder_shown}", user_path.tests);
        assert_eq!(user_path.now(), joined, "{shell}");

        // Again, into the same folder: the PATH holds it once.
        let (said, code) = install(shell, &releases, &local, &[]);
        assert_eq!(code, Some(0), "{shell}\n{said}");
        holds(
            &said,
            &[format!(
                "No terminal to ask in, so Statac goes in {folder_shown}, and doesn't keep itself up to date, as before."
            )],
        );
        assert!(!said.contains("to your PATH"), "{said}");
        assert_eq!(std::fs::read_to_string(&marker).unwrap(), written);
        assert_eq!(user_path.now(), joined, "{shell}");

        let (said, code) = statac(&program, &["upgrade", "--updates=on"]);
        assert_eq!(code, Some(0), "{said}");
        assert_eq!(
            std::fs::read_to_string(&marker).unwrap(),
            written.replace("updates: off", "updates: on")
        );

        let (said, code) = statac(&program, &["uninstall"]);
        assert_eq!(code, Some(0), "{said}");
        holds(
            &said,
            &[
                format!(
                    "Uninstalled Statac {} from `{folder_shown}`. Statac removes its program file once this command ends.",
                    env!("CARGO_PKG_VERSION")
                ),
                format!(
                    "Removed its licences, and their folder `{licences_shown}`, which held nothing else."
                ),
                format!("Removed `{folder_shown}` from your PATH."),
            ],
        );
        assert!(!marker.exists());
        assert!(!licences.exists());
        assert_eq!(user_path.now(), user_path.tests, "{shell}");
        // The program, moved aside, goes once the command has ended.
        let left = || std::fs::read_dir(&folder).unwrap().count();
        let since = Instant::now();
        while left() > 0 && since.elapsed() < Duration::from_secs(30) {
            std::thread::sleep(Duration::from_millis(250));
        }
        assert_eq!(left(), 0);
    }
}

#[test]
fn an_install_goes_to_the_folder_given_with_updates_off() {
    for shell in shells() {
        let releases = Releases::publish(package());
        let local = Local::new(&format!("given-{shell}"));
        let user_path = UserPath::set_aside();
        let folder = local.folder.join("Tools").join("Statac Here");
        let licences = local.licences();
        let (folder_shown, licences_shown) = (folder.display(), licences.display());

        // Pasted with quotes round it and a closing backslash.
        let given = format!("\"{folder_shown}\\\"");
        let options = [("STATAC_TO", given.as_str()), ("STATAC_UPDATES", "off")];
        let (said, code) = install(shell, &releases, &local, &options);
        assert_eq!(code, Some(0), "{shell}\n{said}");
        assert!(!said.contains("No terminal"), "{said}");
        holds(
            &said,
            &[
                format!(
                    "Installed Statac {} at {folder_shown}\\statac.exe",
                    env!("CARGO_PKG_VERSION")
                ),
                "Statac doesn't keep itself up to date.".into(),
                "Turn this on with `statac upgrade --updates=on`.".into(),
            ],
        );
        assert_eq!(
            std::fs::read_to_string(folder.join(".statac-install")).unwrap(),
            format!(
                "# Written by Statac's install script. statac upgrade and statac uninstall read it.\n\
                 updates: off\nlicences: {licences_shown}\npath: {folder_shown}\n"
            )
        );
        assert!(!local.programs().exists());
        assert_eq!(
            user_path.now(),
            format!("{};{folder_shown}", user_path.tests)
        );
    }
}

#[test]
fn a_release_that_doesnt_match_its_signature_installs_nothing() {
    for shell in shells() {
        for (label, line) in [
            (
                "package",
                "The package doesn't match the release's SHA256SUMS, so nothing was installed.",
            ),
            (
                "sums",
                "The release's SHA256SUMS isn't signed with Statac's key, so nothing was installed.",
            ),
        ] {
            let releases = Releases::publish(package());
            let name = format!("statac-{VERSION}-{TARGET}.zip");
            match label {
                "package" => {
                    let mut tampered = package().to_vec();
                    tampered.push(0);
                    releases.put(&format!("/v{VERSION}/{name}"), tampered);
                }
                _ => {
                    let sums = format!("{} *{name}\n", sha256(b"another package"));
                    releases.put(&format!("/v{VERSION}/SHA256SUMS"), sums.clone());
                    releases.put("/latest/SHA256SUMS", sums);
                }
            }
            let local = Local::new(&format!("refused-{label}-{shell}"));
            let user_path = UserPath::set_aside();
            let (said, code) = install(shell, &releases, &local, &[]);
            assert_eq!(code, Some(1), "{shell}\n{said}");
            holds(&said, &[line.to_owned()]);
            assert!(!local.programs().exists(), "{shell}\n{said}");
            assert!(!local.licences().exists(), "{shell}\n{said}");
            assert_eq!(user_path.now(), user_path.tests);
        }
    }
}

#[test]
fn the_scripts_signature_check_meets_rfc_8032() {
    let cases = ed25519::cases();
    let scratch = Scratch::new("install_script", "rfc8032");
    std::fs::create_dir_all(&*scratch).unwrap();
    let mut list = String::new();
    for (at, case) in cases.iter().enumerate() {
        let path = scratch.join(format!("case-{at}"));
        std::fs::write(path.with_extension("key"), hex_of(&case.key)).unwrap();
        std::fs::write(path.with_extension("sig"), &case.signature).unwrap();
        std::fs::write(path.with_extension("message"), &case.message).unwrap();
        list.push_str(&format!("{}\n", path.display()));
    }
    let listed = scratch.join("cases.txt");
    std::fs::write(&listed, list).unwrap();
    // The script as published, with each case checked where it would
    // install: its check called as it calls it, on the files' bytes and
    // the key in hex.
    let script = include_str!("../tools/install.ps1");
    let install = "\n        Install-Statac\n";
    assert_eq!(script.matches(install).count(), 1);
    let checks = format!(
        "\n        foreach ($case in [IO.File]::ReadAllLines({})) {{\n\
         \x20           $caseKey = [IO.File]::ReadAllText(\"$case.key\")\n\
         \x20           $message = [IO.File]::ReadAllBytes(\"$case.message\")\n\
         \x20           $signature = [IO.File]::ReadAllBytes(\"$case.sig\")\n\
         \x20           $Error.Clear()\n\
         \x20           try {{\n\
         \x20               $verdicts = @(Test-Signature $message $signature (ConvertFrom-Hex $caseKey))\n\
         \x20               if ($Error.Count -gt 0) {{\n\
         \x20                   $said = 'error: ' + $Error[0]\n\
         \x20               }} elseif ($verdicts.Count -ne 1 -or $verdicts[0] -isnot [bool]) {{\n\
         \x20                   $said = 'stray: ' + ($verdicts -join ', ')\n\
         \x20               }} elseif ($verdicts[0]) {{\n\
         \x20                   $said = 'accepted'\n\
         \x20               }} else {{\n\
         \x20                   $said = 'refused'\n\
         \x20               }}\n\
         \x20           }} catch {{\n\
         \x20               $said = 'error: ' + $_\n\
         \x20           }}\n\
         \x20           Write-Host ('{{0}} {{1}}' -f [IO.Path]::GetFileName($case), $said)\n\
         \x20       }}\n",
        quoted(&listed.display().to_string())
    );
    let checking = scratch.join("checks.ps1");
    std::fs::write(&checking, script.replace(install, &checks)).unwrap();
    for shell in shells() {
        let output = Command::new(shell)
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(&checking)
            .env_remove("PSModulePath")
            .stdin(Stdio::null())
            .output()
            .unwrap();
        let said = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
        let errors = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success() && errors.is_empty(),
            "{shell}\n{said}\n{errors}"
        );
        let verdicts: HashMap<&str, &str> = (said.lines())
            .filter_map(|line| line.split_once(' '))
            .collect();
        let wrong: Vec<String> = (cases.iter().enumerate())
            .filter_map(|(at, case)| {
                let expected = match case.accepted {
                    true => "accepted",
                    false => "refused",
                };
                let found = verdicts.get(format!("case-{at}").as_str()).copied();
                (found != Some(expected)).then(|| {
                    format!(
                        "{}: {} where {expected}",
                        case.label,
                        found.unwrap_or("nothing said")
                    )
                })
            })
            .collect();
        assert!(wrong.is_empty(), "{shell}\n{}", wrong.join("\n"));
    }
}
