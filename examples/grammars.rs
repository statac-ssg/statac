//! Writes what Statac colours code with, from the grammars of npm's
//! `tm-grammars` package and the themes in `src/highlight/themes/`.
//!
//! The grammars checked in were taken from `tm-grammars` 1.32.22, the
//! version [`PACKAGE`] names. To take them again, fetch and unpack that
//! version and hand the generator its folder:
//!
//! ```text
//! curl -LO https://registry.npmjs.org/tm-grammars/-/tm-grammars-1.32.22.tgz
//! tar xzf tm-grammars-1.32.22.tgz
//! cargo run --example grammars -- package
//! ```
//!
//! The themes are kept in the repository, each with its source, version or
//! commit, and licence in [`THEME_SOURCES`].
//!
//! - `src/highlight/grammars.bin`: the grammars the binary carries, the same
//!   bytes on every run. A grammar is carried only under a licence that lets
//!   it ship inside a permissively licensed binary. Each is its JSON without
//!   the space between its parts or an empty `match`, deflated on its own,
//!   so a build reads only the grammars its code is in. Before them the file
//!   lists, for each, its scope, its deflated length, the grammars that add
//!   rules to it and the grammars it can't be read without; then every name
//!   and alias in order, with its grammar. A number is four bytes, the lowest
//!   first, and a text follows its length.
//! - `src/highlight/languages.rs`: every name and alias a fence can use.
//! - `src/highlight/styles.rs`: for each style, the colour its light theme
//!   and its dark give each kind of code, ordinary text and the background.
//!   The tests fail while it differs from what the themes give.
//! - `src/highlight/licences.md`: every grammar and theme carried, with its
//!   licence and source.
//! - `THIRD-PARTY-GRAMMARS.md`: the same list, each with the notice it asks
//!   for, which a release carries. A grammar's notice is the one the
//!   package's `NOTICE` file gives; where it gives none, and for a theme,
//!   the one `src/highlight/notices-read-at-source.txt` holds. The tests
//!   fail while a grammar or theme carried has no notice there.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use ferriki_textmate::{
    EncodedTokenAttributes, FontStyle, GrammarConfiguration, RawTheme, RawThemeScope,
    RawThemeSetting, RawThemeStyle, SyncRegistry, parse_raw_grammar,
};
use statac::highlight::KINDS;
use statac::source::Sources;
use statac::yaml::{Node, Resolved, Value};

/// A style `code_style` names: its light theme and its dark twin, by their
/// files in `src/highlight/themes/`.
pub struct Pair {
    pub name: &'static str,
    pub light: &'static str,
    pub dark: &'static str,
}

pub const STYLES: &[Pair] = &[
    Pair {
        name: "ocean",
        light: "base16-ocean-light",
        dark: "base16-ocean-dark",
    },
    Pair {
        name: "gruvbox",
        light: "gruvbox-light-medium",
        dark: "gruvbox-dark-medium",
    },
    Pair {
        name: "solarized",
        light: "solarized-light",
        dark: "solarized-dark",
    },
    Pair {
        name: "catppuccin",
        light: "catppuccin-latte",
        dark: "catppuccin-mocha",
    },
    Pair {
        name: "one-half",
        light: "one-half-light",
        dark: "one-half-dark",
    },
];

/// The version of `tm-grammars` the grammars are taken from. A new one is
/// a change to this line, so the documentation above stays true.
pub const PACKAGE: &str = "1.32.22";

