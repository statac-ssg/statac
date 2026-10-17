//! Writes `THIRD-PARTY.md`, the list of other people's work a built
//! `statac` and a new site carry, each with its licence and the notice it
//! asks for: `cargo run --example third_party`. The tests fail while the
//! crates it names differ from `Cargo.lock`, or the standard library it
//! names is not the one `Cargo.toml` asks for.
//!
//! The crates are those `cargo tree` says a binary links on any of
//! [`TARGETS`]: build scripts' own crates, procedural macros and what only
//! the tests use are in no binary. Each crate's licence texts are read from
//! its source in cargo's registry, which a build fetches.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The systems a binary is built for.
const TARGETS: &[&str] = &[
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "aarch64-unknown-linux-gnu",
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-musl",
    "x86_64-unknown-linux-musl",
    "aarch64-pc-windows-msvc",
    "x86_64-pc-windows-msvc",
    "x86_64-pc-windows-gnu",
];

/// The licences taken where a crate offers a choice, most wanted first. A
/// binary under Apache-2.0 carries the licence's text and what the crate's
/// `NOTICE` file says, so one text serves every crate taken under it.
const PREFERRED: &[&str] = &["Apache-2.0", "MIT"];

/// What a licence file's name says it holds, by a mark in the name in
/// capitals. The first mark found counts, so `0BSD` is before `BSD`.
const MARKS: &[(&str, &str)] = &[
    ("LLVM", "Apache-2.0 WITH LLVM-exception"),
    ("APACHE", "Apache-2.0"),
    ("_A2", "Apache-2.0"),
    ("MIT0", "MIT-0"),
    ("MIT", "MIT"),
    ("ZLIB", "Zlib"),
    ("CC0", "CC0-1.0"),
    ("UNLICENSE", "Unlicense"),
    ("0BSD", "0BSD"),
    ("GPL", "GPL"),
    ("BSD", "BSD"),
    ("ISC", "ISC"),
    ("IJG", "IJG"),
    ("UNICODE", "Unicode-3.0"),
];

/// How a file holding a licence or a notice is named, in capitals.
const NOTICES: &[&str] = &[
    "LICENSE",
    "LICENCE",
    "COPYING",
    "COPYRIGHT",
    "UNLICENSE",
    "NOTICE",
    "PATENTS",
];

/// The licence of another's work a crate holds, which the crate's own
/// licence doesn't name, by the crate's name.
const HELD: &[(&str, &str)] = &[("ferriki-textmate", "LICENSE-vscode-textmate.md")];

const APACHE: &str = "Apache-2.0";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Where the list is written.
pub fn path() -> PathBuf {
    root().join("THIRD-PARTY.md")
}

/// Every crate `Cargo.lock` names that is not the workspace's own, by name
/// and version. A crate of the workspace has no `source`.
pub fn locked(lock: &str) -> BTreeSet<(String, String)> {
    let value = |line: &str, key: &str| {
        let rest = line.strip_prefix(key)?.strip_prefix(" = \"")?;
        Some(rest.strip_suffix('"')?.to_owned())
    };
    let mut crates = BTreeSet::new();
    for package in lock.replace("\r\n", "\n").split("[[package]]\n").skip(1) {
        let find = |key| package.lines().find_map(|line| value(line, key));
        if let (Some(name), Some(version), Some(_)) =
            (find("name"), find("version"), find("source"))
        {
            crates.insert((name, version));
        }
    }
    crates
}

/// Every crate the list names, by name and version: each row of a table
/// that opens with a name in backticks.
pub fn listed(written: &str) -> BTreeSet<(String, String)> {
    written
        .lines()
        .filter_map(|line| {
            let mut cells = line.strip_prefix("| `")?.split(" | ");
            let name = cells.next()?.strip_suffix('`')?;
            let version = cells.next()?.trim_end_matches(" |");
            Some((name.to_owned(), version.to_owned()))
        })
        .collect()
}

/// A crate a binary links.
struct Linked {
    licence: String,
    repository: String,
}

