use super::super::{Read, Run, read};
use crate::content::words::PageWord;
use crate::origin::Arrival;
use crate::problem::{AddressFault, Kind};
use crate::site::{Item, ListKey, Page};
use crate::testing::TempSite;

/// A site read, in a time zone unless its settings are given.
fn site(files: &[(&str, &str)]) -> Read {
    let mut files = files.to_vec();
    if !files.iter().any(|(path, _)| *path == "statac.yaml") {
        files.push(("statac.yaml", "language: en\ntimezone: Europe/London\n"));
    }
    read(&TempSite::site(&files).root, &run(false))
}

/// A run on 4 October 2026, with or without drafts.
fn run(drafts: bool) -> Run {
    Run {
        today: Some(crate::date::Date::day_only(2026, 10, 4)),
        drafts,
        url: None,
        renamed: Vec::new(),
    }
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
            let place = p.place.expect("placed");
            (
                p.kind.name(),
                read.sources.path(place.source).display().to_string(),
                read.sources.spelling(place).to_owned(),
            )
        })
        .collect()
}

/// Each made page's path and address.
fn made(read: &Read) -> Vec<(String, String)> {
    read.site
        .pages
        .iter()
        .filter(|(_, page)| page.made.is_some())
        .map(|(_, page)| (page.path.clone(), page.address.as_str().to_owned()))
        .collect()
}

const PEOPLE: &str = "---\ntitle: People\neach: data.people\npermalink: /people/{key}/\nlayout: person\n---\nThe people.\n";

#[test]
fn a_page_is_made_for_each_item_by_name() {
    let read = site(&[
        ("content/people.md", PEOPLE),
        (
            "data/people.yaml",
            "person-1:\n  title: Person 1\n  role: Role 1\nPerson 2:\n  date: 2026-10-04\nperson-3:\n  draft: true\n42:\n",
        ),
    ]);
    assert_eq!(
        problems(&read),
        [("draft-left-out", "data/people.yaml".into(), "true".into())]
    );
    assert_eq!(
        made(&read),
        [
            ("people.md/person-1".into(), "/people/person-1/".into()),
            ("people.md/Person 2".into(), "/people/person-2/".into()),
            ("people.md/42".into(), "/people/42/".into()),
        ]
    );
    let index = page(&read, "people.md");
    assert_eq!(index.address.as_str(), "/people/");
    assert!(index.lands());
    let one = page(&read, "people.md/person-1");
    assert_eq!(
        read.sources.path(one.source.unwrap()).to_str(),
        Some("data/people.yaml")
    );
    assert_eq!(one.title.as_ref().unwrap().value, "Person 1");
    assert_eq!(one.words.field("role").unwrap().name, "role");
    let layout = one.words.get(PageWord::Layout).unwrap();
    assert_eq!(layout.value.as_text(), Some("person"));
    assert_eq!(layout.origin.arrival, Arrival::HandedDown);
    let made = one.made.as_ref().unwrap();
    assert_eq!(made.item, Item::Key("person-1".into()));
    assert_eq!(read.sources.spelling(made.head), "person-1");
    assert_eq!(
        one.file_named(&read.sources).as_deref(),
        Some("data/people.yaml: person-1")
    );
    let two = page(&read, "people.md/Person 2");
    assert_eq!(two.date.as_ref().unwrap().value.iso_day(), "2026-10-04");
    assert_eq!(read.drafts.len(), 1);
    let (by, _) = read
        .site
        .pages
        .iter()
        .find(|(_, page)| page.path == "people.md")
        .unwrap();
    let run = read.site.pages.get(by).speaks_for.unwrap();
    let run = read.site.lists.get(run);
    assert_eq!(run.key, ListKey::Declared(by));
    let path = |id: &crate::site::PageId| read.site.pages.get(*id).path.as_str();
    assert_eq!(
        run.members.iter().map(path).collect::<Vec<_>>(),
        ["people.md/Person 2"]
    );
    assert_eq!(
        run.undated.iter().map(path).collect::<Vec<_>>(),
        ["people.md/42", "people.md/person-1"]
    );
}

