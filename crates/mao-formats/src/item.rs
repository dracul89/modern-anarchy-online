//! Item templates: the objects of type 0xf4254, one per item id (the low or
//! high id of an inventory entry). Little-endian words: the type, the
//! instance, 1000000, the identity type (0xc74e), then (stat, value) pairs,
//! then a word holding the name's length (low half) and the description's
//! (high half), then the two strings. Among the stats: 79 is the icon (a
//! 48x48 texture of type 0xf6958, by instance) and 12 the item's mesh (type
//! 0xf6952).

/// The object type of an item template.
pub const TEMPLATE: u32 = 0xf4254;
/// The texture type of an item's icon.
pub const ICON_TYPE: u32 = 0xf6958;

/// The stat keys this client reads.
pub mod stat {
    pub const MESH: u32 = 12;
    pub const ICON: u32 = 79;
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Item {
    pub id: u32,
    pub name: String,
    pub description: String,
    /// (stat, value) pairs as stored.
    pub stats: Vec<(u32, u32)>,
}

impl Item {
    pub fn parse(data: &[u8]) -> Option<Item> {
        let words: Vec<u32> = data.chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect();
        if words.len() < 6 || words[0] != TEMPLATE {
            return None;
        }
        let printable = |bytes: &[u8]| bytes.iter().all(|&b| b >= 0x20 && b != 0x7f || matches!(b, 9 | 10 | 13));
        // The length word is the first one, after the stats, whose two halves
        // measure printable text that fills the rest of the record (the
        // description and the name; then the trailing tables).
        // The stats are pairs from word 4, so the length word sits at an even index.
        for at in (6..words.len()).step_by(2) {
            let (name_len, description_len) = ((words[at] & 0xffff) as usize, (words[at] >> 16) as usize);
            let start = (at + 1) * 4;
            if name_len < 2 || name_len > 80 || description_len > 4000 || start + name_len + description_len > data.len() {
                continue;
            }
            let name = &data[start..start + name_len];
            let description = &data[start + name_len..start + name_len + description_len];
            if printable(name) && printable(description) && name.iter().any(|b| b.is_ascii_alphanumeric()) {
                let stats = words[4..at].chunks_exact(2).map(|p| (p[0], p[1])).collect();
                return Some(Item { id: words[1], name: String::from_utf8_lossy(name).into_owned(), description: String::from_utf8_lossy(description).into_owned(), stats });
            }
        }
        None
    }

    /// The icon texture's instance.
    pub fn icon(&self) -> Option<u32> {
        self.stat(stat::ICON)
    }

    pub fn stat(&self, key: u32) -> Option<u32> {
        self.stats.iter().find(|s| s.0 == key).map(|s| s.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Item 0x4321c, "Dream Mesh Circuit", as the game's database holds it.
    fn circuit() -> Vec<u8> {
        let mut words: Vec<u32> = vec![TEMPLATE, 0x4321c, 1_000_000, 0xc74e, 4, 0xf, 0x17, 0x333d, 0x4a, 1, 0x4f, 0x249b2, 0xd3, 0x1b58, 0x15, 0x21];
        let (name, description) = ("Dream Mesh Circuit", "This circuit board is woven with the pulsing ether.");
        words.push(name.len() as u32 | (description.len() as u32) << 16);
        let mut bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        bytes.extend_from_slice(name.as_bytes());
        bytes.extend_from_slice(description.as_bytes());
        bytes.extend_from_slice(&[0, 0, 0, 0]);
        bytes
    }

    #[test]
    fn name_description_and_icon() {
        let item = Item::parse(&circuit()).unwrap();
        assert_eq!(item.id, 0x4321c);
        assert_eq!(item.name, "Dream Mesh Circuit");
        assert!(item.description.starts_with("This circuit board"));
        assert_eq!(item.icon(), Some(0x249b2));
    }

    #[test]
    fn other_objects_are_refused() {
        assert_eq!(Item::parse(&[0; 64]), None);
    }
}
