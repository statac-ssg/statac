use std::path::Path;

use super::*;
use crate::origin::{Origin, Style};
use crate::problem::OptionHint;
use crate::source::Span;
use crate::yaml::Malformed;

const SITE: &str = "site";

/// A site of one file, and the place of the first `needle` in it.
fn one(text: &str, needle: &str) -> (Sources, Place) {
    let mut sources = Sources::default();
    let source = sources.add("site/content/post-1.md", text.to_owned());
    let start = text.find(needle).expect("the needle is in the text");
    let place = Place {
        source,
        span: Span::new(start, start + needle.len()),
    };
    (sources, place)
}

fn at(source: SourceId, start: usize, end: usize) -> Place {
    Place {
        source,
        span: Span::new(start, end),
    }
}

fn drawn(sources: &Sources, problem: &Problem) -> String {
    Report::new(sources, Path::new(SITE), false).problem(problem)
}

fn broken_link() -> Kind {
    Kind::BrokenLink {
        target: "/about-us/".into(),
        suggestion: Some("/about/".into()),
        inside: false,
    }
}

/// The lines of an expected drawing, each written after a `|` so the
/// leading spaces are visible.
fn lines(expected: &str) -> String {
    expected
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix('|'))
        .map(|line| format!("{line}\n"))
        .collect()
}

#[test]
fn a_simple_place_is_drawn_as_the_guide_lays_it_out() {
    let text = "---\ntitle: Post 1\n---\nRead more [about us](/about-us/).\n";
    let (sources, place) = one(text, "/about-us/");
    let problem = Problem::at(broken_link(), place);
    assert_eq!(
        drawn(&sources, &problem),
        lines(
            "
            |warning  Broken link: nothing is published at `/about-us/`
            |  content/post-1.md, line 4
            |   4 │ Read more [about us](/about-us/).
            |     │                      ^^^^^^^^^^
            |  Did you mean `/about/`?
            |  More: https://statac.dev/e/broken-link/
            "
        )
    );
}

#[test]
fn tabs_are_expanded_alike_in_the_line_and_under_it() {
    let (sources, place) = one("tags:\n\tkey:\tPost 1\n", "\t");
    let problem = Problem::at(
        Kind::MalformedYaml {
            found: Malformed::Tab,
        },
        place,
    );
    assert_eq!(
        drawn(&sources, &problem),
        lines(
            "
            |error  Malformed YAML: a tab where YAML needs spaces
            |  content/post-1.md, line 2
            |   2 │     key:    Post 1
            |     │ ^^^^
            |  Use spaces instead.
            |  More: https://statac.dev/e/malformed-yaml/
            "
        )
    );
    let (sources, place) = one("tags:\n\tkey:\tPost 1\n", "Post 1");
    let problem = Problem::at(broken_link(), place);
    assert!(drawn(&sources, &problem).contains("\n     │             ^^^^^^\n"));
}

#[test]
fn wide_characters_take_two_columns() {
    let text = "文章 1 🙂 [Post 2](/about-us/).\n";
    let (sources, place) = one(text, "/about-us/");
    let problem = Problem::at(broken_link(), place);
    assert_eq!(
        drawn(&sources, &problem),
        lines(
            "
            |warning  Broken link: nothing is published at `/about-us/`
            |  content/post-1.md, line 1
            |   1 │ 文章 1 🙂 [Post 2](/about-us/).
            |     │                    ^^^^^^^^^^
            |  Did you mean `/about/`?
            |  More: https://statac.dev/e/broken-link/
            "
        )
    );
    let (sources, place) = one("title: 文章\n", "文章");
    let problem = Problem::at(broken_link(), place);
    assert!(drawn(&sources, &problem).contains("\n     │        ^^^^\n"));
}

#[test]
fn combining_marks_take_no_column() {
    let line = "Cafe\u{301} [Post 2](/post-2/)";
    let from = line.find("/post-2/").unwrap();
    let drawn = Drawn::new(line, from, from + "/post-2/".len());
    assert_eq!(drawn.column, 14);
    assert_eq!(drawn.carets, 8);
}

