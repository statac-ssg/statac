//! What a page answers when called: its pictures, the files beside it, and
//! videos and recordings. `page.photo("photo-1.jpg")` in any template or
//! snippet finds the picture as the page's writing would, and gives what a
//! tag showing it needs.

use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use minijinja::machinery::Span;
use minijinja::machinery::ast::{CallType, Expr};
use minijinja::value::{Enumerator, Object, ObjectRepr, Value};
use minijinja::{Error, State};

use super::check::{full_span, parse_template, place, visit_exprs};
use super::frame;
use super::render::{Shared, met_building};
use super::values::node_value;
use crate::album::{Picture, Root, Seen};
use crate::content::address::{is_full, on_the_wire};
use crate::fetched::stills;
use crate::highlight::{self, Language};
use crate::origin::Sourced;
use crate::pictures::{Format, Kind as PictureKind};
use crate::problem::Host;
use crate::problem::{ArgumentFault, Hint, Kind, Needed, Problem, Reason, ShownBy, Sort};
use crate::reads::Key;
use crate::site::{PageId, PictureId};
use crate::source::Place;

/// The names a page answers only when called.
pub(super) const CALLS: &[&str] = &[
    "image", "images", "file", "video", "audio", "youtube", "vimeo",
];

/// What `page.image` and `page.images` take by name.
const PHOTO_OPTIONS: &[&str] = &["sizes", "full", "decorative"];

/// Who is asking, for a problem to be told where it was asked.
#[derive(Clone, Debug)]
pub(super) struct Asking {
    /// The call as a problem names it: `page.photo`.
    call: String,
    method: String,
    /// The call in the writing whose snippet or piece is asking, where one
    /// is.
    calling: Option<Place>,
    /// The template asking, by the engine's name for it.
    template: String,
    /// The call in the template, as the engine places it, where it does.
    written: Option<Span>,
}

impl Asking {
    pub(super) fn new(method: &str, template: &str) -> Asking {
        Asking {
            call: format!("page.{method}"),
            method: method.to_owned(),
            calling: frame::with(|frame| frame.calling).flatten(),
            template: template.to_owned(),
            written: None,
        }
    }

    /// Asked by the call the template `state` is running.
    fn running(method: &str, state: &State<'_, '_>) -> Asking {
        Asking {
            written: state.calling_span(),
            ..Asking::new(method, state.name())
        }
    }

    fn wrong(&self, needed: Needed) -> Error {
        frame::raise(Kind::WrongArgument {
            snippet: self.call.clone(),
            fault: ArgumentFault::Needs(needed),
        })
    }

    /// Where the asking is written: the call in the writing, or else the
    /// call in the template, as the engine places it or, where it doesn't,
    /// where the template makes only one such.
    fn place(&self, shared: &Shared) -> Option<Place> {
        if self.calling.is_some() {
            return self.calling;
        }
        let &at = shared.templates.by_name.get(&self.template)?;
        if let Some(found) = self.written_in(shared, at) {
            return Some(found);
        }
        let found = calls(
            shared,
            at,
            &|call| matches!(call, CallType::Method(_, name) if *name == self.method),
        );
        match found.as_slice() {
            [only] => Some(*only),
            _ => None,
        }
    }

    /// Where the asking is written, or else the first such call in the
    /// template, where it makes several.
    fn place_or_first(&self, shared: &Shared) -> Option<Place> {
        if self.calling.is_some() {
            return self.calling;
        }
        let &at = shared.templates.by_name.get(&self.template)?;
        if let Some(found) = self.written_in(shared, at) {
            return Some(found);
        }
        let wanted =
            |call: &CallType| matches!(call, CallType::Method(_, name) if *name == self.method);
        calls(shared, at, &wanted).first().copied()
    }

    /// The call in the template file `at` the engine placed the asking at.
    fn written_in(&self, shared: &Shared, at: usize) -> Option<Place> {
        call_at(
            shared,
            at,
            self.written?,
            &|call| matches!(call, CallType::Method(_, name) if *name == self.method),
        )
    }

    /// Tells `kind`, which doesn't stop the page, where the asking is
    /// written.
    /// Told at a template as met building the page, so that the pages
    /// building it are told of once, together.
    fn tell(&self, shared: &Shared, kind: Kind) {
        let at = self.place(shared);
        frame::with(|frame| {
            let problem = match at {
                Some(at) => Problem::at(kind, at),
                None => Problem::nowhere(kind),
            };
            let problem = met_building(problem, frame);
            if !frame.problems.contains(&problem) {
                frame.problems.push(problem);
            }
        });
    }
}

/// Every call in the template file `at` that `wanted` picks.
fn calls(shared: &Shared, at: usize, wanted: &dyn Fn(&CallType) -> bool) -> Vec<Place> {
    let file = &shared.templates.files[at];
    let source = file.source;
    let Ok(ast) = parse_template(&file.name, shared.sources.text(source)) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    visit_exprs(&ast, &mut |expr| {
        if let Expr::Call(call) = expr
            && wanted(&call.identify_call())
        {
            found.push(place(source, full_span(expr, shared.sources.text(source))));
        }
    });
    found
}

/// The call `wanted` picks in the template file `at` that the parser
/// places at `span`, as the engine places a call it is running.
pub(super) fn call_at(
    shared: &Shared,
    at: usize,
    span: Span,
    wanted: &dyn Fn(&CallType) -> bool,
) -> Option<Place> {
    let file = &shared.templates.files[at];
    let text = shared.sources.text(file.source);
    let ast = parse_template(&file.name, text).ok()?;
    let mut found = None;
    visit_exprs(&ast, &mut |expr| {
        if found.is_none()
            && let Expr::Call(call) = expr
            && call.span() == span
            && wanted(&call.identify_call())
        {
            found = Some(place(file.source, full_span(expr, text)));
        }
    });
    found
}

