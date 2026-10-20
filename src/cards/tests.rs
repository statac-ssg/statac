use super::*;
use crate::testing::TempSite;

/// The files a description names, standing in for a picture and fonts.
const FILES: &[(&str, &str)] = &[
    ("templates/og/background.png", "png"),
    ("templates/og/text.ttf", "ttf"),
    ("templates/og/more.ttf", "ttf"),
];

fn read_site(files: &[(&str, &str)]) -> (Sources, Cards, Vec<Problem>) {
    let site = TempSite::new(&[FILES, files].concat());
    let mut sources = Sources::default();
    let (cards, problems) = read(&site.root, &mut sources);
    (sources, cards, problems)
}

const VALID: &str = "\
background: background.png
font: [text.ttf, more.ttf]
title.text: x
title.place: centre
title.size: up to 75
title.colour: \"#F2E9D3\"
";

/// The valid description with the line for each key given replaced, or
/// taken out for none, and any other line given added.
fn varied(changes: &[(&str, Option<&str>)]) -> String {
    let mut lines: Vec<String> = VALID.lines().map(str::to_owned).collect();
    for (key, line) in changes {
        let at = lines.iter().position(|l| l.split(':').next() == Some(*key));
        match (at, line) {
            (Some(at), Some(line)) => lines[at] = (*line).to_owned(),
            (Some(at), None) => {
                lines.remove(at);
            }
            (None, Some(line)) => lines.push((*line).to_owned()),
            (None, None) => panic!("no line for {key}"),
        }
    }
    lines.join("\n") + "\n"
}

