//! A page's sharing card as a layout reads it: `page.card`.
//!
//! A card's texts, and a colour written as a template, are templates
//! over `page` and `site`, read and checked with the site's own and built
//! for each page that asks. Each is held as a source of its own, and a
//! problem in one is told where it stands in the description.
//!
//! Asking is what gives a page its card: the texts are built and placed,
//! and the card's address answered, while the page is built, and the card
//! itself is drawn once every page is. A page naming a picture of its own
//! with `og_image` answers with that instead.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use minijinja::value::Value;

use super::files::{File, Role};
use super::frame::{self, Frame, Making};
use super::render::{Shared, met_building};
use super::values::{Ordered, PageValue, SiteValue};
use super::{Templates, explain, media};
use crate::album::Seen;
use crate::cards::draw::{Failure, Key as CardKey, Kit, KitFailure, Plan, Text};
use crate::cards::keep::{Store, Wanted, kept_as};
use crate::cards::layout::{self, FLOOR};
use crate::cards::{self, Colour, HEIGHT, Placement, Size, WIDTH};
use crate::content::address::is_full;
use crate::content::words::{self, PageWord};
use crate::origin::{Arrival, Sourced};
use crate::pictures::{self, Kind as PictureKind};
use crate::problem::{
    CardFileFault, Expected, FontWithThem, Found, Kind, NotDrawn, Problem, SetAside, Stopped,
    Waiting, Word, system_reason,
};
use crate::reads::{Key, Reads};
use crate::site::{Id, PageId, PictureId, Site};
use crate::source::{Place, SourceId, Sources, Span};

/// What the engine's name for a card's text opens with, which no file's
/// name does.
pub(super) const PREFIX: &str = "card:";

/// The kinds of picture every link preview is taken to show. A picture
/// `og_image` names of any other kind is used all the same, and warned of.
/// Which kinds these are changes with the years, and this list is all
/// that changes with them: nothing else asks what kind such a picture is.
const PREVIEWED: [PictureKind; 3] = [
    PictureKind::Jpeg,
    PictureKind::Png,
    PictureKind::AnimatedPng,
];

/// The cards a site describes, their texts held as templates.
#[derive(Default)]
pub(super) struct Cards {
    described: Vec<cards::Described>,
    /// For each description, the engine's names of each text's template
    /// and of its colour's, where that is one.
    names: Vec<Vec<(String, Option<String>)>>,
    /// Where each template stands in its description, by the source it is
    /// read as.
    stands: HashMap<SourceId, Stand>,
    /// The cards picked by pages left out of the build.
    picked_elsewhere: Vec<String>,
    /// The site's folder.
    root: PathBuf,
}

/// Where a template stands in a description.
struct Stand {
    /// The value it is, as written.
    scalar: Place,
    /// Where its first character is written, where the value is written
    /// character for character as YAML reads it.
    starts: Option<usize>,
}

impl Cards {
    /// The description of the card called `name`, by its index.
    fn find(&self, name: Option<&str>) -> Option<usize> {
        self.described
            .iter()
            .position(|described| described.name.as_deref() == name)
    }

    /// `place` as it stands in a description, where it is in one of its
    /// templates.
    fn real(&self, place: Place) -> Place {
        match self.stands.get(&place.source) {
            Some(Stand {
                scalar,
                starts: Some(starts),
            }) => Place {
                source: scalar.source,
                span: Span::new(
                    starts + place.span.start as usize,
                    starts + place.span.end as usize,
                ),
            },
            Some(stand) => stand.scalar,
            None => place,
        }
    }

    /// `problem` told where it stands in a description.
    pub(super) fn placed(&self, problem: Problem) -> Problem {
        match self.stands.is_empty() {
            true => problem,
            false => problem.moved(|place| self.real(place)),
        }
    }
}

/// The files the cards are drawn with, their backgrounds and fonts, each
/// by its path in the site.
pub(super) fn files(cards: &Cards) -> Vec<String> {
    let described = cards.described.iter();
    described
        .filter_map(|described| described.card.as_ref())
        .flat_map(|card| std::iter::once(&card.background).chain(&card.fonts))
        .map(|named| cards::file_named(&named.value))
        .collect()
}

