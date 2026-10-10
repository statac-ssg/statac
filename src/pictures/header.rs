//! Reading a picture's kind and size from its first bytes, without decoding
//! any pixel.

use std::borrow::Cow;
use std::io::Cursor;
use std::panic::catch_unwind;

use image::codecs::png::PngDecoder;
use image::{ImageDecoder, Limits};

use super::{Header, Kind, NoSize, Photograph, Plain, Size, Unreadable};

/// What reading a header finds, with the photograph's colour profile kept
/// for opening it.
pub(super) struct Parsed {
    pub header: Header,
    pub profile: Option<Vec<u8>>,
}

pub(super) fn parse(bytes: &[u8]) -> Result<Parsed, Unreadable> {
    let plain = |kind, size| Parsed {
        header: Header::Plain(Plain { kind, size }),
        profile: None,
    };
    match sniff(bytes)? {
        Kind::Jpeg => jpeg(bytes),
        Kind::WebP => webp(bytes),
        Kind::Png => {
            let (still, animated) = png(bytes)?;
            match (animated, &still.header) {
                (true, Header::Photograph(first)) => Ok(plain(Kind::AnimatedPng, Ok(first.stored))),
                _ => Ok(still),
            }
        }
        kind @ (Kind::Gif | Kind::Svg | Kind::Avif) => {
            let size = match kind {
                Kind::Gif => gif(bytes).map_err(NoSize::Unreadable),
                Kind::Svg => svg(bytes),
                _ => avif(bytes),
            };
            Ok(plain(kind, size))
        }
        kind => Ok(plain(kind, Err(NoSize::NotRead))),
    }
}

/// The kind of picture `bytes` begin, telling a still WebP from an animated
/// one only later, from its extended header.
pub(super) fn sniff(bytes: &[u8]) -> Result<Kind, Unreadable> {
    const SIGNATURES: [(&[u8], Kind); 4] = [
        (b"\xff\xd8\xff", Kind::Jpeg),
        (b"\x89PNG\r\n\x1a\n", Kind::Png),
        (b"GIF87a", Kind::Gif),
        (b"GIF89a", Kind::Gif),
    ];
    for (signature, kind) in SIGNATURES {
        if bytes.starts_with(signature) {
            return Ok(kind);
        }
    }
    if bytes.starts_with(b"RIFF") && bytes.get(8..12).is_some_and(|t| t == b"WEBP") {
        return Ok(Kind::WebP);
    }
    if bytes.get(4..8).is_some_and(|t| t == b"ftyp") {
        return brand(bytes);
    }
    if is_svg(bytes) {
        return Ok(Kind::Svg);
    }
    let cut = |signature: &[u8]| !bytes.is_empty() && signature.starts_with(bytes);
    if SIGNATURES.iter().any(|(signature, _)| cut(signature)) || cut(b"RIFF") {
        return Err(Unreadable::CutShort);
    }
    Err(Unreadable::NotAPicture)
}

/// An ISO media file's kind, from the brands in its `ftyp` box.
fn brand(bytes: &[u8]) -> Result<Kind, Unreadable> {
    let size = be32(bytes, 0)? as usize;
    let end = size.min(bytes.len());
    if size < 16 {
        return Err(Unreadable::Unsupported(
            "ISO media file with a short ftyp box",
        ));
    }
    let major = bytes.get(8..12).ok_or(Unreadable::CutShort)?;
    let compatible = bytes.get(16..end).unwrap_or_default().as_chunks::<4>().0;
    let brands: Vec<&[u8]> = std::iter::once(major)
        .chain(compatible.iter().map(|brand| &brand[..]))
        .collect();
    let any = |names: &[&[u8; 4]]| brands.iter().any(|b| names.iter().any(|n| b == n));
    if any(&[b"avif", b"avis"]) {
        Ok(Kind::Avif)
    } else if any(&[
        b"heic", b"heix", b"heim", b"heis", b"hevc", b"hevx", b"hevm", b"hevs",
    ]) {
        Ok(Kind::Heic)
    } else if any(&[b"mif1", b"msf1"]) {
        Ok(Kind::Heif)
    } else {
        Err(Unreadable::NotAPicture)
    }
}

/// The `length` bytes of `bytes` from `at`, where it holds them all:
/// every place and length read from a file is asked for through this,
/// whatever the file says of itself.
fn held(bytes: &[u8], at: usize, length: usize) -> Result<&[u8], Unreadable> {
    let from = bytes.get(at..).ok_or(Unreadable::CutShort)?;
    from.get(..length).ok_or(Unreadable::CutShort)
}

