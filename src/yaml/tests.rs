use super::*;
use crate::origin::Style;
use crate::source::LineColumn;

fn one(path: &str, text: &str) -> (Sources, SourceId) {
    let mut sources = Sources::default();
    let id = sources.add(path, text.to_owned());
    (sources, id)
}

fn line_column(sources: &Sources, place: Place) -> (usize, usize) {
    let LineColumn { line, column } = sources.line_column(place.source, place.span.start as usize);
    (line, column)
}

fn read_yaml(text: &str) -> (Sources, Result<Node, Problem>) {
    let (sources, id) = one("statac.yaml", text);
    let node = read(&sources, id);
    (sources, node)
}

/// The problem reading `text`, where it is (line, column) and its spelling there.
fn problem(text: &str) -> (Kind, (usize, usize), String) {
    let (sources, node) = read_yaml(text);
    let problem = node.unwrap_err();
    let place = problem.place.expect("a problem in a file has a place");
    (
        problem.kind,
        line_column(&sources, place),
        sources.spelling(place).to_owned(),
    )
}

fn items(node: &Node) -> &[Node] {
    match &node.value {
        Value::Sequence(items) => items,
        other => panic!("a list, not {other:?}"),
    }
}

#[test]
fn a_value_three_levels_deep_is_placed_exactly() {
    let text = "book:\n  editions:\n    - formats:\n        - title: Post 1\n";
    let (sources, node) = read_yaml(text);
    let node = node.unwrap();
    let editions = items(node.get("book").unwrap().get("editions").unwrap());
    let formats = items(editions[0].get("formats").unwrap());
    let title = formats[0].get("title").unwrap();
    assert_eq!(line_column(&sources, title.place), (4, 18));
    assert_eq!(sources.spelling(title.place), "Post 1");

    let Value::Mapping(entries) = &formats[0].value else {
        panic!("a mapping")
    };
    let key = &entries[0].key;
    assert_eq!(line_column(&sources, key.place), (4, 11));
    assert_eq!(sources.spelling(key.place), "title");
    assert_eq!(key.style, Style::Plain);
}

#[test]
fn every_scalar_style_is_kept_with_its_text() {
    let cases = [
        ("Post 1", Style::Plain, "Post 1", "Post 1"),
        ("'it''s'", Style::SingleQuoted, "'it''s'", "it's"),
        (
            "\"Post\\t1\"",
            Style::DoubleQuoted,
            "\"Post\\t1\"",
            "Post\t1",
        ),
        (
            "|\n  one\n  two\n",
            Style::Literal,
            "|\n  one\n  two",
            "one\ntwo\n",
        ),
        (
            ">\n  one\n  two\n",
            Style::Folded,
            ">\n  one\n  two",
            "one two\n",
        ),
        ("|-\n  one\n", Style::Literal, "|-\n  one", "one"),
        ("", Style::Plain, "", ""),
    ];
    for (written, style, spelling, value) in cases {
        let text = format!("v: {written}\n");
        let (sources, node) = read_yaml(&text);
        let node = node.unwrap();
        let v = node.get("v").unwrap();
        assert_eq!(v.style, style, "{written:?}");
        assert_eq!(sources.spelling(v.place), spelling, "{written:?}");
        assert_eq!(v.value, Value::Scalar(value.into()), "{written:?}");
    }
}

#[test]
fn scalars_resolve_by_the_yaml_1_2_core_schema() {
    use Resolved::*;
    let cases = [
        ("3.10", Float(3.1)),
        ("1e3", Float(1000.0)),
        ("0x10", Int(Some(16))),
        ("010", Int(Some(10))),
        ("1_000", Str),
        ("yes", Str),
        ("on", Str),
        ("~", Null),
        ("null", Null),
        ("", Null),
        ("true", Bool(true)),
        ("1984", Int(Some(1984))),
        // Quoted, block and `!!str` scalars are always strings.
        ("'3.10'", Str),
        ("\"null\"", Str),
        ("|\n  1984\n", Str),
        ("!!str 3.10", Str),
        ("!!float 1", Float(1.0)),
        ("!!int 1984", Int(Some(1984))),
    ];
    for (written, resolved) in cases {
        let (_, node) = read_yaml(&format!("v: {written}\n"));
        let node = node.unwrap();
        assert_eq!(
            node.get("v").unwrap().resolved(),
            Some(resolved),
            "{written:?}"
        );
    }
    let (_, node) = read_yaml("v: !!int abc\n");
    assert_eq!(node.unwrap().get("v").unwrap().resolved(), None);
}

#[test]
fn crlf_and_a_byte_order_mark_keep_places_exact() {
    let text = "\u{feff}title: Post 1\r\nbook:\r\n  author: 'Author 1'\r\n  note: |\r\n    one\r\n    two\r\n\r\n  isbn: 1\r\n";
    let (sources, node) = read_yaml(text);
    let node = node.unwrap();
    let title = node.get("title").unwrap();
    assert_eq!(line_column(&sources, title.place), (1, 8));
    assert_eq!(sources.spelling(title.place), "Post 1");
    let book = node.get("book").unwrap();
    let author = book.get("author").unwrap();
    assert_eq!(line_column(&sources, author.place), (3, 11));
    assert_eq!(sources.spelling(author.place), "'Author 1'");
    let note = book.get("note").unwrap();
    assert_eq!(sources.spelling(note.place), "|\r\n    one\r\n    two");
    assert_eq!(note.value, Value::Scalar("one\ntwo\n".into()));
    assert_eq!(
        line_column(&sources, book.get("isbn").unwrap().place),
        (8, 9)
    );

    let markdown =
        "\u{feff}---\r\ntitle: Post 1\r\nbook:\r\n  author: Author 1\r\n---\r\nBody.\r\n";
    let (sources, id) = one("content/post-1.md", markdown);
    let (node, body) = front_matter(&sources, id).unwrap();
    let node = node.unwrap();
    let author = node.get("book").unwrap().get("author").unwrap();
    assert_eq!(line_column(&sources, author.place), (4, 11));
    assert_eq!(sources.spelling(author.place), "Author 1");
    assert_eq!(&markdown[body.range()], "Body.\r\n");
}

