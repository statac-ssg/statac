//! Encodes pictures drawn in code and reads the boxes of the files written.

use avif_aom_sys::{Error, Image, Layout, Settings, build_facts, encode, version};

const SETTINGS: Settings = Settings {
    speed: 10,
    quality: 60,
    alpha_quality: 60,
    threads: 1,
};

const ALPHA_URN: &[u8] = b"urn:mpeg:mpegB:cicp:systems:auxiliary:alpha";

/// Draws `width` by `height` pixels from a function of the position, with
/// alpha from `alpha` when given.
fn draw(
    width: u32,
    height: u32,
    colour: impl Fn(u32, u32) -> [u8; 3],
    alpha: Option<&dyn Fn(u32, u32) -> u8>,
) -> Vec<u8> {
    let mut pixels = Vec::new();
    for y in 0..height {
        for x in 0..width {
            pixels.extend_from_slice(&colour(x, y));
            if let Some(alpha) = alpha {
                pixels.push(alpha(x, y));
            }
        }
    }
    pixels
}

fn gradient(width: u32, height: u32) -> impl Fn(u32, u32) -> [u8; 3] {
    move |x, y| {
        let r = x * 255 / width.max(2).saturating_sub(1);
        let g = y * 255 / height.max(2).saturating_sub(1);
        [r as u8, g as u8, 128]
    }
}

fn bars(x: u32, _y: u32) -> [u8; 3] {
    const BARS: [[u8; 3]; 8] = [
        [235, 235, 235],
        [235, 235, 16],
        [16, 235, 235],
        [16, 235, 16],
        [235, 16, 235],
        [235, 16, 16],
        [16, 16, 235],
        [16, 16, 16],
    ];
    BARS[(x / 8 % 8) as usize]
}

fn zone_plate(width: u32, height: u32) -> impl Fn(u32, u32) -> [u8; 3] {
    move |x, y| {
        let dx = f64::from(x) - f64::from(width) / 2.0;
        let dy = f64::from(y) - f64::from(height) / 2.0;
        let v = (127.5 + 127.5 * ((dx * dx + dy * dy) / 40.0).cos()) as u8;
        [v, v, v]
    }
}

/// xorshift32, so the noise is the same on every run and machine.
fn noise(seed: u32) -> impl FnMut() -> u8 {
    let mut state = seed;
    move || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        (state >> 24) as u8
    }
}

struct Box<'a> {
    kind: [u8; 4],
    body: &'a [u8],
}

/// The boxes laid end to end in `bytes`.
fn boxes(mut bytes: &[u8]) -> Vec<Box<'_>> {
    let mut found = Vec::new();
    while bytes.len() >= 8 {
        let size = u32::from_be_bytes(bytes[0..4].try_into().unwrap()) as usize;
        let kind = bytes[4..8].try_into().unwrap();
        let (header, size) = match size {
            0 => (8, bytes.len()),
            1 => (
                16,
                u64::from_be_bytes(bytes[8..16].try_into().unwrap()) as usize,
            ),
            size => (8, size),
        };
        assert!(size >= header && size <= bytes.len(), "a truncated box");
        found.push(Box {
            kind,
            body: &bytes[header..size],
        });
        bytes = &bytes[size..];
    }
    assert!(bytes.is_empty(), "bytes after the last box");
    found
}

/// Every box under `meta/iprp/ipco` of the given kind.
fn properties<'a>(file: &'a [u8], kind: &[u8; 4]) -> Vec<&'a [u8]> {
    let child = |boxes: Vec<Box<'a>>, kind: &[u8; 4]| {
        boxes
            .into_iter()
            .find(|b| &b.kind == kind)
            .unwrap_or_else(|| panic!("no {} box", String::from_utf8_lossy(kind)))
            .body
    };
    // meta is a full box: a version and flags come before its children.
    let meta = &child(boxes(file), b"meta")[4..];
    let ipco = child(boxes(child(boxes(meta), b"iprp")), b"ipco");
    boxes(ipco)
        .into_iter()
        .filter(|b| &b.kind == kind)
        .map(|b| b.body)
        .collect()
}

fn major_brand(file: &[u8]) -> [u8; 4] {
    let ftyp = boxes(file).into_iter().next().expect("an empty file");
    assert_eq!(&ftyp.kind, b"ftyp");
    ftyp.body[0..4].try_into().unwrap()
}

/// Asserts that the file has an `ispe` property and that each gives this
/// width and height. libavif shares one between colour and alpha when they
/// match.
fn assert_size(file: &[u8], width: u32, height: u32) {
    let sizes = properties(file, b"ispe");
    assert!(!sizes.is_empty(), "no ispe property");
    for body in sizes {
        let word = |at: usize| u32::from_be_bytes(body[at..at + 4].try_into().unwrap());
        assert_eq!((word(4), word(8)), (width, height));
    }
}

fn has_alpha_item(file: &[u8]) -> bool {
    properties(file, b"auxC")
        .into_iter()
        .any(|body| body[4..].strip_suffix(&[0]) == Some(ALPHA_URN))
}

