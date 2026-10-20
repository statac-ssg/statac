//! The site's `templates/` and `snippets/`, and what each snippet takes.
//!
//! A template is named bare, `base` for `templates/base.html`, matching
//! `{% extends "base" %}` and `layout: base`. A template written as
//! anything other than HTML keeps its whole name, `feed.xml`. A snippet is
//! called by its file's name, `{{ card(post) }}` for `snippets/card.html`.
//!
//! A snippet takes the names it reads, other than `page` and `site`. A call
//! hands each by name; a snippet reading one name may be handed it without
//! one. A snippet's first line may say which values a call gives without
//! names, in order, `{# takes photo, alt #}`, or that every value given
//! without a name is one of a list, `{# takes photos... #}`.
//!
//! `snippets/markdown/` holds the pieces of the writing a site writes
//! itself, `heading.html` for every heading. No call reaches them.

use std::path::Path;

use crate::markdown::Piece;
use crate::problem::{Kind, Problem, TakesFault, TemplateFault, Word};
use crate::source::{Place, SourceId, Sources, Span};

pub(crate) const TEMPLATES: &str = "templates";
pub(crate) const SNIPPETS: &str = "snippets";
/// The folder in `snippets/` holding the pieces, as a path in it begins.
const PIECES: &str = "markdown/";

/// The name Statac hands a snippet itself, which no call may give.
pub(super) const SUPPLIED: &[&str] = &["standalone"];

/// The names a block call in the writing hands a snippet, which a call in
/// a template hands it by name.
pub(super) const WRAPPED: &[&str] = &["body", "lines"];

/// What a file is for.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) enum Role {
    Template,
    /// A snippet, by the name it is called by.
    Snippet(String),
    Piece(Piece),
    /// A text or a colour of a card's description, built for each page.
    CardText,
}

#[derive(Debug)]
pub(super) struct File {
    /// The name the engine holds it under.
    pub name: String,
    pub source: SourceId,
    pub role: Role,
}

/// What a call's values without names fill.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) enum Takes {
    /// One or two names, filled in the order the values are written.
    InOrder(Vec<String>),
    /// Every value is one of these, handed over as a list.
    AnyNumberOf(String),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) struct Snippet {
    pub name: String,
    /// The file's index among the site's template files.
    pub file: usize,
    /// The names the snippet reads, in the order first read.
    pub params: Vec<String>,
    /// The names of the writing a block call wraps that the snippet reads,
    /// which a call in a template hands by name.
    pub wrapped: Vec<String>,
    pub takes: Option<Takes>,
    /// Whether the engine could read the file. Where it couldn't, what the
    /// snippet reads is unknown, and no call is held to it.
    pub readable: bool,
}