/// Holds the texts of `cards` as templates among `files`, each read as a
/// source of its own.
pub(super) fn gather(
    root: &Path,
    cards: cards::Cards,
    sources: &mut Sources,
    files: &mut Vec<File>,
) -> Cards {
    let mut gathered = Cards {
        root: root.to_path_buf(),
        picked_elsewhere: cards.picked_elsewhere,
        ..Cards::default()
    };
    for described in cards.described {
        let mut names = Vec::new();
        let rel = described.path.rsplit('/').next().unwrap_or_default();
        for block in described.card.iter().flat_map(|card| &card.blocks) {
            let mut hold = |field: &str, text: &Sourced<String>| {
                let name = format!("{PREFIX}{rel}:{}.{field}", block.name.value);
                let source = sources.add(described.path.clone(), text.value.clone());
                if let Some(scalar) = text.origin.place {
                    let spelling = sources.spelling(scalar);
                    let start = scalar.span.start as usize;
                    let quoted = spelling.len() == text.value.len() + 2
                        && spelling.get(1..spelling.len() - 1) == Some(text.value.as_str());
                    let starts = match (spelling == text.value, quoted) {
                        (true, _) => Some(start),
                        (false, true) => Some(start + 1),
                        (false, false) => None,
                    };
                    gathered.stands.insert(source, Stand { scalar, starts });
                }
                files.push(File {
                    name: name.clone(),
                    source,
                    role: Role::CardText,
                });
                name
            };
            let text = hold("text", &block.text);
            let colour = match &block.colour.value {
                Colour::Template(written) => {
                    let written = Sourced::new(written.clone(), block.colour.origin.clone());
                    Some(hold("colour", &written))
                }
                Colour::Rgb(_) => None,
            };
            names.push((text, colour));
        }
        gathered.names.push(names);
        gathered.described.push(described);
    }
    gathered
}

/// What is wrong with the cards the pages of `site` pick: a `card` naming
/// none, each told once where it is written, and a `card` on a page that
/// names its own picture.
pub(super) fn check_pages(site: &Site, cards: &Cards) -> Vec<Problem> {
    let mut problems = Vec::new();
    let mut told = HashSet::new();
    let known = || cards.described.iter().filter_map(|d| d.name.as_deref());
    for (_, page) in site.pages.iter() {
        let Some(picked) = page.words.text(PageWord::Card) else {
            continue;
        };
        let Some(place) = picked.origin.place else {
            continue;
        };
        if cards.find(Some(picked.value)).is_none() && told.insert(place) {
            let kind = Kind::UnknownWord {
                word: Word::Card,
                written: picked.value.to_owned(),
                nearest: crate::suggest::nearest(picked.value, known()).map(str::to_owned),
            };
            problems.push(Problem::at(kind, place));
        }
        let own = page.words.text(PageWord::OgImage);
        if picked.origin.arrival == Arrival::Written
            && own.is_some_and(|own| own.origin.arrival == Arrival::Written)
        {
            let kind = Kind::SetAside {
                field: "card".into(),
                why: SetAside::OwnPicture,
            };
            problems.push(Problem::at(kind, place));
        }
    }
    problems
}

/// What the site's cards come to, each worked out once and kept. Nothing
/// here says which output asked for what: each output carries that itself.
pub(super) struct Room {
    /// What each description's cards are drawn with, made when first
    /// asked for; none where its files can't be used.
    kits: Vec<OnceLock<Option<Kit>>>,
    /// What is wrong with those files.
    unusable: Mutex<Vec<Problem>>,
    /// Each page's card, worked out when first asked for.
    answers: Vec<OnceLock<Arc<Answer>>>,
    /// The cards earlier builds kept.
    store: Store,
}

impl Room {
    pub fn new(cards: &Cards, pages: usize) -> Room {
        Room {
            kits: std::iter::repeat_with(OnceLock::new)
                .take(cards.described.len())
                .collect(),
            unusable: Mutex::new(Vec::new()),
            answers: std::iter::repeat_with(OnceLock::new).take(pages).collect(),
            store: Store::open(&cards.root),
        }
    }
}

/// What asking for a page's card came to.
struct Answer {
    reads: Reads,
    problems: Vec<Problem>,
    /// The photographs whose copies working it out named.
    developed: Vec<crate::album::Shown>,
    /// Whether the page has a card that waits for the site's `url`.
    waiting: bool,
    outcome: Outcome,
}

/// A page's card as an output asked for it.
#[derive(Clone)]
pub struct Asked {
    page: PageId,
    answer: Arc<Answer>,
}

impl std::fmt::Debug for Asked {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Asked({:?})", self.page)
    }
}

impl Asked {
    /// The page whose card it is.
    pub fn page(&self) -> PageId {
        self.page
    }

    /// The file the card is published as, from the top of the built site,
    /// where one is drawn.
    pub fn file(&self) -> Option<&str> {
        match &self.answer.outcome {
            Outcome::Drawn(shown, _) => Some(&shown.file),
            _ => None,
        }
    }
}

