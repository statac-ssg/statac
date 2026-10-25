//! What a built page leaves without words a screen reader can read out: a
//! link or a button with none, an `<iframe>` with no `title`, a form
//! control with no label, and a page with no `<title>`. Each is told where
//! its tag was written, known by where the build recorded each byte of the
//! page as written: in a template, once for every page it is met on; in
//! the writing, at the writing.
//!
//! What is inside an element marked `aria-hidden="true"` is never read
//! out, so it names nothing and needs no name, and what is inside a
//! `<template>` is on no page until a script puts it there. A control
//! marked `disabled` takes nothing in. A page is held to having
//! a `<title>` where it is written as a document, opening with a doctype,
//! `<html>` or `<head>`: what a template writes without any of them is a
//! piece of a page, such as one a script fetches, and a page built without
//! a template has no `<title>` by design.

use std::ops::Range;

use crate::html::{self, Markup, Tag};
use crate::problem::{Code, Kind, LabelledBy, Problem};
use crate::site::{Id, PageId, TemplateId};
use crate::source::{Place, Span};
use crate::templates::{Own, Renderer};

/// Whether this module finds problems of kind `code`.
pub(crate) fn finds(code: Code) -> bool {
    matches!(
        code,
        Code::MissingLinkText
            | Code::MissingButtonText
            | Code::MissingIframeTitle
            | Code::MissingLabel
            | Code::MissingTitle
    )
}

/// Whether `a` and `b`, problems of this module's, are one: at one place,
/// of one kind but for what a template printed in the element, where it
/// is a template's.
pub(crate) fn alike(renderer: &Renderer, a: &Problem, b: &Problem) -> bool {
    let templates = |place: Place| renderer.template_of(place.source).is_some();
    a.place == b.place
        && a.kind.code() == b.kind.code()
        && (a.kind == b.kind || a.place.is_some_and(templates))
}

/// Each element of `html`, the page `number` of `page`'s list counted from
/// 1, built with the template `layout`, that a screen reader has no words
/// for, told where it was written.
pub(crate) fn told(
    renderer: &Renderer,
    page: PageId,
    number: usize,
    own: &Own,
    html: &str,
    layout: Option<usize>,
) -> Vec<Problem> {
    let source = renderer.site().pages.get(page).source;
    let found = unnamed(html, layout.is_some());
    let building = (!found.is_empty())
        .then(|| renderer.building(page, number))
        .flatten();
    let mut told = Vec::with_capacity(found.len());
    for (kind, at) in found {
        let place = at.and_then(|at| placed(renderer, own, html, at));
        // With nothing on the page to place it at, a missing `<title>` is
        // the layout's.
        let place = place.or_else(|| {
            let layout = layout.filter(|_| matches!(kind, Kind::MissingTitle { .. }))?;
            let template = renderer.template_source(TemplateId::from_index(layout));
            Some(crate::content::words::head(renderer.sources(), template))
        });
        // What the page's own writing holds is told at the writing, naming
        // no page: it is the writing's wherever the writing is shown.
        let writing = number == 1 && place.is_some_and(|place| Some(place.source) == source);
        let problem = Problem::placed(kind, place);
        // A template's line writing one element many times on the page is
        // told once.
        if told.iter().any(|told| alike(renderer, told, &problem)) {
            continue;
        }
        told.push(match &building {
            Some((relation, head)) if !writing => problem.met(relation.clone(), *head),
            _ => problem,
        });
    }
    told
}

/// Where the tag whose opening, `<` and its name, is at `at` on a built
/// page was written: at that opening in a template's own text, else where
/// the value holding it was written.
fn placed(renderer: &Renderer, own: &Own, html: &str, at: Range<usize>) -> Option<Place> {
    if let Some((template, start)) = renderer.written_at(own, at.start) {
        let last = renderer.written_at(own, at.end - 1);
        let end = match last == Some((template, start + at.len() - 1)) {
            true => start + at.len(),
            false => start + 1,
        };
        return Some(Place {
            source: renderer.template_source(template),
            span: Span::new(start, end),
        });
    }
    renderer.printed_at(own, html, at.start, at.len())
}

/// A link or a button open on the page.
struct Open {
    name: &'static str,
    /// A link's `href`, where it isn't empty.
    href: Option<String>,
    /// Where its tag opens.
    at: Range<usize>,
    /// Whether anything inside it, or on its tag, gives it words.
    named: bool,
    /// Whether it holds an image with no alt text.
    image: bool,
}

