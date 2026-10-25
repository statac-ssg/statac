use image::{Rgb, Rgba, RgbaImage};
use moxcms::{ColorProfile, LocalizableString, ProfileText};

use super::*;
use crate::cards::Size as TextSize;
use crate::cards::layout::{Line, Placed, lay_out};

const LATIN: &[u8] = include_bytes!("../fonts/test/test-latin.ttf");
const LETTERS: &[u8] = include_bytes!("../fonts/test/test-latin-letters-only.ttf");
const ARABIC: &[u8] = include_bytes!("../fonts/test/test-arabic.ttf");
const JAPANESE: &[u8] = include_bytes!("../fonts/test/test-japanese.ttf");
const KOREAN: &[u8] = include_bytes!("../fonts/test/test-korean.ttf");
const EMOJI: &[u8] = include_bytes!("../fonts/test/test-emoji.ttf");

const INK: [u8; 3] = [250, 240, 200];

fn png_of(image: &RgbImage) -> Vec<u8> {
    encode_png(image, &Settings::default()).unwrap()
}

fn flat_image() -> RgbImage {
    RgbImage::from_pixel(WIDTH, HEIGHT, Rgb([20, 30, 40]))
}

fn flat() -> Vec<u8> {
    png_of(&flat_image())
}

fn gradient_image() -> RgbImage {
    RgbImage::from_fn(WIDTH, HEIGHT, |x, y| {
        Rgb([(x * 120 / WIDTH) as u8, (y * 100 / HEIGHT) as u8, 90])
    })
}

/// Seeded grain over every pixel, which no PNG compresses far.
fn noisy() -> Vec<u8> {
    let mut state = 0x5eed_u32;
    let image = RgbImage::from_fn(WIDTH, HEIGHT, |_, _| {
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            (state >> 24) as u8
        };
        Rgb([next(), next(), next()])
    });
    png_of(&image)
}

/// A kit whose one list of fonts is `files` in order.
fn one_list(background: &[u8], files: &[&[u8]], padding: u32) -> Result<Kit, KitFailure> {
    Kit::new(background, files, &[(0..files.len()).collect()], padding)
}

fn kit(fonts: &[&[u8]]) -> Kit {
    one_list(&flat(), fonts, 60).unwrap()
}

fn text(placement: Placement, size: TextSize, built: &str) -> Text<'_> {
    Text {
        placement,
        size,
        built,
        colour: INK,
        fonts: 0,
    }
}

fn lines(kit: &Kit, built: &str, size: u32, width: f32) -> Vec<String> {
    kit.measure(built, 0, size, width).lines
}

fn width(kit: &Kit, built: &str, size: u32) -> f32 {
    kit.measure(built, 0, size, f32::INFINITY).width
}

/// Asserts that each line measured alone comes back whole, with the width
/// it has among the others.
fn each_line_alone_is_whole(kit: &Kit, built: &str, size: u32, within: f32) {
    let measured = kit.measure(built, 0, size, within);
    for line in &measured.lines {
        let alone = kit.measure(line, 0, size, within);
        assert_eq!(alone.lines, std::slice::from_ref(line));
        assert!(alone.width <= within, "{line:?} is {} wide", alone.width);
        assert!(alone.width <= measured.width);
    }
}

#[test]
fn a_kit_is_shareable_between_threads() {
    fn shareable<T: Send + Sync>() {}
    shareable::<Kit>();
}

#[test]
fn a_background_that_does_not_read_is_refused_with_the_reason() {
    assert_eq!(
        one_list(b"not a picture", &[LATIN], 60).err(),
        Some(KitFailure::Background(pictures::Error::Unreadable(
            pictures::Unreadable::NotAPicture
        )))
    );
    let cut = &flat()[..200];
    assert!(matches!(
        one_list(cut, &[LATIN], 60).err(),
        Some(KitFailure::Background(pictures::Error::Unreadable(_)))
    ));
}