/// Every crate a binary links on any of [`TARGETS`], from `cargo tree`.
fn linked(known: &BTreeSet<(String, String)>) -> BTreeMap<(String, String), Linked> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = Command::new(cargo);
    command.current_dir(root()).args([
        "tree",
        "--locked",
        "--package=statac",
        "--edges=normal",
        "--prefix=depth",
        "--no-dedupe",
        "--format={p}\t{l}\t{r}",
    ]);
    for target in TARGETS {
        command.arg(format!("--target={target}"));
    }
    let output = command.output().expect("cargo runs");
    assert!(
        output.status.success(),
        "cargo tree: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut crates = BTreeMap::new();
    // A procedural macro runs in the compiler: neither it nor what it
    // links is in a binary.
    let mut skipped: Option<usize> = None;
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let digits = line.bytes().take_while(u8::is_ascii_digit).count();
        let Ok(depth) = line[..digits].parse::<usize>() else {
            continue;
        };
        if skipped.is_some_and(|above| depth > above) {
            continue;
        }
        skipped = None;
        let mut cells = line[digits..].split('\t');
        let package = cells.next().unwrap_or_default();
        if package.ends_with("(proc-macro)") {
            skipped = Some(depth);
            continue;
        }
        let mut words = package.split(' ');
        let name = words.next().unwrap_or_default().to_owned();
        let version = words.next().unwrap_or_default().trim_start_matches('v');
        let key = (name, version.to_owned());
        if known.contains(&key) {
            crates.entry(key).or_insert_with(|| Linked {
                licence: cells.next().unwrap_or_default().to_owned(),
                repository: cells.next().unwrap_or_default().to_owned(),
            });
        }
    }
    crates
}

/// The licences a crate is taken under, from the expression its manifest
/// gives: every licence joined by `AND`, and of those offered by `OR` the
/// one [`PREFERRED`] puts first.
fn taken(expression: &str) -> BTreeSet<String> {
    struct Terms<'a> {
        words: Vec<&'a str>,
        at: usize,
    }
    impl Terms<'_> {
        fn eat(&mut self, word: &str) -> bool {
            let found = self.words.get(self.at) == Some(&word);
            self.at += usize::from(found);
            found
        }
        fn word(&mut self) -> String {
            let word = self.words.get(self.at).copied().unwrap_or_default();
            self.at += 1;
            word.to_owned()
        }
        fn any(&mut self) -> BTreeSet<String> {
            let mut best = self.all();
            while self.eat("OR") {
                let other = self.all();
                if rank(&other) < rank(&best) {
                    best = other;
                }
            }
            best
        }
        fn all(&mut self) -> BTreeSet<String> {
            let mut set = self.one();
            while self.eat("AND") {
                set.extend(self.one());
            }
            set
        }
        fn one(&mut self) -> BTreeSet<String> {
            if self.eat("(") {
                let set = self.any();
                self.eat(")");
                return set;
            }
            let mut id = self.word();
            if self.eat("WITH") {
                id = format!("{id} WITH {}", self.word());
            }
            BTreeSet::from([id])
        }
    }
    fn rank(set: &BTreeSet<String>) -> usize {
        let place = |id: &String| PREFERRED.iter().position(|wanted| wanted == id);
        set.iter()
            .map(|id| place(id).unwrap_or(PREFERRED.len()))
            .max()
            .unwrap_or(PREFERRED.len())
    }
    // An older manifest writes a choice with a slash.
    let spaced = expression
        .replace('/', " OR ")
        .replace('(', " ( ")
        .replace(')', " ) ");
    let mut terms = Terms {
        words: spaced.split_whitespace().collect(),
        at: 0,
    };
    terms.any()
}

/// The folder of a crate's source in cargo's registry.
fn source(name: &str, version: &str) -> PathBuf {
    let home = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::home_dir().map(|home| home.join(".cargo")))
        .expect("cargo has a home folder");
    let registries = home.join("registry/src");
    let folder = format!("{name}-{version}");
    std::fs::read_dir(&registries)
        .unwrap_or_else(|error| panic!("{}: {error}", registries.display()))
        .filter_map(|entry| Some(entry.ok()?.path().join(&folder)))
        .find(|path| path.is_dir())
        .unwrap_or_else(|| panic!("{folder} is not in {}", registries.display()))
}

