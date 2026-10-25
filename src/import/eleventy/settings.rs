//! What Eleventy's settings file says, as far as its text shows it in the
//! plain forms most sites write. The file is a program, which Statac never
//! runs: it is read as words, quotes and marks, comments left out, and only
//! a value written out in quotes, or as `true` or `false`, is taken.

use std::path::Path;

use crate::import::inside;
use crate::source::Sources;
use crate::yaml::Value as Yaml;

/// The settings files Eleventy reads, the first found.
pub(super) const FILES: [&str; 4] = [
    ".eleventy.js",
    "eleventy.config.js",
    "eleventy.config.mjs",
    "eleventy.config.cjs",
];

/// Whether `folder` holds an Eleventy site: a settings file, or a
/// `package.json` that names Eleventy among what it needs.
pub(super) fn owns(folder: &Path) -> bool {
    FILES
        .iter()
        .any(|name| inside::a_settings_file(folder, name))
        || needs_eleventy(folder)
}

/// Whether the folder's `package.json` names `@11ty/eleventy` among its
/// dependencies.
fn needs_eleventy(folder: &Path) -> bool {
    needs(folder, &|name| name == "@11ty/eleventy")
}

/// Whether the folder's `package.json` names among its dependencies what
/// sets maths written between dollar signs: a package with KaTeX,
/// MathJax or TeX maths in its name.
pub(super) fn sets_maths(folder: &Path) -> bool {
    needs(folder, &|name| {
        let name = name.to_ascii_lowercase();
        ["katex", "mathjax", "texmath", "markdown-it-math"]
            .iter()
            .any(|maths| name.contains(maths))
    })
}

/// Whether the folder's `package.json` names among its dependencies
/// Eleventy's RSS plugin, whose feed templates give each post its full URL
/// as its id.
pub(super) fn makes_feeds(folder: &Path) -> bool {
    needs(folder, &|name| name == "@11ty/eleventy-plugin-rss")
}

/// Whether the folder's `package.json` names among its dependencies a
/// package `wanted` picks out.
fn needs(folder: &Path, wanted: &dyn Fn(&str) -> bool) -> bool {
    if !inside::a_settings_file(folder, "package.json") {
        return false;
    }
    let Ok(text) = std::fs::read_to_string(inside::path(folder, "package.json")) else {
        return false;
    };
    let mut sources = Sources::default();
    let source = sources.add("package.json", text);
    let Ok(node) = crate::yaml::read_json(&sources, source) else {
        return false;
    };
    let Yaml::Mapping(entries) = &node.value else {
        return false;
    };
    entries.iter().any(|entry| {
        matches!(&entry.key.value, Yaml::Scalar(key) if key == "dependencies" || key == "devDependencies")
            && matches!(&entry.value.value, Yaml::Mapping(needed)
                if needed.iter().any(|n| matches!(&n.key.value, Yaml::Scalar(name) if wanted(name))))
    })
}

/// A piece of the settings' text.
#[derive(Clone, PartialEq, Debug)]
enum Token {
    /// A name, or a word such as `true`.
    Word(String),
    /// Text in quotes, as it reads.
    Text(String),
    /// A number, or text in backquotes that holds `${`: something that
    /// isn't text written out.
    Other,
    Mark(char),
}

#[derive(Clone, Debug)]
struct Piece {
    token: Token,
    line: usize,
    /// Where it is in the text.
    start: usize,
    end: usize,
}

/// The settings' text as pieces, comments left out.
fn pieces(text: &str) -> Vec<Piece> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut at = 0;
    let mut line = 1;
    while at < bytes.len() {
        let c = bytes[at];
        let start = at;
        let start_line = line;
        let token = match c {
            b'\n' => {
                line += 1;
                at += 1;
                continue;
            }
            _ if c.is_ascii_whitespace() => {
                at += 1;
                continue;
            }
            b'/' if bytes.get(at + 1) == Some(&b'/') => {
                at = text[at..].find('\n').map_or(bytes.len(), |n| at + n);
                continue;
            }
            b'/' if bytes.get(at + 1) == Some(&b'*') => {
                let end = text[at + 2..]
                    .find("*/")
                    .map_or(bytes.len(), |n| at + 2 + n + 2);
                line += text[at..end].matches('\n').count();
                at = end;
                continue;
            }
            b'"' | b'\'' | b'`' => {
                let mut value = String::new();
                let mut i = at + 1;
                let mut closed = false;
                while let Some(ch) = text[i..].chars().next() {
                    i += ch.len_utf8();
                    match ch {
                        '\\' => {
                            if let Some(next) = text[i..].chars().next() {
                                i += next.len_utf8();
                                if next == '\n' {
                                    line += 1;
                                } else {
                                    value.push(next);
                                }
                            }
                        }
                        ch if ch == char::from(c) => {
                            closed = true;
                            break;
                        }
                        '\n' if c != b'`' => break,
                        ch => {
                            if ch == '\n' {
                                line += 1;
                            }
                            value.push(ch);
                        }
                    }
                }
                at = i;
                match closed && !(c == b'`' && value.contains("${")) {
                    true => Token::Text(value),
                    false => Token::Other,
                }
            }
            _ if c.is_ascii_alphabetic() || c == b'_' || c == b'$' => {
                let end = text[at..]
                    .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'))
                    .map_or(bytes.len(), |n| at + n);
                let word = text[at..end].to_owned();
                at = end;
                Token::Word(word)
            }
            _ if c.is_ascii_digit() => {
                let end = text[at..]
                    .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '.' || ch == '_'))
                    .map_or(bytes.len(), |n| at + n);
                at = end;
                Token::Other
            }
            _ => {
                let ch = text[at..].chars().next().unwrap_or(' ');
                at += ch.len_utf8();
                Token::Mark(ch)
            }
        };
        out.push(Piece {
            token,
            line: start_line,
            start,
            end: at,
        });
    }
    out
}