#[test]
fn a_background_of_another_size_is_refused_with_its_size() {
    let small = png_of(&RgbImage::from_pixel(1200, 600, Rgb([0, 0, 0])));
    assert_eq!(
        one_list(&small, &[LATIN], 60).err(),
        Some(KitFailure::BackgroundSize(pictures::Size {
            width: 1200,
            height: 600
        }))
    );
}

#[test]
fn a_font_that_does_not_read_is_named_by_its_place() {
    let junk = b"not a font at all, only words".as_slice();
    assert_eq!(
        one_list(&flat(), &[LATIN, junk, KOREAN], 60).err(),
        Some(KitFailure::Font(1))
    );
    assert_eq!(
        one_list(&flat(), &[&LATIN[..400]], 60).err(),
        Some(KitFailure::Font(0))
    );
}

fn display_p3() -> Vec<u8> {
    let mut profile = ColorProfile::new_display_p3();
    profile.description = Some(ProfileText::Localizable(vec![LocalizableString::new(
        "en".into(),
        "US".into(),
        "Display P3".into(),
    )]));
    profile.encode().unwrap()
}

fn close_to(pixel: [u8; 3], expected: [u8; 3], within: u8) -> bool {
    pixel
        .iter()
        .zip(expected)
        .all(|(a, b)| a.abs_diff(b) <= within)
}

#[test]
fn a_display_p3_background_is_converted_to_srgb() {
    let flat = RgbImage::from_pixel(WIDTH, HEIGHT, Rgb([128, 0, 199]));
    let mut jpeg = Vec::new();
    let encoder = jpeg_encoder::Encoder::new(&mut jpeg, 95);
    encoder
        .encode(&flat, 1200, 630, jpeg_encoder::ColorType::Rgb)
        .unwrap();
    let profile = display_p3();
    let mut segment = vec![0xff, 0xe2];
    segment.extend(((profile.len() + 16) as u16).to_be_bytes());
    segment.extend(b"ICC_PROFILE\0\x01\x01");
    segment.extend(&profile);
    jpeg.splice(2..2, segment);
    let kit = one_list(&jpeg, &[LATIN], 60).unwrap();
    let pixel = kit.background.get_pixel(600, 315).0;
    assert!(close_to(pixel, [141, 0, 207], 6), "{pixel:?}");

    let mut png = Vec::new();
    let mut encoder = PngEncoder::new(&mut png);
    encoder.set_icc_profile(profile).unwrap();
    encoder
        .write_image(&flat, WIDTH, HEIGHT, ExtendedColorType::Rgb8)
        .unwrap();
    let kit = one_list(&png, &[LATIN], 60).unwrap();
    assert!(
        kit.background
            .pixels()
            .all(|p| close_to(p.0, [141, 0, 207], 2))
    );
}

#[test]
fn a_background_with_alpha_is_drawn_over_black() {
    let image = RgbaImage::from_pixel(WIDTH, HEIGHT, Rgba([200, 100, 50, 128]));
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(&image, WIDTH, HEIGHT, ExtendedColorType::Rgba8)
        .unwrap();
    let kit = one_list(&png, &[LATIN], 60).unwrap();
    assert!(kit.background.pixels().all(|p| p.0 == [100, 50, 25]));
}

#[test]
fn a_sentence_wraps_at_spaces_within_the_width() {
    let kit = kit(&[LATIN]);
    let built = "A longer title that wraps over the lines";
    let measured = kit.measure(built, 0, 40, 300.0);
    assert!(measured.lines.len() > 1);
    assert_eq!(measured.lines.join(" "), built);
    for line in &measured.lines {
        assert!(width(&kit, line, 40) <= 300.0);
    }
    assert!(measured.width <= 300.0);
    // Each line is as long as it can be: its next word would not fit.
    for pair in measured.lines.windows(2) {
        let next = pair[1].split(' ').next().unwrap();
        assert!(width(&kit, &format!("{} {next}", pair[0]), 40) > 300.0);
    }
    each_line_alone_is_whole(&kit, built, 40, 300.0);
}

