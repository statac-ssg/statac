//! The page the panel is on, as facts for the panel to lay out: what
//! builds it and which lists hold it, what its build read, and every name
//! a template can write on it, with whether its build read each.

use std::collections::BTreeSet;
use std::path::Path;

use super::pieced;
use super::problems::lines;
use crate::content::address::on_the_wire;
use crate::json;
use crate::origin::Arrival;
use crate::reads::{Asked, Key, Reads};
use crate::report::{Report, shown};
use crate::source::Place;
use crate::templates::{Explained, Held, Holds, Neighbour, Reached, Renderer, Row, SEARCH_BOX};
use crate::wording;

/// The page as the last round to build it left it.
pub(super) struct On<'a> {
    pub report: &'a Report<'a>,
    pub renderer: &'a Renderer,
    pub explained: &'a Explained,
    /// What its output read while it was built, where that is known.
    pub reads: Option<&'a Reads>,
    /// What its output asked for by name while it was built.
    pub asked: Option<&'a Asked>,
    /// The site's folder.
    pub site: &'a Path,
}

impl On<'_> {
    /// What builds the page and holds it, and what its build read, as
    /// JSON; `waiting` is the day it waits for, where it does.
    pub fn facts(&self, draft: bool, waiting: Option<&str>) -> String {
        let explained = self.explained;
        let report = self.report;
        let title = explained.title.as_deref().map(shown);
        let file = explained.source.map(|source| report.file(source));
        let language = format!(
            "{{\"tag\":{},\"direction\":{}}}",
            json::string(&shown(&explained.language)),
            json::string(explained.direction.as_str()),
        );
        let layout = match &explained.layout {
            Some(chosen) => {
                let mut files = explained.templates.iter().map(|&at| report.file(at));
                let file = files.next();
                let extends: Vec<String> = files.map(|file| json::string(&file)).collect();
                format!(
                    "{{\"name\":{},\"file\":{},\"extends\":[{}],\"chosenAt\":{},\"handedDown\":{}}}",
                    json::string(&shown(&chosen.name)),
                    or_null(file.as_deref()),
                    extends.join(","),
                    self.place(chosen.origin.place),
                    chosen.origin.arrival == Arrival::HandedDown,
                )
            }
            None => "null".to_owned(),
        };
        let lists: Vec<String> = (explained.lists.iter())
            .map(|member| {
                let said = wording::panel_list(member.word, &shown(&member.list));
                format!(
                    "{{\"said\":{},\"at\":{},\"of\":{},\"previous\":{},\"next\":{}}}",
                    pieced(&said),
                    member.at,
                    member.of,
                    link(member.previous.as_ref()),
                    link(member.next.as_ref()),
                )
            })
            .collect();
        let feeds: Vec<String> = (explained.feeds.iter())
            .map(|feed| json::string(&shown(feed)))
            .collect();
        let (files, also) = self.read();
        let files: Vec<String> = files.iter().map(|file| json::string(file)).collect();
        let also: Vec<String> = also.iter().map(|said| pieced(said)).collect();
        format!(
            "{{\"title\":{},\"draft\":{draft},\"waiting\":{},\"file\":{},\"address\":{},\
             \"language\":{language},\"layout\":{layout},\"lists\":[{}],\"feeds\":[{}],\
             \"reads\":{{\"files\":[{}],\"also\":[{}]}}}}",
            or_null(title.as_deref()),
            or_null(waiting),
            or_null(file.as_deref()),
            json::string(&shown(&explained.address)),
            lists.join(","),
            feeds.join(","),
            files.join(","),
            also.join(","),
        )
    }

    /// Every name a template can write on the page, a group for `page`,
    /// `site` and `site.data`, as JSON.
    pub fn names(&self) -> String {
        let groups: Vec<String> = (self.explained.groups.iter())
            .map(|group| {
                let held = |row: &&Row| row.held != Held::Nothing;
                let rows: Vec<String> = group.rows.iter().map(|row| self.row(row)).collect();
                format!(
                    "{{\"group\":{},\"count\":{},\"held\":{},\"rows\":[{}]}}",
                    json::string(group.of),
                    group.rows.len(),
                    group.rows.iter().filter(held).count(),
                    rows.join(","),
                )
            })
            .collect();
        format!("[{}]", groups.join(","))
    }

    fn row(&self, row: &Row) -> String {
        let has: Vec<String> = row.has.iter().map(|name| json::string(name)).collect();
        let used = self.asked.is_some_and(|asked| match &row.reached {
            Reached::Page(name) => asked.asked_page(*name),
            Reached::Site(name) => asked.asked_site(*name),
            Reached::Data(files) => files.iter().any(|&file| asked.reached(file)),
        });
        // A call is written with its brackets, which hold what it takes.
        let written = match row.holds {
            Holds::Call => {
                let takes = !row.name.ends_with(&format!(".{SEARCH_BOX}"));
                let brackets = if takes { "(\u{2026})" } else { "()" };
                format!(
                    ",\"written\":{}",
                    json::string(&shown(&format!("{}{brackets}", row.name)))
                )
            }
            _ => String::new(),
        };
        format!(
            "{{\"name\":{}{written},\"holds\":{},\"value\":{},\"empty\":{},\"has\":[{}],\"from\":{},\"used\":{used}}}",
            json::string(&shown(&row.name)),
            json::string(wording::panel_holds(row.holds)),
            json::string(&value(row)),
            row.held == Held::Nothing,
            has.join(","),
            self.written_at(row.from),
        )
    }

    /// A place as its file and line and that line as written, or null.
    fn written_at(&self, place: Option<Place>) -> String {
        let Some(place) = place else {
            return "null".to_owned();
        };
        let excerpt = self.report.excerpt(place, false);
        format!(
            "{{\"file\":{},\"line\":{},\"excerpt\":{}}}",
            json::string(&self.report.file(place.source)),
            excerpt.first,
            lines(&excerpt),
        )
    }

    /// A place as its file and line, or null.
    fn place(&self, place: Option<Place>) -> String {
        match place {
            Some(place) => format!(
                "{{\"file\":{},\"line\":{}}}",
                json::string(&self.report.file(place.source)),
                self.report.first_line(place),
            ),
            None => "null".to_owned(),
        }
    }

    /// The site's files the page's build read, each once, and what else
    /// it read, in a few words each.
    fn read(&self) -> (BTreeSet<String>, Vec<String>) {
        let mut files = BTreeSet::new();
        let mut also: Vec<String> = Vec::new();
        for &key in self.reads.map_or(&[][..], Reads::keys) {
            match self.behind(key) {
                Behind::Files(behind) => files.extend(behind),
                Behind::Said(said) if !also.contains(&said) => also.push(said),
                Behind::Said(_) => {}
            }
        }
        (files, also)
    }

    /// What stands behind a read of `key`.
    fn behind(&self, key: Key) -> Behind {
        let (report, renderer) = (self.report, self.renderer);
        let site = renderer.site();
        let settings = || crate::config::FILE_NAME.to_owned();
        let files = match key {
            Key::Field(page, _) | Key::Body(page) | Key::Card(page) => {
                let source = site.pages.get(page).source;
                source
                    .map(|source| report.file(source))
                    .into_iter()
                    .collect()
            }
            Key::Template(id) => vec![report.file(renderer.template_source(id))],
            Key::Data(id) => vec![report.file(site.data.file(id).source)],
            Key::Asset(id) => (renderer.assets().iter())
                .filter(|&(other, _)| other == id)
                .map(|(_, asset)| shown(&asset.source))
                .collect(),
            Key::Picture(id) => {
                let picture = site.pictures.get(id);
                let sidecar = picture.sidecar.as_ref();
                let sidecar = sidecar.map(|said| report.file(said.source));
                [shown(&picture.shown())]
                    .into_iter()
                    .chain(sidecar)
                    .collect()
            }
            Key::Beside(at) => (site.files.get(at as usize).into_iter())
                .map(|file| shown(&format!("content/{}", file.path)))
                .collect(),
            Key::Static(at) => (renderer.assets().statics().get(at as usize).into_iter())
                .map(|file| {
                    let path = file.path();
                    shown(
                        &path
                            .strip_prefix(self.site)
                            .unwrap_or(path)
                            .to_string_lossy(),
                    )
                })
                .collect(),
            Key::Setting(_) => vec![settings()],
            // Its tag is declared in the settings, and its words are in a
            // file of the site's where it has one.
            Key::Language(id) => match &renderer.dates().language(id).whence {
                crate::language::Whence::Site(path) => vec![settings(), shown(path)],
                _ => vec![settings()],
            },
            Key::Fetched(fetched) => (site.fetched.files.iter())
                .filter(|(path, _)| crate::fetched::key(path) == fetched)
                .map(|(path, _)| shown(&format!("{}/{path}", crate::fetched::FOLDER)))
                .collect(),
            Key::Pages => return Behind::Said(wording::PANEL_READ_PAGES.to_owned()),
            Key::Lists | Key::ListsIn(_) | Key::Spellings | Key::Folders(_) | Key::Home(_) => {
                return Behind::Said(wording::PANEL_READ_LISTS.to_owned());
            }
            Key::ListPage(list, _) | Key::ListHead(list) | Key::Members(list) | Key::List(list) => {
                let (word, name) = crate::feeds::described(site, renderer.sources(), list);
                return Behind::Said(wording::panel_read_list(word, &shown(&name)));
            }
            Key::Neighbours(_) => return Behind::Said(wording::PANEL_READ_ORDER.to_owned()),
            Key::Translations(_) => {
                return Behind::Said(wording::PANEL_READ_TRANSLATIONS.to_owned());
            }
            Key::FieldOfEvery(name) | Key::FieldOfWriters(_, _, name) => {
                let name = shown(&renderer.name_of(name));
                return Behind::Said(wording::panel_read_field(&name));
            }
            Key::Pictures | Key::Files => {
                return Behind::Said(wording::PANEL_READ_FILES.to_owned());
            }
            Key::Mentions(_) => return Behind::Said(wording::PANEL_READ_MENTIONS.to_owned()),
            Key::Today => return Behind::Said(wording::PANEL_READ_TODAY.to_owned()),
        };
        Behind::Files(files)
    }
}

/// What stands behind a read: files of the site's, or where no one file
/// does, what was read in a few words.
enum Behind {
    Files(Vec<String>),
    Said(String),
}

/// A post beside the page, as a link to it.
fn link(neighbour: Option<&Neighbour>) -> String {
    match neighbour {
        Some(neighbour) => format!(
            "{{\"text\":{},\"href\":{}}}",
            json::string(&shown(&neighbour.called)),
            json::string(&on_the_wire(&neighbour.address)),
        ),
        None => "null".to_owned(),
    }
}

fn or_null(text: Option<&str>) -> String {
    text.map_or_else(|| "null".to_owned(), json::string)
}

/// What a name holds on the page, as the panel shows it.
pub(super) fn value(row: &Row) -> String {
    match &row.held {
        Held::Shown(text) => shown(text),
        Held::Count(count) => wording::panel_count(row.holds, *count),
        Held::Fields => wording::PANEL_FIELDS_BELOW.to_owned(),
        Held::Call => wording::PANEL_A_CALL.to_owned(),
        Held::More(more) => wording::panel_more_fields(*more),
        Held::Nothing => wording::PANEL_NO_VALUE.to_owned(),
    }
}
