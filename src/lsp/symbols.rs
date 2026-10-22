//! Symbols: the headings of a page, for the editor's outline of it, and
//! the site's pages by title and its snippets and templates by name, for
//! finding one from anywhere in the site.

#[cfg(test)]
mod tests;

use std::ops::Range;
use std::path::Path;

use crate::markdown;
use crate::source::Sources;
use crate::templates::Renderer;

/// A heading of a page, with the headings beneath it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) struct Heading {
    /// Its words, as plain text.
    pub words: String,
    pub level: u8,
    /// The heading itself, as offsets in the page's text.
    pub at: Range<usize>,
    /// The heading and the writing under it, up to the next heading at
    /// its level or above.
    pub section: Range<usize>,
    pub within: Vec<Heading>,
}

/// The headings of `text`, the text of `file` in the site at `site`, each
/// with those beneath it; none where the file is no page's. `known` is
/// the site as it was last read, where it has been.
pub(super) fn headings(
    site: &Path,
    file: &Path,
    text: &str,
    known: Option<&Renderer>,
) -> Option<Vec<Heading>> {
    let rel = file.strip_prefix(site).ok()?;
    let rel = rel.to_string_lossy().replace('\\', "/");
    if !rel.starts_with("content/") || !rel.ends_with(".md") {
        return None;
    }
    // A page whose front matter is never closed has no writing.
    let Ok(split) = crate::yaml::split(text) else {
        return Some(Vec::new());
    };
    let mut sources = Sources::default();
    let source = sources.add(rel.as_str(), text.to_owned());
    let wraps = |name: &str| known.and_then(|renderer| renderer.wraps(name));
    let base = known.map(|renderer| renderer.settings().base());
    let document = markdown::read(
        &sources,
        source,
        split.body,
        &wraps,
        base.as_deref().unwrap_or(""),
    );
    let found = markdown::headings(&document);
    let end = split.body.end as usize;
    let mut flat = Vec::with_capacity(found.len());
    for (at, heading) in found.iter().enumerate() {
        if heading.text.trim().is_empty() {
            continue;
        }
        let start = heading.place.span.start as usize;
        let next = (found[at + 1..].iter())
            .find(|next| next.level <= heading.level)
            .map_or(end, |next| next.place.span.start as usize);
        flat.push(Heading {
            words: heading.text.trim().to_owned(),
            level: heading.level,
            at: heading.place.span.range(),
            section: start..next,
            within: Vec::new(),
        });
    }
    Some(nested(flat))
}

/// `flat`, in the order written, each heading placed beneath the last
/// heading before it at a higher level.
fn nested(flat: Vec<Heading>) -> Vec<Heading> {
    let mut top: Vec<Heading> = Vec::new();
    for heading in flat {
        let mut level = &mut top;
        while level.last().is_some_and(|last| last.level < heading.level) {
            level = &mut level.last_mut().expect("just seen").within;
        }
        level.push(heading);
    }
    top
}

/// What a symbol of the site is.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(super) enum Sort {
    Page,
    Snippet,
    Template,
}

impl Sort {
    /// The protocol's number for the kind of symbol, which picks its
    /// icon.
    pub(super) fn number(self) -> u8 {
        match self {
            Sort::Page => 1,
            Sort::Template => 2,
            Sort::Snippet => 12,
        }
    }
}

/// A page, snippet or template of the site, by its name.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) struct Found {
    /// A page's title, or a snippet's or template's name.
    pub name: String,
    pub sort: Sort,
    /// The path of its file in the site.
    pub path: String,
}

/// Every page of the site that has a file of its own, by its title, and
/// every snippet and template, by its name, whose name holds the letters
/// of `query` in the order typed, whatever their case.
pub(super) fn found(renderer: &Renderer, query: &str) -> Vec<Found> {
    let wanted: Vec<char> = (query.chars())
        .filter(|c| !c.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect();
    let matches = |name: &str| {
        let mut letters = name.chars().flat_map(char::to_lowercase);
        wanted.iter().all(|c| letters.any(|letter| letter == *c))
    };
    let site = renderer.site();
    let pages = (site.pages.iter())
        .filter(|(_, page)| page.source.is_some())
        .map(|(id, page)| {
            let name = match &page.title {
                Some(title) => title.value.clone(),
                None => renderer.named(id).0,
            };
            (name, Sort::Page, renderer.page_path(id))
        });
    let snippets = (renderer.snippets().into_iter())
        .map(|(snippet, path)| (snippet.name.clone(), Sort::Snippet, path));
    let templates =
        (renderer.layouts().into_iter()).map(|(name, path)| (name, Sort::Template, path));
    (pages.chain(snippets).chain(templates))
        .filter(|(name, ..)| !name.trim().is_empty() && matches(name))
        .map(|(name, sort, path)| Found { name, sort, path })
        .collect()
}
