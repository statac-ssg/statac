//! Zola's tags in the writing, `{{ name(a="b") }}` and `{% name(a="b") %}`
//! up to `{% end %}`, and the components that replaced them in Zola 0.23,
//! `{{ <name a="b" /> }}` and `{% <name a="b"> %}` up to `{% </name> %}`,
//! each converted where the starter has an equal and otherwise a snippet
//! call, or left exactly as written; links to a page
//! by its file, `@/folder/page.md`, made the page's address; and the forms
//! of Zola's Markdown that Statac's reads differently, counted. Nothing
//! inside a code block or a code span is converted, but `{{/* name() */}}`,
//! Zola's way of showing a tag as text, is the tag as text wherever it
//! stands.
//!
//! Zola 0.23 and later run the whole writing as a template before reading
//! it as Markdown: `raw` is taken out there and what it held left alone,
//! inside code as anywhere, `get_url` is given the address Zola gave, and
//! every other piece of template code is told. Having no shortcodes,
//! 0.23 reads `{{ name(a="b") }}` as a call to one of its functions.

use std::collections::HashSet;
use std::ops::Range;

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag};

use super::{CODE_OPTIONS, EMOJI, LINK_NOWHERE, TAG_IN_CODE, TAG_LEFT, TEMPLATE_CODE};
use crate::import::common::{emoji_names, ends_line, opens_line};
use crate::import::liquid::{self, finding, quoted, split_word, within};
use crate::import::writing::Marked;
use crate::import::{Finding, Spot};

/// How Zola read a page's writing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Reading {
    /// Zola 0.23 and later: as a template, before Markdown.
    Template,
    /// Zola 0.23 and later, for a file `skip_content_templating` names:
    /// nothing in it is run.
    AsWritten,
    /// Before 0.23: only Zola's own tags are run.
    Tags,
    /// As before 0.23, nothing in the old site saying which version made
    /// it.
    Unknown,
}

/// What the writing is converted with.
pub(super) struct Context<'a> {
    pub reading: Reading,
    /// The page's language, and the site's main one.
    pub language: &'a str,
    pub main_language: &'a str,
    /// The names the settings give the feeds.
    pub feeds: &'a [String],
    /// Whether the new site has the starter, whose snippets a call names
    /// or is kept clear of.
    pub starter: bool,
    /// The path the site is under, as `/blog`; empty at the top of its
    /// host.
    pub base: &'a str,
    /// The address, from the top of the site, of the page an `@/` link
    /// names by its path in `content/`, where there is one.
    pub refs: &'a dyn Fn(&str) -> Option<String>,
    /// Where an `@/` link names a file beside a page that came across: the
    /// page, as an `@/` link names it, and the file's path from the page's
    /// folder.
    pub beside: &'a dyn Fn(&str) -> Option<(String, String)>,
    /// The names of the site's own tags, from its templates.
    pub own: &'a [String],
    /// The tags some call anywhere in the site wraps writing with.
    pub wrapping: &'a HashSet<String>,
    pub emoji: bool,
}

/// A snippet call put in place of a tag.
#[derive(Clone, Debug)]
pub(super) struct Call {
    pub snippet: String,
    /// The tag's name, as written.
    pub tag: String,
    /// The names it hands, in order.
    pub names: Vec<String>,
    pub wraps: bool,
    pub spot: Spot,
}

/// The writing converted, with what didn't convert.
pub(super) struct Converted {
    pub writing: String,
    pub findings: Vec<Finding>,
    pub calls: Vec<Call>,
    /// Where each snippet call written here starts, and each `@end`, in
    /// order.
    pub called: Vec<usize>,
}

/// A value handed to a tag.
#[derive(Clone, PartialEq, Debug)]
enum Given {
    Text(String),
    /// A number, `true` or `false`, as written.
    Bare(String),
    List,
    /// What only a template can work out: an expression in braces, a
    /// spread, or a name handing the variable of that name.
    Worked,
}

/// What a tag is.
#[derive(Clone, PartialEq, Debug)]
enum Form {
    /// `{{ name() }}`.
    Alone,
    /// `{% name() %}`, wrapping writing up to `{% end %}`.
    Opening,
    /// `{% end %}`, or `{% </name> %}`.
    End,
    /// `{{/* name() */}}`, and the text Zola showed for it.
    Shown(String),
}

/// A tag as written.
#[derive(Clone, Debug)]
struct Written {
    start: usize,
    end: usize,
    form: Form,
    name: String,
    args: Vec<(String, Given)>,
    /// Whether it is written as a component call.
    component: bool,
}

/// The tag starting at `at`, where one does.
fn tag_at(text: &str, at: usize) -> Option<Written> {
    let rest = &text[at..];
    let (open, close) = match rest.get(..2)? {
        "{{" => ("{{", "}}"),
        "{%" => ("{%", "%}"),
        _ => return None,
    };
    let mut i = 2;
    let mut tag = i;
    skip(rest, &mut tag);
    if rest[tag..].starts_with('<') {
        return component_at(rest, at, close, tag + 1);
    }
    if rest[i..].starts_with("/*") {
        let inner_end = rest[i..].find("*/")? + i;
        let mut after = inner_end + 2;
        skip(rest, &mut after);
        if !rest[after..].starts_with(close) {
            return None;
        }
        return Some(Written {
            start: at,
            end: at + after + 2,
            form: Form::Shown(format!("{open}{}{close}", &rest[i + 2..inner_end])),
            name: String::new(),
            args: Vec::new(),
            component: false,
        });
    }
    skip(rest, &mut i);
    let name = ident(&rest[i..])?;
    i += name.len();
    skip(rest, &mut i);
    if open == "{%" && name == "end" && rest[i..].starts_with(close) {
        return Some(Written {
            start: at,
            end: at + i + 2,
            form: Form::End,
            name: String::new(),
            args: Vec::new(),
            component: false,
        });
    }
    if !rest[i..].starts_with('(') {
        return None;
    }
    i += 1;
    let mut args = Vec::new();
    loop {
        skip(rest, &mut i);
        if rest[i..].starts_with(')') {
            i += 1;
            break;
        }
        let arg = ident(&rest[i..])?;
        i += arg.len();
        skip(rest, &mut i);
        if !rest[i..].starts_with('=') {
            return None;
        }
        i += 1;
        skip(rest, &mut i);
        let (given, used) = value(&rest[i..])?;
        i += used;
        args.push((arg.to_owned(), given));
        skip(rest, &mut i);
        if rest[i..].starts_with(',') {
            i += 1;
        } else if !rest[i..].starts_with(')') {
            return None;
        }
    }
    skip(rest, &mut i);
    if !rest[i..].starts_with(close) {
        return None;
    }
    Some(Written {
        start: at,
        end: at + i + 2,
        form: match open {
            "{{" => Form::Alone,
            _ => Form::Opening,
        },
        name: name.to_owned(),
        args,
        component: false,
    })
}