/// A form control with no label but perhaps a `<label for>` further on.
struct Control {
    kind: Kind,
    at: Range<usize>,
    id: Option<String>,
}

/// What the text after a tag is.
#[derive(Copy, Clone, PartialEq)]
enum Text {
    /// Words shown on the page.
    Shown,
    /// The page's title.
    Title,
    /// What isn't shown: a script's, a style's, or the like.
    Unshown,
}

/// Each element of `html` a screen reader has no words for, in the order
/// of the page, with where its tag opens: `<` and its name. `titled` says
/// the page, where it is a document, should have a `<title>`. A missing
/// `<title>` is placed at the `<head>`, else at the `<html>`, else at the
/// doctype.
pub(crate) fn unnamed(html: &str, titled: bool) -> Vec<(Kind, Option<Range<usize>>)> {
    let mut found: Vec<(Kind, Option<Range<usize>>)> = Vec::new();
    let mut open: Vec<Open> = Vec::new();
    let mut controls: Vec<Control> = Vec::new();
    // The ids `<label for>` names, and how many labels are open.
    let mut labelled: Vec<String> = Vec::new();
    let mut labels = 0usize;
    // The open element marked `aria-hidden="true"`, by name, and how many
    // of that name are open inside it and it.
    let mut hidden: Option<(String, usize)> = None;
    // How deep in `<svg>` or `<math>`, whose `<title>` is no page's.
    let mut foreign = 0usize;
    // The page's `<title>`, and whether it holds words.
    let mut title: Option<(Range<usize>, bool)> = None;
    let mut head: Option<Range<usize>> = None;
    let mut root: Option<Range<usize>> = None;
    let mut doctype: Option<Range<usize>> = None;
    let mut text = Text::Shown;
    let mut end = 0;
    for (range, piece) in html::markup(html) {
        let says = heard(&html[end..range.start], text, hidden.is_some(), &mut open);
        if text == Text::Title
            && let Some((_, named)) = &mut title
        {
            *named |= says;
        }
        end = range.end;
        text = Text::Shown;
        let tag = match piece {
            Markup::Start(tag) => tag,
            Markup::End(tag) => {
                if let Some((name, depth)) = &mut hidden
                    && *name == tag.name
                {
                    *depth -= 1;
                    if *depth == 0 {
                        hidden = None;
                    }
                }
                match tag.name.as_str() {
                    "a" | "button" => {
                        if let Some(at) = open.iter().rposition(|open| open.name == tag.name) {
                            close(open.remove(at), &mut found);
                        }
                    }
                    "label" => labels = labels.saturating_sub(1),
                    "svg" | "math" => foreign = foreign.saturating_sub(1),
                    _ => {}
                }
                continue;
            }
            Markup::Other => {
                let opens = html[range.clone()].get(..9);
                if opens.is_some_and(|opens| opens.eq_ignore_ascii_case("<!doctype")) {
                    doctype.get_or_insert(range.start..range.start + 9);
                }
                continue;
            }
        };
        let name = tag.name.as_str();
        let opening = range.start..range.start + 1 + name.len();
        // In foreign content, `/>` closes an element.
        let closed = html::VOID.contains(&name)
            || (foreign > 0 || matches!(name, "svg" | "math")) && html[range].ends_with("/>");
        text = match name {
            "title" | "xmp" | "plaintext" => Text::Shown,
            name if name == "script" || html::RAW_TEXT.contains(&name) => Text::Unshown,
            _ => Text::Shown,
        };
        match name {
            "label" => {
                labels += usize::from(!closed);
                labelled.extend(given(&tag, "for"));
            }
            "svg" | "math" if !closed => foreign += 1,
            "title" if foreign == 0 => {
                title.get_or_insert((opening.clone(), false));
                text = Text::Title;
            }
            "head" => _ = head.get_or_insert(opening.clone()),
            "html" => _ = root.get_or_insert(opening.clone()),
            _ => {}
        }
        match &mut hidden {
            Some((hiding, depth)) => {
                if *hiding == tag.name && !closed {
                    *depth += 1;
                }
                continue;
            }
            None if is_hidden(&tag) => {
                if !closed {
                    hidden = Some((tag.name.clone(), 1));
                }
                continue;
            }
            None => {}
        }
        let names_itself = ["aria-label", "aria-labelledby", "title"]
            .iter()
            .any(|attribute| given(&tag, attribute).is_some());
        let alt = given(&tag, "alt").is_some();
        for open in &mut open {
            if names_itself || name == "img" && alt {
                open.named = true;
            } else if name == "img" {
                open.image = true;
            }
        }
        match name {
            "a" | "button" if name == "button" || tag.get("href").is_some() => {
                // A link, or a button, opened inside another closes it.
                if let Some(at) = open.iter().rposition(|open| open.name == name) {
                    close(open.remove(at), &mut found);
                }
                let opened = Open {
                    name: if name == "a" { "a" } else { "button" },
                    href: given(&tag, "href"),
                    at: opening,
                    named: names_itself,
                    image: false,
                };
                match closed {
                    true => close(opened, &mut found),
                    false => open.push(opened),
                }
            }
            "iframe" if !names_itself => {
                let src = given(&tag, "src");
                found.push((Kind::MissingIframeTitle { src }, Some(opening)));
            }
            "input" | "select" | "textarea"
                if !names_itself && labels == 0 && tag.get("disabled").is_none() =>
            {
                let kind = (tag.get("type"))
                    .filter(|_| name == "input")
                    .map(|kind| kind.trim().to_ascii_lowercase());
                // A browser labels a submit or reset button itself.
                let by = match kind.as_deref() {
                    Some("hidden" | "submit" | "reset") => continue,
                    Some("button") if given(&tag, "value").is_some() => continue,
                    Some("image") if alt => continue,
                    Some("button") => LabelledBy::Value,
                    Some("image") => LabelledBy::Alt,
                    _ => LabelledBy::Label,
                };
                let id = given(&tag, "id");
                let kind = Kind::MissingLabel {
                    control: name.to_owned(),
                    name: given(&tag, "name"),
                    id: id.clone(),
                    by,
                };
                controls.push(Control {
                    kind,
                    at: opening,
                    id,
                });
            }
            _ => {}
        }
    }
    let says = heard(&html[end..], text, hidden.is_some(), &mut open);
    if text == Text::Title
        && let Some((_, named)) = &mut title
    {
        *named |= says;
    }
    for open in open {
        close(open, &mut found);
    }
    for control in controls {
        if !(control.id.as_ref()).is_some_and(|id| labelled.contains(id)) {
            found.push((control.kind, Some(control.at)));
        }
    }
    if titled {
        match title {
            Some((_, true)) => {}
            Some((at, false)) => found.push((Kind::MissingTitle { empty: true }, Some(at))),
            None => {
                if let Some(at) = head.or(root).or(doctype) {
                    found.push((Kind::MissingTitle { empty: false }, Some(at)));
                }
            }
        }
    }
    found.sort_by_key(|(_, at)| at.as_ref().map(|at| at.start));
    found
}

