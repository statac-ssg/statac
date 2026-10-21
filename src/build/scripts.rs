//! The JavaScript a built page carries: scripts it loads by address,
//! JavaScript written into it, and what it embeds that may run its own.
//!
//! Two readings are made. One follows the HTML tokenizer as a browser runs
//! it in an HTML document: comments, and the text of elements such as
//! `<style>` and `<textarea>`, hold no markup. The other looks for anything
//! that could run in any context a browser might read the page in, comments
//! and SVG included, and with character references decoded. Whatever the
//! second finds past the first is still counted, as unsure: saying too much
//! is safe, and missing one is not.

use crate::html::{ANY, Tag, decoded, end_tag, ends_name, is_space, start_tags, tag};

/// Something on a page that runs JavaScript, or may.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Found {
    /// A script loaded from this address, as written.
    Loaded(String),
    /// JavaScript written into the page: a script's text, an event handler
    /// attribute, or a `javascript:` address.
    Written,
    /// An `<object>`, `<embed>`, `<iframe>`, `<frame>` or `<applet>`, by
    /// its address, empty where it has none, which may run its own.
    Embedded(String),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Finding {
    pub found: Found,
    /// Whether a browser reading the page as HTML runs it; where not, it
    /// was found only by the wider reading.
    pub sure: bool,
}

/// Everything in `html` that runs JavaScript or may, the sure first.
pub(super) fn in_html(html: &str) -> Vec<Finding> {
    let mut sure = Vec::new();
    faithful(html, &mut sure);
    let mut maybe = Vec::new();
    wide(html, &mut maybe);
    let mut unmatched = sure.clone();
    let mut findings: Vec<Finding> = sure
        .into_iter()
        .map(|found| Finding { found, sure: true })
        .collect();
    for found in maybe {
        match unmatched.iter().position(|f| *f == found) {
            Some(at) => {
                unmatched.swap_remove(at);
            }
            None => findings.push(Finding { found, sure: false }),
        }
    }
    findings
}

/// What stands for one piece of JavaScript written in: which kind it is, a
/// script's text or an attribute's value, and that text as written. Two
/// are equal only where both are. A script's lines count without their
/// indent, which a snippet's take from the line it is called on.
pub(super) type Print = [u8; 16];

/// A piece of JavaScript a text writes in.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) struct Piece {
    pub print: Print,
    /// Whether a template fills some of it in as a page is built, so that
    /// what a page holds is not what the file holds.
    pub filled: bool,
}

/// Every piece of JavaScript `html` writes in where a browser runs it: one
/// for each [`Found::Written`] that [`in_html`] finds sure.
pub(super) fn written(html: &str) -> Vec<Piece> {
    let mut out = Vec::new();
    start_tags(html, |tag| {
        if writes_script(tag) {
            let end = end_tag(html.as_bytes(), tag.end, b"script").unwrap_or(html.len());
            out.push(piece(b's', &html[tag.end.min(end)..end]));
        }
        if EMBEDS.contains(&tag.name.as_str())
            && let Some(document) = tag.get("srcdoc")
        {
            out.extend(written(&decoded(document, false)));
        }
        for attribute in &tag.attributes {
            if runs(attribute) {
                out.push(piece(b'a', &attribute.value));
            }
        }
    });
    out
}

fn piece(kind: u8, text: &str) -> Piece {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&[kind]);
    if kind == b's' {
        for line in text.split_inclusive('\n') {
            hasher.update(line.trim_start_matches([' ', '\t']).as_bytes());
        }
    } else {
        hasher.update(text.as_bytes());
    }
    let mut print = Print::default();
    print.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
    Piece {
        print,
        filled: ["{{", "{%", "{#"].iter().any(|open| text.contains(open)),
    }
}

/// Whether a `<script>` holds JavaScript of its own, not an address.
fn writes_script(tag: &Tag) -> bool {
    tag.name == "script"
        && script_address(tag).is_none()
        && (tag.cut || tag.get("type").is_none_or(is_javascript))
}

fn script_address(tag: &Tag) -> Option<&str> {
    ["src", "href", "xlink:href"]
        .iter()
        .find_map(|name| tag.get(name))
}

/// Whether an attribute is an event handler or holds a `javascript:`
/// address.
fn runs(attribute: &crate::html::Attribute) -> bool {
    is_handler(&attribute.name) || runs_as_address(&decoded(&attribute.value, false))
}

/// Elements that embed something which may run scripts of its own.
const EMBEDS: &[&str] = &["object", "embed", "iframe", "frame", "applet"];

/// The attributes an embed is given its address by.
const EMBED_ADDRESS: &[&str] = &["data", "src", "code"];