#[test]
fn a_word_wider_than_the_width_breaks_inside() {
    let kit = kit(&[LATIN]);
    let built = "Title Longerwordthanthewidthallows";
    let measured = kit.measure(built, 0, 40, 200.0);
    assert_eq!(measured.lines[0], "Title");
    assert!(measured.lines.len() > 2);
    assert_eq!(measured.lines[1..].concat(), "Longerwordthanthewidthallows");
    for line in &measured.lines {
        assert!(width(&kit, line, 40) <= 200.0);
    }
    each_line_alone_is_whole(&kit, built, 40, 200.0);
}

#[test]
fn a_line_measured_alone_comes_back_whole() {
    let kit = kit(&[LATIN, ARABIC, JAPANESE, KOREAN]);
    let texts = [
        ("A longer title that wraps", 300.0),
        ("Atitlewithoutanyspaces", 150.0),
        ("عنوان كتاب عنوان كتاب عنوان", 200.0),
        ("عنوانعنوانعنوانعنوان", 100.0),
        ("日本語の新聞と写真の時間", 150.0),
        ("한국어 번역 상위 활동 사용자", 150.0),
    ];
    for (built, within) in texts {
        let measured = kit.measure(built, 0, 40, within);
        assert!(measured.lines.len() > 1, "{built}");
        each_line_alone_is_whole(&kit, built, 40, within);
    }
}

#[test]
fn a_run_of_hangul_is_one_word() {
    let kit = kit(&[KOREAN]);
    // Wide enough for all but the last syllable: the text engine alone
    // would break between syllables.
    let within = width(&kit, "글자 한국", 40) + 1.0;
    assert!(width(&kit, "글자 한국어", 40) > within);
    assert_eq!(lines(&kit, "글자 한국어", 40, within), ["글자", "한국어"]);
    // A run wider than the width by itself still breaks.
    let narrow = width(&kit, "한국", 40) + 1.0;
    assert_eq!(lines(&kit, "한국어", 40, narrow), ["한국", "어"]);
}

#[test]
fn a_hangul_word_is_not_parted_where_its_script_changes() {
    let kit = kit(&[LATIN, KOREAN]);
    // Wide enough for the figures and no further: the text engine alone
    // would break between them and the syllables that follow.
    let within = width(&kit, "번역 100%", 40) + 1.0;
    assert!(width(&kit, "번역 100%서울", 40) > within);
    assert_eq!(
        lines(&kit, "번역 100%서울 글자", 40, within),
        ["번역", "100%서울", "글자"]
    );
    let within = width(&kit, "서울(Seoul)", 40) + 1.0;
    assert_eq!(
        lines(&kit, "번역 서울(Seoul) 글자", 40, within),
        ["번역", "서울(Seoul)", "글자"]
    );
}

#[test]
fn japanese_breaks_anywhere_but_leaves_no_character_alone() {
    let kit = kit(&[JAPANESE]);
    // Six characters fit; the seventh would sit alone.
    let within = width(&kit, "あいうえおか", 40) + 1.0;
    assert_eq!(
        lines(&kit, "あいうえおかき", 40, within),
        ["あいうえお", "かき"]
    );
    assert_eq!(
        lines(&kit, "あいうえおかきく", 40, within),
        ["あいうえおか", "きく"]
    );
    // With only two on the line before, one is left alone.
    let within = width(&kit, "あい", 40) + 1.0;
    assert_eq!(lines(&kit, "あいう", 40, within), ["あい", "う"]);
    each_line_alone_is_whole(&kit, "あいうえおかき", 40, within);
}

#[test]
fn presentation_selectors_are_removed() {
    let kit = kit(&[LATIN]);
    assert_eq!(
        lines(&kit, "Title\u{FE0F} and\u{FE0E}", 40, 1000.0),
        ["Title and"]
    );
    assert_eq!(width(&kit, "Title\u{FE0F}", 40), width(&kit, "Title", 40));
}

/// Which font file draws each visible character of `text`.
fn files(kit: &Kit, text: &str) -> Vec<(char, usize)> {
    let mut engine = kit.engine(0);
    let runs = engine.faces.runs(text);
    runs.iter()
        .flat_map(|run| {
            let file = engine.faces.faces[run.face].file;
            text[run.range.clone()].chars().map(move |c| (c, file))
        })
        .filter(|(c, _)| !c.is_whitespace())
        .collect()
}

