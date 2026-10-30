use crate::build::incremental::Checking;
use crate::build::{Mode, Options};
use crate::templates::Renderer;
use crate::testing::TempSite;

const PAGE: &str = "{{ page.content }}\n<ul>{% for other in page.backlinks %}\
                    <li>{{ headline(other) }}</li>{% endfor %}</ul>\n";

fn write(site: &TempSite, rel: &str, text: &str) {
    let path = site.root.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn options(site: &TempSite) -> Options {
    Options {
        mode: Mode::Check,
        site: site.root.clone(),
        out: None,
        today: None,
        drafts: false,
        url: None,
    }
}

/// The paths of the pages linking to the page at `path`, in order.
fn backlinks(renderer: &Renderer, path: &str) -> Vec<String> {
    let site = renderer.site();
    let (page, _) = (site.pages.iter())
        .find(|(_, page)| page.path == path)
        .unwrap_or_else(|| panic!("no page {path}"));
    (renderer.backlinks(page).iter())
        .map(|&other| site.pages.get(other).path.clone())
        .collect()
}

fn post(date: &str, writing: &str) -> String {
    match date {
        "" => format!("---\ntitle: A post\n---\n{writing}\n"),
        date => format!("---\ntitle: A post\ndate: {date}\n---\n{writing}\n"),
    }
}

#[test]
fn links_a_reader_follows_from_the_writing_count_newest_first() {
    let site = TempSite::site(&[
        ("statac.yaml", "url: https://example.org/\n"),
        (
            "templates/page.html",
            "<a href=\"/target/\">menu</a>{{ page.content }}",
        ),
        ("snippets/button.html", "<a href=\"{{ link }}\">Go</a>\n"),
        (
            "content/target.md",
            "---\ntitle: Target\nredirect_from: [/old/]\n---\n[Itself](/target/)\n",
        ),
        ("content/markdown.md", &post("2026-01-02", "[1](/target/)")),
        (
            "content/html.md",
            &post("2026-01-04", "<p><a href=\"/target/\">2</a></p>"),
        ),
        (
            "content/helper.md",
            &post("2026-01-03", "[3](@url(/target/))"),
        ),
        ("content/moved.md", &post("2026-01-01", "[4](/old/)")),
        (
            "content/whole.md",
            &post("", "[5](https://example.org/target)"),
        ),
        (
            "content/twice.md",
            &post("", "[6](/target/) and [7](../target/)"),
        ),
        (
            "content/area.html",
            "---\ntitle: Area\n---\n<map><area href=\"/target/\"></map>\n",
        ),
        ("content/picture.md", &post("", "![A picture](/target/)")),
        ("content/handed.md", &post("", "@button(link=\"/target/\")")),
        (
            "content/elsewhere.md",
            &post("", "[8](https://example.com/target/)"),
        ),
    ]);
    let mut checking = Checking::new();
    checking.check(&options(&site));
    let renderer = checking.renderer().unwrap();
    assert_eq!(
        backlinks(renderer, "target.md"),
        [
            "html.md",
            "helper.md",
            "markdown.md",
            "moved.md",
            "area.html",
            "twice.md",
            "whole.md"
        ]
    );
    assert!(backlinks(renderer, "markdown.md").is_empty());
}

#[test]
fn headline_names_untitled_posts_linking_to_the_page() {
    let site = TempSite::site(&[
        ("statac.yaml", "language: en\ntimezone: UTC\n"),
        ("templates/page.html", PAGE),
        ("content/target.md", "---\ntitle: Target\n---\nHere.\n"),
        (
            "content/dated.md",
            "---\ndate: 2026-01-05\nuntitled: true\n---\nA note linking [there](/target/).\n",
        ),
        (
            "content/undated.md",
            "---\nuntitled: true\n---\nAnother note linking [there](/target/).\n",
        ),
    ]);
    let built = crate::build::run(&Options {
        mode: Mode::Build,
        out: Some(site.root.join("_site")),
        ..options(&site)
    });
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    let html = std::fs::read_to_string(site.root.join("_site/target/index.html")).unwrap();
    assert!(
        html.contains("<ul><li>January 5, 2026</li><li>Another note linking there.</li></ul>"),
        "{html}"
    );
}

#[test]
fn a_check_after_an_edit_reads_only_the_page_that_changed() {
    let site = TempSite::site(&[
        ("templates/page.html", PAGE),
        ("content/target.md", "---\ntitle: Target\n---\nHere.\n"),
        ("content/one.md", &post("2026-01-01", "[1](/target/)")),
        ("content/two.md", &post("2026-01-02", "Nothing yet.")),
        ("content/three.md", &post("2026-01-03", "Nothing.")),
    ]);
    let mut checking = Checking::new();
    checking.check(&options(&site));
    let renderer = checking.renderer().unwrap();
    assert_eq!(renderer.links_read(), Some(4));
    assert_eq!(backlinks(renderer, "target.md"), ["one.md"]);
    write(
        &site,
        "content/two.md",
        &post("2026-01-02", "[2](/target/)"),
    );
    checking.check(&options(&site));
    let renderer = checking.renderer().unwrap();
    assert_eq!(renderer.links_read(), Some(1));
    assert_eq!(backlinks(renderer, "target.md"), ["two.md", "one.md"]);
    // A snippet that now wraps the writing below its call reads the
    // writing calling it otherwise.
    write(
        &site,
        "content/three.md",
        &post("2026-01-03", "@aside()\n[3](/target/)\n@end"),
    );
    checking.check(&options(&site));
    assert_eq!(checking.renderer().unwrap().links_read(), Some(1));
    write(&site, "snippets/aside.html", "<aside>{{ body }}</aside>\n");
    checking.check(&options(&site));
    assert_eq!(checking.renderer().unwrap().links_read(), Some(1));
}

#[test]
fn a_site_that_never_asks_reads_no_links() {
    let site = TempSite::site(&[
        ("templates/page.html", "{{ page.content }}\n"),
        ("content/target.md", "---\ntitle: Target\n---\nHere.\n"),
        ("content/one.md", &post("2026-01-01", "[1](/target/)")),
    ]);
    let mut checking = Checking::new();
    checking.check(&options(&site));
    assert_eq!(checking.renderer().unwrap().links_read(), None);
    write(
        &site,
        "content/one.md",
        &post("2026-01-01", "[2](/target/) again"),
    );
    checking.check(&options(&site));
    assert_eq!(checking.renderer().unwrap().links_read(), None);
}