#[test]
fn malformed_yaml_is_placed_where_the_author_would_fix_it() {
    use Malformed::*;
    let cases = [
        // The parser notices an unclosed quote on a later line; it is placed at the quote.
        (
            "title: Post 1\nbook:\n  author: \"Author 1\n  isbn: 1\n",
            UnclosedQuote('"'),
            (3, 11),
            "\"",
        ),
        ("title: 'Post 1\n", UnclosedQuote('\''), (1, 8), "'"),
        ("title: \"Post 1", UnclosedQuote('"'), (1, 8), "\""),
        (
            "title: Post 1 # it's\nsummary: \"Folded\n",
            UnclosedQuote('"'),
            (2, 10),
            "\"",
        ),
        ("title: [Post 1\n", UnclosedBracket('['), (1, 8), "["),
        (
            "book: {author: Author 1\n",
            UnclosedBracket('{'),
            (1, 7),
            "{",
        ),
        (
            "tags: [Tag 1, Tag 2}\n",
            MismatchedBracket {
                open: '[',
                close: '}',
            },
            (1, 7),
            "[",
        ),
        ("title: ]\n", Unexpected("]".into()), (1, 8), "]"),
        (
            "title: \"Post\\q1\"\n",
            UnknownEscape("\\q".into()),
            (1, 13),
            "\\q",
        ),
        (
            "title: \"Post\\x4\"\n",
            UnknownEscape("\\x4".into()),
            (1, 13),
            "\\x4",
        ),
        ("title: 'Post 1' extra\n", TextAfterQuote, (1, 17), "e"),
        ("tags:\n\t- Tag 1\n", Tab, (2, 1), "\t"),
        ("title: Post 1\n\ttags: Tag 1\n", Tab, (2, 1), "\t"),
        (
            "book: *nope\n",
            UnknownAnchor("*nope".into()),
            (1, 7),
            "*nope",
        ),
        (
            "title: Post 1\n---\ntitle: Post 2\n",
            SecondDocument,
            (2, 1),
            "---",
        ),
        ("title: Post: 1\n", Unexpected(":".into()), (1, 12), ":"),
        (
            "title: Post 1\n  author: Author 1\n",
            Unexpected(":".into()),
            (2, 9),
            ":",
        ),
        (
            "- Post 1\ntitle: Post 2\n",
            Unexpected("title".into()),
            (2, 1),
            "title",
        ),
        ("title: @x\n", Unexpected("@".into()), (1, 8), "@"),
    ];
    for (text, found, at, spelling) in cases {
        assert_eq!(
            problem(text),
            (Kind::MalformedYaml { found }, at, spelling.to_owned()),
            "{text:?}"
        );
    }
}

#[test]
fn malformed_front_matter_is_placed_in_its_file() {
    let markdown = "---\ntitle: Post 1\nbook:\n  author: \"Author 1\n  isbn: 1\n---\n";
    let (sources, id) = one("content/post-1.md", markdown);
    let problem = front_matter(&sources, id).unwrap_err();
    let place = problem.place.unwrap();
    assert_eq!(line_column(&sources, place), (4, 11));
    assert_eq!(sources.path(place.source), "content/post-1.md");
    assert_eq!(problem.headline(), "Malformed YAML: `\"` is never closed");
}

#[test]
fn front_matter_is_optional_but_must_be_closed() {
    let (sources, id) = one("content/post-1.md", "Body.\n");
    let (node, body) = front_matter(&sources, id).unwrap();
    assert!(node.is_none());
    assert_eq!(body, Span::new(0, 6));

    let (sources, id) = one("content/post-1.md", "---\ntitle: Post 1\n\nBody.\n");
    let problem = front_matter(&sources, id).unwrap_err();
    assert_eq!(problem.kind, Kind::UnclosedFrontMatter);
    assert_eq!(sources.spelling(problem.place.unwrap()), "---");

    let (sources, id) = one("content/post-1.md", "---\n---\nBody.\n");
    let (node, _) = front_matter(&sources, id).unwrap();
    assert_eq!(node.unwrap().resolved(), Some(Resolved::Null));
}

#[test]
fn a_key_given_twice_is_placed_at_the_second_with_the_first_known() {
    let text = "title: Post 1\nbook:\n  author: Author 1\n  'author': Author 2\n";
    let (sources, node) = read_yaml(text);
    let problem = node.unwrap_err();
    let Kind::DuplicateKey { key, first } = &problem.kind else {
        panic!("{problem:?}")
    };
    assert_eq!(key, "author");
    let second = problem.place.unwrap();
    assert_eq!(line_column(&sources, second), (4, 3));
    assert_eq!(sources.spelling(second), "'author'");
    assert_eq!(line_column(&sources, *first), (3, 3));
    assert_eq!(problem.headline(), "Duplicate key: `author`");

    // The same key in two mappings is two keys.
    let (_, node) = read_yaml("a:\n  title: Post 1\nb:\n  title: Post 2\n");
    assert!(node.is_ok());
}

