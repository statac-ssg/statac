use std::sync::atomic::{AtomicUsize, Ordering as Counting};

use super::*;
use crate::content::{Read, Run};
use crate::date::Date;

/// A site written to a fresh folder and read on `today`, the folder removed
/// once read. A site that names no time zone is given UTC.
fn read_on(files: &[(&str, &str)], today: Date, drafts: bool) -> Read {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        "statac-lists-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Counting::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("content")).unwrap();
    let settings = files.iter().any(|(path, _)| *path == "statac.yaml");
    let zone = [("statac.yaml", "language: en\ntimezone: UTC\n")];
    let defaults = if settings { &[][..] } else { &zone[..] };
    for (path, text) in files.iter().chain(defaults) {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    let read = crate::content::read(
        &root,
        &Run {
            today: Some(today),
            drafts,
            url: None,
            renamed: Vec::new(),
        },
    );
    let _ = std::fs::remove_dir_all(&root);
    read
}

fn site(files: &[(&str, &str)]) -> Read {
    read_on(files, Date::day_only(2026, 10, 4), false)
}

fn post(date: &str) -> String {
    format!("---\ndate: {date}\n---\n")
}

fn list<'a>(read: &'a Read, key: &ListKey) -> &'a List {
    read.site
        .lists
        .iter()
        .find(|(_, list)| &list.key == key)
        .map(|(_, list)| list)
        .unwrap_or_else(|| panic!("no list {key:?}"))
}

fn folder(path: &str) -> ListKey {
    ListKey::Folder(path.to_owned())
}

fn tag(name: &str) -> ListKey {
    ListKey::Tag(name.to_owned())
}

/// Each page by its path under `content/`, or by its address where Statac
/// made it.
fn shown(read: &Read, pages: &[PageId]) -> Vec<String> {
    pages
        .iter()
        .map(|&id| {
            let page = read.site.pages.get(id);
            match page.place {
                PagePlace::Made => page.address.as_str().to_owned(),
                _ => page.path.clone(),
            }
        })
        .collect()
}

fn members(read: &Read, key: &ListKey) -> Vec<String> {
    shown(read, &list(read, key).members)
}

fn undated(read: &Read, key: &ListKey) -> Vec<String> {
    shown(read, &list(read, key).undated)
}

fn speaker(read: &Read, key: &ListKey) -> Option<String> {
    list(read, key)
        .page
        .map(|page| shown(read, &[page]).remove(0))
}

/// Each problem's name, the file it is in and the text it is at.
fn problems(read: &Read) -> Vec<(&'static str, String, String)> {
    read.problems
        .iter()
        .map(|p| match p.place {
            Some(place) => (
                p.kind.name(),
                read.sources.path(place.source).display().to_string(),
                read.sources.spelling(place).to_owned(),
            ),
            None => (p.kind.name(), String::new(), String::new()),
        })
        .collect()
}

fn address_of(read: &Read, path: &str) -> String {
    read.site
        .pages
        .iter()
        .find(|(_, page)| page.path == path)
        .map(|(_, page)| page.address.as_str().to_owned())
        .unwrap_or_else(|| panic!("no page {path}"))
}

#[test]
fn a_folder_lists_every_post_beneath_it_and_the_site_every_post() {
    let read = site(&[
        ("content/index.md", "---\ntitle: Home\n---\n"),
        ("content/about.md", "---\ntitle: About\n---\n"),
        (
            "content/posts/index.md",
            "---\ntitle: Posts\ndate: 2026-01-01\n---\n",
        ),
        ("content/posts/post-1.md", &post("2026-01-01")),
        ("content/posts/2024/post-2.md", &post("2024-05-01")),
        ("content/posts/2024/page-1.md", "# Page 1\n"),
        ("content/notes/post-3.md", &post("2025-01-01")),
    ]);
    assert_eq!(problems(&read), []);
    assert_eq!(
        members(&read, &folder("")),
        ["posts/post-1.md", "notes/post-3.md", "posts/2024/post-2.md"]
    );
    assert_eq!(
        undated(&read, &folder("")),
        ["about.md", "posts/2024/page-1.md"]
    );
    assert_eq!(
        members(&read, &folder("posts")),
        ["posts/post-1.md", "posts/2024/post-2.md"]
    );
    assert_eq!(undated(&read, &folder("posts")), ["posts/2024/page-1.md"]);
    assert_eq!(
        members(&read, &folder("posts/2024")),
        ["posts/2024/post-2.md"]
    );
    // A folder's own page speaks for it, dated or not, and is none of its posts.
    assert_eq!(
        speaker(&read, &folder("posts")),
        Some("posts/index.md".into())
    );
    // A folder inside another has a page only by its own `index.md`; one
    // at the top of `content/` has a page made where it has none.
    assert_eq!(speaker(&read, &folder("posts/2024")), None);
    assert_eq!(speaker(&read, &folder("notes")), Some("/notes/".into()));
    let made = list(&read, &folder("notes")).page.unwrap();
    let page = read.site.pages.get(made);
    assert!(page.title.is_none());
    assert_eq!(page.speaks_for, Some(page.folder));
}

#[test]
fn the_sites_own_list_has_a_page_made_where_it_has_no_index() {
    let read = site(&[
        ("content/about.md", "---\ntitle: About\n---\n"),
        ("content/posts/post-1.md", &post("2026-01-01")),
    ]);
    assert_eq!(problems(&read), []);
    assert_eq!(speaker(&read, &folder("")), Some("/".into()));
    let made = list(&read, &folder("")).page.unwrap();
    let page = read.site.pages.get(made);
    assert_eq!(page.place, PagePlace::Made);
    assert!(page.title.is_none());
    assert!(page.lands());
    assert_eq!(page.body, Span::new(0, 0));
    assert_eq!(members(&read, &folder("")), ["posts/post-1.md"]);
    // A page of the author's standing at the top of the site speaks
    // instead, whatever its file is called.
    let read = site(&[
        ("content/home.md", "---\ntitle: Home\npermalink: /\n---\n"),
        ("content/posts/post-1.md", &post("2026-01-01")),
    ]);
    assert_eq!(problems(&read), []);
    assert_eq!(speaker(&read, &folder("")), Some("home.md".into()));
    // A site with no pages has none made.
    let read = site(&[]);
    assert!(read.site.pages.is_empty());
}

