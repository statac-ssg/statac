//! Pictures as a site publishes them. A photograph, a JPEG, a still WebP
//! or a still PNG, is published as resized copies: AVIF with a JPEG beside
//! it, or a PNG where the photograph is transparent anywhere. Any other
//! picture is published as it is, and only its size is read.
//!
//! Everything here is worked out from a file's bytes and a few numbers:
//! the kind and size from the header alone, whether it is transparent from
//! its pixels where the header says it may be, the widths and names of the
//! copies from those and [`Settings`], and the copies' bytes from one
//! decoding of the photograph, [`Opened`].

mod copies;
mod header;
#[cfg(test)]
mod tests;

use image::codecs::png::{CompressionType, FilterType as PngFilter};
use image::imageops::FilterType;
use jpeg_encoder::{ChromaSubsamplingMethod, SamplingFactor};

pub use copies::{Opened, Resized, converts, decode_still, open, remade, transparent};

/// What a picture is, told from its bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Jpeg,
    /// A WebP of one frame.
    WebP,
    AnimatedWebP,
    /// A PNG of one picture.
    Png,
    AnimatedPng,
    Gif,
    Svg,
    Avif,
    /// Recognised so a caller can name it, but not read further.
    Heic,
    /// HEIF holding something other than HEVC or AV1, recognised and not
    /// read further.
    Heif,
}

impl Kind {
    /// Whether pictures of this kind are published as resized copies.
    pub(crate) fn is_photograph(self) -> bool {
        matches!(self, Kind::Jpeg | Kind::WebP | Kind::Png)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

/// Why the bytes given do not read as a picture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unreadable {
    /// They begin as no kind of picture known here.
    NotAPicture,
    /// They end before the picture does, or are damaged within it.
    CutShort,
    /// They are a known kind written in a form not read here, named.
    Unsupported(&'static str),
}

/// What a picture's header says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Header {
    Photograph(Photograph),
    Plain(Plain),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Photograph {
    /// [`Kind::Jpeg`], [`Kind::WebP`] or [`Kind::Png`].
    pub(crate) kind: Kind,
    /// The size as the pixels are stored.
    pub(crate) stored: Size,
    /// The EXIF orientation, 1 to 8, or 1 where there is none. A PNG's is
    /// not read, and it is never turned.
    pub(crate) orientation: u8,
    /// The size once turned by its orientation.
    pub shown: Size,
    /// Whether the container says the pixels carry alpha. Most that say so
    /// are opaque throughout; [`transparent`] tells.
    pub(crate) alpha: bool,
    /// Present where a colour profile is embedded, holding its description
    /// where it has one.
    pub(crate) profile: Option<Option<String>>,
}

/// The formats of a photograph's copies: AVIF, then PNG where it is
/// [`transparent`] and JPEG where it is not, whatever kind of file it is.
pub(crate) fn formats(transparent: bool) -> [Format; 2] {
    [
        Format::Avif,
        match transparent {
            true => Format::Png,
            false => Format::Jpeg,
        },
    ]
}

/// A picture published as it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plain {
    pub(crate) kind: Kind,
    pub(crate) size: Result<Size, NoSize>,
}

/// Why a plain picture's size is not given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NoSize {
    /// Its kind is not one whose size is read.
    NotRead,
    /// An SVG whose root element does not give both its width and height in
    /// pixels.
    SvgWithoutPixels,
    /// Its header does not read.
    Unreadable(Unreadable),
}

/// Reads the kind of picture `bytes` hold and what its header says of it,
/// without decoding any pixel.
///
/// Only the first bytes are read, except that a WebP whose extended header
/// announces EXIF is read up to its EXIF chunk, which follows the pixels.
pub fn read_header(bytes: &[u8]) -> Result<Header, Unreadable> {
    header::parse(bytes).map(|parsed| parsed.header)
}

/// The kind of picture `bytes` begin as, told from their first few alone;
/// none where they begin as no picture, or as too little of one to tell.
/// A WebP or a PNG that moves is told as one that doesn't.
pub(crate) fn begins_as(bytes: &[u8]) -> Option<Kind> {
    header::sniff(bytes).ok()
}

/// Whether `bytes` are a JPEG, PNG or WebP whose EXIF holds where it was
/// taken, a latitude and a longitude. Nothing else it holds is read for
/// one, and a file that does not read holds none.
pub(crate) fn holds_location(bytes: &[u8]) -> bool {
    header::located(bytes)
}

