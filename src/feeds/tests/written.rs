//! Sites built end to end: the feeds they write, what is said of those
//! they don't, the ids entries carry, and the other files that aren't
//! HTML.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::build::{self, Mode, Options, Outcome};
use crate::problem::{AddressFault, Kind, Named, Unwritten, Waiting, Word};

/// A site written to a fresh folder and built into another beside it,
/// both removed when dropped.
struct Built {
    root: PathBuf,
    outcome: Outcome,
}

impl Built {
    fn new(files: &[(&str, &str)]) -> Built {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "statac-feeds-written-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("site/content")).unwrap();
        for (path, text) in files {
            let path = root.join("site").join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        let outcome = build::run(&Options {
            mode: Mode::Build,
            site: root.join("site"),
            out: Some(root.join("out")),
            today: Some(crate::date::Date::day_only(2026, 10, 4)),
            drafts: false,
            url: None,
        });
        Built { root, outcome }
    }

    /// The built file at `path`, where one was written.
    fn file(&self, path: &str) -> Option<String> {
        std::fs::read_to_string(self.root.join("out").join(path)).ok()
    }

    fn out(&self) -> &Path {
        &self.root
    }

    /// Each problem's kind.
    fn kinds(&self) -> Vec<Kind> {
        self.outcome
            .problems
            .iter()
            .map(|p| p.kind.clone())
            .collect()
    }

    /// Each problem's name and the text it is placed at, where it is placed.
    fn told(&self) -> Vec<(&'static str, Option<String>)> {
        let sources = self.outcome.sources();
        self.outcome
            .problems
            .iter()
            .map(|p| {
                let at = p.place.map(|place| sources.spelling(place).to_owned());
                (p.kind.name(), at)
            })
            .collect()
    }
}

impl Drop for Built {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(self.out());
    }
}

/// Files of a site, each by its path in the site and its text.
type Files<'a> = Vec<(&'a str, &'a str)>;

const SETTINGS: (&str, &str) = (
    "statac.yaml",
    "language: en\ntitle: Site\nurl: https://example.org\nfeed_key: key\ntimezone: UTC\n",
);
const HOME: (&str, &str) = ("content/index.md", "---\ntitle: Home\n---\n");
const PAGE: (&str, &str) = (
    "templates/page.html",
    "{{ headline(page) }}{{ page.content }}",
);
const HOME_TEMPLATE: (&str, &str) = ("templates/home.html", "{{ page.title }}");
const FOLDER: (&str, &str) = ("templates/folder.html", "{{ headline(page) }}");
/// A feed template printing what it is handed.
const FEED: (&str, &str) = (
    "templates/feed.xml",
    "{{ list.word }}|{% for post in posts %}{{ post.title }};{% endfor %}|{{ page.url }}|\
     {{ list.feed }}",
);

fn post(n: u32, extra: &str) -> (String, String) {
    (
        format!("content/posts/post-{n}.md"),
        format!("---\ntitle: Post {n} & co\ndate: 2026-01-0{n}\n{extra}---\nText.\n"),
    )
}

/// A site of the given files, with posts 1 to 3 in `content/posts/` in a
/// series, besides.
fn with_posts(files: &[(&str, &str)]) -> Built {
    let posts: Vec<(String, String)> = (1..=3).map(|n| post(n, "series: Series 1\n")).collect();
    let mut all: Vec<(&str, &str)> = posts
        .iter()
        .map(|(p, t)| (p.as_str(), t.as_str()))
        .collect();
    all.extend_from_slice(files);
    Built::new(&all)
}

