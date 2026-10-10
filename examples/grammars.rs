//! Writes what Statac colours code with, from the grammars of npm's
//! `tm-grammars` package and the themes in `src/highlight/themes/`.
//!
//! The registry checked in was built from `tm-grammars` 1.32.22, the
//! version [`PACKAGE`] names. To build it again, fetch and unpack that
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
//! - `src/highlight/registry.zst`: the grammars and themes the binary
//!   carries. A grammar is carried only under a licence that lets it ship
//!   inside a permissively licensed binary. Its bytes differ from one run
//!   to the next, as giallo writes its themes from hash maps, in no fixed
//!   order; what it holds does not.
//! - `src/highlight/languages.rs`: every name and alias a fence can use.
//! - `src/highlight/looks.rs`: for each style, every pair of looks, light
//!   and dark, a piece of code can be given. The tests fail while it
//!   differs from what the themes give.
//! - `src/highlight/licences.md`: every grammar and theme carried, with its
//!   licence and source.
//! - `THIRD-PARTY-GRAMMARS.md`: the same list, each with the notice it asks
//!   for, which a release carries. A grammar's notice is the one the
//!   package's `NOTICE` file gives; where it gives none, and for a theme,
//!   the one `src/highlight/notices-read-at-source.txt` holds. The tests
//!   fail while a grammar or theme carried has no notice there.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use giallo::{
    Color, FontStyle, HighlightOptions, PLAIN_GRAMMAR_NAME, Registry, Style, ThemeVariant,
};
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

