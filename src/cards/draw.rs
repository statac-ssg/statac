//! Drawing a card: its texts measured and set in the author's fonts, then
//! drawn over the background and encoded.
//!
//! A [`Kit`] is made once for a description, from the bytes of its
//! background and fonts, the lists its texts are drawn in, and its
//! padding. It measures text for [`layout`](super::layout), draws a
//! card's texts once built, and names the card by a [`Key`] known before
//! drawing it.

mod fonts;
mod measure;
#[cfg(test)]
mod tests;

use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use std::sync::{Mutex, MutexGuard, PoisonError};

use cosmic_text::fontdb::Database;
use cosmic_text::skrifa::MetadataProvider;
use cosmic_text::skrifa::instance::{LocationRef, Size};
use cosmic_text::{FontSystem, SwashContent};
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{ExtendedColorType, ImageEncoder, RgbImage};
use jpeg_encoder::SamplingFactor;

use super::layout::{self, Measure, Measured};
use super::{HEIGHT, Placement, WIDTH};
use crate::pictures;
use fonts::Faces;
use measure::Engine;

/// Raised by hand whenever a change here draws the same card differently,
/// so that its key changes with it.
pub const DRAWING: u32 = 2;

/// The largest a background may be as a PNG for cards drawn over it to be
/// PNGs; past it they are JPEGs.
pub const MOST_PNG: usize = 600 * 1024;

/// How far past its line's ends a glyph may be drawn, in ems.
const OVERHANG: f32 = 0.25;

/// Why a kit can't be made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KitFailure {
    /// The background doesn't read as a still picture, for this reason.
    Background(pictures::Error),
    /// The background isn't 1200 by 630, but this.
    BackgroundSize(pictures::Size),
    /// A font file doesn't read, by its index among those given.
    Font(usize),
}

/// Why a card can't be drawn.
#[derive(Clone, Debug, PartialEq)]
pub enum Failure {
    /// Characters no font of a text's list has: the first text lacking
    /// one, by its index, and every character its list lacks among the
    /// texts drawn in it, each once, in the order met.
    Missing {
        text: usize,
        characters: Vec<char>,
    },
    Layout(layout::Failure),
    /// The encoder refused, with its message.
    Encode(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Format {
    Png,
    Jpeg,
}

impl Format {
    pub fn extension(self) -> &'static str {
        match self {
            Format::Png => "png",
            Format::Jpeg => "jpg",
        }
    }
}

/// Every choice that decides a card's bytes besides its description and
/// texts. Each goes into the card's [`Key`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub drawing: u32,
    pub png_compression: CompressionType,
    pub png_filter: FilterType,
    /// On `jpeg-encoder`'s scale, 1 to 100.
    pub jpeg_quality: u8,
    pub jpeg_subsampling: SamplingFactor,
    pub jpeg_progressive: bool,
    pub jpeg_optimised_huffman: bool,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            drawing: DRAWING,
            png_compression: CompressionType::Best,
            png_filter: FilterType::Adaptive,
            jpeg_quality: 90,
            jpeg_subsampling: SamplingFactor::R_4_4_4,
            jpeg_progressive: true,
            jpeg_optimised_huffman: true,
        }
    }
}

impl Settings {
    /// Writes every setting, named, into `hasher`.
    fn feed(&self, hasher: &mut blake3::Hasher) {
        // Destructured in full so that a new setting cannot be left out.
        let Settings {
            drawing,
            png_compression,
            png_filter,
            jpeg_quality,
            jpeg_subsampling,
            jpeg_progressive,
            jpeg_optimised_huffman,
        } = self;
        let fields: [(&str, &dyn std::fmt::Debug); 7] = [
            ("drawing", drawing),
            ("png_compression", png_compression),
            ("png_filter", png_filter),
            ("jpeg_quality", jpeg_quality),
            ("jpeg_subsampling", jpeg_subsampling),
            ("jpeg_progressive", jpeg_progressive),
            ("jpeg_optimised_huffman", jpeg_optimised_huffman),
        ];
        for (name, value) in fields {
            hasher.update(format!("{name}={value:?}\n").as_bytes());
        }
    }
}