#[test]
fn a_feed_is_built_through_the_sites_template_handed_its_list_and_posts_newest_first() {
    let series = (
        "content/series/series-1.md",
        "---\ntitle: Series 1\nfeed: true\n---\n",
    );
    let built = with_posts(&[SETTINGS, HOME, PAGE, HOME_TEMPLATE, FOLDER, FEED, series]);
    assert!(built.outcome.problems.is_empty(), "{:?}", built.kinds());
    assert_eq!(
        built.file("feed.xml").as_deref(),
        Some("site|Post 3 &amp; co;Post 2 &amp; co;Post 1 &amp; co;|/|/feed.xml")
    );
    assert_eq!(
        built.file("posts/feed.xml").as_deref(),
        Some("folder|Post 3 &amp; co;Post 2 &amp; co;Post 1 &amp; co;|/posts/|/posts/feed.xml")
    );
    // A series runs oldest first; its feed, newest first all the same.
    assert_eq!(
        built.file("series/series-1/feed.xml").as_deref(),
        Some(
            "series|Post 3 &amp; co;Post 2 &amp; co;Post 1 &amp; co;|/series/series-1/|\
             /series/series-1/feed.xml"
        )
    );
    let published = built.outcome.published.expect("published");
    assert_eq!(published.feeds, 3);
}

#[test]
fn a_site_with_no_index_has_its_feed_handed_the_page_statac_makes() {
    let feed = (
        "templates/feed.xml",
        "{{ page.url }}|{{ page.list.word }}|{{ page.list.posts | length }}|{{ list.feed }}",
    );
    let home = (
        "templates/home.html",
        "{{ page.list.word }}:{{ page.content }}",
    );
    let built = with_posts(&[SETTINGS, PAGE, home, FOLDER, feed]);
    assert!(built.outcome.problems.is_empty(), "{:?}", built.kinds());
    assert_eq!(
        built.file("feed.xml").as_deref(),
        Some("/|site|3|/feed.xml")
    );
    assert_eq!(built.file("index.html").as_deref(), Some("site:"));
}

#[test]
fn a_feed_is_built_with_the_template_its_extension_names_or_else_feed_xml() {
    let rss = (
        "content/index.md",
        "---\ntitle: Home\nfeed: /index.rss\n---\n",
    );
    let json = (
        "content/index.md",
        "---\ntitle: Home\nfeed: /feed.json\n---\n",
    );
    let own_rss = ("templates/feed.rss", "rss:{{ list.word }}");
    let own_json = (
        "templates/feed.json",
        "{\"title\": {{ list.title }}, \"items\": [{% for post in posts %}{{ post.title }}{% if not loop.last %}, {% endif %}{% endfor %}]}",
    );
    let quiet = (
        "statac.yaml",
        "language: en\ntitle: Site\nurl: https://example.org\nfeed_key: key\ntimezone: UTC\npages.feed: false\n",
    );
    let cases: Vec<(&str, Files, &str, &str)> = vec![
        ("only feed.xml", vec![rss], "index.rss", "site|"),
        ("its own", vec![rss, own_rss], "index.rss", "rss:site"),
        (
            "written as JSON",
            vec![json, own_json],
            "feed.json",
            "{\"title\": \"Home\", \"items\": [\"Post 3 \\u0026 co\", \"Post 2 \\u0026 co\", \"Post 1 \\u0026 co\"]}",
        ),
    ];
    for (case, files, at, starts) in cases {
        let mut all = vec![quiet, PAGE, HOME_TEMPLATE, FOLDER, FEED];
        all.extend(files);
        let built = with_posts(&all);
        assert!(
            built.outcome.problems.is_empty(),
            "{case}: {:?}",
            built.kinds()
        );
        let written = built.file(at).unwrap_or_else(|| panic!("{case}: no {at}"));
        assert!(written.starts_with(starts), "{case}: {written}");
    }
}