#[test]
fn an_alias_reads_as_its_anchor_and_is_placed_there() {
    let (sources, node) = read_yaml("a: &x Post 1\nb: *x\n");
    let node = node.unwrap();
    let (a, b) = (node.get("a").unwrap(), node.get("b").unwrap());
    assert_eq!(a, b);
    assert_eq!(b.value, Value::Scalar("Post 1".into()));
    assert_eq!(line_column(&sources, b.place), (1, 7));
}

#[test]
fn aliases_may_repeat_only_so_much() {
    let anchors = "a: &a [x, x, x, x, x, x, x, x, x, x]\n\
                   b: &b [*a, *a, *a, *a, *a, *a, *a, *a, *a, *a]\n\
                   c: &c [*b, *b, *b, *b, *b, *b, *b, *b, *b, *b]\n";
    let (_, node) = read_yaml(&format!("{anchors}d: [*c, *c]\n"));
    assert!(node.is_ok());

    // Each `*c` repeats 1,111 nodes; the eighth takes the total past the limit.
    let text = format!("{anchors}d: [*c, *c, *c, *c, *c, *c, *c, *c, *c, *c]\n");
    let (kind, at, spelling) = problem(&text);
    assert_eq!(
        kind,
        Kind::AliasLimit {
            alias: "*c".into(),
            limit: Repeated::Values(ALIAS_LIMIT)
        }
    );
    assert_eq!(at, (4, 33));
    assert_eq!(spelling, "*c");
}

#[test]
fn aliases_may_repeat_only_so_much_text() {
    // One long scalar stays far under the node limit however often it is repeated.
    let long = "x".repeat(300_000);
    let (_, node) = read_yaml(&format!("a: &a {long}\nb: [*a, *a, *a]\n"));
    assert!(node.is_ok());

    let (kind, _, spelling) = problem(&format!("a: &a {long}\nb: [*a, *a, *a, *a]\n"));
    assert_eq!(
        kind,
        Kind::AliasLimit {
            alias: "*a".into(),
            limit: Repeated::TextBytes(ALIAS_TEXT_LIMIT)
        }
    );
    assert_eq!(spelling, "*a");
}

#[test]
fn a_tag_outside_the_core_schema_is_refused() {
    let (kind, at, spelling) = problem("title: !local Post 1\n");
    assert_eq!(
        kind,
        Kind::UnknownYamlTag {
            tag: "!local".into()
        }
    );
    assert_eq!((at, spelling.as_str()), ((1, 8), "!local"));
    for tag in ["!!timestamp", "!<tag:example.com,2000:x>"] {
        let (kind, _, spelling) = problem(&format!("date: {tag} 2026-10-04\n"));
        assert_eq!(kind, Kind::UnknownYamlTag { tag: tag.into() });
        assert_eq!(spelling, tag);
    }
}

#[test]
fn a_binary_value_is_read_as_its_bytes() {
    let text =
        "a: !!binary SGVsbG8=\nb: !!binary |\n  SGVs\n  bG8h\nc: !!binary /w==\nd: !!binary \"\"\n";
    let (_, node) = read_yaml(text);
    let node = node.unwrap();
    let cases = [
        ("a", "Hello", Some(Tag::Binary), Some(Resolved::Str)),
        ("b", "Hello!", Some(Tag::Binary), Some(Resolved::Str)),
        ("c", "", Some(Tag::Bytes), None),
        ("d", "", Some(Tag::Binary), Some(Resolved::Str)),
    ];
    for (key, value, tag, resolved) in cases {
        let found = node.get(key).unwrap();
        assert_eq!(found.value, Value::Scalar(value.into()), "{key}");
        assert_eq!((found.tag, found.resolved()), (tag, resolved), "{key}");
    }
    for (written, at) in [
        ("a: !!binary SGVsbG8*\n", "SGVsbG8*"),
        ("a: !!binary [SGVs]\n", "["),
    ] {
        let (kind, _, spelling) = problem(written);
        assert_eq!(
            kind,
            Kind::MalformedYaml {
                found: Malformed::NotBase64
            },
            "{written}"
        );
        assert_eq!(spelling, at, "{written}");
    }
}

fn read_json_text(text: &str) -> (Sources, Result<Node, Problem>) {
    let (sources, id) = one("data/a.json", text);
    let node = read_json(&sources, id);
    (sources, node)
}

#[test]
fn json_is_read_into_the_tree_with_every_value_placed() {
    let text = "\u{feff}{\n\t\"a\":1, \"b\": [true, null, -2.5e3, \"\"],\n  \"c\": {\"d\": \"x\\u00e9\\/\\ud83d\\ude00\\n\\\"\"}}\n";
    let (sources, node) = read_json_text(text);
    let node = node.unwrap();
    let a = node.get("a").unwrap();
    assert_eq!(
        (a.resolved(), a.style, sources.spelling(a.place)),
        (Some(Resolved::Int(Some(1))), Style::Plain, "1")
    );
    let b: Vec<_> = items(node.get("b").unwrap())
        .iter()
        .map(|n| (n.resolved(), sources.spelling(n.place)))
        .collect();
    assert_eq!(
        b,
        [
            (Some(Resolved::Bool(true)), "true"),
            (Some(Resolved::Null), "null"),
            (Some(Resolved::Float(-2500.0)), "-2.5e3"),
            (Some(Resolved::Str), "\"\""),
        ]
    );
    let d = node.get("c").unwrap().get("d").unwrap();
    assert_eq!(d.value, Value::Scalar("xé/\u{1f600}\n\"".into()));
    assert_eq!(d.style, Style::DoubleQuoted);
    let Value::Mapping(entries) = &node.value else {
        panic!("an object");
    };
    assert_eq!(sources.spelling(entries[2].key.place), "\"c\"");
    assert_eq!(line_column(&sources, entries[2].key.place), (3, 3));
    assert_eq!(
        sources.spelling(node.get("c").unwrap().place),
        "{\"d\": \"x\\u00e9\\/\\ud83d\\ude00\\n\\\"\"}"
    );
}