/// The version of `tm-grammars` the registry is built from. A new one is
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

    let mut registry = Registry::default();
    let mut languages = Vec::new();
    for (grammar, _, _) in &carried {
        let path = package
            .join("grammars")
            .join(format!("{}.json", grammar.name));
        registry
            .add_grammar_from_path(&path)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert!(registry.contains_grammar(&grammar.name), "{}", grammar.name);
        languages.push(grammar.name.to_lowercase());
        for alias in &grammar.aliases {
            registry.add_alias(&grammar.name, alias);
            languages.push(alias.to_lowercase());
        }
    }
    for stem in STYLES.iter().flat_map(|style| [style.light, style.dark]) {
        registry
            .add_theme_from_path(theme_path(stem))
            .unwrap_or_else(|error| panic!("{stem}: {error}"));
    }
    let dump = registry.dump().expect("the registry can be written");
    std::fs::write(highlight_folder().join("registry.zst"), &dump).expect("registry.zst");

    languages.sort();
    languages.dedup();
    std::fs::write(
        highlight_folder().join("languages.rs"),
        languages_rs(&languages),
    )
    .expect("languages.rs");
    std::fs::write(highlight_folder().join("looks.rs"), looks_rs()).expect("looks.rs");
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
        "Carried {} grammars ({} names) and {} themes in {} bytes.",
        carried.len(),
        languages.len(),
        STYLES.len() * 2,
        dump.len()
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
                    if key == "aliases"
                        && let Some(item) = quoted(line.trim().trim_end_matches(','))
                    {
                        grammar.aliases.push(item);
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

/// `looks.rs`, from the themes alone.
pub fn looks_rs() -> String {
    let mut out = String::from(
        "// Written by `cargo run --example grammars`. For each style, its two\n\
         // themes' ordinary look, then every other pair of looks, light and dark,\n\
         // a piece of code can be given.\n\n\
         use super::{Look, Looks};\n\n#[rustfmt::skip]\npub(super) const LOOKS: &[Looks] = &[\n",
    );
    for style in STYLES {
        let light = Theme::read(style.light);
        let dark = Theme::read(style.dark);
        let ordinary = [light.ordinary, dark.ordinary];
        writeln!(out, "    Looks {{").unwrap();
        writeln!(out, "        name: {:?},", style.name).unwrap();
        writeln!(out, "        light: {:?},", light.name).unwrap();
        writeln!(out, "        dark: {:?},", dark.name).unwrap();
        writeln!(out, "        ordinary: {},", pair(&ordinary)).unwrap();
        writeln!(out, "        pairs: &[").unwrap();
        for looks in reachable(&light, &dark) {
            if looks != ordinary {
                writeln!(out, "            {},", pair(&looks)).unwrap();
            }
        }
        writeln!(out, "        ],").unwrap();
        writeln!(out, "    }},").unwrap();
    }
    out.push_str("];\n");
    out
}

fn pair(looks: &[Style; 2]) -> String {
    format!("[{}, {}]", look(&looks[0]), look(&looks[1]))
}

fn look(style: &Style) -> String {
    let mut font = Vec::new();
    for (bit, word) in [
        (FontStyle::BOLD, "bold"),
        (FontStyle::ITALIC, "italic"),
        (FontStyle::UNDERLINE, "underline"),
        (FontStyle::STRIKETHROUGH, "strikethrough"),
    ] {
        if style.font_style.contains(bit) {
            font.push(word);
        }
    }
    format!(
        "Look({:?}, {:?}, {:?})",
        style.foreground.as_hex().to_lowercase(),
        style.background.as_hex().to_lowercase(),
        font.join(" ")
    )
}

/// A theme read as giallo reads it: its ordinary look and its rules, least
/// specific first.
struct Theme {
    name: String,
    ordinary: Style,
    rules: Vec<Rule>,
}

struct Rule {
    /// The scope it colours, cut to the atoms giallo keeps.
    target: String,
    /// What must stand above it, outermost last, as written.
    parents: Vec<String>,
    foreground: Option<Color>,
    background: Option<Color>,
    font: Option<FontStyle>,
}

/// What a run of rules sets, where it sets anything.
#[derive(Copy, Clone, Default, PartialEq, Eq, Hash)]
struct Settings {
    foreground: Option<Color>,
    background: Option<Color>,
    font: Option<FontStyle>,
}

/// A JSON string's text.
fn text_of(node: &Node) -> Option<String> {
    match &node.value {
        Value::Scalar(text) if node.resolved() == Some(Resolved::Str) => Some(text.clone()),
        _ => None,
    }
}

/// giallo keeps the first eight atoms of a scope.
const ATOMS: usize = 8;

impl Theme {
    fn read(stem: &str) -> Theme {
        let path = theme_path(stem);
        let text = std::fs::read_to_string(&path).expect("a theme file");
        let mut sources = Sources::default();
        let source = sources.add(&path, text);
        let json = statac::yaml::read_json(&sources, source).expect("a theme in JSON");
        let mut rules = Vec::new();
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
                _ => Vec::new(),
            };
            if scope.is_empty() {
                continue;
            }
            let setting = |key: &str| {
                item.get("settings")
                    .and_then(|settings| settings.get(key))
                    .and_then(text_of)
                    .filter(|value| value != "inherit")
            };
            let colour = |key| setting(key).map(|hex| Color::from_hex(&hex).expect(stem));
            let foreground = colour("foreground");
            let background = colour("background");
            let font = setting("fontStyle").map(|font| FontStyle::from_theme_str(&font));
            for selector in &scope {
                let parts: Vec<&str> = selector.split_whitespace().collect();
                let Some((&last, rest)) = parts.split_last() else {
                    continue;
                };
                if last == ">" {
                    continue;
                }
                rules.push(Rule {
                    target: last.split('.').take(ATOMS).collect::<Vec<_>>().join("."),
                    parents: rest.iter().rev().map(|part| (*part).to_owned()).collect(),
                    foreground,
                    background,
                    font,
                });
            }
        }
        // giallo's order: by the atoms in the scope, then by the parents
        // named, keeping the file's order between equals.
        rules.sort_by_key(|rule| {
            let atoms = if rule.target.is_empty() {
                0
            } else {
                rule.target.matches('.').count() + 1
            };
            let parents = rule.parents.iter().filter(|part| *part != ">").count();
            (atoms, parents)
        });
        // The ordinary look as giallo gives it, which leaves no reading of
        // the theme's colours to repeat here.
        let mut registry = Registry::default();
        registry.add_plain_grammar(&[]).expect("a plain grammar");
        registry
            .add_theme_from_path(&path)
            .expect("a theme giallo reads");
        registry.link_grammars();
        let name = json.get("name").and_then(text_of).expect("a theme's name");
        let options = HighlightOptions::new(PLAIN_GRAMMAR_NAME, ThemeVariant::Single(&name));
        let ordinary = match registry.highlight("", &options).expect("highlighted").theme {
            ThemeVariant::Single(theme) => theme.default_style,
            ThemeVariant::Dual { .. } => unreachable!("one theme asked for"),
        };
        Theme {
            name,
            ordinary,
            rules,
        }
    }
}