#[test]
fn a_place_over_several_lines_says_where_it_ends() {
    let text = "title: Post 1\nsummary: !local |\n  one\n  two\n";
    let start = text.find("!local").unwrap();
    let (mut sources, _) = one("", "");
    let source = sources.add("site/content/post-2.md", text.to_owned());
    let problem = Problem::at(
        Kind::UnknownYamlTag {
            tag: "!local".into(),
        },
        at(source, start, text.len()),
    );
    assert_eq!(
        drawn(&sources, &problem),
        lines(
            "
            |error  Unknown YAML tag: `!local`
            |  content/post-2.md, line 2
            |   2 │ summary: !local |
            |     │          ^^^^^^^^ runs to line 4
            |  More: https://statac.dev/e/unknown-yaml-tag/
            "
        )
    );
}

#[test]
fn a_place_at_the_end_of_the_file_is_drawn_on_the_empty_line_after() {
    let text = "title: Post 1\n";
    let (sources, place) = one(text, "");
    let place = at(place.source, text.len(), text.len());
    let problem = Problem::at(
        Kind::MalformedYaml {
            found: Malformed::EndsEarly,
        },
        place,
    );
    assert_eq!(
        drawn(&sources, &problem),
        lines(
            "
            |error  Malformed YAML: ends too soon
            |  content/post-1.md, line 2
            |   2 │
            |     │ ^
            |  More: https://statac.dev/e/malformed-yaml/
            "
        )
    );
}

#[test]
fn a_place_past_the_last_character_sits_after_it() {
    let text = "title: \"Post 1";
    let (sources, place) = one(text, "");
    let place = at(place.source, text.len(), text.len());
    let problem = Problem::at(
        Kind::MalformedYaml {
            found: Malformed::EndsEarly,
        },
        place,
    );
    assert!(drawn(&sources, &problem).contains("   1 │ title: \"Post 1\n     │               ^\n"));
}

#[test]
fn a_place_on_an_empty_line_has_one_caret() {
    let text = "title: Post 1\n\ndate: 2026-10-04\n";
    let (sources, place) = one(text, "");
    let blank = "title: Post 1\n".len();
    let problem = Problem::at(
        Kind::MalformedYaml {
            found: Malformed::Indentation,
        },
        at(place.source, blank, blank),
    );
    assert!(drawn(&sources, &problem).contains("   2 │\n     │ ^\n"));
}

#[test]
fn a_long_line_is_shown_as_a_window_around_the_place() {
    let words = "one two three four five six seven eight nine ten ";
    let text = format!(
        "{}[about us](/about-us/) {}\n",
        words.repeat(3),
        words.repeat(3)
    );
    let (sources, place) = one(&text, "/about-us/");
    let problem = Problem::at(broken_link(), place);
    assert_eq!(
        drawn(&sources, &problem),
        lines(
            "
            |warning  Broken link: nothing is published at `/about-us/`
            |  content/post-1.md, line 1
            |   1 │ …ght nine ten [about us](/about-us/) one two three four five six seven \
             eight nine…
            |     │                          ^^^^^^^^^^
            |  Did you mean `/about/`?
            |  More: https://statac.dev/e/broken-link/
            "
        )
    );
}

#[test]
fn carets_longer_than_the_window_stop_at_its_edge() {
    let line = format!("{}{}", "x".repeat(50), "y".repeat(200));
    let drawn = Drawn::new(&line, 50, 250);
    let ys = "y".repeat(56);
    assert_eq!(drawn.text, format!("…{}{ys}…", "x".repeat(24)));
    assert_eq!(drawn.column, 25);
    assert_eq!(drawn.carets, 56);
}

#[test]
fn a_long_line_ending_in_its_place_keeps_its_end() {
    let text = format!("{}[about us](/about-us/)\n", "word ".repeat(40));
    let (sources, place) = one(&text, "/about-us/");
    let problem = Problem::at(broken_link(), place);
    let drawing = drawn(&sources, &problem);
    let line = drawing.lines().nth(2).unwrap();
    assert!(line.starts_with("   1 │ …"), "{line}");
    assert!(line.ends_with("[about us](/about-us/)"), "{line}");
}

#[test]
fn a_duplicate_key_shows_where_it_was_first_written() {
    let text = "title: Post 1\ndate: 2026-10-04\ntitle: Post 2\n";
    let (sources, first) = one(text, "title");
    let second = text.rfind("title").unwrap();
    let problem = Problem::at(
        Kind::DuplicateKey {
            key: "title".into(),
            first,
        },
        at(first.source, second, second + "title".len()),
    );
    assert_eq!(
        drawn(&sources, &problem),
        lines(
            "
            |error  Duplicate key: `title`
            |  content/post-1.md, line 3
            |   3 │ title: Post 2
            |     │ ^^^^^
            |  First written: content/post-1.md, line 1
            |   1 │ title: Post 1
            |     │ ^^^^^
            |  More: https://statac.dev/e/duplicate-key/
            "
        )
    );
}

