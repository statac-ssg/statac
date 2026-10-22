use super::super::{Read, Run, read};
use crate::problem::{Kind, Relation, Translated};
use crate::site::{Id, PageId};
use crate::testing::TempSite;

fn site(files: &[(&str, &str)]) -> Read {
    site_with(files, false)
}

fn site_with(files: &[(&str, &str)], drafts: bool) -> Read {
    let run = Run {
        today: Some(crate::date::Date::day_only(2026, 10, 4)),
        drafts,
        url: None,
        renamed: Vec::new(),
    };
    read(&TempSite::site(files).root, &run)
}

const TWO: (&str, &str) = ("statac.yaml", "language: [ar, en]\ntimezone: UTC\n");

fn id(read: &Read, path: &str) -> PageId {
    read.site
        .pages
        .iter()
        .find(|(_, page)| page.path == path || page.address.as_str() == path)
        .map(|(id, _)| id)
        .unwrap_or_else(|| panic!("no page {path}"))
}

/// The translations of the page at `path`, or standing at the address
/// `path`, each by its path or, where Statac made it, its address.
fn translations(read: &Read, path: &str) -> Vec<String> {
    let id = id(read, path);
    let found = read.site.translations.get(id.index());
    found
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .map(|&other| {
            let page = read.site.pages.get(other);
            match page.path.is_empty() {
                true => page.address.as_str().to_owned(),
                false => page.path.clone(),
            }
        })
        .collect()
}

/// Each problem's name, the file it is in and the text it is at.
fn problems(read: &Read) -> Vec<(&'static str, String, String)> {
    read.problems
        .iter()
        .map(|p| {
            let (file, at) = match p.place {
                Some(place) => (
                    read.sources.path(place.source).display().to_string(),
                    read.sources.spelling(place).to_owned(),
                ),
                None => (String::new(), String::new()),
            };
            (p.kind.name(), file, at)
        })
        .collect()
}

#[test]
fn pages_at_one_path_under_their_languages_tops_are_translations() {
    let read = site(&[
        TWO,
        ("content/post-1.md", "# Post 1\n"),
        ("content/en/post-1.md", "# Post 1\n"),
        ("content/blog/post-2.md", "# Post 2\n"),
        ("content/en/blog/post-2.md", "# Post 2\n"),
        ("content/post-3.md", "# Post 3\n"),
    ]);
    assert_eq!(problems(&read), []);
    assert_eq!(translations(&read, "post-1.md"), ["en/post-1.md"]);
    assert_eq!(translations(&read, "en/post-1.md"), ["post-1.md"]);
    assert_eq!(translations(&read, "en/blog/post-2.md"), ["blog/post-2.md"]);
    assert_eq!(translations(&read, "post-3.md"), Vec::<String>::new());
}

#[test]
fn where_the_main_language_has_a_folder_a_page_outside_every_folder_matches_by_key_alone() {
    let read = site(&[
        TWO,
        ("content/index.md", "# Home\n"),
        ("content/post-1.md", "# Post 1\n"),
        ("content/ar/index.md", "# Home\n"),
        ("content/ar/post-1.md", "# Post 1\n"),
        ("content/en/index.md", "# Home\n"),
        ("content/en/post-1.md", "# Post 1\n"),
        (
            "content/en/post-2.md",
            "---\ntranslation_key: welcome\n---\n",
        ),
        ("content/post-2.md", "---\ntranslation_key: welcome\n---\n"),
    ]);
    assert_eq!(problems(&read), []);
    assert_eq!(translations(&read, "ar/post-1.md"), ["en/post-1.md"]);
    assert_eq!(translations(&read, "ar/index.md"), ["en/index.md"]);
    assert_eq!(translations(&read, "post-1.md"), Vec::<String>::new());
    assert_eq!(translations(&read, "index.md"), Vec::<String>::new());
    assert_eq!(translations(&read, "post-2.md"), ["en/post-2.md"]);
}

#[test]
fn a_path_is_matched_without_its_files_extension() {
    let read = site(&[
        TWO,
        ("content/about.md", "# About\n"),
        (
            "content/en/about.html",
            "---\ntitle: About\n---\n<p>About</p>\n",
        ),
        ("content/blog/index.md", "# Blog\n"),
        ("content/en/blog/index.html", "---\ntitle: Blog\n---\n"),
        ("content/blog/post-1.md", "# Post 1\n"),
        ("content/en/blog/post-1.md", "# Post 1\n"),
    ]);
    assert_eq!(problems(&read), []);
    assert_eq!(translations(&read, "about.md"), ["en/about.html"]);
    assert_eq!(translations(&read, "blog/index.md"), ["en/blog/index.html"]);
}

