use super::*;

use crate::build::incremental::Checking;
use crate::build::{Mode, Options};
use crate::testing::TempSite;

/// Where the cursor, written `|`, stands in the writing `text`, said
/// shortly.
fn spot(text: &str) -> String {
    let offset = text.find('|').expect("a cursor");
    let text = text.replacen('|', "", 1);
    let body = Span::new(0, text.len());
    match find(&text, body, offset) {
        None => "none".to_owned(),
        Some(Spot::Name {
            typed,
            opened,
            alone,
            ..
        }) => format!(
            "name `{}`{}{}",
            &text[typed],
            if opened { " opened" } else { "" },
            if alone { " alone" } else { "" }
        ),
        Some(Spot::Value {
            name,
            typed,
            quote,
            named,
            place,
            given,
            ..
        }) => format!(
            "value of {name} `{}`{} {}{}",
            &text[typed],
            quote.map(|q| format!(" {q}")).unwrap_or_default(),
            named.map_or(format!("at {place}"), |named| format!("named {named}")),
            if given.is_empty() {
                String::new()
            } else {
                format!(" {given:?}")
            }
        ),
        Some(Spot::Address {
            typed, definition, ..
        }) => format!(
            "address `{}`{}",
            &text[typed],
            if definition { " defined" } else { "" }
        ),
    }
}

#[test]
fn the_cursor_is_found_in_a_call_its_values_or_an_address() {
    let cases = [
        ("@fi|", "name `fi` alone"),
        ("@|", "name `` alone"),
        ("Read @bo| here.", "name `bo`"),
        ("@fig|ure(\"a.jpg\")", "name `figure` opened"),
        ("> @no|", "name `no` alone"),
        ("@1|", "none"),
        ("@figure(\"pho|", "value of figure `pho` \" at 0"),
        ("@figure('a.jpg', '|", "value of figure `` ' at 1"),
        ("@figure(\"a.jpg\", ca|", "value of figure `ca` at 1"),
        (
            "@figure(\"a.jpg\", caption=\"A\", wi|",
            "value of figure `wi` at 1 [\"caption\"]",
        ),
        (
            "@figure(caption=\"A|",
            "value of figure `A` \" named caption",
        ),
        ("@figure(caption: |", "value of figure `` named caption"),
        ("@figure(\"a\\\"b|", "value of figure `a\\\"b` \" at 0"),
        ("@url(/ab|", "value of url `/ab` at 0"),
        ("@figure(\"a.jpg\") and |", "none"),
        ("@figure(\"a\nb|", "none"),
        ("@figure(\n  \"a.jpg\",\n  |", "value of figure `` at 1"),
        ("@figure(\n\n|", "none"),
        ("[About](/ab|", "address `/ab`"),
        ("![A photo](|", "address ``"),
        ("[About](/about/#te|", "address `/about/#te`"),
        ("[About](@url(/ab|", "value of url `/ab` at 0"),
        ("[about]: /ab|", "address `/ab` defined"),
        ("[about]: /about/ \"A|", "none"),
        ("[About](/about/) and |", "none"),
        ("Some words|", "none"),
    ];
    for (text, expected) in cases {
        assert_eq!(spot(text), expected, "{text:?}");
    }
}

const BOOK: &str = "{# takes title, author #}\n<p><cite>{{ title }}</cite> by {{ author }}</p>\n";

