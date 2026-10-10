//! Decoding a photograph once and making its copies.

use std::borrow::Cow;
use std::io::Cursor;
use std::panic::{AssertUnwindSafe, catch_unwind};

use image::codecs::png::PngEncoder;
use image::imageops::resize;
use image::metadata::Orientation;
use image::{
    DynamicImage, ExtendedColorType, ImageDecoder, ImageEncoder, ImageError, ImageFormat,
    ImageReader, RgbImage, RgbaImage,
};
use moxcms::{ColorProfile, DataColorSpace, Layout, TransformOptions};

use super::header;
use super::{
    Bounds, Error, Format, Header, Kind, PHOTOGRAPH, Photograph, Plain, Settings, Size, Unreadable,
    copy_height,
};

/// A photograph decoded, turned upright and in sRGB, from which any number
/// of copies can be made, from several threads at once.
pub struct Opened {
    pub(super) pixels: Pixels,
    /// Whether the pixels were converted to sRGB from an embedded profile.
    pub converted: bool,
    /// The formats its copies are made in.
    formats: [Format; 2],
    /// Present where a colour profile is embedded, holding its description
    /// where it has one.
    pub profile: Option<Option<String>>,
}

pub(super) enum Pixels {
    Rgb(RgbImage),
    /// Straight alpha, and the same premultiplied for resizing.
    Rgba {
        straight: RgbaImage,
        premultiplied: RgbaImage,
    },
}

/// Decodes the photograph in `bytes`, for its copies.
pub fn open(bytes: &[u8]) -> Result<Opened, Error> {
    let parsed = header::parse(bytes).map_err(Error::Unreadable)?;
    match parsed.header {
        Header::Photograph(photograph) => {
            opened(bytes, photograph, parsed.profile.as_deref(), PHOTOGRAPH)
        }
        Header::Plain(plain) => Err(Error::NotAPhotograph(plain.kind)),
    }
}

/// Whether `pixels`, each of `bytes_per_pixel`, are more than `bounds`
/// allow. Every picture decoded is asked this twice before a pixel is
/// allocated: of what its decoder gives, and of what is kept.
fn too_large(pixels: Size, bytes_per_pixel: u64, bounds: Bounds) -> bool {
    let bytes = u64::from(pixels.width) * u64::from(pixels.height);
    pixels.width.max(pixels.height) > bounds.side
        || bytes.saturating_mul(bytes_per_pixel) > bounds.bytes
}

/// The pixels of the picture in `bytes` that `photograph` describes, as its
/// decoder gives them.
fn decoded(bytes: &[u8], photograph: &Photograph, bounds: Bounds) -> Result<DynamicImage, Error> {
    let stored = photograph.stored;
    // As kept, before a decoder is made at all.
    if too_large(stored, if photograph.alpha { 4 } else { 3 }, bounds) {
        return Err(Error::TooLarge(stored));
    }
    if !header::complete(bytes, photograph.kind) {
        return Err(Error::Unreadable(Unreadable::CutShort));
    }
    let (format, stream) = match photograph.kind {
        Kind::Jpeg => (ImageFormat::Jpeg, Cow::Borrowed(bytes)),
        Kind::Png => (ImageFormat::Png, header::png_without_profile(bytes)),
        _ => (ImageFormat::WebP, Cow::Borrowed(bytes)),
    };
    let decoded = catch_unwind(|| {
        let mut reader = ImageReader::with_format(Cursor::new(&stream[..]), format);
        reader.no_limits();
        let decoder = reader.into_decoder()?;
        // As the decoder gives them, by its own account, before it
        // allocates any.
        let (width, height) = decoder.dimensions();
        let pixels = Size { width, height };
        if pixels != stored {
            return Ok(Err(Error::Unreadable(Unreadable::Unsupported(
                "picture whose header and pixels differ in size",
            ))));
        }
        if too_large(
            pixels,
            u64::from(decoder.color_type().bytes_per_pixel()),
            bounds,
        ) {
            return Ok(Err(Error::TooLarge(stored)));
        }
        DynamicImage::from_decoder(decoder).map(Ok)
    });
    match decoded {
        Ok(Ok(image)) => image,
        Ok(Err(ImageError::Unsupported(_))) => Err(Error::Unreadable(Unreadable::Unsupported(
            "picture in a form its decoder does not read",
        ))),
        Ok(Err(_)) | Err(_) => Err(Error::Unreadable(Unreadable::CutShort)),
    }
}

