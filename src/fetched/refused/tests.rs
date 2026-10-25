use super::*;
use crate::source::Sources;

fn spot(address: &str) -> Spot {
    Spot::of(address).unwrap()
}

/// What `statac.yaml` holding `text` refuses.
fn refused(text: &str) -> Refused {
    let mut sources = Sources::default();
    let source = sources.add(crate::config::FILE_NAME, text.to_owned());
    let (settings, problems) = crate::config::read_source(&sources, source);
    assert_eq!(problems, [], "{text}");
    Refused::of(&settings)
}

#[test]
fn an_address_is_itself_whatever_its_scheme_www_capitals_and_final_slash() {
    let one = spot("https://a.example/post-1/");
    for other in [
        "http://a.example/post-1",
        "https://www.a.example/post-1/",
        "https://A.EXAMPLE/post-1",
        "http://WWW.A.example/post-1/",
    ] {
        assert_eq!(spot(other), one, "{other}");
    }
    for other in [
        "https://a.example/Post-1",
        "https://a.example:8080/post-1",
        "https://b.example/post-1",
        "https://a.example/post-1?page=2",
    ] {
        assert_ne!(spot(other), one, "{other}");
    }
    for other in ["ftp://a.example/post-1", "a.example/post-1", "https://", ""] {
        assert_eq!(Spot::of(other), None, "{other}");
    }
}

#[test]
fn an_address_is_under_another_at_a_path_boundary_on_the_same_host_and_port() {
    let under = |address: &str, over: &str| spot(address).at_or_under(&spot(over));
    assert!(under("https://a.example/a/post-1/", "https://a.example/a"));
    assert!(under("https://a.example/a", "http://www.a.example/a/"));
    assert!(under(
        "https://a.example/post-1?reply=2",
        "https://a.example/"
    ));
    assert!(under(
        "https://a.example/post-1#reply-2",
        "https://a.example/post-1/"
    ));
    assert!(!under("https://a.example/ab", "https://a.example/a"));
    assert!(!under("https://a.example/a", "https://a.example/a/post-1"));
    assert!(!under("https://a.example:8080/a", "https://a.example/"));
    assert!(!under("https://a.example/a", "https://a.example:8080/"));
    assert!(!under("https://b.a.example/a", "https://a.example/"));
    assert!(!under("https://a.example/a/b", "https://a.example/a?b=1"));
}

#[test]
fn a_mention_is_refused_by_its_own_url_or_its_senders_at_or_under_a_line() {
    let list = refused(
        "refuse_webmentions_from:\n  - https://a.example/post-1/\n  - https://b.example/c/\n",
    );
    let refusing = |url: &str, author: Option<&str>| list.refusing(url, author);
    assert_eq!(
        refusing("http://A.example/post-1", None),
        Some("https://a.example/post-1/")
    );
    assert_eq!(
        refusing("https://a.example/post-1/reply-2", None),
        Some("https://a.example/post-1/")
    );
    assert_eq!(refusing("https://a.example/post-2", None), None);
    assert_eq!(
        refusing("https://b.example/c/post-1", None),
        Some("https://b.example/c/")
    );
    assert_eq!(
        refusing("https://d.example/post-1", Some("https://b.example/c")),
        Some("https://b.example/c/")
    );
    assert_eq!(
        refusing("https://b.example/cd", Some("https://b.example/")),
        None
    );
    assert_eq!(refusing("https://e.example/", Some("javascript:a")), None);

    // One URL stands for a list of one; none refuses nothing.
    let one = refused("refuse_webmentions_from: https://a.example/\n");
    assert_eq!(
        one.refusing("https://a.example/post-1/", None),
        Some("https://a.example/")
    );
    let none = refused("title: Site\n");
    assert_eq!(none.refusing("https://a.example/", None), None);
}

#[test]
fn the_retired_list_gives_its_urls_and_counts_the_lines_that_dont_read() {
    assert_eq!(
        carried(
            "statac refused 1\n\
             mention\thttps://a.example/post-1/\t2026-10-06\n\
             sender\thttps://b.example/\t2026-10-06\n"
        ),
        (
            vec![
                "https://a.example/post-1/".to_owned(),
                "https://b.example/".to_owned()
            ],
            0
        )
    );
    assert_eq!(
        carried(
            "statac refused 1\n\
             mention\thttps://a.example/post-1/\n\
             sender\ta.example\t2026-10-06\n\
             \n\
             sender\thttps://c.example/\t2026-10-06\n"
        ),
        (vec!["https://c.example/".to_owned()], 2)
    );
    // A line in the opening's place is a line like any other.
    assert_eq!(
        carried("sender\thttps://a.example/\t2026-10-06\n"),
        (vec!["https://a.example/".to_owned()], 0)
    );
    assert_eq!(carried("statac fetched 1\n"), (Vec::new(), 1));
    assert_eq!(carried(""), (Vec::new(), 0));
    assert_eq!(carried("statac refused 1\n"), (Vec::new(), 0));
}
