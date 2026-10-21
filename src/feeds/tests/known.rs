//! Sites built, changed and built again: what is kept of the posts their
//! feeds carry, and what is said of a post that has moved.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::build::{self, Mode, Options, Outcome};
use crate::problem::{Kind, Reason};
use crate::serve::session::Session;

/// A site in a folder of its own, removed when dropped.
struct Kept {
    root: PathBuf,
}

const SETTINGS: &str =
    "language: en\ntitle: Site\nurl: https://example.org\nfeed_key: key\ntimezone: UTC\n";

/// The id of a post at `/posts/post-1/` on a site with `feed_key: key`.
const ID: &str = "fc370633-88f0-8092-9f96-7749dbfff0eb";

const POST: &str = "---\ntitle: Post 1\ndate: 2026-01-01\n---\nText.\n";

impl Kept {
    /// A site with `settings`, a folder `posts` and a folder `notes`, each
    /// feed printing its posts' ids.
    fn new(settings: &str) -> Kept {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "statac-feeds-known-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&root);
        let kept = Kept { root };
        for (path, text) in [
            ("statac.yaml", settings),
            ("content/index.md", "---\ntitle: Home\n---\n"),
            ("content/posts/post-1.md", POST),
            (
                "content/notes/post-2.md",
                "---\ntitle: Post 2\ndate: 2026-01-02\n---\n",
            ),
            ("templates/page.html", "{{ page.title }}{{ page.content }}"),
            ("templates/home.html", "{{ page.title }}"),
            ("templates/folder.html", "{{ headline(page) }}"),
            ("templates/redirect.html", "{{ page.url }}"),
            (
                "templates/feed.xml",
                "{% for post in posts %}{{ post.title }}={{ feed_id(post) }};{% endfor %}",
            ),
        ] {
            kept.write(path, text);
        }
        kept
    }

    fn site(&self) -> PathBuf {
        self.root.join("site")
    }

