//! The site as arenas of ids. Language is a dimension of pages and lists from
//! the start, so a one-language site is the case of one language, not a
//! special path.

use std::marker::PhantomData;

use crate::content::address::Address;
use crate::content::words::Words;
use crate::date::Date;
use crate::origin::Sourced;
use crate::problem::{Held, Named, Word};
use crate::source::{Place, SourceId, Span};

pub trait Id: Copy {
    fn from_index(index: usize) -> Self;
    fn index(self) -> usize;
}

macro_rules! id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
        pub struct $name(u32);

        impl Id for $name {
            fn from_index(index: usize) -> Self {
                $name(u32::try_from(index).expect(concat!("too many ", stringify!($name))))
            }

            fn index(self) -> usize {
                self.0 as usize
            }
        }
    };
}

id!(LanguageId);
id!(PageId);
id!(ListId);
id!(TemplateId);
id!(DataId);
id!(AssetId);
id!(NameId);
id!(
    /// A picture file under `content/` or `static/`.
    PictureId
);
id!(
    /// Something the build writes: a page, one page of a list, a feed, a card.
    OutputId
);

/// Field and setting names, interned so a read can be recorded as two numbers.
#[derive(Default)]
pub struct Names {
    names: Vec<Box<str>>,
    ids: std::collections::HashMap<Box<str>, NameId>,
}

impl Names {
    pub fn intern(&mut self, name: &str) -> NameId {
        if let Some(&id) = self.ids.get(name) {
            return id;
        }
        let id = NameId::from_index(self.names.len());
        self.names.push(name.into());
        self.ids.insert(name.into(), id);
        id
    }

    pub fn get(&self, name: &str) -> Option<NameId> {
        self.ids.get(name).copied()
    }

    pub fn name(&self, id: NameId) -> &str {
        &self.names[id.index()]
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

pub struct Arena<I, T> {
    items: Vec<T>,
    id: PhantomData<I>,
}

impl<I: Id, T> Default for Arena<I, T> {
    fn default() -> Self {
        Arena {
            items: Vec::new(),
            id: PhantomData,
        }
    }
}

impl<I: Id, T> Arena<I, T> {
    pub fn push(&mut self, item: T) -> I {
        let id = I::from_index(self.items.len());
        self.items.push(item);
        id
    }

    pub fn get(&self, id: I) -> &T {
        &self.items[id.index()]
    }

    pub fn get_mut(&mut self, id: I) -> &mut T {
        &mut self.items[id.index()]
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (I, &T)> {
        self.items
            .iter()
            .enumerate()
            .map(|(i, item)| (I::from_index(i), item))
    }
}

pub struct Language {
    /// A BCP 47 tag as the author wrote it, such as `en-GB`; none where the
    /// site doesn't say.
    pub tag: Option<String>,
    /// Its folder at the top of `content/`, by its path there, on a site in
    /// several languages: `en`.
    pub folder: Option<String>,
}

pub struct Page {
    /// The file it is read from, or for a page made by `each:`, the data
    /// file or the page that made it; none for a list's page Statac made,
    /// which no file holds.
    pub source: Option<SourceId>,
    pub language: LanguageId,
    /// Its path under `content/`, names parted by `/`.
    pub path: String,
    pub place: PagePlace,
    /// The folder it is listed in.
    pub folder: ListId,
    /// Its own words, and those handed down to it.
    pub words: Words,
    pub title: Option<Sourced<String>>,
    pub date: Option<Sourced<Date>>,
    pub address: Address,
    /// The writing after any front matter.
    pub body: Span,
    /// The list the page speaks for: it is the list's page, its writing the
    /// list's introduction.
    pub speaks_for: Option<ListId>,
    /// Where the page was made by a page's `each:` rather than written: its
    /// `source` is then the data file, or for a post the page that made it.
    pub made: Option<Made>,
}

/// How a page was made by a page's `each:`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Made {
    /// The page whose `each:` made it.
    pub by: PageId,
    pub item: Item,
    /// Where the item is written: its name, or its first line; for a post,
    /// the `each:` line.
    pub head: Place,
}

/// What a page was made for: an item of a table, by its name or by its
/// place in a list from 1, or a post.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Item {
    Key(String),
    At(usize),
    Post {
        page: PageId,
        /// How a problem names the post.
        named: String,
    },
}

impl Item {
    /// How a problem names what was made from the file at `path`:
    /// `data/people.yaml: ada`, `data/people.yaml: item 3`, or
    /// `content/cards.md: content/post-1.md`.
    pub fn named(&self, path: &str) -> String {
        match self {
            Item::Key(key) => format!("{path}: {key}"),
            Item::At(at) => format!("{path}: item {at}"),
            Item::Post { named, .. } => format!("{path}: {named}"),
        }
    }
}

impl Page {
    /// How a problem names a page read from a file: by its file, or by the
    /// item it was made from. None for a list's page Statac made, which
    /// [`Site::named`] names by its list.
    pub fn file_named(&self, sources: &crate::source::Sources) -> Option<String> {
        let path = sources.path(self.source?).to_string_lossy();
        Some(match &self.made {
            Some(made) => made.item.named(&path),
            None => path.into_owned(),
        })
    }

