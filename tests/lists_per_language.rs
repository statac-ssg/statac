//! A site in several languages built end to end: each language's pages,
//! lists and feeds published beneath its own top.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

struct Site {
    root: PathBuf,
}

const SETTINGS: &str = "language: [ar, en]\ntitle: Site 1\nen.title: Site 2\n\
                        url: https://example.org\nfeed_key: key\ntimezone: UTC\n\
                        tags.pages: true\n";

/// What each list's page shows: its language, the site's title, posts,
/// feed and tags as `site` answers them on it.
const HOME: &str = "{{ page.language }}|{{ site.title }}|\
                    {% for post in site.posts %}{{ post.url }};{% endfor %}|{{ site.feed }}|\
                    {% for tag in site.tags %}{{ tag.url }};{% endfor %}";

/// What a post shows: where it stands, and its neighbours.
const PAGE: &str = "{{ page.url }}|{% if page.previous %}{{ page.previous.url }}{% endif %}|\
                    {% if page.next %}{{ page.next.url }}{% endif %}";

/// What a folder's, tag's or series' page shows: its posts, and the page
/// after it.
const FOLDER: &str = "{% for post in page.posts %}{{ post.url }};{% endfor %}|\
                      {% if page.next %}{{ page.next.url }}{% endif %}";

const FEED: &str = "{{ site.title }}|{% for post in posts %}{{ post.url }};{% endfor %}";

fn post(date: &str, tags: &str) -> String {
    format!("---\ndate: {date}\ntags: [{tags}]\n---\nText.\n")
}

