//! Where each text on a card goes: a calculation with no fonts in it,
//! every text measured through [`Measure`].
//!
//! The safe area is the card inset by its padding. Every text wraps
//! within the safe area's width, and a line is 1.2 times its size tall.
//! Texts at one place stack in the order written, half the lower text's
//! line height apart. The top row hangs from the top of the safe area and
//! the bottom row sits on its bottom, each as tall as its tallest stack;
//! the middle row has the height left between them, and each of its
//! stacks is centred in it.
//!
//! In the middle row, the texts sized `up to` share one search for the
//! largest size, down to [`FLOOR`], at which every middle stack fits.
//! Heights are counted in fifths of a pixel, which every line height and
//! gap is a whole number of, so a fit on the boundary is exact.

use super::{Align, HEIGHT, Placement, Row, Size, WIDTH};

/// The smallest size the middle row's search tries.
pub const FLOOR: u32 = 45;

/// Measures text set in the card's fonts.
pub trait Measure {
    /// The lines `text` breaks into at `size`, wrapped within `width`
    /// pixels, and the width of the widest.
    fn measure(&self, text: &str, size: u32, width: f32) -> Measured;
}

#[derive(Clone, PartialEq, Debug)]
pub struct Measured {
    pub lines: Vec<String>,
    pub width: f32,
}

/// A text to place: where and how large, and its text once built.
#[derive(Copy, Clone, Debug)]
pub struct Text<'a> {
    pub placement: Placement,
    pub size: Size,
    pub built: &'a str,
}

/// A text placed, by its index among those given.
#[derive(Clone, PartialEq, Debug)]
pub struct Placed {
    pub text: usize,
    pub size: u32,
    pub lines: Vec<Line>,
}

/// A line's box on the card, in pixels from its top left corner.
#[derive(Clone, PartialEq, Debug)]
pub struct Line {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Why the texts can't be placed. Heights are in pixels.
#[derive(Clone, PartialEq, Debug)]
pub enum Failure {
    /// The top and bottom rows overlap, or leave a middle text no height:
    /// the heights of the two rows and of the safe area.
    NoRoom { top: f32, bottom: f32, safe: f32 },
    /// A middle text that doesn't fit at the size tried, by its index: the
    /// height its stack needs down to its last line, and the row's.
    TooTall {
        text: usize,
        size: u32,
        needs: f32,
        room: f32,
    },
}

/// One text sized and broken into lines.
struct Sized {
    text: usize,
    size: u32,
    lines: Vec<String>,
}

impl Sized {
    fn height(&self) -> u64 {
        line_height(self.size) * self.lines.len() as u64
    }
}

/// In fifths of a pixel.
fn line_height(size: u32) -> u64 {
    6 * u64::from(size)
}

/// Half a line, in fifths of a pixel.
fn gap(size: u32) -> u64 {
    3 * u64::from(size)
}

fn pixels(fifths: u64) -> f32 {
    fifths as f32 / 5.0
}

/// How far down each text in a stack starts, from the stack's top, and
/// the stack's height, all in fifths of a pixel.
fn stack(texts: &[&Sized]) -> (Vec<u64>, u64) {
    let mut tops = Vec::with_capacity(texts.len());
    let mut height = 0;
    for (at, text) in texts.iter().enumerate() {
        if at > 0 {
            height += gap(text.size);
        }
        tops.push(height);
        height += text.height();
    }
    (tops, height)
}

/// The texts at `placement`, in the order written.
fn at<'s>(sized: &'s [Sized], texts: &[Text], placement: Placement) -> Vec<&'s Sized> {
    sized
        .iter()
        .filter(|s| texts[s.text].placement == placement)
        .collect()
}

/// The height of the tallest stack in `row`, in fifths of a pixel.
fn row_height(sized: &[Sized], texts: &[Text], row: Row) -> u64 {
    Placement::ALL
        .iter()
        .filter(|p| p.row() == row)
        .map(|&p| stack(&at(sized, texts, p)).1)
        .max()
        .unwrap_or(0)
}