/// The one call `wanted` picks in the template `template` and those it
/// extends, where there is one.
pub(super) fn only_call(
    shared: &Shared,
    template: &str,
    wanted: &dyn Fn(&CallType) -> bool,
) -> Option<Place> {
    let chain = shared.templates.chain(template);
    let found: Vec<Place> = (chain.into_iter())
        .flat_map(|at| calls(shared, at, wanted))
        .collect();
    match found.as_slice() {
        [only] => Some(*only),
        _ => None,
    }
}

/// Tells `kind`, which doesn't stop the page, at `at`, or where none,
/// as met building the page.
pub(super) fn told_at(at: Option<Place>, kind: Kind) {
    frame::with(|frame| {
        let problem = match at {
            Some(at) => Problem::at(kind, at),
            None => met_building(Problem::nowhere(kind), frame),
        };
        if !frame.problems.contains(&problem) {
            frame.problems.push(problem);
        }
    });
}

/// Values a call hands by name.
type ByName = Vec<(String, Value)>;

/// The values of a call without names, and those with.
fn parted(args: &[Value]) -> Result<(Vec<Value>, ByName), Error> {
    let mut unnamed = Vec::new();
    let mut named = Vec::new();
    for arg in args {
        if !arg.is_kwargs() {
            unnamed.push(arg.clone());
            continue;
        }
        for key in arg.try_iter()? {
            let value = arg.get_item(&key)?;
            named.push((key.as_str().unwrap_or_default().to_owned(), value));
        }
    }
    Ok((unnamed, named))
}

/// A value used that was never given, told as any other is.
fn undefined() -> Error {
    Error::from(minijinja::ErrorKind::UndefinedError)
}

/// The one name a call is handed, as text.
fn one_name<'a>(asking: &Asking, unnamed: &'a [Value], sort: Sort) -> Result<&'a str, Error> {
    match unnamed {
        [only] if only.is_undefined() => Err(undefined()),
        [only] => match only.as_str().filter(|name| !name.trim().is_empty()) {
            Some(name) => Ok(name),
            None => Err(asking.wrong(Needed::Name(sort))),
        },
        _ => Err(asking.wrong(Needed::Name(sort))),
    }
}

/// Refuses any named value of a call that takes none.
fn takes_no_names(asking: &Asking, named: &[(String, Value)]) -> Result<(), Error> {
    match named.first() {
        Some((given, _)) => Err(frame::raise(Kind::WrongArgument {
            snippet: asking.call.clone(),
            fault: ArgumentFault::NoSuchName {
                given: given.clone(),
                nearest: None,
            },
        })),
        None => Ok(()),
    }
}

/// One of a page's calls named and not called: `page.photo` alone.
pub(super) fn uncalled(method: &str) -> Error {
    let video = |site| Needed::VideoOn {
        site,
        written: None,
    };
    Asking::new(method, "").wrong(match method {
        "images" => Needed::Names,
        "file" => Needed::Name(Sort::File),
        "video" => Needed::Name(Sort::Video),
        "audio" => Needed::Name(Sort::Recording),
        "youtube" => video(Host::YouTube.site()),
        "vimeo" => video(Host::Vimeo.site()),
        _ => Needed::Name(Sort::Picture),
    })
}

/// `page.<method>(...)` on the page `of`, asked in the template `state`
/// is building.
pub(super) fn call(
    shared: &Arc<Shared>,
    of: PageId,
    state: &State<'_, '_>,
    method: &str,
    args: &[Value],
) -> Result<Value, Error> {
    let asking = Asking::running(method, state);
    let (unnamed, named) = parted(args)?;
    match method {
        "image" => {
            let options = Options::read(&asking, &named)?;
            let name = one_name(&asking, &unnamed, Sort::Picture)?;
            photo(shared, of, name, &options, &asking)
        }
        "images" => {
            let options = Options::read(&asking, &named)?;
            photos(shared, of, &unnamed, &options, &asking)
        }
        "file" | "video" | "audio" => {
            takes_no_names(&asking, &named)?;
            let sort = match method {
                "video" => Sort::Video,
                "audio" => Sort::Recording,
                _ => Sort::File,
            };
            let name = one_name(&asking, &unnamed, sort)?;
            file(shared, of, name, sort, &asking)
        }
        _ => {
            takes_no_names(&asking, &named)?;
            let host = match method {
                "vimeo" => Host::Vimeo,
                _ => Host::YouTube,
            };
            let missing = || {
                asking.wrong(Needed::VideoOn {
                    site: host.site(),
                    written: None,
                })
            };
            let written = match unnamed.as_slice() {
                [only] if only.is_undefined() => return Err(undefined()),
                [only] => only.as_str().ok_or_else(missing)?,
                _ => return Err(missing()),
            };
            let Some(video) = host.read(written) else {
                return Err(asking.wrong(Needed::VideoOn {
                    site: host.site(),
                    written: Some(written.to_owned()),
                }));
            };
            Ok(Value::from_object(VideoValue {
                shared: Arc::clone(shared),
                host,
                video,
                written: written.to_owned(),
                asking,
                still: OnceLock::new(),
            }))
        }
    }
}

/// How a call asks for a photograph to be shown.
#[derive(Default)]
struct Options {
    sizes: Option<String>,
    full: bool,
    decorative: bool,
}

