use super::*;
use crate::origin::Arrival;
use crate::problem::{Code, Expected, Found, Named, Reason, SetAside, Word, WrongPlace};
use crate::testing::TempSite;

/// A run on the day given, so posts dated after it wait.
fn run_on(year: i32, month: u8, day: u8) -> Run {
    Run {
        today: Some(Date::day_only(year, month, day)),
        drafts: false,
        url: None,
        renamed: Vec::new(),
    }
}

/// The site `temp` holds, read.
fn read_site(temp: &TempSite) -> Read {
    read(&temp.root, &run_on(2026, 12, 31))
}

fn site(files: &[(&str, &str)]) -> Read {
    read_site(&TempSite::site(files))
}

fn page<'a>(read: &'a Read, path: &str) -> &'a Page {
    read.site
        .pages
        .iter()
        .map(|(_, page)| page)
        .find(|page| page.path == path)
        .unwrap_or_else(|| panic!("no page {path}"))
}

/// Each problem's name, the file it is in and the text it is at.
fn problems(read: &Read) -> Vec<(&'static str, String, String)> {
    read.problems
        .iter()
        .map(|p| {
            let (file, at) = match p.place {
                Some(place) => (
                    read.sources.path(place.source).display().to_string(),
                    read.sources.spelling(place).to_owned(),
                ),
                None => (String::new(), String::new()),
            };
            (p.kind.name(), file, at)
        })
        .collect()
}

/// Each problem and the text it is at.
fn kinds_at(read: &Read) -> Vec<(Kind, String)> {
    (read.problems.iter())
        .map(|p| {
            let at = p.place.map(|place| read.sources.spelling(place).to_owned());
            (p.kind.clone(), at.unwrap_or_default())
        })
        .collect()
}

fn addresses(read: &Read) -> Vec<(String, String)> {
    let mut found: Vec<_> = read
        .site
        .pages
        .iter()
        .map(|(_, page)| (page.path.clone(), page.address.as_str().to_owned()))
        .collect();
    found.sort();
    found
}

/// Where a setting came from, times how it was written: every value says
/// the file and the text it was written as, its style, and how it arrived.
#[test]
fn where_a_setting_came_from_times_how_it_was_written() {
    use crate::origin::Style;
    // (word, how it is written, the value read, the style)
    let written_as = [
        ("layout", "post", "post", Style::Plain),
        ("layout", "'post'", "post", Style::SingleQuoted),
        ("layout", "\"post\"", "post", Style::DoubleQuoted),
        ("layout", "|-\n    post", "post", Style::Literal),
        ("layout", ">-\n    post", "post", Style::Folded),
        ("layout", "404", "404", Style::Plain),
        ("tags", "[Tag 1, 'Tag 2']", "Tag 1, Tag 2", Style::None),
        (
            "tags",
            "\n    - Tag 1\n    - \"Tag 2\"",
            "Tag 1, Tag 2",
            Style::None,
        ),
        ("tags", "Tag 1", "Tag 1", Style::Plain),
    ];
    // (file the line is in, how it reaches `content/a/b/post-1.md`)
    let came_from = [
        ("content/a/b/post-1.md", Arrival::Written),
        ("content/a/b/index.md", Arrival::HandedDown),
        ("content/a/index.md", Arrival::HandedDown),
        ("statac.yaml", Arrival::HandedDown),
    ];
    for (word, written, value, style) in written_as {
        for (at, (from, arrival)) in came_from.iter().enumerate() {
            let line = |here: usize, key: &str| match here == at {
                true => format!("{key}: {written}\n"),
                false => String::new(),
            };
            let handed = format!("pages.{word}");
            let files = [
                (
                    "statac.yaml",
                    format!("language: en\ntitle: Site 1\n{}", line(3, &handed)),
                ),
                (
                    "content/a/index.md",
                    format!("---\ntitle: A\n{}---\n", line(2, &handed)),
                ),
                (
                    "content/a/b/index.md",
                    format!("---\ntitle: B\n{}---\n", line(1, &handed)),
                ),
                (
                    "content/a/b/post-1.md",
                    format!("---\ntitle: Post 1\n{}---\nBody.\n", line(0, word)),
                ),
            ];
            let files: Vec<(&str, &str)> = files.iter().map(|(p, t)| (*p, t.as_str())).collect();
            let read = site(&files);
            let case = format!("{word}: {written} from {from}");
            assert_eq!(problems(&read), [], "{case}");
            let words = &page(&read, "a/b/post-1.md").words;
            let found = words.get(PageWord::named(word).unwrap()).unwrap();
            let read_as = match &found.value {
                Value::Text(text) => text.clone(),
                Value::Texts(texts) => {
                    let texts: Vec<&str> = texts.iter().map(|t| t.value.as_str()).collect();
                    texts.join(", ")
                }
                other => panic!("{case}: {other:?}"),
            };
            assert_eq!(read_as, value, "{case}");
            assert_eq!(found.origin.arrival, *arrival, "{case}");
            assert_eq!(found.origin.style, style, "{case}");
            let place = found.origin.place.unwrap();
            assert_eq!(
                read.sources.path(place.source).to_str(),
                Some(*from),
                "{case}"
            );
            assert_eq!(read.sources.spelling(place), written.trim_start(), "{case}");
        }
    }
}

#[test]
fn the_nearest_setting_wins_and_a_folders_page_is_handed_from_above() {
    let read = site(&[
        (
            "statac.yaml",
            "language: en\npages.layout: site\npages.feed: false\npages.active: site\n",
        ),
        (
            "content/a/index.md",
            "---\npages.layout: a\npages.active: a\n---\n",
        ),
        ("content/a/b/index.md", "---\npages.layout: b\n---\n"),
        ("content/a/b/post-1.md", "---\ntitle: Post 1\n---\n"),
        (
            "content/a/post-2.md",
            "---\ntitle: Post 2\nlayout: own\n---\n",
        ),
    ]);
    assert_eq!(problems(&read), []);
    let text = |path: &str, word| {
        let page = page(&read, path);
        let value = page.words.text(word).unwrap();
        let from = read
            .sources
            .path(value.origin.place.unwrap().source)
            .display()
            .to_string();
        (value.value.to_owned(), from)
    };
    let s = |a: &str, b: &str| (a.to_owned(), b.to_owned());
    assert_eq!(
        text("a/b/post-1.md", PageWord::Layout),
        s("b", "content/a/b/index.md")
    );
    assert_eq!(
        text("a/post-2.md", PageWord::Layout),
        s("own", "content/a/post-2.md")
    );
    // A folder's own page is handed what is above it, not its own lines.
    assert_eq!(
        text("a/b/index.md", PageWord::Layout),
        s("a", "content/a/index.md")
    );
    assert_eq!(
        text("a/index.md", PageWord::Layout),
        s("site", "statac.yaml")
    );
    let active = page(&read, "a/b/post-1.md").words.field("active").unwrap();
    assert_eq!(read.sources.spelling(active.value.place), "a");
    assert_eq!(active.origin.arrival, Arrival::HandedDown);
    let feed = page(&read, "a/b/post-1.md")
        .words
        .get(PageWord::Feed)
        .unwrap();
    assert_eq!(feed.value, Value::Bool(false));
    assert_eq!(
        read.sources.path(feed.origin.place.unwrap().source),
        Path::new("statac.yaml")
    );
}

/// `untitled` is handed down as any word is: a page's own line comes
/// first, then the nearest folder's. Anything but `true` or `false` is
/// refused where it is written.
#[test]
fn untitled_is_handed_down_and_refused_where_it_is_not_true_or_false() {
    let read = site(&[
        (
            "content/notes/index.md",
            "---\ntitle: Notes\npages.untitled: true\n---\n",
        ),
        ("content/notes/note-1.md", "---\nsummary: A note.\n---\n"),
        (
            "content/notes/note-2.md",
            "---\nsummary: A note.\nuntitled: false\n---\n",
        ),
        (
            "content/notes/more/index.md",
            "---\ntitle: More\npages.untitled: false\n---\n",
        ),
        (
            "content/notes/more/note-3.md",
            "---\nsummary: A note.\n---\n",
        ),
        ("content/post-1.md", "---\nsummary: A note.\n---\n"),
    ]);
    assert_eq!(problems(&read), []);
    let untitled = |path: &str| {
        let value = page(&read, path).words.get(PageWord::Untitled)?;
        Some((value.value.as_bool().unwrap(), value.origin.arrival.clone()))
    };
    assert_eq!(
        untitled("notes/note-1.md"),
        Some((true, Arrival::HandedDown))
    );
    assert_eq!(untitled("notes/note-2.md"), Some((false, Arrival::Written)));
    assert_eq!(
        untitled("notes/more/note-3.md"),
        Some((false, Arrival::HandedDown))
    );
    assert_eq!(untitled("notes/index.md"), None);
    assert_eq!(untitled("post-1.md"), None);

    let read = site(&[
        (
            "content/notes/index.md",
            "---\ntitle: Notes\npages.untitled: perhaps\n---\n",
        ),
        ("content/notes/note-1.md", "---\nsummary: A note.\n---\n"),
    ]);
    assert_eq!(
        problems(&read),
        [(
            "wrong-kind-of-value",
            "content/notes/index.md".to_owned(),
            "perhaps".to_owned()
        )]
    );
}

#[test]
fn front_matter_refusals_are_placed_at_what_was_written() {
    let cases: &[(&str, &str, &str)] = &[
        ("template: post", "other-spelling", "template"),
        ("aliases: [/a/]", "other-spelling", "aliases"),
        ("default.layout: post", "other-spelling", "default.layout"),
        ("pages.layout: post", "wrong-place", "pages.layout"),
        (
            "tags.permalink: /t/{slug}/",
            "wrong-place",
            "tags.permalink",
        ),
        ("site.author: Author 1", "wrong-place", "site.author"),
        ("source: a.txt", "set-aside", "source"),
        ("slug: post", "set-aside", "slug"),
        ("pages:\n  layout: post", "nested-setting", "layout"),
        ("draft: yes", "wrong-kind-of-value", "yes"),
        ("draft: perhaps", "wrong-kind-of-value", "perhaps"),
        ("untitled: yes", "wrong-kind-of-value", "yes"),
        ("untitled: perhaps", "wrong-kind-of-value", "perhaps"),
        ("title: 3.10", "needs-quotes", "3.10"),
        ("title: [a, b]", "wrong-kind-of-value", "[a, b]"),
        ("date: 3/4/2026", "unclear-date", "3/4/2026"),
        ("date: 2026-02-30", "no-such-date", "2026-02-30"),
        ("date: soon", "wrong-kind-of-value", "soon"),
        ("neighbours: all", "wrong-kind-of-value", "all"),
        ("neighbours: no", "wrong-kind-of-value", "no"),
        ("neighbours: true", "wrong-kind-of-value", "true"),
        ("posts_per_page: -1", "wrong-kind-of-value", "-1"),
        ("feed: yes", "wrong-kind-of-value", "yes"),
        ("needs: site", "wrong-kind-of-value", "site"),
        ("id: abc", "wrong-kind-of-value", "abc"),
        ("permalink: /{yaer}/", "unusable-address", "{yaer}"),
        ("permalink: /:year/", "other-spelling", ":year"),
        ("permalink: /a:b/", "unusable-address", "a:b"),
    ];
    for (line, name, at) in cases {
        let text = format!("---\nsummary: Post 1\n{line}\n---\n");
        let read = site(&[("content/post-1.md", &text)]);
        assert_eq!(
            problems(&read),
            [(*name, "content/post-1.md".to_owned(), (*at).to_owned())],
            "{line}"
        );
    }
}