/// A dated page standing at a list's address, times the list it speaks
/// for and where that is said.
#[test]
fn a_post_standing_at_a_lists_address_is_said_to_be_no_post() {
    let cases = [
        (
            "content/tags/rust.md",
            "---\ntitle: Rust\ndate: 2026-01-02\n---\n",
            "2026-01-02",
            "Not a post: `content/tags/rust.md` speaks for the tag `Rust`",
        ),
        (
            "content/notes.md",
            "---\ndate: 2026-01-02\npermalink: /notes/\n---\n",
            "2026-01-02",
            "Not a post: `content/notes.md` speaks for the folder `content/notes/`",
        ),
        (
            "content/2026-01-02-home.md",
            "---\npermalink: /\n---\n",
            "---",
            "Not a post: `content/2026-01-02-home.md` speaks for the site's own list",
        ),
    ];
    for (path, text, at, said) in cases {
        let read = site(&[
            (
                "content/notes/post-1.md",
                "---\ndate: 2026-01-01\ntags: [Rust]\n---\n",
            ),
            ("content/tags/index.md", "# Tags\n"),
            (path, text),
        ]);
        assert_eq!(
            problems(&read),
            [("not-a-post", path.to_owned(), at.to_owned())],
            "{path}"
        );
        assert_eq!(read.problems[0].headline(), said);
        assert_eq!(members(&read, &folder("")), ["notes/post-1.md"], "{path}");
    }
}

/// The settings for tag pages, times what they make.
#[test]
fn tags_and_series_have_pages_where_the_site_says() {
    // (settings, the tag's page, the series' page)
    let cases = [
        ("", Some("/tags/rust/"), Some("/series/series-1/")),
        (
            "tags.permalink: /topics/{slug}/\nseries.permalink: /runs/{slug}/\n",
            Some("/topics/rust/"),
            Some("/runs/series-1/"),
        ),
        ("tags.pages: false\n", None, Some("/series/series-1/")),
        ("series.pages: false\n", Some("/tags/rust/"), None),
        (
            "tags.slugs:\n  Rust: rust-lang\n",
            Some("/tags/rust-lang/"),
            Some("/series/series-1/"),
        ),
    ];
    for (settings, tag_page, series_page) in cases {
        let settings = format!("language: en\ntimezone: UTC\n{settings}");
        let read = site(&[
            ("statac.yaml", &settings),
            (
                "content/post-1.md",
                "---\ndate: 2026-01-01\ntags: [Rust]\nseries: Series 1\n---\n",
            ),
            (
                "content/post-2.md",
                "---\ndate: 2026-02-01\ntags: [Rust]\nseries: Series 1\n---\n",
            ),
        ]);
        assert_eq!(problems(&read), [], "{settings}");
        assert_eq!(
            speaker(&read, &tag("Rust")).as_deref(),
            tag_page,
            "{settings}"
        );
        let series = ListKey::Series("Series 1".into());
        assert_eq!(
            speaker(&read, &series).as_deref(),
            series_page,
            "{settings}"
        );
        // Tags still work where they have no pages.
        assert_eq!(members(&read, &tag("Rust")), ["post-2.md", "post-1.md"]);
        assert_eq!(members(&read, &series), ["post-1.md", "post-2.md"]);
    }
}

#[test]
fn a_made_page_is_untitled_and_placed_by_its_list() {
    let read = site(&[(
        "content/post-1.md",
        "---\ndate: 2026-01-01\ntags: [Rust]\n---\n",
    )]);
    let made = list(&read, &tag("Rust")).page.unwrap();
    let page = read.site.pages.get(made);
    assert_eq!(page.place, PagePlace::Made);
    assert_eq!(
        page.speaks_for,
        Some(read.site.spelt(page.language, &tag("Rust")).unwrap())
    );
    assert!(page.title.is_none());
}

#[test]
fn a_page_standing_at_a_lists_address_speaks_for_it() {
    let read = site(&[
        (
            "content/post-1.md",
            "---\ndate: 2026-01-01\ntags: [Rust]\n---\n",
        ),
        ("content/tags/rust.md", "---\ntitle: Rust posts\n---\n"),
        ("content/tags/index.md", "---\ntitle: Tags\n---\n"),
        ("content/home.md", "---\ntitle: Home\npermalink: /\n---\n"),
        (
            "content/notes.md",
            "---\ntitle: Notes\npermalink: /notes/\n---\n",
        ),
        ("content/notes/post-2.md", &post("2026-02-01")),
    ]);
    assert_eq!(problems(&read), []);
    assert_eq!(speaker(&read, &tag("Rust")), Some("tags/rust.md".into()));
    assert_eq!(speaker(&read, &folder("notes")), Some("notes.md".into()));
    assert_eq!(speaker(&read, &folder("")), Some("home.md".into()));
    assert!(
        read.site
            .pages
            .iter()
            .all(|(_, page)| page.place != PagePlace::Made)
    );
    // Speaking for a list, it is in none of the lists it would be in.
    assert_eq!(undated(&read, &folder("tags")), Vec::<String>::new());
    assert_eq!(undated(&read, &folder("")), Vec::<String>::new());
}

/// How pages spell a tag, times the spelling it goes by and what is said.
#[test]
fn a_tag_spelled_two_ways_is_one_list_under_the_commoner() {
    // (each page's tags, the spelling chosen, the spellings warned of with
    // their counts, the text the warning is at)
    type Case = (&'static [&'static str], &'static str, Counted, &'static str);
    type Counted = &'static [(&'static str, usize)];
    let cases: [Case; 4] = [
        (
            &["Rust", "Rust", "rust"],
            "Rust",
            &[("Rust", 2), ("rust", 1)],
            "rust",
        ),
        (
            &["rust", "rust", "Rust"],
            "rust",
            &[("rust", 2), ("Rust", 1)],
            "Rust",
        ),
        // Of two as common, capitals come first.
        (
            &["rust", "Rust"],
            "Rust",
            &[("Rust", 1), ("rust", 1)],
            "rust",
        ),
        (
            &["RUST", "Rust", "rust", "Rust"],
            "Rust",
            &[("Rust", 2), ("RUST", 1), ("rust", 1)],
            "RUST",
        ),
    ];
    for (spellings, chosen, warned, at) in cases {
        let files: Vec<(String, String)> = spellings
            .iter()
            .enumerate()
            .map(|(i, spelling)| {
                (
                    format!("content/post-{}.md", i + 1),
                    format!("---\ndate: 2026-01-0{}\ntags: [{spelling}]\n---\n", i + 1),
                )
            })
            .collect();
        let files: Vec<(&str, &str)> = files
            .iter()
            .map(|(path, text)| (path.as_str(), text.as_str()))
            .collect();
        let read = site(&files);
        let tags: Vec<&ListKey> = read
            .site
            .lists
            .iter()
            .filter(|(_, list)| matches!(list.key, ListKey::Tag(_)))
            .map(|(_, list)| &list.key)
            .collect();
        assert_eq!(tags, [&tag(chosen)], "{spellings:?}");
        assert_eq!(members(&read, &tag(chosen)).len(), spellings.len());
        let [problem] = &read.problems[..] else {
            panic!("{spellings:?}: {:?}", read.problems);
        };
        assert_eq!(
            problem.kind,
            Kind::SpelledTwoWays {
                word: Word::Tag,
                spellings: warned.iter().map(|(s, n)| ((*s).to_owned(), *n)).collect(),
                chosen: chosen.to_owned(),
            }
        );
        assert_eq!(read.sources.spelling(problem.place.unwrap()), at);
        // Every spelling finds the one list.
        for spelling in spellings {
            assert!(read.site.spelt(Id::from_index(0), &tag(spelling)).is_some());
        }
    }
}

