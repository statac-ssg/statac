//! Times opening a 4000 by 3000 JPEG drawn in code and making each copy of
//! its default widths in each format:
//! `cargo run --release --example picture_timing`.

use std::time::Instant;

use statac::pictures::{self, Header, Settings};

const WIDTH: u32 = 4000;
const HEIGHT: u32 = 3000;

/// A gradient under a zone plate in the middle, with seeded noise for grain.
fn picture() -> Vec<u8> {
    let mut state = 0x5eed_u32;
    let mut grain = move || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        (state >> 27) as u8
    };
    let mut pixels = Vec::with_capacity((WIDTH * HEIGHT * 3) as usize);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let dx = f64::from(x) - f64::from(WIDTH) / 2.0;
            let dy = f64::from(y) - f64::from(HEIGHT) / 2.0;
            let ring = 0.5 + 0.5 * ((dx * dx + dy * dy) / 4000.0).cos();
            let fade = (1.0 - (dx * dx + dy * dy).sqrt() / 1000.0).max(0.0);
            let mix = |base: u32| {
                let base = f64::from(base);
                (base * (1.0 - fade) + 255.0 * ring * fade) as u8
            };
            pixels.push(mix(x * 255 / WIDTH).saturating_add(grain()));
            pixels.push(mix(y * 255 / HEIGHT).saturating_add(grain()));
            pixels.push(mix(160).saturating_add(grain()));
        }
    }
    pixels
}

fn main() {
    let start = Instant::now();
    let pixels = picture();
    let mut jpeg = Vec::new();
    jpeg_encoder::Encoder::new(&mut jpeg, 92)
        .encode(
            &pixels,
            WIDTH as u16,
            HEIGHT as u16,
            jpeg_encoder::ColorType::Rgb,
        )
        .expect("an encoded picture");
    let ms = |start: Instant| start.elapsed().as_secs_f64() * 1000.0;
    println!(
        "draw and encode {WIDTH}x{HEIGHT} JPEG: {:.0} ms, {} bytes",
        ms(start),
        jpeg.len()
    );

    let source = blake3::hash(&jpeg);
    let start = Instant::now();
    let Ok(Header::Photograph(photograph)) = pictures::read_header(&jpeg) else {
        panic!("the JPEG reads as a photograph");
    };
    let settings = Settings::default();
    let transparent = pictures::transparent(&jpeg);
    let ladder = pictures::ladder(
        &photograph,
        transparent,
        "picture",
        &source,
        u32::MAX,
        &settings,
    );
    println!(
        "read header and name {} copies: {:.2} ms",
        ladder.len(),
        ms(start)
    );

    let start = Instant::now();
    let opened = pictures::open(&jpeg).expect("an opened photograph");
    println!("open: {:.0} ms", ms(start));

    let all = Instant::now();
    let mut resized = None;
    for copy in &ladder {
        // One resize serves every format at a width.
        if !matches!(&resized, Some((width, _)) if *width == copy.width) {
            let start = Instant::now();
            let at_width = opened.resize(copy.width, &settings).expect("a resize");
            println!(
                "{:>4}x{:<4} resize: {:>3.0} ms",
                copy.width,
                copy.height,
                ms(start)
            );
            resized = Some((copy.width, at_width));
        }
        let Some((_, at_width)) = &resized else {
            unreachable!("a resize was just made");
        };
        let start = Instant::now();
        let bytes = at_width.encode(copy.format, &settings).expect("a copy");
        println!(
            "{:>4}x{:<4} {:<6}: {:>3.0} ms, {:>8} bytes",
            copy.width,
            copy.height,
            copy.format.extension(),
            ms(start),
            bytes.len()
        );
    }
    println!("all copies, one after another: {:.0} ms", ms(all));
}
