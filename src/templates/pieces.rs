//! The pieces of the writing a site writes itself, built with the files in
//! `snippets/markdown/`.

use std::collections::BTreeMap;
use std::sync::Arc;

use minijinja::{Error, Value};
use url::Url;

use super::Templates;
use super::frame;
use super::render::Shared;
use super::values::{PageValue, SiteValue};
use crate::markdown::{Piece, PieceValue};
use crate::reads::Key;
use crate::site::PageId;

/// Builds `piece` for the page `page` with the site's file for it, handed
/// `values` and what the site says of them: whether a link leaves the
/// site, and the words for a callout's kind. None where the site has no
/// file for it.
pub(super) fn build(
    shared: &Arc<Shared>,
    page: PageId,
    piece: Piece,
    values: Vec<(&'static str, PieceValue)>,
) -> Option<Result<String, Error>> {
    let templates = &shared.templates;
    let &file = templates.pieces.get(&piece)?;
    frame::read(Key::Template(Templates::id(file)));
    if templates.unusable.contains(&file) {
        return Some(Err(frame::told()));
    }
    let mut context: BTreeMap<&str, Value> = BTreeMap::new();
    for (name, handed) in values {
        if piece == Piece::Link
            && name == "address"
            && let PieceValue::Text(address) = &handed
        {
            context.insert("outside", Value::from(outside(shared, address)));
        }
        if piece == Piece::Image
            && name == "address"
            && let PieceValue::Text(address) = &handed
        {
            let template = &templates.files[file].name;
            let photo = super::media::for_piece(shared, page, address, template);
            context.insert("photo", photo);
        }
        if piece == Piece::Callout
            && name == "kind"
            && let PieceValue::Text(kind) = &handed
        {
            let language = shared.site.pages.get(page).language;
            let words = super::render::words_of(shared, language).callout(kind);
            context.insert("title", Value::from(words.unwrap_or_default()));
        }
        context.insert(name, value(handed));
    }
    context.insert("page", PageValue::value(shared, page));
    context.insert("site", SiteValue::value(shared, frame::language()));
    // The line ending a file closes with would otherwise stand inside the
    // sentence a link or picture sits in.
    let built = templates
        .env
        .get_template(&templates.files[file].name)
        .and_then(|template| {
            frame::begin(file, None)?;
            frame::end(
                file,
                super::indent::render(
                    &template,
                    context,
                    templates.placing(&templates.files[file].name),
                    stands_alone(piece),
                ),
            )?
        });
    Some(built.map(|html| html.trim_end().to_owned()))
}

/// Whether the writing puts `piece` on lines of its own, not among the words.
fn stands_alone(piece: Piece) -> bool {
    match piece {
        Piece::Heading | Piece::Code | Piece::Footnotes | Piece::Callout | Piece::Table => true,
        Piece::Link | Piece::Image | Piece::InlineCode | Piece::Footnote => false,
    }
}

/// Whether the site's file for `piece` can read what it is handed as
/// `name`: it spells the name somewhere, or a word that reaches what it
/// is handed some other way. A file that spells none of them can't, and
/// one that spells one may still not.
pub(super) fn reads(shared: &Shared, piece: Piece, name: &str) -> bool {
    let templates = &shared.templates;
    let Some(&file) = templates.pieces.get(&piece) else {
        return true;
    };
    frame::read(Key::Template(Templates::id(file)));
    let text = shared.sources.text(templates.files[file].source);
    [name, "debug", "extends"]
        .iter()
        .any(|word| text.contains(word))
}

fn value(handed: PieceValue) -> Value {
    match handed {
        PieceValue::Text(text) => Value::from(text),
        PieceValue::Number(n) => Value::from(n),
        PieceValue::Truth(truth) => Value::from(truth),
        PieceValue::Html(html) => Value::from_safe_string(html),
        PieceValue::List(items) => Value::from(items.into_iter().map(value).collect::<Vec<_>>()),
        PieceValue::Record(fields) => Value::from(
            fields
                .into_iter()
                .map(|(name, field)| (name, value(field)))
                .collect::<BTreeMap<_, _>>(),
        ),
    }
}

/// Whether a link to `address` leaves the site, by the site's `url` and
/// `also_at`.
fn outside(shared: &Shared, address: &str) -> bool {
    frame::read(Key::Setting(shared.name_id("url")));
    frame::read(Key::Setting(shared.name_id("also_at")));
    let settings = &shared.settings;
    // Where the run was given another address, the site's own is one it
    // is also at.
    let home = settings.home().filter(|_| settings.elsewhere());
    let also_at: Vec<&str> = settings
        .texts("also_at")
        .iter()
        .map(|other| other.value.as_str())
        .chain(home)
        .collect();
    leaves(address, settings.text("url").map(|url| url.value), &also_at)
}

/// Whether `address` leads to another site: an `http`, `https` or `//`
/// address on a host that is neither `url`'s nor one `also_at` names. On a
/// site with no `url`, every such address does. Hosts are compared as the
/// URL standard writes them, so capitals and an internationalised name
/// spelled either way are alike.
fn leaves(address: &str, url: Option<&str>, also_at: &[&str]) -> bool {
    let site = url.and_then(|url| Url::parse(url).ok());
    let link = match Url::parse(address) {
        Ok(link) => link,
        Err(url::ParseError::RelativeUrlWithoutBase) if address.starts_with("//") => {
            let joined = match &site {
                Some(site) => site.join(address),
                None => Url::parse(&format!("https:{address}")),
            };
            match joined {
                Ok(link) => link,
                Err(_) => return false,
            }
        }
        Err(_) => return false,
    };
    if !matches!(link.scheme(), "http" | "https") {
        return false;
    }
    let Some(host) = link.host_str() else {
        return false;
    };
    let own = site.as_ref().and_then(Url::host_str) == Some(host);
    let also = also_at
        .iter()
        .any(|other| host_named(other).as_deref() == Some(host));
    !(own || also)
}

/// The host an `also_at` entry names, written bare, `example.net`, or as a
/// whole address.
fn host_named(written: &str) -> Option<String> {
    let whole = Url::parse(written)
        .ok()
        .filter(|url| url.host_str().is_some());
    let url = match whole {
        Some(url) => url,
        None => Url::parse(&format!("https://{written}")).ok()?,
    };
    url.host_str().map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_link_leaves_the_site_only_for_a_host_the_site_doesnt_name() {
        let url = Some("https://example.org/blog/");
        let also = ["example.net"];
        for (address, leaving) in [
            ("https://Example.org/a/", false),
            ("//example.org", false),
            ("http://user@example.org:8080?q", false),
            ("https://example.net/a", false),
            ("https://other.org", true),
            ("//other.org/a", true),
            ("/about/", false),
            ("mailto:a@example.org", false),
        ] {
            assert_eq!(leaves(address, url, &also), leaving, "{address}");
        }
        assert!(leaves("https://example.org", None, &[]));
        assert!(leaves("//example.org", None, &[]));
        assert!(!leaves("/about/", None, &[]));
    }

    #[test]
    fn an_internationalised_host_is_alike_however_it_is_spelled() {
        let url = Some("https://xn--bcher-kva.example/");
        assert!(!leaves("https://Bücher.example/a", url, &[]));
        assert!(!leaves(
            "https://bücher.example/a",
            None,
            &["BÜCHER.example"]
        ));
        assert!(leaves("https://buecher.example/", url, &[]));
    }
}
