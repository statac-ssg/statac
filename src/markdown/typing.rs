//! What the writing would read where an editor's cursor stands, while
//! what is there is still being typed: whether a call or a link written
//! there would be read as one, rather than as code, text shown as typed,
//! or HTML. Each is answered by reading the writing with the call or
//! link closed at the cursor, so it follows every rule reading does.

use super::{Document, Kind, Node, Wraps};
use crate::source::{Sources, Span};

/// The name a call is read under while it is typed, which no snippet
/// wraps.
const TYPED: &str = "x";

/// Whether a call opening `@name` at `at`, typed as far as `offset`, is
/// read as a call there. `closing` closes what is typed of its values,
/// such as a quote and the bracket, where its bracket is open. An `@url(...)` is asked as one
/// handed an address, which it is wherever a call is read, a link's
/// address among them; any other call as a call to a snippet.
pub fn reads_a_call(
    text: &str,
    body: Span,
    at: usize,
    offset: usize,
    closing: &str,
    wraps: Wraps<'_>,
) -> bool {
    let typed = &text[at..offset];
    let open = typed.find('(');
    let in_site = open.is_some_and(|open| &typed[1..open] == super::IN_SITE);
    let written = match open {
        None => format!("@{TYPED}()"),
        Some(_) if in_site => format!("@{}(/)", super::IN_SITE),
        Some(open) => format!("@{TYPED}{}{closing}", &typed[open..]),
    };
    let wraps = |name: &str| match name {
        TYPED => Some(false),
        name => wraps(name),
    };
    let Some(document) = read_with(text, body, at, offset, &written, &wraps) else {
        return false;
    };
    match in_site {
        true => (document.in_site.iter()).any(|stands| stands.place.span.start as usize == at),
        false => called_at(&document, at),
    }
}

/// Whether a link or picture whose address opens after the bracket at
/// `open`, typed as far as `offset`, is read as one there, and whether
/// it is a picture.
pub fn reads_a_link(
    text: &str,
    body: Span,
    open: usize,
    offset: usize,
    wraps: Wraps<'_>,
) -> Option<bool> {
    let document = read_with(text, body, offset, offset, ")", wraps)?;
    let mut found = None;
    for top in std::iter::once(&document.root).chain(&document.notes) {
        top.walk(&mut |node| {
            let span = node.place.span;
            let holds = (span.start as usize) < open && offset < span.end as usize;
            match node.kind {
                Kind::Link { .. } if holds => found = Some(false),
                Kind::Image { .. } if holds => found = Some(true),
                _ => {}
            }
        });
    }
    found
}

/// Whether `offset` is in the address of a line defining a link
/// reference, `[label]: /address`.
pub fn in_a_definition(text: &str, body: Span, offset: usize, wraps: Wraps<'_>) -> bool {
    let mut sources = Sources::default();
    let source = sources.add("typed", text.to_owned());
    let document = super::read(&sources, source, body, wraps, "");
    (document.definitions.iter()).any(|definition| {
        let span = definition.place.span;
        (span.start as usize) < offset && offset <= span.end as usize
    })
}

/// The writing read with `written` in place of what lies from `at` to
/// `end`; none where `body` doesn't reach `end`.
fn read_with(
    text: &str,
    body: Span,
    at: usize,
    end: usize,
    written: &str,
    wraps: Wraps<'_>,
) -> Option<Document> {
    let (start, stop) = (body.start as usize, body.end as usize);
    if at < start || end > stop {
        return None;
    }
    let read = format!("{}{written}{}", &text[..at], &text[end..]);
    let body = Span::new(start, stop - (end - at) + written.len());
    let mut sources = Sources::default();
    let source = sources.add("typed", read);
    Some(super::read(&sources, source, body, wraps, ""))
}

/// Whether the document holds a call to a snippet written at `at`.
fn called_at(document: &Document, at: usize) -> bool {
    let mut found = false;
    let mut look = |node: &Node| {
        if let Kind::Call(call) = &node.kind {
            found |= node.place.span.start as usize == at && call.name.value == TYPED;
        }
    };
    for top in std::iter::once(&document.root).chain(&document.notes) {
        top.walk(&mut look);
    }
    found
}
