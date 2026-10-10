//! What a stylesheet's rules for `img` say of a picture's size. A picture
//! published with its width and height keeps that height where a rule
//! narrows it and none gives its height back to the browser, so it is drawn
//! out of proportion.
//!
//! A stylesheet is read far enough to find its rules and their
//! declarations, Sass in its source as much as CSS: a rule inside another,
//! or inside `@media`, is a rule of its own.

use std::ops::Range;

/// The properties that narrow a picture.
const NARROWS: [&str; 4] = ["max-width", "width", "max-inline-size", "inline-size"];

/// The properties that leave a picture's height to the browser, or to the
/// author.
const HEIGHTS: [&str; 3] = ["height", "block-size", "aspect-ratio"];

/// Values of a narrowing property that give no size.
const SIZELESS: [&str; 5] = ["auto", "none", "initial", "unset", "revert"];

/// What one stylesheet's rules for `img` say.
#[derive(Default)]
pub(super) struct Said {
    /// Where the first property narrowing a picture is named, in a rule
    /// that picks out `img` itself.
    pub(super) narrows: Option<Range<usize>>,
    /// Whether any rule that could be for `img` says what its height is.
    pub(super) heights: bool,
}

/// What the rules of `css` say, read as CSS or as Sass written with
/// braces.
pub(super) fn said(css: &str) -> Said {
    let mut said = Said::default();
    let mut read = Reader {
        css: css.as_bytes(),
        text: css,
        at: 0,
    };
    // A `}` that closes nothing ends no stylesheet.
    while read.at < css.len() {
        read.block(None, &mut said, 0);
    }
    said
}

/// How deep rules are read inside one another.
const DEEPEST: usize = 64;

/// What the rule being read is to `img`.
#[derive(Copy, Clone)]
struct Rule {
    /// Whether one of its selectors ends at `img` itself.
    picks: bool,
    /// Whether one of its selectors ends at something naming `img` at all,
    /// as `:is(img, video)` does.
    names: bool,
}

struct Reader<'a> {
    css: &'a [u8],
    text: &'a str,
    at: usize,
}

impl Reader<'_> {
    /// Reads the declarations and rules of a block to its `}`, or of the
    /// whole stylesheet. `rule` is the rule the declarations belong to.
    fn block(&mut self, rule: Option<Rule>, said: &mut Said, depth: usize) {
        let mut start = self.at;
        let mut brackets = 0usize;
        while let Some(&byte) = self.css.get(self.at) {
            match byte {
                b'/' if self.css.get(self.at + 1) == Some(&b'*') => {
                    let from = self.at;
                    self.at = self.text[from + 2..]
                        .find("*/")
                        .map_or(self.css.len(), |end| from + 2 + end + 2);
                    // A comment between declarations is no part of the next.
                    if self.text[start..from].trim().is_empty() {
                        start = self.at;
                    }
                    continue;
                }
                // A Sass comment, which CSS has no use for outside brackets.
                b'/' if brackets == 0 && self.css.get(self.at + 1) == Some(&b'/') => {
                    let from = self.at;
                    self.at = self.text[from..]
                        .find('\n')
                        .map_or(self.css.len(), |end| from + end);
                    if self.text[start..from].trim().is_empty() {
                        start = self.at;
                    }
                    continue;
                }
                b'"' | b'\'' => {
                    self.string(byte);
                    continue;
                }
                b'\\' => self.at += 1,
                // Sass writes a value into a name or a selector in these.
                b'#' if self.css.get(self.at + 1) == Some(&b'{') => {
                    self.at += 2;
                    self.interpolated();
                    continue;
                }
                b'(' | b'[' => brackets += 1,
                b')' | b']' => brackets = brackets.saturating_sub(1),
                b';' if brackets == 0 => {
                    declared(self.text, start..self.at, rule, said);
                    start = self.at + 1;
                }
                b'{' if brackets == 0 => {
                    let prelude = self.text[start..self.at].trim();
                    self.at += 1;
                    // What `@media` and its like hold belongs to the rule
                    // they are written in.
                    let inner = match prelude.starts_with('@') {
                        true => rule,
                        false => Some(Rule {
                            picks: last_pieces(prelude).any(picks_img),
                            names: last_pieces(prelude).any(names_img),
                        }),
                    };
                    // Each block deeper is a call deeper: past any nesting
                    // a stylesheet is written with, the rest goes unread.
                    if depth == DEEPEST {
                        self.at = self.css.len();
                        return;
                    }
                    self.block(inner, said, depth + 1);
                    start = self.at;
                    continue;
                }
                b'}' if brackets == 0 => {
                    declared(self.text, start..self.at, rule, said);
                    self.at += 1;
                    return;
                }
                _ => {}
            }
            self.at += 1;
        }
        self.at = self.css.len();
        if brackets == 0 {
            declared(self.text, start..self.at, rule, said);
        }
    }

    /// Past a string opened by `quote`, which ends with its line where it
    /// never closes.
    fn string(&mut self, quote: u8) {
        self.at += 1;
        while let Some(&byte) = self.css.get(self.at) {
            self.at += 1;
            match byte {
                b'\\' => self.at += 1,
                b'\n' => return,
                byte if byte == quote => return,
                _ => {}
            }
        }
        self.at = self.css.len();
    }

    /// Past what follows `#{`, to its `}`.
    fn interpolated(&mut self) {
        let mut depth = 1usize;
        while let Some(&byte) = self.css.get(self.at) {
            match byte {
                b'"' | b'\'' => {
                    self.string(byte);
                    continue;
                }
                b'{' => depth += 1,
                b'}' => depth -= 1,
                _ => {}
            }
            self.at += 1;
            if depth == 0 {
                return;
            }
        }
    }
}

