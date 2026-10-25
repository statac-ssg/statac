//! `page.translations`: the same page in the site's other languages, as
//! templates write alternates and a language switch from it.

use super::*;

/// Alternates for every language a page is in, itself among them, and the
/// main language's for readers in any other.
const ALTERNATES: &str = "{% if page.translations %}\
{% set main = site.languages | first %}\
<link rel=\"alternate\" hreflang=\"{{ page.language }}\" href=\"{{ page.url | absolute }}\">\
{% for other in page.translations %}\
<link rel=\"alternate\" hreflang=\"{{ other.language }}\" href=\"{{ other.url | absolute }}\">\
{% endfor %}\
{% for one in [page] + page.translations if one.language == main %}\
<link rel=\"alternate\" hreflang=\"x-default\" href=\"{{ one.url | absolute }}\">\
{% endfor %}\
{% endif %}";

const SETTINGS: (&str, &str) = (
    "statac.yaml",
    "language: [ar, en, fr]\nurl: https://example.org\ntimezone: UTC\npages.feed: false\n",
);

#[test]
fn a_page_writes_an_alternate_for_each_of_its_translations_and_itself() {
    let built = site(&[
        SETTINGS,
        ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
        ("content/en/post-1.md", "---\ntitle: Post 1\n---\n"),
        ("content/fr/post-1.md", "---\ntitle: Post 1\n---\n"),
        ("content/en/post-2.md", "---\ntitle: Post 2\n---\n"),
        ("templates/page.html", ALTERNATES),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    let link = |language: &str, address: &str| {
        format!(
            "<link rel=\"alternate\" hreflang=\"{language}\" href=\"https://example.org{address}\">"
        )
    };
    assert_eq!(
        built.html("en/post-1.md"),
        [
            link("en", "/en/post-1/"),
            link("ar", "/post-1/"),
            link("fr", "/fr/post-1/"),
            link("x-default", "/post-1/"),
        ]
        .concat()
    );
    assert_eq!(
        built.html("post-1.md"),
        [
            link("ar", "/post-1/"),
            link("en", "/en/post-1/"),
            link("fr", "/fr/post-1/"),
            link("x-default", "/post-1/"),
        ]
        .concat()
    );
    assert_eq!(built.html("en/post-2.md"), "");
}

#[test]
fn a_page_depends_on_which_pages_are_its_translations_and_what_it_reads_of_them() {
    let built = site(&[
        SETTINGS,
        ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
        ("content/en/post-1.md", "---\ntitle: Post 1\n---\n"),
        (
            "templates/page.html",
            "{% for other in page.translations %}{{ other.title }}{% endfor %}",
        ),
    ]);
    assert_eq!(built.html("post-1.md"), "Post 1");
    let (arabic, english) = (built.id("post-1.md"), built.id("en/post-1.md"));
    let reads = built.render("post-1.md").reads;
    assert!(reads.keys().contains(&Key::Translations(arabic)));
    let title = Key::Field(english, built.renderer.name_id("title"));
    assert!(reads.keys().contains(&title));
}

/// A list's page, saying its language and its translations'.
const MADE: &str =
    "{{ page.language }}:{% for other in page.translations %}{{ other.language }} {% endfor %}";

#[test]
fn the_pages_statac_makes_have_their_translations_too() {
    let settings = (
        "statac.yaml",
        "language: [ar, en, fr]\nurl: https://example.org\ntimezone: UTC\npages.feed: false\n\
         tags.pages: true\n",
    );
    let built = site(&[
        settings,
        (
            "content/post-1.md",
            "---\ndate: 2026-01-01\ntags: [Rust]\n---\n",
        ),
        (
            "content/en/post-1.md",
            "---\ndate: 2026-01-01\ntags: [Rust]\n---\n",
        ),
        ("content/fr/post-1.md", "---\ndate: 2026-01-01\n---\n"),
        ("templates/page.html", ""),
        ("templates/home.html", MADE),
        ("templates/folder.html", MADE),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    let made = |language: usize, word: &str| {
        let site = built.renderer.site();
        site.lists
            .iter()
            .find(|(_, list)| list.language.index() == language && list.key.word() == word)
            .and_then(|(_, list)| list.page)
            .expect("a page Statac made")
    };
    let html = |page: PageId| built.renderer.render(page).html.expect("built");
    // The made home of each language matches the others'.
    assert_eq!(html(made(1, "folder")), "en:ar fr ");
    assert_eq!(html(made(2, "folder")), "fr:ar en ");
    // A tag's page matches the same tag's page in another language.
    assert_eq!(html(made(0, "tag")), "ar:en ");
    assert_eq!(html(made(1, "tag")), "en:ar ");
}

#[test]
fn a_lists_later_page_has_the_same_page_of_its_translations_where_there_is_one() {
    let mut files = vec![
        (
            "statac.yaml",
            "language: [ar, en]\ntimezone: UTC\npages.feed: false\nposts_per_page: 1\n",
        ),
        ("content/blog/index.md", "---\ntitle: Blog\n---\n"),
        ("content/en/blog/index.md", "---\ntitle: Blog\n---\n"),
        ("templates/page.html", ""),
        (
            "templates/folder.html",
            "{% for other in page.translations %}{{ other.url }}{% endfor %}",
        ),
    ];
    files.push(("content/blog/post-1.md", "---\ndate: 2026-01-01\n---\n"));
    files.push(("content/blog/post-2.md", "---\ndate: 2026-01-02\n---\n"));
    files.push(("content/blog/post-3.md", "---\ndate: 2026-01-03\n---\n"));
    files.push(("content/en/blog/post-1.md", "---\ndate: 2026-01-01\n---\n"));
    files.push(("content/en/blog/post-2.md", "---\ndate: 2026-01-02\n---\n"));
    let built = site(&files);
    let blog = built.id("blog/index.md");
    let later = |number: usize| {
        let rendered = built.renderer.render_later(blog, number);
        rendered.html.expect("built")
    };
    assert_eq!(built.html("blog/index.md"), "/en/blog/");
    assert_eq!(later(2), "/en/blog/page/2/");
    assert_eq!(later(3), "");
}

#[test]
fn a_language_switch_goes_to_the_translation_or_else_to_that_languages_home() {
    let switch = "{% for code in site.languages if code != page.language %}\
{% set same = page.translations | selectattr(\"language\", \"eq\", code) | list %}\
{% set there = same[0] if same else site.languages[code] %}\
<a href=\"{{ there.url }}\" hreflang=\"{{ code }}\">{{ code }}</a>\
{% endfor %}";
    let built = site(&[
        SETTINGS,
        ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
        ("content/en/post-1.md", "---\ntitle: Post 1\n---\n"),
        ("content/fr/index.md", "---\ntitle: Accueil\n---\n"),
        ("templates/page.html", switch),
        ("templates/home.html", ""),
        ("templates/folder.html", ""),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(
        built.html("post-1.md"),
        "<a href=\"/en/post-1/\" hreflang=\"en\">en</a><a href=\"/fr/\" hreflang=\"fr\">fr</a>"
    );
}
