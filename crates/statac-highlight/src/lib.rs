//! Code coloured by the kind of each piece: a comment, a keyword, a
//! string and so on, as [Statac](https://statac.dev) colours it.
//!
//! The grammars are TextMate grammars from npm's `tm-grammars`, carried
//! inside the crate: nothing is read from disk or fetched. Each language's
//! grammar is read the first time code in it is coloured.
//!
//! Code is given two ways, from one reading:
//!
//! - [`html`]: the inside of a `<code>`, each piece of a kind in a `<span>`
//!   whose one class is `hl-` and the kind's [name](Kind::name), such as
//!   `hl-keyword`. Text of no kind carries no class. A stylesheet gives
//!   each class its colour; the crate writes none.
//! - [`pieces`]: each piece of a kind as its byte range in the code, with
//!   its [`Kind`], for an editor's own colouring.
//!
//! A language is named by the word that opens a Markdown code block,
//! such as `rust` or `py`; [`language`] reads one, and [`first_word`] a
//! word that may instead be a file's name, such as `example.rs`.
//!
//! ```
//! use statac_highlight::{Kind, Language};
//!
//! let Language::Known(rust) = statac_highlight::language("Rust") else {
//!     unreachable!()
//! };
//! let html = statac_highlight::html(rust, "let x = 1;").unwrap();
//! assert!(html.starts_with("<span class=\"hl-keyword\">let</span>"));
//!
//! let pieces = statac_highlight::pieces(rust, "let x = 1;").unwrap();
//! assert_eq!(pieces[0].range, 0..3);
//! assert_eq!(pieces[0].kind, Kind::Keyword);
//! ```

mod languages;

use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;
use std::sync::{LazyLock, OnceLock};

use ferriki_textmate::{
    EncodedTokenAttributes, Grammar, GrammarConfiguration, RawGrammar, RawTheme, RawThemeScope,
    RawThemeSetting, RawThemeStyle, SyncRegistry, parse_raw_grammar,
};

/// What [`html`] puts before a kind's name to make its class.
const CLASS_PREFIX: &str = "hl-";

/// The kind of thing a piece of code is.
///
/// Each kind's [name](Kind::name), after `hl-`, is the class [`html`]
/// gives it. Stylesheets colour the classes by name, so a kind keeps its
/// name from one release to the next; a later release may add kinds.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
#[non_exhaustive]
pub enum Kind {
    /// A comment, and the marks that open and close it.
    Comment,
    /// A keyword, such as `fn` or `return`.
    Keyword,
    /// A string, with its quotes.
    String,
    /// A number.
    Number,
    /// A constant, such as `true` or a character escape.
    Constant,
    /// A function's name, where it is defined or called.
    Function,
    /// A type's, class's or module's name.
    Type,
    /// A variable.
    Variable,
    /// A property or a key, such as a field's name.
    Property,
    /// A tag's name, as in HTML.
    Tag,
    /// An attribute's name, as in HTML, or an annotation.
    Attribute,
    /// An operator, such as `+` or `=`.
    Operator,
    /// Punctuation, such as brackets, commas and semicolons.
    Punctuation,
    /// A heading, as in Markdown, or a diff's header.
    Heading,
    /// Emphasised text, as in Markdown.
    Emphasis,
    /// Strong text, as in Markdown.
    Strong,
    /// A link.
    Link,
    /// A line a diff adds.
    Inserted,
    /// A line a diff takes away.
    Deleted,
}

impl Kind {
    /// Every kind, in the order the documentation gives them.
    pub const ALL: &'static [Kind] = &[
        Kind::Comment,
        Kind::Keyword,
        Kind::String,
        Kind::Number,
        Kind::Constant,
        Kind::Function,
        Kind::Type,
        Kind::Variable,
        Kind::Property,
        Kind::Tag,
        Kind::Attribute,
        Kind::Operator,
        Kind::Punctuation,
        Kind::Heading,
        Kind::Emphasis,
        Kind::Strong,
        Kind::Link,
        Kind::Inserted,
        Kind::Deleted,
    ];

    /// The kind's name in lower case, such as `keyword`: its class in
    /// [`html`] after `hl-`.
    pub fn name(self) -> &'static str {
        match self {
            Kind::Comment => "comment",
            Kind::Keyword => "keyword",
            Kind::String => "string",
            Kind::Number => "number",
            Kind::Constant => "constant",
            Kind::Function => "function",
            Kind::Type => "type",
            Kind::Variable => "variable",
            Kind::Property => "property",
            Kind::Tag => "tag",
            Kind::Attribute => "attribute",
            Kind::Operator => "operator",
            Kind::Punctuation => "punctuation",
            Kind::Heading => "heading",
            Kind::Emphasis => "emphasis",
            Kind::Strong => "strong",
            Kind::Link => "link",
            Kind::Inserted => "inserted",
            Kind::Deleted => "deleted",
        }
    }
}

