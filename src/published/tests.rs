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
             after_build_makes: [/made/]\nnot_ours: [/blog/shop]\n",
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
        .filter(|p| p.kind.code() == Code::AddressGone)
        .map(|p| {
            let at = sources.spelling(p.place.unwrap()).to_owned();
            (crate::wording::headline(&p.kind), at)
        })
        .collect();
    let told = |address: &str| {
        format!(
            "Address gone: `/blog{address}` was published before, and nothing answers there now"
        )
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
        record.keep(scratch, &addresses(now)).unwrap()
    };
    let text = |scratch: &Scratch| std::fs::read_to_string(scratch.join(FILE)).unwrap();
    assert_eq!(
        kept(&scratch, &[]),
        Kept {
            added: 0,
            started: true
        }
    );
    assert_eq!(text(&scratch), ABOUT);
    assert_eq!(
        kept(&scratch, &["/notes/", "/about/", "/about/"]),
        Kept {
            added: 2,
            started: false
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
