//! A page's words: Statac's own, typed, and the author's own fields, kept
//! as placed YAML. Each value carries its origin, so a value handed down
//! from a folder still says where it was written.

use crate::address::{item_pattern_faults, pattern_faults};
use crate::config::declarations::Declarations;
use crate::origin::{Origin, Sourced};
use crate::problem::{Expected, Kind, Own, Problem, Root, SetAside, Word, WrongPlace};
use crate::source::{Place, Sources, Span};
use crate::value::{Reader, Value, place_within};
use crate::yaml::{Node, Resolved, Value as Yaml};

/// Statac's own words on a page.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum PageWord {
    Title,
    Date,
    Updated,
    Summary,
    Permalink,
    RedirectFrom,
    RedirectTo,
    Tags,
    Draft,
    Layout,
    Series,
    Type,
    Mentions,
    Needs,
    Card,
    Feed,
    IndexPage,
    Neighbours,
    OgImage,
    OgTitle,
    Order,
    Numbered,
    Posts,
    Each,
    PostsPerPage,
    Id,
    TranslationKey,
    Untitled,
    Search,
}

/// How a word's value is read.
#[derive(Copy, Clone, Debug)]
enum Shape {
    Text,
    Date,
    /// A list of text, or one text.
    Texts,
    Truth,
    OneOf(&'static [&'static str]),
    /// One of a set of words, or `false` for none of them.
    OneOfOrFalse(&'static [&'static str]),
    Whole(u64),
    /// `true`, `false`, or where, or a list of wheres.
    Feed,
    Id,
    /// An address, or a pattern to fill one in from.
    Pattern,
    /// `oldest first` or `newest first`, or a list of pages' names.
    Order,
    /// A pattern for the pages made from a table's items, whose pieces are
    /// the items' fields.
    ItemPattern,
}

/// The ways `order:` runs a list, besides a list of names.
const ORDERS: &[&str] = &["oldest first", "newest first"];

struct Spec {
    word: PageWord,
    name: &'static str,
    shape: Shape,
    /// Whether a folder may hand it down. A title, a date, an old address
    /// and the like are each page's own.
    handed: bool,
}

const fn spec(word: PageWord, name: &'static str, shape: Shape, handed: bool) -> Spec {
    Spec {
        word,
        name,
        shape,
        handed,
    }
}

const WORDS: &[Spec] = &[
    spec(PageWord::Title, "title", Shape::Text, false),
    spec(PageWord::Date, "date", Shape::Date, false),
    spec(PageWord::Updated, "updated", Shape::Date, true),
    spec(PageWord::Summary, "summary", Shape::Text, true),
    spec(PageWord::Permalink, "permalink", Shape::Pattern, true),
    spec(PageWord::RedirectFrom, "redirect_from", Shape::Texts, false),
    spec(PageWord::RedirectTo, "redirect_to", Shape::Text, false),
    spec(PageWord::Tags, "tags", Shape::Texts, true),
    spec(PageWord::Draft, "draft", Shape::Truth, true),
    spec(PageWord::Layout, "layout", Shape::Text, true),
    spec(PageWord::Series, "series", Shape::Text, true),
    spec(PageWord::Type, "type", Shape::Text, true),
    spec(PageWord::Mentions, "mentions", Shape::Truth, true),
    spec(PageWord::Needs, "needs", Shape::OneOf(&["url"]), false),
    spec(PageWord::Card, "card", Shape::Text, true),
    spec(PageWord::Feed, "feed", Shape::Feed, true),
    spec(PageWord::IndexPage, "index_page", Shape::Truth, true),
    spec(
        PageWord::Neighbours,
        "neighbours",
        Shape::OneOfOrFalse(&["site", "folder", "series"]),
        true,
    ),
    spec(PageWord::OgImage, "og_image", Shape::Text, true),
    spec(PageWord::OgTitle, "og_title", Shape::Text, true),
    spec(PageWord::Order, "order", Shape::Order, true),
    spec(PageWord::Numbered, "numbered", Shape::Truth, false),
    spec(PageWord::Posts, "posts", Shape::Text, false),
    spec(PageWord::Each, "each", Shape::Text, false),
    spec(
        PageWord::PostsPerPage,
        "posts_per_page",
        Shape::Whole(0),
        false,
    ),
    spec(PageWord::Id, "id", Shape::Id, false),
    spec(
        PageWord::TranslationKey,
        "translation_key",
        Shape::Text,
        false,
    ),
    spec(PageWord::Untitled, "untitled", Shape::Truth, true),
    spec(PageWord::Search, "search", Shape::Truth, true),
];

