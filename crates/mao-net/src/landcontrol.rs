//! Notum Wars land control: the tower sites of a playfield, as Gamecode.dll's `LandControl_t` (DAT_102eab98) keeps them.
//!
//! * `PlayfieldAllTowers` (`55220726`, `PlayfieldAllTowersIIR_t`), sent at every playfield entry with the identity
//!   `(0x9c50, playfield)`: a header byte, then the towers counted `(n + 1) * 0x3f1` (FUN_1012b192 refuses a count that is
//!   not a multiple of 0x3f1 or above 30000), each read by FUN_1012bb14 into a `TowerProxyBase_t` ([`Tower`]). The list
//!   replaces the one before (FUN_1012a1f4 clears, then FUN_1012a0e9 adds each); a playfield without towers sends none.
//! * `PlayfieldTowerUpdateClient` (`5b1e052c`, FUN_1012b38a): a site's identity and a kind, 1 a tower destroyed (FUN_1012a153
//!   drops the entry whose site matches), 2 a tower planted (its [`Tower`] follows; FUN_1012a0e9 adds it).
//! * `PlayfieldAllCities` (`59210126`, `PlayfieldAllCitiesIIR_t`, FUN_101304f8): an `i16` length and that many bytes, handed
//!   whole to `PlayfieldCityHolderClient_c::UpdateNewHouses` (the player cities' houses). Always empty in the captures.
//!
//! All big-endian, as the protocol is.

/// `PlayfieldAllTowers`.
pub const ALL_TOWERS: u32 = 0x5522_0726;
/// `PlayfieldAllCities`.
pub const ALL_CITIES: u32 = 0x5921_0126;
/// `PlayfieldTowerUpdateClient`.
pub const TOWER_UPDATE: u32 = 0x5b1e_052c;

/// `TowerClass` (the schema's names): the tower kinds.
pub const CONTROL_TOWER: i32 = 1;
pub const TURRET: i32 = 2;
pub const CONDUCTOR: i32 = 4;

/// One tower site (`TowerProxyBase_t`, read by FUN_1012bb14 in this order).
#[derive(Clone, Debug, PartialEq)]
pub struct Tower {
    /// The site (`+0x04`, type 0xc354): what an update names.
    pub site: (u32, u32),
    /// The tower itself (`+0x0c`, type 0xc350).
    pub tower: (u32, u32),
    /// Its place (`+0x14`): x, height, z.
    pub position: [f32; 3],
    /// The model standing (`+0x20`) and the model once destroyed (`+0x28`), and its scale (`+0x2c`).
    pub mesh: u32,
    pub destroyed_mesh: u32,
    pub scale: f32,
    /// The side (`+0x24`: 0 neutral, 1 Clan, 2 Omni) and the kind (`+0x30`, `TowerClass`).
    pub side: i32,
    pub class: i32,
}

struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn u32(&mut self) -> Option<u32> {
        let v = u32::from_be_bytes(self.data.get(self.at..self.at + 4)?.try_into().ok()?);
        self.at += 4;
        Some(v)
    }

    fn f32(&mut self) -> Option<f32> {
        self.u32().map(f32::from_bits)
    }

    fn identity(&mut self) -> Option<(u32, u32)> {
        Some((self.u32()?, self.u32()?))
    }

    /// FUN_1012bb14.
    fn tower(&mut self) -> Option<Tower> {
        let (site, tower) = (self.identity()?, self.identity()?);
        let position = [self.f32()?, self.f32()?, self.f32()?];
        let (mesh, side, destroyed_mesh, scale, class) = (self.u32()?, self.u32()? as i32, self.u32()?, self.f32()?, self.u32()? as i32);
        Some(Tower { site, tower, position, mesh, destroyed_mesh, scale, side, class })
    }
}

/// The towers of a `PlayfieldAllTowers` message's data (the header byte first); `None` when it does not read (FUN_1012b192
/// sets the stream's error then and takes none).
pub fn all_towers(data: &[u8]) -> Option<Vec<Tower>> {
    let mut r = Reader { data, at: 1 };
    let marker = r.u32()?;
    if marker % 0x3f1 != 0 || marker == 0 || marker / 0x3f1 - 1 >= 0x7531 {
        return None;
    }
    (0..marker / 0x3f1 - 1).map(|_| r.tower()).collect()
}

