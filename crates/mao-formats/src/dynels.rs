//! A playfield's static dynels: the doors, terminals, vending machines and chests placed in it. They are in the playfield's
//! object (type `0xf425a`, the instance is the playfield's number). After the 12-byte object head: a count, then that many
//! records, each a length (84 for all that were seen) and the record: an identity (type, instance), a word (1), two zeros, the
//! playfield's number, the position `x y z` (`f32`), the rotation as `w x y z`, a zero and the item template (type `0xf4254`)
//! whose mesh stat (12) is the model. Door instances are `0xc000_0000 | number << 16 | playfield`.

use crate::texture::OBJECT_HEAD;

/// The database type of a playfield's dynel table.
pub const KIND: u32 = 0xf425a;

#[derive(Clone, Debug, PartialEq)]
pub struct Dynel {
    pub kind: u32,
    pub instance: u32,
    pub position: [f32; 3],
    /// `x y z w`.
    pub rotation: [f32; 4],
    pub template: u32,
    /// The words after the template: `0x66, 0x67, 0x3f1`, a count, then that many blocks; a block headed `0x0f, 0x18, 0x7e2` holds
    /// `(stat, value)` pairs for the object itself (12 its mesh, 21 ...) and then a counted name (the door's destination, "Basic Armor
    /// Shop"); a block headed `2, 0x10, 0x7e2` is an action with a spell (0xcf5a: the teleport through the door).
    pub tail: Vec<u32>,
    /// The object's own stats from the tail: a door's mesh (stat 12) here replaces its template's.
    pub stats: Vec<(u32, u32)>,
}

impl Dynel {
    pub fn stat(&self, key: u32) -> Option<u32> {
        self.stats.iter().find(|s| s.0 == key).map(|s| s.1)
    }
}

/// The object's own `(stat, value)` pairs of a record tail: those after the `0x0f, 0x18, 0x7e2` header, read while the stat is a small
/// number (the name's length word and text end them).
pub fn tail_stats(tail: &[u32]) -> Vec<(u32, u32)> {
    // (the words after the first are big-endian, unlike the record's head)
    let tail: Vec<u32> = tail.iter().map(|w| w.swap_bytes()).collect();
    // (the header's third word differs between records: 0x7e2 on the shop doors, 0xbd3 on the ICC HQ entrance)
    let Some(at) = tail.windows(3).position(|w| w[0] == 0x0f && w[1] == 0x18) else { return Vec::new() };
    let mut out = Vec::new();
    let mut k = at + 3;
    while k + 1 < tail.len() && tail[k] < 0x400 && (tail[k] != 0 || tail[k + 1] != 0) {
        out.push((tail[k], tail[k + 1]));
        k += 2;
        if out.len() >= 8 {
            break;
        }
    }
    out
}

/// The records of a dynel table (a record that is too short ends the list).
pub fn parse(object: &[u8]) -> Vec<Dynel> {
    let word = |at: usize| object.get(at..at + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
    let float = |at: usize| word(at).map(f32::from_bits);
    let Some(count) = word(OBJECT_HEAD) else { return Vec::new() };
    let mut out = Vec::new();
    let mut at = OBJECT_HEAD + 4;
    for _ in 0..count.min(100_000) {
        let Some(length) = word(at) else { break };
        let (start, length) = (at + 4, length as usize);
        at = start + length;
        if length < 60 || at > object.len() {
            break;
        }
        let f = |k: usize| float(start + k).unwrap_or(0.0);
        let tail: Vec<u32> = (60..length).step_by(4).filter_map(|k| word(start + k)).collect();
        let stats = tail_stats(&tail);
        out.push(Dynel {
            kind: word(start).unwrap(),
            instance: word(start + 4).unwrap(),
            position: [f(24), f(28), f(32)],
            rotation: [f(40), f(44), f(48), f(36)],
            template: word(start + 56).unwrap(),
            tail,
            stats,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_door_s_own_mesh_comes_from_its_tail() {
        // the pharmacy door of Andromeda (3286, 36.3, 791.1): a teleport action, then its own stats and the name "Basic Armor Shop"
        let tail: Vec<u32> = [0x88, 0x66, 0x67, 0x3f1, 3, 2, 0x10, 0x7e2, 0xcf5a, 0, 4, 0, 1, 0, 2, 0, 0xc79e, 0x80d, 0, 0xc01c028f, 0x186a2, 1, 0xf, 0x18, 0x7e2, 0xc, 0x48ae6, 0x15, 0x21, 0x1000, 0x69736142, 0x72412063, 0x20726f6d, 0x706f6853, 0u32].iter().map(|w| w.swap_bytes()).collect();
        let stats = tail_stats(&tail);
        assert_eq!(stats, vec![(12, 0x48ae6), (21, 0x21)]);
        // an Old Athen door: nothing but its stats
        let swapped = |v: &[u32]| v.iter().map(|w| w.swap_bytes()).collect::<Vec<u32>>();
        assert_eq!(tail_stats(&swapped(&[0x28, 0x66, 0x67, 0x3f1, 1, 0xf, 0x18, 0x7e2, 0xc, 0xa685, 0])), vec![(12, 0xa685)]);
        assert!(tail_stats(&swapped(&[0x14, 0x66, 0x67, 0x3f1, 0])).is_empty());
        // the ICC HQ entrance: another header word, the flags stat first
        assert_eq!(tail_stats(&swapped(&[0x0f, 0x18, 0xbd3, 0, 0x800a1401, 0xc, 0x3eff3, 0x15, 0x21, 0xe00])), vec![(0, 0x800a1401), (12, 0x3eff3), (21, 0x21)]);
    }

    #[test]
    fn a_door_record_is_read() {
        // The first door of playfield 0x49c.
        let record = "48c700009c0401c0010000000000000000000000".to_string() + "9c04000034024943ec51a040250340430000803f000000008862d6b300000000" + "000000004fa20000140000000000006600000067000003f100000000";
        let bytes: Vec<u8> = (0..record.len() / 2).map(|i| u8::from_str_radix(&record[2 * i..2 * i + 2], 16).unwrap()).collect();
        let mut object = vec![0u8; OBJECT_HEAD];
        object.extend_from_slice(&1u32.to_le_bytes());
        object.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        object.extend_from_slice(&bytes);
        let d = parse(&object);
        assert_eq!(d.len(), 1);
        assert_eq!((d[0].kind, d[0].instance, d[0].template), (0xc748, 0xc001049c, 0xa24f));
        assert!((d[0].position[0] - 201.01).abs() < 0.01 && (d[0].position[2] - 192.01).abs() < 0.01);
        assert_eq!(d[0].rotation[3], 1.0);
    }
}