    /// Where a problem with the whole page is shown: the first line of its
    /// file, or the item it was made from. None for a list's page Statac
    /// made, which [`Site::head`] places.
    pub fn head(&self, sources: &crate::source::Sources) -> Option<Place> {
        match &self.made {
            Some(made) => Some(made.head),
            None => Some(crate::content::words::head(sources, self.source?)),
        }
    }

    /// Whether the page makes a page per item of a table, by its own `each:`.
    pub fn makes_pages(&self) -> bool {
        !self.sends_on()
            && self
                .words
                .get(crate::content::words::PageWord::Each)
                .is_some_and(|each| each.origin.arrival == crate::origin::Arrival::Written)
    }

    /// Whether the page speaks for a list: a folder's own page, a page
    /// standing at a list's address, or one naming the posts it lists or
    /// making a page per item.
    pub fn speaks_for_a_list(&self) -> bool {
        use crate::content::words::PageWord;
        self.speaks_for.is_some()
            || matches!(self.place, PagePlace::Folder | PagePlace::Made)
            || [PageWord::Posts, PageWord::Each].into_iter().any(|word| {
                self.words
                    .get(word)
                    .is_some_and(|value| value.origin.arrival == crate::origin::Arrival::Written)
            })
    }

    /// Whether the page is published at its address: a page speaking for a
    /// list with `index_page: false` is published nowhere, and a page
    /// sending readers on leaves a redirect there instead.
    pub fn lands(&self) -> bool {
        !self.sends_on()
            && !(self.speaks_for_a_list()
                && self.words.truth(crate::content::words::PageWord::IndexPage) == Some(false))
    }

    /// Whether the page sends readers on with `redirect_to`, publishing
    /// a redirect at its address and nothing else.
    pub fn sends_on(&self) -> bool {
        self.words
            .text(crate::content::words::PageWord::RedirectTo)
            .is_some()
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum PagePlace {
    /// A page in a folder of pages.
    Ordinary,
    /// A folder's own page, its `index.md`.
    Folder,
    /// A folder that is one page: an `index.md` and the files that travel
    /// with it, listed in the folder above.
    Bundle,
    /// A list's page that Statac makes, where no page of the author's
    /// stands at its address.
    Made,
}

/// A file under `content/` that is no page, published beside the writing.
pub struct Travelling {
    /// Its path under `content/`.
    pub path: String,
    pub address: Address,
    /// The page it travels with, where its folder is one page.
    pub page: Option<PageId>,
}

/// A page left out as a draft or until its day: its address, the old
/// addresses its `redirect_from` names, why it is left out, and where what
/// leaves it out is written.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct HeldBack {
    pub address: Address,
    pub redirect_from: Vec<Address>,
    pub held: Held,
    pub place: Place,
}

/// An address that sends readers on: an old address of a page's, or the
/// address of a page with `redirect_to`.
pub struct Redirect {
    pub from: Address,
    /// The page that writes it: the page readers are sent to from its old
    /// address, or the page sending them on.
    pub page: PageId,
    pub to: SentTo,
    /// Where the old address, or the address sent to, is written.
    pub place: Place,
}

/// Where a redirect sends readers.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SentTo {
    /// The page itself, from one of its old addresses.
    Page,
    /// An address on the site, from its top, as written and as read, with
    /// the part from any `#`, which it keeps.
    Site {
        written: String,
        address: Address,
        fragment: String,
    },
    /// A full address elsewhere, as written.
    Elsewhere(String),
}

/// Folders, tags and series are all lists; paging, feeds and neighbours are
/// answered once, for lists.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ListKey {
    /// A folder, by its path under `content/`; the site's own list is the
    /// folder at the top.
    Folder(String),
    /// A tag, by the spelling it goes by where pages spell it more than one
    /// way.
    Tag(String),
    Series(String),
    /// A list a page declares with `posts:`, by that page.
    Declared(PageId),
}

impl ListKey {
    /// The word an author uses for the list, as `list.word` answers it.
    pub fn word(&self) -> &'static str {
        match self {
            ListKey::Folder(path) if path.is_empty() => "site",
            ListKey::Folder(_) => "folder",
            ListKey::Tag(_) => "tag",
            ListKey::Series(_) => "series",
            ListKey::Declared(_) => "chosen",
        }
    }
}

