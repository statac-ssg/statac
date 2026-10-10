//! The addresses in a stylesheet that name other files: every `url(...)`
//! and every `@import`. The stylesheet is read as a browser reads it, far
//! enough to tell an address from a comment or a string that merely looks
//! like one, and as bytes, since every character that shapes CSS is ASCII.

use std::ops::Range;

/// Where each address sits, inside its quotes or its `url()`.
pub(super) fn references(css: &[u8]) -> Vec<Range<usize>> {
    let mut found = Vec::new();
    let mut read = Reader { css, at: 0 };
    let mut importing = false;
    while let Some(byte) = read.peek(0) {
        match byte {
            b'/' if read.peek(1) == Some(b'*') => {
                read.comment();
                continue;
            }
            b'"' | b'\'' => {
                if let Some(inside) = read.string()
                    && importing
                {
                    found.push(inside);
                }
            }
            b'\\' => read.at += 2,
            b'@' => {
                read.at += 1;
                importing = read.name().eq_ignore_ascii_case(b"import");
                continue;
            }
            byte if starts_a_name(byte) => {
                if read.name().eq_ignore_ascii_case(b"url") && read.peek(0) == Some(b'(') {
                    read.at += 1;
                    found.extend(read.url());
                }
            }
            byte if byte.is_ascii_whitespace() => {
                read.at += 1;
                continue;
            }
            _ => read.at += 1,
        }
        importing = false;
    }
    found
}

/// The stylesheet with each range replaced, ranges in order.
pub(super) fn rewritten(css: &[u8], replacements: &[(Range<usize>, String)]) -> Vec<u8> {
    let mut out = Vec::with_capacity(css.len());
    let mut done = 0;
    for (range, with) in replacements {
        out.extend_from_slice(&css[done..range.start]);
        out.extend_from_slice(with.as_bytes());
        done = range.end;
    }
    out.extend_from_slice(&css[done..]);
    out
}

fn starts_a_name(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_' || byte == b'-' || byte >= 0x80
}

struct Reader<'a> {
    css: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn peek(&self, ahead: usize) -> Option<u8> {
        self.css.get(self.at + ahead).copied()
    }

    /// Past a comment, which runs to the end if it never closes.
    fn comment(&mut self) {
        self.at += 2;
        while self.at < self.css.len() && !self.css[self.at..].starts_with(b"*/") {
            self.at += 1;
        }
        self.at = (self.at + 2).min(self.css.len());
    }

    /// Past a string, and where its contents sit. A string a line ends
    /// before it closes is no string to a browser.
    fn string(&mut self) -> Option<Range<usize>> {
        let quote = self.css[self.at];
        self.at += 1;
        let start = self.at;
        while let Some(byte) = self.peek(0) {
            match byte {
                b'\\' => self.at += 2,
                b'\n' | b'\r' | b'\x0c' => return None,
                byte if byte == quote => {
                    self.at += 1;
                    return Some(start..self.at - 1);
                }
                _ => self.at += 1,
            }
        }
        self.at = self.css.len();
        None
    }

    fn name(&mut self) -> &[u8] {
        let start = self.at;
        while let Some(byte) = self.peek(0) {
            match byte {
                b'\\' => self.at += 2,
                byte if starts_a_name(byte) || byte.is_ascii_digit() => self.at += 1,
                _ => break,
            }
        }
        self.at = self.at.min(self.css.len());
        &self.css[start..self.at]
    }

    /// Past what follows `url(`, and where its address sits. An unquoted
    /// address broken by a quote, a bracket or a space before its end is
    /// nothing a browser follows.
    fn url(&mut self) -> Option<Range<usize>> {
        while self.peek(0).is_some_and(|b| b.is_ascii_whitespace()) {
            self.at += 1;
        }
        if matches!(self.peek(0), Some(b'"' | b'\'')) {
            return self.string();
        }
        let start = self.at;
        let mut end = None;
        while let Some(byte) = self.peek(0) {
            match byte {
                b')' => {
                    let end = end.unwrap_or(self.at);
                    self.at += 1;
                    return (end > start).then_some(start..end);
                }
                byte if byte.is_ascii_whitespace() => {
                    end.get_or_insert(self.at);
                    self.at += 1;
                }
                b'"' | b'\'' | b'(' => break,
                _ if end.is_some() => break,
                b'\\' => self.at += 2,
                _ => self.at += 1,
            }
        }
        // The rest of a broken `url(`, to its bracket.
        while let Some(byte) = self.peek(0) {
            self.at += if byte == b'\\' { 2 } else { 1 };
            if byte == b')' {
                break;
            }
        }
        self.at = self.at.min(self.css.len());
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn written(css: &str) -> Vec<&str> {
        references(css.as_bytes())
            .into_iter()
            .map(|range| &css[range])
            .collect()
    }

    #[test]
    fn every_spelling_of_an_address_is_found() {
        let css = "@import 'a.css';\n@import \"b.css\" screen;\n@import url(c.css);\n\
                   @IMPORT URL( \"d.css\" ) layer(base);\n\
                   @font-face { src: url(fonts/e.woff2) format('woff2'), url('f.woff'); }\n\
                   .g { background: url( g.png ); }\n";
        assert_eq!(
            written(css),
            [
                "a.css",
                "b.css",
                "c.css",
                "d.css",
                "fonts/e.woff2",
                "f.woff",
                "g.png"
            ]
        );
    }

    #[test]
    fn nothing_in_a_comment_or_a_string_is_an_address() {
        let css = "/* @import 'a.css'; url(b.png) */\n\
                   .x::before { content: \"url(c.png) @import 'd.css'\"; }\n\
                   .y { font-family: 'e.css'; }\n\
                   .z { background: url(bad'f.png); }\n\
                   @importance 'g.css';\n\
                   @import 'h.css';\n";
        assert_eq!(written(css), ["h.css"]);
    }

    #[test]
    fn only_the_addresses_are_replaced() {
        let css = "@import 'a.css';\nbody { background: url(b.png); }\n";
        let found = references(css.as_bytes());
        let replaced: Vec<_> = found
            .into_iter()
            .zip(["a.11111111.css", "b.22222222.png"])
            .map(|(range, with)| (range, with.to_owned()))
            .collect();
        assert_eq!(
            String::from_utf8(rewritten(css.as_bytes(), &replaced)).unwrap(),
            "@import 'a.11111111.css';\nbody { background: url(b.22222222.png); }\n"
        );
    }
}