/// The files of a crate that a binary taken under `taken` has to carry: the
/// text of each licence taken, any file that names no licence, and any
/// `NOTICE`. The Apache text is the same for every crate and is not among
/// them.
fn notices(name: &str, folder: &Path, taken: &BTreeSet<String>) -> Vec<String> {
    let only_apache = taken.len() == 1 && taken.contains(APACHE);
    let mut files: Vec<String> = std::fs::read_dir(folder)
        .unwrap_or_else(|error| panic!("{}: {error}", folder.display()))
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let upper = name.to_uppercase();
            let wanted = entry.path().is_file() && NOTICES.iter().any(|way| upper.starts_with(way));
            wanted.then_some(name)
        })
        .filter(|file| {
            let upper = file.to_uppercase();
            if upper.starts_with("NOTICE") {
                return true;
            }
            let says = MARKS.iter().find(|(mark, _)| upper.contains(mark));
            match says {
                _ if only_apache => false,
                None => true,
                Some((_, APACHE)) => false,
                Some((_, licence)) => taken.iter().any(|id| id.starts_with(licence)),
            }
        })
        .collect();
    files.extend(
        HELD.iter()
            .filter(|(of, _)| *of == name)
            .map(|(_, file)| (*file).to_owned()),
    );
    files.sort();
    files
}

/// Every distinct text, in the order first met, with what each is for.
#[derive(Default)]
struct Texts {
    all: Vec<(String, Vec<String>)>,
}

impl Texts {
    /// The number of `text` in the list, adding it where it is new. Texts
    /// that differ only in line endings or space at a line's end are one.
    fn number(&mut self, of: String, text: &str) -> usize {
        let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
        let text = lines.join("\n").trim_matches('\n').to_owned();
        let at = match self.all.iter().position(|(held, _)| *held == text) {
            Some(at) => at,
            None => {
                self.all.push((text, Vec::new()));
                self.all.len() - 1
            }
        };
        if !self.all[at].1.contains(&of) {
            self.all[at].1.push(of);
        }
        at + 1
    }

