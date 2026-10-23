//! Renaming a page's file, or a folder of pages: the edits that keep each
//! link in the site leading to the pages it moves, and that send readers
//! on from each address a page leaves. A page's new address is found by
//! reading the site as if renamed, so it is whatever the build would give
//! it. Each link keeps the form it is written in: from the top of the
//! host, from the top of the site in `@url(...)` and `url(...)`, whole, or
//! from the page it is written in.

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::path::Path;

use super::references;
use crate::content::address::is_full;
use crate::content::words::PageWord;
use crate::content::{self, Run, Texts};
use crate::links;
use crate::origin::Arrival;
use crate::site::PageId;
use crate::source::{Place, Span};
use crate::templates::Renderer;
use crate::unsaved::Unsaved;

/// The edits that rename the site at `root`, as `renderer` last read it,
/// by `moves`, each a file or folder from its path in the site to its new
/// one: each the text written in place of what is at its place, which is
/// empty where the text goes in. The site's files are read as `unsaved`
/// has them.
pub(super) fn renamed(
    renderer: &Renderer,
    root: &Path,
    unsaved: Unsaved,
    moves: &[(String, String)],
) -> Vec<(Place, String)> {
    let under = |path: &str| {
        let path = path.strip_prefix("content/")?.trim_end_matches('/');
        (!path.is_empty()).then(|| path.to_owned())
    };
    let renamed: Vec<(String, String)> = (moves.iter())
        .filter_map(|(from, to)| Some((under(from)?, under(to)?)))
        .filter(|(from, to)| from != to)
        .collect();
    if renamed.is_empty() {
        return Vec::new();
    }
    let mut texts = Texts::default();
    texts.set_unsaved(unsaved);
    let run = Run {
        renamed: renamed.clone(),
        ..Run::default()
    };
    let read = content::read_with(root, &run, &mut texts);
    let site = renderer.site();
    let own = |page: &crate::site::Page| page.source.is_some() && page.made.is_none();
    let mut moved: HashMap<PageId, String> = HashMap::new();
    for (id, page) in site.pages.iter().filter(|(_, page)| own(page)) {
        let to = content::renamed_to(page.path.clone(), &renamed);
        if to == page.path {
            continue;
        }
        let after = (read.site.pages.iter()).find(|(_, page)| own(page) && page.path == to);
        if let Some((_, after)) = after
            && after.address != page.address
        {
            moved.insert(id, after.address.as_str().to_owned());
        }
    }
    let base = renderer.settings().base();
    let top = format!("{base}/");
    let sources = renderer.sources();
    let address = |page: PageId| match moved.get(&page) {
        Some(after) => after.as_str(),
        None => site.pages.get(page).address.as_str(),
    };
    let from = |link: &references::Link| match link.from {
        Some(page) => format!("{base}{}", address(page)),
        None => top.clone(),
    };
    let mut edits = Vec::new();
    let mut pages: Vec<_> = moved.keys().copied().collect();
    pages.sort_unstable();
    for &page in &pages {
        let before = &site.pages.get(page).address;
        for link in references::links(renderer, vec![before.file()], None) {
            let written = sources.spelling(link.place);
            if let Some(text) = rewritten(written, link.helper, address(page), &base, &from(&link))
            {
                edits.push((link.place, text));
            }
        }
        edits.extend(redirect(renderer, page, before.as_str()));
    }
    // A moved page's links to a page that stays, written from where the
    // page was, are written again from where it goes.
    let follow = links::Site::new(
        renderer.settings(),
        (site.pages.iter())
            .filter(|(_, page)| page.lands())
            .map(|(_, page)| page.address.file()),
    );
    let landed: HashMap<String, PageId> = (site.pages.iter())
        .filter(|(_, page)| page.lands())
        .map(|(id, page)| (page.address.file(), id))
        .collect();
    for &page in &pages {
        let writing = renderer.writing_built(page);
        let was = format!("{base}{}", site.pages.get(page).address.as_str());
        let is = format!("{base}{}", address(page));
        let handed = (writing.handed.iter()).filter(|linked| linked.address.contains('/'));
        for linked in writing.written.iter().chain(handed) {
            let written = sources.spelling(linked.place);
            if written.starts_with("@url(") || is_full(&linked.address) {
                continue;
            }
            let Some((file, _)) = follow.lands_on(&linked.address, &was) else {
                continue;
            };
            let Some(&to) = landed.get(&file).filter(|to| !moved.contains_key(to)) else {
                continue;
            };
            if let Some(text) = rewritten(written, false, address(to), &base, &is) {
                edits.push((linked.place, text));
            }
        }
    }
    edits.sort_by_key(|(place, _)| (place.source, place.span.start));
    edits.dedup_by_key(|(place, _)| *place);
    edits
}

