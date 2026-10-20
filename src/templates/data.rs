//! `site.data`: the files under `data/` as a template sees them. A file
//! is read the first time a template reaches it, recorded as read by every
//! output that reaches it, and a lookup that finds nothing names the file
//! or folder it was asked of.

use std::collections::HashSet;
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

    /// Every file in the folder, however deep.
    pub fn files(&self) -> Vec<DataId> {
        let mut files = Vec::new();
        files_in(self.folder(), &mut files);
        files
    }

    /// The files `name` in the folder is: itself, or every file in it.
    pub fn files_of(&self, name: &str) -> Vec<DataId> {
        let mut files = Vec::new();
        match self.folder().entries.get(name) {
            Some(Entry::File(id)) => files.push(*id),
            Some(Entry::Folder(inner)) => files_in(inner, &mut files),
            None => {}
        }
        files
    }
}

fn files_in(folder: &Folder, files: &mut Vec<DataId>) {
    for entry in folder.entries.values() {
        match entry {
            Entry::File(id) => files.push(*id),
            Entry::Folder(inner) => files_in(inner, files),
        }
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
    frame::reach(id);
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
        Yaml::Sequence(items) => {
            let items: Vec<Value> = items
                .iter()
                .map(|item| value(item, file, origins))
                .collect();
            let holds = Holds::any(items.iter());
            Value::from_object(SeqValue { items, holds })
        }
        Yaml::Mapping(entries) => {
            let entries: Vec<(String, Value)> = entries
                .iter()
                .map(|entry| {
                    (
                        crate::content::words::key_name(&entry.key),
                        value(&entry.value, file, origins),
                    )
                })
                .collect();
            let holds = Holds::any(entries.iter().map(|(_, value)| value));
            Value::from_object(MapValue {
                entries,
                file,
                holds,
            })
        }
    }
}

/// What a data file's list or map holds, however deep, that a guard
/// refuses in places: an empty value, or bytes that aren't text. A data
/// file holds no missing value, so a guard need search one only where this
/// says. It counts the value of every key, even one a lookup never finds
/// because a key before it has the same name.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Holds {
    pub empty: bool,
    pub bytes: bool,
}

impl Holds {
    fn any<'a>(values: impl Iterator<Item = &'a Value>) -> Holds {
        let mut all = Holds::default();
        for value in values {
            let inner = holds(value).unwrap_or(Holds {
                empty: value.is_none(),
                bytes: value
                    .downcast_object_ref::<super::values::BytesValue>()
                    .is_some(),
            });
            all.empty |= inner.empty;
            all.bytes |= inner.bytes;
        }
        all
    }
}

/// What `value` holds, where it is a data file's list or map.
pub(super) fn holds(value: &Value) -> Option<Holds> {
    if let Some(map) = value.downcast_object_ref::<MapValue>() {
        return Some(map.holds);
    }
    value.downcast_object_ref::<SeqValue>().map(|seq| seq.holds)
}

/// Each name in a data file's map, in order, with the value a lookup of it
/// finds, each name once. Keys that aren't text are all named `''`, and a
/// lookup finds the first.
pub(super) fn entries(value: &Value) -> Option<impl Iterator<Item = (&str, &Value)>> {
    let map = value.downcast_object_ref::<MapValue>()?;
    let mut named = HashSet::new();
    Some(
        map.entries
            .iter()
            .filter(move |(name, _)| named.insert(name.as_str()))
            .map(|(name, value)| (name.as_str(), value)),
    )
}

/// A list in a data file.
#[derive(Debug)]
struct SeqValue {
    items: Vec<Value>,
    holds: Holds,
}

impl Object for SeqValue {
    fn repr(self: &Arc<Self>) -> ObjectRepr {
        ObjectRepr::Seq
    }

    fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
        self.items.get(key.as_usize()?).cloned()
    }

    fn enumerate(self: &Arc<Self>) -> Enumerator {
        Enumerator::Seq(self.items.len())
    }
}

/// `name: value` lines in a data file, in the order they are written.
#[derive(Debug)]
struct MapValue {
    entries: Vec<(String, Value)>,
    file: DataId,
    holds: Holds,
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