/// Whether some pixel of `image` is not fully opaque, at the depth its
/// decoder gave: 16 bits of alpha one short of opaque are not opaque,
/// though they round to it at 8.
fn sees_through(image: &DynamicImage) -> bool {
    if !image.color().has_alpha() {
        return false;
    }
    match image {
        DynamicImage::ImageLumaA8(image) => image.pixels().any(|pixel| pixel[1] != u8::MAX),
        DynamicImage::ImageRgba8(image) => image.pixels().any(|pixel| pixel[3] != u8::MAX),
        DynamicImage::ImageLumaA16(image) => image.pixels().any(|pixel| pixel[1] != u16::MAX),
        DynamicImage::ImageRgba16(image) => image.pixels().any(|pixel| pixel[3] != u16::MAX),
        DynamicImage::ImageRgba32F(image) => image.pixels().any(|pixel| pixel[3] < 1.0),
        // A layout not known here keeps whatever alpha it holds.
        _ => true,
    }
}

/// Whether the photograph in `bytes` is transparent anywhere: its header
/// says it carries alpha and some pixel is not fully opaque. One whose
/// header says so and whose pixels can't be read is taken as transparent,
/// since copies that keep alpha lose nothing. A header that says it
/// carries none is answered without decoding a pixel.
pub fn transparent(bytes: &[u8]) -> bool {
    let Ok(parsed) = header::parse(bytes) else {
        return false;
    };
    match parsed.header {
        Header::Photograph(photograph) if photograph.alpha => {
            decoded(bytes, &photograph, PHOTOGRAPH).map_or(true, |image| sees_through(&image))
        }
        _ => false,
    }
}

/// Decodes the picture in `bytes` that `photograph` describes, converting
/// it to sRGB from `profile`.
fn opened(
    bytes: &[u8],
    photograph: Photograph,
    profile: Option<&[u8]>,
    bounds: Bounds,
) -> Result<Opened, Error> {
    let mut image = decoded(bytes, &photograph, bounds)?;
    let transparent = photograph.alpha && sees_through(&image);
    if let Some(orientation) = Orientation::from_exif(photograph.orientation) {
        image.apply_orientation(orientation);
    }
    let converted;
    let pixels = if transparent {
        let mut straight = image.into_rgba8();
        converted = profile.is_some_and(|profile| convert(profile, &mut straight, true));
        let mut premultiplied = straight.clone();
        for pixel in premultiplied.pixels_mut() {
            let a = u32::from(pixel[3]);
            for c in &mut pixel.0[..3] {
                *c = ((u32::from(*c) * a + 127) / 255) as u8;
            }
        }
        Pixels::Rgba {
            straight,
            premultiplied,
        }
    } else {
        let mut image = image.into_rgb8();
        converted = profile.is_some_and(|profile| convert(profile, &mut image, false));
        Pixels::Rgb(image)
    };
    Ok(Opened {
        pixels,
        converted,
        formats: super::formats(transparent),
        profile: photograph.profile,
    })
}

/// Decodes a still JPEG, PNG or WebP in `bytes` to 8-bit RGB in sRGB, turned
/// upright, with any alpha drawn over black. Of an animated PNG, the
/// picture it shows where it is not played.
pub fn decode_still(bytes: &[u8]) -> Result<RgbImage, Error> {
    let mut parsed = header::parse(bytes).map_err(Error::Unreadable)?;
    if let Header::Plain(Plain {
        kind: Kind::AnimatedPng,
        ..
    }) = parsed.header
    {
        parsed = header::png_still(bytes).map_err(Error::Unreadable)?;
    }
    let photograph = match parsed.header {
        Header::Photograph(photograph) => photograph,
        Header::Plain(plain) => return Err(Error::NotAPhotograph(plain.kind)),
    };
    Ok(
        match opened(bytes, photograph, parsed.profile.as_deref(), PHOTOGRAPH)?.pixels {
            Pixels::Rgb(image) => image,
            Pixels::Rgba { premultiplied, .. } => {
                image::DynamicImage::ImageRgba8(premultiplied).into_rgb8()
            }
        },
    )
}

/// Decodes the photograph in `bytes`, within `bounds`, and makes it again
/// from its pixels as a JPEG at most `widest` wide, without any colour
/// profile or metadata, any alpha drawn over black as [`decode_still`]
/// draws it.
pub fn remade(
    bytes: &[u8],
    widest: u32,
    bounds: Bounds,
    settings: &Settings,
) -> Result<Vec<u8>, Error> {
    let parsed = header::parse(bytes).map_err(Error::Unreadable)?;
    let photograph = match parsed.header {
        Header::Photograph(photograph) => photograph,
        Header::Plain(plain) => return Err(Error::NotAPhotograph(plain.kind)),
    };
    let mut opened = opened(bytes, photograph, parsed.profile.as_deref(), bounds)?;
    opened.pixels = match opened.pixels {
        Pixels::Rgba { premultiplied, .. } => {
            Pixels::Rgb(image::DynamicImage::ImageRgba8(premultiplied).into_rgb8())
        }
        rgb => rgb,
    };
    opened.formats = super::formats(false);
    let width = opened.size().width.min(widest);
    opened.make(width, Format::Jpeg, settings)
}