/// Why a photograph's copies could not be made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Unreadable(Unreadable),
    /// The bytes are a picture of a kind published as it is.
    NotAPhotograph(Kind),
    /// Its pixels would take more than [`MAX_DECODED_BYTES`], as its
    /// decoder gives them or as they are kept.
    TooLarge(Size),
    /// Copies of this width or format are not made of this photograph: the
    /// width is nought or wider than the photograph, or the format is not
    /// one of its [`formats`].
    NotOffered {
        width: u32,
        format: Format,
    },
    /// The encoder refused, with its message.
    Encode {
        format: Format,
        message: String,
    },
}

/// The most memory a photograph's pixels may take, both as its decoder
/// gives them, which for a PNG of 16 bits a sample is twice what is kept,
/// and as they are kept, 8-bit RGB or RGBA.
pub(crate) const MAX_DECODED_BYTES: u64 = 1 << 30;

/// How large a picture may be for its pixels to be decoded at all.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Bounds {
    /// The most pixels either side may hold.
    pub(crate) side: u32,
    /// The most memory its pixels may take, as [`MAX_DECODED_BYTES`] is.
    pub(crate) bytes: u64,
}

/// The bounds of a photograph of the site's own.
pub(crate) const PHOTOGRAPH: Bounds = Bounds {
    side: u32::MAX,
    bytes: MAX_DECODED_BYTES,
};

/// The widths copies are made at, where a photograph is wide enough.
pub(crate) const RUNGS: [u32; 5] = [400, 800, 1200, 1600, 2400];

/// The widths of the copies of a photograph `natural` pixels wide as shown,
/// none wider than `max_width` nor than the photograph, narrowest first.
pub(crate) fn widths(natural: u32, max_width: u32) -> Vec<u32> {
    let max_width = max_width.max(1);
    let mut widths: Vec<u32> = RUNGS
        .into_iter()
        .filter(|&rung| u64::from(rung) * 10 < u64::from(natural) * 9)
        .chain([natural])
        .filter(|&width| width <= max_width)
        .collect();
    if natural > max_width && RUNGS.contains(&max_width) && !widths.contains(&max_width) {
        widths.push(max_width);
    }
    if widths.is_empty() {
        widths.push(max_width.min(RUNGS[0]));
    }
    widths
}

/// The width a tag's `src` names among `widths`: the widest at or below
/// 1200, or else the widest.
pub(crate) fn src_width(widths: &[u32]) -> Option<u32> {
    let below = widths.iter().copied().filter(|&w| w <= 1200).max();
    below.or_else(|| widths.iter().copied().max())
}

/// The height of a copy `width` wide of a photograph `shown`, rounded and
/// at least 1.
pub(crate) fn copy_height(shown: Size, width: u32) -> u32 {
    let numerator = 2 * u64::from(shown.height) * u64::from(width);
    let denominator = 2 * u64::from(shown.width.max(1));
    let height = (numerator + denominator / 2) / denominator;
    u32::try_from(height).unwrap_or(u32::MAX).max(1)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Format {
    Avif,
    Jpeg,
    Png,
}

impl Format {
    /// The ending of a copy's file name.
    pub fn extension(self) -> &'static str {
        match self {
            Format::Avif => "avif",
            Format::Jpeg => "jpg",
            Format::Png => "png",
        }
    }
}

/// Raised by hand whenever a change here makes copies differ for the same
/// photograph and [`Settings`], so their keys change with them.
pub(crate) const PIPELINE: u32 = 1;

