use super::*;

fn token(text: &str) -> Token {
    Token::new(text.into()).unwrap()
}

#[test]
fn the_token_is_carried_in_the_query_with_every_byte_it_needs_escaped() {
    assert_eq!(
        relay(&token("a+b/c=d&e f"), 7),
        "https://webmention.io/api/mentions.jf2?token=a%2Bb%2Fc%3Dd%26e%20f&since_id=7\
         &per-page=100&sort-by=created&sort-dir=up"
    );
    assert_eq!(encoded("Az09-._~é"), "Az09-._~%C3%A9");
}

#[test]
fn no_refusal_of_the_relay_says_its_token() {
    let token = token("a+b/c");
    let refusals = [
        Refusal::NotAnAddress,
        Refusal::Status { status: 401 },
        Refusal::Failed {
            reason: format!("{} gave no answer", relay(&token, 0)),
        },
        Refusal::Failed {
            reason: "a+b/c was refused".into(),
        },
    ];
    for refusal in refusals {
        let said = refused(&refusal, &token);
        assert!(
            !said.contains("a+b/c") && !said.contains("a%2Bb%2Fc"),
            "{said}"
        );
    }
}

#[test]
fn a_relays_time_gives_its_day_as_written() {
    for (time, day) in [
        ("2026-09-03T21:48:39", Some("2026-09-03")),
        ("2026-09-03T23:48:39-05:00", Some("2026-09-03")),
        ("2026-09-04T04:54:54Z", Some("2026-09-04")),
        ("2026-09-03", Some("2026-09-03")),
        ("2026-02-30T00:00:00", None),
        ("2026-9-3", None),
        ("2026-09-03x", None),
        ("yesterday", None),
        ("", None),
    ] {
        assert_eq!(
            day_of(time).map(|day| day.iso_day()).as_deref(),
            day,
            "{time}"
        );
    }
}

#[test]
fn the_relays_answer_is_read_only_as_its_feed() {
    let read = children(
        br#"{"type":"feed","children":[{"wm-id":5,"wm-property":"like-of","author":{"name":7}},
            {"wm-id":"6"},{"wm-id":6.5},{}]}"#,
    )
    .unwrap();
    assert_eq!(read.len(), 4);
    assert_eq!(read[0].id, Some(5));
    assert_eq!(read[0].property.as_deref(), Some("like-of"));
    // A name that isn't text is none.
    assert_eq!(read[0].name, None);
    assert!(read[1..].iter().all(|child| child.id.is_none()));
    for answer in [&b"{\"children\":[]}"[..], b"\xff", b"null"] {
        assert!(children(answer).is_none());
    }
}
