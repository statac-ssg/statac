//! The writing's maths made MathML Core. Every expression on a page goes
//! in one batch, so equations number on through the page and `\eqref` finds
//! an equation written after it.

use std::collections::HashSet;
use std::ops::Range;
use std::sync::LazyLock;

use math_core::{LatexError, LatexToMathML, MathCoreConfig, MathDisplay};

use super::controls;
use super::node::{Kind, Node};
use super::{Document, MathsFault};
use crate::problem::{Kind as ProblemKind, Problem};
use crate::source::{Place, Span};

static CONVERTER: LazyLock<LatexToMathML> =
    LazyLock::new(|| LatexToMathML::new(MathCoreConfig::default()).expect("no macros to read"));

/// Gives every maths node in `document` its MathML, and tells the maths
/// that can't have any. `file` is the text the document was read from.
pub(super) fn write(document: &mut Document, file: &str) {
    let mut found: Vec<(&str, MathDisplay)> = Vec::new();
    for top in std::iter::once(&document.root).chain(&document.notes) {
        top.walk(&mut |node| {
            if let Kind::Maths { display, text, .. } = &node.kind {
                let display = match display {
                    true => MathDisplay::Block,
                    false => MathDisplay::Inline,
                };
                found.push((text, display));
            }
        });
    }
    if found.is_empty() {
        return;
    }
    let labels: HashSet<String> = found
        .iter()
        .flat_map(|(text, _)| named_by(text, "label"))
        .map(|(_, name)| name.to_owned())
        .collect();
    let mut written = CONVERTER.convert_all(&found).into_iter();
    let mut problems = Vec::new();
    let mut give = |node: &mut Node| {
        let place = node.place;
        let Kind::Maths {
            display,
            text,
            mathml,
        } = &mut node.kind
        else {
            return;
        };
        let maths = Maths {
            file,
            text,
            display: *display,
            place,
        };
        match written.next().expect("one for each expression") {
            Ok(result) => {
                if result.warnings.has_undefined_references() {
                    problems.extend(maths.unlabelled(&labels));
                }
                *mathml = Some(result.mathml);
            }
            Err(error) => problems.push(maths.fault(&error)),
        }
    };
    document.root.walk_mut(&mut give);
    for note in &mut document.notes {
        note.walk_mut(&mut give);
    }
    document.problems.extend(problems);
}

/// One expression, and where it is written.
struct Maths<'a> {
    file: &'a str,
    /// As written, without its dollars, the margin of its later lines or
    /// its control characters.
    text: &'a str,
    display: bool,
    /// The whole of it, dollars and all.
    place: Place,
}

