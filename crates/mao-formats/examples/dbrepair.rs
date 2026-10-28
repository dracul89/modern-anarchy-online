//! `dbrepair <game> [--write]`: points the index's broken entries at their records where `dbscan` found them
//! (`docs/db-repair/dbscan.txt`, or `$DBSCAN`: `type instance position used`). Only the leaf entries' position words change (the keys, and so the
//! tree, stay as they are). Each new position is checked to hold that key's record and to be no other entry's. Without
//! `--write` it only reports; with it, it copies the index to `ResourceDatabase.idx.before_repair` first (and refuses if
//! that copy exists) and then checks the whole index again.
use std::collections::{HashMap, HashSet};
use std::io::{Seek, SeekFrom, Write};

const NODE_HEADER: usize = 0x20;
const ENTRY: usize = 16;

fn word(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

fn main() {
    let mut args = std::env::args().skip(1);
    let game = args.next().unwrap();
    let write = args.next().as_deref() == Some("--write");
    let base = std::path::Path::new(&game).join("cd_image/data/db/ResourceDatabase");
    let idx_path = base.with_extension("idx");
    let db = mao_formats::ctree::Database::open(&base).unwrap();

    let mut found: HashMap<[u8; 8], u64> = HashMap::new();
    for line in std::fs::read_to_string(std::env::var("DBSCAN").unwrap_or("docs/db-repair/dbscan.txt".into())).unwrap().lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        let kind = u32::from_str_radix(f[0], 16).unwrap();
        let instance = u32::from_str_radix(f[1], 16).unwrap();
        let position = u64::from_str_radix(f[2].trim_start_matches("0x"), 16).unwrap();
        assert!(found.insert(mao_formats::ctree::key(kind, instance), position).is_none(), "a key twice in dbscan.txt");
    }

    // (the index is one file, its header's segment count 1: a node's position is its offset in the file)
    let idx = std::fs::read(&idx_path).unwrap();
    assert_eq!(word(&idx, 0xdc), 1, "an index in more than one file");
    let size = word(&idx, 0x0c) as usize;
    let node = |at: u32| &idx[at as usize..at as usize + size];
    let mut at = word(&idx, 0x1c);
    while node(at)[0x11] != 1 {
        at = word(node(at), NODE_HEADER);
    }
    let mut used: HashSet<u64> = HashSet::new();
    let mut patches = Vec::new();
    let mut entries = 0;
    loop {
        let n = node(at);
        let count = u16::from_le_bytes([n[8], n[9]]) as usize;
        for i in 0..count {
            let e = NODE_HEADER + i * ENTRY;
            let key: [u8; 8] = n[e + 4..e + 12].try_into().unwrap();
            let position = (word(n, e + 12) as u64) << 32 | word(n, e) as u64;
            entries += 1;
            if db.check(&key, position).is_ok() {
                used.insert(position);
            } else if let Some(&new) = found.get(&key) {
                db.check(&key, new).expect("a found record that isn't the key's");
                patches.push((at as u64 + e as u64, key, position, new));
            }
        }
        at = word(n, 0);
        if at == 0 {
            break;
        }
    }
    assert!(patches.iter().all(|p| !used.contains(&p.3)), "a found record another entry points at");
    assert_eq!(patches.iter().map(|p| p.3).collect::<HashSet<_>>().len(), patches.len(), "two entries to one record");
    println!("{entries} entries; {} to point at their records ({} in dbscan.txt)", patches.len(), found.len());
    for (offset, key, old, new) in patches.iter().take(5) {
        let (kind, instance) = mao_formats::ctree::unkey(key);
        println!("  {kind:x}:{instance:x} at index offset {offset:#x}: {old:#x} -> {new:#x}");
    }
    if !write {
        println!("(a dry run: --write to change the index)");
        return;
    }
    drop(db);

    let backup = base.with_extension("idx.before_repair");
    assert!(!backup.exists(), "{} exists already", backup.display());
    std::fs::copy(&idx_path, &backup).unwrap();
    assert_eq!(std::fs::read(&backup).unwrap(), idx, "the copy differs");
    let mut f = std::fs::OpenOptions::new().write(true).open(&idx_path).unwrap();
    for (offset, _, _, new) in &patches {
        f.seek(SeekFrom::Start(*offset)).unwrap();
        f.write_all(&(*new as u32).to_le_bytes()).unwrap();
        f.seek(SeekFrom::Start(*offset + 12)).unwrap();
        f.write_all(&((*new >> 32) as u32).to_le_bytes()).unwrap();
    }
    f.sync_all().unwrap();
    drop(f);
    println!("written; the old index is {}", backup.display());

    let db = mao_formats::ctree::Database::open(&base).unwrap();
    let (mut total, mut broken) = (0, 0);
    for e in db.entries().unwrap() {
        let (k, pos) = e.unwrap();
        total += 1;
        if db.check(&k, pos).is_err() {
            broken += 1;
        }
    }
    println!("after: {total} entries, {broken} still broken");
}