/// The names Statac fills on every page itself, so a page writing one is
/// read by nothing.
pub const STATAC_FILLS: &[&str] = &[
    "slug",
    "url",
    "key",
    "content",
    "toc",
    "word_count",
    "folder",
    "list",
    "number",
    "next",
    "previous",
    "image",
    "images",
    "file",
    "video",
    "audio",
    "youtube",
    "vimeo",
    "language",
    "direction",
    "opening",
    "excerpt",
    "excerpt_marked",
    "maths",
    "source",
    "translations",
];

/// Other tools' words for Statac's own, and Statac's.
pub const OTHER_SPELLINGS: &[(&str, &str)] = &[
    ("template", "layout"),
    ("aliases", "redirect_from"),
    ("categories", "tags"),
    ("publishdate", "date"),
    ("lastmod", "updated"),
];

/// Other tools' words for Statac's own that mean something only on a site
/// in several languages. A site in one language may use them as its own.
const OTHER_SPELLINGS_WITH_LANGUAGES: &[(&str, &str)] = &[("translationKey", "translation_key")];

/// Statac's word for `name` where `name` is another tool's.
fn ours_for(name: &str, several_languages: bool) -> Option<&'static str> {
    let with_languages = OTHER_SPELLINGS_WITH_LANGUAGES
        .iter()
        .filter(|_| several_languages);
    OTHER_SPELLINGS
        .iter()
        .chain(with_languages)
        .find(|(other, _)| *other == name)
        .map(|&(_, ours)| ours)
}

const ID_ALPHABET: &str = "0123456789abcdefghjkmnpqrstvwxyz";

impl PageWord {
    fn spec(self) -> &'static Spec {
        WORDS
            .iter()
            .find(|spec| spec.word == self)
            .expect("every word has a row")
    }

    pub fn name(self) -> &'static str {
        self.spec().name
    }

    pub fn named(name: &str) -> Option<PageWord> {
        WORDS
            .iter()
            .find(|spec| spec.name == name)
            .map(|spec| spec.word)
    }

    pub fn handed_down(self) -> bool {
        self.spec().handed
    }

    pub fn all() -> impl Iterator<Item = PageWord> {
        WORDS.iter().map(|spec| spec.word)
    }

    /// The values it takes where they are a few words to choose from.
    pub fn choices(self) -> Vec<&'static str> {
        match self.spec().shape {
            Shape::Truth | Shape::Feed => vec!["true", "false"],
            Shape::OneOf(words) => words.to_vec(),
            Shape::OneOfOrFalse(words) => [words, &["false"]].concat(),
            Shape::Order => ORDERS.to_vec(),
            _ => Vec::new(),
        }
    }
}

/// Whether `name` is a word Statac gives a meaning to on a page.
pub fn is_statacs(name: &str) -> bool {
    PageWord::named(name).is_some() || STATAC_FILLS.contains(&name)
}

/// One of the author's own fields.
#[derive(Clone, PartialEq, Debug)]
pub struct Field {
    pub name: String,
    /// Where its name is written.
    pub key: Place,
    pub value: Node,
    pub origin: Origin,
}

/// The values on a page, or the values a folder hands down.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct Words {
    pub words: Vec<(PageWord, Sourced<Value>)>,
    pub fields: Vec<Field>,
}

impl Words {
    pub fn get(&self, word: PageWord) -> Option<&Sourced<Value>> {
        self.words.iter().find(|(w, _)| *w == word).map(|(_, v)| v)
    }

    pub fn text(&self, word: PageWord) -> Option<Sourced<&str>> {
        let value = self.get(word)?;
        Some(Sourced::new(value.value.as_text()?, value.origin.clone()))
    }

    pub fn truth(&self, word: PageWord) -> Option<bool> {
        self.get(word)?.value.as_bool()
    }

