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
//!
//! Where the site lacks the template so chosen, a page with content of
//! its own is built with `page`, and where the site lacks that too, as its
//! content alone. A page Statac made, which has none, needs its template.

use super::Templates;
use super::files::{Role, template_path};
use super::render::Shared;
use crate::config::words::PageWord;
use crate::ids::PageId;
use crate::origin::{Arrival, Origin};
use crate::problem::{Chooser, Kind, Problem};
use crate::reads::Key;
use crate::site::{Page, Site};
use crate::source::Sources;

/// The template a page is built with, and where the choice came from.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Chosen {
    /// The template's name, as `extends` and `layout:` write it.
    pub(crate) name: String,
    /// Where the name is written; a default where nothing named it.
    pub(crate) origin: Origin,
    /// The template a default choice would have been, `home` or
    /// `folder`, where the site lacks it and `page` builds in its place.
    pub(crate) instead_of: Option<&'static str>,
}

/// The names a page's template is chosen by. What it chooses turns on more
/// than the page's answer to them: on which templates the site has, and
/// on where the page stands.
pub(crate) const CHOOSING: &[&str] = &["layout", "type"];

/// Records, through `read`, reading what chooses `page`'s template.
pub(super) fn read_choice(shared: &Shared, page: PageId, mut read: impl FnMut(Key)) {
    for name in CHOOSING {
        read(Key::Field(page, shared.name_id(name)));
    }
}

/// Chooses the template for `page`, or says why it can't be had: none
/// for a page with content of its own that the site has no template for,
/// which is built as that content alone.
pub(super) fn choose(
    templates: &Templates,
    sources: &Sources,
    site: &Site,
    page: &Page,
) -> Result<Option<Chosen>, Problem> {
    let named = |word: PageWord| {
        page.words.text(word).map(|text| Chosen {
            name: super::files::named(text.value).to_owned(),
            origin: text.origin,
            instead_of: None,
        })
    };
    let makes = page.makes_pages();
    let layout = named(PageWord::Layout).filter(|chosen| !chosen.name.is_empty());
    let written = layout
        .clone()
        .filter(|chosen| chosen.origin.arrival == Arrival::Written && !makes);
    let typed = named(PageWord::Type).filter(|chosen| templates.is_layout(&chosen.name));
    let face = page.speaks_for_a_list();
    let chosen = match written.or(typed).or(layout.filter(|_| !face)) {
        Some(chosen) => chosen,
        None => {
            let home = |list| site.is_home(list);
            let top = (page.place == crate::site::PagePlace::Folder && home(page.folder))
                || page.speaks_for.is_some_and(home);
            let name = match (face, top) {
                (true, true) => "home",
                (true, false) => "folder",
                (false, _) => "page",
            };
            let defaulted = |name: &str, instead_of| Chosen {
                name: name.to_owned(),
                origin: Origin::defaulted(),
                instead_of,
            };
            // A list's page Statac made has no file, and a page made from
            // a table or for a post has no content of its own.
            let own_content = page.source.is_some() && page.made.is_none();
            match (templates.is_layout(name), templates.is_layout("page")) {
                (true, _) => return Ok(Some(defaulted(name, None))),
                _ if !own_content => defaulted(name, None),
                (false, true) => return Ok(Some(defaulted("page", Some(name)))),
                (false, false) => return Ok(None),
            }
        }
    };
    if templates.is_layout(&chosen.name) {
        return Ok(Some(chosen));
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
                Arrival::Defaulted if page.source.is_none() => Chooser::Made,
                Arrival::Defaulted => Chooser::Default,
                _ => Chooser::Written,
            },
            nearest,
        },
        place,
    ))
}
