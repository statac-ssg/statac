//! Code coloured as the site is built, and `code.css`, the colours it is
//! shown in, written from the site's `code_style`.
//!
//! A style is a light theme and its dark twin, and the reader's own
//! preference chooses between them. Each piece of code is given its look in
//! both themes; each distinct look a stylesheet rule can show is one short
//! class, every one carrying `hl-` so a site's own stylesheet never meets
//! one by chance. Code in the ordinary look carries none. A theme's colour
//! too faint to read on its background is written darker or lighter.
//!
//! The grammars, themes and looks are written by `cargo run --example
//! grammars`, which says where they come from.

mod languages;
mod looks;

use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::{Arc, LazyLock, Mutex};

use giallo::{Color, FontStyle, HighlightOptions, Registry, Style, ThemeVariant};

/// The name a template asks for the stylesheet by.
pub const STYLESHEET: &str = "code.css";

/// The styles `code_style` takes.
pub const NAMES: &[&str] = &["ocean", "gruvbox", "solarized", "catppuccin", "one-half"];

const CLASS_PREFIX: &str = "hl-";

/// The longest line, in characters, a block may hold and be coloured: the
/// time colouring a line takes grows with the square of its length.
pub const LONGEST_LINE: usize = 5_000;

/// Words that ask for code to be shown as it is.
const PLAIN: &[&str] = &["text", "txt", "plain", "plaintext"];

/// Common names for a language that its grammar isn't known by.
const ALIASES: &[(&str, &str)] = &[
    ("cl", "common-lisp"),
    ("golang", "go"),
    ("udiff", "diff"),
    ("patch", "diff"),
    ("nasm", "asm"),
    ("octave", "matlab"),
    ("env", "dotenv"),
    ("python3", "python"),
    ("node", "javascript"),
    ("terminal", "shellsession"),
    ("shell-session", "shellsession"),
    ("sh-session", "shellsession"),
    ("containerfile", "docker"),
    ("rhtml", "erb"),
    ("jinja2", "jinja"),
    ("j2", "jinja"),
];

/// Files known by their whole names and by no language's, each as it is
/// usually written, with its language.
const FILES: &[(&str, &str)] = &[
    ("GNUmakefile", "makefile"),
    ("CMakeLists.txt", "cmake"),
    ("Gemfile", "ruby"),
    ("Rakefile", "ruby"),
    ("Cargo.lock", "toml"),
    (".bashrc", "bash"),
    (".zshrc", "zsh"),
];

/// Words for a language that are its files' whole names. On a fence each
/// is the language; a file on disk named so is in it.
const NAMED_AS_THEIR_FILES: &[&str] = &[
    "makefile",
    "dockerfile",
    "containerfile",
    "justfile",
    "codeowners",
];

/// Endings of a file's name that no language is named by.
const ENDINGS: &[(&str, &str)] = &[("h", "c"), ("hpp", "cpp"), ("cc", "cpp"), ("htm", "html")];

