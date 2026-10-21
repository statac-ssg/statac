//! Front matter other tools read and Statac's reader stops at, changed so
//! Statac reads the same fields: a text in quotes whose later lines are
//! indented less than Statac's reader wants, and a control character.
//!
//! Nothing here reads YAML for itself. Statac's reader says where it
//! stopped and at what; the one thing it named is changed; and the change
//! stands only where the reader then reads the whole, and reads every
//! field but the one changed exactly as it reads the front matter with a
//! placeholder where the change was made. Anything else leaves the front
//! matter as it was written, for the reader to refuse.

use std::collections::HashSet;

use crate::origin::Style;
use crate::problem::Kind as ProblemKind;
use crate::problem::Malformed;
use crate::source::{Sources, Span};
use crate::yaml::{self, Node, Value};

/// The longest front matter that is mended, in bytes: many times what a
/// page's fields come to, and small enough that reading it again each
/// round costs little. A longer one is left for the reader to refuse.
const LONGEST: usize = 64 * 1024;

/// How many times the reader is asked again for one front matter: once
/// for each text given deeper lines and each control character taken
/// out, which is more than a page's front matter has of either.
///
/// A mend adds no more spaces than the front matter is long, so the most
/// work for one page is `TRIES` readings of `2 * LONGEST` bytes, each
/// with a few walks of that length beside it: some 8 MB read.
const TRIES: usize = 64;

/// The characters the placeholders may be made of, for what was changed:
/// private use characters, which the reader takes as any letter. There
/// are more than `LONGEST` of them, so one is missing from any front
/// matter that is mended.
fn marks() -> impl Iterator<Item = char> {
    ('\u{e000}'..='\u{f8ff}').chain('\u{f0000}'..='\u{ffffd}')
}

/// A character that the front matter does not hold.
fn mark_for(front: &str) -> Option<char> {
    let held: HashSet<char> = front.chars().collect();
    marks().find(|mark| !held.contains(mark))
}

/// A file's text with its front matter mended, and what was done.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) struct Mended {
    pub text: String,
    /// The control characters taken out.
    pub controls: usize,
}

/// `text` with its front matter mended where that is safe, and else as
/// it is. Each line stays the line it was. A control character is taken
/// out only where `controls` is true.
pub(super) fn mended(text: &str, controls: bool) -> Mended {
    let unchanged = || Mended {
        text: text.to_owned(),
        controls: 0,
    };
    let Ok(split) = yaml::split(text) else {
        return unchanged();
    };
    let Some(span) = split.front_matter else {
        return unchanged();
    };
    let front = &text[span.range()];
    match mend(front, controls) {
        Some((mended, taken)) => Mended {
            text: format!(
                "{}{mended}{}",
                &text[..span.range().start],
                &text[span.range().end..]
            ),
            controls: taken,
        },
        None => unchanged(),
    }
}

/// A text in quotes that was given deeper lines: where its quotes stand
/// in the front matter as written.
struct Quoted {
    opens: usize,
    closes: usize,
}