/// Front matter written with no `---` lines is told at the file's first
/// line, and the page is built from the file as it is. A file opening
/// with words, a heading, a field that isn't one of Statac's or another
/// tool's front matter is never told.
#[test]
fn front_matter_with_no_lines_around_it_is_told_at_the_first_line() {
    let read = site(&[
        (
            "content/post-1.md",
            "title: Post 1\n\nThe first paragraph.\n",
        ),
        (
            "content/post-2.md",
            "title: Post 2\r\ndate: 2026-10-04\r\nsummary: One.\r\n",
        ),
        (
            "content/post-3.md",
            "tags:\n  - one\n- two\ntitle: Post 3\n\nThe first paragraph.\n",
        ),
        ("content/post-4.md", "\u{feff}title: Post 4\n"),
        ("content/post-5.md", "Note: the first paragraph.\n"),
        ("content/post-6.md", "# Post 6\n\ntitle: Post 6\n"),
        ("content/post-7.md", "mood: calm\ntitle: Post 7\n"),
        (
            "content/post-8.md",
            "---\ntitle: Post 8\n---\ntitle: Post 8\n",
        ),
        ("content/post-9.md", "title: Post 9\nThe first paragraph.\n"),
        ("content/post-10.md", "+++\ntitle = \"Post 10\"\n+++\n"),
        ("content/post-11.md", "Title: Post 11\n"),
    ]);
    let mut told: Vec<_> = problems(&read)
        .into_iter()
        .filter(|(name, ..)| *name == "published-as-text")
        .map(|(_, file, at)| (file, at))
        .collect();
    told.sort();
    let at = |file: &str, line: &str| (format!("content/{file}"), line.to_owned());
    assert_eq!(
        told,
        [
            at("post-1.md", "title: Post 1"),
            at("post-2.md", "title: Post 2"),
            at("post-3.md", "tags:"),
            at("post-4.md", "title: Post 4"),
        ]
    );
    let headline = (read.problems.iter())
        .find(|problem| problem.kind.code() == Code::PublishedAsText)
        .map(Problem::headline);
    assert_eq!(
        headline.as_deref(),
        Some("Published as text: `title: Post 1` isn't between `---` lines")
    );
    assert_eq!(page(&read, "post-1.md").title, None);
}

#[test]
fn a_key_one_slip_from_a_known_word_is_told_and_kept_as_written() {
    let settings = "language: en\nfields.subtitle: text\n";
    // (key, the word it is near)
    let cases: &[(&str, Option<&str>)] = &[
        ("titles", Some("title")),
        ("titl", Some("title")),
        ("titel", Some("title")),
        ("tilte", Some("title")),
        ("Title", Some("title")),
        ("subtitel", Some("subtitle")),
        ("tilet_", None),
        ("tag", None),
        ("subtitle", None),
        ("contents", None),
        ("author", None),
    ];
    for (key, nearest) in cases {
        let text = format!("---\nsummary: Post 1\n{key}: A\n---\n");
        let read = site(&[("statac.yaml", settings), ("content/post-1.md", &text)]);
        let told = kinds_at(&read);
        let expected: Vec<(Kind, String)> = (nearest.iter())
            .map(|nearest| {
                let kind = Kind::ReadAsYourOwn {
                    own: crate::problem::Own::Field((*key).into()),
                    written: (*key).into(),
                    nearest: (*nearest).into(),
                };
                (kind, (*key).to_owned())
            })
            .collect();
        assert_eq!(told, expected, "{key}");
        assert!(page(&read, "post-1.md").words.field(key).is_some(), "{key}");
    }
}

#[test]
fn a_handed_down_key_one_slip_from_a_word_is_told_where_it_is_written() {
    let read = site(&[
        ("statac.yaml", "language: en\npages.sumary: A\n"),
        ("content/a/index.md", "---\npages.layuot: post\n---\n"),
        ("content/a/post-1.md", "---\ntitle: Post 1\n---\n"),
    ]);
    let told = kinds_at(&read);
    let slip = |name: &str, nearest: &str| Kind::ReadAsYourOwn {
        own: crate::problem::Own::Field(name.into()),
        written: format!("pages.{name}"),
        nearest: format!("pages.{nearest}"),
    };
    assert!(
        told.contains(&(slip("sumary", "summary"), "pages.sumary".into())),
        "{told:?}"
    );
    assert!(told.contains(&(slip("layuot", "layout"), "pages.layuot".into())));
    assert_eq!(told.len(), 2, "{told:?}");
}

#[test]
fn a_type_one_slip_from_a_declared_type_is_told_at_its_value() {
    let settings = "language: en\ntypes.reply.in_reply_to: url\ntypes.note.mood: text\n";
    // (type, whether `templates/repyl.html` is there, told)
    let cases = [
        ("repyl", false, true),
        ("Reply", false, true),
        ("reply", false, false),
        ("note", false, false),
        ("repyl", true, false),
        ("replies", false, false),
        ("nte", false, false),
    ];
    for (written, template, told) in cases {
        let text = format!("---\ntitle: Post 1\ntype: {written}\n---\n");
        let mut files = vec![("statac.yaml", settings), ("content/post-1.md", &text)];
        if template {
            files.push(("templates/repyl.html", "{{ page.title }}"));
        }
        let read = site(&files);
        let expected = told.then(|| {
            (
                Kind::ReadAsYourOwn {
                    own: crate::problem::Own::Type,
                    written: written.into(),
                    nearest: "reply".into(),
                },
                written.to_owned(),
            )
        });
        assert_eq!(
            kinds_at(&read),
            Vec::from_iter(expected),
            "{written} {template}"
        );
    }
}

#[test]
fn a_type_handed_down_is_told_once_where_it_is_written() {
    let read = site(&[
        (
            "statac.yaml",
            "language: en\ntypes.reply.in_reply_to: url\n",
        ),
        ("content/a/index.md", "---\npages.type: repyl\n---\n"),
        ("content/a/post-1.md", "---\ntitle: Post 1\n---\n"),
        ("content/a/post-2.md", "---\ntitle: Post 2\n---\n"),
    ]);
    assert_eq!(
        problems(&read),
        [(
            "read-as-your-own",
            "content/a/index.md".to_owned(),
            "repyl".to_owned()
        )]
    );
}

#[test]
fn tags_on_one_line_with_commas_are_one_tag_and_told() {
    let one_tag = |line: &str, tag: &str, parts: &[&str]| Kind::OneTagNotTwo {
        key: "tags".into(),
        line: line.into(),
        tag: tag.into(),
        parts: parts.iter().map(|&part| part.into()).collect(),
    };
    // (value, told at it, the tags made)
    let cases: &[(&str, Option<Kind>, &[&str])] = &[
        (
            "hills, water",
            Some(one_tag(
                "tags: hills, water",
                "hills, water",
                &["hills", "water"],
            )),
            &["hills, water"],
        ),
        (
            "a, b, c",
            Some(one_tag("tags: a, b, c", "a, b, c", &["a", "b", "c"])),
            &["a, b, c"],
        ),
        ("\"hills, water\"", None, &["hills, water"]),
        ("[hills, water]", None, &["hills", "water"]),
        ("hills", None, &["hills"]),
        ("hills,", None, &["hills,"]),
    ];
    for (value, told, tags) in cases {
        let text = format!("---\ntitle: Post 1\ntags: {value}\n---\n");
        let read = site(&[("content/post-1.md", &text)]);
        let expected = told.iter().map(|kind| (kind.clone(), (*value).to_owned()));
        assert_eq!(kinds_at(&read), Vec::from_iter(expected), "{value}");
        let made = page(&read, "post-1.md").words.get(PageWord::Tags).unwrap();
        let made: Vec<&str> = (made.value.as_texts().unwrap().iter())
            .map(|tag| tag.value.as_str())
            .collect();
        assert_eq!(made, *tags, "{value}");
    }
}

#[test]
fn tags_handed_down_on_one_line_with_commas_are_told_where_written() {
    let read = site(&[
        ("statac.yaml", "language: en\npages.tags: a, b\n"),
        ("content/a/index.md", "---\npages.tags: c, d, e\n---\n"),
        ("content/a/post-1.md", "---\ntitle: Post 1\n---\n"),
    ]);
    let one_tag = |line: &str, tag: &str, parts: &[&str]| Kind::OneTagNotTwo {
        key: "pages.tags".into(),
        line: line.into(),
        tag: tag.into(),
        parts: parts.iter().map(|&part| part.into()).collect(),
    };
    let told = kinds_at(&read);
    assert!(
        told.contains(&(
            one_tag("pages.tags: a, b", "a, b", &["a", "b"]),
            "a, b".into()
        )),
        "{told:?}"
    );
    assert!(told.contains(&(
        one_tag("pages.tags: c, d, e", "c, d, e", &["c", "d", "e"]),
        "c, d, e".into()
    )));
    assert_eq!(told.len(), 2, "{told:?}");
}

#[test]
fn front_matter_between_plus_lines_is_refused_at_the_opening_line() {
    let toml = "+++\ntitle = 'Post 1'\n+++\nBody.\n";
    let read = site(&[
        ("content/post-1.md", toml),
        ("content/post-2.md", &format!("\u{feff}{toml}")),
        ("content/post-3.md", "Body.\n+++\ntitle = 'Post 3'\n+++\n"),
        (
            "content/page.html",
            "+++\ntitle = 'Page'\n+++\n<p>Page.</p>\n",
        ),
    ]);
    let refused = |path: &str| ("other-spelling", path.to_owned(), "+++".to_owned());
    assert_eq!(
        problems(&read),
        [refused("content/post-1.md"), refused("content/post-2.md")]
    );
    assert!(matches!(
        &read.problems[0].kind,
        Kind::OtherSpelling { word: Word::Marker, written, ours } if written == "+++" && ours == "---"
    ));
    let paths: Vec<String> = addresses(&read).into_iter().map(|(path, _)| path).collect();
    assert_eq!(paths, ["", "post-3.md"]);
}

#[test]
fn a_word_with_nothing_after_its_colon_is_set_aside_at_its_name() {
    let settings = "language: en\nfields.isbn: text\n";
    // One of Statac's words of each shape, and a declared field.
    let told = [
        "title",
        "date",
        "tags",
        "draft",
        "needs",
        "posts_per_page",
        "feed",
        "id",
        "permalink",
        "order",
        "isbn",
    ];
    for name in told {
        let text = format!("---\nsummary: Post 1\n{name}:\n---\n");
        let read = site(&[("statac.yaml", settings), ("content/post-1.md", &text)]);
        let kind = Kind::SetAside {
            field: name.into(),
            why: SetAside::NothingWritten(Word::Field),
        };
        assert_eq!(kinds_at(&read), [(kind, name.to_owned())], "{name}");
    }
    for line in ["mood:", "title: ~", "title: null", "isbn: ~", "title: ''"] {
        let text = format!("---\nsummary: Post 1\n{line}\n---\n");
        let read = site(&[("statac.yaml", settings), ("content/post-1.md", &text)]);
        assert_eq!(kinds_at(&read), [], "{line}");
    }
}

#[test]
fn a_page_made_from_items_with_an_empty_pattern_is_set_aside_at_its_name() {
    let read = site(&[
        (
            "content/links/index.md",
            "---\ntitle: Links\neach: data.lists.links\npermalink:\n---\n",
        ),
        ("data/lists.yaml", "links:\n  - title: Link 1\n"),
    ]);
    let kind = Kind::SetAside {
        field: "permalink".into(),
        why: SetAside::NothingWritten(Word::Field),
    };
    assert!(
        kinds_at(&read).contains(&(kind, "permalink".into())),
        "{:?}",
        kinds_at(&read)
    );
}

#[test]
fn a_word_handed_down_with_nothing_after_its_colon_is_set_aside_at_its_name() {
    let read = site(&[
        (
            "statac.yaml",
            "language: en\nfields.isbn: text\npages.isbn:\n",
        ),
        (
            "content/a/index.md",
            "---\npages.layout:\npages.mood:\n---\n",
        ),
        ("content/a/post-1.md", "---\ntitle: Post 1\n---\n"),
    ]);
    let set_aside = |field: &str| {
        let kind = Kind::SetAside {
            field: field.into(),
            why: SetAside::NothingWritten(Word::Field),
        };
        (kind, field.to_owned())
    };
    assert_eq!(
        kinds_at(&read),
        [set_aside("pages.isbn"), set_aside("pages.layout")]
    );
}

#[test]
fn what_a_folder_hands_down_is_checked_where_it_is_written() {
    let place = |key: &str, why| Kind::WrongPlace {
        key: key.into(),
        why,
    };
    let read = site(&[
        ("content/index.md", "---\npages.layout: post\n---\n"),
        (
            "content/a/index.md",
            "---\npages.title: A\npages.later: \"{list}{n}/\"\npages.permalink: /a/\npages.content: x\n---\n",
        ),
        ("content/a/post-1.md", "---\ntitle: Post 1\n---\n"),
        (
            "content/a/post-2.md",
            "---\ntitle: Post 2\npages.layout: x\npages.later: \"{list}{n}/\"\n---\n",
        ),
    ]);
    let kinds: Vec<Kind> = read.problems.iter().map(|p| p.kind.clone()).collect();
    assert!(
        kinds.contains(&place("pages.layout", WrongPlace::TopFolder)),
        "{kinds:?}"
    );
    assert!(kinds.contains(&place("pages.title", WrongPlace::EachPagesOwn)));
    let respelt = Kind::OtherSpelling {
        word: Word::Setting,
        written: "pages.later".into(),
        ours: "later_pages".into(),
    };
    assert_eq!(kinds.iter().filter(|kind| **kind == respelt).count(), 2);
    assert!(kinds.contains(&place("pages.layout", WrongPlace::NothingInside)));
    assert!(kinds.contains(&Kind::UnusableAddress {
        fault: AddressFault::FillsNothing {
            pattern: "/a/".into()
        }
    }));
    assert!(kinds.iter().any(|k| k.name() == "set-aside"));
    // A refused pattern is told once, at the line, however many pages it reached.
    assert_eq!(
        kinds
            .iter()
            .filter(|k| k.name() == "unusable-address")
            .count(),
        1
    );
}

