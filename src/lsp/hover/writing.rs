//! Hover in a page's writing, read as the build reads it: the page or
//! heading a link, a link's definition or an `@url(...)` leads to, the
//! snippet a call names, and the file beside the page a picture or a
//! call's value names.

use std::ops::Range;

use super::{Goes, Said};
use crate::address::Address;
use crate::ids::PageId;
use crate::lsp::complete::Known;
use crate::lsp::complete::writing::{takes_written, unnamed};
use crate::markdown::{self, Call, Document, Kind, Node};
use crate::problem::Sort;
use crate::site::SentTo;
use crate::source::{Sources, Span};
use crate::templates::{Renderer, Snippet};
use crate::wording;

/// What is said of the character at `offset` in the writing at `body` of
/// `text`, the text of the page at `rel`, and the range it is said of.
pub(super) fn said(
    site: &Known,
    rel: &str,
    text: &str,
    body: Span,
    offset: usize,
) -> Option<(Range<usize>, Said)> {
    let renderer = site.renderer?;
    let mut sources = Sources::default();
    let source = sources.add(rel, text.to_owned());
    let wraps = |name: &str| renderer.wraps(name);
    let base = renderer.settings().base();
    let document = markdown::read(&sources, source, body, &wraps, &base);
    let folder = rel.strip_prefix("content/").unwrap_or(rel);
    let folder = folder.rfind('/').map_or("", |at| &folder[..=at]);
    let reading = Reading {
        renderer,
        page: site.page,
        folder,
        base: &base,
    };
    let holds = |span: Span| span.start as usize <= offset && offset < span.end as usize;
    if let Some(stands) = (document.in_site.iter()).find(|stands| holds(stands.place.span)) {
        // The call is handed the address from the top of the site.
        let written = stands.address.strip_prefix(base.as_str());
        let said = reading.address(&stands.address, written.unwrap_or(&stands.address))?;
        return Some((stands.place.span.range(), said));
    }
    if let Some(node) = innermost(&document, offset) {
        return match &node.kind {
            Kind::Call(call) => reading.call(call, node, offset),
            Kind::Link { destination, .. } => {
                let said = reading.address(destination, destination)?;
                Some((node.place.span.range(), said))
            }
            Kind::Image { destination, .. } => {
                let said = reading.beside(destination, Sort::Picture)?;
                Some((node.place.span.range(), said))
            }
            _ => None,
        };
    }
    let definition = (document.definitions.iter()).find(|line| holds(line.place.span))?;
    let said = reading.address(&definition.destination, &definition.destination)?;
    Some((definition.place.span.range(), said))
}

/// The innermost call, link or picture holding `offset`, the head of a
/// call that wraps writing alone being the call's.
fn innermost(document: &Document, offset: usize) -> Option<&Node> {
    let mut found = None;
    for top in std::iter::once(&document.root).chain(&document.notes) {
        top.walk(&mut |node| {
            let span = match &node.kind {
                Kind::Call(call) => call.head.unwrap_or(node.place.span),
                Kind::Link { .. } | Kind::Image { .. } => node.place.span,
                _ => return,
            };
            if span.start as usize <= offset && offset < span.end as usize {
                found = Some(node);
            }
        });
    }
    found
}

/// What is said of the snippet `snippet`, whose file is at `path`, as a
/// call to it is written: `wraps` where the call wraps the writing below
/// it.
pub(super) fn snippet_said(snippet: &Snippet, path: &str, wraps: bool) -> Said {
    let mut body = vec![wording::its_file(path)];
    if wraps {
        body.push(wording::WRAPS_WRITING.to_owned());
    }
    let head = format!("{}({})", snippet.name, takes_written(snippet));
    Said {
        head: wording::hover_snippet(&head),
        body,
        more: None,
        to: Some(Goes::File(path.to_owned())),
    }
}

/// What the page's writing is read against.
pub(super) struct Reading<'a> {
    pub(crate) renderer: &'a Renderer,
    /// The page being written, where the site as last read holds it.
    pub(crate) page: Option<PageId>,
    /// The folder under `content/` the page's file is in, ending with
    /// `/`, or empty at the top.
    pub(crate) folder: &'a str,
    /// The path the site is published under.
    pub(crate) base: &'a str,
}