/// A value the settings give, with the line it is on.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct Given<T> {
    pub value: T,
    pub line: usize,
}

/// One of `dir`'s folders.
#[derive(Clone, PartialEq, Debug)]
pub(super) enum Folder {
    /// Written in quotes.
    Plain(Given<String>),
    /// Written some other way, such as by a name.
    Unplain(usize),
}

/// A file or folder `addPassthroughCopy` names, and where it goes in the
/// built site where the settings say.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct Copy {
    pub from: String,
    pub to: Option<String>,
    pub line: usize,
    /// The call as written, for a finding.
    pub written: String,
}

/// A feed the RSS plugin's virtual template makes, as an
/// `addPlugin(feedPlugin, {...})` call gives it.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct PluginFeed {
    /// `outputPath`, `/feed.xml` where the call gives none; none where it
    /// gives one some other way than text in quotes.
    pub path: Option<String>,
    /// `type`, `atom` where the call gives none; none where it gives one
    /// some other way than text in quotes.
    pub kind: Option<String>,
    pub line: usize,
    /// The call as the settings Statac didn't read show it.
    pub written: String,
}

/// What the settings' text shows.
#[derive(Default, Debug)]
pub(super) struct Settings {
    /// The settings file read, where there is one.
    pub file: Option<String>,
    /// Whether a `dir` was found at all.
    pub dir: bool,
    pub input: Option<Folder>,
    pub output: Option<Folder>,
    pub includes: Option<Folder>,
    pub data: Option<Folder>,
    pub layouts: Option<Folder>,
    pub copies: Vec<Copy>,
    pub breaks: Option<Given<bool>>,
    pub linkify: Option<Given<bool>>,
    pub html: Option<Given<bool>>,
    /// Where the settings give Markdown a library of their own, whose
    /// `html` is off unless they say.
    pub library: Option<usize>,
    /// The plugin for attributes in braces, by the name it is fetched by.
    pub attributes: Option<Given<String>>,
    pub separator: Option<Given<String>>,
    /// Where `excerpt: true` is, which with no separator ends an excerpt at
    /// the first `---` line.
    pub excerpt: Option<usize>,
    /// `markdownTemplateEngine`: a language's name, or `false` as `None`.
    pub engine: Option<Given<Option<String>>>,
    pub html_engine: Option<Given<Option<String>>>,
    /// Where `setUseGitIgnore(false)` is.
    pub no_gitignore: Option<usize>,
    /// `pathPrefix`, as written.
    pub path_prefix: Option<Given<String>>,
    /// The feeds `feedPlugin` makes, in order.
    pub feeds: Vec<PluginFeed>,
    /// What the settings do that Statac didn't read, by the names it
    /// shows, each with its line.
    pub unread: Vec<(usize, String)>,
    /// The files the settings fetch that Markdown's options were read
    /// in, where the settings file itself held none.
    pub markdown_in: Vec<String>,
}

impl Settings {
    /// The path the old site was published under, from `pathPrefix`, as
    /// `/blog`; empty at the top of its host.
    pub(super) fn base(&self) -> String {
        let written = self.path_prefix.as_ref().map(|given| given.value.trim());
        match written.map(|written| written.trim_matches('/')) {
            Some(path) if !path.is_empty() => format!("/{path}"),
            _ => String::new(),
        }
    }
}

/// The settings' calls Statac reads, or reads the plain forms of.
const READ_CALLS: &[&str] = &[
    "addPassthroughCopy",
    "setLibrary",
    "amendLibrary",
    "setFrontMatterParsingOptions",
    "setUseGitIgnore",
    "set",
];

/// Eleventy's own settings, returned or exported beside `dir`, which
/// Statac doesn't read.
const OTHER_SETTINGS: &[&str] = &[
    "templateFormats",
    "dataTemplateEngine",
    "passthroughFileCopy",
    "htmlOutputSuffix",
    "jsDataFileSuffix",
];