#[test]
fn an_item_is_held_back_as_a_written_page_is() {
    let files = [
        (
            "statac.yaml",
            "language: en\ntimezone: Europe/London\npages.layout: page\n",
        ),
        ("content/people.md", PEOPLE),
        (
            "data/people.yaml",
            "person-1:\n  draft: true\nperson-2:\n  date: 2026-10-05\nperson-3:\n  title: Person 3\n",
        ),
    ];
    let site = TempSite::site(&files);
    let read = super::super::read(&site.root, &run(false));
    assert_eq!(
        read.problems
            .iter()
            .map(|p| p.headline())
            .collect::<Vec<_>>(),
        [
            "Draft left out: `data/people.yaml: person-1`",
            "Scheduled post: `data/people.yaml: person-2` is dated `2026-10-05`",
        ]
    );
    assert_eq!(made(&read).len(), 1);
    assert_eq!((read.drafts.len(), read.waiting.len()), (1, 1));
    let read = super::super::read(&site.root, &run(true));
    assert_eq!(made(&read).len(), 3);
    assert_eq!((read.included.drafts, read.included.waiting), (1, 1));
}

#[test]
fn an_items_key_or_type_one_slip_from_a_known_word_is_told() {
    let read = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: Europe/London\ntypes.link.source_url: url\n",
        ),
        (
            "content/links/index.md",
            "---\ntitle: Links\neach: data.lists.links\npermalink: /{folder}/{slug}/\n---\n",
        ),
        (
            "data/lists.yaml",
            "links:\n  - title: Link 1\n    sumary: A\n  - title: Link 2\n    type: lnik\n",
        ),
    ]);
    assert_eq!(
        problems(&read),
        [
            (
                "read-as-custom",
                "data/lists.yaml".to_owned(),
                "sumary".to_owned()
            ),
            (
                "read-as-custom",
                "data/lists.yaml".to_owned(),
                "lnik".to_owned()
            ),
        ]
    );
    assert!(
        page(&read, "links/index.md/1")
            .words
            .field("sumary")
            .is_some()
    );
}

#[test]
fn a_page_is_made_for_each_item_of_a_list() {
    let read = site(&[
        (
            "content/links/index.md",
            "---\ntitle: Links\neach: data.lists.links\npermalink: /{folder}/{year}/{slug}/\npages.type: link\n---\n",
        ),
        ("content/links/post-1.md", "---\ntitle: Post 1\n---\n"),
        (
            "data/lists.yaml",
            "links:\n  - title: Link 1\n    date: 2026-10-04\n  - title: Link 2\n    date: 2025-01-02\n    permalink: /link-2.html\n",
        ),
    ]);
    assert_eq!(problems(&read), []);
    assert_eq!(
        made(&read),
        [
            ("links/index.md/1".into(), "/links/2026/link-1/".into()),
            ("links/index.md/2".into(), "/link-2.html".into()),
        ]
    );
    let one = page(&read, "links/index.md/1");
    assert_eq!(one.words.text(PageWord::Type).unwrap().value, "link");
    assert_eq!(
        one.file_named(&read.sources).as_deref(),
        Some("data/lists.yaml: item 1")
    );
    assert_eq!(
        read.sources.spelling(one.made.as_ref().unwrap().head),
        "title: Link 1"
    );
}

