//! What each output read while it was built.
//!
//! The record decides what to publish and what serve rebuilds, so it must be
//! complete: everything an output can see of the site goes through a reader
//! that records the key first. A read that skipped it would leave serve
//! showing a stale page; the test that serve's output equals a full build is
//! what catches one.

use std::collections::HashMap;

use crate::site::{
    AssetId, DataId, LanguageId, ListId, NameId, OutputId, PageId, PictureId, TemplateId,
};
use crate::source::{Place, SourceId};

/// One thing an output read.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum Key {
    /// A field of a page, by name: `title`, `date`, or the author's own.
    Field(PageId, NameId),
    /// What every page writes under one name in its front matter, and
    /// which pages there are to write it.
    FieldOfEvery(NameId),
    /// A page's body as built.
    Body(PageId),
    /// The previous and next page in each of the page's lists.
    Neighbours(PageId),
    /// Which pages are the page's translations.
    Translations(PageId),
    /// A page's sharing card. Reading it announces the card, so it is drawn.
    Card(PageId),
    /// The members on one page of a list (counted from 1), and whether another follows.
    ListPage(ListId, u32),
    /// The newest members of a list, as its feed carries them.
    ListHead(ListId),
    /// Every member of a list, dated or not.
    Members(ListId),
    /// Which lists exist, and their sizes.
    Lists,
    /// Which of one language's lists exist, and their sizes.
    ListsIn(LanguageId),
    Setting(NameId),
    /// A language: its tag as `language` declares it, and its words, from
    /// the site's file in `languages/` or Statac's own.
    Language(LanguageId),
    Template(TemplateId),
    Data(DataId),
    Asset(AssetId),
    /// A picture: what its header and the file describing it say, and its
    /// copies.
    Picture(PictureId),
    /// Which picture files the site has, as a name that reached none reads.
    Pictures,
    /// A file beside the writing, by its place among them: where it is
    /// published, its size and what it holds.
    Beside(u32),
    /// A file in `static/`, by its place among them.
    Static(u32),
    /// Which files the site has beside the writing and in `static/`, as a
    /// name that reached none reads.
    Files,
    /// A file under `fetched/`, by [`crate::fetched::key`] of its path
    /// there, as a read that found none reads: whether it is there now,
    /// and what the record says of it.
    Fetched(u64),
    /// The mentions of a page kept in `fetched/mentions/`, each as its
    /// file says it.
    Mentions(PageId),
    /// The build's date. Reading it is what makes an output change from one day to the next.
    Today,
}

/// The ids of one reading of a site as a later reading of it has them. An
/// id is an index into one reading's arenas, so what an earlier reading
/// recorded is carried over by what each id named there; none where the
/// later reading has nothing of that name, or holds it changed.
#[derive(Default)]
pub struct Translation {
    /// A file read is the same file only where its text is the same, so a
    /// place in it is the same place.
    pub sources: Vec<Option<SourceId>>,
    pub pages: Vec<Option<PageId>>,
    pub lists: Vec<Option<ListId>>,
    pub names: Vec<NameId>,
    pub templates: Vec<Option<TemplateId>>,
    pub data: Vec<Option<DataId>>,
    pub assets: Vec<Option<AssetId>>,
    pub pictures: Vec<Option<PictureId>>,
    pub beside: Vec<Option<u32>>,
    pub statics: Vec<Option<u32>>,
    /// The old addresses that send readers on, and the feeds, each by its
    /// place among them.
    pub redirects: Vec<Option<usize>>,
    pub feeds: Vec<Option<usize>>,
}

fn carried<I: crate::site::Id>(over: &[Option<I>], id: I) -> Option<I> {
    over.get(id.index()).copied().flatten()
}

impl Translation {
    pub fn page(&self, id: PageId) -> Option<PageId> {
        carried(&self.pages, id)
    }

    pub fn list(&self, id: ListId) -> Option<ListId> {
        carried(&self.lists, id)
    }

    pub fn template(&self, id: TemplateId) -> Option<TemplateId> {
        carried(&self.templates, id)
    }

