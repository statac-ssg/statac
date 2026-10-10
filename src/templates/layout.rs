//! Which template builds a page, and what chose it.
//!
//! A page names its template with `layout:`. Failing that, a `type:`
//! naming one of the site's templates chooses it. Failing that, a
//! `layout:` handed down from a folder does, except to a page speaking for
//! a list, which stays what it is. A page that names none is built with
//! `home` where it speaks for the site's own list, `folder` where it
//! speaks for any other list, the pages Statac makes for tags and series
//! among them, and `page` for any other. A page making pages from a table
//! speaks for their list: its `layout:` builds the pages it makes.

use super::Templates;
use super::files::{Role, template_path};
use crate::content::words::PageWord;
use crate::origin::{Arrival, Origin};
use crate::problem::{Chooser, Kind, Problem};
use crate::site::{Page, Site};
use crate::source::Sources;

/// The template a page is built with, and where the choice came from.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Chosen {
    /// The template's name, as `extends` and `layout:` write it.
    pub name: String,
    /// Where the name is written; a default where nothing named it.
    pub origin: Origin,
}

/// Chooses the template for `page`, or says why it can't be had.
pub(super) fn choose(
    templates: &Templates,
    sources: &Sources,
    site: &Site,
    page: &Page,
) -> Result<Chosen, Problem> {
    let named = |word: PageWord| {
        page.words.text(word).map(|text| {
            let name = text.value.strip_suffix(".html").unwrap_or(text.value);
            Chosen {
                name: name.to_owned(),
                origin: text.origin,
            }
        })
    };
    let makes = page.makes_pages();
    let layout = named(PageWord::Layout).filter(|chosen| !chosen.name.is_empty());
    let written = layout
        .clone()
        .filter(|chosen| chosen.origin.arrival == Arrival::Written && !makes);
    let typed = named(PageWord::Type).filter(|chosen| templates.is_layout(&chosen.name));
    let face = page.speaks_for_a_list();
    let chosen = written
        .or(typed)
        .or(layout.filter(|_| !face))
        .unwrap_or_else(|| {
            let home = |list| site.is_home(list);
            let top = (page.place == crate::site::PagePlace::Folder && home(page.folder))
                || page.speaks_for.is_some_and(home);
            let name = match (face, top) {
                (true, true) => "home",
                (true, false) => "folder",
                (false, _) => "page",
            };
            Chosen {
                name: name.to_owned(),
                origin: Origin::defaulted(),
            }
        });
    if templates.is_layout(&chosen.name) {
        return Ok(chosen);
    }
    let layouts = templates
        .files
        .iter()
        .filter(|f| f.role == Role::Template)
        .map(|f| f.name.as_str());
    let nearest = crate::suggest::nearest(&chosen.name, layouts).map(str::to_owned);
    // A list's page Statac made has no file to place it in, and is named
    // as the page being built instead.
    let place = chosen.origin.place.or_else(|| page.head(sources));
    Err(Problem::placed(
        Kind::MissingTemplate {
            path: template_path(&chosen.name),
            chosen: match chosen.origin.arrival {
                Arrival::Defaulted => Chooser::Default,
                _ => Chooser::Written,
            },
            nearest,
        },
        place,
    ))
}