#[test]
fn every_problem_is_told_at_what_was_written() {
    let wrong = |field: &str, expected, written: &str| Kind::WrongKind {
        field: field.into(),
        expected,
        found: Found::Written(written.into()),
        respelt: None,
    };
    let unread = |field: &str, nearest: Option<&str>| Kind::SetAside {
        field: field.into(),
        why: SetAside::NotOnCard {
            nearest: nearest.map(Into::into),
        },
    };
    let cases = [
        (
            varied(&[("background", None)]),
            Kind::MissingField {
                field: "background".into(),
                required_by: Requirer::Card,
            },
            "font: [text.ttf, more.ttf]",
        ),
        (
            varied(&[("font", None)]),
            Kind::MissingField {
                field: "font".into(),
                required_by: Requirer::Card,
            },
            "background: background.png",
        ),
        (
            varied(&[("font", Some("font: []"))]),
            Kind::MissingField {
                field: "font".into(),
                required_by: Requirer::Card,
            },
            "font",
        ),
        (
            varied(&[("title.size", None)]),
            Kind::MissingField {
                field: "title.size".into(),
                required_by: Requirer::CardText,
            },
            "title.text",
        ),
        (
            varied(&[("title.place", Some("title.place: top lft"))]),
            Kind::UnknownWord {
                word: Word::Place,
                written: "top lft".into(),
                nearest: Some("top left".into()),
            },
            "top lft",
        ),
        (
            varied(&[("title.place", Some("title.place: middle"))]),
            Kind::UnknownWord {
                word: Word::Place,
                written: "middle".into(),
                nearest: None,
            },
            "middle",
        ),
        (
            varied(&[("title.place", Some("title.place: Center"))]),
            Kind::OtherSpelling {
                word: Word::Place,
                written: "Center".into(),
                ours: "centre".into(),
            },
            "Center",
        ),
        (
            varied(&[("title.place", Some("title.place: Top Left"))]),
            Kind::OtherSpelling {
                word: Word::Place,
                written: "Top Left".into(),
                ours: "top left".into(),
            },
            "Top Left",
        ),
        (
            varied(&[("title.place", Some("title.place: CENTRE"))]),
            Kind::OtherSpelling {
                word: Word::Place,
                written: "CENTRE".into(),
                ours: "centre".into(),
            },
            "CENTRE",
        ),
        (
            varied(&[("title.size", Some("title.size: big"))]),
            wrong("title.size", Expected::CardSize, "big"),
            "big",
        ),
        (
            varied(&[("title.size", Some("title.size: 631"))]),
            wrong("title.size", Expected::CardSize, "631"),
            "631",
        ),
        (
            varied(&[("title.size", Some("title.size: up to 0.4"))]),
            wrong("title.size", Expected::CardSize, "up to 0.4"),
            "up to 0.4",
        ),
        (
            varied(&[("title.size", Some("title.size: Up to 75"))]),
            Kind::WrongKind {
                field: "title.size".into(),
                expected: Expected::CardSize,
                found: Found::Written("Up to 75".into()),
                respelt: Some("up to 75".into()),
            },
            "Up to 75",
        ),
        (
            varied(&[("title.colour", Some("title.colour: red"))]),
            wrong("title.colour", Expected::Colour, "red"),
            "red",
        ),
        (
            varied(&[("title.colour", Some("title.colour: \"#F2E9D\""))]),
            wrong("title.colour", Expected::Colour, "\"#F2E9D\""),
            "\"#F2E9D\"",
        ),
        (
            varied(&[("title.colour", Some("title.colour: #F2E9D3"))]),
            Kind::NeedsQuotes {
                written: "#F2E9D3".into(),
                reading: "a comment".into(),
            },
            "#F2E9D3",
        ),
        (
            varied(&[("background", Some("background: ../background.png"))]),
            Kind::OutsideSite {
                reach: Reach::CardFile("../background.png".into()),
            },
            "../background.png",
        ),
        (
            varied(&[("font", Some("font: [text.ttf, /fonts/more.ttf]"))]),
            Kind::OutsideSite {
                reach: Reach::CardFile("/fonts/more.ttf".into()),
            },
            "/fonts/more.ttf",
        ),
        (
            varied(&[("background", Some("background: backgrund.png"))]),
            Kind::MissingFile {
                path: "templates/og/backgrund.png".into(),
                hint: Some(Hint::Closest("background.png".into())),
            },
            "backgrund.png",
        ),
        (
            varied(&[("font", Some("font: [text.ttf, other.otf]"))]),
            Kind::MissingFile {
                path: "templates/og/other.otf".into(),
                hint: None,
            },
            "other.otf",
        ),
        (
            varied(&[("font", Some("font: text.woff"))]),
            wrong("font", Expected::FontFile, "text.woff"),
            "text.woff",
        ),
        (
            varied(&[("footer", Some("footer:\n  text: x"))]),
            Kind::NestedSetting {
                key: "text".into(),
                under: "footer".into(),
                flat: "footer.text: x".into(),
            },
            "text",
        ),
        (
            varied(&[("padding", Some("padding: 315"))]),
            wrong(
                "padding",
                Expected::RoundedBetween { from: 0, to: 314 },
                "315",
            ),
            "315",
        ),
        (
            varied(&[("padding", Some("padding: 314.5"))]),
            wrong(
                "padding",
                Expected::RoundedBetween { from: 0, to: 314 },
                "314.5",
            ),
            "314.5",
        ),
        (
            varied(&[("padding", Some("padding: -10"))]),
            wrong(
                "padding",
                Expected::RoundedBetween { from: 0, to: 314 },
                "-10",
            ),
            "-10",
        ),
        (
            varied(&[("padding", Some("padding: wide"))]),
            wrong(
                "padding",
                Expected::RoundedBetween { from: 0, to: 314 },
                "wide",
            ),
            "wide",
        ),
        (
            varied(&[("padding", Some("padding: \"70\""))]),
            Kind::WrongKind {
                field: "padding".into(),
                expected: Expected::RoundedBetween { from: 0, to: 314 },
                found: Found::Written("\"70\"".into()),
                respelt: Some("70".into()),
            },
            "\"70\"",
        ),
        (
            varied(&[("padding", Some("padding: [70]"))]),
            Kind::WrongKind {
                field: "padding".into(),
                expected: Expected::RoundedBetween { from: 0, to: 314 },
                found: Found::List,
                respelt: None,
            },
            "[70]",
        ),
        (
            varied(&[("backgroud", Some("backgroud: background.png"))]),
            unread("backgroud", Some("background")),
            "backgroud",
        ),
        (
            varied(&[("title.colr", Some("title.colr: \"#FFF\""))]),
            unread("title.colr", Some("title.colour")),
            "title.colr",
        ),
        (
            varied(&[("title.colour", Some("title.color: \"#FFF\""))]),
            Kind::OtherSpelling {
                word: Word::Field,
                written: "title.color".into(),
                ours: "title.colour".into(),
            },
            "title.color",
        ),
        (
            varied(&[("title.place", Some("title.place: top"))]),
            Kind::SizeKept {
                field: "title.size".into(),
                written: "up to 75".into(),
                at: "top".into(),
                size: 75,
            },
            "up to 75",
        ),
    ];
    for (text, kind, written) in cases {
        let (sources, cards, problems) = read_site(&[("templates/og.yaml", &text)]);
        let [problem] = &problems[..] else {
            panic!("{text}{problems:#?}");
        };
        assert_eq!(problem.kind, kind, "{text}");
        let place = problem.place.unwrap();
        assert_eq!(sources.path(place.source), Path::new("templates/og.yaml"));
        assert_eq!(sources.spelling(place), written, "{text}");
        let warned = kind.severity() == Severity::Warning;
        assert_eq!(cards.card(None).is_some(), warned, "{text}");
    }
}