    pub fn picture(&self, id: PictureId) -> Option<PictureId> {
        carried(&self.pictures, id)
    }

    pub fn place(&self, place: Place) -> Option<Place> {
        let source = self.sources.get(place.source.index() as usize)?;
        Some(Place {
            source: (*source)?,
            span: place.span,
        })
    }

    /// `key` as the later reading names it.
    pub fn key(&self, key: Key) -> Option<Key> {
        use crate::site::Id;
        let at = |over: &[Option<u32>], at: u32| over.get(at as usize).copied().flatten();
        Some(match key {
            Key::Field(page, name) => Key::Field(self.page(page)?, *self.names.get(name.index())?),
            Key::Body(page) => Key::Body(self.page(page)?),
            Key::Neighbours(page) => Key::Neighbours(self.page(page)?),
            Key::Translations(page) => Key::Translations(self.page(page)?),
            Key::Card(page) => Key::Card(self.page(page)?),
            Key::Mentions(page) => Key::Mentions(self.page(page)?),
            Key::ListPage(list, number) => Key::ListPage(self.list(list)?, number),
            Key::ListHead(list) => Key::ListHead(self.list(list)?),
            Key::Members(list) => Key::Members(self.list(list)?),
            Key::FieldOfEvery(name) => Key::FieldOfEvery(*self.names.get(name.index())?),
            Key::Setting(name) => Key::Setting(*self.names.get(name.index())?),
            Key::Template(id) => Key::Template(self.template(id)?),
            Key::Data(id) => Key::Data(carried(&self.data, id)?),
            Key::Asset(id) => Key::Asset(carried(&self.assets, id)?),
            Key::Picture(id) => Key::Picture(self.picture(id)?),
            Key::Beside(file) => Key::Beside(at(&self.beside, file)?),
            Key::Static(file) => Key::Static(at(&self.statics, file)?),
            // The languages a site declares are part of what makes a
            // later reading comparable at all.
            Key::Lists | Key::ListsIn(_) | Key::Language(_) => key,
            Key::Pictures | Key::Files | Key::Fetched(_) | Key::Today => key,
        })
    }
}

/// The reads of one output, in the order they happened.
///
/// An output that reads fields of page after page, as a loop over every
/// page does, would hold a read for each on a site of any size, and every
/// output built so would. Past [`MANY`] of them it holds one read for each
/// name instead, of that field on every page, which covers the rest.
#[derive(Default, Clone, Debug)]
pub struct Reads {
    keys: Vec<Key>,
    /// How many reads of a page's field it holds.
    fields: usize,
    /// The names it holds a read of on every page for, having held
    /// [`MANY`] reads of fields.
    every: Vec<NameId>,
}

/// How many reads of a page's field an output holds one by one.
pub const MANY: usize = 1024;

impl Reads {
    /// The reads `keys`, in that order.
    pub fn of(keys: Vec<Key>) -> Reads {
        let mut reads = Reads::default();
        for key in keys {
            reads.read(key);
        }
        reads
    }

    #[inline]
    pub fn read(&mut self, key: Key) {
        // The read before it over again, as a loop asking each page the
        // same thing of the site makes.
        if self.keys.last() == Some(&key) {
            return;
        }
        if let Key::Field(_, name) = key {
            if self.fields >= MANY {
                if !self.every.contains(&name) {
                    self.every.push(name);
                    self.keys.push(Key::FieldOfEvery(name));
                }
                return;
            }
            self.fields += 1;
        }
        self.keys.push(key);
    }

    pub fn keys(&self) -> &[Key] {
        &self.keys
    }

    /// The cards this output announced.
    pub fn cards(&self) -> impl Iterator<Item = PageId> + '_ {
        self.keys.iter().filter_map(|key| match key {
            Key::Card(page) => Some(*page),
            _ => None,
        })
    }
}

/// Every output's reads, and the reverse: who read a key.
#[derive(Default)]
pub struct Record {
    reads: HashMap<OutputId, Reads>,
    readers: HashMap<Key, Vec<OutputId>>,
}

