use image::codecs::webp::WebPEncoder;
use image::imageops::{self, FilterType};
use image::{ExtendedColorType, ImageEncoder, Rgb, RgbImage, Rgba, RgbaImage};
use moxcms::{ColorProfile, LocalizableString, ProfileText};

use super::*;

/// A seeded xorshift, for grain and random bytes that are the same on
/// every run.
struct Noise(u32);

impl Noise {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }

    fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| (self.next() >> 24) as u8).collect()
    }
}

/// Colour bars across the top third, a gradient below with a zone plate in
/// its middle, and seeded grain throughout.
fn picture(width: u32, height: u32) -> RgbImage {
    const BARS: [[u8; 3]; 7] = [
        [235, 235, 235],
        [235, 235, 16],
        [16, 235, 235],
        [16, 235, 16],
        [235, 16, 235],
        [235, 16, 16],
        [16, 16, 235],
    ];
    let mut noise = Noise(0x5eed);
    RgbImage::from_fn(width, height, |x, y| {
        let grain = (noise.next() >> 29) as u8;
        if y < height / 3 {
            let [r, g, b] = BARS[(x * 7 / width) as usize];
            return Rgb([r.saturating_add(grain), g.saturating_add(grain), b]);
        }
        let dx = f64::from(x) - f64::from(width) / 2.0;
        let dy = f64::from(y) - f64::from(height) * 2.0 / 3.0;
        let r2 = dx * dx + dy * dy;
        let fade = (1.0 - r2.sqrt() / (f64::from(height) / 4.0)).max(0.0);
        let ring = 127.5 + 127.5 * (r2 / 600.0).cos();
        let mix = |base: f64| (base * (1.0 - fade) + ring * fade) as u8;
        Rgb([
            mix(f64::from(x * 255 / width)).saturating_add(grain),
            mix(f64::from(y * 255 / height)).saturating_add(grain),
            mix(160.0),
        ])
    })
}

fn encode_jpeg(image: &RgbImage) -> Vec<u8> {
    let mut out = Vec::new();
    let encoder = jpeg_encoder::Encoder::new(&mut out, 95);
    let (width, height) = (image.width() as u16, image.height() as u16);
    encoder
        .encode(image, width, height, jpeg_encoder::ColorType::Rgb)
        .unwrap();
    out
}

/// `jpeg` with a segment of `marker` holding `body` placed after its start
/// of image.
fn splice(jpeg: &[u8], marker: u8, body: &[u8]) -> Vec<u8> {
    let length = u16::try_from(body.len() + 2).unwrap().to_be_bytes();
    let mut out = jpeg[..2].to_vec();
    out.extend([0xff, marker, length[0], length[1]]);
    out.extend(body);
    out.extend(&jpeg[2..]);
    out
}

/// An EXIF APP1 body giving only `orientation`.
fn exif(orientation: u16, little_endian: bool) -> Vec<u8> {
    let u16b = |v: u16| {
        if little_endian {
            v.to_le_bytes()
        } else {
            v.to_be_bytes()
        }
    };
    let u32b = |v: u32| {
        if little_endian {
            v.to_le_bytes()
        } else {
            v.to_be_bytes()
        }
    };
    let mut out = b"Exif\0\0".to_vec();
    out.extend(if little_endian { b"II*\0" } else { b"MM\0*" });
    out.extend(u32b(8));
    out.extend(u16b(1));
    out.extend(u16b(0x0112));
    out.extend(u16b(3));
    out.extend(u32b(1));
    out.extend(u16b(orientation));
    out.extend([0, 0]);
    out.extend(u32b(0));
    out
}

/// `jpeg` carrying `profile` in APP2 segments of at most `chunk` bytes,
/// placed in reverse order.
fn with_profile(jpeg: &[u8], profile: &[u8], chunk: usize) -> Vec<u8> {
    let chunks: Vec<&[u8]> = profile.chunks(chunk).collect();
    let mut out = jpeg.to_vec();
    for (n, piece) in chunks.iter().enumerate() {
        let mut body = b"ICC_PROFILE\0".to_vec();
        body.extend([n as u8 + 1, chunks.len() as u8]);
        body.extend(*piece);
        out = splice(&out, 0xe2, &body);
    }
    out
}

fn profile_named(mut profile: ColorProfile, name: &str) -> Vec<u8> {
    profile.description = Some(ProfileText::Localizable(vec![LocalizableString::new(
        "en".into(),
        "US".into(),
        name.into(),
    )]));
    profile.encode().unwrap()
}

/// An ICC profile of the older form, whose `desc` tag holds ASCII, and
/// nothing else a transform needs.
fn profile_v2(name: &str) -> Vec<u8> {
    let mut tag = b"desc\0\0\0\0".to_vec();
    tag.extend((name.len() as u32 + 1).to_be_bytes());
    tag.extend(name.as_bytes());
    tag.extend([0; 79]);
    let mut out = vec![0; 128];
    out.extend(1u32.to_be_bytes());
    out.extend(b"desc");
    out.extend(144u32.to_be_bytes());
    out.extend((tag.len() as u32).to_be_bytes());
    out.extend(tag);
    let size = (out.len() as u32).to_be_bytes();
    out[..4].copy_from_slice(&size);
    out[36..40].copy_from_slice(b"acsp");
    out
}

fn riff(chunks: &[(&[u8; 4], &[u8])]) -> Vec<u8> {
    let mut body = b"WEBP".to_vec();
    for (name, chunk) in chunks {
        body.extend(*name);
        body.extend((chunk.len() as u32).to_le_bytes());
        body.extend(*chunk);
        if chunk.len() % 2 == 1 {
            body.push(0);
        }
    }
    let mut out = b"RIFF".to_vec();
    out.extend((body.len() as u32).to_le_bytes());
    out.extend(body);
    out
}

/// The chunks of a RIFF file after its header.
fn chunks_of(webp: &[u8]) -> Vec<([u8; 4], Vec<u8>)> {
    let mut at = 12;
    let mut out = Vec::new();
    while at + 8 <= webp.len() {
        let size = u32::from_le_bytes(webp[at + 4..at + 8].try_into().unwrap()) as usize;
        let name = webp[at..at + 4].try_into().unwrap();
        out.push((name, webp[at + 8..at + 8 + size].to_vec()));
        at += 8 + size + size % 2;
    }
    out
}

fn lossless(image: &RgbaImage, alpha: bool) -> Vec<u8> {
    let mut out = Vec::new();
    let encoder = WebPEncoder::new_lossless(&mut out);
    if alpha {
        encoder
            .encode(
                image,
                image.width(),
                image.height(),
                ExtendedColorType::Rgba8,
            )
            .unwrap();
    } else {
        let rgb = image::DynamicImage::ImageRgba8(image.clone()).into_rgb8();
        encoder
            .encode(&rgb, image.width(), image.height(), ExtendedColorType::Rgb8)
            .unwrap();
    }
    out
}

/// A WebP of `image`'s lossless bitstream under an extended header with
/// `flags`, followed by `extra` chunks.
fn extended(image: &RgbaImage, alpha: bool, flags: u8, extra: &[(&[u8; 4], &[u8])]) -> Vec<u8> {
    let vp8l = chunks_of(&lossless(image, alpha)).remove(0).1;
    let mut header = vec![flags, 0, 0, 0];
    header.extend(&(image.width() - 1).to_le_bytes()[..3]);
    header.extend(&(image.height() - 1).to_le_bytes()[..3]);
    let mut chunks: Vec<(&[u8; 4], &[u8])> = vec![(b"VP8X", &header)];
    let profile = |&&(name, _): &&(&[u8; 4], &[u8])| name == b"ICCP";
    chunks.extend(extra.iter().filter(profile));
    chunks.push((b"VP8L", &vp8l));
    chunks.extend(extra.iter().filter(|chunk| !profile(chunk)));
    riff(&chunks)
}

fn bmff_box(name: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out = (body.len() as u32 + 8).to_be_bytes().to_vec();
    out.extend(name);
    out.extend(body);
    out
}

fn full_box(name: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut full = vec![0; 4];
    full.extend(body);
    bmff_box(name, &full)
}

/// The header of an AVIF, or another ISO media file with `brands`, whose
/// primary item, 2, has an `ispe` of `size` and the further `properties`.
fn avif_header(brands: &[&[u8; 4]], size: Size, properties: &[Vec<u8>]) -> Vec<u8> {
    let mut ftyp = brands[0].to_vec();
    ftyp.extend([0; 4]);
    for brand in brands {
        ftyp.extend(*brand);
    }
    let mut ispe = size.width.to_be_bytes().to_vec();
    ispe.extend(size.height.to_be_bytes());
    // A decoy first, so only the association names the primary item's.
    let mut decoy = 7u32.to_be_bytes().to_vec();
    decoy.extend(7u32.to_be_bytes());
    let mut ipco = full_box(b"ispe", &decoy);
    ipco.extend(full_box(b"ispe", &ispe));
    for property in properties {
        ipco.extend(property);
    }
    let mut ipma = 2u32.to_be_bytes().to_vec();
    ipma.extend(1u16.to_be_bytes());
    ipma.push(1);
    ipma.push(0x81);
    ipma.extend(2u16.to_be_bytes());
    ipma.push(1 + properties.len() as u8);
    ipma.push(0x82);
    for n in 0..properties.len() as u8 {
        ipma.push(0x83 + n);
    }
    let mut iprp = bmff_box(b"ipco", &ipco);
    iprp.extend(full_box(b"ipma", &ipma));
    let mut meta = full_box(b"hdlr", &[0, 0, 0, 0, b'p', b'i', b'c', b't', 0]);
    meta.extend(full_box(b"pitm", &2u16.to_be_bytes()));
    meta.extend(bmff_box(b"iprp", &iprp));
    let mut out = bmff_box(b"ftyp", &ftyp);
    out.extend(full_box(b"meta", &meta));
    out
}

