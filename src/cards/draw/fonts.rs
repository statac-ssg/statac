//! The fonts a card is drawn in, and which of them draws each character.
//!
//! The text engine would choose fonts by family, weight and its own
//! fallback lists. Here each face is given a family name no other face
//! has, and every run of text is told which face to use, so that the
//! first font in the kit's order with a glyph for a character draws it.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

use cosmic_text::fontdb::{self, Database, Language, Source};
use cosmic_text::skrifa::instance::{LocationRef, Size};
use cosmic_text::skrifa::outline::{DrawSettings, OutlinePen};
use cosmic_text::skrifa::{FontRef, MetadataProvider};
use cosmic_text::{Attrs, AttrsList, CacheKeyFlags, Family};

/// One face of a font file, as the text engine knows it.
#[derive(Clone)]
pub(super) struct Face {
    pub id: fontdb::ID,
    pub name: String,
    pub weight: fontdb::Weight,
    pub style: fontdb::Style,
    pub stretch: fontdb::Stretch,
    /// The font file it is in, by its place in the kit's list.
    pub file: usize,
    /// Whether its digits, `#` and `*` are there only as the bases of
    /// keycaps.
    pub keycaps_only: bool,
    data: Arc<Vec<u8>>,
    index: u32,
}

impl Face {
    pub fn font(&self) -> FontRef<'_> {
        FontRef::from_index(&self.data, self.index).expect("read when the kit was made")
    }

    pub fn attrs(&self) -> Attrs<'_> {
        Attrs::new()
            .family(Family::Name(&self.name))
            .weight(self.weight)
            .style(self.style)
            .stretch(self.stretch)
            .cache_key_flags(CacheKeyFlags::DISABLE_HINTING)
    }
}

/// Reads each font file in `files` into `db`, every face under its own
/// name. A file that doesn't read is answered by its index.
pub(super) fn load(files: &[&[u8]], db: &mut Database) -> Result<Vec<Face>, usize> {
    let mut faces = Vec::new();
    for (file, bytes) in files.iter().enumerate() {
        let data = Arc::new(bytes.to_vec());
        let mut alone = Database::new();
        let ids = alone.load_font_source(Source::Binary(data.clone()));
        if ids.is_empty() {
            return Err(file);
        }
        for id in ids {
            let mut info = alone.face(id).expect("just loaded").clone();
            let font = FontRef::from_index(&data, info.index).map_err(|_| file)?;
            if font.charmap().mappings().next().is_none() {
                return Err(file);
            }
            let keycaps_only = keycaps_only(&font);
            let name = format!("statac face {}", faces.len());
            info.families = vec![(name.clone(), Language::English_UnitedStates)];
            let (weight, style, stretch, index) =
                (info.weight, info.style, info.stretch, info.index);
            faces.push(Face {
                id: db.push_face_info(info),
                name,
                weight,
                style,
                stretch,
                file,
                keycaps_only,
                data: data.clone(),
                index,
            });
        }
    }
    Ok(faces)
}

/// Whether a character draws nothing by design: white space, controls and
/// the default ignorable characters.
fn invisible(c: char) -> bool {
    c.is_whitespace() || c.is_control() || ignorable(c)
}

/// Default ignorable characters, which shape to nothing and never decide
/// a font.
fn ignorable(c: char) -> bool {
    matches!(c,
        '\u{00AD}' | '\u{034F}' | '\u{061C}' | '\u{115F}'..='\u{1160}' | '\u{17B4}'..='\u{17B5}'
        | '\u{180B}'..='\u{180F}' | '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}'
        | '\u{2060}'..='\u{206F}' | '\u{3164}' | '\u{FE00}'..='\u{FE0F}' | '\u{FEFF}'
        | '\u{FFA0}' | '\u{FFF0}'..='\u{FFF8}' | '\u{1BCA0}'..='\u{1BCA3}'
        | '\u{1D173}'..='\u{1D17A}' | '\u{E0000}'..='\u{E0FFF}')
}