#[test]
fn a_series_spelled_two_ways_is_one_series() {
    let read = site(&[
        (
            "content/post-1.md",
            "---\ndate: 2026-01-01\nseries: Book 1\n---\n",
        ),
        (
            "content/post-2.md",
            "---\ndate: 2026-01-02\nseries: Book 1\n---\n",
        ),
        (
            "content/post-3.md",
            "---\ndate: 2026-01-03\nseries: book 1\n---\n",
        ),
    ]);
    assert_eq!(
        problems(&read),
        [(
            "spelled-two-ways",
            "content/post-3.md".to_owned(),
            "book 1".to_owned()
        )]
    );
    assert_eq!(
        members(&read, &ListKey::Series("Book 1".into())),
        ["post-1.md", "post-2.md", "post-3.md"]
    );
}

/// Posts' dates and names, times the order a list puts them in.
#[test]
fn posts_are_ordered_by_the_moment_their_dates_name() {
    // (time zone, each post's file and date, the site's list newest first)
    type Case = (&'static str, Dated, &'static [&'static str]);
    type Dated = &'static [(&'static str, &'static str)];
    let cases: [Case; 5] = [
        // One day and no time: A to Z by file name, capitals aside.
        (
            "UTC",
            &[
                ("b.md", "2026-01-01"),
                ("C.md", "2026-01-01"),
                ("a.md", "2026-01-01"),
            ],
            &["a.md", "b.md", "C.md"],
        ),
        // A time on the date wins over the name.
        (
            "UTC",
            &[("a.md", "2026-01-01"), ("b.md", "2026-01-01 09:00")],
            &["b.md", "a.md"],
        ),
        (
            "UTC",
            &[("a.md", "2026-01-01 10:00"), ("b.md", "2026-01-01 09:00")],
            &["a.md", "b.md"],
        ),
        // A date with no offset is read on the site's clock.
        (
            "UTC",
            &[
                ("a.md", "2026-01-01T23:45+00:00"),
                ("b.md", "2026-01-02 00:30"),
            ],
            &["b.md", "a.md"],
        ),
        (
            "Europe/Berlin",
            &[
                ("a.md", "2026-01-01T23:45+00:00"),
                ("b.md", "2026-01-02 00:30"),
            ],
            &["a.md", "b.md"],
        ),
    ];
    for (zone, posts, newest_first) in cases {
        let settings = format!("language: en\ntimezone: {zone}\n");
        let mut files: Vec<(String, String)> = vec![("statac.yaml".into(), settings)];
        for (name, date) in posts {
            files.push((format!("content/{name}"), post(date)));
        }
        let files: Vec<(&str, &str)> = files
            .iter()
            .map(|(p, t)| (p.as_str(), t.as_str()))
            .collect();
        let read = site(&files);
        assert_eq!(problems(&read), [], "{posts:?}");
        assert_eq!(
            members(&read, &folder("")),
            newest_first,
            "{zone} {posts:?}"
        );
    }
}

#[test]
fn names_are_a_to_z_with_capitals_aside_until_they_are_all_that_differs() {
    assert_eq!(a_to_z("a", "B"), Ordering::Less);
    assert_eq!(a_to_z("b", "A"), Ordering::Greater);
    assert_eq!(a_to_z("A", "a"), Ordering::Less);
    assert_eq!(a_to_z("post-2", "Post-10"), Ordering::Greater);
}

/// What a list's page says of its order, times the order it takes.
#[test]
fn a_lists_page_chooses_which_way_it_runs() {
    let files = |series_page: &str, folder_page: &str| {
        vec![
            (
                "content/series/series-1.md".to_owned(),
                series_page.to_owned(),
            ),
            ("content/blog/index.md".to_owned(), folder_page.to_owned()),
            (
                "content/blog/post-1.md".to_owned(),
                "---\ndate: 2026-01-01\nseries: Series 1\n---\n".to_owned(),
            ),
            (
                "content/blog/post-2.md".to_owned(),
                "---\ndate: 2026-02-01\nseries: Series 1\n---\n".to_owned(),
            ),
        ]
    };
    // (the series' page, the folder's page, the series' order, the folder's)
    let cases = [
        (
            "# Series 1\n",
            "# Blog\n",
            ["post-1", "post-2"],
            ["post-2", "post-1"],
        ),
        (
            "---\norder: newest first\n---\n",
            "---\norder: oldest first\n---\n",
            ["post-2", "post-1"],
            ["post-1", "post-2"],
        ),
    ];
    for (series_page, folder_page, series_order, folder_order) in cases {
        let files = files(series_page, folder_page);
        let files: Vec<(&str, &str)> = files
            .iter()
            .map(|(p, t)| (p.as_str(), t.as_str()))
            .collect();
        let read = site(&files);
        assert_eq!(problems(&read), []);
        let series = ListKey::Series("Series 1".into());
        assert_eq!(speaker(&read, &series), Some("series/series-1.md".into()));
        let named = |key: &ListKey| -> Vec<String> {
            members(&read, key)
                .into_iter()
                .map(|path| {
                    path.trim_start_matches("blog/")
                        .trim_end_matches(".md")
                        .to_owned()
                })
                .collect()
        };
        assert_eq!(named(&series), series_order);
        assert_eq!(named(&folder("blog")), folder_order);
    }
}

#[test]
fn a_name_opens_with_a_number_and_a_hyphen() {
    let cases = [
        ("03-day-one", Some((3, "day-one"))),
        ("07-50-notes", Some((7, "50-notes"))),
        ("10-x", Some((10, "x"))),
        ("day-one", None),
        ("03day", None),
        ("03-", None),
        ("2026-01-02-post-1", None),
        ("-1-a", None),
    ];
    for (name, wanted) in cases {
        assert_eq!(numbered(name), wanted, "{name}");
    }
}

/// What a folder holds, times whether it is read as numbered and where its
/// pages land.
#[test]
fn a_folder_is_numbered_only_when_every_page_in_it_is() {
    // (files in `content/days/`, its `index.md`, numbered, the folder's list,
    // the address of its first file)
    type Names = &'static [&'static str];
    let cases: [(Names, &str, bool, Names, &str); 7] = [
        (
            &["10-day-ten.md", "9-day-nine.md", "01-day-one.md"],
            "",
            true,
            &[
                "days/01-day-one.md",
                "days/9-day-nine.md",
                "days/10-day-ten.md",
            ],
            "/days/day-ten/",
        ),
        (
            &["07-50-notes.md"],
            "",
            true,
            &["days/07-50-notes.md"],
            "/days/50-notes/",
        ),
        (
            &["02-b/index.md", "01-a.md"],
            "",
            true,
            &["days/01-a.md", "days/02-b/index.md"],
            "/days/b/",
        ),
        (
            &["01-day-one.md", "day-two.md"],
            "",
            false,
            &[],
            "/days/01-day-one/",
        ),
        (
            &["01-day-one.md", "02-day-two.md"],
            "---\nnumbered: false\n---\n",
            false,
            &[],
            "/days/01-day-one/",
        ),
        (
            &["2026-01-01-post-1.md"],
            "",
            false,
            &["days/2026-01-01-post-1.md"],
            "/days/post-1/",
        ),
        (&["day-one.md"], "", false, &[], "/days/day-one/"),
    ];
    for (names, index, numbered, listed, first) in cases {
        let mut files: Vec<(String, String)> = names
            .iter()
            .map(|name| (format!("content/days/{name}"), "# Day\n".to_owned()))
            .collect();
        if !index.is_empty() {
            files.push(("content/days/index.md".into(), index.into()));
        }
        let files: Vec<(&str, &str)> = files
            .iter()
            .map(|(p, t)| (p.as_str(), t.as_str()))
            .collect();
        let read = site(&files);
        let noted: Vec<_> = problems(&read)
            .into_iter()
            .filter(|(name, _, _)| *name == "numbered-folder")
            .collect();
        assert_eq!(noted.len(), usize::from(numbered), "{names:?}");
        assert_eq!(list(&read, &folder("days")).numbered, numbered, "{names:?}");
        assert_eq!(members(&read, &folder("days")), listed, "{names:?}");
        assert_eq!(
            address_of(&read, &format!("days/{}", names[0])),
            first,
            "{names:?}"
        );
    }
}

#[test]
fn a_dated_page_in_a_numbered_folder_is_still_a_post_in_date_order() {
    let read = site(&[
        ("content/days/01-day-one.md", &post("2026-02-01")),
        ("content/days/02-day-two.md", "# Day two\n"),
        ("content/days/03-day-three.md", &post("2026-01-01")),
        ("content/post-1.md", &post("2026-01-15")),
    ]);
    assert_eq!(
        problems(&read),
        [("numbered-folder", String::new(), String::new())]
    );
    assert_eq!(
        members(&read, &folder("days")),
        [
            "days/01-day-one.md",
            "days/02-day-two.md",
            "days/03-day-three.md"
        ]
    );
    assert_eq!(
        members(&read, &folder("")),
        ["days/01-day-one.md", "post-1.md", "days/03-day-three.md"]
    );
    assert_eq!(undated(&read, &folder("")), ["days/02-day-two.md"]);
}

#[test]
fn an_order_list_puts_a_folder_in_order_by_hand() {
    let read = site(&[
        (
            "content/days/index.md",
            "---\norder: [day-three, day-for, day-one]\n---\n",
        ),
        ("content/days/day-one.md", "# Day one\n"),
        ("content/days/day-two.md", "# Day two\n"),
        ("content/days/day-three.md", &post("2026-01-01")),
        ("content/days/day-four.md", "# Day four\n"),
    ]);
    assert_eq!(
        members(&read, &folder("days")),
        [
            "days/day-three.md",
            "days/day-one.md",
            "days/day-four.md",
            "days/day-two.md"
        ]
    );
    assert_eq!(
        problems(&read),
        [
            (
                "unknown-word",
                "content/days/index.md".to_owned(),
                "day-for".to_owned()
            ),
            (
                "unordered-pages",
                "content/days/index.md".to_owned(),
                "[day-three, day-for, day-one]".to_owned()
            ),
        ]
    );
    assert_eq!(
        read.problems[0].kind,
        Kind::UnknownWord {
            word: Word::Page,
            written: "day-for".into(),
            nearest: Some("day-four".into()),
        }
    );
    assert_eq!(
        read.problems[1].kind,
        Kind::UnorderedPages {
            names: vec!["day-four".into(), "day-two".into()],
        }
    );
}

#[test]
fn numbered_is_a_folders_own_setting() {
    let read = site(&[("content/post-1.md", "---\nnumbered: false\n---\n")]);
    assert_eq!(
        problems(&read),
        [(
            "wrong-place",
            "content/post-1.md".to_owned(),
            "numbered".to_owned()
        )]
    );
}

#[test]
fn a_page_naming_its_posts_speaks_for_them() {
    let read = site(&[
        (
            "content/blog/post-1.md",
            "---\ndate: 2026-01-01\ntags: [Rust]\nlang: en\n---\n",
        ),
        (
            "content/blog/post-2.md",
            "---\ndate: 2026-02-01\nlang: [de, en]\n---\n",
        ),
        (
            "content/notes/post-3.md",
            "---\ndate: 2026-03-01\ntags: [rust]\nlang: de\n---\n",
        ),
        (
            "content/rust.md",
            "---\nposts: site.posts | in(\"blog\") | tagged(\"Rust\")\n---\n",
        ),
        (
            "content/english.md",
            "---\nposts: site.posts | where('lang', 'en')\norder: oldest first\n---\n",
        ),
        (
            "content/year.md",
            "---\nposts: site.posts | where(\"year\", \"2026\") | in(notes)\n---\n",
        ),
    ]);
    let declared = |path: &str| {
        let (id, _) = read
            .site
            .pages
            .iter()
            .find(|(_, page)| page.path == path)
            .unwrap();
        ListKey::Declared(id)
    };
    assert_eq!(members(&read, &declared("rust.md")), ["blog/post-1.md"]);
    assert_eq!(
        members(&read, &declared("english.md")),
        ["blog/post-1.md", "blog/post-2.md"]
    );
    assert_eq!(members(&read, &declared("year.md")), ["notes/post-3.md"]);
    assert_eq!(speaker(&read, &declared("rust.md")), Some("rust.md".into()));
    // `Rust` and `rust` are one tag, which this warns of and nothing more.
    assert_eq!(
        problems(&read)
            .into_iter()
            .map(|(name, _, _)| name)
            .collect::<Vec<_>>(),
        ["spelled-two-ways"]
    );
}

/// Mistakes in a `posts:` line, times the problem and where it is drawn.
#[test]
fn a_mistake_in_a_posts_line_is_placed_in_the_line() {
    let cases = [
        (
            "site.pages | tagged(\"Rust\")",
            "malformed-list",
            "site.pages",
        ),
        ("site.posts | tagged(\"Rust\"", "malformed-list", "("),
        ("site.posts | tagged(\"Rust)", "malformed-list", "\""),
        ("site.posts | | tagged(\"Rust\")", "malformed-list", "|"),
        ("site.posts | sort", "malformed-list", "sort"),
        ("site.posts | tagged", "malformed-list", "tagged"),
        (
            "site.posts | where(\"lang\")",
            "malformed-list",
            "where(\"lang\")",
        ),
        ("site.posts | taged(\"Rust\")", "unknown-word", "taged"),
        ("site.posts | tagged(\"Rusty\")", "unknown-word", "Rusty"),
        ("site.posts | in(\"blgo\")", "unknown-word", "blgo"),
        (
            "site.posts | where(\"lnag\", \"en\")",
            "unknown-word",
            "lnag",
        ),
    ];
    for (line, kind, at) in cases {
        let front = format!("---\nposts: {line}\n---\n");
        let read = site(&[
            (
                "content/blog/post-1.md",
                "---\ndate: 2026-01-01\ntags: [Rust]\nlang: en\n---\n",
            ),
            ("content/list.md", &front),
        ]);
        assert_eq!(
            problems(&read),
            [(kind, "content/list.md".to_owned(), at.to_owned())],
            "{line}"
        );
    }
}

/// A draft or a waiting post, times what a run makes of it.
#[test]
fn drafts_and_posts_waiting_for_their_day_are_left_out_and_said() {
    let files = [
        ("statac.yaml", "language: en\ntimezone: Europe/Berlin\n"),
        (
            "content/post-1.md",
            "---\ndate: 2026-10-01\ntags: [Rust]\n---\n",
        ),
        (
            "content/post-2.md",
            "---\ndate: 2026-10-05\ntags: [Later]\n---\n",
        ),
        ("content/post-3.md", "---\ndraft: true\n---\n"),
        ("content/drafts/index.md", "---\npages.draft: true\n---\n"),
        ("content/drafts/post-4.md", &post("2026-01-01")),
        // 23:30 on the 4th in UTC is already the 5th in Berlin.
        ("content/post-5.md", &post("2026-10-04T23:30+00:00")),
    ];
    let read = read_on(&files, Date::day_only(2026, 10, 4), false);
    assert_eq!(
        problems(&read),
        [
            (
                "waiting-for-its-day",
                "content/post-2.md".to_owned(),
                "2026-10-05".to_owned()
            ),
            (
                "draft-left-out",
                "content/post-3.md".to_owned(),
                "true".to_owned()
            ),
            (
                "waiting-for-its-day",
                "content/post-5.md".to_owned(),
                "2026-10-04T23:30+00:00".to_owned()
            ),
            // Handed down, the draft is drawn where its folder writes it.
            (
                "draft-left-out",
                "content/drafts/index.md".to_owned(),
                "true".to_owned()
            ),
        ]
    );
    assert_eq!(
        read.problems[3].kind,
        Kind::DraftLeftOut {
            path: "content/drafts/post-4.md".into()
        }
    );
    assert_eq!(members(&read, &folder("")), ["post-1.md"]);
    assert!(read.site.spelt(Id::from_index(0), &tag("Later")).is_none());
    assert_eq!((read.drafts.len(), read.waiting.len()), (2, 2));
    assert!(read.problems.iter().all(|p| !p.kind.quietenable()));

    let read = read_on(&files, Date::day_only(2026, 10, 4), true);
    assert_eq!(problems(&read), []);
    assert_eq!(
        read.included,
        crate::content::Included {
            drafts: 2,
            waiting: 2
        }
    );
    assert_eq!(
        members(&read, &folder("")),
        ["post-5.md", "post-2.md", "post-1.md", "drafts/post-4.md"]
    );

    // On its day, a post is published.
    let read = read_on(&files, Date::day_only(2026, 10, 5), false);
    assert_eq!(read.waiting.len(), 0);
}

/// Where each later page of `key` stands.
fn later(read: &Read, key: &ListKey) -> Vec<String> {
    list(read, key)
        .later
        .iter()
        .map(|address| address.as_str().to_owned())
        .collect()
}

fn page_id(read: &Read, path: &str) -> PageId {
    read.site
        .pages
        .iter()
        .find(|(_, page)| page.path == path)
        .map(|(id, _)| id)
        .unwrap_or_else(|| panic!("no page {path}"))
}

/// How many posts a list's pages show, from the site or its page, times
/// where the list's page stands and where its later pages go.
#[test]
fn a_list_goes_on_onto_later_pages_beneath_its_own() {
    let tagged = |date: &str| format!("---\ndate: {date}\ntags: [Rust]\nseries: Series 1\n---\n");
    let posts = [
        ("content/blog/post-1.md", tagged("2026-01-01")),
        ("content/blog/post-2.md", tagged("2026-02-01")),
        ("content/blog/post-3.md", tagged("2026-03-01")),
    ];
    let blog = || Some(folder("blog"));
    // (settings, a page and what it says, the list where it isn't the
    // page's own, its posts on each page, where its later pages stand)
    type Case<'a> = (
        &'a str,
        (&'a str, &'a str),
        Option<ListKey>,
        usize,
        &'a [&'a str],
    );
    let cases: [Case; 17] = [
        ("", ("blog/index.md", ""), blog(), 10, &[]),
        (
            "posts_per_page: 2\n",
            ("blog/index.md", ""),
            blog(),
            2,
            &["/blog/page/2/"],
        ),
        // A folder's page its `permalink` moves goes on beneath where it
        // stands, at a file beneath its name as a folder.
        (
            "posts_per_page: 2\n",
            ("blog/index.md", "permalink: /posts/\n"),
            blog(),
            2,
            &["/posts/page/2/"],
        ),
        (
            "posts_per_page: 2\n",
            ("blog/index.md", "permalink: /posts.html\n"),
            blog(),
            2,
            &["/posts/page/2/"],
        ),
        (
            "posts_per_page: 2\n",
            ("index.md", "permalink: /start/\n"),
            Some(folder("")),
            2,
            &["/start/page/2/"],
        ),
        (
            "posts_per_page: 2\n",
            ("blog/index.md", "posts_per_page: 1\n"),
            blog(),
            1,
            &["/blog/page/2/", "/blog/page/3/"],
        ),
        (
            "posts_per_page: 2\n",
            ("blog/index.md", "posts_per_page: 0\n"),
            blog(),
            0,
            &[],
        ),
        ("posts_per_page: 0\n", ("blog/index.md", ""), blog(), 0, &[]),
        (
            "posts_per_page: 2\nlater_pages: \"{list}{n}/\"\n",
            ("blog/index.md", ""),
            blog(),
            2,
            &["/blog/2/"],
        ),
        (
            "posts_per_page: 2\n",
            ("blog/index.md", "index_page: false\n"),
            blog(),
            2,
            &[],
        ),
        // A folder at the top with no `index.md` goes on from the page
        // made for it.
        (
            "posts_per_page: 2\n",
            ("about.md", ""),
            blog(),
            2,
            &["/blog/page/2/"],
        ),
        (
            "posts_per_page: 2\n",
            ("index.md", ""),
            Some(folder("")),
            2,
            &["/page/2/"],
        ),
        (
            "posts_per_page: 2\n",
            ("about.md", ""),
            Some(tag("Rust")),
            2,
            &["/tags/rust/page/2/"],
        ),
        (
            "posts_per_page: 2\n",
            ("about.md", ""),
            Some(ListKey::Series("Series 1".into())),
            2,
            &["/series/series-1/page/2/"],
        ),
        (
            "posts_per_page: 2\n",
            ("picks.md", "posts: site.posts | in(\"blog\")\n"),
            None,
            2,
            &["/picks/page/2/"],
        ),
        // A page at a file pages beneath its name as a folder.
        (
            "posts_per_page: 2\n",
            (
                "journal.md",
                "posts: site.posts\npermalink: /journal.html\n",
            ),
            None,
            2,
            &["/journal/page/2/"],
        ),
        // A page published as something other than HTML shows every post.
        (
            "posts_per_page: 2\n",
            ("rust.md", "posts: site.posts\npermalink: /rust.xml\n"),
            None,
            0,
            &[],
        ),
    ];
    for (settings, (path, said), key, per_page, wanted) in cases {
        let settings = format!("language: en\ntimezone: UTC\n{settings}");
        let page = format!("---\ntitle: Page\n{said}---\n");
        let file = format!("content/{path}");
        let mut files: Vec<(&str, &str)> = posts.iter().map(|(p, t)| (*p, t.as_str())).collect();
        files.push(("statac.yaml", &settings));
        files.push((&file, &page));
        let read = site(&files);
        assert_eq!(problems(&read), [], "{settings}{path}");
        let key = key.unwrap_or_else(|| ListKey::Declared(page_id(&read, path)));
        assert_eq!(list(&read, &key).per_page, per_page, "{settings}{path}");
        assert_eq!(later(&read, &key), wanted, "{settings}{path}");
    }
}

