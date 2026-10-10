//! `site.data`: the files under `data/` as a template sees them. A file
//! is read the first time a template reaches it, recorded as read by every
//! output that reaches it, and a lookup that finds nothing names the file
//! or folder it was asked of.

use std::sync::Arc;

use minijinja::value::{Enumerator, Object, ObjectRepr, Value};

use super::frame::{self, InData};
use super::render::Shared;
use crate::data::{Entry, Folder};
use crate::reads::Key;
use crate::site::{DataId, Id};
use crate::yaml::{Node, Resolved, Tag, Value as Yaml};

/// `data/`, or a folder inside it.
#[derive(Debug)]
pub(super) struct FolderValue {
    pub shared: Arc<Shared>,
    /// The folder's names from the top of `data/`.
    pub names: Vec<String>,
}

impl FolderValue {
    pub fn top(shared: &Arc<Shared>) -> Value {
        Value::from_object(FolderValue {
            shared: Arc::clone(shared),
            names: Vec::new(),
        })
    }

    fn folder(&self) -> &Folder {
        let mut folder = &self.shared.site.data.top;
        for name in &self.names {
            match folder.entries.get(name) {
                Some(Entry::Folder(inner)) => folder = inner,
                _ => unreachable!("a folder's value is made only for a folder"),
            }
        }
        folder
    }
}

impl Object for FolderValue {
    fn repr(self: &Arc<Self>) -> ObjectRepr {
        ObjectRepr::Map
    }

    fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
        let name = key.as_str()?;
        let folder = self.folder();
        match folder.entries.get(name) {
            Some(Entry::File(id)) => Some(file_value(&self.shared, *id)),
            Some(Entry::Folder(_)) => {
                let mut names = self.names.clone();
                names.push(name.to_owned());
                Some(Value::from_object(FolderValue {
                    shared: Arc::clone(&self.shared),
                    names,
                }))
            }
            None => {
                frame::missed_in_data(InData::Folder(folder.path.clone()), name);
                None
            }
        }
    }

    fn enumerate(self: &Arc<Self>) -> Enumerator {
        Enumerator::Values(
            self.folder()
                .entries
                .keys()
                .map(|name| Value::from(name.as_str()))
                .collect(),
        )
    }
}

/// A data file's value, made once and kept.
fn file_value(shared: &Arc<Shared>, id: DataId) -> Value {
    frame::read(Key::Data(id));
    let Some(node) = &shared.site.data.file(id).node else {
        // The file couldn't be read, which is told already.
        return Value::from(frame::told());
    };
    shared.data_values[id.index()]
        .get_or_init(|| {
            let mut origins = frame::Origins::new();
            (value(node, id, &mut origins), origins)
        })
        .0
        .clone()
}

/// A value as YAML's standard reading makes it, each text in it noted in
/// `origins` with where it is written.
fn value(node: &Node, file: DataId, origins: &mut frame::Origins) -> Value {
    let text_value = |text: &str, origins: &mut frame::Origins| {
        if text.is_empty() {
            return Value::from("");
        }
        let made = Value::from(Arc::<str>::from(text));
        origins.insert(frame::address(&made), node.place);
        made
    };
    match &node.value {
        Yaml::Scalar(_) if node.tag == Some(Tag::Bytes) => {
            Value::from_object(super::values::BytesValue)
        }
        Yaml::Scalar(text) => match node.resolved() {
            Some(Resolved::Null) => Value::from(()),
            Some(Resolved::Bool(b)) => Value::from(b),
            Some(Resolved::Int(Some(n))) => match i64::try_from(n) {
                Ok(n) => Value::from(n),
                Err(_) => text_value(text, origins),
            },
            Some(Resolved::Float(f)) => Value::from(f),
            _ => text_value(text, origins),
        },
        Yaml::Sequence(items) => Value::from(
            items
                .iter()
                .map(|item| value(item, file, origins))
                .collect::<Vec<_>>(),
        ),
        Yaml::Mapping(entries) => Value::from_object(MapValue {
            entries: entries
                .iter()
                .map(|entry| {
                    (
                        crate::content::words::key_name(&entry.key),
                        value(&entry.value, file, origins),
                    )
                })
                .collect(),
            file,
        }),
    }
}

/// `name: value` lines in a data file, in the order they are written.
#[derive(Debug)]
struct MapValue {
    entries: Vec<(String, Value)>,
    file: DataId,
}

impl Object for MapValue {
    fn repr(self: &Arc<Self>) -> ObjectRepr {
        ObjectRepr::Map
    }

    /// A key written as a number, such as `42:`, is found by the number too.
    fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
        let key = match key.as_str() {
            Some(key) => key.to_owned(),
            None if key.is_number() => key.to_string(),
            None => return None,
        };
        let key = key.as_str();
        let found = self.entries.iter().find(|(name, _)| name == key);
        if found.is_none() {
            frame::missed_in_data(InData::File(self.file), key);
        }
        found.map(|(_, value)| value.clone())
    }

    fn enumerate(self: &Arc<Self>) -> Enumerator {
        Enumerator::Values(
            self.entries
                .iter()
                .map(|(name, _)| Value::from(name.as_str()))
                .collect(),
        )
    }
}