/// The card of `page`, as an output that read it asked for it.
pub(super) fn asked(shared: &Arc<Shared>, page: PageId) -> Asked {
    let slot = &shared.cards.answers[page.index()];
    let answer = slot.get_or_init(|| Arc::new(answer(shared, page)));
    Asked {
        page,
        answer: Arc::clone(answer),
    }
}

/// What the outputs built asked of the site's cards: each page's card
/// once, in the order of the pages.
pub struct Gathered {
    answers: BTreeMap<PageId, Arc<Answer>>,
}

impl Gathered {
    pub fn of<'a>(asked: impl IntoIterator<Item = &'a Asked>) -> Gathered {
        let asked = asked.into_iter();
        Gathered {
            answers: asked
                .map(|asked| (asked.page, Arc::clone(&asked.answer)))
                .collect(),
        }
    }

    /// The files the cards drawn are published as, from the top of the
    /// built site.
    pub fn files(&self) -> impl Iterator<Item = &str> {
        self.answers
            .values()
            .filter_map(|answer| match &answer.outcome {
                Outcome::Drawn(shown, _) => Some(shown.file.as_str()),
                _ => None,
            })
    }
}

enum Outcome {
    /// No card: the page has none to have, or a problem told says why.
    None,
    /// The picture the page names itself.
    Own(Shown, Own),
    Drawn(Shown, Drawing),
    /// A card that can't be drawn, of the description by its index, and
    /// where in it the cause is written.
    NotDrawn(usize, NotDrawn, Option<Place>),
}

/// A card as a layout reads it.
struct Shown {
    url: String,
    /// The file published, from the top of the built site.
    file: String,
    size: Option<pictures::Size>,
    alt: String,
}

impl Shown {
    fn value(&self) -> Value {
        let side = |side: Option<u32>| side.map_or_else(|| Value::from(""), Value::from);
        Value::from_object(Ordered(vec![
            ("url".into(), Value::from(self.url.as_str())),
            ("width".into(), side(self.size.map(|size| size.width))),
            ("height".into(), side(self.size.map(|size| size.height))),
            ("alt".into(), Value::from(self.alt.as_str())),
        ]))
    }
}

/// A picture a page names as its own.
struct Own {
    /// Which of the site's pictures it is, where its bytes begin as one.
    picture: Option<PictureId>,
    /// Its file, as a problem names it.
    path: String,
    /// What its bytes say it is, where they say.
    kind: Option<PictureKind>,
    /// Where the page names it.
    named: Option<Place>,
}

/// A card to draw.
struct Drawing {
    /// Its description, by its index.
    described: usize,
    texts: Vec<Built>,
    key: CardKey,
    /// The texts placed, where no card kept showed it can be drawn.
    plan: Option<Plan>,
}

/// A text built for a page.
struct Built {
    placement: Placement,
    size: Size,
    built: String,
    colour: [u8; 3],
}

fn texts(built: &[Built]) -> Vec<Text<'_>> {
    built
        .iter()
        .map(|text| Text {
            placement: text.placement,
            size: text.size,
            built: &text.built,
            colour: text.colour,
        })
        .collect()
}

/// `page.card` on `page`: its card's address, size and words, or none
/// where it has no card.
pub(super) fn of(shared: &Arc<Shared>, page: PageId) -> Option<Value> {
    let refused = match frame::with(|frame| frame.making) {
        Some(Making::Writing) => Some(Stopped::CardInWriting),
        Some(Making::Card) => Some(Stopped::CardInCard),
        _ => None,
    };
    if let Some(reason) = refused {
        let error = frame::raise(Kind::TemplateStopped { reason });
        return Some(Value::from(error));
    }
    frame::read(Key::Card(page));
    let Asked { answer, .. } = asked(shared, page);
    frame::with(|frame| {
        for &key in answer.reads.keys() {
            frame.reads.read(key);
        }
        frame.developed.extend(answer.developed.iter().copied());
    });
    match &answer.outcome {
        Outcome::Own(shown, _) | Outcome::Drawn(shown, _) => Some(shown.value()),
        Outcome::None | Outcome::NotDrawn(..) => None,
    }
}

fn answer(shared: &Arc<Shared>, page: PageId) -> Answer {
    frame::push(Frame {
        making: Making::Card,
        ..Frame::new(Arc::clone(shared), Some(page))
    });
    let mut waiting = false;
    let outcome = outcome(shared, page, &mut waiting);
    let frame = frame::pop();
    let cards = &shared.templates.cards;
    Answer {
        reads: frame.reads,
        problems: frame
            .problems
            .into_iter()
            .map(|problem| cards.placed(problem))
            .collect(),
        developed: frame.developed,
        waiting,
        outcome,
    }
}