/// Each theme file's licence and where it came from.
pub const THEME_SOURCES: &[(&str, &str, &str)] = &[
    (
        "base16-ocean-light",
        "MIT",
        "https://github.com/jryans/base16-vscode/blob/81ad380452b8828062d1be91501b495637eff2ac/themes/ocean-light.json",
    ),
    (
        "base16-ocean-dark",
        "MIT",
        "https://github.com/jryans/base16-vscode/blob/81ad380452b8828062d1be91501b495637eff2ac/themes/ocean-dark.json",
    ),
    (
        "gruvbox-light-medium",
        "MIT",
        "tm-themes 1.12.12, from https://github.com/jdinhify/vscode-theme-gruvbox/blob/f0d6e1fc1f325da636b90591eccc45d2f44738fd/src/main.ts",
    ),
    (
        "gruvbox-dark-medium",
        "MIT",
        "tm-themes 1.12.12, from https://github.com/jdinhify/vscode-theme-gruvbox/blob/f0d6e1fc1f325da636b90591eccc45d2f44738fd/src/main.ts",
    ),
    (
        "solarized-light",
        "MIT",
        "tm-themes 1.12.12, from https://github.com/microsoft/vscode/blob/0132168df99ab8830a1eb158b1a5e8242f3e94ff/extensions/theme-solarized-light/themes/solarized-light-color-theme.json",
    ),
    (
        "solarized-dark",
        "MIT",
        "tm-themes 1.12.12, from https://github.com/microsoft/vscode/blob/e886f3e07de31233c89ecdba812b05bfd07616d7/extensions/theme-solarized-dark/themes/solarized-dark-color-theme.json",
    ),
    (
        "catppuccin-latte",
        "MIT",
        "tm-themes 1.12.12, from https://github.com/catppuccin/vscode/blob/befc9e6fc41980f4241408f7049755d47c06ff45/packages/catppuccin-vsc/package.json",
    ),
    (
        "catppuccin-mocha",
        "MIT",
        "tm-themes 1.12.12, from https://github.com/catppuccin/vscode/blob/befc9e6fc41980f4241408f7049755d47c06ff45/packages/catppuccin-vsc/package.json",
    ),
    (
        "one-half-light",
        "MIT",
        "https://github.com/sonph/onehalf/blob/75eb2e97acd74660779fed8380989ee7891eec56/vscode/OneHalfLight.tmTheme, written as JSON",
    ),
    (
        "one-half-dark",
        "MIT",
        "https://github.com/sonph/onehalf/blob/75eb2e97acd74660779fed8380989ee7891eec56/vscode/OneHalfDark.tmTheme, written as JSON",
    ),
];

/// Licences that let a grammar ship inside a permissively licensed binary.
const PERMISSIVE: &[&str] = &["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC"];

/// The TextMate bundles' own licence, in each bundle's README.
const TEXTMATE: &str = "TextMate bundle licence: \"Permission to copy, use, modify, sell and \
                        distribute this software is granted.\"";

/// Grammars the package records no licence for, whose licence was read at
/// the source named. A grammar the package now takes from anywhere else is
/// left out until it is read again.
const READ_AT_SOURCE: &[(&str, &str, &str)] = &[
    (
        "applescript",
        TEXTMATE,
        "https://github.com/textmate/applescript.tmbundle/blob/4947cad7494eba084c21cfe04689b4efce96700f/Syntaxes/AppleScript.tmLanguage",
    ),
    (
        "erb",
        TEXTMATE,
        "https://github.com/textmate/ruby.tmbundle/blob/ecf0f1df23984bcff9b51552e35d9696cb0d9539/Syntaxes/HTML%20(Ruby%20-%20ERB).tmLanguage",
    ),
    (
        "logo",
        TEXTMATE,
        "https://github.com/textmate/logo.tmbundle/blob/3205e987ed2a2aa11accdd55a7d094832441f89c/Syntaxes/Logo.tmLanguage",
    ),
    (
        "matlab",
        "BSD-2-Clause",
        "https://github.com/mathworks/MATLAB-Language-grammar/blob/69dbf20c1eb19efd629c0438a2a493e60617e8de/Matlab.tmbundle/Syntaxes/MATLAB.tmLanguage",
    ),
    (
        "mipsasm",
        TEXTMATE,
        "https://github.com/textmate/mips.tmbundle/blob/eb7601016a109bf912a48a1c21988f3c4a59fb3a/Syntaxes/MIPS.tmLanguage",
    ),
    (
        "po",
        TEXTMATE,
        "https://github.com/textmate/gettext.tmbundle/blob/08bb69558d82ada8d4f36b8869f871ce69014749/Syntaxes/Gettext.tmLanguage",
    ),
    (
        "ssh-config",
        TEXTMATE,
        "https://github.com/textmate/ssh-config.tmbundle/blob/212a0000c50e0c877303b2c4fcead539d2af2670/Syntaxes/SSH-Config.tmLanguage",
    ),
    (
        "toml",
        TEXTMATE,
        "https://github.com/textmate/toml.tmbundle/blob/e82b64c1e86396220786846201e9aa3f0a2d9ca2/Syntaxes/TOML.tmLanguage",
    ),
    (
        "yaml",
        "MIT",
        "https://github.com/textmate/yaml.tmbundle/blob/e54ceae3b719506dba7e481a77cea4a8b576ae46/Syntaxes/YAML.tmLanguage",
    ),
];

