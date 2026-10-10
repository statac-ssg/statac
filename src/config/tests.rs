use super::*;
use crate::origin::{Arrival, Style};
use crate::problem::{Expected, Found, WrongPlace};

fn read_settings(text: &str) -> (Sources, Settings, Vec<Problem>) {
    let mut sources = Sources::default();
    let id = sources.add(FILE_NAME, text.to_owned());
    let (settings, problems) = read_source(&sources, id);
    (sources, settings, problems)
}

/// Each problem's kind and the text it is placed at.
fn found(text: &str) -> Vec<(Kind, String)> {
    let (sources, _, problems) = read_settings(text);
    problems
        .into_iter()
        .map(|p| {
            let at = p.place.map(|place| sources.spelling(place).to_owned());
            (p.kind, at.unwrap_or_default())
        })
        .collect()
}

#[test]
fn every_setting_the_sites_write_is_read() {
    let cases = [
        ("title", "./brooks.sh", "./brooks.sh"),
        ("title", "null program", "null program"),
        ("title", "1984", "1984"),
        ("url", "https://example.org", "https://example.org"),
        ("url", "http://localhost:8080/", "http://localhost:8080/"),
        ("language", "en-US", "en-US"),
        ("language", "fa-IR", "fa-IR"),
        (
            "timezone",
            "America/Argentina/Cordoba",
            "America/Argentina/Cordoba",
        ),
        ("date_format", "December 31, 2025", "December 31, 2025"),
        ("date_format", "\"2025-12-31\"", "2025-12-31"),
        ("code_style", "one-half", "one-half"),
        (
            "feed_key",
            "6b0f3c52-9e1d-8a47-b3c2-5d8e7f1a2c90",
            "6b0f3c52-9e1d-8a47-b3c2-5d8e7f1a2c90",
        ),
        (
            "image_sizes",
            "\"(max-width: 44rem) 100vw, 42rem\"",
            "(max-width: 44rem) 100vw, 42rem",
        ),
        (
            "after_build",
            "npx pagefind --site \"$STATAC_OUT\"",
            "npx pagefind --site \"$STATAC_OUT\"",
        ),
        ("before_build", "rm -f _site/a", "rm -f _site/a"),
        ("later_pages", "\"{list}{n}/\"", "{list}{n}/"),
        ("later_pages", "\"{list}page{n}/\"", "{list}page{n}/"),
        ("tags.permalink", "/tag/{slug}/", "/tag/{slug}/"),
        ("series.permalink", "/runs/{slug}/", "/runs/{slug}/"),
        ("feed_ids_from", "jekyll", "jekyll"),
    ];
    for (name, written, value) in cases {
        let (sources, settings, problems) = read_settings(&format!("{name}: {written}\n"));
        assert_eq!(problems, [], "{name}: {written}");
        let read = settings.text(name).unwrap();
        assert_eq!(read.value, value, "{name}");
        assert_eq!(read.origin.arrival, Arrival::Written);
        assert_eq!(sources.spelling(read.origin.place.unwrap()), written);
    }
    let (_, settings, problems) = read_settings(
        "typography: false\ndate_zeros: false\noptimise_images: false\ntags.pages: false\n\
         write_feed_ids: true\n\
         series.pages: false\nposts_per_page: 0\nmax_width: 1200\n\
         feed_ids_before: 2026-09-29\nnot_ours: [/en/]\nalso_at: [example.net]\n\
         after_build_makes: [pagefind/]\nleft_behind: [notes.md]\nquiet: [broken-link]\n\
         tags.slugs:\n  C++: cpp\n",
    );
    assert_eq!(problems, []);
    assert_eq!(settings.truth("typography"), Some(false));
    assert_eq!(settings.truth("date_zeros"), Some(false));
    assert_eq!(settings.truth("optimise_images"), Some(false));
    assert_eq!(settings.truth("write_feed_ids"), Some(true));
    assert_eq!(settings.truth("tags.pages"), Some(false));
    assert_eq!(settings.truth("series.pages"), Some(false));
    assert_eq!(settings.whole("posts_per_page"), Some(0));
    assert_eq!(settings.whole("max_width"), Some(1200));
    assert_eq!(
        settings.get("feed_ids_before").unwrap().value.as_date(),
        Some(&crate::date::Date::day_only(2026, 9, 29))
    );
    for (name, value) in [
        ("not_ours", "/en/"),
        ("also_at", "example.net"),
        ("after_build_makes", "pagefind/"),
        ("left_behind", "notes.md"),
        ("quiet", "broken-link"),
    ] {
        assert_eq!(settings.texts(name)[0].value, value, "{name}");
    }
    assert_eq!(settings.quiet()[0].value, "broken-link");
    let Value::Table(table) = &settings.get("tags.slugs").unwrap().value else {
        panic!("a table");
    };
    assert_eq!(
        (table[0].0.value.as_str(), table[0].1.value.as_str()),
        ("C++", "cpp")
    );
}

