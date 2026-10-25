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
use crate::links::Page;
use crate::problem::{AllOf, Code, Kind, LabelledBy, Problem, ShownBy};
use crate::reads::Key;
use crate::site::{Id, TemplateId};
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

/// Each element of `html`, `page` as built with the template `layout`,
/// that a screen reader has no words for, told where it was written. A
/// link or a button that is all an image with no alt text, where `shown`,
/// the page's problems, tells that image, is told there instead: an image
/// of the writing's, or one a template shows that its tag is written with
/// the alt text of.
pub(crate) fn told(
    renderer: &Renderer,
    page: &Page,
    html: &str,
    layout: Option<usize>,
    shown: &mut [Problem],
) -> Told {
    let (own, number) = (page.own, page.number);
    let source = renderer.site().pages.get(page.id).source;
    let found = unnamed(html, layout.is_some());
    let building = (!found.is_empty())
        .then(|| renderer.building(page.id, number))
        .flatten();
    let mut told = Vec::with_capacity(found.len());
    let mut folded = Vec::new();
    for Unnamed { kind, at, image } in found {
        let all_of = match &kind {
            Kind::MissingLinkText { href, .. } => AllOf::Link(href.clone()),
            _ => AllOf::Button,
        };
        let at_place = (at.clone()).and_then(|at| placed(renderer, own, html, at));
        // A template's link is folded on this page alone: on another, or in
        // another round of a loop, it may hold another image.
        let across = at_place.filter(|place| renderer.template_of(place.source).is_none());
        // The picture whose address a template printed in the `<img>`, and
        // whether it has no alt text; else the one with none a snippet the
        // writing calls asked for inside it.
        let picture = (image.as_ref()).and_then(|image| {
            (renderer.picture_in(own, image.tag.clone()))
                .or_else(|| Some((asked_inside(renderer, page, html, image.tag.clone())?, true)))
        });
        // A template's picture gives the link words only where the `<img>`
        // is written with its alt text.
        if let (Some(image), Some((picture, true))) = (&image, &picture)
            && image.alt
        {
            if let Some(told) = shown.iter_mut().find(|problem| prints(problem, picture))
                && let Kind::MissingAltText { all_of: at, .. } = &mut told.kind
            {
                AllOf::join(at, Some(&all_of));
                folded.extend(across.map(Some));
                continue;
            }
            if told_there(renderer, page, at.clone(), image.tag.clone(), picture) {
                continue;
            }
        }
        // Else by where the picture is told, inside the `<img>`, unless the
        // `<img>` prints a picture that needs no alt text: a template's
        // `<img>` may show another picture on each page or round.
        let placed = (image.as_ref())
            .filter(|_| !matches!(picture, Some((_, false))))
            .and_then(|image| tag_placed(renderer, own, html, image.tag.clone()));
        if let Some(placed) = placed
            && let Some(told) = shown.iter_mut().find(|problem| shows(problem, placed))
            && let Kind::MissingAltText { all_of: at, .. } = &mut told.kind
        {
            *at = Some(all_of);
            folded.extend(across.map(Some));
            continue;
        }
        // With nothing on the page to place it at, a missing `<title>` is
        // the layout's.
        let place = at_place.or_else(|| {
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
    Told {
        problems: told,
        folded,
    }
}

/// What [`told`] found on a page: its problems, and where each link or
/// button the writing holds was written that is all an image told as
/// missing alt text.
pub(crate) struct Told {
    pub problems: Vec<Problem>,
    pub folded: Vec<Option<Place>>,
}

/// Whether `problem` is a link or a button with no words that is all an
/// image with no alt text.
pub(crate) fn of_an_image(problem: &Problem) -> bool {
    matches!(
        problem.kind,
        Kind::MissingLinkText { image: true, .. } | Kind::MissingButtonText { image: true }
    )
}

/// Whether `problem` is an image's missing alt text told within the tag
/// written at `image`, not yet named as all of a link or a button.
fn shows(problem: &Problem, image: Place) -> bool {
    matches!(problem.kind, Kind::MissingAltText { all_of: None, .. })
        && problem.place.is_some_and(|place| within(place, image))
}

/// Whether `place` is within `tag`.
fn within(place: Place, tag: Place) -> bool {
    place.source == tag.source
        && tag.span.start <= place.span.start
        && place.span.end <= tag.span.end
}

/// The picture with no alt text, as a problem names it, that the snippet
/// or piece a call in the writing builds asked for within the tag at
/// `image` on `page`, which that snippet or piece writes whole: told at
/// the call, it isn't told within the tag.
fn asked_inside(
    renderer: &Renderer,
    page: &Page,
    html: &str,
    image: Range<usize>,
) -> Option<String> {
    let call = renderer.in_text(page.own, image.start)?;
    let tag = tag_placed(renderer, page.own, html, image)?;
    let pages = &renderer.site().pages;
    page.reads.keys().iter().find_map(|key| match *key {
        Key::Body(other) if pages.get(other).source == Some(call.source) => {
            (renderer.asked_at_calls(other).iter())
                .find(|(at, asked, _)| *at == call && within(*asked, tag))
                .map(|(.., picture)| picture.clone())
        }
        _ => None,
    })
}

/// Whether `problem` is the missing alt text of `picture`, as a problem
/// names it, where a template shows it.
fn prints(problem: &Problem, picture: &str) -> bool {
    matches!(
        &problem.kind,
        Kind::MissingAltText { picture: told, shown: ShownBy::Template { .. }, .. } if told == picture
    )
}

/// Whether the link or button whose tag opens at `at` on `page`, holding
/// the tag `image` that prints `picture`, is all of a snippet's call in
/// another page's writing shown there, whose writing told that picture:
/// it is told on that page, as it is the same there.
fn told_there(
    renderer: &Renderer,
    page: &Page,
    at: Option<Range<usize>>,
    image: Range<usize>,
    picture: &str,
) -> bool {
    let Some(call) = renderer.in_text(page.own, image.start) else {
        return false;
    };
    if at.and_then(|at| renderer.in_text(page.own, at.start)) != Some(call) {
        return false;
    }
    let pages = &renderer.site().pages;
    page.reads.keys().iter().any(|key| match *key {
        Key::Body(other) => {
            pages.get(other).source == Some(call.source)
                && (renderer.writing_told(other).iter()).any(|problem| prints(problem, picture))
        }
        _ => false,
    })
}

/// Where the whole tag at `at` on a built page was written: in a
/// template's own text where it opens and closes there, else where the
/// value holding it was written.
fn tag_placed(renderer: &Renderer, own: &Own, html: &str, at: Range<usize>) -> Option<Place> {
    let opens = renderer.written_at(own, at.start);
    if let Some((template, start)) = opens {
        return match renderer.written_at(own, at.end - 1) {
            Some((closing, end)) if closing == template && end >= start => Some(Place {
                source: renderer.template_source(template),
                span: Span::new(start, end + 1),
            }),
            _ => None,
        };
    }
    renderer.printed_at(own, html, at.start, at.len())
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
    /// The first image with no alt text it holds.
    image: Option<Image>,
}

/// An element a screen reader has no words for, with where its tag opens,
/// `<` and its name, and for a link or a button, the first image with no
/// alt text it holds.
pub(crate) struct Unnamed {
    pub kind: Kind,
    pub at: Option<Range<usize>>,
    pub image: Option<Image>,
}

/// An image with no alt text: its whole tag, and whether the tag has an
/// `alt`, empty.
pub(crate) struct Image {
    pub tag: Range<usize>,
    pub alt: bool,
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
pub(crate) fn unnamed(html: &str, titled: bool) -> Vec<Unnamed> {
    let mut found: Vec<Unnamed> = Vec::new();
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
            || (foreign > 0 || matches!(name, "svg" | "math"))
                && html[range.clone()].ends_with("/>");
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
                open.image.get_or_insert_with(|| Image {
                    tag: range.clone(),
                    alt: tag.get("alt").is_some(),
                });
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
                    image: None,
                };
                match closed {
                    true => close(opened, &mut found),
                    false => open.push(opened),
                }
            }
            "iframe" if !names_itself => {
                let src = given(&tag, "src");
                found.push(Unnamed::at(Kind::MissingIframeTitle { src }, opening));
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
            found.push(Unnamed::at(control.kind, control.at));
        }
    }
    if titled {
        match title {
            Some((_, true)) => {}
            Some((at, false)) => found.push(Unnamed::at(Kind::MissingTitle { empty: true }, at)),
            None => {
                if let Some(at) = head.or(root).or(doctype) {
                    found.push(Unnamed::at(Kind::MissingTitle { empty: false }, at));
                }
            }
        }
    }
    found.sort_by_key(|found| found.at.as_ref().map(|at| at.start));
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

impl Unnamed {
    fn at(kind: Kind, at: Range<usize>) -> Unnamed {
        Unnamed {
            kind,
            at: Some(at),
            image: None,
        }
    }
}

/// Tells `open` where nothing gave it words.
fn close(open: Open, found: &mut Vec<Unnamed>) {
    if !open.named {
        let image = open.image.is_some();
        let kind = match open.name {
            "a" => Kind::MissingLinkText {
                href: open.href,
                image,
            },
            _ => Kind::MissingButtonText { image },
        };
        found.push(Unnamed {
            kind,
            at: Some(open.at),
            image: open.image,
        });
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
