//! Code coloured as the site is built, and `code.css`, the colours it is
//! shown in, written from the site's `code_style`.
//!
//! Each piece of code a grammar tells apart is marked with the kind of thing
//! it is, a comment or a keyword, by one class carrying `hl-` so a site's own
//! stylesheet never meets one by chance; code of no kind carries none. The
//! marking is the same in every style. A style is a light theme and its dark
//! twin, giving each kind a colour in both, and the reader's own preference
//! chooses between them. A theme's colour too faint to read on its
//! background is written darker or lighter.
//!
//! The grammars and styles are written by `cargo run --example grammars`,
//! which says where they come from.

mod languages;
mod styles;

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::rc::Rc;
use std::sync::{Arc, LazyLock, Mutex, OnceLock};

use ferriki_textmate::{
    EncodedTokenAttributes, Grammar, GrammarConfiguration, RawGrammar, RawTheme, RawThemeScope,
    RawThemeSetting, RawThemeStyle, SyncRegistry, parse_raw_grammar,
};

/// The name a template asks for the stylesheet by.
pub const STYLESHEET: &str = "code.css";

/// The styles `code_style` takes.
pub const NAMES: &[&str] = &["ocean", "gruvbox", "solarized", "catppuccin", "one-half"];

/// What `code_style` is where the site wants no code coloured.
pub const OFF: &str = "none";

/// Every word `code_style` takes: the styles, then `OFF`.
pub const SETTING: &[&str] = &[
    "ocean",
    "gruvbox",
    "solarized",
    "catppuccin",
    "one-half",
    OFF,
];

const CLASS_PREFIX: &str = "hl-";

/// The kinds of thing code is marked as, each the name of its class after
/// `hl-` and of its property in `code.css` after `--code-`.
pub const KINDS: [&str; 19] = [
    "comment",
    "keyword",
    "string",
    "number",
    "constant",
    "function",
    "type",
    "variable",
    "property",
    "tag",
    "attribute",
    "operator",
    "punctuation",
    "heading",
    "emphasis",
    "strong",
    "link",
    "inserted",
    "deleted",
];

