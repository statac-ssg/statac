use super::*;

use crate::build::incremental::Checking;
use crate::build::{Mode, Options};
use crate::testing::TempSite;

/// Where the cursor, written `|`, stands in `text`, said shortly.
fn spot(text: &str) -> String {
    let offset = text.find('|').expect("a cursor");
    let text = text.replacen('|', "", 1);
    let mut sources = Sources::default();
    let id = sources.add("a.yaml", text.clone());
    let place = Place {
        source: id,
        span: Span::new(0, text.len()),
    };
    match spot::find(&sources, place, offset) {
        None => "none".to_owned(),
        Some(Spot::Name {
            path,
            typed,
            written,
            colon,
        }) => format!(
            "name {path:?} `{}` {written:?}{}",
            &text[typed],
            if colon { " colon" } else { "" }
        ),
        Some(Spot::Value {
            path,
            typed,
            flow,
            quote,
            written,
            spaced,
        }) => format!(
            "value {path:?} `{}` {written:?}{}{}{}",
            &text[typed],
            if flow { " flow" } else { "" },
            quote.map(|q| format!(" {q}")).unwrap_or_default(),
            if spaced { " spaced" } else { "" }
        ),
    }
}

#[test]
fn the_cursor_is_found_in_a_name_or_a_value_and_under_its_fields() {
    let cases = [
        (
            "title: A\nta|\ndate: 2026-01-01\n",
            r#"name [] `ta` ["title", "date"]"#,
        ),
        ("ti|tle: A\n", "name [] `ti` [] colon"),
        ("title: A\n|", r#"name [] `` ["title"]"#),
        ("title: A\ntags: [wri|\n", "value [\"tags\"] `wri` [] flow"),
        ("tags: [a, 'Ta|\n", "value [\"tags\"] `Ta` [\"a\"] flow '"),
        ("tags: [a, |]\n", "value [\"tags\"] `` [\"a\"] flow"),
        ("tags: [a, |\n", "value [\"tags\"] `` [\"a\"] flow"),
        ("tags: [a,\n  b|", "value [\"tags\"] `b` [\"a\"] flow"),
        ("tags:|\n", "value [\"tags\"] `` [] spaced"),
        ("tags: |\n", "value [\"tags\"] `` []"),
        ("tags: \"wr|\n", "value [\"tags\"] `wr` [] \""),
        ("tags:\n  - wri|\n", "value [\"tags\"] `wri` []"),
        ("tags:\n  - a\n  - |\n", "value [\"tags\"] `` [\"a\"]"),
        ("tags:\n- a\n- |\n", "value [\"tags\"] `` [\"a\"]"),
        ("book:\n  au|\n", "name [\"book\"] `au` []"),
        (
            "book:\n  author: x\n  |\n",
            "name [\"book\"] `` [\"author\"]",
        ),
        (
            "book:\n  author: x\n  |\ntitle: A\n",
            "name [\"book\"] `` [\"author\"]",
        ),
        ("book:\n  author: |\n", "value [\"book\", \"author\"] `` []"),
        ("layout: po|\n", "value [\"layout\"] `po` []"),
        ("title: A long t|\n", "value [\"title\"] `A long t` []"),
        ("# a comment ta|\n", "none"),
        ("title: A # no|\n", "none"),
        ("title: A\nsome words|\n", "none"),
    ];
    for (text, expected) in cases {
        assert_eq!(spot(text), expected, "{text:?}");
    }
}

const SETTINGS: &str = "\
title: Site 1
language: en
fields.isbn: text
fields.status: one of draft, final
fields.book.author: text
fields.shelf: one of data.shelves
fields.shelved: one of folder.posts
fields.reviewed: true/false
types.review.rating: required number
";

fn site() -> TempSite {
    TempSite::site(&[
        ("statac.yaml", SETTINGS),
        (
            "data/shelves.yaml",
            "top:\n  name: Top\nlow:\n  name: Low\n",
        ),
        ("templates/page.html", "{{ page.content }}\n"),
        ("templates/post.html", "{{ page.content }}\n"),
        ("templates/folder.html", "{{ page.content }}\n"),
        ("templates/home.html", "{{ page.content }}\n"),
        ("content/posts/index.md", "---\ntitle: Posts\n---\n"),
        (
            "content/posts/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-01-01\ntags: [Writing, rust]\n\
             series: Series 1\n---\nA post.\n",
        ),
        (
            "content/posts/post-2.md",
            "---\ntitle: Post 2\ndate: 2026-01-02\ntags: [Writing]\n---\nA post.\n",
        ),
    ])
}

/// What is offered at the cursor, written `|`, in `text` as the file `rel`
/// of `site`, once the site is read: each as its label, what is said
/// beside it and what is written.
fn offered(site: &TempSite, rel: &str, text: &str) -> Option<Vec<(String, String, String)>> {
    let mut checking = Checking::new();
    let options = Options {
        mode: Mode::Check,
        site: site.root.clone(),
        out: None,
        today: None,
        drafts: false,
        url: None,
    };
    checking.check(&options);
    let offset = text.find('|').expect("a cursor");
    let text = text.replacen('|', "", 1);
    let file = site.root.join(rel);
    let completion = complete(&site.root, &file, &text, offset, checking.renderer(), true)?;
    let items = completion.items.into_iter().map(|item| {
        let detail = item.detail.unwrap_or_default();
        (item.label, detail, item.insert)
    });
    Some(items.collect())
}

fn labels(offered: &[(String, String, String)]) -> Vec<&str> {
    offered.iter().map(|(label, ..)| label.as_str()).collect()
}

fn find<'a>(offered: &'a [(String, String, String)], label: &str) -> &'a (String, String, String) {
    (offered.iter())
        .find(|(said, ..)| said == label)
        .unwrap_or_else(|| panic!("`{label}` not among {:?}", labels(offered)))
}