/// The front matter mended, and how many control characters were taken
/// out; none where the reader reads it as it is, or where no change is
/// sure to keep every other field.
fn mend(front: &str, controls: bool) -> Option<(String, usize)> {
    if front.len() > LONGEST || read(front).is_ok() {
        return None;
    }
    let mark = mark_for(front)?;
    // The spaces a mend may still put in: in all, no more than the front
    // matter's own length.
    let mut room = front.len();
    // Each character with where it stood as written, a space put in
    // having no such place.
    let mut now: Vec<(Option<usize>, char)> =
        front.char_indices().map(|(at, c)| (Some(at), c)).collect();
    let mut quoted: Vec<Quoted> = Vec::new();
    let mut taken: Vec<usize> = Vec::new();
    let mut read_now = None;
    for _ in 0..TRIES {
        let text: String = now.iter().map(|(_, c)| c).collect();
        let (found, at) = match read(&text) {
            Ok(node) => {
                read_now = Some((text, node));
                break;
            }
            Err(stopped) => stopped?,
        };
        // A text in quotes is deepened from its opening quote, wherever
        // the reader stopped in it.
        let (found, at) = match found {
            Malformed::ShallowLine { quote, back, .. } => {
                (Malformed::UnclosedQuote(quote), at.checked_sub(back)?)
            }
            found => (found, at),
        };
        let index = index_at(&now, at)?;
        match found {
            Malformed::UnclosedQuote(quote) if now[index].1 == quote => {
                let opens = now[index].0?;
                if quoted.iter().any(|known| known.opens == opens) {
                    return None;
                }
                let end = closing(&now, index, quote)?;
                let closes = now[end].0?;
                now = deepened(&now, index, end, &mut room)?;
                quoted.push(Quoted { opens, closes });
            }
            Malformed::ControlCharacter(c)
                if controls
                    && now[index].1 == c
                    && c.is_control()
                    && !matches!(c, '\t' | '\n' | '\r' | '\u{85}') =>
            {
                taken.push(now[index].0?);
                now.remove(index);
            }
            _ => return None,
        }
    }
    let (text, node) = read_now?;
    // The front matter as written, with a placeholder for each text given
    // deeper lines and a mark for each character taken out.
    let mut shadow = String::with_capacity(front.len());
    for (at, c) in front.char_indices() {
        match quoted
            .iter()
            .position(|q| (q.opens..=q.closes).contains(&at))
        {
            Some(n) if quoted[n].opens == at => shadow.push_str(&placeholder(mark, n)),
            Some(_) => {}
            None if taken.contains(&at) => shadow.push(mark),
            None => shadow.push(c),
        }
    }
    let shadow_node = read(&shadow).ok()?;
    let mut held: Vec<(usize, Span)> = Vec::new();
    if !same(mark, &shadow_node, &node, false, &mut held) || held.len() != quoted.len() {
        return None;
    }
    // The reader must have read each text from its own quote to its own.
    for (n, text_quoted) in quoted.iter().enumerate() {
        let place_of = |written: usize| {
            let index = now.iter().position(|(at, _)| *at == Some(written))?;
            Some(
                now[..index]
                    .iter()
                    .map(|(_, c)| c.len_utf8())
                    .sum::<usize>(),
            )
        };
        let (opens, closes) = (place_of(text_quoted.opens)?, place_of(text_quoted.closes)?);
        let read_at = held.iter().filter(|(held, _)| *held == n).map(|(_, at)| at);
        if read_at.clone().count() != 1 || read_at.clone().next()?.range() != (opens..closes + 1) {
            return None;
        }
    }
    Some((text, taken.len()))
}

/// What Statac's reader makes of `front`: the fields, or what it stopped
/// at and where, where that is a fault in the YAML with a place.
fn read(front: &str) -> Result<Node, Option<(Malformed, usize)>> {
    let mut sources = Sources::default();
    let source = sources.add("front matter", front.to_owned());
    yaml::read(&sources, source).map_err(|problem| match (problem.kind, problem.place) {
        (ProblemKind::MalformedYaml { found }, Some(place)) => {
            Some((found, place.span.start as usize))
        }
        _ => None,
    })
}

/// The character that starts at byte `at` of the text `now` makes.
fn index_at(now: &[(Option<usize>, char)], at: usize) -> Option<usize> {
    let mut byte = 0;
    for (index, (_, c)) in now.iter().enumerate() {
        if byte == at {
            return Some(index);
        }
        byte += c.len_utf8();
    }
    None
}

/// The quote that closes the text opened at `opens`, by YAML's rule for
/// its kind: in single quotes `''` is a quote written, and in double
/// quotes a backslash takes the next character with it.
fn closing(now: &[(Option<usize>, char)], opens: usize, quote: char) -> Option<usize> {
    let mut at = opens + 1;
    while at < now.len() {
        let c = now[at].1;
        match (quote, c) {
            ('"', '\\') => at += 1,
            ('\'', '\'') if now.get(at + 1).is_some_and(|(_, next)| *next == '\'') => at += 1,
            _ if c == quote => return Some(at),
            _ => {}
        }
        at += 1;
    }
    None
}