#[test]
fn a_setting_nobody_wrote_is_defaulted() {
    let (_, settings, problems) = read_settings("title: Site 1\n");
    assert_eq!(problems, []);
    let cases = [
        ("posts_per_page", Value::Whole(10)),
        ("max_width", Value::Whole(1600)),
        ("typography", Value::Bool(true)),
        ("date_zeros", Value::Bool(true)),
        ("optimise_images", Value::Bool(true)),
        ("write_feed_ids", Value::Bool(false)),
        ("tags.pages", Value::Bool(true)),
        ("code_style", Value::Text("ocean".into())),
        (
            "image_sizes",
            Value::Text("(max-width: 46rem) 100vw, 46rem".into()),
        ),
        ("tags.permalink", Value::Text("/tags/{slug}/".into())),
        ("series.permalink", Value::Text("/series/{slug}/".into())),
        ("later_pages", Value::Text("{list}page/{n}/".into())),
    ];
    for (name, value) in cases {
        let read = settings.get(name).unwrap();
        assert_eq!(read.value, value, "{name}");
        assert_eq!(read.origin, Origin::defaulted(), "{name}");
    }
    let left: Vec<&str> = settings
        .texts("left_behind")
        .iter()
        .map(|t| t.value.as_str())
        .collect();
    assert_eq!(left, [".DS_Store", "Thumbs.db", "desktop.ini"]);
    assert!(settings.get("language").is_none());
    assert!(settings.get("url").is_none());
}

#[test]
fn the_authors_own_values_nest_and_keep_their_places() {
    let (sources, settings, problems) = read_settings(
        "title: Site 1\nauthor: Author 1\nbook:\n  author: Author 2\n  isbn: '0000000001'\n",
    );
    assert_eq!(problems, []);
    let author = settings.own("author").unwrap();
    assert_eq!(sources.spelling(author.key), "author");
    assert_eq!(sources.spelling(author.value.place), "Author 1");
    let book = settings.own("book").unwrap();
    let isbn = book.value.get("isbn").unwrap();
    assert_eq!(isbn.style, Style::SingleQuoted);
    assert_eq!(sources.spelling(isbn.place), "'0000000001'");
    assert!(settings.own("title").is_none());
}

#[test]
fn pages_lines_are_handed_to_every_page() {
    let (sources, settings, problems) = read_settings(
        "pages.permalink: /{year}/{month}/{day}/{slug}/\npages.feed: false\npages.active: notes\n",
    );
    assert_eq!(problems, []);
    let permalink = settings.pages.text(words::PageWord::Permalink).unwrap();
    assert_eq!(permalink.value, "/{year}/{month}/{day}/{slug}/");
    assert_eq!(
        sources.spelling(permalink.origin.place.unwrap()),
        "/{year}/{month}/{day}/{slug}/"
    );
    assert_eq!(settings.pages.truth(words::PageWord::Feed), Some(false));
    assert_eq!(settings.pages.field("active").unwrap().name, "active");
}