/// Tells `kind` at `place` as met building the page the frame is for.
fn tell(kind: Kind, place: Option<Place>) {
    frame::with(|frame| {
        let problem = met_building(Problem::placed(kind, place), frame);
        if !frame.problems.contains(&problem) {
            frame.problems.push(problem);
        }
    });
}

/// The card of the page `id`; `waiting` is set where it has one that
/// waits for the site's `url`.
fn outcome(shared: &Arc<Shared>, id: PageId, waiting: &mut bool) -> Outcome {
    let page = shared.site.pages.get(id);
    let cards = &shared.templates.cards;
    for name in ["redirect_to", "index_page", "og_image", "card", "url"] {
        frame::read(Key::Field(id, shared.name_id(name)));
    }
    frame::read(Key::Setting(shared.name_id("url")));
    if !page.lands() {
        return Outcome::None;
    }
    let own = page.words.text(PageWord::OgImage);
    let picked = page.words.text(PageWord::Card);
    let described = cards
        .find(picked.as_ref().map(|picked| picked.value))
        .filter(|&at| cards.described[at].card.is_some());
    if shared.settings.text("url").is_none() {
        *waiting = own.is_some() || described.is_some();
        return Outcome::None;
    }
    if let Some(own) = own {
        return own_picture(shared, id, &own);
    }
    match described {
        Some(at) => drawn(shared, id, at),
        None => Outcome::None,
    }
}

/// Where the card named `name` is published for the page `id`, beside
/// it: its file from the top of the built site, and its address.
fn beside(shared: &Shared, id: PageId, name: &str) -> (String, String) {
    let page = shared.site.pages.get(id).address.file();
    let folder = &page[..page.rfind('/').map_or(0, |at| at + 1)];
    let file = format!("{folder}{name}");
    let url = media::address_in(shared, &format!("/{file}"));
    (file, url)
}

/// The words of `page` that stand for a picture with none of its own:
/// its `og_title`, or else what it is called.
fn title(shared: &Arc<Shared>, id: PageId) -> String {
    frame::read(Key::Field(id, shared.name_id("og_title")));
    match shared.site.pages.get(id).words.text(PageWord::OgTitle) {
        Some(title) => title.value.to_owned(),
        None => super::own::headline(shared, id).unwrap_or_else(|_| {
            frame::with(|frame| frame.raised = None);
            String::new()
        }),
    }
}

/// The picture `og_image` names on the page `id`, published as it is,
/// whatever kind of file it is.
fn own_picture(shared: &Arc<Shared>, id: PageId, named: &Sourced<&str>) -> Outcome {
    let place = named.origin.place;
    if is_full(named.value) {
        let kind = Kind::WrongKind {
            field: "og_image".into(),
            expected: Expected::OwnPicture,
            found: Found::Written(named.value.to_owned()),
            respelt: None,
        };
        tell(kind, place);
        return Outcome::None;
    }
    let Some(found) = media::found(shared, id, named.value) else {
        // A file whose bytes begin as no picture Statac knows is a file
        // of the site's like any other.
        let Some(file) = media::named(shared, id, named.value) else {
            tell(media::missing_picture(shared, id, named.value), place);
            return Outcome::None;
        };
        let shown = Shown {
            url: media::address_in(shared, &file.address),
            file: file.address.trim_start_matches('/').to_owned(),
            size: None,
            alt: title(shared, id),
        };
        let own = Own {
            picture: None,
            path: file.shown,
            kind: None,
            named: place,
        };
        return Outcome::Own(shown, own);
    };
    frame::read(Key::Picture(found));
    let picture = shared.site.pictures.get(found);
    let (kind, size) = match &picture.seen {
        // The file travels as it is, so its size is as its pixels are
        // stored, whichever way up it is shown.
        Seen::Photograph { photograph, .. } => (photograph.kind, Some(photograph.stored)),
        Seen::Plain { kind, size } => (*kind, *size),
        Seen::Unreadable(fault) => {
            let kind = Kind::UnreadablePicture {
                path: picture.shown(),
                fault: fault.clone(),
            };
            tell(kind, place);
            return Outcome::None;
        }
    };
    let Some(address) = picture.address.as_deref() else {
        let kind = Kind::MissingFile {
            path: picture.shown(),
            hint: None,
        };
        tell(kind, place);
        return Outcome::None;
    };
    let described = picture.sidecar.as_ref().and_then(|said| said.alt.as_ref());
    let alt = match described.filter(|alt| !alt.value.trim().is_empty()) {
        Some(alt) => alt.value.clone(),
        None => title(shared, id),
    };
    let shown = Shown {
        url: media::address_in(shared, address),
        file: address.trim_start_matches('/').to_owned(),
        size,
        alt,
    };
    let own = Own {
        picture: Some(found),
        path: picture.shown(),
        kind: Some(kind),
        named: place,
    };
    Outcome::Own(shown, own)
}