#[test]
fn an_emoji_font_is_known_by_what_it_holds() {
    let known = |font: &[u8]| {
        let faces = fonts::load(&[font], &mut Database::new()).unwrap();
        faces.iter().all(|face| face.keycaps_only)
    };
    assert!(known(EMOJI));
    for font in [LATIN, LETTERS, ARABIC, JAPANESE, KOREAN] {
        assert!(!known(font));
    }
}

#[test]
fn a_bare_digit_passes_over_an_emoji_font_to_a_later_font() {
    let kit = kit(&[LETTERS, EMOJI, LATIN]);
    assert_eq!(
        files(&kit, "Title 2 #*"),
        [
            ('T', 0),
            ('i', 0),
            ('t', 0),
            ('l', 0),
            ('e', 0),
            ('2', 2),
            ('#', 0),
            ('*', 0)
        ]
    );
}

#[test]
fn digits_only_an_emoji_font_has_are_missing() {
    let kit = kit(&[LETTERS, EMOJI]);
    let texts = [
        text(Placement::Top, TextSize::Fixed(40), "Title 2024"),
        text(Placement::Bottom, TextSize::Fixed(40), "Title 3"),
    ];
    assert_eq!(
        kit.draw(&texts).err(),
        Some(Failure::Missing {
            text: 0,
            characters: vec!['2', '0', '4', '3']
        })
    );
}

#[test]
fn missing_characters_are_listed_once_each_in_order() {
    let kit = kit(&[LATIN]);
    let texts = [
        text(Placement::Top, TextSize::Fixed(40), "Title 日本 한국어"),
        text(Placement::Centre, TextSize::Fixed(40), "本 Title 日 عنوان"),
    ];
    let expected = vec!['日', '本', '한', '국', '어', 'ع', 'ن', 'و', 'ا'];
    assert_eq!(
        kit.draw(&texts).err(),
        Some(Failure::Missing {
            text: 0,
            characters: expected
        })
    );
}

#[test]
fn an_emoji_is_drawn_by_an_emoji_font_and_missing_without_one() {
    let texts = [
        text(
            Placement::Top,
            TextSize::Fixed(60),
            "Title 😀 and 😀\u{FE0F}",
        ),
        text(
            Placement::Bottom,
            TextSize::Fixed(60),
            "Title ❤\u{FE0F} 1\u{20E3}",
        ),
    ];
    let with = kit(&[LATIN, EMOJI]);
    assert_eq!(files(&with, "Title 😀")[5], ('😀', 1));
    // A keycap is drawn whole by the emoji font, its digit with it.
    assert_eq!(
        files(&with, "1\u{20E3} 1"),
        [('1', 1), ('\u{20E3}', 1), ('1', 0)]
    );
    assert!(with.draw(&texts).is_ok());
    assert_eq!(
        kit(&[LATIN]).draw(&texts).err(),
        Some(Failure::Missing {
            text: 0,
            characters: vec!['😀', '❤', '\u{20E3}']
        })
    );
}

#[test]
fn a_font_is_asked_whether_it_draws_characters() {
    assert!(draws_all(EMOJI, &['😀', '❤']));
    assert!(!draws_all(EMOJI, &['😀', 'a']));
    assert!(!draws_all(EMOJI, &['1']));
    assert!(draws_all(LATIN, &['a', '1']));
    assert!(!draws_all(b"not a font", &['a']));
}

#[test]
fn arabic_letters_join_when_shaped() {
    let kit = kit(&[ARABIC]);
    let word = "عنوان";
    let joined = width(&kit, word, 100);
    let apart: f32 = word.chars().map(|c| width(&kit, &c.to_string(), 100)).sum();
    assert!(joined < apart, "{joined} against {apart}");
}

#[test]
fn a_zero_width_non_joiner_changes_a_persian_word() {
    let kit = kit(&[ARABIC]);
    let with = width(&kit, "کتاب\u{200C}ها", 100);
    let without = width(&kit, "کتابها", 100);
    assert_ne!(with, without);
}