    pub fn field(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|field| field.name == name)
    }

    /// Whether the field at `path` is written: under that name, or nested,
    /// as `book:` with `author:` beneath it is `book.author`.
    pub fn has(&self, path: &str) -> bool {
        self.at(path).is_some()
    }

    /// The value of the field at `path`, written under that name or
    /// nested, as [`Words::has`] finds it.
    pub fn at(&self, path: &str) -> Option<&Node> {
        if let Some(field) = self.field(path) {
            return Some(&field.value);
        }
        let mut names = path.split('.');
        let mut node = &self.field(names.next()?)?.value;
        for name in names {
            node = node.get(name)?;
        }
        Some(node)
    }

    fn set(&mut self, word: PageWord, value: Sourced<Value>) {
        self.words.push((word, value));
    }

    /// Takes from `layer` whatever isn't set here, as handed down.
    pub fn hand_down_from(&mut self, layer: &Words) {
        for (word, value) in &layer.words {
            if self.get(*word).is_none() {
                let origin = value.origin.handed_down();
                self.words
                    .push((*word, Sourced::new(value.value.clone(), origin)));
            }
        }
        for field in &layer.fields {
            if self.field(&field.name).is_none() {
                self.fields.push(Field {
                    origin: field.origin.handed_down(),
                    ..field.clone()
                });
            }
        }
    }
}

/// What holds the words being read.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Holder {
    Page,
    /// A folder's own page, which may hand words down to the pages inside.
    FolderPage {
        top: bool,
    },
}

/// One file's front matter read.
#[derive(Default, Debug)]
pub struct Read {
    pub own: Words,
    /// What a folder's own page hands down.
    pub layer: Words,
    pub problems: Vec<Problem>,
}

/// The name of a mapping's key as written.
pub fn key_name(key: &Node) -> String {
    match &key.value {
        Yaml::Scalar(text) => text.clone(),
        _ => String::new(),
    }
}

/// Reads front matter: Statac's words typed, the author's fields kept and
/// checked against their declarations, `pages.` lines into the layer.
pub fn read_front_matter(
    sources: &Sources,
    node: Option<&Node>,
    holder: Holder,
    declarations: &Declarations,
    several_languages: bool,
) -> Read {
    let mut read = Read::default();
    let Some(node) = node else {
        return read;
    };
    let entries = match &node.value {
        Yaml::Mapping(entries) => entries,
        Yaml::Scalar(_) if node.resolved() == Some(crate::yaml::Resolved::Null) => return read,
        _ => {
            let reader = Reader::new(sources, crate::wording::FRONT_MATTER);
            read.problems
                .push(reader.wrong(node, Expected::Fields, None));
            return read;
        }
    };
    // A page making a page per item reads `permalink` as the items'
    // pattern, whose pieces are the items' fields.
    let makes_pages = entries.iter().any(|entry| key_name(&entry.key) == "each");
    for entry in entries {
        let name = key_name(&entry.key);
        let key = &entry.key;
        let value = &entry.value;
        if let Some(ours) = ours_for(&name, several_languages) {
            read.problems.push(other_spelling(&name, ours, key));
        } else if let Some(rest) = name.strip_prefix("default.") {
            read.problems
                .push(other_spelling(&name, &format!("pages.{rest}"), key));
        } else if name == "pages" && matches!(value.value, Yaml::Mapping(_)) {
            read.problems.extend(nested(sources, "pages", value));
        } else if let Some(settings) = nested_site_settings(&name, value) {
            // Nested, a site setting is as out of place as written whole.
            for (flat, key) in settings {
                read.problems
                    .push(wrong_place(&flat, WrongPlace::SiteSetting, key));
            }
        } else if name == "pages.later" {
            read.problems.push(Problem::at(
                Kind::OtherSpelling {
                    word: Word::Setting,
                    written: name,
                    ours: "pagination.permalink".into(),
                },
                key.place,
            ));
        } else if let Some(rest) = name.strip_prefix("pages.") {
            match holder {
                Holder::FolderPage { top: false } => read_handed(
                    sources,
                    key,
                    rest,
                    value,
                    (declarations, several_languages),
                    &mut read.layer,
                    &mut read.problems,
                ),
                Holder::FolderPage { top: true } => {
                    read.problems
                        .push(wrong_place(&name, WrongPlace::TopFolder, key))
                }
                Holder::Page => {
                    read.problems
                        .push(wrong_place(&name, WrongPlace::NothingInside, key))
                }
            }
        } else if is_list_setting(&name) {
            read.problems
                .push(wrong_place(&name, WrongPlace::SiteSetting, key));
        } else if name.starts_with("site.") {
            read.problems
                .push(wrong_place(&name, WrongPlace::SiteValue, key));
        } else if STATAC_FILLS.contains(&name.as_str()) {
            read.problems.push(set_aside(&name, key));
        } else if name == PageWord::Numbered.name() && holder == Holder::Page {
            read.problems
                .push(wrong_place(&name, WrongPlace::FolderOnly, key));
        } else if let Some(word) = PageWord::named(&name) {
            let shape = match word {
                PageWord::Permalink if makes_pages => Some(Shape::ItemPattern),
                _ => None,
            };
            read.problems.extend(empty(&name, Word::Field, key, value));
            read.problems.extend(one_tag(sources, word, key, value));
            match read_word(sources, word, value, (false, declarations), shape) {
                Ok(Some(value)) => read.own.set(word, value),
                Ok(None) => {}
                Err(problems) => {
                    read.problems.extend(problems);
                    // So the pages it would make aren't also told they have no pattern.
                    if shape.is_some() {
                        read.own
                            .set(word, Sourced::new(Value::Refused, value.origin()));
                    }
                }
            }
        } else {
            let words = PageWord::all().map(PageWord::name);
            read.problems
                .extend(slip("", &name, words, declarations, key));
            if declarations.kind(&name).is_some() {
                read.problems.extend(empty(&name, Word::Field, key, value));
            }
            read.problems
                .extend(declarations.check_all(sources, &name, value));
            read.own.fields.push(Field {
                name,
                key: key.place,
                value: value.clone(),
                origin: value.origin(),
            });
        }
    }
    read
}