fn highlight_folder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/highlight")
}

/// Where the notices are written.
pub fn notices_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("THIRD-PARTY-GRAMMARS.md")
}

fn theme_path(stem: &str) -> PathBuf {
    highlight_folder()
        .join("themes")
        .join(format!("{stem}.json"))
}

fn main() {
    let package = PathBuf::from(
        std::env::args()
            .nth(1)
            .expect("give the folder of a tm-grammars package"),
    );
    let version = package_version(&package);
    assert_eq!(
        version, PACKAGE,
        "the folder holds tm-grammars {version}; to build from it, change PACKAGE"
    );
    let index = std::fs::read_to_string(package.join("index.js")).expect("index.js can be read");
    let grammars = read_index(&index);

    let mut carried = Vec::new();
    let mut left_out = Vec::new();
    for grammar in grammars {
        match licence(&grammar) {
            Some((licence, source)) => carried.push((grammar, licence, source)),
            None => left_out.push(format!(
                "{} ({})",
                grammar.name,
                grammar.license.as_deref().unwrap_or("no licence recorded")
            )),
        }
    }

    let mut languages = Vec::new();
    for (grammar, _, _) in &carried {
        languages.push(grammar.name.to_lowercase());
        languages.extend(grammar.aliases.iter().map(|alias| alias.to_lowercase()));
    }
    let grammars: Vec<&Grammar> = carried.iter().map(|(grammar, _, _)| grammar).collect();
    let held = grammars_bin(&package, &grammars);
    std::fs::write(highlight_folder().join("grammars.bin"), &held).expect("grammars.bin");

    languages.sort();
    languages.dedup();
    std::fs::write(
        highlight_folder().join("languages.rs"),
        languages_rs(&languages),
    )
    .expect("languages.rs");
    std::fs::write(highlight_folder().join("styles.rs"), styles_rs()).expect("styles.rs");
    std::fs::write(
        highlight_folder().join("licences.md"),
        licences_md(&version, &carried),
    )
    .expect("licences.md");
    let mut notices = Vec::new();
    for path in [
        package.join("NOTICE"),
        highlight_folder().join("notices-read-at-source.txt"),
    ] {
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        notices.extend(read_notices(&text));
    }
    std::fs::write(notices_path(), notices_md(&version, &carried, &notices))
        .expect("THIRD-PARTY-GRAMMARS.md");

    println!(
        "Carried {} grammars ({} names) in {} bytes, and {} themes.",
        carried.len(),
        languages.len(),
        held.len(),
        STYLES.len() * 2
    );
    println!("Left out: {}.", left_out.join(", "));
}

/// The licence a grammar is carried under and where it came from, or none
/// for one that can't be carried.
fn licence(grammar: &Grammar) -> Option<(String, String)> {
    if let Some(license) = &grammar.license
        && PERMISSIVE.contains(&license.as_str())
    {
        return Some((license.clone(), grammar.source.clone()));
    }
    READ_AT_SOURCE
        .iter()
        .find(|(name, _, source)| *name == grammar.name && *source == grammar.source)
        .map(|(_, licence, source)| ((*licence).to_owned(), (*source).to_owned()))
}

/// One grammar as the package's `index.js` lists it.
#[derive(Default)]
struct Grammar {
    name: String,
    display_name: String,
    license: Option<String>,
    source: String,
    aliases: Vec<String>,
    scope: String,
    /// The scopes of the grammars it adds rules of its own to.
    inject_to: Vec<String>,
}