impl Options {
    fn read(asking: &Asking, named: &[(String, Value)]) -> Result<Options, Error> {
        let mut options = Options::default();
        for (name, value) in named {
            if value.is_undefined() {
                return Err(undefined());
            }
            let truth = || match value.kind() {
                minijinja::value::ValueKind::Bool => Ok(value.is_true()),
                minijinja::value::ValueKind::None => Ok(false),
                _ => Err(asking.wrong(Needed::TrueFalse(name.clone()))),
            };
            match name.as_str() {
                "sizes" if value.is_none() => {}
                "sizes" => match value.as_str() {
                    Some(sizes) => options.sizes = Some(sizes.to_owned()),
                    None => return Err(asking.wrong(Needed::Text(name.clone()))),
                },
                "full" => options.full = truth()?,
                "decorative" => options.decorative = truth()?,
                _ => {
                    return Err(frame::raise(Kind::WrongArgument {
                        snippet: asking.call.clone(),
                        fault: ArgumentFault::NoSuchName {
                            given: name.clone(),
                            nearest: crate::suggest::nearest(name, PHOTO_OPTIONS.iter().copied())
                                .map(str::to_owned),
                        },
                    }));
                }
            }
        }
        Ok(options)
    }
}

/// The folder under `content/` of the file `page` is written in, ending
/// with `/`, or empty at the top; none for a page no file there holds.
fn folder_of(shared: &Shared, page: PageId) -> Option<&str> {
    let page = shared.site.pages.get(page);
    page.made
        .is_none()
        .then(|| page.path.rfind('/').map_or("", |at| &page.path[..=at]))
}

/// The picture `written` names from the page `of`, as its writing would
/// name it.
pub(super) fn found(shared: &Shared, of: PageId, written: &str) -> Option<PictureId> {
    let published = shared.address_of(of);
    let from = crate::album::From {
        folder: folder_of(shared, of),
        page: &published,
        base: &shared.base,
    };
    let found = shared.site.pictures.find(written, &from);
    if found.is_none() {
        frame::read(Key::Pictures);
    }
    found.map(|found| found.id)
}

/// The file a relative name, or one from the top, reaches no file as: how
/// a problem names it, and the nearest of `beside`, the names there.
fn missing<'a>(
    shared: &Shared,
    of: PageId,
    written: &str,
    beside: impl Iterator<Item = &'a str>,
) -> Kind {
    let name = crate::content::address::decoded(written);
    let folder = folder_of(shared, of).filter(|_| !name.starts_with('/'));
    let near = folder.and_then(|_| crate::suggest::nearest(&name, beside));
    let hint = near.map(|near| Hint::Closest(near.to_owned()));
    Kind::MissingFile {
        path: match folder {
            Some(folder) => format!("content/{folder}{name}"),
            None => name,
        },
        hint,
    }
}

/// The problem with a picture `written` names from the page `of` that
/// reaches no file.
pub(super) fn missing_picture(shared: &Shared, of: PageId, written: &str) -> Kind {
    let pictures = shared.site.pictures.iter();
    let paths = pictures
        .filter(|(_, picture)| picture.root == Root::Content)
        .map(|(_, picture)| picture.path.as_str());
    missing(shared, of, written, names_in(folder_of(shared, of), paths))
}

/// The names of the files under `content/` directly in `folder`, among
/// `paths`.
fn names_in<'a>(
    folder: Option<&'a str>,
    paths: impl Iterator<Item = &'a str>,
) -> impl Iterator<Item = &'a str> {
    paths.filter_map(move |path| {
        let name = path.strip_prefix(folder?)?;
        (!name.contains('/')).then_some(name)
    })
}

/// `page.photo(name)`: the picture, as a tag showing it reads it.
fn photo(
    shared: &Arc<Shared>,
    of: PageId,
    written: &str,
    options: &Options,
    asking: &Asking,
) -> Result<Value, Error> {
    if is_full(written) {
        return Err(asking.wrong(Needed::OnSite(written.to_owned())));
    }
    let Some(id) = found(shared, of, written) else {
        let pictures = shared.site.pictures.iter();
        let paths = pictures
            .filter(|(_, picture)| picture.root == Root::Content)
            .map(|(_, picture)| picture.path.as_str());
        return Err(frame::raise(missing(
            shared,
            of,
            written,
            names_in(folder_of(shared, of), paths),
        )));
    };
    shown(shared, id, options, asking)
}

/// `page.photos([names])`, or with no names every photograph beside the
/// page, by file name.
fn photos(
    shared: &Arc<Shared>,
    of: PageId,
    unnamed: &[Value],
    options: &Options,
    asking: &Asking,
) -> Result<Value, Error> {
    let names = match unnamed {
        [] => None,
        [names] if names.is_undefined() => return Err(undefined()),
        [names] if names.is_none() => None,
        [names] if names.kind() == minijinja::value::ValueKind::Seq => Some(names),
        _ => return Err(asking.wrong(Needed::Names)),
    };
    let mut values = Vec::new();
    let Some(names) = names else {
        frame::read(Key::Pictures);
        for &id in shared.photographs_beside(of) {
            values.push(shown(shared, id, options, asking)?);
        }
        return Ok(Value::from(values));
    };
    for name in names.try_iter()? {
        if name.is_undefined() {
            return Err(undefined());
        }
        let name = name
            .as_str()
            .filter(|name| !name.trim().is_empty())
            .ok_or_else(|| asking.wrong(Needed::Names))?;
        values.push(photo(shared, of, name, options, asking)?);
    }
    Ok(Value::from(values))
}

/// The picture a piece writing a Markdown picture is handed: none where
/// the address reaches no file of the site, or the file can't be shown.
pub(super) fn for_piece(shared: &Arc<Shared>, of: PageId, address: &str, template: &str) -> Value {
    let asking = Asking::new("image", template);
    match found(shared, of, address) {
        // Told where the writing shows the picture.
        Some(id) => {
            shown(shared, id, &Options::default(), &asking).unwrap_or_else(|_| Value::from(()))
        }
        None => Value::from(()),
    }
}

/// A file's address in a tag: the site's own path, then the file's from
/// the top, spelled as an address travels. A comma parts the addresses of
/// a `srcset`, so one in a name is written as its byte.
pub(super) fn address_in(shared: &Shared, address: &str) -> String {
    let spelled = on_the_wire(address).replace(',', "%2C");
    format!("{}{spelled}", shared.base)
}