impl Maths<'_> {
    fn dollars(&self) -> usize {
        if self.display { 2 } else { 1 }
    }

    /// Where the text starts and ends in the file.
    fn inner(&self) -> Range<usize> {
        let span = self.place.span.range();
        span.start + self.dollars()..span.end - self.dollars()
    }

    /// Where byte `at` of the text is in the file: on its first line,
    /// counted from the start; on a later one, back from the line's end,
    /// as the margin taken off is at its start. Control characters in the
    /// file are stepped over.
    fn in_file(&self, at: usize) -> usize {
        let inner = self.inner();
        let left_out = |c: &char| controls::left_out(*c);
        let Some(newline) = self.text[..at].rfind('\n') else {
            let mut file_at = inner.start;
            for c in self.text[..at].chars() {
                while let Some(skipped) = self.file[file_at..].chars().next().filter(left_out) {
                    file_at += skipped.len_utf8();
                }
                file_at += c.len_utf8();
            }
            return file_at;
        };
        let line = newline + 1;
        let lines_before = self.text[..at].matches('\n').count();
        let line_end = self.text[line..]
            .find('\n')
            .map_or(self.text.len(), |n| line + n);
        let file = &self.file[inner.clone()];
        let ends = match self.text[line..].contains('\n') {
            false => inner.end,
            true => {
                let newline = file
                    .match_indices('\n')
                    .nth(lines_before)
                    .map_or(file.len(), |(n, _)| n);
                let end = inner.start + newline;
                match self.file.as_bytes()[end - 1] {
                    b'\r' => end - 1,
                    _ => end,
                }
            }
        };
        let mut file_at = ends;
        for c in self.text[at..line_end].chars().rev() {
            while let Some(skipped) = self.file[..file_at].chars().next_back().filter(left_out) {
                file_at -= skipped.len_utf8();
            }
            file_at -= c.len_utf8();
        }
        file_at
    }

    fn span(&self, range: Range<usize>) -> Span {
        let start = self.in_file(range.start);
        let mut end = self.in_file(range.end).max(start);
        while let Some(c) = self.file[start..end].chars().next_back()
            && controls::left_out(c)
        {
            end -= c.len_utf8();
        }
        Span::new(start, end)
    }

    fn problem(&self, found: MathsFault, span: Span) -> Problem {
        Problem::at(
            ProblemKind::MalformedMaths { found },
            Place { span, ..self.place },
        )
    }

    /// math-core's fault told in Statac's words, at the exact text: where
    /// math-core stops at the end of the maths, at what the fault is about.
    fn fault(&self, error: &LatexError) -> Problem {
        let text = self.text;
        let start = text.floor_char_boundary(error.0.start.min(text.len()));
        let end = text.ceil_char_boundary(error.0.end.min(text.len()).max(start));
        let range = start..end;
        let written = text[range.clone()].trim();
        let closing = Span::new(self.inner().end, self.place.span.end as usize);
        let at_end = || {
            let dollars = &self.file[closing.range()];
            match written.is_empty() {
                true => (dollars.to_owned(), closing),
                false => (written.to_owned(), self.span(range.clone())),
            }
        };
        let message = error.error_message();
        let quoted = quoted(&message).unwrap_or_default().to_owned();
        // The name math-core quotes, where it stands inside its braces.
        let name = || {
            let found = text[range.clone()].find(quoted.as_str());
            match found {
                Some(at) if !quoted.is_empty() => self.span(start + at..start + at + quoted.len()),
                _ => self.span(range.clone()),
            }
        };
        let (found, span) = match error.label() {
            "unknown command" => (
                MathsFault::UnknownCommand(written.to_owned()),
                self.span(range),
            ),
            "unknown environment" => (MathsFault::UnknownEnvironment(quoted.clone()), name()),
            "unknown color" => (MathsFault::UnknownColour(quoted.clone()), name()),
            "a group was never closed" => match opening(text, &quoted) {
                Some((opened, opening, closing)) => (
                    MathsFault::Unclosed {
                        opening: Some(opening),
                        closing,
                    },
                    self.span(opened),
                ),
                None => (
                    MathsFault::Unclosed {
                        opening: None,
                        closing: quoted,
                    },
                    closing,
                ),
            },
            "no matching opening for this" => (
                MathsFault::ClosesNothing(written.to_owned()),
                self.span(range),
            ),
            "expected a different environment name here" => (
                MathsFault::Mismatched {
                    written: written.to_owned(),
                    expected: quoted,
                },
                self.span(range),
            ),
            "expected an argument here" => {
                let (next, span) = at_end();
                (MathsFault::MissingBefore(next), span)
            }
            "duplicate" if matches!(written, "^" | "_") => (
                MathsFault::Twice(written.chars().next().expect("one character")),
                self.span(range),
            ),
            _ if written.is_empty() => (
                MathsFault::Unreadable(text.trim().to_owned()),
                self.place.span,
            ),
            _ => (MathsFault::Unreadable(written.to_owned()), self.span(range)),
        };
        self.problem(found, span)
    }

    /// Each `\eqref` naming a label no equation on the page has.
    fn unlabelled(&self, labels: &HashSet<String>) -> Vec<Problem> {
        named_by(self.text, "eqref")
            .into_iter()
            .filter(|(_, name)| !labels.contains(*name))
            .map(|(range, name)| {
                self.problem(MathsFault::NoSuchLabel(name.to_owned()), self.span(range))
            })
            .collect()
    }
}

/// The first stretch of `message` in double quotes.
fn quoted(message: &str) -> Option<&str> {
    let (_, rest) = message.split_once('"')?;
    Some(rest.split_once('"')?.0)
}

/// Each `\command{name}` in `text`: the whole of it, and the name.
fn named_by<'t>(text: &'t str, command: &str) -> Vec<(Range<usize>, &'t str)> {
    let call = format!("\\{command}");
    text.match_indices(&call)
        .filter_map(|(at, _)| {
            let after = at + call.len();
            let rest = &text[after..];
            if rest.starts_with(|c: char| c.is_ascii_alphabetic()) {
                return None;
            }
            let open = after + rest.len() - rest.trim_start().len();
            let name_at = open + text[open..].strip_prefix('{').map(|_| 1)?;
            let close = name_at + text[name_at..].find('}')?;
            Some((at..close + 1, text[name_at..close].trim()))
        })
        .collect()
}