/// Whether `c` belongs with the character before it: a combining mark,
/// a joiner, a selector, an emoji modifier or tag, or a vowel or final
/// jamo of a Hangul syllable.
fn joins(c: char) -> bool {
    matches!(c,
        '\u{0300}'..='\u{036F}' | '\u{0483}'..='\u{0489}' | '\u{0591}'..='\u{05BD}' | '\u{05BF}'
        | '\u{05C1}'..='\u{05C2}' | '\u{05C4}'..='\u{05C5}' | '\u{05C7}' | '\u{0610}'..='\u{061A}'
        | '\u{064B}'..='\u{065F}' | '\u{0670}' | '\u{06D6}'..='\u{06DC}' | '\u{06DF}'..='\u{06E4}'
        | '\u{06E7}'..='\u{06E8}' | '\u{06EA}'..='\u{06ED}' | '\u{0E31}' | '\u{0E34}'..='\u{0E3A}'
        | '\u{0E47}'..='\u{0E4E}' | '\u{1160}'..='\u{11FF}' | '\u{1AB0}'..='\u{1AFF}'
        | '\u{1DC0}'..='\u{1DFF}' | '\u{200C}'..='\u{200D}' | '\u{20D0}'..='\u{20FF}'
        | '\u{302A}'..='\u{302F}' | '\u{3099}'..='\u{309A}' | '\u{D7B0}'..='\u{D7FF}'
        | '\u{FE00}'..='\u{FE0F}' | '\u{FE20}'..='\u{FE2F}' | '\u{1F3FB}'..='\u{1F3FF}'
        | '\u{E0020}'..='\u{E007F}' | '\u{E0100}'..='\u{E01EF}')
}

fn regional(c: char) -> bool {
    matches!(c, '\u{1F1E6}'..='\u{1F1FF}')
}

/// The byte ranges of `text`'s clusters: the characters that are drawn
/// together and so in one font.
pub(super) fn clusters(text: &str) -> Vec<Range<usize>> {
    let mut clusters: Vec<Range<usize>> = Vec::new();
    let mut previous: Option<char> = None;
    // Regional indicators pair into flags.
    let mut regionals = 0;
    for (at, c) in text.char_indices() {
        let pairs = regional(c) && regionals % 2 == 1;
        let together = joins(c) || previous == Some('\u{200D}') || pairs;
        match clusters.last_mut() {
            Some(cluster) if together => cluster.end = at + c.len_utf8(),
            _ => {
                clusters.push(at..at + c.len_utf8());
                regionals = 0;
            }
        }
        if regional(c) {
            regionals += 1;
        }
        previous = Some(c);
    }
    clusters
}

/// Counts the segments of an outline.
struct Segments(usize);

impl OutlinePen for Segments {
    fn move_to(&mut self, _: f32, _: f32) {}
    fn line_to(&mut self, _: f32, _: f32) {
        self.0 += 1;
    }
    fn quad_to(&mut self, _: f32, _: f32, _: f32, _: f32) {
        self.0 += 1;
    }
    fn curve_to(&mut self, _: f32, _: f32, _: f32, _: f32, _: f32, _: f32) {
        self.0 += 1;
    }
    fn close(&mut self) {}
}

/// Whether `face` has a glyph for `c`. For a character that should be
/// visible, the glyph must draw something.
fn has(face: &Face, c: char) -> bool {
    let font = face.font();
    let Some(glyph) = font.charmap().map(c).filter(|glyph| glyph.to_u32() != 0) else {
        return false;
    };
    if invisible(c) {
        return true;
    }
    let mut segments = Segments(0);
    if let Some(outline) = font.outline_glyphs().get(glyph) {
        let settings = DrawSettings::unhinted(Size::unscaled(), LocationRef::default());
        if outline.draw(settings, &mut segments).is_err() {
            segments.0 = 0;
        }
    }
    segments.0 > 0
        || font.color_glyphs().get(glyph).is_some()
        || font
            .bitmap_strikes()
            .glyph_for_size(Size::unscaled(), glyph)
            .is_some()
}

/// Whether `c` is a base of a keycap sequence. An emoji font draws one
/// only as part of such a sequence; alone, it is text.
fn keycap_base(c: char) -> bool {
    matches!(c, '0'..='9' | '#' | '*')
}

const KEYCAP: char = '\u{20E3}';

/// Whether `font` is an emoji font, whose digits, `#` and `*` are the
/// bases of its keycaps and not text: it maps the keycap mark and no
/// letter. Told by what the font holds, since its file may have any name.
fn keycaps_only(font: &FontRef) -> bool {
    let maps = |c: char| {
        font.charmap()
            .map(c)
            .is_some_and(|glyph| glyph.to_u32() != 0)
    };
    maps(KEYCAP) && !maps('a') && !maps('A')
}