/// A file's name read as words: `first-photo-1.jpg` is "First Photo
/// 1". A name with no words in it stands as written.
fn as_words(name: &str) -> String {
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    let words: Vec<String> = stem
        .split(['-', '_', ' '])
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut letters = word.chars();
            match letters.next() {
                Some(first) => first.to_uppercase().collect::<String>() + letters.as_str(),
                None => String::new(),
            }
        })
        .collect();
    match words.is_empty() {
        true => stem.to_owned(),
        false => words.join(" "),
    }
}

fn sourced(text: Option<&Sourced<String>>) -> Value {
    match text {
        Some(text) => match text.origin.place {
            Some(place) => frame::sourced(&text.value, place),
            None => Value::from(text.value.as_str()),
        },
        None => Value::from(""),
    }
}

/// The picture `id` as a template sees it.
fn shown(
    shared: &Arc<Shared>,
    id: PictureId,
    options: &Options,
    asking: &Asking,
) -> Result<Value, Error> {
    let picture = shared.site.pictures.get(id);
    frame::read(Key::Picture(id));
    for name in ["url", "optimise_images", "max_width", "image_sizes"] {
        frame::read(Key::Setting(shared.name_id(name)));
    }
    // What is fetched is never published as it is.
    let left_alone = picture.root != Root::Fetched
        && (picture.left_alone() || shared.settings.truth("optimise_images") == Some(false));
    match &picture.seen {
        Seen::Unreadable(fault) if !left_alone => {
            return Err(frame::raise(Kind::UnreadableImage {
                path: picture.shown(),
                fault: fault.clone(),
            }));
        }
        Seen::Plain {
            kind: PictureKind::Heic | PictureKind::Heif,
            ..
        } => asking.tell(
            shared,
            Kind::HeicImage {
                path: picture.shown(),
            },
        ),
        _ => {}
    }
    let Some(address) = picture.address.as_deref() else {
        return Err(frame::raise(Kind::MissingFile {
            path: picture.shown(),
            hint: None,
        }));
    };
    let own = address_in(shared, address);
    let folder = &address[..=address.rfind('/').unwrap_or(0)];
    let beside = |name: &str| address_in(shared, &format!("{folder}{name}"));
    let fallback = picture.copies.last().map(|copy| copy.format);
    let srcset = |format: Format| {
        let copies = picture.copies.iter().filter(|copy| copy.format == format);
        let named: Vec<String> = copies
            .map(|copy| format!("{} {}w", beside(&copy.name), copy.width))
            .collect();
        named.join(", ")
    };
    let widths: Vec<u32> = picture.copies.iter().map(|copy| copy.width).collect();
    let src = crate::pictures::src_width(&widths).and_then(|width| {
        let mut copies = picture.copies.iter();
        copies.find(|copy| Some(copy.format) == fallback && copy.width == width)
    });
    let widest = match (&picture.full, options.full) {
        (Some(full), true) => Some(full),
        _ => picture.copies.last(),
    };
    let size = |side: Option<u32>| side.map_or_else(|| Value::from(""), Value::from);
    let size_of = picture.size();
    let sidecar = picture.sidecar.as_ref();
    let alt = sidecar.and_then(|sidecar| sidecar.alt.as_ref());
    let name = picture.path.rsplit('/').next().unwrap_or(&picture.path);
    let sizes = match &options.sizes {
        Some(sizes) => sizes.clone(),
        None => shared
            .settings
            .text("image_sizes")
            .map(|sizes| sizes.value.to_owned())
            .unwrap_or_default(),
    };
    let fields = vec![
        (
            "src",
            Value::from(src.map_or_else(|| own.clone(), |copy| beside(&copy.name))),
        ),
        ("avif", Value::from(srcset(Format::Avif))),
        (
            "fallback",
            Value::from(fallback.map(srcset).unwrap_or_default()),
        ),
        ("sizes", Value::from(sizes)),
        (
            "full",
            Value::from(widest.map_or_else(|| own.clone(), |copy| beside(&copy.name))),
        ),
        ("width", size(size_of.map(|size| size.width))),
        ("height", size(size_of.map(|size| size.height))),
        ("alt", sourced(alt)),
        (
            "caption",
            sourced(sidecar.and_then(|sidecar| sidecar.caption.as_ref())),
        ),
        ("name", Value::from(as_words(name))),
    ];
    let own_fields = sidecar.map_or_else(Vec::new, |sidecar| {
        sidecar
            .own
            .iter()
            .filter(|field| !fields.iter().any(|(name, _)| *name == field.name))
            .map(|field| (field.name.clone(), node_value(shared, &field.value, None)))
            .collect()
    });
    let described = alt.is_some_and(|alt| !alt.value.trim().is_empty());
    let decorative = options.decorative || picture.decorative();
    Ok(Value::from_object(PhotoValue {
        shared: Arc::clone(shared),
        id,
        fields,
        own: own_fields,
        copied: !picture.copies.is_empty(),
        full: options.full,
        undescribed: !(described || decorative),
        asking: asking.clone(),
    }))
}

/// The face `name` in `fetched/faces/`, as `page.photo` gives a picture:
/// decoration, never asked to be described. None where it isn't there.
pub(super) fn face(shared: &Arc<Shared>, name: &str) -> Value {
    let path = format!("{}/{name}", crate::fetched::mentions::FACES);
    let Some(id) = shared.site.pictures.fetched(&path) else {
        frame::read(Key::Fetched(crate::fetched::key(&path)));
        return Value::from(());
    };
    let options = Options {
        decorative: true,
        ..Options::default()
    };
    shown(shared, id, &options, &Asking::new("webmentions", "")).unwrap_or_else(Value::from)
}

