//! A `posts:` line read as the run of filters it writes:
//! `site.posts | in("blog") | tagged("Rust")`. Each filter is the one
//! templates use.

use super::filter::{CHAINABLE, Filter};
use crate::problem::{Kind, ListFault, Word};
use crate::site::{LanguageId, PageId, Site};
use crate::source::Span;

/// Where every line starts.
const THE_POSTS: &str = "site.posts";

/// The posts of `posts` the line keeps, in their order; or the problem
/// with it and where in the line it is.
pub(super) fn members(
    site: &Site,
    language: LanguageId,
    written: &str,
    posts: &[PageId],
) -> Result<Vec<PageId>, (Kind, Span)> {
    let malformed = |fault, at: Span| (Kind::MalformedList { fault }, at);
    let mut segments = split(written, '|').into_iter();
    let first = trimmed(written, segments.next().expect("one segment at least"));
    let start = &written[first.range()];
    if start != THE_POSTS {
        return Err(malformed(
            ListFault::NotFromThePosts(start.to_owned()),
            first,
        ));
    }
    let mut kept = posts.to_vec();
    for segment in segments {
        let at = trimmed(written, segment);
        let text = &written[at.range()];
        if text.is_empty() {
            let pipe = segment.start as usize - 1;
            return Err(malformed(
                ListFault::NothingAfterPipe,
                Span::new(pipe, pipe + 1),
            ));
        }
        let open = text.find('(');
        let name_end = at.start as usize + open.map_or(text.len(), |o| text[..o].trim_end().len());
        let name_at = Span::new(at.start as usize, name_end);
        let name = &written[name_at.range()];
        let Some(&(_, wants)) = CHAINABLE.iter().find(|(filter, _)| *filter == name) else {
            let chainable = CHAINABLE.iter().map(|(filter, _)| *filter);
            return Err(match crate::suggest::nearest(name, chainable) {
                Some(nearest) => (
                    Kind::UnknownWord {
                        word: Word::Filter,
                        written: name.to_owned(),
                        nearest: Some(nearest.to_owned()),
                    },
                    name_at,
                ),
                None => malformed(ListFault::NotChainable(name.to_owned()), name_at),
            });
        };
        let Some(open) = open else {
            return Err(malformed(ListFault::NoBrackets(name.to_owned()), at));
        };
        let open_at = at.start as usize + open;
        if !text.ends_with(')') {
            return Err(malformed(
                ListFault::Unclosed('('),
                Span::new(open_at, open_at + 1),
            ));
        }
        let inside = Span::new(open_at + 1, at.end as usize - 1);
        let mut values = Vec::new();
        for part in split(&written[inside.range()], ',') {
            let part = Span::new(
                inside.start as usize + part.start as usize,
                inside.start as usize + part.end as usize,
            );
            let at = trimmed(written, part);
            if at.start < at.end {
                values.push(value(written, at)?);
            }
        }
        if values.len() != wants {
            return Err(malformed(
                ListFault::Values {
                    filter: name.to_owned(),
                    wants,
                    given: values.len(),
                },
                at,
            ));
        }
        let filter = match name {
            "tagged" => Filter::tagged(site, language, &values[0].0),
            "in" => Filter::in_folder(site, language, &values[0].0),
            _ => Filter::where_field(site, &values[0].0, &values[1].0),
        };
        let filter = filter.map_err(|kind| (kind, values[0].1))?;
        kept.retain(|&page| filter.keeps(site, page));
    }
    Ok(kept)
}

/// One value inside a filter's brackets, its quotes taken off, and where
/// its words are.
fn value(written: &str, at: Span) -> Result<(String, Span), (Kind, Span)> {
    let text = &written[at.range()];
    let Some(quote) = text.chars().next().filter(|c| matches!(c, '"' | '\'')) else {
        return Ok((text.to_owned(), at));
    };
    if text.len() < 2 || !text.ends_with(quote) {
        let start = at.start as usize;
        return Err((
            Kind::MalformedList {
                fault: ListFault::Unclosed(quote),
            },
            Span::new(start, start + 1),
        ));
    }
    let inner = Span::new(at.start as usize + 1, at.end as usize - 1);
    Ok((written[inner.range()].to_owned(), inner))
}

/// The line cut at each `by` outside quotes; always one piece at least.
fn split(written: &str, by: char) -> Vec<Span> {
    let mut pieces = Vec::new();
    let mut start = 0;
    let mut quote = None;
    for (i, c) in written.char_indices() {
        match quote {
            Some(open) if c == open => quote = None,
            Some(_) => {}
            None if c == '\'' || c == '"' => quote = Some(c),
            None if c == by => {
                pieces.push(Span::new(start, i));
                start = i + c.len_utf8();
            }
            None => {}
        }
    }
    pieces.push(Span::new(start, written.len()));
    pieces
}

/// A piece of the line without the spaces around it.
fn trimmed(written: &str, span: Span) -> Span {
    let text = &written[span.range()];
    let start = span.start as usize + (text.len() - text.trim_start().len());
    Span::new(start, start + text.trim().len())
}