fn placed(kit: &Kit, texts: &[Text]) -> Vec<Placed> {
    let laid: Vec<layout::Text> = texts
        .iter()
        .map(|t| layout::Text {
            placement: t.placement,
            size: t.size,
            built: t.built,
            fonts: t.fonts,
        })
        .collect();
    lay_out(kit.padding, &laid, kit).unwrap()
}

/// Whether a pixel is within a line's box or the little way past it a
/// glyph may reach: a tenth of a line above and below, a quarter of an em
/// either side.
fn inside(line: &Line, x: u32, y: u32) -> bool {
    let (x, y) = (x as f32, y as f32);
    let (up, across) = (line.height * 0.1, line.height / 1.2 * 0.25);
    x + 1.0 > line.x - across
        && x < line.x + line.width + across
        && y + 1.0 > line.y - up
        && y < line.y + line.height + up
}

fn between(pixel: u8, under: u8, ink: u8) -> bool {
    under.min(ink) <= pixel && pixel <= under.max(ink)
}

#[test]
fn text_is_drawn_only_about_its_lines() {
    let kit = one_list(&png_of(&gradient_image()), &[LATIN, ARABIC], 60).unwrap();
    let texts = [
        text(Placement::TopLeft, TextSize::Fixed(40), "Title"),
        text(
            Placement::Centre,
            TextSize::UpTo(120),
            "A longer title that wraps",
        ),
        text(Placement::BottomRight, TextSize::Fixed(50), "عنوان كتاب"),
    ];
    let drawn = kit.draw(&texts).unwrap();
    assert_eq!(drawn.pixels.dimensions(), (WIDTH, HEIGHT));
    let lines: Vec<Line> = placed(&kit, &texts)
        .into_iter()
        .flat_map(|p| p.lines)
        .collect();
    assert_eq!(lines.len(), 4);
    let background = gradient_image();
    let mut inked = vec![0; lines.len()];
    for (x, y, pixel) in drawn.pixels.enumerate_pixels() {
        let under = background.get_pixel(x, y).0;
        match lines.iter().position(|line| inside(line, x, y)) {
            None => assert_eq!(pixel.0, under, "at {x}, {y}"),
            Some(at) => {
                let ok = (0..3).all(|c| between(pixel.0[c], under[c], INK[c]));
                assert!(ok, "{:?} over {under:?} at {x}, {y}", pixel.0);
                if pixel.0 == INK {
                    inked[at] += 1;
                }
            }
        }
    }
    assert!(inked.iter().all(|&n| n > 0), "{inked:?}");
}

#[test]
fn right_to_left_text_placed_left_starts_at_the_safe_areas_left_edge() {
    let kit = kit(&[ARABIC]);
    let texts = [text(Placement::Left, TextSize::Fixed(80), "عنوان كتاب")];
    let line = placed(&kit, &texts)[0].lines[0].clone();
    assert_eq!(line.x, 60.0);
    let drawn = kit.draw(&texts).unwrap();
    let under = flat_image().get_pixel(0, 0).0;
    let rows = line.y as u32..(line.y + line.height) as u32;
    let first = (0..WIDTH)
        .find(|&x| {
            rows.clone()
                .any(|y| drawn.pixels.get_pixel(x, y).0 != under)
        })
        .unwrap();
    assert!((60..68).contains(&first), "{first}");
}

#[test]
fn the_same_kit_and_texts_draw_the_same_pixels() {
    let kit = kit(&[LATIN, JAPANESE]);
    let texts = [
        text(
            Placement::Centre,
            TextSize::UpTo(100),
            "A longer title that wraps",
        ),
        text(Placement::Bottom, TextSize::Fixed(40), "日本語の新聞"),
    ];
    let first = kit.draw(&texts).unwrap().pixels;
    let other = kit.draw(&[text(Placement::Top, TextSize::Fixed(30), "Title")]);
    assert!(other.is_ok());
    assert!(first == kit.draw(&texts).unwrap().pixels);
    assert!(
        first
            == one_list(&flat(), &[LATIN, JAPANESE], 60)
                .unwrap()
                .draw(&texts)
                .unwrap()
                .pixels
    );
}

