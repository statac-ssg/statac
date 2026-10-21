use super::*;
use crate::testing::Scratch;

fn read_text(text: &str) -> (Sources, Record, Vec<Problem>) {
    let mut sources = Sources::default();
    let source = sources.add(record_shown(), text.to_owned());
    let (record, problems) = Record::read(&sources, source);
    (sources, record, problems)
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
    let mut record = Record::default();
    record.set(
        "stills/youtube-abcdefghijk.jpg",
        Line {
            held: Held::Hash("0123456789abcdef".into()),
            day: day("2026-10-06"),
        },
    );
    record.set(
        "stills/vimeo-123456789.jpg",
        Line {
            held: Held::Absent,
            day: day("2026-10-05"),
        },
    );
    let text = record.text(|_| true);
    assert_eq!(
        text,
        "statac fetched 1\n\
         stills/vimeo-123456789.jpg\tabsent\t2026-10-05\n\
         stills/youtube-abcdefghijk.jpg\t0123456789abcdef\t2026-10-06\n"
    );
    let (_, again, problems) = read_text(&text);
    assert_eq!(problems, []);
    assert_eq!(again, record);
    assert_eq!(again.text(|_| true), text);
}

#[test]
fn a_line_whose_file_is_gone_is_dropped_and_one_saying_absent_kept() {
    let (_, record, _) = read_text(
        "statac fetched 1\n\
         stills/vimeo-123456789.jpg\tabsent\t2026-10-05\n\
         stills/youtube-abcdefghijk.jpg\t0123456789abcdef\t2026-10-06\n\
         stills/youtube-bcdefghijkl.jpg\tfedcba9876543210\t2026-10-06\n",
    );
    let text = record.text(|path| path != "stills/youtube-abcdefghijk.jpg");
    assert_eq!(
        text,
        "statac fetched 1\n\
         stills/vimeo-123456789.jpg\tabsent\t2026-10-05\n\
         stills/youtube-bcdefghijkl.jpg\tfedcba9876543210\t2026-10-06\n"
    );
    assert_eq!(
        record.text(|_| false),
        "statac fetched 1\nstills/vimeo-123456789.jpg\tabsent\t2026-10-05\n"
    );
}

#[test]
fn each_line_that_doesnt_read_is_told_where_it_is() {
    let (sources, record, problems) = read_text(
        "statac fetched 1\r\n\
         stills/youtube-abcdefghijk.jpg\t0123456789abcdef\r\n\
         stills/youtube-bcdefghijkl.jpg\t0123456789ABCDEF\t2026-10-06\n\
         stills/youtube-cdefghijklm.jpg\tabsent\t6 October\n\
         stills/youtube-defghijklmn.jpg\tabsent\t2026-10-06 09:00\n\
         ../record\tabsent\t2026-10-06\n\
         stills/youtube-efghijklmno.jpg\tabsent\t2026-10-06\n\
         stills/youtube-efghijklmno.jpg\tabsent\t2026-10-07\n\
         \n\
         stills/youtube-fghijklmnop.jpg\tabsent\t2026-02-30\n",
    );
    assert_eq!(
        told(&sources, &problems),
        [
            (
                RecordFault::Fields(2),
                2,
                "stills/youtube-abcdefghijk.jpg\t0123456789abcdef".into()
            ),
            (
                RecordFault::Held("0123456789ABCDEF".into()),
                3,
                "0123456789ABCDEF".into()
            ),
            (RecordFault::Day("6 October".into()), 4, "6 October".into()),
            (
                RecordFault::Day("2026-10-06 09:00".into()),
                5,
                "2026-10-06 09:00".into()
            ),
            (RecordFault::Path("../record".into()), 6, "../record".into()),
            (
                RecordFault::Twice("stills/youtube-efghijklmno.jpg".into()),
                8,
                "stills/youtube-efghijklmno.jpg".into()
            ),
            (RecordFault::Fields(1), 9, String::new()),
            (
                RecordFault::Day("2026-02-30".into()),
                10,
                "2026-02-30".into()
            ),
        ]
    );
    // What does read is kept, the first of two lines kept.
    let kept: Vec<(&str, &Line)> = record.iter().collect();
    assert_eq!(
        kept,
        [(
            "stills/youtube-efghijklmno.jpg",
            &Line {
                held: Held::Absent,
                day: day("2026-10-06")
            }
        )]
    );
}