#[test]
fn line_numbers_share_one_gutter() {
    let text = format!("title: Post 1\n{}title: Post 2\n", "x: 1\n".repeat(120));
    let (sources, first) = one(&text, "title");
    let second = text.rfind("title").unwrap();
    let problem = Problem::at(
        Kind::DuplicateKey {
            key: "title".into(),
            first,
        },
        at(first.source, second, second + "title".len()),
    );
    let drawing = drawn(&sources, &problem);
    assert!(drawing.contains("  122 │ title: Post 2\n      │ ^^^^^\n"));
    assert!(drawing.contains("    1 │ title: Post 1\n      │ ^^^^^\n"));
}

#[test]
fn a_problem_in_no_file_has_no_place_drawn() {
    let problem = Problem::nowhere(Kind::CantWrite {
        path: "out/index.html".into(),
        reason: "a folder already has that name".into(),
    });
    assert_eq!(
        drawn(&Sources::default(), &problem),
        lines(
            "
            |error  Can't write `out/index.html`: a folder already has that name
            |  More: https://statac.dev/e/cant-write/
            "
        )
    );
}

#[test]
fn control_characters_are_shown_not_acted_on() {
    let (sources, place) = one("title: Post\u{1b}[2J 1\n", "Post");
    let problem = Problem::at(broken_link(), place);
    assert!(drawn(&sources, &problem).contains("   1 │ title: Post\u{241b}[2J 1\n"));
}

/// C0 and DEL have pictures of their own, one column wide; C1 has none,
/// so each is named, and the caret lands under the character either way.
#[test]
fn a_control_character_is_drawn_where_it_can_be_seen() {
    let text = "a\u{b}b\u{92}c\u{7f}d\n";
    let (sources, place) = one(text, "\u{92}");
    let problem = Problem::at(
        Kind::ControlCharacter {
            character: '\u{92}',
        },
        place,
    );
    assert_eq!(
        drawn(&sources, &problem),
        lines(
            "
            |warning  Control character: U+0092 is left out of the page
            |  content/post-1.md, line 1
            |   1 │ a\u{240b}b<U+0092>c\u{2421}d
            |     │    ^^^^^^^^
            |  This is usually a character such as \u{2019} saved in an old encoding: type it again.
            |  More: https://statac.dev/e/control-character/
            "
        )
    );
    let (sources, place) = one(text, "d");
    let drawing = drawn(&sources, &Problem::at(broken_link(), place));
    assert!(
        drawing.contains(&format!("     │ {}^\n", " ".repeat(13))),
        "{drawing}"
    );
}

#[test]
fn direction_and_invisible_characters_are_named_not_acted_on() {
    let (sources, place) = one("title: Po\u{202e}st 1\u{200b}\n", "st 1");
    let problem = Problem::at(broken_link(), place);
    let drawing = drawn(&sources, &problem);
    let under = format!(
        "   1 │ title: Po<U+202E>st 1<U+200B>\n     │ {}^^^^\n",
        " ".repeat("title: Po<U+202E>".len())
    );
    assert!(drawing.contains(&under), "{drawing}");

    let problem = Problem::at(
        Kind::BrokenLink {
            target: "/a\u{202e}b/".into(),
            suggestion: Some("/c\u{2066}d/".into()),
            inside: false,
        },
        place,
    );
    let drawing = drawn(&sources, &problem);
    assert!(drawing.contains("`/a<U+202E>b/`"), "{drawing}");
    assert!(drawing.contains("`/c<U+2066>d/`"), "{drawing}");
    assert!(
        !drawing.contains(['\u{202e}', '\u{2066}', '\u{200b}']),
        "{drawing}"
    );
}