/// What the cards of the description at `at` are drawn with.
fn kit(shared: &Shared, at: usize) -> Option<&Kit> {
    shared.cards.kits[at]
        .get_or_init(|| {
            let made = make_kit(&shared.templates.cards, at);
            made.map_err(|problem| {
                let mut unusable = shared.cards.unusable.lock().expect("never poisoned");
                unusable.push(problem);
            })
            .ok()
        })
        .as_ref()
}

fn make_kit(cards: &Cards, at: usize) -> Result<Kit, Problem> {
    let described = &cards.described[at];
    let card = described.card.as_ref().expect("a description read whole");
    let read = |named: &Sourced<String>| {
        let path = cards::file_named(&named.value);
        std::fs::read(crate::platform::join(&cards.root, &path)).map_err(|error| {
            let kind = Kind::CantRead {
                path: PathBuf::from(&path),
                reason: system_reason(&error),
            };
            Problem::placed(kind, named.origin.place)
        })
    };
    let background = read(&card.background)?;
    let fonts: Vec<Vec<u8>> = card.fonts.iter().map(read).collect::<Result<_, _>>()?;
    let fonts: Vec<&[u8]> = fonts.iter().map(Vec::as_slice).collect();
    let made = crate::album::quietly(|| Kit::new(&background, &fonts, card.padding.value));
    made.map_err(|failure| {
        let unusable = |named: &Sourced<String>, fault| {
            let kind = Kind::UnusableCardFile {
                path: cards::file_named(&named.value),
                fault,
            };
            Problem::placed(kind, named.origin.place)
        };
        match failure {
            KitFailure::BackgroundSize(size) => unusable(
                &card.background,
                CardFileFault::BackgroundSize {
                    width: size.width,
                    height: size.height,
                },
            ),
            KitFailure::Background(pictures::Error::NotAPhotograph(_)) => {
                unusable(&card.background, CardFileFault::NotAStill)
            }
            KitFailure::Background(error) => {
                let kind = Kind::UnreadablePicture {
                    path: cards::file_named(&card.background.value),
                    fault: crate::album::fault(error),
                };
                Problem::placed(kind, card.background.origin.place)
            }
            KitFailure::Font(font) => unusable(&card.fonts[font], CardFileFault::Font),
        }
    })
}

/// Builds the template `name` for the page `id`. None where it stopped,
/// which is told.
fn build(shared: &Arc<Shared>, name: &str, id: PageId) -> Option<String> {
    let templates = &shared.templates;
    let at = templates.by_name[name];
    frame::read(Key::Template(Templates::id(at)));
    if templates.unusable.contains(&at) {
        return None;
    }
    let context = minijinja::context! {
        page => PageValue::value(shared, id),
        site => SiteValue::value(shared, frame::language()),
    };
    // Each text is told on its own; what an earlier one met is no part
    // of this one.
    frame::with(|frame| {
        frame.handed = None;
        frame.refusal = None;
        frame.raised = None;
    });
    frame::with(|frame| frame.layout = Some(at));
    let built = templates.env.get_template(name).and_then(|template| {
        frame::begin(at, None)?;
        frame::end(
            at,
            super::indent::render(&template, context, templates.placing(name), false),
        )?
    });
    match built {
        Ok(text) => Some(text.trim().to_owned()),
        Err(error) => {
            let problems =
                frame::with(|frame| explain::explain(&error, frame, &[])).unwrap_or_default();
            frame::with(|frame| frame.problems.extend(problems));
            None
        }
    }
}

/// Texts joined as sentences, for a reader who can't see them drawn: a
/// full stop after each but the last, where it doesn't end with a mark of
/// its own.
fn sentences<'a>(texts: impl Iterator<Item = &'a str>) -> String {
    const ENDS: [char; 9] = [
        '.', '!', '?', '\u{2026}', '\u{3002}', '\u{ff01}', '\u{ff1f}', '\u{61f}', '\u{6d4}',
    ];
    let mut joined = String::new();
    for text in texts.filter(|text| !text.is_empty()) {
        if !joined.is_empty() {
            joined.push_str(match joined.ends_with(ENDS) {
                true => " ",
                false => ". ",
            });
        }
        joined.push_str(text);
    }
    joined
}

