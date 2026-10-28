//! `dbbroken <game>`: the index entries whose position holds no record or another key, grouped: by what is there, by
//! the segment the position falls in, by type; and a sample.
use std::collections::BTreeMap;

fn main() {
    let game = std::env::args().nth(1).unwrap();
    let db = mao_formats::ctree::Database::open(&std::path::Path::new(&game).join("cd_image/data/db/ResourceDatabase")).unwrap();
    let (mut by_error, mut by_segment, mut by_kind) = (BTreeMap::new(), BTreeMap::new(), BTreeMap::new());
    let mut sample = Vec::new();
    for e in db.entries().unwrap() {
        let (k, pos) = e.unwrap();
        let Err(err) = db.check(&k, pos) else { continue };
        let kind = u32::from_be_bytes(k[..4].try_into().unwrap());
        let inst = u32::from_be_bytes(k[4..].try_into().unwrap());
        *by_error.entry(format!("{err:?}")).or_insert(0) += 1;
        // (segments of 1 GB less the 4 KB header after the first)
        let seg = if pos < 0x4000_0000 { 0 } else { 1 + (pos - 0x4000_0000) / (0x4000_0000 - 0x1000) };
        *by_segment.entry(seg).or_insert(0) += 1;
        *by_kind.entry(kind).or_insert(0) += 1;
        if sample.len() < 12 {
            let there = db.record_at(pos).ok().and_then(|r| r.get(..8).map(|b| (u32::from_le_bytes(b[..4].try_into().unwrap()), u32::from_le_bytes(b[4..8].try_into().unwrap()))));
            sample.push(format!("{kind:x}:{inst:x} at {pos:#x} (segment {seg}): {err:?}; there {there:x?}"));
        }
    }
    println!("by error {by_error:?}");
    println!("by segment {by_segment:?}");
    println!("by type {:x?}", by_kind);
    for s in sample {
        println!("  {s}");
    }
}