#[test]
fn a_layout_failure_is_passed_on() {
    let kit = kit(&[LATIN]);
    let texts = [
        text(Placement::Top, TextSize::Fixed(300), "Title"),
        text(Placement::Bottom, TextSize::Fixed(150), "Title"),
    ];
    assert_eq!(
        kit.draw(&texts).err(),
        Some(Failure::Layout(layout::Failure::NoRoom {
            top: 360.0,
            bottom: 180.0,
            safe: 510.0
        }))
    );
}

/// The names of a PNG's chunks.
fn chunks(png: &[u8]) -> Vec<String> {
    let mut at = 8;
    let mut names = Vec::new();
    while at + 8 <= png.len() {
        let length = u32::from_be_bytes(png[at..at + 4].try_into().unwrap()) as usize;
        names.push(String::from_utf8_lossy(&png[at + 4..at + 8]).into_owned());
        at += 12 + length;
    }
    names
}

/// The markers of a JPEG's segments before its first scan.
fn markers(jpeg: &[u8]) -> Vec<u8> {
    let mut at = 2;
    let mut markers = Vec::new();
    while jpeg[at] == 0xff && jpeg[at + 1] != 0xda {
        markers.push(jpeg[at + 1]);
        at += 2 + u16::from_be_bytes([jpeg[at + 2], jpeg[at + 3]]) as usize;
    }
    markers
}

#[test]
fn a_flat_background_gives_pngs_and_a_noisy_one_jpegs() {
    let texts = [
        text(Placement::Centre, TextSize::UpTo(100), "Title"),
        text(
            Placement::Bottom,
            TextSize::Fixed(40),
            "A longer title that wraps",
        ),
    ];
    let kit = kit(&[LATIN]);
    assert_eq!(kit.format(), Format::Png);
    for texts in [&texts[..1], &texts] {
        let card = kit.card(texts).unwrap();
        assert_eq!(card.format, Format::Png);
        let decoded = image::load_from_memory(&card.bytes).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (WIDTH, HEIGHT));
        assert_eq!(decoded.color(), image::ColorType::Rgb8);
        assert_eq!(chunks(&card.bytes), ["IHDR", "IDAT", "IEND"]);
    }

    let kit = one_list(&noisy(), &[LATIN], 60).unwrap();
    assert_eq!(kit.format(), Format::Jpeg);
    for texts in [&texts[..1], &texts] {
        let card = kit.card(texts).unwrap();
        assert_eq!(card.format, Format::Jpeg);
        let decoded = image::load_from_memory(&card.bytes).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (WIDTH, HEIGHT));
        // The JFIF header and no other application segment or comment.
        let markers = markers(&card.bytes);
        assert_eq!(markers[0], 0xe0);
        assert!(
            markers[1..]
                .iter()
                .all(|&m| !(0xe0..=0xef).contains(&m) && m != 0xfe),
            "{markers:x?}"
        );
    }
}

