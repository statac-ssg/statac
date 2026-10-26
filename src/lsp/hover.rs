//! Hover: what the word under the editor's pointer is, and where what it
//! names is written, to go to. In the site's
//! settings and in a page's front matter: what each of Statac's words
//! does, what a value of the author's own is, and what the site holds of
//! a tag, a series or a template a page names. In the writing: the page
//! a link leads to, the snippet a call names and the file beside the page
//! a picture or a call's value names. In a template or a snippet: what a
//! name or field holds where it is read, the snippet a call names, and
//! the page or template a string names.

mod template;
#[cfg(test)]
mod tests;
mod writing;

use std::ops::Range;
use std::path::Path;

use super::complete::Known;
use super::complete::spot::{self, Under};
use crate::config::words::{Holder, PageWord, is_statacs};
use crate::problem::Code;
use crate::site::ListKey;
use crate::source::{Place, Sources, Span};
use crate::templates::{Renderer, SNIPPETS, TEMPLATES};
use crate::wording::{self, Described};

/// What is said of what the pointer is over.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) struct Hover {
    /// What it is said of, as offsets in the file's text.
    pub(crate) range: Range<usize>,
    /// What it is, in a line.
    pub(crate) head: String,
    /// What more is said, a paragraph each.
    pub(crate) body: Vec<String>,
    /// Where the docs say more.
    pub(crate) more: Option<String>,
    /// Where what it names is written.
    pub(crate) to: Option<Goes>,
}

/// Where a file of the site holds what the pointer is over.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) enum Goes {
    /// The file at this path in the site, as a whole.
    File(String),
    /// This place among the files the site was last read from.
    At(Place),
}

/// What a hover says, before it is placed.
struct Said {
    head: String,
    body: Vec<String>,
    more: Option<String>,
    to: Option<Goes>,
}

impl Said {
    fn of(head: String, described: &Described) -> Said {
        Said {
            head,
            body: vec![described.text.clone()],
            more: described.more.clone(),
            to: None,
        }
    }

    fn placed(self, range: Range<usize>) -> Hover {
        Hover {
            range,
            head: self.head,
            body: self.body,
            more: self.more,
            to: self.to,
        }
    }
}

/// What the file being read holds.
#[derive(Copy, Clone, PartialEq, Eq)]
enum Holds {
    Settings,
    FrontMatter,
}

/// What is said of the character at `offset` in `text`, the text of
/// `file` in the site at `site`; none where it is nothing Statac knows.
/// `known` is the site as it was last read, where it has been.
pub(super) fn hover(
    site: &Path,
    file: &Path,
    text: &str,
    offset: usize,
    known: Option<&Renderer>,
) -> Option<Hover> {
    let rel = file.strip_prefix(site).ok()?;
    let rel = rel.to_string_lossy().replace('\\', "/");
    let offset = text.floor_char_boundary(offset.min(text.len()));
    let site = Known::of(known, &rel);
    let (holds, span) = match rel.as_str() {
        crate::config::FILE_NAME => (Holds::Settings, Span::new(0, text.len())),
        _ if rel.starts_with("content/") && rel.ends_with(".md") => {
            let yaml = match crate::yaml::split(text) {
                Ok(split)
                    if split
                        .front_matter
                        .is_none_or(|yaml| offset >= yaml.end as usize) =>
                {
                    let (range, said) = writing::said(&site, &rel, text, split.body, offset)?;
                    return Some(said.placed(range));
                }
                Ok(split) => split.front_matter?,
                Err(unclosed) => unclosed.rest,
            };
            (Holds::FrontMatter, yaml)
        }
        _ if [TEMPLATES, SNIPPETS]
            .iter()
            .any(|top| (rel.strip_prefix(top)).is_some_and(|rest| rest.starts_with('/'))) =>
        {
            let (range, said) = template::said(known?, &rel, text, offset)?;
            return Some(said.placed(range));
        }
        _ => return None,
    };
    let mut sources = Sources::default();
    let source = sources.add(rel.as_str(), text.to_owned());
    let under = spot::under(&sources, Place { source, span }, offset)?;
    let (range, said) = match under {
        Under::Name { path, span } => {
            let name = path.join(".");
            let said = match holds {
                Holds::Settings => setting(&site, &name),
                Holds::FrontMatter => field(&site, &name),
            };
            (span, said?)
        }
        Under::Value { path, span, text } => {
            let name = path.join(".");
            let said = match (holds, name.as_str()) {
                (Holds::Settings, "quiet") => kind_of_problem(&text),
                (Holds::Settings, name) => match name.strip_prefix("pages.") {
                    Some(field) => value(&site, field, &text),
                    None => None,
                },
                (Holds::FrontMatter, name) => {
                    value(&site, name.strip_prefix("pages.").unwrap_or(name), &text)
                }
            };
            (span, said?)
        }
    };
    Some(said.placed(range))
}

/// What is said of `name` as a line of the site's settings.
fn setting(site: &Known, name: &str) -> Option<Said> {
    if let Some(described) = wording::described_setting(name) {
        return Some(Said::of(wording::hover_setting(name), described));
    }
    if let Some(field) = name.strip_prefix("pages.") {
        return handed(site, field, false);
    }
    let renderer = site.renderer?;
    if let Some((tag, setting)) = name.split_once('.')
        && crate::config::PER_LANGUAGE.contains(&setting)
        && (renderer.site().languages.iter())
            .any(|(_, language)| language.tag.as_deref() == Some(tag))
    {
        let described = wording::described_setting(setting)?;
        return Some(Said {
            head: wording::hover_for_a_language(name),
            body: vec![
                wording::for_a_language(setting, tag),
                described.text.clone(),
            ],
            more: Some(wording::FOR_ONE_LANGUAGE.to_owned()),
            to: None,
        });
    }
    let top = name.split('.').next().unwrap_or(name);
    let own = &renderer.settings().own;
    if !own.iter().any(|value| value.name == top) {
        return None;
    }
    Some(Said {
        head: wording::hover_own_value(name),
        body: vec![wording::shown_as(&format!("site.{name}"))],
        more: Some(wording::OWN_VALUE.to_owned()),
        to: None,
    })
}

