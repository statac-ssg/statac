use std::collections::BTreeMap;

use super::{relative, renamed, rewritten};
use crate::build::incremental::Checking;
use crate::build::{Mode, Options};
use crate::testing::TempSite;
use crate::unsaved::Unsaved;

const ABOUT: &str = "---\ntitle: About\n---\n## The team\n\n\
                     See [the team](#the-team) and [note 1](../notes/note-1/).\n";

const NOTE: &str = "---\ntitle: Note 1\n---\n\
                    [About](/blog/about/) [Team](../../about/#the-team) [Helper](@url(/about/))\n\
                    [Full](https://example.org/blog/about/?a=1) [Away](https://example.com/blog/about/)\n\n\
                    @button(link=\"/blog/about/\")\n\n[By label]\n\n[by label]: /blog/about/\n";

fn files(about: &str) -> Vec<(&'static str, String)> {
    vec![
        (
            "statac.yaml",
            "title: Site 1\nurl: https://example.org/blog/\n".to_owned(),
        ),
        (
            "templates/page.html",
            "<a href=\"/blog/about/\">1</a> <a href=\"{{ url('/about/#the-team') }}\">2</a>\n\
             {{ page.content }}\n"
                .to_owned(),
        ),
        (
            "snippets/button.html",
            "<a href=\"{{ link }}\">Go</a>\n".to_owned(),
        ),
        ("content/about.md", about.to_owned()),
        ("content/notes/note-1.md", NOTE.to_owned()),
    ]
}

/// Each file of the site at `files` that renaming it by `moves` changes,
/// by its path before the renaming, with its text once changed.
fn changed(files: &[(&str, String)], moves: &[(&str, &str)]) -> BTreeMap<String, String> {
    changed_after(1, files, moves)
}

/// [`changed`], once the site has been checked `rounds` times.
fn changed_after(
    rounds: usize,
    files: &[(&str, String)],
    moves: &[(&str, &str)],
) -> BTreeMap<String, String> {
    let files: Vec<(&str, &str)> = files.iter().map(|(path, text)| (*path, &**text)).collect();
    let site = TempSite::site(&files);
    let mut checking = Checking::new();
    for _ in 0..rounds {
        checking.check(&Options {
            mode: Mode::Check,
            site: site.root.clone(),
            out: None,
            today: None,
            drafts: false,
            url: None,
        });
    }
    let renderer = checking.renderer().unwrap();
    let moves: Vec<(String, String)> = (moves.iter())
        .map(|(from, to)| ((*from).to_owned(), (*to).to_owned()))
        .collect();
    let edits = renamed(renderer, &site.root, Unsaved::default(), &moves, None);
    let sources = renderer.sources();
    let mut changed: BTreeMap<String, String> = BTreeMap::new();
    for (place, text) in edits.iter().rev() {
        let path = sources
            .path(place.source)
            .to_string_lossy()
            .replace('\\', "/");
        let file = (changed.entry(path)).or_insert_with(|| sources.text(place.source).to_owned());
        file.replace_range(place.span.range(), text);
    }
    changed
}

#[test]
fn renaming_a_page_writes_every_link_to_it_anew_and_sends_readers_on_from_its_old_address() {
    let found = changed(
        &files(ABOUT),
        &[("content/about.md", "content/about-us.md")],
    );
    assert_eq!(
        found,
        BTreeMap::from([
            (
                "content/about.md".to_owned(),
                ABOUT.replace("title: About\n", "title: About\nredirect_from: [/about/]\n")
            ),
            (
                "content/notes/note-1.md".to_owned(),
                NOTE.replace("/blog/about/", "/blog/about-us/")
                    .replace("../../about/", "../../about-us/")
                    .replace("@url(/about/)", "@url(/about-us/)")
                    .replace("example.com/blog/about-us/", "example.com/blog/about/")
            ),
            (
                "templates/page.html".to_owned(),
                "<a href=\"/blog/about-us/\">1</a> \
                 <a href=\"{{ url('/about-us/#the-team') }}\">2</a>\n{{ page.content }}\n"
                    .to_owned()
            ),
        ])
    );
}

