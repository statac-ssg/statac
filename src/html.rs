//! HTML read as a browser's tokenizer reads it, far enough to find its
//! tags and their attributes: comments, and the text of elements such as
//! `<script>` and `<textarea>`, hold no markup. And text escaped to stand
//! in HTML.

use std::fmt;
use std::ops::Range;

/// A start or end tag as the tokenizer reads it.
pub(crate) struct Tag {
    /// In small letters.
    pub(crate) name: String,
    pub(crate) attributes: Vec<Attribute>,
    /// Where the tag ends, past its `>`.
    pub(crate) end: usize,
    /// Whether the tag ran past the longest it was read to before it ended.
    pub(crate) cut: bool,
}

pub(crate) struct Attribute {
    /// In small letters.
    pub(crate) name: String,
    /// As written, its character references undecoded.
    pub(crate) value: String,
    /// Where the value is written, inside any quotes.
    pub(crate) at: Range<usize>,
}

impl Tag {
    /// The value of the attribute `name`, as written.
    pub(crate) fn get(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|attribute| attribute.name == name)
            .map(|attribute| attribute.value.as_str())
    }
}

/// Elements whose text holds no markup, up to their end tag.
pub(crate) const RAW_TEXT: &[&str] = &[
    "style", "xmp", "iframe", "noembed", "noframes", "noscript", "title", "textarea",
];

/// Elements whose text a browser shows with its spaces and line breaks.
pub(crate) const PREFORMATTED: &[&str] = &["listing", "plaintext", "pre", "textarea", "xmp"];

/// Elements that hold nothing, so are never left open.
pub(crate) const VOID: &[&str] = &[
    "area", "base", "basefont", "bgsound", "br", "col", "embed", "frame", "hr", "image", "img",
    "input", "keygen", "link", "meta", "param", "source", "track", "wbr",
];

/// Calls `each` with every start tag in `html`, read as a browser reads an
/// HTML document.
pub(crate) fn start_tags(html: &str, mut each: impl FnMut(&Tag)) {
    for (_, piece) in markup(html) {
        if let Markup::Start(tag) = piece {
            each(&tag);
        }
    }
}

/// A piece of markup as the tokenizer reads it.
pub(crate) enum Markup {
    Start(Tag),
    End(Tag),
    /// A comment, a doctype or the like, which holds nothing.
    Other,
}

/// Each piece of markup in `html` with where it is written, read as a
/// browser reads an HTML document. What stands between them is text.
pub(crate) fn markup(html: &str) -> impl Iterator<Item = (Range<usize>, Markup)> + '_ {
    pieces(html, false)
}

/// Each piece of markup in `html` as [`markup`] reads it, except that an
/// element of raw text, such as `<iframe>`, whose end tag never comes is
/// read as holding markup. A browser reads the rest of the document as its
/// text; where that decides spaces, an author's `<iframe>` named in the
/// words is likelier than one meant to swallow the page.
pub(crate) fn markup_where_closed(html: &str) -> impl Iterator<Item = (Range<usize>, Markup)> + '_ {
    pieces(html, true)
}

fn pieces(html: &str, closed: bool) -> impl Iterator<Item = (Range<usize>, Markup)> + '_ {
    let b = html.as_bytes();
    let mut at = 0;
    std::iter::from_fn(move || {
        loop {
            let lt = next(b, b'<', at)?;
            let rest = &b[lt..];
            let (end, piece) = if rest.starts_with(b"<!--") {
                (comment_end(b, lt + 4), Markup::Other)
            } else if rest.starts_with(b"<!") || rest.starts_with(b"<?") {
                let end = next(b, b'>', lt + 2).map_or(b.len(), |gt| gt + 1);
                (end, Markup::Other)
            } else if rest.starts_with(b"</") {
                match b.get(lt + 2) {
                    Some(c) if c.is_ascii_alphabetic() => {
                        let tag = tag(html, lt + 2, usize::MAX);
                        (tag.end, Markup::End(tag))
                    }
                    Some(_) => {
                        let end = next(b, b'>', lt + 2).map_or(b.len(), |gt| gt + 1);
                        (end, Markup::Other)
                    }
                    None => return None,
                }
            } else if b.get(lt + 1).is_some_and(u8::is_ascii_alphabetic) {
                let tag = tag(html, lt + 1, usize::MAX);
                let end = tag.end;
                at = match tag.name.as_str() {
                    "plaintext" => b.len(),
                    name if name == "script" || RAW_TEXT.contains(&name) => {
                        let unclosed = if closed { end } else { b.len() };
                        end_tag(b, end, name.as_bytes()).unwrap_or(unclosed)
                    }
                    _ => end,
                };
                return Some((lt..end, Markup::Start(tag)));
            } else {
                at = lt + 1;
                continue;
            };
            at = end;
            return Some((lt..end, piece));
        }
    })
}