/// Takes the declaration written at `at` into what `rule` says.
fn declared(css: &str, at: Range<usize>, rule: Option<Rule>, said: &mut Said) {
    let Some(rule) = rule.filter(|rule| rule.names) else {
        return;
    };
    let written = &css[at.clone()];
    let Some((name, value)) = written.split_once(':') else {
        return;
    };
    let start = at.start + (name.len() - name.trim_start().len());
    let name = name.trim();
    let is = |properties: &[&str]| properties.iter().any(|p| name.eq_ignore_ascii_case(p));
    if is(&HEIGHTS) {
        said.heights = true;
    }
    if rule.picks && said.narrows.is_none() && is(&NARROWS) {
        let value = value.trim();
        let value = match value.rsplit_once('!') {
            Some((value, flag)) if flag.trim().eq_ignore_ascii_case("important") => value.trim(),
            _ => value,
        };
        let sized = !value.is_empty() && !SIZELESS.iter().any(|s| value.eq_ignore_ascii_case(s));
        if sized {
            said.narrows = Some(start..start + name.len());
        }
    }
}

/// The last piece of each selector in a list, the piece that says what the
/// rule is for: `img:hover` of `article > img:hover`. Commas, spaces and
/// combinators inside brackets are part of a piece.
fn last_pieces(selectors: &str) -> impl Iterator<Item = &str> {
    let mut pieces = Vec::new();
    let mut brackets = 0usize;
    let mut start = 0;
    let mut last = None;
    for (at, c) in selectors.char_indices() {
        match c {
            '(' | '[' => brackets += 1,
            ')' | ']' => brackets = brackets.saturating_sub(1),
            _ => {}
        }
        let ends = brackets == 0 && (c.is_whitespace() || matches!(c, '>' | '+' | '~' | ','));
        if !ends {
            continue;
        }
        if start < at {
            last = Some(&selectors[start..at]);
        }
        start = at + c.len_utf8();
        if c == ',' {
            pieces.extend(last.take());
        }
    }
    if start < selectors.len() {
        last = Some(&selectors[start..]);
    }
    pieces.extend(last);
    pieces.into_iter()
}

fn in_a_name(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '-' | '_')
}

/// Whether a selector's piece is `img` itself, with or without a class or
/// a state: `img`, `img.wide`, `img:hover`. What a piece draws before or
/// after a picture is not the picture.
fn picks_img(piece: &str) -> bool {
    element(piece).eq_ignore_ascii_case("img") && !piece.contains("::")
}

/// Whether a selector's piece could be for `img`: `img` itself and
/// whatever is drawn about it, or `img` among others, as `:is(img, video)`
/// and `:where(picture > img)` have it.
fn names_img(piece: &str) -> bool {
    let small = piece.to_ascii_lowercase();
    element(&small) == "img"
        || (small.contains(":is(") || small.contains(":where("))
            && small.split(|c| !in_a_name(c)).any(|word| word == "img")
}