/// Places every text with a built text on a card with `padding`.
pub fn lay_out(
    padding: u32,
    texts: &[Text],
    measure: &impl Measure,
) -> Result<Vec<Placed>, Failure> {
    let width = (WIDTH - 2 * padding) as f32;
    let safe = u64::from(HEIGHT - 2 * padding) * 5;
    let size = |text: usize, size: u32| Sized {
        text,
        size,
        lines: measure.measure(texts[text].built, size, width).lines,
    };
    let shown: Vec<usize> = (0..texts.len())
        .filter(|&at| !texts[at].built.is_empty())
        .collect();
    let in_row = |row: Row| {
        shown
            .iter()
            .copied()
            .filter(move |&at| texts[at].placement.row() == row)
    };

    let mut edges: Vec<Sized> = in_row(Row::Top)
        .chain(in_row(Row::Bottom))
        .map(|at| size(at, texts[at].size.most()))
        .collect();
    edges.sort_by_key(|s| s.text);
    let top = row_height(&edges, texts, Row::Top);
    let bottom = row_height(&edges, texts, Row::Bottom);
    let middle: Vec<usize> = in_row(Row::Middle).collect();
    // Rows that exactly meet leave no room, which matters only to a text
    // between them.
    let room = safe
        .checked_sub(top + bottom)
        .filter(|&room| room > 0 || middle.is_empty());
    let Some(room) = room else {
        return Err(Failure::NoRoom {
            top: pixels(top),
            bottom: pixels(bottom),
            safe: pixels(safe),
        });
    };

    let searching = |at: &usize| matches!(texts[*at].size, Size::UpTo(n) if n >= FLOOR);
    let largest = middle
        .iter()
        .filter(|at| searching(at))
        .map(|&at| texts[at].size.most())
        .max();
    let fixed: Vec<Sized> = middle
        .iter()
        .filter(|at| !searching(at))
        .map(|&at| size(at, texts[at].size.most()))
        .collect();
    let mut tried = largest.unwrap_or(FLOOR);
    let centred = loop {
        let mut sized: Vec<Sized> = middle
            .iter()
            .filter(|at| searching(at))
            .map(|&at| size(at, texts[at].size.most().min(tried)))
            .collect();
        sized.extend(fixed.iter().map(|s| Sized {
            text: s.text,
            size: s.size,
            lines: s.lines.clone(),
        }));
        sized.sort_by_key(|s| s.text);
        match too_tall(&sized, texts, room) {
            None => break sized,
            Some(failure) if largest.is_none() || tried == FLOOR => return Err(failure),
            Some(_) => tried -= 1,
        }
    };

    // The safe area's left edge, and its top.
    let left = padding as f32;
    let mut placed: Vec<Placed> = Vec::new();
    for &placement in &Placement::ALL {
        let (sized, from) = match placement.row() {
            Row::Top => (at(&edges, texts, placement), left),
            Row::Bottom => {
                let sized = at(&edges, texts, placement);
                let (_, height) = stack(&sized);
                (sized, (HEIGHT - padding) as f32 - pixels(height))
            }
            Row::Middle => {
                let sized = at(&centred, texts, placement);
                let (_, height) = stack(&sized);
                (sized, left + pixels(top) + pixels(room - height) / 2.0)
            }
        };
        let (tops, _) = stack(&sized);
        for (text, offset) in sized.into_iter().zip(tops) {
            let height = line_height(text.size);
            let lines = text
                .lines
                .iter()
                .enumerate()
                .map(|(n, line)| {
                    let line_width = measure.measure(line, text.size, width).width;
                    let x = match placement.align() {
                        Align::Left => left,
                        Align::Right => left + width - line_width,
                        Align::Centre => left + (width - line_width) / 2.0,
                    };
                    Line {
                        text: line.clone(),
                        x,
                        y: from + pixels(offset + height * n as u64),
                        width: line_width,
                        height: pixels(height),
                    }
                })
                .collect();
            placed.push(Placed {
                text: text.text,
                size: text.size,
                lines,
            });
        }
    }
    placed.sort_by_key(|p| p.text);
    Ok(placed)
}

/// The first middle text, in the order written, whose stack passes
/// `room` by its last line.
fn too_tall(sized: &[Sized], texts: &[Text], room: u64) -> Option<Failure> {
    Placement::ALL
        .iter()
        .filter(|p| p.row() == Row::Middle)
        .filter_map(|&p| {
            let stacked = at(sized, texts, p);
            let (tops, _) = stack(&stacked);
            stacked
                .into_iter()
                .zip(tops)
                .map(|(text, top)| (text, top + text.height()))
                .find(|&(_, bottom)| bottom > room)
        })
        .min_by_key(|(text, _)| text.text)
        .map(|(text, needs)| Failure::TooTall {
            text: text.text,
            size: text.size,
            needs: pixels(needs),
            room: pixels(room),
        })
}

#[cfg(test)]
mod tests;