#[test]
fn a_whole_description_is_read_as_written() {
    let text = "\
background: background.png
font: [text.ttf, more.ttf]
padding: 70

title.text: \"{{ page.og_title }}\"
title.place: centre
title.size: up to 75
title.colour: \"#F2E9D3\"

footer.text: \"{{ site.title }}\"
footer.place: bottom right
footer.size: 36.4
footer.colour: \"{{ page.accent | default('#B9A88F') }}\"
";
    let (sources, cards, problems) = read_site(&[("templates/og.yaml", text)]);
    assert_eq!(problems, []);
    let card = cards.card(None).unwrap();
    let spelt = |origin: &Origin| sources.spelling(origin.place.unwrap()).to_owned();
    assert_eq!(card.background.value, "background.png");
    assert_eq!(spelt(&card.background.origin), "background.png");
    let fonts: Vec<(&str, String)> = card
        .fonts
        .iter()
        .map(|font| (font.value.as_str(), spelt(&font.origin)))
        .collect();
    assert_eq!(
        fonts,
        [
            ("text.ttf", "text.ttf".to_owned()),
            ("more.ttf", "more.ttf".to_owned())
        ]
    );
    assert_eq!(card.padding.value, 70);
    assert_eq!(spelt(&card.padding.origin), "70");
    let blocks: Vec<_> = card
        .blocks
        .iter()
        .map(|block| {
            (
                block.name.value.as_str(),
                spelt(&block.name.origin),
                block.text.value.as_str(),
                spelt(&block.text.origin),
                block.placement.value,
                block.size.value,
                spelt(&block.size.origin),
                block.colour.value.clone(),
            )
        })
        .collect();
    assert_eq!(
        blocks,
        [
            (
                "title",
                "title.text".to_owned(),
                "{{ page.og_title }}",
                "\"{{ page.og_title }}\"".to_owned(),
                Placement::Centre,
                Size::UpTo(75),
                "up to 75".to_owned(),
                Colour::Rgb([0xF2, 0xE9, 0xD3]),
            ),
            (
                "footer",
                "footer.text".to_owned(),
                "{{ site.title }}",
                "\"{{ site.title }}\"".to_owned(),
                Placement::BottomRight,
                Size::Fixed(36),
                "36.4".to_owned(),
                Colour::Template("{{ page.accent | default('#B9A88F') }}".into()),
            ),
        ]
    );
    let (_, cards, _) = read_site(&[(
        "templates/og.yaml",
        "background: background.png\nfont: text.ttf\n",
    )]);
    let card = cards.card(None).unwrap();
    assert_eq!(card.padding.value, 60);
    assert_eq!(card.padding.origin, Origin::defaulted());
    assert_eq!(card.blocks, []);
}

