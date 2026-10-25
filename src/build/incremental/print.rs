//! What each thing an output can read is known by across readings of a
//! site, and the fingerprint of what the site holds under it.
//!
//! A fingerprint covers a value and where each part of it is written, so
//! an output kept for it tells its problems at the right text. Most are
//! taken from the text of the files a value is written in, which decides
//! both. Where what an output can see through a key is wider than the key
//! says, the fingerprint is the wider thing: a page's fields are one
//! fingerprint, and a picture's carries which pictures the site has.
//!
//! What a key covers beyond a value, such as what a page's answer to a
//! name turns on or what a list is known by, is the build's to say, beside
//! the code that reads it; the fingerprints here take it from there.

use std::collections::{HashMap, HashSet};
use std::fmt::{self, Debug, Write};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use crate::album::Root;
use crate::build::{Options, Output, Plan};
use crate::content::words::PageWord;
use crate::disk::{Clock, Stamp};
use crate::origin::{Origin, Sourced};
use crate::reads::{Key, Translation};
use crate::site::{Id, Item, ListId, ListKey, NameId, PageId, Site};
use crate::source::{Place, SourceId, Sources};
use crate::templates::{Answering, Renderer};
use crate::value::Value;
use crate::yaml::{Node, Value as Yaml};

/// One reading of a site, and what each of its ids names.
pub(crate) struct Reading {
    pub renderer: Renderer,
    pages: Vec<String>,
    lists: Vec<String>,
    templates: Vec<String>,
    /// The file each template is read from, and whether it builds
    /// anything.
    template_files: Vec<(SourceId, bool)>,
    data: Vec<String>,
    assets: Vec<String>,
    pictures: Vec<String>,
    beside: Vec<String>,
    statics: Vec<String>,
    /// Each old address that sends readers on, with the page writing it.
    redirects: Vec<String>,
    /// Each feed's address, with its list.
    feeds: Vec<String>,
    /// Each file read, by its path and which of the files read at that
    /// path it is.
    sources: Vec<(PathBuf, usize)>,
}

fn page_name(site: &Site, id: PageId) -> String {
    let page = site.pages.get(id);
    let mut name = format!("{} {:?}", page.path, page.place);
    if let Some(made) = &page.made {
        let by = &site.pages.get(made.by).path;
        let _ = match &made.item {
            Item::Key(key) => write!(name, " by {by} for {key:?}"),
            Item::At(at) => write!(name, " by {by} at {at}"),
            Item::Post { named, .. } => write!(name, " by {by} of {named}"),
        };
    }
    // A list's page no file holds stands only where it is published.
    if page.source.is_none() {
        let _ = write!(name, " {} {:?}", page.language.index(), page.address);
    }
    name
}

impl Reading {
    pub(super) fn of(renderer: Renderer) -> Reading {
        let site = renderer.site();
        let pages: Vec<String> = (site.pages.iter())
            .map(|(id, _)| page_name(site, id))
            .collect();
        let lists: Vec<String> = (site.lists.iter())
            .map(|(_, list)| {
                let language = list.language.index();
                match &list.key {
                    ListKey::Declared(page) => {
                        format!("{language} declared {}", pages[page.index()])
                    }
                    key => format!("{language} {key:?}"),
                }
            })
            .collect();
        let redirects = site.redirects.iter().map(|redirect| {
            let page = &pages[redirect.page.index()];
            format!("{} from {page}", redirect.from.as_str())
        });
        let feeds = renderer.feeds().iter().map(|feed| {
            let list = &lists[feed.list.index()];
            format!("{} of {list}", feed.address.as_str())
        });
        let mut met: HashMap<&Path, usize> = HashMap::new();
        let sources = renderer.sources();
        let sources = sources.ids().map(|id| {
            let path = sources.path(id);
            let nth = met.entry(path).or_default();
            *nth += 1;
            (path.to_path_buf(), *nth)
        });
        Reading {
            templates: renderer.templates().map(|(held, ..)| held).collect(),
            template_files: (renderer.templates())
                .map(|(_, source, builds)| (source, builds))
                .collect(),
            data: (site.data.files.iter())
                .map(|(_, file)| file.path.clone())
                .collect(),
            assets: (renderer.assets().iter())
                .map(|(_, asset)| asset.name.clone())
                .collect(),
            pictures: (site.pictures.iter())
                .map(|(_, picture)| picture.shown())
                .collect(),
            beside: site.files.iter().map(|file| file.path.clone()).collect(),
            statics: (renderer.assets().statics().iter())
                .map(|file| file.name.clone())
                .collect(),
            redirects: redirects.collect(),
            feeds: feeds.collect(),
            sources: sources.collect(),
            pages,
            lists,
            renderer,
        }
    }