#[test]
fn settings_offer_statacs_names_and_what_each_takes() {
    let site = site();
    let names = offered(&site, "statac.yaml", "title: Site 1\nco|\n").unwrap();
    assert_eq!(
        find(&names, "code_colours"),
        &(
            "code_colours".into(),
            "setting".into(),
            "code_colours: ".into()
        )
    );
    assert_eq!(
        find(&names, "pages.layout"),
        &(
            "pages.layout".into(),
            "cascade".into(),
            "pages.layout: ".into()
        )
    );
    assert_eq!(find(&names, "pages.isbn").1, "text");
    assert!(!labels(&names).contains(&"title"), "written already");
    assert!(!labels(&names).contains(&"pages.title"), "each page's own");
    assert!(!labels(&names).contains(&"pages.index_page"));

    let words = offered(&site, "statac.yaml", "search_without_javascript: |\n").unwrap();
    assert_eq!(
        labels(&words),
        crate::search::elsewhere::SETTING.to_vec(),
        "every word, in order"
    );
    let truth = offered(&site, "statac.yaml", "typography:|\n").unwrap();
    assert_eq!(find(&truth, "false").2, " false");

    let quiet = offered(&site, "statac.yaml", "quiet: [|]\n").unwrap();
    assert_eq!(find(&quiet, "broken-link").1, "warning");
    assert!(!labels(&quiet).contains(&"unclear-date"), "an error");

    let kinds = offered(&site, "statac.yaml", "fields.pages_read: |\n").unwrap();
    assert!(labels(&kinds).contains(&"true/false"));
    assert!(labels(&kinds).contains(&"required"));
    let required = offered(&site, "statac.yaml", "types.review.isbn: required t|\n").unwrap();
    assert!(labels(&required).contains(&"required text"));
    let named = offered(&site, "statac.yaml", "fields.who: one of |\n").unwrap();
    assert_eq!(
        labels(&named),
        [
            "one of data.shelves",
            "one of data.shelves.top",
            "one of data.shelves.low",
            "one of folder.posts"
        ]
    );
}

#[test]
fn front_matter_offers_statacs_fields_then_the_sites() {
    let site = site();
    let names = offered(
        &site,
        "content/posts/post-3.md",
        "---\ntitle: A\nta|\n---\n",
    )
    .unwrap();
    assert_eq!(
        find(&names, "tags"),
        &("tags".into(), "field".into(), "tags: ".into())
    );
    assert_eq!(find(&names, "status").1, "one of draft, final");
    assert_eq!(find(&names, "book").1, "", "a field holding fields");
    let labels_of = labels(&names);
    assert!(!labels_of.contains(&"title"), "written already");
    assert!(!labels_of.contains(&"numbered"), "a folder's own");
    assert!(!labels_of.iter().any(|name| name.starts_with("pages.")));
    assert!(
        labels_of.iter().position(|n| *n == "tags") < labels_of.iter().position(|n| *n == "isbn")
    );

    let folder = offered(
        &site,
        "content/posts/index.md",
        "---\n|\ntitle: Posts\n---\n",
    )
    .unwrap();
    assert!(labels(&folder).contains(&"numbered"));
    assert_eq!(find(&folder, "pages.layout").1, "cascade");
    assert_eq!(find(&folder, "pages.status").1, "one of draft, final");

    let nested = offered(&site, "content/posts/post-3.md", "---\nbook:\n  |\n---\n").unwrap();
    assert_eq!(
        nested,
        [("author".into(), "text".into(), "author: ".into())]
    );

    let unclosed = offered(&site, "content/posts/post-3.md", "---\nti|").unwrap();
    assert!(labels(&unclosed).contains(&"title"));
    assert_eq!(
        offered(&site, "content/posts/post-3.md", "---\ntitle: A\n---\nta|"),
        None,
        "the writing"
    );
    assert_eq!(offered(&site, "templates/page.html", "ta|"), None);
}

