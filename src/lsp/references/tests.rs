use super::{Written, found};
use crate::build::incremental::Checking;
use crate::build::{Mode, Options};
use crate::testing::TempSite;

const ABOUT: &str = "---\ntitle: About\nredirect_from: [/old-about/]\n---\n\
                     ## The team\n\nSee [the team](#the-team).\n";

fn site() -> TempSite {
    TempSite::site(&[
        (
            "statac.yaml",
            "title: Site 1\nurl: https://example.org/blog/\n",
        ),
        (
            "templates/page.html",
            "<a href=\"/blog/about/\">1</a> <a href=\"/about/\">2</a> <a href=\"about/\">3</a>\n\
             <a href=\"{{ url('/about/#the-team') }}\">4</a> {{ url(\"/notes/note-1/\") }}\n\
             {{ page.content }}\n",
        ),
        ("snippets/button.html", "<a href=\"{{ link }}\">Go</a>\n"),
        ("content/about.md", ABOUT),
        (
            "content/index.md",
            "---\ntitle: Home\n---\n@button(link=\"about\")\n",
        ),
        (
            "content/notes/note-1.md",
            "---\ntitle: Note 1\n---\n[About](/blog/about/) [Old](/blog/old-about/)\n\
             [Team](../../about/#the-team) [Helper](@url(/about/)) [Full](https://example.org/blog/about/)\n\
             [Away](https://example.com/blog/about/) [Note](/blog/notes/note-1/)\n\n\
             [By label]\n\n@button(link=\"/blog/about/\")\n\n[by label]: /blog/about/\n",
        ),
    ])
}

/// Every link to what the character after the cursor, written `|`, in
/// `text` as the text of the site's file at `path` names, once the site is
/// read: each by its file, and the text it is at where it is at a place.
fn references(site: &TempSite, path: &str, text: &str, declaration: bool) -> Option<Vec<String>> {
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
    let offset = text.find('|').expect("a cursor");
    let text = text.replacen('|', "", 1);
    let file = site.root.join(path);
    let to = crate::lsp::hover::hover(&site.root, &file, &text, offset, Some(renderer))
        .and_then(|hover| hover.to);
    let sources = renderer.sources();
    let found = found(renderer, path, offset, to, declaration)?;
    Some(
        (found.into_iter())
            .map(|written| match written {
                Written::File(path) => path,
                Written::At(place) => format!(
                    "{} at `{}`",
                    sources
                        .path(place.source)
                        .to_string_lossy()
                        .replace('\\', "/"),
                    sources.spelling(place)
                ),
            })
            .collect(),
    )
}

/// Every link to the About page, in the order of their files.
const TO_ABOUT: [&str; 10] = [
    "content/about.md at `#the-team`",
    "content/notes/note-1.md at `/blog/about/`",
    "content/notes/note-1.md at `/blog/old-about/`",
    "content/notes/note-1.md at `../../about/#the-team`",
    "content/notes/note-1.md at `@url(/about/)`",
    "content/notes/note-1.md at `https://example.org/blog/about/`",
    "content/notes/note-1.md at `\"/blog/about/\"`",
    "content/notes/note-1.md at `/blog/about/`",
    "templates/page.html at `/blog/about/`",
    "templates/page.html at `'/about/#the-team'`",
];

/// Every link to the About page's heading.
const TO_THE_TEAM: [&str; 3] = [
    "content/about.md at `#the-team`",
    "content/notes/note-1.md at `../../about/#the-team`",
    "templates/page.html at `'/about/#the-team'`",
];

#[test]
fn a_page_is_linked_from_the_writing_and_templates_at_any_of_its_addresses() {
    let site = site();
    let about = ABOUT.replacen("See", "S|ee", 1);
    assert_eq!(
        references(&site, "content/about.md", &about, false).unwrap(),
        TO_ABOUT
    );
    // From front matter too, where a word names nothing of its own.
    let about = ABOUT.replacen("title", "ti|tle", 1);
    assert_eq!(
        references(&site, "content/about.md", &about, false).unwrap(),
        TO_ABOUT
    );
    let mut declared = vec!["content/about.md"];
    declared.extend(TO_ABOUT);
    assert_eq!(
        references(&site, "content/about.md", &about, true).unwrap(),
        declared
    );
}

#[test]
fn a_heading_is_linked_by_its_id() {
    let site = site();
    let about = ABOUT.replacen("The team", "The t|eam", 1);
    assert_eq!(
        references(&site, "content/about.md", &about, false).unwrap(),
        TO_THE_TEAM
    );
    let mut declared = vec!["content/about.md at `## The team`"];
    declared.extend(TO_THE_TEAM);
    assert_eq!(
        references(&site, "content/about.md", &about, true).unwrap(),
        declared
    );
}

#[test]
fn a_link_finds_every_other_link_to_what_it_names() {
    let site = site();
    let note = std::fs::read_to_string(site.root.join("content/notes/note-1.md")).unwrap();
    let on = |from: &str, to: &str| note.replacen(from, to, 1);
    let cases = [
        (on("(/blog/about/)", "(/blog/ab|out/)"), &TO_ABOUT[..]),
        (
            on("(/blog/old-about/)", "(/blog/old-ab|out/)"),
            &TO_ABOUT[..],
        ),
        (on("#the-team", "#the-t|eam"), &TO_THE_TEAM[..]),
    ];
    for (text, expected) in cases {
        let found = references(&site, "content/notes/note-1.md", &text, false);
        assert_eq!(found.unwrap(), expected, "{text}");
    }
    let page = std::fs::read_to_string(site.root.join("templates/page.html")).unwrap();
    let text = page.replacen("the-team", "the-t|eam", 1);
    let found = references(&site, "templates/page.html", &text, false);
    assert_eq!(found.unwrap(), TO_THE_TEAM);
}

#[test]
fn what_is_no_page_or_heading_has_no_links_sought() {
    let site = site();
    let cases = [
        ("statac.yaml", "title: S|ite 1\n"),
        ("templates/page.html", "<a href=\"/blog/ab|out/\">1</a>\n"),
        (
            "content/index.md",
            "---\ntitle: Home\n---\n@butt|on(link=\"about\")\n",
        ),
    ];
    for (path, text) in cases {
        assert_eq!(references(&site, path, text, false), None, "{path}");
    }
    // A link that leads nowhere known names no page, though it is in one.
    let note = std::fs::read_to_string(site.root.join("content/notes/note-1.md")).unwrap();
    let text = note.replacen(
        "[Away](https://example.com/",
        "[Away](https://exa|mple.com/",
        1,
    );
    let found = references(&site, "content/notes/note-1.md", &text, false);
    assert_eq!(found, None);
}
