//! What a reader of the site would find for some words, ranked as the
//! site's own search ranks it, and why each page ranks where it does.

use rayon::prelude::*;

use super::{Built, VERSION};
use crate::content::address::on_the_wire;
use crate::json;
use crate::report::{Report, shown};
use crate::search::{self, Asked, Index, Shown};
use crate::serve::session::{Read, held_back};
use crate::site::{Id, LanguageId};

/// The most pages the panel is handed of each language's search.
const MOST: usize = 100;

/// What a reader would find for `words` in the site as `built` left it,
/// its last word being typed where `typing`, each language's pages apart,
/// the language of the page published as `file` first; `drafts` is
/// whether serve shows drafts and posts waiting for their day. As JSON.
pub(super) fn found(
    built: &Built,
    drafts: bool,
    words: &str,
    typing: bool,
    file: Option<&str>,
) -> String {
    let Some(read) = &built.read else {
        return format!("{{\"v\":{VERSION},\"drafts\":{drafts},\"languages\":null}}");
    };
    let site = read.renderer.site();
    let mut languages: Vec<LanguageId> = site.languages.iter().map(|(id, _)| id).collect();
    let own = (file.and_then(|file| read.pages.get(file)))
        .map(|&(page, _)| site.pages.get(page).language);
    if let Some(own) = own {
        languages.sort_by_key(|&language| language != own);
    }
    let blank = words.trim().is_empty();
    let languages: Vec<String> = (languages.into_iter())
        .map(|language| {
            let code = json::string(search::code(site.languages.get(language)));
            if blank {
                return format!(
                    "{{\"code\":{code},\"pages\":0,\"some\":false,\"suggestion\":null,\
                     \"parts\":0,\"average\":0,\"titleAverage\":0,\"results\":[]}}"
                );
            }
            let index = &built.indexes(read)[language.index()];
            in_language(built, read, index, &code, words, typing)
        })
        .collect();
    format!(
        "{{\"v\":{VERSION},\"drafts\":{drafts},\"languages\":[{}]}}",
        languages.join(",")
    )
}

/// What `index`, of the language whose code is `code` as JSON, finds for
/// `words`, as JSON.
fn in_language(
    built: &Built,
    read: &Read,
    index: &Index,
    code: &str,
    words: &str,
    typing: bool,
) -> String {
    let renderer = &read.renderer;
    let found = search::results(renderer, index, words, typing, MOST);
    let asked = Asked::of(words, typing, index);
    let report = Report::new(renderer.sources(), &built.site, false);
    let results: Vec<String> = (found.shown.iter())
        .map(|shown| result(built, read, index, &report, &asked, shown))
        .collect();
    format!(
        "{{\"code\":{code},\"pages\":{},\"some\":{},\"suggestion\":{},\"parts\":{},\
         \"average\":{},\"titleAverage\":{},\"results\":[{}]}}",
        found.pages,
        found.some,
        or_null(found.suggestion.as_deref()),
        index.records.len(),
        index.average(),
        index.title_average(),
        results.join(","),
    )
}

/// One page found, at its part that matched best, and why it ranks where
/// it does, as JSON.
fn result(
    built: &Built,
    read: &Read,
    index: &Index,
    report: &Report,
    asked: &Asked,
    found: &Shown,
) -> String {
    let renderer = &read.renderer;
    let site = renderer.site();
    let record = &index.records[found.found.record];
    let (draft, waiting) = held_back(renderer, record.page);
    let file = (site.pages.get(record.page).source).map(|source| report.file(source));
    let excerpt: Vec<String> = (found.excerpt.iter())
        .map(|(text, marked)| {
            format!(
                "{{\"text\":{},\"marked\":{marked}}}",
                json::string(&lines_shown(text))
            )
        })
        .collect();
    let why = index.why(site, found.found.record, asked);
    let words: Vec<String> = (why.words.iter())
        .map(|counted| {
            let forms: Vec<String> = (counted.forms.iter())
                .map(|form| json::string(&shown(form)))
                .collect();
            let places: Vec<String> = (counted.places.iter())
                .map(|stands| {
                    format!(
                        "{{\"place\":\"{}\",\"times\":{},\"adds\":{}}}",
                        stands.place.name(),
                        stands.times,
                        stands.adds,
                    )
                })
                .collect();
            format!(
                "{{\"word\":{},\"forms\":[{}],\"starting\":{},\"places\":[{}],\"holding\":{},\
                 \"score\":{}}}",
                json::string(&counted.word),
                forms.join(","),
                counted.starting,
                places.join(","),
                counted.holding,
                counted.score,
            )
        })
        .collect();
    format!(
        "{{\"title\":{},\"day\":{},\"heading\":{},\"href\":{},\"file\":{},\"draft\":{draft},\
         \"waiting\":{},\"excerpt\":[{}],\"matched\":{},\"score\":{},\"length\":{},\
         \"titleLength\":{},\"words\":[{}]}}",
        json::string(&shown(&found.title)),
        or_null(found.day.as_deref().map(shown).as_deref()),
        or_null(found.heading.as_deref().map(shown).as_deref()),
        json::string(&on_the_wire(&format!("{}{}", built.base, found.address))),
        or_null(file.as_deref()),
        or_null(waiting.as_deref()),
        excerpt.join(","),
        found.found.matched,
        found.found.score,
        why.length,
        why.title,
        words.join(","),
    )
}

impl Built {
    /// Each language's search index, by the language, made the first time
    /// any is asked for.
    fn indexes(&self, read: &Read) -> &[Index] {
        self.indexes.get_or_init(|| {
            let renderer = &read.renderer;
            let site = renderer.site();
            let languages: Vec<LanguageId> = site.languages.iter().map(|(id, _)| id).collect();
            (languages.into_par_iter())
                .map(|language| Index::of(site, language, search::records(renderer, language)))
                .collect()
        })
    }
}

/// Text from the site, each of its lines made safe as [`shown`] makes it,
/// with the breaks between them kept.
fn lines_shown(text: &str) -> String {
    text.split('\n').map(shown).collect::<Vec<_>>().join("\n")
}

fn or_null(text: Option<&str>) -> String {
    text.map_or_else(|| "null".to_owned(), json::string)
}
