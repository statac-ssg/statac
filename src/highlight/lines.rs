//! Code shown a line at a time, where a block asks for its lines numbered
//! or some of them marked. Each line is wrapped in `<span class="hl-line">`,
//! a marked one carrying `hl-marked` too, and a numbered one carries its
//! number in `data-line`, which the site's stylesheet shows: the number is
//! no text of the page, so copying the code copies no numbers.

use std::fmt::Write as _;

/// The class round each line of a block that asks for its lines.
pub(crate) const LINE: &str = "hl-line";

/// The class a marked line carries besides [`LINE`].
pub(crate) const MARKED: &str = "hl-marked";

/// Lines by number as written, `3-5,8`: each a first and a last line, in
/// the order written; none where it isn't one or more numbers from 1 or
/// runs of them, each run ending no earlier than it starts. Spaces may
/// stand round each number.
pub(crate) fn read(written: &str) -> Option<Vec<(u64, u64)>> {
    let number = |text: &str| {
        let text = text.trim();
        let digits = !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit());
        digits
            .then(|| text.parse::<u64>().ok())
            .flatten()
            .filter(|&n| n > 0)
    };
    written
        .split(',')
        .map(|run| match run.split_once('-') {
            Some((first, last)) => {
                let (first, last) = (number(first)?, number(last)?);
                (first <= last).then_some((first, last))
            }
            None => number(run).map(|n| (n, n)),
        })
        .collect()
}

/// How many lines `code` holds, as a block shows it: an empty block holds
/// one, empty.
pub(crate) fn count(code: &str) -> u64 {
    code.split('\n').count() as u64
}

/// The first number of `marked` that lies outside `first..=last`, in the
/// order written.
pub(crate) fn outside(marked: &[(u64, u64)], first: u64, last: u64) -> Option<u64> {
    marked
        .iter()
        .find_map(|&(from, to)| [from, to].into_iter().find(|&n| n < first || n > last))
}

/// The inside of a `<code>` holding `html`, the HTML of a block's code
/// whose first line is numbered `first`, a line at a time: numbered where
/// `numbered`, and marked where `marked` holds its number.
pub(crate) fn wrapped(html: &str, first: u64, numbered: bool, marked: &[(u64, u64)]) -> String {
    let mut out = String::with_capacity(html.len() + html.len() / 2);
    for (at, line) in html.split('\n').enumerate() {
        let number = first + at as u64;
        if at > 0 {
            out.push('\n');
        }
        out.push_str("<span class=\"");
        out.push_str(LINE);
        if marked
            .iter()
            .any(|&(from, to)| (from..=to).contains(&number))
        {
            out.push(' ');
            out.push_str(MARKED);
        }
        out.push('"');
        if numbered {
            write!(out, " data-line=\"{number}\"").unwrap();
        }
        out.push('>');
        out.push_str(&without_return(line));
        out.push_str("</span>");
    }
    out
}

/// A line of HTML without the carriage return its code's line ends with,
/// which may stand inside the classes closing at its end. A page reads a
/// carriage return before a line feed as no more than the line feed, but
/// alone, inside the line's own element, as a second line.
pub(crate) fn without_return(line: &str) -> std::borrow::Cow<'_, str> {
    let mut end = line.len();
    while line[..end].ends_with("</span>") {
        end -= "</span>".len();
    }
    match line[..end].ends_with('\r') {
        true => format!("{}{}", &line[..end - 1], &line[end..]).into(),
        false => line.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_are_read_as_numbers_and_runs_of_them() {
        assert_eq!(read("3"), Some(vec![(3, 3)]));
        assert_eq!(read("3-5,8"), Some(vec![(3, 5), (8, 8)]));
        assert_eq!(read("3 - 5, 8"), Some(vec![(3, 5), (8, 8)]));
        assert_eq!(read("8,3"), Some(vec![(8, 8), (3, 3)]));
        for refused in ["", "0", "5-3", "3-", "-3", "3,,5", "x", "+3", "3.5", "3 5"] {
            assert_eq!(read(refused), None, "{refused}");
        }
    }

    #[test]
    fn the_first_number_outside_the_lines_shown_is_found_in_the_order_written() {
        assert_eq!(outside(&[(3, 5), (8, 8)], 1, 12), None);
        assert_eq!(outside(&[(10, 15), (20, 20)], 1, 12), Some(15));
        assert_eq!(outside(&[(13, 15)], 1, 12), Some(13));
        assert_eq!(outside(&[(5, 5), (12, 12)], 10, 20), Some(5));
    }

    #[test]
    fn each_line_is_wrapped_numbered_and_marked_as_asked() {
        let html = "<span class=\"hl-keyword\">fn</span> a() {\n    b();\n}";
        assert_eq!(
            wrapped(html, 1, false, &[(2, 2)]),
            "<span class=\"hl-line\"><span class=\"hl-keyword\">fn</span> a() {</span>\n\
             <span class=\"hl-line hl-marked\">    b();</span>\n\
             <span class=\"hl-line\">}</span>"
        );
        assert_eq!(
            wrapped("a\nb", 10, true, &[]),
            "<span class=\"hl-line\" data-line=\"10\">a</span>\n\
             <span class=\"hl-line\" data-line=\"11\">b</span>"
        );
        assert_eq!(
            wrapped("", 1, true, &[]),
            "<span class=\"hl-line\" data-line=\"1\"></span>"
        );
    }

    #[test]
    fn a_lines_carriage_return_is_left_out_wherever_its_classes_close() {
        assert_eq!(
            wrapped(
                "a\r\n<span class=\"hl-comment\">// b\r</span>\nc",
                1,
                false,
                &[]
            ),
            "<span class=\"hl-line\">a</span>\n\
             <span class=\"hl-line\"><span class=\"hl-comment\">// b</span></span>\n\
             <span class=\"hl-line\">c</span>"
        );
    }
}