#[test]
fn every_ingredient_changes_the_key() {
    let background = flat();
    let other_background = png_of(&RgbImage::from_pixel(WIDTH, HEIGHT, Rgb([20, 30, 41])));
    let make = |background: &[u8], files: &[&[u8]], padding| {
        let lists = [(0..files.len()).collect()];
        Kit::make(background, files, &lists, padding, Settings::default()).unwrap()
    };
    let kit = make(&background, &[LATIN, KOREAN, EMOJI], 60);
    let texts = [
        text(Placement::Top, TextSize::Fixed(40), "Title"),
        text(Placement::Centre, TextSize::UpTo(80), "A longer title"),
    ];
    let key = kit.key(&texts);
    assert_eq!(key, kit.key(&texts));
    assert_eq!(key.hex().len(), 64);
    assert_eq!(key.hash8(), key.hex()[..8]);
    let mut keys = vec![key];
    let kits = [
        make(&other_background, &[LATIN, KOREAN, EMOJI], 60),
        make(&background, &[LETTERS, KOREAN, EMOJI], 60),
        make(&background, &[LATIN, JAPANESE, EMOJI], 60),
        make(&background, &[KOREAN, LATIN, EMOJI], 60),
        make(&background, &[LATIN, KOREAN, ARABIC], 60),
        make(&background, &[LATIN, KOREAN, EMOJI], 61),
    ];
    keys.extend(kits.iter().map(|kit| kit.key(&texts)));
    let changed: [&dyn Fn(&mut Text); 7] = [
        &|t| t.built = "Titles",
        &|t| t.placement = Placement::Bottom,
        &|t| t.size = TextSize::Fixed(41),
        &|t| t.size = TextSize::UpTo(40),
        &|t| t.colour = [250, 240, 201],
        &|t| t.colour = [251, 240, 200],
        &|t| t.colour = [250, 241, 200],
    ];
    for change in changed {
        for at in 0..texts.len() {
            let mut texts = texts;
            change(&mut texts[at]);
            keys.push(kit.key(&texts));
        }
    }
    keys.push(kit.key(&texts[..1]));
    keys.push(kit.key(&[texts[1], texts[0]]));

    let mut kit = kit;
    kit.format = Format::Jpeg;
    keys.push(kit.key(&texts));
    kit.format = Format::Png;
    let settings: [&dyn Fn(&mut Settings); 7] = [
        &|s| s.drawing += 1,
        &|s| s.png_compression = CompressionType::Default,
        &|s| s.png_filter = FilterType::Paeth,
        &|s| s.jpeg_quality = 89,
        &|s| s.jpeg_subsampling = SamplingFactor::R_4_2_0,
        &|s| s.jpeg_progressive = false,
        &|s| s.jpeg_optimised_huffman = false,
    ];
    for change in settings {
        change(&mut kit.settings);
        keys.push(kit.key(&texts));
        kit.settings = Settings::default();
    }
    assert_eq!(kit.key(&texts), key);

    let mut unique = keys.clone();
    unique.sort_by_key(Key::hex);
    unique.dedup();
    assert_eq!(unique.len(), keys.len());
}

#[test]
fn a_card_drawn_in_the_cards_own_fonts_keeps_its_key() {
    // A key names a card kept and published, so a card whose texts have
    // no fonts of their own keeps the key it has always had.
    let kit = kit(&[LATIN, EMOJI]);
    let texts = [
        text(Placement::Centre, TextSize::UpTo(75), "A longer title"),
        text(Placement::Bottom, TextSize::Fixed(36), "Title"),
    ];
    assert_eq!(
        kit.key(&texts).hex(),
        "13aa36400a3ef5b82fb61b9f29e5dab50f1640bda91fdcab73c5c9398c500964"
    );
    let other = Kit::new(&flat(), &[LATIN, EMOJI, KOREAN], &[vec![0, 1], vec![2]], 60);
    assert_eq!(other.unwrap().key(&texts), kit.key(&texts));
}

#[test]
fn a_texts_own_fonts_are_in_the_key() {
    let make = |files: &[&[u8]], lists: &[Vec<usize>]| Kit::new(&flat(), files, lists, 60).unwrap();
    let mut texts = [
        text(Placement::Top, TextSize::Fixed(40), "Title"),
        text(Placement::Centre, TextSize::UpTo(80), "A longer title"),
    ];
    texts[1].fonts = 1;
    let key = make(&[LATIN, KOREAN], &[vec![0], vec![1]]).key(&texts);
    // A file no list names is no part of it.
    let unnamed = make(&[LATIN, JAPANESE, KOREAN], &[vec![0], vec![2]]);
    assert_eq!(unnamed.key(&texts), key);
    let mut keys = vec![key];
    let kits = [
        make(&[LATIN, JAPANESE], &[vec![0], vec![1]]),
        make(&[LATIN, KOREAN], &[vec![0], vec![1, 0]]),
        make(&[LATIN, KOREAN], &[vec![0], vec![0, 1]]),
        make(&[LETTERS, KOREAN], &[vec![0], vec![1]]),
    ];
    keys.extend(kits.iter().map(|kit| kit.key(&texts)));
    let kit = make(&[LATIN, KOREAN], &[vec![0], vec![1]]);
    for fonts in [[0, 0], [1, 1], [1, 0]] {
        texts[0].fonts = fonts[0];
        texts[1].fonts = fonts[1];
        keys.push(kit.key(&texts));
    }

    let mut unique = keys.clone();
    unique.sort_by_key(Key::hex);
    unique.dedup();
    assert_eq!(unique.len(), keys.len());
}

