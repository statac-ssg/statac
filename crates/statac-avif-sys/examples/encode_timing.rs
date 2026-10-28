//! Times encoding a 1200 by 800 picture drawn in code, as RGB at speed 8,
//! quality 67, on one thread:
//! `cargo run --release -p statac-avif-sys --example encode_timing`.

use std::time::Instant;

use statac_avif_sys::{Image, Layout, Settings, build_facts, encode, version};

const WIDTH: u32 = 1200;
const HEIGHT: u32 = 800;
const RUNS: usize = 7;

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
            let ring = 0.5 + 0.5 * ((dx * dx + dy * dy) / 400.0).cos();
            let fade = (1.0 - (dx * dx + dy * dy).sqrt() / 300.0).max(0.0);
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
    let pixels = picture();
    let image = Image {
        pixels: &pixels,
        width: WIDTH,
        height: HEIGHT,
        layout: Layout::Rgb,
    };
    let settings = Settings {
        speed: 8,
        quality: 67,
        alpha_quality: 67,
        threads: 1,
    };
    let mut times = Vec::with_capacity(RUNS);
    let mut size = 0;
    for _ in 0..RUNS {
        let start = Instant::now();
        let file = encode(&image, &settings).expect("an encoded picture");
        times.push(start.elapsed().as_secs_f64() * 1000.0);
        size = file.len();
    }
    times.sort_by(f64::total_cmp);
    println!("{WIDTH}x{HEIGHT} RGB, speed 8, quality 67, 1 thread, {RUNS} runs");
    println!(
        "median {:.1} ms, min {:.1} ms, max {:.1} ms",
        times[RUNS / 2],
        times[0],
        times[RUNS - 1]
    );
    println!("{size} bytes");
    println!("{}", version());
    let facts = build_facts();
    println!("libaom build type: {}", facts.build_type);
    println!("libaom C flags: {}", facts.c_flags);
}