#[test]
fn a_page_standing_where_a_later_page_goes_is_told() {
    let read = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\nposts_per_page: 1\n",
        ),
        ("content/blog/index.md", "---\ntitle: Blog\n---\n"),
        ("content/blog/post-1.md", &post("2026-01-01")),
        ("content/blog/post-2.md", &post("2026-02-01")),
        ("content/blog/page/2.md", "---\ntitle: Page 2\n---\n"),
    ]);
    assert_eq!(
        read.problems.iter().map(|p| &p.kind).collect::<Vec<_>>(),
        [&Kind::LaterPageTaken {
            list: "/blog/".into(),
            number: 2,
            page: Box::new("content/blog/page/2.md".into()),
            page_at: None,
            address: "/blog/page/2/".into(),
        }]
    );
    // Told once, as a later page, and not again among the addresses.
    let claims = read.claims();
    let at = claims
        .iter()
        .filter(|(file, _)| file == "blog/page/2/index.html");
    assert_eq!(at.count(), 1);
}

#[test]
fn a_later_page_taken_is_told_at_the_setting_placing_it_or_else_at_the_page() {
    // (what the site sets, the page in the way, where it is told, and the
    // place shown beneath)
    type Case<'a> = (&'a str, &'a str, [&'a str; 2], Option<[&'a str; 2]>);
    let cases: [Case; 2] = [
        (
            "",
            "content/blog/page/2.md",
            ["content/blog/page/2.md", "---"],
            None,
        ),
        (
            "later_pages: \"{list}more/{n}/\"\n",
            "content/blog/more/2.md",
            ["statac.yaml", "\"{list}more/{n}/\""],
            Some(["content/blog/more/2.md", "---"]),
        ),
    ];
    for (setting, path, [file, written], beneath) in cases {
        let settings = format!("language: en\ntimezone: UTC\nposts_per_page: 1\n{setting}");
        let read = site(&[
            ("statac.yaml", &settings),
            ("content/blog/index.md", "---\ntitle: Blog\n---\n"),
            ("content/blog/post-1.md", &post("2026-01-01")),
            ("content/blog/post-2.md", &post("2026-02-01")),
            (path, "---\ntitle: Page 2\n---\n"),
        ]);
        assert_eq!(
            problems(&read),
            [("same-address", file.to_owned(), written.to_owned())],
            "{setting}"
        );
        let related: Vec<(String, String, String)> = read.problems[0]
            .related()
            .into_iter()
            .map(|related| {
                let place = related.place.expect("placed");
                (
                    related.label,
                    read.sources.path(place.source).display().to_string(),
                    read.sources.spelling(place).to_owned(),
                )
            })
            .collect();
        let wanted: Vec<(String, String, String)> = beneath
            .map(|[file, written]| ("Also published there".into(), file.into(), written.into()))
            .into_iter()
            .collect();
        assert_eq!(related, wanted, "{setting}");
    }
}