/// Every choice that decides a copy's bytes. Each goes into the copy's
/// [`Key`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub(crate) pipeline: u32,
    /// The AVIF encoder's [`avif_aom_sys::version`].
    pub(crate) avif_encoder: String,
    pub(crate) avif_speed: u8,
    pub(crate) avif_quality: u8,
    pub(crate) avif_alpha_quality: u8,
    /// Copies are the same from run to run only with 1.
    pub(crate) avif_threads: u32,
    pub(crate) resize: FilterType,
    /// On `jpeg-encoder`'s scale, 1 to 100.
    pub(crate) jpeg_quality: u8,
    pub(crate) jpeg_subsampling: SamplingFactor,
    pub(crate) jpeg_chroma_method: ChromaSubsamplingMethod,
    pub(crate) jpeg_progressive: bool,
    pub(crate) jpeg_optimised_huffman: bool,
    /// The slowest setting takes near three times as long for files a
    /// hundredth smaller.
    pub(crate) png_compression: CompressionType,
    pub(crate) png_filter: PngFilter,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            pipeline: PIPELINE,
            avif_encoder: avif_aom_sys::version(),
            avif_speed: 8,
            avif_quality: 67,
            avif_alpha_quality: 90,
            avif_threads: 1,
            resize: FilterType::Lanczos3,
            jpeg_quality: 77,
            jpeg_subsampling: SamplingFactor::R_4_4_4,
            jpeg_chroma_method: ChromaSubsamplingMethod::Average,
            jpeg_progressive: true,
            jpeg_optimised_huffman: true,
            png_compression: CompressionType::Default,
            png_filter: PngFilter::Adaptive,
        }
    }
}

impl Settings {
    /// Writes every setting, named, into `hasher`.
    fn feed(&self, hasher: &mut blake3::Hasher) {
        // Destructured in full so that a new setting cannot be left out.
        let Settings {
            pipeline,
            avif_encoder,
            avif_speed,
            avif_quality,
            avif_alpha_quality,
            avif_threads,
            resize,
            jpeg_quality,
            jpeg_subsampling,
            jpeg_chroma_method,
            jpeg_progressive,
            jpeg_optimised_huffman,
            png_compression,
            png_filter,
        } = self;
        let fields: [(&str, &dyn std::fmt::Debug); 14] = [
            ("pipeline", pipeline),
            ("avif_encoder", avif_encoder),
            ("avif_speed", avif_speed),
            ("avif_quality", avif_quality),
            ("avif_alpha_quality", avif_alpha_quality),
            ("avif_threads", avif_threads),
            ("resize", resize),
            ("jpeg_quality", jpeg_quality),
            ("jpeg_subsampling", jpeg_subsampling),
            ("jpeg_chroma_method", jpeg_chroma_method),
            ("jpeg_progressive", jpeg_progressive),
            ("jpeg_optimised_huffman", jpeg_optimised_huffman),
            ("png_compression", png_compression),
            ("png_filter", png_filter),
        ];
        for (name, value) in fields {
            hasher.update(format!("{name}={value:?}\n").as_bytes());
        }
    }
}

/// What a copy's bytes are decided by, hashed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Key(blake3::Hash);

impl Key {
    /// The key of the copy `width` wide in `format` of the file whose bytes
    /// hash to `source`.
    pub(crate) fn new(
        source: &blake3::Hash,
        width: u32,
        format: Format,
        settings: &Settings,
    ) -> Key {
        let mut hasher = blake3::Hasher::new();
        hasher.update(source.as_bytes());
        hasher.update(&width.to_le_bytes());
        hasher.update(format.extension().as_bytes());
        hasher.update(b"\n");
        settings.feed(&mut hasher);
        Key(hasher.finalize())
    }

    /// The whole key, as 64 hex digits.
    pub(crate) fn hex(&self) -> String {
        self.0.to_hex().to_string()
    }

    /// The first eight hex digits, which name the copy.
    pub(crate) fn hash8(&self) -> String {
        self.0.to_hex()[..8].to_owned()
    }
}

/// One copy of a photograph, as named before it is made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rendition {
    pub width: u32,
    pub height: u32,
    pub format: Format,
    pub(crate) key: Key,
    /// `{stem}-{width}.{hash8}.{extension}`.
    pub name: String,
}

/// Every copy of a photograph, widths narrowest first and, at each width,
/// in the order of [`formats`]. `transparent` is what [`transparent`] says
/// of it, `stem` its file name without its ending, and `source` the hash
/// of its bytes.
pub fn ladder(
    photograph: &Photograph,
    transparent: bool,
    stem: &str,
    source: &blake3::Hash,
    max_width: u32,
    settings: &Settings,
) -> Vec<Rendition> {
    let mut ladder = Vec::new();
    for width in widths(photograph.shown.width, max_width) {
        for format in formats(transparent) {
            let key = Key::new(source, width, format, settings);
            ladder.push(Rendition {
                width,
                height: copy_height(photograph.shown, width),
                format,
                name: format!("{stem}-{width}.{}.{}", key.hash8(), format.extension()),
                key,
            });
        }
    }
    ladder
}