/// [`to_srgb`], answering false where reading the profile panics.
fn convert(profile: &[u8], pixels: &mut [u8], alpha: bool) -> bool {
    catch_unwind(AssertUnwindSafe(|| to_srgb(profile, pixels, alpha))).unwrap_or(false)
}

/// Whether [`open`] converts the photograph in `bytes` to sRGB: it embeds a
/// colour profile that reads and is not sRGB in all but name. Told from the
/// header and the profile, without decoding a pixel.
pub fn converts(bytes: &[u8]) -> bool {
    let Ok(parsed) = header::parse(bytes) else {
        return false;
    };
    let Header::Photograph(photograph) = parsed.header else {
        return false;
    };
    let Some(profile) = parsed.profile else {
        return false;
    };
    catch_unwind(|| transform(&profile, photograph.alpha).is_some()).unwrap_or(false)
}

type Transform = std::sync::Arc<moxcms::Transform8BitExecutor>;

/// What converts 8-bit pixels to sRGB from `profile`, and whether the
/// profile describes grey alone. None where the profile does not read, is
/// of no colour space converted here, or is sRGB in all but name.
fn transform(profile: &[u8], alpha: bool) -> Option<(Transform, bool)> {
    let source = ColorProfile::new_from_slice(profile).ok()?;
    let grey = match source.color_space {
        DataColorSpace::Rgb => false,
        DataColorSpace::Gray => true,
        _ => return None,
    };
    // A profile whose transform cannot be made is left unused, as one that
    // does not read is.
    if is_srgb(&source, grey).unwrap_or(true) {
        return None;
    }
    let srgb = ColorProfile::new_srgb();
    let options = TransformOptions::default();
    let (from, to) = match (grey, alpha) {
        (false, false) => (Layout::Rgb, Layout::Rgb),
        (false, true) => (Layout::Rgba, Layout::Rgba),
        (true, false) => (Layout::Gray, Layout::Rgb),
        (true, true) => (Layout::GrayAlpha, Layout::Rgba),
    };
    let transform = source
        .create_transform_8bit(from, &srgb, to, options)
        .ok()?;
    Some((transform, grey))
}

/// Converts `pixels` to sRGB from `profile`, unless the profile is sRGB in
/// all but name. Answers whether they were converted.
fn to_srgb(profile: &[u8], pixels: &mut [u8], alpha: bool) -> bool {
    let Some((transform, grey)) = transform(profile, alpha) else {
        return false;
    };
    // A grey profile describes one channel; the decoder has copied it into
    // red, green and blue alike.
    let input: Vec<u8> = match (grey, alpha) {
        (false, _) => pixels.to_vec(),
        (true, false) => pixels.iter().step_by(3).copied().collect(),
        (true, true) => pixels
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|pixel| [pixel[0], pixel[3]])
            .collect(),
    };
    let mut output = vec![0; pixels.len()];
    if transform.transform(&input, &mut output).is_err() {
        return false;
    }
    pixels.copy_from_slice(&output);
    true
}

/// Whether `profile` maps every sample to sRGB within one 8-bit level: 1,024
/// levels of each channel alone and of grey, and a 17 by 17 by 17 lattice.
/// None where its transform cannot be made.
fn is_srgb(profile: &ColorProfile, grey: bool) -> Option<bool> {
    let srgb = ColorProfile::new_srgb();
    let levels = (0..1024).map(|i| i as f32 / 1023.0);
    let mut samples: Vec<[f32; 3]> = levels.clone().map(|v| [v, v, v]).collect();
    if !grey {
        for channel in 0..3 {
            samples.extend(levels.clone().map(|v| {
                let mut sample = [0.0; 3];
                sample[channel] = v;
                sample
            }));
        }
        let steps = (0..17).map(|i| i as f32 / 16.0);
        for r in steps.clone() {
            for g in steps.clone() {
                samples.extend(steps.clone().map(|b| [r, g, b]));
            }
        }
    }
    let (layout, input): (_, Vec<f32>) = if grey {
        (Layout::Gray, samples.iter().map(|s| s[0]).collect())
    } else {
        (Layout::Rgb, samples.iter().flatten().copied().collect())
    };
    let transform = profile
        .create_transform_f32(layout, &srgb, Layout::Rgb, TransformOptions::default())
        .ok()?;
    let mut output = vec![0.0; samples.len() * 3];
    transform.transform(&input, &mut output).ok()?;
    let expected = samples.iter().flatten();
    Some(
        output
            .iter()
            .zip(expected)
            .all(|(out, expected)| ((out - expected) * 255.0).abs() <= 1.0),
    )
}