/// The card of the description at `at`, as the page `id` fills it.
fn drawn(shared: &Arc<Shared>, id: PageId, at: usize) -> Outcome {
    let cards = &shared.templates.cards;
    let described = &cards.described[at];
    let card = described.card.as_ref().expect("a description read whole");
    let Some(kit) = kit(shared, at) else {
        return Outcome::None;
    };
    let mut built = Vec::with_capacity(card.blocks.len());
    let mut whole = true;
    for (block, (text, colour)) in card.blocks.iter().zip(&cards.names[at]) {
        let text = build(shared, text, id);
        let colour = match (&block.colour.value, colour) {
            (Colour::Rgb(rgb), _) => Some(*rgb),
            (Colour::Template(_), Some(colour)) => build(shared, colour, id).and_then(|built| {
                let colour = cards::hex(&built);
                if colour.is_none() {
                    let kind = Kind::WrongKind {
                        field: format!("{}.colour", block.name.value),
                        expected: Expected::BuiltColour,
                        found: match built.is_empty() {
                            true => Found::Nothing,
                            false => Found::Written(built),
                        },
                        respelt: None,
                    };
                    tell(kind, block.colour.origin.place);
                }
                colour
            }),
            (Colour::Template(_), None) => None,
        };
        match (text, colour) {
            (Some(text), Some(colour)) => built.push(Built {
                placement: block.placement.value,
                size: block.size.value,
                built: text,
                colour,
            }),
            _ => whole = false,
        }
    }
    if !whole {
        return Outcome::None;
    }

    let texts = texts(&built);
    let key = kit.key(&texts);
    // A card kept was drawn, so it can be.
    let plan = match shared.cards.store.has(&key, kit.format()) {
        true => None,
        false => match kit.plan(&texts) {
            Ok(plan) => Some(plan),
            Err(failure) => {
                let (why, place) = not_drawn(shared, at, &built, failure);
                return Outcome::NotDrawn(at, why, place);
            }
        },
    };
    let name = format!("og.{}.{}", key.hash8(), kit.format().extension());
    let (file, url) = beside(shared, id, &name);
    let said = built.iter().map(|text| text.built.as_str());
    let shown = Shown {
        url,
        file,
        size: Some(pictures::Size {
            width: WIDTH,
            height: HEIGHT,
        }),
        alt: sentences(said),
    };
    Outcome::Drawn(
        shown,
        Drawing {
            described: at,
            texts: built,
            key,
            plan,
        },
    )
}

/// Why a card of the description at `at` can't be drawn, and where in
/// the description that is written.
fn not_drawn(
    shared: &Shared,
    at: usize,
    built: &[Built],
    failure: Failure,
) -> (NotDrawn, Option<Place>) {
    let described = &shared.templates.cards.described[at];
    let card = described.card.as_ref().expect("a description read whole");
    let head = described
        .source
        .map(|source| words::head(&shared.sources, source));
    match failure {
        Failure::Missing(characters) => {
            let holds = |text: &Built| text.built.contains(characters.as_slice());
            let block = built.iter().position(holds);
            let place = block.and_then(|block| card.blocks[block].text.origin.place);
            let why = NotDrawn::NoFont {
                characters,
                more: 0,
                font: None,
            };
            (why, place.or(head))
        }
        Failure::Layout(layout::Failure::TooTall { text, size, .. }) => {
            let block = &card.blocks[text];
            let fitted = matches!(block.size.value, Size::UpTo(most) if most >= FLOOR);
            let why = NotDrawn::TooTall {
                text: block.name.value.clone(),
                size,
                smallest: fitted && size == FLOOR,
            };
            (why, block.size.origin.place.or(head))
        }
        // Placing texts encodes nothing.
        Failure::Layout(layout::Failure::NoRoom { .. }) | Failure::Encode(_) => {
            (NotDrawn::NoRoom, head)
        }
    }
}

/// The most characters a cause names of those no font has.
const NAMED: usize = 5;

/// Why some pages of one description are left without a card.
struct Cause<'a> {
    described: usize,
    why: &'a NotDrawn,
    place: Option<Place>,
    /// The first page left without for it, and how many are.
    first: PageId,
    pages: usize,
    /// The characters no font has, each with how many pages need it.
    needed: Vec<(char, usize)>,
}

