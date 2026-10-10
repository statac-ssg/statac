//! Writes a few finished sharing cards, drawn over gradients in the test
//! fonts, for a person to look at:
//! `cargo run --release --example card_samples -- <folder>`.

use std::path::PathBuf;
use std::time::Instant;

use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder, Rgb, RgbImage};
use statac::cards::draw::{Kit, Text};
use statac::cards::{HEIGHT, Placement, Size, WIDTH};

const LATIN: &[u8] = include_bytes!("../src/cards/fonts/test/test-latin.ttf");
const ARABIC: &[u8] = include_bytes!("../src/cards/fonts/test/test-arabic.ttf");
const JAPANESE: &[u8] = include_bytes!("../src/cards/fonts/test/test-japanese.ttf");
const KOREAN: &[u8] = include_bytes!("../src/cards/fonts/test/test-korean.ttf");
const EMOJI: &[u8] = include_bytes!("../starter/site/templates/og/NotoEmoji.ttf");

/// A diagonal gradient from one colour to another, as a PNG.
fn gradient(from: [u8; 3], to: [u8; 3]) -> Vec<u8> {
    let image = RgbImage::from_fn(WIDTH, HEIGHT, |x, y| {
        let t = (x + y) as f32 / (WIDTH + HEIGHT) as f32;
        Rgb(std::array::from_fn(|c| {
            (f32::from(from[c]) + (f32::from(to[c]) - f32::from(from[c])) * t) as u8
        }))
    });
    let mut out = Vec::new();
    PngEncoder::new(&mut out)
        .write_image(&image, WIDTH, HEIGHT, ExtendedColorType::Rgb8)
        .expect("a gradient encodes");
    out
}

fn text(placement: Placement, size: Size, built: &str, colour: [u8; 3]) -> Text<'_> {
    Text {
        placement,
        size,
        built,
        colour,
    }
}

fn main() {
    let Some(folder) = std::env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("usage: card_samples <folder>");
        std::process::exit(2);
    };
    std::fs::create_dir_all(&folder).expect("the folder can be made");

    let dark = gradient([24, 32, 64], [96, 40, 88]);
    let light = gradient([250, 236, 210], [200, 220, 240]);
    let white = [255, 255, 255];
    let ink = [30, 30, 40];
    let fonts: &[&[u8]] = &[LATIN, ARABIC, JAPANESE, KOREAN, EMOJI];
    let centre = |built, colour| text(Placement::Centre, Size::UpTo(120), built, colour);
    let long =
        "A longer title that wraps over more than one line and has to shrink to fit the card";
    let samples: Vec<(&str, &[u8], Vec<Text>)> = vec![
        ("short-latin", &dark, vec![centre("Title", white)]),
        ("long-latin", &dark, vec![centre(long, white)]),
        ("arabic", &dark, vec![centre("عنوان كتاب", white)]),
        ("japanese", &light, vec![centre("日本語の新聞と写真", ink)]),
        ("korean", &light, vec![centre("한국어 번역 글자", ink)]),
        ("mixed", &dark, vec![centre("Title 2024 😀", white)]),
        (
            "title-and-footer",
            &light,
            vec![
                centre("A longer title that wraps", ink),
                text(Placement::BottomLeft, Size::Fixed(32), "Title", ink),
            ],
        ),
    ];
    for (name, background, texts) in samples {
        let started = Instant::now();
        let kit = Kit::new(background, fonts, 60).expect("the kit is made");
        let card = kit.card(&texts).expect("the card is drawn");
        let file = format!(
            "{name}.{}.{}",
            kit.key(&texts).hash8(),
            card.format.extension()
        );
        std::fs::write(folder.join(&file), &card.bytes).expect("the card is written");
        println!(
            "{file}: {} bytes, {:.0} ms",
            card.bytes.len(),
            started.elapsed().as_secs_f64() * 1000.0
        );
    }
}