/// Reads the tag whose name starts at `at`, as the tokenizer does: its
/// name to whitespace, `/` or `>`, then its attributes, each value quoted
/// or not. `longest` bounds how far it reads.
pub(crate) fn tag(html: &str, at: usize, longest: usize) -> Tag {
    let b = html.as_bytes();
    let limit = b.len().min(at.saturating_add(longest));
    let mut i = at;
    while i < limit && !ends_name(b[i]) {
        i += 1;
    }
    let name = html[at..i].to_ascii_lowercase();
    let mut attributes = Vec::new();
    loop {
        while i < limit && (is_space(b[i]) || b[i] == b'/') {
            i += 1;
        }
        if i >= limit {
            return Tag {
                name,
                attributes,
                end: i,
                cut: i < b.len(),
            };
        }
        if b[i] == b'>' {
            return Tag {
                name,
                attributes,
                end: i + 1,
                cut: false,
            };
        }
        // A name may open with `=`; after that, `=` ends it.
        let start = i;
        i += 1;
        while i < limit && !is_space(b[i]) && !matches!(b[i], b'/' | b'>' | b'=') {
            i += 1;
        }
        let name = html[start..i].to_ascii_lowercase();
        let mut after = i;
        while after < limit && is_space(b[after]) {
            after += 1;
        }
        if after >= limit || b[after] != b'=' {
            attributes.push(Attribute {
                name,
                value: String::new(),
                at: i..i,
            });
            i = after;
            continue;
        }
        i = after + 1;
        while i < limit && is_space(b[i]) {
            i += 1;
        }
        let at = match b.get(i).filter(|_| i < limit) {
            Some(&quote @ (b'"' | b'\'')) => {
                let from = i + 1;
                let to = next(&b[..limit], quote, from).unwrap_or(limit);
                i = (to + 1).min(limit);
                from..to
            }
            _ => {
                let from = i;
                while i < limit && !is_space(b[i]) && b[i] != b'>' {
                    i += 1;
                }
                from..i
            }
        };
        attributes.push(Attribute {
            name,
            value: html[at.clone()].to_owned(),
            at,
        });
    }
}

/// Where a comment opened at `at`, just past its `<!--`, ends: at once for
/// `<!-->` and `<!--->`, otherwise past the first `-->` or `--!>`.
pub(crate) fn comment_end(b: &[u8], at: usize) -> usize {
    let rest = &b[at.min(b.len())..];
    if rest.starts_with(b">") {
        return at + 1;
    }
    if rest.starts_with(b"->") {
        return at + 2;
    }
    comment_close(b, at).unwrap_or(b.len())
}

/// Where a comment's text from `at` ends: past the first `-->` or `--!>`.
/// None where neither follows.
pub(crate) fn comment_close(b: &[u8], at: usize) -> Option<usize> {
    let mut i = at;
    while let Some(dash) = next(b, b'-', i) {
        let after = &b[dash..];
        if after.starts_with(b"-->") {
            return Some(dash + 3);
        }
        if after.starts_with(b"--!>") {
            return Some(dash + 4);
        }
        i = dash + 1;
    }
    None
}

/// Where the end tag of the element `name` starts, from `at`: `</` and
/// the name in any case, then whitespace, `/` or `>`.
pub(crate) fn end_tag(b: &[u8], at: usize, name: &[u8]) -> Option<usize> {
    let mut i = at;
    while let Some(lt) = next(b, b'<', i) {
        let end = lt + 2 + name.len();
        if b.get(lt + 1) == Some(&b'/')
            && b.len() >= end
            && b[lt + 2..end].eq_ignore_ascii_case(name)
            && b.get(end).is_none_or(|&c| ends_name(c))
        {
            return Some(lt);
        }
        i = lt + 1;
    }
    None
}