#[test]
fn hidden_characters_are_named_but_spelling_characters_kept() {
    for hidden in ['\u{ad}', '\u{3164}', '\u{e0041}', '\u{2066}', '\u{180e}'] {
        let text = format!("title: Post{hidden} 1\n");
        let (sources, place) = one(&text, "Post");
        let drawing = drawn(&sources, &Problem::at(broken_link(), place));
        assert!(!drawing.contains(hidden), "{hidden:?} in {drawing}");
        assert!(
            drawing.contains(&format!("<U+{:04X}>", hidden as u32)),
            "{drawing}"
        );
    }
    // Persian spells with the zero-width non-joiner; emoji join with the joiner
    // and choose their glyph with a variation selector.
    for kept in [
        "\u{645}\u{6cc}\u{200c}\u{62e}\u{648}\u{627}\u{647}\u{645}",
        "\u{1f469}\u{200d}\u{1f4bb}",
        "\u{2764}\u{fe0f}",
    ] {
        let text = format!("title: {kept}\n");
        let (sources, place) = one(&text, kept);
        let drawing = drawn(&sources, &Problem::at(broken_link(), place));
        assert!(drawing.contains(kept), "{kept:?} in {drawing}");
    }
}

#[test]
fn a_byte_order_mark_is_not_shown() {
    let (sources, place) = one("\u{feff}title: Post 1\n", "Post 1");
    let problem = Problem::at(broken_link(), place);
    assert!(drawn(&sources, &problem).contains("   1 │ title: Post 1\n     │        ^^^^^^\n"));
}

#[test]
fn colour_adds_nothing_the_text_does_not_say() {
    let (sources, place) = one("Read more [about us](/about-us/).\n", "/about-us/");
    let problem = Problem::at(broken_link(), place);
    let coloured = Report::new(&sources, Path::new(SITE), true).problem(&problem);
    assert!(coloured.contains('\x1b'));
    let mut plain = String::new();
    let mut rest = coloured.as_str();
    while let Some(i) = rest.find('\x1b') {
        plain.push_str(&rest[..i]);
        rest = &rest[i + rest[i..].find('m').unwrap() + 1..];
    }
    plain.push_str(rest);
    // The severity's badge has a space either side of its word, which the
    // colour makes part of the badge.
    let drawn = drawn(&sources, &problem);
    let badged = drawn.replacen("warning ", " warning  ", 1);
    assert_eq!(plain, badged);
}

#[test]
fn a_run_opens_with_statacs_name_only_where_there_is_colour() {
    assert_eq!(badge(false, "build"), "");
    let opened = badge(true, "build");
    assert!(opened.contains(" Statac \x1b[0m \x1b[1mbuild\x1b[0m\n\n"));
    assert!(opened.starts_with("\x1b[1;30;48;"), "{opened:?}");
}

#[test]
fn a_sentence_comes_apart_into_its_names_and_the_words_round_them() {
    use Piece::{Name, Words};
    assert_eq!(
        pieces("No `a` in `b`?"),
        [
            Words("No "),
            Name("a"),
            Words(" in "),
            Name("b"),
            Words("?")
        ]
    );
    assert_eq!(pieces("`a`"), [Name("a")]);
    assert_eq!(
        pieces("`a` and ` alone"),
        [Name("a"), Words(" and ` alone")]
    );
    assert_eq!(pieces("It ended on `"), [Words("It ended on `")]);
    assert_eq!(
        pieces("Empty: ``."),
        [Words("Empty: "), Name(""), Words(".")]
    );
    assert_eq!(pieces(""), []);
}

#[test]
fn a_name_in_backticks_is_picked_out_and_a_lone_backtick_is_left() {
    let paint = Paint {
        on: true,
        every: true,
    };
    let named = "\x1b[38;2;215;95;0m";
    assert_eq!(
        paint.inline(BOLD, "No `a` in `b`?"),
        format!(
            "\x1b[1mNo \x1b[0m{named}`a`\x1b[0m\x1b[1m in \x1b[0m{named}`b`\x1b[0m\x1b[1m?\x1b[0m"
        )
    );
    assert_eq!(paint.inline("", "It ended on `"), "It ended on `");
    assert_eq!(
        paint.inline("", "`a` and ` alone"),
        format!("{named}`a`\x1b[0m and ` alone")
    );
    let plain = Paint {
        on: false,
        every: true,
    };
    assert_eq!(plain.inline(BOLD, "No `a`"), "No `a`");
    assert_eq!(plain.chip(Severity::Error, "error"), "error");
    assert_eq!(
        plain.counted("1 warning: 1 broken-link"),
        "1 warning: 1 broken-link"
    );
    assert_eq!(
        paint.counted("1 warning: 1 broken-link"),
        "\x1b[1;33m1 warning\x1b[0m: 1 broken-link"
    );
}