    fn file(&mut self, of: String, path: &Path) -> usize {
        let bytes =
            std::fs::read(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        self.number(of, &String::from_utf8_lossy(&bytes))
    }
}

/// The numbers as the list shows them, each once, in order.
fn numbers(numbers: &[usize]) -> String {
    let numbers: BTreeSet<usize> = numbers.iter().copied().collect();
    let said: Vec<String> = numbers.iter().map(|number| format!("[{number}]")).collect();
    said.join(" ")
}

fn read(path: &str) -> String {
    let path = root().join(path);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The lines of a record's lists, each with the numbers of the texts it
/// names: the files in backticks named as a licence or a notice is, read
/// from the record's folder, and the Apache text for what is taken under
/// it.
fn recorded(out: &mut String, texts: &mut Texts, folder: &str, apache: usize) {
    let record = read(&format!("{folder}/licences.md"));
    let mut all = Vec::new();
    for line in record.lines().filter(|line| line.starts_with("- ")) {
        let mut found = Vec::new();
        if line.contains("taken under Apache-2.0") {
            found.push(apache);
        }
        for file in line.split('`').skip(1).step_by(2) {
            let name = file.rsplit('/').next().unwrap_or_default();
            if NOTICES.iter().any(|way| name.starts_with(way)) {
                let path = root().join(folder).join(file);
                found.push(texts.file(format!("`{folder}/{file}`"), &path));
            }
        }
        all.extend(found.iter().copied());
        writeln!(out, "{line} {}", numbers(&found)).unwrap();
    }
    assert!(!all.is_empty(), "{folder} names no licence");
}

/// The lines of a list under a heading of the highlighter's notices.
fn carried(record: &str, heading: &str) -> usize {
    let lines = record.lines().skip_while(|line| *line != heading).skip(1);
    lines
        .take_while(|line| !line.starts_with("## "))
        .filter(|line| line.starts_with("- "))
        .count()
}

/// The whole list.
fn list() -> String {
    let lock = locked(&read("Cargo.lock"));
    let linked = linked(&lock);
    let mut texts = Texts::default();
    let apache = texts.number("The Apache License 2.0".to_owned(), &read("LICENSE-APACHE"));

    let mut out = String::from(
        "# What Statac carries\n\n\
         Written by `cargo run --example third_party`.\n\n\
         Statac's own code is offered under MIT or Apache-2.0, at your choice. \
         A built `statac` and a site `statac new` makes also carry other \
         people's work, and each piece keeps its own licence. This is all of \
         it. A number in square brackets is a text under \
         [Licence texts](#licence-texts), where each text is given once.\n",
    );

    out.push_str(
        "\n## In every new site\n\n\
         - The sample photographs, the recording and the video a new site \
         starts with, kept here under `starter/site/content/`, are Statac's \
         own. Each was made for it by a program: none is a picture or a \
         recording of anything, and none holds anyone else's work. They are \
         offered as Statac is, under MIT or Apache-2.0.\n",
    );

    out.push_str("\n## In a site that turns sharing cards on\n\n");
    let font = texts.file(
        "Noto Sans, `templates/og/OFL.txt` in a site that turns sharing cards on".to_owned(),
        &root().join("starter/site/templates/og/OFL.txt"),
    );
    writeln!(
        out,
        "- Noto Sans, the font of the sharing cards, written as \
         `templates/og/NotoSans-Regular.ttf`: SIL Open Font License 1.1. \
         `statac new --cards` writes the licence beside the font, as \
         `templates/og/OFL.txt`. [{font}] Its source is recorded in \
         `src/cards/fonts/fonts.md`."
    )
    .unwrap();

    let emoji = texts.file(
        "Noto Emoji, `templates/og/NotoEmoji-OFL.txt` in a site that turns sharing cards on"
            .to_owned(),
        &root().join("starter/site/templates/og/NotoEmoji-OFL.txt"),
    );
    writeln!(
        out,
        "- Noto Emoji, the font of the emoji on the sharing cards, written as \
         `templates/og/NotoEmoji.ttf`: SIL Open Font License 1.1. \
         `statac new --cards` writes the licence beside the font, as \
         `templates/og/NotoEmoji-OFL.txt`. [{emoji}] Its source is recorded in \
         `src/cards/fonts/fonts.md`."
    )
    .unwrap();

    out.push_str("\n## In the binary\n");
    out.push_str("\n### Words for dates\n\n");
    let cldr = texts.file(
        "The Unicode CLDR data, `src/language/bundled/LICENSE-CLDR.txt`".to_owned(),
        &root().join("src/language/bundled/LICENSE-CLDR.txt"),
    );
    writeln!(
        out,
        "- What Statac has for every language but English, in \
         `src/language/bundled/`, is from the Unicode CLDR: the names of the \
         months and the weekdays, the words for am and pm, the quotation \
         marks, the direction of writing and the digits. So is the usual \
         form of a date in each of those languages, in \
         `src/dates/format.rs`, so are English's months, weekdays and \
         quotation marks, and so is the direction of a language Statac has no \
         words for, in `src/language/direction.rs`: Unicode License V3. \
         [{cldr}]"
    )
    .unwrap();

    out.push_str("\n### Grammars and themes\n\n");
    let record = read("THIRD-PARTY-GRAMMARS.md");
    writeln!(
        out,
        "- {} grammars and {} themes colour code. Each is listed with its \
         licence, its source and the notice it asks for, copyright line and \
         all, in `THIRD-PARTY-GRAMMARS.md`, beside this file.",
        carried(&record, "## Grammars"),
        carried(&record, "## Themes")
    )
    .unwrap();

    out.push_str("\n### The server\n\n");
    writeln!(
        out,
        "- The HTTP server `statac serve` answers with, in \
         `src/serve/server/`, is the server of oxhttp 0.3.3, Copyright (c) \
         2018 Oxigraph developers: MIT OR Apache-2.0, taken under \
         Apache-2.0. https://github.com/oxigraph/oxhttp. [{apache}]"
    )
    .unwrap();

    out.push_str("\n### The template engine\n\n");
    writeln!(
        out,
        "- The template engine that writes a site's pages, kept here \
         in `crates/minijinja/`, is MiniJinja 3.0.1, whose licence names no \
         copyright holder: Apache-2.0. https://github.com/mitsuhiko/minijinja. \
         It holds part of self_cell, in \
         `crates/minijinja/src/vendor/self_cell/`, also under Apache-2.0. A \
         change made to the engine here is marked in the file it touches. \
         [{apache}]"
    )
    .unwrap();

    out.push_str(
        "\n### C libraries built from source kept here\n\n\
         The paths are in `crates/avif-aom-sys/`.\n\n",
    );
    recorded(&mut out, &mut texts, "crates/avif-aom-sys", apache);

    out.push_str(
        "\n### C libraries a crate carries\n\n\
         Each is built from the source its crate holds, under the crate's \
         entry below.\n\n\
         - AWS-LC, in `aws-lc-sys`: the licences its entry names.\n",
    );

    out.push_str(
        "\n### An acknowledgement\n\n\
         `jpeg-encoder` holds code from the Independent JPEG Group's \
         software, whose terms ask for these words: this software is based \
         in part on the work of the Independent JPEG Group.\n",
    );

    out.push_str(
        "\n### Rust's standard library\n\n\
         A binary links the standard library of the Rust that builds it. \
         What that carries is recorded in `licences/rust-std/licences.md`, \
         with the texts beside it. Neither Rust nor any crate here has a \
         `NOTICE` file, so what is taken under Apache-2.0 asks for the \
         licence's text alone.\n\n",
    );
    recorded(&mut out, &mut texts, "licences/rust-std", apache);

    out.push_str(
        "\n### What the toolchain adds\n\n\
         A binary for Linux that asks nothing of the system's C library \
         links a C library and an unwinder that come with Rust's standard \
         library, recorded in `licences/toolchain/licences.md`, with the \
         texts beside it. No other binary links either. A binary for \
         Windows links Microsoft's C runtime, which asks for no notice.\n\n",
    );
    recorded(&mut out, &mut texts, "licences/toolchain", apache);

    let mut rows = String::new();
    let mut bare = Vec::new();
    let mut with_notice = 0;
    for ((name, version), about) in &linked {
        let taken = taken(&about.licence);
        let folder = source(name, version);
        let files = notices(name, &folder, &taken);
        let mut found: Vec<usize> = files
            .iter()
            .map(|file| texts.file(format!("`{name}` {version}, `{file}`"), &folder.join(file)))
            .collect();
        if taken.contains(APACHE) {
            found.insert(0, apache);
        }
        with_notice += usize::from(
            files
                .iter()
                .any(|file| file.to_uppercase().starts_with("NOTICE")),
        );
        let needs_its_own = taken.iter().any(|id| id != APACHE);
        let said = match files.is_empty() && needs_its_own {
            true => {
                bare.push(format!("`{name}` {version} ({})", about.repository));
                format!("{} none in the crate", numbers(&found))
                    .trim()
                    .to_owned()
            }
            false => numbers(&found),
        };
        let taken: Vec<&str> = taken.iter().map(String::as_str).collect();
        writeln!(
            rows,
            "| `{name}` | {version} | {} | {} | {said} |",
            about.licence,
            taken.join(", ")
        )
        .unwrap();
    }
    let notice = match with_notice {
        0 => "none of these crates has one",
        _ => "each such file is among its crate's texts",
    };
    writeln!(
        out,
        "\n### Rust crates\n\n\
         The {} crates a binary links on macOS, Linux or Windows; some are \
         linked on one of them only. Where a crate offers a choice of \
         licence, the one taken is Apache-2.0 where it is offered and \
         otherwise MIT. A binary taken under Apache-2.0 carries the \
         licence's text [{apache}] and what the crate's `NOTICE` file says; \
         {notice}. Under the other licences it carries the crate's own text, \
         with its copyright line.\n\n\
         | Crate | Version | Its licence | Taken under | Texts |\n\
         | --- | --- | --- | --- | --- |\n{rows}",
        linked.len()
    )
    .unwrap();
    if !bare.is_empty() {
        writeln!(
            out,
            "These crates hold no licence file, so there is no copyright \
             line of theirs to give. Each one's manifest names its licence \
             and its source: {}.",
            bare.join(", ")
        )
        .unwrap();
    }

    let unlinked: Vec<&(String, String)> = lock
        .iter()
        .filter(|key| !linked.contains_key(key))
        .collect();
    writeln!(
        out,
        "\n## Crates in no binary\n\n\
         `Cargo.lock` names {} more crates. They build or test Statac, or \
         another crate names them for a system or a feature Statac does not \
         use, and no binary links them.\n\n\
         | Crate | Version |\n\
         | --- | --- |",
        unlinked.len()
    )
    .unwrap();
    for (name, version) in unlinked {
        writeln!(out, "| `{name}` | {version} |").unwrap();
    }

    out.push_str("\n## Licence texts\n");
    for (at, (text, of)) in texts.all.iter().enumerate() {
        assert!(
            !text.lines().any(|line| line.starts_with("````")),
            "a text of {of:?} would close its own block"
        );
        writeln!(
            out,
            "\n### [{}]\n\n{}.\n\n````text\n{text}\n````",
            at + 1,
            of.join("; ")
        )
        .unwrap();
    }
    out
}

fn main() {
    let written = list();
    std::fs::write(path(), &written).expect("the list can be written");
    println!(
        "Wrote {} crates to {}.",
        listed(&written).len(),
        path().display()
    );
}