/// `index.js` is generated, one key to a line: `    name: 'value',`, and
/// a list as `    aliases: [`, one `      'item',` to a line, `    ],`.
fn read_index(index: &str) -> Vec<Grammar> {
    let mut grammars = Vec::new();
    let mut current: Option<Grammar> = None;
    let mut list: Option<String> = None;
    for line in index.lines() {
        match line {
            "  {" => current = Some(Grammar::default()),
            "  }," => grammars.extend(current.take()),
            "    ]," => list = None,
            _ => {
                let Some(grammar) = current.as_mut() else {
                    continue;
                };
                if let Some(key) = &list {
                    if let Some(item) = quoted(line.trim().trim_end_matches(',')) {
                        match key.as_str() {
                            "aliases" => grammar.aliases.push(item),
                            "injectTo" => grammar.inject_to.push(item),
                            _ => {}
                        }
                    }
                    continue;
                }
                let Some((key, value)) = line.trim().split_once(": ") else {
                    continue;
                };
                if value == "[" {
                    list = Some(key.to_owned());
                    continue;
                }
                let Some(value) = quoted(value.trim_end_matches(',')) else {
                    continue;
                };
                match key {
                    "name" => grammar.name = value,
                    "displayName" => grammar.display_name = value,
                    "license" => grammar.license = Some(value),
                    "source" => grammar.source = value,
                    "scopeName" => grammar.scope = value,
                    _ => {}
                }
            }
        }
    }
    grammars
}

fn quoted(text: &str) -> Option<String> {
    let inside = text.strip_prefix('\'')?.strip_suffix('\'')?;
    Some(inside.replace("\\'", "'").replace("\\\\", "\\"))
}

fn package_version(package: &Path) -> String {
    let json = std::fs::read_to_string(package.join("package.json")).expect("package.json");
    let after = json
        .split_once("\"version\": \"")
        .expect("package.json has a version")
        .1;
    after.split('"').next().unwrap_or_default().to_owned()
}

fn languages_rs(languages: &[String]) -> String {
    let mut out = String::from(
        "// Written by `cargo run --example grammars`. Every name and alias of a\n\
         // grammar Statac carries, in order.\n\npub(super) const LANGUAGES: &[&str] = &[\n",
    );
    for language in languages {
        writeln!(out, "    {language:?},").unwrap();
    }
    out.push_str("];\n");
    out
}

/// One thing carried: the file a notice names it by, its name, its licence
/// and its line in a list.
struct Entry {
    file: String,
    name: String,
    licence: String,
    line: String,
}

/// The grammars carried, then the themes.
fn entries(carried: &[(Grammar, String, String)]) -> [Vec<Entry>; 2] {
    let grammars = carried.iter().map(|(grammar, licence, source)| Entry {
        file: format!("{}.json", grammar.name),
        name: grammar.name.clone(),
        licence: licence.clone(),
        line: format!(
            "- {} (`{}`): {licence}. {source}",
            grammar.display_name, grammar.name
        ),
    });
    let themes = THEME_SOURCES.iter().map(|(stem, licence, source)| Entry {
        file: format!("themes/{stem}.json"),
        name: (*stem).to_owned(),
        licence: (*licence).to_owned(),
        line: format!("- `{stem}`: {licence}. {source}"),
    });
    [grammars.collect(), themes.collect()]
}

fn licences_md(version: &str, carried: &[(Grammar, String, String)]) -> String {
    let mut out = format!(
        "# Grammars and themes Statac carries\n\n\
         Written by `cargo run --example grammars` from npm's tm-grammars {version},\n\
         https://registry.npmjs.org/tm-grammars/-/tm-grammars-{version}.tgz, and\n\
         the themes in `src/highlight/themes/`.\n"
    );
    for (heading, entries) in LISTS.iter().zip(entries(carried)) {
        writeln!(out, "\n{heading}\n").unwrap();
        for entry in entries {
            writeln!(out, "{}", entry.line).unwrap();
        }
    }
    out
}

/// The headings of the two lists `licences.md` and the notices both have.
pub const LISTS: [&str; 2] = ["## Grammars", "## Themes"];

/// The length of the line of equals signs that opens each notice of the
/// package's `NOTICE` file, and of the line of hyphens that ends a
/// notice's heading.
const RULE: usize = 105;