fn be16(bytes: &[u8], at: usize) -> Result<u16, Unreadable> {
    match held(bytes, at, 2)? {
        &[a, b] => Ok(u16::from_be_bytes([a, b])),
        _ => Err(Unreadable::CutShort),
    }
}

fn be32(bytes: &[u8], at: usize) -> Result<u32, Unreadable> {
    match held(bytes, at, 4)? {
        &[a, b, c, d] => Ok(u32::from_be_bytes([a, b, c, d])),
        _ => Err(Unreadable::CutShort),
    }
}

fn le16(bytes: &[u8], at: usize) -> Result<u16, Unreadable> {
    be16(bytes, at).map(u16::swap_bytes)
}

fn le24(bytes: &[u8], at: usize) -> Result<u32, Unreadable> {
    match held(bytes, at, 3)? {
        &[a, b, c] => Ok(u32::from_le_bytes([a, b, c, 0])),
        _ => Err(Unreadable::CutShort),
    }
}

fn le32(bytes: &[u8], at: usize) -> Result<u32, Unreadable> {
    be32(bytes, at).map(u32::swap_bytes)
}

fn photograph(
    kind: Kind,
    stored: Size,
    orientation: u8,
    alpha: bool,
    profile: Option<Vec<u8>>,
) -> Result<Parsed, Unreadable> {
    if stored.width == 0 || stored.height == 0 {
        return Err(Unreadable::Unsupported("picture without pixels"));
    }
    let shown = if orientation >= 5 {
        Size {
            width: stored.height,
            height: stored.width,
        }
    } else {
        stored
    };
    let description = profile.as_deref().map(description);
    Ok(Parsed {
        header: Header::Photograph(Photograph {
            kind,
            stored,
            orientation,
            shown,
            alpha,
            profile: description,
        }),
        profile,
    })
}

fn jpeg(bytes: &[u8]) -> Result<Parsed, Unreadable> {
    let mut at = 2;
    let mut orientation = 1;
    let mut chunks: Vec<(u8, &[u8])> = Vec::new();
    loop {
        if *bytes.get(at).ok_or(Unreadable::CutShort)? != 0xff {
            return Err(Unreadable::Unsupported(
                "JPEG with bytes between its segments",
            ));
        }
        while bytes.get(at + 1) == Some(&0xff) {
            at += 1;
        }
        let marker = *bytes.get(at + 1).ok_or(Unreadable::CutShort)?;
        at += 2;
        if matches!(marker, 0x01 | 0xd0..=0xd7) {
            continue;
        }
        if matches!(marker, 0xd9 | 0xda) {
            return Err(Unreadable::Unsupported("JPEG without a frame header"));
        }
        let length = usize::from(be16(bytes, at)?);
        if length < 2 {
            return Err(Unreadable::Unsupported(
                "JPEG segment shorter than its length",
            ));
        }
        let body = bytes.get(at + 2..at + length).ok_or(Unreadable::CutShort)?;
        match marker {
            0xe1 if body.starts_with(b"Exif\0\0") => orientation = exif_orientation(&body[6..]),
            0xe2 if body.starts_with(b"ICC_PROFILE\0") && body.len() >= 14 => {
                chunks.push((body[12], &body[14..]));
            }
            0xc0..=0xc2 => {
                let precision = *body.first().ok_or(Unreadable::CutShort)?;
                if precision != 8 {
                    return Err(Unreadable::Unsupported(
                        "JPEG with more than 8 bits a sample",
                    ));
                }
                let stored = Size {
                    height: u32::from(be16(body, 1)?),
                    width: u32::from(be16(body, 3)?),
                };
                if stored.height == 0 {
                    return Err(Unreadable::Unsupported(
                        "JPEG whose height follows its scan",
                    ));
                }
                chunks.sort_by_key(|&(sequence, _)| sequence);
                let profile = (!chunks.is_empty()).then(|| {
                    chunks
                        .iter()
                        .flat_map(|&(_, chunk)| chunk)
                        .copied()
                        .collect()
                });
                return photograph(Kind::Jpeg, stored, orientation, false, profile);
            }
            0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf => {
                return Err(Unreadable::Unsupported(
                    "JPEG coded losslessly, hierarchically or arithmetically",
                ));
            }
            _ => {}
        }
        at += length;
    }
}

