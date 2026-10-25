//! A site in several languages: `site` answering with the lists of the
//! language of the page being built, and each page recording only them.

use super::*;
use crate::site::LanguageId;

const SETTINGS: (&str, &str) = (
    "statac.yaml",
    "language: [ar, en]\ntimezone: UTC\npages.feed: false\ntags.pages: true\n",
);

const POST: &str = "---\ndate: 2026-10-01\ntags: [Rust]\n---\n";

#[test]
fn site_posts_holds_the_posts_of_the_pages_own_language() {
    let built = site(&[
        SETTINGS,
        ("content/post-1.md", POST),
        ("content/en/post-1.md", POST),
        ("content/en/post-2.md", POST),
        (
            "templates/page.html",
            "{% for post in site.posts %}{{ post.url }};{% endfor %}|\
             {% for tag in site.tags %}{{ tag.url }}{% endfor %}",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(built.html("post-1.md"), "/post-1/;|/tags/rust/");
    assert_eq!(
        built.html("en/post-1.md"),
        "/en/post-1/;/en/post-2/;|/en/tags/rust/"
    );
    let site = built.renderer.site();
    let (ar, en) = (LanguageId::from_index(0), LanguageId::from_index(1));
    let (main, english) = (site.own_list(ar).unwrap(), site.own_list(en).unwrap());
    let keys = built.render("en/post-1.md").reads.keys().to_vec();
    assert!(keys.contains(&Key::Members(english)));
    assert!(!keys.contains(&Key::Members(main)));
    assert!(keys.contains(&Key::ListsIn(en)));
    assert!(!keys.contains(&Key::ListsIn(ar)) && !keys.contains(&Key::Lists));
}

#[test]
fn site_pages_is_every_languages_pages() {
    let built = site(&[
        SETTINGS,
        NO_HOME,
        ("content/en/index.md", "---\nindex_page: false\n---\n"),
        ("content/post-1.md", POST),
        ("content/en/post-1.md", POST),
        (
            "templates/page.html",
            "{% for page in site.pages %}{{ page.url }}:{{ page.language }};{% endfor %}",
        ),
        ("templates/folder.html", ""),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(
        built.html("post-1.md"),
        "/en/post-1/:en;/en/tags/rust/:en;/post-1/:ar;/tags/rust/:ar;"
    );
}

#[test]
fn site_languages_gives_each_tag_and_its_home_page() {
    let built = site(&[
        SETTINGS,
        ("content/index.md", "---\ntitle: Home 1\n---\n"),
        ("content/post-1.md", POST),
        ("content/en/post-1.md", POST),
        (
            "templates/page.html",
            "{% for code in site.languages %}{{ code }}={{ site.languages[code].url }};{% endfor %}\
             {{ site.languages.ar.title }}|{{ page.folder.page.url }}",
        ),
        ("templates/home.html", ""),
        ("templates/folder.html", ""),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    // English has no `index.md`, so its home is the page Statac makes.
    assert_eq!(built.html("post-1.md"), "ar=/;en=/en/;Home 1|/");
    assert_eq!(built.html("en/post-1.md"), "ar=/;en=/en/;Home 1|/en/");
}

#[test]
fn a_languages_top_with_no_index_is_made_a_home_page() {
    let built = site(&[
        (
            "statac.yaml",
            "language: [ar, en]\ntimezone: UTC\npages.feed: false\nen.title: Site 2\n",
        ),
        ("content/post-1.md", POST),
        ("content/en/post-1.md", POST),
        (
            "templates/home.html",
            "{{ page.list.word }}|{{ page.list.title }}",
        ),
        ("templates/folder.html", ""),
        ("templates/page.html", ""),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    let site = built.renderer.site();
    let english = site.own_list(LanguageId::from_index(1)).unwrap();
    let home = site.lists.get(english).page.expect("a page is made");
    assert_eq!(site.pages.get(home).address.as_str(), "/en/");
    let rendered = built.renderer.render(home);
    assert!(rendered.problems.is_empty(), "{:?}", rendered.problems);
    assert_eq!(rendered.html.as_deref(), Some("site|Site 2"));
}

#[test]
fn filters_and_related_stay_within_the_pages_language() {
    let built = site(&[
        SETTINGS,
        (
            "content/post-1.md",
            "---\ndate: 2026-10-01\ntags: [Go]\n---\n",
        ),
        ("content/blog/post-2.md", POST),
        ("content/en/post-1.md", POST),
        ("content/en/blog/post-2.md", POST),
        (
            "templates/page.html",
            "{% for post in site.posts | tagged(\"Rust\") %}{{ post.url }};{% endfor %}|\
             {% for post in site.posts | in(\"blog\") %}{{ post.url }};{% endfor %}|\
             {% for post in related(page) %}{{ post.url }};{% endfor %}",
        ),
        ("templates/folder.html", ""),
        ("templates/home.html", ""),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(
        built.html("en/post-1.md"),
        "/en/post-1/;/en/blog/post-2/;|/en/blog/post-2/;|/en/blog/post-2/;"
    );
    assert_eq!(built.html("post-1.md"), "/blog/post-2/;|/blog/post-2/;|");
}

#[test]
fn a_page_making_a_page_per_post_makes_them_for_its_own_languages_posts() {
    let built = site(&[
        SETTINGS,
        ("content/post-1.md", POST),
        ("content/en/post-1.md", POST),
        (
            "content/en/cards.md",
            "---\neach: site.posts\npermalink: /cards/{slug}/\nlayout: card\n---\n",
        ),
        ("templates/card.html", "{{ page.post.url }}"),
        ("templates/page.html", ""),
        ("templates/folder.html", ""),
        ("templates/home.html", ""),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    let site = built.renderer.site();
    let made: Vec<&str> = site
        .pages
        .iter()
        .filter(|(_, page)| page.made.is_some())
        .map(|(_, page)| page.address.as_str())
        .collect();
    assert_eq!(made, ["/en/cards/post-1/"]);
}