/// What opens the innermost group in `text` that a closing `token` would
/// close, as math-core names the token: where it is, as written, and what
/// closes it as written.
fn opening(text: &str, token: &str) -> Option<(Range<usize>, String, String)> {
    let b = text.as_bytes();
    let mut braces = Vec::new();
    let mut brackets = Vec::new();
    let mut lefts: Vec<(Range<usize>, &str)> = Vec::new();
    let mut begins: Vec<(Range<usize>, &str)> = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'\\' => {
                let name = command_at(text, i);
                let after = i + name.len();
                match (name, argument(text, after)) {
                    ("\\begin", Some((end, env))) => {
                        begins.push((i..end, env));
                        i = end;
                        continue;
                    }
                    ("\\end", Some((end, env))) => {
                        if begins.last().is_some_and(|(_, open)| *open == env) {
                            begins.pop();
                        }
                        i = end;
                        continue;
                    }
                    ("\\left", _) => {
                        let at = after + text[after..].len() - text[after..].trim_start().len();
                        let delimiter = match at < b.len() {
                            true => command_at(text, at),
                            false => "",
                        };
                        lefts.push((i..at + delimiter.len(), delimiter));
                        i = at + delimiter.len();
                        continue;
                    }
                    ("\\right", _) => {
                        lefts.pop();
                    }
                    _ => {}
                }
                i = after;
                continue;
            }
            b'{' => braces.push(i),
            b'}' => {
                braces.pop();
            }
            b'[' => brackets.push(i),
            b']' => {
                brackets.pop();
            }
            _ => {}
        }
        i += text[i..].chars().next().map_or(1, char::len_utf8);
    }
    let one = |at: usize| (at..at + 1, text[at..=at].to_owned());
    match token {
        "}" => braces.pop().map(|at| {
            let (range, opening) = one(at);
            (range, opening, "}".to_owned())
        }),
        "]" => brackets.pop().map(|at| {
            let (range, opening) = one(at);
            (range, opening, "]".to_owned())
        }),
        "\\right" => lefts.pop().map(|(range, delimiter)| {
            let closing = format!("\\right{}", closing_delimiter(delimiter));
            (range.clone(), text[range].to_owned(), closing)
        }),
        _ if token.starts_with("\\end") => begins.pop().map(|(range, env)| {
            (
                range.clone(),
                text[range].to_owned(),
                format!("\\end{{{env}}}"),
            )
        }),
        _ => None,
    }
}

/// The command starting at the backslash at `at`, backslash and all: its
/// letters, or the one character after it. Anything else is one character.
fn command_at(text: &str, at: usize) -> &str {
    let rest = &text[at..];
    let Some(after) = rest.strip_prefix('\\') else {
        return &rest[..rest.chars().next().map_or(0, char::len_utf8)];
    };
    let letters = after.len()
        - after
            .trim_start_matches(|c: char| c.is_ascii_alphabetic())
            .len();
    let length = match letters {
        0 => after.chars().next().map_or(0, char::len_utf8),
        n => n,
    };
    &rest[..1 + length]
}

/// The `{name}` after a command ending at `at`: where it ends, and the name.
fn argument(text: &str, at: usize) -> Option<(usize, &str)> {
    let rest = &text[at..];
    let open = at + rest.len() - rest.trim_start().len();
    let inside = open + text[open..].strip_prefix('{').map(|_| 1)?;
    let close = inside + text[inside..].find('}')?;
    Some((close + 1, text[inside..close].trim()))
}

/// What closes a delimiter `\left` opens, where it is one of a pair.
fn closing_delimiter(opening: &str) -> &str {
    match opening {
        "(" => ")",
        "[" => "]",
        "\\{" => "\\}",
        "\\langle" => "\\rangle",
        "\\lvert" => "\\rvert",
        "\\lVert" => "\\rVert",
        "\\lfloor" => "\\rfloor",
        "\\lceil" => "\\rceil",
        same @ ("|" | "\\|" | ".") => same,
        _ => "",
    }
}