/// An EXIF block from its TIFF header on, read in the byte order it
/// opens with.
#[derive(Clone, Copy)]
struct Tiff<'a> {
    bytes: &'a [u8],
    little: bool,
}

impl<'a> Tiff<'a> {
    fn of(bytes: &'a [u8]) -> Option<Tiff<'a>> {
        let little = match bytes.get(..4) {
            Some(b"II*\0") => true,
            Some(b"MM\0*") => false,
            _ => return None,
        };
        Some(Tiff { bytes, little })
    }

    fn u16_at(self, at: usize) -> Result<u16, Unreadable> {
        if self.little {
            le16(self.bytes, at)
        } else {
            be16(self.bytes, at)
        }
    }

    fn u32_at(self, at: usize) -> Result<u32, Unreadable> {
        if self.little {
            le32(self.bytes, at)
        } else {
            be32(self.bytes, at)
        }
    }

    /// Where each entry of the directory at `ifd` stands. None is known to
    /// be held.
    fn entries(self, ifd: usize) -> Result<impl Iterator<Item = usize>, Unreadable> {
        let count = usize::from(self.u16_at(ifd)?);
        Ok((0..count).map(move |entry| ifd.saturating_add(2 + entry * 12)))
    }
}

/// The orientation in an EXIF block, from its TIFF header on, or 1 where
/// it gives none that reads.
fn exif_orientation(tiff: &[u8]) -> u8 {
    let read = || -> Result<u8, Unreadable> {
        let Some(tiff) = Tiff::of(tiff) else {
            return Ok(1);
        };
        let ifd = tiff.u32_at(4)? as usize;
        for at in tiff.entries(ifd)? {
            if tiff.u16_at(at)? == 0x0112 && tiff.u16_at(at.saturating_add(2))? == 3 {
                return Ok(match tiff.u16_at(at.saturating_add(8))? {
                    value @ 1..=8 => value as u8,
                    _ => 1,
                });
            }
        }
        Ok(1)
    };
    read().unwrap_or(1)
}

/// Whether an EXIF block, from its TIFF header on, gives a position: its
/// GPS directory holds a latitude and a longitude. False where any of it
/// does not read.
fn exif_located(tiff: &[u8]) -> bool {
    let read = || -> Result<bool, Unreadable> {
        let Some(tiff) = Tiff::of(tiff) else {
            return Ok(false);
        };
        let ifd = tiff.u32_at(4)? as usize;
        for at in tiff.entries(ifd)? {
            if tiff.u16_at(at)? != 0x8825 {
                continue;
            }
            let gps = tiff.u32_at(at.saturating_add(8))? as usize;
            let (mut latitude, mut longitude) = (false, false);
            for at in tiff.entries(gps)? {
                match tiff.u16_at(at)? {
                    0x0002 => latitude = true,
                    0x0004 => longitude = true,
                    _ => {}
                }
            }
            return Ok(latitude && longitude);
        }
        Ok(false)
    };
    read().unwrap_or(false)
}

/// Whether a JPEG, PNG or WebP holds where it was taken, in its EXIF.
/// False for any other file, and for one that does not read.
pub(super) fn located(bytes: &[u8]) -> bool {
    let read = || -> Result<bool, Unreadable> {
        match sniff(bytes)? {
            Kind::Jpeg => jpeg_located(bytes),
            Kind::Png => png_located(bytes),
            Kind::WebP => {
                for chunk in riff_chunks(bytes) {
                    let (name, body) = chunk?;
                    if &name == b"EXIF" {
                        let tiff = body.strip_prefix(b"Exif\0\0").unwrap_or(body);
                        return Ok(exif_located(tiff));
                    }
                }
                Ok(false)
            }
            _ => Ok(false),
        }
    };
    read().unwrap_or(false)
}

/// [`located`] of a JPEG, read as far as its first scan.
fn jpeg_located(bytes: &[u8]) -> Result<bool, Unreadable> {
    let mut at = 2;
    loop {
        if *bytes.get(at).ok_or(Unreadable::CutShort)? != 0xff {
            return Ok(false);
        }
        while bytes.get(at + 1) == Some(&0xff) {
            at += 1;
        }
        let marker = *bytes.get(at + 1).ok_or(Unreadable::CutShort)?;
        at += 2;
        if matches!(marker, 0x01 | 0xd0..=0xd7) {
            continue;
        }
        if matches!(marker, 0xd9 | 0xda) {
            return Ok(false);
        }
        let length = usize::from(be16(bytes, at)?);
        let body = held(bytes, at, length)?.get(2..);
        let body = body.ok_or(Unreadable::CutShort)?;
        if marker == 0xe1
            && let Some(tiff) = body.strip_prefix(b"Exif\0\0")
        {
            return Ok(exif_located(tiff));
        }
        at += length;
    }
}

/// [`located`] of a PNG, whose `eXIf` chunk may follow its pixels.
fn png_located(bytes: &[u8]) -> Result<bool, Unreadable> {
    let mut at = 8;
    loop {
        let (name, next) = png_chunk(bytes, at)?;
        match name {
            b"IEND" => return Ok(false),
            b"eXIf" => {
                let body = held(bytes, at + 8, next - at - 12)?;
                return Ok(exif_located(body));
            }
            _ => {}
        }
        at = next;
    }
}

/// The text of an ICC profile's `desc` tag, in English where it is written
/// in several languages.
fn description(profile: &[u8]) -> Option<String> {
    let count = be32(profile, 128).ok()? as usize;
    let (offset, size) = (0..count.min(1024)).find_map(|n| {
        let at = 132 + n * 12;
        (profile.get(at..at + 4)? == b"desc")
            .then(|| Some((be32(profile, at + 4).ok()?, be32(profile, at + 8).ok()?)))?
    })?;
    let tag = profile.get(offset as usize..)?.get(..size as usize)?;
    let text = match tag.get(..4)? {
        b"desc" => {
            let length = be32(tag, 8).ok()? as usize;
            let ascii = held(tag, 12, length).ok()?;
            let ascii = ascii.split(|&b| b == 0).next().unwrap_or_default();
            String::from_utf8_lossy(ascii).into_owned()
        }
        b"mluc" => {
            let records = be32(tag, 8).ok()? as usize;
            let record = |n: usize| -> Option<(&[u8], String)> {
                let at = 16 + n * 12;
                let language = tag.get(at..at + 2)?;
                let length = be32(tag, at + 4).ok()? as usize;
                let offset = be32(tag, at + 8).ok()? as usize;
                let units: Vec<u16> = held(tag, offset, length)
                    .ok()?
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|&pair| u16::from_be_bytes(pair))
                    .collect();
                Some((language, String::from_utf16_lossy(&units)))
            };
            let all: Vec<_> = (0..records.min(256)).filter_map(record).collect();
            let english = all.iter().find(|(language, _)| *language == b"en");
            english.or(all.first())?.1.clone()
        }
        _ => return None,
    };
    let text = text.trim_end_matches('\0').trim();
    (!text.is_empty()).then(|| text.to_owned())
}

