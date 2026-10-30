use super::*;

fn read_text(text: &str) -> (Sources, Links, Vec<Problem>) {
    let mut sources = Sources::default();
    let source = sources.add(shown(), text.to_owned());
    let (links, problems) = Links::read(&sources, source);
    (sources, links, problems)
}

fn day(written: &str) -> Date {
    crate::date::read(written).unwrap()
}

/// Each problem's fault, its line counted from 1, and the text it is
/// placed under.
fn told(sources: &Sources, problems: &[Problem]) -> Vec<(RecordFault, usize, String)> {
    problems
        .iter()
        .map(|problem| {
            let Kind::MalformedRecord { fault } = &problem.kind else {
                panic!("{problem:?}");
            };
            let place = problem.place.unwrap();
            let line = sources.line_column(place.source, place.span.start as usize);
            (fault.clone(), line.line, sources.spelling(place).to_owned())
        })
        .collect()
}

#[test]
fn a_record_written_reads_back_the_same_and_sorted() {
    let mut links = Links::default();
    let answers = [
        ("https://b.example/", Answer::Ok),
        (
            "http://a.example/post-1/",
            Answer::Moved("https://a.example/posts/post-1/".into()),
        ),
        ("https://c.example/gone", Answer::Missing(410)),
        ("https://c.example/missing", Answer::Missing(404)),
        ("https://d.example/", Answer::NoHost),
        ("https://e.example/", Answer::Certificate),
        ("https://f.example/", Answer::Unanswered("403".into())),
        ("https://g.example/", Answer::Unanswered("challenge".into())),
    ];
    for (url, answer) in answers {
        let line = Line {
            answer,
            day: day("2026-10-06"),
        };
        links.set(url, line);
    }
    let text = links.text(|_| true);
    assert_eq!(
        text,
        "statac links 1\n\
         http://a.example/post-1/\tmoved https://a.example/posts/post-1/\t2026-10-06\n\
         https://b.example/\tok\t2026-10-06\n\
         https://c.example/gone\t410\t2026-10-06\n\
         https://c.example/missing\t404\t2026-10-06\n\
         https://d.example/\tno-host\t2026-10-06\n\
         https://e.example/\tcertificate\t2026-10-06\n\
         https://f.example/\tunanswered 403\t2026-10-06\n\
         https://g.example/\tunanswered challenge\t2026-10-06\n"
    );
    let (_, again, problems) = read_text(&text);
    assert_eq!(problems, []);
    assert_eq!(again, links);
    assert_eq!(
        links.text(|url| url.contains("c.example")),
        "statac links 1\n\
         https://c.example/gone\t410\t2026-10-06\n\
         https://c.example/missing\t404\t2026-10-06\n"
    );
}

#[test]
fn every_line_not_as_statac_writes_it_is_told_at_the_field_and_left_out() {
    let text = "statac links 1\n\
                https://a.example/\tok\t2026-10-06\n\
                https://b.example/\tok\n\
                b.example/\tok\t2026-10-06\n\
                https://b.example/#top\tok\t2026-10-06\n\
                HTTPS://B.EXAMPLE/\tok\t2026-10-06\n\
                https://b.example/\tfine\t2026-10-06\n\
                https://b.example/\t200\t2026-10-06\n\
                https://b.example/\tunanswered sometimes\t2026-10-06\n\
                https://b.example/\tunanswered 700\t2026-10-06\n\
                https://b.example/\tmoved elsewhere\t2026-10-06\n\
                https://b.example/\tok\t6 October\n\
                https://a.example/\t404\t2026-10-06\n\
                https://c.example/\tunanswered 503\t2026-10-06\n";
    let (sources, links, problems) = read_text(text);
    assert_eq!(
        told(&sources, &problems),
        [
            (
                RecordFault::LinkFields(2),
                3,
                "https://b.example/\tok".into()
            ),
            (
                RecordFault::NotALink("b.example/".into()),
                4,
                "b.example/".into()
            ),
            (
                RecordFault::NotALink("https://b.example/#top".into()),
                5,
                "https://b.example/#top".into()
            ),
            (
                RecordFault::NotALink("HTTPS://B.EXAMPLE/".into()),
                6,
                "HTTPS://B.EXAMPLE/".into()
            ),
            (RecordFault::Answer("fine".into()), 7, "fine".into()),
            (RecordFault::Answer("200".into()), 8, "200".into()),
            (
                RecordFault::Answer("unanswered sometimes".into()),
                9,
                "unanswered sometimes".into()
            ),
            (
                RecordFault::Answer("unanswered 700".into()),
                10,
                "unanswered 700".into()
            ),
            (
                RecordFault::Answer("moved elsewhere".into()),
                11,
                "moved elsewhere".into()
            ),
            (RecordFault::Day("6 October".into()), 12, "6 October".into()),
            (
                RecordFault::Twice("https://a.example/".into()),
                13,
                "https://a.example/".into()
            ),
        ]
    );
    assert_eq!(links.len(), 2);
    assert_eq!(links.get("https://a.example/").unwrap().answer, Answer::Ok);
    assert_eq!(
        links.get("https://c.example/").unwrap().answer,
        Answer::Unanswered("503".into())
    );
}