pub struct List {
    pub key: ListKey,
    pub language: LanguageId,
    /// Its posts, in its order. A folder ordered by hand holds every page
    /// in it, dated or not.
    pub members: Vec<PageId>,
    /// The pages it would hold were they dated, A to Z by file name; the
    /// pages made for posts in their posts' order.
    pub undated: Vec<PageId>,
    /// The page that speaks for the list, such as a folder's `index.md`.
    pub page: Option<PageId>,
    /// Where a tag's or a series' page stands, from `tags.permalink` or
    /// `series.permalink`, whether or not a page is made there.
    pub address: Option<Sourced<Address>>,
    /// Whether a folder's pages are ordered by the numbers their names
    /// open with.
    pub numbered: bool,
    /// Which way its posts run by their dates; none where the numbers or
    /// an `order:` list place them.
    pub order: Option<crate::lists::Order>,
    /// How many posts each of its pages shows; 0 shows them all on one.
    pub per_page: usize,
    /// Where its pages after the first stand, page 2 first: none where
    /// nothing is published at its address or its posts fit on one page.
    pub later: Vec<Address>,
}

impl List {
    pub fn new(key: ListKey, language: LanguageId) -> List {
        List {
            key,
            language,
            members: Vec::new(),
            undated: Vec::new(),
            page: None,
            address: None,
            numbered: false,
            order: None,
            per_page: 0,
            later: Vec::new(),
        }
    }

    /// How many stretches its posts are shown in, `per_page` at a time,
    /// one at least: where its page is published, one on each of its pages.
    pub fn stretches(&self) -> usize {
        match self.per_page {
            0 => 1,
            per => self.members.len().div_ceil(per).max(1),
        }
    }

    /// The posts shown on its page `number`, counted from 1.
    pub fn stretch(&self, number: usize) -> &[PageId] {
        match self.per_page {
            0 if number == 1 => &self.members,
            0 => &[],
            per => {
                let start = (number.saturating_sub(1) * per).min(self.members.len());
                let end = (start + per).min(self.members.len());
                &self.members[start..end]
            }
        }
    }

    /// Where its page `number` stands, counted from 2.
    pub fn later_address(&self, number: usize) -> Option<&Address> {
        self.later.get(number.checked_sub(2)?)
    }
}

#[derive(Default)]
pub struct Site {
    /// Its `content/` folder on disk.
    pub content: std::path::PathBuf,
    pub languages: Arena<LanguageId, Language>,
    pub pages: Arena<PageId, Page>,
    pub lists: Arena<ListId, List>,
    pub files: Vec<Travelling>,
    pub redirects: Vec<Redirect>,
    /// Each spelling of a tag or a series a page writes, by language, and
    /// the list it belongs to: spellings whose address parts come out alike
    /// are one list.
    pub spellings: std::collections::HashMap<(LanguageId, ListKey), ListId>,
    pub data: crate::data::Data,
    pub pictures: crate::album::Pictures,
    /// What Statac fetched into the site's `fetched/`.
    pub fetched: crate::fetched::Fetched,
    /// The mentions of each page kept in `fetched/mentions/`.
    pub mentions: crate::fetched::mentions::Mentions,
    /// Where each page left out as a draft or until its day would stand.
    pub held_back: Vec<HeldBack>,
    /// Each page's previous and next page, by page, in the run it walks.
    pub neighbours: Vec<[Option<PageId>; 2]>,
    /// Each post's place in its language's own list, by page.
    pub places: Vec<Option<usize>>,
    /// Each page's translations, by page, in the order the languages are
    /// declared; empty where no page has one.
    pub translations: Vec<Vec<PageId>>,
}

impl Site {
    /// The folder at the top of `language`'s pages, by its path under
    /// `content/`: its own folder, or for the main language with none,
    /// `content/` itself, the empty path.
    pub fn top(&self, language: LanguageId) -> &str {
        self.languages
            .get(language)
            .folder
            .as_deref()
            .unwrap_or_default()
    }

    /// The top that the folder at `path` under `content/` stands beneath:
    /// the language folder it is in, or else `content/` itself. Folder
    /// lists, and the addresses pages write, stop there.
    pub fn top_above(&self, path: &str) -> &str {
        self.languages
            .iter()
            .filter_map(|(_, language)| language.folder.as_deref())
            .find(|folder| {
                path.strip_prefix(folder)
                    .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
            })
            .unwrap_or_default()
    }

    /// A language's own list: the folder list at its top.
    pub fn own_list(&self, language: LanguageId) -> Option<ListId> {
        let top = self.top(language);
        self.lists
            .iter()
            .find(|(_, list)| {
                list.language == language
                    && matches!(&list.key, ListKey::Folder(path) if path == top)
            })
            .map(|(id, _)| id)
    }

