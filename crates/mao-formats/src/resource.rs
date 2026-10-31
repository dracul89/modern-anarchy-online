//! The resource database: objects keyed by type and instance.
//!
//! Each record is the type and instance words, a length, and the object's
//! data; [`ResourceDatabase::get`] returns that data (the object's own
//! stream, which starts with its type, instance and revision).

use std::path::Path;

use crate::ctree::{self, Database, Result};

/// `<game>/cd_image/data/db/ResourceDatabase`.
pub struct ResourceDatabase {
    db: Database,
}

/// A key: an object's type and instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id {
    pub kind: u32,
    pub instance: u32,
}

impl ResourceDatabase {
    /// Opens the database of the game installed at `game`.
    pub fn open_game(game: &Path) -> Result<ResourceDatabase> {
        ResourceDatabase::open(&crate::files::find(&game.join("cd_image/data/db")).join("ResourceDatabase"))
    }

    /// Opens `<base>.dat` and `<base>.idx`.
    pub fn open(base: &Path) -> Result<ResourceDatabase> {
        Ok(ResourceDatabase { db: Database::open(base)? })
    }

    /// The number of objects.
    pub fn len(&self) -> u32 {
        self.db.len()
    }

    pub fn is_empty(&self) -> bool {
        self.db.is_empty()
    }

    /// The object's data, if there is one.
    pub fn get(&self, id: Id) -> Result<Option<Vec<u8>>> {
        match self.db.record(&ctree::key(id.kind, id.instance)) {
            Ok(mut record) => {
                // The type and instance words and a length come first.
                record.drain(..12.min(record.len()));
                Ok(Some(record))
            }
            Err(ctree::Error::NotFound) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Checks every object's record header against the index: the number
    /// of objects, the bytes of data they hold, the highest position seen,
    /// how many failed, and the first few of those (id, position, why).
    pub fn verify(&self) -> Result<(u64, u64, u64, u64, Vec<(Id, u64, String)>)> {
        let (mut count, mut bytes, mut highest, mut failed, mut failures) = (0, 0, 0, Vec::new(), 0u64);
        for entry in self.db.entries()? {
            let (key, position) = entry?;
            match self.db.check(&key, position) {
                Ok(used) => bytes += used as u64,
                Err(e) => {
                    let (kind, instance) = ctree::unkey(&key);
                    failures += 1;
                    if failed.len() < 8 {
                        failed.push((Id { kind, instance }, position, e.to_string()));
                    }
                }
            }
            highest = highest.max(position);
            count += 1;
        }
        Ok((count, bytes, highest, failures, failed))
    }

    /// Every object's id, in order.
    pub fn ids(&self) -> Result<impl Iterator<Item = Result<Id>> + '_> {
        Ok(self.db.entries()?.map(|entry| {
            entry.map(|(key, _)| {
                let (kind, instance) = ctree::unkey(&key);
                Id { kind, instance }
            })
        }))
    }
}