/// Reads the settings file `name` of `folder`, already found to be a file
/// reached through no link that leads outside the old site.
pub(super) fn read(folder: &Path, name: &str) -> std::io::Result<Settings> {
    let text = std::fs::read_to_string(inside::path(folder, name))?;
    let mut settings = scan(&text);
    settings.file = Some(name.to_owned());
    // A site can make Markdown's library in a file of its own that the
    // settings fetch, so each such file in the old folder is read for
    // Markdown's options alone, where none has been found yet.
    let mut waiting = fetched(&text, "");
    let mut seen = vec![name.to_owned()];
    while let Some(path) = waiting.pop() {
        if seen.len() > FETCHED {
            break;
        }
        if seen.contains(&path) || !inside::a_settings_file(folder, &path) {
            continue;
        }
        seen.push(path.clone());
        let Ok(text) = std::fs::read_to_string(inside::path(folder, &path)) else {
            continue;
        };
        let more = scan(&text);
        let mut took = false;
        let mut take = |ours: &mut Option<Given<bool>>, theirs: Option<Given<bool>>| {
            if ours.is_none() && theirs.is_some() {
                *ours = theirs;
                took = true;
            }
        };
        take(&mut settings.breaks, more.breaks);
        take(&mut settings.linkify, more.linkify);
        take(&mut settings.html, more.html);
        if settings.attributes.is_none() && more.attributes.is_some() {
            settings.attributes = more.attributes;
            took = true;
        }
        if took {
            settings.markdown_in.push(path.clone());
        }
        let dir = path.rsplit_once('/').map_or("", |(dir, _)| dir);
        waiting.extend(fetched(&text, dir));
    }
    Ok(settings)
}

/// How many files the settings fetch, and those fetch in turn, are read
/// for Markdown's options.
const FETCHED: usize = 16;

/// The files of the old site that `text`, a file in the folder `dir`,
/// fetches by a path from itself, as `import x from "./x.js"` or
/// `require("./x")`: each by its path in the old folder, with `.js` and
/// `/index.js` tried where it names no ending. One outside the old
/// folder is left out.
fn fetched(text: &str, dir: &str) -> Vec<String> {
    let pieces = pieces(text);
    let mut found = Vec::new();
    for (at, piece) in pieces.iter().enumerate() {
        let Token::Text(named) = &piece.token else {
            continue;
        };
        let before = |back: usize| at.checked_sub(back).map(|i| &pieces[i].token);
        let fetch = matches!(before(1), Some(Token::Word(word)) if word == "from" || word == "import")
            || (before(1) == Some(&Token::Mark('('))
                && matches!(before(2), Some(Token::Word(word)) if word == "require" || word == "import"));
        if !fetch || !(named.starts_with("./") || named.starts_with("../")) {
            continue;
        }
        let mut path: Vec<&str> = dir.split('/').filter(|piece| !piece.is_empty()).collect();
        let mut inside = true;
        for piece in named.split('/') {
            match piece {
                "" | "." => {}
                ".." => inside &= path.pop().is_some(),
                piece => path.push(piece),
            }
        }
        let path = path.join("/");
        if !inside || path.is_empty() || !crate::platform::stays_inside(&path) {
            continue;
        }
        match [".js", ".mjs", ".cjs"]
            .iter()
            .any(|ending| path.ends_with(ending))
        {
            true => found.push(path),
            false => {
                found.push(format!("{path}.js"));
                found.push(format!("{path}/index.js"));
            }
        }
    }
    found
}

