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

/// What a case is called, what the settings add, the files it adds, and
/// the two feeds.
type Asking<'a> = (&'a str, &'a str, Vec<(&'a str, String)>, Feeds);

/// A file under `content/` and its text.
type File<'a> = (&'a str, &'a str);

/// What a case is called, what the settings add, the folder's page where
/// one is written, and the folder's feeds.
type Naming<'a> = (&'a str, &'a str, Option<(&'a str, String)>, Vec<&'a str>);

/// The site's feed and the feed of the folder `posts`, for a site of
/// `files` beside a `statac.yaml` holding `settings`.
fn feeds(settings: &str, files: &[(&str, &str)]) -> Feeds {
    feeds_of(settings, files, "posts")
}

/// The same, with the feed of `folder`.
fn feeds_of(settings: &str, files: &[(&str, &str)], folder: &str) -> Feeds {
    let read = read(settings, files);
    let list = read
        .site
        .lists
        .iter()
        .find(|(_, list)| list.key == ListKey::Folder(folder.into()))
        .map(|(id, _)| id);
    let site = own_feed(&read, 0);
    let folder = list.and_then(|list| feed_at(&read, list));
    (site, folder)
}

/// The feeds of the own lists of a site's first two languages.
fn languages(settings: &str, files: &[(&str, &str)]) -> Feeds {
    let read = read(settings, files);
    (own_feed(&read, 0), own_feed(&read, 1))
}

/// Where `list`'s feed is published.
fn feed_at(read: &crate::content::Read, list: ListId) -> Option<String> {
    address(&read.site, &read.settings, list).map(|address| address.as_str().to_owned())
}

/// Where the own list of the language at `index` has its feed.
fn own_feed(read: &crate::content::Read, index: usize) -> Option<String> {
    let language = crate::ids::Id::from_index(index);
    read.site
        .own_list(language)
        .and_then(|list| feed_at(read, list))
}

