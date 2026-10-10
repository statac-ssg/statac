//! How a problem met building a page names that page, drawn as `check`
//! draws it: a page of the author's by its file, a page made from an item
//! by the item, and the pages Statac makes by what they are.

use std::path::Path;

use super::*;
use crate::report::Report;

/// Every kind of page a site has: the author's own, a list's page the
/// author wrote, pages made from a table, and the pages Statac makes for a
/// tag, a series, a folder, the site itself and a list's later pages.
fn every_kind() -> Built {
    site(&[
        (
            "statac.yaml",
            "timezone: UTC\npages.feed: false\nposts_per_page: 1\n\
             tags.permalink: /topics/{slug}/\nlanguage: en\n",
        ),
        ("content/blog/index.md", "---\ntitle: Blog\n---\n"),
        (
            "content/blog/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-10-01\ntags: [Rust]\nseries: Intro\nsummary: One\n---\n",
        ),
        (
            "content/blog/post-2.md",
            "---\ntitle: Post 2\ndate: 2026-10-02\ntags: [Rust]\nseries: Intro\n---\n",
        ),
        (
            "content/notes/note-1.md",
            "---\ntitle: Note 1\ndate: 2026-10-03\n---\n",
        ),
        (
            "content/people.md",
            "---\ntitle: People\neach: data.people\npermalink: /people/{key}/\n\
             layout: person\nindex_page: false\n---\n",
        ),
        ("data/people.yaml", "person-1:\n  title: Person 1\n"),
        ("templates/home.html", "{{ page.summary }}"),
        ("templates/folder.html", "{{ page.summary }}"),
        ("templates/page.html", "{{ page.summary }}"),
        ("templates/person.html", "{{ page.summary }}"),
    ])
}

impl Built {
    /// The one problem building the page at `address`, page `number` of
    /// its list's, drawn in full.
    fn drawn(&self, address: &str, number: usize) -> String {
        let page = self.at(address);
        let rendered = match number {
            1 => self.renderer.render(page),
            _ => self.renderer.render_later(page, number),
        };
        assert_eq!(rendered.problems.len(), 1, "{:?}", rendered.problems);
        Report::new(self.renderer.sources(), Path::new(""), false).problem(&rendered.problems[0])
    }
}

/// The error bar for `page.summary` missing on `page`, met building a
/// page named by the `building` lines.
fn no_summary(template: &str, page: &str, building: &[&str]) -> String {
    let mut lines = vec![
        format!("error  No value: `page.summary` isn't set on {page}"),
        format!("  templates/{template}.html, line 1"),
        "   1 │ {{ page.summary }}".to_owned(),
        "     │    ^^^^^^^^^^^^".to_owned(),
    ];
    lines.extend(building.iter().map(|line| format!("  {line}")));
    lines.push("  Write `page.summary | default(\"\")` to show nothing where it isn't set.".into());
    lines.push("  More: https://statac.dev/e/no-value/".into());
    lines.join("\n") + "\n"
}

#[test]
fn a_problem_names_the_page_it_was_met_building() {
    let built = every_kind();
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    for (address, number, template, page, building) in [
        (
            "/blog/post-2/",
            1,
            "page",
            "`content/blog/post-2.md`",
            &["While building: content/blog/post-2.md"][..],
        ),
        (
            "/people/person-1/",
            1,
            "person",
            "`data/people.yaml: person-1`",
            &[
                "While building: data/people.yaml, line 1",
                " 1 │ person-1:",
                "   │ ^^^^^^^^",
            ],
        ),
        (
            "/topics/rust/",
            1,
            "folder",
            "the page for the tag `Rust`",
            &[
                "While building the page for the tag `Rust`: statac.yaml, line 4",
                " 4 │ tags.permalink: /topics/{slug}/",
                "   │                 ^^^^^^^^^^^^^^^",
            ],
        ),
        (
            "/series/intro/",
            1,
            "folder",
            "the page for the series `Intro`",
            &["While building the page for the series `Intro`"],
        ),
        (
            "/notes/",
            1,
            "folder",
            "the page for the folder `content/notes/`",
            &["While building the page for the folder `content/notes/`"],
        ),
        (
            "/",
            1,
            "home",
            "the site's home page",
            &["While building the site's home page"],
        ),
        (
            "/blog/",
            2,
            "folder",
            "`content/blog/index.md`",
            &["While building page 2 of `/blog/`: content/blog/index.md"],
        ),
        (
            "/topics/rust/",
            2,
            "folder",
            "the page for the tag `Rust`",
            &[
                "While building page 2 of `/topics/rust/`: statac.yaml, line 4",
                " 4 │ tags.permalink: /topics/{slug}/",
                "   │                 ^^^^^^^^^^^^^^^",
            ],
        ),
        (
            "/",
            2,
            "home",
            "the site's home page",
            &["While building page 2 of `/`"],
        ),
    ] {
        assert_eq!(
            built.drawn(address, number),
            no_summary(template, page, building),
            "{address} page {number}"
        );
    }
}