/// A notice: the files it is for, the address its text was read at, the
/// licence it is recorded as, and the text.
struct Notice {
    files: Vec<String>,
    from: String,
    spdx: Option<String>,
    text: String,
}

/// The notices of a file in the form of the package's `NOTICE`: after a
/// line of equals signs, lines that open `Files:`, `License:` and `SPDX:`,
/// a line of hyphens, and the text.
fn read_notices(file: &str) -> Vec<Notice> {
    let opens = format!("{}\n", "=".repeat(RULE));
    let ends = format!("{}\n", "-".repeat(RULE));
    let notices = file.split(&opens).skip(1).map(|notice| {
        let (heading, text) = notice.split_once(&ends).expect("a notice has a heading");
        let said = |key: &str| {
            let value = heading.lines().find_map(|line| line.strip_prefix(key))?;
            Some(value.trim().to_owned())
        };
        let files = said("Files:").expect("a notice names its files");
        Notice {
            files: files.split(", ").map(str::to_owned).collect(),
            from: said("License:").expect("a notice names its address"),
            spdx: said("SPDX:"),
            text: text.to_owned(),
        }
    });
    notices.collect()
}

/// A text of the notices, with the names of what it is for and the
/// addresses it was read at.
struct Text {
    text: String,
    of: Vec<String>,
    from: Vec<String>,
}

fn notices_md(version: &str, carried: &[(Grammar, String, String)], notices: &[Notice]) -> String {
    let mut out = format!(
        "# The notices of the grammars and themes Statac carries\n\n\
         Written by `cargo run --example grammars` from the `NOTICE` file of npm's\n\
         tm-grammars {version},\n\
         https://registry.npmjs.org/tm-grammars/-/tm-grammars-{version}.tgz, and\n\
         `src/highlight/notices-read-at-source.txt`.\n\n\
         Every grammar and theme a built `statac` colours code with, each with its\n\
         licence, its source and the notice it asks for. A number in square brackets\n\
         is a text under [Notices](#notices), where each text is given once, as the\n\
         address above it gave it.\n"
    );
    let mut texts: Vec<Text> = Vec::new();
    let mut without = Vec::new();
    for (heading, entries) in LISTS.iter().zip(entries(carried)) {
        writeln!(out, "\n{heading}\n").unwrap();
        for entry in entries {
            let Some(notice) = notices
                .iter()
                .find(|notice| notice.files.contains(&entry.file))
            else {
                without.push(entry.file);
                continue;
            };
            if let Some(spdx) = &notice.spdx {
                assert_eq!(
                    *spdx, entry.licence,
                    "the notice of {} is of another licence",
                    entry.file
                );
            }
            // Texts that differ only in space at a line's end are one.
            let lines: Vec<&str> = notice.text.lines().map(str::trim_end).collect();
            let text = lines.join("\n").trim_matches('\n').to_owned();
            let at = match texts.iter().position(|held| held.text == text) {
                Some(at) => at,
                None => {
                    texts.push(Text {
                        text,
                        of: Vec::new(),
                        from: Vec::new(),
                    });
                    texts.len() - 1
                }
            };
            texts[at].of.push(format!("`{}`", entry.name));
            if !texts[at].from.contains(&notice.from) {
                texts[at].from.push(notice.from.clone());
            }
            writeln!(out, "{} [{}]", entry.line, at + 1).unwrap();
        }
    }
    assert!(
        without.is_empty(),
        "no notice for {}; give each one in src/highlight/notices-read-at-source.txt",
        without.join(", ")
    );
    out.push_str("\n## Notices\n");
    for (at, text) in texts.iter().enumerate() {
        assert!(
            !text.text.lines().any(|line| line.starts_with("````")),
            "a text of {:?} would close its own block",
            text.of
        );
        writeln!(
            out,
            "\n### [{}]\n\n{}. From {}.\n\n````text\n{}\n````",
            at + 1,
            text.of.join(", "),
            text.from.join(" and "),
            text.text
        )
        .unwrap();
    }
    out
}