#[test]
fn pages_are_found_with_their_titles_and_lists() {
    let read = site(&[
        ("statac.yaml", "language: en\ntimezone: UTC\n"),
        ("content/index.md", "---\ntitle: Home\n---\n"),
        ("content/about.markdown", "---\ntitle: About\n---\n"),
        ("content/page.html", "---\ntitle: Page\n---\n<p>Page.</p>\n"),
        ("content/late.html", "\n---\ntitle: Late\n---\n"),
        ("content/fragment.html", "<p>Not a page.</p>\n"),
        ("content/notes.md", "# Notes, ## and more ##\n\nBody.\n"),
        ("content/plain.md", "Body with no heading.\n"),
        ("content/.hidden.md", "---\ntitle: Hidden\n---\n"),
        ("content/.DS_Store", ""),
        (
            "content/posts/2024-01-02-post-1.md",
            "---\ntitle: Post 1\n---\n",
        ),
        (
            "content/posts/post-2.md",
            "---\ntitle: Post 2\ndraft: true\n---\n",
        ),
        (
            "content/posts/post-3/index.md",
            "---\ntitle: Post 3\n---\n![A](a.png)\n",
        ),
        ("content/posts/post-3/a.png", "png"),
        ("content/posts/post-3/more/b.png", "png"),
        ("content/files/c.pdf", "pdf"),
    ]);
    assert_eq!(
        problems(&read),
        [(
            "draft-left-out",
            "content/posts/post-2.md".to_owned(),
            "true".to_owned()
        )]
    );
    let title = |path: &str| page(&read, path).title.as_ref().map(|t| t.value.clone());
    assert_eq!(title("index.md").as_deref(), Some("Home"));
    assert_eq!(title("about.markdown").as_deref(), Some("About"));
    assert_eq!(title("page.html").as_deref(), Some("Page"));
    assert_eq!(title("notes.md").as_deref(), Some("Notes, ## and more"));
    assert_eq!(title("plain.md"), None);
    let notes = page(&read, "notes.md").title.clone().unwrap();
    assert_eq!(
        read.sources.spelling(notes.origin.place.unwrap()),
        "Notes, ## and more"
    );
    assert_eq!(
        addresses(&read),
        [
            // `posts/` has no `index.md`, so Statac makes its page.
            ("", "/posts/"),
            ("about.markdown", "/about/"),
            ("index.md", "/"),
            ("notes.md", "/notes/"),
            ("page.html", "/page/"),
            ("plain.md", "/plain/"),
            ("posts/2024-01-02-post-1.md", "/posts/post-1/"),
            ("posts/post-3/index.md", "/posts/post-3/"),
        ]
        .map(|(a, b)| (a.to_owned(), b.to_owned()))
    );
    assert_eq!(read.drafts.len(), 1);
    assert_eq!(
        read.sources.path(read.drafts[0]),
        Path::new("content/posts/post-2.md")
    );
    let files: Vec<(&str, &str)> = read
        .site
        .files
        .iter()
        .map(|f| (f.path.as_str(), f.address.as_str()))
        .collect();
    assert_eq!(
        files,
        [
            ("fragment.html", "/fragment.html"),
            ("late.html", "/late.html"),
            ("files/c.pdf", "/files/c.pdf"),
            ("posts/post-3/a.png", "/posts/post-3/a.png"),
            ("posts/post-3/more/b.png", "/posts/post-3/more/b.png"),
        ]
    );
    let lists: Vec<(&ListKey, Option<&str>, usize, usize)> = read
        .site
        .lists
        .iter()
        .map(|(_, list)| {
            let page = list.page.map(|id| read.site.pages.get(id).path.as_str());
            (&list.key, page, list.members.len(), list.undated.len())
        })
        .collect();
    assert_eq!(
        lists,
        [
            (&ListKey::Folder(String::new()), Some("index.md"), 1, 5),
            (&ListKey::Folder("posts".into()), Some(""), 1, 1),
        ]
    );
    let dated = page(&read, "posts/2024-01-02-post-1.md")
        .date
        .clone()
        .unwrap();
    assert_eq!(dated.value, Date::day_only(2024, 1, 2));
    assert_eq!(dated.origin.arrival, Arrival::FileName);
}

#[test]
fn a_bundles_files_follow_its_permalink() {
    let read = site(&[
        ("statac.yaml", "language: en\ntimezone: UTC\n"),
        (
            "content/notes/index.md",
            "---\npages.permalink: /n/{slug}.html\n---\n",
        ),
        (
            "content/notes/2022-10-18-post-1.md",
            "---\ntitle: Post 1\n---\n",
        ),
        (
            "content/notes/2022-10-19-post-2/index.md",
            "---\ntitle: Post 2\npermalink: /notes/2022-10-19-post-2/\n---\n",
        ),
        ("content/notes/2022-10-19-post-2/a.png", "png"),
        (
            "content/notes/2022-10-20-post-3/index.md",
            "---\ntitle: Post 3\n---\n",
        ),
        ("content/notes/2022-10-20-post-3/b.png", "png"),
    ]);
    assert_eq!(problems(&read), []);
    let files: Vec<&str> = read.site.files.iter().map(|f| f.address.as_str()).collect();
    assert_eq!(files, ["/notes/2022-10-19-post-2/a.png", "/n/b.png"]);
    assert_eq!(
        page(&read, "notes/2022-10-20-post-3/index.md")
            .address
            .as_str(),
        "/n/post-3.html"
    );
}

#[test]
fn every_piece_of_an_address_is_filled() {
    let read = site(&[
        (
            "statac.yaml",
            "language: en\npages.permalink: /{year}/{month}/{day}/{slug}/\ntimezone: UTC\n",
        ),
        ("content/2014-07-17-post-1.md", "---\ntitle: Post 1\n---\n"),
        (
            "content/post-2.md",
            "---\ntitle: Post 2\ndate: 2026-10-04T09:00:00+01:00\n---\n",
        ),
        (
            "content/Café Notes/Post Mòr.md",
            "---\ntitle: Post 3\ndate: 2026-01-02\npermalink: /{folder}/{name}/\n---\n",
        ),
        (
            "content/Café Notes/2026-01-03.md",
            "---\ntitle: Post 4\npermalink: /{folder}/{slug}/\n---\n",
        ),
        (
            "content/日記/привет.md",
            "---\ntitle: Post 5\ndate: 2026-01-04\npermalink: /{folder}/{slug}/\n---\n",
        ),
        (
            "content/404.html",
            "---\ntitle: Not found\npermalink: /404.html\n---\n",
        ),
        (
            "content/feed.md",
            "---\ntitle: Feed\nlayout: feed.xml\npermalink: /feed\n---\n",
        ),
        (
            "content/x.md",
            "---\ntitle: X\nlayout: page\npermalink: /x\n---\n",
        ),
    ]);
    assert_eq!(problems(&read), []);
    let at = |path: &str| {
        let page = page(&read, path);
        (page.address.as_str().to_owned(), page.address.is_file())
    };
    let s = |a: &str, file: bool| (a.to_owned(), file);
    assert_eq!(at("2014-07-17-post-1.md"), s("/2014/07/17/post-1/", false));
    assert_eq!(at("post-2.md"), s("/2026/10/04/post-2/", false));
    assert_eq!(
        at("Café Notes/Post Mòr.md"),
        s("/cafe-notes/Post Mòr/", false)
    );
    assert_eq!(
        at("Café Notes/2026-01-03.md"),
        s("/cafe-notes/2026-01-03/", false)
    );
    assert_eq!(at("日記/привет.md"), s("/日記/привет/", false));
    assert_eq!(at("404.html"), s("/404.html", true));
    assert_eq!(at("feed.md"), s("/feed", true));
    assert_eq!(at("x.md"), s("/x/", false));
}

#[test]
fn a_declared_field_fills_a_pages_pattern_as_it_fills_an_items() {
    let declared = "language: en\ntimezone: UTC\nfields.isbn: text\ntypes.book.edition: number\n";
    let read = site(&[
        ("statac.yaml", declared),
        (
            "content/a.md",
            "---\ntitle: A\nisbn: Number 1\npermalink: /books/{isbn}/\n---\n",
        ),
        (
            "content/b.md",
            "---\ntitle: B\ntype: book\nedition: 2\npermalink: /books/{edition}.html\n---\n",
        ),
        (
            "content/c/index.md",
            "---\npages.permalink: /{year}/{isbn}/\n---\n",
        ),
        (
            "content/c/d.md",
            "---\ntitle: D\ndate: 2026-01-01\nisbn: Number 4\n---\n",
        ),
        (
            "content/e.md",
            "---\neach: data.e\npermalink: /made/{isbn}/\n---\n",
        ),
        (
            "data/e.yaml",
            "- isbn: Number 1\n- isbn: Number 5\n  permalink: /own/{isbn}/\n",
        ),
    ]);
    assert_eq!(problems(&read), []);
    let at = |path: &str| page(&read, path).address.as_str().to_owned();
    assert_eq!(at("a.md"), "/books/number-1/");
    assert_eq!(at("e.md/1"), "/made/number-1/");
    assert_eq!(at("b.md"), "/books/2.html");
    assert_eq!(at("c/d.md"), "/2026/number-4/");
    assert_eq!(at("e.md/2"), "/own/number-5/");
}

#[test]
fn a_nested_declared_field_fills_its_piece_from_where_it_is_nested() {
    let declared = "language: en\nfields.book.author: text\n";
    let read = site(&[
        ("statac.yaml", declared),
        (
            "content/a.md",
            "---\ntitle: A\nbook:\n  author: Name 1\npermalink: /a/{book.author}/\n---\n",
        ),
        (
            "content/b.md",
            "---\neach: data.b\npermalink: /b/{book.author}/\n---\n",
        ),
        (
            "data/b.yaml",
            "- book:\n    author: Name 2\n- book:\n    author: Name 3\n  permalink: /own/{book.author}/\n",
        ),
    ]);
    assert_eq!(problems(&read), []);
    let at = |path: &str| page(&read, path).address.as_str().to_owned();
    assert_eq!(at("a.md"), "/a/name-1/");
    assert_eq!(at("b.md/1"), "/b/name-2/");
    assert_eq!(at("b.md/2"), "/own/name-3/");

    let lacks = |fault| (Kind::UnusableAddress { fault }, "{book.author}".to_owned());
    let read = site(&[
        ("statac.yaml", declared),
        (
            "content/a.md",
            "---\nbook:\n  title: Title 1\npermalink: /a/{book.author}/\n---\n",
        ),
        (
            "content/b.md",
            "---\neach: data.b\npermalink: /b/{book.author}/\n---\n",
        ),
        ("data/b.yaml", "- book: Text\n"),
    ]);
    assert_eq!(
        kinds_at(&read),
        [
            lacks(AddressFault::PageLacks {
                page: "content/a.md".into(),
                field: "book.author".into(),
                piece: "{book.author}".into(),
            }),
            lacks(AddressFault::ItemLacks {
                item: "data/b.yaml: item 1".into(),
                field: "book.author".into(),
                piece: "{book.author}".into(),
            }),
        ]
    );
}