/// A `PlayfieldTowerUpdateClient`: the site, and the planted tower (`None`: the site's tower was destroyed). Other kinds
/// read nothing more and do what a plant does (FUN_1012b350 takes every kind but 1 for one) with the tower the message
/// was made with: FUN_1012b311 makes it with `TowerProxyBase_t`'s constructor (FUN_1012b9d9), which zeroes the site, the
/// tower and the place (+0x04 to +0x1c) and leaves the rest as the memory had it; taken as zero here.
pub fn tower_update(data: &[u8]) -> Option<((u32, u32), Option<Tower>)> {
    let mut r = Reader { data, at: 1 };
    let site = r.identity()?;
    match r.u32()? {
        1 => Some((site, None)),
        2 => Some((site, Some(r.tower()?))),
        _ => Some((site, Some(Tower { site: (0, 0), tower: (0, 0), position: [0.0; 3], mesh: 0, destroyed_mesh: 0, scale: 0.0, side: 0, class: 0 }))),
    }
}

/// Applies an update to a playfield's list as `LandControl_t` does: a destroyed site's entry goes (FUN_1012a153), a planted
/// tower is added at the front (FUN_1012a0e9 → FUN_1012a8a6 inserts before the list's first node).
pub fn apply_update(towers: &mut Vec<Tower>, site: (u32, u32), planted: Option<Tower>) {
    match planted {
        None => {
            if let Some(i) = towers.iter().position(|t| t.site == site) {
                towers.remove(i);
            }
        }
        Some(t) => towers.insert(0, t),
    }
}

/// Whether a site of a class is among the four sites nearest a place within 40 m (FUN_1012a2e6: `LandControl_t`'s tree of
/// the sites by their places, which FUN_1012a2a8 asks for at most 4 within `DAT_101673f0` = 40.0, the distance taken in three
/// dimensions, FUN_1012a644; each entry found is the list node whose +0x38 is the site's `TowerClass`). A flyer near a class-5
/// site leaves fly mode (FUN_100450c0). (The tree's search, FUN_1012ace1, is taken as the nearest four; how its pruning
/// treats sites past the fourth was not followed.)
pub fn class_near(towers: &[Tower], at: [f32; 3], class: i32) -> bool {
    let mut near: Vec<(f32, i32)> = towers
        .iter()
        .map(|t| (((t.position[0] - at[0]).powi(2) + (t.position[1] - at[1]).powi(2) + (t.position[2] - at[2]).powi(2)).sqrt(), t.class))
        .filter(|&(d, _)| d <= 40.0)
        .collect();
    near.sort_by(|a, b| a.0.total_cmp(&b.0));
    near.iter().take(4).any(|&(_, c)| c == class)
}

/// The houses' blob of a `PlayfieldAllCities` message (FUN_101304f8: an `i16` length, then the bytes).
pub fn all_cities(data: &[u8]) -> Option<Vec<u8>> {
    let n = u16::from_be_bytes(data.get(1..3)?.try_into().ok()?) as usize;
    data.get(3..3 + n).map(<[u8]>::to_vec)
}

/// One house of a player city (`BaseClientHouseData_t`, 25 bytes on the wire, read by city.dll 0x1000dec3): its place,
/// the least corner of the tiles it stands on as it is turned; its rotation in degrees (quarter turns); a flag (set when
/// the byte is 1: a demolition under way, by what sets it later); and its template, a database record of type 0xf4268.
/// A house has no identity of its own: it is known by its tile, `(x / 4, z / 4)` rounded down.
#[derive(Clone, Debug, PartialEq)]
pub struct House {
    pub position: [f32; 3],
    pub rotation: i32,
    pub flag: bool,
    pub template: (u32, u32),
}

impl House {
    pub const SIZE: usize = 25;

    pub fn parse(record: &[u8]) -> Option<House> {
        let word = |at: usize| Some(u32::from_be_bytes(record.get(at..at + 4)?.try_into().ok()?));
        let float = |at: usize| word(at).map(f32::from_bits);
        Some(House { position: [float(0)?, float(4)?, float(8)?], rotation: word(12)? as i32, flag: *record.get(16)? == 1, template: (word(17)?, word(21)?) })
    }

    /// The tile the house is known by (`GetTilePos`, 0x100011a7).
    pub fn tile(&self) -> (i32, i32) {
        ((self.position[0] / 4.0).floor() as i32, (self.position[2] / 4.0).floor() as i32)
    }
}