/// Reads `pages.<rest>: value`, a word handed down to the pages inside,
/// checked against the site's declarations, on a site in several
/// languages or not.
pub fn read_handed(
    sources: &Sources,
    key: &Node,
    rest: &str,
    value: &Node,
    (declarations, several_languages): (&Declarations, bool),
    layer: &mut Words,
    problems: &mut Vec<Problem>,
) {
    let name = format!("pages.{rest}");
    if rest == "url" {
        problems.push(other_spelling(&name, "pages.permalink", key));
    } else if let Some(ours) = ours_for(rest, several_languages) {
        problems.push(other_spelling(&name, &format!("pages.{ours}"), key));
    } else if let Some(word) = PageWord::named(rest) {
        if !word.handed_down() {
            problems.push(wrong_place(&name, WrongPlace::EachPagesOwn, key));
            return;
        }
        problems.extend(empty(&name, Word::Field, key, value));
        problems.extend(one_tag(sources, word, key, value));
        match read_word(sources, word, value, (true, declarations), None) {
            Ok(Some(value)) => layer.set(word, value),
            Ok(None) => {}
            Err(found) => problems.extend(found),
        }
    } else if STATAC_FILLS.contains(&rest) {
        problems.push(set_aside(rest, key));
    } else if crate::config::is_setting(rest) {
        problems.push(wrong_place(&name, WrongPlace::SiteSetting, key));
    } else if rest.starts_with("site.") {
        problems.push(wrong_place(rest, WrongPlace::SiteValue, key));
    } else {
        let words = PageWord::all()
            .filter(|word| word.handed_down())
            .map(PageWord::name);
        problems.extend(slip("pages.", rest, words, declarations, key));
        if declarations.kind(rest).is_some() {
            problems.extend(empty(&name, Word::Field, key, value));
        }
        problems.extend(declarations.check_all(sources, rest, value));
        layer.fields.push(Field {
            name: rest.to_owned(),
            key: key.place,
            value: value.clone(),
            origin: value.origin(),
        });
    }
}