/// A piece of code of one kind.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Piece {
    /// Where the piece is in the code, in bytes.
    pub range: Range<usize>,
    /// What kind of thing it is.
    pub kind: Kind,
}

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
    ("vb.net", "vb"),
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
const ENDINGS: &[(&str, &str)] = &[
    ("h", "c"),
    ("hpp", "cpp"),
    ("cc", "cpp"),
    ("htm", "html"),
    ("atom", "xml"),
    ("csproj", "xml"),
    ("fsproj", "xml"),
    ("nuspec", "xml"),
    ("opml", "xml"),
    ("plist", "xml"),
    ("rdf", "xml"),
    ("rss", "xml"),
    ("svg", "xml"),
    ("vbproj", "xml"),
    ("vcxproj", "xml"),
    ("wsdl", "xml"),
    ("xaml", "xml"),
    ("xlf", "xml"),
    ("xliff", "xml"),
    ("xsd", "xml"),
    ("xslt", "xsl"),
];

/// What a word names, as a language.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Language {
    /// Code to be shown as it is: `text`, `txt`, `plain` or `plaintext`.
    Plain,
    /// A grammar the crate carries, by the name [`html`] and [`pieces`]
    /// take: the word in lower case, or the grammar's own name where the
    /// word is a common name its grammar isn't known by, such as `golang`.
    Known(&'static str),
    /// No language the crate knows.
    Unknown,
}

/// The language `word` names, in any capitals.
pub fn language(word: &str) -> Language {
    let word = word.to_lowercase();
    if PLAIN.contains(&word.as_str()) {
        return Language::Plain;
    }
    if let Some((_, name)) = ALIASES.iter().find(|(alias, _)| *alias == word) {
        return Language::Known(name);
    }
    match languages::LANGUAGES.binary_search(&word.as_str()) {
        Ok(at) => Language::Known(languages::LANGUAGES[at]),
        Err(_) => Language::Unknown,
    }
}

/// Every word [`language`] knows, in lower case, for offering the
/// nearest to a word it doesn't: the words for plain code, then every
/// grammar's names in order, then the common names grammars aren't known
/// by.
pub fn names() -> impl Iterator<Item = &'static str> {
    PLAIN
        .iter()
        .chain(languages::LANGUAGES)
        .chain(ALIASES.iter().map(|(alias, _)| alias))
        .copied()
}

/// What the first word of a Markdown code block's opening line is.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum FirstWord {
    /// A language's name, or a word that is neither a language's nor a
    /// file's.
    Language(Language),
    /// A file's name, with the language its name gives.
    File(Language),
}

impl FirstWord {
    /// The language the word gives, whichever it is.
    pub fn language(self) -> Language {
        match self {
            FirstWord::Language(language) | FirstWord::File(language) => language,
        }
    }
}

/// What the first word of a Markdown code block's opening line is: a
/// language, as every word that names one is, in any capitals; else a
/// file, where it is written as a file's name is, `example.rs` or
/// `site/Makefile`; else a language the crate doesn't know.
///
/// A file's language is the one its whole name gives, as it is usually
/// written (`Gemfile`), or else its ending's (`.rs`). A file whose ending
/// names no language is a file all the same, its code plain
/// (`.gitignore`); a word with no ending and no whole name known, such as
/// `Caddyfile`, names no file.
pub fn first_word(word: &str) -> FirstWord {
    match language(word) {
        Language::Unknown => {
            file_named(word).map_or(FirstWord::Language(Language::Unknown), FirstWord::File)
        }
        named => FirstWord::Language(named),
    }
}

/// The language a word written as a file's name gives: by its whole name
/// as it is usually written, `Gemfile`, or after a folder as a language
/// is, `site/Makefile`; else by its ending, plain where the ending names
/// none, `.gitignore`. None for a word that is no file's name, such as
/// `Caddyfile`, with no ending and no whole name known.
fn file_named(word: &str) -> Option<Language> {
    if !file_shaped(word) {
        return None;
    }
    let own = || {
        let name = last_part(word);
        let after_a_folder = name.len() < word.len();
        let known = NAMED_AS_THEIR_FILES
            .iter()
            .any(|known| known.eq_ignore_ascii_case(name));
        (after_a_folder && known).then(|| language(name))
    };
    by_whole_name(word, |a, b| a == b)
        .or_else(own)
        .or_else(|| ending(word).map(|_| by_ending(word).unwrap_or(Language::Plain)))
}

/// The language the name of a file on disk gives, after any folders
/// before it: by its whole name, in any capitals, or else by its ending.
/// Plain where neither names one.
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