#[test]
fn a_json_string_takes_every_character_from_u0020_up() {
    let (sources, node) = read_json_text("{\"a\": \"x\u{92}y\u{7f}z\u{80}\u{85}\"}");
    let a = node.unwrap();
    let a = a.get("a").unwrap();
    assert_eq!(a.value, Value::Scalar("x\u{92}y\u{7f}z\u{80}\u{85}".into()));
    assert_eq!(sources.spelling(a.place), "\"x\u{92}y\u{7f}z\u{80}\u{85}\"");
    // YAML allows them nowhere, as before.
    let (kind, _, spelling) = problem("a: \"x\u{92}\"\n");
    assert_eq!(
        kind,
        Kind::MalformedYaml {
            found: Malformed::ControlCharacter('\u{92}')
        }
    );
    assert_eq!(spelling, "\u{92}");
}

#[test]
fn malformed_json_is_placed_where_the_author_would_fix_it() {
    let cases = [
        ("{\"a\": \"x}", JsonFault::UnclosedString, "\""),
        ("{\"a\": \"x\ny\"}", JsonFault::UnclosedString, "\""),
        ("[1, 2", JsonFault::UnclosedBracket('['), "["),
        ("{\"a\": 1", JsonFault::UnclosedBracket('{'), "{"),
        (
            "[1, 2}",
            JsonFault::MismatchedBracket {
                open: '[',
                close: '}',
            },
            "}",
        ),
        ("[\"a\\q\"]", JsonFault::UnknownEscape("\\q".into()), "\\q"),
        (
            "[\"a\\u12g4\"]",
            JsonFault::UnknownEscape("\\u12".into()),
            "\\u12",
        ),
        (
            "[\"\\ud83d x\"]",
            JsonFault::HalfCharacter("\\ud83d".into()),
            "\\ud83d",
        ),
        (
            "[\"\\ude00\"]",
            JsonFault::HalfCharacter("\\ude00".into()),
            "\\ude00",
        ),
        ("[\"a\tb\"]", JsonFault::ControlCharacter('\t'), "\t"),
        ("[01]", JsonFault::NotANumber("01".into()), "01"),
        ("[1.]", JsonFault::NotANumber("1.".into()), "1."),
        ("[-]", JsonFault::NotANumber("-".into()), "-"),
        ("[1e]", JsonFault::NotANumber("1e".into()), "1e"),
        ("[1, 2,]", JsonFault::TrailingComma, ","),
        ("{\"a\": 1,}", JsonFault::TrailingComma, ","),
        ("[.5]", JsonFault::Unexpected(".5".into()), ".5"),
        ("['a']", JsonFault::Unexpected("'a'".into()), "'a'"),
        ("{a: 1}", JsonFault::Unexpected("a".into()), "a"),
        ("[True]", JsonFault::Unexpected("True".into()), "True"),
        ("[1 2]", JsonFault::Unexpected("2".into()), "2"),
        ("{\"a\" 1}", JsonFault::Unexpected("1".into()), "1"),
        ("{\"a\": }", JsonFault::Unexpected("}".into()), "}"),
        ("[1] [2]", JsonFault::Unexpected("[".into()), "["),
        ("", JsonFault::EndsEarly, ""),
        ("[1,", JsonFault::UnclosedBracket('['), "["),
    ];
    for (text, fault, at) in cases {
        let (sources, node) = read_json_text(text);
        let problem = node.unwrap_err();
        assert_eq!(
            problem.kind,
            Kind::MalformedJson { found: fault },
            "{text:?}"
        );
        assert_eq!(sources.spelling(problem.place.unwrap()), at, "{text:?}");
    }
    let deep = format!("{}{}", "[".repeat(300), "]".repeat(300));
    let (_, node) = read_json_text(&deep);
    assert_eq!(
        node.unwrap_err().kind,
        Kind::MalformedJson {
            found: JsonFault::TooDeep
        }
    );
}

#[test]
fn a_json_key_written_twice_is_placed_at_the_second() {
    let (sources, node) = read_json_text("{\"a\": 1, \"b\": {\"a\": 2}, \"a\": 3}");
    let problem = node.unwrap_err();
    let Kind::DuplicateKey { key, first } = &problem.kind else {
        panic!("{problem:?}");
    };
    assert_eq!(key, "a");
    assert_eq!(first.span.start, 1);
    assert_eq!(problem.place.unwrap().span.start, 24);
    assert_eq!(sources.spelling(problem.place.unwrap()), "\"a\"");
}

#[test]
fn columns_count_characters_and_spans_count_bytes() {
    let (sources, node) = read_yaml("title: Café\ntags: [ü, 'ö', x]\n");
    let node = node.unwrap();
    assert_eq!(sources.spelling(node.get("title").unwrap().place), "Café");
    let places: Vec<_> = items(node.get("tags").unwrap())
        .iter()
        .map(|tag| {
            (
                line_column(&sources, tag.place).1,
                sources.spelling(tag.place),
            )
        })
        .collect();
    assert_eq!(places, [(8, "ü"), (11, "'ö'"), (16, "x")]);
}

#[test]
fn a_quoted_scalar_ends_at_its_quote_not_at_a_comment() {
    let (sources, node) = read_yaml("title: 'Post 1'   # a comment\nyear: \"1984\" # another\n");
    let node = node.unwrap();
    assert_eq!(
        sources.spelling(node.get("title").unwrap().place),
        "'Post 1'"
    );
    assert_eq!(
        sources.spelling(node.get("year").unwrap().place),
        "\"1984\""
    );
}