/// A warning where the author's own field `name`, written after `prefix`,
/// is one slip from one of `words` or a declared field. A declared field
/// is never told, nor a name another tool gives meaning to only on a site
/// in several languages.
fn slip(
    prefix: &str,
    name: &str,
    words: impl Iterator<Item = &'static str>,
    declarations: &Declarations,
    key: &Node,
) -> Option<Problem> {
    if declarations.kind(name).is_some()
        || declarations.nests(name)
        || OTHER_SPELLINGS_WITH_LANGUAGES
            .iter()
            .any(|(other, _)| *other == name)
    {
        return None;
    }
    let known = words.map(|word| word as &str).chain(declarations.names());
    let nearest = crate::suggest::one_slip(name, known)?;
    Some(Problem::at(
        Kind::ReadAsCustom {
            own: Own::Field(name.to_owned()),
            written: format!("{prefix}{name}"),
            nearest: format!("{prefix}{nearest}"),
        },
        key.place,
    ))
}

/// A warning where nothing at all is written after the colon of `name`,
/// one of Statac's words or settings or a declared field. `~` and `null`
/// are written, and meant.
pub fn empty(name: &str, word: Word, key: &Node, value: &Node) -> Option<Problem> {
    let nothing = value.tag.is_none()
        && value.style == crate::origin::Style::Plain
        && matches!(&value.value, Yaml::Scalar(text) if text.is_empty());
    nothing.then(|| {
        Problem::at(
            Kind::SetAside {
                field: name.to_owned(),
                why: SetAside::NothingWritten(word),
            },
            key.place,
        )
    })
}

/// A warning where `tags` is one plain text holding commas, which YAML
/// reads as one tag.
fn one_tag(sources: &Sources, word: PageWord, key: &Node, value: &Node) -> Option<Problem> {
    let Yaml::Scalar(tag) = &value.value else {
        return None;
    };
    if word != PageWord::Tags || value.style != crate::origin::Style::Plain || value.tag.is_some() {
        return None;
    }
    let parts: Vec<String> = (tag.split(','))
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect();
    if parts.len() < 2 {
        return None;
    }
    let line = Place {
        source: key.place.source,
        span: Span::new(key.place.span.range().start, value.place.span.range().end),
    };
    Some(Problem::at(
        Kind::OneTagNotTwo {
            key: sources.spelling(key.place).into(),
            line: sources.spelling(line).into(),
            tag: tag.as_str().into(),
            parts: parts.into(),
        },
        value.place,
    ))
}

/// A warning for each `type` one slip from a type the site declares,
/// where it names no declared type and no template in `templates`. A
/// value handed down is told once, where it is written.
pub fn type_slips<'a>(
    all: impl Iterator<Item = &'a Words>,
    declarations: &Declarations,
    templates: &std::path::Path,
) -> Vec<Problem> {
    let mut problems = Vec::new();
    let mut told = std::collections::HashSet::new();
    for words in all {
        let Some(written) = words.text(PageWord::Type) else {
            continue;
        };
        let Some(place) = written.origin.place else {
            continue;
        };
        if !told.insert(place)
            || declarations
                .types()
                .any(|declared| declared == written.value)
            || templates.join(format!("{}.html", written.value)).is_file()
        {
            continue;
        }
        if let Some(nearest) = crate::suggest::one_slip(written.value, declarations.types()) {
            problems.push(Problem::at(
                Kind::ReadAsCustom {
                    own: Own::Type,
                    written: written.value.to_owned(),
                    nearest: nearest.to_owned(),
                },
                place,
            ));
        }
    }
    problems
}

/// Whether `name` is one of the settings under `tags.`, `series.` and
/// `pagination.`, which only the site sets.
/// The site settings `value` holds where `name` is what they start with,
/// as `pagination:` with `permalink:` beneath it, each written whole and
/// by its key; none where it holds none.
fn nested_site_settings<'a>(name: &str, value: &'a Node) -> Option<Vec<(String, &'a Node)>> {
    let Yaml::Mapping(entries) = &value.value else {
        return None;
    };
    let settings: Vec<(String, &Node)> = entries
        .iter()
        .map(|entry| (format!("{name}.{}", key_name(&entry.key)), &entry.key))
        .filter(|(flat, _)| is_list_setting(flat))
        .collect();
    (!settings.is_empty()).then_some(settings)
}

fn is_list_setting(name: &str) -> bool {
    name.split_once('.')
        .is_some_and(|(list, _)| matches!(list, "tags" | "series" | "pagination"))
        && crate::config::is_setting(name)
}