/// What follows the last dot of a file's name, where that is letters and
/// digits with a letter among them: `1.5` and `v1.2` name no file.
fn ending(name: &str) -> Option<&str> {
    let (_, ending) = last_part(name).rsplit_once('.')?;
    let fits = ending.chars().all(|c| c.is_ascii_alphanumeric())
        && ending.chars().any(|c| c.is_ascii_alphabetic());
    fits.then_some(ending)
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
        Language::Unknown => None,
        named => Some(named),
    }
}

/// The longest line, in characters, code may hold and be coloured as
/// Statac colours it: the time colouring a line takes grows with the
/// square of its length.
pub const LONGEST_LINE: usize = 5_000;

/// How long the longest line of `code` is, in characters, where it is
/// longer than [`LONGEST_LINE`]; none where every line fits.
pub fn too_long(code: &str) -> Option<usize> {
    // A line holds no more characters than bytes.
    if code.len() <= LONGEST_LINE {
        return None;
    }
    let longest = code.lines().map(|line| line.chars().count()).max()?;
    (longest > LONGEST_LINE).then_some(longest)
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
            let at = Kind::ALL.iter().position(|known| known.name() == kind);
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
    kinds: Vec<Option<Kind>>,
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
            Kind::ALL.get(kind.checked_sub(1)?).copied()
        });
        Some(Tokeniser {
            kinds: kinds.collect(),
            grammar,
        })
    }

    /// Reads `code`, handing `each` every token in order, as its place in
    /// `code` and its kind, and each line feed between lines as of no
    /// kind, so the places run end to end; none where the grammar fails
    /// on it.
    fn read(&self, code: &str, mut each: impl FnMut(Range<usize>, Option<Kind>)) -> Option<()> {
        let mut state = None;
        let mut start = 0;
        for (number, line) in code.split('\n').enumerate() {
            if number > 0 {
                each(start - 1..start, None);
            }
            let read = self.grammar.tokenize_line2(line, state.take(), 0).ok()?;
            // Each token is where it starts, counted in UTF-16 units, and
            // what it is.
            let (tokens, _) = read.tokens.as_chunks::<2>();
            let ends = tokens.iter().skip(1).map(|[start, _]| *start as usize);
            let mut at = start;
            let mut units = 0;
            for (&[_, what], end) in tokens.iter().zip(ends.chain([usize::MAX])) {
                let from = at;
                for c in line[at - start..].chars() {
                    if units >= end {
                        break;
                    }
                    units += c.len_utf16();
                    at += c.len_utf8();
                }
                if at == from {
                    continue;
                }
                let colour = EncodedTokenAttributes::new(what).foreground() as usize;
                each(from..at, self.kinds.get(colour).copied().flatten());
            }
            start += line.len() + 1;
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

/// The inside of a `<code>` holding `code`, coloured as `language`, a
/// name [`Language::Known`] holds: each piece of a kind in a `<span>`
/// whose class is `hl-` and its kind's name, and text of no kind, line
/// feeds among it, as it is. `&`, `<`, `>` and `"` are escaped. A piece
/// never holds a line feed: a kind carried over a line's end closes there
/// and opens again on the next line.
///
/// None for a name of no grammar the crate carries, and for code its
/// grammar fails on.
pub fn html(language: &str, code: &str) -> Option<String> {
    let tokeniser = tokeniser(language)?;
    let mut out = String::with_capacity(code.len() * 2);
    let mut open = None;
    tokeniser.read(code, |range, kind| {
        if kind != open {
            if open.is_some() {
                out.push_str("</span>");
            }
            if let Some(kind) = kind {
                out.push_str("<span class=\"");
                out.push_str(CLASS_PREFIX);
                out.push_str(kind.name());
                out.push_str("\">");
            }
            open = kind;
        }
        escape(&code[range], &mut out);
    })?;
    if open.is_some() {
        out.push_str("</span>");
    }
    Some(out)
}

/// Each piece of `code` of a kind, coloured as `language`, a name
/// [`Language::Known`] holds, in order: the same pieces as [`html`]
/// marks, as byte ranges in `code`. Text of no kind is in no piece, and a
/// piece never holds a line feed.
///
/// None for a name of no grammar the crate carries, and for code its
/// grammar fails on.
pub fn pieces(language: &str, code: &str) -> Option<Vec<Piece>> {
    let tokeniser = tokeniser(language)?;
    let mut pieces: Vec<Piece> = Vec::new();
    let mut open = false;
    tokeniser.read(code, |range, kind| {
        let Some(kind) = kind else {
            open = false;
            return;
        };
        match pieces.last_mut() {
            Some(last) if open && last.kind == kind => last.range.end = range.end,
            _ => pieces.push(Piece { range, kind }),
        }
        open = true;
    })?;
    Some(pieces)
}

/// Writes `text` into `out` to stand between tags, with `&`, `<`, `>` and
/// `"` escaped.
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

#[cfg(test)]
mod tests;