#[test]
fn what_a_feed_lacks_is_said_once_for_the_site() {
    let site = (Word::Folder, String::new());
    let posts = (Word::Folder, "posts".into());
    let cases: Vec<(&str, &str, Files, Vec<Kind>)> = vec![
        (
            "no address",
            "language: en\ntitle: Site\nfeed_key: key\ntimezone: UTC\n",
            vec![FEED],
            vec![Kind::NoSiteAddress {
                waiting: Waiting::Feeds {
                    list: site.clone(),
                    more: 1,
                },
            }],
        ),
        (
            "no title",
            "language: en\nurl: https://example.org\nfeed_key: key\ntimezone: UTC\n",
            vec![FEED],
            vec![Kind::FeedNotWritten {
                list: site.clone(),
                more: 1,
                why: Unwritten::NoTitle,
            }],
        ),
        (
            "no template",
            SETTINGS.1,
            vec![],
            vec![Kind::FeedNotWritten {
                list: site.clone(),
                more: 1,
                why: Unwritten::NoTemplate("templates/feed.xml".into()),
            }],
        ),
        (
            "no template for a folder's alone",
            "language: en\ntitle: Site\nurl: https://example.org\nfeed_key: key\ntimezone: UTC\n",
            vec![("content/index.md", "---\ntitle: Home\nfeed: false\n---\n")],
            vec![Kind::FeedNotWritten {
                list: posts,
                more: 0,
                why: Unwritten::NoTemplate("templates/feed.xml".into()),
            }],
        ),
    ];
    for (case, settings, files, wanted) in cases {
        let mut all = vec![("statac.yaml", settings), HOME, PAGE, HOME_TEMPLATE, FOLDER];
        all.extend(files);
        let built = with_posts(&all);
        assert_eq!(built.kinds(), wanted, "{case}");
        assert!(built.file("feed.xml").is_none(), "{case}");
        assert!(
            built.file("index.html").is_some(),
            "{case}: the rest is published"
        );
    }
}

#[test]
fn a_site_wanting_a_feed_with_no_key_stops_with_a_fresh_one_to_paste() {
    let settings = (
        "statac.yaml",
        "language: en\ntitle: Site\nurl: https://example.org\ntimezone: UTC\n",
    );
    let built = with_posts(&[settings, HOME, PAGE, HOME_TEMPLATE, FOLDER, FEED]);
    let [problem] = built.outcome.problems.as_slice() else {
        panic!("{:?}", built.kinds());
    };
    let Kind::NoFeedKey { key } = &problem.kind else {
        panic!("{problem:?}");
    };
    assert_eq!(key.len(), 36, "{key}");
    let suggestion = problem.suggestion().unwrap_or_default();
    assert!(
        suggestion.starts_with(&format!("Add `feed_key: {key}` to `statac.yaml`")),
        "{suggestion}"
    );
    assert!(built.outcome.published.is_none());
    // No list with dated posts wants a feed, so nothing asks for a key.
    let undated = Built::new(&[settings, HOME, PAGE, HOME_TEMPLATE, FEED]);
    assert!(undated.outcome.problems.is_empty(), "{:?}", undated.kinds());
}

/// Where a warning is placed, the file it is in, and what it says.
type Warned<'a> = Option<(&'a str, &'a str, &'a str)>;