/// The longest stretch read as one tag in the wider reading, which starts
/// a tag at every `<` it could start at.
const LONGEST_TAG: usize = 64 * 1024;

/// The reading a browser makes of an HTML document.
fn faithful(html: &str, out: &mut Vec<Found>) {
    start_tags(html, |tag| findings(tag, out, true));
}

/// The wider reading: a tag wherever one could start, comments and the
/// text of every element included; addresses with their character
/// references decoded; and what a frame's `srcdoc` holds, read the same way.
fn wide(html: &str, out: &mut Vec<Found>) {
    let b = html.as_bytes();
    for lt in (0..b.len()).filter(|&i| b[i] == b'<') {
        let name_end = (lt + 1..b.len())
            .find(|&i| !b[i].is_ascii_alphabetic())
            .unwrap_or(b.len());
        if name_end == lt + 1 || !b.get(name_end).is_none_or(|&c| ends_name(c)) {
            continue;
        }
        let name = html[lt + 1..name_end].to_ascii_lowercase();
        if name == "script" || EMBEDS.contains(&name.as_str()) {
            findings(&tag(html, lt + 1, LONGEST_TAG), out, false);
        }
    }
    handlers(html, out);
    addresses(html, out);
}

/// What `tag` runs or may. The faithful reading also takes event handlers
/// and `javascript:` addresses from it; the wider one finds those wherever
/// they are written.
fn findings(tag: &Tag, out: &mut Vec<Found>, faithful: bool) {
    if tag.name == "script" {
        match script_address(tag) {
            Some(src) => out.push(Found::Loaded(decoded(src, false))),
            None if writes_script(tag) => out.push(Found::Written),
            None => {}
        }
    }
    if EMBEDS.contains(&tag.name.as_str()) {
        let address = EMBED_ADDRESS.iter().find_map(|name| tag.get(name));
        match address {
            Some(address) => out.push(Found::Embedded(decoded(address, false))),
            None if tag.name != "iframe" && tag.name != "frame" => {
                out.push(Found::Embedded(String::new()));
            }
            None => {}
        }
        if let Some(document) = tag.get("srcdoc") {
            let document = decoded(document, false);
            match faithful {
                true => self::faithful(&document, out),
                false => wide(&document, out),
            }
        }
    }
    if !faithful {
        return;
    }
    for attribute in &tag.attributes {
        if runs(attribute) {
            out.push(Found::Written);
        }
    }
}

/// Every event handler attribute that could be written: `on` and a name,
/// then `=`, after whatever may come before an attribute's name, inside a
/// tag that could hold it.
fn handlers(html: &str, out: &mut Vec<Found>) {
    let b = html.as_bytes();
    for i in 1..b.len() {
        if !(is_space(b[i - 1]) || matches!(b[i - 1], b'/' | b'"' | b'\'')) {
            continue;
        }
        if b.len() < i + 3 || !b[i..i + 2].eq_ignore_ascii_case(b"on") {
            continue;
        }
        let mut j = i + 2;
        while j < b.len() && !is_space(b[j]) && !matches!(b[j], b'/' | b'>' | b'=') {
            j += 1;
        }
        if j == i + 2 {
            continue;
        }
        while j < b.len() && is_space(b[j]) {
            j += 1;
        }
        if b.get(j) == Some(&b'=') && in_a_tag(html, i) {
            out.push(Found::Written);
        }
    }
}

/// Whether a tag that could start before `at` runs past it and has an
/// event handler among its attributes. Text between tags has no
/// attributes, whatever it spells: a code sample's `<` is written `&lt;`,
/// and starts no tag.
fn in_a_tag(html: &str, at: usize) -> bool {
    let b = html.as_bytes();
    (at.saturating_sub(LONGEST_TAG)..at)
        .rev()
        .filter(|&lt| b[lt] == b'<' && b[lt + 1].is_ascii_alphabetic())
        .any(|lt| {
            let mut longest = LONGEST_TAG.min(b.len() - lt - 1);
            while !html.is_char_boundary(lt + 1 + longest) {
                longest -= 1;
            }
            let tag = tag(html, lt + 1, longest);
            tag.end > at && tag.attributes.iter().any(|a| is_handler(&a.name))
        })
}