/// The chunks of a RIFF file after its 12-byte header, each as its type and
/// its body, ending with an error where one is cut short.
fn riff_chunks(bytes: &[u8]) -> impl Iterator<Item = Result<([u8; 4], &[u8]), Unreadable>> {
    let mut at = 12;
    std::iter::from_fn(move || {
        if at >= bytes.len() {
            return None;
        }
        let chunk = (|| {
            let name = held(bytes, at, 4)?;
            let size = le32(bytes, at + 4)? as usize;
            let body = held(bytes, at + 8, size)?;
            Ok(([name[0], name[1], name[2], name[3]], body))
        })();
        match &chunk {
            Ok((_, body)) => at += 8 + body.len() + body.len() % 2,
            Err(_) => at = bytes.len(),
        }
        Some(chunk)
    })
}

fn webp(bytes: &[u8]) -> Result<Parsed, Unreadable> {
    let (name, body) = riff_chunks(bytes).next().ok_or(Unreadable::CutShort)??;
    match &name {
        b"VP8 " => {
            if body.len() < 10 {
                return Err(Unreadable::CutShort);
            }
            if body[0] & 1 != 0 || body[3..6] != [0x9d, 0x01, 0x2a] {
                return Err(Unreadable::Unsupported(
                    "WebP whose first frame is not a key frame",
                ));
            }
            let stored = Size {
                width: u32::from(le16(body, 6)? & 0x3fff),
                height: u32::from(le16(body, 8)? & 0x3fff),
            };
            photograph(Kind::WebP, stored, 1, false, None)
        }
        b"VP8L" => {
            if body.len() < 5 {
                return Err(Unreadable::CutShort);
            }
            if body[0] != 0x2f {
                return Err(Unreadable::Unsupported(
                    "lossless WebP without its signature",
                ));
            }
            let bits = le32(body, 1)?;
            if bits >> 29 != 0 {
                return Err(Unreadable::Unsupported("lossless WebP of a later version"));
            }
            let stored = Size {
                width: (bits & 0x3fff) + 1,
                height: ((bits >> 14) & 0x3fff) + 1,
            };
            photograph(Kind::WebP, stored, 1, bits >> 28 & 1 == 1, None)
        }
        b"VP8X" => {
            if body.len() < 10 {
                return Err(Unreadable::CutShort);
            }
            let flags = body[0];
            let canvas = Size {
                width: le24(body, 4)? + 1,
                height: le24(body, 7)? + 1,
            };
            if flags & 0x02 != 0 {
                return Ok(Parsed {
                    header: Header::Plain(Plain {
                        kind: Kind::AnimatedWebP,
                        size: Ok(canvas),
                    }),
                    profile: None,
                });
            }
            let mut profile = None;
            let mut orientation = 1;
            let wanted = |flag: u8| flags & flag != 0;
            let (mut want_profile, mut want_exif) = (wanted(0x20), wanted(0x08));
            for chunk in riff_chunks(bytes).skip(1) {
                if !want_profile && !want_exif {
                    break;
                }
                match chunk? {
                    (name, body) if &name == b"ICCP" => {
                        profile = Some(body.to_vec());
                        want_profile = false;
                    }
                    (name, body) if &name == b"EXIF" => {
                        let tiff = body.strip_prefix(b"Exif\0\0").unwrap_or(body);
                        orientation = exif_orientation(tiff);
                        want_exif = false;
                    }
                    _ => {}
                }
            }
            photograph(Kind::WebP, canvas, orientation, wanted(0x10), profile)
        }
        _ => Err(Unreadable::Unsupported(
            "WebP without a VP8, VP8L or VP8X chunk",
        )),
    }
}