#[test]
fn second_spellings_and_nested_settings_are_refused() {
    let other = |written: &str, ours: &str| Kind::OtherSpelling {
        word: Word::Setting,
        written: written.into(),
        ours: ours.into(),
    };
    let nested = |key: &str, under: &str, flat: &str| Kind::NestedSetting {
        key: key.into(),
        under: under.into(),
        flat: flat.into(),
    };
    let cases = [
        (
            "jpeg_max_width: 1200\n",
            vec![(other("jpeg_max_width", "max_width"), "jpeg_max_width")],
        ),
        (
            "pages.later: \"{list}{n}/\"\n",
            vec![(other("pages.later", "later_pages"), "pages.later")],
        ),
        (
            "default.layout: post\n",
            vec![(other("default.layout", "pages.layout"), "default.layout")],
        ),
        (
            "pages.url: /{slug}/\n",
            vec![(
                Kind::OtherSpelling {
                    word: Word::Field,
                    written: "pages.url".into(),
                    ours: "pages.permalink".into(),
                },
                "pages.url",
            )],
        ),
        (
            "pages.template: post\n",
            vec![(
                Kind::OtherSpelling {
                    word: Word::Field,
                    written: "pages.template".into(),
                    ours: "pages.layout".into(),
                },
                "pages.template",
            )],
        ),
        (
            "pages:\n  layout: post\n  feed: false\n",
            vec![
                (nested("layout", "pages", "pages.layout: post"), "layout"),
                (nested("feed", "pages", "pages.feed: false"), "feed"),
            ],
        ),
        (
            "tags:\n  permalink: /topics/{slug}/\n",
            vec![(
                nested("permalink", "tags", "tags.permalink: /topics/{slug}/"),
                "permalink",
            )],
        ),
        (
            "tags:\n  slugs:\n    C++: cpp\n",
            vec![(nested("slugs", "tags", "tags.slugs:"), "slugs")],
        ),
        (
            "fields:\n  isbn: text\n",
            vec![(nested("isbn", "fields", "fields.isbn: text"), "isbn")],
        ),
        (
            "fields:\n  book:\n    author: text\n",
            vec![(
                nested("author", "fields.book", "fields.book.author: text"),
                "author",
            )],
        ),
        (
            "fields.book:\n  author: text\n  isbn: required text\n",
            vec![
                (
                    nested("author", "fields.book", "fields.book.author: text"),
                    "author",
                ),
                (
                    nested("isbn", "fields.book", "fields.book.isbn: required text"),
                    "isbn",
                ),
            ],
        ),
        (
            "types:\n  review:\n    book:\n      rating: number\n",
            vec![(
                nested(
                    "rating",
                    "types.review.book",
                    "types.review.book.rating: number",
                ),
                "rating",
            )],
        ),
    ];
    for (text, expected) in cases {
        let expected: Vec<(Kind, String)> = expected
            .into_iter()
            .map(|(k, at)| (k, at.to_owned()))
            .collect();
        assert_eq!(found(text), expected, "{text}");
    }
}