/// What is said of `name` as a field of a page's front matter.
fn field(site: &Known, name: &str) -> Option<Said> {
    if let Some(word) = PageWord::named(name) {
        let described = wording::described_field(word.name())?;
        return Some(Said::of(wording::hover_field(name), described));
    }
    if let Some(field) = name.strip_prefix("pages.") {
        return match site.holder {
            Holder::FolderPage { top: false } => handed(site, field, true),
            _ => None,
        };
    }
    let top = name.split('.').next().unwrap_or(name);
    if is_statacs(top) {
        return None;
    }
    Some(own_field(
        site,
        name,
        wording::hover_own_field(name),
        OwnField::Alone,
    ))
}

/// What is said of a line handing `field` down: in a folder's page, or
/// in the site's settings.
fn handed(site: &Known, field: &str, in_folder: bool) -> Option<Said> {
    let head = wording::hover_handed(&format!("pages.{field}"));
    let hands = wording::hands_down(field, in_folder);
    if let Some(word) = PageWord::named(field) {
        let described = wording::described_field(word.name())?;
        return word.handed_down().then(|| Said {
            head,
            body: vec![hands, described.text.clone()],
            more: Some(wording::HANDING_DOWN.to_owned()),
            to: None,
        });
    }
    let top = field.split('.').next().unwrap_or(field);
    if is_statacs(top) {
        return None;
    }
    Some(own_field(site, field, head, OwnField::Handed(hands)))
}

/// How a field of the author's own is written.
enum OwnField {
    Alone,
    /// In a line handing it down, which does what is said.
    Handed(String),
}

/// What is said of `name`, a field of the author's own, headed `head`.
fn own_field(site: &Known, name: &str, head: String, written: OwnField) -> Said {
    let mut body = Vec::new();
    let more = match written {
        OwnField::Alone => wording::OWN_FIELD,
        OwnField::Handed(hands) => {
            body.push(hands);
            wording::HANDING_DOWN
        }
    };
    body.push(wording::shown_as(&format!("page.{name}")));
    let mut to = None;
    if let Some(renderer) = site.renderer
        && let Some(declared) = renderer.settings().declarations.declared(name)
    {
        let kind = renderer.sources().spelling(declared.place);
        body.push(wording::declared_as(kind.trim_matches(['"', '\''])));
        to = Some(Goes::At(declared.place));
    }
    Said {
        head,
        body,
        more: Some(more.to_owned()),
        to,
    }
}

/// What is said of `text`, a value of the front matter word `name`: a
/// tag, a series or a template the site has.
fn value(site: &Known, name: &str, text: &str) -> Option<Said> {
    let renderer = site.renderer?;
    match PageWord::named(name)? {
        PageWord::Tags => list(site, renderer, text, tag, wording::hover_tag),
        PageWord::Series => list(site, renderer, text, series, wording::hover_series),
        PageWord::Layout => {
            let (_, path) = (renderer.layouts().into_iter()).find(|(layout, _)| layout == text)?;
            Some(Said {
                head: wording::hover_template(text),
                body: vec![wording::its_file(&path)],
                more: None,
                to: Some(Goes::File(path)),
            })
        }
        _ => None,
    }
}

/// What is said of the tag or series `name`, as `which` picks the lists
/// of its kind: how many pages carry it, and where its page is.
fn list(
    site: &Known,
    renderer: &Renderer,
    name: &str,
    which: fn(&ListKey) -> Option<&String>,
    head: fn(&str, usize) -> String,
) -> Option<Said> {
    let held = renderer.site();
    // A spelling that differs in case is the same tag.
    let (_, list) = (held.lists.iter()).find(|(_, list)| {
        which(&list.key).is_some_and(|named| named.to_lowercase() == name.to_lowercase())
            && site
                .language
                .is_none_or(|language| language == list.language)
    })?;
    let pages = list.members.len() + list.undated.len();
    let base = renderer.settings().base();
    let page = (list.page).map(|page| held.pages.get(page).address.as_str());
    // Only a page of the author's own has a file to go to.
    let file = (list.page)
        .filter(|page| held.pages.get(*page).source.is_some())
        .map(|page| Goes::File(renderer.page_path(page)));
    Some(Said {
        head: head(name, pages),
        body: (page.into_iter())
            .map(|address| wording::list_page_at(&format!("{base}{address}")))
            .collect(),
        more: None,
        to: file,
    })
}

fn tag(key: &ListKey) -> Option<&String> {
    match key {
        ListKey::Tag(name) => Some(name),
        _ => None,
    }
}

fn series(key: &ListKey) -> Option<&String> {
    match key {
        ListKey::Series(name) => Some(name),
        _ => None,
    }
}

/// What is said of `name` in `quiet:`, where it names a kind of problem:
/// the opening of its explanation.
fn kind_of_problem(name: &str) -> Option<Said> {
    let code = Code::from_name(name)?;
    let explanation = code.explanation().replace("\r\n", "\n");
    let opening = (explanation.split("\n\n").nth(1))
        .unwrap_or_default()
        .split('\n')
        .collect::<Vec<_>>()
        .join(" ");
    Some(Said {
        head: wording::hover_kind(name, code.severity()),
        body: vec![opening],
        more: Some(wording::explanation_link(name)),
        to: None,
    })
}