/// A picture as a template sees it. Its copies are made once a template
/// reads where one is published, and it is told as undescribed where a
/// template reads its description and it has none.
#[derive(Debug)]
struct PhotoValue {
    shared: Arc<Shared>,
    id: PictureId,
    /// Statac's own names.
    fields: Vec<(&'static str, Value)>,
    /// What else the file describing it says.
    own: Vec<(String, Value)>,
    /// Whether it is published as copies.
    copied: bool,
    full: bool,
    undescribed: bool,
    asking: Asking,
}

/// What the file describing a picture says beyond Statac's own names, where
/// `value` is a picture. Statac's own are never missing, and looking through
/// them would read the description as a template showing it does.
pub(super) fn own_fields(value: &Value) -> Option<&[(String, Value)]> {
    let photo = value.downcast_object_ref::<PhotoValue>()?;
    Some(&photo.own)
}

/// Tells a picture shown with no description, once for the output being
/// built. `key` is the picture, however it is named where it is shown.
pub(super) fn undescribed(key: String, problem: impl FnOnce(&frame::Frame) -> Problem) {
    frame::with(|frame| {
        if frame.undescribed.contains(&key) {
            return;
        }
        let problem = problem(frame);
        frame.undescribed.push(key);
        frame.problems.push(problem);
    });
}

/// How a problem names the file that may describe `picture`.
fn described_beside(picture: &Picture) -> Option<String> {
    (picture.root == Root::Content).then(|| format!("{}.yaml", picture.shown()))
}

impl Object for PhotoValue {
    fn repr(self: &Arc<Self>) -> ObjectRepr {
        ObjectRepr::Map
    }

    fn is_true(self: &Arc<Self>) -> bool {
        true
    }

    fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
        let key = key.as_str()?;
        let shared = &self.shared;
        let found = self.fields.iter().find(|(name, _)| *name == key);
        let Some((name, value)) = found else {
            let own = self.own.iter().find(|(name, _)| name == key);
            if own.is_none() {
                frame::missed(None, key);
            }
            return own.map(|(_, value)| value.clone());
        };
        match *name {
            "src" | "avif" | "fallback" | "full" if self.copied => {
                let at = self.asking.calling;
                frame::develops(self.id, at, self.full);
            }
            "alt" if self.undescribed => {
                let picture = shared.site.pictures.get(self.id);
                let kind = Kind::MissingAltText {
                    picture: picture.shown(),
                    shown: ShownBy::Template {
                        beside: described_beside(picture),
                    },
                };
                let at = self.asking.place(shared);
                undescribed(picture.shown(), |frame| {
                    let problem = match at {
                        Some(at) => Problem::at(kind, at),
                        None => Problem::nowhere(kind),
                    };
                    met_building(problem, frame)
                });
            }
            _ => {}
        }
        Some(value.clone())
    }

    fn enumerate(self: &Arc<Self>) -> Enumerator {
        let names = self.fields.iter().map(|(name, _)| Value::from(*name));
        let own = self.own.iter().map(|(name, _)| Value::from(name.as_str()));
        Enumerator::Values(names.chain(own).collect())
    }

    /// A picture printed is where it is published.
    fn render(self: &Arc<Self>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let src = self.get_value(&Value::from("src")).unwrap_or_default();
        write!(f, "{src}")
    }
}

/// A file of the site a page names: one beside the writing, or one in
/// `static/`.
pub(super) struct Named {
    key: Key,
    /// From the top of the site.
    pub(super) address: String,
    on_disk: OnDisk,
    /// As a problem names it.
    pub(super) shown: String,
}

/// Where a file really is: its path past every link under the site's real
/// path.
#[derive(Debug)]
struct OnDisk {
    real: PathBuf,
    at: String,
}

impl OnDisk {
    /// The file opened to read, through no link.
    fn open(&self) -> std::io::Result<std::fs::File> {
        crate::platform::open_file(&self.real, &self.at)
    }
}

/// The file `written` names from the page `of`: a relative name from the
/// folder of the page's file, a name from the top in `content/` and then
/// in `static/`.
pub(super) fn named(shared: &Shared, of: PageId, written: &str) -> Option<Named> {
    let site = &shared.site;
    let name = crate::content::address::decoded(written);
    let name = &name[..name.find(['?', '#']).unwrap_or(name.len())];
    let beside = |path: &str| {
        let at = shared.file_at(path)?;
        Some(Named {
            key: Key::Beside(u32::try_from(at).ok()?),
            address: site.files[at].address.as_str().to_owned(),
            on_disk: OnDisk {
                real: site.real.clone(),
                at: site.found.get(path)?.clone(),
            },
            shown: format!("content/{path}"),
        })
    };
    let found = match name.strip_prefix('/') {
        Some(_) => {
            let inside = match shared.base.is_empty() {
                true => None,
                false => name
                    .strip_prefix(shared.base.as_str())
                    .filter(|rest| rest.starts_with('/')),
            };
            let tops = [inside, Some(name)].into_iter().flatten();
            let mut paths = tops.filter_map(|top| crate::album::walked("", top));
            paths.find_map(|path| {
                beside(&path).or_else(|| {
                    let statics = shared.assets.statics();
                    let at = shared.static_at(&path)?;
                    Some(Named {
                        key: Key::Static(u32::try_from(at).ok()?),
                        address: format!("/{path}"),
                        on_disk: OnDisk {
                            real: shared.assets.real().to_path_buf(),
                            at: statics[at].at().to_owned(),
                        },
                        shown: format!("{}/{path}", crate::assets::STATIC),
                    })
                })
            })
        }
        None => folder_of(shared, of)
            .and_then(|folder| crate::album::walked(folder, name))
            .and_then(|path| beside(&path)),
    };
    match &found {
        Some(found) => frame::read(found.key),
        None => frame::read(Key::Files),
    }
    found
}