/// A text to draw: where and how large, its text once built, its colour
/// as red, green and blue, and the kit's list of fonts it is drawn in, by
/// its index.
#[derive(Copy, Clone, Debug)]
pub struct Text<'a> {
    pub placement: Placement,
    pub size: super::Size,
    pub built: &'a str,
    pub colour: [u8; 3],
    pub fonts: usize,
}

/// A card's texts placed, ready to draw.
#[derive(Clone, Debug)]
pub struct Plan {
    placed: Vec<layout::Placed>,
    /// Each text's colour, by its index among those planned.
    colours: Vec<[u8; 3]>,
    /// Each text's list of fonts, by its index among those planned.
    fonts: Vec<usize>,
}

/// A card drawn, before encoding.
pub struct Drawn {
    /// 1200 by 630.
    pub pixels: RgbImage,
}

/// A card drawn and encoded.
pub struct Card {
    pub bytes: Vec<u8>,
    pub format: Format,
}

/// What a card's bytes are decided by, hashed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Key(blake3::Hash);

impl Key {
    /// The whole key, as 64 hex digits.
    pub fn hex(&self) -> String {
        self.0.to_hex().to_string()
    }

    /// The first eight hex digits, which name the card.
    pub fn hash8(&self) -> String {
        self.0.to_hex()[..8].to_owned()
    }
}

/// What a card description's cards are drawn with.
pub struct Kit {
    background: RgbImage,
    padding: u32,
    format: Format,
    settings: Settings,
    /// The hash of the background's bytes and those of each font in the
    /// first list.
    sources: blake3::Hash,
    /// What every engine is made from.
    fonts: Database,
    lists: Vec<List>,
}

/// A list of fonts texts are drawn in.
struct List {
    faces: Vec<fonts::Face>,
    /// The first face's ascent and descent, as fractions of its size.
    ascent: f32,
    descent: f32,
    /// The hash of the bytes of each of its fonts, but for the first
    /// list's, which are in the kit's.
    hash: Option<blake3::Hash>,
    /// The engines no thread is using. A thread measuring or drawing takes
    /// one, or makes one where none is free, so none waits on another.
    engines: Mutex<Vec<Engine>>,
}

impl List {
    fn free(&self) -> MutexGuard<'_, Vec<Engine>> {
        self.engines.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Draws one line at its box, the coverage of all its glyphs
    /// gathered first so that where glyphs overlap they are blended once.
    fn draw_line(
        &self,
        engine: &mut Engine,
        pixels: &mut RgbImage,
        line: &layout::Line,
        size: u32,
        colour: [u8; 3],
    ) {
        let size = size as f32;
        let Some(set) = engine.set(&line.text, size) else {
            return;
        };
        let baseline =
            line.y + (line.height - (self.ascent + self.descent) * size) / 2.0 + self.ascent * size;
        // A font's ascent and descent together are taller than a line, and
        // a glyph may reach past its advance, so a line is drawn within
        // its font's own reach rather than cut at its box.
        let reach = OVERHANG * size;
        let within = |at: f32, most: u32| (at.max(0.0) as u32).min(most);
        let left = within((line.x - reach).floor(), WIDTH);
        let right = within((line.x + line.width + reach).ceil(), WIDTH);
        let top = within((baseline - self.ascent * size).floor(), HEIGHT);
        let bottom = within((baseline + self.descent * size).ceil(), HEIGHT);
        if left >= right || top >= bottom {
            return;
        }
        let across = (right - left) as usize;
        let mut coverage = vec![0u8; across * (bottom - top) as usize];
        let Engine { system, glyphs, .. } = engine;
        for glyph in &set.glyphs {
            let physical = glyph.physical((line.x, baseline), 1.0);
            let Some(image) = glyphs.get_image(system, physical.cache_key) else {
                continue;
            };
            let placement = image.placement;
            let (x0, y0) = (physical.x + placement.left, physical.y - placement.top);
            for row in 0..placement.height as i32 {
                for column in 0..placement.width as i32 {
                    let (x, y) = (x0 + column, y0 + row);
                    if x < left as i32 || x >= right as i32 || y < top as i32 || y >= bottom as i32
                    {
                        continue;
                    }
                    let at = (row * placement.width as i32 + column) as usize;
                    let value = match image.content {
                        SwashContent::Mask => image.data[at],
                        SwashContent::Color => image.data[at * 4 + 3],
                        SwashContent::SubpixelMask => {
                            let rgba = &image.data[at * 4..at * 4 + 3];
                            ((u32::from(rgba[0]) + u32::from(rgba[1]) + u32::from(rgba[2])) / 3)
                                as u8
                        }
                    };
                    let cell = &mut coverage
                        [(y as u32 - top) as usize * across + (x as u32 - left) as usize];
                    *cell = cell.saturating_add(value);
                }
            }
        }
        let colour = colour.map(linear);
        for (at, &value) in coverage.iter().enumerate() {
            if value == 0 {
                continue;
            }
            let x = left + (at % across) as u32;
            let y = top + (at / across) as u32;
            let pixel = pixels.get_pixel_mut(x, y);
            let alpha = f32::from(value) / 255.0;
            for (channel, colour) in pixel.0.iter_mut().zip(colour) {
                let under = linear(*channel);
                *channel = encoded(under + (colour - under) * alpha);
            }
        }
    }
}