impl Site {
    fn new(name: &str, settings: &str) -> Site {
        let root =
            std::env::temp_dir().join(format!("statac-languages-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let site = Site { root };
        site.write("statac.yaml", settings);
        site.write("templates/home.html", HOME);
        site.write("templates/page.html", PAGE);
        site.write("templates/folder.html", FOLDER);
        site.write("templates/feed.xml", FEED);
        site.write("templates/redirect.html", "{{ to }}");
        site
    }

    fn write(&self, path: &str, text: &str) {
        let path = self.root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn run(&self, command: &str) -> (String, i32) {
        let output = Command::new(env!("CARGO_BIN_EXE_statac"))
            .args([command, "--today=2026-10-04"])
            .arg(&self.root)
            .env("NO_COLOR", "1")
            .output()
            .unwrap();
        assert!(output.stderr.is_empty(), "{output:?}");
        let said = String::from_utf8(output.stdout).unwrap();
        let shown = self.root.display().to_string();
        (said.replace(&shown, "SITE"), output.status.code().unwrap())
    }

    fn built(&self) -> String {
        let (said, code) = self.run("build");
        assert_eq!(code, 0, "{said}");
        said
    }

    fn read(&self, path: &str) -> String {
        fs::read_to_string(self.root.join("_site").join(path))
            .unwrap_or_else(|_| panic!("nothing at {path}"))
    }

    fn has(&self, path: &str) -> bool {
        self.root.join("_site").join(path).exists()
    }
}

impl Drop for Site {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn each_language_has_its_own_home_posts_tags_and_feed() {
    let site = Site::new("tops", &format!("{SETTINGS}feed: true\n"));
    site.write("content/post-1.md", &post("2026-01-01", "Rust"));
    site.write("content/en/post-1.md", &post("2026-01-02", "Rust"));
    site.built();
    assert_eq!(
        site.read("index.html"),
        "ar|Site 1|/post-1/;|/feed.xml|/tags/rust/;"
    );
    assert_eq!(
        site.read("en/index.html"),
        "en|Site 2|/en/post-1/;|/en/feed.xml|/en/tags/rust/;"
    );
    assert_eq!(site.read("tags/rust/index.html"), "/post-1/;|");
    assert_eq!(site.read("en/tags/rust/index.html"), "/en/post-1/;|");
    assert_eq!(site.read("feed.xml"), "Site 1|/post-1/;");
    assert_eq!(site.read("en/feed.xml"), "Site 2|/en/post-1/;");
}

#[test]
fn the_main_language_in_a_folder_of_its_own_leaves_the_top_to_the_author() {
    let site = Site::new("main-folder", &format!("{SETTINGS}feed: true\n"));
    site.write("content/index.md", "---\ntitle: Welcome\n---\n");
    site.write("content/ar/post-1.md", &post("2026-01-01", "Rust"));
    site.write("content/en/post-1.md", &post("2026-01-02", "Rust"));
    site.built();
    // The page outside every language folder is in the main language, and
    // `site` answers for it.
    assert_eq!(
        site.read("index.html"),
        "ar|Site 1|/ar/post-1/;|/ar/feed.xml|/ar/tags/rust/;"
    );
    assert_eq!(
        site.read("ar/index.html"),
        "ar|Site 1|/ar/post-1/;|/ar/feed.xml|/ar/tags/rust/;"
    );
    assert_eq!(site.read("ar/feed.xml"), "Site 1|/ar/post-1/;");
    assert_eq!(site.read("en/feed.xml"), "Site 2|/en/post-1/;");
    // The top of `content/` is no language's own list, so the setting gives
    // it no feed.
    assert!(!site.has("feed.xml"));
}

#[test]
fn a_pattern_handed_down_is_read_from_each_languages_top() {
    let settings = format!("{SETTINGS}pages.permalink: /{{year}}/{{slug}}/\n");
    let site = Site::new("handed", &settings);
    site.write("content/post-1.md", &post("2026-01-01", "Rust"));
    site.write("content/en/post-1.md", &post("2026-01-02", "Rust"));
    site.write(
        "content/en/about.md",
        "---\npermalink: /about/\nredirect_from: [/old-about/]\n---\n",
    );
    site.built();
    assert_eq!(site.read("2026/post-1/index.html"), "/2026/post-1/||");
    assert_eq!(site.read("en/2026/post-1/index.html"), "/en/2026/post-1/||");
    assert!(site.has("en/about/index.html"));
    // An old address is the one readers used, from the top of the site.
    assert!(site.has("old-about/index.html"));
}

#[test]
fn a_pattern_opening_with_its_languages_folder_is_read_once_and_warned_of() {
    let site = Site::new("doubled", SETTINGS);
    site.write("content/post-1.md", &post("2026-01-01", "Rust"));
    site.write(
        "content/en/index.md",
        "---\npages.permalink: /en/{slug}/\n---\n",
    );
    site.write("content/en/post-1.md", &post("2026-01-02", "Rust"));
    site.write("content/en/post-2.md", &post("2026-01-03", "Rust"));
    site.built();
    assert!(site.has("en/post-1/index.html"));
    assert!(!site.has("en/en/post-1/index.html"));
    let (said, code) = site.run("check");
    assert_eq!(code, 0, "{said}");
    assert_eq!(
        said.matches("warning  Language folder in pattern").count(),
        1,
        "{said}"
    );
    assert!(
        said.contains(
            "warning  Language folder in pattern: `/en/{slug}/` is read from `/en/` already\n  \
             content/en/index.md, line 2\n"
        ),
        "{said}"
    );
    assert!(said.contains("  Write it as `/{slug}/`.\n"), "{said}");
}

#[test]
fn neighbours_and_later_pages_stay_within_a_language() {
    let settings = format!("{SETTINGS}posts_per_page: 1\n");
    let site = Site::new("paging", &settings);
    site.write("content/post-1.md", &post("2026-01-01", "Rust"));
    site.write("content/post-2.md", &post("2026-01-02", "Rust"));
    site.write("content/en/post-1.md", &post("2026-01-03", "Rust"));
    site.write("content/en/post-2.md", &post("2026-01-04", "Rust"));
    site.write("templates/home.html", FOLDER);
    site.built();
    assert_eq!(site.read("post-1/index.html"), "/post-1/|/post-2/|");
    assert_eq!(
        site.read("en/post-2/index.html"),
        "/en/post-2/||/en/post-1/"
    );
    assert_eq!(site.read("index.html"), "/post-2/;|/page/2/");
    assert_eq!(site.read("page/2/index.html"), "/post-1/;|");
    assert_eq!(site.read("en/index.html"), "/en/post-2/;|/en/page/2/");
    assert_eq!(site.read("en/page/2/index.html"), "/en/post-1/;|");
    assert_eq!(
        site.read("en/tags/rust/index.html"),
        "/en/post-2/;|/en/tags/rust/page/2/"
    );
}