/// Whether a file named `name` may be found as a file of `sort`, by the
/// ending of its name.
pub(crate) fn fits(sort: Sort, name: &str) -> bool {
    match sort {
        Sort::File => true,
        Sort::Picture => crate::album::is_picture(name),
        Sort::Video | Sort::Recording => media_type(sort, name).is_some(),
    }
}

/// What a browser calls a video or a recording, by the ending of its
/// name.
fn media_type(sort: Sort, name: &str) -> Option<&'static str> {
    let ending = name.rsplit_once('.')?.1.to_ascii_lowercase();
    match (sort, ending.as_str()) {
        (Sort::Video, "mp4" | "m4v") => Some("video/mp4"),
        (Sort::Video, "webm") => Some("video/webm"),
        (Sort::Video, "ogv") => Some("video/ogg"),
        (Sort::Video, "mov") => Some("video/quicktime"),
        (Sort::Recording, "mp3") => Some("audio/mpeg"),
        (Sort::Recording, "m4a") => Some("audio/mp4"),
        (Sort::Recording, "ogg" | "oga") => Some("audio/ogg"),
        (Sort::Recording, "wav") => Some("audio/wav"),
        (Sort::Recording, "flac") => Some("audio/flac"),
        _ => None,
    }
}

/// `page.file(name)`, `page.video(name)` or `page.audio(name)`.
fn file(
    shared: &Arc<Shared>,
    of: PageId,
    written: &str,
    sort: Sort,
    asking: &Asking,
) -> Result<Value, Error> {
    if is_full(written) {
        return Err(asking.wrong(Needed::OnSite(written.to_owned())));
    }
    let Some(found) = named(shared, of, written) else {
        let paths = shared.site.files.iter().map(|file| file.path.as_str());
        return Err(frame::raise(missing(
            shared,
            of,
            written,
            names_in(folder_of(shared, of), paths),
        )));
    };
    let name = found
        .shown
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_owned();
    frame::read(Key::Setting(shared.name_id("url")));
    let address = address_in(shared, &found.address);
    if sort == Sort::File {
        return Ok(Value::from_object(FileValue {
            url: address,
            name,
            on_disk: found.on_disk,
            shown: found.shown,
            asking: asking.clone(),
        }));
    }
    let Some(media_type) = media_type(sort, &name) else {
        return Err(asking.wrong(Needed::Playable {
            sort,
            written: written.to_owned(),
        }));
    };
    Ok(Value::from_object(super::values::Ordered(vec![
        ("src".into(), Value::from(address)),
        ("type".into(), Value::from(media_type)),
    ])))
}

/// A file as a template sees it. What it holds is read when asked for.
#[derive(Debug)]
struct FileValue {
    url: String,
    name: String,
    on_disk: OnDisk,
    shown: String,
    asking: Asking,
}

/// The largest file, in bytes, a page shows as code.
const LARGEST_SHOWN_AS_CODE: u64 = 200_000;

impl FileValue {
    fn cant_read(&self, error: &std::io::Error) -> Value {
        Value::from(frame::raise(Kind::CantRead {
            path: self.shown.clone().into(),
            reason: Reason::of(error),
        }))
    }

    /// The language the file's name gives, by the name a block of code
    /// would carry in its class; none where Statac has no colours for it.
    fn language(&self) -> &'static str {
        match highlight::file_language(&self.name) {
            Language::Known(name) => name,
            _ => "",
        }
    }

    /// What the file holds as the inside of a `<code>`, coloured as its
    /// name says. Nothing for a file too large to show or one that isn't
    /// text.
    fn code(&self) -> Value {
        let mut file = match self.on_disk.open() {
            Ok(file) => file,
            Err(error) => return self.cant_read(&error),
        };
        match file.metadata() {
            Ok(found) if found.len() > LARGEST_SHOWN_AS_CODE => return Value::from(""),
            Ok(_) => {}
            Err(error) => return self.cant_read(&error),
        }
        let mut bytes = Vec::new();
        let text = match std::io::Read::read_to_end(&mut file, &mut bytes) {
            Ok(_) => String::from_utf8(bytes),
            Err(error) => return self.cant_read(&error),
        };
        let Ok(text) = text else {
            return Value::from("");
        };
        // As a fenced block's last line ending is no part of its code.
        let code = text.strip_suffix('\n').unwrap_or(&text);
        let code = code.strip_suffix('\r').unwrap_or(code);
        let Some(shared) = frame::shared() else {
            return Value::from_safe_string(highlight::plain(code));
        };
        let language = highlight::file_language(&self.name);
        let at = || self.asking.place_or_first(&shared);
        Value::from_safe_string(super::render::code_html(&shared, code, language, at))
    }
}

impl Object for FileValue {
    fn repr(self: &Arc<Self>) -> ObjectRepr {
        ObjectRepr::Map
    }

    fn is_true(self: &Arc<Self>) -> bool {
        true
    }

    fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
        match key.as_str()? {
            "url" => Some(Value::from(self.url.as_str())),
            "name" => Some(Value::from(self.name.as_str())),
            "size" => Some(match self.on_disk.open().and_then(|file| file.metadata()) {
                Ok(found) => Value::from(found.len()),
                Err(error) => self.cant_read(&error),
            }),
            "text" => Some(
                match self.on_disk.open().and_then(std::io::read_to_string) {
                    Ok(text) => Value::from(text),
                    Err(error) => self.cant_read(&error),
                },
            ),
            "code" => Some(self.code()),
            "language" => Some(Value::from(self.language())),
            other => {
                frame::missed(None, other);
                None
            }
        }
    }

    fn enumerate(self: &Arc<Self>) -> Enumerator {
        Enumerator::Str(&["url", "name", "size", "text", "code", "language"])
    }

    /// A file printed is where it is published.
    fn render(self: &Arc<Self>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.url)
    }
}