/// The lines of the list under a heading of `licences.md` or the notices.
pub fn listed<'a>(written: &'a str, heading: &str) -> Vec<&'a str> {
    let lines = written.lines().skip_while(|line| *line != heading).skip(1);
    lines
        .take_while(|line| !line.starts_with("## "))
        .filter(|line| line.starts_with("- "))
        .collect()
}

/// The numbers of the texts the notices give, each of a text that is not
/// empty.
pub fn texts(written: &str) -> BTreeSet<usize> {
    let mut numbers = BTreeSet::new();
    let mut number: Option<usize> = None;
    let mut inside = false;
    for line in written.lines() {
        match (inside, line) {
            (false, "````text") => inside = true,
            (true, "````") => inside = false,
            (true, _) if !line.trim().is_empty() => numbers.extend(number),
            (true, _) => {}
            (false, _) => {
                if let Some(said) = line.strip_prefix("### [") {
                    number = said.strip_suffix(']').and_then(|said| said.parse().ok());
                }
            }
        }
    }
    numbers
}

/// A JSON string's text.
fn text_of(node: &Node) -> Option<String> {
    match &node.value {
        Value::Scalar(text) if node.resolved() == Some(Resolved::Str) => Some(text.clone()),
        _ => None,
    }
}

fn read_json(path: &Path) -> Node {
    let text =
        std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let mut sources = Sources::default();
    let source = sources.add(path, text);
    statac::yaml::read_json(&sources, source)
        .unwrap_or_else(|_| panic!("{} is not JSON", path.display()))
}

/// JSON without the space between its parts.
fn minified(json: &str) -> String {
    let mut out = String::with_capacity(json.len());
    let mut inside = false;
    let mut escaped = false;
    for c in json.chars() {
        if inside {
            out.push(c);
            match c {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => inside = false,
                _ => {}
            }
        } else if !c.is_ascii_whitespace() {
            out.push(c);
            inside = c == '"';
        }
    }
    out
}

/// Minified JSON with each empty `match` taken out. vscode-textmate, which
/// the grammars are written for, reads one as no `match` at all;
/// ferriki-textmate reads it as matching everywhere, and leaves the rest of
/// the line uncoloured
/// (<https://github.com/sebastian-software/ferriki/issues/271>).
fn without_empty_matches(json: &str) -> String {
    const EMPTY: &str = "\"match\":\"\"";
    json.replace(&format!(",{EMPTY}"), "")
        .replace(&format!("{EMPTY},"), "")
        .replace(EMPTY, "")
}

/// The scopes of the other grammars a grammar includes rules of.
fn included(node: &Node, scopes: &mut BTreeSet<String>) {
    match &node.value {
        Value::Scalar(_) => {}
        Value::Sequence(items) => items.iter().for_each(|item| included(item, scopes)),
        Value::Mapping(entries) => {
            for entry in entries {
                included(&entry.value, scopes);
            }
            let scope = node.get("include").and_then(text_of);
            let scope = scope.and_then(|include| include.split('#').next().map(str::to_owned));
            scopes.extend(scope.filter(|scope| !scope.is_empty() && !scope.starts_with('$')));
        }
    }
}