/// Whether `target` names `scope` or a scope it lies within, atom by atom.
/// None for `scope` is one no rule names.
fn names(target: &str, scope: Option<&str>) -> bool {
    target.is_empty()
        || scope.is_some_and(|scope| {
            scope == target
                || scope
                    .strip_prefix(target)
                    .is_some_and(|rest| rest.starts_with('.'))
        })
}

/// Every pair of looks code can be given, whatever scopes stand within
/// which.
///
/// giallo builds a token's look scope by scope, outermost first, applying
/// at each the rules that name it. Which of a theme's rules name a scope
/// depends only on the longest rule target it lies within, so the targets
/// of both themes, and a scope no rule names, stand for every scope there
/// is. A rule that also needs other scopes above it may or may not apply,
/// and both are followed; the same need in the two themes is met or not in
/// both. From the two ordinary looks, every scope may then follow any
/// other.
fn reachable(light: &Theme, dark: &Theme) -> Vec<[Style; 2]> {
    let themes = [light, dark];
    let mut scopes: Vec<Option<&str>> = vec![None];
    let targets: HashSet<&str> = themes
        .iter()
        .flat_map(|theme| theme.rules.iter().map(|rule| rule.target.as_str()))
        .collect();
    let mut targets: Vec<&str> = targets.into_iter().collect();
    targets.sort_unstable();
    scopes.extend(targets.into_iter().map(Some));

    // What one scope can do to a pair of looks: each way its rules can
    // apply, as the settings they leave in each theme, which don't depend
    // on the looks they are applied to.
    let steps: Vec<HashSet<[Settings; 2]>> = scopes
        .iter()
        .map(|&scope| {
            let rules = themes.map(|theme| {
                theme
                    .rules
                    .iter()
                    .filter(|rule| names(&rule.target, scope))
                    .collect::<Vec<_>>()
            });
            let mut needs: Vec<&[String]> = rules
                .iter()
                .flatten()
                .filter(|rule| !rule.parents.is_empty())
                .map(|rule| rule.parents.as_slice())
                .collect();
            needs.sort_unstable();
            needs.dedup();
            assert!(needs.len() <= 20, "too many conditions on one scope");
            let mut ways = HashSet::new();
            for met in 0..1u32 << needs.len() {
                let applies = |rule: &Rule| {
                    rule.parents.is_empty()
                        || needs
                            .iter()
                            .position(|need| *need == rule.parents.as_slice())
                            .is_some_and(|at| met & (1 << at) != 0)
                };
                ways.insert([0, 1].map(|side| {
                    let mut settings = Settings::default();
                    for rule in rules[side].iter().filter(|rule| applies(rule)) {
                        settings = Settings {
                            foreground: rule.foreground.or(settings.foreground),
                            background: rule.background.or(settings.background),
                            font: rule.font.or(settings.font),
                        };
                    }
                    settings
                }));
            }
            ways
        })
        .collect();

    let start = [light.ordinary, dark.ordinary];
    let mut seen: HashSet<[Style; 2]> = HashSet::from([start]);
    let mut waiting = vec![start];
    while let Some(looks) = waiting.pop() {
        for ways in &steps {
            for way in ways {
                let next = [0, 1].map(|side| {
                    let style = looks[side];
                    Style {
                        foreground: way[side].foreground.unwrap_or(style.foreground),
                        background: way[side].background.unwrap_or(style.background),
                        font_style: way[side].font.unwrap_or(style.font_style),
                    }
                });
                if seen.insert(next) {
                    waiting.push(next);
                }
            }
        }
    }
    let mut pairs: BTreeMap<String, [Style; 2]> = BTreeMap::new();
    for looks in seen {
        pairs.insert(pair(&looks), looks);
    }
    pairs.into_values().collect()
}