/// A video read from an address or an id.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Video {
    id: String,
    /// Where the address says to start, in seconds.
    start: Option<u64>,
    /// What else the address asks of a player, as written, handed on to
    /// the player.
    rest: Vec<String>,
}

/// What a YouTube address may ask of a player besides where to start.
/// What only YouTube's own pages read is left behind.
const PLAYER_READS: &[&str] = &["end", "list", "index", "loop", "playlist"];

impl Host {
    fn read(self, written: &str) -> Option<Video> {
        let written = written.trim();
        let rest = written
            .strip_prefix("https://")
            .or_else(|| written.strip_prefix("http://"))
            .unwrap_or(written);
        let first = |path: &str| {
            let end = path.find(['?', '&', '/', '#']).unwrap_or(path.len());
            path[..end].to_owned()
        };
        let (before, fragment) = written.split_once('#').unwrap_or((written, ""));
        let query = before.split_once('?').map_or("", |(_, query)| query);
        match self {
            Host::YouTube => {
                let plausible = |id: &String| {
                    id.len() == 11
                        && id
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                };
                let pairs = || {
                    query
                        .split('&')
                        .filter(|pair| !pair.is_empty())
                        .map(|pair| (pair.split_once('=').unwrap_or((pair, "")), pair))
                };
                let id = if !written.contains(['/', '.']) {
                    written.to_owned()
                } else {
                    let rest = rest
                        .strip_prefix("www.")
                        .or_else(|| rest.strip_prefix("m."))
                        .unwrap_or(rest);
                    if let Some(path) = rest.strip_prefix("youtu.be/") {
                        first(path)
                    } else {
                        let path = rest
                            .strip_prefix("youtube.com/")
                            .or_else(|| rest.strip_prefix("youtube-nocookie.com/"))?;
                        let shaped = ["embed/", "shorts/", "live/"]
                            .iter()
                            .find_map(|shape| path.strip_prefix(shape).map(first));
                        let watched = pairs()
                            .find(|((key, _), _)| *key == "v")
                            .map(|((_, id), _)| id.to_owned());
                        watched.or(shaped)?
                    }
                };
                let mut video = Video {
                    id,
                    start: None,
                    rest: Vec::new(),
                };
                if !plausible(&video.id) {
                    return None;
                }
                for ((key, value), pair) in pairs() {
                    match key {
                        "t" | "start" if video.start.is_none() => video.start = seconds(value),
                        key if PLAYER_READS.contains(&key) => video.rest.push(pair.to_owned()),
                        _ => {}
                    }
                }
                Some(video)
            }
            Host::Vimeo => {
                let digits = |id: &str| !id.is_empty() && id.chars().all(|c| c.is_ascii_digit());
                let id = if digits(written) {
                    written.to_owned()
                } else {
                    let rest = rest.strip_prefix("www.").unwrap_or(rest);
                    let path = rest
                        .strip_prefix("vimeo.com/")
                        .or_else(|| rest.strip_prefix("player.vimeo.com/video/"))?;
                    first(path)
                };
                digits(&id).then(|| Video {
                    id,
                    start: fragment.strip_prefix("t=").and_then(seconds),
                    rest: Vec::new(),
                })
            }
        }
    }

    /// Where the player is, fetched when a reader presses play, starting
    /// where the address said.
    fn embed(self, video: &Video) -> String {
        let id = &video.id;
        match self {
            Host::YouTube => {
                let mut embed = format!("https://www.youtube-nocookie.com/embed/{id}?autoplay=1");
                if let Some(start) = video.start {
                    embed.push_str(&format!("&start={start}"));
                }
                for pair in &video.rest {
                    embed.push('&');
                    embed.push_str(pair);
                }
                embed
            }
            Host::Vimeo => match video.start {
                Some(start) => format!("https://player.vimeo.com/video/{id}?autoplay=1#t={start}s"),
                None => format!("https://player.vimeo.com/video/{id}?autoplay=1"),
            },
        }
    }
}

/// A time as a video site writes one: `90`, `90s`, `1m30s` or `1h2m3s`.
fn seconds(written: &str) -> Option<u64> {
    if !written.is_empty() && written.chars().all(|c| c.is_ascii_digit()) {
        return written.parse().ok();
    }
    let mut total: u64 = 0;
    let mut digits = String::new();
    let mut last = 0;
    for c in written.chars() {
        if c.is_ascii_digit() {
            digits.push(c);
            continue;
        }
        // Hours, then minutes, then seconds, each once and in that order.
        let (rank, scale) = match c {
            'h' => (1, 3600),
            'm' => (2, 60),
            's' => (3, 1),
            _ => return None,
        };
        if digits.is_empty() || rank <= last {
            return None;
        }
        total = total.checked_add(digits.parse::<u64>().ok()?.checked_mul(scale)?)?;
        digits.clear();
        last = rank;
    }
    (digits.is_empty() && last > 0).then_some(total)
}

/// A video kept on another site, as a template sees it.
#[derive(Debug)]
struct VideoValue {
    shared: Arc<Shared>,
    host: Host,
    video: Video,
    /// The address or id as the call was handed it.
    written: String,
    asking: Asking,
    /// The picture fetched to show until a reader presses play, or none;
    /// looked for when first asked.
    still: OnceLock<Value>,
}