/// `grammars.bin`, in the form the documentation above gives.
fn grammars_bin(package: &Path, grammars: &[&Grammar]) -> Vec<u8> {
    let place = |scope: &str| grammars.iter().position(|grammar| grammar.scope == scope);
    let mut deflated = Vec::new();
    let mut includes: Vec<BTreeSet<usize>> = Vec::new();
    for grammar in grammars {
        let path = package
            .join("grammars")
            .join(format!("{}.json", grammar.name));
        let json = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let json = without_empty_matches(&minified(&json));
        let read = parse_raw_grammar(&json, Some("grammar.json"))
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert_eq!(read.scope_name, grammar.scope, "{}", path.display());
        deflated.push(miniz_oxide::deflate::compress_to_vec(json.as_bytes(), 10));
        let mut scopes = BTreeSet::new();
        included(&read_json(&path), &mut scopes);
        includes.push(scopes.iter().filter_map(|scope| place(scope)).collect());
    }
    let mut injectors = vec![BTreeSet::new(); grammars.len()];
    for (at, grammar) in grammars.iter().enumerate() {
        for into in grammar.inject_to.iter().filter_map(|scope| place(scope)) {
            injectors[into].insert(at);
        }
    }

    let mut out = Vec::new();
    let number = |out: &mut Vec<u8>, number: usize| {
        out.extend(u32::try_from(number).expect("four bytes").to_le_bytes());
    };
    let text = |out: &mut Vec<u8>, text: &str| {
        number(out, text.len());
        out.extend(text.as_bytes());
    };
    number(&mut out, grammars.len());
    for (at, grammar) in grammars.iter().enumerate() {
        let mut needs = BTreeSet::from([at]);
        let mut waiting = vec![at];
        while let Some(at) = waiting.pop() {
            for &other in includes[at].iter().chain(&injectors[at]) {
                if needs.insert(other) {
                    waiting.push(other);
                }
            }
        }
        needs.remove(&at);
        text(&mut out, &grammar.scope);
        number(&mut out, deflated[at].len());
        for places in [&injectors[at], &needs] {
            number(&mut out, places.len());
            places.iter().for_each(|&place| number(&mut out, place));
        }
    }
    let mut names = BTreeMap::new();
    for (at, grammar) in grammars.iter().enumerate() {
        for name in std::iter::once(&grammar.name).chain(&grammar.aliases) {
            let other = names.insert(name.to_lowercase(), at);
            assert!(
                other.is_none_or(|other| other == at),
                "two grammars named {name}"
            );
        }
    }
    number(&mut out, names.len());
    for (name, at) in names {
        text(&mut out, &name);
        number(&mut out, at);
    }
    out.extend(deflated.into_iter().flatten());
    out
}

/// The scopes each kind's colour is read at in a theme, the innermost last:
/// the kind's usual scope, or where themes differ over that, the scopes of a
/// common piece of code of the kind.
const READ_AT: [(&str, &str); KINDS.len()] = [
    ("comment", "comment"),
    ("keyword", "keyword"),
    ("string", "string"),
    ("number", "constant.numeric"),
    ("constant", "constant.language"),
    ("function", "entity.name.function"),
    ("type", "entity.name.type.class"),
    ("variable", "variable.other.readwrite"),
    ("property", "variable.other.property"),
    ("tag", "entity.name.tag"),
    ("attribute", "entity.other.attribute-name"),
    ("operator", "keyword.operator"),
    ("punctuation", "punctuation"),
    (
        "heading",
        "markup.heading.markdown heading.1.markdown entity.name.section.markdown",
    ),
    ("emphasis", "markup.italic"),
    ("strong", "markup.bold"),
    (
        "link",
        "meta.link.inline.markdown markup.underline.link.markdown",
    ),
    ("inserted", "markup.inserted.diff"),
    ("deleted", "markup.deleted.diff"),
];

/// A theme's rules for text: its editor's colours for ordinary text, then
/// each rule's colour and font style. A rule's own background is left out.
fn theme(stem: &str) -> RawTheme {
    let json = read_json(&theme_path(stem));
    let rule = |scope: Option<Vec<String>>, style: RawThemeStyle| {
        let mut rule = RawThemeSetting::default();
        rule.scope = scope.map(RawThemeScope::Array);
        rule.settings = Some(style);
        rule
    };
    let editor = |key: &str| {
        let colour = json.get("colors").and_then(|colours| colours.get(key));
        Some(colour.and_then(text_of).expect("an editor's colour"))
    };
    let mut ordinary = RawThemeStyle::default();
    ordinary.foreground = editor("editor.foreground");
    ordinary.background = editor("editor.background");
    let mut rules = vec![rule(None, ordinary)];
    let items = match json.get("tokenColors").map(|node| &node.value) {
        Some(Value::Sequence(items)) => items.as_slice(),
        _ => &[],
    };
    for item in items {
        let scope: Vec<String> = match item.get("scope").map(|node| &node.value) {
            Some(Value::Scalar(list)) => {
                list.split(',').map(|part| part.trim().to_owned()).collect()
            }
            Some(Value::Sequence(list)) => list.iter().filter_map(text_of).collect(),
            _ => continue,
        };
        let setting = |key: &str| {
            let settings = item.get("settings")?;
            settings.get(key).and_then(text_of)
        };
        let mut style = RawThemeStyle::default();
        style.foreground = setting("foreground");
        style.font_style = setting("fontStyle");
        rules.push(rule(Some(scope), style));
    }
    let mut theme = RawTheme::default();
    theme.settings = rules;
    theme
}

