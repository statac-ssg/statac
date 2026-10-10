//! The file describing a picture, beside it under `content/` and named
//! after it: `photo-1.jpg.yaml` for `photo-1.jpg`.

use crate::content::words::{Field, key_name};
use crate::origin::Sourced;
use crate::problem::{Expected, Problem};
use crate::source::{SourceId, Sources};
use crate::value::Reader;
use crate::yaml::{self, Resolved, Value as Yaml};

/// What a picture's own file says of it.
#[derive(Clone, PartialEq, Debug)]
pub struct Sidecar {
    pub source: SourceId,
    /// The picture's description, for readers who can't see it.
    pub alt: Option<Sourced<String>>,
    pub caption: Option<Sourced<String>>,
    /// False leaves a photograph as it is, with no copies made.
    pub optimise: Option<Sourced<bool>>,
    /// Whether the picture is decoration, wanting no description.
    pub decorative: Option<Sourced<bool>>,
    /// Every other key, the author's own.
    pub own: Vec<Field>,
}

/// Reads the file at `source`. None where it can't be read as fields,
/// which is told.
pub(super) fn read(sources: &Sources, source: SourceId) -> (Option<Sidecar>, Vec<Problem>) {
    let mut problems = Vec::new();
    let node = match yaml::read(sources, source) {
        Ok(node) => node,
        Err(problem) => return (None, vec![problem]),
    };
    let mut sidecar = Sidecar {
        source,
        alt: None,
        caption: None,
        optimise: None,
        decorative: None,
        own: Vec::new(),
    };
    let entries = match &node.value {
        Yaml::Mapping(entries) => entries,
        Yaml::Scalar(_) if node.resolved() == Some(Resolved::Null) => {
            return (Some(sidecar), problems);
        }
        _ => {
            let path = sources.path(source).to_string_lossy();
            let name = path.rsplit('/').next().unwrap_or(&path);
            problems.push(Reader::new(sources, name).wrong(&node, Expected::Fields, None));
            return (None, problems);
        }
    };
    for entry in entries {
        let name = key_name(&entry.key);
        let reader = Reader::new(sources, &name);
        let read = match name.as_str() {
            "alt" => reader.text(&entry.value).map(|text| sidecar.alt = text),
            "caption" => reader.text(&entry.value).map(|text| sidecar.caption = text),
            "optimise" => reader
                .truth(&entry.value)
                .map(|truth| sidecar.optimise = truth),
            "decorative" => reader
                .truth(&entry.value)
                .map(|truth| sidecar.decorative = truth),
            _ => {
                sidecar.own.push(Field {
                    name,
                    key: entry.key.place,
                    value: entry.value.clone(),
                    origin: entry.value.origin(),
                });
                Ok(())
            }
        };
        problems.extend(read.err());
    }
    (Some(sidecar), problems)
}