#[test]
fn a_record_without_its_first_line_is_told_there_and_read_no_further() {
    for text in ["", "statac fetched 1\nhttps://a.example/\tok\t2026-10-06\n"] {
        let (sources, links, problems) = read_text(text);
        let spelling = text.lines().next().unwrap_or_default();
        assert_eq!(
            told(&sources, &problems),
            [(RecordFault::LinksOpening, 1, spelling.to_owned())]
        );
        assert_eq!(links.len(), 0);
    }
}

#[test]
fn a_url_is_recorded_as_the_url_standard_writes_it_without_its_fragment() {
    let key_of = |written: &str| key(&Url::parse(written).unwrap());
    assert_eq!(
        key_of("HTTPS://Example.ORG/a b#part").as_deref(),
        Some("https://example.org/a%20b")
    );
    assert_eq!(
        key_of("http://example.org").as_deref(),
        Some("http://example.org/")
    );
    assert_eq!(
        key_of("https://bücher.example/").as_deref(),
        Some("https://xn--bcher-kva.example/")
    );
    assert_eq!(key_of("mailto:someone@example.org"), None);
    assert_eq!(key_of("ftp://example.org/file"), None);
}

#[test]
fn a_success_holds_thirty_to_forty_five_days_spread_by_url_and_anything_else_a_week() {
    let asked = day("2026-10-01");
    let line = |answer| Line { answer, day: asked };
    let after = |days: i8| {
        let date = jiff::civil::date(2026, 10, 1)
            .checked_add(jiff::Span::new().days(days))
            .unwrap();
        day(&date.to_string())
    };
    for answer in [
        Answer::Missing(404),
        Answer::NoHost,
        Answer::Certificate,
        Answer::Unanswered("timeout".into()),
    ] {
        let line = line(answer.clone());
        assert!(!line.due("https://a.example/", &after(6)), "{answer:?}");
        assert!(line.due("https://a.example/", &after(7)), "{answer:?}");
    }
    let mut holds = HashSet::new();
    for n in 0..200 {
        let url = format!("https://a.example/post-{n}/");
        for answer in [Answer::Ok, Answer::Moved("https://b.example/".into())] {
            let line = line(answer);
            let first = (29..=46)
                .find(|&days| line.due(&url, &after(days)))
                .unwrap();
            assert!((30..=45).contains(&first), "{url}: {first}");
            holds.insert(first);
            assert!((first..=60).all(|days| line.due(&url, &after(days))));
        }
    }
    // Spread over the whole span, so a site checked on one day isn't all
    // due again on one day.
    assert_eq!(holds.len(), 16);
    // A day before the answer, as a clock set wrong gives, holds it.
    assert!(!line(Answer::Ok).due("https://a.example/", &day("2026-09-30")));
}

use std::collections::HashSet;