/// Which run a page walks, as it says or by default, and where it stands
/// in it.
#[test]
fn a_page_walks_its_series_or_its_outermost_folder_in_reading_order() {
    // (the page, what it says, its previous and next)
    type Case<'a> = (&'a str, &'a str, [Option<&'a str>; 2]);
    let cases: [Case; 10] = [
        (
            "blog/post-2.md",
            "",
            [Some("blog/post-3.md"), Some("blog/post-1.md")],
        ),
        ("blog/post-1.md", "", [None, Some("blog/post-3.md")]),
        ("blog/post-3.md", "", [Some("blog/post-1.md"), None]),
        (
            "blog/post-1.md",
            "neighbours: folder\n",
            [Some("blog/post-2.md"), Some("blog/2024/post-5.md")],
        ),
        (
            "blog/post-3.md",
            "neighbours: site\n",
            [Some("notes/post-4.md"), Some("blog/post-2.md")],
        ),
        ("blog/post-1.md", "neighbours: false\n", [None, None]),
        // Asked to walk a series it isn't in, a post walks none.
        ("blog/post-2.md", "neighbours: series\n", [None, None]),
        ("blog/2024/post-5.md", "", [Some("blog/post-1.md"), None]),
        (
            "book/02-two.md",
            "",
            [Some("book/01-one.md"), Some("book/03-three.md")],
        ),
        ("book/01-one.md", "", [None, Some("book/02-two.md")]),
    ];
    for (path, said, wanted) in cases {
        let mut files = vec![
            ("blog/post-1.md", "date: 2026-01-01\nseries: Series 1\n"),
            ("blog/post-2.md", "date: 2026-02-01\n"),
            ("blog/post-3.md", "date: 2026-03-01\nseries: Series 1\n"),
            ("blog/2024/post-5.md", "date: 2024-01-01\n"),
            ("notes/post-4.md", "date: 2026-04-01\n"),
            ("book/index.md", "title: Book\n"),
            ("book/01-one.md", "title: One\n"),
            ("book/02-two.md", "title: Two\n"),
            ("book/03-three.md", "title: Three\n"),
        ]
        .into_iter()
        .map(|(file, front)| {
            let said = if file == path { said } else { "" };
            (
                format!("content/{file}"),
                format!("---\n{front}{said}---\n"),
            )
        })
        .collect::<Vec<_>>();
        files.push(("statac.yaml".into(), "language: en\ntimezone: UTC\n".into()));
        let files: Vec<(&str, &str)> = files
            .iter()
            .map(|(p, t)| (p.as_str(), t.as_str()))
            .collect();
        let read = site(&files);
        let found = read.site.neighbours[page_id(&read, path).index()]
            .map(|page| page.map(|page| read.site.pages.get(page).path.clone()));
        assert_eq!(
            found,
            wanted.map(|page| page.map(str::to_owned)),
            "{path} {said}"
        );
    }
}

