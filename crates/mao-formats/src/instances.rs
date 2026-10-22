//! The instance index: the database's one `0xf424a` object, which the
//! original hands to its `InstanceManager` so that things can be found by
//! file name: `GetTypeInstance(type, "solitus_male.cir")`.
//!
//! Format (little-endian), after the 12-byte head: an `i32` count of types
//! and, per type, its type `i32`, a count `i32` and that many entries, each
//! an instance `i32`, a length `i32` (counting the NUL) and the name.
//! (The original keys names by `h * 37 + c`; here the names are kept.)

use std::collections::HashMap;

use crate::texture::OBJECT_HEAD;

#[derive(Debug, Default)]
pub struct Instances {
    by_name: HashMap<(u32, String), u32>,
}

#[derive(Debug)]
pub struct Truncated;

impl std::fmt::Display for Truncated {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "the instance index is truncated")
    }
}

impl std::error::Error for Truncated {}

impl Instances {
    pub fn parse(object: &[u8]) -> Result<Instances, Truncated> {
        let mut at = OBJECT_HEAD;
        let int = |at: &mut usize| -> Result<u32, Truncated> {
            let b = object.get(*at..*at + 4).ok_or(Truncated)?;
            *at += 4;
            Ok(u32::from_le_bytes(b.try_into().unwrap()))
        };
        let mut by_name = HashMap::new();
        for _ in 0..int(&mut at)? {
            let kind = int(&mut at)?;
            for _ in 0..int(&mut at)? {
                let instance = int(&mut at)?;
                let length = int(&mut at)? as usize;
                let text = object.get(at..at + length).ok_or(Truncated)?;
                at += length;
                by_name.insert((kind, String::from_utf8_lossy(text).trim_end_matches('\0').to_string()), instance);
            }
        }
        Ok(Instances { by_name })
    }

    /// The instance of the object of `kind` with this file name (the
    /// original's lookup is case-sensitive).
    pub fn find(&self, kind: u32, name: &str) -> Option<u32> {
        self.by_name.get(&(kind, name.to_string())).copied()
    }

    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }

    /// Every (type, name, instance), for exploration.
    pub fn iter(&self) -> impl Iterator<Item = (u32, &str, u32)> + '_ {
        self.by_name.iter().map(|((k, n), &i)| (*k, n.as_str(), i))
    }
}