fn quiet(names: &[&str]) -> (Quiet, Vec<Problem>) {
    let names: Vec<_> = names
        .iter()
        .map(|name| Sourced::new(name.to_string(), Origin::defaulted()))
        .collect();
    Quiet::new(&names)
}

/// Three warnings and an error, in two files.
fn several() -> (Sources, Vec<Problem>) {
    let mut sources = Sources::default();
    let two = sources.add(
        "site/content/post-2.md",
        "[Post 1](/post-1/)\n[Post 3](/post-3/)\n".into(),
    );
    let one = sources.add("site/content/post-1.md", "title: 1\ntitle: 2\n".into());
    let link = |target: &str| Kind::BrokenLink {
        target: target.into(),
        suggestion: None,
        inside: false,
    };
    let problems = vec![
        Problem::at(link("/post-3/"), at(two, 28, 36)),
        Problem::at(
            Kind::DuplicateKey {
                key: "title".into(),
                first: at(one, 0, 5),
            },
            at(one, 9, 14),
        ),
        Problem::at(link("/post-1/"), at(two, 9, 17)),
        Problem::nowhere(link("/post-4/")),
    ];
    (sources, problems)
}

#[test]
fn build_draws_errors_and_counts_the_rest() {
    let (sources, problems) = several();
    let report = Report::new(&sources, Path::new(SITE), false);
    assert_eq!(
        report.build(&problems, &Quiet::default()),
        lines(
            "
            |error  Duplicate key: `title`
            |  content/post-1.md, line 2
            |   2 │ title: 2
            |     │ ^^^^^
            |  First written: content/post-1.md, line 1
            |   1 │ title: 1
            |     │ ^^^^^
            |  More: https://statac.dev/e/duplicate-key/
            |
            |3 warnings: 3 broken-link. Run `statac check` to see them.
            "
        )
    );
}

#[test]
fn build_counts_quietened_kinds_apart() {
    let (sources, problems) = several();
    let report = Report::new(&sources, Path::new(SITE), false);
    let (quiet, unquietened) = quiet(&["broken-link"]);
    assert!(unquietened.is_empty());
    let warnings: Vec<_> = problems
        .into_iter()
        .filter(|p| p.kind.severity() == Severity::Warning)
        .collect();
    assert_eq!(
        report.build(&warnings, &quiet),
        "3 warnings, all quietened.\n"
    );
    assert_eq!(
        report.build(&warnings[..1], &Quiet::default()),
        "1 warning: 1 broken-link. Run `statac check` to see it.\n"
    );
    assert_eq!(report.build(&[], &quiet), "");
}

#[test]
fn check_draws_everything_in_order_of_file_and_place() {
    let (sources, problems) = several();
    let report = Report::new(&sources, Path::new(SITE), false);
    let said = report.check(&problems, &Quiet::default());
    let headlines: Vec<_> = said
        .lines()
        .filter(|line| line.starts_with(['e', 'w']))
        .collect();
    assert_eq!(
        headlines,
        [
            "warning  Broken link: nothing is published at `/post-4/`",
            "error  Duplicate key: `title`",
            "warning  Broken link: nothing is published at `/post-1/`",
            "warning  Broken link: nothing is published at `/post-3/`",
        ]
    );
    assert!(said.ends_with("\n\n1 error: 1 duplicate-key.\n3 warnings: 3 broken-link.\n"));
}

#[test]
fn check_lists_the_javascript_between_the_problems_and_the_count() {
    let (sources, problems) = several();
    let report = Report::new(&sources, Path::new(SITE), false);
    let javascript = [crate::build::Javascript {
        carried: crate::build::Carried::Written("templates/page.html".to_owned()),
        sure: true,
        pages: 2,
    }];
    let said = report.check_with(&problems, &Quiet::default(), &javascript);
    assert!(
        said.contains(
            "\n\nJavaScript the site carries:\n  Written in `templates/page.html`, on 2 pages\n\n\
             1 error: 1 duplicate-key.\n"
        ),
        "{said}"
    );
    assert_eq!(
        report.check_with(&[], &Quiet::default(), &javascript),
        "JavaScript the site carries:\n  Written in `templates/page.html`, on 2 pages\n\n\
         No problems.\n"
    );
    assert_eq!(
        report.check_with(&problems, &Quiet::default(), &[]),
        report.check(&problems, &Quiet::default())
    );
}

