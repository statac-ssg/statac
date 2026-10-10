//! Writes the pictures `statac new` puts in a site: the first post's
//! photograph, the plain background of the sharing card and the icon:
//! `cargo run --example starter_pictures`. The same bytes come out each
//! time.

use std::path::Path;

use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{ExtendedColorType, ImageEncoder, Rgb, RgbImage, Rgba, RgbaImage};

/// A diagonal gradient from one colour to another.
fn gradient(width: u32, height: u32, from: [u8; 3], to: [u8; 3]) -> RgbImage {
    RgbImage::from_fn(width, height, |x, y| {
        let t = (x + y) as f32 / (width + height) as f32;
        Rgb(std::array::from_fn(|c| {
            (f32::from(from[c]) + (f32::from(to[c]) - f32::from(from[c])) * t).round() as u8
        }))
    })
}

/// A square of one colour with rounded corners, clear outside them. Each
/// pixel is covered by as much of the shape as lies over it, sampled.
fn rounded_square(side: u32, radius: f32, colour: [u8; 3]) -> RgbaImage {
    const SAMPLES: u32 = 8;
    let (side_f, [r, g, b]) = (side as f32, colour);
    RgbaImage::from_fn(side, side, |x, y| {
        let mut inside = 0;
        for sample in 0..SAMPLES * SAMPLES {
            let px = x as f32 + ((sample % SAMPLES) as f32 + 0.5) / SAMPLES as f32;
            let py = y as f32 + ((sample / SAMPLES) as f32 + 0.5) / SAMPLES as f32;
            // The distance past the square whose corners are the corners'
            // centres.
            let dx = (radius - px).max(px - (side_f - radius)).max(0.0);
            let dy = (radius - py).max(py - (side_f - radius)).max(0.0);
            inside += u32::from(dx * dx + dy * dy <= radius * radius);
        }
        let alpha = (inside * 255 + SAMPLES * SAMPLES / 2) / (SAMPLES * SAMPLES);
        Rgba([r, g, b, alpha as u8])
    })
}

/// An icon file holding one picture, as a PNG.
fn icon(picture: &RgbaImage) -> Vec<u8> {
    let (width, height) = picture.dimensions();
    let mut png = Vec::new();
    PngEncoder::new_with_quality(&mut png, CompressionType::Best, FilterType::Adaptive)
        .write_image(picture, width, height, ExtendedColorType::Rgba8)
        .expect("the icon encodes");
    // The file's header, then the one picture's entry: its size, one
    // plane of 32 bits a pixel, the PNG's length and where it starts.
    let mut file = vec![0, 0, 1, 0, 1, 0];
    file.extend([width as u8, height as u8, 0, 0, 1, 0, 32, 0]);
    file.extend((png.len() as u32).to_le_bytes());
    file.extend(22u32.to_le_bytes());
    file.extend(png);
    file
}

fn main() {
    let starter = Path::new(env!("CARGO_MANIFEST_DIR")).join("starter/site");

    let photo = gradient(800, 533, [40, 72, 120], [232, 160, 96]);
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, 80)
        .write_image(&photo, 800, 533, ExtendedColorType::Rgb8)
        .expect("a gradient encodes");
    assert!(
        jpeg.len() < 20_000,
        "the photograph is {} bytes",
        jpeg.len()
    );
    let path = starter.join("content/posts/first-post/photo-1.jpg");
    std::fs::write(&path, &jpeg).expect("the photograph is written");

    let background = RgbImage::from_pixel(1200, 630, Rgb([244, 242, 238]));
    let mut png = Vec::new();
    PngEncoder::new_with_quality(&mut png, CompressionType::Best, FilterType::Adaptive)
        .write_image(&background, 1200, 630, ExtendedColorType::Rgb8)
        .expect("the background encodes");
    let path = starter.join("templates/og/background.png");
    std::fs::write(&path, &png).expect("the background is written");

    // The stylesheet's accent, `oklch(90% 0.17 98)`, in sRGB.
    let square = rounded_square(32, 7.0, [252, 222, 61]);
    let path = starter.join("static/favicon.ico");
    std::fs::create_dir_all(path.parent().expect("the icon is in a folder"))
        .expect("the folder is made");
    std::fs::write(&path, icon(&square)).expect("the icon is written");
}