/// An engine a thread holds while it measures or draws, given back when
/// let go.
struct Lent<'a> {
    list: &'a List,
    engine: Option<Engine>,
}

impl Deref for Lent<'_> {
    type Target = Engine;

    fn deref(&self) -> &Engine {
        self.engine.as_ref().expect("held until dropped")
    }
}

impl DerefMut for Lent<'_> {
    fn deref_mut(&mut self) -> &mut Engine {
        self.engine.as_mut().expect("held until dropped")
    }
}

impl Drop for Lent<'_> {
    fn drop(&mut self) {
        if let Some(engine) = self.engine.take() {
            self.list.free().push(engine);
        }
    }
}

/// An engine for each list, each taken when first asked for and held
/// until let go.
struct Engines<'a> {
    kit: &'a Kit,
    lent: Vec<Option<Lent<'a>>>,
}

impl<'a> Engines<'a> {
    fn new(kit: &'a Kit) -> Engines<'a> {
        Engines {
            kit,
            lent: std::iter::repeat_with(|| None)
                .take(kit.lists.len())
                .collect(),
        }
    }

    fn get(&mut self, list: usize) -> &mut Engine {
        self.lent[list].get_or_insert_with(|| self.kit.engine(list))
    }
}

/// Measures with one engine for each list held for a whole layout.
struct Measuring<'a>(RefCell<Engines<'a>>);

impl Measure for Measuring<'_> {
    fn measure(&self, text: &str, fonts: usize, size: u32, width: f32) -> Measured {
        measure(self.0.borrow_mut().get(fonts), text, size, width)
    }
}

fn measure(engine: &mut Engine, text: &str, size: u32, width: f32) -> Measured {
    let size = size as f32;
    let lines = engine.lines(text, size, width);
    let width = lines
        .iter()
        .map(|line| engine.width(line, size))
        .fold(0.0, f32::max);
    Measured { lines, width }
}