/// The component call in `rest`, which starts at `at` in the writing and
/// closes with `close`, its tag's name starting at `i`.
fn component_at(rest: &str, at: usize, close: &str, mut i: usize) -> Option<Written> {
    let alone = close == "}}";
    if !alone && rest[i..].starts_with('/') {
        i += 1;
        let name = component_name(&rest[i..])?;
        i += name.len();
        skip(rest, &mut i);
        i += rest[i..].starts_with('>').then_some(1)?;
        skip(rest, &mut i);
        return rest[i..].starts_with(close).then(|| Written {
            start: at,
            end: at + i + 2,
            form: Form::End,
            name: name.to_owned(),
            args: Vec::new(),
            component: true,
        });
    }
    let name = component_name(&rest[i..])?;
    i += name.len();
    let mut args = Vec::new();
    loop {
        let before = i;
        skip(rest, &mut i);
        let ends = if alone { "/>" } else { ">" };
        if rest[i..].starts_with(ends) {
            i += ends.len();
            break;
        }
        if i == before {
            return None;
        }
        if rest[i..].starts_with('{') {
            i += braced(&rest[i..])?;
            args.push((String::new(), Given::Worked));
            continue;
        }
        let arg = attribute(&rest[i..])?;
        i += arg.len();
        if !rest[i..].starts_with('=') {
            args.push((arg.to_owned(), Given::Worked));
            continue;
        }
        i += 1;
        let given = match rest[i..].chars().next()? {
            '"' | '\'' => {
                let (given, used) = value(&rest[i..])?;
                i += used;
                given
            }
            '{' => {
                let used = braced(&rest[i..])?;
                let inner = rest[i + 1..i + used - 1].trim();
                i += used;
                match value(inner) {
                    Some((given @ (Given::Text(_) | Given::Bare(_)), n)) if n == inner.len() => {
                        given
                    }
                    _ => Given::Worked,
                }
            }
            _ => return None,
        };
        args.push((arg.to_owned(), given));
    }
    skip(rest, &mut i);
    rest[i..].starts_with(close).then(|| Written {
        start: at,
        end: at + i + 2,
        form: match alone {
            true => Form::Alone,
            false => Form::Opening,
        },
        name: name.to_owned(),
        args,
        component: true,
    })
}

/// A component's name at the start of `text`: names joined by `.`.
fn component_name(text: &str) -> Option<&str> {
    let mut end = ident(text)?.len();
    while text[end..].starts_with('.')
        && let Some(next) = ident(&text[end + 1..])
    {
        end += 1 + next.len();
    }
    Some(&text[..end])
}

/// An attribute's name at the start of `text`: a name, which may hold `-`
/// after its first character.
fn attribute(text: &str) -> Option<&str> {
    ident(text)?;
    let end = text
        .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '-')))
        .unwrap_or(text.len());
    Some(&text[..end])
}

/// The length of the braces opening `text`, with what they hold, braces
/// within and quoted text included.
fn braced(text: &str) -> Option<usize> {
    let mut depth = 0;
    let mut quote = None;
    let mut chars = text.char_indices();
    while let Some((i, c)) = chars.next() {
        match (quote, c) {
            (Some(_), '\\') => {
                chars.next();
            }
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'' | '`') => quote = Some(c),
            (None, '{') => depth += 1,
            (None, '}') => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
    }
    None
}

/// The names of the components `template` defines, each written
/// `{% component name(...) %}`.
pub(super) fn components(template: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut at = 0;
    while let Some(found) = template[at..].find("{%") {
        let mut i = at + found + 2;
        at = i;
        i += usize::from(template[i..].starts_with('-'));
        skip(template, &mut i);
        let Some(rest) = template[i..].strip_prefix("component") else {
            continue;
        };
        let mut j = 0;
        skip(rest, &mut j);
        if j == 0 {
            continue;
        }
        if let Some(name) = component_name(&rest[j..])
            && rest[j + name.len()..].trim_start().starts_with('(')
            && !names.iter().any(|n| n == name)
        {
            names.push(name.to_owned());
        }
    }
    names
}

/// Whether `template` defines or loads a macro, which Zola 0.23's
/// template reader refuses.
pub(super) fn macros(template: &str) -> bool {
    let mut at = 0;
    while let Some(found) = template[at..].find("{%") {
        let mut i = at + found + 2;
        at = i;
        i += usize::from(template[i..].starts_with('-'));
        skip(template, &mut i);
        let word = ident(&template[i..]).unwrap_or_default();
        if matches!(word, "macro" | "import")
            && template[i + word.len()..].starts_with(char::is_whitespace)
        {
            return true;
        }
    }
    false
}

/// The functions Zola 0.23's templates have: its own, and its template
/// reader's.
const FUNCTIONS: [&str; 17] = [
    "get_env",
    "get_hash",
    "get_image_metadata",
    "get_page",
    "get_random",
    "get_section",
    "get_taxonomy",
    "get_taxonomy_term",
    "get_taxonomy_url",
    "get_url",
    "load_data",
    "now",
    "range",
    "resize_image",
    "text_direction",
    "throw",
    "trans",
];