/// A look as `looks.rs` writes it: its text colour, its background and its
/// font style, as a theme writes them.
struct Look(&'static str, &'static str, &'static str);

/// A style's looks, as `looks.rs` writes them.
struct Looks {
    name: &'static str,
    /// The themes, by the names the registry holds them by.
    light: &'static str,
    dark: &'static str,
    /// Text in neither theme's colour.
    ordinary: [Look; 2],
    /// Every other pair of looks code can be given.
    pairs: &'static [[Look; 2]],
}

impl Look {
    fn style(&self) -> Style {
        let colour =
            |hex| Color::from_hex(hex).expect("looks.rs writes colours the highlighter reads");
        Style {
            foreground: colour(self.0),
            background: colour(self.1),
            font_style: FontStyle::from_theme_str(self.2),
        }
    }
}

/// Every grammar and theme, read once for the whole process.
static REGISTRY: LazyLock<Registry> = LazyLock::new(|| {
    Registry::load(include_bytes!("registry.zst")).expect("the registry Statac carries")
});

/// What a fence's first word names.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Language {
    /// Code shown as it is, as asked.
    Plain,
    /// A grammar, by a name the registry knows it by.
    Known(&'static str),
    /// A word no grammar is known by, with the nearest that is, where one
    /// is near.
    Unknown { nearest: Option<&'static str> },
}

/// The language a fence's first word names.
pub fn language(word: &str) -> Language {
    let word = word.to_lowercase();
    if PLAIN.contains(&word.as_str()) {
        return Language::Plain;
    }
    if let Some((_, name)) = ALIASES.iter().find(|(alias, _)| *alias == word) {
        return Language::Known(name);
    }
    if let Ok(at) = languages::LANGUAGES.binary_search(&word.as_str()) {
        return Language::Known(languages::LANGUAGES[at]);
    }
    // Short names lie a slip apart from other languages altogether (`glsl`
    // and `hlsl`, `obj` and `objc`), so only a longer word is offered one.
    if word.chars().count() < 5 {
        return Language::Unknown { nearest: None };
    }
    let known = PLAIN
        .iter()
        .chain(languages::LANGUAGES)
        .chain(ALIASES.iter().map(|(alias, _)| alias))
        .copied();
    Language::Unknown {
        nearest: crate::suggest::nearest(&word, known),
    }
}

/// What a fence's first word is.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum FirstWord {
    Language(Language),
    /// A file's name, with the language its name gives.
    File(Language),
}

impl FirstWord {
    pub fn language(self) -> Language {
        match self {
            FirstWord::Language(language) | FirstWord::File(language) => language,
        }
    }
}

/// What a fence's first word is: a language, as every word that names one
/// is, in any capitals; else a file whose name gives a language, by its
/// whole name as it is usually written, `Gemfile`, or by its ending,
/// `example.rs`; else a language Statac doesn't know.
pub fn first_word(word: &str) -> FirstWord {
    match language(word) {
        unknown @ Language::Unknown { .. } => {
            let named = match file_shaped(word) {
                true => by_whole_name(word, |a, b| a == b).or_else(|| by_ending(word)),
                false => None,
            };
            named.map_or(FirstWord::Language(unknown), FirstWord::File)
        }
        named => FirstWord::Language(named),
    }
}

/// The language the name of a file on disk gives: by its whole name, in
/// any capitals, or else by its ending. Plain where neither names one.
pub fn file_language(name: &str) -> Language {
    let name = last_part(name);
    let own = || {
        NAMED_AS_THEIR_FILES
            .iter()
            .any(|known| known.eq_ignore_ascii_case(name))
            .then(|| language(name))
    };
    by_whole_name(name, str::eq_ignore_ascii_case)
        .or_else(own)
        .or_else(|| by_ending(name))
        .unwrap_or(Language::Plain)
}

/// Whether `word` is made only of what a file's name or a path to it is:
/// letters, digits, `.`, `_`, `-` and `/`.
fn file_shaped(word: &str) -> bool {
    word.chars()
        .all(|c| c.is_alphanumeric() || matches!(c, '.' | '_' | '-' | '/'))
}

/// A file's name without the folders before it.
fn last_part(name: &str) -> &str {
    name.rsplit('/').next().unwrap_or(name)
}

fn by_whole_name(name: &str, same: impl Fn(&str, &str) -> bool) -> Option<Language> {
    FILES
        .iter()
        .find(|(known, _)| same(known, last_part(name)))
        .map(|(_, language)| Language::Known(language))
}

/// The language named by what follows the last dot of a file's name, in
/// any capitals; none for an ending that names none.
fn by_ending(name: &str) -> Option<Language> {
    let (_, ending) = last_part(name).rsplit_once('.')?;
    let ending = ending.to_lowercase();
    if let Some((_, name)) = ENDINGS.iter().find(|(known, _)| *known == ending) {
        return Some(Language::Known(name));
    }
    match language(&ending) {
        Language::Unknown { .. } => None,
        named => Some(named),
    }
}

/// How long the longest line of `code` is, in characters, where it is too
/// long for the block to be coloured.
pub fn too_long(code: &str) -> Option<usize> {
    // A line holds no more characters than bytes.
    if code.len() <= LONGEST_LINE {
        return None;
    }
    let longest = code.lines().map(|line| line.chars().count()).max()?;
    (longest > LONGEST_LINE).then_some(longest)
}

/// A style's classes: each pair of looks code can be given, by the class
/// standing for how a stylesheet shows it, or none where it is shown as
/// ordinary text.
struct Classes {
    looks: &'static Looks,
    ordinary: [Style; 2],
    of: HashMap<[Style; 2], Option<usize>>,
    /// Each class's declarations, in the order the classes are numbered.
    rules: Vec<String>,
}

impl Classes {
    fn new(looks: &'static Looks) -> Classes {
        let ordinary = [looks.ordinary[0].style(), looks.ordinary[1].style()];
        let mut of = HashMap::new();
        let mut rules: Vec<String> = Vec::new();
        for pair in looks.pairs {
            let pair = [pair[0].style(), pair[1].style()];
            let declarations = declarations(&pair, &ordinary);
            let class = (!declarations.is_empty()).then(|| {
                rules
                    .iter()
                    .position(|rule| *rule == declarations)
                    .unwrap_or_else(|| {
                        rules.push(declarations);
                        rules.len() - 1
                    })
            });
            of.insert(pair, class);
        }
        Classes {
            looks,
            ordinary,
            of,
            rules,
        }
    }

    fn class(&self, pair: [Style; 2]) -> Option<usize> {
        if pair == self.ordinary {
            return None;
        }
        let class = self.of.get(&pair);
        debug_assert!(class.is_some(), "a pair of looks looks.rs doesn't list");
        class.copied().flatten()
    }
}

static CLASSES: LazyLock<Vec<Classes>> =
    LazyLock::new(|| looks::LOOKS.iter().map(Classes::new).collect());

fn classes(code_style: &str) -> Option<&'static Classes> {
    CLASSES
        .iter()
        .find(|classes| classes.looks.name == code_style)
}

