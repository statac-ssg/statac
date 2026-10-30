//! What the words ending a code block's opening line ask of its lines,
//! `numbered=true` and `mark=3-5,8`, and what is wrong with them. A block
//! asking neither is written as it would be without them.

use super::Document;
use super::node::{Kind, Node};
use crate::highlight::lines;
use crate::problem::{Expected, Found, Kind as ProblemKind, Problem};
use crate::source::{Place, Span};

/// The names of the words that may end a fence's opening line to ask how
/// its lines are shown: `numbered=true`, and `mark=` with the lines to
/// mark, `mark=3-5,8`.
pub(crate) const LINE_WORDS: [&str; 2] = ["numbered", "mark"];

/// A word ending a fence's opening line that asks how its lines are shown.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct LineWord<'a> {
    /// One of [`LINE_WORDS`].
    pub(crate) name: &'static str,
    /// Its value, without the quotes round it where it is quoted.
    pub(crate) value: &'a str,
    /// The whole word as the line holds it: `mark="3, 5"`.
    pub(crate) written: &'a str,
}

/// The line opening a fence without the words at its end that ask how its
/// lines are shown, and those words in the order written. A value may be
/// quoted, to hold spaces. A word anywhere else on the line is the
/// caption's.
pub(crate) fn line_words(info: &str) -> (&str, Vec<LineWord<'_>>) {
    let mut rest = info.trim_end();
    let mut words = Vec::new();
    while let Some((start, word)) = last_line_word(rest) {
        words.push(word);
        rest = rest[..start].trim_end();
    }
    words.reverse();
    (rest, words)
}

/// The word asking how a block's lines are shown that ends `line`, with
/// where it starts.
fn last_line_word(line: &str) -> Option<(usize, LineWord<'_>)> {
    let quote = line.chars().next_back().filter(|c| matches!(c, '"' | '\''));
    let (named, value) = match quote {
        Some(quote) => {
            let inside = &line[..line.len() - 1];
            let opens = inside.rfind(quote)?;
            (&line[..opens], &inside[opens + 1..])
        }
        None => {
            let start = line.rfind(char::is_whitespace).map_or(0, |at| at + 1);
            let (name, value) = line[start..].split_once('=')?;
            (&line[..start + name.len() + 1], value)
        }
    };
    let before = named.strip_suffix('=')?;
    let start = before.rfind(char::is_whitespace).map_or(0, |at| at + 1);
    let name = LINE_WORDS
        .into_iter()
        .find(|name| *name == &before[start..])?;
    let word = LineWord {
        name,
        value,
        written: &line[start..],
    };
    Some((start, word))
}

/// What a code block's words ask of its lines, each value that can be
/// read: one that can't is told as the writing is read, and asks nothing.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub(crate) struct Asked {
    pub(crate) numbered: bool,
    pub(crate) marked: Vec<(u64, u64)>,
}

impl Asked {
    pub(crate) fn of(words: &[LineWord<'_>]) -> Asked {
        let mut asked = Asked::default();
        for word in words {
            match read(word) {
                Ok(Read::Numbered(numbered)) => asked.numbered = numbered,
                Ok(Read::Marked(runs)) => asked.marked.extend(runs),
                Err(_) => {}
            }
        }
        asked
    }

    /// Whether the block is written a line at a time.
    pub(crate) fn any(&self) -> bool {
        self.numbered || !self.marked.is_empty()
    }
}

enum Read {
    Numbered(bool),
    Marked(Vec<(u64, u64)>),
}

/// A word's value read, or what is wrong with it.
fn read(word: &LineWord<'_>) -> Result<Read, ProblemKind> {
    let wrong = |expected, respelt: Option<String>| ProblemKind::WrongKind {
        field: word.name.to_owned(),
        expected,
        found: Found::Written(word.value.to_owned()),
        respelt: respelt.map(Into::into),
    };
    match word.name {
        "numbered" => match word.value {
            "true" => Ok(Read::Numbered(true)),
            "false" => Ok(Read::Numbered(false)),
            other => {
                let respelt = match other.to_lowercase().as_str() {
                    "true" | "yes" | "on" => Some("true"),
                    "false" | "no" | "off" => Some("false"),
                    _ => None,
                };
                Err(wrong(Expected::TrueFalse, respelt.map(str::to_owned)))
            }
        },
        _ => lines::read(word.value).map(Read::Marked).ok_or_else(|| {
            let hyphened = word.value.replace(['\u{2013}', '\u{2014}'], "-");
            let respelt = lines::read(&hyphened).map(|_| hyphened);
            wrong(Expected::Lines, respelt)
        }),
    }
}

/// A code block's code: its text without the last line's ending, as a
/// blank line before the closing fence is the code's.
pub(crate) fn block_code(text: &str) -> &str {
    let code = text.strip_suffix('\n').unwrap_or(text);
    code.strip_suffix('\r').unwrap_or(code)
}

/// What is wrong with the words ending each code block's opening line in
/// `document`, read from `text`: a value that can't be read, which stops,
/// and a line to mark that the block hasn't, which is warned of. Each is
/// told at its word.
pub(super) fn problems(document: &Document, text: &str) -> Vec<Problem> {
    let mut problems = Vec::new();
    let mut visit = |node: &Node| {
        let Kind::CodeBlock {
            info,
            text: code,
            fenced: true,
        } = &node.kind
        else {
            return;
        };
        let last = lines::count(block_code(code));
        for word in line_words(info).1 {
            let place = written_at(text, node.place, word.written);
            match read(&word) {
                Err(kind) => problems.push(Problem::at(kind, place)),
                Ok(Read::Marked(runs)) => {
                    if let Some(line) = lines::outside(&runs, 1, last) {
                        let kind = ProblemKind::NoSuchLine {
                            line,
                            first: 1,
                            last,
                            file: None,
                        };
                        problems.push(Problem::at(kind, place));
                    }
                }
                Ok(Read::Numbered(_)) => {}
            }
        }
    };
    document.root.walk(&mut visit);
    for note in &document.notes {
        note.walk(&mut visit);
    }
    problems
}

/// Where `written`, a word ending the opening line of the block at
/// `block`, is written; the block where it can't be found as typed.
fn written_at(text: &str, block: Place, written: &str) -> Place {
    let start = block.span.start as usize;
    let line = &text[start..block.span.end as usize];
    let line = &line[..line.find('\n').unwrap_or(line.len())];
    match line.rfind(written) {
        Some(at) => Place {
            span: Span::new(start + at, start + at + written.len()),
            ..block
        },
        None => block,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_ending_a_fence_ask_how_its_lines_are_shown() {
        let word = |name, value, written| LineWord {
            name,
            value,
            written,
        };
        assert_eq!(
            line_words("rust main.rs numbered=true mark=3-5,8"),
            (
                "rust main.rs",
                vec![
                    word("numbered", "true", "numbered=true"),
                    word("mark", "3-5,8", "mark=3-5,8"),
                ]
            )
        );
        assert_eq!(
            line_words("rust \"The example\" mark=\"3, 5\""),
            (
                "rust \"The example\"",
                vec![word("mark", "3, 5", "mark=\"3, 5\"")]
            )
        );
        assert_eq!(
            line_words("mark='2'"),
            ("", vec![word("mark", "2", "mark='2'")])
        );
        for kept in [
            "rust",
            "rust mark=3 main.rs",
            "rust marks=3",
            "rust x=\"a b\"",
            "rust mark",
        ] {
            assert_eq!(line_words(kept), (kept, Vec::new()), "{kept}");
        }
    }
}