    /// Whether `list` is a home's: a language's own list, or the folder at
    /// the top of `content/`.
    pub fn is_home(&self, list: ListId) -> bool {
        let list = self.lists.get(list);
        match &list.key {
            ListKey::Folder(path) => path.is_empty() || path == self.top(list.language),
            _ => false,
        }
    }

    /// Whether `list` is a folder at the top of a language's pages, or one
    /// directly inside it: one of the lists with a page and a feed where
    /// the author writes none. The folder at the top of `content/` is one
    /// only where it is a language's top.
    pub fn outermost(&self, list: ListId) -> bool {
        let list = self.lists.get(list);
        let ListKey::Folder(path) = &list.key else {
            return false;
        };
        if path == self.top(list.language) {
            return true;
        }
        let parent = path.rsplit_once('/').map(|(parent, _)| parent);
        !path.is_empty() && parent.unwrap_or_default() == self.top_above(path)
    }

    /// The lists of one kind, one per language.
    pub fn lists_for<'a>(&'a self, key: &'a ListKey) -> impl Iterator<Item = (ListId, &'a List)> {
        self.lists.iter().filter(move |(_, list)| &list.key == key)
    }

    /// The list a tag or a series spelt as `key` belongs to in `language`.
    pub fn spelt(&self, language: LanguageId, key: &ListKey) -> Option<ListId> {
        self.spellings.get(&(language, key.clone())).copied()
    }

    /// How a problem names `page`: by its file or the item it was made
    /// from, or a list's page Statac made by its list.
    pub fn named(&self, page: PageId, sources: &crate::source::Sources) -> Named {
        let p = self.pages.get(page);
        if let Some(named) = p.file_named(sources) {
            return Named::File(named);
        }
        let list = p.speaks_for.expect("a page Statac made speaks for a list");
        let (word, name) = match &self.lists.get(list).key {
            ListKey::Folder(path) => (Word::Folder, path.clone()),
            ListKey::Tag(name) => (Word::Tag, name.clone()),
            ListKey::Series(name) => (Word::Series, name.clone()),
            ListKey::Declared(_) => unreachable!("a page declaring a list is the author's"),
        };
        Named::ListPage(word, name)
    }

    /// Where a problem with the whole of `page` is shown: the first line
    /// of its file or the item it was made from, or for a list's page
    /// Statac made, the setting placing it where the site writes one.
    pub fn head(&self, page: PageId, sources: &crate::source::Sources) -> Option<Place> {
        let p = self.pages.get(page);
        match p.source {
            Some(_) => p.head(sources),
            None => p
                .speaks_for
                .and_then(|list| self.lists.get(list).address.as_ref())
                .and_then(|address| address.origin.place),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::Sources;

    fn page(source: SourceId, language: LanguageId, path: &str) -> Page {
        Page {
            source: Some(source),
            language,
            path: path.into(),
            place: PagePlace::Ordinary,
            folder: ListId::from_index(0),
            words: Words::default(),
            title: None,
            date: None,
            address: Address::read(path).unwrap(),
            body: Span::new(0, 0),
            speaks_for: None,
            made: None,
        }
    }

    #[test]
    fn a_tag_is_one_list_per_language() {
        let mut sources = Sources::default();
        let mut site = Site::default();
        let en = site.languages.push(Language {
            tag: Some("en".into()),
            folder: None,
        });
        let ar = site.languages.push(Language {
            tag: Some("ar".into()),
            folder: Some("ar".into()),
        });
        let one = site.pages.push(page(
            sources.add("content/post-1.md", String::new()),
            en,
            "/post-1/",
        ));
        let two = site.pages.push(page(
            sources.add("content/ar/post-1.md", String::new()),
            ar,
            "/ar/post-1/",
        ));
        let rust = ListKey::Tag("Rust".into());
        site.lists.push(List {
            members: vec![one],
            ..List::new(rust.clone(), en)
        });
        site.lists.push(List {
            members: vec![two],
            ..List::new(rust.clone(), ar)
        });
        let languages: Vec<_> = site
            .lists_for(&rust)
            .map(|(_, list)| site.languages.get(list.language).tag.as_deref())
            .collect();
        assert_eq!(languages, [Some("en"), Some("ar")]);
    }

    #[test]
    fn ids_index_their_own_arena() {
        let mut arena: Arena<PageId, &str> = Arena::default();
        let a = arena.push("post-1");
        let b = arena.push("post-2");
        assert_eq!(*arena.get(b), "post-2");
        assert_eq!(arena.iter().map(|(id, _)| id).collect::<Vec<_>>(), [a, b]);
    }
}