#[test]
fn a_folder_that_is_one_page_matches_a_page_named_for_it() {
    let read = site(&[
        TWO,
        ("content/post-1/index.md", "# Post 1\n"),
        ("content/post-1/photo-1.jpg", ""),
        ("content/en/post-1.md", "# Post 1\n"),
        ("content/blog/index.md", "# Blog\n"),
        ("content/blog/post-2.md", "# Post 2\n"),
        ("content/en/blog.md", "# Blog\n"),
        ("content/index.md", "# Home\n"),
        ("content/en/index.md", "# Home\n"),
    ]);
    assert_eq!(problems(&read), []);
    assert_eq!(translations(&read, "post-1/index.md"), ["en/post-1.md"]);
    assert_eq!(translations(&read, "en/blog.md"), ["blog/index.md"]);
    assert_eq!(translations(&read, "index.md"), ["en/index.md"]);
}

#[test]
fn pages_with_different_names_are_linked_by_one_key() {
    let read = site(&[
        TWO,
        ("content/about.md", "---\ntranslation_key: about\n---\n"),
        (
            "content/en/about-us.md",
            "---\ntranslation_key: about\n---\n",
        ),
        ("content/en/about.md", "---\ntranslation_key: other\n---\n"),
        ("content/other.md", "---\ntranslation_key: other\n---\n"),
    ]);
    assert_eq!(problems(&read), []);
    assert_eq!(translations(&read, "about.md"), ["en/about-us.md"]);
    assert_eq!(translations(&read, "en/about.md"), ["other.md"]);
}

#[test]
fn a_key_on_a_folders_own_page_pairs_the_folders_and_the_pages_beneath() {
    let read = site(&[
        ("statac.yaml", "language: [ar, en]\n"),
        (
            "content/ar/books/index.md",
            "---\ntranslation_key: books\n---\n",
        ),
        ("content/ar/books/book-1/post-1.md", "# Post 1\n"),
        (
            "content/en/book-list/index.md",
            "---\ntranslation_key: books\n---\n",
        ),
        ("content/en/book-list/book-1/post-1.md", "# Post 1\n"),
        ("content/en/book-list/post-2.md", "# Post 2\n"),
        ("content/en/books/post-1.md", "# Post 1\n"),
    ]);
    assert_eq!(problems(&read), []);
    assert_eq!(
        translations(&read, "ar/books/index.md"),
        ["en/book-list/index.md"]
    );
    assert_eq!(
        translations(&read, "ar/books/book-1/post-1.md"),
        ["en/book-list/book-1/post-1.md"]
    );
    assert_eq!(
        translations(&read, "en/books/post-1.md"),
        Vec::<String>::new()
    );
}

#[test]
fn translations_come_main_language_first_then_as_declared() {
    let read = site(&[
        ("statac.yaml", "language: [ar, en, fr]\n"),
        ("content/fr/post-1.md", "# Post 1\n"),
        ("content/en/post-1.md", "# Post 1\n"),
        ("content/post-1.md", "# Post 1\n"),
        ("content/fr/post-2.md", "# Post 2\n"),
        ("content/en/post-2.md", "# Post 2\n"),
    ]);
    assert_eq!(problems(&read), []);
    assert_eq!(
        translations(&read, "fr/post-1.md"),
        ["post-1.md", "en/post-1.md"]
    );
    assert_eq!(
        translations(&read, "en/post-1.md"),
        ["post-1.md", "fr/post-1.md"]
    );
    assert_eq!(
        translations(&read, "post-1.md"),
        ["en/post-1.md", "fr/post-1.md"]
    );
    assert_eq!(translations(&read, "fr/post-2.md"), ["en/post-2.md"]);
}

#[test]
fn pages_statac_makes_match_by_their_lists() {
    let read = site(&[
        TWO,
        (
            "content/post-1.md",
            "---\ndate: 2026-01-01\ntags: [Rust]\n---\n",
        ),
        (
            "content/en/post-1.md",
            "---\ndate: 2026-01-01\ntags: [Rust]\n---\n",
        ),
    ]);
    assert_eq!(problems(&read), []);
    // Each language's top with no `index.md` has a page Statac made.
    assert_eq!(translations(&read, "/"), ["/en/"]);
    assert_eq!(translations(&read, "/en/"), ["/"]);
    let tags: Vec<PageId> = read
        .site
        .lists
        .iter()
        .filter(|(_, list)| matches!(list.key, crate::site::ListKey::Tag(_)))
        .filter_map(|(_, list)| list.page)
        .collect();
    assert_eq!(tags.len(), 2);
    assert_eq!(read.site.translations[tags[0].index()], [tags[1]]);
    assert_eq!(read.site.translations[tags[1].index()], [tags[0]]);
}