/// Every `javascript:` address that could be written: after an `=` or a
/// `;`, and a quote, with character references decoded, and the tabs,
/// newlines and leading spaces a browser passes over dropped. A reference
/// Statac doesn't know could be any character, so it matches any.
fn addresses(html: &str, out: &mut Vec<Found>) {
    let text: Vec<char> = decoded(html, true)
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .map(|c| c.to_ascii_lowercase())
        .collect();
    let scheme: Vec<char> = "javascript:".chars().collect();
    for start in 0..text.len().saturating_sub(scheme.len() - 1) {
        let matches = scheme
            .iter()
            .zip(&text[start..])
            .all(|(want, got)| want == got || *got == ANY);
        if !matches {
            continue;
        }
        let mut i = start;
        while i > 0 && (text[i - 1] <= ' ' || text[i - 1] == ANY) {
            i -= 1;
        }
        if i > 0 && matches!(text[i - 1], '"' | '\'') {
            i -= 1;
            while i > 0 && text[i - 1] <= ' ' {
                i -= 1;
            }
        }
        if i > 0 && matches!(text[i - 1], '=' | ';') {
            out.push(Found::Written);
        }
    }
}

/// Whether a `type` names JavaScript: none, a module, or a JavaScript
/// media type. Anything else is data, such as an import map or JSON.
fn is_javascript(kind: &str) -> bool {
    let kind = kind.trim().to_ascii_lowercase();
    let essence = kind.split(';').next().unwrap_or_default().trim();
    essence.is_empty()
        || essence == "module"
        || [
            "javascript",
            "ecmascript",
            "x-javascript",
            "x-ecmascript",
            "jscript",
            "livescript",
        ]
        .iter()
        .any(|name| essence.strip_suffix(name).is_some_and(|t| t.ends_with('/')))
}

fn is_handler(name: &str) -> bool {
    name.len() > 2 && name.starts_with("on")
}

