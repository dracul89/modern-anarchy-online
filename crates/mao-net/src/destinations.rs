//! The destination list the server sends on entering some playfields (`371d0542`): the
//! places a travel network there reaches. After a nine-byte header (a zero byte, the
//! playfield's identity type 0xc79c and number, and a word) come entries of a `u32` id, a
//! `u16` name length, the name and two flag bytes.

/// The message that carries the list.
pub const DESTINATION_LIST: u32 = 0x371d0542;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Destination {
    pub id: u32,
    pub name: String,
    /// The two bytes after the name (1, 1 for most; 1, 2 for one in the capture).
    pub flags: [u8; 2],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DestinationList {
    /// The playfield the list was sent in.
    pub playfield: u32,
    pub entries: Vec<Destination>,
}

pub fn parse(data: &[u8]) -> Option<DestinationList> {
    if data.len() < 13 || u32::from_be_bytes(data[1..5].try_into().ok()?) != 0xc79c {
        return None;
    }
    let playfield = u32::from_be_bytes(data[5..9].try_into().ok()?);
    let mut entries = Vec::new();
    let mut at = 13;
    while at + 6 <= data.len() {
        let id = u32::from_be_bytes(data[at..at + 4].try_into().ok()?);
        let len = u16::from_be_bytes(data[at + 4..at + 6].try_into().ok()?) as usize;
        let end = at + 6 + len;
        if end + 2 > data.len() {
            break;
        }
        entries.push(Destination { id, name: String::from_utf8_lossy(&data[at + 6..end]).into_owned(), flags: [data[end], data[end + 1]] });
        at = end + 2;
    }
    Some(DestinationList { playfield, entries })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_captured_list() {
        let hex = "000000c79c00000227000023790000001b000a53747978204d61676d61010100000015000e436861727265642047726f6f7665010100000019000f4e6f727468206f6620597574746f7301010000001f001157657374206f6620506572646974696f6e0101000000200009447573742042616e6b01010000001d000c436172626f6e2047726f766501020000001200134265747765656e20746865204372617465727301010000000e000e506f7764657265642044756e65730101";
        let data: Vec<u8> = (0..hex.len() / 2).map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap()).collect();
        let list = parse(&data).unwrap();
        assert_eq!(list.playfield, 0x227);
        assert_eq!(list.entries.len(), 8);
        assert_eq!((list.entries[0].id, list.entries[0].name.as_str()), (0x1b, "Styx Magma"));
        assert_eq!(list.entries[5].name, "Carbon Grove");
        assert_eq!(list.entries[5].flags, [1, 2]);
        assert_eq!(list.entries[7].name, "Powdered Dunes");
        assert_eq!(parse(&data[..3]), None);
    }
}