/// The kind of what a grammar gives each scope, as a theme's selectors: the
/// innermost scope named decides, by its longest selector. None is code of
/// no kind, for a scope that lies within a kind's and is none of it.
const SCOPES: &[(&str, Option<&str>)] = &[
    ("comment", Some("comment")),
    ("punctuation.definition.comment", Some("comment")),
    ("keyword", Some("keyword")),
    ("punctuation.definition.keyword", Some("keyword")),
    ("storage", Some("keyword")),
    ("variable.language", Some("keyword")),
    ("keyword.operator.alignas", Some("keyword")),
    ("keyword.operator.alignof", Some("keyword")),
    ("keyword.operator.cast", Some("keyword")),
    ("keyword.operator.expression", Some("keyword")),
    ("keyword.operator.instanceof", Some("keyword")),
    ("keyword.operator.logical.python", Some("keyword")),
    ("keyword.operator.new", Some("keyword")),
    ("keyword.operator.noexcept", Some("keyword")),
    ("keyword.operator.sizeof", Some("keyword")),
    ("keyword.operator.typeid", Some("keyword")),
    ("keyword.operator.word", Some("keyword")),
    ("keyword.operator.wordlike", Some("keyword")),
    ("string", Some("string")),
    ("punctuation.definition.string", Some("string")),
    ("punctuation.definition.char", Some("string")),
    ("markup.inline.raw", Some("string")),
    ("punctuation.definition.raw", Some("string")),
    ("variable.parameter.url", Some("string")),
    ("constant.numeric", Some("number")),
    ("constant.numeric keyword.operator", Some("number")),
    ("constant.numeric punctuation", Some("number")),
    ("constant.numeric storage", Some("number")),
    ("keyword.other.unit", Some("number")),
    ("constant", Some("constant")),
    ("constant.character.entity punctuation", Some("constant")),
    ("punctuation.definition.constant", Some("constant")),
    ("support.constant", Some("constant")),
    ("variable.other.enummember", Some("constant")),
    ("entity.name.function", Some("function")),
    ("entity.name.command", Some("function")),
    ("meta.function-call.generic", Some("function")),
    ("support.function", Some("function")),
    ("variable.function", Some("function")),
    ("entity.name.type", Some("type")),
    ("entity.name.class", Some("type")),
    ("entity.name.module", Some("type")),
    ("entity.name.namespace", Some("type")),
    ("entity.other.inherited-class", Some("type")),
    ("support.type", Some("type")),
    ("support.class", Some("type")),
    ("storage.type.built-in", Some("type")),
    ("storage.type.primitive", Some("type")),
    ("source.go storage.type", Some("type")),
    ("source.java storage.type", Some("type")),
    ("storage.modifier.import", Some("type")),
    ("storage.modifier.package", Some("type")),
    ("variable", Some("variable")),
    ("entity.name.variable", Some("variable")),
    ("punctuation.definition.variable", Some("variable")),
    ("support.variable", Some("variable")),
    ("variable.other.property", Some("property")),
    ("variable.other.object.property", Some("property")),
    ("variable.object.property", Some("property")),
    ("variable.other.member", Some("property")),
    ("variable.other.key", Some("property")),
    ("support.type.property-name", Some("property")),
    ("punctuation.support.type.property-name", Some("property")),
    ("meta.property-name", Some("property")),
    ("meta.object-literal.key", Some("property")),
    ("meta.attribute.python", Some("property")),
    ("entity.name.tag.yaml", Some("property")),
    ("keyword.other.definition.ini", Some("property")),
    ("entity.name.tag", Some("tag")),
    ("entity.other.attribute-name", Some("attribute")),
    (
        "entity.other.attribute-name punctuation.definition.entity",
        Some("attribute"),
    ),
    ("meta.attribute.rust", Some("attribute")),
    ("storage.type.annotation", Some("attribute")),
    ("punctuation.definition.annotation", Some("attribute")),
    ("keyword.operator", Some("operator")),
    ("storage.type.function.arrow", Some("operator")),
    ("punctuation", Some("punctuation")),
    ("meta.brace", Some("punctuation")),
    (
        "meta.link punctuation.definition.constant",
        Some("punctuation"),
    ),
    ("markup.heading", Some("heading")),
    ("punctuation.definition.heading", Some("heading")),
    ("entity.name.section", Some("heading")),
    ("meta.diff.header", Some("heading")),
    ("meta.diff.header punctuation", Some("heading")),
    ("meta.diff.range", Some("heading")),
    ("meta.diff.range punctuation", Some("heading")),
    ("markup.italic", Some("emphasis")),
    ("punctuation.definition.italic", Some("emphasis")),
    ("markup.bold", Some("strong")),
    ("punctuation.definition.bold", Some("strong")),
    ("markup.underline.link", Some("link")),
    ("string.other.link", Some("link")),
    (
        "string.other.link punctuation.definition.string",
        Some("link"),
    ),
    ("constant.other.reference.link", Some("link")),
    ("markup.inserted", Some("inserted")),
    ("punctuation.definition.inserted", Some("inserted")),
    ("markup.deleted", Some("deleted")),
    ("punctuation.definition.deleted", Some("deleted")),
    ("meta.embedded", None),
    ("meta.interpolation", None),
    ("meta.template.expression", None),
];

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