#[test]
fn a_record_opening_with_another_line_is_told_at_it_and_read_no_further() {
    for (text, spelling) in [
        (
            "statac fetched 2\nstills/a.jpg\tabsent\t2026-10-06\n",
            "statac fetched 2",
        ),
        (
            "stills/a.jpg\tabsent\t2026-10-06\n",
            "stills/a.jpg\tabsent\t2026-10-06",
        ),
        ("", ""),
    ] {
        let (sources, record, problems) = read_text(text);
        assert_eq!(
            told(&sources, &problems),
            [(RecordFault::Opening, 1, spelling.to_owned())],
            "{text:?}"
        );
        assert!(record.is_empty());
    }
}

#[test]
fn a_site_with_no_fetched_folder_has_nothing_fetched_and_nothing_said() {
    let site = Scratch::new("none");
    let mut sources = Sources::default();
    let (fetched, problems) = read(&site.0, &mut sources);
    assert_eq!(problems, []);
    assert!(fetched.record.is_empty() && fetched.files.is_empty());
    assert_eq!(sources.ids().count(), 0);
    assert!(!site.0.join(FOLDER).exists());
}

#[test]
fn a_file_with_no_line_is_kept_with_those_the_record_names() {
    let site = Scratch::new("files");
    site.write(
        "fetched/record",
        b"statac fetched 1\nstills/youtube-abcdefghijk.jpg\t0123456789abcdef\t2026-10-06\n",
    );
    site.write("fetched/stills/youtube-abcdefghijk.jpg", b"fetched");
    site.write(
        "fetched/stills/youtube-bcdefghijkl.jpg",
        b"the author's own",
    );
    site.write(
        "fetched/stills/.youtube-cdefghijklm.jpg.1.tmp",
        b"half written",
    );
    let mut sources = Sources::default();
    let (fetched, problems) = read(&site.0, &mut sources);
    assert_eq!(problems, []);
    let paths: Vec<&str> = fetched
        .files
        .iter()
        .map(|(path, _)| path.as_str())
        .collect();
    assert_eq!(
        paths,
        [
            "stills/youtube-abcdefghijk.jpg",
            "stills/youtube-bcdefghijkl.jpg"
        ]
    );
    assert!(
        fetched
            .record
            .get("stills/youtube-bcdefghijkl.jpg")
            .is_none()
    );
    let text = fetched.record.text(|_| true);
    assert_eq!(text, sources.text(sources.ids().next().unwrap()));
}

#[cfg(unix)]
#[test]
fn a_link_out_of_the_folder_is_refused() {
    let site = Scratch::new("link");
    site.write("statac.yaml", b"title: Site\n");
    site.write("fetched/stills/youtube-abcdefghijk.jpg", b"fetched");
    std::os::unix::fs::symlink(
        site.0.join("statac.yaml"),
        site.0
            .join("fetched")
            .join("stills")
            .join("youtube-bcdefghijkl.jpg"),
    )
    .unwrap();
    let mut sources = Sources::default();
    let (fetched, problems) = read(&site.0, &mut sources);
    assert_eq!(fetched.files.len(), 1);
    assert!(
        matches!(&problems[..], [problem] if problem.kind.code() == crate::problem::Code::OutsideSite),
        "{problems:?}"
    );
}

#[test]
fn a_still_is_named_after_its_video() {
    use crate::problem::Host;
    use stills::path;
    assert_eq!(
        path(Host::YouTube, "abcdefghijk"),
        "stills/youtube-abcdefghijk.jpg"
    );
    assert_eq!(path(Host::Vimeo, "123456789"), "stills/vimeo-123456789.jpg");
    assert!(!Host::YouTube.is_one_video("videoseries"));
    assert!(Host::Vimeo.is_one_video("123456789"));
    assert_eq!(hash_of(b"fetched").len(), 16);
    assert_ne!(key("stills/a.jpg"), key("stills/b.jpg"));
}
