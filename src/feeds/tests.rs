use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

/// The site's feed and a folder's.
type Feeds = (Option<String>, Option<String>);

/// What a case is called, the files it adds, and the two feeds.
type Case<'a> = (
    &'a str,
    Vec<(&'a str, String)>,
    Option<String>,
    Option<String>,
);

/// The site's feed and the feed of the folder `posts`, for a site of
/// `files` beside a `statac.yaml` holding `settings`.
fn feeds(settings: &str, files: &[(&str, &str)]) -> Feeds {
    feeds_of(settings, files, "posts")
}

/// The same, with the feed of `folder`.
fn feeds_of(settings: &str, files: &[(&str, &str)], folder: &str) -> Feeds {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root: PathBuf = std::env::temp_dir().join(format!(
        "statac-feeds-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("content")).unwrap();
    std::fs::write(root.join("statac.yaml"), settings).unwrap();
    for (path, text) in files {
        let path = root.join("content").join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    let run = crate::content::Run {
        today: Some(crate::date::Date::day_only(2026, 10, 4)),
        drafts: false,
        url: None,
    };
    let read = crate::content::read(&root, &run);
    let _ = std::fs::remove_dir_all(&root);
    assert!(
        read.problems
            .iter()
            .all(|p| p.kind.severity() != crate::problem::Severity::Error),
        "{:?}",
        read.problems
    );
    let list = read
        .site
        .lists
        .iter()
        .find(|(_, list)| list.key == ListKey::Folder(folder.into()))
        .map(|(id, _)| id);
    let at = |list: ListId| {
        address(&read.site, &read.settings, list).map(|address| address.as_str().to_owned())
    };
    let site = read
        .site
        .own_list(crate::site::Id::from_index(0))
        .and_then(at);
    let folder = list.and_then(at);
    (site, folder)
}

const SETTINGS: &str = "title: Site\nurl: https://example.org\nfeed_key: key\ntimezone: UTC\n";

const POST: (&str, &str) = ("posts/post-1.md", "---\ndate: 2026-01-01\n---\nText.\n");

fn some(address: &str) -> Option<String> {
    Some(address.into())
}

#[test]
fn a_list_with_posts_has_its_feed_beside_its_page() {
    let index = |feed: &str| format!("---\n{feed}\n---\n");
    let posts_index = |feed: &str| ("posts/index.md", index(feed));
    let cases: Vec<Case> = vec![
        (
            "nothing said",
            vec![],
            some("/feed.xml"),
            some("/posts/feed.xml"),
        ),
        (
            "the site's page names its feed",
            vec![("index.md", index("feed: /feed/index.xml"))],
            some("/feed/index.xml"),
            some("/posts/feed.xml"),
        ),
        (
            "a name set beside the page",
            vec![posts_index("feed: atom.xml")],
            some("/feed.xml"),
            some("/posts/atom.xml"),
        ),
        (
            "a folder's page moved",
            vec![posts_index("permalink: /blog/")],
            some("/feed.xml"),
            some("/blog/feed.xml"),
        ),
        (
            "a folder's page moved to a file",
            vec![posts_index("permalink: /blog.html")],
            some("/feed.xml"),
            some("/blog.html.feed.xml"),
        ),
        (
            "the site's page moved",
            vec![("index.md", index("permalink: /start/"))],
            some("/start/feed.xml"),
            some("/posts/feed.xml"),
        ),
        (
            "a folder holds its feed back",
            vec![posts_index("feed: false")],
            some("/feed.xml"),
            None,
        ),
        (
            "a name set beside the site's page",
            vec![("index.md", index("feed: index.rss"))],
            some("/index.rss"),
            some("/posts/feed.xml"),
        ),
        (
            "a page published nowhere",
            vec![posts_index("index_page: false")],
            some("/feed.xml"),
            None,
        ),
        (
            "a page published nowhere that asks",
            vec![posts_index("index_page: false\nfeed: true")],
            some("/feed.xml"),
            some("/posts/feed.xml"),
        ),
        (
            "a name that is a folder",
            vec![posts_index("feed: /feeds/")],
            some("/feed.xml"),
            None,
        ),
        (
            "a folder's page sending readers on",
            vec![posts_index("redirect_to: /")],
            some("/feed.xml"),
            None,
        ),
        (
            "a folder's page sending readers on that asks",
            vec![posts_index("redirect_to: /\nfeed: true")],
            some("/feed.xml"),
            None,
        ),
        (
            "the site's page sending readers on that names its feed",
            vec![("index.md", index("redirect_to: /posts/\nfeed: /index.rss"))],
            None,
            some("/posts/feed.xml"),
        ),
    ];
    for (case, files, site, folder) in cases {
        let mut all: Vec<(&str, &str)> = vec![POST];
        all.extend(files.iter().map(|(path, text)| (*path, text.as_str())));
        assert_eq!(feeds(SETTINGS, &all), (site, folder), "{case}");
    }
}

#[test]
fn every_feed_is_held_back_by_a_line_handed_to_every_page() {
    let settings = format!("{SETTINGS}pages.feed: false\n");
    assert_eq!(feeds(&settings, &[POST]), (None, None));
    let named = [POST, ("index.md", "---\nfeed: /index.rss\n---\n")];
    assert_eq!(feeds(&settings, &named), (some("/index.rss"), None));
}

#[test]
fn a_feed_needs_the_sites_address_key_and_title_and_a_post() {
    let unkeyed = "title: Site\nurl: https://example.org\ntimezone: UTC\n";
    assert_eq!(feeds(unkeyed, &[POST]), (None, None));
    let undated = ("posts/post-1.md", "Text.\n");
    assert_eq!(feeds(SETTINGS, &[undated]), (None, None));
}

#[test]
fn a_folder_below_the_top_has_a_feed_only_where_its_page_asks() {
    let nested = (
        "posts/2026/post-2.md",
        "---\ndate: 2026-01-02\n---\nText.\n",
    );
    // The folder `posts` carries the posts of the folders inside it.
    assert_eq!(
        feeds(SETTINGS, &[POST, nested]),
        (some("/feed.xml"), some("/posts/feed.xml"))
    );
    assert_eq!(
        feeds_of(SETTINGS, &[POST, nested], "posts/2026"),
        (some("/feed.xml"), None)
    );
    let asks = ("posts/2026/index.md", "---\nfeed: true\n---\n");
    assert_eq!(
        feeds_of(SETTINGS, &[POST, nested, asks], "posts/2026"),
        (some("/feed.xml"), some("/posts/2026/feed.xml"))
    );
}

mod known;
mod written;