/// The houses of [`all_cities`]' blob (`PlayfieldCityHolderClient_c::UpdateNewHouses`, city.dll 0x10016317): a plain
/// count, then that many [`House`]s. An empty blob is no houses. The original does not check the count against the
/// length: it reads its count of houses and leaves any bytes after them; a blob too short for its count gives the houses it
/// holds (the original would read on past its end).
pub fn houses(blob: &[u8]) -> Option<Vec<House>> {
    if blob.is_empty() {
        return Some(Vec::new());
    }
    let count = u32::from_be_bytes(blob.get(..4)?.try_into().ok()?) as usize;
    blob[4..].chunks_exact(House::SIZE).take(count).map(House::parse).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(h: &str) -> Vec<u8> {
        (0..h.len() / 2).map(|i| u8::from_str_radix(&h[2 * i..2 * i + 2], 16).unwrap()).collect()
    }

    #[test]
    fn an_empty_playfield_has_no_towers_or_cities() {
        // (every playfield entry of the captures that is not a Notum Wars one)
        assert_eq!(all_towers(&hex("01000003f1")), Some(vec![]));
        assert_eq!(all_cities(&hex("010000")), Some(vec![]));
        assert_eq!(all_towers(&hex("01000003f2")), None);
        assert_eq!(all_towers(&hex("01000007e2")), None);
    }

    #[test]
    fn captured_towers_read() {
        // (the first two of the 105 towers of Andromeda (655) in the 2026-10-31 08:42 capture, counted as two)
        let data = hex(concat!(
            "0100000bd3",
            "0000c354002b60d00000c3507af9a6ba44f9dd39420c70a443bf84d00003128100000000000313a33f8b851f00000001",
            "0000c354002b60d10000c3507af9a6bb44fb0e5b42040a3d43c72f760003127200000000000313a23f8b851f00000002",
        ));
        let towers = all_towers(&data).unwrap();
        assert_eq!(towers.len(), 2);
        let t = &towers[0];
        assert_eq!((t.site, t.tower, t.mesh, t.destroyed_mesh, t.side, t.class), ((0xc354, 0x2b60d0), (0xc350, 0x7af9a6ba), 0x31281, 0x313a3, 0, CONTROL_TOWER));
        assert!((t.position[0] - 1998.9).abs() < 0.1 && (t.position[2] - 383.0).abs() < 0.1 && (t.scale - 1.09).abs() < 0.001);
        assert_eq!(towers[1].class, TURRET);
        // (a count the data does not hold reads nothing)
        assert_eq!(all_towers(&data[..data.len() - 1]), None);
    }

    #[test]
    fn whole_captures_read() {
        // (the captures are not in the repository: read them from the main checkout when they are there)
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../modern-anarchy-online/captures");
        let Ok(dirs) = std::fs::read_dir(&root) else { return };
        let mut seen = 0;
        for dir in dirs.flatten() {
            let Ok(text) = std::fs::read_to_string(dir.path().join("dump.txt")) else { continue };
            for line in text.lines().filter(|l| l.contains(" S>C ")) {
                let Some(bytes) = line.split("bytes: ").nth(1).map(hex) else { continue };
                if line.contains(" kind 55220726 ") {
                    let towers = all_towers(&bytes).expect(line);
                    assert_eq!(5 + 48 * towers.len(), bytes.len());
                    assert!(towers.iter().all(|t| matches!(t.class, CONTROL_TOWER | TURRET | CONDUCTOR) && (0..=2).contains(&t.side)));
                    seen += 1;
                } else if line.contains(" kind 59210126 ") {
                    assert!(all_cities(&bytes).is_some());
                }
            }
        }
        eprintln!("{seen} tower lists read");
    }

    #[test]
    fn updates_destroy_and_plant() {
        let tower = Tower { site: (0xc354, 1), tower: (0xc350, 2), position: [1.0, 2.0, 3.0], mesh: 0x31281, destroyed_mesh: 0x313a3, scale: 1.0, side: 1, class: CONTROL_TOWER };
        let mut list = vec![tower.clone()];
        let planted = hex(concat!("01", "0000c35400000002", "00000002", "0000c354000000030000c35000000004", "3f8000004000000040400000", "00031281", "00000002", "000313a3", "3f800000", "00000002"));
        let (site, t) = tower_update(&planted).unwrap();
        let t = t.unwrap();
        assert_eq!((site, t.site, t.side, t.class), ((0xc354, 2), (0xc354, 3), 2, TURRET));
        apply_update(&mut list, site, Some(t));
        assert_eq!(list.len(), 2);
        let (site, gone) = tower_update(&hex("010000c3540000000100000001")).unwrap();
        assert_eq!((site, gone), ((0xc354, 1), None));
        apply_update(&mut list, site, None);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].site, (0xc354, 3));
        // (another kind adds an empty entry at the front, FUN_1012b350)
        let (site, empty) = tower_update(&hex("010000c3540000000500000007")).unwrap();
        assert_eq!((site, empty.as_ref().map(|t| (t.site, t.mesh, t.class))), ((0xc354, 5), Some(((0, 0), 0, 0))));
        apply_update(&mut list, site, empty);
        assert_eq!((list.len(), list[0].site), (2, (0, 0)));
    }

    #[test]
    fn a_class_is_looked_for_among_the_four_nearest_within_40_m() {
        let site = |x: f32, class: i32| Tower { site: (0xc354, x as u32), tower: (0xc350, 0), position: [x, 0.0, 0.0], mesh: 0, destroyed_mesh: 0, scale: 1.0, side: 0, class };
        assert!(class_near(&[site(30.0, 5)], [0.0, 10.0, 0.0], 5));
        // (the height counts: 30 along and 30 up is 42 m)
        assert!(!class_near(&[site(30.0, 5)], [0.0, 30.0, 0.0], 5));
        assert!(!class_near(&[site(10.0, 1)], [0.0; 3], 5));
        // (four nearer sites hide a fifth)
        let mut list: Vec<Tower> = (1..=4).map(|i| site(i as f32, TURRET)).collect();
        list.push(site(20.0, 5));
        assert!(!class_near(&list, [0.0; 3], 5));
        list.remove(0);
        assert!(class_near(&list, [0.0; 3], 5));
    }

    /// The start of the user's log of 2026-11-15 (playfield 6010, 80 houses in 2007 bytes; the log kept 28 of them):
    /// the first house, with a template's instance put after it.
    #[test]
    fn a_citys_houses() {
        let message = hex("01001d000000014459000040a00000449900000000005a00000f4268006a0000");
        let blob = all_cities(&message).unwrap();
        let list = houses(&blob).unwrap();
        assert_eq!(list, vec![House { position: [868.0, 5.0, 1224.0], rotation: 90, flag: false, template: (0xf4268, 0x6a0000) }]);
        assert_eq!(list[0].tile(), (217, 306));
        assert_eq!(houses(&all_cities(&hex("010000")).unwrap()), Some(Vec::new()));
        // (ZON-013: a count that the bytes do not hold gives what they hold; bytes past the count are left)
        assert_eq!(houses(&hex("00000002445900004")), Some(Vec::new()));
        let mut two = hex("00000001");
        two.extend(&blob[4..]);
        two.extend([0u8; 7]);
        assert_eq!(houses(&two), Some(list.clone()));
        // (80 houses are 2004 bytes, the length the log's message gave)
        assert_eq!(4 + House::SIZE * 80, 0x7d4);
    }

    /// A whole list (the user's city in their log of 2026-11-15: nine houses in 229 bytes).
    #[test]
    fn a_real_citys_houses() {
        let list = houses(&hex("000000094459000040a0000044a000000000010e00000f4268006a00034459000040a0000044a980000000000000000f4268006a0011446f000040a0000044b58000000000b400000f4268006a00014480800040a00000449e00000000005a00000f4268006a00184480800040a00000449f00000000005a00000f4268006a001a4482000040a00000449e0000000000b400000f4268006a00194485000040a00000449b80000000000000000f4268006a00024487800040a0000044a980000000000000000f4268006a000b448a000040a0000044b080000000000000000f4268006a000e")).unwrap();
        assert_eq!(list.len(), 9);
        assert_eq!(list[0], House { position: [868.0, 5.0, 1280.0], rotation: 270, flag: false, template: (0xf4268, 0x6a0003) });
        assert_eq!(list[2], House { position: [956.0, 5.0, 1452.0], rotation: 180, flag: false, template: (0xf4268, 0x6a0001) });
        assert_eq!(list.iter().map(|h| h.template.1 & 0xffff).collect::<Vec<_>>(), [3, 0x11, 1, 0x18, 0x1a, 0x19, 2, 0xb, 0xe]);
        assert!(list.iter().all(|h| !h.flag && h.rotation % 90 == 0));
    }
}