/// The faces in the kit's order, with what is known of which characters
/// each draws.
pub(super) struct Faces {
    pub faces: Vec<Face>,
    known: HashMap<(usize, char), bool>,
}

/// A run of text drawn in one face, by its index.
pub(super) struct Run {
    pub range: Range<usize>,
    pub face: usize,
}

impl Faces {
    pub fn new(faces: Vec<Face>) -> Faces {
        Faces {
            faces,
            known: HashMap::new(),
        }
    }

    /// Whether `face` may draw `c` in a cluster with a keycap or without.
    fn draws(&mut self, face: usize, c: char, keycap: bool) -> bool {
        if self.faces[face].keycaps_only && keycap_base(c) && !keycap {
            return false;
        }
        *self
            .known
            .entry((face, c))
            .or_insert_with(|| has(&self.faces[face], c))
    }

    /// The face for a cluster: the first with a glyph for each of its
    /// characters that needs one, or else the first with one for the
    /// first of them. None for a cluster with nothing to draw, or nothing
    /// any face has.
    fn choose(&mut self, cluster: &str) -> Option<usize> {
        let needed: Vec<char> = cluster.chars().filter(|&c| !ignorable(c)).collect();
        let first = *needed.first()?;
        let keycap = needed.contains(&KEYCAP);
        (0..self.faces.len())
            .find(|&face| needed.iter().all(|&c| self.draws(face, c, keycap)))
            .or_else(|| (0..self.faces.len()).find(|&face| self.draws(face, first, keycap)))
    }

    /// Adds to `found` the characters of `text` no face draws, each once.
    /// An ignorable character is never missing.
    pub fn missing(&mut self, text: &str, found: &mut Vec<char>) {
        for cluster in clusters(text) {
            let cluster = &text[cluster];
            let keycap = cluster.contains(KEYCAP);
            for c in cluster.chars().filter(|&c| !ignorable(c)) {
                if !found.contains(&c)
                    && !(0..self.faces.len()).any(|face| self.draws(face, c, keycap))
                {
                    found.push(c);
                }
            }
        }
    }

    /// `text` in runs of one face each. A cluster no face has is given the
    /// first face, and one with nothing to draw the face before it.
    pub fn runs(&mut self, text: &str) -> Vec<Run> {
        let mut runs: Vec<Run> = Vec::new();
        for cluster in clusters(text) {
            let chosen = self.choose(&text[cluster.clone()]);
            let face = chosen.or(runs.last().map(|run| run.face)).unwrap_or(0);
            match runs.last_mut() {
                Some(run) if run.face == face => run.range.end = cluster.end,
                _ => runs.push(Run {
                    range: cluster,
                    face,
                }),
            }
        }
        runs
    }

    /// The text engine's attributes for `text` set in [`Faces::runs`].
    pub fn attrs(&mut self, text: &str) -> AttrsList {
        let runs = self.runs(text);
        let mut list = AttrsList::new(&self.faces[0].attrs());
        for run in runs {
            list.add_span(run.range, &self.faces[run.face].attrs());
        }
        list
    }
}

/// Whether `c` is written in a script that leaves no spaces between words:
/// Chinese characters, kana, and the punctuation and full-width forms
/// written with them.
pub(super) fn spaceless(c: char) -> bool {
    matches!(c,
        '\u{2E80}'..='\u{2FDF}' | '\u{3000}'..='\u{303F}' | '\u{3040}'..='\u{30FF}'
        | '\u{31F0}'..='\u{31FF}' | '\u{3400}'..='\u{4DBF}' | '\u{4E00}'..='\u{9FFF}'
        | '\u{F900}'..='\u{FAFF}' | '\u{FF00}'..='\u{FFEF}' | '\u{20000}'..='\u{3FFFF}')
}

/// Whether `c` is a Hangul syllable or jamo.
pub(super) fn hangul(c: char) -> bool {
    matches!(c,
        '\u{1100}'..='\u{11FF}' | '\u{3130}'..='\u{318F}' | '\u{A960}'..='\u{A97F}'
        | '\u{AC00}'..='\u{D7A3}' | '\u{D7B0}'..='\u{D7FF}')
}