/// The addresses an attribute's value holds, as written, each with where
/// it starts counted from `at`: the value itself, or each address a
/// `srcset` lists. An empty one links nowhere new.
pub(crate) fn addresses(value: &str, srcset: bool, at: usize) -> Vec<(&str, usize)> {
    if !srcset {
        let trimmed = value.trim_start_matches(|c: char| c.is_ascii_whitespace());
        let start = at + value.len() - trimmed.len();
        let trimmed = trimmed.trim_end_matches(|c: char| c.is_ascii_whitespace());
        return match trimmed.is_empty() {
            true => Vec::new(),
            false => vec![(trimmed, start)],
        };
    }
    // Each candidate is an address, up to whitespace, and what describes
    // it, up to a comma; an address can hold commas of its own.
    let b = value.as_bytes();
    let mut found = Vec::new();
    let mut i = 0;
    while i < b.len() {
        while i < b.len() && (b[i].is_ascii_whitespace() || b[i] == b',') {
            i += 1;
        }
        let start = i;
        while i < b.len() && !b[i].is_ascii_whitespace() {
            i += 1;
        }
        let mut address = &value[start..i];
        if address.ends_with(',') {
            address = address.trim_end_matches(',');
        } else {
            let mut depth = 0usize;
            while i < b.len() && !(b[i] == b',' && depth == 0) {
                match b[i] {
                    b'(' => depth += 1,
                    b')' => depth = depth.saturating_sub(1),
                    _ => {}
                }
                i += 1;
            }
        }
        if !address.is_empty() {
            found.push((address, at + start));
        }
    }
    found
}

/// Writes `text` into `out` to stand between tags or in an attribute's
/// double quotes, with `&`, `<`, `>` and `"` escaped.
pub(crate) fn escape(text: &str, out: &mut String) {
    escape_each(text, &['&', '<', '>', '"'], out).expect("writing to a string can't fail");
}

/// `text` as [`escape`] writes it.
pub(crate) fn escaped(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    escape(text, &mut out);
    out
}

/// Writes `text` into `out` to stand between tags or in an attribute in
/// either quotes, with `&`, `<`, `>`, `"` and `'` escaped and nothing
/// else: an address keeps its slashes.
pub(crate) fn escape_anywhere(text: &str, out: &mut impl fmt::Write) -> fmt::Result {
    escape_each(text, &['&', '<', '>', '"', '\''], out)
}

fn escape_each(text: &str, marks: &[char], out: &mut impl fmt::Write) -> fmt::Result {
    let mut rest = text;
    while let Some(at) = rest.find(marks) {
        out.write_str(&rest[..at])?;
        out.write_str(match rest.as_bytes()[at] {
            b'&' => "&amp;",
            b'<' => "&lt;",
            b'>' => "&gt;",
            b'"' => "&quot;",
            _ => "&#x27;",
        })?;
        rest = &rest[at + 1..];
    }
    out.write_str(rest)
}

/// Stands for a character reference Statac can't decode.
pub(crate) const ANY: char = '\u{FFFF}';

/// Named character references that can spell an address's scheme or what
/// comes before it, each as HTML spells it: a name's case is part of it,
/// `&Lt;` being `≪`. Others are kept as written, or with `any`, read as
/// `ANY`.
const NAMED: &[(&str, char)] = &[
    ("amp", '&'),
    ("AMP", '&'),
    ("lt", '<'),
    ("LT", '<'),
    ("gt", '>'),
    ("GT", '>'),
    ("quot", '"'),
    ("QUOT", '"'),
    ("apos", '\''),
    ("colon", ':'),
    ("semi", ';'),
    ("equals", '='),
    ("Tab", '\t'),
    ("NewLine", '\n'),
    ("nbsp", '\u{a0}'),
];

/// The character a browser reads a numbered character reference as: none
/// for 0, a surrogate or a number past Unicode, which stand for U+FFFD, and
/// 128 to 159 as Windows-1252 has them, but for the five it leaves empty.
pub(crate) fn numbered(number: u32) -> char {
    const WINDOWS_1252: [u32; 32] = [
        0x20ac, 0x81, 0x201a, 0x192, 0x201e, 0x2026, 0x2020, 0x2021, 0x2c6, 0x2030, 0x160, 0x2039,
        0x152, 0x8d, 0x17d, 0x8f, 0x90, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022, 0x2013, 0x2014,
        0x2dc, 0x2122, 0x161, 0x203a, 0x153, 0x9d, 0x17e, 0x178,
    ];
    let number = match number {
        0x80..=0x9f => WINDOWS_1252[(number - 0x80) as usize],
        n => n,
    };
    char::from_u32(number)
        .filter(|&c| c != '\0')
        .unwrap_or('\u{fffd}')
}