#[test]
fn a_page_moved_to_another_folder_has_the_links_it_writes_from_where_it_was_written_again() {
    let found = changed(
        &files(ABOUT),
        &[("content/about.md", "content/notes/about.md")],
    );
    assert_eq!(
        found["content/about.md"],
        ABOUT
            .replace("title: About\n", "title: About\nredirect_from: [/about/]\n")
            .replace("](../notes/note-1/)", "](../note-1/)")
    );
    let note = &found["content/notes/note-1.md"];
    assert!(note.contains("[Team](../about/#the-team)"), "{note}");
    assert!(note.contains("[About](/blog/notes/about/)"), "{note}");
}

#[test]
fn pages_whose_output_a_check_kept_from_the_one_before_have_their_links_written_anew() {
    let moves = [("content/about.md", "content/notes/about.md")];
    assert_eq!(
        changed_after(2, &files(ABOUT), &moves),
        changed(&files(ABOUT), &moves)
    );
}

#[test]
fn renaming_a_folder_moves_every_page_in_it() {
    let found = changed(&files(ABOUT), &[("content/notes", "content/jottings")]);
    assert_eq!(
        found,
        BTreeMap::from([
            (
                "content/about.md".to_owned(),
                ABOUT.replace("../notes/note-1/", "../jottings/note-1/")
            ),
            (
                "content/notes/note-1.md".to_owned(),
                NOTE.replace(
                    "title: Note 1\n",
                    "title: Note 1\nredirect_from: [/notes/note-1/]\n"
                )
            ),
        ])
    );
}

#[test]
fn the_old_address_joins_a_redirect_from_as_it_is_written() {
    let cases = [
        (
            "redirect_from: [/old/]\n",
            "redirect_from: [/old/, /about/]\n",
        ),
        ("redirect_from: []\n", "redirect_from: [/about/]\n"),
        (
            "redirect_from: /old/\n",
            "redirect_from: [/old/, /about/]\n",
        ),
        (
            "redirect_from:\n  - /old/\n",
            "redirect_from:\n  - /old/\n  - /about/\n",
        ),
    ];
    for (written, wanted) in cases {
        let about = ABOUT.replace("title: About\n", &format!("title: About\n{written}"));
        let found = changed(
            &files(&about),
            &[("content/about.md", "content/about-us.md")],
        );
        assert_eq!(
            found["content/about.md"],
            ABOUT.replace("title: About\n", &format!("title: About\n{wanted}")),
            "{written}"
        );
    }
    let bare = "A page with no front matter.\n";
    let found = changed(&files(bare), &[("content/about.md", "content/about-us.md")]);
    assert_eq!(
        found["content/about.md"],
        format!("---\nredirect_from: [/about/]\n---\n{bare}")
    );
}

#[test]
fn a_page_moved_back_where_it_was_takes_that_address_out_of_its_redirect_from() {
    let there = changed(
        &files(ABOUT),
        &[("content/about.md", "content/about-us.md")],
    );
    let moved: Vec<(&str, String)> = vec![
        files(ABOUT)[0].clone(),
        ("templates/page.html", there["templates/page.html"].clone()),
        files(ABOUT)[2].clone(),
        ("content/about-us.md", there["content/about.md"].clone()),
        (
            "content/notes/note-1.md",
            there["content/notes/note-1.md"].clone(),
        ),
    ];
    let back = changed(&moved, &[("content/about-us.md", "content/about.md")]);
    assert_eq!(
        back,
        BTreeMap::from([
            (
                "content/about-us.md".to_owned(),
                ABOUT.replace(
                    "title: About\n",
                    "title: About\nredirect_from: [/about-us/]\n"
                )
            ),
            ("content/notes/note-1.md".to_owned(), NOTE.to_owned()),
            ("templates/page.html".to_owned(), files(ABOUT)[1].1.clone()),
        ])
    );
}

