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
    let (record, problems) = Record::read(&scratch, &mut sources, &Settings::default());
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
    let (record, problems) = Record::read(&scratch, &mut Sources::default(), &Settings::default());
    assert!(!record.held && record.addresses.is_empty() && problems.is_empty());
}

#[test]
fn a_line_starting_with_slash_that_no_address_can_be_is_told_by_its_part_with_the_line_meant() {
    let told = |line: &str| {
        let scratch = Scratch::new("published-part");
        scratch.write(FILE, format!("/about/\n{line}\n"));
        let mut sources = Sources::default();
        let (record, problems) = Record::read(&scratch, &mut sources, &Settings::default());
        assert_eq!(record.addresses.len(), 1, "{line}");
        let [problem] = problems.as_slice() else {
            panic!("{line}: {problems:?}");
        };
        (
            crate::wording::headline(&problem.kind),
            sources.spelling(problem.place.unwrap()).to_owned(),
            crate::wording::suggestion(&problem.kind),
        )
    };
    let percent = "Malformed record: `%` can't be in a name";
    let hash = "Malformed record: `#` can't be in a name";
    let dots = "Malformed record: `..` can't be a name";
    let never = "Take the line out: Statac never publishes this URL.";
    for (line, headline, at, suggestion) in [
        (
            "/caf%C3%A9/",
            percent,
            "caf%C3%A9",
            "Write the characters themselves, not `%` codes: `/café/`.",
        ),
        (
            "/notes/caf%c3%a9.pdf",
            percent,
            "caf%c3%a9.pdf",
            "Write the characters themselves, not `%` codes: `/notes/café.pdf`.",
        ),
        // Not text, and another URL, each read as written.
        (
            "/caf%E9/",
            percent,
            "caf%E9",
            "Write the characters themselves, not `%` codes.",
        ),
        (
            "/a%2Fb/",
            percent,
            "a%2Fb",
            "Write the characters themselves, not `%` codes.",
        ),
        (
            "/notes/#top",
            hash,
            "#top",
            "Leave out the `#` and what follows it: `/notes/`.",
        ),
        (
            "/notes.pdf#page=2",
            hash,
            "notes.pdf#page=2",
            "Leave out the `#` and what follows it: `/notes.pdf`.",
        ),
        (
            "/notes/../about-2/",
            dots,
            "..",
            "Write the URL without `..`: `/about-2/`.",
        ),
        (
            "/notes/post-1/..",
            dots,
            "..",
            "Write the URL without `..`: `/notes/`.",
        ),
        (
            "/../../feed.xml",
            dots,
            "..",
            "Write the URL without `..`: `/feed.xml`.",
        ),
        (
            "/notes/./post-1/",
            "Malformed record: `.` can't be a name",
            ".",
            "Write the URL without `.`: `/notes/post-1/`.",
        ),
        // Mending the first fault leaves another.
        (
            "/notes/../caf%C3%A9/",
            dots,
            "..",
            "Write the URL without `..`.",
        ),
        (
            "/notes/a\u{7}b/",
            "Malformed record: a control character can't be in a name",
            "a\u{7}b",
            never,
        ),
        (
            "/search?q=1",
            "Malformed record: `?` can't be in a name",
            "search?q=1",
            never,
        ),
        (
            "/notes./",
            "Malformed record: a name can't end with `.`",
            "notes.",
            never,
        ),
        (
            "/notes /post-1/",
            "Malformed record: a name can't end with a space",
            "notes ",
            never,
        ),
        (
            "/con/",
            "Malformed record: Windows keeps `con` for a device",
            "con",
            never,
        ),
    ] {
        assert_eq!(
            told(line),
            (headline.into(), at.into(), Some(suggestion.into())),
            "{line}"
        );
    }
    // A line with no opening `/` is told so, whatever else it holds.
    assert_eq!(
        told("caf%C3%A9/"),
        (
            "Malformed record: `caf%C3%A9/` isn't a URL starting with `/`".into(),
            "caf%C3%A9/".into(),
            Some(
                "Write it as a URL starting with `/`, such as `/about/`, or take the line out."
                    .into()
            ),
        )
    );
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
        let (record, _) = Record::read(scratch, &mut Sources::default(), &Settings::default());
        record.keep(scratch, &addresses(now), None).unwrap().kept
    };
    let text = |scratch: &Scratch| std::fs::read_to_string(scratch.join(FILE)).unwrap();
    assert_eq!(kept(&scratch, &[]), Kept::default());
    assert!(!scratch.join(FILE).exists());
    assert_eq!(
        kept(&scratch, &["/notes/", "/about/", "/about/"]),
        Kept {
            added: 2,
            started: true,
            found: 0
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

#[test]
fn a_site_with_no_file_has_published_what_its_earlier_build_holds_that_a_reader_can_find() {
    let built = [
        "_site/.statac-built",
        "_site/index.html",
        "_site/404.html",
        "_site/CNAME",
        "_site/.well-known/security.txt",
        "_site/.DS_Store",
        "_site/site.1a2b3c4d.css",
        "_site/notes/index.html",
        "_site/notes/feed.xml",
        "_site/notes/page/2/index.html",
        "_site/notes/note-1/index.html",
        "_site/notes/note-1/image-400.1a2b3c4d.jpg",
        "_site/notes/note-1/notes.pdf",
        "_site/search/index.html",
        "_site/search/index.js",
        "_site/search/records.0123456789abcdef.bin",
        "_site/search/pages/0123456789abcdef.bin",
        "_site/scripts/index.js",
    ];
    let files: Vec<(&str, &str)> = built.iter().map(|path| (*path, "")).collect();
    let site = TempSite::site(&files);
    let found = |site: &TempSite| {
        let mut sources = Sources::default();
        let settings = crate::config::read(&site.root, &mut sources).0;
        let (record, problems) = Record::read(&site.root, &mut sources, &settings);
        assert!(problems.is_empty(), "{problems:?}");
        assert!(!record.held);
        let addresses: Vec<String> = (record.addresses.iter())
            .map(|(address, place)| {
                assert!(place.is_none());
                address.as_str().to_owned()
            })
            .collect();
        (record.found, addresses)
    };
    assert_eq!(
        found(&site),
        (
            true,
            [
                "/",
                "/.well-known/security.txt",
                "/404.html",
                "/CNAME",
                "/notes/",
                "/notes/feed.xml",
                "/notes/note-1/",
                "/notes/note-1/notes.pdf",
                "/notes/page/2/",
                "/scripts/index.js",
                "/search/",
            ]
            .map(String::from)
            .to_vec()
        )
    );
    // A folder Statac didn't build is nobody's record.
    std::fs::remove_file(site.root.join("_site/.statac-built")).unwrap();
    assert_eq!(found(&site), (false, Vec::new()));
    #[cfg(unix)]
    {
        let elsewhere = TempSite::site(&files);
        std::fs::remove_dir_all(site.root.join("_site")).unwrap();
        std::os::unix::fs::symlink(elsewhere.root.join("_site"), site.root.join("_site")).unwrap();
        assert_eq!(found(&site), (false, Vec::new()));
    }
    // The file, where there is one, is the record alone.
    let mut files = files;
    files.push((FILE, "/notes/\n"));
    let site = TempSite::site(&files);
    let mut sources = Sources::default();
    let (record, _) = Record::read(&site.root, &mut sources, &Settings::default());
    assert!(record.held && !record.found && record.addresses.len() == 1);
}