fn small_png() -> Vec<u8> {
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(&[0; 37 * 23 * 3], 37, 23, ExtendedColorType::Rgb8)
        .unwrap();
    out
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & (crc & 1).wrapping_neg());
        }
    }
    !crc
}

/// `bytes` as a zlib stream that compresses nothing.
fn stored(bytes: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = match bytes.is_empty() {
        true => vec![bytes],
        false => bytes.chunks(65_535).collect(),
    };
    for (n, block) in blocks.iter().enumerate() {
        out.push(u8::from(n + 1 == blocks.len()));
        let length = block.len() as u16;
        out.extend(length.to_le_bytes());
        out.extend((!length).to_le_bytes());
        out.extend(*block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in bytes {
        a = (a + u32::from(byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    out.extend((b << 16 | a).to_be_bytes());
    out
}

fn png_chunk(name: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out = (body.len() as u32).to_be_bytes().to_vec();
    out.extend(name);
    out.extend(body);
    out.extend(crc32(&out[4..]).to_be_bytes());
    out
}

/// One row's samples, each `depth` bits, as a PNG packs them.
fn packed(samples: &[u16], depth: u8) -> Vec<u8> {
    match depth {
        16 => samples.iter().flat_map(|s| s.to_be_bytes()).collect(),
        8 => samples.iter().map(|&s| s as u8).collect(),
        _ => {
            let each = usize::from(8 / depth);
            let row = samples.chunks(each);
            row.map(|byte| {
                let bits = byte.iter().fold(0, |bits, &s| bits << depth | s as u8);
                bits << (depth * (each - byte.len()) as u8)
            })
            .collect()
        }
    }
}

/// A PNG `width` wide of `colour` type at `depth` bits a sample, holding
/// `samples` row after row, with the chunks `before` standing before its
/// pixels.
fn png_of(
    width: u32,
    (depth, colour): (u8, u8),
    samples: &[u16],
    before: &[(&[u8; 4], Vec<u8>)],
) -> Vec<u8> {
    let channels = [1, 0, 3, 1, 2, 0, 4][usize::from(colour)];
    let rows: Vec<&[u16]> = samples.chunks(width as usize * channels).collect();
    let mut header = width.to_be_bytes().to_vec();
    header.extend((rows.len() as u32).to_be_bytes());
    header.extend([depth, colour, 0, 0, 0]);
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    out.extend(png_chunk(b"IHDR", &header));
    for (name, body) in before {
        out.extend(png_chunk(name, body));
    }
    let mut lines = Vec::new();
    for row in rows {
        lines.push(0);
        lines.extend(packed(row, depth));
    }
    out.extend(png_chunk(b"IDAT", &stored(&lines)));
    out.extend(png_chunk(b"IEND", &[]));
    out
}

/// A PNG of `image`, with its alpha where `alpha` says.
fn encode_png(image: &RgbaImage, alpha: bool) -> Vec<u8> {
    let mut out = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut out);
    let (width, height) = image.dimensions();
    match alpha {
        true => encoder.write_image(image, width, height, ExtendedColorType::Rgba8),
        false => {
            let rgb = image::DynamicImage::ImageRgba8(image.clone()).into_rgb8();
            encoder.write_image(&rgb, width, height, ExtendedColorType::Rgb8)
        }
    }
    .unwrap();
    out
}

/// An `iCCP` chunk's body holding `profile`.
fn iccp(profile: &[u8]) -> Vec<u8> {
    let mut body = b"profile\0\0".to_vec();
    body.extend(stored(profile));
    body
}

/// An opened picture's pixels with their alpha, 255 where it has none.
fn opened_rgba(opened: &Opened) -> Vec<[u8; 4]> {
    match &opened.pixels {
        copies::Pixels::Rgb(image) => image.pixels().map(|p| [p[0], p[1], p[2], 255]).collect(),
        copies::Pixels::Rgba { straight, .. } => straight.pixels().map(|p| p.0).collect(),
    }
}

fn plain(bytes: &[u8]) -> Plain {
    match read_header(bytes) {
        Ok(Header::Plain(plain)) => plain,
        other => panic!("not a plain picture: {other:?}"),
    }
}

fn photograph(bytes: &[u8]) -> Photograph {
    match read_header(bytes) {
        Ok(Header::Photograph(photograph)) => photograph,
        other => panic!("not a photograph: {other:?}"),
    }
}

fn size(width: u32, height: u32) -> Size {
    Size { width, height }
}

fn kind(header: &Header) -> Kind {
    match header {
        Header::Photograph(photograph) => photograph.kind,
        Header::Plain(plain) => plain.kind,
    }
}

#[test]
fn kinds_are_told_from_bytes_whatever_the_name() {
    let rgba = RgbaImage::from_pixel(3, 2, Rgba([1, 2, 3, 4]));
    let animated = riff(&[(b"VP8X", &[0x02, 0, 0, 0, 9, 0, 0, 4, 0, 0])]);
    let svg =
        b"<?xml version=\"1.0\"?>\n<!-- drawn -->\n<svg xmlns=\"http://www.w3.org/2000/svg\"/>";
    let size = size(1, 1);
    let moving = png_of(1, (8, 0), &[0], &[(b"acTL", vec![0, 0, 0, 1, 0, 0, 0, 0])]);
    let cases: [(&str, Vec<u8>, Kind); 11] = [
        ("photo.png", encode_jpeg(&picture(8, 8)), Kind::Jpeg),
        ("photo.jpg", lossless(&rgba, true), Kind::WebP),
        ("photo.webp", animated, Kind::AnimatedWebP),
        ("drawing.jpg", small_png(), Kind::Png),
        ("drawing.png", moving, Kind::AnimatedPng),
        (
            "drawing.svg",
            b"GIF89a\x01\0\x01\0\0\0\0".to_vec(),
            Kind::Gif,
        ),
        ("drawing.gif", svg.to_vec(), Kind::Svg),
        ("photo.heic", avif_header(&[b"avif"], size, &[]), Kind::Avif),
        ("photo.avif", avif_header(&[b"heic"], size, &[]), Kind::Heic),
        ("photo.HEIC", avif_header(&[b"mif1"], size, &[]), Kind::Heif),
        (
            "photo.jpeg",
            avif_header(&[b"mif1", b"avif"], size, &[]),
            Kind::Avif,
        ),
    ];
    for (name, bytes, expected) in cases {
        let header = read_header(&bytes).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!(kind(&header), expected, "{name}");
        assert_eq!(
            expected.is_photograph(),
            matches!(header, Header::Photograph(_))
        );
    }
}

#[test]
fn plain_sizes_and_the_reasons_none_is_given() {
    let not_read = Err(NoSize::NotRead);
    let unsupported = |what| Err(NoSize::Unreadable(Unreadable::Unsupported(what)));
    let irot = |turns: u8| bmff_box(b"irot", &[turns]);
    let clap = bmff_box(b"clap", &[0; 32]);
    let svg = |root: &str| format!("\u{feff}<!DOCTYPE svg>\n{root}<rect/></svg>").into_bytes();
    let cases: Vec<(&str, Vec<u8>, Result<Size, NoSize>)> = vec![
        (
            "animated PNG",
            png_of(
                3,
                (8, 0),
                &[0; 6],
                &[(b"acTL", vec![0, 0, 0, 1, 0, 0, 0, 0])],
            ),
            Ok(size(3, 2)),
        ),
        (
            "GIF",
            b"GIF87a\x40\x01\xc8\0\0\0\0".to_vec(),
            Ok(size(320, 200)),
        ),
        (
            "animated WebP",
            riff(&[(b"VP8X", &[0x12, 0, 0, 0, 0x1f, 0x03, 0, 0x57, 0x02, 0])]),
            Ok(size(800, 600)),
        ),
        (
            "AVIF",
            avif_header(&[b"avif"], size(640, 480), &[]),
            Ok(size(640, 480)),
        ),
        (
            "AVIF turned a quarter",
            avif_header(&[b"avif"], size(640, 480), &[irot(1)]),
            Ok(size(480, 640)),
        ),
        (
            "AVIF turned a half",
            avif_header(&[b"avif"], size(640, 480), &[irot(2)]),
            Ok(size(640, 480)),
        ),
        (
            "AVIF turned three quarters",
            avif_header(&[b"avif"], size(640, 480), &[irot(3)]),
            Ok(size(480, 640)),
        ),
        (
            "AVIF cropped",
            avif_header(&[b"avif"], size(640, 480), &[clap]),
            unsupported("AVIF cropped by a clean aperture"),
        ),
        (
            "SVG in numbers",
            svg(r#"<svg width="120" height='80.4'>"#),
            Ok(size(120, 80)),
        ),
        (
            "SVG under a prefix",
            svg(r#"<svg:svg xmlns:svg="http://www.w3.org/2000/svg" width="120" height="80">"#),
            Ok(size(120, 80)),
        ),
        (
            "SVG in px",
            svg("<svg\n  viewBox=\"0 0 1 1\"\n  width = \"64px\" height=\" 32 px \">"),
            Ok(size(64, 32)),
        ),
        (
            "SVG with a viewBox alone",
            svg(r#"<svg viewBox="0 0 100 50">"#),
            Err(NoSize::SvgWithoutPixels),
        ),
        (
            "SVG in percent",
            svg(r#"<svg width="100%" height="50">"#),
            Err(NoSize::SvgWithoutPixels),
        ),
        (
            "SVG in em",
            svg(r#"<svg width="10em" height="5em">"#),
            Err(NoSize::SvgWithoutPixels),
        ),
        (
            "SVG with a width alone",
            svg(r#"<svg width="100">"#),
            Err(NoSize::SvgWithoutPixels),
        ),
        (
            "SVG cut short",
            b"<svg width=\"100\" height=\"5".to_vec(),
            Err(NoSize::Unreadable(Unreadable::CutShort)),
        ),
        (
            "HEIC",
            avif_header(&[b"heic"], size(640, 480), &[]),
            not_read.clone(),
        ),
        (
            "HEIF",
            avif_header(&[b"mif1"], size(640, 480), &[]),
            not_read,
        ),
        (
            "GIF cut short",
            b"GIF89a\x40".to_vec(),
            Err(NoSize::Unreadable(Unreadable::CutShort)),
        ),
        (
            "AVIF cut short",
            avif_header(&[b"avif"], size(640, 480), &[])[..60].to_vec(),
            Err(NoSize::Unreadable(Unreadable::CutShort)),
        ),
    ];
    for (name, bytes, expected) in cases {
        assert_eq!(plain(&bytes).size, expected, "{name}");
    }
}

#[test]
fn a_photograph_is_shown_turned_by_each_orientation() {
    let jpeg = encode_jpeg(&picture(48, 32));
    for orientation in 1..=8 {
        for little_endian in [false, true] {
            let bytes = splice(&jpeg, 0xe1, &exif(orientation, little_endian));
            let photograph = photograph(&bytes);
            assert_eq!(photograph.orientation, orientation as u8);
            assert_eq!(photograph.stored, size(48, 32));
            let shown = if orientation >= 5 {
                size(32, 48)
            } else {
                size(48, 32)
            };
            assert_eq!(photograph.shown, shown, "orientation {orientation}");
        }
    }
    let rgba = RgbaImage::from_pixel(48, 32, Rgba([9, 9, 9, 255]));
    let exif_chunk = exif(6, true);
    let webp = extended(&rgba, false, 0x08, &[(b"EXIF", &exif_chunk[6..])]);
    let photograph = photograph(&webp);
    assert_eq!(
        (photograph.orientation, photograph.shown),
        (6, size(32, 48))
    );
}

#[test]
fn missing_or_unreadable_orientations_are_upright() {
    let jpeg = encode_jpeg(&picture(48, 32));
    assert_eq!(photograph(&jpeg).orientation, 1);
    for body in [
        exif(0, false),
        exif(9, true),
        b"Exif\0\0MM\0*\0\0\xff\xff".to_vec(),
        // A place for its entries far past the end of the file.
        b"Exif\0\0II*\0\xfe\xff\xff\xff".to_vec(),
    ] {
        assert_eq!(photograph(&splice(&jpeg, 0xe1, &body)).orientation, 1);
    }
}

#[test]
fn alpha_is_taken_from_the_container() {
    let rgba = RgbaImage::from_pixel(5, 4, Rgba([10, 20, 30, 40]));
    let vp8 = riff(&[(b"VP8 ", &[0x10, 0x02, 0, 0x9d, 0x01, 0x2a, 5, 0, 4, 0])]);
    let cases = [
        ("JPEG", encode_jpeg(&picture(5, 4)), false),
        ("lossy WebP", vp8, false),
        ("lossless WebP with alpha", lossless(&rgba, true), true),
        ("lossless WebP without", lossless(&rgba, false), false),
        (
            "extended WebP with alpha",
            extended(&rgba, true, 0x10, &[]),
            true,
        ),
        (
            "extended WebP without",
            extended(&rgba, false, 0x00, &[]),
            false,
        ),
    ];
    for (name, bytes, alpha) in cases {
        let photograph = photograph(&bytes);
        assert_eq!(photograph.alpha, alpha, "{name}");
        assert_eq!(photograph.stored, size(5, 4), "{name}");
    }
}

#[test]
fn a_photograph_converts_only_from_a_profile_that_is_not_srgb() {
    let jpeg = encode_jpeg(&picture(16, 16));
    assert!(!converts(&jpeg));
    let p3 = profile_named(ColorProfile::new_display_p3(), "Display P3");
    assert!(converts(&with_profile(&jpeg, &p3, 60_000)));
    let srgb = profile_named(ColorProfile::new_srgb(), "Another name");
    assert!(!converts(&with_profile(&jpeg, &srgb, 60_000)));
    assert!(!converts(&with_profile(
        &jpeg,
        b"no profile at all",
        60_000
    )));
    assert!(!converts(b"no picture at all"));
    for bytes in [
        with_profile(&jpeg, &p3, 60_000),
        with_profile(&jpeg, &srgb, 60_000),
    ] {
        assert_eq!(open(&bytes).unwrap().converted, converts(&bytes));
    }
}

#[test]
fn a_profile_is_found_with_its_description() {
    let jpeg = encode_jpeg(&picture(16, 16));
    assert_eq!(photograph(&jpeg).profile, None);
    let p3 = profile_named(ColorProfile::new_display_p3(), "Display P3");
    let named = |name: &str| Some(Some(name.to_owned()));
    assert_eq!(
        photograph(&with_profile(&jpeg, &p3, 60_000)).profile,
        named("Display P3")
    );
    // Split into many segments, placed in reverse order.
    assert_eq!(
        photograph(&with_profile(&jpeg, &p3, 100)).profile,
        named("Display P3")
    );
    let v2 = profile_v2("Camera RGB");
    assert_eq!(
        photograph(&with_profile(&jpeg, &v2, 60)).profile,
        named("Camera RGB")
    );
    let nameless = vec![0; 132];
    assert_eq!(
        photograph(&with_profile(&jpeg, &nameless, 1000)).profile,
        Some(None)
    );
    let rgba = RgbaImage::from_pixel(4, 4, Rgba([1, 2, 3, 255]));
    let webp = extended(&rgba, false, 0x20, &[(b"ICCP", &p3)]);
    assert_eq!(photograph(&webp).profile, named("Display P3"));
}

#[test]
fn damaged_headers_answer_what_was_wrong() {
    let jpeg = encode_jpeg(&picture(16, 16));
    let mut lossless_jpeg = jpeg.clone();
    let sof = lossless_jpeg
        .windows(2)
        .position(|w| w == [0xff, 0xc0])
        .unwrap();
    lossless_jpeg[sof + 1] = 0xc3;
    let rgba = RgbaImage::from_pixel(4, 4, Rgba([1, 2, 3, 255]));
    let mut later_lossless = lossless(&rgba, true);
    later_lossless[24] |= 0x20;
    let png = small_png();
    let mut unknown_colour = png.clone();
    unknown_colour[25] = 5;
    let mut no_pixels = png[..33].to_vec();
    no_pixels.extend(png_chunk(b"IEND", &[]));
    // A chunk that says it holds more than the file does.
    let mut long_chunk = png[..33].to_vec();
    long_chunk.extend([0x7f, 0xff, 0xff, 0xff]);
    long_chunk.extend(b"tEXtComment");
    let mut long_profile = png[..33].to_vec();
    long_profile.extend(1000u32.to_be_bytes());
    long_profile.extend(b"iCCPprofile\0\0");
    let mut long_webp = extended(&rgba, false, 0x20, &[]);
    long_webp.extend(b"ICCP\xff\xff\xff\x7fprofile");
    let cases: [(&str, Vec<u8>, Unreadable); 17] = [
        (
            "PNG chunk longer than the file",
            long_chunk,
            Unreadable::CutShort,
        ),
        (
            "PNG profile longer than the file",
            long_profile,
            Unreadable::CutShort,
        ),
        (
            "WebP chunk longer than the file",
            long_webp,
            Unreadable::CutShort,
        ),
        ("PNG signature cut", png[..5].to_vec(), Unreadable::CutShort),
        (
            "PNG cut in its header",
            png[..20].to_vec(),
            Unreadable::CutShort,
        ),
        (
            "PNG cut before its pixels",
            png[..36].to_vec(),
            Unreadable::CutShort,
        ),
        (
            "PNG of an unknown colour type",
            unknown_colour,
            Unreadable::Unsupported("PNG of an unknown colour type"),
        ),
        (
            "PNG without pixels",
            no_pixels,
            Unreadable::Unsupported("PNG without an IDAT chunk"),
        ),
        ("empty", Vec::new(), Unreadable::NotAPicture),
        ("text", b"Dear reader,".to_vec(), Unreadable::NotAPicture),
        ("random", Noise(7).bytes(4096), Unreadable::NotAPicture),
        (
            "JPEG signature cut",
            jpeg[..2].to_vec(),
            Unreadable::CutShort,
        ),
        (
            "JPEG cut before its frame",
            jpeg[..30].to_vec(),
            Unreadable::CutShort,
        ),
        (
            "lossless JPEG",
            lossless_jpeg,
            Unreadable::Unsupported("JPEG coded losslessly, hierarchically or arithmetically"),
        ),
        (
            "WebP cut",
            lossless(&rgba, true)[..22].to_vec(),
            Unreadable::CutShort,
        ),
        (
            "lossless WebP of a later version",
            later_lossless,
            Unreadable::Unsupported("lossless WebP of a later version"),
        ),
        (
            "unknown WebP chunk",
            riff(&[(b"VP9 ", &[0; 10])]),
            Unreadable::Unsupported("WebP without a VP8, VP8L or VP8X chunk"),
        ),
    ];
    for (name, bytes, expected) in cases {
        assert_eq!(read_header(&bytes), Err(expected), "{name}");
    }
}

const UNCAPPED: u32 = u32::MAX;

#[test]
fn widths_follow_the_rungs() {
    let cases: [(u32, u32, &[u32]); 9] = [
        (3000, UNCAPPED, &[400, 800, 1200, 1600, 2400, 3000]),
        (2560, UNCAPPED, &[400, 800, 1200, 1600, 2560]),
        (1650, UNCAPPED, &[400, 800, 1200, 1650]),
        (240, UNCAPPED, &[240]),
        (1650, 1600, &[400, 800, 1200, 1600]),
        (3000, 200, &[200]),
        (3000, 1000, &[400, 800]),
        (444, UNCAPPED, &[444]),
        (445, UNCAPPED, &[400, 445]),
    ];
    for (natural, max_width, expected) in cases {
        assert_eq!(
            widths(natural, max_width),
            expected,
            "{natural} capped at {max_width}"
        );
    }
}

#[test]
fn no_width_is_wider_than_the_photograph() {
    for natural in (1..3200).step_by(7) {
        for max_width in [1, 200, 400, 1000, 1200, 1600, 2400, UNCAPPED] {
            let widths = widths(natural, max_width);
            assert!(!widths.is_empty());
            assert!(widths.iter().all(|&w| w <= natural && w <= max_width));
            assert!(widths.is_sorted());
        }
    }
}

#[test]
fn the_src_width_is_the_widest_at_or_below_1200() {
    assert_eq!(src_width(&[400, 800, 1200, 1600, 2400, 3000]), Some(1200));
    assert_eq!(src_width(&[400, 800, 1100]), Some(1100));
    assert_eq!(src_width(&[1500]), Some(1500));
    assert_eq!(src_width(&[]), None);
}

#[test]
fn copy_heights_keep_the_proportions() {
    assert_eq!(copy_height(size(3000, 2000), 400), 267);
    assert_eq!(copy_height(size(3000, 2000), 3000), 2000);
    assert_eq!(copy_height(size(1000, 333), 3), 1);
    assert_eq!(copy_height(size(4000, 1), 400), 1);
    assert_eq!(copy_height(size(1000, 1001), 500), 501);
    assert_eq!(copy_height(size(2, 1), 1), 1);
    assert_eq!(copy_height(size(800, 1200), 400), 600);
}

fn a_photograph() -> Photograph {
    Photograph {
        kind: Kind::Jpeg,
        stored: size(2000, 1500),
        orientation: 6,
        shown: size(1500, 2000),
        alpha: false,
        profile: None,
    }
}

#[test]
fn copies_are_named_by_stem_width_and_key() {
    let source = blake3::hash(b"the photograph's bytes");
    let settings = Settings::default();
    let stem = "Été à midi (2).final";
    let copies = ladder(&a_photograph(), false, stem, &source, u32::MAX, &settings);
    let widths: Vec<_> = copies.iter().map(|p| (p.width, p.format)).collect();
    assert_eq!(widths.len(), 8);
    assert_eq!(widths[..2], [(400, Format::Avif), (400, Format::Jpeg)]);
    assert_eq!(widths[6..], [(1500, Format::Avif), (1500, Format::Jpeg)]);
    for copy in &copies {
        let key = Key::new(&source, copy.width, copy.format, &settings);
        assert_eq!(copy.key, key);
        assert_eq!(copy.height, copy_height(size(1500, 2000), copy.width));
        let hex = key.hex();
        assert_eq!(hex.len(), 64);
        assert!(
            hex.bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        );
        assert_eq!(key.hash8(), hex[..8]);
        let name = format!(
            "{stem}-{}.{}.{}",
            copy.width,
            &hex[..8],
            copy.format.extension()
        );
        assert_eq!(copy.name, name);
    }
    // The second copy follows the pixels, not what the header says.
    let alpha = Photograph {
        alpha: true,
        ..a_photograph()
    };
    for (photograph, transparent, endings) in [
        (&alpha, true, ["avif", ".png"]),
        (&alpha, false, ["avif", ".jpg"]),
        (&a_photograph(), false, ["avif", ".jpg"]),
    ] {
        let copies = ladder(photograph, transparent, "a", &source, 400, &settings);
        let names: Vec<_> = copies.iter().map(|p| &p.name[p.name.len() - 4..]).collect();
        assert_eq!(names, endings);
    }
}

#[test]
fn a_key_changes_with_the_source_the_width_and_the_format() {
    let settings = Settings::default();
    let a = blake3::hash(b"a");
    let b = blake3::hash(b"b");
    let key = Key::new(&a, 800, Format::Avif, &settings);
    assert_eq!(key, Key::new(&a, 800, Format::Avif, &settings));
    assert_ne!(key, Key::new(&b, 800, Format::Avif, &settings));
    assert_ne!(key, Key::new(&a, 1200, Format::Avif, &settings));
    assert_ne!(key, Key::new(&a, 800, Format::Jpeg, &settings));
    assert_ne!(
        Key::new(&a, 800, Format::Jpeg, &settings),
        Key::new(&a, 800, Format::Png, &settings)
    );
}

#[test]
fn the_settings_are_as_documented() {
    let settings = Settings::default();
    assert_eq!(settings.pipeline, PIPELINE);
    assert_eq!(settings.avif_encoder, avif_aom_sys::version());
    let avif = (
        settings.avif_speed,
        settings.avif_quality,
        settings.avif_alpha_quality,
        settings.avif_threads,
    );
    assert_eq!(avif, (8, 67, 90, 1));
    assert_eq!(settings.resize, FilterType::Lanczos3);
    assert_eq!(settings.jpeg_quality, 77);
    assert_eq!(
        settings.jpeg_subsampling,
        jpeg_encoder::SamplingFactor::R_4_4_4
    );
    assert!(settings.jpeg_progressive && settings.jpeg_optimised_huffman);
    assert_eq!(
        settings.png_compression,
        image::codecs::png::CompressionType::Default
    );
    assert_eq!(
        settings.png_filter,
        image::codecs::png::FilterType::Adaptive
    );
}

#[test]
fn every_setting_changes_the_key() {
    use image::codecs::png::{CompressionType, FilterType as PngFilter};
    use jpeg_encoder::{ChromaSubsamplingMethod, SamplingFactor};
    let source = blake3::hash(b"photograph");
    let base = Settings::default();
    let key = |settings: &Settings| Key::new(&source, 800, Format::Jpeg, settings);
    type Change = fn(&mut Settings);
    let changes: [(&str, Change); 14] = [
        ("pipeline", |s| s.pipeline += 1),
        ("avif_encoder", |s| s.avif_encoder.push_str(" patched")),
        ("avif_speed", |s| s.avif_speed = 6),
        ("avif_quality", |s| s.avif_quality = 70),
        ("avif_alpha_quality", |s| s.avif_alpha_quality = 80),
        ("avif_threads", |s| s.avif_threads = 4),
        ("resize", |s| s.resize = FilterType::CatmullRom),
        ("jpeg_quality", |s| s.jpeg_quality = 80),
        ("jpeg_subsampling", |s| {
            s.jpeg_subsampling = SamplingFactor::R_4_2_0
        }),
        ("jpeg_chroma_method", |s| {
            s.jpeg_chroma_method = ChromaSubsamplingMethod::Nearest;
        }),
        ("jpeg_progressive", |s| s.jpeg_progressive = false),
        ("jpeg_optimised_huffman", |s| {
            s.jpeg_optimised_huffman = false
        }),
        ("png_compression", |s| {
            s.png_compression = CompressionType::Best
        }),
        ("png_filter", |s| s.png_filter = PngFilter::Paeth),
    ];
    for (name, change) in changes {
        let mut changed = base.clone();
        change(&mut changed);
        assert_ne!(changed, base, "{name} did not change");
        assert_ne!(key(&changed), key(&base), "{name} changed without the key");
    }
}

fn decode(bytes: &[u8]) -> image::DynamicImage {
    image::load_from_memory(bytes).unwrap()
}

/// An AVIF's size, from its `ispe`.
fn avif_size(bytes: &[u8]) -> Size {
    let plain = plain(bytes);
    assert_eq!(plain.kind, Kind::Avif);
    plain.size.unwrap()
}

#[test]
fn each_copy_has_the_size_named_in_its_ladder() {
    let rgba = RgbaImage::from_fn(900, 500, |x, y| Rgba([x as u8, y as u8, 90, (x ^ y) as u8]));
    let pictures = [encode_jpeg(&picture(900, 500)), lossless(&rgba, true)];
    let source = blake3::hash(b"unused");
    for bytes in pictures {
        let header = photograph(&bytes);
        let opened = open(&bytes).unwrap();
        assert_eq!(opened.size(), header.shown);
        assert_eq!(opened.alpha(), header.alpha);
        let settings = Settings::default();
        for copy in ladder(&header, header.alpha, "p", &source, u32::MAX, &settings) {
            let made = opened.make(copy.width, copy.format, &settings).unwrap();
            let made_size = match copy.format {
                Format::Avif => avif_size(&made),
                Format::Jpeg | Format::Png => {
                    let image = decode(&made);
                    size(image.width(), image.height())
                }
            };
            assert_eq!(made_size, size(copy.width, copy.height), "{copy:?}");
        }
    }
}

fn psnr(a: &[u8], b: &[u8]) -> f64 {
    assert_eq!(a.len(), b.len());
    let error: f64 = a
        .iter()
        .zip(b)
        .map(|(&a, &b)| (f64::from(a) - f64::from(b)).powi(2))
        .sum::<f64>()
        / a.len() as f64;
    10.0 * (255.0 * 255.0 / error).log10()
}

#[test]
fn a_jpeg_copy_is_close_to_resizing_the_source_directly() {
    let jpeg = encode_jpeg(&picture(1000, 640));
    let source = decode(&jpeg).into_rgb8();
    let opened = open(&jpeg).unwrap();
    let settings = Settings::default();
    for width in [400, 800, 1000] {
        let height = copy_height(size(1000, 640), width);
        let direct = imageops::resize(&source, width, height, FilterType::Lanczos3);
        let copy = decode(&opened.make(width, Format::Jpeg, &settings).unwrap()).into_rgb8();
        let psnr = psnr(&direct, &copy);
        assert!(psnr >= 30.0, "{width}: {psnr:.1} dB");
    }
}

/// A picture `shown`, stored so that `orientation` turns it upright.
fn stored_for(shown: &RgbImage, orientation: u8) -> RgbImage {
    match orientation {
        1 => shown.clone(),
        2 => imageops::flip_horizontal(shown),
        3 => imageops::rotate180(shown),
        4 => imageops::flip_vertical(shown),
        5 => imageops::rotate270(&imageops::flip_horizontal(shown)),
        6 => imageops::rotate270(shown),
        7 => imageops::rotate90(&imageops::flip_horizontal(shown)),
        _ => imageops::rotate90(shown),
    }
}

#[test]
fn a_photograph_stored_on_its_side_comes_out_upright() {
    // Upright: red at the top left, blue at the bottom left, grey elsewhere.
    let shown = RgbImage::from_fn(64, 96, |x, y| match (x < 24, y < 24, y >= 72) {
        (true, true, _) => Rgb([220, 30, 30]),
        (true, _, true) => Rgb([30, 30, 220]),
        _ => Rgb([128, 128, 128]),
    });
    let settings = Settings::default();
    for orientation in 1..=8 {
        let stored = stored_for(&shown, orientation);
        let jpeg = splice(
            &encode_jpeg(&stored),
            0xe1,
            &exif(orientation.into(), false),
        );
        let opened = open(&jpeg).unwrap();
        assert_eq!(opened.size(), size(64, 96), "orientation {orientation}");
        let copy = decode(&opened.make(64, Format::Jpeg, &settings).unwrap()).into_rgb8();
        let near = |p: &Rgb<u8>, q: [u8; 3]| p.0.iter().zip(q).all(|(a, b)| a.abs_diff(b) < 24);
        assert!(
            near(copy.get_pixel(8, 8), [220, 30, 30]),
            "orientation {orientation}"
        );
        assert!(
            near(copy.get_pixel(8, 88), [30, 30, 220]),
            "orientation {orientation}"
        );
        assert!(
            near(copy.get_pixel(56, 8), [128, 128, 128]),
            "orientation {orientation}"
        );
    }
}

fn opened_rgb(opened: &Opened) -> &RgbImage {
    match &opened.pixels {
        copies::Pixels::Rgb(image) => image,
        copies::Pixels::Rgba { .. } => panic!("held with alpha"),
    }
}

#[test]
fn display_p3_is_converted_to_srgb() {
    let flat = RgbaImage::from_pixel(32, 16, Rgba([128, 0, 199, 255]));
    let p3 = profile_named(ColorProfile::new_display_p3(), "Display P3");
    let webp = extended(&flat, false, 0x20, &[(b"ICCP", &p3)]);
    let opened = open(&webp).unwrap();
    assert!(opened.converted);
    assert_eq!(opened.profile, Some(Some("Display P3".into())));
    for pixel in opened_rgb(&opened).pixels() {
        let close = pixel
            .0
            .iter()
            .zip([141, 0, 207])
            .all(|(a, b)| a.abs_diff(b) <= 2);
        assert!(close, "{pixel:?}");
    }
}

#[test]
fn a_profile_that_is_srgb_under_another_name_is_not_converted() {
    let flat = RgbaImage::from_pixel(32, 16, Rgba([128, 0, 199, 255]));
    let srgb = profile_named(ColorProfile::new_srgb(), "Camera standard");
    let webp = extended(&flat, false, 0x20, &[(b"ICCP", &srgb)]);
    let opened = open(&webp).unwrap();
    assert!(!opened.converted);
    assert_eq!(opened.profile, Some(Some("Camera standard".into())));
    assert!(opened_rgb(&opened).pixels().all(|p| p.0 == [128, 0, 199]));
}

#[test]
fn an_unreadable_profile_leaves_the_pixels_alone() {
    let flat = RgbaImage::from_pixel(8, 8, Rgba([128, 0, 199, 255]));
    let v2 = profile_v2("Broken");
    let webp = extended(&flat, false, 0x20, &[(b"ICCP", &v2)]);
    let opened = open(&webp).unwrap();
    assert!(!opened.converted);
    assert_eq!(opened.profile, Some(Some("Broken".into())));
    assert!(opened_rgb(&opened).pixels().all(|p| p.0 == [128, 0, 199]));
}

#[test]
fn a_jpeg_with_a_display_p3_profile_is_converted() {
    let flat = RgbImage::from_pixel(32, 32, Rgb([128, 0, 199]));
    let p3 = profile_named(ColorProfile::new_display_p3(), "Display P3");
    let jpeg = with_profile(&encode_jpeg(&flat), &p3, 60_000);
    let opened = open(&jpeg).unwrap();
    assert!(opened.converted);
    let pixel = opened_rgb(&opened).get_pixel(16, 16);
    assert!(
        pixel
            .0
            .iter()
            .zip([141, 0, 207])
            .all(|(a, b)| a.abs_diff(b) <= 6),
        "{pixel:?}"
    );
}

#[test]
fn transparent_colour_does_not_bleed_into_the_edge_of_what_is_opaque() {
    // A green disc, softened at its rim, on pixels transparent but magenta.
    let rgba = RgbaImage::from_fn(600, 400, |x, y| {
        let dx = f64::from(x) - 300.0;
        let dy = f64::from(y) - 200.0;
        let edge = 150.0 - (dx * dx + dy * dy).sqrt();
        let a = (edge * 64.0).clamp(0.0, 255.0) as u8;
        if a == 0 {
            Rgba([255, 0, 255, 0])
        } else {
            Rgba([0, 200, 0, a])
        }
    });
    let webp = lossless(&rgba, true);
    assert!(
        decode(&webp)
            .into_rgba8()
            .pixels()
            .any(|p| p.0 == [255, 0, 255, 0])
    );
    let opened = open(&webp).unwrap();
    let settings = Settings::default();
    for width in [400, 250, 97] {
        let copy = decode(&opened.make(width, Format::Png, &settings).unwrap()).into_rgba8();
        let edge: Vec<_> = copy.pixels().filter(|p| p[3] > 0 && p[3] < 255).collect();
        assert!(!edge.is_empty());
        for pixel in edge.iter().filter(|p| p[3] >= 32) {
            assert!(pixel[0] <= 16 && pixel[2] <= 16, "{width}: {pixel:?}");
        }
    }
}

/// The markers of a JPEG's segments before its first scan.
fn jpeg_markers(jpeg: &[u8]) -> Vec<(u8, Vec<u8>)> {
    let mut at = 2;
    let mut out = Vec::new();
    while jpeg[at + 1] != 0xda {
        let length = usize::from(u16::from_be_bytes([jpeg[at + 2], jpeg[at + 3]]));
        out.push((jpeg[at + 1], jpeg[at + 4..at + 2 + length].to_vec()));
        at += 2 + length;
    }
    out
}

/// The types of a PNG's chunks.
fn png_chunks(png: &[u8]) -> Vec<[u8; 4]> {
    let mut at = 8;
    let mut out = Vec::new();
    while at + 8 <= png.len() {
        let length = u32::from_be_bytes(png[at..at + 4].try_into().unwrap()) as usize;
        out.push(png[at + 4..at + 8].try_into().unwrap());
        at += 12 + length;
    }
    out
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[test]
fn no_copy_carries_a_profile_or_exif() {
    let p3 = profile_named(ColorProfile::new_display_p3(), "Display P3");
    let jpeg = splice(
        &with_profile(&encode_jpeg(&picture(500, 300)), &p3, 60_000),
        0xe1,
        &exif(3, true),
    );
    let rgba = RgbaImage::from_pixel(500, 300, Rgba([50, 100, 150, 200]));
    let exif_chunk = exif(3, true);
    let webp = extended(
        &rgba,
        true,
        0x38,
        &[(b"ICCP", &p3), (b"EXIF", &exif_chunk[6..])],
    );
    let settings = Settings::default();
    for bytes in [jpeg, webp] {
        let header = photograph(&bytes);
        assert!(header.profile.is_some() && header.orientation == 3);
        let opened = open(&bytes).unwrap();
        for width in [400, 500] {
            for format in formats(opened.alpha()) {
                let made = opened.make(width, format, &settings).unwrap();
                match format {
                    Format::Jpeg => {
                        for (marker, body) in jpeg_markers(&made) {
                            assert!(marker != 0xe2 && !body.starts_with(b"Exif"));
                        }
                    }
                    Format::Png => {
                        let chunks = png_chunks(&made);
                        assert!(!chunks.iter().any(|c| [b"iCCP", b"eXIf"].contains(&c)));
                    }
                    Format::Avif => {
                        for marker in [&b"prof"[..], b"rICC", b"Exif", b"mime"] {
                            assert!(!contains(&made, marker));
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn the_same_input_gives_the_same_bytes() {
    let jpeg = encode_jpeg(&picture(640, 400));
    let rgba = RgbaImage::from_fn(640, 400, |x, y| Rgba([x as u8, y as u8, 7, (x + y) as u8]));
    let webp = lossless(&rgba, true);
    let settings = Settings::default();
    for (bytes, formats) in [
        (jpeg, [Format::Avif, Format::Jpeg]),
        (webp, [Format::Avif, Format::Png]),
    ] {
        let (first, second) = (open(&bytes).unwrap(), open(&bytes).unwrap());
        for format in formats {
            for width in [400, 640] {
                let a = first.make(width, format, &settings).unwrap();
                let b = second.make(width, format, &settings).unwrap();
                assert!(a == b, "{format:?} at {width}");
            }
        }
    }
}

#[test]
fn copies_can_be_made_from_several_threads() {
    fn shared<T: Send + Sync>() {}
    shared::<Opened>();
    shared::<Settings>();
}

#[test]
fn damaged_files_give_typed_errors() {
    let jpeg = encode_jpeg(&picture(300, 200));
    let rgba = RgbaImage::from_fn(300, 200, |x, y| Rgba([x as u8, y as u8, 7, 200]));
    let webp = lossless(&rgba, true);
    let png = encode_png(&rgba, true);
    for bytes in [&jpeg, &webp, &png] {
        for cut in [
            1,
            3,
            12,
            40,
            200,
            bytes.len() / 2,
            bytes.len() - 3,
            bytes.len() - 1,
        ] {
            match open(&bytes[..cut]) {
                Err(Error::Unreadable(_)) => {}
                other => panic!("cut at {cut}: {:?}", other.map(|o| o.size())),
            }
        }
    }
    let mut noise = Noise(42);
    for length in [0, 5, 100, 10_000] {
        assert!(matches!(
            open(&noise.bytes(length)),
            Err(Error::Unreadable(Unreadable::NotAPicture))
        ));
    }
    // Headers that read, over pixels that are noise: decoded or refused,
    // but never a panic.
    let scan = jpeg_markers(&jpeg)
        .iter()
        .map(|(_, b)| b.len() + 4)
        .sum::<usize>()
        + 2;
    for seed in 1..20 {
        let mut damaged = jpeg.clone();
        let mut noise = Noise(seed);
        let end = damaged.len() - 2;
        for byte in &mut damaged[scan + 20..end] {
            if noise.next().is_multiple_of(8) {
                *byte = (noise.next() >> 24) as u8;
            }
        }
        let _ = open(&damaged);
        let mut damaged = webp.clone();
        let end = damaged.len();
        for byte in &mut damaged[30..end] {
            if noise.next().is_multiple_of(8) {
                *byte = (noise.next() >> 24) as u8;
            }
        }
        let _ = open(&damaged);
    }
}

#[test]
fn a_photograph_too_large_to_decode_is_refused_first() {
    let mut jpeg = encode_jpeg(&picture(16, 16));
    let sof = jpeg.windows(2).position(|w| w == [0xff, 0xc0]).unwrap();
    jpeg[sof + 5..sof + 9].copy_from_slice(&[0xff, 0xff, 0xff, 0xff]);
    assert_eq!(open(&jpeg).err(), Some(Error::TooLarge(size(65535, 65535))));
}

#[test]
fn only_the_copies_a_photograph_has_are_made() {
    let opened = open(&encode_jpeg(&picture(300, 200))).unwrap();
    let settings = Settings::default();
    for (width, format) in [(200, Format::Png), (301, Format::Jpeg), (0, Format::Avif)] {
        let refused = opened.make(width, format, &settings);
        assert_eq!(refused, Err(Error::NotOffered { width, format }));
    }
    // Nor a PNG of one that is a PNG, where nothing of it is transparent.
    let opaque = RgbaImage::from_pixel(300, 200, Rgba([10, 20, 30, 255]));
    for alpha in [false, true] {
        let png = open(&encode_png(&opaque, alpha)).unwrap();
        let refused = png.make(200, Format::Png, &settings);
        let format = Format::Png;
        assert_eq!(refused, Err(Error::NotOffered { width: 200, format }));
    }
    assert_eq!(
        open(b"GIF89a\x01\0\x01\0\0\0\0").err(),
        Some(Error::NotAPhotograph(Kind::Gif))
    );
}

/// What each of eight pixels holds in a PNG of a colour type and depth,
/// as its samples and as the 8-bit pixel they open to.
fn png_pixel(n: u16, depth: u8, colour: u8) -> (Vec<u16>, [u8; 4]) {
    let wide = |v: u16| if depth == 16 { v * 257 } else { v };
    let (r, g, b, a) = (n * 30, 255 - n * 30, 7, n * 36);
    let narrow = |v: u16| (v as u8, v as u8, v as u8);
    match colour {
        0 if depth < 8 => {
            let v = n % (1 << depth);
            let shown = (v * (255 / ((1 << depth) - 1))) as u8;
            (vec![v], [shown, shown, shown, 255])
        }
        0 => (vec![wide(r)], [r as u8, r as u8, r as u8, 255]),
        2 => (
            vec![wide(r), wide(g), wide(b)],
            [r as u8, g as u8, b as u8, 255],
        ),
        3 => {
            let v = n % (1 << depth).min(8);
            (
                vec![v],
                [(v * 30) as u8, (200 - v * 20) as u8, v as u8, 255],
            )
        }
        4 => {
            let (r, g, b) = narrow(r);
            (vec![wide(u16::from(r)), wide(a)], [r, g, b, a as u8])
        }
        _ => (
            vec![wide(r), wide(g), wide(b), wide(a)],
            [r as u8, g as u8, b as u8, a as u8],
        ),
    }
}

/// A PNG four pixels wide and two high of [`png_pixel`], and the pixels
/// it opens to.
fn png_case(depth: u8, colour: u8, before: &[(&[u8; 4], Vec<u8>)]) -> (Vec<u8>, Vec<[u8; 4]>) {
    let pixels: Vec<_> = (0..8).map(|n| png_pixel(n, depth, colour)).collect();
    let samples: Vec<u16> = pixels.iter().flat_map(|(s, _)| s.clone()).collect();
    let mut chunks = Vec::new();
    if colour == 3 {
        let entries = (0..8u8).flat_map(|v| [v * 30, 200 - v * 20, v]);
        chunks.push((b"PLTE", entries.take(3 << depth.min(3)).collect()));
    }
    chunks.extend(before.iter().cloned());
    let shown = pixels.into_iter().map(|(_, shown)| shown).collect();
    (png_of(4, (depth, colour), &samples, &chunks), shown)
}

#[test]
fn a_png_of_each_colour_type_and_depth_opens_to_its_pixels() {
    let cases: [(u8, &[u8]); 5] = [
        (0, &[1, 2, 4, 8, 16]),
        (2, &[8, 16]),
        (3, &[1, 2, 4, 8]),
        (4, &[8, 16]),
        (6, &[8, 16]),
    ];
    for (colour, depths) in cases {
        for &depth in depths {
            let (png, shown) = png_case(depth, colour, &[]);
            let name = format!("colour type {colour} at {depth} bits");
            let header = photograph(&png);
            assert_eq!(header.kind, Kind::Png, "{name}");
            assert_eq!((header.stored, header.shown), (size(4, 2), size(4, 2)));
            assert_eq!(header.orientation, 1, "{name}");
            assert_eq!(header.alpha, matches!(colour, 4 | 6), "{name}");
            assert_eq!(header.profile, None, "{name}");
            let opened = open(&png).unwrap_or_else(|e| panic!("{name}: {e:?}"));
            assert_eq!(opened.alpha(), header.alpha, "{name}");
            assert_eq!(opened_rgba(&opened), shown, "{name}");
        }
    }
}

#[test]
fn a_pngs_alpha_is_told_from_its_colour_type_or_its_transparency_chunk() {
    // Each names what is transparent: a grey, a colour, or each palette
    // entry's own alpha.
    let cases = [
        ("grey", 8, 0, vec![0, 30], [255, 0, 255]),
        ("deep grey", 16, 0, vec![0x1e, 0x1e], [255, 0, 255]),
        ("colour", 8, 2, vec![0, 30, 0, 225, 0, 7], [255, 0, 255]),
        ("palette", 8, 3, vec![0, 128], [0, 128, 255]),
    ];
    for (name, depth, colour, transparent, first_alphas) in cases {
        let (png, mut shown) = png_case(depth, colour, &[(b"tRNS", transparent)]);
        assert!(photograph(&png).alpha, "{name}");
        let opened = open(&png).unwrap();
        assert!(opened.alpha(), "{name}");
        for (pixel, alpha) in shown.iter_mut().zip(first_alphas) {
            pixel[3] = alpha;
        }
        assert_eq!(opened_rgba(&opened), shown, "{name}");
    }
}

#[test]
fn an_animated_png_is_plain_and_still_decodes_as_the_picture_it_first_shows() {
    let mut frame = vec![0, 0, 0, 0];
    frame.extend([0, 0, 0, 4, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0]);
    frame.extend([0, 1, 0, 10, 0, 0]);
    let moving = [(b"acTL", vec![0, 0, 0, 1, 0, 0, 0, 0]), (b"fcTL", frame)];
    let (png, shown) = png_case(8, 2, &moving);
    let plain = plain(&png);
    assert_eq!(
        (plain.kind, plain.size),
        (Kind::AnimatedPng, Ok(size(4, 2)))
    );
    assert!(!Kind::AnimatedPng.is_photograph());
    assert_eq!(
        open(&png).err(),
        Some(Error::NotAPhotograph(Kind::AnimatedPng))
    );
    assert!(!converts(&png));
    let still: Vec<[u8; 3]> = decode_still(&png).unwrap().pixels().map(|p| p.0).collect();
    let expected: Vec<[u8; 3]> = shown.iter().map(|p| [p[0], p[1], p[2]]).collect();
    assert_eq!(still, expected);
    // The same file without the chunks that make it move is a photograph.
    assert_eq!(photograph(&png_case(8, 2, &[]).0).kind, Kind::Png);
}

#[test]
fn a_png_with_a_profile_is_converted() {
    let flat = vec![[128u16, 0, 199]; 32 * 16].concat();
    let p3 = profile_named(ColorProfile::new_display_p3(), "Display P3");
    let png = png_of(32, (8, 2), &flat, &[(b"iCCP", iccp(&p3))]);
    assert_eq!(photograph(&png).profile, Some(Some("Display P3".into())));
    assert!(converts(&png));
    let opened = open(&png).unwrap();
    assert!(opened.converted);
    assert_eq!(opened.profile, Some(Some("Display P3".into())));
    for pixel in opened_rgb(&opened).pixels() {
        let close = pixel
            .0
            .iter()
            .zip([141, 0, 207])
            .all(|(a, b)| a.abs_diff(b) <= 2);
        assert!(close, "{pixel:?}");
    }

    let srgb = profile_named(ColorProfile::new_srgb(), "Another name");
    let png = png_of(32, (8, 2), &flat, &[(b"iCCP", iccp(&srgb))]);
    assert_eq!(photograph(&png).profile, Some(Some("Another name".into())));
    assert!(!converts(&png));
    let opened = open(&png).unwrap();
    assert!(!opened.converted);
    assert!(opened_rgb(&opened).pixels().all(|p| p.0 == [128, 0, 199]));
}

#[test]
fn a_pngs_second_copy_is_a_png_only_where_it_is_transparent() {
    let rgb = picture(900, 500);
    let mut rgba = image::DynamicImage::ImageRgb8(rgb).into_rgba8();
    let source = blake3::hash(b"unused");
    let settings = Settings::default();
    // No alpha, alpha every pixel leaves opaque, and alpha in use.
    for (alpha, used) in [(false, false), (true, false), (true, true)] {
        if used {
            for (x, _, pixel) in rgba.enumerate_pixels_mut() {
                pixel[3] = (x % 256) as u8;
            }
        }
        let png = encode_png(&rgba, alpha);
        let header = photograph(&png);
        assert_eq!(header.alpha, alpha);
        assert_eq!(transparent(&png), used);
        let second = if used { Format::Png } else { Format::Jpeg };
        assert_eq!(formats(used), [Format::Avif, second]);
        let opened = open(&png).unwrap();
        assert_eq!(opened.alpha(), used);
        let copies = ladder(&header, used, "diagram-1", &source, u32::MAX, &settings);
        let named: Vec<_> = copies.iter().map(|c| (c.width, c.format)).collect();
        let expected: Vec<_> = [400, 800, 900]
            .into_iter()
            .flat_map(|width| [(width, Format::Avif), (width, second)])
            .collect();
        assert_eq!(named, expected);
        for copy in copies {
            let ending = format!(".{}", copy.format.extension());
            assert!(copy.name.ends_with(&ending), "{}", copy.name);
            let made = opened.make(copy.width, copy.format, &settings).unwrap();
            if copy.format == Format::Avif {
                assert_eq!(avif_size(&made), size(copy.width, copy.height));
                assert_eq!(contains(&made, AVIF_ALPHA), used, "{}", copy.name);
                continue;
            }
            let decoded = decode(&made);
            let colour = match used {
                true => image::ColorType::Rgba8,
                false => image::ColorType::Rgb8,
            };
            assert_eq!(decoded.color(), colour, "{}", copy.name);
            if !used {
                let direct = image::DynamicImage::ImageRgba8(rgba.clone()).into_rgb8();
                let direct = match copy.width {
                    900 => direct,
                    width => imageops::resize(&direct, width, copy.height, FilterType::Lanczos3),
                };
                let psnr = psnr(&direct, &decoded.into_rgb8());
                assert!(psnr >= 30.0, "{}: {psnr:.1} dB", copy.name);
                continue;
            }
            let direct = match copy.width {
                900 => rgba.clone(),
                width => {
                    let mut premultiplied = rgba.clone();
                    for pixel in premultiplied.pixels_mut() {
                        let a = u32::from(pixel[3]);
                        for c in &mut pixel.0[..3] {
                            *c = ((u32::from(*c) * a + 127) / 255) as u8;
                        }
                    }
                    let mut resized =
                        imageops::resize(&premultiplied, width, copy.height, FilterType::Lanczos3);
                    for pixel in resized.pixels_mut() {
                        let a = u32::from(pixel[3]);
                        for c in &mut pixel.0[..3] {
                            *c = ((u32::from(*c) * 255 + a / 2) / a.max(1)).min(255) as u8;
                        }
                    }
                    resized
                }
            };
            let most = direct
                .pixels()
                .zip(decoded.into_rgba8().pixels())
                .filter(|(a, _)| a[3] != 0)
                .flat_map(|(a, b)| (0..4).map(move |c| a[c].abs_diff(b[c])))
                .max();
            assert!(most <= Some(1), "{}: differs by {most:?}", copy.name);
        }
    }
}

/// What an AVIF names its alpha plane by.
const AVIF_ALPHA: &[u8] = b"auxiliary:alpha";

/// A PNG four pixels wide and two high whose every pixel is `pixel`, but
/// the last of the last row, which is `last`.
fn png_ending(depth: u8, colour: u8, pixel: &[u16], last: &[u16]) -> Vec<u8> {
    let mut samples = pixel.repeat(7);
    samples.extend(last);
    let mut chunks = Vec::new();
    if colour == 3 {
        chunks.push((b"PLTE", vec![10, 20, 30, 40, 50, 60]));
    }
    png_of(4, (depth, colour), &samples, &chunks)
}

/// `png` with `chunk` standing before its pixels.
fn with_chunk(png: &[u8], name: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let at = png.windows(4).position(|w| w == b"IDAT").unwrap() - 4;
    let mut out = png[..at].to_vec();
    out.extend(png_chunk(name, body));
    out.extend(&png[at..]);
    out
}

#[test]
fn a_photograph_is_transparent_only_where_some_pixel_is() {
    let opaque = RgbaImage::from_pixel(4, 2, Rgba([10, 20, 30, 255]));
    let mut pierced = opaque.clone();
    pierced.put_pixel(3, 1, Rgba([10, 20, 30, 254]));
    let palette = png_ending(8, 3, &[0], &[1]);
    let one_entry = png_ending(8, 3, &[0], &[0]);
    let colour = png_ending(8, 2, &[10, 20, 30], &[40, 50, 60]);
    // Each with what its header says of alpha, and what its pixels do.
    let cases = [
        ("PNG, no alpha", colour.clone(), false, false),
        (
            "PNG, alpha unused",
            png_ending(8, 6, &[10, 20, 30, 255], &[10, 20, 30, 255]),
            true,
            false,
        ),
        (
            "PNG, the last pixel of the last row",
            png_ending(8, 6, &[10, 20, 30, 255], &[10, 20, 30, 254]),
            true,
            true,
        ),
        (
            "grey PNG, alpha unused",
            png_ending(8, 4, &[10, 255], &[10, 255]),
            true,
            false,
        ),
        (
            "grey PNG, the last pixel clear",
            png_ending(8, 4, &[10, 255], &[10, 0]),
            true,
            true,
        ),
        (
            "16-bit PNG, alpha unused",
            png_ending(16, 6, &[10, 20, 30, 0xffff], &[10, 20, 30, 0xffff]),
            true,
            false,
        ),
        (
            "16-bit PNG, one level short in the low byte",
            png_ending(16, 6, &[10, 20, 30, 0xffff], &[10, 20, 30, 0xfffe]),
            true,
            true,
        ),
        (
            "16-bit PNG, short in the high byte",
            png_ending(16, 6, &[10, 20, 30, 0xffff], &[10, 20, 30, 0xfeff]),
            true,
            true,
        ),
        (
            "16-bit grey PNG, one level short",
            png_ending(16, 4, &[10, 0xffff], &[10, 0xfffe]),
            true,
            true,
        ),
        ("palette PNG, no tRNS", palette.clone(), false, false),
        (
            "palette PNG, tRNS all opaque",
            with_chunk(&palette, b"tRNS", &[255, 255]),
            true,
            false,
        ),
        (
            "palette PNG, a clear entry in use",
            with_chunk(&palette, b"tRNS", &[255, 0]),
            true,
            true,
        ),
        (
            "palette PNG, a clear entry no pixel uses",
            with_chunk(&one_entry, b"tRNS", &[255, 0]),
            true,
            false,
        ),
        (
            "PNG, a clear colour in use",
            with_chunk(&colour, b"tRNS", &[0, 40, 0, 50, 0, 60]),
            true,
            true,
        ),
        (
            "PNG, a clear colour no pixel has",
            with_chunk(&colour, b"tRNS", &[0, 1, 0, 2, 0, 3]),
            true,
            false,
        ),
        (
            "WebP, alpha unused",
            extended(&opaque, true, 0x10, &[]),
            true,
            false,
        ),
        (
            "WebP, one pixel short of opaque",
            extended(&pierced, true, 0x10, &[]),
            true,
            true,
        ),
        ("lossless WebP", lossless(&pierced, true), true, true),
        ("JPEG", encode_jpeg(&picture(4, 2)), false, false),
    ];
    let settings = Settings::default();
    for (name, bytes, alpha, used) in cases {
        assert_eq!(photograph(&bytes).alpha, alpha, "{name}");
        assert_eq!(transparent(&bytes), used, "{name}");
        // Opening it finds the same, so its copies are those named.
        let opened = open(&bytes).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!(opened.alpha(), used, "{name}");
        let [avif, second] = formats(used);
        // An AVIF holds 8 bits of alpha, and none where those are opaque.
        let at_8_bits = opened_rgba(&opened).iter().any(|pixel| pixel[3] != 255);
        assert_eq!(
            at_8_bits,
            used && !name.contains("one level short"),
            "{name}"
        );
        let made = opened.make(4, avif, &settings).unwrap();
        assert_eq!(contains(&made, AVIF_ALPHA), at_8_bits, "{name}");
        let made = decode(&opened.make(4, second, &settings).unwrap());
        assert_eq!(made.color().has_alpha(), used, "{name}");
        let other = if used { Format::Jpeg } else { Format::Png };
        let refused = opened.make(4, other, &settings);
        let format = other;
        assert_eq!(refused, Err(Error::NotOffered { width: 4, format }));
    }
}

#[test]
fn a_photograph_whose_alpha_cannot_be_looked_at_is_taken_as_transparent() {
    let whole = png_ending(8, 6, &[10, 20, 30, 255], &[10, 20, 30, 255]);
    assert!(!transparent(&whole));
    // Its pixels damaged where they are packed.
    let mut damaged = whole.clone();
    let packed = damaged.windows(4).position(|w| w == b"IDAT").unwrap() + 4;
    damaged[packed + 3] ^= 0xff;
    // Its end missing.
    let cut = &whole[..whole.len() - 14];
    // More pixels than may be decoded, none of them there to read.
    let mut header = 40_000u32.to_be_bytes().to_vec();
    header.extend(40_000u32.to_be_bytes());
    header.extend([8, 6, 0, 0, 0]);
    let mut vast = b"\x89PNG\r\n\x1a\n".to_vec();
    vast.extend(png_chunk(b"IHDR", &header));
    vast.extend(png_chunk(b"IDAT", &stored(&[])));
    vast.extend(png_chunk(b"IEND", &[]));
    for (name, bytes) in [("damaged", &damaged[..]), ("cut", cut), ("vast", &vast)] {
        assert!(photograph(bytes).alpha, "{name}");
        assert!(transparent(bytes), "{name}");
        assert!(open(bytes).is_err(), "{name}");
    }
    // What no header says carries alpha is never looked at.
    let mut jpeg = encode_jpeg(&picture(16, 16));
    jpeg.truncate(jpeg.len() / 2);
    assert!(!transparent(&jpeg));
    assert!(!transparent(b"GIF89a\x01\0\x01\0\0\0\0"));
    assert!(!transparent(b"not a picture"));
}

/// A PNG of a flat colour carrying `profile`, compressed.
fn png_with_profile(profile: Vec<u8>) -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = image::codecs::png::PngEncoder::new(&mut out);
    encoder.set_icc_profile(profile).unwrap();
    let flat = RgbImage::from_pixel(8, 8, Rgb([128, 0, 199]));
    encoder
        .write_image(&flat, 8, 8, ExtendedColorType::Rgb8)
        .unwrap();
    out
}

#[test]
fn a_png_profile_that_inflates_past_the_ceiling_is_not_read() {
    // A profile of noughts is no profile, but one small enough is still
    // found; one over the ceiling is a small file all the same.
    let small = png_with_profile(vec![0; 4096]);
    assert_eq!(photograph(&small).profile, Some(None));
    let large = png_with_profile(vec![0; 8 << 20]);
    assert_eq!(photograph(&large).profile, Some(None));
    let huge = png_with_profile(vec![0; (16 << 20) + 1]);
    assert!(huge.len() < 1 << 20, "{} bytes", huge.len());
    assert!(png_chunks(&huge).contains(b"iCCP"));
    assert_eq!(photograph(&huge).profile, None);
    assert!(!converts(&huge));
    let opened = open(&huge).unwrap();
    assert!(!opened.converted);
    assert_eq!(opened.profile, None);
    assert!(opened_rgb(&opened).pixels().all(|p| p.0 == [128, 0, 199]));
    assert!(decode_still(&huge).is_ok());

    // The decoder is never handed the chunk to inflate for itself.
    let handed = header::png_without_profile(&huge);
    let mut chunks = png_chunks(&huge);
    chunks.retain(|name| name != b"iCCP");
    assert_eq!(png_chunks(&handed), chunks);
    assert!(handed.len() < 1024, "{} bytes", handed.len());
    let plain = small_png();
    assert!(matches!(
        header::png_without_profile(&plain),
        std::borrow::Cow::Borrowed(_)
    ));
}

#[test]
fn a_picture_its_decoder_would_give_more_bytes_than_the_cap_is_refused_first() {
    // At 16 bits a sample it is within the cap as kept, 8-bit RGBA, and
    // over it as its decoder gives it.
    let (width, height) = (16_384u32, 8193u32);
    let pixels = u64::from(width) * u64::from(height);
    assert!(pixels * 4 <= MAX_DECODED_BYTES && pixels * 8 > MAX_DECODED_BYTES);
    let mut header = width.to_be_bytes().to_vec();
    header.extend(height.to_be_bytes());
    header.extend([16, 6, 0, 0, 0]);
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    png.extend(png_chunk(b"IHDR", &header));
    png.extend(png_chunk(b"IDAT", &stored(&[])));
    png.extend(png_chunk(b"IEND", &[]));
    assert_eq!(photograph(&png).stored, size(width, height));
    let refused = Some(Error::TooLarge(size(width, height)));
    assert_eq!(open(&png).err(), refused);
    assert_eq!(decode_still(&png).err(), refused);
}

#[test]
fn a_photograph_made_again_is_a_jpeg_of_its_pixels_alone_within_its_bounds() {
    let bounds = Bounds {
        side: 300,
        bytes: 300 * 200 * 3,
    };
    let jpeg = encode_jpeg(&picture(300, 200));
    let made = remade(&jpeg, 96, bounds, &Settings::default()).unwrap();
    let markers: Vec<u8> = jpeg_markers(&made)
        .iter()
        .map(|(marker, _)| *marker)
        .collect();
    assert!(
        !markers
            .iter()
            .any(|marker| (0xe1..=0xef).contains(marker) || *marker == 0xfe)
    );
    let header = read_header(&made).unwrap();
    let Header::Photograph(photograph) = header else {
        panic!("{header:?}");
    };
    assert_eq!(photograph.stored, size(96, 64));
    // A side too long, or pixels taking too much memory, aren't decoded.
    let narrower = Bounds {
        side: 299,
        ..bounds
    };
    let smaller = Bounds {
        bytes: bounds.bytes - 1,
        ..bounds
    };
    for bounds in [narrower, smaller] {
        assert_eq!(
            remade(&jpeg, 96, bounds, &Settings::default()),
            Err(Error::TooLarge(size(300, 200)))
        );
    }
    // Narrower than asked, it keeps its width.
    let small = remade(
        &encode_jpeg(&picture(40, 30)),
        96,
        bounds,
        &Settings::default(),
    );
    let Header::Photograph(photograph) = read_header(&small.unwrap()).unwrap() else {
        panic!("not a photograph");
    };
    assert_eq!(photograph.stored, size(40, 30));
}

/// An EXIF block from its TIFF header on: a directory pointing at a GPS
/// directory at `gps`, which stands straight after it and holds `tags`,
/// each three rationals of 0 over 1.
fn exif_with_gps(little_endian: bool, gps: u32, tags: &[u16]) -> Vec<u8> {
    let u16b = |v: u16| match little_endian {
        true => v.to_le_bytes(),
        false => v.to_be_bytes(),
    };
    let u32b = |v: u32| match little_endian {
        true => v.to_le_bytes(),
        false => v.to_be_bytes(),
    };
    let mut out = if little_endian { b"II*\0" } else { b"MM\0*" }.to_vec();
    out.extend(u32b(8));
    out.extend(u16b(1));
    out.extend(u16b(0x8825));
    out.extend(u16b(4));
    out.extend(u32b(1));
    out.extend(u32b(gps));
    out.extend(u32b(0));
    let values = out.len() + 2 + tags.len() * 12 + 4;
    out.extend(u16b(tags.len() as u16));
    for &tag in tags {
        out.extend(u16b(tag));
        out.extend(u16b(5));
        out.extend(u32b(3));
        out.extend(u32b(values as u32));
    }
    out.extend(u32b(0));
    for _ in 0..3 {
        out.extend(u32b(0));
        out.extend(u32b(1));
    }
    out
}

/// Where the GPS directory of [`exif_with_gps`] stands.
const GPS: u32 = 26;

#[test]
fn a_location_is_a_latitude_and_a_longitude_in_a_gps_directory_that_reads() {
    let jpeg = |tiff: &[u8]| {
        let body = [b"Exif\0\0", tiff].concat();
        splice(&encode_jpeg(&picture(16, 8)), 0xe1, &body)
    };
    let png_at = |tiff: &[u8], before: &[u8; 4]| {
        let png = small_png();
        let at = png.windows(4).position(|w| w == before).unwrap() - 4;
        [&png[..at], &png_chunk(b"eXIf", tiff), &png[at..]].concat()
    };
    let png = |tiff: &[u8]| png_at(tiff, b"IDAT");
    let png_after = |tiff: &[u8]| png_at(tiff, b"IEND");
    let image = RgbaImage::from_pixel(16, 8, Rgba([10, 20, 30, 255]));
    let webp = |tiff: &[u8]| extended(&image, false, 0x08, &[(b"EXIF", tiff)]);
    let webp_prefixed = |tiff: &[u8]| {
        let body = [b"Exif\0\0", tiff].concat();
        extended(&image, false, 0x08, &[(b"EXIF", &body)])
    };
    type Written<'a> = &'a dyn Fn(&[u8]) -> Vec<u8>;
    let formats: [(&str, Written); 5] = [
        ("JPEG", &jpeg),
        ("PNG", &png),
        ("PNG, after its pixels", &png_after),
        ("WebP", &webp),
        ("WebP, its EXIF as a JPEG opens it", &webp_prefixed),
    ];
    let whole = [1, 2, 3, 4];
    let cases: [(&str, Vec<u8>, bool); 11] = [
        ("a location", exif_with_gps(true, GPS, &whole), true),
        (
            "a location, big end first",
            exif_with_gps(false, GPS, &whole),
            true,
        ),
        (
            "a latitude and a longitude alone",
            exif_with_gps(true, GPS, &[2, 4]),
            true,
        ),
        ("no GPS directory", exif(6, true)[6..].to_vec(), false),
        ("an empty one", exif_with_gps(true, GPS, &[]), false),
        (
            "neither of the two",
            exif_with_gps(true, GPS, &[0, 1, 3]),
            false,
        ),
        ("no longitude", exif_with_gps(true, GPS, &[1, 2, 3]), false),
        ("no latitude", exif_with_gps(false, GPS, &[3, 4]), false),
        (
            "a directory past the end of the block",
            exif_with_gps(true, 4000, &whole),
            false,
        ),
        (
            "a directory far past the end of the file",
            exif_with_gps(false, u32::MAX - 1, &whole),
            false,
        ),
        (
            "a first directory far past the end of the file",
            b"II*\0\xfe\xff\xff\xff".to_vec(),
            false,
        ),
    ];
    for (format, file) in formats {
        for (case, tiff, located) in &cases {
            assert_eq!(holds_location(&file(tiff)), *located, "{format}: {case}");
        }
        // Cut short at every byte of its EXIF, none reads as a location
        // until the whole GPS directory is held, and none stops anything.
        let (tiff, bytes) = (&cases[0].1, file(&cases[0].1));
        let at = bytes.windows(tiff.len()).position(|w| w == &tiff[..]);
        let directory = at.unwrap() + GPS as usize + 2 + whole.len() * 12;
        for cut in 0..bytes.len() {
            let located = holds_location(&bytes[..cut]);
            assert!(!located || cut >= directory, "{format}: cut at {cut}");
        }
        // A directory said to hold more entries than the file has room for.
        let mut crowded = tiff.clone();
        crowded[GPS as usize..GPS as usize + 2].copy_from_slice(&[0xff, 0xff]);
        assert!(!holds_location(&file(&crowded)), "{format}");
        assert!(!holds_location(&file(&[])), "{format}: no EXIF to speak of");
    }
    // No EXIF at all, and files of other kinds or none.
    let gif = b"GIF89a\x01\0\x01\0\0\0\0";
    for bytes in [
        &encode_jpeg(&picture(16, 8))[..],
        &small_png(),
        &extended(&image, false, 0, &[]),
        gif,
        b"",
        b"\xff\xd8\xff",
        &exif_with_gps(true, GPS, &whole),
    ] {
        assert!(!holds_location(bytes));
    }
}