/// `written`, a link as it is written at its place, leading instead to
/// `to`, an address from the top of the site published under `base`, in
/// the form it is written in: from the top of the host, from the top of
/// the site where `helper` is handed it so, whole, or from `from`, the
/// address it is written at. None where it stays as it is.
fn rewritten(written: &str, helper: bool, to: &str, base: &str, from: &str) -> Option<String> {
    let (call, inner) = match written.strip_prefix("@url(") {
        Some(rest) => (true, rest.strip_suffix(')')?),
        None => (false, written),
    };
    let trimmed = inner.trim();
    let quoted = match trimmed.chars().next() {
        Some(quote @ ('"' | '\'')) if trimmed.len() > 1 && trimmed.ends_with(quote) => Some(quote),
        _ => None,
    };
    let address = match quoted {
        Some(_) => &trimmed[1..trimmed.len() - 1],
        None => trimmed,
    };
    let cut = address.find(['#', '?']).unwrap_or(address.len());
    let (path, rest) = address.split_at(cut);
    if path.is_empty() || path.starts_with("//") {
        return None;
    }
    let new = if helper || call {
        to.to_owned()
    } else if let Some(scheme) = path.find("://") {
        let host = path[scheme + 3..]
            .find('/')
            .map_or(path.len(), |at| scheme + 3 + at);
        format!("{}{base}{to}", &path[..host])
    } else if path.starts_with('/') {
        format!("{base}{to}")
    } else {
        relative(from, &format!("{base}{to}"))
    };
    if new == path {
        return None;
    }
    let address = format!("{new}{rest}");
    let address = match quoted {
        Some(quote) => format!("{quote}{address}{quote}"),
        None => address,
    };
    let at = inner.find(trimmed).unwrap_or(0);
    let inner = format!("{}{address}{}", &inner[..at], &inner[at + trimmed.len()..]);
    Some(match call {
        true => format!("@url({inner})"),
        false => inner,
    })
}

/// The path that leads from the address `from` to the address `to`, both
/// from the top of the host, as a browser follows it.
fn relative(from: &str, to: &str) -> String {
    let folder = &from[..from.rfind('/').map_or(0, |at| at + 1)];
    let from: Vec<&str> = folder.split('/').filter(|part| !part.is_empty()).collect();
    let to: Vec<&str> = to.trim_start_matches('/').split('/').collect();
    let (name, folders) = to.split_last().expect("split gives one at least");
    let shared = (from.iter().zip(folders))
        .take_while(|(from, to)| from == to)
        .count();
    let mut path = "../".repeat(from.len() - shared);
    for folder in &folders[shared..] {
        path.push_str(folder);
        path.push('/');
    }
    path.push_str(name);
    match path.is_empty() {
        true => "./".to_owned(),
        false => path,
    }
}

/// The edit adding `old`, an address `page` leaves, to its
/// `redirect_from`: to the list it writes, or as a line of its own.
fn redirect(renderer: &Renderer, page: PageId, old: &str) -> Option<(Place, String)> {
    let held = renderer.site().pages.get(page);
    let source = held.source?;
    let text = renderer.sources().text(source);
    let at = |start: usize, text: String| {
        let span = Span::new(start, start);
        Some((Place { source, span }, text))
    };
    let written = (held.words.get(PageWord::RedirectFrom))
        .filter(|from| from.origin.arrival == Arrival::Written)
        .and_then(|from| Some((from, from.origin.place?)))
        .filter(|(_, place)| place.source == source);
    if let Some((from, place)) = written {
        let listed = from.value.as_texts().unwrap_or_default();
        if listed.iter().any(|item| item.value == old) {
            return None;
        }
        let value = &text[place.span.range()];
        let start = place.span.start as usize;
        let end = start + value.trim_end().len();
        if value.starts_with('[') {
            let close = start + value.rfind(']')?;
            let last = start + text[start + 1..close].trim_end().len() + 1;
            let comma = match last == start + 1 {
                true => "",
                false => ", ",
            };
            return at(last, format!("{comma}{old}"));
        }
        if value.starts_with('-') {
            let line = text[..start].rfind('\n').map_or(0, |at| at + 1);
            let indent = " ".repeat(start - line);
            return at(end, format!("\n{indent}- {old}"));
        }
        let span = Span::new(start, end);
        let list = format!("[{}, {old}]", &text[start..end]);
        return Some((Place { source, span }, list));
    }
    let opened = text.trim_start_matches('\u{feff}');
    let mark = text.len() - opened.len();
    if !opened.starts_with("---") {
        return at(mark, format!("---\nredirect_from: [{old}]\n---\n"));
    }
    let head = text[..held.body.start as usize].trim_end_matches(['\n', '\r']);
    let closing = head.rfind('\n').map_or(0, |at| at + 1);
    if !head[closing..].starts_with("---") || closing <= mark {
        return None;
    }
    at(closing, format!("redirect_from: [{old}]\n"))
}