#[test]
fn block_scalars_keep_their_header_and_no_trailing_blank_lines() {
    let text = "a: |+ # kept\n  one\n\n\nb: >2-\n   two\nc:\n  - |\n    three\n    # inside\n  # outside\nd: |\n";
    let (sources, node) = read_yaml(text);
    let node = node.unwrap();
    let cases = [
        ("a", "|+ # kept\n  one", "one\n\n\n"),
        ("b", ">2-\n   two", " two"),
        ("d", "|", ""),
    ];
    for (key, spelling, value) in cases {
        let v = node.get(key).unwrap();
        assert_eq!(sources.spelling(v.place), spelling, "{key}");
        assert_eq!(v.value, Value::Scalar(value.into()), "{key}");
    }
    let c = &items(node.get("c").unwrap())[0];
    assert_eq!(sources.spelling(c.place), "|\n    three\n    # inside");
    assert_eq!(line_column(&sources, c.place), (8, 5));
}

#[test]
fn an_empty_file_is_no_value() {
    for text in ["", "# a comment\n", "\u{feff}"] {
        let (_, node) = read_yaml(text);
        assert_eq!(node.unwrap().resolved(), Some(Resolved::Null), "{text:?}");
    }
}

mod typed {
    use std::collections::BTreeMap;

    use serde::Deserialize;

    use super::*;
    use crate::origin::{Arrival, Sourced};

    /// Line, column, spelling and style of a value, and that it arrived as written.
    fn check<T>(
        sources: &Sources,
        sourced: &Sourced<T>,
        line: usize,
        column: usize,
        spelling: &str,
        style: Style,
    ) {
        let place = sourced.origin.place.expect("a written value has a place");
        assert_eq!(
            (
                line_column(sources, place),
                sources.spelling(place),
                sourced.origin.style
            ),
            ((line, column), spelling, style)
        );
        assert_eq!(sourced.origin.arrival, Arrival::Written);
    }

    fn typed<T: serde::de::DeserializeOwned>(text: &str) -> Result<T, Error> {
        let (_, node) = read_yaml(text);
        from_node(&node.expect("well-formed YAML"))
    }