fn other_spelling(written: &str, ours: &str, key: &Node) -> Problem {
    Problem::at(
        Kind::OtherSpelling {
            word: Word::Field,
            written: written.to_owned(),
            ours: ours.to_owned(),
        },
        key.place,
    )
}

fn wrong_place(key: &str, why: WrongPlace, at: &Node) -> Problem {
    Problem::at(
        Kind::WrongPlace {
            key: key.to_owned(),
            why,
        },
        at.place,
    )
}

fn set_aside(field: &str, key: &Node) -> Problem {
    Problem::at(
        Kind::SetAside {
            field: field.to_owned(),
            why: SetAside::StatacFills(Root::Page),
        },
        key.place,
    )
}

/// Each setting nested under `under`, refused with its flat line offered.
/// A declaration is a line of its own however deep it is nested, so each
/// is offered whole: `fields.book.author: text`.
pub fn nested(sources: &Sources, under: &str, value: &Node) -> Vec<Problem> {
    let mut problems = Vec::new();
    let Yaml::Mapping(entries) = &value.value else {
        return problems;
    };
    let declares = ["fields", "types"].contains(&under.split('.').next().unwrap_or(under));
    for entry in entries {
        let key = key_name(&entry.key);
        let flat_key = format!("{under}.{key}");
        if declares && matches!(entry.value.value, Yaml::Mapping(_)) {
            problems.extend(nested(sources, &flat_key, &entry.value));
            continue;
        }
        let flat = match &entry.value.value {
            Yaml::Scalar(_) => format!("{flat_key}: {}", sources.spelling(entry.value.place)),
            _ => format!("{flat_key}:"),
        };
        problems.push(Problem::at(
            Kind::NestedSetting {
                key,
                under: under.to_owned(),
                flat,
            },
            entry.key.place,
        ));
    }
    problems
}

/// A `feed:` value, on a page or in the settings: `true`, `false`, or the
/// name or URL of the list's feed, or a list of them for several feeds.
pub(crate) fn feed(reader: &Reader, node: &Node) -> Result<Option<Sourced<Value>>, Problem> {
    if let Yaml::Sequence(_) = &node.value {
        return Ok(reader.texts(node)?.map(|names| names.map(Value::Texts)));
    }
    match reader.truth(node) {
        Ok(truth) => Ok(truth.map(|b| b.map(Value::Bool))),
        // `yes` is another spelling of `true` here too, not a feed's name.
        Err(problem)
            if matches!(
                problem.kind,
                Kind::WrongKind {
                    respelt: Some(_),
                    ..
                }
            ) =>
        {
            Err(problem)
        }
        Err(_) => match reader.text(node) {
            Ok(text) => Ok(text.map(|t| t.map(Value::Text))),
            Err(_) => Err(reader.wrong(node, Expected::TrueFalseOrAddress, None)),
        },
    }
}