#[test]
fn an_authors_page_at_a_lists_address_matches_the_page_statac_makes_for_it() {
    let post = |path, line| (path, format!("---\ndate: 2026-01-01\n{line}\n---\n"));
    let cases = [
        (
            [
                post("content/post-1.md", "tags: [Rust]"),
                post("content/en/post-1.md", "tags: [Rust]"),
                ("content/tags/rust.md", "# Rust\n".into()),
            ],
            ("tags/rust.md", "/en/tags/rust/"),
        ),
        (
            [
                post("content/post-1.md", "tags: [Rust]"),
                post("content/en/post-1.md", "tags: [Rust]"),
                ("content/en/tags/rust.md", "# Rust\n".into()),
            ],
            ("en/tags/rust.md", "/tags/rust/"),
        ),
        (
            [
                post("content/post-1.md", "series: One"),
                post("content/en/post-1.md", "series: One"),
                ("content/series/one.md", "# One\n".into()),
            ],
            ("series/one.md", "/en/series/one/"),
        ),
        // A page in another language at its path is its translation.
        (
            [
                post("content/post-1.md", "tags: [Rust]"),
                ("content/tags/rust.md", "# Rust\n".into()),
                ("content/en/tags/rust.md", "# Rust\n".into()),
            ],
            ("tags/rust.md", "en/tags/rust.md"),
        ),
    ];
    for (files, (page, translation)) in cases {
        let mut files: Vec<(&str, &str)> = files.iter().map(|(p, t)| (*p, t.as_str())).collect();
        files.push(TWO);
        let read = site(&files);
        assert_eq!(problems(&read), [], "{page}");
        assert_eq!(translations(&read, page), [translation], "{page}");
        assert_eq!(translations(&read, translation), [page], "{page}");
    }
}

#[test]
fn a_draft_translation_is_no_pages_translation_and_shares_its_key() {
    let files = [
        TWO,
        ("content/post-1.md", "# Post 1\n"),
        ("content/en/post-1.md", "---\ndraft: true\n---\n"),
        ("content/about.md", "---\ntranslation_key: about\n---\n"),
        (
            "content/en/about-us.md",
            "---\ntranslation_key: about\ndraft: true\n---\n",
        ),
        ("content/post-2.md", "# Post 2\n"),
        ("content/en/post-2.md", "---\ndate: 2026-12-01\n---\n"),
    ];
    let read = site(&files);
    let names: Vec<_> = problems(&read).into_iter().map(|(name, ..)| name).collect();
    assert_eq!(
        names,
        ["draft-left-out", "draft-left-out", "waiting-for-its-day"]
    );
    assert_eq!(translations(&read, "post-1.md"), Vec::<String>::new());
    assert_eq!(translations(&read, "about.md"), Vec::<String>::new());
    assert_eq!(translations(&read, "post-2.md"), Vec::<String>::new());
    let read = site_with(&files, true);
    assert_eq!(translations(&read, "post-1.md"), ["en/post-1.md"]);
    assert_eq!(translations(&read, "about.md"), ["en/about-us.md"]);
    assert_eq!(translations(&read, "post-2.md"), ["en/post-2.md"]);
}

#[test]
fn two_pages_in_one_language_with_one_key_stop_at_the_second() {
    let read = site(&[
        TWO,
        ("content/about.md", "---\ntranslation_key: about\n---\n"),
        (
            "content/en/about-1.md",
            "---\ntranslation_key: about\n---\n",
        ),
        (
            "content/en/about-2.md",
            "---\ntranslation_key: about\n---\n",
        ),
    ]);
    assert_eq!(
        problems(&read),
        [(
            "same-translation",
            "content/en/about-2.md".to_owned(),
            "about".to_owned()
        )]
    );
    let Kind::SameTranslation { tag, by, first } = &read.problems[0].kind else {
        unreachable!()
    };
    assert_eq!((tag.as_str(), by), ("en", &Translated::Key("about".into())));
    let first = first.expect("placed");
    assert_eq!(
        read.sources.path(first.source).to_str(),
        Some("content/en/about-1.md")
    );
    assert_eq!(read.problems[0].kind.related()[0].0, Relation::FirstWritten);
    // One language alone may not give one key to two pages either.
    let read = site(&[
        ("statac.yaml", "language: en\n"),
        ("content/about-1.md", "---\ntranslation_key: about\n---\n"),
        ("content/about-2.md", "---\ntranslation_key: about\n---\n"),
    ]);
    assert_eq!(
        problems(&read),
        [(
            "same-translation",
            "content/about-2.md".to_owned(),
            "about".to_owned()
        )]
    );
}