    const POST: &str = "---
title: Post 1
year: 1984
draft: false
weight: 3
tags: [Tag 1, 'Tag 2', \"Tag 3\"]
book:
  author: Author 1
  isbn: '0000000001'
  editions:
    - year: 2001
      note: |
        Line one.
        Line two.
summary: >-
  Folded
  text.
---

Body.
";

    #[derive(Debug, Deserialize)]
    struct Page {
        title: Sourced<String>,
        year: Option<Sourced<String>>,
        draft: Option<Sourced<bool>>,
        weight: Option<Sourced<i64>>,
        tags: Option<Sourced<Vec<Sourced<String>>>>,
        book: Option<Sourced<Book>>,
        summary: Option<Sourced<String>>,
    }

    #[derive(Debug, Deserialize)]
    struct Book {
        author: Sourced<String>,
        isbn: Sourced<String>,
        editions: Sourced<Vec<Sourced<Edition>>>,
    }

    #[derive(Debug, Deserialize)]
    struct Edition {
        year: Sourced<u16>,
        note: Sourced<String>,
    }

    #[test]
    fn every_value_in_nested_front_matter_has_its_place_and_style() {
        let (sources, id) = one("content/post-1.md", POST);
        let (node, body) = front_matter(&sources, id).unwrap();
        assert_eq!(&POST[body.range()], "\nBody.\n");
        let node = node.unwrap();
        let page: Page = from_node(&node).unwrap();
        let s = &sources;

        check(s, &page.title, 2, 8, "Post 1", Style::Plain);
        assert_eq!(page.title.value, "Post 1");
        let year = page.year.unwrap();
        check(s, &year, 3, 7, "1984", Style::Plain);
        assert_eq!(year.value, "1984");
        let draft = page.draft.unwrap();
        check(s, &draft, 4, 8, "false", Style::Plain);
        assert!(!draft.value);
        let weight = page.weight.unwrap();
        check(s, &weight, 5, 9, "3", Style::Plain);
        assert_eq!(weight.value, 3);

        let tags = page.tags.unwrap();
        check(s, &tags, 6, 7, "[Tag 1, 'Tag 2', \"Tag 3\"]", Style::None);
        check(s, &tags.value[0], 6, 8, "Tag 1", Style::Plain);
        check(s, &tags.value[1], 6, 15, "'Tag 2'", Style::SingleQuoted);
        check(s, &tags.value[2], 6, 24, "\"Tag 3\"", Style::DoubleQuoted);
        assert_eq!(tags.value[1].value, "Tag 2");
        assert_eq!(tags.value[2].value, "Tag 3");

        let book = page.book.unwrap();
        let place = book.origin.place.unwrap();
        assert_eq!(line_column(s, place), (8, 3));
        assert!(sources.spelling(place).starts_with("author: Author 1\n"));
        assert!(sources.spelling(place).ends_with("Line two."));
        check(s, &book.value.author, 8, 11, "Author 1", Style::Plain);
        check(
            s,
            &book.value.isbn,
            9,
            9,
            "'0000000001'",
            Style::SingleQuoted,
        );
        assert_eq!(book.value.isbn.value, "0000000001");
        let editions = &book.value.editions;
        assert_eq!(line_column(s, editions.origin.place.unwrap()), (11, 5));
        let edition = &editions.value[0];
        assert_eq!(line_column(s, edition.origin.place.unwrap()), (11, 7));
        check(s, &edition.value.year, 11, 13, "2001", Style::Plain);
        assert_eq!(edition.value.year.value, 2001);
        let note = &edition.value.note;
        check(
            s,
            note,
            12,
            13,
            "|\n        Line one.\n        Line two.",
            Style::Literal,
        );
        assert_eq!(note.value, "Line one.\nLine two.\n");

        let summary = page.summary.unwrap();
        check(s, &summary, 15, 10, ">-\n  Folded\n  text.", Style::Folded);
        assert_eq!(summary.value, "Folded text.");
    }

    #[test]
    fn an_alias_hands_on_its_anchors_origin() {
        #[derive(Debug, Deserialize)]
        struct Two {
            a: Sourced<String>,
            b: Sourced<String>,
        }
        let two: Two = typed("a: &x Post 1\nb: *x\n").unwrap();
        assert_eq!(two.b.value, "Post 1");
        assert_eq!(two.a.origin, two.b.origin);
    }

    #[derive(Debug, Deserialize)]
    struct Text {
        v: Sourced<String>,
    }

    #[test]
    fn a_text_value_takes_a_bare_scalar_only_where_reading_keeps_its_characters() {
        let kept = [
            ("1984", "1984"),
            ("-7", "-7"),
            ("2.5", "2.5"),
            ("true", "true"),
            ("Post 1", "Post 1"),
            ("yes", "yes"),
            ("on", "on"),
            ("1_000", "1_000"),
            ("2026-10-04", "2026-10-04"),
            ("'3.10'", "3.10"),
            ("\"0x10\"", "0x10"),
            ("!!str 3.10", "3.10"),
        ];
        for (written, value) in kept {
            let text: Text = typed(&format!("v: {written}\n")).unwrap();
            assert_eq!(text.v.value, value, "{written}");
        }
        for written in [
            "3.10", "1e3", "0x10", "0o10", "010", "+12", "1.0", ".inf", "True", "~", "null", "",
        ] {
            let error = typed::<Text>(&format!("v: {written}\n")).unwrap_err();
            assert!(error.place.is_some(), "{written}");
        }
    }

    #[test]
    fn true_or_false_is_what_yaml_reads_as_true_or_false() {
        #[derive(Debug, Deserialize)]
        struct Flag {
            v: bool,
        }
        for (written, value) in [
            ("true", true),
            ("false", false),
            ("True", true),
            ("FALSE", false),
        ] {
            assert_eq!(typed::<Flag>(&format!("v: {written}")).unwrap().v, value);
        }
        for written in ["yes", "on", "'true'", "1"] {
            let (sources, node) = read_yaml(&format!("v: {written}"));
            let error = from_node::<Flag>(&node.unwrap()).unwrap_err();
            let place = error.place.unwrap();
            assert_eq!(line_column(&sources, place), (1, 4), "{written}");
            assert_eq!(sources.spelling(place), written);
        }
    }

    #[test]
    fn numbers_are_read_by_the_yaml_1_2_core_schema() {
        #[derive(Debug, Deserialize)]
        struct Whole {
            v: i64,
        }
        #[derive(Debug, Deserialize)]
        struct Number {
            v: f64,
        }
        for (written, value) in [
            ("0x10", 16),
            ("0o10", 8),
            ("010", 10),
            ("+12", 12),
            ("-3", -3),
        ] {
            assert_eq!(
                typed::<Whole>(&format!("v: {written}")).unwrap().v,
                value,
                "{written}"
            );
        }
        for written in [
            "1_000",
            "1e3",
            "'12'",
            "0b101",
            "1:30",
            "2.0",
            "99999999999999999999",
        ] {
            assert!(
                typed::<Whole>(&format!("v: {written}")).is_err(),
                "{written}"
            );
        }
        for (written, value) in [("3.10", 3.1), ("1e3", 1000.0), (".5", 0.5), ("7", 7.0)] {
            assert_eq!(
                typed::<Number>(&format!("v: {written}")).unwrap().v,
                value,
                "{written}"
            );
        }
        assert!(typed::<Number>("v: .inf").unwrap().v.is_infinite());
    }

    #[test]
    fn plain_fields_read_without_places() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Plain {
            title: String,
            tags: Vec<String>,
            weight: Option<u8>,
            draft: Option<bool>,
        }
        let plain: Plain = typed("title: Post 1\ntags: [Tag 1, Tag 2]\nweight: 3\n").unwrap();
        assert_eq!(
            plain,
            Plain {
                title: "Post 1".into(),
                tags: vec!["Tag 1".into(), "Tag 2".into()],
                weight: Some(3),
                draft: None,
            }
        );
    }

    #[test]
    fn a_wrong_type_deep_inside_is_placed_at_the_value() {
        let text = "title: Post 1\nbook:\n  author: Author 1\n  isbn: '1'\n  editions:\n    - year: soon\n      note: x\n";
        let (sources, node) = read_yaml(text);
        let error = from_node::<Page>(&node.unwrap()).unwrap_err();
        let place = error.place.unwrap();
        assert_eq!(line_column(&sources, place), (6, 13), "{error}");
        assert_eq!(sources.spelling(place), "soon");
    }

    #[test]
    fn a_missing_field_is_placed_at_its_mapping() {
        let text = "title: Post 1\nbook:\n  author: Author 1\n  editions: []\n";
        let (sources, node) = read_yaml(text);
        let error = from_node::<Page>(&node.unwrap()).unwrap_err();
        assert_eq!(error.message, "missing field `isbn`");
        assert_eq!(line_column(&sources, error.place.unwrap()), (3, 3));
    }

