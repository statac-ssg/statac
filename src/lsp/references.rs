//! Find references: every link in the site to a page, or to a heading on
//! it. A link counts where the author wrote it: in a page's writing, as
//! Markdown, as HTML, as `@url(...)`, by a label's line, or as a value
//! handed to a snippet call that is written as an address; and in a
//! template or snippet, as a string handed to `url(...)` or as an address
//! written whole from the top of the site. A link to an old address the
//! page sends readers on from is a link to the page.

#[cfg(test)]
mod tests;

use super::complete::template::{Pointed, pointed};
use super::hover::Goes;
use crate::content::address;
use crate::links;
use crate::site::{PageId, SentTo};
use crate::source::{Place, Span};
use crate::templates::Renderer;

/// Where a link, or what links lead to, is written.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) enum Written {
    At(Place),
    /// The file at this path in the site, as a whole.
    File(String),
}

/// A link to what is sought, as it is written.
pub(super) struct Link {
    pub place: Place,
    /// The page it is written in; none in a template.
    pub from: Option<PageId>,
    /// Whether it is handed an address from the top of the site, without
    /// the path the site is published under, as `@url(...)` and `url(...)`
    /// are.
    pub helper: bool,
}

/// Every link in the site's writing and templates that leads to one of
/// `files`, the files the site publishes, and where `id` is given, to the
/// heading with that id.
pub(super) fn links(renderer: &Renderer, files: Vec<String>, id: Option<&str>) -> Vec<Link> {
    let site = renderer.site();
    let follow = links::Site::new(renderer.settings(), files);
    let leads = |written: &str, from: &str| {
        let Some((_, fragment)) = follow.lands_on(written, from) else {
            return false;
        };
        match (id, fragment) {
            (None, _) => true,
            (Some(id), Some(fragment)) => fragment == id || address::decoded(&fragment) == id,
            (Some(_), None) => false,
        }
    };
    let sources = renderer.sources();
    let mut found = Vec::new();
    let base = renderer.settings().base();
    for (other, held) in site.pages.iter() {
        let Some(writing) = renderer.writing(other) else {
            continue;
        };
        let from = format!("{base}{}", held.address.as_str());
        // Any text can be handed to a call; one that isn't written as an
        // address is taken as something else.
        let handed = (writing.handed.iter())
            .filter(|linked| linked.address.contains('/') || linked.address.starts_with('#'));
        for linked in writing.written.iter().chain(handed) {
            if leads(&linked.address, &from) {
                found.push(Link {
                    place: linked.place,
                    from: Some(other),
                    helper: sources.spelling(linked.place).starts_with("@url("),
                });
            }
        }
    }
    let top = format!("{base}/");
    for (_, source, _) in renderer.templates() {
        let text = sources.text(source);
        let path = sources.path(source).to_string_lossy().replace('\\', "/");
        for (address, at) in links::fixed(text) {
            if leads(&address, &top) {
                let span = Span::new(at.start, at.end);
                found.push(Link {
                    place: Place { source, span },
                    from: None,
                    helper: false,
                });
            }
        }
        let mut from = 0;
        while let Some(at) = text[from..].find("url(") {
            from += at + "url(".len();
            let Some(quote) = text[from..].find(|c: char| !c.is_whitespace()) else {
                break;
            };
            let quote = from + quote;
            if !text[quote..].starts_with(['"', '\'']) {
                continue;
            }
            let Some(Pointed::Address { range, address }) =
                pointed(text, quote + 1, &path, renderer)
            else {
                continue;
            };
            if leads(&format!("{base}{address}"), &top) {
                let span = Span::new(range.start, range.end);
                found.push(Link {
                    place: Place { source, span },
                    from: None,
                    helper: true,
                });
            }
        }
    }
    found
}

/// What links are sought to.
enum Sought {
    Page(PageId),
    /// A heading of the page, by its id, written at the place.
    Heading(PageId, String, Place),
}

/// Every link to what the character at `offset` in the file at `rel`
/// names, `to` being where hover goes from there: a page, or a heading on
/// one, and the page or heading the file is, or the cursor is in, where it
/// names nothing. `declaration` adds the page's file or the heading
/// itself. None where it is no page or heading.
pub(super) fn found(
    renderer: &Renderer,
    rel: &str,
    offset: usize,
    to: Option<Goes>,
    declaration: bool,
) -> Option<Vec<Written>> {
    let sought = sought(renderer, rel, offset, to)?;
    let (page, id) = match &sought {
        Sought::Page(page) => (*page, None),
        Sought::Heading(page, id, _) => (*page, Some(id.as_str())),
    };
    let mut found = Vec::new();
    if declaration {
        found.push(match &sought {
            Sought::Page(page) => Written::File(renderer.page_path(*page)),
            Sought::Heading(_, _, place) => Written::At(*place),
        });
    }
    let linked = links(renderer, files(renderer, page), id);
    found.extend(linked.into_iter().map(|link| Written::At(link.place)));
    let sources = renderer.sources();
    let key = |written: &Written| match written {
        Written::At(place) => (sources.path(place.source).to_path_buf(), place.span.start),
        Written::File(path) => (path.into(), 0),
    };
    found.sort_by_key(key);
    found.dedup();
    Some(found)
}

/// What the cursor names, or is in where it names nothing.
fn sought(renderer: &Renderer, rel: &str, offset: usize, to: Option<Goes>) -> Option<Sought> {
    let site = renderer.site();
    let page_at = |path: &str| {
        (site.pages.iter())
            .filter(|(_, page)| page.source.is_some() && page.made.is_none())
            .find(|(id, _)| renderer.page_path(*id) == path)
            .map(|(id, _)| id)
    };
    let heading = |page: PageId, holds: &dyn Fn(Place) -> bool| {
        let heading = (renderer.headings(page).into_iter()).find(|heading| holds(heading.place))?;
        Some(Sought::Heading(page, heading.id, heading.place))
    };
    match to {
        Some(Goes::File(path)) => page_at(&path).map(Sought::Page),
        Some(Goes::At(place)) => (site.pages.iter())
            .filter(|(_, page)| page.source == Some(place.source))
            .find_map(|(page, _)| heading(page, &|at| at == place)),
        None => {
            let page = page_at(rel)?;
            let under = |at: Place| at.span.range().contains(&offset);
            // A link that leads nowhere known names no page.
            let linked = renderer
                .writing(page)
                .is_some_and(|writing| (writing.written.iter()).any(|linked| under(linked.place)));
            if linked {
                return None;
            }
            Some(heading(page, &under).unwrap_or(Sought::Page(page)))
        }
    }
}

/// The files the site publishes for `page`: its own, and a redirect at
/// each old address sending readers on to it, by however many redirects.
fn files(renderer: &Renderer, page: PageId) -> Vec<String> {
    let site = renderer.site();
    let mut addresses = vec![site.pages.get(page).address.clone()];
    loop {
        let more: Vec<_> = (site.redirects.iter())
            .filter(|redirect| !addresses.contains(&redirect.from))
            .filter(|redirect| match &redirect.to {
                SentTo::Page => redirect.page == page,
                SentTo::Site { address, .. } => addresses.contains(address),
                SentTo::Elsewhere(_) => false,
            })
            .map(|redirect| redirect.from.clone())
            .collect();
        if more.is_empty() {
            break;
        }
        addresses.extend(more);
    }
    addresses.iter().map(|address| address.file()).collect()
}