/// A `feed:` line asking for the feed of a list with nothing dated, times
/// what is said: a warning where the line is written, once for the
/// first such list it reaches; nothing where the list has a dated page,
/// or has a feed only by default.
#[test]
fn a_feed_asked_of_a_list_with_nothing_dated_is_warned_of_at_the_line() {
    const NOTES: &str = "Feed not written: the folder `content/notes/` has nothing dated";
    let undated = |n: u32| {
        (
            format!("content/notes/note-{n}.md"),
            format!("---\ntitle: Note {n}\n---\n"),
        )
    };
    let (note_1, note_2) = (undated(1), undated(2));
    let notes = |front: &str| format!("---\ntitle: Notes\n{front}\n---\n");
    let (true_line, address, empty) = (notes("feed: true"), notes("feed: atom.xml"), notes(""));
    let in_folder = "---\ntitle: Folder\npages.feed: true\n---\n";
    let dated = "---\ntitle: Note 3\ndate: 2026-01-01\n---\n";
    let handed = "language: en\ntitle: Site\nurl: https://example.org\nfeed_key: key\ntimezone: UTC\n\
                  pages.feed: true\n";
    // (the files besides the notes, where the warning is placed and the
    // file it is in, and what it says)
    let cases: Vec<(Files, Warned)> = vec![
        (
            vec![("content/notes/index.md", &true_line)],
            Some(("true", "content/notes/index.md", NOTES)),
        ),
        (
            vec![("content/notes/index.md", &address)],
            Some(("atom.xml", "content/notes/index.md", NOTES)),
        ),
        (
            vec![("statac.yaml", handed), ("content/notes/index.md", &empty)],
            Some(("true", "statac.yaml", NOTES)),
        ),
        (
            vec![
                ("content/folder-1/index.md", in_folder),
                (
                    "content/folder-1/notes-1/index.md",
                    "---\ntitle: Notes 1\n---\n",
                ),
                (
                    "content/folder-1/notes-2/index.md",
                    "---\ntitle: Notes 2\n---\n",
                ),
                (
                    "content/folder-1/notes-1/note-1.md",
                    "---\ntitle: Note 1\n---\n",
                ),
                (
                    "content/folder-1/notes-2/note-2.md",
                    "---\ntitle: Note 2\n---\n",
                ),
            ],
            Some((
                "true",
                "content/folder-1/index.md",
                "Feed not written: the folder `content/folder-1/notes-1/` has nothing dated",
            )),
        ),
        (
            vec![
                ("content/notes/index.md", &true_line),
                ("content/notes/note-3.md", dated),
            ],
            None,
        ),
        (vec![("content/notes/index.md", &empty)], None),
    ];
    for (files, warned) in cases {
        let mut all: Files = vec![
            SETTINGS,
            HOME,
            PAGE,
            HOME_TEMPLATE,
            FOLDER,
            FEED,
            (&note_1.0, &note_1.1),
            (&note_2.0, &note_2.1),
        ];
        all.retain(|(path, _)| !files.iter().any(|(p, _)| p == path));
        all.extend(files.iter().copied());
        let built = with_posts(&all);
        let sources = built.outcome.sources();
        let found: Vec<(String, String, String)> = (built.outcome.problems.iter())
            .map(|p| {
                let place = p.place.expect("placed");
                (
                    sources.spelling(place).to_owned(),
                    sources.path(place.source).display().to_string(),
                    crate::wording::headline(&p.kind),
                )
            })
            .collect();
        let wanted: Vec<(String, String, String)> = (warned.iter())
            .map(|(at, path, said)| (at.to_string(), path.to_string(), said.to_string()))
            .collect();
        assert_eq!(found, wanted, "{files:?}");
        if warned.is_some() {
            let suggestion = built.outcome.problems[0].suggestion();
            assert_eq!(
                suggestion.as_deref(),
                Some("Give a page in it a `date:`, or take the `feed:` line out.")
            );
        }
    }
}

/// A site whose only feed asked for is of a list with nothing dated asks
/// for no `url` or `feed_key`.
#[test]
fn a_feed_of_nothing_dated_asks_for_no_address_or_key() {
    let settings = ("statac.yaml", "language: en\ntitle: Site\ntimezone: UTC\n");
    let notes = (
        "content/notes/index.md",
        "---\ntitle: Notes\nfeed: true\n---\n",
    );
    let note = ("content/notes/note-1.md", "---\ntitle: Note 1\n---\n");
    let built = Built::new(&[
        settings,
        HOME,
        PAGE,
        HOME_TEMPLATE,
        FOLDER,
        FEED,
        notes,
        note,
    ]);
    assert_eq!(built.told(), [("feed-not-written", Some("true".into()))]);
}