#[test]
fn a_pattern_asking_for_a_field_a_page_lacks_is_refused_where_it_asks() {
    let declared = "language: en\nfields.isbn: text\n";
    let lacks = |page: &str| Kind::UnusableAddress {
        fault: AddressFault::PageLacks {
            page: page.into(),
            field: "isbn".into(),
            piece: "{isbn}".into(),
        },
    };
    let no_piece = |nearest: Option<&str>| Kind::UnusableAddress {
        fault: AddressFault::UnknownPiece {
            written: "{isbn}".into(),
            nearest: nearest.map(str::to_owned),
        },
    };
    type Files<'a> = Vec<(&'a str, &'a str)>;
    let cases: [(Files, Kind, &str, &str); 5] = [
        (
            vec![
                ("statac.yaml", declared),
                ("content/a.md", "---\npermalink: /books/{isbn}/\n---\n"),
            ],
            lacks("content/a.md"),
            "content/a.md",
            "{isbn}",
        ),
        (
            vec![
                ("statac.yaml", declared),
                (
                    "content/c/index.md",
                    "---\npages.permalink: /c/{isbn}/\n---\n",
                ),
                ("content/c/a.md", "---\ntitle: A\n---\n"),
            ],
            lacks("content/c/a.md"),
            "content/c/index.md",
            "{isbn}",
        ),
        (
            vec![
                ("statac.yaml", declared),
                (
                    "content/a.md",
                    "---\nisbn: [1, 2]\npermalink: /{isbn}/\n---\n",
                ),
            ],
            Kind::WrongKind {
                field: "isbn".into(),
                expected: Expected::Text,
                found: Found::List,
                respelt: None,
            },
            "content/a.md",
            "[1, 2]",
        ),
        (
            vec![(
                "content/a.md",
                "---\nisbn: Number 1\npermalink: /books/{isbn}/\n---\n",
            )],
            no_piece(None),
            "content/a.md",
            "{isbn}",
        ),
        (
            vec![
                ("statac.yaml", "language: en\nfields.isbm: text\n"),
                (
                    "content/a.md",
                    "---\nisbm: Number 1\npermalink: /{isbn}/\n---\n",
                ),
            ],
            no_piece(Some("{isbm}")),
            "content/a.md",
            "{isbn}",
        ),
    ];
    for (files, kind, file, at) in cases {
        let read = site(&files);
        assert_eq!(kinds_at(&read), [(kind, at.to_owned())], "{files:?}");
        let place = read.problems[0].place.unwrap();
        assert_eq!(read.sources.path(place.source), Path::new(file));
    }
    let read = site(&[
        ("statac.yaml", declared),
        (
            "content/a.md",
            "---\nisbn: 3.10\npermalink: /{isbn}/\n---\n",
        ),
    ]);
    assert_eq!(
        problems(&read),
        [("needs-quotes", "content/a.md".into(), "3.10".into())]
    );
}

#[test]
fn a_pattern_needing_a_date_on_a_page_without_one_is_refused() {
    let read = site(&[
        (
            "content/posts/index.md",
            "---\npages.permalink: /{year}/{slug}/\n---\n",
        ),
        ("content/posts/post-1.md", "---\ntitle: Post 1\n---\n"),
    ]);
    assert_eq!(
        read.problems
            .iter()
            .map(|p| p.kind.clone())
            .collect::<Vec<_>>(),
        [Kind::UnusableAddress {
            fault: AddressFault::NoDate {
                piece: "{year}".into()
            }
        }]
    );
    assert_eq!(
        read.sources.spelling(read.problems[0].place.unwrap()),
        "---"
    );
}

#[test]
fn dates_in_the_name_and_the_front_matter_are_compared() {
    let read = site(&[
        ("statac.yaml", "language: en\ntimezone: UTC\n"),
        (
            "content/2026-10-04-post-1.md",
            "---\ntitle: Post 1\ndate: 2026-10-05 10:00\n---\n",
        ),
    ]);
    assert_eq!(
        problems(&read),
        [(
            "dates-disagree",
            "content/2026-10-04-post-1.md".to_owned(),
            "2026-10-05 10:00".to_owned()
        )]
    );
    assert_eq!(
        page(&read, "2026-10-04-post-1.md")
            .date
            .as_ref()
            .unwrap()
            .value
            .day,
        5
    );
    let read = site(&[(
        "content/2026-10-04-post-1.md",
        "---\ntitle: Post 1\ndate: 2026-10-04T23:00:00-05:00\n---\n",
    )]);
    assert_eq!(problems(&read), []);
}

/// A `permalink` written on a folder's own page moves it as it moves any
/// page, filled from the folder's name; the folder's other files stay, and
/// nothing is left at the folder's address.
#[test]
fn a_folders_own_permalink_moves_its_page() {
    // (the folder's page, its `permalink`, where it lands)
    let cases: [(&str, &str, &str); 8] = [
        ("posts/index.md", "/blog/", "/blog/"),
        ("posts/index.md", "/posts.html", "/posts.html"),
        ("posts/index.md", "/posts/", "/posts/"),
        ("posts/index.md", "/{year}/{slug}/", "/2026/posts/"),
        (
            "Folder 1/Folder 2/index.md",
            "/{folder}/{name}/",
            "/folder-2/Folder 2/",
        ),
        (
            "Folder 1/Folder 2/index.md",
            "/{slug}.html",
            "/folder-2.html",
        ),
        ("index.md", "/start/", "/start/"),
        ("index.md", "/index.html", "/index.html"),
    ];
    for (path, permalink, wanted) in cases {
        let folder = path.strip_suffix("index.md").unwrap();
        let index = format!("content/{path}");
        let post = format!("content/{folder}post-1.md");
        let file = format!("content/{folder}file-1.pdf");
        let said = format!("---\ntitle: Folder\ndate: 2026-01-02\npermalink: {permalink}\n---\n");
        let read = site(&[
            ("statac.yaml", "language: en\ntimezone: UTC\n"),
            (&index, &said),
            (&post, "---\ntitle: Post 1\n---\n"),
            (&file, "pdf"),
        ]);
        assert_eq!(problems(&read), [], "{path} {permalink}");
        let moved = page(&read, path);
        assert_eq!(moved.address.as_str(), wanted, "{path} {permalink}");
        let speaker = read.site.lists.get(moved.folder).page;
        let speaker = speaker.map(|id| read.site.pages.get(id).path.as_str());
        assert_eq!(speaker, Some(path), "{path} {permalink}");
        let files: Vec<&str> = read.site.files.iter().map(|f| f.address.as_str()).collect();
        let stays = format!("/{}file-1.pdf", slug::tidied_path(folder));
        assert_eq!(files, [stays.as_str()], "{path} {permalink}");
        let old = format!("{}index.html", slug::tidied_path(folder));
        let claimed: Vec<String> = read.claims().into_iter().map(|(file, _)| file).collect();
        assert_eq!(
            claimed.contains(&old),
            moved.address.file() == old,
            "{path} {permalink}: {claimed:?}"
        );
    }
}

/// A `permalink` handed down from a folder above, or from the settings,
/// moves the pages beneath but never a folder's own page.
#[test]
fn a_handed_down_permalink_leaves_a_folders_own_page_at_its_folder() {
    let read = site(&[
        (
            "statac.yaml",
            "language: en\npages.permalink: /site/{slug}/\n",
        ),
        ("content/index.md", "---\ntitle: Home\n---\n"),
        (
            "content/posts/index.md",
            "---\ntitle: Posts\npages.permalink: /p/{slug}/\n---\n",
        ),
        ("content/posts/post-1.md", "---\ntitle: Post 1\n---\n"),
        (
            "content/posts/folder-1/index.md",
            "---\ntitle: Folder 1\n---\n",
        ),
        (
            "content/posts/folder-1/post-2.md",
            "---\ntitle: Post 2\n---\n",
        ),
        ("content/folder-2/index.md", "---\ntitle: Folder 2\n---\n"),
        ("content/folder-2/post-3.md", "---\ntitle: Post 3\n---\n"),
    ]);
    assert_eq!(problems(&read), []);
    assert_eq!(
        addresses(&read),
        [
            ("folder-2/index.md".into(), "/folder-2/".into()),
            ("folder-2/post-3.md".into(), "/site/post-3/".into()),
            ("index.md".into(), "/".into()),
            ("posts/folder-1/index.md".into(), "/posts/folder-1/".into()),
            ("posts/folder-1/post-2.md".into(), "/p/post-2/".into()),
            ("posts/index.md".into(), "/posts/".into()),
            ("posts/post-1.md".into(), "/p/post-1/".into()),
        ]
    );
}

/// A `permalink` on a language folder's own page, or a folder's in it, is
/// read from the language folder's address.
#[test]
fn a_language_folders_own_permalink_is_read_from_its_address() {
    let read = site(&[
        ("statac.yaml", "language: [en, fr]\n"),
        ("content/index.md", "---\ntitle: Home\n---\n"),
        (
            "content/fr/index.md",
            "---\ntitle: Accueil\npermalink: /accueil/\n---\n",
        ),
        ("content/fr/post-1.md", "---\ntitle: Post 1\n---\n"),
        (
            "content/fr/folder-1/index.md",
            "---\ntitle: Folder 1\npermalink: /{slug}.html\n---\n",
        ),
        ("content/fr/folder-1/post-2.md", "---\ntitle: Post 2\n---\n"),
    ]);
    assert_eq!(problems(&read), []);
    assert_eq!(page(&read, "fr/index.md").address.as_str(), "/fr/accueil/");
    assert_eq!(
        page(&read, "fr/folder-1/index.md").address.as_str(),
        "/fr/folder-1.html"
    );
}

#[test]
fn a_page_making_pages_reads_permalink_as_their_pattern() {
    let read = site(&[
        (
            "content/feeds.md",
            "---\ntitle: Feeds\neach: data.feeds\npermalink: /{key}.xml\nindex_page: false\n---\n",
        ),
        ("data/feeds.yaml", "feed-1:\n  title: Feed 1\n"),
    ]);
    assert_eq!(problems(&read), []);
    let feeds = page(&read, "feeds.md");
    assert_eq!(feeds.address.as_str(), "/feeds/");
    assert!(!feeds.lands());
    assert_eq!(
        page(&read, "feeds.md/feed-1").address.as_str(),
        "/feed-1.xml"
    );
}

#[test]
fn declared_fields_are_checked_and_required() {
    let read = site(&[
        (
            "statac.yaml",
            "language: en\nfields.isbn: text\ntypes.book.author: required text\n",
        ),
        (
            "content/book-1.md",
            "---\ntitle: Book 1\ntype: book\nauthor: Author 1\nisbn: 3.10\n---\n",
        ),
        ("content/book-2.md", "---\ntitle: Book 2\ntype: book\n---\n"),
        ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
    ]);
    assert_eq!(
        problems(&read),
        [
            (
                "needs-quotes",
                "content/book-1.md".to_owned(),
                "3.10".to_owned()
            ),
            (
                "missing-field",
                "content/book-2.md".to_owned(),
                "---".to_owned()
            ),
        ]
    );
}

/// Each error as it reads, with its suggestion, the file it is in and the
/// text it is at.
fn errors(read: &Read) -> Vec<(String, Option<String>, String, String)> {
    let errors = (read.problems.iter())
        .filter(|p| p.kind.code().severity() == crate::problem::Severity::Error);
    errors
        .map(|p| {
            let place = p.place.expect("placed");
            (
                crate::wording::headline(&p.kind),
                crate::wording::suggestion(&p.kind),
                read.sources.path(place.source).display().to_string(),
                read.sources.spelling(place).to_owned(),
            )
        })
        .collect()
}