#[test]
fn rgb_is_an_avif_file_of_the_size_given() {
    let (width, height) = (97, 61);
    let pixels = draw(width, height, gradient(width, height), None);
    let image = Image {
        pixels: &pixels,
        width,
        height,
        layout: Layout::Rgb,
    };
    let file = encode(&image, &SETTINGS).unwrap();
    assert_eq!(&major_brand(&file), b"avif");
    assert_size(&file, width, height);
}

#[test]
fn rgba_writes_an_alpha_item_and_rgb_none() {
    let (width, height) = (64, 48);
    let fade = |x: u32, _y: u32| (x * 4) as u8;
    let rgba = draw(width, height, bars, Some(&fade));
    let file = encode(
        &Image {
            pixels: &rgba,
            width,
            height,
            layout: Layout::Rgba,
        },
        &SETTINGS,
    )
    .unwrap();
    assert!(has_alpha_item(&file));
    assert_size(&file, width, height);

    let rgb = draw(width, height, bars, None);
    let file = encode(
        &Image {
            pixels: &rgb,
            width,
            height,
            layout: Layout::Rgb,
        },
        &SETTINGS,
    )
    .unwrap();
    assert!(!has_alpha_item(&file));
}

#[test]
fn opaque_rgba_writes_no_alpha_item() {
    let rgba = draw(32, 32, bars, Some(&|_, _| 255));
    let image = Image {
        pixels: &rgba,
        width: 32,
        height: 32,
        layout: Layout::Rgba,
    };
    assert!(!has_alpha_item(&encode(&image, &SETTINGS).unwrap()));
}

#[test]
fn one_thread_gives_the_same_bytes_every_time() {
    let (width, height) = (120, 80);
    let plate = zone_plate(width, height);
    let mut grain = noise(0x5eed);
    let pixels: Vec<u8> = draw(width, height, plate, None)
        .into_iter()
        .map(|v| v.saturating_add(grain() / 8))
        .collect();
    let image = Image {
        pixels: &pixels,
        width,
        height,
        layout: Layout::Rgb,
    };
    let settings = Settings {
        speed: 6,
        ..SETTINGS
    };
    let first = encode(&image, &settings).unwrap();
    let second = encode(&image, &settings).unwrap();
    assert!(first == second, "two encodes differ");
}

fn refusal(pixels: &[u8], width: u32, height: u32, layout: Layout) -> Error {
    let image = Image {
        pixels,
        width,
        height,
        layout,
    };
    encode(&image, &SETTINGS).unwrap_err()
}

#[test]
fn a_zero_width_or_height_is_refused() {
    assert_eq!(refusal(&[], 0, 4, Layout::Rgb), Error::ZeroDimension);
    assert_eq!(refusal(&[], 4, 0, Layout::Rgba), Error::ZeroDimension);
}

#[test]
fn a_buffer_of_the_wrong_length_is_refused() {
    let pixels = vec![0; 4 * 4 * 3];
    assert_eq!(
        refusal(&pixels[1..], 4, 4, Layout::Rgb),
        Error::LengthMismatch {
            expected: 48,
            actual: 47
        }
    );
    assert_eq!(
        refusal(&pixels, 4, 4, Layout::Rgba),
        Error::LengthMismatch {
            expected: 64,
            actual: 48
        }
    );
}

#[test]
fn dimensions_whose_byte_count_overflows_are_refused() {
    assert_eq!(
        refusal(&[], u32::MAX, u32::MAX, Layout::Rgba),
        Error::TooLarge
    );
    // The row alone overflows the 32 bits libavif counts a row's bytes in.
    assert_eq!(
        refusal(&[], u32::MAX / 3 + 1, 1, Layout::Rgb),
        Error::TooLarge
    );
}

#[test]
fn settings_out_of_range_are_refused() {
    let pixels = draw(8, 8, bars, None);
    let image = Image {
        pixels: &pixels,
        width: 8,
        height: 8,
        layout: Layout::Rgb,
    };
    for (settings, name) in [
        (
            Settings {
                speed: 11,
                ..SETTINGS
            },
            "speed",
        ),
        (
            Settings {
                quality: 101,
                ..SETTINGS
            },
            "quality",
        ),
        (
            Settings {
                alpha_quality: 101,
                ..SETTINGS
            },
            "alpha quality",
        ),
        (
            Settings {
                threads: 0,
                ..SETTINGS
            },
            "threads",
        ),
    ] {
        match encode(&image, &settings) {
            Err(Error::InvalidSetting { name: refused, .. }) => assert_eq!(refused, name),
            other => panic!("{name}: {other:?}"),
        }
    }
}

#[test]
fn libavif_errors_carry_its_message() {
    // Wider than AV1 allows a frame to be.
    let width = 65537;
    let pixels = vec![0; width as usize * 3];
    let error = refusal(&pixels, width, 1, Layout::Rgb);
    let Error::Encoder(message) = error else {
        panic!("{error:?}");
    };
    assert!(message.contains("65537x1"), "{message}");
}

#[test]
fn version_names_libavif_and_libaom() {
    let version = version();
    assert!(version.starts_with("libavif 1.4.2 "), "{version}");
    assert!(
        version.contains("aom") && version.contains("v3.15.1"),
        "{version}"
    );
}

#[test]
fn libaom_was_built_for_release() {
    let facts = build_facts();
    assert_eq!(facts.build_type, "Release");
    assert!(facts.c_flags.contains("NDEBUG"), "{}", facts.c_flags);
}