/// Reads one of Statac's words, as `shape` where it is given. `handed`
/// says the value is written for the pages inside, where a pattern must ask
/// for a piece; a pattern may ask for a field `declarations` declare.
fn read_word(
    sources: &Sources,
    word: PageWord,
    node: &Node,
    (handed, declarations): (bool, &Declarations),
    shape: Option<Shape>,
) -> Result<Option<Sourced<Value>>, Vec<Problem>> {
    let reader = Reader::new(sources, word.name());
    let one = |problem| vec![problem];
    let value = match shape.unwrap_or(word.spec().shape) {
        Shape::Text => reader.text(node).map_err(one)?.map(|t| t.map(Value::Text)),
        Shape::Date => reader.date(node).map_err(one)?.map(|d| d.map(Value::Date)),
        Shape::Texts => reader
            .texts(node)
            .map_err(one)?
            .map(|t| t.map(Value::Texts)),
        Shape::Truth => reader.truth(node).map_err(one)?.map(|b| b.map(Value::Bool)),
        Shape::OneOf(words) => reader
            .one_of(node, words)
            .map_err(one)?
            .map(|t| t.map(Value::Text)),
        Shape::OneOfOrFalse(words) => match node.resolved() {
            Some(Resolved::Bool(false)) => {
                reader.truth(node).map_err(one)?.map(|b| b.map(Value::Bool))
            }
            _ => reader
                .one_of_or_false(node, words)
                .map_err(one)?
                .map(|t| t.map(Value::Text)),
        },
        Shape::Whole(from) => reader
            .whole(node, from)
            .map_err(one)?
            .map(|n| n.map(Value::Whole)),
        Shape::Feed => feed(&reader, node).map_err(one)?,
        Shape::Order => match &node.value {
            Yaml::Sequence(_) => reader
                .texts(node)
                .map_err(one)?
                .map(|t| t.map(Value::Texts)),
            _ => reader
                .one_of(node, ORDERS)
                .map_err(one)?
                .map(|t| t.map(Value::Text)),
        },
        Shape::Id => match reader.text(node) {
            Ok(Some(id)) if well_formed_id(&id.value) => Some(id.map(Value::Text)),
            Ok(None) => None,
            _ => return Err(one(reader.wrong(node, Expected::Id, None))),
        },
        Shape::Pattern | Shape::ItemPattern => match reader.text(node).map_err(one)? {
            Some(pattern) => {
                let faults = match shape {
                    Some(Shape::ItemPattern) => item_pattern_faults(&pattern.value),
                    _ => {
                        let declared: Vec<&str> = declarations.names().collect();
                        pattern_faults(&pattern.value, handed, &declared)
                    }
                };
                if faults.is_empty() {
                    Some(pattern.map(Value::Text))
                } else {
                    return Err(faults
                        .into_iter()
                        .map(|(kind, span)| {
                            let place =
                                place_within(sources, &pattern.origin, &pattern.value, span)
                                    .unwrap_or(node.place);
                            Problem::at(kind, place)
                        })
                        .collect());
                }
            }
            None => None,
        },
    };
    Ok(value)
}

/// Six letters and digits from Statac's id alphabet, or a feed entry's id
/// in five runs of hex.
fn well_formed_id(id: &str) -> bool {
    let ours = id.len() == 6 && id.chars().all(|c| ID_ALPHABET.contains(c));
    let runs: Vec<&str> = id.split('-').collect();
    let feed = runs.len() == 5
        && runs.iter().zip([8, 4, 4, 4, 12]).all(|(run, length)| {
            run.len() == length
                && run
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        });
    ours || feed
}

/// The author's own fields at every depth, for finding one written flat on
/// one page and nested on another: each flat name holding a dot, and each
/// nested path, with the place of its last key.
pub fn shapes(
    fields: &[Field],
    flat: &mut Vec<(String, Place)>,
    nested: &mut Vec<(String, Place)>,
) {
    fn walk(prefix: &str, node: &Node, nested: &mut Vec<(String, Place)>) {
        let Yaml::Mapping(entries) = &node.value else {
            return;
        };
        for entry in entries {
            let path = format!("{prefix}.{}", key_name(&entry.key));
            nested.push((path.clone(), entry.key.place));
            walk(&path, &entry.value, nested);
        }
    }
    for field in fields {
        if field.name.contains('.') {
            flat.push((field.name.clone(), field.key));
        }
        walk(&field.name, &field.value, nested);
    }
}

/// A warning for each flat name that is also written nested.
pub fn written_two_ways(
    on: Root,
    flat: &[(String, Place)],
    nested: &[(String, Place)],
) -> Vec<Problem> {
    let mut problems = Vec::new();
    let mut told = std::collections::HashSet::new();
    for (name, key) in flat {
        if !told.insert(*key) {
            continue;
        }
        if let Some((_, at)) = nested.iter().find(|(path, _)| path == name) {
            let nested_under = name.split('.').next().unwrap_or(name).to_owned();
            problems.push(Problem::at(
                Kind::FieldWrittenTwoWays {
                    on,
                    flat: name.clone(),
                    nested_under,
                    nested: *at,
                },
                *key,
            ));
        }
    }
    problems
}

/// The first line of a source, where a problem about a whole page is shown.
pub fn head(sources: &Sources, source: crate::source::SourceId) -> Place {
    let text = sources.text(source);
    let start = if text.starts_with('\u{feff}') { 3 } else { 0 };
    let end = text[start..]
        .find(['\n', '\r'])
        .map_or(text.len(), |at| start + at);
    Place {
        source,
        span: Span::new(start, end),
    }
}