/// Writes the bytes of each of `files` into `hasher`, its length first.
fn feed<'a>(hasher: &mut blake3::Hasher, files: impl Iterator<Item = &'a [u8]>) {
    for bytes in files {
        hasher.update(&(bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }
}

impl Kit {
    /// A kit from the bytes of a background and of the author's font
    /// files, each given once; the lists of fonts texts are drawn in, each
    /// the files' indices in the order tried; and the padding. The first
    /// list is the card's own, and none is empty.
    pub fn new(
        background: &[u8],
        files: &[&[u8]],
        lists: &[Vec<usize>],
        padding: u32,
    ) -> Result<Kit, KitFailure> {
        Kit::make(background, files, lists, padding, Settings::default())
    }

    fn make(
        background: &[u8],
        files: &[&[u8]],
        lists: &[Vec<usize>],
        padding: u32,
        settings: Settings,
    ) -> Result<Kit, KitFailure> {
        let pixels = pictures::decode_still(background).map_err(KitFailure::Background)?;
        if pixels.dimensions() != (WIDTH, HEIGHT) {
            let (width, height) = pixels.dimensions();
            return Err(KitFailure::BackgroundSize(pictures::Size { width, height }));
        }
        let mut db = Database::new();
        let faces = fonts::load(files, &mut db).map_err(KitFailure::Font)?;
        // The fallback lists name families, and every face here has a name
        // of Statac's own, so they never match.
        let mut system = FontSystem::new_with_locale_and_db("en".into(), db.clone());
        if let Some(face) = faces
            .iter()
            .find(|face| system.get_font(face.id, face.weight).is_none())
        {
            return Err(KitFailure::Font(face.file));
        }

        let mut hasher = blake3::Hasher::new();
        feed(&mut hasher, std::iter::once(background));
        feed(&mut hasher, lists[0].iter().map(|&file| files[file]));
        let mut system = Some(system);
        let lists = (lists.iter().enumerate())
            .map(|(at, list)| {
                let faces: Vec<fonts::Face> = list
                    .iter()
                    .flat_map(|&file| faces.iter().filter(move |face| face.file == file))
                    .cloned()
                    .collect();
                let metrics = faces[0]
                    .font()
                    .metrics(Size::unscaled(), LocationRef::default());
                let em = f32::from(metrics.units_per_em.max(1));
                let hash = (at > 0).then(|| {
                    let mut hasher = blake3::Hasher::new();
                    feed(&mut hasher, list.iter().map(|&file| files[file]));
                    hasher.finalize()
                });
                let system = system
                    .take()
                    .unwrap_or_else(|| FontSystem::new_with_locale_and_db("en".into(), db.clone()));
                List {
                    ascent: metrics.ascent / em,
                    descent: -metrics.descent / em,
                    hash,
                    engines: Mutex::new(vec![Engine::new(system, Faces::new(faces.clone()))]),
                    faces,
                }
            })
            .collect();
        let bare = encode_png(&pixels, &settings).expect("1200 by 630 RGB encodes as PNG");
        let format = match bare.len() <= MOST_PNG {
            true => Format::Png,
            false => Format::Jpeg,
        };
        Ok(Kit {
            background: pixels,
            padding,
            format,
            settings,
            sources: hasher.finalize(),
            fonts: db,
            lists,
        })
    }

    /// The format every card from this kit is encoded in.
    pub fn format(&self) -> Format {
        self.format
    }

    /// An engine for the list at `list`, of the calling thread's own for
    /// as long as it is held.
    fn engine(&self, list: usize) -> Lent<'_> {
        let list = &self.lists[list];
        let free = list.free().pop();
        let engine = free.unwrap_or_else(|| {
            let system = FontSystem::new_with_locale_and_db("en".into(), self.fonts.clone());
            Engine::new(system, Faces::new(list.faces.clone()))
        });
        Lent {
            list,
            engine: Some(engine),
        }
    }

    /// The key of the card drawn with `texts`, without drawing it.
    pub fn key(&self, texts: &[Text]) -> Key {
        let mut hasher = blake3::Hasher::new();
        hasher.update(self.sources.as_bytes());
        hasher.update(&self.padding.to_le_bytes());
        hasher.update(&(texts.len() as u64).to_le_bytes());
        for text in texts {
            hasher.update(&(text.built.len() as u64).to_le_bytes());
            hasher.update(text.built.as_bytes());
            let line = match self.lists[text.fonts].hash {
                Some(fonts) => format!("{:?} {:?} {}\n", text.placement, text.size, fonts.to_hex()),
                None => format!("{:?} {:?}\n", text.placement, text.size),
            };
            hasher.update(line.as_bytes());
            hasher.update(&text.colour);
        }
        hasher.update(self.format.extension().as_bytes());
        hasher.update(b"\n");
        self.settings.feed(&mut hasher);
        Key(hasher.finalize())
    }

    /// Places `texts`, given in the description's order, or says why no
    /// card of them can be drawn.
    pub fn plan(&self, texts: &[Text]) -> Result<Plan, Failure> {
        let mut engines = Engines::new(self);
        let mut missing = vec![Vec::new(); self.lists.len()];
        let mut first = None;
        for (at, text) in texts.iter().enumerate() {
            let lacking = &mut missing[text.fonts];
            let before = lacking.len();
            engines.get(text.fonts).faces.missing(text.built, lacking);
            if lacking.len() > before {
                first.get_or_insert(at);
            }
        }
        if let Some(text) = first {
            let characters = std::mem::take(&mut missing[texts[text].fonts]);
            return Err(Failure::Missing { text, characters });
        }
        let laid: Vec<layout::Text> = texts
            .iter()
            .map(|text| layout::Text {
                placement: text.placement,
                size: text.size,
                built: text.built,
                fonts: text.fonts,
            })
            .collect();
        let measuring = Measuring(RefCell::new(engines));
        let placed = layout::lay_out(self.padding, &laid, &measuring).map_err(Failure::Layout)?;
        Ok(Plan {
            placed,
            colours: texts.iter().map(|text| text.colour).collect(),
            fonts: texts.iter().map(|text| text.fonts).collect(),
        })
    }

    /// Draws a card's texts as planned over the background.
    pub fn paint(&self, plan: &Plan) -> Drawn {
        let mut pixels = self.background.clone();
        let mut engines = Engines::new(self);
        for text in &plan.placed {
            let colour = plan.colours[text.text];
            let fonts = plan.fonts[text.text];
            let list = &self.lists[fonts];
            for line in &text.lines {
                list.draw_line(engines.get(fonts), &mut pixels, line, text.size, colour);
            }
        }
        Drawn { pixels }
    }

    /// Draws `texts`, given in the description's order, over the
    /// background.
    pub fn draw(&self, texts: &[Text]) -> Result<Drawn, Failure> {
        Ok(self.paint(&self.plan(texts)?))
    }

    /// Encodes a drawn card in the kit's format, without any colour
    /// profile or metadata.
    pub fn encode(&self, pixels: &RgbImage) -> Result<Vec<u8>, Failure> {
        match self.format {
            Format::Png => encode_png(pixels, &self.settings),
            Format::Jpeg => {
                let mut out = Vec::new();
                let settings = &self.settings;
                let mut encoder = jpeg_encoder::Encoder::new(&mut out, settings.jpeg_quality);
                encoder.set_sampling_factor(settings.jpeg_subsampling);
                encoder.set_progressive(settings.jpeg_progressive);
                encoder.set_optimized_huffman_tables(settings.jpeg_optimised_huffman);
                encoder
                    .encode(
                        pixels,
                        WIDTH as u16,
                        HEIGHT as u16,
                        jpeg_encoder::ColorType::Rgb,
                    )
                    .map_err(|e| Failure::Encode(e.to_string()))?;
                Ok(out)
            }
        }
    }

    /// Draws `texts` and encodes the card.
    pub fn card(&self, texts: &[Text]) -> Result<Card, Failure> {
        let drawn = self.draw(texts)?;
        Ok(Card {
            bytes: self.encode(&drawn.pixels)?,
            format: self.format,
        })
    }
}

/// Whether the font file `font` draws every one of `characters`, each
/// written alone.
pub fn draws_all(font: &[u8], characters: &[char]) -> bool {
    let Ok(faces) = fonts::load(&[font], &mut Database::new()) else {
        return false;
    };
    let mut faces = Faces::new(faces);
    let mut missing = Vec::new();
    for c in characters {
        faces.missing(c.encode_utf8(&mut [0; 4]), &mut missing);
    }
    missing.is_empty()
}

impl Measure for Kit {
    fn measure(&self, text: &str, fonts: usize, size: u32, width: f32) -> Measured {
        measure(&mut self.engine(fonts), text, size, width)
    }
}

fn encode_png(pixels: &RgbImage, settings: &Settings) -> Result<Vec<u8>, Failure> {
    let mut out = Vec::new();
    PngEncoder::new_with_quality(&mut out, settings.png_compression, settings.png_filter)
        .write_image(
            pixels,
            pixels.width(),
            pixels.height(),
            ExtendedColorType::Rgb8,
        )
        .map_err(|e| Failure::Encode(e.to_string()))?;
    Ok(out)
}

/// An sRGB level as linear light, from 0 to 1.
fn linear(level: u8) -> f32 {
    let v = f32::from(level) / 255.0;
    match v <= 0.04045 {
        true => v / 12.92,
        false => ((v + 0.055) / 1.055).powf(2.4),
    }
}

/// Linear light as the nearest sRGB level.
fn encoded(light: f32) -> u8 {
    let light = light.clamp(0.0, 1.0);
    let v = match light <= 0.003_130_8 {
        true => light * 12.92,
        false => 1.055 * light.powf(1.0 / 2.4) - 0.055,
    };
    (v * 255.0).round() as u8
}
