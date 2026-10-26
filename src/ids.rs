//! The ids the site's arenas are keyed by, the arenas, and field and
//! setting names interned as ids.

use std::marker::PhantomData;

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
pub(crate) struct Names {
    names: Vec<Box<str>>,
    ids: std::collections::HashMap<Box<str>, NameId>,
}

impl Names {
    pub(crate) fn intern(&mut self, name: &str) -> NameId {
        if let Some(&id) = self.ids.get(name) {
            return id;
        }
        let id = NameId::from_index(self.names.len());
        self.names.push(name.into());
        self.ids.insert(name.into(), id);
        id
    }

    pub(crate) fn get(&self, name: &str) -> Option<NameId> {
        self.ids.get(name).copied()
    }

    pub(crate) fn name(&self, id: NameId) -> &str {
        &self.names[id.index()]
    }

    pub(crate) fn len(&self) -> usize {
        self.names.len()
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
    pub(crate) fn push(&mut self, item: T) -> I {
        let id = I::from_index(self.items.len());
        self.items.push(item);
        id
    }

    pub(crate) fn get(&self, id: I) -> &T {
        &self.items[id.index()]
    }

    pub(crate) fn get_mut(&mut self, id: I) -> &mut T {
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