impl Reading<'_> {
    /// What is said of the call `call`, written at `node`: of the file
    /// beside the page where `offset` is in a value naming one, and of
    /// its snippet otherwise.
    fn call(&self, call: &Call, node: &Node, offset: usize) -> Option<(Range<usize>, Said)> {
        let snippets = self.renderer.snippets();
        let (snippet, path) = snippets.iter().find(|(s, _)| s.name == call.name.value)?;
        let mut place = 0;
        for argument in &call.arguments {
            let param = match &argument.name {
                Some(name) => Some(name.value.clone()),
                None => {
                    place += 1;
                    unnamed(snippet, place - 1)
                }
            };
            let Some(at) = argument.value.origin.place else {
                continue;
            };
            if !at.span.range().contains(&offset) {
                continue;
            }
            let sort = (snippet.files.iter())
                .find(|(name, _)| Some(name) == param.as_ref())
                .map(|(_, sort)| *sort)?;
            let markdown::Value::Text(name) = &argument.value.value else {
                return None;
            };
            return Some((at.span.range(), self.beside(name, sort)?));
        }
        let span = call.head.unwrap_or(node.place.span);
        Some((span.range(), snippet_said(snippet, path, call.wraps())))
    }

    /// What is said of `name`, written as the address of a file beside the
    /// page of `sort`, where the site has one.
    fn beside(&self, name: &str, sort: Sort) -> Option<Said> {
        let path = format!("{}{name}", self.folder);
        let site = self.renderer.site();
        if name.contains('/') || !site.files.iter().any(|file| file.path == path) {
            return None;
        }
        let picture = sort == Sort::Picture || crate::templates::fits(Sort::Picture, name);
        let path = format!("content/{path}");
        Some(Said {
            head: wording::hover_beside(name, picture),
            body: vec![wording::its_file(&path)],
            more: None,
            to: Some(Goes::File(path)),
        })
    }

    /// What is said of `address`, as a link in the writing leads to it: the
    /// page, or the heading of a page, the site publishes there or sends
    /// readers on to from there, named as `written`.
    pub(super) fn address(&self, address: &str, written: &str) -> Option<Said> {
        let site = self.renderer.site();
        let (path, heading) = match address.split_once('#') {
            Some((path, heading)) => (path, Some(heading)),
            None => (address, None),
        };
        let page = match path {
            "" => self.page?,
            path if markdown::from_the_top(path) => self.landing(path)?,
            path => self.relative(path)?,
        };
        let held = site.pages.get(page);
        let title = match &held.title {
            Some(title) => title.value.clone(),
            None => self.renderer.named(page).0,
        };
        let file = held.source.map(|_| self.renderer.page_path(page));
        let Some(heading) = heading else {
            let mut body = vec![wording::titled(&title)];
            body.extend(file.as_ref().map(|file| wording::its_file(file)));
            return Some(Said {
                head: wording::hover_page(written),
                body,
                more: None,
                to: file.map(Goes::File),
            });
        };
        let heading = (self.renderer.headings(page).into_iter()).find(|h| h.id == heading)?;
        let mut body = vec![wording::heading_words(&heading.words)];
        if Some(page) != self.page
            && let Some(file) = file
        {
            body.push(wording::heading_on(&title, &file));
        }
        Some(Said {
            head: wording::hover_heading(written),
            body,
            more: None,
            to: Some(Goes::At(heading.place)),
        })
    }

    /// The page `path`, an address written from the page being written,
    /// leads to, as a browser follows it.
    fn relative(&self, path: &str) -> Option<PageId> {
        if path.contains(':') {
            return None;
        }
        let site = self.renderer.site();
        let from = site.pages.get(self.page?).address.as_str();
        let top = url::Url::parse("http://site.invalid/").ok()?;
        let reached = top
            .join(&format!("{}{from}", self.base))
            .ok()?
            .join(path)
            .ok()?;
        self.landing(reached.path())
    }

    /// The page published at `path`, from the top of the host, or the one
    /// readers are sent on to from there.
    fn landing(&self, path: &str) -> Option<PageId> {
        let site = self.renderer.site();
        let mut at = path.to_owned();
        // However many redirects lead on, short of a ring.
        for _ in 0..=site.redirects.len() {
            let here = |address: &Address| format!("{}{}", self.base, address.as_str()) == at;
            let landed = (site.pages.iter()).find(|(_, page)| page.lands() && here(&page.address));
            if let Some((id, _)) = landed {
                return Some(id);
            }
            let redirect = (site.redirects.iter()).find(|redirect| here(&redirect.from))?;
            at = match &redirect.to {
                SentTo::Page => return Some(redirect.page),
                SentTo::Site { address, .. } => format!("{}{}", self.base, address.as_str()),
                SentTo::Elsewhere(_) => return None,
            };
        }
        None
    }
}
