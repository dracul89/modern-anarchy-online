//! The character's items: the first part of the big world-entry message
//! (`29304349`, 4 KB, sent once on entering the world). After 9 header bytes
//! come 32-byte entries, big-endian, one per item, until the stat tables
//! start:
//!
//! ```text
//! u32 slot       the placement: 0x03..0x35 worn or equipped, 0x40.. in the
//!                backpack (the original's numbering: a belt is 7, the NCU
//!                units it carries are 9..13)
//! u16 flags      0x01 / 0x21 / 0x81 / 0xa1 an item, 0x02 / 0x22 a bag
//! u16 count      the stack size
//! u32 type, u32 id   the bag's identity (type 0xc749) for a container, else 0
//! u32 low id, u32 high id   the item's templates (type 0xf4254)
//! u32 quality
//! u32 extra
//! ```

/// The message that carries the inventory.
pub const WORLD_ENTRY: u32 = 0x29304349;

/// The identity type of a bag.
pub const BAG: u32 = 0xc749;

/// Where the backpack's slots start.
pub const BACKPACK_FIRST: u32 = 0x40;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    pub slot: u32,
    pub flags: u16,
    pub count: u16,
    /// The container's identity (type, id), when the entry is a bag.
    pub container: (u32, u32),
    pub low_id: u32,
    pub high_id: u32,
    pub quality: u32,
    pub extra: u32,
}

impl Entry {
    pub fn is_bag(&self) -> bool {
        self.container.0 == BAG
    }

    pub fn worn(&self) -> bool {
        self.slot < BACKPACK_FIRST
    }
}

/// The items in the world-entry message's data (as `Message::data`).
pub fn parse(data: &[u8]) -> Vec<Entry> {
    let long = |at: usize| u32::from_be_bytes(data[at..at + 4].try_into().unwrap());
    let short = |at: usize| u16::from_be_bytes(data[at..at + 2].try_into().unwrap());
    let mut entries = Vec::new();
    let mut at = 9;
    while at + 32 <= data.len() {
        let entry = Entry { slot: long(at), flags: short(at + 4), count: short(at + 6), container: (long(at + 8), long(at + 12)), low_id: long(at + 16), high_id: long(at + 20), quality: long(at + 24), extra: long(at + 28) };
        // The stat tables that follow don't look like items: a slot past the
        // backpack, flags outside the few in use, or no item id.
        if entry.slot > 0xff || entry.flags & !0xa3 != 0 || entry.flags == 0 || entry.low_id == 0 {
            break;
        }
        entries.push(entry);
        at += 32;
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The start of a real world-entry message: header, a worn item, a
    /// backpack's first stack and a bag.
    fn sample() -> Vec<u8> {
        let hex = concat!(
            "000000001a0000e49a",
            "00000003000100010000000000000000", "0003d8fd0003d8fd0000000100000000",
            "00000007008100010000000000000000", "0004321c0004321c0000000100000000",
            "00000040", "00a10032", "0000000000000000", "0004710a0004710b", "0000002100000000",
            "0000004d", "00020001", "0000c749001c003d", "0001839c0001839c", "0000000100000000",
            "0000b93f00000000",
        );
        (0..hex.len() / 2).map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap()).collect()
    }

    #[test]
    fn entries_stop_where_the_tables_start() {
        let entries = parse(&sample());
        assert_eq!(entries.len(), 4);
        assert_eq!((entries[0].slot, entries[0].low_id, entries[0].quality), (3, 0x3d8fd, 1));
        assert_eq!((entries[1].slot, entries[1].low_id), (7, 0x4321c));
        assert!(entries[0].worn());
        let stack = entries[2];
        assert_eq!((stack.slot, stack.count, stack.low_id, stack.high_id, stack.quality), (0x40, 50, 0x4710a, 0x4710b, 33));
        assert!(!stack.worn());
        assert!(entries[3].is_bag());
        assert_eq!(entries[3].container, (BAG, 0x1c003d));
    }
}
