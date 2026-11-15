//! A playfield's city record: where its organisation's city may build and what the land costs.
//!
//! The record is `RDBPlayfieldCityData_t`, resource type 0xf4267 with the playfield's number, read by city.dll's
//! `RDBCityController_c::ReadBlob` (0x1000d03a) and `CityControllerData_c::ReadBlob` (0x1000cf57); little-endian. After
//! the object's 12-byte head: a version, the count of controllers and the playfield's size in tiles; then per
//! controller its dynel's identity, the least and greatest house level, the price of the land, the upkeep, the landing
//! points (a list: an id and a place each) and the zones (a list: a 16-bit id, a type byte and a mask of ten by ten
//! tiles, 13 bytes). The lists' counts are the game's `(n + 1) * 1009` words.
//!
//! A zone is ten tiles by ten at the place its id gives (`id % (tiles x / 10)`, `id / (tiles x / 10)`, in tens of
//! tiles); a house may stand on its set tiles (bit `z * 10 + x`, low bit first). Playfield 6010 has one controller
//! with 74 zones, 2569 tiles of them set: the plots, with lanes between.

pub const KIND: u32 = 0xf4267;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Zone {
    pub id: u16,
    /// A byte nothing in the client reads (0 in every zone of 6010).
    pub kind: u8,
    /// The zone's least corner, in tiles.
    pub corner: (i32, i32),
    pub mask: [u8; 13],
}

impl Zone {
    /// Whether the tile at (x, z) of the zone (0 to 9 each) is one to build on.
    pub fn set(&self, x: i32, z: i32) -> bool {
        (0..10).contains(&x) && (0..10).contains(&z) && {
            let bit = (z * 10 + x) as usize;
            self.mask[bit / 8] >> (bit % 8) & 1 == 1
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Controller {
    pub identity: (u32, u32),
    pub levels: (u16, u16),
    /// What the land costs to buy, and its upkeep.
    pub purchase_cost: u32,
    pub upkeep: u32,
    pub landings: Vec<(u32, [f32; 3])>,
    pub zones: Vec<Zone>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CityData {
    /// The playfield's size in tiles (of 4 units).
    pub tiles: (i32, i32),
    pub controllers: Vec<Controller>,
}

struct Cursor<'a>(&'a [u8]);

impl Cursor<'_> {
    fn take(&mut self, n: usize) -> Option<&[u8]> {
        let (head, rest) = self.0.split_at_checked(n)?;
        self.0 = rest;
        Some(head)
    }
    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.take(2)?.try_into().ok()?))
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn f32(&mut self) -> Option<f32> {
        self.u32().map(f32::from_bits)
    }
    fn count(&mut self) -> Option<usize> {
        let coded = self.u32()?;
        (coded % 1009 == 0 && coded >= 1009 && coded / 1009 <= 100_000).then(|| (coded / 1009 - 1) as usize)
    }
}

/// The record (the whole database object, its head included); none for another version or one that ends early.
pub fn parse(record: &[u8]) -> Option<CityData> {
    let mut r = Cursor(record.get(12..)?);
    if r.u32()? != 1 {
        return None;
    }
    let count = r.u32()? as usize;
    let tiles = (r.u32()? as i32, r.u32()? as i32);
    let per_row = tiles.0 / 10;
    if count > 64 || per_row <= 0 {
        return None;
    }
    let controllers = (0..count)
        .map(|_| {
            let identity = (r.u32()?, r.u32()?);
            let levels = (r.u16()?, r.u16()?);
            let (purchase_cost, upkeep) = (r.u32()?, r.u32()?);
            let landings = (0..r.count()?).map(|_| Some((r.u32()?, [r.f32()?, r.f32()?, r.f32()?]))).collect::<Option<Vec<_>>>()?;
            let zones = (0..r.count()?)
                .map(|_| {
                    let (id, kind) = (r.u16()?, r.take(1)?[0]);
                    let mask: [u8; 13] = r.take(13)?.try_into().ok()?;
                    Some(Zone { id, kind, corner: ((id as i32 % per_row) * 10, (id as i32 / per_row) * 10), mask })
                })
                .collect::<Option<Vec<_>>>()?;
            Some(Controller { identity, levels, purchase_cost, upkeep, landings, zones })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(CityData { tiles, controllers })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A record made up to the layout: one controller, one landing point, two zones.
    #[test]
    fn a_made_up_record_reads() {
        let mut record = vec![0u8; 12];
        for word in [1u32, 1, 600, 600, 0xc418, 0xc000_177a] {
            record.extend_from_slice(&word.to_le_bytes());
        }
        record.extend_from_slice(&[1, 0, 1, 0]);
        for word in [500_000_000u32, 15_000_000, 2 * 1009, 7] {
            record.extend_from_slice(&word.to_le_bytes());
        }
        for f in [1.0f32, 2.0, 3.0] {
            record.extend_from_slice(&f.to_le_bytes());
        }
        record.extend_from_slice(&(3 * 1009u32).to_le_bytes());
        // (zone 1703: the 23rd of row 28; its tile (1, 0) set. Zone 61: the second of row 1, all set)
        record.extend_from_slice(&[0xa7, 0x06, 0, 0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        record.extend_from_slice(&[61, 0, 0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x0f]);
        let city = parse(&record).unwrap();
        assert_eq!(city.tiles, (600, 600));
        let c = &city.controllers[0];
        assert_eq!((c.identity, c.levels, c.purchase_cost, c.upkeep), ((0xc418, 0xc000_177a), (1, 1), 500_000_000, 15_000_000));
        assert_eq!(c.landings, [(7, [1.0, 2.0, 3.0])]);
        assert_eq!((c.zones[0].id, c.zones[0].corner), (1703, (230, 280)));
        assert!(c.zones[0].set(1, 0) && !c.zones[0].set(0, 0) && !c.zones[0].set(1, 1));
        assert_eq!(c.zones[1].corner, (10, 10));
        assert!(c.zones[1].set(9, 9) && !c.zones[1].set(10, 9) && !c.zones[1].set(-1, 0));
        // (cut short, or another version)
        assert_eq!(parse(&record[..record.len() - 1]), None);
        record[12] = 2;
        assert_eq!(parse(&record), None);
    }
}