/// What a page's writing, outside code, shows of the version of Zola it
/// was written for.
#[derive(Clone, Copy, Default, Debug)]
pub(super) struct Signs {
    /// A component call, which only 0.23 and later read.
    pub component: bool,
    /// A call to one of Zola 0.23's functions, of a name the site has no
    /// shortcode by, which would have stopped an earlier version.
    pub function: bool,
    /// `raw`, which only 0.23 and later take out.
    pub raw: bool,
    /// A tag shown as text, `{{/* name() */}}`, which only versions before
    /// 0.23 read.
    pub shown: bool,
}

/// The [`Signs`] in `body`, on a site whose own tags are `own`.
pub(super) fn signs(body: &str, own: &[String]) -> Signs {
    let code = code_ranges(body);
    let mut signs = Signs::default();
    let mut at = 0;
    while let Some(found) = body[at..].find('{') {
        let start = at + found;
        at = start + 1;
        if within(&code, start) {
            continue;
        }
        if let Some((_, _, resume)) = raw_at(body, start) {
            signs.raw = true;
            at = resume;
            continue;
        }
        let Some(tag) = tag_at(body, start) else {
            continue;
        };
        signs.shown |= matches!(tag.form, Form::Shown(_));
        signs.component |= tag.component;
        signs.function |=
            !tag.component && FUNCTIONS.contains(&tag.name.as_str()) && !own.contains(&tag.name);
        at = tag.end;
    }
    signs
}

/// What `{{ get_url(...) }}` wrote, as an address from the top of the
/// host: `Err` where it names a page by its file and no page came across
/// from there. None where it isn't a call to `get_url` whose address can
/// be worked out here, such as one adding the hash of a file.
fn get_url(tag: &Written, cx: &Context) -> Option<Result<String, ()>> {
    if tag.form != Form::Alone || tag.name != "get_url" {
        return None;
    }
    let (mut path, mut language, mut trailing, mut cachebust) = (None, None, false, false);
    for (name, given) in &tag.args {
        match (name.as_str(), given) {
            ("path", Given::Text(text)) => path = Some(text.as_str()),
            ("lang", Given::Text(text)) => language = Some(text.as_str()),
            ("trailing_slash", Given::Bare(truth)) => trailing = truth.parse().ok()?,
            ("cachebust", Given::Bare(truth)) => cachebust = truth.parse().ok()?,
            _ => return None,
        }
    }
    let path = path?;
    // A page named by its file, in the page's own language unless `lang`
    // says otherwise: `post.fr.md` for French.
    if let Some(page) = path.strip_prefix("@/") {
        let (file, anchor) = match page.split_once('#') {
            Some((file, anchor)) => (file, format!("#{anchor}")),
            None => (page, String::new()),
        };
        let language = language.unwrap_or(cx.language);
        // Zola looks for a file beside a page by the whole path, so never
        // one with a `#` part.
        if let Some(address) = beside(page, language, false, cx) {
            return Some(address);
        }
        let file = match language == cx.main_language {
            true => file.to_owned(),
            false => in_language(file, language),
        };
        let address = (cx.refs)(&file).ok_or(());
        return Some(address.map(|address| format!("{}{address}{anchor}", cx.base)));
    }
    if cachebust {
        return None;
    }
    let path = match language.filter(|language| *language != cx.main_language) {
        Some(language) => {
            let bare = path.strip_prefix('/').unwrap_or(path);
            match bare == language || bare.starts_with(&format!("{language}/")) {
                true => path.to_owned(),
                false => format!("{language}/{}", path.trim_start_matches('/')),
            }
        }
        None => path.to_owned(),
    };
    // Zola ends the address with a slash, unless it names a feed or a
    // place on a page, and then takes the slash off unless asked to keep it.
    let slash = match path.ends_with('/')
        || path.is_empty()
        || path.contains('#')
        || cx.feeds.iter().any(|feed| path.ends_with(feed.as_str()))
    {
        true => "",
        false => "/",
    };
    let mut address = match path.as_str() {
        "/" => format!("{}/", cx.base),
        _ if path.starts_with('/') => format!("{}{path}{slash}", cx.base),
        _ => format!("{}/{path}{slash}", cx.base),
    };
    if !trailing && address.ends_with('/') {
        address.pop();
    }
    if address.is_empty() {
        address.push('/');
    }
    Some(Ok(address))
}

/// The file `file`, of the main language, in `language`: `post.fr.md`
/// for French.
fn in_language(file: &str, language: &str) -> String {
    match file.rsplit_once('.') {
        Some((stem, extension)) => format!("{stem}.{language}.{extension}"),
        None => format!("{language}.{file}"),
    }
}

/// The address of the file beside a page that `path`, in `content/`,
/// names from a page in `language`: the page's address in that language,
/// or else in the main one where `either` says, then the file's path from
/// the page's folder. `Err` where no such page came across; None where
/// `path` names no file beside a page.
fn beside(path: &str, language: &str, either: bool, cx: &Context) -> Option<Result<String, ()>> {
    let (page, rest) = (cx.beside)(path)?;
    let own = (language != cx.main_language)
        .then(|| (cx.refs)(&in_language(&page, language)))
        .flatten();
    let address = match own {
        None if language == cx.main_language || either => (cx.refs)(&page),
        own => own,
    };
    Some(
        address
            .map(|address| format!("{}{address}{rest}", cx.base))
            .ok_or(()),
    )
}

/// `raw` opening at `at` up to its `endraw`: where the writing before it
/// ends, what it held, and where the writing after it starts.
fn raw_at(body: &str, at: usize) -> Option<(usize, Range<usize>, usize)> {
    if !body[at..].starts_with("{%") {
        return None;
    }
    let (length, inner, _) = liquid::tag_at(body, at)?;
    if split_word(&inner) != ("raw", "") {
        return None;
    }
    liquid::raw(body, at, at + length, "endraw")
}

/// The length of the template code opening `text`, `{{ }}`, `{% %}` or
/// `{# #}`, as Zola 0.23's template reader reads it, where it opens some.
fn template_code(text: &str) -> Option<usize> {
    let close = match text.get(..2)? {
        "{{" => "}}",
        "{%" => "%}",
        "{#" => return Some(text[2..].find("#}")? + 4),
        _ => return None,
    };
    let mut quote = None;
    let mut chars = text.char_indices().skip(2);
    while let Some((i, c)) = chars.next() {
        match (quote, c) {
            (Some(_), '\\') => {
                chars.next();
            }
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'' | '`') => quote = Some(c),
            (None, _) if text[i..].starts_with(close) => return Some(i + 2),
            _ => {}
        }
    }
    None
}