/// The most a PNG's colour profile, with the chunk it is compressed in,
/// may take to read. No profile in use comes near it.
const PROFILE_CEILING: usize = 16 << 20;

/// A PNG's chunk at `at`: its type, and where the chunk after it begins.
/// Only its type is known to be held; its body may be cut short.
fn png_chunk(bytes: &[u8], at: usize) -> Result<(&[u8], usize), Unreadable> {
    let length = be32(bytes, at)? as usize;
    let name = held(bytes, at.saturating_add(4), 4)?;
    let next = at.checked_add(12).and_then(|at| at.checked_add(length));
    Ok((name, next.ok_or(Unreadable::CutShort)?))
}

/// A PNG read as a still picture, and whether it is animated: an `acTL`
/// chunk stands before its pixels. Only the chunks before the first `IDAT`
/// are read.
fn png(bytes: &[u8]) -> Result<(Parsed, bool), Unreadable> {
    if held(bytes, 12, 4)? != b"IHDR" {
        return Err(Unreadable::Unsupported(
            "PNG not starting with its IHDR chunk",
        ));
    }
    let stored = Size {
        width: be32(bytes, 16)?,
        height: be32(bytes, 20)?,
    };
    let colour = *bytes.get(25).ok_or(Unreadable::CutShort)?;
    if !matches!(colour, 0 | 2 | 3 | 4 | 6) {
        return Err(Unreadable::Unsupported("PNG of an unknown colour type"));
    }
    let mut alpha = matches!(colour, 4 | 6);
    let mut animated = false;
    let mut profile = None;
    let mut at = 8;
    loop {
        let (name, next) = png_chunk(bytes, at)?;
        match name {
            b"IDAT" => break,
            b"IEND" => return Err(Unreadable::Unsupported("PNG without an IDAT chunk")),
            b"acTL" => animated = true,
            b"tRNS" => alpha = true,
            b"iCCP" if profile.is_none() => {
                profile = Some(held(bytes, at, next - at)?);
            }
            _ => {}
        }
        at = next;
    }
    let profile = profile.and_then(png_profile);
    Ok((photograph(Kind::Png, stored, 1, alpha, profile)?, animated))
}

