//! Writing shown on a page other than its own, such as a post's excerpt on
//! a list or in a feed. An address the writing reads from where it is
//! published is spelled from the top of the site instead, so it leads where
//! it leads on the writing's own page. Every other address, and everything
//! around them, is left as it is.

use std::ops::Range;

use crate::content::address;
use crate::html::{self, start_tags};

/// `html`, writing published at `page`, with each address in it read from
/// where it stands spelled from the top of the site, and where each such
/// address is in `html` and in what is made of it; none where it holds no
/// such address. `page` is a path from the top of the host, the site's own
/// path included.
pub(crate) fn html(html: &str, page: &str) -> Option<(String, Respelt)> {
    let mut moved: Vec<(Range<usize>, String)> = Vec::new();
    start_tags(html, |tag| {
        for attribute in &tag.attributes {
            let Some(holds) = holds(&tag.name, &attribute.name) else {
                continue;
            };
            for (raw, at) in addresses(&attribute.value, holds, attribute.at.start) {
                let Some(top) = from_top(raw, page) else {
                    continue;
                };
                let mut spelt = String::with_capacity(top.site.len() + raw.len());
                crate::markdown::escape_href(&top.site, &mut spelt);
                spelt.push_str(&top.own);
                spelt.push_str(&raw[top.rest..]);
                moved.push((at..at + raw.len(), spelt));
            }
        }
    });
    if moved.is_empty() {
        return None;
    }
    let mut out = String::with_capacity(html.len() + moved.len() * page.len());
    let mut respelt = Vec::with_capacity(moved.len());
    let mut from = 0;
    for (range, spelt) in moved {
        out.push_str(&html[from..range.start]);
        respelt.push((range.clone(), out.len()..out.len() + spelt.len()));
        out.push_str(&spelt);
        from = range.end;
    }
    out.push_str(&html[from..]);
    Some((out, respelt))
}

/// Each address spelled again, in order: where it is in the HTML as
/// written, and where in the HTML made of it.
pub(crate) type Respelt = Vec<(Range<usize>, Range<usize>)>;

/// `written`, an address as a browser follows it on writing published at
/// `page`, spelled from the top of the site where it is read from where it
/// stands.
pub(crate) fn address(written: &str, page: &str) -> Option<String> {
    let top = from_top(written, page)?;
    Some(format!("{}{}{}", top.site, top.own, &written[top.rest..]))
}

/// How many addresses an attribute holds.
#[derive(Copy, Clone)]
enum Holds {
    One,
    /// A `srcset`'s candidates.
    Candidates,
    /// Separated by spaces.
    Several,
}

/// Whether the HTML standard reads `attribute` on `element` as addresses,
/// and how many.
fn holds(element: &str, attribute: &str) -> Option<Holds> {
    let named = match attribute {
        "href" => matches!(element, "a" | "area" | "link"),
        "src" => matches!(
            element,
            "audio"
                | "embed"
                | "iframe"
                | "img"
                | "input"
                | "script"
                | "source"
                | "track"
                | "video"
        ),
        "srcset" => matches!(element, "img" | "source"),
        "imagesrcset" => element == "link",
        "poster" => element == "video",
        "cite" => matches!(element, "blockquote" | "del" | "ins" | "q"),
        "action" => element == "form",
        "formaction" => matches!(element, "button" | "input"),
        "data" => element == "object",
        "ping" => matches!(element, "a" | "area"),
        _ => false,
    };
    named.then_some(match attribute {
        "srcset" | "imagesrcset" => Holds::Candidates,
        "ping" => Holds::Several,
        _ => Holds::One,
    })
}

/// The addresses an attribute's value holds, as written, each with where
/// it starts counted from `at`.
fn addresses(value: &str, holds: Holds, at: usize) -> Vec<(&str, usize)> {
    match holds {
        Holds::One => html::addresses(value, false, at),
        Holds::Candidates => html::addresses(value, true, at),
        Holds::Several => {
            let mut found = Vec::new();
            let mut start = None;
            for (i, c) in value.char_indices().chain([(value.len(), ' ')]) {
                match (c.is_ascii_whitespace(), start) {
                    (true, Some(from)) => {
                        found.push((&value[from..i], at + from));
                        start = None;
                    }
                    (false, None) => start = Some(i),
                    _ => {}
                }
            }
            found
        }
    }
}

/// An address spelled from the top of the site: the site's own part, the
/// part of the address kept after it, and where in the address what is
/// kept as written begins.
struct FromTop {
    site: String,
    own: String,
    rest: usize,
}

/// `written` spelled from the top of the site, where a browser reads it
/// from `page`: one opening with neither a scheme nor `/`. A browser leaves
/// tabs and line breaks out of an address, and reads `\` as `/`.
fn from_top(written: &str, page: &str) -> Option<FromTop> {
    let read: String = html::decoded(written, true)
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .collect();
    if read.is_empty() || read.starts_with(['/', '\\']) || address::is_full(&read) {
        return None;
    }
    let whole = |site: &str| FromTop {
        site: site.to_owned(),
        own: String::new(),
        rest: 0,
    };
    if read.starts_with(['#', '?']) {
        return Some(whole(page));
    }
    let folder = &page[..page.rfind('/').map_or(0, |at| at + 1)];
    let end = written.find(['?', '#']).unwrap_or(written.len());
    let path = &written[..end];
    // A reference or a `\` could spell what parts the path; the folder in
    // front of the address whole leads to the same place.
    if path.contains(['&', '\\', '\t', '\n', '\r']) {
        return Some(whole(folder));
    }
    let mut site: Vec<&str> = folder
        .split('/')
        .filter(|piece| !piece.is_empty())
        .collect();
    let mut own: Vec<&str> = Vec::new();
    let mut ends_folder = false;
    for piece in path.split('/') {
        let dots = dots(piece);
        match dots {
            Some(1) => {}
            Some(_) => {
                if own.pop().is_none() {
                    site.pop();
                }
            }
            None => own.push(piece),
        }
        ends_folder = dots.is_some();
    }
    let mut site = site.join("/");
    site.insert(0, '/');
    if site.len() > 1 {
        site.push('/');
    }
    let mut own = own.join("/");
    if ends_folder && !own.is_empty() {
        own.push('/');
    }
    Some(FromTop {
        site,
        own,
        rest: end,
    })
}