    #[test]
    fn an_unknown_field_refused_by_its_struct_is_placed_at_its_key() {
        #[derive(Debug, Deserialize)]
        #[serde(deny_unknown_fields)]
        #[allow(dead_code)]
        struct Strict {
            title: String,
        }
        let (sources, node) = read_yaml("title: Post 1\ntitel: Post 1\n");
        let error = from_node::<Strict>(&node.unwrap()).unwrap_err();
        assert_eq!(line_column(&sources, error.place.unwrap()), (2, 1));
    }

    #[test]
    fn keys_no_field_takes_are_noted_with_their_places() {
        #[derive(Debug, Deserialize)]
        #[allow(dead_code)]
        struct Known {
            title: String,
            book: Inner,
        }
        #[derive(Debug, Deserialize)]
        #[allow(dead_code)]
        struct Inner {
            author: String,
        }
        let text = "title: Post 1\ntemplate: post\nbook:\n  author: Author 1\n  isbn: [1, 2]\n";
        let (sources, node) = read_yaml(text);
        let node = node.unwrap();
        let (known, unknown) = from_node_noting_unknown::<Known>(&node).unwrap();
        assert_eq!(known.book.author, "Author 1");
        let noted: Vec<_> = unknown
            .iter()
            .map(|u| {
                (
                    sources.spelling(u.key.place),
                    line_column(&sources, u.key.place),
                    sources.spelling(u.value.place),
                )
            })
            .collect();
        assert_eq!(
            noted,
            [("template", (2, 1), "post"), ("isbn", (5, 3), "[1, 2]")]
        );
    }

    #[test]
    fn an_empty_value_is_none_for_an_option_and_refused_for_text() {
        #[derive(Debug, Deserialize)]
        struct T {
            title: Option<Sourced<String>>,
            year: Option<String>,
        }
        let t: T = typed("title:\nyear: ~\n").unwrap();
        assert!(t.title.is_none() && t.year.is_none());
        // The parser places a value written as nothing at its key's colon.
        let (sources, node) = read_yaml("v:\n");
        let error = from_node::<Text>(&node.unwrap()).unwrap_err();
        assert_eq!(line_column(&sources, error.place.unwrap()), (1, 2));
    }

    #[test]
    fn untyped_values_read_by_the_core_schema() {
        #[derive(Debug, Deserialize, PartialEq)]
        #[serde(untagged)]
        enum Any {
            Bool(bool),
            Int(i64),
            Float(f64),
            Text(String),
            List(Vec<Any>),
            Map(BTreeMap<String, Any>),
            Null(()),
        }
        let any: BTreeMap<String, Any> =
            typed("a: 1984\nb: 3.10\nc: yes\nd: [true, ~]\ne: {f: '1'}\n").unwrap();
        assert_eq!(any["a"], Any::Int(1984));
        assert_eq!(any["b"], Any::Float(3.1));
        assert_eq!(any["c"], Any::Text("yes".into()));
        assert_eq!(any["d"], Any::List(vec![Any::Bool(true), Any::Null(())]));
        assert_eq!(
            any["e"],
            Any::Map(BTreeMap::from([("f".to_string(), Any::Text("1".into()))]))
        );
    }
}

#[test]
fn a_control_character_yaml_allows_nowhere_is_named_at_its_place() {
    for (text, c, at) in [
        ("title: \"Post\u{92} 1\"\n", '\u{92}', (1, 13)),
        ("title: Post\u{92} 1\n", '\u{92}', (1, 12)),
        ("title: 'Post\u{1} 1'\n", '\u{1}', (1, 13)),
    ] {
        let (kind, place, spelling) = problem(text);
        assert_eq!(
            kind,
            Kind::MalformedYaml {
                found: Malformed::ControlCharacter(c)
            },
            "{text:?}"
        );
        assert_eq!(place, at, "{text:?}");
        assert_eq!(spelling, c.to_string());
    }
}

/// Shapes of a text in quotes on two lines, with `@` for the spaces
/// before its second, and the fewest YAML takes there.
const TWO_LINES: [(&str, usize); 14] = [
    ("title: \"One\n@two\"\n", 1),
    ("title: 'One\n@two'\n", 1),
    ("book:\n  name: \"One\n@two\"\n", 3),
    ("a:\n  b:\n    c: \"One\n@two\"\n", 5),
    ("- \"One\n@two\"\n", 0),
    ("tags:\n  - \"One\n@two\"\n", 2),
    ("tags:\n- \"One\n@two\"\n", 0),
    ("books:\n  - name: \"One\n@two\"\n", 5),
    ("books:\n  - a: 1\n    name: \"One\n@two\"\n", 5),
    ("- - \"One\n@two\"\n", 2),
    ("title:\n  \"One\n@two\"\n", 1),
    ("? a\n: \"One\n@two\"\n", 1),
    ("title: !!str \"One\n@two\"\n", 1),
    ("title: &a \"One\n@two\"\n", 1),
];

#[test]
fn a_later_line_of_a_text_in_quotes_is_read_from_the_depth_yaml_takes() {
    for (shape, fewest) in TWO_LINES {
        for spaces in 0..fewest + 3 {
            let text = shape.replace('@', &" ".repeat(spaces));
            let read = read_yaml(&text).1;
            assert_eq!(read.is_ok(), spaces >= fewest, "{text:?}");
        }
    }
}