/// The colour profile an `iCCP` chunk holds, inflated; none where it does
/// not inflate, or would take more than [`PROFILE_CEILING`].
fn png_profile(chunk: &[u8]) -> Option<Vec<u8>> {
    // A PNG of one grey pixel, to stand around the chunk: the decoder
    // inflates a profile only as part of a PNG, and inflates no further
    // than the limit it is given.
    const BEFORE: &[u8] =
        b"\x89PNG\r\n\x1a\n\0\0\0\x0dIHDR\0\0\0\x01\0\0\0\x01\x08\0\0\0\0\x3a\x7e\x9b\x55";
    const AFTER: &[u8] =
        b"\0\0\0\x0aIDAT\x78\x9c\x63\x60\0\0\0\x02\0\x01\x48\xaf\xa4\x71\0\0\0\0IEND\xae\x42\x60\x82";
    if chunk.len() > PROFILE_CEILING {
        return None;
    }
    let png = [BEFORE, chunk, AFTER].concat();
    let mut limits = Limits::no_limits();
    limits.max_alloc = Some(PROFILE_CEILING as u64);
    catch_unwind(|| {
        let decoder = PngDecoder::with_limits(Cursor::new(&png), limits);
        decoder.and_then(|mut decoder| decoder.icc_profile()).ok()?
    })
    .ok()?
}

/// A PNG as its decoder is handed it: without any `iCCP` chunk, which the
/// decoder would inflate however large it came out. The profile is read
/// apart, by [`png_profile`].
pub(super) fn png_without_profile(bytes: &[u8]) -> Cow<'_, [u8]> {
    let mut kept: Vec<u8> = Vec::new();
    let (mut at, mut copied) = (8, 0);
    while let Ok((name, next)) = png_chunk(bytes, at) {
        if name == b"IDAT" || next > bytes.len() {
            break;
        }
        if name == b"iCCP" {
            kept.extend(&bytes[copied..at]);
            copied = next;
        }
        at = next;
    }
    match copied {
        0 => Cow::Borrowed(bytes),
        _ => {
            kept.extend(&bytes[copied..]);
            Cow::Owned(kept)
        }
    }
}

/// A PNG read as a still picture, though it may be animated.
pub(super) fn png_still(bytes: &[u8]) -> Result<Parsed, Unreadable> {
    png(bytes).map(|(still, _)| still)
}

/// Whether a PNG's chunks run whole to its `IEND`.
fn png_ends(bytes: &[u8]) -> bool {
    let mut at = 8;
    loop {
        let Ok(length) = be32(bytes, at) else {
            return false;
        };
        let end = at.saturating_add(length as usize).saturating_add(12);
        if end > bytes.len() {
            return false;
        }
        if &bytes[at + 4..at + 8] == b"IEND" {
            return true;
        }
        at = end;
    }
}

fn gif(bytes: &[u8]) -> Result<Size, Unreadable> {
    Ok(Size {
        width: u32::from(le16(bytes, 6)?),
        height: u32::from(le16(bytes, 8)?),
    })
}

/// The boxes laid one after another in `bytes`, each as its type and body.
fn boxes(bytes: &[u8]) -> impl Iterator<Item = Result<([u8; 4], &[u8]), Unreadable>> {
    let mut at = 0;
    std::iter::from_fn(move || {
        if at >= bytes.len() {
            return None;
        }
        let found = (|| {
            let size = be32(bytes, at)? as usize;
            let name = bytes.get(at + 4..at + 8).ok_or(Unreadable::CutShort)?;
            let (start, size) = match size {
                0 => (at + 8, bytes.len() - at),
                1 => {
                    let high = u64::from(be32(bytes, at + 8)?);
                    let low = u64::from(be32(bytes, at + 12)?);
                    let size = usize::try_from(high << 32 | low).unwrap_or(usize::MAX);
                    (at + 16, size)
                }
                size => (at + 8, size),
            };
            let end = at.checked_add(size).ok_or(Unreadable::CutShort)?;
            let body = bytes.get(start..end).ok_or(Unreadable::CutShort)?;
            Ok(([name[0], name[1], name[2], name[3]], body, end))
        })();
        match found {
            Ok((name, body, end)) if end > at => {
                at = end;
                Some(Ok((name, body)))
            }
            Ok(_) => {
                at = bytes.len();
                Some(Err(Unreadable::Unsupported(
                    "ISO media box shorter than its header",
                )))
            }
            Err(e) => {
                at = bytes.len();
                Some(Err(e))
            }
        }
    })
}

fn child<'a>(bytes: &'a [u8], name: &[u8; 4]) -> Result<Option<&'a [u8]>, Unreadable> {
    for found in boxes(bytes) {
        let (found, body) = found?;
        if &found == name {
            return Ok(Some(body));
        }
    }
    Ok(None)
}