impl Snippet {
    /// What a call that hands the snippet everything it reads holds,
    /// written as the call would be: `post`, or `photo=photo, alt=alt`.
    pub fn call_arguments(&self) -> String {
        let unnamed = match &self.takes {
            Some(Takes::InOrder(names)) => names.clone(),
            None if self.params.len() == 1 => self.params.clone(),
            _ => Vec::new(),
        };
        self.params
            .iter()
            .map(|p| match unnamed.contains(p) {
                true => p.clone(),
                false => format!("{p}={p}"),
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// The file a template's name stands for, as the author would find it.
pub(super) fn template_path(name: &str) -> String {
    let last = name.rsplit('/').next().unwrap_or(name);
    match last.contains('.') {
        true => format!("{TEMPLATES}/{name}"),
        false => format!("{TEMPLATES}/{name}.html"),
    }
}

/// The template a name in `extends` or `layout:` refers to, which may be
/// written with its `.html`.
pub(super) fn named(written: &str) -> &str {
    written.strip_suffix(".html").unwrap_or(written)
}

/// The name the engine holds a piece's file under, which no call can
/// name.
pub(super) fn piece_name(piece: Piece) -> String {
    format!("{SNIPPETS}/{PIECES}{}", piece.name())
}

/// A file in `snippets/markdown/` that is no piece, by its path there.
fn not_a_piece(file: &str) -> Problem {
    let path = |name: &str| format!("{SNIPPETS}/{PIECES}{name}");
    let bare = file.strip_suffix(".html").unwrap_or(file);
    Problem::nowhere(Kind::UnknownWord {
        word: Word::File,
        written: path(file),
        nearest: Piece::meant_by(bare).map(|piece| path(&format!("{}.html", piece.name()))),
    })
}

/// Reads every template and snippet of the site at `root` into `sources`.
pub(super) fn gather(root: &Path, sources: &mut Sources, problems: &mut Vec<Problem>) -> Vec<File> {
    let real = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let mut files = Vec::new();
    for folder in [TEMPLATES, SNIPPETS] {
        let found = crate::assets::walk(root, &real, folder, false, problems);
        for (rel, at) in found {
            // A card's description and the files in `templates/og/` are
            // kept in `templates/`, and none of them is a template.
            if folder == TEMPLATES && crate::cards::owns(&rel) {
                continue;
            }
            let (name, role) = match folder {
                TEMPLATES => match rel.strip_suffix(".html") {
                    Some(bare) => (bare.to_owned(), Role::Template),
                    None => (rel.clone(), Role::Template),
                },
                _ => {
                    if let Some(file) = rel.strip_prefix(PIECES) {
                        match file.strip_suffix(".html").and_then(Piece::of_name) {
                            Some(piece) => (piece_name(piece), Role::Piece(piece)),
                            None => {
                                problems.push(not_a_piece(file));
                                continue;
                            }
                        }
                    } else {
                        let Some(bare) = rel.strip_suffix(".html") else {
                            continue;
                        };
                        (format!("{SNIPPETS}/{bare}"), Role::Snippet(bare.to_owned()))
                    }
                }
            };
            let shown = format!("{folder}/{rel}");
            match crate::assets::read_walked_text(&real, &at) {
                Ok(text) => files.push(File {
                    name,
                    source: sources.add(shown, text),
                    role,
                }),
                Err(error) => problems.push(Problem::nowhere(Kind::CantRead {
                    path: shown.into(),
                    reason: crate::problem::system_reason(&error),
                })),
            }
        }
    }
    files
}

/// A `{# takes #}` line as read, with where each name sits.
#[derive(Debug)]
pub(super) struct Line {
    pub takes: Takes,
    pub names: Vec<(String, Span)>,
}

const TAKES: &str = "takes";
const ANY_NUMBER: &str = "...";
/// How many values a call may give without names, one by one: a third
/// would be told from the second by its place alone.
const IN_ORDER_AT_MOST: usize = 2;

/// Reads the line a snippet opens with. None for a file that doesn't open
/// with one.
pub(super) fn takes_line(source: SourceId, text: &str) -> Option<Result<Line, Problem>> {
    let refuse = |fault, span: Span| Problem::at(fault_kind(fault), Place { source, span });
    let first = text.split('\n').next().unwrap_or_default();
    let indent = first.len() - first.trim_start().len();
    let rest = first[indent..].strip_prefix("{#")?;
    let mut at = indent + 2;
    let rest = match rest.strip_prefix('-') {
        Some(rest) => {
            at += 1;
            rest
        }
        None => rest,
    };
    let spaced = rest.len() - rest.trim_start().len();
    let rest = rest[spaced..].strip_prefix(TAKES)?;
    at += spaced + TAKES.len();
    // `{# takesomething #}` is a comment about something else.
    if !(rest.starts_with(char::is_whitespace) || rest.starts_with("#}") || rest.starts_with("-#}"))
    {
        return None;
    }
    let opened = Span::new(indent, at);
    let Some(close) = rest.find("#}") else {
        return Some(Err(refuse(TakesFault::Unclosed, opened)));
    };
    let body = rest[..close].strip_suffix('-').unwrap_or(&rest[..close]);
    Some(names(body, at).map_err(|(fault, span)| refuse(fault, span)))
}

fn fault_kind(fault: TakesFault) -> Kind {
    Kind::MalformedTemplate {
        found: TemplateFault::Takes(fault),
    }
}

/// The names a line's body holds, `at` being where the body starts.
fn names(body: &str, at: usize) -> Result<Line, (TakesFault, Span)> {
    if body.trim().is_empty() {
        return Err((
            TakesFault::NamesNothing,
            Span::new(at, at + body.len().max(1)),
        ));
    }
    let mut pieces: Vec<(&str, Span)> = Vec::new();
    let mut start = 0;
    for piece in body.split(',') {
        let trimmed = piece.trim();
        let lead = piece.len() - piece.trim_start().len();
        let from = at + start + lead;
        if trimmed.is_empty() {
            let comma = if start + piece.len() < body.len() {
                at + start + piece.len()
            } else {
                at + start - 1
            };
            return Err((TakesFault::Missing, Span::new(comma, comma + 1)));
        }
        pieces.push((trimmed, Span::new(from, from + trimmed.len())));
        start += piece.len() + 1;
    }
    if pieces.len() > 1
        && let Some((name, span)) = pieces.iter().find(|(name, _)| name.ends_with(ANY_NUMBER))
    {
        return Err((TakesFault::NotAlone((*name).to_owned()), *span));
    }
    let mut names: Vec<(String, Span)> = Vec::new();
    for (piece, span) in &pieces {
        let name = piece.strip_suffix(ANY_NUMBER).unwrap_or(piece);
        if !is_a_name(name) {
            return Err((TakesFault::NotAName((*piece).to_owned()), *span));
        }
        if names.iter().any(|(known, _)| known == name) {
            return Err((TakesFault::Twice(name.to_owned()), *span));
        }
        let start = span.start as usize;
        names.push((name.to_owned(), Span::new(start, start + name.len())));
    }
    if names.len() > IN_ORDER_AT_MOST {
        let third = names[IN_ORDER_AT_MOST].1;
        let last = names[names.len() - 1].1;
        return Err((
            TakesFault::TooMany,
            Span::new(third.start as usize, last.end as usize),
        ));
    }
    let takes = match pieces.as_slice() {
        [(piece, _)] if piece.ends_with(ANY_NUMBER) => Takes::AnyNumberOf(names[0].0.clone()),
        _ => Takes::InOrder(names.iter().map(|(name, _)| name.clone()).collect()),
    };
    Ok(Line { takes, names })
}

fn is_a_name(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// What the line names that the snippet can't take: a name it never
/// reads, or one Statac hands it.
pub(super) fn line_against(line: &Line, source: SourceId, reads: &[String]) -> Vec<Problem> {
    let mut problems = Vec::new();
    for (name, span) in &line.names {
        let fault = if SUPPLIED.contains(&name.as_str()) {
            TakesFault::Supplied(name.clone())
        } else if !reads.contains(name) {
            TakesFault::NeverRead(name.clone())
        } else {
            continue;
        };
        problems.push(Problem::at(
            fault_kind(fault),
            Place {
                source,
                span: *span,
            },
        ));
    }
    problems
}

/// Where the values a call gives without names go.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Unnamed {
    Nothing,
    /// Each fills the name beside it, in the order written.
    Fills(Vec<String>),
    /// Every one is one of these, handed over as a list.
    Gathers(String),
    /// Which is which is the author's to say.
    NeedsNames,
    /// More than the snippet takes without names; the first extra is at
    /// `past`.
    TooMany {
        takes: Vec<String>,
        past: usize,
    },
    /// The value at `at` fills a name the call also gives by name.
    Twice {
        name: String,
        at: usize,
    },
}

/// The rule for values given without names, written once. Without a
/// `takes` line, a value with no name is clear only when the snippet reads
/// exactly one name and nothing else claims it: the names a snippet reads
/// have no order of their own, and matching by one the author never wrote
/// would be a guess.
pub(super) fn unnamed_fills(
    takes: Option<&Takes>,
    params: &[String],
    given: usize,
    already: impl Fn(&str) -> bool,
) -> Unnamed {
    if given == 0 {
        return Unnamed::Nothing;
    }
    match takes {
        Some(Takes::InOrder(names)) => {
            if given > names.len() {
                return Unnamed::TooMany {
                    takes: names.clone(),
                    past: names.len(),
                };
            }
            match names[..given].iter().position(|name| already(name)) {
                Some(at) => Unnamed::Twice {
                    name: names[at].clone(),
                    at,
                },
                None => Unnamed::Fills(names[..given].to_vec()),
            }
        }
        Some(Takes::AnyNumberOf(name)) if already(name) => Unnamed::Twice {
            name: name.clone(),
            at: 0,
        },
        Some(Takes::AnyNumberOf(name)) => Unnamed::Gathers(name.clone()),
        None => match (given, params) {
            (1, [only]) if !already(only) => Unnamed::Fills(vec![only.clone()]),
            _ => Unnamed::NeedsNames,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(text: &str) -> Option<Result<Line, Problem>> {
        let mut sources = Sources::default();
        let id = sources.add("snippets/image.html", text.to_owned());
        takes_line(id, text)
    }

    fn takes(text: &str) -> Takes {
        read(text)
            .expect("a line")
            .expect("a line that reads")
            .takes
    }

    fn refused(text: &str) -> TakesFault {
        match read(text)
            .expect("a line")
            .expect_err("a refused line")
            .kind
        {
            Kind::MalformedTemplate {
                found: TemplateFault::Takes(fault),
            } => fault,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn one_or_two_names_are_filled_in_order() {
        assert_eq!(
            takes("{# takes photo, alt #}\n<img>"),
            Takes::InOrder(vec!["photo".into(), "alt".into()])
        );
        assert_eq!(
            takes("{#- takes text -#}\n"),
            Takes::InOrder(vec!["text".into()])
        );
        assert_eq!(
            takes("{# takes photos... #}\n"),
            Takes::AnyNumberOf("photos".into())
        );
    }

    #[test]
    fn a_file_that_opens_otherwise_has_no_line() {
        for text in [
            "<figure>\n{# takes photo #}",
            "{# a note about takes #}\n",
            "{# takesomething #}\n",
            "\n{# takes photo #}",
            "",
        ] {
            assert!(read(text).is_none(), "{text:?}");
        }
    }

    #[test]
    fn each_mistake_in_the_line_is_refused() {
        assert_eq!(refused("{# takes #}"), TakesFault::NamesNothing);
        assert_eq!(refused("{# takes photo, #}"), TakesFault::Missing);
        assert_eq!(refused("{# takes , alt #}"), TakesFault::Missing);
        assert_eq!(
            refused("{# takes photo-name #}"),
            TakesFault::NotAName("photo-name".into())
        );
        assert_eq!(
            refused("{# takes photo, photo #}"),
            TakesFault::Twice("photo".into())
        );
        assert_eq!(refused("{# takes a, b, c #}"), TakesFault::TooMany);
        assert_eq!(
            refused("{# takes photos..., alt #}"),
            TakesFault::NotAlone("photos...".into())
        );
        assert_eq!(refused("{# takes photo\n#}"), TakesFault::Unclosed);
    }

    #[test]
    fn values_without_names_fill_in_order_or_need_names() {
        let in_order = Takes::InOrder(vec!["photo".into(), "alt".into()]);
        let params: Vec<String> = vec!["photo".into(), "alt".into(), "caption".into()];
        assert_eq!(
            unnamed_fills(Some(&in_order), &params, 2, |_| false),
            Unnamed::Fills(vec!["photo".into(), "alt".into()])
        );
        assert_eq!(
            unnamed_fills(Some(&in_order), &params, 3, |_| false),
            Unnamed::TooMany {
                takes: vec!["photo".into(), "alt".into()],
                past: 2
            }
        );
        assert_eq!(
            unnamed_fills(Some(&in_order), &params, 1, |name| name == "photo"),
            Unnamed::Twice {
                name: "photo".into(),
                at: 0
            }
        );
        assert_eq!(
            unnamed_fills(None, &params, 1, |_| false),
            Unnamed::NeedsNames
        );
        assert_eq!(
            unnamed_fills(None, &["post".into()], 1, |_| false),
            Unnamed::Fills(vec!["post".into()])
        );
    }

    #[test]
    fn a_template_is_found_by_its_bare_name() {
        assert_eq!(template_path("base"), "templates/base.html");
        assert_eq!(template_path("feed.xml"), "templates/feed.xml");
    }
}