impl Opened {
    /// The photograph's size as shown.
    pub fn size(&self) -> Size {
        let (width, height) = match &self.pixels {
            Pixels::Rgb(image) => image.dimensions(),
            Pixels::Rgba { straight, .. } => straight.dimensions(),
        };
        Size { width, height }
    }

    /// Whether it is transparent anywhere, and so held with its alpha.
    pub fn alpha(&self) -> bool {
        matches!(self.pixels, Pixels::Rgba { .. })
    }

    /// Makes the copy `width` wide in `format`, without any colour profile
    /// or metadata.
    pub fn make(&self, width: u32, format: Format, settings: &Settings) -> Result<Vec<u8>, Error> {
        self.resize(width, settings)
            .map_err(|_| Error::NotOffered { width, format })?
            .encode(format, settings)
    }

    /// The photograph `width` wide, from which that width's copy in each
    /// format is encoded: resizing is the larger part of a small copy's
    /// cost, and is the same for every format.
    pub fn resize(&self, width: u32, settings: &Settings) -> Result<Resized<'_>, Error> {
        let shown = self.size();
        if width == 0 || width > shown.width {
            let format = Format::Avif;
            return Err(Error::NotOffered { width, format });
        }
        let height = copy_height(shown, width);
        let pixels = match &self.pixels {
            Pixels::Rgb(image) if width == shown.width => Cow::Borrowed(image.as_raw().as_slice()),
            Pixels::Rgb(image) => {
                Cow::Owned(resize(image, width, height, settings.resize).into_raw())
            }
            Pixels::Rgba { straight, .. } if width == shown.width => {
                Cow::Borrowed(straight.as_raw().as_slice())
            }
            Pixels::Rgba { premultiplied, .. } => {
                let mut image = resize(premultiplied, width, height, settings.resize);
                for pixel in image.pixels_mut() {
                    let a = u32::from(pixel[3]);
                    if a == 0 {
                        continue;
                    }
                    for c in &mut pixel.0[..3] {
                        *c = ((u32::from(*c) * 255 + a / 2) / a).min(255) as u8;
                    }
                }
                Cow::Owned(image.into_raw())
            }
        };
        Ok(Resized {
            pixels,
            width,
            height,
            alpha: self.alpha(),
            formats: self.formats,
        })
    }
}

/// A photograph at one copy's size, ready to encode.
pub struct Resized<'a> {
    pixels: Cow<'a, [u8]>,
    width: u32,
    height: u32,
    alpha: bool,
    formats: [Format; 2],
}

impl Resized<'_> {
    /// Encodes the copy in `format`, without any colour profile or
    /// metadata.
    pub fn encode(&self, format: Format, settings: &Settings) -> Result<Vec<u8>, Error> {
        let (width, height) = (self.width, self.height);
        let pixels: &[u8] = &self.pixels;
        if !self.formats.contains(&format) {
            return Err(Error::NotOffered { width, format });
        }
        let encode_error = |message: String| Error::Encode { format, message };
        match format {
            Format::Avif => {
                let image = avif_aom_sys::Image {
                    pixels,
                    width,
                    height,
                    layout: if self.alpha {
                        avif_aom_sys::Layout::Rgba
                    } else {
                        avif_aom_sys::Layout::Rgb
                    },
                };
                let avif = avif_aom_sys::Settings {
                    speed: settings.avif_speed,
                    quality: settings.avif_quality,
                    alpha_quality: settings.avif_alpha_quality,
                    threads: settings.avif_threads,
                };
                avif_aom_sys::encode(&image, &avif).map_err(|e| encode_error(e.to_string()))
            }
            Format::Jpeg => {
                let too_large = || encode_error("too large for JPEG".into());
                let width = u16::try_from(width).map_err(|_| too_large())?;
                let height = u16::try_from(height).map_err(|_| too_large())?;
                let mut out = Vec::new();
                let mut encoder = jpeg_encoder::Encoder::new(&mut out, settings.jpeg_quality);
                encoder.set_sampling_factor(settings.jpeg_subsampling);
                encoder.set_chroma_subsampling_method(settings.jpeg_chroma_method);
                encoder.set_progressive(settings.jpeg_progressive);
                encoder.set_optimized_huffman_tables(settings.jpeg_optimised_huffman);
                encoder
                    .encode(pixels, width, height, jpeg_encoder::ColorType::Rgb)
                    .map_err(|e| encode_error(e.to_string()))?;
                Ok(out)
            }
            Format::Png => {
                let mut out = Vec::new();
                PngEncoder::new_with_quality(
                    &mut out,
                    settings.png_compression,
                    settings.png_filter,
                )
                .write_image(
                    pixels,
                    width,
                    height,
                    match self.alpha {
                        true => ExtendedColorType::Rgba8,
                        false => ExtendedColorType::Rgb8,
                    },
                )
                .map_err(|e| encode_error(e.to_string()))?;
                Ok(out)
            }
        }
    }
}