#[test]
fn related_posts_share_the_most_tags_first_then_keep_the_sites_order() {
    let tagged = |date: &str, tags: &str| format!("---\ndate: {date}\ntags: [{tags}]\n---\n");
    let read = site(&[
        ("content/post-1.md", &tagged("2026-01-01", "Rust, Go")),
        ("content/post-2.md", &tagged("2026-02-01", "Rust")),
        ("content/post-3.md", &tagged("2026-03-01", "Go, Rust")),
        ("content/post-4.md", &tagged("2026-04-01", "C")),
        ("content/post-5.md", &tagged("2026-05-01", "Go")),
        ("content/post-6.md", &post("2026-06-01")),
    ]);
    let related = |path: &str| {
        let mut groups = super::related(&read.site, page_id(&read, path));
        for group in &mut groups {
            super::in_site_order(&read.site, group);
        }
        shown(&read, &groups.concat())
    };
    assert_eq!(
        related("post-1.md"),
        ["post-3.md", "post-5.md", "post-2.md"]
    );
    assert_eq!(related("post-2.md"), ["post-3.md", "post-1.md"]);
    assert_eq!(related("post-4.md"), Vec::<String>::new());
    assert_eq!(related("post-6.md"), Vec::<String>::new());
}

/// A page sending readers on is no post: it joins no list, and has no
/// neighbours and no related posts.
#[test]
fn a_page_sending_readers_on_is_in_no_list() {
    let read = site(&[
        (
            "content/post-1.md",
            "---\ndate: 2026-01-01\ntags: [Tag 1]\n---\n",
        ),
        (
            "content/2026-01-02-post-2.md",
            "---\ntags: [Tag 1, Tag 2]\nredirect_to: /post-1/\n---\n",
        ),
        (
            "content/post-3.md",
            "---\ntags: [Tag 1]\nredirect_to: /post-1/\n---\n",
        ),
    ]);
    let set_aside = |path: &str, at: &str| ("set-aside", path.to_owned(), at.to_owned());
    assert_eq!(
        problems(&read),
        [
            set_aside("content/2026-01-02-post-2.md", "[Tag 1, Tag 2]"),
            set_aside("content/post-3.md", "[Tag 1]"),
        ]
    );
    assert_eq!(members(&read, &folder("")), ["post-1.md"]);
    assert_eq!(undated(&read, &folder("")), Vec::<String>::new());
    assert_eq!(members(&read, &tag("Tag 1")), ["post-1.md"]);
    assert!(
        read.site
            .lists
            .iter()
            .all(|(_, list)| list.key != tag("Tag 2"))
    );
    let sending = page_id(&read, "2026-01-02-post-2.md");
    assert_eq!(read.site.neighbours[sending.index()], [None, None]);
    let post = page_id(&read, "post-1.md");
    assert_eq!(read.site.neighbours[post.index()], [None, None]);
    assert_eq!(super::related(&read.site, post), Vec::<Vec<PageId>>::new());
}