/// `now` with spaces before each line after the first of the text in
/// quotes from `opens` to `closes`, as far as the column its quote opens
/// at, so the reader takes the line as more of the text: YAML drops the
/// spaces a line of such a text starts with. Done only where each line
/// starts with spaces alone or with none, where a line no deeper than the
/// line the text opens on, which YAML doesn't take and the other tools'
/// readers do, [`reads_as_text`], and where the spaces come to no more
/// than `room`, which they are taken from.
fn deepened(
    now: &[(Option<usize>, char)],
    opens: usize,
    closes: usize,
    room: &mut usize,
) -> Option<Vec<(Option<usize>, char)>> {
    let first = now[..opens]
        .iter()
        .rposition(|(_, c)| *c == '\n')
        .map_or(0, |n| n + 1);
    let column = opens - first;
    // The line's own depth, with the `- ` of each list it opens.
    let depth = now[first..opens]
        .iter()
        .take_while(|(_, c)| matches!(c, ' ' | '-'))
        .count();
    // Each line to deepen, by where it starts, with the spaces it needs:
    // counted before any is put in.
    let mut wanted: Vec<(usize, usize)> = Vec::new();
    let mut lines = 0;
    let mut spaces_wanted = 0usize;
    for start in (opens..closes)
        .filter(|at| now[*at].1 == '\n')
        .map(|at| at + 1)
    {
        lines += 1;
        let spaces = now[start..].iter().take_while(|(_, c)| *c == ' ').count();
        match now.get(start + spaces).map(|(_, c)| *c) {
            // An empty line is a line break in the text, and stays one.
            Some('\n' | '\r') | None => continue,
            Some('\t') => return None,
            Some(_) => {}
        }
        if spaces <= depth {
            let line = now[start + spaces..].iter().map(|(_, c)| *c);
            let line: String = line.take_while(|c| !matches!(c, '\n' | '\r')).collect();
            if !yaml::reads_as_text(&line, spaces) {
                return None;
            }
        }
        if spaces < column {
            wanted.push((start, column - spaces));
            spaces_wanted = spaces_wanted.checked_add(column - spaces)?;
            if spaces_wanted > *room {
                return None;
            }
        }
    }
    if lines == 0 {
        return None;
    }
    *room -= spaces_wanted;
    let mut out = Vec::with_capacity(now.len() + spaces_wanted);
    let mut wanted = wanted.into_iter().peekable();
    for (index, written) in now.iter().enumerate() {
        if let Some((_, spaces)) = wanted.next_if(|(start, _)| *start == index) {
            out.extend(std::iter::repeat_n((None, ' '), spaces));
        }
        out.push(*written);
    }
    Some(out)
}

fn placeholder(mark: char, n: usize) -> String {
    format!("\"{mark}{n}{mark}\"")
}