    /// The ids of this reading as `later` has them.
    pub(super) fn to(&self, later: &Reading) -> Translation {
        /// Where each of `was` is among `now`, by name; none for a name
        /// `now` gives to two.
        fn places(was: &[String], now: &[String]) -> impl Iterator<Item = Option<usize>> {
            let mut named: HashMap<&str, Option<usize>> = HashMap::new();
            for (at, name) in now.iter().enumerate() {
                named
                    .entry(name)
                    .and_modify(|twice| *twice = None)
                    .or_insert(Some(at));
            }
            let was = was.iter();
            was.map(move |name| named.get(name.as_str()).copied().flatten())
        }
        fn carried<I: Id>(was: &[String], now: &[String]) -> Vec<Option<I>> {
            places(was, now).map(|at| at.map(I::from_index)).collect()
        }
        fn index(was: &[String], now: &[String]) -> Vec<Option<u32>> {
            places(was, now)
                .map(|at| at.and_then(|at| u32::try_from(at).ok()))
                .collect()
        }
        let (old, new) = (self.renderer.sources(), later.renderer.sources());
        let now: HashMap<&(PathBuf, usize), SourceId> =
            later.sources.iter().zip(new.ids()).collect();
        let sources = self.sources.iter().zip(old.ids()).map(|(name, was)| {
            let now = *now.get(name)?;
            (old.text(was) == new.text(now)).then_some(now)
        });
        let names = (0..self.renderer.names()).map(|at| {
            let name = self.renderer.name_of(NameId::from_index(at));
            later.renderer.name_id(&name)
        });
        Translation {
            sources: sources.collect(),
            pages: carried(&self.pages, &later.pages),
            lists: carried(&self.lists, &later.lists),
            names: names.collect(),
            templates: carried(&self.templates, &later.templates),
            data: carried(&self.data, &later.data),
            assets: carried(&self.assets, &later.assets),
            pictures: carried(&self.pictures, &later.pictures),
            beside: index(&self.beside, &later.beside),
            statics: index(&self.statics, &later.statics),
            redirects: places(&self.redirects, &later.redirects).collect(),
            feeds: places(&self.feeds, &later.feeds).collect(),
        }
    }
}

/// A fingerprint being taken.
#[derive(Default)]
struct Print(DefaultHasher);

struct Shown<'a>(&'a mut DefaultHasher);

impl Write for Shown<'_> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.0.write(text.as_bytes());
        Ok(())
    }
}

impl Print {
    fn with(mut self, value: impl Hash) -> Print {
        value.hash(&mut self.0);
        self
    }

    /// With a value that holds no id, as it prints itself.
    fn shown(mut self, value: &dyn Debug) -> Print {
        let _ = write!(Shown(&mut self.0), "{value:?}");
        self.with(0u8)
    }

    fn place(self, sources: &Sources, place: Option<Place>) -> Print {
        match place {
            Some(place) => self
                .with(sources.path(place.source))
                .with((place.span.start, place.span.end)),
            None => self.with(()),
        }
    }

    fn origin(self, sources: &Sources, origin: &Origin) -> Print {
        self.place(sources, origin.place)
            .shown(&(&origin.style, &origin.arrival))
    }

    /// With a value as YAML reads it, and where each part of it is
    /// written.
    fn node(self, sources: &Sources, node: &Node) -> Print {
        let print = (self.place(sources, Some(node.place))).shown(&(&node.style, &node.tag));
        match &node.value {
            Yaml::Scalar(text) => print.with((0u8, text)),
            Yaml::Sequence(items) => (items.iter())
                .fold(print.with((1u8, items.len())), |print, item| {
                    print.node(sources, item)
                }),
            Yaml::Mapping(entries) => (entries.iter())
                .fold(print.with((2u8, entries.len())), |print, entry| {
                    print.node(sources, &entry.key).node(sources, &entry.value)
                }),
        }
    }

    /// With a value as Statac reads one of its words, and where each part
    /// of it is written.
    fn word(self, sources: &Sources, word: &Sourced<Value>) -> Print {
        let text = |print: Print, text: &Sourced<String>| {
            print.with(&text.value).origin(sources, &text.origin)
        };
        let print = self.origin(sources, &word.origin);
        match &word.value {
            Value::Text(said) => print.with((0u8, said)),
            Value::Bool(truth) => print.with((1u8, truth)),
            Value::Whole(n) => print.with((2u8, n)),
            Value::Texts(texts) => (texts.iter()).fold(print.with((3u8, texts.len())), text),
            Value::Date(date) => print.with(4u8).shown(date),
            Value::Table(rows) => (rows.iter())
                .fold(print.with((5u8, rows.len())), |print, (name, given)| {
                    text(text(print, name), given)
                }),
            Value::Refused => print.with(6u8),
        }
    }

    /// With a file on disk as the reading read it, by `stamp`, its stamp
    /// where it is there: that, where `clock`, read as the reading began,
    /// settles it. Otherwise the file may have changed since it was read,
    /// and the print is equal to no other.
    fn on_disk(self, stamp: Option<Stamp>, clock: Option<&Clock>) -> Print {
        static UNSETTLED: AtomicU64 = AtomicU64::new(0);
        match stamp {
            Some(stamp) if !clock.is_some_and(|clock| stamp.settled(clock)) => {
                self.with((1u8, UNSETTLED.fetch_add(1, Ordering::Relaxed)))
            }
            stamp => self.with((0u8, stamp)),
        }
    }

