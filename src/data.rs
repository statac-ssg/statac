//! The site's data files: YAML and JSON under `data/`, each reaching
//! templates as `site.data` and its name, with folders nesting as names do:
//! `data/people/staff.yaml` is `site.data.people.staff`.
//!
//! Each is read by its own format's reader into one tree, so every value
//! keeps the place it was written and means what that reader makes of it.
//! A data file is read exactly as it stands: YAML's merge key, `<<`, is
//! refused.

use std::collections::BTreeMap;
use std::path::Path;

use crate::ids::{Arena, DataId};
use crate::problem::{Kind, Problem};
use crate::source::{SourceId, Sources};
use crate::yaml::{Node, Value as Yaml};

/// The folder data files are read from.
pub(crate) const FOLDER: &str = "data";

/// One data file read.
pub struct DataFile {
    /// Its path in the site, such as `data/people.yaml`.
    pub path: String,
    pub source: SourceId,
    /// None where it couldn't be read, which a problem has said.
    pub node: Option<Node>,
}

/// A folder under `data/`, or `data/` itself.
#[derive(Default)]
pub struct Folder {
    /// Its path in the site, ending with `/`: `data/people/`.
    pub path: String,
    pub entries: BTreeMap<String, Entry>,
}

pub enum Entry {
    File(DataId),
    Folder(Folder),
}

/// Everything under `data/`.
#[derive(Default)]
pub struct Data {
    pub files: Arena<DataId, DataFile>,
    pub top: Folder,
}

/// What a name under `data/` leads to.
pub enum Reached<'a> {
    Folder(&'a Folder),
    /// A value in a file, the whole file where the names stop at it.
    Value {
        file: DataId,
        node: &'a Node,
    },
    /// A file that couldn't be read.
    Unread(DataId),
}

impl Data {
    pub fn file(&self, id: DataId) -> &DataFile {
        self.files.get(id)
    }

    /// What `names` lead to from the top of `data/`: folders, then a file,
    /// then keys inside it.
    pub fn reach(&self, names: &[&str]) -> Option<Reached<'_>> {
        let mut folder = &self.top;
        for (at, name) in names.iter().enumerate() {
            match folder.entries.get(*name)? {
                Entry::Folder(inner) => folder = inner,
                Entry::File(id) => {
                    let Some(mut node) = self.file(*id).node.as_ref() else {
                        return Some(Reached::Unread(*id));
                    };
                    for key in &names[at + 1..] {
                        node = node.get(key)?;
                    }
                    return Some(Reached::Value { file: *id, node });
                }
            }
        }
        Some(Reached::Folder(folder))
    }

    /// Every name a template or `each:` can reach from the top, as
    /// `a.b.c`, for offering the nearest to one that reaches nothing.
    pub fn names(&self) -> Vec<String> {
        fn walk(data: &Data, prefix: &str, folder: &Folder, names: &mut Vec<String>) {
            for (name, entry) in &folder.entries {
                let path = match prefix.is_empty() {
                    true => name.clone(),
                    false => format!("{prefix}.{name}"),
                };
                names.push(path.clone());
                match entry {
                    Entry::Folder(inner) => walk(data, &path, inner, names),
                    Entry::File(id) => {
                        if let Some(Yaml::Mapping(entries)) =
                            data.file(*id).node.as_ref().map(|n| &n.value)
                        {
                            for entry in entries {
                                if let Yaml::Scalar(key) = &entry.key.value {
                                    names.push(format!("{path}.{key}"));
                                }
                            }
                        }
                    }
                }
            }
        }
        let mut names = Vec::new();
        walk(self, "", &self.top, &mut names);
        names
    }
}