/// Where a page sending readers on speaks for a list, times what it
/// leaves there: its redirect, and no list pages.
#[test]
fn a_page_sending_readers_on_leaves_its_redirect_at_a_lists_address() {
    // (the page, the list it speaks for)
    let cases = [
        ("content/folder-1/index.md", folder("folder-1")),
        ("content/index.md", folder("")),
        ("content/tags/tag-1.md", tag("Tag 1")),
        ("content/page-1.md", folder("folder-2")),
    ];
    for (path, key) in cases {
        let front = match path {
            "content/page-1.md" => "permalink: /folder-2/\n",
            _ => "",
        };
        let read = site(&[
            (
                "statac.yaml",
                "language: en\ntimezone: UTC\nposts_per_page: 1\n",
            ),
            (
                path,
                &format!("---\ndate: 2026-01-03\n{front}redirect_to: /post-1/\n---\n"),
            ),
            (
                "content/folder-1/post-1.md",
                "---\ndate: 2026-01-01\ntags: [Tag 1]\n---\n",
            ),
            (
                "content/folder-1/post-2.md",
                "---\ndate: 2026-01-02\ntags: [Tag 1]\n---\n",
            ),
            ("content/folder-2/post-3.md", "---\ndate: 2026-01-02\n---\n"),
            ("content/folder-2/post-4.md", "---\ndate: 2026-01-03\n---\n"),
        ]);
        assert_eq!(problems(&read), [], "{path}");
        assert_eq!(
            speaker(&read, &key),
            Some(path.trim_start_matches("content/").to_owned()),
            "{path}"
        );
        assert_eq!(later(&read, &key), Vec::<String>::new(), "{path}");
        assert!(
            read.site
                .pages
                .iter()
                .all(|(_, page)| page.address != list_address(&read, &key)
                    || page.path == path.trim_start_matches("content/")),
            "{path}"
        );
    }
}