/// What a theme gives code: the colour of ordinary text, the background,
/// and each kind's colour and font style.
struct Given {
    text: String,
    background: String,
    kinds: Vec<(String, String)>,
}

/// Reads a theme by colouring, under it, a grammar of one rule to a kind,
/// each giving one letter the scopes the kind is read at.
fn given(stem: &str) -> Given {
    let letters = || ('a'..).take(KINDS.len());
    let mut grammar = String::from("{\"scopeName\":\"source.kinds\",\"patterns\":[");
    for (letter, (kind, scope)) in letters().zip(READ_AT) {
        assert_eq!(kind, KINDS[letter as usize - 'a' as usize]);
        write!(grammar, "{{\"match\":\"{letter}\",\"name\":\"{scope}\"}},").unwrap();
    }
    grammar.pop();
    grammar.push_str("]}");
    let mut registry = SyncRegistry::new(Some(theme(stem)), None).expect(stem);
    let grammar = parse_raw_grammar(&grammar, Some("kinds.json")).expect("a grammar");
    registry.add_grammar(grammar, Vec::new());
    let grammar = registry
        .grammar_for_scope_name("source.kinds", GrammarConfiguration::default())
        .expect(stem)
        .expect("the grammar added");
    let colours = registry.get_color_map();
    let look = |text: &str| {
        let read = grammar.tokenize_line2(text, None, 0).expect(stem);
        let token = EncodedTokenAttributes::new(read.tokens[1]);
        let colour = |number: u32| {
            let colour = colours[number as usize].to_lowercase();
            assert!(matches!(colour.len(), 7 | 9), "{stem}: {colour}");
            colour
        };
        let mut font = Vec::new();
        for (bit, word) in [
            (FontStyle::BOLD, "bold"),
            (FontStyle::ITALIC, "italic"),
            (FontStyle::UNDERLINE, "underline"),
            (FontStyle::STRIKETHROUGH, "strikethrough"),
        ] {
            if token.font_style().contains(bit) {
                font.push(word);
            }
        }
        (
            colour(token.foreground()),
            colour(token.background()),
            font.join(" "),
        )
    };
    let (text, background, _) = look(".");
    let kinds = letters().map(|letter| {
        let (colour, _, font) = look(&letter.to_string());
        (colour, font)
    });
    Given {
        kinds: kinds.collect(),
        text,
        background,
    }
}

/// `styles.rs`, from the themes alone.
pub fn styles_rs() -> String {
    let mut out = String::from(
        "// Written by `cargo run --example grammars`. For each style, the colour of\n\
         // ordinary text and of the background, then each kind's colour and the\n\
         // font style its light theme gives it. Each colour is the light theme's,\n\
         // then the dark's.\n\n\
         use super::Style;\n\n#[rustfmt::skip]\npub(super) const STYLES: &[Style] = &[\n",
    );
    for style in STYLES {
        let (light, dark) = (given(style.light), given(style.dark));
        writeln!(out, "    Style {{").unwrap();
        writeln!(out, "        name: {:?},", style.name).unwrap();
        writeln!(out, "        text: [{:?}, {:?}],", light.text, dark.text).unwrap();
        writeln!(
            out,
            "        background: [{:?}, {:?}],",
            light.background, dark.background
        )
        .unwrap();
        writeln!(out, "        kinds: [").unwrap();
        for (kind, (light, dark)) in KINDS.iter().zip(light.kinds.iter().zip(&dark.kinds)) {
            writeln!(
                out,
                "            ([{:?}, {:?}], {:?}), // {kind}",
                light.0, dark.0, light.1
            )
            .unwrap();
        }
        writeln!(out, "        ],").unwrap();
        writeln!(out, "    }},").unwrap();
    }
    out.push_str("];\n");
    out
}
