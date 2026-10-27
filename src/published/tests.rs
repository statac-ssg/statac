use super::*;
use crate::content::Run;
use crate::problem::Code;
use crate::testing::{Scratch, TempSite};

fn run() -> Run {
    Run {
        today: None,
        drafts: false,
        url: None,
        renamed: Vec::new(),
    }
}

/// What reading `text` as the file finds: each address, and each
/// problem's code with the text it is placed at.
fn read(text: &str) -> (Vec<String>, Vec<(Code, String)>) {
    let scratch = Scratch::new("published");
    scratch.write(FILE, text);
    let mut sources = Sources::default();
    let (record, problems) = Record::read(&scratch, &mut sources);
    let addresses = (record.addresses.iter())
        .map(|(address, _)| address.as_str().to_owned())
        .collect();
    let problems = (problems.iter())
        .map(|p| (p.kind.code(), sources.spelling(p.place.unwrap()).to_owned()))
        .collect();
    (addresses, problems)
}

#[test]
fn each_line_is_an_address_from_the_top_of_the_site_but_what_says_what_the_file_is() {
    let (addresses, problems) = read(
        "# What the file is.\n\n/about/\n  /notes/feed.xml\r\n/CNAME\n/notes/\n\
         about/\n/about/\n",
    );
    assert_eq!(
        addresses,
        ["/about/", "/notes/feed.xml", "/CNAME", "/notes/"]
    );
    assert_eq!(
        problems,
        [
            (Code::MalformedRecord, "about/".into()),
            (Code::MalformedRecord, "/about/".into()),
        ]
    );
    let scratch = Scratch::new("published-none");
    let (record, problems) = Record::read(&scratch, &mut Sources::default());
    assert!(!record.held && record.addresses.is_empty() && problems.is_empty());
}

#[test]
fn an_address_kept_that_nothing_answers_at_is_told_at_its_line_with_the_sites_path() {
    let site = TempSite::site(&[
        (
            "statac.yaml",
            "language: en\nurl: https://example.org/blog/\n\
             after_build_outputs: [/made/]\nserved_elsewhere: [/blog/shop]\n",
        ),
        ("templates/page.html", "{{ page.content }}\n"),
        ("content/about.md", "---\ntitle: About\n---\n"),
        ("static/CNAME", "example.org\n"),
        (
            FILE,
            "/about/\n/CNAME\n/made/index.html\n/shop/cart/\n/team/\n/notes.pdf\n",
        ),
    ]);
    let prepared = crate::build::prepared(&site.root, &run());
    let sources = prepared.renderer.sources();
    let gone: Vec<(String, String)> = (prepared.problems.iter())
        .filter(|p| p.kind.code() == Code::UrlGone)
        .map(|p| {
            let at = sources.spelling(p.place.unwrap()).to_owned();
            (crate::wording::headline(&p.kind), at)
        })
        .collect();
    let told = |address: &str| {
        format!("URL gone: `/blog{address}` was published before, and nothing is there now")
    };
    assert_eq!(
        gone,
        [
            (told("/team/"), "/team/".into()),
            (told("/notes.pdf"), "/notes.pdf".into()),
        ]
    );
}

#[test]
fn a_build_adds_only_the_addresses_the_file_lacks_sorted_and_writes_nothing_where_it_lacks_none() {
    let scratch = Scratch::new("published-keep");
    let addresses = |written: &[&str]| -> Vec<Address> {
        (written.iter())
            .map(|w| Address::read(w).unwrap())
            .collect()
    };
    let kept = |scratch: &Scratch, now: &[&str]| {
        let (record, _) = Record::read(scratch, &mut Sources::default());
        record.keep(scratch, &addresses(now), None).unwrap().kept
    };
    let text = |scratch: &Scratch| std::fs::read_to_string(scratch.join(FILE)).unwrap();
    assert_eq!(kept(&scratch, &[]), Kept::default());
    assert!(!scratch.join(FILE).exists());
    assert_eq!(
        kept(&scratch, &["/notes/", "/about/", "/about/"]),
        Kept {
            added: 2,
            started: true
        }
    );
    assert_eq!(text(&scratch), format!("{ABOUT}/about/\n/notes/\n"));
    // What the author took out, and wrote, stays as they left it until
    // something is added.
    std::fs::write(scratch.join(FILE), "/notes/\n").unwrap();
    assert_eq!(kept(&scratch, &["/notes/"]), Kept::default());
    assert_eq!(text(&scratch), "/notes/\n");
    assert_eq!(kept(&scratch, &["/about/"]).added, 1);
    assert_eq!(text(&scratch), format!("{ABOUT}/about/\n/notes/\n"));
}