/// A style as `styles.rs` writes it, each colour as its light theme gives
/// it, then its dark.
struct Style {
    name: &'static str,
    /// Text of no kind.
    text: [&'static str; 2],
    background: [&'static str; 2],
    /// Each kind's colour and the font style the light theme gives it, in
    /// the order of `KINDS`.
    kinds: [([&'static str; 2], &'static str); KINDS.len()],
}

/// A grammar as `grammars.bin` holds it.
struct Held {
    scope: &'static str,
    /// Its JSON, deflated.
    deflated: &'static [u8],
    /// The grammars that add rules of their own to it.
    injectors: Vec<usize>,
    /// The grammars it can't be read without: those it and they include,
    /// and those that add to any of them.
    needs: Vec<usize>,
    raw: OnceLock<Option<RawGrammar>>,
}

impl Held {
    /// The grammar read, once for every thread; none for one that can't be.
    fn raw(&self) -> Option<&RawGrammar> {
        let read = || {
            let json = miniz_oxide::inflate::decompress_to_vec(self.deflated).ok()?;
            parse_raw_grammar(std::str::from_utf8(&json).ok()?, Some("grammar.json")).ok()
        };
        self.raw.get_or_init(read).as_ref()
    }
}

/// Every grammar carried, and every name of one in order, with its
/// grammar's place.
struct Carried {
    grammars: Vec<Held>,
    names: Vec<(&'static str, usize)>,
}

/// What is left of `grammars.bin` to read: numbers of four bytes, the
/// lowest first, and texts, each after its length.
struct Unread(&'static [u8]);

impl Unread {
    fn bytes(&mut self, length: usize) -> &'static [u8] {
        let (bytes, rest) = self.0.split_at(length);
        self.0 = rest;
        bytes
    }

    fn number(&mut self) -> usize {
        let bytes = self.bytes(4).try_into().expect("four bytes");
        u32::from_le_bytes(bytes) as usize
    }

    fn numbers(&mut self) -> Vec<usize> {
        (0..self.number()).map(|_| self.number()).collect()
    }

    fn text(&mut self) -> &'static str {
        let length = self.number();
        std::str::from_utf8(self.bytes(length)).expect("a name the generator wrote")
    }
}

static CARRIED: LazyLock<Carried> = LazyLock::new(|| {
    let mut unread = Unread(include_bytes!("grammars.bin"));
    let listed: Vec<_> = (0..unread.number())
        .map(|_| {
            let scope = unread.text();
            (scope, unread.number(), unread.numbers(), unread.numbers())
        })
        .collect();
    let names = (0..unread.number())
        .map(|_| (unread.text(), unread.number()))
        .collect();
    let grammars = listed
        .into_iter()
        .map(|(scope, length, injectors, needs)| Held {
            scope,
            deflated: unread.bytes(length),
            injectors,
            needs,
            raw: OnceLock::new(),
        })
        .collect();
    Carried { grammars, names }
});

/// What a fence's first word names.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Language {
    /// Code shown as it is, as asked.
    Plain,
    /// A grammar, by a name of it Statac carries.
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

/// The colour that stands for a kind, or for none, in the theme the
/// grammars are read with.
fn stand_in(kind: Option<usize>) -> String {
    format!("#{:06x}", kind.map_or(0, |kind| kind + 1))
}

/// A theme whose colours are kinds, so a token's colour is its kind, chosen
/// among `SCOPES` as a theme's rules are.
fn kinds_as_a_theme() -> RawTheme {
    let rule = |scope: Option<&str>, kind: Option<usize>| {
        let mut style = RawThemeStyle::default();
        style.foreground = Some(stand_in(kind));
        let mut rule = RawThemeSetting::default();
        rule.scope = scope.map(|scope| RawThemeScope::String(scope.to_owned()));
        rule.settings = Some(style);
        rule
    };
    let rules = SCOPES.iter().map(|(scope, kind)| {
        let kind = kind.map(|kind| {
            let at = KINDS.iter().position(|known| *known == kind);
            at.expect("a kind there is")
        });
        rule(Some(scope), kind)
    });
    let mut theme = RawTheme::default();
    theme.settings = std::iter::once(rule(None, None)).chain(rules).collect();
    theme
}

/// One language's grammar, ready to read code.
struct Tokeniser {
    grammar: Rc<Grammar>,
    /// The kind each of the theme's colours stands for, by the colour's
    /// number.
    kinds: Vec<Option<usize>>,
}

impl Tokeniser {
    /// None where the grammar or one it needs can't be read.
    fn new(at: usize) -> Option<Tokeniser> {
        let carried = &*CARRIED;
        let held = &carried.grammars[at];
        let mut registry = SyncRegistry::new(Some(kinds_as_a_theme()), None).ok()?;
        for &at in std::iter::once(&at).chain(&held.needs) {
            let grammar = &carried.grammars[at];
            let injectors = grammar.injectors.iter();
            let injectors = injectors.map(|&at| carried.grammars[at].scope.to_owned());
            registry.add_grammar(grammar.raw()?.clone(), injectors.collect());
        }
        let grammar = registry
            // A prefilter costs more to make than it saves over one build's
            // code.
            .grammar_for_scope_name(
                held.scope,
                GrammarConfiguration::default().with_regex_prefilter(false),
            )
            .ok()??;
        let kinds = grammar.color_map().into_iter().map(|colour| {
            let kind = usize::from_str_radix(colour.strip_prefix('#')?, 16).ok()?;
            kind.checked_sub(1).filter(|kind| *kind < KINDS.len())
        });
        Some(Tokeniser {
            kinds: kinds.collect(),
            grammar,
        })
    }

    /// Writes `code` with each piece of a kind in its class; none where
    /// the grammar fails on it.
    fn colour(&self, code: &str, out: &mut String) -> Option<()> {
        let mut state = None;
        for (number, line) in code.split('\n').enumerate() {
            if number > 0 {
                out.push('\n');
            }
            let read = self.grammar.tokenize_line2(line, state.take(), 0).ok()?;
            // Each token is where it starts, counted in UTF-16 units, and
            // what it is.
            let (tokens, _) = read.tokens.as_chunks::<2>();
            let ends = tokens.iter().skip(1).map(|[start, _]| *start as usize);
            let mut rest = line;
            let mut units = 0;
            let mut open: Option<usize> = None;
            for (&[_, what], end) in tokens.iter().zip(ends.chain([usize::MAX])) {
                let mut length = 0;
                for c in rest.chars() {
                    if units >= end {
                        break;
                    }
                    units += c.len_utf16();
                    length += c.len_utf8();
                }
                let (text, after) = rest.split_at(length);
                rest = after;
                if text.is_empty() {
                    continue;
                }
                let colour = EncodedTokenAttributes::new(what).foreground() as usize;
                let kind = self.kinds.get(colour).copied().flatten();
                if kind != open {
                    if open.is_some() {
                        out.push_str("</span>");
                    }
                    if let Some(kind) = kind {
                        write!(out, "<span class=\"{CLASS_PREFIX}{}\">", KINDS[kind]).unwrap();
                    }
                    open = kind;
                }
                escape(text, out);
            }
            if open.is_some() {
                out.push_str("</span>");
            }
            state = Some(read.rule_stack);
        }
        Some(())
    }
}

thread_local! {
    /// Each language asked for on this thread, by its grammar's place. A
    /// tokeniser can't leave its thread, and adding a grammar to one makes
    /// it read all it holds again, so each language has its own.
    static TOKENISERS: RefCell<HashMap<usize, Option<Rc<Tokeniser>>>> =
        RefCell::new(HashMap::new());
}

fn tokeniser(language: &str) -> Option<Rc<Tokeniser>> {
    let names = &CARRIED.names;
    let at = names
        .binary_search_by_key(&language, |(name, _)| name)
        .ok()?;
    let at = names[at].1;
    TOKENISERS.with_borrow_mut(|held| {
        let tokeniser = held.entry(at);
        tokeniser
            .or_insert_with(|| Tokeniser::new(at).map(Rc::new))
            .clone()
    })
}

/// Blocks of code coloured, by their language and their code.
type Done = HashMap<(&'static str, Box<str>), Arc<str>>;

/// Colours code. The same code is coloured the same in every style.
pub struct Highlighter {
    /// Each block once coloured, so a block written twice is coloured once.
    done: Mutex<Done>,
}

impl Highlighter {
    /// None for `none`, and for a name `code_style` doesn't take, which
    /// the setting has already refused.
    pub fn new(code_style: &str) -> Option<Highlighter> {
        NAMES.contains(&code_style).then(|| Highlighter {
            done: Mutex::new(HashMap::new()),
        })
    }

    /// The inside of a `<code>` holding `code`, coloured as `language`.
    pub fn html(&self, language: &'static str, code: &str) -> Arc<str> {
        let key = (language, Box::from(code));
        if let Some(done) = self.done.lock().expect("never poisoned").get(&key) {
            return Arc::clone(done);
        }
        let html: Arc<str> = coloured(language, code).into();
        let mut done = self.done.lock().expect("never poisoned");
        Arc::clone(done.entry(key).or_insert(html))
    }
}

fn coloured(language: &str, code: &str) -> String {
    let mut out = String::with_capacity(code.len() * 2);
    let done = tokeniser(language).and_then(|tokeniser| tokeniser.colour(code, &mut out));
    if done.is_none() {
        out.clear();
        escape(code, &mut out);
    }
    out
}

/// The inside of a `<code>` holding `code` as it is, without colour.
pub fn plain(code: &str) -> String {
    let mut out = String::with_capacity(code.len());
    escape(code, &mut out);
    out
}

/// `html` with each piece of coloured code as its text alone, for a
/// reader that loads no stylesheet. Only a span as [`Highlighter::html`]
/// writes it is taken out: one of the kinds' classes, round text.
pub fn uncoloured(html: &str) -> Cow<'_, str> {
    const OPEN: &str = "<span class=\"hl-";
    const CLOSE: &str = "</span>";
    let mut out = String::new();
    let mut copied = 0;
    let mut from = 0;
    while let Some(at) = html[from..].find(OPEN).map(|at| from + at) {
        from = at + OPEN.len();
        let Some((kind, rest)) = html[from..].split_once("\">") else {
            break;
        };
        let Some(end) = rest.find('<') else {
            break;
        };
        if !KINDS.contains(&kind) || !rest[end..].starts_with(CLOSE) {
            continue;
        }
        out.push_str(&html[copied..at]);
        out.push_str(&rest[..end]);
        from = html.len() - rest.len() + end + CLOSE.len();
        copied = from;
    }
    if copied == 0 {
        return Cow::Borrowed(html);
    }
    out.push_str(&html[copied..]);
    Cow::Owned(out)
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
    let style = styles::STYLES
        .iter()
        .find(|style| style.name == code_style)?;
    let background = style.background.map(colour);
    let text = style.text.map(colour);
    // Some themes carry a half-transparent text colour, which a terminal
    // ignores and a page does not: left alone it washes out every block.
    let shown = |colours: [Colour; 2]| {
        let [light, dark] = [0, 1].map(|side| {
            let colour = match colours[side] == text[side] {
                true => opaque(colours[side]),
                false => colours[side],
            };
            legible(colour, background[side])
        });
        both(light, dark)
    };
    let mut out = format!(
        "/* Code colours: {}, light and dark. Statac writes this file; to change the\n   \
         style, set `code_style` in {}. To change one colour, set its property,\n   \
         `--code-comment` for a comment's, in a stylesheet of your own, on `.code`\n   \
         or on anything around it. */\n\n",
        style.name,
        crate::config::FILE_NAME
    );
    // A custom property is inherited, so the nearest `data-theme` above the
    // code decides, however many stand above it; two rules setting
    // `color-scheme` itself would leave it to whichever is written last.
    out.push_str(
        "[data-theme=\"light\"] { --statac-code-scheme: light; }\n\
         [data-theme=\"dark\"] { --statac-code-scheme: dark; }\n\n",
    );
    writeln!(
        out,
        ".code {{\n  color-scheme: var(--statac-code-scheme, light dark);\n  \
         color: var(--code-text, {});\n  \
         background-color: var(--code-background, {});\n}}\n",
        shown(text),
        both(background[0], background[1])
    )
    .unwrap();
    for (kind, (colours, font)) in KINDS.iter().zip(&style.kinds) {
        write!(
            out,
            ".{CLASS_PREFIX}{kind} {{ color: var(--code-{kind}, {});",
            shown(colours.map(colour))
        )
        .unwrap();
        for word in font.split_whitespace() {
            out.push_str(match word {
                "bold" => " font-weight: bold;",
                "italic" => " font-style: italic;",
                "underline" => " text-decoration: underline;",
                _ => " text-decoration: line-through;",
            });
        }
        out.push_str(" }\n");
    }
    Some(out)
}

/// A colour's red, green and blue, and how far from transparent it is.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
struct Colour([u8; 4]);

impl Colour {
    /// A colour written as `#rrggbb` or `#rrggbbaa`.
    fn from_hex(hex: &str) -> Option<Colour> {
        let digits = hex.strip_prefix('#')?;
        if !matches!(digits.len(), 6 | 8) || !digits.is_ascii() {
            return None;
        }
        let part = |at: usize| match digits.get(at * 2..at * 2 + 2) {
            Some(part) => u8::from_str_radix(part, 16).ok(),
            None => Some(255),
        };
        Some(Colour([part(0)?, part(1)?, part(2)?, part(3)?]))
    }

    fn hex(self) -> String {
        let [r, g, b, a] = self.0;
        match a {
            255 => format!("#{r:02x}{g:02x}{b:02x}"),
            _ => format!("#{r:02x}{g:02x}{b:02x}{a:02x}"),
        }
    }

    /// Its red, green and blue; none where it is part transparent.
    fn solid(self) -> Option<[u8; 3]> {
        let [r, g, b, a] = self.0;
        (a == 255).then_some([r, g, b])
    }
}

fn colour(hex: &str) -> Colour {
    Colour::from_hex(hex).expect("styles.rs writes colours")
}

/// A colour written so the reader's preference chooses it.
fn both(light: Colour, dark: Colour) -> String {
    let (light, dark) = (light.hex(), dark.hex());
    if light == dark {
        light
    } else {
        format!("light-dark({light}, {dark})")
    }
}

/// The least contrast text has with what it stands on, as WCAG asks of
/// text of an ordinary size.
const LEGIBLE: f64 = 4.5;

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
fn legible(text: Colour, on: Colour) -> Colour {
    let (Some(from), Some(on)) = (text.solid(), on.solid()) else {
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
    Colour([r, g, b, 255])
}

fn opaque(colour: Colour) -> Colour {
    let [r, g, b, _] = colour.0;
    Colour([r, g, b, 255])
}

#[cfg(test)]
mod tests;