impl VideoValue {
    /// The still in `fetched/stills/`, as `page.photo` gives a picture.
    /// Where there is none it is told, but for a playlist, which never
    /// has one.
    fn still(&self) -> Value {
        let (host, id) = (self.host, &self.video.id);
        if !host.is_one_video(id) {
            return Value::from(());
        }
        let shared = &self.shared;
        let path = stills::path(host, id);
        if let Some(found) = shared.site.pictures.fetched(&path) {
            return shown(shared, found, &Options::default(), &self.asking)
                .unwrap_or_else(Value::from);
        }
        frame::read(Key::Fetched(crate::fetched::key(&path)));
        let line = shared.site.fetched.record.get(&path);
        let kind = Kind::NoThumbnail {
            host,
            id: id.clone(),
            absent: line.is_some_and(|line| line.held == crate::fetched::Held::Absent),
        };
        // Under the address or id as written, where the call shows it.
        let at = self.asking.place(shared).map(|at| {
            let written = self.written.as_str();
            let found = shared.sources.spelling(at).find(written);
            match found.filter(|_| !written.is_empty()) {
                Some(from) => Place {
                    source: at.source,
                    span: crate::source::Span::new(
                        at.span.start as usize + from,
                        at.span.start as usize + from + written.len(),
                    ),
                },
                None => at,
            }
        });
        told_at(at, kind);
        Value::from(())
    }
}

impl Object for VideoValue {
    fn repr(self: &Arc<Self>) -> ObjectRepr {
        ObjectRepr::Map
    }

    fn is_true(self: &Arc<Self>) -> bool {
        true
    }

    /// A start the address doesn't give, and a still there isn't, are no
    /// values at all, as a field a page doesn't set is none.
    fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
        let name = key.as_str()?;
        let held = match name {
            "id" => Some(Value::from(self.video.id.as_str())),
            "start" => self.video.start.map(Value::from),
            "embed" => Some(Value::from(self.host.embed(&self.video))),
            "thumbnail" => Some(self.still.get_or_init(|| self.still()).clone())
                .filter(|still| !still.is_none()),
            _ => None,
        };
        if held.is_none() {
            frame::missed(None, name);
        }
        held
    }

    fn enumerate(self: &Arc<Self>) -> Enumerator {
        Enumerator::Str(&["id", "start", "embed", "thumbnail"])
    }

    /// A video printed is its id.
    fn render(self: &Arc<Self>, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.video.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn video(id: &str, start: Option<u64>, rest: &[&str]) -> Option<Video> {
        Some(Video {
            id: id.to_owned(),
            start,
            rest: rest.iter().map(|pair| (*pair).to_owned()).collect(),
        })
    }

    #[test]
    fn a_youtube_video_is_read_however_its_address_is_spelled() {
        let id = "abcdefghijk";
        for written in [
            "abcdefghijk",
            "https://www.youtube.com/watch?v=abcdefghijk",
            "https://youtube.com/watch?feature=shared&v=abcdefghijk",
            "http://m.youtube.com/watch?v=abcdefghijk",
            "https://youtu.be/abcdefghijk?si=0123456789",
            "https://www.youtube.com/embed/abcdefghijk",
            "https://www.youtube.com/shorts/abcdefghijk",
            "https://www.youtube.com/live/abcdefghijk",
            "https://www.youtube-nocookie.com/embed/abcdefghijk",
            "youtu.be/abcdefghijk",
        ] {
            assert_eq!(
                Host::YouTube.read(written),
                video(id, None, &[]),
                "{written}"
            );
        }
        for written in [
            "",
            "video-1",
            "https://www.youtube.com/watch?v=short",
            "https://example.org/watch?v=abcdefghijk",
            "https://vimeo.com/123456789",
        ] {
            assert_eq!(Host::YouTube.read(written), None, "{written}");
        }
    }

    #[test]
    fn a_vimeo_video_is_read_however_its_address_is_spelled() {
        for written in [
            "123456789",
            "https://vimeo.com/123456789",
            "https://www.vimeo.com/123456789?share=copy",
            "https://player.vimeo.com/video/123456789",
        ] {
            let read = Host::Vimeo.read(written);
            assert_eq!(read, video("123456789", None, &[]), "{written}");
        }
        for written in ["", "video-1", "https://vimeo.com/channels/staff"] {
            assert_eq!(Host::Vimeo.read(written), None, "{written}");
        }
    }

    #[test]
    fn a_player_starts_where_the_address_says_and_is_handed_what_it_reads() {
        let at = |written: &str| Host::YouTube.read(written).map(|v| Host::YouTube.embed(&v));
        let player = "https://www.youtube-nocookie.com/embed/abcdefghijk?autoplay=1";
        assert_eq!(
            at("https://youtu.be/abcdefghijk?t=90"),
            Some(format!("{player}&start=90"))
        );
        assert_eq!(
            at("https://www.youtube.com/watch?v=abcdefghijk&t=1m30s&end=287&si=0123"),
            Some(format!("{player}&start=90&end=287"))
        );
        assert_eq!(
            at("https://www.youtube.com/watch?v=abcdefghijk&list=PL0123"),
            Some(format!("{player}&list=PL0123"))
        );
        let vimeo = |written: &str| Host::Vimeo.read(written).map(|v| Host::Vimeo.embed(&v));
        assert_eq!(
            vimeo("https://vimeo.com/123456789#t=1h2m3s"),
            Some("https://player.vimeo.com/video/123456789?autoplay=1#t=3723s".into())
        );
        assert_eq!(
            vimeo("https://vimeo.com/123456789"),
            Some("https://player.vimeo.com/video/123456789?autoplay=1".into())
        );
    }

    #[test]
    fn a_time_is_read_as_a_video_site_writes_one() {
        for (written, read) in [
            ("90", Some(90)),
            ("90s", Some(90)),
            ("1m30s", Some(90)),
            ("1h2m3s", Some(3723)),
            ("2m", Some(120)),
            ("", None),
            ("30s1m", None),
            ("1m1m", None),
            ("soon", None),
        ] {
            assert_eq!(seconds(written), read, "{written}");
        }
    }

    #[test]
    fn a_file_name_reads_as_words() {
        assert_eq!(as_words("first-photo-1.jpg"), "First Photo 1");
        assert_eq!(as_words("photo_1.jpeg"), "Photo 1");
        assert_eq!(as_words("---.jpg"), "---");
    }
}