#[test]
fn serve_keeps_what_it_publishes_and_lets_go_quietly_of_what_came_and_went_while_it_ran() {
    let site = TempSite::site(&[
        ("templates/page.html", "{{ page.content }}\n"),
        ("templates/home.html", "{{ page.content }}\n"),
        ("content/index.md", "---\ntitle: Home\n---\n"),
        ("content/about.md", "---\ntitle: About\n---\n"),
        (
            "content/draft-1.md",
            "---\ntitle: Draft 1\ndraft: true\n---\n",
        ),
    ]);
    let options = crate::build::Options {
        mode: crate::build::Mode::Build,
        site: site.root.clone(),
        out: None,
        today: None,
        drafts: true,
        url: None,
    };
    let kept = || {
        let text = std::fs::read_to_string(site.root.join(FILE)).unwrap();
        text.lines()
            .filter(|line| !line.starts_with('#'))
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    let gone = |round: &crate::serve::session::Round| {
        (round.outcome.problems.iter())
            .filter(|p| p.kind.code() == Code::UrlGone)
            .count()
    };
    let (mut session, round) = crate::serve::session::Session::start(&options);
    assert!(round.outcome.kept.started, "{:?}", round.outcome.kept);
    // The draft is built, and is no address of the site's yet.
    assert_eq!(kept(), ["/", "/about/"]);
    assert!(round.recorded.is_some());

    std::fs::write(site.root.join("content/untitld.md"), "---\ntitle: A\n---\n").unwrap();
    assert_eq!(session.rebuild().outcome.kept.added, 1);
    assert_eq!(kept(), ["/", "/about/", "/untitld/"]);
    std::fs::rename(
        site.root.join("content/untitld.md"),
        site.root.join("content/untitled.md"),
    )
    .unwrap();
    std::fs::remove_file(site.root.join("content/about.md")).unwrap();
    let round = session.rebuild();
    // Only what the session didn't add itself is told.
    assert_eq!(gone(&round), 1);
    assert_eq!(kept(), ["/", "/about/", "/untitled/"]);
    drop(session);

    // A later session takes nothing out.
    std::fs::remove_file(site.root.join("content/untitled.md")).unwrap();
    let (_, round) = crate::serve::session::Session::start(&options);
    assert_eq!(gone(&round), 2);
    assert_eq!(kept(), ["/", "/about/", "/untitled/"]);
}

#[test]
fn a_warning_keeps_its_line_only_where_the_file_as_written_has_the_address_there() {
    let line = |problems: &[Problem], sources: &Sources| -> Vec<Option<usize>> {
        (problems.iter())
            .filter(|p| p.kind.code() == Code::UrlGone)
            .map(|p| (p.place).map(|at| sources.line_number(at.source, at.span.start as usize)))
            .collect()
    };
    for (now, wanted) in [(&["/b/"][..], None), (&["/d/"][..], Some(5))] {
        let site = TempSite::site(&[
            ("templates/page.html", "{{ page.content }}\n"),
            ("content/a.md", "---\ntitle: A\n---\n"),
            (FILE, &format!("{ABOUT}/a/\n/c/\n")),
        ]);
        let prepared = crate::build::prepared(&site.root, &run());
        let sources = prepared.renderer.sources();
        let mut problems = prepared.problems.clone();
        assert_eq!(line(&problems, sources), [Some(5)]);
        let now: Vec<Address> = now.iter().map(|a| Address::read(a).unwrap()).collect();
        let record = &prepared.renderer.site().published;
        let written = record.keep(&site.root, &now, None).unwrap();
        written.re_place(&mut problems, sources);
        assert_eq!(line(&problems, sources), [wanted], "{now:?}");
    }
}

#[test]
fn every_address_under_a_folder_gone_is_one_warning_under_the_outermost_such_folder() {
    let site = TempSite::site(&[
        ("templates/page.html", "{{ page.content }}\n"),
        ("content/about.md", "---\ntitle: About\n---\n"),
        ("content/notes/note-1.md", "---\ntitle: Note 1\n---\n"),
        (
            FILE,
            "/about/\n/blog/2026/post-1/\n/blog/2026/post-2/\n/blog/feed.xml\n\
             /notes/note-1/\n/notes/note-2/\n/old/\n",
        ),
    ]);
    let prepared = crate::build::prepared(&site.root, &run());
    let sources = prepared.renderer.sources();
    let told: Vec<(String, String)> = (prepared.problems.iter())
        .filter(|p| p.kind.code() == Code::UrlGone)
        .map(|p| {
            let at = sources.spelling(p.place.unwrap()).to_owned();
            (crate::wording::headline(&p.kind), at)
        })
        .collect();
    assert_eq!(
        told,
        [
            (
                "URLs gone: the 3 URLs under `/blog/` were published before, and \
                 nothing is at any of them now"
                    .into(),
                "/blog/2026/post-1/\n/blog/2026/post-2/\n/blog/feed.xml".into()
            ),
            (
                "URL gone: `/notes/note-2/` was published before, and nothing is \
                 there now"
                    .into(),
                "/notes/note-2/".into()
            ),
            (
                "URL gone: `/old/` was published before, and nothing is there now".into(),
                "/old/".into()
            ),
        ]
    );
}
