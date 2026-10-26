//! What a feed is written as, from the text it holds or the name it is
//! published under.

use crate::problem::FeedFormat;
use crate::source::Sources;

/// What `text` is written as where it is a feed: XML whose root is
/// RSS's or Atom's, or a JSON Feed.
pub(crate) fn of(text: &str) -> Option<FeedFormat> {
    let mut rest = text.trim_start_matches('\u{feff}').trim_start();
    if rest.starts_with('{') {
        if !rest.contains("jsonfeed.org/version/") {
            return None;
        }
        let mut sources = Sources::default();
        let source = sources.add("feed.json", rest.to_owned());
        let node = crate::yaml::read_json(&sources, source).ok()?;
        let crate::yaml::Value::Mapping(entries) = &node.value else {
            return None;
        };
        return entries
            .iter()
            .any(|entry| {
                matches!(&entry.key.value, crate::yaml::Value::Scalar(key) if key == "version")
                    && matches!(&entry.value.value, crate::yaml::Value::Scalar(version)
                        if version.starts_with("https://jsonfeed.org/version/"))
            })
            .then_some(FeedFormat::Json);
    }
    // The XML declaration, a stylesheet and any comment come before the
    // root.
    while let Some(close) = ["<?", "<!--"]
        .iter()
        .zip(["?>", "-->"])
        .find_map(|(open, close)| rest.starts_with(open).then_some(close))
    {
        rest = rest[rest.find(close)? + close.len()..].trim_start();
    }
    [
        ("<rss", FeedFormat::Rss),
        ("<feed", FeedFormat::Atom),
        ("<rdf:RDF", FeedFormat::Rss),
    ]
    .into_iter()
    .find(|(root, _)| {
        rest.strip_prefix(root)
            .is_some_and(|after| after.starts_with([' ', '\t', '\r', '\n', '>']))
    })
    .map(|(_, format)| format)
}

/// The format a feed's file name says it is in: `rss.xml` and
/// `atom.xml`, or any name ending `.rss`, `.atom` or `.json`. Most
/// names, `feed.xml` and `index.xml` among them, say none.
pub(crate) fn named(file: &str) -> Option<FeedFormat> {
    let file = file.to_ascii_lowercase();
    let (stem, extension) = file.rsplit_once('.')?;
    match (stem, extension) {
        (_, "json") => Some(FeedFormat::Json),
        (_, "rss") | ("rss", "xml") => Some(FeedFormat::Rss),
        (_, "atom") | ("atom", "xml") => Some(FeedFormat::Atom),
        _ => None,
    }
}
