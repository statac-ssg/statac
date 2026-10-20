//! Sharing cards: the picture a link preview shows, 1200 by 630, drawn
//! for each page from a description the author writes.
//!
//! `templates/og.yaml` describes the site's card, and `og.<name>.yaml`
//! beside it another, called `<name>`. A description names a background
//! and fonts, files in `templates/og/`, and the texts drawn over the
//! background, each a block of four flat fields: `title.text`,
//! `title.place`, `title.size` and `title.colour`. Texts are drawn in the
//! order their blocks are first written.
//!
//! Here a description is read and checked, and [`layout`] works out where
//! each text goes. A text and a template colour are kept as written, with
//! where they were written, for building with each page.

pub mod draw;
pub mod keep;
pub mod layout;

use std::path::Path;

use crate::content::words::{self, key_name};
use crate::origin::{Origin, Sourced};
use crate::problem::{
    Expected, Found, Hint, Kind, Problem, Reach, Requirer, SetAside, Severity, Word,
};
use crate::source::{Place, SourceId, Sources, Span};
use crate::value::Reader;
use crate::yaml::{Node, Resolved, Value as Yaml};

/// The folder in `templates/` a description's files are named in.
pub const FOLDER: &str = "templates/og";

pub const WIDTH: u32 = 1200;
pub const HEIGHT: u32 = 630;

const PADDING: u32 = 60;
/// The most padding that leaves a card any height.
const MOST_PADDING: u64 = (HEIGHT / 2 - 1) as u64;
const SMALLEST: u32 = 1;
const LARGEST: u32 = HEIGHT;

const TOP_LEVEL: &[&str] = &["background", "font", "padding"];
const FIELDS: &[&str] = &["text", "place", "size", "colour"];
const FONT_ENDINGS: &[&str] = &["ttf", "otf", "ttc"];

/// The cards a site describes, by the files describing them.
#[derive(Default, Debug)]
pub struct Cards {
    pub described: Vec<Described>,
    /// The cards that pages left out of the build pick by name.
    pub picked_elsewhere: Vec<String>,
}

impl Cards {
    /// The card called `name`, or the site's own card for none. None
    /// where nothing describes it, or its description has an error.
    pub fn card(&self, name: Option<&str>) -> Option<&Card> {
        self.described
            .iter()
            .find(|described| described.name.as_deref() == name)
            .and_then(|described| described.card.as_ref())
    }
}

/// One description file.
#[derive(Debug)]
pub struct Described {
    /// None for the site's own card, `templates/og.yaml`.
    pub name: Option<String>,
    /// Its path in the site, `templates/og.yaml`.
    pub path: String,
    /// None where it couldn't be read, which a problem has said.
    pub source: Option<SourceId>,
    /// None where an error stops it describing a card.
    pub card: Option<Card>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Card {
    /// A name in `templates/og/`.
    pub background: Sourced<String>,
    /// Names in `templates/og/`, tried in this order.
    pub fonts: Vec<Sourced<String>>,
    pub padding: Sourced<u32>,
    pub blocks: Vec<Block>,
}

/// A text drawn on a card.
#[derive(Clone, PartialEq, Debug)]
pub struct Block {
    /// The name the author gave it, where its first field is written.
    pub name: Sourced<String>,
    /// A template over `page` and `site`, as written.
    pub text: Sourced<String>,
    pub placement: Sourced<Placement>,
    pub size: Sourced<Size>,
    pub colour: Sourced<Colour>,
}

/// Where on a card a text is drawn.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Placement {
    TopLeft,
    Top,
    TopRight,
    Left,
    Centre,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

/// The row of the card a placement is in.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Row {
    Top,
    Middle,
    Bottom,
}

/// How a placement lines a text up across the card.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Align {
    Left,
    Centre,
    Right,
}

impl Placement {
    pub const ALL: [Placement; 9] = [
        Placement::TopLeft,
        Placement::Top,
        Placement::TopRight,
        Placement::Left,
        Placement::Centre,
        Placement::Right,
        Placement::BottomLeft,
        Placement::Bottom,
        Placement::BottomRight,
    ];