/// Whether the reader made of the mended front matter, `now`, what it
/// made of the shadow, but for each placeholder, which stands for a text
/// in quotes and is noted in `held` with where the reader read it, and
/// each mark, which stands for a character taken out. A name is never a
/// placeholder.
fn same(mark: char, shadow: &Node, now: &Node, name: bool, held: &mut Vec<(usize, Span)>) -> bool {
    match (&shadow.value, &now.value) {
        (Value::Scalar(was), Value::Scalar(is)) => {
            let stands_for = was
                .strip_prefix(mark)
                .and_then(|rest| rest.strip_suffix(mark))
                .and_then(|n| n.parse::<usize>().ok());
            match stands_for {
                Some(n) => {
                    held.push((n, now.place.span));
                    !name
                        && shadow.style == Style::DoubleQuoted
                        && shadow.tag == now.tag
                        && matches!(now.style, Style::SingleQuoted | Style::DoubleQuoted)
                }
                // A name is what it was, to the letter.
                None if name => shadow.style == now.style && shadow.tag == now.tag && was == is,
                None => {
                    shadow.style == now.style
                        && shadow.tag == now.tag
                        && was.chars().filter(|c| *c != mark).eq(is.chars())
                }
            }
        }
        (Value::Sequence(was), Value::Sequence(is)) => {
            was.len() == is.len()
                && shadow.tag == now.tag
                && was
                    .iter()
                    .zip(is)
                    .all(|(was, is)| same(mark, was, is, false, held))
        }
        (Value::Mapping(was), Value::Mapping(is)) => {
            was.len() == is.len()
                && shadow.tag == now.tag
                && was.iter().zip(is).all(|(was, is)| {
                    same(mark, &was.key, &is.key, true, held)
                        && same(mark, &was.value, &is.value, false, held)
                })
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import::yaml::Value as Read;
    use crate::import::yaml::read_back;

    /// The fields Statac's reader makes of the file's front matter once
    /// mended, each name with its value; none where it can't read it.
    fn fields(text: &str, controls: bool) -> Option<Vec<(String, Read)>> {
        let mended = mended(text, controls).text;
        assert_eq!(mended.lines().count(), text.lines().count(), "{text}");
        let span = yaml::split(&mended).ok()?.front_matter?;
        read_back(&mended[span.range()])
    }

    fn text(value: &str) -> Read {
        Read::Text(value.to_owned())
    }

    fn named(name: &str, value: Read) -> (String, Read) {
        (name.to_owned(), value)
    }

    #[test]
    fn a_text_in_quotes_with_shallow_lines_is_read_as_yaml_reads_it() {
        // Each as written, where Statac's reader takes a line one space
        // deep unaided, and with its later lines no deeper than the field.
        let cases = [
            ("summary: 'One\n two'\n", "One two"),
            ("summary: \"One\n two\"\n", "One two"),
            ("summary: 'One\n\n two\n three'\n", "One\ntwo three"),
            (
                "summary: 'It''s one\n and it''s two'\n",
                "It's one and it's two",
            ),
            ("summary: \"One \\\"\n two\"\n", "One \" two"),
            ("summary: \"One\\\\\n two\"\n", "One\\ two"),
            ("summary: \"One\\\n two\"\n", "Onetwo"),
            ("summary: 'One   \n two'   # said\n", "One two"),
            // A later line no deeper than the field, which the readers
            // of the other tools take.
            ("summary: 'One\ntwo'\n", "One two"),
            ("summary: \"One\ntwo\n three\"\n", "One two three"),
            (
                "summary: \"One\n\ntwo and: three\"\n",
                "One\ntwo and: three",
            ),
        ];
        let shallower = cases.map(|(front, read)| (front.replace("\n ", "\n"), read));
        let cases = cases.map(|(front, read)| (front.to_owned(), read));
        for (front, read) in cases.into_iter().chain(shallower) {
            let file = format!(
                "---\ntitle: Post 1\n{front}draft: true\npermalink: false\n---\nA 'body\n left' alone.\n"
            );
            assert!(
                mended(&file, false)
                    .text
                    .ends_with("---\nA 'body\n left' alone.\n")
            );
            assert_eq!(
                fields(&file, false),
                Some(vec![
                    named("title", text("Post 1")),
                    named("summary", text(read)),
                    named("draft", Read::Truth(true)),
                    named("permalink", Read::Truth(false)),
                ]),
                "{front}"
            );
        }
    }

    #[test]
    fn a_text_in_a_list_or_under_a_field_takes_lines_no_deeper_than_its_own() {
        for front in [
            "list:\n  - summary: 'One\n  two'\n",
            "list:\n  - summary: 'One\ntwo'\n",
            "book:\n  summary: \"One\n  two\"\n",
            "book:\n  summary: \"One\n two\"\n",
        ] {
            let file = format!("---\n{front}draft: true\n---\nBody.\n");
            let read = fields(&file, false).expect(front);
            let summary = named("summary", text("One two"));
            let held = match &read[0].1 {
                Read::List(items) => items[0].clone(),
                held => held.clone(),
            };
            assert_eq!(held, Read::Fields(vec![summary]), "{front}");
            assert_eq!(read[1], named("draft", Read::Truth(true)), "{front}");
        }
    }

    #[test]
    fn a_front_matter_too_long_or_a_mend_too_large_changes_nothing() {
        let mendable = |filler: &str| format!("---\n{filler}summary: 'One\ntwo'\n---\nBody.\n");
        let under = mendable(&format!("a: {}\n", "x".repeat(LONGEST - 100)));
        assert_ne!(mended(&under, false).text, under);
        let over = mendable(&format!("a: {}\n", "x".repeat(LONGEST)));
        assert_eq!(mended(&over, false).text, over);
        // Each later line would take 43 spaces, and there are more such
        // lines than a fortieth of the front matter's length.
        let lines = "bbbb\n".repeat(200);
        let wide = format!("---\n{}: 'One\n{lines}two'\n---\n", "k".repeat(40));
        assert_eq!(mended(&wide, false).text, wide);
        let narrow = format!("---\nk: 'One\n{lines}two'\n---\n");
        assert_ne!(mended(&narrow, false).text, narrow);
    }

    #[test]
    fn a_text_shaped_to_grow_is_left_alone_quickly() {
        // A quote opening at column 60,000 with thousands of lines of one
        // character inside it: under the longest front matter mended, and
        // far over what a mend may add.
        let file = format!(
            "---\n{}: 'a\n{} b'\n---\n",
            "k".repeat(60_000),
            " b\n".repeat(1_000)
        );
        assert!(file.len() < LONGEST);
        let started = std::time::Instant::now();
        assert_eq!(mended(&file, true).text, file);
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
        // Control characters past the rounds allowed are left, all of
        // them.
        let many = format!("---\ntitle: a{}b\n---\n", "\u{1}".repeat(TRIES + 1));
        assert_eq!(mended(&many, true).text, many);
    }

    #[test]
    fn two_such_texts_are_each_read() {
        let file = "---\na: 'One\n two'\nb: \"Three\n four\"\ndraft: true\n---\n";
        assert_eq!(
            fields(file, false),
            Some(vec![
                named("a", text("One two")),
                named("b", text("Three four")),
                named("draft", Read::Truth(true)),
            ])
        );
    }

    #[test]
    fn a_quote_that_opens_no_text_changes_nothing() {
        // Each reads as written, the quote being no text's opening, and a
        // field after it stays a field.
        for (front, name, read) in [
            (
                "summary: |\n  A \"block\n  of two lines\n",
                "summary",
                "A \"block\nof two lines\n",
            ),
            (
                "summary: >\n  A \"block\n  of two lines\n",
                "summary",
                "A \"block of two lines\n",
            ),
            ("summary: One  # a \"comment\n", "summary", "One"),
            ("summary: 5\" disk\n", "summary", "5\" disk"),
            (
                "summary: 'A \" in single\n  quotes'\n",
                "summary",
                "A \" in single quotes",
            ),
            ("summary: [\"One\", \"Two\n three\"]\n", "summary", ""),
        ] {
            let file = format!("---\n{front}draft: true\npermalink: false\n---\n");
            assert_eq!(mended(&file, true).text, file, "{front}");
            let read_fields = fields(&file, true).expect(front);
            assert_eq!(read_fields.len(), 3, "{front}");
            assert_eq!(read_fields[0].0, name);
            match &read_fields[0].1 {
                Read::List(items) => assert_eq!(*items, [text("One"), text("Two three")]),
                value => assert_eq!(*value, text(read), "{front}"),
            }
            assert_eq!(read_fields[1], named("draft", Read::Truth(true)), "{front}");
            assert_eq!(read_fields[2], named("permalink", Read::Truth(false)));
        }
    }

    #[test]
    fn what_cant_be_mended_surely_is_left_for_the_reader_to_refuse() {
        for front in [
            // Never closed: the next field isn't taken into the text.
            "summary: 'One\n two\ndraft: true\n",
            "summary: \"One\ndraft: true\n",
            // A line no deeper than the one the text opens on that reads
            // as a field, or as an item of a list, or ends a document.
            "summary: 'One\ndraft: true\nother: two'\n",
            "summary: 'One\nother:\n two'\n",
            "summary: 'One\n'other': two'\n",
            "list:\n  - summary: 'One\n  - two'\n",
            "summary: 'One\n... two'\n",
            // A name in quotes over two lines.
            "'One\n two': three\ndraft: true\n",
            // A tab before a later line.
            "summary: 'One\n\ttwo'\n",
            // Closed on its first line, after a backslash written twice.
            "summary: \"One\\\\\"\n two\"\ndraft: true\n",
            // Text after the quote that closes it.
            "summary: 'One\n two' three\ndraft: true\n",
        ] {
            let file = format!("---\ntitle: Post 1\n{front}---\nBody.\n");
            assert_eq!(mended(&file, true).text, file, "{front}");
            assert_eq!(fields(&file, true), None, "{front}");
        }
    }

    #[test]
    fn a_control_character_is_taken_out_where_no_field_moves() {
        let file = "---\ntitle: \"Post\u{92} 1\u{0}\"\nsummary: One\u{1} two\ndraft: true\n---\nA body\u{92} left.\n";
        let read = mended(file, true);
        assert_eq!(read.controls, 3);
        assert_eq!(
            read.text,
            "---\ntitle: \"Post 1\"\nsummary: One two\ndraft: true\n---\nA body\u{92} left.\n"
        );
        assert_eq!(mended(file, false).text, file);
        // One that YAML counts as a line end, one whose going would make
        // a field of a text, and one in a name are left.
        for front in [
            "title: Post 1\u{85}draft: true\n",
            "title: Post 1\u{85}\ndraft: true\n",
            "title: One:\u{92} two\ndraft: true\n",
            "title:\u{0} One\ndraft: true\n",
            "ti\u{92}tle: One\ndraft: true\n",
            "title: One \u{92}\ndraft: true\n",
        ] {
            let file = format!("---\n{front}---\nBody.\n");
            assert_eq!(mended(&file, true).text, file, "{front:?}");
        }
    }

    #[test]
    fn a_control_character_in_a_text_given_deeper_lines_is_taken_out_too() {
        let file = "---\nsummary: 'One\u{92}\n two'\ndraft: true\n---\n";
        assert_eq!(
            fields(file, true),
            Some(vec![
                named("summary", text("One two")),
                named("draft", Read::Truth(true)),
            ])
        );
    }

    #[test]
    fn a_front_matter_holding_private_use_characters_is_mended_as_any_other() {
        // The characters a placeholder may be made of, in the field
        // mended, in another and in a name.
        for held in ["\u{e000}", "\u{e001}", "\u{e000}\u{e001}\u{f8ff}"] {
            let file = format!(
                "---\ntitle: Post 1\nicon: \"{held}\"\n{held}: {held}\nsummary: 'One {held}\ntwo'\ndraft: true\n---\n"
            );
            assert_eq!(
                fields(&file, false),
                Some(vec![
                    named("title", text("Post 1")),
                    named("icon", text(held)),
                    named(held, text(held)),
                    named("summary", text(&format!("One {held} two"))),
                    named("draft", Read::Truth(true)),
                ]),
                "{held:?}"
            );
            let file = format!("---\nsummary: 'One{held}\u{92}\ntwo'\nicon: {held}\n---\n");
            assert_eq!(
                fields(&file, true),
                Some(vec![
                    named("summary", text(&format!("One{held} two"))),
                    named("icon", text(held)),
                ]),
                "{held:?}"
            );
        }
    }

    #[test]
    fn a_front_matter_holding_every_private_use_character_of_the_first_plane_is_mended() {
        let held: String = ('\u{e000}'..='\u{f8ff}').collect();
        let file = format!("---\nicons: {held}\nsummary: 'One\ntwo'\n---\n");
        assert_eq!(
            fields(&file, false),
            Some(vec![
                named("icons", text(&held)),
                named("summary", text("One two")),
            ])
        );
    }
}
