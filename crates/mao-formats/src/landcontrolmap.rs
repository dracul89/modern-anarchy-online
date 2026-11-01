//! A playfield's land-control map: the `0xf4248` object (one per Notum Wars playfield), GameData's `LandControlMap_t`
//! (`ReadBlob`, GameData.dll 0x10008680), which Gamecode loads with the playfield's area and district info (FUN_1011f406)
//! and hands the GUI through `N3Msg_GetLandControlBitmap` (0x10016c1c: the bits, the width and the height, +0x24, +0x1c,
//! +0x20). After the object head: a version (16 bits, 1), flags (16 bits, bit 0 compressed), the width and the height in
//! tiles (16 bits each, 1 to 2048), then a bit per tile, rows of `(width + 7) / 8` bytes, the low bit first; compressed
//! (flag bit 0) the rows come as a 32-bit length (1 byte to 1 MB, no more than is left) and a zlib stream of them. A tile
//! is 4 world units a side (`CanPlaceInTileAt`, 0x100085fc); a set bit is ground a tower may be planted on.

use crate::texture::OBJECT_HEAD;
use std::io::Read;

/// The type of a playfield's land-control map.
pub const KIND: u32 = 0xf4248;

/// A tile's side in world units (`CanPlaceInTileAt` divides the position by 4).
pub const TILE: f32 = 4.0;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LandControlMap {
    pub width: u32,
    pub height: u32,
    /// The bits, `(width + 7) / 8` bytes a row, row 0 at z 0.
    pub bits: Vec<u8>,
}

/// The map of a `0xf4248` object, or none where `ReadBlob` fails (a version other than 1, a size out of 1 to 2048, a bad
/// compressed length, a stream that does not inflate to its end, too little data).
pub fn parse(object: &[u8]) -> Option<LandControlMap> {
    let h = |o: usize| Some(u16::from_le_bytes(object.get(OBJECT_HEAD + o..OBJECT_HEAD + o + 2)?.try_into().ok()?));
    let (version, flags, width, height) = (h(0)?, h(2)?, h(4)? as u32, h(6)? as u32);
    if version != 1 || !(1..=0x800).contains(&width) || !(1..=0x800).contains(&height) {
        return None;
    }
    let size = width.div_ceil(8) as usize * height as usize;
    let at = OBJECT_HEAD + 8;
    let bits = if flags & 1 == 0 {
        object.get(at..at + size)?.to_vec()
    } else {
        // (the length, then `inflateInit_` and one `inflate(Z_FINISH)` into the rows, which must end the stream)
        let left = object.len().checked_sub(at)?;
        let packed = u32::from_le_bytes(object.get(at..at + 4)?.try_into().ok()?) as usize;
        if packed.wrapping_sub(1) > 0xfffff || left < packed {
            return None;
        }
        let mut out = Vec::with_capacity(size);
        flate2::read::ZlibDecoder::new(object.get(at + 4..at + 4 + packed)?).read_to_end(&mut out).ok()?;
        if out.len() < size {
            return None;
        }
        out.truncate(size);
        out
    };
    Some(LandControlMap { width, height, bits })
}

impl LandControlMap {
    /// A row's bytes.
    pub fn stride(&self) -> usize {
        self.width.div_ceil(8) as usize
    }

    /// `IsTilePlaceAble(x, z)` (0x100085ae): the tile's bit (no bounds check in the original; none outside here).
    pub fn placeable(&self, x: i32, z: i32) -> bool {
        if x < 0 || z < 0 || x as u32 >= self.width || z as u32 >= self.height {
            return false;
        }
        self.bits.get(z as usize * self.stride() + x as usize / 8).is_some_and(|b| b & (1 << (x & 7)) != 0)
    }

    /// `CanPlaceInTileAt(position)` (0x100085fc): inside `[0, width * 4)` by `[0, height * 4)`, the bit of the tile the
    /// truncated position is in.
    pub fn can_place_at(&self, position: [f32; 3]) -> bool {
        let (x, z) = (position[0], position[2]);
        if !(x >= 0.0 && x < (self.width * 4) as f32 && z >= 0.0 && z < (self.height * 4) as f32) {
            return false;
        }
        self.placeable(x as i32 / 4, z as i32 / 4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn object(flags: u16, width: u16, height: u16, body: &[u8]) -> Vec<u8> {
        let mut o = vec![0x48, 0x42, 0x0f, 0x00, 1, 0, 0, 0, 1, 0, 0, 0];
        for v in [1u16, flags, width, height] {
            o.extend_from_slice(&v.to_le_bytes());
        }
        o.extend_from_slice(body);
        o
    }

    #[test]
    fn reads_the_rows_and_their_bits() {
        // 10 x 2: two bytes a row; tile (0, 0), (9, 0) and (3, 1) set
        let map = parse(&object(0, 10, 2, &[0x01, 0x02, 0x08, 0x00])).unwrap();
        assert_eq!((map.width, map.height, map.stride()), (10, 2, 2));
        assert!(map.placeable(0, 0) && map.placeable(9, 0) && map.placeable(3, 1));
        assert!(!map.placeable(1, 0) && !map.placeable(10, 0) && !map.placeable(-1, 0));
        assert!(map.can_place_at([39.9, 0.0, 3.0]) && map.can_place_at([12.5, 7.0, 4.0]));
        assert!(!map.can_place_at([40.0, 0.0, 0.0]) && !map.can_place_at([-0.5, 0.0, 0.0]));
    }

    #[test]
    fn inflates_a_compressed_map() {
        use std::io::Write;
        let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        e.write_all(&[0xff, 0x00, 0x55]).unwrap();
        let packed = e.finish().unwrap();
        let mut body = (packed.len() as u32).to_le_bytes().to_vec();
        body.extend_from_slice(&packed);
        let map = parse(&object(1, 8, 3, &body)).unwrap();
        assert_eq!(map.bits, vec![0xff, 0x00, 0x55]);
    }

    #[test]
    fn refuses_what_read_blob_refuses() {
        assert!(parse(&object(0, 0, 2, &[])).is_none());
        assert!(parse(&object(0, 0x801, 1, &[0; 0x101])).is_none());
        assert!(parse(&object(0, 8, 2, &[0])).is_none());
        let mut bad = object(0, 8, 1, &[0]);
        bad[OBJECT_HEAD] = 2;
        assert!(parse(&bad).is_none());
    }

    #[test]
    fn every_land_control_map_in_the_game_reads() {
        // (the game's database; skipped without MAO_GAME)
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let db = crate::resource::ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap();
        let ids: Vec<_> = db.ids().unwrap().filter_map(Result::ok).filter(|i| i.kind == KIND).collect();
        assert!(ids.len() > 30, "{} maps", ids.len());
        for id in ids {
            let object = db.get(id).unwrap().unwrap();
            let map = parse(&object).unwrap_or_else(|| panic!("playfield {}", id.instance));
            assert_eq!(map.bits.len(), map.stride() * map.height as usize);
            assert!(map.bits.iter().any(|&b| b != 0), "playfield {} has no ground for towers", id.instance);
        }
        // (playfield 695's map, 0x2b7: 1000 x 1000 tiles, its header 0100 0000 e803 e803)
        let map = parse(&db.get(crate::resource::Id { kind: KIND, instance: 0x2b7 }).unwrap().unwrap()).unwrap();
        assert_eq!((map.width, map.height), (1000, 1000));
    }
}