/// The site of `files` under `content/` beside a `statac.yaml` holding
/// `settings`, read, with no errors. Each folder `files` reaches has an
/// `index.md` where they write none, as a list has a feed only with a page.
fn read(settings: &str, files: &[(&str, &str)]) -> crate::content::Read {
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
    let mut folders = vec![root.join("content")];
    for (path, _) in files {
        let mut at = root.join("content").join(path);
        while at.pop() && at.starts_with(root.join("content")) {
            folders.push(at.clone());
        }
    }
    for folder in folders {
        let index = folder.join("index.md");
        if !index.exists() {
            std::fs::write(index, "---\n---\n").unwrap();
        }
    }
    let run = crate::content::Run {
        today: Some(crate::date::Date::day_only(2026, 10, 4)),
        drafts: false,
        url: None,
        renamed: Vec::new(),
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
    read
}

/// Settings asking for no feed.
const UNFED: &str = "title: Site\nurl: https://example.org\nfeed_key: key\ntimezone: UTC\n";

/// Settings asking for a feed of every list.
const SETTINGS: &str =
    "title: Site\nurl: https://example.org\nfeed_key: key\ntimezone: UTC\npages.feed: true\n";

const POST: (&str, &str) = ("posts/post-1.md", "---\ndate: 2026-01-01\n---\nText.\n");

fn some(address: &str) -> Option<String> {
    Some(address.into())
}

/// Where each list asks, from the settings and its page, times the feeds
/// of the site's own list and the folder `posts`: a list has a feed only
/// where a line asks, the site's page's own line wins over the setting,
/// and the setting over a line handed to every page.
#[test]
fn a_list_has_a_feed_only_where_a_line_asks() {
    let home = |feed: &str| ("index.md", format!("---\n{feed}\n---\n"));
    let posts = |feed: &str| ("posts/index.md", format!("---\n{feed}\n---\n"));
    // (the case, what the settings add, the files, the two feeds)
    let cases: Vec<Asking> = vec![
        ("nothing asks", "", vec![], (None, None)),
        (
            "the setting",
            "feed: true\n",
            vec![],
            (some("/feed.xml"), None),
        ),
        (
            "the setting refusing",
            "feed: false\n",
            vec![],
            (None, None),
        ),
        (
            "the setting naming the feed",
            "feed: index.rss\n",
            vec![],
            (some("/index.rss"), None),
        ),
        (
            "the setting giving the feed a URL",
            "feed: /feeds/site.xml\n",
            vec![],
            (some("/feeds/site.xml"), None),
        ),
        (
            "the setting with a page that says nothing of it",
            "feed: true\n",
            vec![home("title: Home")],
            (some("/feed.xml"), None),
        ),
        (
            "the site's page",
            "",
            vec![home("feed: true")],
            (some("/feed.xml"), None),
        ),
        (
            "the site's page refusing the setting",
            "feed: true\n",
            vec![home("feed: false")],
            (None, None),
        ),
        (
            "the site's page asking over the setting",
            "feed: false\n",
            vec![home("feed: true")],
            (some("/feed.xml"), None),
        ),
        (
            "the site's page naming the feed over the setting",
            "feed: index.rss\n",
            vec![home("feed: atom.xml")],
            (some("/atom.xml"), None),
        ),
        (
            "a folder's page",
            "",
            vec![posts("feed: true")],
            (None, some("/posts/feed.xml")),
        ),
        (
            "a folder's page and the setting",
            "feed: true\n",
            vec![posts("feed: true")],
            (some("/feed.xml"), some("/posts/feed.xml")),
        ),
        (
            "a line handed to every page",
            "pages.feed: true\n",
            vec![],
            (some("/feed.xml"), some("/posts/feed.xml")),
        ),
        (
            "the setting refusing what is handed to every page",
            "pages.feed: true\nfeed: false\n",
            vec![],
            (None, some("/posts/feed.xml")),
        ),
        (
            "the setting asking where every page is refused",
            "pages.feed: false\nfeed: true\n",
            vec![],
            (some("/feed.xml"), None),
        ),
        (
            "the site's page refusing what is handed to every page",
            "pages.feed: true\n",
            vec![home("feed: false")],
            (None, some("/posts/feed.xml")),
        ),
    ];
    for (case, added, files, wanted) in cases {
        let settings = format!("{UNFED}{added}");
        let mut all: Vec<(&str, &str)> = vec![POST];
        all.extend(files.iter().map(|(path, text)| (*path, text.as_str())));
        assert_eq!(feeds(&settings, &all), wanted, "{case}");
    }
}

/// The setting gives each language's own list a feed, on a site in several
/// languages, and a language's page wins over it for that language alone.
#[test]
fn the_setting_asks_for_each_languages_own_list() {
    let settings = "title: Site\nurl: https://example.org\nfeed_key: key\ntimezone: UTC\n\
                    language: [en, fr]\n";
    let posts = [
        ("post-1.md", "---\ndate: 2026-01-01\n---\nText.\n"),
        ("fr/post-1.md", "---\ndate: 2026-01-01\n---\nTexte.\n"),
    ];
    let refused = ("fr/index.md", "---\nfeed: false\n---\n");
    let asked = ("fr/index.md", "---\nfeed: true\n---\n");
    // (what the settings add, the French page, the two languages' feeds)
    let cases: Vec<(&str, Option<File>, Feeds)> = vec![
        ("", None, (None, None)),
        (
            "feed: true\n",
            None,
            (some("/feed.xml"), some("/fr/feed.xml")),
        ),
        ("feed: true\n", Some(refused), (some("/feed.xml"), None)),
        ("", Some(asked), (None, some("/fr/feed.xml"))),
    ];
    for (added, page, wanted) in cases {
        let settings = format!("{settings}{added}");
        let mut files = posts.to_vec();
        files.extend(page);
        assert_eq!(languages(&settings, &files), wanted, "{added}{page:?}");
    }
}

#[test]
fn a_list_with_posts_has_its_feed_beside_its_page() {
    let index = |feed: &str| format!("---\n{feed}\n---\n");
    let posts_index = |feed: &str| ("posts/index.md", index(feed));
    let cases: Vec<Case> = vec![
        (
            "nothing more said",
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
        // A feed links to its list's page, so a list whose page isn't
        // published has none.
        (
            "a page published nowhere",
            vec![posts_index("index_page: false")],
            some("/feed.xml"),
            None,
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
    let settings = format!("{UNFED}pages.feed: false\n");
    assert_eq!(feeds(&settings, &[POST]), (None, None));
    let named = [POST, ("index.md", "---\nfeed: /index.rss\n---\n")];
    assert_eq!(feeds(&settings, &named), (some("/index.rss"), None));
}

#[test]
fn a_feed_needs_the_sites_address_key_and_title_and_a_post() {
    let unkeyed = "title: Site\nurl: https://example.org\ntimezone: UTC\npages.feed: true\n";
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
    assert_eq!(feeds_of(UNFED, &[POST, nested], "posts/2026"), (None, None));
    let asks = ("posts/2026/index.md", "---\nfeed: true\n---\n");
    assert_eq!(
        feeds_of(UNFED, &[POST, nested, asks], "posts/2026"),
        (None, some("/posts/2026/feed.xml"))
    );
}

/// A `feed:` line naming several feeds, times where each list has them:
/// one at each name, in the order written, handed down whole, the first
/// being the list's feed, and none for a name giving an address a name
/// before it gives, or a list naming none.
#[test]
fn a_list_naming_several_feeds_has_one_at_each() {
    let posts = |feed: &str| ("posts/index.md", format!("---\n{feed}\n---\n"));
    // (the case, what the settings add, the folder's page, its feeds)
    let cases: Vec<Naming> = vec![
        (
            "two names",
            "",
            Some(posts("feed: [feed.xml, rss.xml]")),
            vec!["/posts/feed.xml", "/posts/rss.xml"],
        ),
        (
            "in the order written",
            "",
            Some(posts("feed: [rss.xml, feed.xml]")),
            vec!["/posts/rss.xml", "/posts/feed.xml"],
        ),
        (
            "one from the top of the site",
            "",
            Some(posts("feed: [feed.xml, /feeds/posts.xml]")),
            vec!["/posts/feed.xml", "/feeds/posts.xml"],
        ),
        (
            "one name in a list",
            "",
            Some(posts("feed: [rss.xml]")),
            vec!["/posts/rss.xml"],
        ),
        (
            "handed down whole",
            "pages.feed: [feed.xml, rss.xml]\n",
            None,
            vec!["/posts/feed.xml", "/posts/rss.xml"],
        ),
        (
            "a page's own line over what is handed down",
            "pages.feed: [feed.xml, rss.xml]\n",
            Some(posts("feed: atom.xml")),
            vec!["/posts/atom.xml"],
        ),
        (
            "a page refusing what is handed down",
            "pages.feed: [feed.xml, rss.xml]\n",
            Some(posts("feed: false")),
            vec![],
        ),
        (
            "the same name twice",
            "",
            Some(posts("feed: [feed.xml, rss.xml, feed.xml]")),
            vec!["/posts/feed.xml", "/posts/rss.xml"],
        ),
        (
            "two names giving one address",
            "",
            Some(posts("feed: [feed.xml, /posts/feed.xml]")),
            vec!["/posts/feed.xml"],
        ),
        (
            "two names differing in capitals",
            "",
            Some(posts("feed: [feed.xml, Feed.xml]")),
            vec!["/posts/feed.xml"],
        ),
        (
            "a name naming no file beside one that does",
            "",
            Some(posts("feed: [/feeds/, rss.xml]")),
            vec!["/posts/rss.xml"],
        ),
        ("no names", "", Some(posts("feed: []")), vec![]),
    ];
    for (case, added, page, wanted_feeds) in cases {
        let settings = format!("{UNFED}{added}");
        let mut files: Vec<(&str, &str)> = vec![POST];
        files.extend(page.iter().map(|(path, text)| (*path, text.as_str())));
        let read = read(&settings, &files);
        let list = (read.site.lists.iter())
            .find(|(_, list)| list.key == ListKey::Folder("posts".into()))
            .map(|(id, _)| id)
            .unwrap();
        let feeds = wanted(&read.site, &read.settings, list);
        let feeds: Vec<&str> = feeds.iter().map(Address::as_str).collect();
        assert_eq!(feeds, wanted_feeds, "{case}");
        let first = wanted_feeds.first().map(|first| (*first).to_owned());
        assert_eq!(feed_at(&read, list), first, "{case}");
    }
}

#[test]
fn the_setting_naming_several_feeds_gives_the_sites_own_list_each() {
    let settings = format!("{UNFED}feed: [atom.xml, /rss.xml]\n");
    let read = read(&settings, &[POST]);
    let list = read.site.own_list(crate::ids::Id::from_index(0)).unwrap();
    let feeds = wanted(&read.site, &read.settings, list);
    let feeds: Vec<&str> = feeds.iter().map(Address::as_str).collect();
    assert_eq!(feeds, ["/atom.xml", "/rss.xml"]);
    assert_eq!(feed_at(&read, list), some("/atom.xml"));
}

/// Where a feed is published, times the templates it may be built with,
/// in the order they are looked for.
#[test]
fn a_feed_looks_for_the_template_named_as_its_file_then_by_its_extension() {
    let cases = [
        ("/feed.xml", vec!["feed.xml"]),
        ("/rss.xml", vec!["rss.xml", "feed.xml"]),
        ("/posts/atom.xml", vec!["atom.xml", "feed.xml"]),
        ("/feed.json", vec!["feed.json", "feed.xml"]),
        ("/posts.json", vec!["posts.json", "feed.json", "feed.xml"]),
        ("/index.rss", vec!["index.rss", "feed.rss", "feed.xml"]),
        (
            "/blog.html.feed.xml",
            vec!["blog.html.feed.xml", "feed.xml"],
        ),
        ("/feed.html", vec!["feed.xml"]),
        ("/feed", vec!["feed.xml"]),
    ];
    for (address, wanted) in cases {
        let address = Address::read(address).unwrap();
        assert_eq!(looked_for(&address), wanted, "{}", address.as_str());
    }
    let address = Address::read("/rss.xml").unwrap();
    let has = |names: &'static [&'static str]| move |name: &str| names.contains(&name);
    assert_eq!(
        template(&address, &has(&["feed.xml", "rss.xml"])),
        some("rss.xml")
    );
    assert_eq!(template(&address, &has(&["feed.xml"])), some("feed.xml"));
    assert_eq!(template(&address, &has(&["feed.json"])), None);
}

mod known;
mod written;
