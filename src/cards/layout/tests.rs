use super::*;

/// Every character half its size wide, or as wide as its size in the
/// second list of fonts, and lines broken between words.
struct Half;

impl Measure for Half {
    fn measure(&self, text: &str, fonts: usize, size: u32, width: f32) -> Measured {
        let em = match fonts {
            0 => 0.5,
            _ => 1.0,
        };
        let wide = |line: &str| line.chars().count() as f32 * size as f32 * em;
        let mut lines: Vec<String> = Vec::new();
        for word in text.split(' ') {
            match lines.last_mut() {
                Some(line) if wide(&format!("{line} {word}")) <= width => {
                    line.push(' ');
                    line.push_str(word);
                }
                _ => lines.push(word.to_owned()),
            }
        }
        let width = lines.iter().map(|line| wide(line)).fold(0.0, f32::max);
        Measured { lines, width }
    }
}

fn text(placement: Placement, size: Size, built: &str) -> Text<'_> {
    Text {
        placement,
        size,
        built,
        fonts: 0,
    }
}

/// `count` words of one letter.
fn words(count: usize) -> String {
    vec!["a"; count].join(" ")
}

/// A line's left, top, width and height.
type LineBox = (f32, f32, f32, f32);

/// Each placed text's index and size, and each of its lines' box.
fn boxes(placed: &[Placed]) -> Vec<(usize, u32, Vec<LineBox>)> {
    placed
        .iter()
        .map(|p| {
            let lines = p.lines.iter().map(|l| (l.x, l.y, l.width, l.height));
            (p.text, p.size, lines.collect())
        })
        .collect()
}

#[test]
fn the_top_row_hangs_from_the_safe_area_and_lines_up_across_it() {
    use Placement::*;
    let texts = [
        text(TopLeft, Size::Fixed(20), "ab"),
        text(Top, Size::Fixed(20), "abcd"),
        text(TopRight, Size::Fixed(20), "ab"),
    ];
    let placed = lay_out(100, &texts, &Half).unwrap();
    assert_eq!(
        boxes(&placed),
        [
            (0, 20, vec![(100.0, 100.0, 20.0, 24.0)]),
            (1, 20, vec![(580.0, 100.0, 40.0, 24.0)]),
            (2, 20, vec![(1080.0, 100.0, 20.0, 24.0)]),
        ]
    );
}

#[test]
fn a_text_wraps_within_the_safe_area_and_each_line_is_centred() {
    // At 40, the 1080 pixels across hold 54 characters.
    let built = format!("{} {}", "a".repeat(40), "b".repeat(20));
    let texts = [text(Placement::Centre, Size::Fixed(40), &built)];
    let placed = lay_out(60, &texts, &Half).unwrap();
    let lines: Vec<&str> = placed[0].lines.iter().map(|l| l.text.as_str()).collect();
    assert_eq!(lines, ["a".repeat(40), "b".repeat(20)]);
    assert_eq!(
        boxes(&placed),
        [(
            0,
            40,
            vec![(200.0, 267.0, 800.0, 48.0), (400.0, 315.0, 400.0, 48.0)]
        )]
    );
    // 57 characters break at a padding of 60, and fit on one line with
    // none.
    let built = format!("{} {}", "a".repeat(28), "b".repeat(28));
    let texts = [text(Placement::Centre, Size::Fixed(40), &built)];
    assert_eq!(lay_out(60, &texts, &Half).unwrap()[0].lines.len(), 2);
    assert_eq!(lay_out(0, &texts, &Half).unwrap()[0].lines.len(), 1);
}

#[test]
fn each_text_is_measured_in_its_own_fonts() {
    let built = format!("{} {}", "a".repeat(20), "b".repeat(10));
    let mut texts = [
        text(Placement::Top, Size::Fixed(40), &built),
        text(Placement::Bottom, Size::Fixed(40), &built),
    ];
    texts[1].fonts = 1;
    let placed = lay_out(60, &texts, &Half).unwrap();
    assert_eq!(
        boxes(&placed),
        [
            (0, 40, vec![(290.0, 60.0, 620.0, 48.0)]),
            (
                1,
                40,
                vec![(200.0, 474.0, 800.0, 48.0), (400.0, 522.0, 400.0, 48.0)]
            ),
        ]
    );
}

#[test]
fn a_text_built_empty_takes_no_room() {
    use Placement::*;
    let texts = [
        text(Top, Size::Fixed(100), ""),
        text(Centre, Size::Fixed(40), ""),
        text(Centre, Size::Fixed(40), "abc"),
    ];
    let placed = lay_out(60, &texts, &Half).unwrap();
    assert_eq!(boxes(&placed), [(2, 40, vec![(570.0, 291.0, 60.0, 48.0)])]);
}

#[test]
fn texts_at_one_place_stack_half_the_lower_line_apart() {
    let texts = [
        text(Placement::Centre, Size::Fixed(50), "ab"),
        text(Placement::Centre, Size::Fixed(30), "cd"),
    ];
    let placed = lay_out(60, &texts, &Half).unwrap();
    // 60 tall, a gap of 18, then 36: 114 in all, centred in 510.
    assert_eq!(placed[0].lines[0].y, 258.0);
    assert_eq!(placed[1].lines[0].y, 336.0);
}