/// 1 for a piece of a path standing for its own folder, 2 for one standing
/// for the folder above, with a `.` written as itself or as `%2e`.
fn dots(piece: &str) -> Option<u8> {
    match piece.to_ascii_lowercase().as_str() {
        "." | "%2e" => Some(1),
        ".." | ".%2e" | "%2e." | "%2e%2e" => Some(2),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_address_read_from_where_it_stands_is_spelled_from_the_top() {
        let page = "/blog/post-1/";
        let table: &[(&str, Option<&str>)] = &[
            ("photo-1.jpg", Some("/blog/post-1/photo-1.jpg")),
            ("./photo-1.jpg", Some("/blog/post-1/photo-1.jpg")),
            ("../post-2/", Some("/blog/post-2/")),
            ("../post-2/#notes", Some("/blog/post-2/#notes")),
            ("..", Some("/blog/")),
            (".", Some("/blog/post-1/")),
            ("a/.", Some("/blog/post-1/a/")),
            ("a/../b", Some("/blog/post-1/b")),
            ("%2E%2e/post-2/", Some("/blog/post-2/")),
            ("../../../a", Some("/a")),
            ("a//b", Some("/blog/post-1/a//b")),
            ("#notes", Some("/blog/post-1/#notes")),
            ("?page=2#a", Some("/blog/post-1/?page=2#a")),
            (
                "photo-1.jpg?w=2#a/../b",
                Some("/blog/post-1/photo-1.jpg?w=2#a/../b"),
            ),
            ("/blog/post-2/", None),
            ("\\blog\\", None),
            ("//cdn.example/a.js", None),
            ("https://example.org/a", None),
            ("mailto:someone@example.org", None),
            ("java\nscript:void(0)", None),
            ("", None),
        ];
        for (written, top) in table {
            assert_eq!(address(written, page).as_deref(), *top, "{written:?}");
        }
        assert_eq!(address("a.jpg", "/a.html").as_deref(), Some("/a.jpg"));
        assert_eq!(address("#b", "/a.html").as_deref(), Some("/a.html#b"));
        assert_eq!(address("../a.jpg", "/").as_deref(), Some("/a.jpg"));
    }

    #[test]
    fn only_addresses_in_html_are_spelled_again_and_nothing_else() {
        let page = "/post-1/";
        let written = "<p><a href=\"photo-1.jpg\" title=\"photo-1.jpg\">photo-1.jpg</a> \
                       <img src='a.jpg' srcset=\"a.jpg 1x, /b.jpg 2x,c.jpg 3x\" alt=\"\"> \
                       <a href=#notes ping=\"p.php  /q.php\">1</a> \
                       <use href=\"#icon\"></use> <span data=\"x.jpg\"></span> \
                       <object data=\"x.svg\"></object> <a href=\"  ../post-2/ \">2</a>\
                       <!-- <a href=\"c.html\"> --><script>let a = '<a href=\"d.html\">';</script></p>";
        assert_eq!(
            html(written, page).map(|(html, _)| html).as_deref(),
            Some(
                "<p><a href=\"/post-1/photo-1.jpg\" title=\"photo-1.jpg\">photo-1.jpg</a> \
                 <img src='/post-1/a.jpg' srcset=\"/post-1/a.jpg 1x, /b.jpg 2x,/post-1/c.jpg 3x\" alt=\"\"> \
                 <a href=/post-1/#notes ping=\"/post-1/p.php  /q.php\">1</a> \
                 <use href=\"#icon\"></use> <span data=\"x.jpg\"></span> \
                 <object data=\"/post-1/x.svg\"></object> <a href=\"  /post-2/ \">2</a>\
                 <!-- <a href=\"c.html\"> --><script>let a = '<a href=\"d.html\">';</script></p>"
            )
        );
        assert_eq!(html("<a href=\"/post-2/\">2</a> a.jpg", page), None);
        let (_, respelt) = html("<a href=\"a\">a</a><img src=b>", page).unwrap();
        assert_eq!(respelt, [(9..10, 9..18), (26..27, 34..43)]);
    }

    /// The site's part is spelled as an attribute carries an address; the
    /// author's is left as written.
    #[test]
    fn the_sites_part_is_spelled_for_an_attribute() {
        assert_eq!(
            html("<a href=\"a&amp;b.jpg?x=1&amp;y=2\">", "/caf\u{e9} & co/")
                .map(|(html, _)| html)
                .as_deref(),
            Some("<a href=\"/caf%C3%A9%20&amp;%20co/a&amp;b.jpg?x=1&amp;y=2\">")
        );
        assert_eq!(
            html("<img src=\"d\u{e9}j\u{e0}.jpg\">", "/post-1/")
                .map(|(html, _)| html)
                .as_deref(),
            Some("<img src=\"/post-1/d\u{e9}j\u{e0}.jpg\">")
        );
    }
}