#[test]
fn a_page_statac_makes_with_no_template_is_named_as_it_is() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\n",
        ),
        ("content/index.md", "---\ntitle: Home\n---\n"),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-10-01\ntags: [Rust]\n---\n",
        ),
        ("templates/home.html", "{{ page.title }}"),
        ("templates/page.html", "{{ page.title }}"),
    ]);
    assert_eq!(
        built.drawn("/tags/rust/", 1),
        "error  Missing template: there is no `templates/folder.html`\n  \
         While building the page for the tag `Rust`\n  \
         Add the template, or choose another with `layout:`.\n  \
         More: https://statac.dev/e/missing-template/\n"
    );
}

#[test]
fn a_problem_met_building_a_feed_names_the_feed() {
    let built = site(&[
        (
            "statac.yaml",
            "title: Site\nurl: https://example.org\nfeed_key: key\ntimezone: UTC\n\
             pages.feed: feed.xml\nlanguage: en\n",
        ),
        (
            "content/blog/index.md",
            "---\ntitle: Blog\nfeed: true\n---\n",
        ),
        (
            "content/blog/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-10-01\ntags: [Rust]\nsummary: One\n---\n",
        ),
        (
            "content/blog/post-2.md",
            "---\ntitle: Post 2\ndate: 2026-10-02\ntags: [Rust]\n---\n",
        ),
        ("templates/home.html", "{{ page.title }}"),
        ("templates/folder.html", "{{ page.title }}"),
        ("templates/page.html", "{{ page.title }}"),
        (
            "templates/feed.xml",
            "{% for post in posts %}{{ post.summary }}{% endfor %}",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    let placed = [" 5 │ pages.feed: feed.xml", "   │             ^^^^^^^^"];
    for (address, building, at) in [
        (
            "/tags/rust/feed.xml",
            "While building the feed of the tag `Rust` at `/tags/rust/feed.xml`: \
             statac.yaml, line 5",
            &placed[..],
        ),
        (
            "/feed.xml",
            "While building the feed of the site's own list at `/feed.xml`: statac.yaml, line 5",
            &placed,
        ),
        (
            "/blog/feed.xml",
            "While building the feed of the folder `content/blog/` at `/blog/feed.xml`",
            &[],
        ),
    ] {
        let feed = built
            .renderer
            .feeds()
            .iter()
            .find(|feed| feed.address.as_str() == address)
            .unwrap_or_else(|| panic!("no feed at {address}"));
        let rendered = built.renderer.render_feed(feed);
        assert_eq!(rendered.problems.len(), 1, "{:?}", rendered.problems);
        let drawn = Report::new(built.renderer.sources(), Path::new(""), false)
            .problem(&rendered.problems[0]);
        let mut lines = vec![
            "error  No value: `post.summary` isn't set on `content/blog/post-2.md`".to_owned(),
            "  templates/feed.xml, line 1".into(),
            "   1 │ {% for post in posts %}{{ post.summary }}{% endfor %}".into(),
            "     │                           ^^^^^^^^^^^^".into(),
            format!("  {building}"),
        ];
        lines.extend(at.iter().map(|line| format!("  {line}")));
        lines.push(
            "  Write `post.summary | default(\"\")` to show nothing where it isn't set.".into(),
        );
        lines.push("  More: https://statac.dev/e/no-value/".into());
        assert_eq!(drawn, lines.join("\n") + "\n", "{address}");
    }
}