/// What a stylesheet says to show a pair of looks, beside the ordinary
/// look. Bold and italic are the same in both looks, the light theme's.
fn declarations(pair: &[Style; 2], ordinary: &[Style; 2]) -> String {
    let mut out = String::new();
    let [light, dark] = pair;
    let text = |style: &Style| legible(style.foreground, style.background);
    if text(light) != text(&ordinary[0]) || text(dark) != text(&ordinary[1]) {
        write!(out, " color: {};", both(text(light), text(dark))).unwrap();
    }
    if light.background != ordinary[0].background || dark.background != ordinary[1].background {
        write!(
            out,
            " background-color: {};",
            both(light.background, dark.background)
        )
        .unwrap();
    }
    let font = light.font_style;
    let usual = ordinary[0].font_style;
    let differs = |bit| font.contains(bit) != usual.contains(bit);
    if differs(FontStyle::BOLD) {
        let weight = if font.contains(FontStyle::BOLD) {
            "bold"
        } else {
            "normal"
        };
        write!(out, " font-weight: {weight};").unwrap();
    }
    if differs(FontStyle::ITALIC) {
        let style = if font.contains(FontStyle::ITALIC) {
            "italic"
        } else {
            "normal"
        };
        write!(out, " font-style: {style};").unwrap();
    }
    if differs(FontStyle::UNDERLINE) || differs(FontStyle::STRIKETHROUGH) {
        let lines: Vec<&str> = [
            (FontStyle::UNDERLINE, "underline"),
            (FontStyle::STRIKETHROUGH, "line-through"),
        ]
        .into_iter()
        .filter(|(bit, _)| font.contains(*bit))
        .map(|(_, word)| word)
        .collect();
        let lines = if lines.is_empty() {
            "none".to_owned()
        } else {
            lines.join(" ")
        };
        write!(out, " text-decoration: {lines};").unwrap();
    }
    out
}

