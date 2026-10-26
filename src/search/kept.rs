//! What search made of a site's pages in one build, kept for the next
//! build to take in place of making it again, where it would be made
//! from the same.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::{File, Record, Weighed};
use crate::ids::PageId;

/// What search made in one build of a site, kept for the next, each by
/// everything it is made from. The default keeps nothing, and everything
/// is made anew.
#[derive(Default)]
pub(crate) struct Kept {
    keeps: bool,
    /// Each page's records weighed, by everything weighing them reads.
    weighed: HashMap<blake3::Hash, Arc<[Weighed]>>,
    /// Each file packed, by its JSON and where it is put.
    packed: HashMap<blake3::Hash, File>,
    /// The records of each page whose writing is known to be as it was
    /// when they were cut from it, for the next build to take.
    standing: HashMap<PageId, Arc<[Record]>>,
    /// The records of each page the last build searched.
    cut: HashMap<PageId, Arc<[Record]>>,
    /// What this build took or made, which it keeps in place of all kept
    /// before once it ends.
    taken: Mutex<Taken>,
}

#[derive(Default)]
struct Taken {
    weighed: HashMap<blake3::Hash, Arc<[Weighed]>>,
    packed: HashMap<blake3::Hash, File>,
}

impl Kept {
    /// Kept search that keeps what each build made for the next.
    pub(crate) fn keeping() -> Kept {
        Kept {
            keeps: true,
            ..Kept::default()
        }
    }

    pub(super) fn keeps(&self) -> bool {
        self.keeps
    }

    /// Gives the next build `standing`, the records of each page, by its
    /// id in that build, whose writing is known to be as it was when they
    /// were cut from it, in place of any given before.
    pub(crate) fn stand(&mut self, standing: HashMap<PageId, Arc<[Record]>>) {
        self.standing = standing;
    }

    /// The records given the build about to begin of each page whose
    /// writing stands, taken.
    pub(super) fn standing(&mut self) -> HashMap<PageId, Arc<[Record]>> {
        std::mem::take(&mut self.standing)
    }

    /// The records of each page the last build searched, by its id there,
    /// taken; none where it kept nothing, or made no index since they were
    /// last taken.
    pub(crate) fn cut(&mut self) -> HashMap<PageId, Arc<[Record]>> {
        std::mem::take(&mut self.cut)
    }

    /// One page's records weighed: as kept, where they were weighed from
    /// what `from` gives a hasher, or else as `weigh` weighs them.
    pub(super) fn weighed(
        &self,
        from: impl FnOnce(&mut blake3::Hasher),
        weigh: impl FnOnce() -> Vec<Weighed>,
    ) -> Arc<[Weighed]> {
        if !self.keeps {
            return weigh().into();
        }
        let mut hasher = blake3::Hasher::new();
        from(&mut hasher);
        let key = hasher.finalize();
        let weighed = match self.weighed.get(&key) {
            Some(weighed) => Arc::clone(weighed),
            None => weigh().into(),
        };
        let mut taken = self.taken.lock().expect("never poisoned");
        taken.weighed.insert(key, Arc::clone(&weighed));
        weighed
    }

    /// The file of `json` put at `before`: as kept, where the same was put
    /// there, or else as `pack` packs it.
    pub(super) fn packed(&self, before: &str, json: &str, pack: impl FnOnce() -> File) -> File {
        if !self.keeps {
            return pack();
        }
        let mut hasher = blake3::Hasher::new();
        hashed(&mut hasher, before);
        hashed(&mut hasher, json);
        let key = hasher.finalize();
        let file = match self.packed.get(&key) {
            Some(file) => file.clone(),
            None => pack(),
        };
        let mut taken = self.taken.lock().expect("never poisoned");
        taken.packed.insert(key, file.clone());
        file
    }

    /// Ends a build that searched the pages `cut` holds the records of,
    /// keeping what it took or made for the next in place of all kept
    /// before.
    pub(super) fn end(&mut self, cut: HashMap<PageId, Arc<[Record]>>) {
        let taken = std::mem::take(self.taken.get_mut().expect("never poisoned"));
        self.weighed = taken.weighed;
        self.packed = taken.packed;
        self.cut = cut;
    }
}

/// Feeds `hasher` `text`, after its length, so that no two runs of texts
/// feed it the same.
pub(super) fn hashed(hasher: &mut blake3::Hasher, text: &str) {
    hasher.update(&(text.len() as u64).to_le_bytes());
    hasher.update(text.as_bytes());
}