#[test]
fn the_middle_row_has_the_height_the_top_and_bottom_rows_leave() {
    use Placement::*;
    let texts = [
        text(TopLeft, Size::Fixed(20), "a"),
        text(TopLeft, Size::Fixed(20), "b"),
        text(TopRight, Size::UpTo(40), "c"),
        text(Bottom, Size::Fixed(30), "d"),
        text(Centre, Size::Fixed(50), "e"),
        text(Left, Size::Fixed(25), "f"),
    ];
    let placed = lay_out(60, &texts, &Half).unwrap();
    let ys: Vec<(usize, u32, f32)> = placed
        .iter()
        .map(|p| (p.text, p.size, p.lines[0].y))
        .collect();
    // The top row is 60 tall, its left stack's 24, 12 and 24, and the
    // bottom row 36, leaving 414 between them from 120.
    assert_eq!(
        ys,
        [
            (0, 20, 60.0),
            (1, 20, 96.0),
            (2, 40, 60.0),
            (3, 30, 534.0),
            (4, 50, 297.0),
            (5, 25, 312.0),
        ]
    );
}

#[test]
fn the_search_lands_on_the_largest_size_that_fits_exactly() {
    use Placement::*;
    let texts = [
        text(Left, Size::UpTo(60), "a"),
        text(Centre, Size::UpTo(600), "b"),
    ];
    let placed = lay_out(60, &texts, &Half).unwrap();
    // 1.2 times 425 is 510, the whole row; 426 would need 511.2.
    assert_eq!(
        boxes(&placed),
        [
            (0, 60, vec![(60.0, 279.0, 30.0, 72.0)]),
            (1, 425, vec![(493.75, 60.0, 212.5, 510.0)]),
        ]
    );
}

#[test]
fn the_search_stops_at_the_floor() {
    // 210 words take 9 lines of 54 at 45, and 10 of 55.2 at 46.
    let built = words(210);
    let texts = [text(Placement::Centre, Size::UpTo(80), &built)];
    let placed = lay_out(60, &texts, &Half).unwrap();
    assert_eq!(placed[0].size, FLOOR);
    assert_eq!(placed[0].lines.len(), 9);
}

#[test]
fn a_text_under_the_floor_is_drawn_at_its_size_outside_the_search() {
    use Placement::*;
    let texts = [
        text(Left, Size::UpTo(40), "a"),
        text(Centre, Size::UpTo(100), "b"),
    ];
    let placed = lay_out(60, &texts, &Half).unwrap();
    assert_eq!((placed[0].size, placed[1].size), (40, 100));
    // Too tall at 40, it is never tried smaller.
    let built = words(1000);
    let texts = [text(Left, Size::UpTo(40), &built)];
    assert!(matches!(
        lay_out(60, &texts, &Half),
        Err(Failure::TooTall {
            text: 0,
            size: 40,
            ..
        })
    ));
}

#[test]
fn rows_that_fill_the_safe_area_leave_no_room() {
    use Placement::*;
    let texts = [
        text(Top, Size::Fixed(300), "a"),
        text(Bottom, Size::UpTo(150), "b"),
    ];
    assert_eq!(
        lay_out(60, &texts, &Half),
        Err(Failure::NoRoom {
            top: 360.0,
            bottom: 180.0,
            safe: 510.0
        })
    );
    // Rows that exactly meet: no room for a text between them.
    let texts = [
        text(Top, Size::Fixed(250), "a"),
        text(Bottom, Size::Fixed(175), "b"),
        text(Centre, Size::Fixed(10), "c"),
    ];
    assert_eq!(
        lay_out(60, &texts, &Half),
        Err(Failure::NoRoom {
            top: 300.0,
            bottom: 210.0,
            safe: 510.0
        })
    );
    // With nothing between them, both are placed.
    let placed = lay_out(60, &texts[..2], &Half).unwrap();
    assert_eq!(placed[0].lines[0].y, 60.0);
    assert_eq!(placed[1].lines[0].y, 360.0);
}

#[test]
fn nothing_fitting_at_the_floor_names_the_first_text_that_doesnt() {
    use Placement::*;
    // 300 words take 13 lines of 54 at 45.
    let built = words(300);
    let texts = [
        text(Left, Size::Fixed(40), "a"),
        text(Centre, Size::UpTo(100), &built),
        text(Right, Size::UpTo(60), &built),
    ];
    assert_eq!(
        lay_out(60, &texts, &Half),
        Err(Failure::TooTall {
            text: 1,
            size: 45,
            needs: 702.0,
            room: 510.0
        })
    );
    // Lower in a stack, the text passing the row's height is named, with
    // the height down to its last line.
    let texts = [
        text(Centre, Size::Fixed(100), "a"),
        text(Centre, Size::Fixed(300), "b"),
    ];
    assert_eq!(
        lay_out(60, &texts, &Half),
        Err(Failure::TooTall {
            text: 1,
            size: 300,
            needs: 660.0,
            room: 510.0
        })
    );
}