/// A class's name: short, and the same for the same look in every build.
fn class_name(class: usize) -> String {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut digits = Vec::new();
    let mut rest = class;
    loop {
        digits.push(DIGITS[rest % 36]);
        rest /= 36;
        if rest == 0 {
            break;
        }
    }
    digits.reverse();
    format!(
        "{CLASS_PREFIX}{}",
        String::from_utf8(digits).expect("ASCII")
    )
}

/// Blocks of code coloured, by their language and their code.
type Done = HashMap<(&'static str, Box<str>), Arc<str>>;

/// Colours code in one site's style.
pub struct Highlighter {
    classes: &'static Classes,
    /// Each block once coloured, so a block written twice is coloured once.
    done: Mutex<Done>,
}

impl Highlighter {
    /// None for a name `code_style` doesn't take, which the setting has
    /// already refused.
    pub fn new(code_style: &str) -> Option<Highlighter> {
        Some(Highlighter {
            classes: classes(code_style)?,
            done: Mutex::new(HashMap::new()),
        })
    }

    /// The inside of a `<code>` holding `code`, coloured as `language`.
    pub fn html(&self, language: &'static str, code: &str) -> Arc<str> {
        let key = (language, Box::from(code));
        if let Some(done) = self.done.lock().expect("never poisoned").get(&key) {
            return Arc::clone(done);
        }
        let html: Arc<str> = self.coloured(language, code).into();
        let mut done = self.done.lock().expect("never poisoned");
        Arc::clone(done.entry(key).or_insert(html))
    }

    fn coloured(&self, language: &str, code: &str) -> String {
        let looks = self.classes.looks;
        let options = HighlightOptions::new(
            language,
            ThemeVariant::Dual {
                light: looks.light,
                dark: looks.dark,
            },
        )
        .merge_whitespace(false);
        let mut out = String::with_capacity(code.len() * 2);
        let Ok(highlighted) = REGISTRY.highlight(code, &options) else {
            escape(code, &mut out);
            return out;
        };
        for (number, line) in highlighted.tokens.iter().enumerate() {
            if number > 0 {
                out.push('\n');
            }
            let mut open: Option<usize> = None;
            for token in line {
                let class = match token.style {
                    ThemeVariant::Dual { light, dark } => self.classes.class([light, dark]),
                    ThemeVariant::Single(_) => None,
                };
                if class != open {
                    if open.is_some() {
                        out.push_str("</span>");
                    }
                    if let Some(class) = class {
                        out.push_str("<span class=\"");
                        out.push_str(&class_name(class));
                        out.push_str("\">");
                    }
                    open = class;
                }
                escape(&token.text, &mut out);
            }
            if open.is_some() {
                out.push_str("</span>");
            }
        }
        out
    }
}

/// The inside of a `<code>` holding `code` as it is, without colour.
pub fn plain(code: &str) -> String {
    let mut out = String::with_capacity(code.len());
    escape(code, &mut out);
    out
}

/// The four characters that would otherwise be read as HTML.
fn escape(text: &str, out: &mut String) {
    let mut rest = text;
    while let Some(at) = rest.find(['&', '<', '>', '"']) {
        out.push_str(&rest[..at]);
        out.push_str(match rest.as_bytes()[at] {
            b'&' => "&amp;",
            b'<' => "&lt;",
            b'>' => "&gt;",
            _ => "&quot;",
        });
        rest = &rest[at + 1..];
    }
    out.push_str(rest);
}

/// The stylesheet for the style `code_style` names; none for a name that
/// isn't one, which the setting has already refused.
pub fn stylesheet(code_style: &str) -> Option<String> {
    let classes = classes(code_style)?;
    let [light, dark] = classes.ordinary;
    let mut out = format!(
        "/* Code colours: {}, light and dark. Statac writes this file; to change it,\n   \
         set `code_style` in {}. */\n\n",
        classes.looks.name,
        crate::config::FILE_NAME
    );
    // A custom property is inherited, so the nearest `data-theme` above the
    // code decides, however many stand above it; two rules setting
    // `color-scheme` itself would leave it to whichever is written last.
    out.push_str(
        "[data-theme=\"light\"] { --statac-code-scheme: light; }\n\
         [data-theme=\"dark\"] { --statac-code-scheme: dark; }\n\n",
    );
    // Some themes carry a half-transparent text colour, which a terminal
    // ignores and a page does not: left alone it washes out every block.
    writeln!(
        out,
        ".code {{\n  color-scheme: var(--statac-code-scheme, light dark);\n  color: {};\n  \
         background-color: {};\n}}\n",
        both(
            legible(opaque(light.foreground), light.background),
            legible(opaque(dark.foreground), dark.background)
        ),
        both(light.background, dark.background)
    )
    .unwrap();
    for (class, declarations) in classes.rules.iter().enumerate() {
        writeln!(out, ".{} {{{declarations} }}", class_name(class)).unwrap();
    }
    Some(out)
}

/// A colour written so the reader's preference chooses it.
fn both(light: Color, dark: Color) -> String {
    let (light, dark) = (hex(light), hex(dark));
    if light == dark {
        light
    } else {
        format!("light-dark({light}, {dark})")
    }
}

fn hex(colour: Color) -> String {
    colour.as_hex().to_lowercase()
}

/// The least contrast text has with what it stands on, as WCAG asks of
/// text of an ordinary size.
const LEGIBLE: f64 = 4.5;

/// A colour's red, green and blue; none where it is part transparent.
fn solid(colour: Color) -> Option<[u8; 3]> {
    let hex = hex(colour);
    if hex.len() != 7 {
        return None;
    }
    let part = |at: usize| {
        u8::from_str_radix(&hex[at..at + 2], 16).expect("a colour the highlighter wrote")
    };
    Some([part(1), part(3), part(5)])
}

/// How far apart two colours are in lightness, from 1 to 21, as WCAG
/// reckons it.
fn contrast(a: [u8; 3], b: [u8; 3]) -> f64 {
    let luminance = |[r, g, b]: [u8; 3]| {
        let linear = |part: u8| {
            let part = f64::from(part) / 255.0;
            if part <= 0.04045 {
                part / 12.92
            } else {
                ((part + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
    };
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// `text` as it is where it can be read on `on`, and otherwise moved
/// towards black or white, whichever stands further from `on`, by the
/// least that makes it so. A part-transparent colour is left alone.
fn legible(text: Color, on: Color) -> Color {
    let (Some(from), Some(on)) = (solid(text), solid(on)) else {
        return text;
    };
    if contrast(from, on) >= LEGIBLE {
        return text;
    }
    let (black, white) = ([0; 3], [255; 3]);
    let towards = if contrast(black, on) >= contrast(white, on) {
        black
    } else {
        white
    };
    let moved = |hundredths: i32| -> [u8; 3] {
        std::array::from_fn(|part| {
            let (from, to) = (i32::from(from[part]), i32::from(towards[part]));
            u8::try_from(from + (to - from) * hundredths / 100).expect("between two colours")
        })
    };
    let [r, g, b] = (1..100)
        .map(moved)
        .find(|colour| contrast(*colour, on) >= LEGIBLE)
        .unwrap_or(towards);
    Color::from_hex(&format!("#{r:02x}{g:02x}{b:02x}")).expect("a colour written here")
}

fn opaque(colour: Color) -> Color {
    let hex = hex(colour);
    Color::from_hex(&hex[..7]).expect("a colour the highlighter wrote")
}

#[cfg(test)]
mod tests;
