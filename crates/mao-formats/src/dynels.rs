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
        out.push(Dynel {
            kind: word(start).unwrap(),
            instance: word(start + 4).unwrap(),
            position: [f(24), f(28), f(32)],
            rotation: [f(40), f(44), f(48), f(36)],
            template: word(start + 56).unwrap(),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

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
