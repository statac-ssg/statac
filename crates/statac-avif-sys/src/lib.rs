//! libavif built with libaom as its only codec, encode only.
//!
//! Colour is written as 8-bit 4:4:4, full range, with CICP 1/13/6 (BT.709
//! primaries, sRGB transfer, BT.601 matrix).

use std::ffi::{CStr, c_char, c_int};
use std::fmt;

unsafe extern "C" {
    fn avif_shim_encode(
        pixels: *const u8,
        width: u32,
        height: u32,
        row_bytes: u32,
        has_alpha: c_int,
        speed: c_int,
        quality: c_int,
        quality_alpha: c_int,
        threads: c_int,
        out: *mut *mut u8,
        out_len: *mut usize,
        error: *mut c_char,
        error_len: usize,
    ) -> c_int;
    fn avif_shim_free(data: *mut u8, len: usize);
    fn avif_shim_codec_versions(out: *mut c_char, len: usize);
    fn avifVersion() -> *const c_char;
}

/// How the bytes of each pixel are laid out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    /// Red, green, blue.
    Rgb,
    /// Red, green, blue and straight, not premultiplied, alpha.
    Rgba,
}

impl Layout {
    fn channels(self) -> u32 {
        match self {
            Layout::Rgb => 3,
            Layout::Rgba => 4,
        }
    }
}

/// Tightly packed 8-bit pixels, row by row from the top.
#[derive(Clone, Copy, Debug)]
pub struct Image<'a> {
    pub pixels: &'a [u8],
    pub width: u32,
    pub height: u32,
    pub layout: Layout,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    /// 0, slowest and smallest, to 10, fastest.
    pub speed: u8,
    /// libavif's quality for colour, 0 to 100, where 100 is best.
    pub quality: u8,
    /// The same for alpha. Unused for RGB.
    pub alpha_quality: u8,
    /// At least 1. Output is the same from run to run only with 1.
    pub threads: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The width or the height is zero.
    ZeroDimension,
    /// The image's size in bytes does not fit in memory.
    TooLarge,
    /// The pixel buffer's length is not width times height times channels.
    LengthMismatch { expected: usize, actual: usize },
    /// A setting is outside its range.
    InvalidSetting { name: &'static str, value: u32 },
    /// libavif refused the image or failed to encode it, with its message.
    Encoder(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::ZeroDimension => f.write_str("the image has no pixels"),
            Error::TooLarge => f.write_str("the image is too large"),
            Error::LengthMismatch { expected, actual } => write!(
                f,
                "the pixel buffer has {actual} bytes where the dimensions need {expected}"
            ),
            Error::InvalidSetting { name, value } => write!(f, "{name} {value} is out of range"),
            Error::Encoder(message) => write!(f, "libavif: {message}"),
        }
    }
}

impl std::error::Error for Error {}

/// Encodes `image` as an AVIF file. RGBA writes an alpha item unless every
/// pixel is opaque, which libavif leaves out as it would change nothing.
pub fn encode(image: &Image<'_>, settings: &Settings) -> Result<Vec<u8>, Error> {
    let row_bytes = check(image)?;
    let threads = check_settings(settings)?;

    let mut out = std::ptr::null_mut();
    let mut len = 0;
    let mut error = [0 as c_char; 512];
    // SAFETY: `pixels` holds exactly `row_bytes * height` bytes, checked
    // above, and the shim only reads them. `out`, `len` and `error` are live
    // locals the shim writes through, `error` within the length given.
    let result = unsafe {
        avif_shim_encode(
            image.pixels.as_ptr(),
            image.width,
            image.height,
            row_bytes,
            c_int::from(image.layout == Layout::Rgba),
            c_int::from(settings.speed),
            c_int::from(settings.quality),
            c_int::from(settings.alpha_quality),
            threads,
            &mut out,
            &mut len,
            error.as_mut_ptr(),
            error.len(),
        )
    };
    if result != 0 {
        // SAFETY: on failure the shim leaves a terminated string in `error`.
        let message = unsafe { CStr::from_ptr(error.as_ptr()) };
        return Err(Error::Encoder(message.to_string_lossy().into_owned()));
    }
    if out.is_null() {
        return Err(Error::Encoder("no output".into()));
    }
    // SAFETY: on success `out` points to `len` initialised bytes that libavif
    // allocated and nothing else touches until they are freed below.
    let bytes = unsafe { std::slice::from_raw_parts(out, len) }.to_vec();
    // SAFETY: `out` and `len` came from the shim's successful encode and are
    // freed once.
    unsafe { avif_shim_free(out, len) };
    Ok(bytes)
}

fn check(image: &Image<'_>) -> Result<u32, Error> {
    if image.width == 0 || image.height == 0 {
        return Err(Error::ZeroDimension);
    }
    let row_bytes = image
        .width
        .checked_mul(image.layout.channels())
        .ok_or(Error::TooLarge)?;
    let expected = usize::try_from(row_bytes)
        .ok()
        .zip(usize::try_from(image.height).ok())
        .and_then(|(row, height)| row.checked_mul(height))
        .ok_or(Error::TooLarge)?;
    if image.pixels.len() != expected {
        return Err(Error::LengthMismatch {
            expected,
            actual: image.pixels.len(),
        });
    }
    Ok(row_bytes)
}

fn check_settings(settings: &Settings) -> Result<c_int, Error> {
    for (name, value, max) in [
        ("speed", settings.speed, 10),
        ("quality", settings.quality, 100),
        ("alpha quality", settings.alpha_quality, 100),
    ] {
        if value > max {
            return Err(Error::InvalidSetting {
                name,
                value: value.into(),
            });
        }
    }
    if settings.threads == 0 {
        return Err(Error::InvalidSetting {
            name: "threads",
            value: 0,
        });
    }
    Ok(c_int::try_from(settings.threads).unwrap_or(c_int::MAX))
}

/// libavif's version and its codec's, such as
/// `libavif 1.4.2 (aom [enc]:v3.15.1)`.
pub fn version() -> String {
    // SAFETY: avifVersion takes nothing and only returns a pointer.
    let libavif = unsafe { avifVersion() };
    // SAFETY: that pointer is to a terminated string with static lifetime.
    let libavif = unsafe { CStr::from_ptr(libavif) }.to_string_lossy();
    let mut codecs = [0 as c_char; 256];
    // SAFETY: the shim writes at most `codecs.len()` bytes, terminated.
    unsafe { avif_shim_codec_versions(codecs.as_mut_ptr(), codecs.len()) };
    // SAFETY: the shim has just terminated it within its length.
    let codecs = unsafe { CStr::from_ptr(codecs.as_ptr()) }.to_string_lossy();
    format!("libavif {libavif} ({codecs})")
}

/// How libaom was compiled, as CMake recorded it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BuildFacts {
    /// CMake's build type, such as `Release`.
    pub build_type: &'static str,
    /// CMake's C flags for every build type followed by those for this one.
    /// libaom adds its own warning and language flags on top, per target.
    pub c_flags: &'static str,
}

pub fn build_facts() -> BuildFacts {
    BuildFacts {
        build_type: env!("AVIF_AOM_BUILD_TYPE"),
        c_flags: env!("AVIF_AOM_C_FLAGS"),
    }
}
