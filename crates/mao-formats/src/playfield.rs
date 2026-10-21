//! Playfields: the zones of the world. The resource database has one
//! `0xf4241` object per playfield (the instance is the playfield's number)
//! whose data is a version (10), the number again and the name as a
//! NUL-terminated string.

use crate::resource::{Id, ResourceDatabase};
use crate::texture::OBJECT_HEAD;

/// A playfield's number and name.
#[derive(Clone, Debug)]
pub struct Playfield {
    pub number: u32,
    pub name: String,
}

/// The type of a playfield's definition object.
pub const KIND: u32 = 0xf4241;

/// A playfield's name from its definition object's bytes.
pub fn name(object: &[u8]) -> Option<String> {
    let text = object.get(OBJECT_HEAD + 8..)?;
    let end = text.iter().position(|&b| b == 0).unwrap_or(text.len());
    Some(String::from_utf8_lossy(&text[..end]).into_owned())
}

/// Every playfield, by number.
pub fn all(db: &ResourceDatabase) -> Vec<Playfield> {
    let Ok(ids) = db.ids() else { return Vec::new() };
    ids.filter_map(Result::ok)
        .filter(|id| id.kind == KIND)
        .filter_map(|id| {
            let object = db.get(Id { kind: KIND, instance: id.instance }).ok()??;
            Some(Playfield { number: id.instance, name: name(&object)? })
        })
        .collect()
}