/// An AVIF's size from its primary item's `ispe`, turned by its `irot`.
fn avif(bytes: &[u8]) -> Result<Size, NoSize> {
    avif_properties(bytes).map_err(NoSize::Unreadable)?
}

fn avif_properties(bytes: &[u8]) -> Result<Result<Size, NoSize>, Unreadable> {
    let missing = Unreadable::Unsupported("AVIF without an image size");
    let meta = child(bytes, b"meta")?.ok_or(Unreadable::CutShort)?;
    let meta = meta.get(4..).ok_or(Unreadable::CutShort)?;
    let primary = match child(meta, b"pitm")? {
        Some(pitm) if pitm.first() == Some(&0) => Some(u32::from(be16(pitm, 4)?)),
        Some(pitm) => Some(be32(pitm, 4)?),
        None => None,
    };
    let iprp = child(meta, b"iprp")?.ok_or(missing.clone())?;
    let ipco = child(iprp, b"ipco")?.ok_or(missing.clone())?;
    let properties = boxes(ipco).collect::<Result<Vec<_>, _>>()?;
    let associated = match (primary, child(iprp, b"ipma")?) {
        (Some(item), Some(ipma)) => ipma_properties(ipma, item)?,
        _ => (1..=properties.len()).collect(),
    };
    let mut size = None;
    let mut turned = false;
    for index in associated {
        let Some((name, body)) = index.checked_sub(1).and_then(|i| properties.get(i)) else {
            continue;
        };
        match name {
            b"ispe" if size.is_none() => {
                size = Some(Size {
                    width: be32(body, 4)?,
                    height: be32(body, 8)?,
                });
            }
            b"irot" => turned = body.first().ok_or(Unreadable::CutShort)? & 1 == 1,
            b"clap" => {
                return Ok(Err(NoSize::Unreadable(Unreadable::Unsupported(
                    "AVIF cropped by a clean aperture",
                ))));
            }
            _ => {}
        }
    }
    let size = size.ok_or(missing)?;
    Ok(Ok(if turned {
        Size {
            width: size.height,
            height: size.width,
        }
    } else {
        size
    }))
}

/// The 1-based indices into `ipco` of the properties `ipma` gives `item`.
fn ipma_properties(ipma: &[u8], item: u32) -> Result<Vec<usize>, Unreadable> {
    let version = *ipma.first().ok_or(Unreadable::CutShort)?;
    let wide = ipma.get(3).ok_or(Unreadable::CutShort)? & 1 == 1;
    let count = be32(ipma, 4)?;
    let mut at = 8;
    for _ in 0..count {
        let id = if version == 0 {
            at += 2;
            u32::from(be16(ipma, at - 2)?)
        } else {
            at += 4;
            be32(ipma, at - 4)?
        };
        let n = usize::from(*ipma.get(at).ok_or(Unreadable::CutShort)?);
        at += 1;
        let mut indices = Vec::with_capacity(n);
        for _ in 0..n {
            let index = if wide {
                at += 2;
                usize::from(be16(ipma, at - 2)? & 0x7fff)
            } else {
                at += 1;
                usize::from(ipma.get(at - 1).ok_or(Unreadable::CutShort)? & 0x7f)
            };
            indices.push(index);
        }
        if id == item {
            return Ok(indices);
        }
    }
    Ok(Vec::new())
}

/// The text after any byte order mark, XML declaration, comments and
/// document type, where an `<svg` element would begin.
fn svg_start(bytes: &[u8]) -> &[u8] {
    let mut rest = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes);
    loop {
        rest = rest.trim_ascii_start();
        let skip = |open: &[u8], close: &[u8]| -> Option<usize> {
            if !rest.starts_with(open) {
                return None;
            }
            let end = rest.windows(close.len()).position(|w| w == close)?;
            Some(end + close.len())
        };
        match skip(b"<?", b"?>")
            .or_else(|| skip(b"<!--", b"-->"))
            .or_else(|| skip(b"<!DOCTYPE", b">"))
        {
            Some(end) => rest = &rest[end..],
            None => return rest,
        }
    }
}