impl Record {
    /// Replaces what `output` read last time.
    pub fn set(&mut self, output: OutputId, reads: Reads) {
        self.forget(output);
        let mut keys = reads.keys.clone();
        keys.sort_unstable();
        keys.dedup();
        for key in keys {
            self.readers.entry(key).or_default().push(output);
        }
        self.reads.insert(output, reads);
    }

    pub fn forget(&mut self, output: OutputId) {
        let Some(old) = self.reads.remove(&output) else {
            return;
        };
        for key in old.keys {
            if let Some(readers) = self.readers.get_mut(&key) {
                readers.retain(|&o| o != output);
                if readers.is_empty() {
                    self.readers.remove(&key);
                }
            }
        }
    }

    pub fn reads(&self, output: OutputId) -> Option<&Reads> {
        self.reads.get(&output)
    }

    /// Every key some output read, each once.
    pub fn keys(&self) -> impl Iterator<Item = Key> + '_ {
        self.readers.keys().copied()
    }

    /// The outputs that read any of `changed`, each once, in id order.
    pub fn affected(&self, changed: &[Key]) -> Vec<OutputId> {
        let mut outputs: Vec<OutputId> = changed
            .iter()
            .filter_map(|key| self.readers.get(key))
            .flatten()
            .copied()
            .collect();
        outputs.sort_unstable();
        outputs.dedup();
        outputs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::site::{Id, Names};

    fn output(i: usize) -> OutputId {
        OutputId::from_index(i)
    }

    fn page(i: usize) -> PageId {
        PageId::from_index(i)
    }

    #[test]
    fn an_edited_body_reaches_only_what_read_it() {
        let mut names = Names::default();
        let title = names.intern("title");
        let mut record = Record::default();

        let mut post = Reads::default();
        post.read(Key::Field(page(0), title));
        post.read(Key::Body(page(0)));
        record.set(output(0), post);

        let mut list = Reads::default();
        list.read(Key::ListPage(ListId::from_index(0), 1));
        list.read(Key::Field(page(0), title));
        record.set(output(1), list);

        assert_eq!(record.affected(&[Key::Body(page(0))]), [output(0)]);
        assert_eq!(
            record.affected(&[Key::Field(page(0), title)]),
            [output(0), output(1)]
        );
    }

    #[test]
    fn a_rebuilt_output_forgets_what_it_no_longer_reads() {
        let mut record = Record::default();
        let mut before = Reads::default();
        before.read(Key::Today);
        record.set(output(0), before);
        record.set(output(0), Reads::default());
        assert!(record.affected(&[Key::Today]).is_empty());
    }

    #[test]
    fn reading_a_key_twice_affects_the_output_once() {
        let mut record = Record::default();
        let mut reads = Reads::default();
        reads.read(Key::Lists);
        reads.read(Key::Lists);
        record.set(output(3), reads);
        assert_eq!(record.affected(&[Key::Lists, Key::Lists]), [output(3)]);
    }

    #[test]
    fn many_reads_of_fields_are_held_as_one_for_each_name() {
        let mut names = Names::default();
        let (title, date) = (names.intern("title"), names.intern("date"));
        let mut reads = Reads::default();
        for i in 0..MANY {
            reads.read(Key::Field(page(i), title));
        }
        assert_eq!(reads.keys().len(), MANY);
        for i in 0..MANY {
            reads.read(Key::Field(page(MANY + i), title));
            reads.read(Key::Field(page(i), date));
            reads.read(Key::Body(page(i)));
        }
        let held = &reads.keys()[MANY..];
        assert_eq!(
            held[..3],
            [
                Key::FieldOfEvery(title),
                Key::FieldOfEvery(date),
                Key::Body(page(0))
            ]
        );
        assert_eq!(held.len(), 2 + MANY);
        // Carried to a later reading, they are held the same.
        assert_eq!(Reads::of(reads.keys().to_vec()).keys(), reads.keys());
    }

    #[test]
    fn cards_read_are_cards_announced() {
        let mut reads = Reads::default();
        reads.read(Key::Body(page(0)));
        reads.read(Key::Card(page(2)));
        assert_eq!(reads.cards().collect::<Vec<_>>(), [page(2)]);
    }
}