/// Whether an attribute's value, decoded, is an address a browser runs.
fn runs_as_address(value: &str) -> bool {
    let value: String = value
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .collect();
    value
        .trim_start_matches(|c: char| c <= ' ')
        .to_ascii_lowercase()
        .starts_with("javascript:")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sure(found: Found) -> Finding {
        Finding { found, sure: true }
    }

    fn unsure(found: Found) -> Finding {
        Finding { found, sure: false }
    }

    fn loaded(address: &str) -> Found {
        Found::Loaded(address.into())
    }

    #[test]
    fn what_a_browser_runs_is_found_sure() {
        let table: &[(&str, Vec<Finding>)] = &[
            (
                "<script src=\"/a.js\"></script>",
                vec![sure(loaded("/a.js"))],
            ),
            ("<SCRIPT>a()</SCRIPT>", vec![sure(Found::Written)]),
            (
                "<script\ttype=module src=/b.js></script>",
                vec![sure(loaded("/b.js"))],
            ),
            (
                "<script/src='/c.js?x=1&amp;y=2'></script>",
                vec![sure(loaded("/c.js?x=1&y=2"))],
            ),
            (
                "<script type=\"text/javascript\">a()</script>",
                vec![sure(Found::Written)],
            ),
            (
                "<svg><script href=\"/d.js\"/></svg>",
                vec![sure(loaded("/d.js"))],
            ),
            (
                "<svg><script xlink:href=\"/e.js\"></script></svg>",
                vec![sure(loaded("/e.js"))],
            ),
            ("<img src=x onerror=a()>", vec![sure(Found::Written)]),
            ("<body ONLOAD = 'a()'>", vec![sure(Found::Written)]),
            ("<a title=\">\" onclick=a()>", vec![sure(Found::Written)]),
            ("<a href=\"javascript:a()\">", vec![sure(Found::Written)]),
            ("<a href=\" JaVa\tScRiPt:a()\">", vec![sure(Found::Written)]),
            (
                "<a href=\"&#106;avascript&colon;a()\">",
                vec![sure(Found::Written)],
            ),
            (
                "<a href=\"&#x6A;ava&#x0A;script:a()\">",
                vec![sure(Found::Written)],
            ),
            (
                "<a href=\"java&Tab;script&colon;a()\">",
                vec![sure(Found::Written)],
            ),
            (
                "<a href=\"&#32;&#1;javascript:a()\">",
                vec![sure(Found::Written)],
            ),
            ("<form action=javascript:a()>", vec![sure(Found::Written)]),
            (
                "<button formaction='javascript:a()'>",
                vec![sure(Found::Written)],
            ),
            (
                "<iframe srcdoc=\"&lt;script&gt;a()&lt;/script&gt;\"></iframe>",
                vec![sure(Found::Written)],
            ),
            (
                "<object data=\"/f.svg\"></object>",
                vec![sure(Found::Embedded("/f.svg".into()))],
            ),
            (
                "<embed src=\"/g.swf\">",
                vec![sure(Found::Embedded("/g.swf".into()))],
            ),
            (
                "<iframe src=\"https://example.org/\"></iframe>",
                vec![sure(Found::Embedded("https://example.org/".into()))],
            ),
            (
                "<!-->< script><script>a()</script>",
                vec![sure(Found::Written)],
            ),
            ("<!--->-<script>a()</script>", vec![sure(Found::Written)]),
            (
                "<!-- a --!><script>a()</script>",
                vec![sure(Found::Written)],
            ),
            (
                "<style>/* <a b=\" */</style><script>a()</script><p c=\"x\">",
                vec![sure(Found::Written)],
            ),
        ];
        for (html, wanted) in table {
            assert_eq!(&in_html(html), wanted, "{html}");
        }
    }

    #[test]
    fn what_a_browser_may_run_is_found_unsure() {
        let table: &[(&str, Vec<Finding>)] = &[
            (
                "<!--[if lt IE 9]><script src=\"/old.js\"></script><![endif]-->",
                vec![unsure(loaded("/old.js"))],
            ),
            ("<!-- <img onerror=a()> -->", vec![unsure(Found::Written)]),
            (
                "<textarea><script>a()</script></textarea>",
                vec![unsure(Found::Written)],
            ),
            (
                "<svg><style><a b=\"</style>\" onload=\"a()\"></style></svg>",
                vec![unsure(Found::Written)],
            ),
            (
                "<meta http-equiv=refresh content=\"0; url=javascript:a()\">",
                vec![unsure(Found::Written)],
            ),
            (
                "<a href=\"&zwnj;javascript:a()\">",
                vec![unsure(Found::Written)],
            ),
            (
                "<script>let a = '<script>';</script>",
                vec![sure(Found::Written), unsure(Found::Written)],
            ),
        ];
        for (html, wanted) in table {
            assert_eq!(&in_html(html), wanted, "{html}");
        }
    }

    #[test]
    fn each_piece_written_in_is_told_apart_by_its_kind_and_its_text() {
        // One piece for each sure finding of JavaScript written in.
        for html in [
            "<script>a()</script><SCRIPT>a()</SCRIPT><script src=/a.js></script>",
            "<a onclick=a() href=\"javascript:b()\">",
            "<iframe srcdoc=\"&lt;script&gt;a()&lt;/script&gt;\" onload=b()></iframe>",
            "<script type=\"application/ld+json\">{}</script><script>a()",
            "<!-- <script>a()</script> --><textarea><script>a()</script></textarea>",
        ] {
            let sure = in_html(html)
                .into_iter()
                .filter(|f| f.sure && f.found == Found::Written)
                .count();
            assert_eq!(written(html).len(), sure, "{html}");
        }
        let print = |html: &str| written(html)[0].print;
        assert_eq!(
            print("<script>a()</script>"),
            print("<SCRIPT type=module>a()</script>")
        );
        assert_ne!(
            print("<script>a()</script>"),
            print("<script>a() </script>")
        );
        // A script is the same script at any indent, and an attribute is
        // its value as written.
        assert_eq!(
            print("<script>\n  a();\n  b();\n</script>"),
            print("<script>\n      a();\n      b();\n    </script>")
        );
        assert_ne!(
            print("<script>\na();\nb();\n</script>"),
            print("<script>\na(); b();\n</script>")
        );
        assert_ne!(
            print("<a onclick=\"a();\n b()\">"),
            print("<a onclick=\"a();\nb()\">")
        );
        assert_ne!(print("<script>a()</script>"), print("<a onclick=\"a()\">"));
        assert_eq!(print("<a onclick=\"a()\">"), print("<b onload='a()'>"));
        let filled = |html: &str| written(html)[0].filled;
        assert!(!filled("<script>a({})</script>"));
        assert!(filled("<script>a({{ id }})</script>"));
        assert!(filled("<script>{% if id %}a(){% endif %}</script>"));
        assert!(filled("<a onclick=\"a({{ id }})\">"));
    }

    #[test]
    fn data_and_text_carry_no_javascript() {
        for html in [
            "<script type=\"importmap\">{}</script>",
            "<script type=\"application/ld+json\">{}</script>",
            "<script type=\"text/plain\">a()</script>",
            "<scripts></scripts><p>&lt;script&gt;a()&lt;/script&gt;</p>",
            "<p>Turn it on, then off.</p><a href=\"/javascript/\">JavaScript: a book</a>",
            "<code>window.onload = a;</code>",
            "<pre><code>{% for one in [page] + page.translations if one.language == main %}\n\
             &lt;<span class=\"a\">link</span> <span class=\"b\">rel</span>=&quot;alternate&quot;&gt;\n\
             {% endfor %}</code></pre>",
            "<p>Set <code>one = two</code>, then <code>only=first</code>.</p>",
        ] {
            assert_eq!(in_html(html), [], "{html}");
        }
    }
}