/// Where the declaration is, times what it names, times what the page
/// writes.
#[test]
fn a_field_naming_a_record_or_a_page_is_held_to_the_names_there() {
    // The declaration's name, and a page's lines writing `{}` for it.
    let declared = [
        ("fields.field", "field: {}\n"),
        ("types.kind.field", "type: kind\nfield: {}\n"),
        ("fields.outer.field", "outer:\n  field: {}\n"),
    ];
    // What is named, the files holding its names, a name, one near it, and
    // how a value that is no name is told.
    type Files<'a> = &'a [(&'a str, &'a str)];
    let named: [(&str, Files, &str, &str, &str); 3] = [
        (
            "data.people",
            &[(
                "data/people.yaml",
                "person-1:\n  name: Person 1\nperson-2:\n  name: Person 2\n",
            )],
            "person-1",
            "persn-1",
            "Names nothing: `data/people.yaml` has no `{}`",
        ),
        (
            "folder.books",
            &[
                ("content/books/book-1.md", "# Book 1\n"),
                ("content/books/book-2.md", "# Book 2\n"),
            ],
            "book-1",
            "bok-1",
            "Names nothing: no page in `content/books/` is called `{}`",
        ),
        (
            "folder.days",
            &[
                ("content/days/01-day-one.md", "# Day 1\n"),
                ("content/days/02-day-two.md", "# Day 2\n"),
            ],
            "day-one",
            "01-day-one",
            "Names nothing: no page in `content/days/` is called `{}`",
        ),
    ];
    for (declaration, lines) in declared {
        for (names, files, name, near, told) in named {
            // What the page writes, what is told at it, and the name
            // offered.
            let quoted = format!("\"{near}\"");
            let written = [
                (name, None, None),
                (near, Some(near), Some(name)),
                (quoted.as_str(), Some(near), Some(name)),
                ("nothing-like-it", Some("nothing-like-it"), None),
                ("12", Some("12"), None),
                ("\"\"", Some(""), None),
                ("", None, None),
                ("~", None, None),
            ];
            for (value, wrong, nearest) in written {
                let settings = format!("language: en\n{declaration}: one of {names}\n");
                let post = format!("---\ntitle: Post 1\n{}---\n", lines.replace("{}", value));
                let mut all = vec![
                    ("statac.yaml", settings.as_str()),
                    ("content/post-1.md", post.as_str()),
                ];
                all.extend(files);
                let read = site(&all);
                let wanted: Vec<_> = wrong
                    .map(|wrong| {
                        (
                            told.replace("{}", wrong),
                            nearest.map(|name| format!("Did you mean `{name}`?")),
                            "content/post-1.md".to_owned(),
                            value.to_owned(),
                        )
                    })
                    .into_iter()
                    .collect();
                assert_eq!(errors(&read), wanted, "{declaration} {names} {value:?}");
            }
            // A list is the wrong kind of value, and no name.
            let settings = format!("language: en\n{declaration}: one of {names}\n");
            let post = format!("---\n{}---\n", lines.replace("{}", &format!("[{name}]")));
            let mut all = vec![
                ("statac.yaml", settings.as_str()),
                ("content/post-1.md", post.as_str()),
            ];
            all.extend(files);
            let kinds: Vec<_> = (problems(&site(&all)).into_iter())
                .filter(|(kind, ..)| *kind != "numbered-folder")
                .collect();
            let list = format!("[{name}]");
            assert_eq!(
                kinds,
                [("wrong-kind-of-value", "content/post-1.md".to_owned(), list)],
                "{declaration} {names}"
            );
        }
    }
}

/// Where the declaration is, times what it names that can hold no value to
/// a name. Each is told at the declaration, and no value of the field is.
#[test]
fn a_declaration_naming_no_table_or_folder_is_told_where_it_is_written() {
    let declared = ["fields.field", "types.kind.field", "fields.outer.field"];
    let named = [
        (
            "data.poeple",
            "Missing data: there is no `data.poeple` for `{}` to name",
            Some("Did you mean `data.people`?"),
        ),
        (
            "data.nothing.like.it",
            "Missing data: there is no `data.nothing.like.it` for `{}` to name",
            None,
        ),
        (
            "data.items",
            "No names to check: `data/items.json` is a list, so `{}` can't name one of its items",
            Some("Give each item a name with its fields beneath, or declare the field as `text`."),
        ),
        (
            "folder.boks",
            "Missing folder: there is no `content/boks/` for `{}` to name",
            Some("Did you mean `folder.books`?"),
        ),
        (
            "folder.books/shelf-2",
            "Missing folder: there is no `content/books/shelf-2/` for `{}` to name",
            Some("Did you mean `folder.books/shelf-1`?"),
        ),
        (
            "folder.nothing-like-it",
            "Missing folder: there is no `content/nothing-like-it/` for `{}` to name",
            None,
        ),
    ];
    for declaration in declared {
        for (names, told, suggested) in named {
            let kind = format!("one of {names}");
            let settings = format!("{declaration}: {kind}\n");
            let read = site(&[
                ("statac.yaml", &settings),
                ("data/people.yaml", "person-1:\n  name: Person 1\n"),
                ("data/items.json", "[{\"name\": \"Item 1\"}]\n"),
                ("content/books/shelf-1/book-1.md", "# Book 1\n"),
                (
                    "content/post-1.md",
                    "---\ntype: kind\nfield: post-2\nouter:\n  field: post-2\n---\n",
                ),
            ]);
            let wanted = (
                told.replace("{}", declaration),
                suggested.map(str::to_owned),
                "statac.yaml".to_owned(),
                kind.clone(),
            );
            assert_eq!(errors(&read), [wanted], "{declaration} {names}");
        }
    }
}

#[test]
fn a_table_with_no_names_and_a_folder_of_tables_hold_a_value_to_what_they_have() {
    let read = site(&[
        (
            "statac.yaml",
            "language: en\nfields.first: one of data.empty\nfields.second: one of data.people\n\
             fields.third: one of data.people.staff\n",
        ),
        ("data/empty.yaml", ""),
        ("data/people/staff.yaml", "person-1:\n  name: Person 1\n"),
        (
            "content/post-1.md",
            "---\nfirst: person-1\nsecond: staff\nthird: person-1\n---\n",
        ),
        (
            "content/post-2.md",
            "---\nsecond: staf\nthird: person-2\n---\n",
        ),
    ]);
    let told = |headline: &str, nearest: Option<&str>, file: &str, at: &str| {
        (
            headline.to_owned(),
            nearest.map(|name| format!("Did you mean `{name}`?")),
            file.to_owned(),
            at.to_owned(),
        )
    };
    assert_eq!(
        errors(&read),
        [
            told(
                "Names nothing: `data/empty.yaml` has no `person-1`",
                None,
                "content/post-1.md",
                "person-1"
            ),
            told(
                "Names nothing: `data/people/` has no `staf`",
                Some("staff"),
                "content/post-2.md",
                "staf"
            ),
            told(
                "Names nothing: `data/people/staff.yaml` has no `person-2`",
                Some("person-1"),
                "content/post-2.md",
                "person-2"
            ),
        ]
    );
}

/// A folder's pages are those its list holds, in it and beneath it, each
/// by its name in its address, whether or not it is published.
#[test]
fn a_folders_names_are_its_pages_names_in_their_addresses() {
    let mut files = vec![
        (
            "statac.yaml".to_owned(),
            "language: en\nfields.field: one of folder.books\n".to_owned(),
        ),
        ("content/books/index.md".into(), "# Books\n".into()),
        ("content/books/book-1.md".into(), "# Book 1\n".into()),
        (
            "content/books/book-2.md".into(),
            "---\ndraft: true\n---\n".into(),
        ),
        (
            "content/books/book-3.md".into(),
            "---\ndate: 2027-06-01\n---\n".into(),
        ),
        ("content/books/book-4/index.md".into(), "# Book 4\n".into()),
        (
            "content/books/shelf-1/index.md".into(),
            "# Shelf 1\n".into(),
        ),
        (
            "content/books/shelf-1/book-5.md".into(),
            "# Book 5\n".into(),
        ),
        (
            "content/books/book-6.md".into(),
            "---\npermalink: /elsewhere/six/\n---\n".into(),
        ),
        ("content/books/Book 7.md".into(), "# Book 7\n".into()),
        (
            "content/books/book-8.md".into(),
            "---\ntitle: [\n---\n".into(),
        ),
        (
            "content/books/2026-01-02-book-9.md".into(),
            "# Book 9\n".into(),
        ),
        (
            "content/books/book-10.md".into(),
            "---\nredirect_to: /books/book-1/\n---\n".into(),
        ),
    ];
    let written = [
        ("book-1", true),
        ("book-2", true),
        ("book-3", true),
        ("book-4", true),
        ("book-5", true),
        ("book-6", true),
        ("book-7", true),
        ("book-8", true),
        ("book-9", true),
        ("index", false),
        ("books", false),
        ("shelf-1", false),
        ("six", false),
        ("Book 7", false),
        ("2026-01-02-book-9", false),
        ("book-10", false),
    ];
    for (at, (name, _)) in written.iter().enumerate() {
        files.push((
            format!("content/post-{at}.md"),
            format!("---\nfield: {name}\n---\n"),
        ));
    }
    let files: Vec<(&str, &str)> = (files.iter())
        .map(|(path, text)| (path.as_str(), text.as_str()))
        .collect();
    let read = site(&files);
    let mut told: Vec<String> = (read.problems.iter())
        .filter(|p| p.kind.name() == "names-nothing")
        .map(|p| read.sources.spelling(p.place.unwrap()).to_owned())
        .collect();
    told.sort();
    let mut wanted: Vec<&str> = (written.iter())
        .filter(|(_, named)| !named)
        .map(|(name, _)| *name)
        .collect();
    wanted.sort_unstable();
    assert_eq!(told, wanted);
}

/// A folder is the one at its path in each language, and a page there in
/// any language is a name.
#[test]
fn a_folder_named_is_each_languages_and_a_page_in_any_is_a_name() {
    let read = site(&[
        (
            "statac.yaml",
            "language: [en, de]\nfields.first: one of folder.books\n\
             fields.second: one of folder.notes\n",
        ),
        ("content/books/book-1.md", "# Book 1\n"),
        ("content/de/books/buch-1.md", "# Buch 1\n"),
        ("content/de/notes/notiz-1.md", "# Notiz 1\n"),
        ("content/post-1.md", "---\nfirst: buch-1\n---\n"),
        (
            "content/post-2.md",
            "---\nfirst: buch-2\nsecond: notiz-2\n---\n",
        ),
        ("content/de/post-1.md", "---\nfirst: book-1\n---\n"),
        ("content/de/post-2.md", "---\nfirst: book-2\n---\n"),
    ]);
    let told = |folder: &str, written: &str, nearest: &str, file: &str| {
        (
            format!("Names nothing: no page in `{folder}` is called `{written}`"),
            Some(format!("Did you mean `{nearest}`?")),
            file.to_owned(),
            written.to_owned(),
        )
    };
    let mut found = errors(&read);
    found.sort();
    assert_eq!(
        found,
        [
            told("content/books/", "buch-2", "buch-1", "content/post-2.md"),
            told(
                "content/de/books/",
                "book-2",
                "book-1",
                "content/de/post-2.md"
            ),
            told(
                "content/de/notes/",
                "notiz-2",
                "notiz-1",
                "content/post-2.md"
            ),
        ]
    );
}

/// A value handed down, or written for the pages made from a table, is
/// told once, where it is written.
#[test]
fn a_name_handed_down_or_written_in_an_item_is_told_once_where_it_is_written() {
    let read = site(&[
        (
            "statac.yaml",
            "language: en\nfields.field: one of data.people\nfields.other: one of data.people\n\
             pages.other: persn-3\n",
        ),
        (
            "data/people.yaml",
            "person-1:\n  field: persn-1\nperson-2:\n  field: person-1\n",
        ),
        (
            "content/people.md",
            "---\neach: data.people\npermalink: /people/{key}/\n---\n",
        ),
        (
            "content/posts/index.md",
            "---\ntitle: Posts\npages.field: persn-2\n---\n",
        ),
        ("content/posts/post-1.md", "# Post 1\n"),
        ("content/posts/post-2.md", "# Post 2\n"),
    ]);
    let told = |written: &str, nearest: &str, file: &str| {
        (
            format!("Names nothing: `data/people.yaml` has no `{written}`"),
            Some(format!("Did you mean `{nearest}`?")),
            file.to_owned(),
            written.to_owned(),
        )
    };
    let mut found = errors(&read);
    found.sort();
    assert_eq!(
        found,
        [
            told("persn-1", "person-1", "data/people.yaml"),
            told("persn-2", "person-2", "content/posts/index.md"),
            told("persn-3", "person-1", "statac.yaml"),
        ]
    );
}

#[test]
fn a_field_written_flat_and_nested_across_pages_is_warned_about() {
    let read = site(&[
        (
            "content/post-1.md",
            "---\ntitle: Post 1\nbook.author: Author 1\n---\n",
        ),
        (
            "content/post-2.md",
            "---\ntitle: Post 2\nbook:\n  author: Author 2\n---\n",
        ),
    ]);
    assert_eq!(
        problems(&read),
        [(
            "field-written-two-ways",
            "content/post-1.md".to_owned(),
            "book.author".to_owned()
        )]
    );
    let book = page(&read, "post-2.md").words.field("book").unwrap();
    assert_eq!(
        read.sources
            .spelling(book.value.get("author").unwrap().place),
        "Author 2"
    );
}

#[test]
fn old_addresses_are_claimed_and_refused_where_unusable() {
    let read = site(&[(
        "content/post-1.md",
        "---\ntitle: Post 1\nredirect_from: [/old/, /old, '/a:b/']\n---\n",
    )]);
    assert_eq!(
        problems(&read),
        [(
            "unusable-address",
            "content/post-1.md".to_owned(),
            "a:b".to_owned()
        )]
    );
    let old: Vec<&str> = read
        .site
        .redirects
        .iter()
        .map(|r| r.from.as_str())
        .collect();
    assert_eq!(old, ["/old/"]);
    assert_eq!(read.sources.spelling(read.site.redirects[0].place), "/old/");
}