/// `text` with its character references decoded, as an attribute value is.
pub(crate) fn decoded(text: &str, any: bool) -> String {
    if !text.contains('&') {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp + 1..];
        let (c, used) = reference(rest, any);
        match c {
            Some(c) => {
                out.push(c);
                rest = &rest[used..];
            }
            None => out.push('&'),
        }
    }
    out.push_str(rest);
    out
}

/// The character a reference after `&` stands for, and how much of `rest`
/// it takes.
fn reference(rest: &str, any: bool) -> (Option<char>, usize) {
    let b = rest.as_bytes();
    if b.first() == Some(&b'#') {
        let hex = matches!(b.get(1), Some(b'x' | b'X'));
        let from = if hex { 2 } else { 1 };
        let digits = b[from..]
            .iter()
            .take_while(|c| match hex {
                true => c.is_ascii_hexdigit(),
                false => c.is_ascii_digit(),
            })
            .count();
        if digits == 0 {
            return (None, 0);
        }
        let number = u32::from_str_radix(&rest[from..from + digits], if hex { 16 } else { 10 })
            .unwrap_or(u32::MAX);
        let c = numbered(number);
        let used = from + digits + usize::from(b.get(from + digits) == Some(&b';'));
        return (Some(c), used);
    }
    let letters = b.iter().take_while(|c| c.is_ascii_alphanumeric()).count();
    if letters == 0 {
        return (None, 0);
    }
    let name = &rest[..letters];
    let used = letters + usize::from(b.get(letters) == Some(&b';'));
    match NAMED.iter().find(|(n, _)| *n == name) {
        Some(&(_, c)) => (Some(c), used),
        None if any => (Some(ANY), used),
        None => (None, 0),
    }
}

pub(crate) fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r' | b'\x0c')
}

pub(crate) fn ends_name(c: u8) -> bool {
    is_space(c) || matches!(c, b'/' | b'>')
}

pub(crate) fn next(b: &[u8], wanted: u8, from: usize) -> Option<usize> {
    b.get(from..)?
        .iter()
        .position(|&c| c == wanted)
        .map(|i| from + i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reference_is_read_as_a_browser_reads_it() {
        assert_eq!(decoded("&lt;&LT;&Tab;&NewLine;&colon;", false), "<<\t\n:");
        // A name's case is part of it.
        assert_eq!(
            decoded("&Lt;&Colon;&tab;&AMp;", false),
            "&Lt;&Colon;&tab;&AMp;"
        );
        assert_eq!(decoded("&Lt;", true), ANY.to_string());
        assert_eq!(
            decoded("&#0;&#xD800;&#x110000;", false),
            "\u{fffd}".repeat(3)
        );
        assert_eq!(
            decoded("&#150;&#x9F;&#x81;", false),
            "\u{2013}\u{178}\u{81}"
        );
    }

    #[test]
    fn each_escaper_escapes_its_own_characters() {
        let text = "a & b < c > d \" e ' f/g";
        assert_eq!(escaped(text), "a &amp; b &lt; c &gt; d &quot; e ' f/g");
        let mut out = String::new();
        escape_anywhere(text, &mut out).unwrap();
        assert_eq!(out, "a &amp; b &lt; c &gt; d &quot; e &#x27; f/g");
    }

    fn starts(pieces: impl Iterator<Item = (std::ops::Range<usize>, Markup)>) -> Vec<String> {
        pieces
            .filter_map(|(_, piece)| match piece {
                Markup::Start(tag) => Some(tag.name),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn raw_text_whose_end_tag_never_comes_is_read_as_markup_where_asked() {
        let html = "<code><iframe></code>\n<pre>\nx\n</pre>";
        assert_eq!(starts(markup(html)), ["code", "iframe"]);
        assert_eq!(starts(markup_where_closed(html)), ["code", "iframe", "pre"]);
        // Closed, its text is raw either way.
        let html = "<iframe><pre></iframe><p>";
        assert_eq!(starts(markup_where_closed(html)), ["iframe", "p"]);
    }
}