#[test]
fn a_page_of_the_authors_at_a_feeds_address_wins() {
    let page = (
        "content/feed.md",
        "---\ntitle: Feed\npermalink: /feed.xml\n---\n",
    );
    let built = with_posts(&[SETTINGS, HOME, PAGE, HOME_TEMPLATE, FOLDER, FEED, page]);
    assert_eq!(
        built.kinds(),
        [Kind::FeedNotWritten {
            list: (Word::Folder, String::new()),
            more: 0,
            why: Unwritten::PageThere {
                page: "content/feed.md".into(),
                address: "/feed.xml".into(),
            },
        }]
    );
    assert_eq!(built.file("feed.xml").as_deref(), Some("Feed"));
    assert!(built.file("posts/feed.xml").is_some());
}

#[test]
fn a_feed_and_a_static_file_at_one_address_are_refused_naming_both() {
    let file = ("static/feed.xml", "<feed/>");
    let built = with_posts(&[SETTINGS, HOME, PAGE, HOME_TEMPLATE, FOLDER, FEED, file]);
    let same: Vec<(String, String)> = built
        .kinds()
        .into_iter()
        .filter_map(|kind| match kind {
            Kind::SameAddress { first, second, .. } => match (*first, *second) {
                (Named::File(first), Named::File(second)) => Some((first, second)),
                _ => None,
            },
            _ => None,
        })
        .collect();
    assert_eq!(
        same,
        [(
            "static/feed.xml".to_owned(),
            "templates/feed.xml".to_owned()
        )]
    );
}

#[test]
fn a_feed_value_that_names_no_file_stops_at_the_value_with_the_nearest() {
    let cases: Vec<(&str, &str, &str, AddressFault)> = vec![
        (
            "content/index.md",
            "feed: /feeds/",
            "/feeds/",
            AddressFault::FeedAtFolder {
                nearest: "/feeds/feed.xml".into(),
            },
        ),
        (
            "content/index.md",
            "feed: /feed",
            "/feed",
            AddressFault::FeedAtFolder {
                nearest: "/feed.xml".into(),
            },
        ),
        (
            "content/posts/index.md",
            "feed: atom",
            "atom",
            AddressFault::FeedAtFolder {
                nearest: "atom.xml".into(),
            },
        ),
        (
            "content/posts/index.md",
            "feed: \"feeds/atom.xml\"",
            "feeds/atom.xml",
            AddressFault::SlashBesidePage {
                nearest: "/posts/feeds/atom.xml".into(),
            },
        ),
        (
            "content/posts/index.md",
            "feed: a%b.xml",
            "a%b.xml",
            AddressFault::Percent,
        ),
    ];
    for (path, line, at, fault) in cases {
        let text = format!("---\ntitle: List\n{line}\n---\n");
        let built = with_posts(&[SETTINGS, PAGE, HOME_TEMPLATE, FOLDER, FEED, (path, &text)]);
        let told: Vec<(Kind, Option<String>)> = built
            .kinds()
            .into_iter()
            .zip(built.told().into_iter().map(|(_, at)| at))
            .filter(|(kind, _)| matches!(kind, Kind::UnusableAddress { .. }))
            .collect();
        assert_eq!(
            told,
            [(Kind::UnusableAddress { fault }, Some(at.to_owned()))],
            "{line}"
        );
    }
}

/// A feed printing the id of its list's page, then each post's.
const IDS: (&str, &str) = (
    "templates/feed.xml",
    "{{ feed_id(page) }}{% for post in posts %} {{ feed_id(post) }}{% endfor %}",
);