#[test]
fn a_former_file_name_is_refused_by_name() {
    let root = std::env::temp_dir().join(format!("statac-config-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("statac.yml"), "title: Site 1\n").unwrap();
    let mut sources = Sources::default();
    let (settings, problems) = read(&root, &mut sources);
    std::fs::remove_dir_all(&root).unwrap();
    assert_eq!(
        problems,
        [Problem::nowhere(Kind::OtherSpelling {
            word: Word::File,
            written: "statac.yml".into(),
            ours: "statac.yaml".into(),
        })]
    );
    assert!(settings.get("title").is_none());
    assert_eq!(settings.whole("posts_per_page"), Some(10));
}

#[test]
fn reserved_and_unknown_names_are_refused() {
    let reserved = |name: &str| Kind::ReservedName { name: name.into() };
    let unknown = |written: &str, nearest: Option<&str>| Kind::UnknownWord {
        word: Word::Setting,
        written: written.into(),
        nearest: nearest.map(Into::into),
    };
    let cases = [
        ("posts: 12\n", reserved("site.posts"), "posts"),
        ("today: x\n", reserved("site.today"), "today"),
        ("data.people: x\n", reserved("site.data"), "data.people"),
        ("tags: [a]\n", reserved("site.tags"), "tags"),
        (
            "tags.permalnk: /t/{slug}/\n",
            unknown("tags.permalnk", Some("tags.permalink")),
            "tags.permalnk",
        ),
        (
            "series.colour: red\n",
            unknown("series.colour", None),
            "series.colour",
        ),
        (
            "types.isbn: text\n",
            unknown("types.isbn", None),
            "types.isbn",
        ),
    ];
    for (text, kind, at) in cases {
        assert_eq!(found(text), [(kind, at.to_owned())], "{text}");
    }
}

#[test]
fn settings_of_the_wrong_kind_are_refused_at_the_value() {
    let wrong =
        |field: &str, expected: Expected, found: &str, respelt: Option<&str>| Kind::WrongKind {
            field: field.into(),
            expected,
            found: Found::Written(found.into()),
            respelt: respelt.map(Into::into),
        };
    let styles = || {
        Expected::OneOf(
            ["ocean", "gruvbox", "solarized", "catppuccin", "one-half"]
                .map(String::from)
                .to_vec(),
        )
    };
    let cases = [
        (
            "url: example.org",
            wrong("url", Expected::SiteAddress, "example.org", None),
        ),
        (
            "language: english",
            wrong("language", Expected::LanguageCode, "english", None),
        ),
        (
            "code_style: dracula",
            wrong("code_style", styles(), "dracula", None),
        ),
        (
            "posts_per_page: -1",
            wrong(
                "posts_per_page",
                Expected::WholeNumber { from: 0 },
                "-1",
                None,
            ),
        ),
        (
            "max_width: 0",
            wrong("max_width", Expected::WholeNumber { from: 1 }, "0", None),
        ),
        (
            "typography: yes",
            wrong("typography", Expected::TrueFalse, "yes", Some("true")),
        ),
        (
            "tags.permalink: /tags/",
            wrong("tags.permalink", Expected::ListAddress, "/tags/", None),
        ),
        (
            "tags.permalink: /{slug}/",
            wrong("tags.permalink", Expected::ListAddress, "/{slug}/", None),
        ),
        (
            "later_pages: page/{n}/",
            wrong("later_pages", Expected::LaterAddress, "page/{n}/", None),
        ),
        (
            "later_pages: \"{list}page/\"",
            wrong(
                "later_pages",
                Expected::LaterAddress,
                "\"{list}page/\"",
                None,
            ),
        ),
        (
            "later_pages: \"{list}page:{n}/\"",
            wrong(
                "later_pages",
                Expected::LaterAddress,
                "\"{list}page:{n}/\"",
                None,
            ),
        ),
        (
            "title: 3.10",
            Kind::NeedsQuotes {
                written: "3.10".into(),
                reading: "3.1".into(),
            },
        ),
        (
            "feed_ids_before: 29/09/2026",
            wrong(
                "feed_ids_before",
                Expected::Date,
                "29/09/2026",
                Some("2026-09-29"),
            ),
        ),
    ];
    for (text, kind) in cases {
        let value = text.split_once(": ").unwrap().1;
        assert_eq!(found(text), [(kind, value.to_owned())], "{text}");
    }
}

#[test]
fn pages_lines_that_cant_be_handed_down_are_refused() {
    let place = |key: &str, why| Kind::WrongPlace {
        key: key.into(),
        why,
    };
    let cases = [
        (
            "pages.title: Post",
            place("pages.title", WrongPlace::EachPagesOwn),
        ),
        (
            "pages.date: 2026-10-04",
            place("pages.date", WrongPlace::EachPagesOwn),
        ),
        (
            "pages.redirect_from: /a/",
            place("pages.redirect_from", WrongPlace::EachPagesOwn),
        ),
        (
            "pages.index_page: false",
            place("pages.index_page", WrongPlace::WholeSite),
        ),
        (
            "pages.index_page: true",
            place("pages.index_page", WrongPlace::WholeSite),
        ),
        (
            "pages.timezone: UTC",
            place("pages.timezone", WrongPlace::SiteSetting),
        ),
        (
            "pages.tags.permalink: /t/{slug}/",
            place("pages.tags.permalink", WrongPlace::SiteSetting),
        ),
        (
            "pages.content: x",
            Kind::SetAside {
                field: "content".into(),
                why: crate::problem::SetAside::StatacFills(Root::Page),
            },
        ),
    ];
    for (text, kind) in cases {
        let key = text.split_once(": ").unwrap().0;
        assert_eq!(found(text), [(kind, key.to_owned())], "{text}");
    }
}

#[test]
fn fields_are_declared_and_checked() {
    let (sources, settings, problems) = read_settings(
        "fields.featured: true/false\nfields.seriesOrder: number\nfields.externalUrl: url\n\
         fields.categorie: one of toering, nederlandjes\ntypes.book.isbn: required text\n\
         fields.subfooters: list\nfields.started: date\nfields.isbn: text\n",
    );
    assert_eq!(problems, []);
    let declarations = &settings.declarations;
    assert_eq!(
        declarations.kind("featured"),
        Some(&declarations::FieldKind::TrueFalse)
    );
    let required: Vec<_> = declarations.required(Some("book")).collect();
    assert_eq!(required, [("isbn", Some("book"))]);
    assert_eq!(declarations.required(Some("post")).count(), 0);
    let check = |name: &str, written: &str| {
        let mut sources = Sources::default();
        let id = sources.add("content/post-1.md", format!("v: {written}\n"));
        let node = crate::yaml::read(&sources, id).unwrap();
        declarations
            .check(&sources, name, node.get("v").unwrap())
            .map(|p| p.kind.name())
    };
    let _ = sources;
    let cases = [
        ("featured", "true", None),
        ("featured", "yes", Some("wrong-kind-of-value")),
        ("seriesOrder", "9", None),
        ("seriesOrder", "2.5", None),
        ("seriesOrder", "nine", Some("wrong-kind-of-value")),
        ("externalUrl", "https://example.org/a/", None),
        ("externalUrl", "/about/", None),
        ("externalUrl", "example.org", Some("wrong-kind-of-value")),
        ("categorie", "toering", None),
        ("categorie", "dagelijks", Some("wrong-kind-of-value")),
        ("subfooters", "[a, b]", None),
        ("subfooters", "a", Some("wrong-kind-of-value")),
        ("started", "\"2016-08-07\"", None),
        ("started", "2016-02-30", Some("no-such-date")),
        ("isbn", "'0000000001'", None),
        ("isbn", "3.10", Some("needs-quotes")),
        ("undeclared", "3.10", None),
    ];
    for (name, written, problem) in cases {
        assert_eq!(check(name, written), problem, "{name}: {written}");
    }
}

#[test]
fn declarations_that_cant_be_read_are_refused() {
    let cases = [
        (
            "fields.isbn: txt\n",
            Kind::UnknownWord {
                word: Word::FieldKind,
                written: "txt".into(),
                nearest: Some("text".into()),
            },
            "txt",
        ),
        (
            "fields.more: yes/no\n",
            Kind::OtherSpelling {
                word: Word::FieldKind,
                written: "yes/no".into(),
                ours: "true/false".into(),
            },
            "yes/no",
        ),
        (
            "fields.title: text\n",
            Kind::ReservedName {
                name: "page.title".into(),
            },
            "fields.title",
        ),
        (
            "fields.title.main: text\n",
            Kind::ReservedName {
                name: "page.title".into(),
            },
            "fields.title.main",
        ),
        (
            "fields.isbn: [text]\n",
            Kind::WrongKind {
                field: "fields.isbn".into(),
                expected: Expected::FieldKind,
                found: Found::List,
                respelt: None,
            },
            "[text]",
        ),
    ];
    for (text, kind, at) in cases {
        assert_eq!(found(text), [(kind, at.to_owned())], "{text}");
    }
    let problems = found("fields.isbn: text\ntypes.book.isbn: number\n");
    assert!(
        matches!(&problems[..], [(Kind::DuplicateKey { key, .. }, at)] if key == "isbn" && at == "number")
    );
}

#[test]
fn a_field_written_flat_and_nested_is_warned_about() {
    let (sources, _, problems) =
        read_settings("book.author: Author 1\nbook:\n  isbn: '1'\n  author: Author 2\n");
    let [problem] = &problems[..] else {
        panic!("{problems:?}");
    };
    let Kind::FieldWrittenTwoWays {
        on,
        flat,
        nested_under,
        nested,
    } = &problem.kind
    else {
        panic!("{problem:?}");
    };
    assert_eq!(
        (*on, flat.as_str(), nested_under.as_str()),
        (Root::Site, "book.author", "book")
    );
    assert_eq!(sources.spelling(problem.place.unwrap()), "book.author");
    assert_eq!(sources.spelling(*nested), "author");
    assert_eq!(problem.kind.severity(), crate::problem::Severity::Warning);
}

#[test]
fn settings_that_arent_name_value_lines_are_refused() {
    let (_, _, problems) = read_settings("- title\n");
    assert_eq!(
        problems.into_iter().map(|p| p.kind).collect::<Vec<_>>(),
        [Kind::WrongKind {
            field: FILE_NAME.into(),
            expected: Expected::Fields,
            found: Found::List,
            respelt: None,
        }]
    );
    let (_, settings, problems) = read_settings("");
    assert_eq!(problems, []);
    assert_eq!(settings.whole("posts_per_page"), Some(10));
}

#[test]
fn a_token_in_the_settings_stops_and_is_never_quoted_or_shown() {
    for text in [
        "webmention_token: s3cr3t\n",
        "webmention_token: \"s3cr3t\"\n",
        "webmention_token: 0x1F2E\n",
        "webmention_token: [s3cr3t]\n",
    ] {
        let root = std::env::temp_dir().join(format!("statac-secret-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(FILE_NAME), text).unwrap();
        let mut sources = Sources::default();
        let (settings, problems) = read(&root, &mut sources);
        std::fs::remove_dir_all(&root).unwrap();
        assert_eq!(settings.secrets.len(), 1, "{text}");
        assert!(settings.get("webmention_token").is_none(), "{text}");
        assert!(settings.own("webmention_token").is_none(), "{text}");
        let [problem] = &problems[..] else {
            panic!("{text}: {problems:?}");
        };
        assert_eq!(problem.kind, Kind::TokenInSettings, "{text}");
        assert_eq!(sources.spelling(problem.place.unwrap()), "webmention_token");
        let drawn = crate::report::Report::new(&sources, &root, false).problem(problem);
        for secret in ["s3cr3t", "0x1F2E"] {
            assert!(!drawn.contains(secret), "{drawn}");
        }
        assert!(drawn.contains("webmention_token: •••"), "{drawn}");
    }
    // A secret written where it can't be read is still hidden.
    let (sources, settings, _) = read_settings("pages.webmention_token: abc\n");
    assert_eq!(sources.spelling(settings.secrets[0]), "abc");
}

#[test]
fn webmentions_are_off_unless_turned_on() {
    let (_, settings, problems) = read_settings("title: Site 1\n");
    assert_eq!(problems, []);
    assert_eq!(settings.truth("webmentions"), Some(false));
    let (_, settings, problems) = read_settings("webmentions: true\n");
    assert_eq!(problems, []);
    assert_eq!(settings.truth("webmentions"), Some(true));
}

#[test]
fn a_secret_on_a_line_yaml_cant_read_is_still_hidden() {
    let text = "title: Site 1\nwebmention_token: \"abc123\n";
    let root = std::env::temp_dir().join(format!("statac-secret-bad-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join(FILE_NAME), text).unwrap();
    let mut sources = Sources::default();
    let (_, problems) = read(&root, &mut sources);
    std::fs::remove_dir_all(&root).unwrap();
    assert!(!problems.is_empty());
    let report = crate::report::Report::new(&sources, &root, false);
    for problem in &problems {
        let drawn = report.problem(problem);
        assert!(!drawn.contains("abc123"), "{drawn}");
    }
}

#[test]
fn a_dotted_declaration_types_a_nested_field() {
    let (_, settings, problems) = read_settings(
        "fields.book.author: text\nfields.book.published: date\ntypes.review.book.rating: required number\n",
    );
    assert_eq!(problems, []);
    let declarations = &settings.declarations;
    assert!(declarations.nests("book"));
    assert!(!declarations.nests("boo"));
    assert!(!declarations.nests("book.author"));
    let required: Vec<_> = declarations.required(Some("review")).collect();
    assert_eq!(required, [("book.rating", Some("review"))]);
    let check = |name: &str, written: &str| {
        let mut sources = Sources::default();
        let id = sources.add("content/post-1.md", format!("v: {written}\n"));
        let node = crate::yaml::read(&sources, id).unwrap();
        declarations
            .check_all(&sources, name, node.get("v").unwrap())
            .into_iter()
            .map(|p| (p.kind.name(), sources.spelling(p.place.unwrap()).to_owned()))
            .collect::<Vec<_>>()
    };
    type Told<'a> = &'a [(&'a str, &'a str)];
    let cases: [(&str, &str, Told); 6] = [
        ("book", "{author: Author 1, published: 2026-10-04}", &[]),
        ("book", "{author: 3.10}", &[("needs-quotes", "3.10")]),
        (
            "book",
            "{author: [a], published: soon}",
            &[
                ("wrong-kind-of-value", "[a]"),
                ("wrong-kind-of-value", "soon"),
            ],
        ),
        ("book", "{other: 3.10}", &[]),
        ("book.author", "3.10", &[("needs-quotes", "3.10")]),
        ("author", "3.10", &[]),
    ];
    for (name, written, expected) in cases {
        let expected: Vec<_> = expected
            .iter()
            .map(|(kind, at)| (*kind, (*at).to_owned()))
            .collect();
        assert_eq!(check(name, written), expected, "{name}: {written}");
    }
}

#[test]
fn language_takes_one_tag_or_a_list_the_first_the_main_one() {
    let tags = |text: &str| {
        let (_, settings, problems) = read_settings(text);
        assert_eq!(problems, [], "{text}");
        settings
            .languages()
            .iter()
            .map(|tag| tag.value.to_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(tags("language: en-GB\n"), ["en-GB"]);
    assert_eq!(tags("language: [ar, en]\n"), ["ar", "en"]);
    assert_eq!(tags("language:\n  - ar\n  - en\n"), ["ar", "en"]);
    assert_eq!(tags("language: [en]\n"), ["en"]);
    assert!(tags("title: Site 1\n").is_empty());
}

#[test]
fn a_language_written_twice_stops_at_the_second_whatever_its_capitals() {
    let (sources, settings, problems) = read_settings("language: [en, ar, EN]\n");
    let first = problems[0].related()[0].place.unwrap();
    assert_eq!(sources.spelling(first), "en");
    let told: Vec<_> = problems
        .iter()
        .map(|p| {
            (
                p.kind.clone(),
                sources.spelling(p.place.unwrap()).to_owned(),
            )
        })
        .collect();
    assert_eq!(
        told,
        [(
            Kind::LanguageWrittenTwice {
                written: "EN".into(),
                first_written: "en".into(),
                first,
            },
            "EN".to_owned()
        )]
    );
    let tags: Vec<_> = settings.languages().iter().map(|t| t.value).collect();
    assert_eq!(tags, ["en", "ar"]);
    assert_eq!(
        found("language: [en, english]\n"),
        [(
            Kind::WrongKind {
                field: "language".into(),
                expected: Expected::LanguageCode,
                found: Found::Written("english".into()),
                respelt: None,
            },
            "english".to_owned()
        )]
    );
}

#[test]
fn a_setting_readers_see_is_written_for_one_language_flat() {
    let (sources, settings, problems) = read_settings(
        "language: [ar, en]\ntitle: Site 1\nen.title: Site 2\nen.date_format: Dec 31, 2025\n",
    );
    assert_eq!(problems, []);
    let title = |tag| settings.text_for(tag, "title").unwrap().value;
    assert_eq!(title(Some("en")), "Site 2");
    assert_eq!(title(Some("ar")), "Site 1");
    assert_eq!(title(None), "Site 1");
    let format = settings.for_language("en", "date_format").unwrap();
    assert_eq!(
        sources.spelling(format.origin.place.unwrap()),
        "Dec 31, 2025"
    );
    assert!(settings.own("en").is_none());
    assert_eq!(
        found("language: [ar, en]\nen:\n  title: Site 2\n"),
        [(
            Kind::NestedSetting {
                key: "title".into(),
                under: "en".into(),
                flat: "en.title: Site 2".into(),
            },
            "title".to_owned()
        )]
    );
}

#[test]
fn a_setting_for_a_language_not_declared_or_that_no_language_has_is_unknown() {
    let unknown = |written: &str, nearest: Option<&str>| Kind::UnknownWord {
        word: Word::Setting,
        written: written.into(),
        nearest: nearest.map(Into::into),
    };
    let cases = [
        ("fr.title: Site 1", unknown("fr.title", None)),
        ("EN.title: Site 1", unknown("EN.title", Some("en.title"))),
        ("en.timezone: UTC", unknown("en.timezone", None)),
        ("en.titel: Site 1", unknown("en.titel", Some("en.title"))),
    ];
    for (line, kind) in cases {
        let at = line.split(':').next().unwrap().to_owned();
        assert_eq!(
            found(&format!("language: [ar, en]\n{line}\n")),
            [(kind, at)],
            "{line}"
        );
    }
    // A name no language goes by is the author's own.
    let (_, settings, problems) = read_settings("language: [ar, en]\nog.title: Site 1\n");
    assert_eq!(problems, []);
    assert!(settings.own("og.title").is_some());
}

#[test]
fn a_site_in_one_language_writes_no_setting_for_one() {
    let (_, settings, problems) =
        read_settings("language: en\nen.title: Site 1\nen:\n  date_format: soon\n");
    assert_eq!(problems, []);
    assert!(settings.own("en.title").is_some());
    assert!(settings.own("en").is_some());
    assert!(settings.for_language("en", "title").is_none());
}

#[test]
fn site_languages_is_statacs_own() {
    assert_eq!(
        found("languages: [en]\n"),
        [(
            Kind::ReservedName {
                name: "site.languages".into()
            },
            "languages".to_owned()
        )]
    );
}

#[test]
fn a_name_one_slip_from_a_setting_is_told_and_kept_as_the_sites_own() {
    let cases = [
        ("posts_per_pge", Some("posts_per_page")),
        ("titel", Some("title")),
        ("Timezone", Some("timezone")),
        ("time_zone", Some("timezone")),
        ("urls", Some("url")),
        ("og.titel", None),
        ("en.title", None),
        ("tags.permalnk", None),
        ("posts_per_pg_", None),
        ("uri", None),
        ("author", None),
    ];
    for (name, nearest) in cases {
        let text = format!("{name}: 5\n");
        let told: Vec<_> = found(&text)
            .into_iter()
            .filter(|(kind, _)| kind.name() == "read-as-your-own")
            .collect();
        let expected = nearest.map(|nearest| {
            let kind = Kind::ReadAsYourOwn {
                own: Own::Setting,
                written: name.into(),
                nearest: nearest.into(),
            };
            (kind, name.to_owned())
        });
        assert_eq!(told, Vec::from_iter(expected), "{name}");
    }
    let (_, settings, _) = read_settings("posts_per_pge: 5\n");
    assert!(settings.own("posts_per_pge").is_some());
    assert_eq!(settings.whole("posts_per_page"), Some(10));
}

#[test]
fn a_setting_with_nothing_after_its_colon_is_set_aside_at_its_name() {
    let set_aside = |name: &str| {
        let kind = Kind::SetAside {
            field: name.into(),
            why: crate::problem::SetAside::NothingWritten(Word::Setting),
        };
        (kind, name.to_owned())
    };
    for name in ["url", "posts_per_page", "quiet", "tags.pages", "language"] {
        assert_eq!(found(&format!("{name}:\n")), [set_aside(name)], "{name}");
    }
    assert_eq!(
        found("language: [en, fr]\nen.title:\n"),
        [set_aside("en.title")]
    );
    for text in ["url: ~\n", "url: null\n", "author:\n", "og.title:\n"] {
        assert_eq!(found(text), [], "{text}");
    }
}

#[test]
fn an_address_given_for_the_run_is_the_sites_address_and_came_from_the_command_line() {
    let cases = [
        (
            "url: https://example.org/blog/\n",
            "https://preview.example.org/",
            "",
            Some("https://example.org/blog/"),
        ),
        (
            "url: https://example.org\n",
            "https://example.org/preview/42/",
            "/preview/42",
            Some("https://example.org"),
        ),
        (
            "title: Site\n",
            "https://preview.example.org/42",
            "/42",
            Some("https://preview.example.org/42"),
        ),
    ];
    for (written, given, path, home) in cases {
        let (_, mut settings, problems) = read_settings(written);
        assert_eq!(problems, [], "{written}");
        assert!(!settings.elsewhere(), "{written}");
        let own = settings.text("url").map(|url| url.value.to_owned());
        assert_eq!(settings.home(), own.as_deref(), "{written}");

        settings.at(given);
        let url = settings.text("url").unwrap();
        assert_eq!(url.value, given, "{written}");
        assert_eq!(url.origin.arrival, Arrival::CommandLine, "{written}");
        assert_eq!(url.origin.place, None, "{written}");
        assert_eq!(settings.base(), path, "{written}");
        assert_eq!(settings.home(), home, "{written}");
        assert!(settings.elsewhere(), "{written}");
    }
}