    fn done(self) -> u64 {
        self.0.finish()
    }
}

/// The stamp of the file `opened` is, where it could be opened.
fn stamp_of(opened: std::io::Result<std::fs::File>) -> Option<Stamp> {
    Stamp::of(&opened.ok()?.metadata().ok()?)
}

type Slots = Vec<OnceLock<u64>>;

/// Fingerprints by what they are of, added to as a reading is asked of.
type Keyed<K> = Mutex<HashMap<K, Arc<OnceLock<u64>>>>;

fn slots(len: usize) -> Slots {
    std::iter::repeat_with(OnceLock::new).take(len).collect()
}

/// The fingerprints taken of one reading, each kept once it is taken, so
/// what a later reading is held to is what this one held when it was read
/// and not what the disk holds by then.
pub(super) struct Prints {
    /// What the site's disk gave a file written as the reading began.
    clock: Option<Clock>,
    /// Each file read, by its source.
    texts: Slots,
    fronts: Slots,
    bodies: Slots,
    neighbours: Slots,
    translations: Slots,
    lists: Slots,
    lists_own: Slots,
    heads: Slots,
    /// Each list's pages, the first first.
    stretches: Vec<OnceLock<Vec<u64>>>,
    templates: Slots,
    data: Slots,
    assets: Slots,
    pictures: Slots,
    beside: Slots,
    statics: Slots,
    every_list: OnceLock<u64>,
    every_front: OnceLock<u64>,
    every_page: OnceLock<u64>,
    spellings: OnceLock<u64>,
    /// Each language's folders and home, by the language.
    folders: Slots,
    homes: Slots,
    /// What every page answers to a name, by the name; names are added to
    /// a reading as it is asked of.
    fields_of_every: Keyed<NameId>,
    /// What every page writing a name answers to another, by the names.
    fields_of_writers: Keyed<(NameId, Option<NameId>, NameId)>,
    every_picture: OnceLock<u64>,
    every_file: OnceLock<u64>,
    /// What the reading holds under `fetched/`, by each file's key.
    fetched: OnceLock<HashMap<u64, u64>>,
    mentions: Slots,
    today: OnceLock<u64>,
}

impl Prints {
    /// The prints of `reading`, which began once `clock` was read.
    pub(super) fn new(reading: &Reading, clock: Option<Clock>) -> Prints {
        let (pages, lists) = (reading.pages.len(), reading.lists.len());
        let languages = reading.renderer.site().languages.len();
        Prints {
            clock,
            texts: slots(reading.sources.len()),
            fronts: slots(pages),
            bodies: slots(pages),
            neighbours: slots(pages),
            translations: slots(pages),
            lists: slots(lists),
            lists_own: slots(lists),
            heads: slots(lists),
            stretches: (std::iter::repeat_with(OnceLock::new).take(lists)).collect(),
            templates: slots(reading.templates.len()),
            data: slots(reading.data.len()),
            assets: slots(reading.assets.len()),
            pictures: slots(reading.pictures.len()),
            beside: slots(reading.beside.len()),
            statics: slots(reading.statics.len()),
            every_list: OnceLock::new(),
            every_front: OnceLock::new(),
            every_page: OnceLock::new(),
            spellings: OnceLock::new(),
            folders: slots(languages),
            homes: slots(languages),
            fields_of_every: Mutex::default(),
            fields_of_writers: Mutex::default(),
            every_picture: OnceLock::new(),
            every_file: OnceLock::new(),
            fetched: OnceLock::new(),
            mentions: slots(pages),
            today: OnceLock::new(),
        }
    }
}

/// One reading of the site at `at`, and the fingerprints taken of it.
#[derive(Copy, Clone)]
pub(super) struct Taken<'a> {
    pub reading: &'a Reading,
    pub at: &'a Path,
    pub prints: &'a Prints,
}