/// Whether the `{#` at `at` opens a heading's id, which Zola keeps from
/// its template reader: the first `{#name}` on a line opening with `#` to
/// `######` and a space.
fn heading_id(text: &str, at: usize) -> bool {
    let start = text[..at].rfind('\n').map_or(0, |i| i + 1);
    let end = text[at..].find('\n').map_or(text.len(), |i| at + i);
    let line = &text[start..end];
    let indent = line.len() - line.trim_start_matches(' ').len();
    let hashes = line[indent..].len() - line[indent..].trim_start_matches('#').len();
    if indent > 3 || !(1..=6).contains(&hashes) || !line[indent + hashes..].starts_with(' ') {
        return false;
    }
    let first = (indent + hashes + 1..line.len())
        .filter(|i| line.is_char_boundary(*i))
        .find(|i| id_at(&line[*i..]));
    first == Some(at - start)
}

/// Whether `text` opens with `{#name}`, where the name holds no brace,
/// and neither opens with a space or `#` nor ends with `#`.
fn id_at(text: &str) -> bool {
    let Some(rest) = text.strip_prefix("{#") else {
        return false;
    };
    let Some(end) = rest.find(['{', '}']) else {
        return false;
    };
    let name = &rest[..end];
    rest[end..].starts_with('}')
        && !name.starts_with(|c: char| c.is_whitespace() || c == '#')
        && !name.ends_with('#')
        && !name.is_empty()
}

fn skip(text: &str, at: &mut usize) {
    while let Some(c) = text[*at..].chars().next().filter(|c| c.is_whitespace()) {
        *at += c.len_utf8();
    }
}

/// The name at the start of `text`: a letter or `_`, then letters,
/// figures and `_`.
fn ident(text: &str) -> Option<&str> {
    if !text.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') {
        return None;
    }
    let end = text
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(text.len());
    Some(&text[..end])
}

/// A value at the start of `text`, with how much of it it took.
fn value(text: &str) -> Option<(Given, usize)> {
    let first = text.chars().next()?;
    match first {
        '"' | '\'' | '`' => {
            let mut out = String::new();
            let mut chars = text.char_indices().skip(1);
            while let Some((i, c)) = chars.next() {
                match c {
                    '\\' if first == '"' => out.push(chars.next()?.1),
                    c if c == first => return Some((Given::Text(out), i + 1)),
                    c => out.push(c),
                }
            }
            None
        }
        '[' => {
            let mut at = 1;
            loop {
                skip(text, &mut at);
                if text[at..].starts_with(']') {
                    return Some((Given::List, at + 1));
                }
                let (_, used) = value(&text[at..])?;
                at += used;
                skip(text, &mut at);
                if text[at..].starts_with(',') {
                    at += 1;
                }
            }
        }
        _ => {
            let end = text
                .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+')))
                .unwrap_or(text.len());
            let word = &text[..end];
            let number = word.parse::<f64>().is_ok();
            (end > 0 && (number || word == "true" || word == "false"))
                .then(|| (Given::Bare(word.to_owned()), end))
        }
    }
}

/// Where the tag opened before `from` is closed: the `{% end %}`, counting
/// others opened between.
fn end_of(text: &str, from: usize) -> Option<Written> {
    let mut depth = 0;
    let mut at = from;
    while let Some(found) = text[at..].find("{%") {
        let start = at + found;
        match tag_at(text, start) {
            Some(tag) => {
                match tag.form {
                    Form::End if depth == 0 => return Some(tag),
                    Form::End => depth -= 1,
                    Form::Opening => depth += 1,
                    _ => {}
                }
                at = tag.end;
            }
            None => at = start + 2,
        }
    }
    None
}

/// The names of the tags in `body` that wrap writing.
pub(super) fn paired(body: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut at = 0;
    while let Some(found) = body[at..].find("{%") {
        let start = at + found;
        match tag_at(body, start) {
            Some(tag) => {
                if tag.form == Form::Opening && !names.contains(&tag.name) {
                    names.push(tag.name.clone());
                }
                at = tag.end;
            }
            None => at = start + 2,
        }
    }
    names
}

fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_HEADING_ATTRIBUTES
}

/// The ranges of `text` Markdown reads as code.
fn code_ranges(text: &str) -> Vec<Range<usize>> {
    Parser::new_ext(text, options())
        .into_offset_iter()
        .filter(|(event, _)| matches!(event, Event::Start(Tag::CodeBlock(_)) | Event::Code(_)))
        .map(|(_, range)| range)
        .collect()
}

/// What a tag left open stands for when its `{% end %}` comes.
enum Open {
    End,
    Left,
}