#[test]
fn each_text_is_drawn_in_its_own_fonts() {
    // Either order draws each character in the same font, but a line sits
    // by the first font of its list.
    let (title, footer) = ("日本 Title", "Title 日本");
    let mut own = text(Placement::Top, TextSize::Fixed(60), title);
    own.fonts = 1;
    let texts = [own, text(Placement::Bottom, TextSize::Fixed(60), footer)];
    let both = Kit::new(&flat(), &[LATIN, JAPANESE], &[vec![0, 1], vec![1, 0]], 60)
        .unwrap()
        .draw(&texts)
        .unwrap()
        .pixels;
    let alone = |fonts: &[&[u8]], top: &str, bottom: &str| {
        let texts = [
            text(Placement::Top, TextSize::Fixed(60), top),
            text(Placement::Bottom, TextSize::Fixed(60), bottom),
        ];
        kit(fonts).draw(&texts).unwrap().pixels
    };
    let half = (WIDTH * HEIGHT / 2 * 3) as usize;
    let (top, bottom) = both.as_raw().split_at(half);
    let title_alone = alone(&[JAPANESE, LATIN], title, "");
    assert!(top == &title_alone.as_raw()[..half]);
    assert!(bottom == &alone(&[LATIN, JAPANESE], "", footer).as_raw()[half..]);
    assert!(top != &alone(&[LATIN, JAPANESE], title, "").as_raw()[..half]);
}

#[test]
fn characters_are_missing_from_the_fonts_of_the_text_needing_them() {
    // The card's fonts have no emoji, and the second list has them.
    let kit = Kit::new(&flat(), &[LATIN, EMOJI], &[vec![0], vec![0, 1]], 60).unwrap();
    let lacking = |text, characters: &str| {
        let characters = characters.chars().collect();
        Some(Failure::Missing { text, characters })
    };
    let cases = [
        ("Title 😀", 1, "Title", 0, None),
        ("Title 😀", 0, "Title", 1, lacking(0, "😀")),
        ("Title", 1, "Title 😀 ❤", 0, lacking(1, "😀❤")),
        ("Title 😀", 0, "Title ❤", 0, lacking(0, "😀❤")),
        ("Title 日", 1, "Title 本", 1, lacking(0, "日本")),
        ("Title 日", 1, "Title 😀", 0, lacking(0, "日")),
    ];
    for (first, first_fonts, second, second_fonts, missing) in cases {
        let mut texts = [
            text(Placement::Top, TextSize::Fixed(40), first),
            text(Placement::Bottom, TextSize::Fixed(40), second),
        ];
        texts[0].fonts = first_fonts;
        texts[1].fonts = second_fonts;
        assert_eq!(kit.draw(&texts).err(), missing, "{first}, {second}");
    }
}

#[test]
fn a_font_file_in_several_lists_is_read_once() {
    let kit = Kit::new(&flat(), &[LATIN, EMOJI], &[vec![0], vec![1, 0]], 60).unwrap();
    assert_eq!(kit.fonts.len(), 2);
    assert_eq!(kit.lists[1].faces[1].id, kit.lists[0].faces[0].id);
}

#[test]
fn the_settings_are_as_documented() {
    let settings = Settings::default();
    assert_eq!(settings.drawing, DRAWING);
    assert_eq!(settings.png_compression, CompressionType::Best);
    assert_eq!(settings.jpeg_quality, 90);
    assert_eq!(settings.jpeg_subsampling, SamplingFactor::R_4_4_4);
    assert!(settings.jpeg_progressive && settings.jpeg_optimised_huffman);
}
