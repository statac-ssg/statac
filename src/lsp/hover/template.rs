//! Hover in a template or a snippet: what a name or field it reads holds
//! where it stands, the snippet a call names, and the page or template a
//! string it hands `url(...)` or `extends` names.

use std::ops::Range;

use super::writing::{Reading, snippet_said};
use super::{Goes, Said};
use crate::lsp::complete::template::{Pointed, pointed};
use crate::templates::Renderer;
use crate::wording;

/// What is said of the character at `offset` in `text`, the text of the
/// site's template or snippet at `rel`, and the range it is said of.
pub(super) fn said(
    renderer: &Renderer,
    rel: &str,
    text: &str,
    offset: usize,
) -> Option<(Range<usize>, Said)> {
    match pointed(text, offset, rel, renderer)? {
        Pointed::Value {
            range,
            written,
            holds,
        } => {
            let said = Said {
                head: wording::hover_holds(&written, holds)?,
                body: Vec::new(),
                more: None,
                to: None,
            };
            Some((range, said))
        }
        Pointed::Snippet { range, name } => {
            let snippets = renderer.snippets();
            let (snippet, path) = snippets.iter().find(|(snippet, _)| snippet.name == name)?;
            Some((range, snippet_said(snippet, path, false)))
        }
        Pointed::Address { range, address } => {
            let base = renderer.settings().base();
            let reading = Reading {
                renderer,
                page: None,
                folder: "",
                base: &base,
            };
            let said = reading.address(&format!("{base}{address}"), &address)?;
            Some((range, said))
        }
        Pointed::Extended { range, name } => {
            let (_, path) = (renderer.layouts().into_iter()).find(|(layout, _)| *layout == name)?;
            let said = Said {
                head: wording::hover_template(&name),
                body: vec![wording::its_file(&path)],
                more: None,
                to: Some(Goes::File(path)),
            };
            Some((range, said))
        }
    }
}