/// Each id fixed for good, as feeds have published it for these posts,
/// in a feed of `content/posts/`, with `feed_key: key`.
#[test]
fn entry_ids_are_those_the_previous_build_published() {
    let posts = [
        (
            "content/posts/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-01-01\n---\n",
        ),
        (
            "content/posts/post-2.md",
            "---\ntitle: Post 2\ndate: 2026-01-02\nid: abc123\n---\n",
        ),
        (
            "content/posts/post-3.md",
            "---\ntitle: Post 3\ndate: 2026-01-03\nid: 0a1b2c3d-0000-8000-8000-000000000001\n---\n",
        ),
        (
            "content/posts/post-4.md",
            "---\ntitle: Post 4\ndate: 2026-01-04\npermalink: /posts/café 4/\n---\n",
        ),
    ];
    let base = "language: en\ntitle: Site\nfeed_key: key\ntimezone: Europe/London\n";
    let cases = [
        (
            "url: https://example.org\n",
            "urn:uuid:1cae012e-d114-86b3-a710-291c0057851c \
             urn:uuid:38b188e3-d6db-8c9c-96b2-861ba5ccf39d \
             urn:uuid:0a1b2c3d-0000-8000-8000-000000000001 \
             urn:uuid:d7717683-3178-8849-9ff5-aa862293d9ef \
             urn:uuid:fc370633-88f0-8092-9f96-7749dbfff0eb",
        ),
        (
            // Under a path, the same ids: they are made from addresses
            // within the site.
            "url: https://example.org/sub/\n",
            "urn:uuid:1cae012e-d114-86b3-a710-291c0057851c \
             urn:uuid:38b188e3-d6db-8c9c-96b2-861ba5ccf39d \
             urn:uuid:0a1b2c3d-0000-8000-8000-000000000001 \
             urn:uuid:d7717683-3178-8849-9ff5-aa862293d9ef \
             urn:uuid:fc370633-88f0-8092-9f96-7749dbfff0eb",
        ),
        (
            "url: https://example.org\nfeed_ids_before: 2026-01-05\n",
            "https://example.org/posts/feed.xml \
             https://example.org/posts/caf%C3%A9%204/ \
             https://example.org/posts/post-3/ \
             https://example.org/posts/post-2/ \
             https://example.org/posts/post-1/",
        ),
        (
            "url: https://example.org/sub/\nfeed_ids_before: 2026-01-03\n",
            "https://example.org/sub/posts/feed.xml \
             urn:uuid:38b188e3-d6db-8c9c-96b2-861ba5ccf39d \
             urn:uuid:0a1b2c3d-0000-8000-8000-000000000001 \
             https://example.org/sub/posts/post-2/ \
             https://example.org/sub/posts/post-1/",
        ),
        (
            "url: https://example.org/sub/\nfeed_ids_before: 2026-01-05\nfeed_ids_from: jekyll\n",
            "https://example.org/sub/posts/feed.xml \
             https://example.org/sub/posts/post-4 \
             https://example.org/sub/posts/post-3 \
             https://example.org/sub/posts/post-2 \
             https://example.org/sub/posts/post-1",
        ),
    ];
    for (settings, ids) in cases {
        let settings = format!("{base}{settings}");
        let mut files = vec![
            ("statac.yaml", settings.as_str()),
            HOME,
            PAGE,
            HOME_TEMPLATE,
        ];
        files.extend([FOLDER, IDS]);
        files.extend(posts);
        let built = Built::new(&files);
        assert!(
            built.outcome.problems.is_empty(),
            "{settings}: {:?}",
            built.kinds()
        );
        assert_eq!(
            built.file("posts/feed.xml").as_deref(),
            Some(ids),
            "{settings}"
        );
    }
}

#[test]
fn feed_id_without_a_key_stops_the_template_with_one_to_paste() {
    let settings = ("statac.yaml", "language: en\ntitle: Site\ntimezone: UTC\n");
    let page = ("templates/page.html", "{{ feed_id(page) }}");
    let built = Built::new(&[
        settings,
        ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
        page,
        ("templates/home.html", ""),
    ]);
    let kinds = built.kinds();
    assert!(
        matches!(
            kinds.as_slice(),
            [Kind::TemplateStopped {
                reason: crate::problem::Stopped::NoFeedKey(_)
            }]
        ),
        "{kinds:?}"
    );
}