impl Taken<'_> {
    fn named(&self, pages: &[PageId]) -> Vec<&str> {
        let pages = pages.iter();
        pages
            .map(|page| self.reading.pages[page.index()].as_str())
            .collect()
    }

    fn lists_named(&self, lists: &[ListId]) -> Vec<&str> {
        let lists = lists.iter();
        lists
            .map(|list| self.reading.lists[list.index()].as_str())
            .collect()
    }

    /// `print` with the file read as `source`: where it is, and all it
    /// says.
    fn file(&self, print: Print, source: SourceId) -> Print {
        let said = self.prints.texts[source.index() as usize].get_or_init(|| {
            let sources = self.reading.renderer.sources();
            Print::default()
                .with((sources.path(source), sources.text(source)))
                .done()
        });
        print.with(said)
    }

    /// What the session accounts for by no key: every file read that is
    /// no page's, template's, data's or asset's, which holds the site's
    /// settings, its languages and its cards; which templates, data
    /// files, assets and languages the site has; what its cards are drawn
    /// with; which files a link publishes; and how it is asked to build.
    pub(super) fn whole(&self, plan: &Plan, options: &Options) -> u64 {
        let reading = self.reading;
        let renderer = &reading.renderer;
        let sources = renderer.sources();
        let site = renderer.site();
        let clock = self.prints.clock.as_ref();
        let templates: HashSet<SourceId> =
            (reading.template_files.iter().map(|(source, _)| *source)).collect();
        let mut print = Print::default();
        for id in sources.ids() {
            let path = sources.path(id);
            let keyed = ["content", "data", "assets", crate::fetched::FOLDER];
            // No output reads what the site published before.
            let unread = path == Path::new(crate::published::FILE);
            if !templates.contains(&id)
                && !unread
                && !keyed.iter().any(|folder| path.starts_with(folder))
            {
                print = self.file(print, id);
            }
        }
        print = print
            .with(&reading.templates)
            .with(&reading.data)
            .with(&reading.assets);
        for (_, language) in site.languages.iter() {
            print = print.with((&language.tag, &language.folder));
        }
        for file in renderer.card_files() {
            print = print
                .on_disk(stamp_of(renderer.open_card_file(&file)), clock)
                .with(file);
        }
        print
            .with(plan.linked.watches())
            .with(options.drafts)
            .done()
    }

    /// What `output` is built from besides what it reads.
    pub(super) fn own(&self, output: Output) -> u64 {
        let renderer = &self.reading.renderer;
        let site = renderer.site();
        let sources = renderer.sources();
        match output {
            Output::Page(page) => self.front(page),
            Output::Later(list, number) => {
                let page = site.lists.get(list).page;
                Print::default()
                    .with((self.list(list), number))
                    .with(page.map(|page| self.front(page)))
                    .done()
            }
            Output::Redirect(at) => {
                let redirect = &site.redirects[at];
                Print::default()
                    .shown(&(&redirect.from, &redirect.to))
                    .place(sources, Some(redirect.place))
                    .with(self.front(redirect.page))
                    .done()
            }
            Output::Feed(at) => {
                let feed = &renderer.feeds()[at];
                Print::default()
                    .shown(&(&feed.address, &feed.template))
                    .place(sources, feed.placed)
                    .with(self.list(feed.list))
                    .done()
            }
        }
    }

    /// Each of `keys`, read in the reading `was`, that this reading holds
    /// otherwise, `to` carrying `was`'s ids here. A setting or a language
    /// prints as nothing, the whole site's fingerprint covering it, so it
    /// is taken as changed where the file its words are in is.
    pub(super) fn changed(
        &self,
        was: &Taken<'_>,
        to: &Translation,
        keys: impl Iterator<Item = Key>,
    ) -> Vec<Key> {
        let reading = was.reading;
        let file_changed = |path: &str| {
            (reading.sources.iter().zip(&to.sources))
                .any(|((held, _), now)| held == Path::new(path) && now.is_none())
        };
        let changed = keys.filter(|&key| match key {
            Key::Setting(_) => file_changed(crate::config::FILE_NAME),
            Key::Language(id) => match &reading.renderer.dates().language(id).whence {
                crate::language::Whence::Site(path) => file_changed(path),
                _ => false,
            },
            _ => to
                .key(key)
                .is_none_or(|carried| was.of(key) != self.of(carried)),
        });
        changed.collect()
    }

    /// The fingerprint of what the reading holds under `key`.
    pub(super) fn of(&self, key: Key) -> u64 {
        let reading = self.reading;
        let prints = self.prints;
        let renderer = &reading.renderer;
        let site = renderer.site();
        let clock = prints.clock.as_ref();
        match key {
            Key::Field(page, _) | Key::Card(page) => self.front(page),
            Key::Body(page) => self.body(page),
            Key::Neighbours(page) => *prints.neighbours[page.index()].get_or_init(|| {
                let beside = site.neighbours.get(page.index());
                let named = |page: &Option<PageId>| page.map(|page| self.named(&[page]));
                Print::default()
                    .with(beside.map(|beside| beside.each_ref().map(named)))
                    .done()
            }),
            Key::Translations(page) => *prints.translations[page.index()].get_or_init(|| {
                let others = site.translations.get(page.index());
                Print::default()
                    .with(others.map(|others| self.named(others)))
                    .done()
            }),
            Key::ListPage(list, number) => {
                let stretches = prints.stretches[list.index()].get_or_init(|| {
                    let held = site.lists.get(list);
                    let stretch = |number| {
                        Print::default()
                            .with((self.list(list), self.named(held.stretch(number))))
                            .done()
                    };
                    // One past the last, which every page past it is.
                    (1..=held.stretches() + 1).map(stretch).collect()
                });
                let at = (number as usize).clamp(1, stretches.len()) - 1;
                stretches[at]
            }
            Key::ListHead(list) | Key::Members(list) => {
                *prints.heads[list.index()].get_or_init(|| {
                    let held = site.lists.get(list);
                    let (members, undated) = (self.named(&held.members), self.named(&held.undated));
                    Print::default()
                        .with((self.list(list), members, undated))
                        .done()
                })
            }
            Key::List(list) => self.list_own(list),
            Key::Pages => self.every_page(),
            Key::Spellings => self.spellings(),
            Key::Folders(language) => *prints.folders[language.index()].get_or_init(|| {
                let folders = site.folders(language);
                Print::default().with(self.lists_named(&folders)).done()
            }),
            Key::Home(language) => *prints.homes[language.index()].get_or_init(|| {
                let own = site
                    .own_list(language)
                    .map(|list| self.lists_named(&[list]));
                let home = site.home(language).map(|page| self.named(&[page]));
                Print::default().with((own, home)).done()
            }),
            Key::FieldOfEvery(name) => self.field_of_every(name),
            Key::FieldOfWriters(writing, wanted, name) => {
                self.field_of_writers(writing, wanted, name)
            }
            Key::Lists | Key::ListsIn(_) => self.every_list(),
            // Settings and languages are written in files the whole
            // site's fingerprint covers.
            Key::Setting(_) | Key::Language(_) => 0,
            Key::Template(id) => *prints.templates[id.index()].get_or_init(|| {
                let (source, builds) = reading.template_files[id.index()];
                let print = Print::default().with((&reading.templates[id.index()], builds));
                self.file(print, source).done()
            }),
            Key::Data(id) => *prints.data[id.index()].get_or_init(|| {
                let file = site.data.file(id);
                self.file(Print::default().with(&file.path), file.source)
                    .done()
            }),
            Key::Asset(id) => *prints.assets[id.index()].get_or_init(|| {
                let asset = (renderer.assets().iter())
                    .find(|(other, _)| *other == id)
                    .map(|(_, asset)| asset)
                    .expect("an asset of this reading");
                Print::default()
                    .with((&asset.name, &asset.published, &asset.source))
                    .shown(&asset.kind)
                    .with(asset.bytes())
                    .done()
            }),
            Key::Picture(id) => *prints.pictures[id.index()].get_or_init(|| {
                let picture = site.pictures.get(id);
                let mut print = Print::default()
                    .with((self.every_picture(), picture.shown(), &picture.address))
                    .with((picture.hash.as_bytes(), picture.travelling))
                    .shown(&(&picture.seen, &picture.copies, &picture.full));
                if let Some(said) = &picture.sidecar {
                    print = self.file(print, said.source);
                }
                print.done()
            }),
            Key::Pictures => self.every_picture(),
            Key::Beside(at) => *prints.beside[at as usize].get_or_init(|| {
                let file = &site.files[at as usize];
                let page = file.page.map(|page| self.named(&[page]));
                Print::default()
                    .with((self.every_file(), &file.path, page))
                    .shown(&file.address)
                    .on_disk(stamp_of(site.open_content(&file.path)), clock)
                    .done()
            }),
            Key::Static(at) => *prints.statics[at as usize].get_or_init(|| {
                let assets = renderer.assets();
                let file = &assets.statics()[at as usize];
                let opened = crate::platform::open_file(assets.real(), file.at());
                Print::default()
                    .with((self.every_file(), &file.name))
                    .on_disk(stamp_of(opened), clock)
                    .done()
            }),
            Key::Files => self.every_file(),
            Key::Fetched(key) => self.fetched(key),
            Key::Mentions(page) => *prints.mentions[page.index()]
                .get_or_init(|| Print::default().shown(&site.mentions.of(page)).done()),
            Key::Today => {
                *(prints.today).get_or_init(|| Print::default().shown(&renderer.today()).done())
            }
        }
    }

    /// Which pictures the site has, and where each is published.
    fn every_picture(&self) -> u64 {
        *self.prints.every_picture.get_or_init(|| {
            let site = self.reading.renderer.site();
            let mut print = Print::default().with(&self.reading.pictures);
            for (_, picture) in site.pictures.iter() {
                print = print.with(&picture.address);
            }
            print.done()
        })
    }

    /// What the reading holds of the file under `fetched/` that `key`
    /// stands for: the picture there, and its line in the record. Nothing
    /// for a file with neither.
    fn fetched(&self, key: u64) -> u64 {
        let by_key = self.prints.fetched.get_or_init(|| {
            let site = self.reading.renderer.site();
            let mut held: HashMap<u64, Vec<String>> = HashMap::new();
            let pictures = site.pictures.iter();
            for (_, picture) in pictures.filter(|(_, picture)| picture.root == Root::Fetched) {
                let said = format!("{} {}", picture.path, picture.hash);
                held.entry(crate::fetched::key(&picture.path))
                    .or_default()
                    .push(said);
            }
            for (path, line) in site.fetched.record.iter() {
                let said = format!("{path} {line:?}");
                held.entry(crate::fetched::key(path))
                    .or_default()
                    .push(said);
            }
            let prints = held.into_iter();
            prints
                .map(|(key, said)| (key, Print::default().with(said).done()))
                .collect()
        });
        by_key.get(&key).copied().unwrap_or_default()
    }

    /// Which files the site has beside the writing and in `static/`.
    fn every_file(&self) -> u64 {
        *self.prints.every_file.get_or_init(|| {
            let site = self.reading.renderer.site();
            let mut print = Print::default().with((&self.reading.beside, &self.reading.statics));
            for file in &site.files {
                print = print.shown(&file.address);
            }
            print.done()
        })
    }

    /// Everything a page says of itself before its writing: its words,
    /// taken from the text of every file one of them is written in, and
    /// what Statac made of them.
    fn front(&self, id: PageId) -> u64 {
        *self.prints.fronts[id.index()].get_or_init(|| {
            let reading = self.reading;
            let renderer = &reading.renderer;
            let site = renderer.site();
            let sources = renderer.sources();
            let page = site.pages.get(id);
            let list = |list: ListId| reading.lists[list.index()].as_str();
            let series = page.words.text(PageWord::Series).and_then(|series| {
                site.spelt(page.language, &ListKey::Series(series.value.to_owned()))
            });
            let tags = crate::lists::tags_of(site, id);
            let mut print = Print::default()
                .with((&reading.pages[id.index()], page.language.index()))
                .with((page.lands(), page.title.as_ref().map(|title| &title.value)))
                .shown(&(&page.address, page.date.as_ref().map(|date| &date.value)))
                .with((list(page.folder), page.speaks_for.map(list)))
                .with(tags.into_iter().map(list).collect::<Vec<_>>())
                .with(series.map(list));
            if let Some(made) = &page.made {
                print = print.place(sources, Some(made.head));
            }
            match page.source {
                // A page made from an item is all of the file it is in.
                Some(source) if page.made.is_some() => print = self.file(print, source),
                Some(source) => {
                    let text = sources.text(source);
                    let before = &text[..(page.body.start as usize).min(text.len())];
                    print = print.with((sources.path(source), before));
                }
                None => {}
            }
            let written = |origin: &Origin| origin.place.map(|place| place.source);
            let words = page.words.words.iter();
            let mut elsewhere: Vec<SourceId> = words
                .map(|(_, value)| &value.origin)
                .chain(page.words.fields.iter().map(|field| &field.origin))
                .chain(page.title.as_ref().map(|title| &title.origin))
                .chain(page.date.as_ref().map(|date| &date.origin))
                .filter_map(written)
                .filter(|source| Some(*source) != page.source)
                .collect();
            elsewhere.sort_unstable();
            elsewhere.dedup();
            for source in elsewhere {
                print = self.file(print, source);
            }
            print.done()
        })
    }

    /// A page's writing: what it says of itself, and all its file says.
    fn body(&self, id: PageId) -> u64 {
        *self.prints.bodies[id.index()].get_or_init(|| {
            let page = self.reading.renderer.site().pages.get(id);
            let mut print = Print::default()
                .with(self.front(id))
                .with((page.body.start, page.body.end));
            if let Some(source) = page.source {
                print = self.file(print, source);
            }
            print.done()
        })
    }

    /// What a list is, but for which pages it holds.
    fn list(&self, id: ListId) -> u64 {
        *self.prints.lists[id.index()].get_or_init(|| {
            let reading = self.reading;
            let renderer = &reading.renderer;
            let sources = renderer.sources();
            let list = renderer.site().lists.get(id);
            let page = list.page.map(|page| &reading.pages[page.index()]);
            let address = list.address.as_ref();
            let mut print = Print::default()
                .with((
                    &reading.lists[id.index()],
                    page,
                    list.numbered,
                    list.per_page,
                ))
                .shown(&(address.map(|address| &address.value), &list.later))
                .place(sources, address.and_then(|address| address.origin.place));
            if let Some(feed) = renderer.feed_of(id) {
                print = print
                    .shown(&(&feed.address, &feed.template))
                    .place(sources, feed.placed);
            }
            print.done()
        })
    }

    /// What a list is known by but for its members, as the build has it,
    /// its page by name. Which list it is the id carries.
    fn list_own(&self, id: ListId) -> u64 {
        *self.prints.lists_own[id.index()].get_or_init(|| {
            let known = self.reading.renderer.list_known(id);
            let page = (known.page).map(|(page, lands)| (self.named(&[page]), lands));
            Print::default()
                .with(page)
                .shown(&(known.feed, known.order))
                .done()
        })
    }

    /// Everything every page says of itself before its writing.
    fn every_front(&self) -> u64 {
        *self.prints.every_front.get_or_init(|| {
            let site = self.reading.renderer.site();
            let mut print = Print::default();
            for (id, _) in site.pages.iter() {
                print = print.with(self.front(id));
            }
            print.done()
        })
    }

    /// Which pages there are, in the reading's order: in which language
    /// each is, and whether and where it is published, a list's later
    /// pages among them.
    fn every_page(&self) -> u64 {
        *self.prints.every_page.get_or_init(|| {
            let reading = self.reading;
            let site = reading.renderer.site();
            let mut print = Print::default();
            for (id, page) in site.pages.iter() {
                let name = &reading.pages[id.index()];
                print = print
                    .with((name, page.language.index(), page.lands()))
                    .shown(&page.address);
            }
            for (id, list) in site.lists.iter() {
                if let Some(page) = list.page {
                    let named = (&reading.lists[id.index()], &reading.pages[page.index()]);
                    print = print.with(named).shown(&list.later);
                }
            }
            print.done()
        })
    }

    /// What every page answers to `name`, as a template looks it up or
    /// `where` asks it, with where each answer is written, and which pages
    /// there are to answer; for a name a page's template is chosen by,
    /// the template each is built with too. For a name a page answers with
    /// more than it says under that name, everything every page says of
    /// itself.
    fn field_of_every(&self, name: NameId) -> u64 {
        let slot = {
            let mut all = self.prints.fields_of_every.lock().expect("never poisoned");
            Arc::clone(all.entry(name).or_default())
        };
        *slot.get_or_init(|| {
            let renderer = &self.reading.renderer;
            let name = renderer.name_of(name);
            if !crate::templates::of_the_front(&name) {
                return self.every_front();
            }
            let asking = Answering::of(&name);
            let mut print = Print::default().with(self.every_page());
            for (id, _) in renderer.site().pages.iter() {
                print = self.answer(print, id, &asking).with(0u8);
            }
            print.done()
        })
    }

    /// What each page writing `writing`, or handed it, as the text
    /// `wanted` where there is one, answers to `name`, as
    /// [`Taken::field_of_every`] has it, in the order of their addresses,
    /// with where each is published and in which language: a list's later
    /// pages among them. Any other page is no part of it.
    fn field_of_writers(&self, writing: NameId, wanted: Option<NameId>, name: NameId) -> u64 {
        let slot = {
            let mut all = self
                .prints
                .fields_of_writers
                .lock()
                .expect("never poisoned");
            Arc::clone(all.entry((writing, wanted, name)).or_default())
        };
        *slot.get_or_init(|| {
            let reading = self.reading;
            let renderer = &reading.renderer;
            let (writing, name) = (renderer.name_of(writing), renderer.name_of(name));
            let wanted = wanted.map(|wanted| renderer.name_of(wanted));
            if !crate::templates::of_the_front(&name) {
                return self.every_front();
            }
            let site = renderer.site();
            let asking = Answering::of(&name);
            let mut print = Print::default();
            for &(id, number) in renderer.every_page() {
                let page = site.pages.get(id);
                if !crate::templates::writes(page, &writing, wanted.as_deref()) {
                    continue;
                }
                let address = match number {
                    1 => Some(&page.address),
                    _ => (page.speaks_for)
                        .and_then(|list| site.lists.get(list).later_address(number)),
                };
                print = (print.with((&reading.pages[id.index()], number, page.language.index())))
                    .shown(&address);
                print = self.answer(print, id, &asking).with(0u8);
            }
            print.done()
        })
    }

    /// `print` with what page `id` answers to the name `asking`, and where
    /// it is written.
    fn answer(&self, mut print: Print, id: PageId, asking: &Answering) -> Print {
        let renderer = &self.reading.renderer;
        let sources = renderer.sources();
        let page = renderer.site().pages.get(id);
        let words = page.words.words.iter();
        for (_, value) in words.filter(|(written, _)| Some(*written) == asking.word) {
            print = print.with(1u8).word(sources, value);
        }
        let fields = page.words.fields.iter();
        for field in fields.filter(|field| field.name == asking.name) {
            print = (print.with(2u8))
                .place(sources, Some(field.key))
                .node(sources, &field.value)
                .origin(sources, &field.origin);
        }
        if let Some(title) = page.title.as_ref().filter(|_| asking.titled) {
            print = print
                .with((3u8, &title.value))
                .origin(sources, &title.origin);
        }
        if let Some(date) = page.date.as_ref().filter(|_| asking.dated) {
            print = (print.with(4u8))
                .shown(&date.value)
                .origin(sources, &date.origin);
        }
        if asking.choosing {
            print = match renderer.layout_of(id) {
                Ok(Some(chosen)) => (print.with((5u8, &chosen.name, chosen.instead_of)))
                    .origin(sources, &chosen.origin),
                Ok(None) => print.with(7u8),
                Err(problem) => (print.with(6u8))
                    .shown(&problem.kind)
                    .place(sources, problem.place),
            };
        }
        print
    }

    /// Which list each spelling of a tag or a series leads to, by the
    /// list's name, which holds the spelling it is shown by.
    fn spellings(&self) -> u64 {
        *self.prints.spellings.get_or_init(|| {
            let reading = self.reading;
            let mut spelt: Vec<(usize, String, &str)> = (reading.renderer.site().spellings.iter())
                .map(|((language, key), list)| {
                    let list = reading.lists[list.index()].as_str();
                    (language.index(), format!("{key:?}"), list)
                })
                .collect();
            spelt.sort_unstable();
            Print::default().with(spelt).done()
        })
    }

    /// Which lists the site has, which spellings lead to each, and which
    /// pages it publishes where.
    fn every_list(&self) -> u64 {
        *self.prints.every_list.get_or_init(|| {
            let reading = self.reading;
            let site = reading.renderer.site();
            let mut print = Print::default().with(self.spellings());
            for (id, _) in site.lists.iter() {
                print = print.with(self.list(id));
            }
            for (id, page) in site.pages.iter() {
                print = print
                    .with((&reading.pages[id.index()], page.lands()))
                    .shown(&page.address);
            }
            print.done()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::Duration;

    /// What every page answers to each of `names`, the site at `root`
    /// having its first page written as `page`.
    fn fields_of_every(root: &Path, page: &str, names: &[&str]) -> Vec<u64> {
        fs::write(root.join("content/page-1.md"), page).unwrap();
        let (renderer, _) = crate::build::renderer(root, None);
        let reading = Reading::of(renderer);
        let prints = Prints::new(&reading, None);
        let taken = Taken {
            reading: &reading,
            at: root,
            prints: &prints,
        };
        let renderer = &reading.renderer;
        let names = names.iter();
        names
            .map(|name| taken.of(Key::FieldOfEvery(renderer.name_id(name))))
            .collect()
    }

    #[test]
    fn what_every_page_answers_to_one_name_turns_on_that_name_alone() {
        let root = std::env::temp_dir().join(format!("statac-print-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("content")).unwrap();
        fs::create_dir_all(root.join("templates")).unwrap();
        fs::write(root.join("statac.yaml"), "title: Site\n").unwrap();
        for template in ["page", "home"] {
            let path = root.join(format!("templates/{template}.html"));
            fs::write(path, "{{ page.content }}\n").unwrap();
        }
        let other = "---\ntitle: Page 2\nrank: 1\n---\nA page.\n";
        fs::write(root.join("content/page-2.md"), other).unwrap();
        // Each written where changing another leaves it.
        let page = |rank: u32, title: &str, more: &str, text: &str| {
            format!("---\nrank: {rank}\nmenu: true\ntitle: {title}\n{more}---\n{text}\n")
        };
        let names = ["rank", "menu", "title", "layout"];
        let print = |page: String| fields_of_every(&root, &page, &names);
        let was = print(page(1, "Page 1", "", "A page."));
        let ranked = print(page(2, "Page 1", "", "A page."));
        let titled = print(page(1, "Page one", "", "A page."));
        let written = print(page(1, "Page 1", "", "Another page."));
        // A `type` naming a template chooses it; one naming none doesn't.
        let typed = print(page(1, "Page 1", "type: home\n", "A page."));
        let untyped = print(page(1, "Page 1", "type: note\n", "A page."));
        let _ = fs::remove_dir_all(&root);
        assert_ne!(ranked[0], was[0]);
        assert_eq!(ranked[1..], was[1..]);
        assert_eq!(titled[..2], was[..2]);
        assert_ne!(titled[2], was[2]);
        assert_eq!(titled[3], was[3]);
        assert_eq!(written, was);
        assert_eq!(typed[..3], was[..3]);
        assert_ne!(typed[3], was[3]);
        assert_eq!(untyped, was);
    }

    #[test]
    fn a_file_changed_after_the_reading_began_is_held_to_no_print() {
        let root = std::env::temp_dir().join(format!("statac-print-disk-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("content")).unwrap();
        fs::create_dir_all(root.join("templates")).unwrap();
        fs::write(root.join("statac.yaml"), "title: Site\n").unwrap();
        for template in ["page", "home"] {
            let path = root.join(format!("templates/{template}.html"));
            fs::write(path, "{{ page.content }}\n").unwrap();
        }
        fs::write(root.join("content/page-1.md"), "A page.\n").unwrap();
        let note = root.join("content/note-1.txt");
        fs::write(&note, "one\n").unwrap();
        // What a reading begun once `clock` was read takes of the file.
        let print = |clock: Option<Clock>| {
            let (renderer, _) = crate::build::renderer(&root, None);
            let reading = Reading::of(renderer);
            let prints = Prints::new(&reading, clock);
            let taken = Taken {
                reading: &reading,
                at: &root,
                prints: &prints,
            };
            taken.of(Key::Beside(0))
        };
        // A clock its times are surely before, however coarse the disk's.
        let long_after =
            || Clock::of_site(&root).map(|clock| clock.later(Duration::from_secs(3600)));

        let began = Clock::of_site(&root);
        // Saved at its length once the reading began, before what the
        // reading read of it is taken.
        fs::write(&note, "two\n").unwrap();
        let during = print(began);
        let after = print(long_after());
        let again = print(long_after());
        let _ = fs::remove_dir_all(&root);
        assert!(began.is_some());
        assert_ne!(during, after);
        assert_eq!(after, again);
    }
}