#[test]
fn padding_is_rounded_to_a_whole_number() {
    for (written, padding) in [("0", 0), ("12.5", 13), ("12.4", 12), ("314.4", 314)] {
        let text = varied(&[("padding", Some(&format!("padding: {written}")))]);
        let (_, cards, problems) = read_site(&[("templates/og.yaml", &text)]);
        assert_eq!(problems, [], "{written}");
        assert_eq!(cards.card(None).unwrap().padding.value, padding);
    }
}

#[test]
fn a_colour_may_be_three_figures() {
    assert_eq!(hex("#FfF"), Some([255, 255, 255]));
    assert_eq!(hex("#0a1"), Some([0x00, 0xAA, 0x11]));
    for text in ["FFF", "#FFFF", "#GGG", "#"] {
        assert_eq!(hex(text), None, "{text}");
    }
}

#[test]
fn blocks_are_drawn_in_the_order_first_written() {
    let text = "\
background: background.png
font: text.ttf
b.text: b
a.text: a
b.place: top
a.place: bottom
a.size: 20
b.size: 20
a.colour: \"#000\"
b.colour: \"#000\"
";
    let (_, cards, problems) = read_site(&[("templates/og.yaml", text)]);
    assert_eq!(problems, []);
    let names: Vec<&str> = cards
        .card(None)
        .unwrap()
        .blocks
        .iter()
        .map(|block| block.name.value.as_str())
        .collect();
    assert_eq!(names, ["b", "a"]);
}

#[test]
fn alternates_are_found_by_name() {
    let (_, cards, problems) = read_site(&[
        (
            "templates/og.yaml",
            "background: background.png\nfont: text.ttf\n",
        ),
        (
            "templates/og.wide.yaml",
            "background: background.png\nfont: more.ttf\n",
        ),
        ("templates/og.broken.yaml", "background: background.png\n"),
    ]);
    assert_eq!(problems.len(), 1);
    let font = |name| cards.card(name).map(|card| card.fonts[0].value.as_str());
    assert_eq!(font(None), Some("text.ttf"));
    assert_eq!(font(Some("wide")), Some("more.ttf"));
    assert_eq!(font(Some("broken")), None);
    assert_eq!(font(Some("narrow")), None);
    let named: Vec<(Option<&str>, &str)> = cards
        .described
        .iter()
        .map(|d| (d.name.as_deref(), d.path.as_str()))
        .collect();
    assert_eq!(
        named,
        [
            (Some("broken"), "templates/og.broken.yaml"),
            (Some("wide"), "templates/og.wide.yaml"),
            (None, "templates/og.yaml"),
        ]
    );
}

#[test]
fn a_description_is_name_value_lines() {
    let (_, cards, problems) = read_site(&[("templates/og.yaml", "- background.png\n")]);
    let kinds: Vec<&Kind> = problems.iter().map(|p| &p.kind).collect();
    assert_eq!(
        kinds,
        [&Kind::WrongKind {
            field: "templates/og.yaml".into(),
            expected: Expected::Fields,
            found: Found::List,
            respelt: None,
        }]
    );
    assert!(cards.card(None).is_none());
}

#[test]
fn a_site_with_no_description_has_no_cards() {
    let (_, cards, problems) = read_site(&[]);
    assert_eq!(problems, []);
    assert!(cards.described.is_empty());
}

#[test]
fn descriptions_and_their_files_are_known_by_name() {
    for (rel, described) in [
        ("og.yaml", Some(None)),
        ("og.wide.yaml", Some(Some("wide"))),
        ("og..yaml", None),
        ("og.a.b.yaml", None),
        ("og.yml", None),
        ("og/og.yaml", None),
        ("base.html", None),
    ] {
        assert_eq!(description(rel), described, "{rel}");
    }
    assert!(owns("og/background.png"));
    assert!(owns("og/OFL.txt"));
    assert!(!owns("ogle.html"));
}