#[test]
fn two_claims_on_one_address_are_refused_naming_both() {
    let temp = TempSite::site(&[
        ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
        ("content/post-1/index.md", "---\ntitle: Post 1 again\n---\n"),
        (
            "content/Post-2.md",
            "---\ntitle: Post 2\npermalink: /post-2/\n---\n",
        ),
        (
            "content/post-3.md",
            "---\ntitle: Post 3\nredirect_from: [/POST-2/]\n---\n",
        ),
        ("content/a.md", "---\ntitle: A\npermalink: /logo.png\n---\n"),
        ("static/logo.png", "png"),
    ]);
    let (read, ..) = crate::build::read(&temp.root, &run_on(2026, 12, 31));
    let mut same: Vec<(String, String)> = read
        .problems
        .iter()
        .filter_map(|p| match &p.kind {
            Kind::SameAddress { first, second, .. } => match (&**first, &**second) {
                (Named::File(first), Named::File(second)) => Some((first.clone(), second.clone())),
                _ => None,
            },
            _ => None,
        })
        .collect();
    same.sort();
    let s = |a: &str, b: &str| (a.to_owned(), b.to_owned());
    assert_eq!(
        same,
        [
            s("content/Post-2.md", "content/post-3.md"),
            s("content/post-1.md", "content/post-1/index.md"),
            s("static/logo.png", "content/a.md"),
        ]
    );
}

#[test]
fn a_claim_on_a_page_statac_makes_names_that_page() {
    let temp = TempSite::site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\nposts_per_page: 1\n",
        ),
        ("content/blog/index.md", "---\ntitle: Blog\n---\n"),
        (
            "content/blog/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-10-01\ntags: [Rust]\n---\n",
        ),
        (
            "content/blog/post-2.md",
            "---\ntitle: Post 2\ndate: 2026-10-02\n---\n",
        ),
        ("static/index.html", "<p>Home</p>"),
        ("static/tags/rust/index.html", "<p>Rust</p>"),
        ("static/blog/page/2/index.html", "<p>Blog</p>"),
    ]);
    let (read, ..) = crate::build::read(&temp.root, &run_on(2026, 12, 31));
    let mut same: Vec<String> = read
        .problems
        .iter()
        .filter(|p| p.kind.code() == Code::SameAddress)
        .map(Problem::headline)
        .collect();
    same.sort();
    assert_eq!(
        same,
        [
            "Same address: `static/blog/page/2/index.html` and page 2 of `/blog/` are both \
             published at `/blog/page/2/index.html`",
            "Same address: `static/index.html` and the site's home page are both published \
             at `/index.html`",
            "Same address: `static/tags/rust/index.html` and the page for the tag `Rust` are \
             both published at `/tags/rust/index.html`",
        ]
    );
}

#[test]
fn a_file_that_isnt_text_is_refused_and_the_rest_read() {
    let temp = TempSite::site(&[("content/post-1.md", "---\ntitle: Post 1\n---\n")]);
    std::fs::write(temp.root.join("content/post-2.md"), [0xff, 0xfe, 0x00]).unwrap();
    let read = read_site(&temp);
    assert_eq!(
        read.problems,
        [Problem::nowhere(Kind::CantRead {
            path: "content/post-2.md".into(),
            reason: Reason::NotUtf8,
        })]
    );
    let pages = read.site.pages.iter();
    let written = pages.filter(|(_, page)| page.place != PagePlace::Made);
    assert_eq!(written.count(), 1);
}

#[test]
fn front_matter_that_isnt_fields_is_refused() {
    let read = site(&[("content/post-1.md", "---\n- a\n---\n")]);
    assert_eq!(
        read.problems
            .iter()
            .map(|p| p.kind.clone())
            .collect::<Vec<_>>(),
        [Kind::WrongKind {
            field: crate::wording::FRONT_MATTER.into(),
            expected: Expected::Fields,
            found: Found::List,
            respelt: None,
        }]
    );
    let read = site(&[("content/post-1.md", "---\ntemplate: post\n---\n")]);
    assert!(matches!(
        &read.problems[0].kind,
        Kind::OtherSpelling {
            word: Word::Field,
            ..
        }
    ));
}

#[test]
fn the_site_has_one_language_from_its_settings() {
    let read = site(&[
        ("statac.yaml", "language: en-GB\n"),
        ("content/post-1.md", "# Post 1\n"),
    ]);
    let (id, language) = read.site.languages.iter().next().unwrap();
    assert_eq!(read.site.languages.len(), 1);
    assert_eq!(language.tag.as_deref(), Some("en-GB"));
    assert_eq!(page(&read, "post-1.md").language, id);
}

/// Each written page's path with its language's tag, A to Z by path.
fn languages_of(read: &Read) -> Vec<(String, Option<String>)> {
    let mut found: Vec<_> = read
        .site
        .pages
        .iter()
        .filter(|(_, page)| page.source.is_some())
        .map(|(_, page)| {
            let tag = &read.site.languages.get(page.language).tag;
            (page.path.clone(), tag.clone())
        })
        .collect();
    found.sort();
    found
}

#[test]
fn a_page_in_a_languages_folder_is_in_that_language_and_every_other_in_the_main_one() {
    let read = site(&[
        ("statac.yaml", "language: [ar, en]\n"),
        ("content/index.md", "# Home\n"),
        ("content/post-1.md", "# Post 1\n"),
        ("content/en/index.md", "# Home\n"),
        ("content/en/post-1.md", "# Post 1\n"),
        ("content/en/blog/post-2.md", "# Post 2\n"),
        ("content/english/post-3.md", "# Post 3\n"),
    ]);
    assert_eq!(problems(&read), []);
    let tags: Vec<_> = read
        .site
        .languages
        .iter()
        .map(|(_, l)| l.tag.clone())
        .collect();
    assert_eq!(tags, [Some("ar".into()), Some("en".into())]);
    let (ar, en) = (Some("ar".to_owned()), Some("en".to_owned()));
    assert_eq!(
        languages_of(&read),
        [
            ("en/blog/post-2.md".into(), en.clone()),
            ("en/index.md".into(), en.clone()),
            ("en/post-1.md".into(), en.clone()),
            ("english/post-3.md".into(), ar.clone()),
            ("index.md".into(), ar.clone()),
            ("post-1.md".into(), ar.clone()),
        ]
    );
    let folder = |path: &str| {
        let key = ListKey::Folder(path.into());
        let (_, list) = read
            .site
            .lists_for(&key)
            .next()
            .unwrap_or_else(|| panic!("no folder {path}"));
        read.site.languages.get(list.language).tag.clone()
    };
    assert_eq!(folder(""), ar);
    assert_eq!(folder("en"), en);
    assert_eq!(folder("en/blog"), en);
    // Where a page stands is as it always was.
    assert_eq!(page(&read, "en/post-1.md").address.as_str(), "/en/post-1/");
}

#[test]
fn the_main_language_may_have_a_folder_and_a_page_outside_every_folder_is_in_it() {
    let read = site(&[
        ("statac.yaml", "language: [ar, en]\n"),
        ("content/index.md", "# Home\n"),
        ("content/ar/post-1.md", "# Post 1\n"),
        ("content/en/post-1.md", "# Post 1\n"),
    ]);
    assert_eq!(problems(&read), []);
    let (ar, en) = (Some("ar".to_owned()), Some("en".to_owned()));
    assert_eq!(
        languages_of(&read),
        [
            ("ar/post-1.md".into(), ar.clone()),
            ("en/post-1.md".into(), en),
            ("index.md".into(), ar),
        ]
    );
}

#[test]
fn a_site_in_one_language_reads_a_folder_named_for_it_as_any_other() {
    let files = [
        ("content/index.md", "# Home\n"),
        ("content/ar/index.md", "# Home\n"),
        ("content/ar/post-1.md", "# Post 1\n"),
    ];
    let mut with = vec![("statac.yaml", "language: ar\n")];
    with.extend(files);
    let mut without = vec![("statac.yaml", "language: en\ntitle: Site 1\n")];
    without.extend(files);
    let (with, without) = (site(&with), site(&without));
    assert_eq!(problems(&with), []);
    assert_eq!(with.site.languages.len(), 1);
    assert!(with.site.pages.iter().all(|(_, p)| p.language.index() == 0));
    assert_eq!(addresses(&with), addresses(&without));
    let folders = |read: &Read| {
        let mut keys: Vec<_> = read.site.lists.iter().map(|(_, l)| l.key.clone()).collect();
        keys.sort_by_key(|key| format!("{key:?}"));
        keys
    };
    assert_eq!(folders(&with), folders(&without));
}

#[test]
fn a_language_with_no_folder_is_warned_of_once_at_its_tag() {
    let read = site(&[
        ("statac.yaml", "language: [ar, en, fr]\n"),
        ("content/post-1.md", "# Post 1\n"),
        ("content/en/post-1.md", "# Post 1\n"),
    ]);
    assert_eq!(
        problems(&read),
        [(
            "no-pages-in-language",
            "statac.yaml".to_owned(),
            "fr".to_owned()
        )]
    );
    let Kind::NoPagesInLanguage { tag } = &read.problems[0].kind else {
        unreachable!()
    };
    assert_eq!(tag, "fr");
    // The main language's pages may all sit at the top.
    let read = site(&[
        ("statac.yaml", "language: [ar, en]\n"),
        ("content/en/post-1.md", "# Post 1\n"),
    ]);
    assert_eq!(problems(&read), []);
}

#[cfg(unix)]
#[test]
fn a_link_is_followed_only_where_it_stays_inside_content() {
    use std::os::unix::fs::symlink;
    let outside = TempSite::site(&[("content/post-9.md", "---\ntitle: Post 9\n---\n")]);
    let temp = TempSite::site(&[
        ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
        ("content/drafts/post-3.md", "---\ntitle: Post 3\n---\n"),
        ("notes/post-2.md", "---\ntitle: Post 2\n---\n"),
        ("statac.yaml", "language: en\ntitle: Site\n"),
    ]);
    let content = temp.root.join("content");
    symlink(
        outside.root.join("content/post-9.md"),
        content.join("away.md"),
    )
    .unwrap();
    symlink(outside.root.join("content"), content.join("elsewhere")).unwrap();
    symlink(temp.root.join("notes/post-2.md"), content.join("post-2.md")).unwrap();
    symlink(temp.root.join("notes"), content.join("notes")).unwrap();
    symlink(temp.root.join("statac.yaml"), content.join("settings.md")).unwrap();
    symlink(content.join("drafts/post-3.md"), content.join("post-3.md")).unwrap();
    let read = read_site(&temp);
    let link = |path: &str| Kind::OutsideSite {
        reach: crate::problem::Reach::Link {
            path: path.into(),
            within: Some("content/".into()),
        },
    };
    let kinds: Vec<Kind> = read.problems.iter().map(|p| p.kind.clone()).collect();
    assert_eq!(
        kinds,
        [
            link("content/away.md"),
            link("content/elsewhere"),
            link("content/notes"),
            link("content/post-2.md"),
            link("content/settings.md"),
        ]
    );
    let mut paths: Vec<&str> = read
        .site
        .pages
        .iter()
        .filter(|(_, page)| page.place != PagePlace::Made)
        .map(|(_, page)| page.path.as_str())
        .collect();
    paths.sort();
    assert_eq!(paths, ["drafts/post-3.md", "post-1.md", "post-3.md"]);
    // Nothing outside content/ was read, not even as a page left out.
    assert!(read.drafts.is_empty());
}

#[test]
fn a_site_with_no_time_zone_is_told_once_where_a_date_has_no_offset() {
    let unzoned = ("no-time-zone", String::new(), String::new());
    let cases: [(&[(&str, &str)], bool); 5] = [
        (
            &[
                ("content/2026-10-04-post-1.md", "# Post 1\n"),
                ("content/post-2.md", "---\ndate: 2026-10-05 09:30\n---\n"),
            ],
            true,
        ),
        (
            &[("content/post-1.md", "---\nupdated: 2026-10-05\n---\n")],
            true,
        ),
        (
            &[(
                "content/post-1.md",
                "---\ndate: 2026-10-05T09:30+01:00\n---\n",
            )],
            false,
        ),
        (&[("content/post-1.md", "# Post 1\n")], false),
        (
            &[
                ("statac.yaml", "language: en\ntimezone: Europe/London\n"),
                ("content/2026-10-04-post-1.md", "# Post 1\n"),
            ],
            false,
        ),
    ];
    for (files, told) in cases {
        let read = site(files);
        let wanted = match told {
            true => vec![unzoned.clone()],
            false => Vec::new(),
        };
        assert_eq!(problems(&read), wanted, "{files:?}");
    }
}

