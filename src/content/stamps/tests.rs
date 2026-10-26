use super::*;
use crate::date;
use crate::source::Sources;

fn stamp(text: &str) -> Stamp {
    match read(text) {
        Some(Ok(stamp)) => stamp,
        other => panic!("{text}: {other:?}"),
    }
}

/// The moment at midnight as `today` begins.
fn on(today: &str) -> Moment {
    let today = date::read(today).unwrap();
    let now = date::read(&format!("{} 00:00", today.iso_day())).unwrap();
    Moment { today, now }
}

fn written(text: &str, today: &str) -> String {
    stamp(text).written(&on(today)).unwrap()
}

#[test]
fn every_word_is_read_with_and_without_a_time() {
    let days = [
        ("today", Day::Today),
        ("tomorrow", Day::Tomorrow),
        ("yesterday", Day::Yesterday),
        ("monday", Day::Weekday(0)),
        ("tuesday", Day::Weekday(1)),
        ("wednesday", Day::Weekday(2)),
        ("thursday", Day::Weekday(3)),
        ("friday", Day::Weekday(4)),
        ("saturday", Day::Weekday(5)),
        ("sunday", Day::Weekday(6)),
    ];
    for (word, day) in days {
        assert_eq!(stamp(&format!("={word}")), Stamp { day, time: None });
        for (time, read) in [("00:00", (0, 0)), ("09:05", (9, 5)), ("23:59", (23, 59))] {
            let text = format!("={word} {time}");
            let time = Some(read);
            assert_eq!(stamp(&text), Stamp { day, time }, "{text}");
            assert_eq!(stamp(&text).spelling(), text);
        }
    }
    let now = Stamp {
        day: Day::Now,
        time: None,
    };
    assert_eq!(stamp("=now"), now);
}

#[test]
fn each_word_names_its_day_from_every_day_of_a_week() {
    // Monday 5 October 2026 to Sunday 11 October.
    let week = [
        "2026-10-05",
        "2026-10-06",
        "2026-10-07",
        "2026-10-08",
        "2026-10-09",
        "2026-10-10",
        "2026-10-11",
    ];
    for (at, today) in week.iter().enumerate() {
        assert_eq!(written("=today", today), *today);
        if at > 0 {
            assert_eq!(written("=yesterday", today), week[at - 1]);
            assert_eq!(written("=tomorrow", week[at - 1]), *today);
        }
        for (day, name) in WEEKDAYS.iter().enumerate() {
            // The next such day after today: this week's where it is
            // still to come, next week's on the day itself and after.
            let ahead = if day > at { day - at } else { day + 7 - at };
            let expected = format!("2026-10-{:02}", 5 + at + ahead);
            assert_eq!(written(&format!("={name}"), today), expected, "{today}");
            assert_eq!(
                written(&format!("={name} 09:00"), today),
                format!("{expected} 09:00")
            );
        }
    }
    assert_eq!(written("=monday", "2026-10-05"), "2026-10-12");
    assert_eq!(written("=sunday", "2026-10-05"), "2026-10-11");
}

#[test]
fn days_are_counted_across_the_end_of_a_month_and_a_year() {
    let cases = [
        // Saturday 31 October 2026.
        ("=tomorrow", "2026-10-31", "2026-11-01"),
        ("=monday 09:00", "2026-10-31", "2026-11-02 09:00"),
        ("=saturday", "2026-10-31", "2026-11-07"),
        ("=yesterday", "2026-11-01", "2026-10-31"),
        // February in a leap year and out of one.
        ("=tomorrow", "2028-02-28", "2028-02-29"),
        ("=tomorrow", "2027-02-28", "2027-03-01"),
        ("=yesterday", "2028-03-01", "2028-02-29"),
        // Thursday 31 December 2026.
        ("=tomorrow 18:30", "2026-12-31", "2027-01-01 18:30"),
        ("=friday", "2026-12-31", "2027-01-01"),
        ("=thursday", "2026-12-31", "2027-01-07"),
        ("=yesterday", "2027-01-01", "2026-12-31"),
        ("=today 23:59", "2026-12-31", "2026-12-31 23:59"),
    ];
    for (text, today, expected) in cases {
        assert_eq!(written(text, today), expected, "{text} on {today}");
    }
}

#[test]
fn a_day_no_date_can_name_is_not_written() {
    assert_eq!(stamp("=tomorrow").written(&on("9999-12-31")), None);
    assert_eq!(stamp("=yesterday").written(&on("0000-01-01")), None);
    assert_eq!(written("=today", "9999-12-31"), "9999-12-31");
}

