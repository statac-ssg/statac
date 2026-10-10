use super::*;

fn spot(address: &str) -> Spot {
    Spot::of(address).unwrap()
}

fn day(written: &str) -> Date {
    crate::date::read(written).unwrap()
}

fn read_text(text: &str) -> (Sources, List, Vec<Problem>) {
    let mut sources = Sources::default();
    let source = sources.add(shown(), text.to_owned());
    let (list, problems) = List::read(&sources, source);
    (sources, list, problems)
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
fn a_list_written_reads_back_the_same_sorted_by_address() {
    let mut list = List::default();
    for (reach, address) in [
        (Reach::Sender, "https://b.example/"),
        (Reach::Mention, "https://a.example/post-1/"),
        (Reach::Mention, "https://a.example/post-2/"),
    ] {
        list.set(Entry {
            reach,
            address: address.into(),
            day: day("2026-10-06"),
        });
    }
    // The same address again replaces the line.
    list.set(Entry {
        reach: Reach::Sender,
        address: "http://www.a.example/post-2".into(),
        day: day("2026-10-07"),
    });
    let text = list.text();
    assert_eq!(
        text,
        "statac refused 1\n\
         sender\thttp://www.a.example/post-2\t2026-10-07\n\
         mention\thttps://a.example/post-1/\t2026-10-06\n\
         sender\thttps://b.example/\t2026-10-06\n"
    );
    let (_, again, problems) = read_text(&text);
    assert_eq!(problems, []);
    assert_eq!(again.text(), text);
}

#[test]
fn a_mention_is_refused_by_its_address_and_by_its_senders() {
    let (_, list, problems) = read_text(
        "statac refused 1\n\
         mention\thttps://a.example/post-1/\t2026-10-06\n\
         sender\thttps://b.example/c/\t2026-10-06\n",
    );
    assert_eq!(problems, []);
    let refusing = |url: &str, author: Option<&str>| {
        list.refusing(url, author)
            .map(|entry| entry.address.as_str())
    };
    assert_eq!(
        refusing("http://A.example/post-1", None),
        Some("https://a.example/post-1/")
    );
    assert_eq!(refusing("https://a.example/post-1/reply-2", None), None);
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
}

#[test]
fn each_line_that_doesnt_read_is_told_where_it_is() {
    let (sources, list, problems) = read_text(
        "statac refused 1\n\
         mention\thttps://a.example/post-1/\n\
         site\thttps://a.example/\t2026-10-06\n\
         sender\ta.example\t2026-10-06\n\
         sender\thttps://b.example/\t6 October\n\
         sender\thttps://c.example/\t2026-10-06\n\
         mention\thttp://www.C.example\t2026-10-06\n",
    );
    let told: Vec<(RecordFault, String)> = problems
        .iter()
        .map(|problem| {
            let Kind::MalformedRecord { fault } = &problem.kind else {
                panic!("{problem:?}");
            };
            let place = problem.place.unwrap();
            (fault.clone(), sources.spelling(place).to_owned())
        })
        .collect();
    assert_eq!(
        told,
        [
            (
                RecordFault::RefusedFields(2),
                "mention\thttps://a.example/post-1/".into()
            ),
            (RecordFault::Reach("site".into()), "site".into()),
            (RecordFault::Address("a.example".into()), "a.example".into()),
            (RecordFault::Day("6 October".into()), "6 October".into()),
            (
                RecordFault::Twice("http://www.C.example".into()),
                "http://www.C.example".into()
            ),
        ]
    );
    assert_eq!(
        list.text(),
        "statac refused 1\nsender\thttps://c.example/\t2026-10-06\n"
    );
}

#[test]
fn a_list_opening_with_another_line_is_told_at_it_and_read_no_further() {
    for text in [
        "",
        "statac fetched 1\nsender\thttps://a.example/\t2026-10-06\n",
    ] {
        let (_, list, problems) = read_text(text);
        assert!(list.is_empty());
        assert!(
            matches!(
                &problems[..],
                [Problem {
                    kind: Kind::MalformedRecord {
                        fault: RecordFault::RefusedOpening
                    },
                    ..
                }]
            ),
            "{problems:?}"
        );
    }
}