/// Reads every data file of the site at `root`.
pub(crate) fn read(root: &Path, sources: &mut Sources) -> (Data, Vec<Problem>) {
    let mut problems = Vec::new();
    let mut data = Data {
        files: Arena::default(),
        top: Folder {
            path: format!("{FOLDER}/"),
            entries: BTreeMap::new(),
        },
    };
    let real = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let found = crate::assets::walk(root, &real, FOLDER, false, &mut problems);
    let mut told = Vec::new();
    for (rel, at) in found {
        let shown = format!("{FOLDER}/{rel}");
        let (folders, file) = match rel.rsplit_once('/') {
            Some((folders, file)) => (folders.split('/').collect(), file),
            None => (Vec::new(), rel.as_str()),
        };
        let Some((stem, extension)) = file.rsplit_once('.').filter(|(stem, _)| !stem.is_empty())
        else {
            problems.push(Problem::nowhere(Kind::NotData { path: shown }));
            continue;
        };
        if !matches!(
            extension.to_ascii_lowercase().as_str(),
            "yaml" | "yml" | "json"
        ) {
            problems.push(Problem::nowhere(Kind::NotData { path: shown }));
            continue;
        }
        let text = match sources.unsaved().read(&real, &at) {
            Ok(text) => text,
            Err(error) => {
                problems.push(Problem::nowhere(Kind::CantRead {
                    path: shown.into(),
                    reason: crate::problem::Reason::of(&error),
                }));
                continue;
            }
        };
        let source = sources.add(&shown, text);
        let read = match extension.eq_ignore_ascii_case("json") {
            true => crate::yaml::read_json(sources, source),
            false => crate::yaml::read(sources, source),
        };
        let node = match read {
            Ok(node) => {
                merge_keys(&node, &mut problems);
                Some(node)
            }
            Err(problem) => {
                problems.push(problem);
                None
            }
        };
        let id = data.files.push(DataFile {
            path: shown.clone(),
            source,
            node,
        });
        if let Err(problem) = place(&mut data, &folders, stem, id, &shown, &mut told) {
            problems.push(problem);
        }
    }
    (data, problems)
}

/// Files a data file's name in its folder, or says what already has it.
fn place(
    data: &mut Data,
    folders: &[&str],
    stem: &str,
    id: DataId,
    shown: &str,
    told: &mut Vec<String>,
) -> Result<(), Problem> {
    let mut folder = &mut data.top;
    let mut names = vec!["site.data".to_owned()];
    for name in folders {
        names.push((*name).to_owned());
        let path = format!("{}{name}/", folder.path);
        let entry = folder.entries.entry((*name).to_owned()).or_insert_with(|| {
            Entry::Folder(Folder {
                path: path.clone(),
                entries: BTreeMap::new(),
            })
        });
        match entry {
            Entry::Folder(inner) => folder = inner,
            Entry::File(first) => {
                // A folder is met once for every file inside it.
                if told.contains(&path) {
                    return Ok(());
                }
                told.push(path.clone());
                let first = data.files.get(*first).path.clone();
                return Err(named_twice(first, path, &names));
            }
        }
    }
    names.push(stem.to_owned());
    match folder.entries.get(stem) {
        None => {
            folder.entries.insert(stem.to_owned(), Entry::File(id));
            Ok(())
        }
        Some(Entry::File(first)) => {
            let first = data.files.get(*first).path.clone();
            Err(named_twice(first, shown.to_owned(), &names))
        }
        Some(Entry::Folder(inner)) => {
            Err(named_twice(inner.path.clone(), shown.to_owned(), &names))
        }
    }
}

fn named_twice(first: String, second: String, names: &[String]) -> Problem {
    Problem::nowhere(Kind::DataNamedTwice {
        first,
        second,
        name: names.join("."),
    })
}

/// Refuses every plain `<<` key: YAML 1.1's merge key, which other readers
/// take to copy fields in from elsewhere.
fn merge_keys(node: &Node, problems: &mut Vec<Problem>) {
    match &node.value {
        Yaml::Mapping(entries) => {
            for entry in entries {
                if entry.key.style == crate::origin::Style::Plain
                    && entry.key.tag.is_none()
                    && matches!(&entry.key.value, Yaml::Scalar(key) if key == "<<")
                {
                    problems.push(Problem::at(Kind::MergeKey, entry.key.place));
                }
                merge_keys(&entry.value, problems);
            }
        }
        Yaml::Sequence(items) => {
            for item in items {
                merge_keys(item, problems);
            }
        }
        Yaml::Scalar(_) => {}
    }
}

#[cfg(test)]
mod tests;