/// Converts the writing of `file`, whose first line is line `first_line`
/// of the file.
pub(super) fn convert(body: &str, first_line: usize, file: &str, cx: &Context) -> Converted {
    // Each step keeps every line where it was, so a place is told by
    // counting the lines of the writing the step is given.
    let spotter = |text: &str| {
        let text = text.to_owned();
        move |at: usize, written: &str| Spot {
            file: file.to_owned(),
            line: Some(first_line + text[..at].matches('\n').count()),
            text: written.to_owned(),
            times: None,
        }
    };
    let mut findings = Vec::new();
    let body = fences_bare(body, &spotter(body), &mut findings);
    let body = links_and_more(&body, cx, &spotter(&body), &mut findings);
    let body = body.as_str();
    let spot = spotter(body);
    if cx.emoji {
        findings.extend(emoji(body, &spot));
    }
    let code = code_ranges(body);
    let mut calls = Vec::new();
    let mut out = Marked::default();
    let mut open: Vec<Open> = Vec::new();
    let mut copied = 0;
    let mut at = 0;
    while cx.reading != Reading::AsWritten
        && let Some(found) = body[at..].find(['{'])
    {
        let start = at + found;
        if cx.reading == Reading::Template
            && let Some((from, held, resume)) = raw_at(body, start)
        {
            out.push_str(&body[copied..from]);
            out.push_str(&body[held]);
            copied = resume;
            at = resume;
            continue;
        }
        let Some(tag) = tag_at(body, start) else {
            at = start + 1;
            let kind = match (cx.reading, within(&code, start)) {
                (Reading::Template, true) => &TAG_IN_CODE,
                (Reading::Template, false) => &TAG_LEFT,
                (Reading::Unknown, _) => &TEMPLATE_CODE,
                _ => continue,
            };
            let length = template_code(&body[start..]).filter(|_| !heading_id(body, start));
            if let Some(length) = length {
                findings.push(finding(kind, spot(start, &body[start..start + length])));
                at = start + length;
            }
            continue;
        };
        let written = &body[tag.start..tag.end];
        let mut replace = |out: &mut Marked, with: Marked, resume: usize| {
            out.push_str(&body[copied..start]);
            out.push(with);
            copied = resume;
        };
        at = tag.end;
        if let Form::Shown(shown) = &tag.form {
            replace(&mut out, shown.clone().into(), tag.end);
            continue;
        }
        if cx.reading == Reading::Template && !tag.component {
            // With no shortcodes in 0.23, this calls one of Zola's
            // functions, which it ran inside code as anywhere.
            let kind = match get_url(&tag, cx) {
                Some(Ok(address)) => {
                    replace(&mut out, address.into(), tag.end);
                    continue;
                }
                Some(Err(())) => &LINK_NOWHERE,
                None if within(&code, start) => &TAG_IN_CODE,
                None => &TAG_LEFT,
            };
            findings.push(finding(kind, spot(start, written)));
            continue;
        }
        if within(&code, start) {
            findings.push(finding(&TAG_IN_CODE, spot(start, written)));
            continue;
        }
        let left =
            |findings: &mut Vec<Finding>| findings.push(finding(&TAG_LEFT, spot(start, written)));
        match tag.form {
            Form::End => match open.pop() {
                Some(Open::End) => replace(&mut out, Marked::call("@end"), tag.end),
                _ => left(&mut findings),
            },
            Form::Opening => {
                let close = end_of(body, tag.end);
                let alone = close.as_ref().is_some_and(|close| {
                    opens_line(body, start)
                        && ends_line(body, tag.end)
                        && opens_line(body, close.start)
                        && ends_line(body, close.end)
                });
                match snippet_call(&tag, cx).filter(|_| alone) {
                    Some((text, call)) => {
                        calls.push(Call {
                            spot: spot(start, written),
                            ..call
                        });
                        open.push(Open::End);
                        replace(&mut out, text, tag.end);
                    }
                    None => {
                        left(&mut findings);
                        if close.is_some() {
                            open.push(Open::Left);
                        }
                    }
                }
            }
            _ => {
                // Without the starter there is no snippet for a video to
                // call, so it is called as any other tag is.
                let starters = cx.starter && !cx.own.contains(&tag.name);
                let video = match (starters, tag.name.as_str(), tag.args.as_slice()) {
                    (true, "youtube" | "vimeo", [(name, Given::Text(id))]) if name == "id" => {
                        Some(Marked::call(format!("@{}({})", tag.name, quoted(id))))
                    }
                    (true, "youtube" | "vimeo", _) => None,
                    _ => {
                        let wraps = cx.wrapping.contains(&tag.name);
                        let alone = opens_line(body, start) && ends_line(body, tag.end);
                        let made = snippet_call(&tag, cx).filter(|_| !wraps || alone);
                        made.map(|(mut text, call)| {
                            calls.push(Call {
                                spot: spot(start, written),
                                ..call
                            });
                            if wraps {
                                text.push_str("\n");
                                text.push(Marked::call("@end"));
                            }
                            text
                        })
                    }
                };
                match video {
                    Some(text) => replace(&mut out, text, tag.end),
                    None => left(&mut findings),
                }
            }
        }
    }
    out.push_str(&body[copied..]);
    findings.sort_by_key(|finding| finding.spots[0].line);
    Converted {
        writing: out.text,
        findings,
        calls,
        called: out.called,
    }
}

/// The snippet a tag's call names: its own name, kept clear of the
/// starter's where the new site has it.
pub(super) fn snippet_name(tag: &str, starter: bool) -> String {
    match (starter && crate::new::has_snippet(tag)) || tag.starts_with('_') {
        true => format!("old_{tag}"),
        false => tag.to_owned(),
    }
}

/// A tag as a call to a snippet of its name, where its values can be
/// handed.
fn snippet_call(tag: &Written, cx: &Context) -> Option<(Marked, Call)> {
    // A snippet's name holds no `.`.
    if tag.name.contains('.') {
        return None;
    }
    let mut names = Vec::new();
    let mut handed = Vec::new();
    for (name, given) in &tag.args {
        let value = match given {
            Given::Text(text) if !text.contains(['\n', '\r']) => quoted(text),
            Given::Bare(bare) => bare.clone(),
            _ => return None,
        };
        if names.contains(name) {
            return None;
        }
        handed.push(format!("{name}={value}"));
        names.push(name.clone());
    }
    let snippet = snippet_name(&tag.name, cx.starter);
    Some((
        Marked::call(format!("@{snippet}({})", handed.join(", "))),
        Call {
            snippet,
            tag: tag.name.clone(),
            names,
            wraps: cx.wrapping.contains(&tag.name),
            spot: Spot {
                file: String::new(),
                line: None,
                text: String::new(),
                times: None,
            },
        },
    ))
}

/// The writing with Zola's options after a comma on each fenced code
/// block's first line, as in ```` ```rust,linenos ````, taken off, each
/// told.
fn fences_bare(
    body: &str,
    spot: &dyn Fn(usize, &str) -> Spot,
    findings: &mut Vec<Finding>,
) -> String {
    let mut cuts: Vec<Range<usize>> = Vec::new();
    for (event, range) in Parser::new_ext(body, options()).into_offset_iter() {
        let Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) = event else {
            continue;
        };
        if !info.contains(',') {
            continue;
        }
        let line_end = body[range.start..]
            .find('\n')
            .map_or(body.len(), |i| range.start + i);
        let line = &body[range.start..line_end];
        let Some(comma) = line.find(',') else {
            continue;
        };
        findings.push(finding(&CODE_OPTIONS, spot(range.start, line.trim())));
        let cut_to = line.trim_end_matches('\r').len();
        cuts.push(range.start + comma..range.start + cut_to);
    }
    let mut out = String::with_capacity(body.len());
    let mut copied = 0;
    for cut in cuts {
        out.push_str(&body[copied..cut.start]);
        copied = cut.end;
    }
    out.push_str(&body[copied..]);
    out
}

