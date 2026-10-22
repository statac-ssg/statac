use std::path::Path;

use super::{Found, Heading, Sort, found, headings};
use crate::build::incremental::Checking;
use crate::build::{Mode, Options};
use crate::testing::TempSite;

/// Each heading of `text` as the page `content/page.md`, read alone,
/// as its words and the text of its section, indented by how deep it is.
fn outline(text: &str) -> Option<Vec<String>> {
    fn told(headings: &[Heading], text: &str, depth: usize, into: &mut Vec<String>) {
        for heading in headings {
            assert!(heading.section.start == heading.at.start);
            assert!(heading.section.end >= heading.at.end);
            into.push(format!(
                "{}{} {:?}",
                "  ".repeat(depth),
                heading.words,
                &text[heading.section.clone()]
            ));
            told(&heading.within, text, depth + 1, into);
        }
    }
    let site = Path::new("/site");
    let found = headings(site, &site.join("content/page.md"), text, None)?;
    let mut lines = Vec::new();
    told(&found, text, 0, &mut lines);
    Some(lines)
}

#[test]
fn a_page_lists_its_headings_each_beneath_the_one_above_it() {
    let text = "---\ntitle: Page 1\n---\nA start.\n\n# One\nA.\n\n## Two\nB.\n\n\
                ### Three\n\n## Four\n\n# Five\nC.\n";
    assert_eq!(
        outline(text).unwrap(),
        [
            "One \"# One\\nA.\\n\\n## Two\\nB.\\n\\n### Three\\n\\n## Four\\n\\n\"",
            "  Two \"## Two\\nB.\\n\\n### Three\\n\\n\"",
            "    Three \"### Three\\n\\n\"",
            "  Four \"## Four\\n\\n\"",
            "Five \"# Five\\nC.\\n\"",
        ]
    );
}

#[test]
fn only_the_writings_own_headings_are_listed() {
    let text = "---\ntitle: Page 1\n# not: a heading\n---\n```\n# Not a heading\n```\n\n\
                Two\n===\n\n##\n\n### *Three* `3`\n";
    assert_eq!(
        outline(text).unwrap(),
        [
            "Two \"Two\\n===\\n\\n##\\n\\n### *Three* `3`\\n\"",
            "  Three 3 \"### *Three* `3`\\n\""
        ]
    );
    // A heading deeper than the one before goes beneath it however many
    // levels it skips.
    let text = "# One\n### Three\n## Two\n";
    assert_eq!(
        outline(text).unwrap(),
        [
            "One \"# One\\n### Three\\n## Two\\n\"",
            "  Three \"### Three\\n\"",
            "  Two \"## Two\\n\""
        ]
    );
}

#[test]
fn a_file_that_is_no_pages_lists_nothing_and_unclosed_front_matter_no_headings() {
    let site = Path::new("/site");
    for file in [
        "statac.yaml",
        "templates/page.html",
        "content/photo.jpg.yaml",
    ] {
        assert_eq!(
            headings(site, &site.join(file), "# One\n", None),
            None,
            "{file}"
        );
    }
    assert_eq!(
        outline("---\ntitle: Page 1\n# One\n").unwrap(),
        Vec::<String>::new()
    );
}

#[test]
fn the_site_is_searched_by_title_and_name_whatever_the_case() {
    let site = TempSite::site(&[
        ("statac.yaml", "title: Site 1\n"),
        ("templates/page.html", "{{ page.content }}\n"),
        ("templates/post.html", "{{ page.content }}\n"),
        ("snippets/figure.html", "{# takes photo #}\n{{ photo }}\n"),
        ("content/about.md", "---\ntitle: About\n---\nA page.\n"),
        (
            "content/posts/post-1.md",
            "---\ntitle: Post one\ndate: 2026-01-02\n---\nA post.\n",
        ),
    ]);
    let mut checking = Checking::new();
    checking.check(&Options {
        mode: Mode::Check,
        site: site.root.clone(),
        out: None,
        today: None,
        drafts: false,
        url: None,
    });
    let renderer = checking.renderer().unwrap();
    let names = |query: &str| {
        let mut found: Vec<(String, Sort, String)> = (found(renderer, query).into_iter())
            .map(|Found { name, sort, path }| (name, sort, path))
            .collect();
        found.sort_by(|a, b| (&a.0, &a.2).cmp(&(&b.0, &b.2)));
        found
    };
    let page = |name: &str, path: &str| (name.to_owned(), Sort::Page, path.to_owned());
    assert_eq!(
        names("p ONE"),
        [page("Post one", "content/posts/post-1.md")]
    );
    assert_eq!(
        names("po"),
        [
            page("Post one", "content/posts/post-1.md"),
            ("post".into(), Sort::Template, "templates/post.html".into()),
        ]
    );
    assert_eq!(
        names("fig"),
        [(
            "figure".into(),
            Sort::Snippet,
            "snippets/figure.html".into()
        )]
    );
    let all = names("");
    assert!(all.contains(&page("About", "content/about.md")), "{all:?}");
    // Pages Statac makes, such as the folder's, have no file to go to.
    assert_eq!(
        all.iter()
            .filter(|(_, sort, _)| *sort == Sort::Page)
            .count(),
        2
    );
    assert_eq!(names("xyz"), []);
}