    /// How the author writes it.
    pub fn name(self) -> &'static str {
        match self {
            Placement::TopLeft => "top left",
            Placement::Top => "top",
            Placement::TopRight => "top right",
            Placement::Left => "left",
            Placement::Centre => "centre",
            Placement::Right => "right",
            Placement::BottomLeft => "bottom left",
            Placement::Bottom => "bottom",
            Placement::BottomRight => "bottom right",
        }
    }

    pub fn row(self) -> Row {
        match self {
            Placement::TopLeft | Placement::Top | Placement::TopRight => Row::Top,
            Placement::Left | Placement::Centre | Placement::Right => Row::Middle,
            Placement::BottomLeft | Placement::Bottom | Placement::BottomRight => Row::Bottom,
        }
    }

    pub fn align(self) -> Align {
        match self {
            Placement::TopLeft | Placement::Left | Placement::BottomLeft => Align::Left,
            Placement::Top | Placement::Centre | Placement::Bottom => Align::Centre,
            Placement::TopRight | Placement::Right | Placement::BottomRight => Align::Right,
        }
    }
}

/// A text's size in pixels, rounded to a whole number.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Size {
    Fixed(u32),
    /// `up to` this size: in the middle row, as large as fits.
    UpTo(u32),
}

impl Size {
    /// The size written, whether fixed or the most.
    pub fn most(self) -> u32 {
        match self {
            Size::Fixed(n) | Size::UpTo(n) => n,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Colour {
    Rgb([u8; 3]),
    /// A template over `page` and `site`, as written.
    Template(String),
}

/// The card a description file in `templates/` describes, by its path
/// there: `Some(None)` for `og.yaml`, `Some(Some(name))` for
/// `og.<name>.yaml`, and None for any other file.
pub fn description(rel: &str) -> Option<Option<&str>> {
    let name = rel.strip_prefix("og.")?.strip_suffix("yaml")?;
    match name {
        "" => Some(None),
        _ => {
            let name = name.strip_suffix('.')?;
            (!name.is_empty() && !name.contains(['/', '.'])).then_some(Some(name))
        }
    }
}

/// The cards the front matter of a page written as `text` picks with
/// `card:`, read without reading the page.
pub fn picked_in(text: &str) -> impl Iterator<Item = String> + '_ {
    text.lines()
        .filter_map(|line| line.trim_start().strip_prefix("card:"))
        .map(|name| name.trim().trim_matches(['"', '\'']).to_owned())
        .filter(|name| !name.is_empty())
}

/// Whether a file in `templates/`, by its path there, is a card's rather
/// than a template: a description, or a file in `templates/og/`.
pub fn owns(rel: &str) -> bool {
    description(rel).is_some() || rel.starts_with("og/")
}

/// Reads every card description of the site at `root`.
pub fn read(root: &Path, sources: &mut Sources) -> (Cards, Vec<Problem>) {
    let mut problems = Vec::new();
    let mut cards = Cards::default();
    let real = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    // Walking `templates/` for the templates tells what is wrong with its
    // files and links, so it isn't told again here.
    let found = crate::assets::walk(root, &real, "templates", true, &mut Vec::new());
    let in_folder: Vec<&str> = found
        .iter()
        .filter_map(|(rel, _)| rel.strip_prefix("og/"))
        .collect();
    for (rel, at) in &found {
        let Some(name) = description(rel) else {
            continue;
        };
        let shown = format!("templates/{rel}");
        let mut described = Described {
            name: name.map(str::to_owned),
            path: shown.clone(),
            source: None,
            card: None,
        };
        match crate::assets::read_walked_text(&real, at) {
            Ok(text) => {
                let source = sources.add(&shown, text);
                let mut found = Vec::new();
                let card = read_source(sources, source, &in_folder, &mut found);
                let stopped = found.iter().any(|p| p.kind.severity() == Severity::Error);
                described.source = Some(source);
                described.card = card.filter(|_| !stopped);
                problems.extend(found);
            }
            Err(error) => problems.push(Problem::nowhere(Kind::CantRead {
                path: shown.into(),
                reason: crate::problem::system_reason(&error),
            })),
        }
        cards.described.push(described);
    }
    (cards, problems)
}

/// A field of a block: not written, refused with a problem already given,
/// or read.
#[derive(Debug)]
enum Slot<T> {
    Empty,
    Refused,
    Read(Sourced<T>),
}

impl<T> Slot<T> {
    fn from(read: Result<Option<Sourced<T>>, Problem>, problems: &mut Vec<Problem>) -> Self {
        match read {
            Ok(Some(value)) => Slot::Read(value),
            Ok(None) => Slot::Empty,
            Err(problem) => {
                problems.push(problem);
                Slot::Refused
            }
        }
    }
}

#[derive(Debug)]
struct Pending {
    name: String,
    /// The key of the block's first field, of any name.
    key: Origin,
    /// Whether any of its four fields is written.
    known: bool,
    text: Slot<String>,
    placement: Slot<Placement>,
    size: Slot<Size>,
    colour: Slot<Colour>,
}

/// Reads a description from a source already added, given the names of
/// the files in `templates/og/`. None where something it needs is missing
/// or refused; warnings leave it whole.
pub fn read_source(
    sources: &Sources,
    source: SourceId,
    in_folder: &[&str],
    problems: &mut Vec<Problem>,
) -> Option<Card> {
    let head = words::head(sources, source);
    let node = match crate::yaml::read(sources, source) {
        Ok(node) => node,
        Err(problem) => {
            problems.push(problem);
            return None;
        }
    };
    let entries: &[crate::yaml::Entry] = match &node.value {
        Yaml::Mapping(entries) => entries,
        Yaml::Scalar(_) if node.resolved() == Some(Resolved::Null) => &[],
        _ => {
            let path = sources.path(source).to_string_lossy();
            let reader = Reader::new(sources, &path);
            problems.push(reader.wrong(&node, Expected::Fields, None));
            return None;
        }
    };
    let mut background = (Slot::Empty, head);
    let mut fonts = (Slot::Empty, head);
    let mut padding = Slot::Empty;
    let mut blocks: Vec<Pending> = Vec::new();
    for entry in entries {
        let name = key_name(&entry.key);
        let (key, value) = (&entry.key, &entry.value);
        match name.as_str() {
            "background" => {
                let read = Reader::new(sources, "background").text(value);
                background = (Slot::from(read, problems), key.place);
            }
            "font" => {
                let read = Reader::new(sources, "font").texts(value);
                let read = read.map(|fonts| fonts.filter(|fonts| !fonts.value.is_empty()));
                fonts = (Slot::from(read, problems), key.place);
            }
            "padding" => {
                let read = read_padding(&Reader::new(sources, "padding"), value);
                padding = Slot::from(read, problems);
            }
            _ => match name.split_once('.') {
                Some((block, field)) if !block.is_empty() => {
                    let at = match blocks.iter().position(|b| b.name == block) {
                        Some(at) => at,
                        None => {
                            blocks.push(Pending {
                                name: block.to_owned(),
                                key: key.origin(),
                                known: false,
                                text: Slot::Empty,
                                placement: Slot::Empty,
                                size: Slot::Empty,
                                colour: Slot::Empty,
                            });
                            blocks.len() - 1
                        }
                    };
                    let pending = &mut blocks[at];
                    let reader = Reader::new(sources, &name);
                    match field {
                        "text" => pending.text = Slot::from(reader.text(value), problems),
                        "place" => {
                            pending.placement = Slot::from(read_placement(&reader, value), problems)
                        }
                        "size" => pending.size = Slot::from(read_size(&reader, value), problems),
                        "colour" => {
                            let read = read_colour(&reader, key, value);
                            pending.colour = Slot::from(read, problems);
                        }
                        "color" => {
                            pending.colour = Slot::Refused;
                            problems.push(Problem::at(
                                Kind::OtherSpelling {
                                    word: Word::Field,
                                    written: name.clone(),
                                    ours: format!("{block}.colour"),
                                },
                                key.place,
                            ));
                        }
                        _ => {
                            let known = FIELDS.iter().map(|field| format!("{block}.{field}"));
                            problems.push(set_aside(&name, known.collect(), key));
                            continue;
                        }
                    }
                    pending.known = true;
                }
                _ if matches!(value.value, Yaml::Mapping(_)) => {
                    problems.extend(words::nested(sources, &name, value));
                }
                _ => {
                    let known = TOP_LEVEL.iter().map(|name| (*name).to_owned());
                    problems.push(set_aside(&name, known.collect(), key));
                }
            },
        }
    }

    let background = match background {
        (Slot::Read(name), _) => match check_file(sources, &name, in_folder, false) {
            Ok(()) => Some(name),
            Err(problem) => {
                problems.push(problem);
                None
            }
        },
        (Slot::Empty, at) => {
            problems.push(missing("background", Requirer::Card, at));
            None
        }
        (Slot::Refused, _) => None,
    };
    let fonts = match fonts {
        (Slot::Read(fonts), _) => {
            let mut usable = true;
            for font in &fonts.value {
                if let Err(problem) = check_file(sources, font, in_folder, true) {
                    problems.push(problem);
                    usable = false;
                }
            }
            usable.then_some(fonts.value)
        }
        (Slot::Empty, at) => {
            problems.push(missing("font", Requirer::Card, at));
            None
        }
        (Slot::Refused, _) => None,
    };

    let mut whole = Vec::new();
    for pending in blocks.into_iter().filter(|b| b.known) {
        if let Some(block) = finish(sources, pending, problems) {
            whole.push(block);
        }
    }
    let padding = match padding {
        Slot::Read(padding) => Some(padding),
        Slot::Empty => Some(Sourced::new(PADDING, Origin::defaulted())),
        Slot::Refused => None,
    };
    Some(Card {
        background: background?,
        fonts: fonts?,
        padding: padding?,
        blocks: whole,
    })
}

/// A block with each field read, or the problems for those not written.
fn finish(sources: &Sources, pending: Pending, problems: &mut Vec<Problem>) -> Option<Block> {
    let Pending {
        name,
        key,
        text,
        placement,
        size,
        colour,
        ..
    } = pending;
    let at = key.place.expect("a key is written");
    let mut lacking = |field: &str, empty: bool| {
        if empty {
            problems.push(missing(&format!("{name}.{field}"), Requirer::CardText, at));
        }
    };
    lacking("text", matches!(text, Slot::Empty));
    lacking("place", matches!(placement, Slot::Empty));
    lacking("size", matches!(size, Slot::Empty));
    lacking("colour", matches!(colour, Slot::Empty));
    let (Slot::Read(text), Slot::Read(placement), Slot::Read(size), Slot::Read(colour)) =
        (text, placement, size, colour)
    else {
        return None;
    };
    if placement.value.row() != Row::Middle
        && let Size::UpTo(n) = size.value
        && let (Some(size_at), Some(place_at)) = (size.origin.place, placement.origin.place)
    {
        problems.push(Problem::at(
            Kind::SizeKept {
                field: format!("{name}.size"),
                written: scalar_text(sources, size_at),
                at: scalar_text(sources, place_at),
                size: n,
            },
            size_at,
        ));
    }
    Some(Block {
        name: Sourced::new(name, key),
        text,
        placement,
        size,
        colour,
    })
}

/// A scalar as written, without its quotes.
fn scalar_text(sources: &Sources, place: Place) -> String {
    let spelling = sources.spelling(place);
    let quoted = spelling.len() >= 2
        && (spelling.starts_with('"') && spelling.ends_with('"')
            || spelling.starts_with('\'') && spelling.ends_with('\''));
    match quoted {
        true => spelling[1..spelling.len() - 1].to_owned(),
        false => spelling.to_owned(),
    }
}

fn missing(field: &str, required_by: Requirer, at: Place) -> Problem {
    Problem::at(
        Kind::MissingField {
            field: field.to_owned(),
            required_by,
        },
        at,
    )
}

fn set_aside(name: &str, known: Vec<String>, key: &Node) -> Problem {
    let nearest = crate::suggest::nearest(name, known.iter().map(String::as_str));
    Problem::at(
        Kind::SetAside {
            field: name.to_owned(),
            why: SetAside::NotOnCard {
                nearest: nearest.map(str::to_owned),
            },
        },
        key.place,
    )
}

/// A number from 0 to 314 once rounded.
fn read_padding(reader: &Reader, node: &Node) -> Result<Option<Sourced<u32>>, Problem> {
    let expected = Expected::RoundedBetween {
        from: 0,
        to: MOST_PADDING,
    };
    let within = |n: f64| {
        let n = n.round();
        (n >= 0.0 && n <= MOST_PADDING as f64).then_some(n as u32)
    };
    let padding = match (&node.value, node.resolved()) {
        (_, Some(Resolved::Null)) => return Ok(None),
        (_, Some(Resolved::Int(Some(n)))) => within(n as f64),
        (_, Some(Resolved::Float(f))) => within(f),
        (Yaml::Scalar(text), Some(Resolved::Str)) => {
            let text = text.trim();
            let respelt = number(text).and_then(within).map(|_| text.to_owned());
            return Err(reader.wrong(node, expected, respelt));
        }
        _ => None,
    };
    match padding {
        Some(padding) => Ok(Some(Sourced::new(padding, node.origin()))),
        None => Err(reader.wrong(node, expected, None)),
    }
}

/// One of the nine places, in lower case. Other capitals, and `center`,
/// are other spellings.
fn read_placement(reader: &Reader, node: &Node) -> Result<Option<Sourced<Placement>>, Problem> {
    let Some(text) = reader.text(node)? else {
        return Ok(None);
    };
    if let Some(&placement) = Placement::ALL.iter().find(|p| p.name() == text.value) {
        return Ok(Some(text.map(|_| placement)));
    }
    let ours = match text.value.eq_ignore_ascii_case("center") {
        true => Some(Placement::Centre),
        false => Placement::ALL
            .into_iter()
            .find(|p| p.name().eq_ignore_ascii_case(&text.value)),
    };
    let kind = match ours {
        Some(ours) => Kind::OtherSpelling {
            word: Word::Place,
            written: text.value,
            ours: ours.name().into(),
        },
        None => Kind::UnknownWord {
            word: Word::Place,
            nearest: crate::suggest::nearest(&text.value, Placement::ALL.map(Placement::name))
                .map(str::to_owned),
            written: text.value,
        },
    };
    Err(Problem::at(kind, node.place))
}

/// A number, or `up to` one, from 1 to 630 once rounded.
fn read_size(reader: &Reader, node: &Node) -> Result<Option<Sourced<Size>>, Problem> {
    let wrong = |respelt: Option<String>| reader.wrong(node, Expected::CardSize, respelt);
    let size = match (&node.value, node.resolved()) {
        (_, Some(Resolved::Null)) => return Ok(None),
        (_, Some(Resolved::Int(Some(n)))) => {
            Size::Fixed(rounded(n as f64).ok_or_else(|| wrong(None))?)
        }
        (_, Some(Resolved::Float(f))) => Size::Fixed(rounded(f).ok_or_else(|| wrong(None))?),
        (Yaml::Scalar(text), Some(Resolved::Str)) => match text.strip_prefix("up to ") {
            Some(rest) => {
                let n = number(rest.trim()).ok_or_else(|| wrong(None))?;
                Size::UpTo(rounded(n).ok_or_else(|| wrong(None))?)
            }
            None => return Err(wrong(respelt_size(text))),
        },
        _ => return Err(wrong(None)),
    };
    Ok(Some(Sourced::new(size, node.origin())))
}

/// A size written another way that reads one way only: a number in
/// quotes, or `up to` in other capitals or spacing.
fn respelt_size(text: &str) -> Option<String> {
    let text = text.trim();
    if let Some(n) = number(text)
        && rounded(n).is_some()
    {
        return Some(text.to_owned());
    }
    let lower = text.to_ascii_lowercase();
    let rest = lower
        .strip_prefix("up")?
        .trim_start()
        .strip_prefix("to")?
        .trim_start();
    let n = number(rest)?;
    rounded(n).map(|_| format!("up to {rest}"))
}

/// Figures, with a fraction after a point if any.
fn number(text: &str) -> Option<f64> {
    let (whole, fraction) = text.split_once('.').unwrap_or((text, "0"));
    let figures = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    (figures(whole) && figures(fraction))
        .then(|| text.parse().ok())
        .flatten()
}

fn rounded(n: f64) -> Option<u32> {
    let n = n.round();
    (n.is_finite() && n >= f64::from(SMALLEST) && n <= f64::from(LARGEST)).then_some(n as u32)
}

/// `#RGB`, `#RRGGBB`, or a template, which holds `{{`.
fn read_colour(
    reader: &Reader,
    key: &Node,
    node: &Node,
) -> Result<Option<Sourced<Colour>>, Problem> {
    if node.resolved() == Some(Resolved::Null)
        && let Some(comment) = comment_after(reader.sources, key)
    {
        return Err(Problem::at(
            Kind::NeedsQuotes {
                written: reader.sources.spelling(comment).to_owned(),
                reading: "a comment".into(),
            },
            comment,
        ));
    }
    let Some(text) = reader.text(node)? else {
        return Ok(None);
    };
    if text.value.contains("{{") {
        return Ok(Some(text.map(Colour::Template)));
    }
    match hex(&text.value) {
        Some(rgb) => Ok(Some(text.map(|_| Colour::Rgb(rgb)))),
        None => Err(reader.wrong(node, Expected::Colour, None)),
    }
}

/// The colour `text` writes as `#RGB` or `#RRGGBB`.
pub fn hex(text: &str) -> Option<[u8; 3]> {
    let digits = text.strip_prefix('#')?;
    if !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |at: usize, wide: usize| {
        let n = u8::from_str_radix(&digits[at..at + wide], 16).ok()?;
        Some(if wide == 1 { n * 17 } else { n })
    };
    match digits.len() {
        3 => Some([channel(0, 1)?, channel(1, 1)?, channel(2, 1)?]),
        6 => Some([channel(0, 2)?, channel(2, 2)?, channel(4, 2)?]),
        _ => None,
    }
}

/// The comment on the line of `key`, after its colon, where nothing
/// stands between them: YAML's reading of a bare `#F2E9D3`.
fn comment_after(sources: &Sources, key: &Node) -> Option<Place> {
    let source = key.place.source;
    let text = sources.text(source);
    let from = key.place.span.end as usize;
    let line_end = text[from..]
        .find(['\n', '\r'])
        .map_or(text.len(), |at| from + at);
    let rest = text[from..line_end].trim_start().strip_prefix(':')?;
    let comment = rest.trim();
    if !comment.starts_with('#') {
        return None;
    }
    let start = line_end - rest.trim_start().len();
    Some(Place {
        source,
        span: Span::new(start, start + comment.len()),
    })
}

/// The folders and the file a name in `templates/og/` names, in turn.
fn parts(written: &str) -> Vec<&str> {
    written
        .split(['/', '\\'])
        .filter(|part| !part.is_empty() && *part != ".")
        .collect()
}

/// The path in the site of the file a description names as `written`.
pub fn file_named(written: &str) -> String {
    format!("{FOLDER}/{}", parts(written).join("/"))
}

/// Whether a file's name ends as a font Statac reads does.
pub fn is_a_font(name: &str) -> bool {
    let ending = name.rsplit_once('.').map(|(_, ending)| ending);
    ending.is_some_and(|ending| FONT_ENDINGS.iter().any(|f| f.eq_ignore_ascii_case(ending)))
}

/// Refuses a name reaching outside `templates/og/`, a font of a kind
/// Statac doesn't read, and a file that isn't there, in that order.
fn check_file(
    sources: &Sources,
    name: &Sourced<String>,
    in_folder: &[&str],
    font: bool,
) -> Result<(), Problem> {
    let place = name.origin.place.expect("a name is written");
    let written = &name.value;
    let parts = parts(written);
    let drive = written.len() >= 2 && written.as_bytes()[1] == b':';
    if written.starts_with(['/', '\\']) || drive || parts.contains(&"..") {
        return Err(Problem::at(
            Kind::OutsideSite {
                reach: Reach::CardFile(written.clone()),
            },
            place,
        ));
    }
    if font && !is_a_font(written) {
        return Err(Problem::at(
            Kind::WrongKind {
                field: "font".into(),
                expected: Expected::FontFile,
                found: Found::Written(sources.spelling(place).to_owned()),
                respelt: None,
            },
            place,
        ));
    }
    let path = parts.join("/");
    if in_folder.contains(&path.as_str()) {
        return Ok(());
    }
    Err(Problem::at(
        Kind::MissingFile {
            path: format!("{FOLDER}/{path}"),
            hint: crate::suggest::nearest(&path, in_folder.iter().copied())
                .map(|nearest| Hint::Closest(nearest.to_owned())),
        },
        place,
    ))
}

#[cfg(test)]
mod tests;