#[test]
fn a_text_read_with_a_shallow_line_is_placed_as_it_is_written() {
    let text =
        "title: \"One\n two\n\n three\"\nbook:\n  name: 'Four\n   five'\n  isbn: \"6\ntags: [a]\n";
    let (sources, problem) = read_yaml(text);
    // What follows the texts is placed where it is written.
    let problem = problem.unwrap_err();
    let place = problem.place.unwrap();
    assert_eq!(
        problem.kind,
        Kind::MalformedYaml {
            found: Malformed::UnclosedQuote('"')
        }
    );
    assert_eq!(
        (line_column(&sources, place), sources.spelling(place)),
        ((8, 9), "\"")
    );

    let text = text.replace("\"6\n", "\"6\"\n");
    let (sources, node) = read_yaml(&text);
    let node = node.unwrap();
    let spelt = |node: &Node| sources.spelling(node.place).to_owned();
    let title = node.get("title").unwrap();
    assert_eq!(title.value, Value::Scalar("One two\nthree".into()));
    assert_eq!(spelt(title), "\"One\n two\n\n three\"");
    let book = node.get("book").unwrap();
    let name = book.get("name").unwrap();
    assert_eq!(name.value, Value::Scalar("Four five".into()));
    assert_eq!(spelt(name), "'Four\n   five'");
    assert_eq!(spelt(book.get("isbn").unwrap()), "\"6\"");
    assert_eq!(spelt(node.get("tags").unwrap()), "[a]");
}

#[test]
fn a_text_in_quotes_of_very_many_shallow_lines_costs_no_more_than_its_length() {
    let started = std::time::Instant::now();
    // One text of 200,000 lines, each a space short for the reader.
    let long = format!("title: \"One\n{} two\"\n", " b\n".repeat(200_000));
    assert!(read_yaml(&long).1.is_ok());
    // The same with each line a space short for YAML too.
    let long = format!("title: \"One\n{}two\"\n", "b\n".repeat(200_000));
    let (kind, at, _) = problem(&long);
    assert!(
        matches!(
            kind,
            Kind::MalformedYaml {
                found: Malformed::ShallowLine { .. }
            }
        ),
        "{kind:?}"
    );
    assert_eq!(at, (2, 1));
    // More texts than are read again for, each on its own line: told at
    // the first past what is read again.
    let many: String = (0..40_000)
        .map(|n| format!("a{n}: \"One\n two\"\n"))
        .collect();
    let (kind, at, _) = problem(&many);
    let found = Malformed::ShallowLine {
        quote: '"',
        back: 6,
        spaces: 1,
    };
    assert_eq!(kind, Kind::MalformedYaml { found });
    assert!(at.0 > 2 && at.0 % 2 == 0 && at.1 == 2, "{at:?}");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(20),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn a_line_of_a_text_in_quotes_shallower_than_yaml_takes_is_told_with_the_spaces_that_mend_it() {
    // The text, where the line is told, and the spaces offered.
    let cases = [
        ("title: \"One\ntwo\"\n", (2, 1), 1),
        ("title: 'One\ntwo'\n", (2, 1), 1),
        ("title: \"One\n two\nthree\"\n", (3, 1), 1),
        ("title: \"One\nthree\n two\"\n", (2, 1), 1),
        ("title: \"One \\\" two\nthree\"\n", (2, 1), 1),
        ("title: 'One '' two\nthree'\n", (2, 1), 1),
        ("book:\n  name: \"One\n  two\"\n", (3, 3), 3),
        ("book:\n  name: \"One\n\n two\"\n", (4, 2), 3),
        ("books:\n  - name: \"One\n    two\"\n", (3, 5), 5),
        ("tags:\n  - \"One\n two\"\n", (3, 2), 2),
        ("- - \"One\n two\"\n", (2, 2), 2),
        ("title:\n  \"One\ntwo\"\n", (3, 1), 3),
    ];
    for (text, at, spaces) in cases {
        let (kind, told_at, told) = problem(text);
        let Kind::MalformedYaml {
            found:
                Malformed::ShallowLine {
                    back,
                    spaces: offered,
                    ..
                },
        } = kind
        else {
            panic!("{text:?}: {kind:?}");
        };
        assert_eq!(
            (told_at, told.as_str(), offered),
            (at, "t", spaces),
            "{text:?}"
        );
        // The quote opens `back` bytes before the line as written.
        let line: usize = text.lines().take(at.0 - 1).map(|line| line.len() + 1).sum();
        assert!(
            matches!(&text[line + at.1 - 1 - back..], t if t.starts_with(['"', '\''])),
            "{text:?}"
        );
        // The spaces offered, before each later line, are enough.
        let opens = text
            .lines()
            .position(|line| line.contains(['"', '\'']))
            .unwrap();
        let mended: String = text
            .lines()
            .enumerate()
            .map(|(n, line)| match n > opens && !line.is_empty() {
                true => format!("{}{}\n", " ".repeat(spaces), line.trim_start()),
                false => format!("{line}\n"),
            })
            .collect();
        assert!(read_yaml(&mended).1.is_ok(), "{mended:?}");
    }
}

#[test]
fn a_text_in_quotes_that_takes_in_a_field_or_never_closes_is_told_as_never_closed() {
    let cases = [
        "title: \"One\nsummary: \"Two\"\n",
        "title: \"One\n- two\"\n",
        "book:\n  name: \"One\n  isbn: \"1\"\n",
        "title: \"One\n two\n",
        "title: \"One\n---\ntwo\"\n",
    ];
    for text in cases {
        let (kind, at, _) = problem(text);
        let wanted = Kind::MalformedYaml {
            found: Malformed::UnclosedQuote('"'),
        };
        assert_eq!(kind, wanted, "{text:?}");
        let line = text.lines().position(|line| line.contains('"')).unwrap() + 1;
        assert_eq!(at.0, line, "{text:?}");
    }
}