#[test]
fn now_is_the_present_minute() {
    let moment = Moment {
        today: date::read("2026-10-06").unwrap(),
        now: date::read("2026-10-06 14:07").unwrap(),
    };
    assert_eq!(stamp("=now").written(&moment).unwrap(), "2026-10-06 14:07");
    // The second time the clocks show a time, which only an offset names.
    let moment = Moment {
        today: date::read("2026-10-25").unwrap(),
        now: date::read("2026-10-25 01:30 +00:00").unwrap(),
    };
    let written = stamp("=now").written(&moment).unwrap();
    assert_eq!(written, "2026-10-25 01:30 +00:00");
    assert_eq!(date::read(&written), Ok(moment.now));
    let moment = Moment {
        today: date::read("2026-11-01").unwrap(),
        now: date::read("2026-11-01 01:30 -05:00").unwrap(),
    };
    assert_eq!(
        stamp("=now").written(&moment).unwrap(),
        "2026-11-01 01:30 -05:00"
    );
    // A day fixed for a run that can be repeated starts at its midnight.
    assert_eq!(written_now_on("2026-10-04"), "2026-10-04 00:00");
}

fn written_now_on(today: &str) -> String {
    stamp("=now").written(&on(today)).unwrap()
}

#[test]
fn what_is_written_is_read_back_as_the_date_it_names() {
    for text in ["=today", "=monday 09:00", "=tomorrow 18:30", "=now"] {
        let written = written(text, "2026-10-06");
        let read = date::read(&written).unwrap_or_else(|fault| panic!("{written}: {fault:?}"));
        assert_eq!(spelt(&read), written);
    }
}

#[test]
fn another_spelling_is_refused_with_the_stamp_offered() {
    let cases = [
        ("=Today", "=today"),
        ("=TODAY", "=today"),
        ("= today", "=today"),
        ("=today ", "=today"),
        ("=Now", "=now"),
        ("=mon", "=monday"),
        ("=Tue", "=tuesday"),
        ("=tues", "=tuesday"),
        ("=wed", "=wednesday"),
        ("=weds", "=wednesday"),
        ("=thurs", "=thursday"),
        ("=fri", "=friday"),
        ("=sat", "=saturday"),
        ("=sun", "=sunday"),
        ("=next monday", "=monday"),
        ("=next fri", "=friday"),
        ("=Next Friday 19:00", "=friday 19:00"),
        ("=today 9am", "=today 09:00"),
        ("=today 9 am", "=today 09:00"),
        ("=today 9:30pm", "=today 21:30"),
        ("=today 12am", "=today 00:00"),
        ("=today 12 p.m.", "=today 12:00"),
        ("=today 9:00", "=today 09:00"),
        ("=today 09.00", "=today 09:00"),
        ("=today 9h", "=today 09:00"),
        ("=today 9h30", "=today 09:30"),
        ("=today at 09:00", "=today 09:00"),
        ("=today T09:00", "=today 09:00"),
        ("=today  09:00", "=today 09:00"),
        ("=today09:00", "=today 09:00"),
        ("=today 09:00:30", "=today 09:00"),
        ("=monday 0900am", "=monday 09:00"),
        // The present has one time, so a time after it is today's.
        ("=now 09:00", "=today 09:00"),
        ("=now 9am", "=today 09:00"),
    ];
    for (text, ours) in cases {
        assert_eq!(
            read(text),
            Some(Err(Refused::Respelt(ours.into()))),
            "{text}"
        );
    }
}

#[test]
fn a_slip_in_a_word_is_refused_with_the_nearest_offered() {
    let cases = [
        ("=tomorow", "=tomorrow"),
        ("=tommorrow", "=tomorrow"),
        ("=todya", "=today"),
        ("=yesterdy", "=yesterday"),
        ("=wendesday", "=wednesday"),
        ("=Thrusday 9am", "=thursday 09:00"),
        ("=saterday 10:00", "=saturday 10:00"),
    ];
    for (text, nearest) in cases {
        assert_eq!(
            read(text),
            Some(Err(Refused::Misspelt(nearest.into()))),
            "{text}"
        );
    }
}

#[test]
fn a_time_no_clock_shows_and_one_read_two_ways_are_refused() {
    let no_such = [
        ("=today 25:00", "25:00"),
        ("=today 09:60", "09:60"),
        ("=monday 24:00", "24:00"),
        ("=today 13pm", "13pm"),
        ("=today 0am", "0am"),
        ("=Today 9:75 PM", "9:75 PM"),
    ];
    for (text, time) in no_such {
        assert_eq!(
            read(text),
            Some(Err(Refused::NoSuchTime(time.into()))),
            "{text}"
        );
    }
    for text in ["=today 9", "=today 0900", "=monday 18", "=today 2026"] {
        assert_eq!(read(text), Some(Err(Refused::UnclearTime)), "{text}");
    }
}

