//! What a template may read where an editor's cursor stands, while what
//! is there is still being typed. The template is read with [`TYPED`]
//! written at the cursor and checked as every template is, so what is
//! offered is what the check would take.

use super::check::{self, Answer, Checked, Rec, Ty};
use super::files::Role;
use super::{Renderer, guard, own, shows};
use crate::problem::Holds;
use crate::source::Sources;

pub(crate) use check::TYPED;

/// What a template may read where [`TYPED`] is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Readable {
    /// It stands as a name: every name readable there.
    Names(Vec<Name>),
    /// It stands as a field: the fields of the value before its dot.
    Fields(Vec<Name>),
}

/// A name or field, with what it holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Name {
    pub name: String,
    pub holds: Holds,
}

impl Renderer {
    /// What may be read where [`TYPED`] is written in `text`, the text of
    /// the site's template or snippet at `path`; none where the site holds
    /// no such file, the text can't be read, or [`TYPED`] isn't written
    /// where a name or a field is read.
    pub(crate) fn readable(&self, path: &str, text: &str) -> Option<Readable> {
        let shared = &self.shared;
        let templates = &shared.templates;
        let file = (templates.files.iter())
            .find(|file| shared.sources.path(file.source).to_string_lossy() == path)?;
        if file.role == Role::CardText {
            return None;
        }
        let mut schema = super::schema(&shared.site, &shared.settings);
        schema.snippets.clone_from(&templates.snippets);
        schema.templates = (templates.files.iter())
            .filter(|file| file.role == Role::Template)
            .map(|file| file.name.clone())
            .collect();
        schema.feeds = crate::feeds::templates(&shared.site, &shared.settings, &|name| {
            schema.templates.contains(name)
        });
        let mut sources = Sources::default();
        let source = sources.add(path, text.to_owned());
        let checked = Checked {
            name: &file.name,
            source,
            role: &file.role,
        };
        let taken = match &file.role {
            Role::Snippet(name) => templates.taken.get(name).map(Vec::as_slice),
            _ => None,
        };
        match check::asked(&schema, &checked, &sources, taken)? {
            Answer::Names(names) => Some(Readable::Names(
                (names.into_iter())
                    .map(|(name, ty)| Name {
                        holds: shows::holds(&ty, None),
                        name,
                    })
                    .collect(),
            )),
            Answer::Fields { ty, root, path } => {
                let fields = match (&ty, &root) {
                    (Ty::Record(rec), _) => (schema.record(*rec).iter())
                        .map(|(name, field)| Name {
                            name: name.clone(),
                            holds: shows::holds(&field.ty, None),
                        })
                        .collect(),
                    (Ty::Any, Ty::Record(Rec::Page)) => self.nested(&path),
                    (Ty::Any, Ty::Record(Rec::Site))
                        if path.first().is_some_and(|name| name == "data") =>
                    {
                        self.data(&path[1..])
                    }
                    _ => return None,
                };
                Some(Readable::Fields(fields))
            }
        }
    }

    /// The fields the site declares nested in a page's field, as `author`
    /// for `book.author`, `path` naming the fields leading to them.
    fn nested(&self, path: &[String]) -> Vec<Name> {
        let declarations = &self.shared.settings.declarations;
        let prefix: String = path.iter().map(|name| format!("{name}.")).collect();
        let mut fields: Vec<Name> = Vec::new();
        for declared in declarations.names() {
            let Some(rest) = declared.strip_prefix(prefix.as_str()) else {
                continue;
            };
            let (name, holds) = match rest.split_once('.') {
                Some((name, _)) => (name, Holds::Fields),
                None => (
                    rest,
                    shows::holds(&check::declared_ty(declarations.kind(declared)), None),
                ),
            };
            if !fields.iter().any(|field| field.name == name) {
                fields.push(Name {
                    name: name.to_owned(),
                    holds,
                });
            }
        }
        fields
    }

    /// What the site keeps under `data/` at `path`, as a template reaches
    /// it through `site.data`.
    fn data(&self, path: &[String]) -> Vec<Name> {
        use crate::data::{Entry, Reached};
        use crate::yaml::{Node, Value};
        let data = &self.shared.site.data;
        let holds = |node: Option<&Node>| match node.map(|node| &node.value) {
            Some(Value::Mapping(_)) => Holds::Fields,
            Some(Value::Sequence(_)) => Holds::Items,
            _ => Holds::Unknown,
        };
        let names: Vec<&str> = path.iter().map(String::as_str).collect();
        match data.reach(&names) {
            Some(Reached::Folder(folder)) => (folder.entries.iter())
                .map(|(name, entry)| Name {
                    name: name.clone(),
                    holds: match entry {
                        Entry::Folder(_) => Holds::Fields,
                        Entry::File(id) => holds(data.file(*id).node.as_ref()),
                    },
                })
                .collect(),
            Some(Reached::Value { node, .. }) => match &node.value {
                Value::Mapping(entries) => (entries.iter())
                    .filter_map(|entry| match &entry.key.value {
                        Value::Scalar(name) => Some(Name {
                            name: name.clone(),
                            holds: holds(Some(&entry.value)),
                        }),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            },
            _ => Vec::new(),
        }
    }

    /// The blocks of the template named `name` and of each it extends,
    /// which a template extending it may fill.
    pub(crate) fn blocks(&self, name: &str) -> Vec<String> {
        let templates = &self.shared.templates;
        let mut names: Vec<String> = Vec::new();
        for at in templates.chain(super::files::named(name)) {
            for (block, ..) in templates.blocks[at].iter().flatten() {
                if !names.contains(block) {
                    names.push(block.clone());
                }
            }
        }
        names.sort();
        names
    }

    /// The names of the site's stylesheets, scripts and other files a
    /// template may ask for by name.
    pub(crate) fn asset_names(&self) -> Vec<String> {
        self.shared.assets.names().map(str::to_owned).collect()
    }
}

/// The filters a template may use, Statac's own and the engine's.
pub(crate) fn filter_names() -> impl Iterator<Item = &'static str> {
    own::FILTERS.iter().chain(guard::FILTERS).copied()
}

/// The tests a template may use after `is`, by name.
pub(crate) fn test_names() -> impl Iterator<Item = &'static str> {
    (guard::TESTS.iter().copied()).filter(|test| test.starts_with(|c: char| c.is_alphabetic()))
}