#[test]
fn what_stops_a_page_making_pages_is_told_where_it_is() {
    let people = ("data/people.yaml", "person-1:\n  title: Person 1\n");
    type Files<'a> = &'a [(&'a str, &'a str)];
    type Told<'a> = &'a [(&'a str, &'a str, &'a str)];
    let cases: &[(&str, Files, Told)] = &[
        (
            "each: site.pages\npermalink: /{slug}/",
            &[],
            &[("malformed-list", "content/people.md", "site.pages")],
        ),
        (
            "each: people\npermalink: /{key}/",
            &[people],
            &[("wrong-kind-of-value", "content/people.md", "people")],
        ),
        (
            "each: data.peple\npermalink: /{key}/",
            &[people],
            &[("unknown-word", "content/people.md", "data.peple")],
        ),
        (
            "each: data.people",
            &[people],
            &[("missing-field", "content/people.md", "data.people")],
        ),
        (
            "each: data.people\npermalink: /{key/",
            &[people],
            &[("unusable-url", "content/people.md", "{key/")],
        ),
        (
            "each: data.lists\npermalink: /{key}/",
            &[("data/lists/links.yaml", "- a: 1\n")],
            &[("wrong-kind-of-value", "content/people.md", "data.lists")],
        ),
        (
            "each: data.site.title\npermalink: /{key}/",
            &[("data/site.yaml", "title: Site 1\n")],
            &[("wrong-kind-of-value", "data/site.yaml", "Site 1")],
        ),
        (
            "each: data.names\npermalink: /{slug}/",
            &[("data/names.yaml", "- Person 1\n- title: Person 2\n")],
            &[("wrong-kind-of-value", "data/names.yaml", "Person 1")],
        ),
        (
            "each: data.people\npermalink: /{isbn}/",
            &[people],
            &[("unusable-url", "content/people.md", "{isbn}")],
        ),
        (
            "each: data.people\npermalink: /{version}/",
            &[("data/people.yaml", "person-1:\n  version: 3.10\n")],
            &[("needs-quotes", "data/people.yaml", "3.10")],
        ),
        (
            "each: data.people\npermalink: /{key}/",
            &[(
                "data/people.yaml",
                "person-1:\n  id: k3m9x2\nperson-2:\n  id: k3m9x2\n",
            )],
            &[("same-id", "data/people.yaml", "k3m9x2")],
        ),
        (
            "each: data.people\npermalink: /{key}/",
            &[("data/people.yaml", "person-1:\n  each: data.other\n")],
            &[("wrong-place", "data/people.yaml", "data.other")],
        ),
        (
            "each: data.people\npermalink: /{key}/",
            &[("data/people.yaml", "person-1:\n  date: 3/4/2026\n")],
            &[("unclear-date", "data/people.yaml", "3/4/2026")],
        ),
        (
            "each: data.people\npermalink: /{key}/",
            &[("data/people.yaml", "person-1:\n  key: other\n")],
            &[("set-aside", "data/people.yaml", "key")],
        ),
    ];
    for (lines, data, expected) in cases {
        let text = format!("---\ntitle: People\n{lines}\n---\n");
        let mut files = vec![("content/people.md", text.as_str())];
        files.extend_from_slice(data);
        let read = site(&files);
        let expected: Vec<_> = expected
            .iter()
            .map(|(name, file, at)| (*name, (*file).to_owned(), (*at).to_owned()))
            .collect();
        assert_eq!(problems(&read), expected, "{lines}");
    }
}

#[test]
fn an_items_fields_are_checked_as_front_matter_is() {
    let read = site(&[
        (
            "statac.yaml",
            "language: en\nfields.born: date\ntypes.person.role: required text\n",
        ),
        (
            "content/people.md",
            "---\ntitle: People\neach: data.people\npermalink: /{key}/\npages.type: person\n---\n",
        ),
        (
            "data/people.yaml",
            "person-1:\n  born: soon\n  role: Role 1\nperson-2:\n  born: 2026-10-04\n",
        ),
    ]);
    assert_eq!(
        problems(&read),
        [
            (
                "wrong-kind-of-value",
                "data/people.yaml".into(),
                "soon".into()
            ),
            (
                "missing-field",
                "data/people.yaml".into(),
                "person-2".into()
            ),
        ]
    );
}

#[test]
fn a_piece_an_item_lacks_names_the_item() {
    let read = site(&[
        (
            "content/links.md",
            "---\ntitle: Links\neach: data.links\npermalink: /links/{year}/{slug}/\n---\n",
        ),
        (
            "data/links.yaml",
            "- title: Link 1\n  date: 2026-10-04\n- title: Link 2\n- date: 2026-10-04\n",
        ),
    ]);
    let kinds: Vec<Kind> = read.problems.iter().map(|p| p.kind.clone()).collect();
    let lacks = |item: &str, field: &str, piece: &str| Kind::UnusableUrl {
        fault: AddressFault::ItemLacks {
            item: item.into(),
            field: field.into(),
            piece: piece.into(),
        },
    };
    assert_eq!(
        kinds,
        [
            lacks("data/links.yaml: item 2", "date", "{year}"),
            lacks("data/links.yaml: item 3", "title", "{slug}"),
        ]
    );
    assert_eq!(
        problems(&read)
            .into_iter()
            .map(|(_, _, at)| at)
            .collect::<Vec<_>>(),
        ["{year}", "{slug}"]
    );
    assert_eq!(made(&read).len(), 1);
}

#[test]
fn an_index_page_set_aside_publishes_only_the_items() {
    let read = site(&[
        (
            "content/people.md",
            "---\neach: data.people\npermalink: /{key}/\nindex_page: false\n---\n",
        ),
        ("data/people.yaml", "person-1:\n  title: Person 1\n"),
    ]);
    assert_eq!(problems(&read), []);
    assert!(!page(&read, "people.md").lands());
    assert!(page(&read, "people.md/person-1").lands());
    let files: Vec<String> = read.claims().into_iter().map(|(file, _)| file).collect();
    assert_eq!(files, ["person-1/index.html"]);
}

#[test]
fn two_items_at_one_address_are_told_by_name() {
    let site = TempSite::site(&[
        ("content/people.md", PEOPLE),
        (
            "data/people.yaml",
            "person-1:\n  title: Person 1\nPerson-1:\n  title: Person 1\n",
        ),
    ]);
    let (read, ..) = crate::build::read(&site.root, &run(false));
    let kinds: Vec<Kind> = read.problems.iter().map(|p| p.kind.clone()).collect();
    assert_eq!(
        kinds,
        [Kind::SameUrl {
            first: "data/people.yaml: person-1".into(),
            second: "data/people.yaml: Person-1".into(),
            alike: crate::platform::Alike::Exactly,
            address: "/people/person-1/index.html".into(),
        }]
    );
}

/// Each page made for a post: its path, its address, and its post's path.
fn made_for_posts(read: &Read) -> Vec<(String, String, String)> {
    read.site
        .pages
        .iter()
        .filter_map(|(_, page)| match &page.made.as_ref()?.item {
            Item::Post { page: post, .. } => Some((
                page.path.clone(),
                page.address.as_str().to_owned(),
                read.site.pages.get(*post).path.clone(),
            )),
            _ => None,
        })
        .collect()
}

const CARDS: &str = "---\neach: site.posts | tagged(\"Rust\")\npermalink: /cards/{folder}/{year}/{month}/{day}/{slug}/{name}/\nlayout: card\nindex_page: false\n---\n";

const POSTS: &[(&str, &str)] = &[
    ("content/cards.md", CARDS),
    (
        "content/posts/post-1.md",
        "---\ntitle: Post 1\ndate: 2026-01-02\ntags: [Rust]\n---\n",
    ),
    (
        "content/posts/2026-01-03-post-2.md",
        "---\ntitle: Post 2\ntags: [Rust]\n---\n",
    ),
    (
        "content/posts/post-3.md",
        "---\ndate: 2026-01-04\ntags: [Rust]\ndraft: true\n---\n",
    ),
    (
        "content/posts/post-4.md",
        "---\ndate: 2026-10-05\ntags: [Rust]\n---\n",
    ),
    ("content/posts/post-5.md", "---\ndate: 2026-01-05\n---\n"),
    (
        "content/page-1.md",
        "---\ntitle: Page 1\ntags: [Rust]\n---\n",
    ),
    ("statac.yaml", "language: en\ntimezone: Europe/London\n"),
];

#[test]
fn a_page_is_made_for_each_post_the_line_chooses() {
    let read = site(POSTS);
    assert_eq!(
        problems(&read),
        [
            (
                "draft-left-out",
                "content/posts/post-3.md".into(),
                "true".into()
            ),
            (
                "scheduled-post",
                "content/posts/post-4.md".into(),
                "2026-10-05".into()
            ),
        ]
    );
    assert_eq!(
        made_for_posts(&read),
        [
            (
                "cards.md/posts/2026-01-03-post-2.md".into(),
                "/cards/posts/2026/01/03/post-2/2026-01-03-post-2/".into(),
                "posts/2026-01-03-post-2.md".into(),
            ),
            (
                "cards.md/posts/post-1.md".into(),
                "/cards/posts/2026/01/02/post-1/post-1/".into(),
                "posts/post-1.md".into(),
            ),
        ]
    );
    let card = page(&read, "cards.md/posts/post-1.md");
    assert!(card.lands());
    assert!(card.title.is_none() && card.date.is_none());
    assert_eq!(
        card.file_named(&read.sources).as_deref(),
        Some("content/cards.md: content/posts/post-1.md")
    );
    let made = card.made.as_ref().unwrap();
    assert_eq!(
        read.sources.spelling(made.head),
        "site.posts | tagged(\"Rust\")"
    );
    let layout = card.words.get(PageWord::Layout).unwrap();
    assert_eq!(layout.value.as_text(), Some("card"));
    assert_eq!(layout.origin.arrival, Arrival::HandedDown);
    assert!(!page(&read, "cards.md").lands());

    // Listed by the page that made them, in their posts' order, and by no
    // other list.
    let path = |id: &crate::site::PageId| read.site.pages.get(*id).path.as_str();
    let (by, _) = read
        .site
        .pages
        .iter()
        .find(|(_, page)| page.path == "cards.md")
        .unwrap();
    let run_id = read.site.pages.get(by).speaks_for.unwrap();
    let list = read.site.lists.get(run_id);
    assert!(list.members.is_empty());
    assert_eq!(
        list.undated.iter().map(path).collect::<Vec<_>>(),
        [
            "cards.md/posts/2026-01-03-post-2.md",
            "cards.md/posts/post-1.md"
        ]
    );
    for (id, list) in read.site.lists.iter() {
        if id != run_id {
            let mut listed = list.members.iter().chain(&list.undated).map(path);
            assert!(listed.all(|p| !p.starts_with("cards.md/")));
        }
    }

    let site = TempSite::site(POSTS);
    let read = super::super::read(&site.root, &run(true));
    assert_eq!(made_for_posts(&read).len(), 4);
}

#[test]
fn a_post_fills_its_fields_and_is_told_where_it_lacks_one() {
    let read = site(&[
        (
            "content/cards.md",
            "---\neach: site.posts\npermalink: /cards/{isbn}/\n---\n",
        ),
        (
            "content/post-1.md",
            "---\ndate: 2026-01-02\nisbn: Number 1\npermalink: /elsewhere/\n---\n",
        ),
        ("content/post-2.md", "---\ndate: 2026-01-03\n---\n"),
    ]);
    let kinds: Vec<Kind> = read.problems.iter().map(|p| p.kind.clone()).collect();
    assert_eq!(
        kinds,
        [Kind::UnusableUrl {
            fault: AddressFault::PostLacks {
                post: "content/post-2.md".into(),
                field: "isbn".into(),
                piece: "{isbn}".into(),
            },
        }]
    );
    assert_eq!(
        problems(&read),
        [("unusable-url", "content/cards.md".into(), "{isbn}".into())]
    );
    assert_eq!(
        made_for_posts(&read),
        [(
            "cards.md/post-1.md".into(),
            "/cards/number-1/".into(),
            "post-1.md".into()
        )]
    );
}

#[test]
fn a_post_made_from_an_item_fills_the_pattern_as_its_item_does() {
    let read = site(&[
        (
            "content/cards.md",
            "---\neach: site.posts\npermalink: /cards/{slug}/{name}/\n---\n",
        ),
        ("content/people.md", PEOPLE),
        (
            "data/people.yaml",
            "Person 1:\n  title: Title 1\n  date: 2026-01-02\n",
        ),
    ]);
    assert_eq!(problems(&read), []);
    assert_eq!(
        made_for_posts(&read),
        [(
            "cards.md/people.md/Person 1".into(),
            "/cards/person-1/Person 1/".into(),
            "people.md/Person 1".into()
        )]
    );
}

#[test]
fn what_stops_a_page_making_a_page_per_post_is_told_in_its_each_line() {
    let post = (
        "content/post-1.md",
        "---\ndate: 2026-01-02\ntags: [Rust]\n---\n",
    );
    let cases: &[(&str, (&str, &str))] = &[
        ("each: site.posts", ("missing-field", "site.posts")),
        (
            "each: site.posts | taged(\"Rust\")\npermalink: /{slug}/",
            ("unknown-word", "taged"),
        ),
        (
            "each: site.posts | tagged(\"Go\")\npermalink: /{slug}/",
            ("unknown-word", "Go"),
        ),
        (
            "each: site.posts |\npermalink: /{slug}/",
            ("malformed-list", "|"),
        ),
        (
            "each: site.posts | where(\"lang\")\npermalink: /{slug}/",
            ("malformed-list", "where(\"lang\")"),
        ),
    ];
    for (lines, (name, at)) in cases {
        let text = format!("---\n{lines}\n---\n");
        let read = site(&[("content/cards.md", text.as_str()), post]);
        assert_eq!(
            problems(&read),
            [(*name, "content/cards.md".to_owned(), (*at).to_owned())],
            "{lines}"
        );
        assert!(made_for_posts(&read).is_empty(), "{lines}");
    }
}

#[test]
fn a_page_with_both_each_and_posts_stops_at_the_second() {
    let post = (
        "content/post-1.md",
        "---\ndate: 2026-01-02\ntags: [Rust]\n---\n",
    );
    let people = ("data/people.yaml", "person-1:\n  title: Person 1\n");
    let cases = [
        (
            "posts: site.posts\neach: site.posts | tagged(\"Rust\")",
            ("posts", "site.posts"),
            ("each", "site.posts | tagged(\"Rust\")"),
        ),
        (
            "each: data.people\nposts: site.posts | tagged(\"Rust\")",
            ("each", "data.people"),
            ("posts", "site.posts | tagged(\"Rust\")"),
        ),
    ];
    for (lines, (first, first_at), (second, at)) in cases {
        let text = format!("---\n{lines}\npermalink: /cards/{{name}}/\n---\n");
        let read = site(&[("content/cards.md", text.as_str()), post, people]);
        assert_eq!(read.problems.len(), 1, "{lines}: {:?}", read.problems);
        let problem = &read.problems[0];
        assert!(matches!(
            &problem.kind,
            Kind::TwoLists { first: f, second: s, .. } if f == first && s == second
        ));
        assert_eq!(
            problems(&read),
            [("two-lists", "content/cards.md".to_owned(), at.to_owned())]
        );
        let (_, related) = problem.kind.related()[0].clone();
        assert_eq!(read.sources.spelling(related.unwrap()), first_at);
        assert!(made(&read).is_empty(), "{lines}");
    }
}