    fn write(&self, path: &str, text: &str) {
        let path = self.site().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn rename(&self, from: &str, to: &str) {
        let to = self.site().join(to);
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        std::fs::rename(self.site().join(from), to).unwrap();
    }

    fn options(&self, mode: Mode, drafts: bool) -> Options {
        Options {
            mode,
            site: self.site(),
            out: Some(self.root.join("out")),
            today: Some(crate::date::Date::day_only(2026, 10, 4)),
            drafts,
            url: None,
        }
    }

    fn run(&self, mode: Mode) -> Outcome {
        build::run(&self.options(mode, false))
    }

    fn build(&self) -> Outcome {
        let outcome = self.run(Mode::Build);
        assert!(outcome.published.is_some(), "{:?}", outcome.problems);
        outcome
    }

    /// What is kept of the feeds' posts, where anything is.
    fn kept(&self) -> Option<String> {
        std::fs::read_to_string(self.site().join(".statac/feeds")).ok()
    }

    /// What the feed of `folder` says of Post 1.
    fn entry(&self, folder: &str) -> Option<String> {
        let feed = std::fs::read_to_string(self.root.join("out").join(folder).join("feed.xml"));
        let entry = feed
            .ok()?
            .split(';')
            .find(|e| e.starts_with("Post 1="))?
            .to_owned();
        Some(entry)
    }
}

impl Drop for Kept {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Each problem's kind and the text it is placed at.
fn told(outcome: &Outcome) -> Vec<(Kind, Option<String>)> {
    let sources = outcome.sources();
    (outcome.problems.iter())
        .map(|p| {
            let at = p.place.map(|place| sources.spelling(place).to_owned());
            (p.kind.clone(), at)
        })
        .collect()
}

fn changed(was: &str) -> Vec<(Kind, Option<String>)> {
    let kind = Kind::FeedIdChanged {
        was: was.into(),
        id: ID.into(),
        item: false,
    };
    vec![(kind, Some("---".into()))]
}

fn with_line(text: &str, line: &str) -> String {
    text.replacen("---\n", &format!("---\n{line}\n"), 1)
}

/// How Post 1 moves, the file it is in afterwards, and the folder whose
/// feed then carries it.
type Move = (&'static str, fn(&Kept), &'static str, &'static str);

const MOVES: [Move; 4] = [
    (
        "its permalink changed",
        |kept| {
            let text = with_line(POST, "permalink: /writing/post-1/");
            kept.write("content/posts/post-1.md", &text);
        },
        "content/posts/post-1.md",
        "posts",
    ),
    (
        "its file moved to another folder",
        |kept| kept.rename("content/posts/post-1.md", "content/notes/post-1.md"),
        "content/notes/post-1.md",
        "notes",
    ),
    (
        "its file given another name",
        |kept| kept.rename("content/posts/post-1.md", "content/posts/post-one.md"),
        "content/posts/post-one.md",
        "posts",
    ),
    (
        "its file moved and changed, sending readers on from where it was",
        |kept| {
            std::fs::remove_file(kept.site().join("content/posts/post-1.md")).unwrap();
            let text = with_line(POST, "redirect_from: [/posts/post-1/]");
            kept.write(
                "content/notes/post-1.md",
                &text.replace("Text.", "Other text."),
            );
        },
        "content/notes/post-1.md",
        "notes",
    ),
];

#[test]
fn a_post_that_moved_is_offered_the_id_it_had_until_it_writes_it() {
    for (case, moves, file, folder) in MOVES {
        let kept = Kept::new(SETTINGS);
        assert_eq!(told(&kept.build()), [], "{case}");
        let first = kept.entry("posts").unwrap();
        assert_eq!(first, format!("Post 1=urn:uuid:{ID}"));

        moves(&kept);
        assert_eq!(
            told(&kept.run(Mode::Check)),
            changed("/posts/post-1/"),
            "{case}"
        );
        // Said by every build until the line is written, not by the first
        // alone.
        for _ in 0..2 {
            assert_eq!(told(&kept.build()), changed("/posts/post-1/"), "{case}");
            assert_ne!(kept.entry(folder).unwrap(), first, "{case}");
        }

        let text = std::fs::read_to_string(kept.site().join(file)).unwrap();
        kept.write(file, &with_line(&text, &format!("id: {ID}")));
        for _ in 0..2 {
            assert_eq!(told(&kept.build()), [], "{case}");
            assert_eq!(kept.entry(folder).unwrap(), first, "{case}");
        }
    }
}

#[test]
fn a_site_under_a_path_is_shown_the_whole_address_the_post_had() {
    let kept = Kept::new(&SETTINGS.replace("example.org", "example.org/sub/"));
    kept.build();
    MOVES[0].1(&kept);
    assert_eq!(told(&kept.build()), changed("/sub/posts/post-1/"));
}

#[test]
fn a_post_moved_and_changed_at_once_is_a_new_post() {
    let kept = Kept::new(SETTINGS);
    kept.build();
    std::fs::remove_file(kept.site().join("content/posts/post-1.md")).unwrap();
    kept.write("content/notes/post-1.md", &POST.replace("Text.", "Other."));
    assert_eq!(told(&kept.build()), []);
}

#[test]
fn a_copy_of_a_post_still_standing_is_a_new_post() {
    let kept = Kept::new(SETTINGS);
    kept.build();
    kept.write("content/notes/post-1.md", POST);
    assert_eq!(told(&kept.build()), []);
}

/// Each a site built, then changed so that Post 1 moves, of which nothing
/// is said.
#[test]
fn nothing_is_said_of_a_post_no_feed_reader_would_see_anew() {
    let moved = with_line(POST, "permalink: /writing/post-1/");
    let post = "content/posts/post-1.md";
    type Case<'a> = (&'a str, &'a str, Vec<(&'a str, String)>, bool);
    let cases: [Case; 7] = [
        (
            "a draft",
            SETTINGS,
            vec![(post, with_line(&moved, "draft: true"))],
            true,
        ),
        (
            "a post waiting for its day",
            SETTINGS,
            vec![(post, moved.replace("2026-01-01", "2026-12-01"))],
            true,
        ),
        (
            "a post with an id of its own",
            SETTINGS,
            vec![(post, with_line(&moved, "id: abc123"))],
            false,
        ),
        (
            "a post in no feed",
            SETTINGS,
            vec![
                (post, moved.clone()),
                (
                    "content/index.md",
                    "---\ntitle: Home\nfeed: false\n---\n".into(),
                ),
                ("content/posts/index.md", "---\nfeed: false\n---\n".into()),
            ],
            false,
        ),
        (
            "a page with no date",
            SETTINGS,
            vec![(post, moved.replace("date: 2026-01-01\n", ""))],
            false,
        ),
        (
            "a site with no feed",
            SETTINGS,
            vec![
                (post, moved.clone()),
                ("statac.yaml", format!("{SETTINGS}pages.feed: false\n")),
            ],
            false,
        ),
        (
            "a post keeping another tool's id",
            "language: en\ntitle: Site\nurl: https://example.org\nfeed_key: key\ntimezone: UTC\n\
             feed_ids_before: 2026-06-01\n",
            vec![(post, moved.clone())],
            false,
        ),
    ];
    for (case, settings, files, with_drafts) in cases {
        let kept = Kept::new(settings);
        assert_eq!(told(&kept.build()), [], "{case}");
        for (path, text) in &files {
            kept.write(path, text);
        }
        let changed: Vec<Kind> = (told(&kept.build()).into_iter())
            .map(|(kind, _)| kind)
            .filter(|kind| matches!(kind, Kind::FeedIdChanged { .. }))
            .collect();
        assert_eq!(changed, [], "{case}");
        if with_drafts {
            let outcome = build::run(&kept.options(Mode::Build, true));
            assert_eq!(told(&outcome), [], "{case}, built with drafts");
        }
    }
}

#[test]
fn a_build_at_another_address_keeps_nothing_and_knows_each_post_as_it_was_kept() {
    let another_tools = format!("{SETTINGS}feed_ids_before: 2026-06-01\n");
    for settings in [SETTINGS, another_tools.as_str()] {
        for address in [
            "https://preview.example.org/",
            "https://example.org/preview/42/",
        ] {
            let case = format!("{address}, {settings:?}");
            let kept = Kept::new(settings);
            let elsewhere = Options {
                url: Some(address.to_owned()),
                ..kept.options(Mode::Build, false)
            };
            let build_elsewhere = || {
                let outcome = build::run(&elsewhere);
                assert!(
                    outcome.published.is_some(),
                    "{case}: {:?}",
                    outcome.problems
                );
                told(&outcome)
            };
            assert_eq!(build_elsewhere(), [], "{case}");
            assert_eq!(kept.kept(), None, "{case}");

            kept.build();
            let lines = kept.kept().unwrap();
            let entry = kept.entry("posts").unwrap();
            assert_eq!(build_elsewhere(), [], "{case}");
            assert_eq!(kept.kept().unwrap(), lines, "{case}");
            assert_eq!(kept.entry("posts").unwrap(), entry, "{case}");

            // A post that has moved is offered its id as any build offers
            // it, by where it was at the site's own address, and stays
            // known as it was.
            kept.rename("content/posts/post-1.md", "content/posts/post-3.md");
            let offered = match settings == SETTINGS {
                true => changed("/posts/post-1/"),
                false => Vec::new(),
            };
            assert_eq!(build_elsewhere(), offered, "{case}");
            assert_eq!(kept.kept().unwrap(), lines, "{case}");
            assert_eq!(told(&kept.build()), offered, "{case}");
        }
    }
}

#[test]
fn what_is_kept_is_a_post_a_line_in_the_order_of_their_files() {
    let kept = Kept::new(SETTINGS);
    kept.build();
    let hash = |text: &str| blake3::hash(text.as_bytes()).to_hex()[..32].to_owned();
    let lines = format!(
        "statac feeds 1\n\
         notes/post-2.md\t/notes/post-2/\turn:uuid:549b3240-8f9d-8c2b-a68e-c5d8930676b5\t{}\n\
         posts/post-1.md\t/posts/post-1/\turn:uuid:{ID}\t{}\n",
        hash("---\ntitle: Post 2\ndate: 2026-01-02\n---\n"),
        hash(POST),
    );
    assert_eq!(kept.kept().unwrap(), lines);
}

#[test]
fn only_a_build_that_publishes_keeps_anything() {
    let kept = Kept::new(SETTINGS);
    kept.run(Mode::Check);
    assert_eq!(kept.kept(), None);
    let (mut session, round) = Session::start(&kept.options(Mode::Build, true));
    assert!(round.outcome.published.is_some());
    assert_eq!(kept.kept(), None);

    kept.build();
    let before = kept.kept().unwrap();
    MOVES[0].1(&kept);
    // A round says what a build would, and leaves what is kept alone.
    assert_eq!(told(&session.rebuild().outcome), changed("/posts/post-1/"));
    assert_eq!(kept.kept().unwrap(), before);
    // A build that stops publishes nothing, so keeps nothing new.
    kept.write("content/notes/post-2.md", "---\ndate: 02/01/2026\n---\n");
    assert!(kept.run(Mode::Build).published.is_none());
    assert_eq!(kept.kept().unwrap(), before);
}

#[test]
fn a_site_with_no_feed_keeps_nothing_and_leaves_what_was_kept() {
    let kept = Kept::new(&format!("{SETTINGS}pages.feed: false\n"));
    kept.build();
    assert_eq!(kept.kept(), None);
    kept.write("statac.yaml", SETTINGS);
    kept.build();
    let before = kept.kept().unwrap();
    kept.write("statac.yaml", &format!("{SETTINGS}pages.feed: false\n"));
    kept.build();
    assert_eq!(kept.kept().unwrap(), before);
}

#[test]
fn losing_what_was_kept_loses_only_the_offer() {
    for lost in ["gone", "not ours", "half a line"] {
        let kept = Kept::new(SETTINGS);
        kept.build();
        let file = kept.site().join(".statac/feeds");
        match lost {
            "gone" => std::fs::remove_file(&file).unwrap(),
            "not ours" => std::fs::write(&file, "something else\n").unwrap(),
            _ => std::fs::write(&file, "statac feeds 1\nposts/post-1.md\t/posts/\n").unwrap(),
        }
        MOVES[0].1(&kept);
        assert_eq!(told(&kept.build()), [], "{lost}");
        assert!(
            kept.kept().unwrap().contains("\t/writing/post-1/\t"),
            "{lost}"
        );
    }
}

#[test]
fn what_cannot_be_kept_is_said_once_and_the_site_is_published() {
    let kept = Kept::new(SETTINGS);
    std::fs::create_dir_all(kept.site().join(".statac/feeds")).unwrap();
    let outcome = kept.build();
    assert_eq!(
        told(&outcome),
        [(
            Kind::CacheNotWritten {
                path: PathBuf::from(".statac").join("feeds"),
                reason: Reason::Taken,
            },
            None
        )]
    );
    assert!(kept.entry("posts").is_some());
}

/// The lines `write_feed_ids` would write, each with its post's file.
fn to_write(kept: &Kept) -> Vec<(String, String)> {
    let run = kept.options(Mode::Check, false).run();
    let prepared = build::prepared(&kept.site(), &run);
    let renderer = &prepared.renderer;
    let lines = crate::feeds::ids_to_write(
        &prepared.known,
        renderer.site(),
        renderer.settings(),
        renderer.dates(),
    );
    let crate::feeds::IdsToWrite::Lines(lines) = lines else {
        return vec![("no record".to_owned(), String::new())];
    };
    (lines.into_iter())
        .map(|(post, line)| (renderer.site().pages.get(post).path.clone(), line))
        .collect()
}

#[test]
fn the_ids_to_write_are_those_of_new_posts_and_of_posts_that_moved() {
    let on = format!("{SETTINGS}write_feed_ids: true\n");
    let kept = Kept::new(SETTINGS);
    assert_eq!(to_write(&kept), [], "the setting left off");

    // Nothing kept: which posts are new can't be told, and none is
    // given an id.
    kept.write("statac.yaml", &on);
    let new = |path: &str, id: &str| (path.to_owned(), format!("id: {id}"));
    assert_eq!(to_write(&kept), [("no record".to_owned(), String::new())]);

    // Published, a post keeps its id for as long as it stays.
    kept.build();
    assert_eq!(to_write(&kept), []);

    kept.write(
        "content/posts/post-3.md",
        "---\ntitle: Post 3\ndate: 2026-01-03\nid: abc123\n---\n",
    );
    kept.write(
        "content/posts/post-4.md",
        "---\ntitle: Post 4\ndate: 2026-01-04\ndraft: true\n---\n",
    );
    kept.write(
        "content/posts/post-5.md",
        "---\ntitle: Post 5\ndate: 2026-01-05\n---\n",
    );
    MOVES[0].1(&kept);
    let fifth = "aa1fddab-af8c-8e19-9a85-d9089fe858aa";
    assert_eq!(
        to_write(&kept),
        [new("posts/post-1.md", ID), new("posts/post-5.md", fifth),]
    );
    // The line a new post is given leaves it the id it is published with.
    kept.build();
    let feed = std::fs::read_to_string(kept.root.join("out/posts/feed.xml")).unwrap();
    assert!(
        feed.contains(&format!("Post 5=urn:uuid:{fifth};")),
        "{feed}"
    );
}

/// A page making a post of each item of `data/items.yaml`.
const ITEMS: &str = "---\ntitle: Items\neach: data.items\npermalink: /items/{slug}/\n---\n";

const ITEM: &str = "  title: Item 1\n  date: 2026-01-03\n";

/// The id the site's own feed gives the post `title`.
fn id_of(kept: &Kept, title: &str) -> String {
    let feed = std::fs::read_to_string(kept.root.join("out/feed.xml")).unwrap();
    let entry = feed
        .split(';')
        .find_map(|e| e.strip_prefix(&format!("{title}=")));
    let entry = entry.unwrap_or_else(|| panic!("{feed}"));
    entry.strip_prefix("urn:uuid:").unwrap().to_owned()
}

fn item_changed(was: &str, id: &str, at: &str) -> (Kind, Option<String>) {
    let kind = Kind::FeedIdChanged {
        was: was.into(),
        id: id.into(),
        item: true,
    };
    (kind, Some(at.into()))
}

#[test]
fn a_post_made_from_a_named_item_that_moved_is_offered_the_id_it_had() {
    let items = format!("one:\n{ITEM}");
    let cases = [
        (
            "its pattern changed",
            ITEMS.replace("/items/", "/elsewhere/"),
            items.clone(),
            "one",
        ),
        (
            "its name changed",
            ITEMS.replace("{slug}", "{key}"),
            format!("two:\n{ITEM}"),
            "two",
        ),
    ];
    for (case, page, moved, at) in cases {
        let kept = Kept::new(SETTINGS);
        kept.write("content/items.md", &ITEMS.replace("{slug}", "{key}"));
        kept.write("data/items.yaml", &items);
        assert_eq!(told(&kept.build()), [], "{case}");
        let id = id_of(&kept, "Item 1");

        kept.write("content/items.md", &page);
        kept.write("data/items.yaml", &moved);
        let offered = [item_changed("/items/one/", &id, at)];
        for _ in 0..2 {
            assert_eq!(told(&kept.build()), offered, "{case}");
            assert_ne!(id_of(&kept, "Item 1"), id, "{case}");
        }

        kept.write("data/items.yaml", &format!("{moved}  id: {id}\n"));
        assert_eq!(told(&kept.build()), [], "{case}");
        assert_eq!(id_of(&kept, "Item 1"), id, "{case}");
    }
}

/// Each a site built with one item in a list, then the list and the
/// pattern as given, and what is said.
#[test]
fn a_post_made_from_an_item_in_a_list_is_known_by_its_fields_alone() {
    let moved = ITEMS.replace("/items/", "/elsewhere/");
    let item = format!("-{}", &ITEM[1..]);
    let added = format!("- title: Item 2\n  date: 2026-01-04\n{item}");
    let edited = format!("{item}  summary: Text.\n");
    let sent_on = format!("{edited}  redirect_from: [/items/item-1/]\n");
    let cases = [
        ("its pattern changed", &item, &moved, true),
        ("an item added above it", &added, &ITEMS.to_owned(), false),
        (
            "an item added above it, its pattern changed",
            &added,
            &moved,
            true,
        ),
        (
            "its fields changed, its pattern too",
            &edited,
            &moved,
            false,
        ),
        (
            "its fields changed, sending readers on",
            &sent_on,
            &moved,
            true,
        ),
    ];
    for (case, list, page, offered) in cases {
        let kept = Kept::new(SETTINGS);
        kept.write("content/items.md", ITEMS);
        kept.write("data/items.yaml", &item);
        assert_eq!(told(&kept.build()), [], "{case}");
        let id = id_of(&kept, "Item 1");

        kept.write("content/items.md", page);
        kept.write("data/items.yaml", list);
        let expected = match offered {
            true => vec![item_changed("/items/item-1/", &id, "title: Item 1")],
            false => Vec::new(),
        };
        assert_eq!(told(&kept.build()), expected, "{case}");
    }
}

#[test]
fn two_items_written_alike_are_neither_offered_an_id() {
    let kept = Kept::new(SETTINGS);
    let item = format!("-{}", &ITEM[1..]);
    kept.write("data/items.yaml", &item);
    kept.write("content/a.md", &ITEMS.replace("/items/", "/a/"));
    kept.write("content/b.md", &ITEMS.replace("/items/", "/b/"));
    assert_eq!(told(&kept.build()), []);
    // Known by no path, and by fields two posts share.
    let lines = kept.kept().unwrap();
    for address in ["/a/item-1/", "/b/item-1/"] {
        assert!(lines.contains(&format!("\n\t{address}\t")), "{lines}");
    }
    kept.write("content/a.md", &ITEMS.replace("/items/", "/c/"));
    kept.write("content/b.md", &ITEMS.replace("/items/", "/d/"));
    assert_eq!(told(&kept.build()), []);
}

#[test]
fn a_post_made_from_an_item_is_given_no_id_to_write() {
    let kept = Kept::new(&format!("{SETTINGS}write_feed_ids: true\n"));
    kept.write("content/items.md", &ITEMS.replace("{slug}", "{key}"));
    kept.write("data/items.yaml", &format!("one:\n{ITEM}"));
    kept.build();
    assert_eq!(to_write(&kept), []);
    kept.write("content/items.md", &ITEMS.replace("{slug}", "{key}/x"));
    kept.write(
        "data/items.yaml",
        &format!("one:\n{ITEM}two:\n  title: Item 2\n  date: 2026-01-04\n"),
    );
    assert_eq!(to_write(&kept), []);
}