/// What `text`, a settings file, shows.
fn scan(text: &str) -> Settings {
    let pieces = pieces(text);
    let mut s = Settings::default();
    let word = |at: usize| match pieces.get(at).map(|p| &p.token) {
        Some(Token::Word(word)) => Some(word.as_str()),
        _ => None,
    };
    let mark = |at: usize, c: char| pieces.get(at).is_some_and(|p| p.token == Token::Mark(c));
    let text_at = |at: usize| match pieces.get(at).map(|p| &p.token) {
        Some(Token::Text(text)) => Some(text.clone()),
        _ => None,
    };
    // A key: a name or text in quotes, then `:`.
    let key = |at: usize| -> Option<String> {
        let name = match pieces.get(at).map(|p| &p.token) {
            Some(Token::Word(word)) => word.clone(),
            Some(Token::Text(text)) => text.clone(),
            _ => return None,
        };
        let before_ok = at == 0 || mark(at - 1, '{') || mark(at - 1, ',');
        (before_ok && mark(at + 1, ':')).then_some(name)
    };
    // What a call or key shows of its first value: as written, up to the
    // next `,` or the bracket that closes it, on its first line.
    let shown_first = |at: usize| -> String {
        let mut depth = 0;
        let mut end = at;
        while let Some(piece) = pieces.get(end) {
            match piece.token {
                Token::Mark('(' | '{' | '[') => depth += 1,
                Token::Mark(')' | '}' | ']') if depth == 0 => break,
                Token::Mark(')' | '}' | ']') => depth -= 1,
                Token::Mark(',' | ';') if depth == 0 => break,
                _ => {}
            }
            end += 1;
        }
        match (
            pieces.get(at),
            end.checked_sub(1).and_then(|last| pieces.get(last)),
        ) {
            (Some(first), Some(last)) if end > at => {
                let written = &text[first.start..last.end];
                written
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .trim_end()
                    .to_owned()
            }
            _ => String::new(),
        }
    };
    let plugin_names = feed_plugin_names(&pieces);
    let mut at = 0;
    while at < pieces.len() {
        let line = pieces[at].line;
        if let Some(name) = key(at) {
            let value = at + 2;
            match name.as_str() {
                "dir" if mark(value, '{') && !s.dir => {
                    s.dir = true;
                    at = dir(&pieces, value + 1, &mut s);
                    continue;
                }
                "dir" if !s.dir => {
                    s.dir = true;
                    s.input = Some(Folder::Unplain(line));
                    s.output = Some(Folder::Unplain(line));
                    s.includes = Some(Folder::Unplain(line));
                    s.data = Some(Folder::Unplain(line));
                }
                "breaks" | "linkify" | "html" => {
                    if let Some(truth) = word(value).filter(|w| matches!(*w, "true" | "false")) {
                        let given = Some(Given {
                            value: truth == "true",
                            line,
                        });
                        match name.as_str() {
                            "breaks" => s.breaks = s.breaks.take().or(given),
                            "linkify" => s.linkify = s.linkify.take().or(given),
                            _ => s.html = s.html.take().or(given),
                        }
                    }
                }
                "excerpt_separator" => {
                    if let Some(separator) = text_at(value) {
                        s.separator.get_or_insert(Given {
                            value: separator,
                            line,
                        });
                    }
                }
                "excerpt" if word(value) == Some("true") => {
                    s.excerpt.get_or_insert(line);
                }
                "markdownTemplateEngine" | "htmlTemplateEngine" => {
                    let given = match (text_at(value), word(value)) {
                        (Some(engine), _) => Some(Some(engine)),
                        (None, Some("false")) => Some(None),
                        _ => None,
                    };
                    let slot = match name.as_str() {
                        "markdownTemplateEngine" => &mut s.engine,
                        _ => &mut s.html_engine,
                    };
                    match given {
                        Some(value) => {
                            slot.get_or_insert(Given { value, line });
                        }
                        None => s
                            .unread
                            .push((line, format!("{name}: {}", shown_first(value)))),
                    }
                }
                "pathPrefix" => {
                    let ended = (pieces.get(value + 1))
                        .is_none_or(|p| matches!(p.token, Token::Mark(',' | '}' | ';')));
                    let plain = text_at(value)
                        .or_else(|| constant(&pieces, word(value)?))
                        .filter(|_| ended);
                    match plain {
                        Some(value) => {
                            s.path_prefix.get_or_insert(Given { value, line });
                        }
                        None => s
                            .unread
                            .push((line, format!("{name}: {}", shown_first(value)))),
                    }
                }
                other if OTHER_SETTINGS.contains(&other) => {
                    s.unread
                        .push((line, format!("{other}: {}", shown_first(value))));
                }
                _ => {}
            }
            at += 1;
            continue;
        }
        match &pieces[at].token {
            Token::Text(fetched)
                if fetched.contains("markdown-it-attrs") || fetched.contains("plugin-attrs") =>
            {
                s.attributes.get_or_insert(Given {
                    value: fetched.clone(),
                    line,
                });
            }
            Token::Word(fetch) if fetch == "require" || fetch == "import" => {
                // The piece naming what is fetched.
                let named = match (mark(at + 1, '('), text_at(at + 2)) {
                    (true, Some(_)) => Some(at + 2),
                    _ => (at + 1..pieces.len().min(at + 40))
                        .take_while(|i| !mark(*i, ';'))
                        .find(|i| word(*i) == Some("from") && text_at(i + 1).is_some())
                        .map(|i| i + 1)
                        .or_else(|| text_at(at + 1).map(|_| at + 1)),
                };
                if let Some(named) = named
                    && text_at(named).is_some_and(|path| path.starts_with('.'))
                {
                    let end = match mark(named + 1, ')') {
                        true => pieces[named + 1].end,
                        false => pieces[named].end,
                    };
                    s.unread
                        .push((line, text[pieces[at].start..end].to_owned()));
                }
            }
            Token::Word(call) if at > 0 && mark(at - 1, '.') && mark(at + 1, '(') => {
                let first = at + 2;
                match call.as_str() {
                    "addPassthroughCopy" => passthrough(&pieces, text, first, &mut s),
                    "setLibrary" if text_at(first).as_deref() == Some("md") => {
                        s.library.get_or_insert(line);
                    }
                    "setUseGitIgnore" if word(first) == Some("false") => {
                        s.no_gitignore.get_or_insert(line);
                    }
                    "use" => {
                        let mut depth = 0;
                        let end = (first..pieces.len())
                            .find(|i| match pieces[*i].token {
                                Token::Mark('(' | '{' | '[') => {
                                    depth += 1;
                                    false
                                }
                                Token::Mark(')') if depth == 0 => true,
                                Token::Mark(')' | '}' | ']') => {
                                    depth -= 1;
                                    false
                                }
                                _ => false,
                            })
                            .unwrap_or(pieces.len());
                        let used = match (pieces.get(first), pieces.get(end - 1)) {
                            (Some(a), Some(b)) if end > first => &text[a.start..b.end],
                            _ => "",
                        };
                        if !used.to_ascii_lowercase().contains("attrs") {
                            s.unread
                                .push((line, format!("use({})", shown_first(first))));
                        }
                    }
                    "addPlugin" if names_feed_plugin(&pieces, first, &plugin_names) => {
                        let written = format!("addPlugin({})", shown_first(first));
                        let feed = plugin_feed(&pieces, first, line, written);
                        if feed.path.is_none() {
                            s.unread.push((line, feed.written.clone()));
                        }
                        s.feeds.push(feed);
                    }
                    name if READ_CALLS.contains(&name) => {}
                    name if name.starts_with("add")
                        || name.starts_with("set")
                        || name.starts_with("amend")
                        || name == "on" =>
                    {
                        s.unread
                            .push((line, format!("{name}({})", shown_first(first))));
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        at += 1;
    }
    s
}

/// Reads `dir`'s folders from `at`, just inside its `{`, and gives where
/// its `}` ends.
fn dir(pieces: &[Piece], mut at: usize, s: &mut Settings) -> usize {
    while at < pieces.len() {
        let piece = &pieces[at];
        if piece.token == Token::Mark('}') {
            return at + 1;
        }
        let name = match &piece.token {
            Token::Word(word) => word.clone(),
            Token::Text(text) => text.clone(),
            _ => {
                at += 1;
                continue;
            }
        };
        let ended = |at: usize| {
            pieces
                .get(at)
                .is_some_and(|p| matches!(p.token, Token::Mark(',' | '}')))
        };
        // `input,` alone is short for `input: input`.
        let short = matches!(piece.token, Token::Word(_)) && ended(at + 1);
        if !short && pieces.get(at + 1).map(|p| &p.token) != Some(&Token::Mark(':')) {
            at += 1;
            continue;
        }
        let value = match short {
            true => Some(&piece.token),
            false => pieces
                .get(at + 2)
                .map(|p| &p.token)
                .filter(|_| ended(at + 3)),
        };
        let plain = match value {
            Some(Token::Text(text)) => Some(text.clone()),
            Some(Token::Word(word)) => constant(pieces, word),
            _ => None,
        };
        let folder = match plain {
            Some(value) => Folder::Plain(Given {
                value,
                line: piece.line,
            }),
            None => Folder::Unplain(piece.line),
        };
        let slot = match name.as_str() {
            "input" => &mut s.input,
            "output" => &mut s.output,
            "includes" => &mut s.includes,
            "data" => &mut s.data,
            "layouts" => &mut s.layouts,
            _ => {
                at += 1;
                continue;
            }
        };
        *slot = Some(folder);
        // Past the value, as far as the next `,` or `}` at this depth.
        let mut depth = 0;
        at += if short { 1 } else { 2 };
        while let Some(p) = pieces.get(at) {
            match p.token {
                Token::Mark('{' | '(' | '[') => depth += 1,
                Token::Mark('}' | ')' | ']') if depth > 0 => depth -= 1,
                Token::Mark(',') if depth == 0 => {
                    at += 1;
                    break;
                }
                Token::Mark('}') if depth == 0 => break,
                _ => {}
            }
            at += 1;
        }
    }
    at
}

/// The text the settings give the name `name`, where they say
/// `const name = "text"` once and nowhere else, which nothing after can
/// change.
fn constant(pieces: &[Piece], name: &str) -> Option<String> {
    let mut said = pieces.windows(4).filter_map(|four| {
        match (
            &four[0].token,
            &four[1].token,
            &four[2].token,
            &four[3].token,
        ) {
            (Token::Word(word), Token::Word(named), Token::Mark('='), value)
                if matches!(word.as_str(), "const" | "let" | "var") && named == name =>
            {
                Some((word.as_str(), value, four[3].line))
            }
            _ => None,
        }
    });
    let (word, value, line) = said.next()?;
    if said.next().is_some() || word != "const" {
        return None;
    }
    let Token::Text(text) = value else {
        return None;
    };
    // More on the line after the text, as `"a" + b`, makes it something
    // else.
    let after = pieces
        .iter()
        .skip_while(|p| !std::ptr::eq(&p.token, value))
        .nth(1);
    match after {
        Some(p) if p.line == line && p.token != Token::Mark(';') => None,
        _ => Some(text.clone()),
    }
}

/// The names the settings give the RSS plugin's `feedPlugin`: its own, and
/// any it is fetched as, as `import { feedPlugin as feed }` or
/// `const { feedPlugin: feed } = require(...)`.
fn feed_plugin_names(pieces: &[Piece]) -> Vec<String> {
    let mut names = vec!["feedPlugin".to_owned()];
    for three in pieces.windows(3) {
        match (&three[0].token, &three[1].token, &three[2].token) {
            (Token::Word(name), Token::Word(word), Token::Word(alias))
                if name == "feedPlugin" && word == "as" =>
            {
                names.push(alias.clone());
            }
            (Token::Word(name), Token::Mark(':'), Token::Word(alias)) if name == "feedPlugin" => {
                names.push(alias.clone());
            }
            _ => {}
        }
    }
    names
}

/// Where the value of a call that starts at `at` ends: at the `,` or `)`
/// after it.
fn value_end(pieces: &[Piece], mut at: usize) -> usize {
    let mut depth = 0;
    while let Some(piece) = pieces.get(at) {
        match piece.token {
            Token::Mark(',' | ')') if depth == 0 => break,
            Token::Mark('(' | '{' | '[') => depth += 1,
            Token::Mark(')' | '}' | ']') => depth -= 1,
            _ => {}
        }
        at += 1;
    }
    at
}

/// Whether the `addPlugin` call whose first value is at `at` adds
/// `feedPlugin`, by one of `names` or as `pluginRss.feedPlugin`.
fn names_feed_plugin(pieces: &[Piece], at: usize, names: &[String]) -> bool {
    let end = value_end(pieces, at);
    match pieces.get(at..end).unwrap_or_default() {
        [one] => matches!(&one.token, Token::Word(word) if names.contains(word)),
        [.., dot, last] => {
            dot.token == Token::Mark('.')
                && matches!(&last.token, Token::Word(word) if word == "feedPlugin")
        }
        [] => false,
    }
}

/// The feed an `addPlugin(feedPlugin, ...)` call makes, its first value at
/// `at`.
fn plugin_feed(pieces: &[Piece], at: usize, line: usize, written: String) -> PluginFeed {
    let end = value_end(pieces, at);
    let token = |i: usize| pieces.get(i).map(|p| &p.token);
    let (path, kind) = match (token(end), token(end + 1)) {
        (Some(Token::Mark(',')), Some(Token::Mark('{'))) => (
            option(pieces, end + 2, "outputPath"),
            option(pieces, end + 2, "type"),
        ),
        (Some(Token::Mark(',')), Some(Token::Mark(')'))) | (Some(Token::Mark(')')), _) => {
            (None, None)
        }
        _ => (Some(None), Some(None)),
    };
    PluginFeed {
        path: path.unwrap_or_else(|| Some("/feed.xml".to_owned())),
        kind: kind.unwrap_or_else(|| Some("atom".to_owned())),
        line,
        written,
    }
}

/// The option `name` of an object whose first key is at `at`: none where
/// it isn't given, and its text where it is given as text in quotes, or
/// by a name said once as that; where something else gives it, or a
/// `...` might, none within.
fn option(pieces: &[Piece], mut at: usize, name: &str) -> Option<Option<String>> {
    let mark = |i: usize, c: char| pieces.get(i).is_some_and(|p| p.token == Token::Mark(c));
    let ends = |i: usize| mark(i, ',') || mark(i, '}');
    let mut given = None;
    let mut depth = 0;
    let mut key_here = true;
    while let Some(piece) = pieces.get(at) {
        if depth == 0 && key_here {
            if (0..3).all(|i| mark(at + i, '.')) {
                given = Some(None);
            }
            if let Token::Word(key) | Token::Text(key) = &piece.token
                && key == name
            {
                let value = match (mark(at + 1, ':'), &piece.token) {
                    (true, _) => match pieces.get(at + 2).map(|p| &p.token) {
                        Some(Token::Text(text)) if ends(at + 3) => Some(text.clone()),
                        Some(Token::Word(word)) if ends(at + 3) => constant(pieces, word),
                        _ => None,
                    },
                    // `outputPath,` alone is short for `outputPath: outputPath`.
                    (false, Token::Word(word)) if ends(at + 1) => constant(pieces, word),
                    _ => None,
                };
                given = Some(value);
            }
        }
        match piece.token {
            Token::Mark('{' | '(' | '[') => depth += 1,
            Token::Mark('}' | ')' | ']') if depth == 0 => break,
            Token::Mark('}' | ')' | ']') => depth -= 1,
            _ => {}
        }
        key_here = depth == 0 && piece.token == Token::Mark(',');
        at += 1;
    }
    given
}

/// Reads `addPassthroughCopy(` from its first value at `at`: text in
/// quotes, or an object of texts; anything else is told as not read.
fn passthrough(pieces: &[Piece], text: &str, at: usize, s: &mut Settings) {
    let line = pieces.get(at - 2).map_or(1, |p| p.line);
    let close = |from: usize| {
        let mut depth = 0;
        for (i, piece) in pieces.iter().enumerate().skip(from) {
            match piece.token {
                Token::Mark('(' | '{' | '[') => depth += 1,
                Token::Mark(')') if depth == 0 => return i,
                Token::Mark(')' | '}' | ']') => depth -= 1,
                _ => {}
            }
        }
        pieces.len()
    };
    let end = close(at);
    let written = match (pieces.get(at - 2), pieces.get(end)) {
        (Some(first), Some(last)) => text[first.start..last.end].to_owned(),
        _ => "addPassthroughCopy(...)".to_owned(),
    };
    let token = |i: usize| pieces.get(i).map(|p| &p.token);
    let ends = |i: usize| matches!(token(i), Some(Token::Mark(')' | ',')));
    if let Some(Token::Text(from)) = token(at)
        && ends(at + 1)
    {
        s.copies.push(Copy {
            from: from.clone(),
            to: None,
            line,
            written,
        });
        return;
    }
    if token(at) == Some(&Token::Mark('{')) {
        let mut found = Vec::new();
        let mut i = at + 1;
        let plain = loop {
            match (token(i), token(i + 1), token(i + 2), token(i + 3)) {
                (
                    Some(Token::Text(from)),
                    Some(Token::Mark(':')),
                    Some(Token::Text(to)),
                    Some(Token::Mark(after @ (',' | '}'))),
                ) => {
                    found.push((from.clone(), to.clone()));
                    i += 4;
                    if *after == '}' {
                        break true;
                    }
                    if token(i) == Some(&Token::Mark('}')) {
                        i += 1;
                        break true;
                    }
                }
                _ => break false,
            }
        };
        if plain && ends(i) && !found.is_empty() {
            for (from, to) in found {
                s.copies.push(Copy {
                    from,
                    to: Some(to),
                    line,
                    written: written.clone(),
                });
            }
            return;
        }
    }
    s.unread.push((line, written));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn files_fetched_by_a_path_from_the_file_are_found_inside_the_old_folder() {
        let text = "import a from './a.js';\nimport { b } from \"../b\";\n\
                    const c = require('./lib/c');\nimport 'package';\n\
                    import d from '../../d.js';\nconst e = './e.js';\n";
        assert_eq!(
            fetched(text, "src/config"),
            [
                "src/config/a.js",
                "src/b.js",
                "src/b/index.js",
                "src/config/lib/c.js",
                "src/config/lib/c/index.js",
                "d.js"
            ]
        );
        assert_eq!(fetched("import d from '../d.js';", ""), [] as [&str; 0]);
    }

    #[test]
    fn a_folder_given_by_a_name_said_once_is_read() {
        let s = scan(
            "const input = \"src\";\n\
             const output = 'dist'\n\
             const data = \"a\" + \"b\";\n\
             let includes = \"parts\";\n\
             export const config = { dir: { input, output: output, data, includes } };\n",
        );
        let plain = |value: &str| {
            Some(Folder::Plain(Given {
                value: value.into(),
                line: 5,
            }))
        };
        assert_eq!(s.input, plain("src"));
        assert_eq!(s.output, plain("dist"));
        assert_eq!(s.data, Some(Folder::Unplain(5)));
        assert_eq!(s.includes, Some(Folder::Unplain(5)));
        let twice = scan(
            "const input = \"src\";\n\
             function other() { const input = \"lib\"; }\n\
             export const config = { dir: { input } };\n",
        );
        assert_eq!(twice.input, Some(Folder::Unplain(3)));
    }

    #[test]
    fn the_path_a_site_is_under_is_read_where_it_is_written_out() {
        let base = |text: &str| {
            let s = scan(text);
            (s.base(), s.unread)
        };
        for written in ["\"/blog/\"", "'blog'", "`/blog`", "\" /blog/ \""] {
            let text = format!("module.exports = {{ pathPrefix: {written}, }};");
            assert_eq!(base(&text), ("/blog".to_owned(), vec![]), "{written}");
        }
        let named = "const prefix = '/blog/site/';\nexport const config = { pathPrefix: prefix };";
        assert_eq!(base(named), ("/blog/site".to_owned(), vec![]));
        // At the top of its host it is under none.
        for top in ["\"/\"", "''"] {
            let text = format!("return {{ pathPrefix: {top} }};");
            assert_eq!(base(&text), (String::new(), vec![]), "{top}");
        }
        // Written any other way it isn't read, and is told of.
        for (other, shown) in [
            ("process.env.PREFIX", "pathPrefix: process.env.PREFIX"),
            ("\"/blog\" + suffix", "pathPrefix: \"/blog\" + suffix"),
        ] {
            let text = format!("return {{ pathPrefix: {other} }};");
            assert_eq!(base(&text), (String::new(), vec![(1, shown.to_owned())]));
        }
    }

    #[test]
    fn the_feeds_the_rss_plugin_makes_are_read_where_written_out() {
        let esm = "import { feedPlugin } from \"@11ty/eleventy-plugin-rss\";\n\
                   export default function (eleventyConfig) {\n";
        let cjs = "const pluginRss = require(\"@11ty/eleventy-plugin-rss\");\n\
                   module.exports = function (eleventyConfig) {\n";
        let renamed = "import { feedPlugin as feed } from \"@11ty/eleventy-plugin-rss\";\n\
                       const path = \"/atom.xml\";\n\
                       export default function (eleventyConfig) {\n";
        // (what the settings open with, the call, each feed's
        // `outputPath` and `type`, whether the call is told as not read)
        type Feed<'a> = (Option<&'a str>, Option<&'a str>);
        let cases: [(&str, &str, &[Feed], bool); 9] = [
            (
                esm,
                "eleventyConfig.addPlugin(feedPlugin, {\n  type: \"atom\",\n  \
                 outputPath: \"/feed/feed.xml\",\n  collection: { name: \"posts\", limit: 10 },\n});",
                &[(Some("/feed/feed.xml"), Some("atom"))],
                false,
            ),
            (
                esm,
                "eleventyConfig.addPlugin(feedPlugin);",
                &[(Some("/feed.xml"), Some("atom"))],
                false,
            ),
            (
                cjs,
                "eleventyConfig.addPlugin(pluginRss.feedPlugin, { type: 'json', outputPath: 'feed.json' });",
                &[(Some("feed.json"), Some("json"))],
                false,
            ),
            (
                renamed,
                "eleventyConfig.addPlugin(feed, { outputPath: path, metadata: { type: \"x\" } });",
                &[(Some("/atom.xml"), Some("atom"))],
                false,
            ),
            (
                esm,
                "eleventyConfig.addPlugin(feedPlugin, { outputPath: `/${name}.xml`, type });",
                &[(None, None)],
                true,
            ),
            (
                esm,
                "eleventyConfig.addPlugin(feedPlugin, options);",
                &[(None, None)],
                true,
            ),
            (
                esm,
                "eleventyConfig.addPlugin(feedPlugin, { ...shared, outputPath: \"/a.xml\" });",
                &[(Some("/a.xml"), None)],
                false,
            ),
            (
                esm,
                "eleventyConfig.addPlugin(feedPlugin, { type: \"rss\", outputPath: \"/rss.xml\" });\n\
                 eleventyConfig.addPlugin(feedPlugin, { type: \"json\", outputPath: \"/feed.json\" });",
                &[
                    (Some("/rss.xml"), Some("rss")),
                    (Some("/feed.json"), Some("json")),
                ],
                false,
            ),
            (cjs, "eleventyConfig.addPlugin(pluginRss);", &[], true),
        ];
        for (opening, call, feeds, unread) in cases {
            let s = scan(&format!("{opening}  {call}\n}};\n"));
            let found: Vec<(Option<&str>, Option<&str>)> = (s.feeds.iter())
                .map(|feed| (feed.path.as_deref(), feed.kind.as_deref()))
                .collect();
            assert_eq!(found, feeds, "{call}");
            assert_eq!(!s.unread.is_empty(), unread, "{call}: {:?}", s.unread);
        }
    }

    #[test]
    fn the_plain_forms_are_read_in_either_quotes() {
        let s = scan(
            "// dir: { input: \"no\" }\n\
             module.exports = function (eleventyConfig) {\n\
             \x20 eleventyConfig.addPassthroughCopy('img');\n\
             \x20 eleventyConfig.addPassthroughCopy({ \"src/fonts\": \"fonts\" });\n\
             \x20 eleventyConfig.addFilter(\"readable\", (d) => d);\n\
             \x20 eleventyConfig.setLibrary(\"md\", markdownIt({ html: true, breaks: true }).use(require('markdown-it-attrs')));\n\
             \x20 return { dir: { input: 'src', output: \"dist\", data: someName }, markdownTemplateEngine: false };\n\
             };\n",
        );
        assert_eq!(
            s.input,
            Some(Folder::Plain(Given {
                value: "src".into(),
                line: 7
            }))
        );
        assert_eq!(
            s.output,
            Some(Folder::Plain(Given {
                value: "dist".into(),
                line: 7
            }))
        );
        assert_eq!(s.data, Some(Folder::Unplain(7)));
        assert_eq!(s.copies.len(), 2);
        assert_eq!(s.copies[0].from, "img");
        assert_eq!(s.copies[1].to.as_deref(), Some("fonts"));
        assert_eq!(s.breaks.map(|b| b.value), Some(true));
        assert_eq!(s.html.map(|b| b.value), Some(true));
        assert!(s.linkify.is_none());
        assert_eq!(s.library, Some(6));
        assert_eq!(
            s.attributes.map(|a| a.value).as_deref(),
            Some("markdown-it-attrs")
        );
        assert_eq!(s.engine.map(|e| e.value), Some(None));
        assert_eq!(s.unread, vec![(5, "addFilter(\"readable\")".to_owned())]);
    }
}
