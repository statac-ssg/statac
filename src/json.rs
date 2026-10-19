//! Writing a JSON string.

use std::fmt::Write as _;

/// `text` as a JSON string written to `out`, safe wherever it lands in a
/// page: inside a `<script>` element, where `<` could close it or open a
/// comment, and inside an attribute in either quote. U+2028 and U+2029 are
/// written out too, for scripts that read them as line ends.
pub(crate) fn write_string(text: &str, out: &mut String) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '<' | '>' | '&' | '\'' | '\u{2028}' | '\u{2029}' => {
                write!(out, "\\u{:04x}", c as u32).expect("writing to a string can't fail")
            }
            c if c.is_control() => {
                write!(out, "\\u{:04x}", c as u32).expect("writing to a string can't fail")
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// [`write_string`], as a new string.
pub(crate) fn string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    write_string(text, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_string_is_quoted_with_its_quotes_and_backslashes_escaped() {
        assert_eq!(string("a\"b\\c"), "\"a\\\"b\\\\c\"");
    }

    #[test]
    fn a_string_cant_close_a_script() {
        assert_eq!(
            string("a</script>\"\\"),
            "\"a\\u003c/script\\u003e\\\"\\\\\""
        );
    }

    #[test]
    fn a_string_is_safe_inside_an_attribute_in_either_quote() {
        assert_eq!(string("a'b\"c&d"), "\"a\\u0027b\\\"c\\u0026d\"");
    }

    #[test]
    fn a_string_writes_out_the_line_separators() {
        assert_eq!(string("a\u{2028}b\u{2029}c"), "\"a\\u2028b\\u2029c\"");
    }

    #[test]
    fn a_string_writes_out_control_characters() {
        assert_eq!(
            string("a\nb\tc\u{7f}d\u{85}"),
            "\"a\\nb\\u0009c\\u007fd\\u0085\""
        );
    }

    #[test]
    fn a_string_keeps_other_characters() {
        assert_eq!(string("caf\u{e9} \u{1f600}"), "\"caf\u{e9} \u{1f600}\"");
    }

    #[test]
    fn writing_adds_to_what_is_there() {
        let mut out = String::from("[");
        write_string("a", &mut out);
        assert_eq!(out, "[\"a\"");
    }
}