fn site() -> TempSite {
    TempSite::site(&[
        (
            "statac.yaml",
            "title: Site 1\nurl: https://example.org/blog/\n",
        ),
        ("templates/page.html", "{{ page.content }}\n"),
        (
            "snippets/figure.html",
            "{# takes photo, alt #}\n{% set shown = page.photo(photo) %}\
             {{ shown.alt }}{{ alt if alt is defined }}{{ caption | default('') }}\n",
        ),
        (
            "snippets/gallery.html",
            "{# takes photos... #}\n{% for shown in page.photos(photos if photos is defined else none) %}{{ shown.alt }}{% endfor %}\n",
        ),
        (
            "snippets/clip.html",
            "{# takes clip #}\n{{ page.video(clip).src }}\n",
        ),
        ("snippets/book.html", BOOK),
        ("snippets/note.html", "<aside>{{ body }}</aside>\n"),
        ("snippets/card.html", "{# takes post #}\n{{ post.title }}\n"),
        (
            "content/about.md",
            "---\ntitle: About\n---\n## The team\n\n## Getting here\n",
        ),
        (
            "content/notes/note-1.md",
            "---\ntitle: Note 1\n---\nA note.\n",
        ),
        (
            "content/posts/post-1/index.md",
            "---\ntitle: Post 1\n---\n## First\n",
        ),
        ("content/posts/post-1/photo-1.jpg", "not read"),
        ("content/posts/post-1/photo-2.png", "not read"),
        ("content/posts/post-1/clip-1.mp4", "not read"),
        ("content/posts/post-1/notes.txt", "A file.\n"),
    ])
}

type Offered = Vec<(String, String, String)>;

/// What is offered at the cursor, written `|`, in `text` as the writing
/// of `content/posts/post-1/index.md`, once the site is read: each as its
/// label, what is said beside it and what is written.
fn offered(site: &TempSite, text: &str) -> Option<Offered> {
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
    let text = format!("---\ntitle: Post 1\n---\n{text}");
    let offset = text.find('|').expect("a cursor");
    let text = text.replacen('|', "", 1);
    let file = site.root.join("content/posts/post-1/index.md");
    let renderer = checking.renderer();
    let completion = super::super::complete(&site.root, &file, &text, offset, renderer, true)?;
    let items = completion.items.into_iter().map(|item| {
        let detail = item.detail.unwrap_or_default();
        (item.label, detail, item.insert)
    });
    Some(items.collect())
}

fn labels(offered: &Offered) -> Vec<&str> {
    offered.iter().map(|(label, ..)| label.as_str()).collect()
}

fn item<'a>(offered: &'a Offered, label: &str) -> &'a (String, String, String) {
    (offered.iter())
        .find(|(said, ..)| said == label)
        .unwrap_or_else(|| panic!("`{label}` not among {:?}", labels(offered)))
}

#[test]
fn a_call_offers_the_sites_snippets_written_to_be_filled_in() {
    let site = site();
    let calls = offered(&site, "@|").unwrap();
    assert_eq!(
        item(&calls, "figure(photo, alt)"),
        &(
            "figure(photo, alt)".into(),
            "snippets/figure.html".into(),
            "figure(\"$1\")".into()
        )
    );
    assert_eq!(item(&calls, "gallery(photos...)").2, "gallery(\"$1\")");
    assert_eq!(
        item(&calls, "note()").2,
        "note($1)\n$0\n@end",
        "wraps the writing"
    );
    assert_eq!(
        item(&calls, "url(address)"),
        &(
            "url(address)".into(),
            "address in the site".into(),
            "url($1)".into()
        )
    );
    assert!(
        !labels(&calls).contains(&"card(post)"),
        "it reads the fields of what it takes"
    );
    let within = offered(&site, "Read @no|").unwrap();
    assert_eq!(item(&within, "note()").2, "note($1)", "inside a sentence");
    let opened = offered(&site, "@fi|gure(\"photo-1.jpg\")").unwrap();
    assert_eq!(item(&opened, "figure(photo, alt)").2, "figure");
}

#[test]
fn a_value_offers_the_files_beside_the_page_its_snippet_finds() {
    let site = site();
    let photos = offered(&site, "@figure(\"|").unwrap();
    assert_eq!(labels(&photos), ["photo-1.jpg", "photo-2.png"]);
    let gallery = offered(&site, "@gallery(\"photo-1.jpg\", \"|").unwrap();
    assert_eq!(labels(&gallery), ["photo-1.jpg", "photo-2.png"]);
    let clips = offered(&site, "@clip(|").unwrap();
    assert_eq!(item(&clips, "clip-1.mp4").2, "\"clip-1.mp4\"");
    let names = offered(&site, "@figure(\"photo-1.jpg\", |").unwrap();
    assert_eq!(labels(&names), ["alt", "caption"]);
    assert_eq!(
        item(&names, "caption"),
        &("caption".into(), "value".into(), "caption=".into())
    );
    let book = offered(&site, "@book(\"A\", |").unwrap();
    assert_eq!(
        labels(&book),
        ["author"],
        "the name given by place is left out"
    );
}