#[test]
fn anything_else_is_the_authors_own_text() {
    let texts = [
        "today",
        "",
        "=",
        "==",
        "=IF(A1)",
        "=SUM(A1:A9)",
        "=TODAY()",
        "=NOW()",
        "=today's news",
        "=Today is the day",
        "=now playing",
        "=monday motivation",
        "=next",
        "=next week",
        "=next monday morning",
        "=1",
        "=09:00",
        "=mo",
        "=no",
        "=new",
        "=sum",
        "=money",
        "=friend",
        "=> next",
        "=today 9 o'clock",
        "=today 2026-10-06",
        "=today 12345",
        " =today",
        "\u{212a}=today",
    ];
    for text in texts {
        assert_eq!(read(text), None, "{text}");
    }
}

/// The problems with the stamps in a file's front matter, each as its
/// kind and the text it is placed at.
fn found(text: &str) -> Vec<(Kind, String)> {
    let mut sources = Sources::default();
    let source = sources.add("content/post-1.md", text.to_owned());
    let (front, _) = crate::yaml::front_matter(&sources, source).unwrap();
    problems(&front.unwrap())
        .into_iter()
        .map(|problem| {
            let place = problem.place.unwrap();
            (problem.kind, sources.spelling(place).to_owned())
        })
        .collect()
}

fn waiting(field: &str, written: &str) -> Kind {
    Kind::NotStamped {
        field: field.into(),
        written: written.into(),
    }
}

#[test]
fn a_stamp_is_found_wherever_a_value_is_written() {
    let text = "---\n\
                title: Post 1\n\
                date: =today\n\
                updated: \"=now\"\n\
                first: '=monday 09:00'\n\
                event:\n  start: =friday 19:00\n  rooms:\n    - opens: =tomorrow\n\
                days: [=saturday, =sunday 10:00]\n\
                pages.updated: =yesterday\n\
                ---\n\
                Written =today.\n";
    let expected = [
        (waiting("date", "=today"), "=today"),
        (waiting("updated", "=now"), "\"=now\""),
        (waiting("first", "=monday 09:00"), "'=monday 09:00'"),
        (waiting("event.start", "=friday 19:00"), "=friday 19:00"),
        (waiting("event.rooms.opens", "=tomorrow"), "=tomorrow"),
        (waiting("days", "=saturday"), "=saturday"),
        (waiting("days", "=sunday 10:00"), "=sunday 10:00"),
        (waiting("pages.updated", "=yesterday"), "=yesterday"),
    ];
    let found = found(text);
    assert_eq!(found.len(), expected.len(), "{found:?}");
    for ((kind, at), (expected, written)) in found.into_iter().zip(expected) {
        assert_eq!((kind, at.as_str()), (expected, written));
    }
}

#[test]
fn a_key_a_block_and_the_writing_hold_no_stamp() {
    let text = "---\n\
                =today: a key\n\
                kept: |-\n  =today\n\
                folded: >\n  =tomorrow\n\
                formula: =IF(A1)\n\
                ---\n\
                =today\n";
    assert_eq!(found(text), []);
}

#[test]
fn a_stamp_repeated_by_an_alias_is_told_once_where_it_is_written() {
    let text = "---\ndate: &day =today\nupdated: *day\n---\n";
    assert_eq!(found(text), [(waiting("date", "=today"), "=today".into())]);
}

#[test]
fn a_value_meant_as_a_stamp_is_told_as_its_kind() {
    let text = "---\ndate: =Today\nupdated: =tomorow\nstart: =today 25:00\nend: =today 9\n---\n";
    let word = Word::Stamp;
    let expected = [
        Kind::OtherSpelling {
            word,
            written: "=Today".into(),
            ours: "=today".into(),
        },
        Kind::UnknownWord {
            word,
            written: "=tomorow".into(),
            nearest: Some("=tomorrow".into()),
        },
        Kind::NoSuchDate {
            written: "=today 25:00".into(),
            missing: Missing::Time("25:00".into()),
        },
        Kind::UnknownWord {
            word,
            written: "=today 9".into(),
            nearest: None,
        },
    ];
    let kinds: Vec<Kind> = found(text).into_iter().map(|(kind, _)| kind).collect();
    assert_eq!(kinds, expected);
    assert!(kinds.iter().all(of_a_refused_stamp));
    assert!(!of_a_refused_stamp(&waiting("date", "=today")));
}