/// The element a selector's piece opens with, empty where it names none.
fn element(piece: &str) -> &str {
    let end = piece.find(|c| !in_a_name(c)).unwrap_or(piece.len());
    &piece[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The property a stylesheet is found to narrow pictures with, and
    /// whether it says what their height is.
    fn read(css: &str) -> (Option<&str>, bool) {
        let said = said(css);
        (said.narrows.map(|at| &css[at]), said.heights)
    }

    #[test]
    fn a_rule_for_img_narrowing_it_is_found_at_the_property() {
        for (css, property) in [
            ("img { max-width: 100%; }", "max-width"),
            ("img{width:100%}", "width"),
            (
                "main img {\n  WIDTH: 100%;\n  max-width: 40em;\n}\n",
                "WIDTH",
            ),
            (
                ".entry img, video { max-inline-size: 100% }",
                "max-inline-size",
            ),
            (
                "article > img:hover { inline-size: 50% !important; }",
                "inline-size",
            ),
            (
                "a + img.wide { display: block; max-width: 30rem }",
                "max-width",
            ),
            ("IMG { max-width: 100% }", "max-width"),
            (
                "img { color: red; /* narrow */ max-width: 100% }",
                "max-width",
            ),
            (
                "img { background: url(data:image/png;base64,AAAA); width: 2em }",
                "width",
            ),
            ("img { content: \"}\"; width: 2em }", "width"),
        ] {
            assert_eq!(read(css), (Some(property), false), "{css}");
        }
    }

    #[test]
    fn a_rule_inside_another_or_inside_media_is_a_rule_of_its_own() {
        for (css, property) in [
            (
                "@media (min-width: 40em) { .entry img { max-width: 50%; } }",
                "max-width",
            ),
            (".entry { color: red; img { width: 100%; } }", "width"),
            (".entry { > img { width: 100%; } }", "width"),
            (
                ".entry {\n  // img { height: auto }\n  & img { width: 100% }\n}",
                "width",
            ),
            (
                "img { @media (max-width: 40em) { max-width: 100%; } }",
                "max-width",
            ),
            (
                ".a-#{$name} { color: red; }\nimg { width: $wide; }",
                "width",
            ),
            (
                "@supports (display: grid) { @media print { img { width: 100% } } }",
                "width",
            ),
        ] {
            assert_eq!(read(css), (Some(property), false), "{css}");
        }
    }

    #[test]
    fn a_rule_for_something_else_or_giving_no_size_is_passed_over() {
        for css in [
            "p { max-width: 40em }",
            ".img { max-width: 100% }",
            "#img, [img], :img { max-width: 100% }",
            "img::after { width: 1em }",
            "a:has(> img) { width: 100% }",
            "figure:not(img) { width: 100% }",
            ":is(img, video) { max-width: 100% }",
            "img figcaption { width: 100% }",
            "imgs, x-img { width: 100% }",
            "img { width: auto }",
            "img { max-width: none; width: initial; inline-size: UNSET }",
            "img { max-width: revert !important }",
            "img { border-width: 1px; min-width: 2em; }",
            "/* img { max-width: 100% } */",
            "@media (max-width: 40em) { body { margin: 0 } }",
            "@mixin wide { max-width: 100% }",
            "img { &::before { width: 1em } }",
            ".x::before { content: \"img { width: 1em }\"; }",
        ] {
            assert_eq!(read(css), (None, false), "{css}");
        }
    }

    #[test]
    fn a_height_in_any_rule_naming_img_is_found() {
        for (css, narrows) in [
            ("img { max-width: 100%; height: auto }", Some("max-width")),
            (
                "img { max-width: 100% }\n.entry img { height: 10em; }",
                Some("max-width"),
            ),
            (
                "img { inline-size: 100%; block-size: auto; }",
                Some("inline-size"),
            ),
            ("img { width: 100%; aspect-ratio: 4 / 3; }", Some("width")),
            (":where(img, video) { HEIGHT: auto }", None),
            (
                ".wide { img { height: auto; max-width: 100% } }",
                Some("max-width"),
            ),
            ("@media print { img { height: auto } }", None),
            ("img { @media print { height: auto } }", None),
        ] {
            assert_eq!(read(css), (narrows, true), "{css}");
        }
        for css in [
            "p { height: auto }",
            ".img { height: auto }",
            "img { line-height: 1; max-height: 10em }",
            "/* img { height: auto } */",
        ] {
            assert!(!said(css).heights, "{css}");
        }
    }

    #[test]
    fn rules_nested_without_end_are_read_no_deeper() {
        let nested = "a {".repeat(100_000);
        assert_eq!(read(&nested), (None, false));
    }

    #[test]
    fn a_stylesheet_cut_short_is_read_as_far_as_it_goes() {
        for css in ["img { width: 100%", "img { width: 100%; /* never closed"] {
            assert_eq!(read(css), (Some("width"), false), "{css}");
        }
        assert_eq!(read("} }\nimg { width: 100% }"), (Some("width"), false));
        for css in [
            "img { width: url(",
            "img { width: #{ \"",
            "img { width: 1em; } /*",
        ] {
            said(css);
        }
    }
}
