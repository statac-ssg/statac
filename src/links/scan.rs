//! The addresses a built page links to and the ids on it, and the
//! addresses the writing links to, each where it is written.

use crate::html::{Tag, addresses, decoded, start_tags};
use crate::markdown::{self, Document, InSite, Kind, Node};
use crate::source::{Place, SourceId, Sources, Span};

/// The attribute a link is written in.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum Attribute {
    Href,
    Src,
    /// One of the addresses a `srcset` lists.
    Srcset,
    Poster,
}

impl Attribute {
    /// Every attribute read as addresses, by its name.
    pub(crate) const NAMED: [(&'static str, Attribute); 4] = [
        ("href", Attribute::Href),
        ("src", Attribute::Src),
        ("srcset", Attribute::Srcset),
        ("poster", Attribute::Poster),
    ];

    pub(super) fn of(name: &str) -> Option<Attribute> {
        let (_, attribute) = Attribute::NAMED.iter().find(|(named, _)| *named == name)?;
        Some(*attribute)
    }
}

/// An address a built page links to.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) struct Found {
    pub attribute: Attribute,
    /// As written in the HTML, its character references undecoded.
    pub raw: String,
    /// Where it starts on the page.
    pub at: usize,
    /// Where the attribute's value starts on the page.
    pub value: usize,
}

impl Found {
    /// The address a browser follows: its character references decoded.
    pub(super) fn address(&self) -> String {
        decoded(&self.raw, false)
    }
}

/// What a built page links to, and the ids a link can land on in it.
#[derive(Default, Debug)]
pub(super) struct Scanned {
    pub links: Vec<Found>,
    /// Sorted, each once.
    pub ids: Vec<String>,
    /// The page's `<base href>`, which relative links are read from.
    pub base: Option<String>,
}

/// Every link on a built page and every id on it.
pub(super) fn scan(html: &str) -> Scanned {
    let mut scanned = Scanned::default();
    start_tags(html, |tag| {
        for attribute in &tag.attributes {
            match attribute.name.as_str() {
                "id" => scanned.ids.push(decoded(&attribute.value, false)),
                "name" if tag.name == "a" => scanned.ids.push(decoded(&attribute.value, false)),
                "href" if tag.name == "base" => {
                    scanned.base = Some(decoded(&attribute.value, false));
                }
                name => {
                    let Some(kind) = Attribute::of(name) else {
                        continue;
                    };
                    let value = attribute.at.start;
                    for (raw, at) in addresses(&attribute.value, kind == Attribute::Srcset, value) {
                        scanned.links.push(Found {
                            attribute: kind,
                            raw: raw.to_owned(),
                            at,
                            value,
                        });
                    }
                }
            }
        }
    });
    scanned.ids.sort_unstable();
    scanned.ids.dedup();
    scanned
}

/// An address the writing links to, and where it is written.
#[derive(Clone, PartialEq, Debug)]
pub struct Linked {
    /// As a browser follows it: a link's address as Markdown reads it, or
    /// an attribute's with its character references decoded.
    pub address: String,
    /// The address itself where it can be found, else the whole link.
    pub place: Place,
    /// Whether it is a value handed to a snippet call, where an
    /// `@url(...)` is not read.
    pub handed: bool,
}

/// Every address the writing links to: each link's and picture's, placed
/// at its address, or at the line defining its label for a link written
/// by label; and each in the writing's own HTML, at the attribute's value.
/// One written as `@url(...)` is the address it stands for, placed at the
/// call.
pub(crate) fn written(document: &Document, sources: &Sources) -> Vec<Linked> {
    let mut found = Vec::new();
    for top in std::iter::once(&document.root).chain(&document.notes) {
        written_in_node(top, document, sources, &mut found);
    }
    found
}

fn written_in_node(node: &Node, document: &Document, sources: &Sources, out: &mut Vec<Linked>) {
    match &node.kind {
        Kind::Link { destination, .. } | Kind::Image { destination, .. } => {
            if destination.is_empty() {
                return;
            }
            let within = |place: Place| {
                let at = sources.spelling(place).rfind(destination.as_str())?;
                let start = place.span.start as usize + at;
                Some(Place {
                    source: place.source,
                    span: Span::new(start, start + destination.len()),
                })
            };
            let stands = |place: Place| document.in_site_at(destination, place);
            let place = stands(node.place)
                .or_else(|| within(node.place))
                .or_else(|| {
                    document
                        .definitions
                        .iter()
                        .filter(|definition| definition.destination == *destination)
                        .find_map(|definition| {
                            stands(definition.place).or_else(|| within(definition.place))
                        })
                })
                .unwrap_or(node.place);
            out.push(Linked {
                address: destination.clone(),
                place,
                handed: false,
            });
        }
        // A block of HTML is read whole, as a tag may run over its lines.
        Kind::HtmlBlock | Kind::Html(_) => {
            let place = node.place;
            out.extend(in_html(
                sources,
                place.source,
                place.span,
                &document.in_site,
            ));
            return;
        }
        _ => {}
    }
    for child in &node.children {
        written_in_node(child, document, sources, out);
    }
}

/// Every address the HTML at `span` in `source` links to, placed at the
/// address in its attribute.
pub(crate) fn written_in_html(sources: &Sources, source: SourceId, span: Span) -> Vec<Linked> {
    in_html(sources, source, span, &[])
}

/// [`written_in_html`], for HTML among writing whose `@url(...)` calls
/// are `in_site`: each in an address is the address it stands for.
fn in_html(sources: &Sources, source: SourceId, span: Span, in_site: &[InSite]) -> Vec<Linked> {
    let whole = sources.text(source);
    let text = &whole[span.range()];
    let mut found = Vec::new();
    start_tags(text, |tag: &Tag| {
        for attribute in &tag.attributes {
            let Some(kind) = Attribute::of(&attribute.name) else {
                continue;
            };
            if tag.name == "base" {
                continue;
            }
            for (raw, at) in addresses(
                &attribute.value,
                kind == Attribute::Srcset,
                attribute.at.start,
            ) {
                let start = span.start as usize + at;
                let end = start + raw.len();
                let mut address = String::new();
                let mut from = start;
                for stands in in_site {
                    let at = stands.place.span.range();
                    if stands.place.source == source && from <= at.start && at.end <= end {
                        address.push_str(&decoded(&whole[from..at.start], false));
                        address.push_str(&stands.address);
                        from = at.end;
                    }
                }
                address.push_str(&decoded(&whole[from..end], false));
                found.push(Linked {
                    address,
                    place: Place {
                        source,
                        span: Span::new(start, end),
                    },
                    handed: false,
                });
            }
        }
    });
    found
}

/// Every value handed to a snippet call in the writing, which the snippet
/// may write as an address, placed at the value.
pub(crate) fn handed(document: &Document) -> Vec<Linked> {
    let mut found = Vec::new();
    for top in std::iter::once(&document.root).chain(&document.notes) {
        top.walk(&mut |node| {
            let Kind::Call(call) = &node.kind else {
                return;
            };
            for argument in &call.arguments {
                if let (markdown::Value::Text(text), Some(place)) =
                    (&argument.value.value, argument.value.origin.place)
                {
                    found.push(Linked {
                        address: text.clone(),
                        place,
                        handed: true,
                    });
                }
            }
        });
    }
    found
}