#[test]
fn the_address_a_page_moves_to_is_taken_out_of_its_redirect_from_as_it_is_written() {
    let cases = [
        (
            "redirect_from: [/about-us/]\n",
            "redirect_from: [/about/]\n",
        ),
        (
            "redirect_from: [/old/, /about-us]\n",
            "redirect_from: [/old/, /about/]\n",
        ),
        ("redirect_from: /about-us/\n", "redirect_from: /about/\n"),
        (
            "redirect_from:\n  - /about-us/\n  - /old/\n",
            "redirect_from:\n  - /about/\n  - /old/\n",
        ),
        (
            "redirect_from: [/about/, /about-us/]\n",
            "redirect_from: [/about/]\n",
        ),
        (
            "redirect_from: [/about-us/, /about/]\n",
            "redirect_from: [/about/]\n",
        ),
        (
            "redirect_from: [/about-us/, /old/, /about-us/, /about-us/]\n",
            "redirect_from: [/about/, /old/]\n",
        ),
        (
            "redirect_from:\n  - /about/\n  - /about-us/\n",
            "redirect_from:\n  - /about/\n",
        ),
    ];
    for (written, wanted) in cases {
        let about = ABOUT.replace("title: About\n", &format!("title: About\n{written}"));
        let found = changed(
            &files(&about),
            &[("content/about.md", "content/about-us.md")],
        );
        assert_eq!(
            found["content/about.md"],
            ABOUT.replace("title: About\n", &format!("title: About\n{wanted}")),
            "{written}"
        );
    }
}

#[test]
fn a_page_whose_address_stays_is_left_as_it_is() {
    let about = ABOUT.replace("title: About\n", "title: About\npermalink: /about/\n");
    assert!(
        changed(
            &files(&about),
            &[("content/about.md", "content/about-us.md")]
        )
        .is_empty()
    );
    assert!(
        changed(
            &files(ABOUT),
            &[("templates/page.html", "templates/other.html")]
        )
        .is_empty()
    );
}

#[test]
fn a_link_is_written_anew_in_the_form_it_has() {
    let cases = [
        ("/blog/about/", false, "/blog/about-us/"),
        ("/blog/about/#team", false, "/blog/about-us/#team"),
        ("@url(/about/)", false, "@url(/about-us/)"),
        ("@url( \"/about/\" )", false, "@url( \"/about-us/\" )"),
        ("'/about/'", true, "'/about-us/'"),
        ("\"/blog/about/\"", false, "\"/blog/about-us/\""),
        (
            "https://example.org/blog/about/",
            false,
            "https://example.org/blog/about-us/",
        ),
        ("../about/?a=1#b", false, "../about-us/?a=1#b"),
    ];
    for (written, helper, wanted) in cases {
        let found = rewritten(written, helper, "/about-us/", "/blog", "/blog/notes/");
        assert_eq!(found.as_deref(), Some(wanted), "{written}");
    }
    assert_eq!(rewritten("#team", false, "/about-us/", "", "/"), None);
    assert_eq!(rewritten("/about-us/", false, "/about-us/", "", "/"), None);
}

#[test]
fn a_relative_path_leads_from_one_address_to_another() {
    let cases = [
        ("/blog/notes/note-1/", "/blog/about/", "../../about/"),
        ("/blog/notes/note-1/", "/blog/notes/note-2/", "../note-2/"),
        ("/blog/", "/blog/about/", "about/"),
        ("/blog/feed.xml", "/blog/about/", "about/"),
        ("/blog/about/", "/blog/about/", "./"),
        ("/", "/notes/a.html", "notes/a.html"),
    ];
    for (from, to, wanted) in cases {
        assert_eq!(relative(from, to), wanted, "{from} to {to}");
    }
}