impl Cause<'_> {
    /// Whether a card of the description at `described` not drawn for
    /// `why`, `lacking` those characters, is not drawn for this cause.
    fn is(&self, described: usize, why: &NotDrawn, lacking: &[char]) -> bool {
        self.described == described
            && match (self.why, why) {
                (NotDrawn::NoFont { .. }, NotDrawn::NoFont { .. }) => {
                    self.needed.iter().any(|(c, _)| lacking.contains(c))
                }
                (this, other) => this == other,
            }
    }

    fn needs(&mut self, c: char, pages: usize) {
        match self.needed.iter_mut().find(|(needed, _)| *needed == c) {
            Some((_, count)) => *count += pages,
            None => self.needed.push((c, pages)),
        }
    }
}

/// Everything to say of the site's cards once its outputs are built,
/// having `asked` what they did of them.
pub(super) fn told(shared: &Shared, asked: &Gathered) -> Vec<Problem> {
    let cards = &shared.templates.cards;
    let room = &shared.cards;
    let site = &shared.site;
    let mut problems: Vec<Problem> = Vec::new();
    let answers = || {
        let answers = asked.answers.iter();
        answers.map(|(&page, answer)| (page, answer.as_ref()))
    };

    // A description nothing asked for is held to its files all the same.
    let has_url = shared.settings.text("url").is_some();
    if has_url {
        for (at, described) in cards.described.iter().enumerate() {
            if described.card.is_some() {
                kit(shared, at);
            }
        }
    }
    let mut unusable = room.unusable.lock().expect("never poisoned").clone();
    unusable.sort_by_key(|problem| problem.place.map(|at| (at.source, at.span.start)));
    problems.extend(unusable);

    // What working out a page's card met, alike on every page asking, is
    // told once, with the first page that met it.
    let mut met: Vec<&Problem> = Vec::new();
    for (_, answer) in answers() {
        for problem in &answer.problems {
            if !met.iter().any(|told| told.alike_but_for_page(problem)) {
                met.push(problem);
            }
        }
    }
    problems.extend(met.into_iter().cloned());

    // The pages left without a card, one cause at a time. Pages needing
    // some of the same characters are left without for one cause.
    let mut causes: Vec<Cause> = Vec::new();
    for (page, answer) in answers() {
        let Outcome::NotDrawn(at, why, place) = &answer.outcome else {
            continue;
        };
        let lacking: &[char] = match why {
            NotDrawn::NoFont { characters, .. } => characters,
            _ => &[],
        };
        let alike: Vec<usize> = (0..causes.len())
            .filter(|&cause| causes[cause].is(*at, why, lacking))
            .collect();
        let into = match alike.split_first() {
            Some((&first, others)) => {
                for &other in others.iter().rev() {
                    let merged = causes.remove(other);
                    causes[first].pages += merged.pages;
                    for (c, pages) in merged.needed {
                        causes[first].needs(c, pages);
                    }
                }
                first
            }
            None => {
                causes.push(Cause {
                    described: *at,
                    why,
                    place: *place,
                    first: page,
                    pages: 0,
                    needed: Vec::new(),
                });
                causes.len() - 1
            }
        };
        causes[into].pages += 1;
        for &c in lacking {
            causes[into].needs(c, 1);
        }
    }
    for mut cause in causes {
        let why = match cause.why {
            NotDrawn::NoFont { .. } => {
                cause
                    .needed
                    .sort_by_key(|&(_, pages)| std::cmp::Reverse(pages));
                let characters: Vec<char> = cause.needed.iter().map(|&(c, _)| c).collect();
                let named = characters.len().min(NAMED);
                let starter = &crate::new::update::Starter::OWN;
                let font = font_here(&cards.root, &characters)
                    .map(FontWithThem::Here)
                    .or_else(|| {
                        crate::new::update::font_to_bring(&cards.root, starter, &characters)
                            .map(|font| FontWithThem::Brought(font.to_owned()))
                    });
                NotDrawn::NoFont {
                    more: characters.len() - named,
                    characters: characters[..named].to_vec(),
                    font: font.map(Box::new),
                }
            }
            other => other.clone(),
        };
        let kind = Kind::CardNotDrawn {
            why,
            pages: cause.pages,
            first: site.named(cause.first, &shared.sources),
        };
        problems.push(Problem::placed(kind, cause.place));
    }

    // A picture of a kind not every preview is taken to show is told
    // once, where the first page naming it does.
    let mut doubted: Vec<(&Own, usize)> = Vec::new();
    let mut among: HashMap<&str, usize> = HashMap::new();
    for (_, answer) in answers() {
        let Outcome::Own(_, own) = &answer.outcome else {
            continue;
        };
        if own.kind.is_some_and(|kind| PREVIEWED.contains(&kind)) {
            continue;
        }
        let at = *among.entry(own.path.as_str()).or_insert(doubted.len());
        match doubted.get_mut(at) {
            Some((_, pages)) => *pages += 1,
            None => doubted.push((own, 1)),
        }
    }
    for (own, pages) in doubted {
        let kind = Kind::ImageMayNotShow {
            path: own.path.clone(),
            holds: own.kind,
            pages,
        };
        problems.push(Problem::placed(kind, own.named));
    }

    let mut picked: BTreeSet<usize> = BTreeSet::new();
    for (_, answer) in answers() {
        let Outcome::Drawn(_, drawing) = &answer.outcome else {
            continue;
        };
        picked.insert(drawing.described);
    }

    let head = |described: &cards::Described| {
        described
            .source
            .map(|source| words::head(&shared.sources, source))
    };
    // The site's own card first, wherever its file falls among the rest.
    let mut whole: Vec<&cards::Described> = cards
        .described
        .iter()
        .filter(|described| described.card.is_some())
        .collect();
    whole.sort_by_key(|described| described.name.is_some());
    let mut whole = whole.into_iter();
    if !has_url {
        if answers().any(|(_, answer)| answer.waiting) {
            let kind = Kind::NoSiteAddress {
                waiting: Waiting::Cards,
            };
            problems.push(Problem::placed(kind, whole.next().and_then(head)));
        }
    } else if asked.answers.is_empty() {
        if let Some(described) = whole.next() {
            let kind = Kind::UnusedCard { name: None };
            problems.push(Problem::placed(kind, head(described)));
        }
    } else {
        let picks = |name: &str| {
            let mut pages = site.pages.iter();
            pages.any(|(_, page)| {
                let picked = page.words.text(PageWord::Card);
                picked.is_some_and(|picked| picked.value == name)
            }) || cards.picked_elsewhere.iter().any(|picked| picked == name)
        };
        for described in whole {
            if let Some(name) = described.name.as_deref().filter(|name| !picks(name)) {
                let kind = Kind::UnusedCard {
                    name: Some(name.to_owned()),
                };
                problems.push(Problem::placed(kind, head(described)));
            }
        }
    }
    problems
}