/// Whether `between`, the text after a tag, says anything; where it is
/// shown and not hidden, it gives each link and button open words.
fn heard(between: &str, text: Text, hidden: bool, open: &mut [Open]) -> bool {
    let says = text != Text::Unshown && says(between);
    if says && !hidden {
        open.iter_mut().for_each(|open| open.named = true);
    }
    says
}

/// Tells `open` where nothing gave it words.
fn close(open: Open, found: &mut Vec<(Kind, Option<Range<usize>>)>) {
    if !open.named {
        let kind = match open.name {
            "a" => Kind::MissingLinkText {
                href: open.href,
                image: open.image,
            },
            _ => Kind::MissingButtonText { image: open.image },
        };
        found.push((kind, Some(open.at)));
    }
}

/// The value of the attribute `name` on `tag`, its character references
/// read, where it holds more than spaces.
fn given(tag: &Tag, name: &str) -> Option<String> {
    let value = html::decoded(tag.get(name)?, false);
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

/// Whether `tag` keeps itself and what it holds from a screen reader.
fn is_hidden(tag: &Tag) -> bool {
    tag.name == "template"
        || tag
            .get("aria-hidden")
            .is_some_and(|value| value.trim().eq_ignore_ascii_case("true"))
}

/// Whether text between tags holds anything but spaces, its character
/// references read.
fn says(text: &str) -> bool {
    html::decoded(text, true)
        .chars()
        .any(|c| !c.is_whitespace())
}

#[cfg(test)]
mod tests;