#[test]
fn nested_fields_are_checked_and_required_by_their_declarations() {
    let read = site(&[
        (
            "statac.yaml",
            "language: en\nfields.book.author: text\ntypes.review.book.rating: required number\n",
        ),
        (
            "content/review-1.md",
            "---\ntitle: Review 1\ntype: review\nbook:\n  author: 3.10\n  rating: 4\n---\n",
        ),
        (
            "content/review-2.md",
            "---\ntitle: Review 2\ntype: review\nbook:\n  author: Author 1\n---\n",
        ),
        (
            "content/posts/index.md",
            "---\ntitle: Posts\npages.book:\n  rating: many\n---\n",
        ),
        ("content/posts/post-1.md", "---\ntitle: Post 1\n---\n"),
    ]);
    assert_eq!(
        problems(&read),
        [
            (
                "needs-quotes",
                "content/review-1.md".to_owned(),
                "3.10".to_owned()
            ),
            (
                "missing-field",
                "content/review-2.md".to_owned(),
                "---".to_owned()
            ),
            (
                "wrong-kind-of-value",
                "content/posts/index.md".to_owned(),
                "many".to_owned()
            ),
        ]
    );
}

/// How a page sending readers on is named and what it names, times where
/// it stands and where readers are sent.
#[test]
fn a_page_sending_readers_on_leaves_a_redirect_at_its_address() {
    let site_at = |address: &str, fragment: &str| SentTo::Site {
        written: String::new(),
        address: Address::read(address).unwrap(),
        fragment: fragment.to_owned(),
    };
    // (file, its front matter, where it stands, where readers are sent)
    let cases = [
        (
            "content/post-1.md",
            "redirect_to: /post-2/",
            "/post-1/",
            site_at("/post-2/", ""),
        ),
        (
            "content/post-1.md",
            "redirect_to: /post-2#part-1",
            "/post-1/",
            site_at("/post-2/", "#part-1"),
        ),
        (
            "content/post-1.md",
            "redirect_to: /caf%C3%A9/",
            "/post-1/",
            site_at("/café/", ""),
        ),
        (
            "content/post-1.md",
            "redirect_to: https://example.org/post-1/",
            "/post-1/",
            SentTo::Elsewhere("https://example.org/post-1/".into()),
        ),
        (
            "content/2019-05-01-post-1.md",
            "redirect_to: /post-2/",
            "/2019-05-01-post-1/",
            site_at("/post-2/", ""),
        ),
        (
            "content/2019-05-01-post-1/index.md",
            "redirect_to: /post-2/",
            "/2019-05-01-post-1/",
            site_at("/post-2/", ""),
        ),
        (
            "content/2019-05-01-post-1.md",
            "permalink: /old/post-1.html\nredirect_to: /post-2/",
            "/old/post-1.html",
            site_at("/post-2/", ""),
        ),
        (
            "content/folder-1/index.md",
            "redirect_to: /post-2/",
            "/folder-1/",
            site_at("/post-2/", ""),
        ),
    ];
    for (path, front, from, to) in cases {
        let read = site(&[
            (path, &format!("---\ntitle: Page 1\n{front}\n---\n")),
            ("content/folder-1/post-3.md", "---\ntitle: Post 3\n---\n"),
            ("statac.yaml", "language: en\ntimezone: UTC\n"),
        ]);
        assert_eq!(problems(&read), [], "{path}: {front}");
        let page = page(&read, path.trim_start_matches("content/"));
        assert!(!page.lands(), "{path}: {front}");
        let sending: Vec<_> = read
            .site
            .redirects
            .iter()
            .map(|r| {
                let to = match &r.to {
                    SentTo::Site {
                        address, fragment, ..
                    } => site_at(address.as_str(), fragment),
                    other => other.clone(),
                };
                (r.from.as_str(), to, read.sources.spelling(r.place))
            })
            .collect();
        let written = front.rsplit("redirect_to: ").next().unwrap();
        assert_eq!(sending, [(from, to, written)], "{path}: {front}");
    }
}

#[test]
fn a_page_sending_readers_to_itself_or_nowhere_usable_is_refused() {
    // (front matter, the problem, the text it is at)
    let cases = [
        ("redirect_to: /post-1/", "redirect-to-itself", "/post-1/"),
        (
            "redirect_to: /POST-1#part-1",
            "redirect-to-itself",
            "/POST-1#part-1",
        ),
        (
            "permalink: /old/\nredirect_to: /old/",
            "redirect-to-itself",
            "/old/",
        ),
        ("redirect_to: /a:b/", "unusable-address", "a:b"),
        ("redirect_to: '/a%3Ab/'", "unusable-address", "/a%3Ab/"),
    ];
    for (front, name, at) in cases {
        let read = site(&[("content/post-1.md", &format!("---\n{front}\n---\n"))]);
        assert_eq!(
            problems(&read),
            [(name, "content/post-1.md".to_owned(), at.to_owned())],
            "{front}"
        );
        assert!(read.site.redirects.is_empty(), "{front}");
    }
}

/// What a page sending readers on writes, times what is said of it: the
/// words it reads pass, each other word of Statac's it writes is set
/// aside, and its writing is told at its first line.
#[test]
fn what_a_page_sending_readers_on_never_publishes_is_told() {
    /// Each problem's name and the text it is at.
    type Said<'a> = &'a [(&'a str, &'a str)];
    // (file, what follows `redirect_to`, the problems)
    let cases: [(&str, &str, Said); 7] = [
        (
            "content/post-1.md",
            "title: Post 1\ndate: 2026-01-01\npermalink: /old/\ndraft: false\n\
             redirect_from: [/older/]\nmy_field: 1\n---\n\n",
            &[],
        ),
        (
            "content/post-1.md",
            "tags: [Tag 1]\nsummary: Text\nlayout: post\n---\n",
            &[
                ("set-aside", "[Tag 1]"),
                ("set-aside", "Text"),
                ("set-aside", "post"),
            ],
        ),
        (
            "content/post-1.md",
            "---\n\n  The first post.\nMore.\n",
            &[("writing-not-published", "The first post.")],
        ),
        (
            "content/post-1.html",
            "---\n<p>Text</p>\n",
            &[("writing-not-published", "<p>Text</p>")],
        ),
        (
            "content/folder-1/index.md",
            "order: oldest first\nnumbered: false\nfeed: true\nposts_per_page: 2\n---\n",
            &[("set-aside", "true"), ("set-aside", "2")],
        ),
        (
            "content/folder-1/index.md",
            "posts: site.posts\n---\n",
            &[("set-aside", "site.posts")],
        ),
        (
            "content/folder-1/index.md",
            "each: data.items\n---\n",
            &[("set-aside", "data.items")],
        ),
    ];
    for (path, rest, said) in cases {
        let read = site(&[
            (path, &format!("---\nredirect_to: /post-2/\n{rest}")),
            ("content/folder-1/post-2.md", "---\ndate: 2026-01-01\n---\n"),
            ("data/items.yaml", "- name: Item 1\n"),
            ("statac.yaml", "language: en\ntimezone: UTC\n"),
        ]);
        let wanted: Vec<_> = said
            .iter()
            .map(|&(name, at)| (name, path.to_owned(), at.to_owned()))
            .collect();
        assert_eq!(problems(&read), wanted, "{path}: {rest}");
        assert!(
            read.site.pages.iter().all(|(_, page)| page.made.is_none()),
            "{path}: {rest}"
        );
    }
}

/// Where a redirect sends readers on the site, times what is published
/// there: nothing published is a warning at the address, offering the
/// nearest that something is.
#[test]
fn a_redirect_to_an_address_nothing_stands_at_is_warned_of() {
    // (where it sends readers, the suggestion where there is a warning)
    let cases = [
        ("/page-2/", None),
        ("/page-2#part-1", None),
        ("/page-2/index.html", None),
        ("/file-1.txt", None),
        ("/", None),
        ("https://example.org/page-22/", None),
        ("/page-22/", Some(Some("/page-2/"))),
        ("/page-22/#part-1", Some(Some("/page-2/#part-1"))),
        ("/Page-2/", Some(Some("/page-2/"))),
        ("/file-2.txt", Some(Some("/file-1.txt"))),
        ("/another-address-entirely/", Some(None)),
    ];
    for (to, warned) in cases {
        let temp = TempSite::site(&[
            ("content/index.md", "---\ntitle: Home\n---\n"),
            ("content/page-2.md", "---\ntitle: Page 2\n---\n"),
            ("content/old.md", &format!("---\nredirect_to: {to}\n---\n")),
            ("static/file-1.txt", "text"),
        ]);
        let (read, ..) = crate::build::read(&temp.root, &run_on(2026, 12, 31));
        let found: Vec<_> = read
            .problems
            .iter()
            .filter_map(|p| match &p.kind {
                Kind::BrokenLink {
                    target, suggestion, ..
                } => {
                    let at = read.sources.spelling(p.place.unwrap());
                    assert_eq!((target.as_str(), at), (to, to));
                    Some(suggestion.as_deref())
                }
                _ => None,
            })
            .collect();
        assert_eq!(found, Vec::from_iter(warned), "{to}");
    }
}

/// A page left out of the build, times what stands at its address: what
/// is published there already is a warning where the page is left out,
/// naming it.
#[test]
fn a_page_left_out_at_an_address_already_taken_is_warned_of() {
    const DRAFT: &str = "Address taken: this draft would be published at";
    const WAITING: &str = "Address taken: this post would be published at";
    // (the page left out's front matter, the text it is left out by, and
    // the headline where there is a warning)
    let cases = [
        ("draft: true", "true", None),
        ("draft: true\npermalink: /post-2/", "true", None),
        (
            "draft: true\npermalink: /post-1/",
            "true",
            Some(format!("{DRAFT} `/post-1/`, as `content/post-1.md` is")),
        ),
        (
            "draft: true\npermalink: /Post-1/",
            "true",
            Some(format!("{DRAFT} `/Post-1/`, as `content/post-1.md` is")),
        ),
        (
            "date: 2027-01-01\npermalink: /post-1/",
            "2027-01-01",
            Some(format!("{WAITING} `/post-1/`, as `content/post-1.md` is")),
        ),
        (
            "draft: true\npermalink: /file-1.txt",
            "true",
            Some(format!("{DRAFT} `/file-1.txt`, as `static/file-1.txt` is")),
        ),
        (
            "date: 2027-01-01\npermalink: /file-1.txt",
            "2027-01-01",
            Some(format!(
                "{WAITING} `/file-1.txt`, as `static/file-1.txt` is"
            )),
        ),
        (
            "draft: true\npermalink: /folder-1/",
            "true",
            Some(format!(
                "{DRAFT} `/folder-1/`, as the page for the folder `content/folder-1/` is"
            )),
        ),
        (
            "date: 2027-01-01\npermalink: /folder-1/",
            "2027-01-01",
            Some(format!(
                "{WAITING} `/folder-1/`, as the page for the folder `content/folder-1/` is"
            )),
        ),
    ];
    for (front, at, warned) in cases {
        let temp = TempSite::site(&[
            ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
            ("content/held.md", &format!("---\n{front}\n---\n")),
            ("content/folder-1/post-3.md", "---\ntitle: Post 3\n---\n"),
            ("static/file-1.txt", "text"),
            ("statac.yaml", "language: en\ntimezone: UTC\n"),
        ]);
        let (read, ..) = crate::build::read(&temp.root, &run_on(2026, 12, 31));
        let left_out = match at {
            "true" => "draft-left-out",
            _ => "waiting-for-its-day",
        };
        let mut wanted = vec![(left_out, "content/held.md".to_owned(), at.to_owned())];
        if warned.is_some() {
            wanted.push(("address-taken", "content/held.md".to_owned(), at.to_owned()));
        }
        assert_eq!(problems(&read), wanted, "{front}");
        let headlines: Vec<String> = (read.problems.iter())
            .filter(|p| p.kind.code() == Code::AddressTaken)
            .map(|p| crate::wording::headline(&p.kind))
            .collect();
        assert_eq!(headlines, Vec::from_iter(warned), "{front}");
    }
}