#[test]
fn front_matter_offers_the_values_the_site_holds() {
    let site = site();
    let rel = "content/posts/post-3.md";
    let tags = offered(&site, rel, "---\ntags: [|\n---\n").unwrap();
    assert_eq!(
        tags,
        [
            ("Writing".into(), "tag, 2 pages".into(), "Writing".into()),
            ("rust".into(), "tag, 1 page".into(), "rust".into()),
        ]
    );
    let more = offered(&site, rel, "---\ntags: [Writing, r|\n---\n").unwrap();
    assert_eq!(labels(&more), ["rust"], "a tag written already");
    let series = offered(&site, rel, "---\nseries: |\n---\n").unwrap();
    assert_eq!(find(&series, "Series 1").1, "series, 1 page");
    let layouts = offered(&site, rel, "---\nlayout: p|\n---\n").unwrap();
    assert_eq!(find(&layouts, "post").1, "templates/post.html");
    let types = offered(&site, rel, "---\ntype: |\n---\n").unwrap();
    assert_eq!(labels(&types), ["review"]);
    let neighbours = offered(&site, rel, "---\nneighbours: |\n---\n").unwrap();
    assert_eq!(labels(&neighbours), ["site", "folder", "series", "false"]);
    let draft = offered(&site, rel, "---\ndraft: |\n---\n").unwrap();
    assert_eq!(labels(&draft), ["true", "false"]);

    let status = offered(&site, rel, "---\nstatus: |\n---\n").unwrap();
    assert_eq!(labels(&status), ["draft", "final"]);
    let shelf = offered(&site, rel, "---\nshelf: |\n---\n").unwrap();
    assert_eq!(
        shelf,
        [
            ("top".into(), "data/shelves.yaml".into(), "top".into()),
            ("low".into(), "data/shelves.yaml".into(), "low".into()),
        ]
    );
    let shelved = offered(&site, rel, "---\nshelved: |\n---\n").unwrap();
    assert_eq!(labels(&shelved), ["post-1", "post-2"]);
    let reviewed = offered(&site, rel, "---\nreviewed: |\n---\n").unwrap();
    assert_eq!(labels(&reviewed), ["true", "false"]);
    let handed = offered(
        &site,
        "content/posts/index.md",
        "---\npages.layout: |\n---\n",
    );
    assert!(labels(&handed.unwrap()).contains(&"post"));
}

#[test]
fn a_value_is_written_to_read_back_as_itself_where_it_goes() {
    let cases = [
        ("rust", true, false, None, false, "rust"),
        ("rust", true, false, None, true, " rust"),
        ("a, b", true, true, None, false, "\"a, b\""),
        ("a, b", true, false, None, false, "a, b"),
        ("yes", true, false, None, false, "\"yes\""),
        ("false", true, false, None, false, "\"false\""),
        ("false", false, false, None, false, "false"),
        ("it's", true, false, Some('\''), false, "it''s"),
        ("say \"a\"", true, false, Some('"'), false, "say \\\"a\\\""),
        ("a: b", true, false, None, false, "\"a: b\""),
    ];
    for (value, text, flow, quote, spaced, written) in cases {
        assert_eq!(
            value_written(value, text, flow, quote, spaced),
            written,
            "{value}"
        );
    }
}

#[test]
fn nothing_but_statacs_own_is_offered_before_the_site_is_read() {
    let site = site();
    let file = site.root.join("content/posts/post-3.md");
    let text = "---\ntags: [\n---\n";
    let completion = complete(&site.root, &file, text, 11, None, true).unwrap();
    assert!(completion.items.is_empty());
    let text = "---\nta\n---\n";
    let completion = complete(&site.root, &file, text, 6, None, true).unwrap();
    assert!(completion.items.iter().any(|item| item.label == "tags"));
    assert_eq!(completion.typed, 4..6);
}

#[test]
fn a_page_whose_front_matter_is_being_typed_is_known_by_its_folder() {
    // Each file's front matter as it stands while a field is typed, which
    // leaves the page out of the site.
    let typing = "---\ntitle: A\nta\n---\nA.\n";
    let site = TempSite::site(&[
        ("templates/page.html", "{{ page.content }}\n"),
        ("content/posts/index.md", typing),
        (
            "content/posts/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-01-01\n---\nA.\n",
        ),
        ("content/posts/a-folder/index.md", typing),
    ]);
    let text = "---\ntitle: A\nta|\n---\nA.\n";
    let folder = offered(&site, "content/posts/index.md", text).unwrap();
    assert!(
        labels(&folder).contains(&"numbered"),
        "the folder's own page"
    );
    let page = offered(&site, "content/posts/a-folder/index.md", text).unwrap();
    assert!(
        !labels(&page).contains(&"numbered"),
        "a page in a folder of its own"
    );
}