#[test]
fn check_leaves_quietened_kinds_out_and_counts_them() {
    let (sources, problems) = several();
    let report = Report::new(&sources, Path::new(SITE), false);
    let (quiet, _) = quiet(&["broken-link"]);
    let said = report.check(&problems, &quiet);
    assert!(!said.contains("Broken link"));
    assert!(said.ends_with("\n\n1 error: 1 duplicate-key.\n3 warnings, all quietened.\n"));
    assert_eq!(report.check(&[], &quiet), "No problems.\n");
}

#[test]
fn quiet_names_are_checked() {
    let (quiet, problems) = quiet(&["broken-link", "brokn-link", "duplicate-key", "post-1"]);
    assert!(quiet.quietens(&broken_link()));
    let kinds: Vec<_> = problems.into_iter().map(|p| p.kind).collect();
    assert_eq!(
        kinds,
        [
            Kind::UnknownProblemName {
                written: "brokn-link".into(),
                suggestion: Some("broken-link".into()),
            },
            Kind::CantQuieten {
                name: "duplicate-key".into(),
                because: Unquietenable::Error,
            },
            Kind::UnknownProblemName {
                written: "post-1".into(),
                suggestion: None,
            },
        ]
    );
}

#[test]
fn a_quiet_name_is_placed_where_it_was_written() {
    let text = "quiet: [brokn-link]\n";
    let (sources, place) = one(text, "brokn-link");
    let names = [Sourced::new(
        "brokn-link".to_string(),
        Origin::written(place, Style::Plain),
    )];
    let (_, problems) = Quiet::new(&names);
    assert_eq!(
        drawn(&sources, &problems[0]),
        lines(
            "
            |error  Unknown problem name: `brokn-link`
            |  content/post-1.md, line 1
            |   1 │ quiet: [brokn-link]
            |     │         ^^^^^^^^^^
            |  Did you mean `broken-link`?
            |  More: https://statac.dev/e/unknown-problem-name/
            "
        )
    );
}

#[test]
fn command_line_mistakes_are_drawn_without_a_place() {
    let problem = Problem::nowhere(Kind::UnknownOption {
        written: "--sit=docs".into(),
        hint: OptionHint::Similar("--site=docs".into()),
    });
    assert_eq!(
        drawn(&Sources::default(), &problem),
        lines(
            "
            |error  Unknown option: `--sit=docs`
            |  Did you mean `--site=docs`?
            |  More: https://statac.dev/e/unknown-option/
            "
        )
    );
}

#[test]
fn a_hidden_span_is_drawn_masked_whatever_the_problem() {
    let text = "title: Site 1\nwebmention_token: s3cr3t-t0ken\n";
    let (mut sources, token) = one(text, "s3cr3t-t0ken");
    sources.hide(token);
    let key = at(token.source, 14, 30);
    let duplicate = Kind::DuplicateKey {
        key: "webmention_token".into(),
        first: at(token.source, 0, 5),
    };
    for place in [token, key] {
        let out = drawn(&sources, &Problem::at(duplicate.clone(), place));
        assert!(!out.contains("s3cr3t"), "{out}");
        assert!(out.contains("2 │ webmention_token: •••\n"), "{out}");
    }
    // Carets under a secret cover the mask, not the secret's length.
    let out = drawn(&sources, &Problem::at(duplicate, token));
    assert!(out.contains("│                   ^^^\n"), "{out}");
}

/// The lines of a place's excerpt, each after its number, the place
/// between brackets on the line that has it, and where the place starts
/// and ends.
fn excerpt(sources: &Sources, place: Place) -> (Vec<String>, usize, usize, usize) {
    let excerpt = Report::new(sources, Path::new(SITE), false).excerpt(place, true);
    let lines = (excerpt.lines.iter())
        .map(|line| {
            let marked = line.marked.as_ref().map(|marked| format!("[{marked}]"));
            let marked = marked.unwrap_or_default();
            format!("{} {}{marked}{}", line.number, line.before, line.after)
        })
        .collect();
    (lines, excerpt.first, excerpt.last, excerpt.column)
}