/// With `--drafts` a page left out is built, and two pages at one address
/// are told as that, not as an address taken.
#[test]
fn a_draft_built_at_an_address_already_taken_is_the_same_address() {
    let temp = TempSite::site(&[
        ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
        (
            "content/held.md",
            "---\ndraft: true\npermalink: /post-1/\n---\n",
        ),
    ]);
    let run = Run {
        drafts: true,
        ..run_on(2026, 12, 31)
    };
    let (read, ..) = crate::build::read(&temp.root, &run);
    let codes: Vec<Code> = read.problems.iter().map(|p| p.kind.code()).collect();
    assert_eq!(codes, [Code::SameAddress]);
}

/// Pages sending readers on to addresses that are themselves redirects,
/// times where each page's redirect sends readers: straight to where
/// following ends, with the fragment written at the start, or else the
/// first met.
#[test]
fn a_redirect_to_a_redirect_sends_readers_where_following_ends() {
    let on_site = |address: &str, fragment: &str| {
        Some(SentTo::Site {
            written: String::new(),
            address: Address::read(address).unwrap(),
            fragment: fragment.to_owned(),
        })
    };
    // (each page and its front matter, where the first page's redirect
    // sends readers)
    let cases = [
        (
            vec![
                ("post-1", "redirect_to: /post-2/"),
                ("post-2", "redirect_to: /post-3/"),
                ("post-3", "title: Post 3"),
            ],
            on_site("/post-3/", ""),
        ),
        (
            vec![
                ("post-1", "redirect_to: /post-2/"),
                ("post-2", "redirect_to: /POST-3/"),
                ("post-3", "redirect_to: /post-4/"),
                ("post-4", "title: Post 4"),
            ],
            on_site("/post-4/", ""),
        ),
        (
            vec![
                ("post-1", "redirect_to: /old/"),
                ("post-2", "redirect_from: [/old/]\nredirect_to: /post-3/"),
                ("post-3", "title: Post 3"),
            ],
            on_site("/post-3/", ""),
        ),
        (
            vec![
                ("post-1", "redirect_to: /post-2/"),
                ("post-2", "redirect_to: /old/"),
                ("post-3", "redirect_from: [/old/]"),
            ],
            on_site("/post-3/", ""),
        ),
        (
            vec![
                ("post-1", "redirect_to: /post-2/"),
                ("post-2", "redirect_to: https://example.org/post-2/"),
            ],
            Some(SentTo::Elsewhere("https://example.org/post-2/".into())),
        ),
        (
            vec![
                ("post-1", "redirect_to: /post-2/"),
                ("post-2", "redirect_to: /nothing/"),
            ],
            on_site("/nothing/", ""),
        ),
        (
            vec![
                ("post-1", "redirect_to: /post-2#part-1"),
                ("post-2", "redirect_to: /post-3#part-2"),
                ("post-3", "redirect_to: /post-4#part-3"),
            ],
            on_site("/post-4/", "#part-1"),
        ),
        (
            vec![
                ("post-1", "redirect_to: /post-2/"),
                ("post-2", "redirect_to: /post-3#part-2"),
                ("post-3", "redirect_to: /post-4#part-3"),
            ],
            on_site("/post-4/", "#part-2"),
        ),
        (
            vec![
                ("post-1", "redirect_to: /post-2#part-1"),
                ("post-2", "redirect_to: https://example.org/post-2/#part-2"),
            ],
            Some(SentTo::Elsewhere(
                "https://example.org/post-2/#part-1".into(),
            )),
        ),
        (
            vec![
                ("post-1", "redirect_to: /post-2/"),
                ("post-2", "redirect_to: https://example.org/post-2/#part-2"),
            ],
            Some(SentTo::Elsewhere(
                "https://example.org/post-2/#part-2".into(),
            )),
        ),
    ];
    for (pages, sent) in cases {
        let files: Vec<(String, String)> = (pages.iter())
            .map(|(name, front)| (format!("content/{name}.md"), format!("---\n{front}\n---\n")))
            .collect();
        let files: Vec<(&str, &str)> = (files.iter())
            .map(|(path, text)| (path.as_str(), text.as_str()))
            .collect();
        let read = site(&files);
        assert_eq!(problems(&read), [], "{pages:?}");
        let first = (read.site.redirects.iter())
            .find(|r| r.from.as_str() == "/post-1/")
            .map(|r| match &r.to {
                SentTo::Site {
                    address, fragment, ..
                } => on_site(address.as_str(), fragment).unwrap(),
                other => other.clone(),
            });
        assert_eq!(first, sent, "{pages:?}");
    }
}

/// An old address of a page that itself sends readers on sends them
/// straight to where the page does, in one step.
#[test]
fn an_old_address_of_a_page_sending_readers_on_sends_them_where_it_does() {
    let temp = TempSite::site(&[
        (
            "content/post-1.md",
            "---\nredirect_from: [/old/, /older/]\nredirect_to: /post-2#part-1\n---\n",
        ),
        ("content/post-2.md", "---\nredirect_to: /post-3/\n---\n"),
        ("content/post-3.md", "---\ntitle: Post 3\n---\n"),
        (
            "content/post-4.md",
            "---\nredirect_from: /was/\nredirect_to: https://example.org/a/\n---\n",
        ),
        (
            "content/post-5.md",
            "---\ntitle: Post 5\nredirect_from: [/once/]\n---\n",
        ),
    ]);
    let (read, ..) = crate::build::read(&temp.root, &run_on(2026, 12, 31));
    assert_eq!(problems(&read), []);
    let sent: Vec<(&str, String)> = (read.site.redirects.iter())
        .map(|redirect| {
            let to = match &redirect.to {
                SentTo::Page => "its page".to_owned(),
                SentTo::Site {
                    address, fragment, ..
                } => format!("{}{fragment}", address.as_str()),
                SentTo::Elsewhere(elsewhere) => elsewhere.clone(),
            };
            (redirect.from.as_str(), to)
        })
        .collect();
    assert_eq!(
        sent,
        [
            ("/post-1/", "/post-3/#part-1".to_owned()),
            ("/old/", "/post-3/#part-1".to_owned()),
            ("/older/", "/post-3/#part-1".to_owned()),
            ("/post-2/", "/post-3/".to_owned()),
            ("/post-4/", "https://example.org/a/".to_owned()),
            ("/was/", "https://example.org/a/".to_owned()),
            ("/once/", "its page".to_owned()),
        ]
    );
}

/// Following that comes back to the page it started from is refused at
/// each page in the ring; a page sending readers into the ring from
/// outside it is not.
#[test]
fn a_ring_of_redirects_is_refused_at_each_page_in_it() {
    // (each page and its front matter, the problems)
    let cases = [
        (
            vec![
                ("post-1", "redirect_to: /post-2/"),
                ("post-2", "redirect_to: /post-1/"),
            ],
            vec![("post-1", "/post-2/"), ("post-2", "/post-1/")],
        ),
        (
            vec![
                ("post-1", "redirect_to: /post-2/"),
                ("post-2", "redirect_to: /post-3/"),
                ("post-3", "redirect_to: /post-1#part-1"),
            ],
            vec![
                ("post-1", "/post-2/"),
                ("post-2", "/post-3/"),
                ("post-3", "/post-1#part-1"),
            ],
        ),
        (
            vec![("post-1", "redirect_from: [/old/]\nredirect_to: /old/")],
            vec![("post-1", "/old/")],
        ),
        (
            vec![
                ("post-1", "redirect_to: /post-2/"),
                ("post-2", "redirect_to: /post-3/"),
                ("post-3", "redirect_to: /post-2/"),
            ],
            vec![("post-2", "/post-3/"), ("post-3", "/post-2/")],
        ),
    ];
    for (pages, said) in cases {
        let files: Vec<(String, String)> = (pages.iter())
            .map(|(name, front)| (format!("content/{name}.md"), format!("---\n{front}\n---\n")))
            .collect();
        let files: Vec<(&str, &str)> = (files.iter())
            .map(|(path, text)| (path.as_str(), text.as_str()))
            .collect();
        let read = site(&files);
        let wanted: Vec<_> = (said.iter())
            .map(|(name, at)| {
                let path = format!("content/{name}.md");
                ("redirect-to-itself", path, at.to_string())
            })
            .collect();
        assert_eq!(problems(&read), wanted, "{pages:?}");
        let headlines: Vec<String> = (read.problems.iter())
            .map(|p| crate::wording::headline(&p.kind))
            .collect();
        let wanted: Vec<String> = (said.iter())
            .map(|(_, at)| format!("Redirect to itself: `{at}` sends readers back to this page"))
            .collect();
        assert_eq!(headlines, wanted, "{pages:?}");
        let left: Vec<&str> = (read.site.redirects.iter())
            .filter(|r| !matches!(r.to, SentTo::Page))
            .map(|r| r.from.as_str())
            .collect();
        let outside: Vec<String> = (pages.iter())
            .map(|(name, _)| format!("/{name}/"))
            .filter(|from| !said.iter().any(|(name, _)| format!("/{name}/") == *from))
            .collect();
        assert_eq!(left, outside, "{pages:?}");
    }
}

/// A redirect followed to an address nothing stands at is warned of once,
/// where that address is written.
#[test]
fn a_redirect_followed_to_nothing_is_warned_of_where_nothing_is_named() {
    let temp = TempSite::site(&[
        ("content/post-1.md", "---\nredirect_to: /post-2/\n---\n"),
        ("content/post-2.md", "---\nredirect_to: /nothing/\n---\n"),
    ]);
    let (read, ..) = crate::build::read(&temp.root, &run_on(2026, 12, 31));
    assert_eq!(
        problems(&read),
        [(
            "broken-link",
            "content/post-2.md".to_owned(),
            "/nothing/".to_owned()
        )]
    );
}

#[test]
fn a_site_naming_no_language_is_told_once_and_one_naming_any_is_not() {
    let told = |settings: &str| {
        let read = site(&[
            ("statac.yaml", settings),
            ("content/post-1.md", "Text.\n"),
            ("content/post-2.md", "Text.\n"),
        ]);
        problems(&read)
    };
    let note = [("no-language", String::new(), String::new())];
    assert_eq!(told(""), note);
    assert_eq!(told("title: Site 1\n"), note);
    assert_eq!(told("language: ~\n"), note);
    assert_eq!(told("language: en\n"), []);
    assert_eq!(told("language: en-GB\n"), []);
    assert!(!told("language: [ar, en]\n").contains(&note[0]));
    // What is told of the line as written is told alone.
    let at = |name, text: &str| (name, "statac.yaml".to_owned(), text.to_owned());
    assert_eq!(told("language:\n"), [at("set-aside", "language")]);
    assert_eq!(told("language: 12\n"), [at("wrong-kind-of-value", "12")]);

    let temp = TempSite::site(&[("content/post-1.md", "Text.\n")]);
    std::fs::remove_file(temp.root.join("statac.yaml")).unwrap();
    assert_eq!(problems(&read_site(&temp)), note);
    assert!(Kind::NoLanguage.quietenable());
}

#[cfg(unix)]
#[test]
fn a_file_beside_the_writing_is_read_where_its_link_led_when_read() {
    use std::os::unix::fs::symlink;
    let outside = TempSite::site(&[("content/notes.txt", "private\n")]);
    let temp = TempSite::site(&[
        ("content/post-1/index.md", "---\ntitle: Post 1\n---\n"),
        ("content/files/notes.txt", "notes\n"),
    ]);
    let content = temp.root.join("content");
    symlink(
        content.join("files/notes.txt"),
        content.join("post-1/notes.txt"),
    )
    .unwrap();
    let read = read_site(&temp);
    assert_eq!(read.problems, []);
    let open = |rel: &str| std::io::read_to_string(read.site.open_content(rel)?);
    // Pointed elsewhere since, the link isn't followed again.
    std::fs::remove_file(content.join("post-1/notes.txt")).unwrap();
    symlink(
        outside.root.join("content/notes.txt"),
        content.join("post-1/notes.txt"),
    )
    .unwrap();
    assert_eq!(open("post-1/notes.txt").unwrap(), "notes\n");
    // A link put in place of the file it led to is refused.
    std::fs::remove_file(content.join("files/notes.txt")).unwrap();
    symlink(
        outside.root.join("content/notes.txt"),
        content.join("files/notes.txt"),
    )
    .unwrap();
    for rel in ["post-1/notes.txt", "files/notes.txt"] {
        assert_eq!(Reason::of(&open(rel).unwrap_err()), Reason::Link, "{rel}");
    }
}
