//! Front matter: the lines between a `---` line at the very start of a
//! Markdown file and the next `---` line.

use crate::source::Span;

/// A Markdown file's front matter and body, as spans of the whole file.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Split {
    /// The YAML between the `---` lines, without them.
    pub front_matter: Option<Span>,
    /// Everything after the closing `---` line, or after any byte-order
    /// mark where there is no front matter.
    pub body: Span,
}

/// Front matter opened by a `---` line that no later `---` line closes.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Unclosed {
    /// The opening `---`.
    pub opening: Span,
    /// The lines after the opening one, to the end of the file.
    pub rest: Span,
}

pub fn split(text: &str) -> Result<Split, Unclosed> {
    let start = if text.starts_with('\u{feff}') {
        '\u{feff}'.len_utf8()
    } else {
        0
    };
    let mut lines = text[start..].split_inclusive('\n').scan(start, |at, line| {
        let line_start = *at;
        *at += line.len();
        Some((line_start, line))
    });
    let opening = match lines.next() {
        Some((_, line)) if is_delimiter(line) && line.ends_with('\n') => line,
        _ => {
            return Ok(Split {
                front_matter: None,
                body: Span::new(start, text.len()),
            });
        }
    };
    let yaml_start = start + opening.len();
    for (line_start, line) in lines {
        if is_delimiter(line) {
            return Ok(Split {
                front_matter: Some(Span::new(yaml_start, line_start)),
                body: Span::new(line_start + line.len(), text.len()),
            });
        }
    }
    Err(Unclosed {
        opening: Span::new(start, start + 3),
        rest: Span::new(yaml_start, text.len()),
    })
}

/// The opening `+++` line of a file whose front matter is written
/// between `+++` lines, as another tool writes it, after any byte-order
/// mark.
pub(super) fn plus_opening(text: &str) -> Option<Span> {
    let start = if text.starts_with('\u{feff}') {
        '\u{feff}'.len_utf8()
    } else {
        0
    };
    let line = text[start..].split('\n').next().unwrap_or_default();
    let line = line.trim_end_matches(['\r', ' ', '\t']);
    (line == "+++").then(|| Span::new(start, start + 3))
}

fn is_delimiter(line: &str) -> bool {
    line.trim_end_matches(['\n', '\r'])
        .trim_end_matches([' ', '\t'])
        == "---"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spelled(text: &str, span: Span) -> &str {
        &text[span.range()]
    }

    #[test]
    fn front_matter_and_body_are_spans_of_the_whole_file() {
        let cases = [
            (
                "---\ntitle: Post 1\n---\nBody.\n",
                "title: Post 1\n",
                "Body.\n",
            ),
            (
                "---\r\ntitle: Post 1\r\n---\r\nBody.\r\n",
                "title: Post 1\r\n",
                "Body.\r\n",
            ),
            (
                "\u{feff}---\ntitle: Post 1\n---\nBody.\n",
                "title: Post 1\n",
                "Body.\n",
            ),
            ("---\ntitle: Post 1\n---", "title: Post 1\n", ""),
            ("---\n---\nBody.\n", "", "Body.\n"),
            (
                "--- \ntitle: Post 1\n---\t\nBody.\n",
                "title: Post 1\n",
                "Body.\n",
            ),
            (
                "---\ntitle: Post 1\n---\nBody.\n---\nMore.\n",
                "title: Post 1\n",
                "Body.\n---\nMore.\n",
            ),
        ];
        for (text, yaml, body) in cases {
            let split = split(text).unwrap();
            assert_eq!(spelled(text, split.front_matter.unwrap()), yaml, "{text:?}");
            assert_eq!(spelled(text, split.body), body, "{text:?}");
        }
    }

    #[test]
    fn without_front_matter_the_body_is_the_file() {
        let cases = [
            ("Body.\n", "Body.\n"),
            ("", ""),
            ("\u{feff}Body.\n", "Body.\n"),
            (" ---\ntitle: Post 1\n---\n", " ---\ntitle: Post 1\n---\n"),
            ("----\nBody.\n", "----\nBody.\n"),
            ("---", "---"),
            (
                "Body.\n---\ntitle: Post 1\n---\n",
                "Body.\n---\ntitle: Post 1\n---\n",
            ),
        ];
        for (text, body) in cases {
            let split = split(text).unwrap();
            assert_eq!(split.front_matter, None, "{text:?}");
            assert_eq!(spelled(text, split.body), body, "{text:?}");
        }
    }

    #[test]
    fn front_matter_between_plus_lines_is_found_only_at_the_start() {
        let cases = [
            ("+++\ntitle = 'Post 1'\n+++\nBody.\n", Some(0)),
            ("+++\r\ntitle = 'Post 1'\r\n+++\r\n", Some(0)),
            ("\u{feff}+++\ntitle = 'Post 1'\n+++\n", Some(3)),
            ("+++ \n", Some(0)),
            ("Body.\n+++\ntitle = 'Post 1'\n+++\n", None),
            (" +++\n", None),
            ("++++\n", None),
            ("---\ntitle: Post 1\n---\n", None),
        ];
        for (text, at) in cases {
            let found = plus_opening(text);
            assert_eq!(found.map(|span| span.start as usize), at, "{text:?}");
            if let Some(span) = found {
                assert_eq!(spelled(text, span), "+++");
            }
        }
    }

    #[test]
    fn front_matter_never_closed_is_placed_at_its_opening() {
        let text = "\u{feff}---\ntitle: Post 1\nBody.\n";
        let unclosed = split(text).unwrap_err();
        assert_eq!(spelled(text, unclosed.opening), "---");
        assert_eq!(unclosed.opening.start, 3);
    }
}