#[test]
fn an_excerpt_is_the_line_the_terminal_draws_and_those_either_side() {
    let text = "title: Post 1\ntags:\tRust\nsummary: The first post.\n";
    let (sources, place) = one(text, "Rust");
    assert!(drawn(&sources, &Problem::at(broken_link(), place)).contains("2 │ tags:   Rust\n"));
    assert_eq!(
        excerpt(&sources, place),
        (
            vec![
                "1 title: Post 1".to_owned(),
                "2 tags:   [Rust]".to_owned(),
                "3 summary: The first post.".to_owned(),
            ],
            2,
            2,
            7
        )
    );
    // The first line has none before it, and the last none after.
    let (lines, ..) = excerpt(&sources, one(text, "title").1);
    assert_eq!(lines, ["1 [title]: Post 1", "2 tags:   Rust"]);
    let (lines, ..) = excerpt(&sources, one(text, "first").1);
    assert_eq!(lines, ["2 tags:   Rust", "3 summary: The [first] post."]);
}

#[test]
fn an_excerpt_masks_a_secret_on_any_of_its_lines() {
    let text = "title: Site 1\nwebmention_token: s3cr3t-t0ken\nlanguage: en\n";
    let (mut sources, token) = one(text, "s3cr3t-t0ken");
    sources.hide(token);
    let (lines, ..) = excerpt(&sources, token);
    assert_eq!(lines[1], "2 webmention_token: [•••]");
    let (lines, ..) = excerpt(&sources, one(text, "language").1);
    assert_eq!(lines, ["2 webmention_token: •••", "3 [language]: en"]);
}

#[test]
fn an_excerpt_marks_a_place_of_no_width_with_nothing() {
    let text = "title: Post 1\n";
    let (sources, place) = one(text, "Post");
    let place = at(
        place.source,
        place.span.start as usize,
        place.span.start as usize,
    );
    assert_eq!(
        excerpt(&sources, place),
        (vec!["1 title: []Post 1".to_owned()], 1, 1, 8)
    );
}

#[test]
fn an_excerpt_of_a_place_over_several_lines_marks_its_first_to_the_end() {
    let text = "title: Post 1\nsummary: !local |\n  one\n  two\n";
    let (sources, place) = one(text, "!local");
    let place = at(place.source, place.span.start as usize, text.len());
    assert_eq!(
        excerpt(&sources, place),
        (
            vec![
                "1 title: Post 1".to_owned(),
                "2 summary: [!local |]".to_owned(),
                "3   one".to_owned(),
            ],
            2,
            4,
            10
        )
    );
}

#[test]
fn an_excerpt_leaves_out_a_byte_order_mark() {
    let text = "\u{feff}title: Post 1\nsummary: The first post.\n";
    let (sources, place) = one(text, "Post 1");
    assert_eq!(
        excerpt(&sources, place),
        (
            vec![
                "1 title: [Post 1]".to_owned(),
                "2 summary: The first post.".to_owned(),
            ],
            1,
            1,
            8
        )
    );
    let (lines, ..) = excerpt(&sources, one(text, "summary").1);
    assert_eq!(lines[0], "1 title: Post 1");
}

#[test]
fn an_excerpt_of_a_long_line_cuts_the_lines_beside_it_to_the_same_columns() {
    let digits = |n: usize| {
        (0..n)
            .map(|i| char::from(b'0' + (i % 10) as u8))
            .collect::<String>()
    };
    let long = format!("{}[Post 2](/post-2/){}", "x".repeat(60), "y".repeat(80));
    let text = format!("{}\n{long}\n{}\n", digits(150), digits(20));
    let (sources, place) = one(&text, "/post-2/");
    let drawing = drawn(&sources, &Problem::at(broken_link(), place));
    let (lines, ..) = excerpt(&sources, place);
    // The marked line is the terminal's window, and the line before is cut
    // to its columns; the line after ends before them.
    let window = &digits(150)[45..125];
    assert_eq!(lines[0], format!("1 …{window}…"));
    let shown = format!(
        "2 …{}[Post 2]([/post-2/]){}…",
        "x".repeat(15),
        "y".repeat(47)
    );
    assert_eq!(lines[1], shown);
    assert!(
        drawing.contains(&format!(
            "2 │ …{}[Post 2](/post-2/){}…\n",
            "x".repeat(15),
            "y".repeat(47)
        )),
        "{drawing}"
    );
    assert_eq!(lines[2], "3 …");
}