/// What follows the name of the root element where it is `svg`, with or
/// without a namespace prefix, as `<svg:svg` has.
fn svg_root(bytes: &[u8]) -> Option<&[u8]> {
    let rest = svg_start(bytes).strip_prefix(b"<")?;
    let end = rest
        .iter()
        .position(|&b| b.is_ascii_whitespace() || b == b'>' || b == b'/')?;
    let name = &rest[..end];
    let local = name.rsplit(|&b| b == b':').next()?;
    let prefixed = name.iter().filter(|&&b| b == b':').count();
    (local == b"svg" && prefixed <= 1 && name[0] != b':').then_some(&rest[end..])
}

fn is_svg(bytes: &[u8]) -> bool {
    svg_root(bytes).is_some()
}

/// An SVG's size, where its root element gives both `width` and `height`
/// as plain numbers or in `px`.
fn svg(bytes: &[u8]) -> Result<Size, NoSize> {
    let rest = svg_root(bytes).unwrap_or_default();
    let (mut width, mut height) = (None, None);
    let mut at = 0;
    loop {
        while rest.get(at).is_some_and(u8::is_ascii_whitespace) {
            at += 1;
        }
        match rest.get(at) {
            None => return Err(NoSize::Unreadable(Unreadable::CutShort)),
            Some(b'>' | b'/') => break,
            Some(_) => {}
        }
        let name_end = rest[at..]
            .iter()
            .position(|&b| b == b'=' || b.is_ascii_whitespace() || b == b'>' || b == b'/')
            .map_or(rest.len(), |n| at + n);
        let name = &rest[at..name_end];
        at = name_end;
        while rest.get(at).is_some_and(u8::is_ascii_whitespace) {
            at += 1;
        }
        if rest.get(at) != Some(&b'=') {
            if name.is_empty() {
                return Err(NoSize::Unreadable(Unreadable::Unsupported(
                    "SVG whose root element does not read",
                )));
            }
            continue;
        }
        at += 1;
        while rest.get(at).is_some_and(u8::is_ascii_whitespace) {
            at += 1;
        }
        let quote = *rest
            .get(at)
            .ok_or(NoSize::Unreadable(Unreadable::CutShort))?;
        if quote != b'"' && quote != b'\'' {
            return Err(NoSize::Unreadable(Unreadable::Unsupported(
                "SVG attribute without quotes",
            )));
        }
        let value_end = rest[at + 1..]
            .iter()
            .position(|&b| b == quote)
            .ok_or(NoSize::Unreadable(Unreadable::CutShort))?;
        let value = &rest[at + 1..at + 1 + value_end];
        at += value_end + 2;
        match name {
            b"width" => width = Some(value),
            b"height" => height = Some(value),
            _ => {}
        }
    }
    match (width.and_then(pixels), height.and_then(pixels)) {
        (Some(width), Some(height)) => Ok(Size { width, height }),
        _ => Err(NoSize::SvgWithoutPixels),
    }
}

/// A length written as a plain number or in `px`, rounded to whole pixels;
/// none where it rounds to nothing.
fn pixels(value: &[u8]) -> Option<u32> {
    let value = std::str::from_utf8(value).ok()?.trim();
    let number = value.strip_suffix("px").unwrap_or(value).trim_end();
    if number.is_empty() || !number.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
        return None;
    }
    let pixels = number.parse::<f64>().ok()?.round();
    (pixels >= 1.0 && pixels <= f64::from(u32::MAX)).then_some(pixels as u32)
}

/// Whether a photograph's bytes run to its end: a JPEG's end marker after
/// its first scan begins, a PNG's last chunk, or the length a WebP's RIFF
/// header gives.
pub(super) fn complete(bytes: &[u8], kind: Kind) -> bool {
    match kind {
        Kind::Jpeg => jpeg_scan(bytes)
            .and_then(|scan| bytes.get(scan..))
            .is_some_and(|scan| scan.windows(2).any(|w| w == [0xff, 0xd9])),
        Kind::Png => png_ends(bytes),
        _ => le32(bytes, 4).is_ok_and(|size| bytes.len() as u64 >= u64::from(size) + 8),
    }
}

/// Where a JPEG's first scan header begins.
fn jpeg_scan(bytes: &[u8]) -> Option<usize> {
    let mut at = 2;
    loop {
        while bytes.get(at + 1) == Some(&0xff) {
            at += 1;
        }
        let marker = *bytes.get(at + 1)?;
        if marker == 0xda {
            return Some(at);
        }
        if matches!(marker, 0x01 | 0xd0..=0xd7) {
            at += 2;
            continue;
        }
        at += 2 + usize::from(be16(bytes, at + 2).ok()?);
    }
}