/// The writing with each `@/` link outside code made the address of the
/// page it names, and each `<!-- more -->` written as Statac spells it: the
/// shared part sees to one standing where Statac takes none.
fn links_and_more(
    body: &str,
    cx: &Context,
    spot: &dyn Fn(usize, &str) -> Spot,
    findings: &mut Vec<Finding>,
) -> String {
    let code = code_ranges(body);
    let mut out = String::with_capacity(body.len());
    let mut copied = 0;
    let mut at = 0;
    while let Some(found) = body[at..].find(['@', '<']) {
        let start = at + found;
        at = start + 1;
        if within(&code, start) {
            continue;
        }
        if body[start..].starts_with("<!--") {
            let mut i = start + 4;
            skip(body, &mut i);
            if !body[i..].starts_with("more") {
                continue;
            }
            i += 4;
            skip(body, &mut i);
            if !body[i..].starts_with("-->") {
                continue;
            }
            let end = i + 3;
            at = end;
            out.push_str(&body[copied..start]);
            out.push_str("<!--more-->");
            copied = end;
            continue;
        }
        if !body[start..].starts_with("@/") {
            continue;
        }
        let before = body[..start].trim_end_matches([' ', '\t']);
        let linked = before.ends_with(['(', '<']) || before.ends_with("]:");
        if !linked {
            continue;
        }
        let end = body[start..]
            .find(|c: char| c.is_whitespace() || matches!(c, ')' | '>' | '"' | '\''))
            .map_or(body.len(), |n| start + n);
        let written = &body[start..end];
        let (path, anchor) = match written[2..].split_once('#') {
            Some((path, anchor)) => (path, format!("#{anchor}")),
            None => (&written[2..], String::new()),
        };
        // A file beside a page comes first, and Zola drops any `#` part.
        let address = match beside(path, cx.language, true, cx) {
            Some(address) => address.ok(),
            None => (cx.refs)(path).map(|address| format!("{}{address}{anchor}", cx.base)),
        };
        match address {
            Some(address) => {
                out.push_str(&body[copied..start]);
                out.push_str(&address);
                copied = end;
            }
            None => findings.push(finding(&LINK_NOWHERE, spot(start, written))),
        }
        at = end;
    }
    out.push_str(&body[copied..]);
    out
}