/// Where the page speaking for `key` stands.
fn list_address(read: &Read, key: &ListKey) -> Address {
    let page = list(read, key).page.expect("the list has a page");
    read.site.pages.get(page).address.clone()
}

/// The list of `key` in the language `index` declares.
fn list_in<'a>(read: &'a Read, key: &ListKey, index: usize) -> &'a List {
    read.site
        .lists
        .iter()
        .map(|(_, list)| list)
        .find(|list| &list.key == key && list.language.index() == index)
        .unwrap_or_else(|| panic!("no list {key:?} in language {index}"))
}

#[test]
fn a_folder_list_holds_only_its_own_languages_pages() {
    let read = site(&[
        ("statac.yaml", "language: [ar, en]\ntimezone: UTC\n"),
        ("content/post-1.md", &post("2026-01-01")),
        ("content/blog/post-2.md", &post("2026-01-02")),
        ("content/en/post-1.md", &post("2026-01-03")),
        ("content/en/blog/post-2.md", &post("2026-01-04")),
    ]);
    assert_eq!(problems(&read), []);
    assert_eq!(members(&read, &folder("")), ["blog/post-2.md", "post-1.md"]);
    assert_eq!(
        members(&read, &folder("en")),
        ["en/blog/post-2.md", "en/post-1.md"]
    );
    let site = &read.site;
    let own = |index| {
        site.own_list(Id::from_index(index))
            .map(|list| &site.lists.get(list).key)
    };
    assert_eq!((own(0), own(1)), (Some(&folder("")), Some(&folder("en"))));
    let feeds: Vec<String> = site
        .lists
        .iter()
        .filter_map(|(id, _)| crate::feeds::wanted(site, &read.settings, id))
        .map(|address| address.as_str().to_owned())
        .collect();
    assert_eq!(
        feeds,
        [
            "/feed.xml",
            "/blog/feed.xml",
            "/en/feed.xml",
            "/en/blog/feed.xml"
        ]
    );
}

#[test]
fn where_the_main_language_has_a_folder_the_top_holds_only_what_is_outside() {
    let read = site(&[
        ("statac.yaml", "language: [ar, en]\ntimezone: UTC\n"),
        ("content/index.md", "# Home\n"),
        ("content/about.md", &post("2026-01-01")),
        ("content/ar/post-1.md", &post("2026-01-02")),
        ("content/en/index.md", "# Home\n"),
    ]);
    assert_eq!(problems(&read), []);
    assert_eq!(members(&read, &folder("")), ["about.md"]);
    assert_eq!(members(&read, &folder("ar")), ["ar/post-1.md"]);
    let site = &read.site;
    let main = site.own_list(Id::from_index(0)).unwrap();
    assert_eq!(site.lists.get(main).key, folder("ar"));
    // The top is no language's own list: it has no feed of its own.
    let top = site
        .lists
        .iter()
        .find(|(_, list)| list.key == folder(""))
        .map(|(id, _)| id)
        .unwrap();
    assert_eq!(crate::feeds::wanted(site, &read.settings, top), None);
    // A language's folder holding only its `index.md` is still its top.
    let english = site.own_list(Id::from_index(1)).unwrap();
    assert_eq!(site.lists.get(english).key, folder("en"));
}

#[test]
fn a_tag_spelt_alike_in_two_languages_is_two_lists_at_two_addresses() {
    let tagged = "---\ndate: 2026-01-01\ntags: [Rust]\n---\n";
    let read = site(&[
        ("statac.yaml", "language: [ar, en]\ntimezone: UTC\n"),
        ("content/post-1.md", tagged),
        ("content/en/post-1.md", tagged),
    ]);
    assert_eq!(problems(&read), []);
    let address = |index| {
        let list = list_in(&read, &tag("Rust"), index);
        list.address.as_ref().map(|a| a.value.as_str().to_owned())
    };
    assert_eq!(address(0).as_deref(), Some("/tags/rust/"));
    assert_eq!(address(1).as_deref(), Some("/en/tags/rust/"));
    assert_eq!(
        shown(&read, &list_in(&read, &tag("Rust"), 1).members),
        ["en/post-1.md"]
    );
}

#[test]
fn a_pattern_opening_with_its_languages_folder_is_read_once_and_warned_of() {
    let read = site(&[
        (
            "statac.yaml",
            "language: [ar, en]\ntimezone: UTC\ntags.permalink: /en/tags/{slug}/\n",
        ),
        (
            "content/en/post-1.md",
            "---\ndate: 2026-01-01\ntags: [Rust]\npermalink: /EN/post-1/\n---\n",
        ),
        ("content/en/post-2.md", "---\npermalink: /post-2/\n---\n"),
    ]);
    assert_eq!(
        problems(&read),
        [
            (
                "language-folder-in-pattern",
                "content/en/post-1.md".to_owned(),
                "/EN/post-1/".to_owned()
            ),
            (
                "language-folder-in-pattern",
                "statac.yaml".to_owned(),
                "/en/tags/{slug}/".to_owned()
            ),
        ]
    );
    let Kind::LanguageFolderInPattern {
        folder, without, ..
    } = &read.problems[0].kind
    else {
        unreachable!()
    };
    assert_eq!((folder.as_str(), without.as_str()), ("/en/", "/post-1/"));
    assert_eq!(address_of(&read, "en/post-1.md"), "/en/post-1/");
    assert_eq!(address_of(&read, "en/post-2.md"), "/en/post-2/");
    let english = list_in(&read, &tag("Rust"), 1);
    let address = english.address.as_ref().map(|a| a.value.as_str());
    assert_eq!(address, Some("/en/tags/rust/"));
}