const SITEMAP: (&str, &str) = (
    "content/sitemap.md",
    "---\npermalink: /sitemap.xml\nlayout: sitemap.xml\nneeds: url\n---\n",
);
const SITEMAP_TEMPLATE: (&str, &str) = (
    "templates/sitemap.xml",
    "<urlset>{% for entry in site.pages %}<url><loc>{{ entry.url | absolute }}</loc>\
     <title>{{ entry.title | default('') }}</title></url>{% endfor %}</urlset>",
);
const ROBOTS: (&str, &str) = (
    "content/robots.md",
    "---\npermalink: /robots.txt\nlayout: robots.txt\nneeds: url\n---\n",
);
const ROBOTS_TEMPLATE: (&str, &str) = (
    "templates/robots.txt",
    "Sitemap: {{ '/sitemap.xml' | absolute }} & {{ site.title }}",
);
const NOT_FOUND: (&str, &str) = (
    "content/404.md",
    "---\ntitle: Not found\npermalink: /404.html\nsearch: false\n---\n",
);

fn other_files(settings: &str) -> Built {
    Built::new(&[
        ("statac.yaml", settings),
        ("content/index.md", "---\ntitle: Home & away\n---\n"),
        ("content/мир.md", "---\ntitle: Мир\n---\n"),
        PAGE,
        HOME_TEMPLATE,
        SITEMAP,
        SITEMAP_TEMPLATE,
        ROBOTS,
        ROBOTS_TEMPLATE,
        NOT_FOUND,
    ])
}

#[test]
fn other_files_are_built_through_their_templates_escaped_for_their_kind() {
    let built = other_files("language: en\ntitle: Site & co\nurl: https://example.org\n");
    assert!(built.outcome.problems.is_empty(), "{:?}", built.kinds());
    let sitemap = built.file("sitemap.xml").expect("a sitemap");
    assert!(
        sitemap.contains("<loc>https://example.org/%D0%BC%D0%B8%D1%80/</loc><title>Мир</title>"),
        "{sitemap}"
    );
    assert!(
        sitemap.contains("<title>Home &amp; away</title>"),
        "{sitemap}"
    );
    assert_eq!(
        built.file("robots.txt").as_deref(),
        Some("Sitemap: https://example.org/sitemap.xml & Site & co")
    );
    assert_eq!(built.file("404.html").as_deref(), Some("Not found"));
}

#[test]
fn a_page_that_needs_the_sites_address_waits_for_it_and_is_said_left_out() {
    let built = other_files("language: en\ntitle: Site\n");
    assert_eq!(
        built.told(),
        [
            ("no-site-address", Some("url".into())),
            ("no-site-address", Some("url".into())),
        ]
    );
    let kinds = built.kinds();
    assert!(kinds.iter().all(|kind| !kind.quietenable()));
    assert!(matches!(
        &kinds[0],
        Kind::NoSiteAddress {
            waiting: Waiting::Page(path)
        } if path == "content/robots.md"
    ));
    assert!(built.file("sitemap.xml").is_none());
    assert!(built.file("robots.txt").is_none());
    assert!(built.file("404.html").is_some());
}

#[test]
fn a_site_under_a_path_publishes_robots_and_404_beneath_it_and_notes_robots() {
    let built = other_files("language: en\ntitle: Site\nurl: https://example.org/sub/\n");
    assert_eq!(
        built.told(),
        [("robots-beneath-path", Some("/robots.txt".into()))]
    );
    assert_eq!(
        built.kinds()[0],
        Kind::RobotsBeneathPath {
            top: "https://example.org/robots.txt".into()
        }
    );
    assert!(built.file("robots.txt").is_some());
    assert_eq!(built.file("404.html").as_deref(), Some("Not found"));
    let sitemap = built.file("sitemap.xml").expect("a sitemap");
    assert!(
        sitemap.contains("<loc>https://example.org/sub/%D0%BC%D0%B8%D1%80/</loc>"),
        "{sitemap}"
    );
}
