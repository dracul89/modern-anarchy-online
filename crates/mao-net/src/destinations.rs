//! `FightModeUpdate` (`371d0542`, named so in the AOSharp sources), sent on entering some
//! playfields: a list of names (Pandemonium's "Styx Magma", "Charred Groove" ...: area names,
//! **not** travel destinations, which an earlier reading of it wrongly took them for). After a nine-byte header (a zero byte, the
//! playfield's identity type 0xc79c and number, and a word) come entries of a `u32` id, a
//! `u16` name length, the name and two flag bytes.
//!
//! Gamecode's own name for the entries is `FightModeChange_t` (its reader FUN_1011faf5): each changes the fight mode of
//! the playfield's district of that name (`PlayfieldDistrictInfo_t::GetDistrictData`), the PvP setting the suppression
//! gas follows. The two bytes are the change's flags (bit 0 the mode is set rather than added, bit 1 it overrides the
//! changes before it, bit 2 it takes away the change of that id) and the mode, a signed byte. The district's mode is its
//! own (district data +0x54) run through its changes ([`fight_mode`], FUN_1011fddd). The playfield keeps the changes in its
//! `FightModeHandler_t` ([`FightModeHandler`], `PlayfieldAnarchy_t::GetFightModeHandler` 0x10121b8d).

/// The message that carries the list.
pub const DESTINATION_LIST: u32 = 0x371d0542;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Destination {
    pub id: u32,
    pub name: String,
    /// The two bytes after the name (1, 1 for most; 1, 2 for one in the capture): the change's flags and its mode.
    pub flags: [u8; 2],
}

impl Destination {
    /// The mode is set (flag bit 0, `FightModeChange_t` +0x24), else added.
    pub fn absolute(&self) -> bool {
        self.flags[0] & 1 != 0
    }

    /// The change overrides those before it (bit 1, +0x20).
    pub fn overriding(&self) -> bool {
        self.flags[0] & 2 != 0
    }

    /// The change takes away the earlier change of its id (bit 2, +0x2c; FUN_1011fe65) instead of adding itself.
    pub fn removal(&self) -> bool {
        self.flags[0] & 4 != 0
    }

    /// The mode, a signed byte (+0x28).
    pub fn mode(&self) -> i32 {
        self.flags[1] as i8 as i32
    }

    /// What FUN_1011faf5 accepts, else the whole message is refused (it throws): an id and a name, and a mode of 0 to 4
    /// (a set one 0..4, an added one -4..4, and no bit above the third: so 0..4 either way).
    fn valid(&self) -> bool {
        let m = self.flags[1];
        self.id != 0 && !self.name.is_empty() && (self.absolute() || m.wrapping_add(4) < 9) && (!self.absolute() || m < 5) && m & 0xf8 == 0
    }
}

/// A district's fight mode (FUN_1011fddd): its own mode `base` run through its changes in their order; after the first
/// overriding change only overriding ones count; a set mode replaces, another adds; the result held to 0..4
/// (FUN_1002d527).
pub fn fight_mode(base: i32, changes: &[Destination]) -> i32 {
    let mut mode = base;
    let mut overridden = false;
    for c in changes {
        if !overridden || c.overriding() {
            overridden |= c.overriding();
            mode = if c.absolute() { c.mode() } else { mode + c.mode() };
        }
    }
    mode.clamp(0, 4)
}

/// A playfield's `FightModeHandler_t` (`PlayfieldAnarchy_t` +0xbc, made with it by `GetFightModeHandler` 0x10121b8d): the
/// changes the server made to its districts' fight modes, each district's in the order they came (keyed by the district,
/// which the original finds by its name in the playfield's district info).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FightModeHandler {
    districts: Vec<(String, Vec<Destination>)>,
}

impl FightModeHandler {
    /// One change of a `FightModeUpdate` (FUN_101250c3's loop): a removal takes away, in every district, the first change
    /// of its id (FUN_1011fe65); any other is added at the end of its district's list (FUN_1011fed2).
    pub fn apply(&mut self, change: &Destination) {
        if change.removal() {
            for (_, list) in &mut self.districts {
                if let Some(at) = list.iter().position(|c| c.id == change.id) {
                    list.remove(at);
                }
            }
            return;
        }
        match self.districts.iter_mut().find(|d| d.0 == change.name) {
            Some((_, list)) => list.push(change.clone()),
            None => self.districts.push((change.name.clone(), vec![change.clone()])),
        }
    }

    /// The fight mode of the district of this name, its own mode `base` (FUN_1011fddd).
    pub fn mode(&self, district: &str, base: i32) -> i32 {
        fight_mode(base, self.districts.iter().find(|d| d.0 == district).map_or(&[][..], |d| &d.1[..]))
    }
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
        let entry = Destination { id, name: String::from_utf8_lossy(&data[at + 6..end]).into_owned(), flags: [data[end], data[end + 1]] };
        if !entry.valid() {
            return None;
        }
        entries.push(entry);
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
        assert!(list.entries[5].absolute() && !list.entries[5].overriding() && !list.entries[5].removal());
        assert_eq!(list.entries[5].mode(), 2);
        assert_eq!(list.entries[7].name, "Powdered Dunes");
        assert_eq!(parse(&data[..3]), None);
        // (a change with a mode past 4 spoils the whole message, as FUN_1011faf5 throws)
        let mut bad = data.clone();
        let last = bad.len() - 1;
        bad[last] = 5;
        assert_eq!(parse(&bad), None);
    }

    /// FUN_1011fddd: set modes replace, others add; after an overriding change only overriding ones count; 0..4.
    #[test]
    fn a_districts_fight_mode() {
        let change = |flags: u8, mode: i8| Destination { id: 1, name: "A district".into(), flags: [flags, mode as u8] };
        assert_eq!(fight_mode(2, &[]), 2);
        assert_eq!(fight_mode(2, &[change(1, 1)]), 1);
        assert_eq!(fight_mode(2, &[change(0, 1), change(0, 3)]), 4);
        assert_eq!(fight_mode(2, &[change(0, -4)]), 0);
        assert_eq!(fight_mode(0, &[change(3, 3), change(0, 1), change(1, 0)]), 3);
        assert_eq!(fight_mode(0, &[change(3, 3), change(2, -1)]), 2);
    }

    /// FUN_101250c3: changes add to their district's list, a removal takes the first of its id from each district.
    #[test]
    fn the_handler_keeps_changes_by_district() {
        let change = |id: u32, name: &str, flags: u8, mode: i8| Destination { id, name: name.into(), flags: [flags, mode as u8] };
        let mut h = FightModeHandler::default();
        h.apply(&change(7, "North", 1, 0));
        h.apply(&change(8, "North", 0, 1));
        h.apply(&change(9, "South", 0, -1));
        assert_eq!((h.mode("North", 3), h.mode("South", 3), h.mode("East", 3)), (1, 2, 3));
        h.apply(&change(7, "North", 4, 0));
        assert_eq!(h.mode("North", 3), 4);
        h.apply(&change(9, "South", 4, 0));
        assert_eq!(h.mode("South", 3), 3);
    }
}