#[test]
fn two_pages_in_one_language_standing_where_a_page_matches_stop_at_the_second() {
    let read = site(&[
        TWO,
        ("content/about.md", "# About\n"),
        ("content/en/about.md", "---\npermalink: /about-1/\n---\n"),
        ("content/en/about.html", "---\ntitle: About\n---\n"),
        ("content/en/post-1.md", "---\npermalink: /post-2/\n---\n"),
        ("content/en/post-1.html", "---\ntitle: Post 1\n---\n"),
        // At one address, the two are told as that instead.
        ("content/contact.md", "# Contact\n"),
        ("content/en/contact.md", "# Contact\n"),
        ("content/en/contact.html", "---\ntitle: Contact\n---\n"),
    ]);
    assert_eq!(
        problems(&read),
        [(
            "same-translation",
            "content/en/about.md".to_owned(),
            "---".to_owned()
        )]
    );
    let Kind::SameTranslation { by, .. } = &read.problems[0].kind else {
        unreachable!()
    };
    assert_eq!(by, &Translated::Place("content/about.md".into()));
}

#[test]
fn a_key_no_page_in_another_language_has_is_warned_of_offering_the_nearest() {
    let read = site(&[
        TWO,
        ("content/about.md", "---\ntranslation_key: about\n---\n"),
        (
            "content/en/about-us.md",
            "---\ntranslation_key: abuot\n---\n",
        ),
        ("content/contact.md", "---\ntranslation_key: contact\n---\n"),
    ]);
    assert_eq!(
        problems(&read),
        [
            (
                "unmatched-translation-key",
                "content/about.md".to_owned(),
                "about".to_owned()
            ),
            (
                "unmatched-translation-key",
                "content/contact.md".to_owned(),
                "contact".to_owned()
            ),
            (
                "unmatched-translation-key",
                "content/en/about-us.md".to_owned(),
                "abuot".to_owned()
            ),
        ]
    );
    let nearest: Vec<_> = read
        .problems
        .iter()
        .map(|p| match &p.kind {
            Kind::UnmatchedTranslationKey { nearest, .. } => nearest.clone(),
            _ => unreachable!(),
        })
        .collect();
    assert_eq!(
        nearest,
        [Some("abuot".to_owned()), None, Some("about".to_owned())]
    );
}

#[test]
fn a_page_with_no_key_where_a_page_with_one_stands_is_warned_of() {
    let read = site(&[
        TWO,
        ("content/about.md", "---\ntranslation_key: about\n---\n"),
        ("content/en/about.md", "# About\n"),
        ("content/fr.md", "---\ntranslation_key: post\n---\n"),
        ("content/en/post-1.md", "---\ntranslation_key: post\n---\n"),
        ("content/en/fr.md", "# Not a translation\n"),
    ]);
    assert_eq!(
        problems(&read),
        [
            (
                "unmatched-translation-key",
                "content/about.md".to_owned(),
                "about".to_owned()
            ),
            (
                "not-linked-by-place",
                "content/en/about.md".to_owned(),
                "# About".to_owned()
            ),
        ]
    );
    let Kind::NotLinkedByPlace { keyed, key } = &read.problems[1].kind else {
        unreachable!()
    };
    assert_eq!(
        (keyed.as_str(), key.as_str()),
        ("content/about.md", "about")
    );
    assert_eq!(translations(&read, "en/about.md"), Vec::<String>::new());
}

#[test]
fn another_tools_word_for_the_key_is_refused_where_the_site_has_several_languages() {
    let read = site(&[
        TWO,
        ("content/about.md", "---\ntranslationKey: about\n---\n"),
        (
            "content/blog/index.md",
            "---\npages.translation_key: blog\n---\n",
        ),
        ("content/blog/post-1.md", "# Post 1\n"),
        ("content/en/post-1.md", "# Post 1\n"),
    ]);
    assert_eq!(
        problems(&read),
        [
            (
                "other-spelling",
                "content/about.md".to_owned(),
                "translationKey".to_owned()
            ),
            (
                "wrong-place",
                "content/blog/index.md".to_owned(),
                "pages.translation_key".to_owned()
            ),
        ]
    );
    let Kind::OtherSpelling { ours, .. } = &read.problems[0].kind else {
        unreachable!()
    };
    assert_eq!(ours, "translation_key");
    // A site in one language may have a field of its own by that name.
    let read = site(&[
        ("statac.yaml", "language: ar\n"),
        ("content/about.md", "---\ntranslationKey: about\n---\n"),
    ]);
    assert_eq!(problems(&read), []);
    let page = read.site.pages.get(id(&read, "about.md"));
    assert!(page.words.field("translationKey").is_some());
}
