use super::*;
use crate::markdown::{Call, Document, Kind, Options, Placement, Snippets, Value, read, to_html};
use crate::problem::Kind as ProblemKind;
use crate::source::{Place, Sources};

struct Named;

impl Snippets for Named {
    fn html(&self, call: &Call, _: Place, _: Placement, body: Option<&str>) -> String {
        let values: Vec<String> = call
            .arguments
            .iter()
            .map(|argument| match &argument.value.value {
                Value::Text(text) => text.clone(),
                other => format!("{other:?}"),
            })
            .collect();
        format!(
            "<{} {values:?}>{}",
            call.name.value,
            body.unwrap_or_default()
        )
    }
}

fn read_text(text: &str) -> (Sources, Document, String) {
    let mut sources = Sources::default();
    let id = sources.add("content/post-1.md", text.to_owned());
    let document = read(
        &sources,
        id,
        Span::new(0, text.len()),
        &|name| Some(name == "box"),
        "",
    );
    let options = Options {
        snippets: &Named,
        back_to_the_text: None,
        digits: None,
        right_to_left: false,
        highlighter: None,
    };
    let html = to_html(&document, &options).trim_end().to_owned();
    (sources, document, html)
}

/// Each problem said, as the character and the text it is placed at.
fn said(sources: &Sources, document: &Document) -> Vec<(char, String)> {
    document
        .problems
        .iter()
        .map(|problem| match problem.kind {
            ProblemKind::ControlCharacter { character } => (
                character,
                sources.spelling(problem.place.unwrap()).to_owned(),
            ),
            _ => panic!("{problem:?}"),
        })
        .collect()
}

#[test]
fn every_control_character_is_left_out_but_the_tab_and_line_endings() {
    for c in (0..=0x1f).chain([0x7f]).chain(0x80..=0x9f) {
        let c = char::from_u32(c).unwrap();
        assert_eq!(left_out(c), !matches!(c, '\t' | '\n' | '\r'), "{c:?}");
    }
    for c in [
        ' ', 'a', '\u{a0}', '\u{ad}', '\u{200b}', '\u{2028}', '\u{feff}',
    ] {
        assert!(!left_out(c), "{c:?}");
    }
    let text = "a\u{b}b\u{92}c\td";
    let found: Vec<_> = found(text, 0, text.len()).collect();
    assert_eq!(
        found,
        [(Span::new(1, 2), '\u{b}'), (Span::new(3, 5), '\u{92}')]
    );
    assert_eq!(without(text), "abc\td");
}

#[test]
fn each_is_left_out_of_the_page_and_said_where_it_was() {
    for c in (0..=0x1f).chain([0x7f]).chain(0x80..=0x9f) {
        let c = char::from_u32(c).unwrap();
        if matches!(c, '\t' | '\n' | '\r') {
            continue;
        }
        let (sources, document, html) = read_text(&format!("a{c}b\n"));
        assert_eq!(html, "<p>ab</p>", "{c:?}");
        assert_eq!(said(&sources, &document), [(c, c.to_string())], "{c:?}");
    }
    let (sources, document, html) = read_text("a\tb\r\nc\n");
    assert_eq!(html, "<p>a\tb\nc</p>");
    assert!(said(&sources, &document).is_empty());
}

/// No reader can see one in code either, and one copying the code would
/// copy a character they can't see.
#[test]
fn it_is_left_out_wherever_it_is_typed() {
    let cases = [
        ("`a\u{b}b`", "<p><code>ab</code></p>"),
        (
            "```\na\u{b}b\n```",
            "<pre class=\"code\"><code>ab</code></pre>",
        ),
        ("$a\u{b}b$", "<p><math><mi>a</mi><mi>b</mi></math></p>"),
        ("@card(\"a\u{b}b\")", "<card [\"ab\"]>"),
        ("A \\@card(a\u{b}b) b", "<p>A @card(ab) b</p>"),
        ("`@card(a\u{b}b)`", "<p><code>@card(ab)</code></p>"),
        ("[a](/b\u{b}c/)", "<p><a href=\"/bc/\">a</a></p>"),
        ("<div>\na\u{b}b\n</div>", "<div>\nab\n</div>"),
        ("@box()\na\u{b}b\n@end", "<box []><p>ab</p>"),
        ("a\n\u{b}\nb", "<p>a</p>\n<p>b</p>"),
    ];
    for (text, wanted) in cases {
        let (sources, document, html) = read_text(text);
        assert_eq!(html, wanted, "{text:?}");
        assert_eq!(
            said(&sources, &document),
            [('\u{b}', "\u{b}".to_owned())],
            "{text:?}"
        );
    }
}

#[test]
fn a_character_written_as_a_reference_is_the_authors_html() {
    let (sources, document, _) = read_text("a&#11;b\n");
    assert!(said(&sources, &document).is_empty());
    let mut text = String::new();
    document.root.walk(&mut |node| {
        if let Kind::Text(words) = &node.kind {
            text.push_str(words);
        }
    });
    assert_eq!(text, "a\u{b}b");
}