#[test]
fn an_address_offers_the_sites_pages_and_after_a_hash_their_headings() {
    let site = site();
    let links = offered(&site, "[About](|").unwrap();
    assert_eq!(
        item(&links, "/blog/about/"),
        &("/blog/about/".into(), "About".into(), "/blog/about/".into())
    );
    assert!(labels(&links).contains(&"/blog/notes/note-1/"));
    let in_site = offered(&site, "@url(|").unwrap();
    assert_eq!(
        item(&in_site, "/about/").1,
        "About",
        "from the top of the site"
    );
    let quoted = offered(&site, "@url(\"|").unwrap();
    assert_eq!(item(&quoted, "/about/").2, "/about/");
    let headings = offered(&site, "[Getting here](/blog/about/#|").unwrap();
    assert_eq!(
        headings,
        [
            ("the-team".into(), "The team".into(), "the-team".into()),
            (
                "getting-here".into(),
                "Getting here".into(),
                "getting-here".into()
            ),
        ]
    );
    let own = offered(&site, "[First](#|").unwrap();
    assert_eq!(labels(&own), ["first"], "the page's own headings");
    let defined = offered(&site, "[about]: /blog/a|").unwrap();
    assert!(labels(&defined).contains(&"/blog/about/"));
    let picture = offered(&site, "![A photo](|").unwrap();
    assert_eq!(item(&picture, "photo-1.jpg").2, "photo-1.jpg");
}

#[test]
fn nothing_is_offered_where_the_writing_reads_no_call_or_link() {
    let site = site();
    let cases = [
        "`@fi|`",
        "Mail me at name@ex|",
        "\\@fi|",
        "```\n@fi|\n```\n",
        "    @fi|",
        "<pre>\n@fi|\n</pre>\n",
        "<!-- @fi| -->",
        "`@figure(\"|`",
        "`[About](|`",
        "```\n[About](|\n```\n",
        "Some words|",
    ];
    for text in cases {
        assert_eq!(offered(&site, text), None, "{text:?}");
    }
}

#[test]
fn a_large_sites_addresses_are_offered_a_part_at_a_time() {
    let notes: Vec<(String, String)> = (1..=MOST_ADDRESSES + 10)
        .map(|n| {
            let path = format!("content/notes/note-{n}.md");
            (path, format!("---\ntitle: Note {n}\n---\nA note.\n"))
        })
        .collect();
    let mut files: Vec<(&str, &str)> = vec![("templates/page.html", "{{ page.content }}\n")];
    files.extend(
        notes
            .iter()
            .map(|(path, text)| (path.as_str(), text.as_str())),
    );
    let site = TempSite::site(&files);
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
    let file = site.root.join("content/notes/note-1.md");
    let ask = |typed: &str| {
        let text = format!("---\ntitle: Note 1\n---\n[A note]({typed}");
        super::super::complete(
            &site.root,
            &file,
            &text,
            text.len(),
            checking.renderer(),
            true,
        )
        .unwrap()
    };
    let all = ask("");
    assert!(all.incomplete);
    assert_eq!(all.items.len(), MOST_ADDRESSES);
    let some = ask("/notes/note-2");
    assert!(some.incomplete, "the editor asks again as more is typed");
    let labels: Vec<&str> = some.items.iter().map(|item| item.label.as_str()).collect();
    assert_eq!(
        labels.len(),
        22,
        "note-2, note-20 to note-29 and note-200 to note-210"
    );
    assert!(
        labels
            .iter()
            .all(|label| label.starts_with("/notes/note-2"))
    );
    assert_eq!(some.items[0].detail.as_deref(), Some("Note 2"));
}