/// A font in `templates/og/` of the site at `root` that draws every one
/// of `characters`, by its file's name, the first by name where there are
/// several. No font the card names draws them, so it is one the card
/// doesn't name.
fn font_here(root: &std::path::Path, characters: &[char]) -> Option<String> {
    let folder = crate::platform::join(root, cards::FOLDER);
    let mut names: Vec<String> = std::fs::read_dir(folder)
        .ok()?
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| cards::is_a_font(name))
        .collect();
    names.sort();
    names.into_iter().find(|name| {
        let path = format!("{}/{name}", cards::FOLDER);
        matches!(
            crate::platform::read_file(root, &path),
            Ok(Some(bytes)) if cards::draw::draws_all(&bytes, characters)
        )
    })
}

/// The cards to draw: those the outputs built `asked` for.
pub(super) fn wanted<'a>(shared: &'a Shared, asked: &'a Gathered) -> Vec<Wanted<'a>> {
    let room = &shared.cards;
    let answers = asked.answers.values();
    answers
        .filter_map(|answer| match &answer.outcome {
            Outcome::Drawn(shown, drawing) => {
                let kit = room.kits[drawing.described].get()?.as_ref()?;
                Some(Wanted {
                    kit,
                    texts: texts(&drawing.texts),
                    key: drawing.key,
                    plan: drawing.plan.as_ref(),
                    file: shown.file.clone(),
                    from: &shared.templates.cards.described[drawing.described].path,
                })
            }
            _ => None,
        })
        .collect()
}

/// The names the cards the outputs built `asked` for are kept under.
pub(super) fn kept(shared: &Shared, asked: &Gathered) -> HashSet<String> {
    wanted(shared, asked)
        .iter()
        .map(|wanted| kept_as(&wanted.key, wanted.kit.format()))
        .collect()
}

/// The pictures pages name as their own that an output `asked` for, and
/// how many pages each stands for.
pub(super) fn own(asked: &Gathered) -> BTreeMap<PictureId, usize> {
    let mut own = BTreeMap::new();
    for answer in asked.answers.values() {
        if let Outcome::Own(
            _,
            Own {
                picture: Some(picture),
                ..
            },
        ) = &answer.outcome
        {
            *own.entry(*picture).or_default() += 1;
        }
    }
    own
}