/// Each `:name:` outside code that Zola showed as an emoji.
fn emoji(body: &str, spot: &dyn Fn(usize, &str) -> Spot) -> Vec<Finding> {
    let code = code_ranges(body);
    let mut found = Vec::new();
    let mut at = 0;
    for line in body.split_inclusive('\n') {
        let start = at;
        at += line.len();
        for (offset, name) in emoji_names(line.trim_end_matches(['\n', '\r'])) {
            if !within(&code, start + offset) {
                found.push(finding(&EMOJI, spot(start + offset, name)));
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tag_is_read_with_its_values() {
        let text = r#"{{ gist(url="https://example.org/a", n=2, on=true, list=[1, "a"]) }}"#;
        let tag = tag_at(text, 0).unwrap();
        assert_eq!(tag.end, text.len());
        assert_eq!(tag.form, Form::Alone);
        assert_eq!(tag.name, "gist");
        assert_eq!(
            tag.args,
            vec![
                ("url".into(), Given::Text("https://example.org/a".into())),
                ("n".into(), Given::Bare("2".into())),
                ("on".into(), Given::Bare("true".into())),
                ("list".into(), Given::List),
            ]
        );
        let opening = tag_at("{% note(kind='tip') %}", 0).unwrap();
        assert_eq!(
            (opening.form, opening.name.as_str()),
            (Form::Opening, "note")
        );
        assert_eq!(tag_at("{%end%}", 0).unwrap().form, Form::End);
        assert_eq!(
            tag_at("{{/* youtube(id=\"a\") */}}", 0).unwrap().form,
            Form::Shown("{{ youtube(id=\"a\") }}".into())
        );
        assert!(tag_at("{{ page.title }}", 0).is_none());
        assert!(tag_at("{{ a(b=\"open) }}", 0).is_none());
    }

    #[test]
    fn a_component_call_is_read_with_its_values() {
        let text = r#"{{ <ui.button label="a" n={2} on={ true } text={'b'} /> }}"#;
        let tag = tag_at(text, 0).unwrap();
        assert_eq!(tag.end, text.len());
        assert_eq!((tag.form, tag.name.as_str()), (Form::Alone, "ui.button"));
        assert_eq!(
            tag.args,
            vec![
                ("label".into(), Given::Text("a".into())),
                ("n".into(), Given::Bare("2".into())),
                ("on".into(), Given::Bare("true".into())),
                ("text".into(), Given::Text("b".into())),
            ]
        );
        let tight = tag_at(r#"{{<button label="a"/>}}"#, 0).unwrap();
        assert_eq!((tight.form, tight.args.len()), (Form::Alone, 1));
        // What only a template can work out is read, to be told.
        let worked = tag_at("{% <aside title {...values} n={page.n} data-x='y'> %}", 0).unwrap();
        assert_eq!(worked.form, Form::Opening);
        assert_eq!(
            worked.args,
            vec![
                ("title".into(), Given::Worked),
                (String::new(), Given::Worked),
                ("n".into(), Given::Worked),
                ("data-x".into(), Given::Text("y".into())),
            ]
        );
        let end = tag_at("{% </aside> %}", 0).unwrap();
        assert_eq!((end.form, end.name.as_str()), (Form::End, "aside"));
        for text in [
            "{{ <aside> }}",
            "{% <aside /> %}",
            "{{ </aside> }}",
            "{% </aside %}",
            "{{ < aside /> }}",
            "{{ <aside a=\"b\"c=\"d\" /> }}",
            "{{ <aside a=b /> }}",
            "{{ <aside a={b /> }}",
        ] {
            assert!(tag_at(text, 0).is_none(), "{text}");
        }
    }

    #[test]
    fn a_template_names_the_components_it_defines() {
        let template = "{% component quote(author) %}{{ body }}{% endcomponent %}\n\
                        {%- component ui.button( label ) -%}x{%- endcomponent -%}\n\
                        {% component quote(author) %}{% endcomponent %}\n\
                        {% components %}{% component %}{{ component(a) }}";
        assert_eq!(components(template), ["quote", "ui.button"]);
    }

    #[test]
    fn private_use_characters_are_kept_and_never_taken_for_a_call() {
        let wrapping = HashSet::from(["aside".to_owned()]);
        let cx = Context {
            reading: Reading::Tags,
            language: "en",
            main_language: "en",
            feeds: &[],
            starter: true,
            base: "",
            refs: &|_| None,
            beside: &|_| None,
            own: &[],
            wrapping: &wrapping,
            emoji: false,
        };
        // The writing, what it becomes, and the line from each call made.
        let cases: [(&str, &str, &[&str]); 5] = [
            (
                "A \u{E000} and \u{E001}.\n\n<div title=\"\u{E001}\">\u{E000}</div>\n",
                "A \u{E000} and \u{E001}.\n\n<div title=\"\u{E001}\">\u{E000}</div>\n",
                &[],
            ),
            (
                "```\n\u{E001}\u{E000}\n```\n",
                "```\n\u{E001}\u{E000}\n```\n",
                &[],
            ),
            (
                "\u{E000}{{ youtube(id=\"\u{E000}\") }}\n",
                "\u{E000}@youtube('\u{E000}')\n",
                &["@youtube('\u{E000}')"],
            ),
            (
                "{% aside(a=\"\u{E001}\") %}\n\u{E000}\n{% end %}\n",
                "@aside(a='\u{E001}')\n\u{E000}\n@end\n",
                &["@aside(a='\u{E001}')", "@end"],
            ),
            (
                "{{ aside(a=\"\u{E000}\") }}\n",
                "@aside(a='\u{E000}')\n@end\n",
                &["@aside(a='\u{E000}')", "@end"],
            ),
        ];
        for (body, writing, called) in cases {
            let converted = convert(body, 1, "a.md", &cx);
            assert_eq!(converted.writing, writing, "{body:?}");
            let lines: Vec<&str> = (converted.called.iter())
                .map(|at| writing[*at..].lines().next().unwrap_or_default())
                .collect();
            assert_eq!(lines, called, "{body:?}");
        }
    }

    fn converted(body: &str, reading: Reading) -> (String, Vec<(&'static str, String)>) {
        let wrapping = HashSet::new();
        let cx = Context {
            reading,
            language: "en",
            main_language: "en",
            feeds: &[],
            starter: true,
            base: "",
            refs: &|_| None,
            beside: &|_| None,
            own: &[],
            wrapping: &wrapping,
            emoji: false,
        };
        let converted = convert(body, 1, "a.md", &cx);
        let found = (converted.findings.into_iter())
            .map(|f| (f.kind.heading, f.spots[0].text.clone()))
            .collect();
        (converted.writing, found)
    }

    #[test]
    fn writing_run_as_a_template_loses_raw_and_tells_its_template_code() {
        let body = "This is {{ page.title }}.\n\
                    \n\
                    {% raw %}\n\
                    Shown: {{ x }}\n\
                    {% endraw %}\n\
                    \n\
                    {% if true %}Text 1.{% endif %} {# a note #}\n\
                    \n\
                    ## Heading 1 {#id-1}\n\
                    \n\
                    `{{ \"}}\" }}` and {% raw %}{% x %}{% endraw %} as text.\n\
                    \n\
                    ```\n\
                    {{ page.date }}\n\
                    {% raw %}{{ y }}{% endraw %}\n\
                    ```\n";
        let (writing, found) = converted(body, Reading::Template);
        assert_eq!(
            writing,
            "This is {{ page.title }}.\n\
             \n\
             Shown: {{ x }}\n\
             \n\
             {% if true %}Text 1.{% endif %} {# a note #}\n\
             \n\
             ## Heading 1 {#id-1}\n\
             \n\
             `{{ \"}}\" }}` and {% x %} as text.\n\
             \n\
             ```\n\
             {{ page.date }}\n\
             {{ y }}\n\
             ```\n"
        );
        let left = TAG_LEFT.heading;
        let in_code = TAG_IN_CODE.heading;
        assert_eq!(
            found,
            [
                (left, "{{ page.title }}".into()),
                (left, "{% if true %}".into()),
                (left, "{% endif %}".into()),
                (left, "{# a note #}".into()),
                (in_code, "{{ \"}}\" }}".into()),
                (in_code, "{{ page.date }}".into()),
            ]
        );
    }

    #[test]
    fn writing_not_run_as_a_template_keeps_raw() {
        let body = "{% raw %}\n{{ page.title }}\n{% endraw %}\n\n{{ <youtube id=\"a\" /> }}\n";
        // Zola before 0.23 showed both as written, and so does a file
        // `skip_content_templating` names.
        for reading in [Reading::Tags, Reading::AsWritten] {
            let (writing, found) = converted(body, reading);
            let expected = match reading {
                Reading::Tags => body.replace("{{ <youtube id=\"a\" /> }}", "@youtube('a')"),
                _ => body.to_owned(),
            };
            assert_eq!(writing, expected, "{reading:?}");
            assert_eq!(found, [], "{reading:?}");
        }
        let (writing, found) = converted(body, Reading::Unknown);
        assert_eq!(
            writing,
            body.replace("{{ <youtube id=\"a\" /> }}", "@youtube('a')")
        );
        let may = TEMPLATE_CODE.heading;
        assert_eq!(
            found,
            [
                (may, "{% raw %}".into()),
                (may, "{{ page.title }}".into()),
                (may, "{% endraw %}".into()),
            ]
        );
    }

    #[test]
    fn get_url_gives_the_address_zola_gave_and_other_functions_are_told() {
        let wrapping = HashSet::new();
        let refs = |file: &str| match file {
            "posts/post-1.md" => Some("/posts/post-1/".to_owned()),
            "posts/post-1.fr.md" => Some("/fr/posts/post-1/".to_owned()),
            _ => None,
        };
        let feeds = ["atom.xml".to_owned()];
        // The call, the page's language, the site's path, and what it
        // becomes, or None where it is left and told.
        let cases: [(&str, &str, &str, Option<&str>); 17] = [
            (
                r#"get_url(path="@/posts/post-1.md")"#,
                "en",
                "",
                Some("/posts/post-1/"),
            ),
            (
                r#"get_url(path="@/posts/post-1.md#a")"#,
                "en",
                "/blog",
                Some("/blog/posts/post-1/#a"),
            ),
            (
                r#"get_url(path="@/posts/post-1.md")"#,
                "fr",
                "",
                Some("/fr/posts/post-1/"),
            ),
            (
                r#"get_url(path="@/posts/post-1.md", lang="en")"#,
                "fr",
                "",
                Some("/posts/post-1/"),
            ),
            (
                r#"get_url(path="@/posts/post-1.md", trailing_slash=false)"#,
                "en",
                "",
                Some("/posts/post-1/"),
            ),
            (r#"get_url(path="@/posts/post-2.md")"#, "en", "", None),
            (
                r#"get_url(path="files/a.txt")"#,
                "en",
                "",
                Some("/files/a.txt"),
            ),
            (
                r#"get_url(path="/files/")"#,
                "en",
                "/blog",
                Some("/blog/files"),
            ),
            (
                r#"get_url(path="files", trailing_slash=true)"#,
                "en",
                "",
                Some("/files/"),
            ),
            (
                r#"get_url(path="atom.xml", trailing_slash=true)"#,
                "en",
                "",
                Some("/atom.xml"),
            ),
            (
                r#"get_url(path="/", trailing_slash=true)"#,
                "en",
                "/blog",
                Some("/blog/"),
            ),
            (r#"get_url(path="/")"#, "en", "", Some("/")),
            (
                r#"get_url(path="files/a.txt", lang="fr")"#,
                "en",
                "",
                Some("/fr/files/a.txt"),
            ),
            (
                r#"get_url(path="fr/files/a.txt", lang="fr")"#,
                "en",
                "",
                Some("/fr/files/a.txt"),
            ),
            (r#"get_url(path="a.css", cachebust=true)"#, "en", "", None),
            (r#"get_url(path="a.css", other=1)"#, "en", "", None),
            (r#"now()"#, "en", "", None),
        ];
        for (call, language, base, becomes) in cases {
            let cx = Context {
                reading: Reading::Template,
                language,
                main_language: "en",
                feeds: &feeds,
                starter: true,
                base,
                refs: &refs,
                beside: &|_| None,
                own: &[],
                wrapping: &wrapping,
                emoji: false,
            };
            let body = format!("{{{{ {call} }}}}\n");
            let converted = convert(&body, 1, "a.md", &cx);
            match becomes {
                Some(address) => {
                    assert_eq!(converted.writing, format!("{address}\n"), "{call}");
                    assert!(converted.findings.is_empty(), "{call}");
                }
                None => {
                    assert_eq!(converted.writing, body, "{call}");
                    let kind = match call.contains("@/") {
                        true => LINK_NOWHERE.heading,
                        false => TAG_LEFT.heading,
                    };
                    let found: Vec<&str> =
                        converted.findings.iter().map(|f| f.kind.heading).collect();
                    assert_eq!(found, [kind], "{call}");
                }
            }
            assert!(converted.calls.is_empty(), "{call}");
        }
    }

    #[test]
    fn a_function_called_in_code_is_run_there_too() {
        let body = "`{{ get_url(path=\"a.txt\") }}` and `{{ now() }}`\n\n{% end %}\n";
        let (writing, found) = converted(body, Reading::Template);
        assert_eq!(writing, "`/a.txt` and `{{ now() }}`\n\n{% end %}\n");
        assert_eq!(
            found,
            [
                (TAG_IN_CODE.heading, "{{ now() }}".into()),
                (TAG_LEFT.heading, "{% end %}".into()),
            ]
        );
    }

    #[test]
    fn a_heading_keeps_its_first_id_from_the_template_reader() {
        let text = "## A {#a} {#b}\n#### {#c}\n####### {#d}\nText {#e}\n    ## {#f}\n## {# g}";
        let ids: Vec<&str> = ["{#a}", "{#b}", "{#c}", "{#d}", "{#e}", "{#f}", "{# g}"]
            .into_iter()
            .filter(|id| heading_id(text, text.find(id).unwrap()))
            .collect();
        assert_eq!(ids, ["{#a}", "{#c}"]);
    }

    #[test]
    fn the_writing_and_templates_show_which_zola_made_a_site() {
        assert!(macros("{% import \"macros.html\" as m %}"));
        assert!(macros("{%- macro a() -%}{%- endmacro -%}"));
        assert!(!macros(
            "{% component a() %}{% endcomponent %} {% macros %}"
        ));
        let later = signs("{{ <a /> }}\n\n`{{/* a() */}}`\n", &[]);
        assert!(later.component && !later.function && !later.shown && !later.raw);
        // What `raw` holds is text whichever version read it.
        let earlier = signs("{{/* a() */}}\n\n{% raw %}{{ <a /> }}{% endraw %}\n", &[]);
        assert!(!earlier.component && earlier.shown && earlier.raw);
        // A function's name is the site's own shortcode where it has one.
        let call = "{{ now() }} {{ load_data(path=\"a\") }}\n";
        assert!(signs(call, &["now".into()]).function);
        assert!(!signs(call, &["now".into(), "load_data".into()]).function);
    }

    #[test]
    fn a_tag_opened_is_ended_past_others_opened_between() {
        let text = "{% a() %}x{% b() %}y{% end %}z{% end %}";
        let end = end_of(text, 9).unwrap();
        assert_eq!(&text[end.start..], "{% end %}");
        assert_eq!(paired(text), vec!["a".to_owned(), "b".to_owned()]);
    }
}
