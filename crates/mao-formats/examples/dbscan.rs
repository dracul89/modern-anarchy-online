//! `dbscan <game>`: every record header in the data files (`0xfafa`, space, used, then the key's type and instance) whose
//! key is one of the index's broken entries: where each broken key's records really are (a key may have several: old and
//! new copies), written as `type instance position used` lines to `dbscan.txt`.
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};

fn main() {
    let game = std::env::args().nth(1).unwrap();
    let base = std::path::Path::new(&game).join("cd_image/data/db/ResourceDatabase");
    let db = mao_formats::ctree::Database::open(&base).unwrap();
    let mut broken: HashSet<(u32, u32)> = HashSet::new();
    for e in db.entries().unwrap() {
        let (k, pos) = e.unwrap();
        if db.check(&k, pos).is_err() {
            broken.insert((u32::from_be_bytes(k[..4].try_into().unwrap()), u32::from_be_bytes(k[4..].try_into().unwrap())));
        }
    }
    eprintln!("{} broken keys", broken.len());
    let (size, header) = (0x4000_0000u64, 0x1000u64);
    let end = |k: u64| size + k * (size - header);
    let mut found: HashMap<(u32, u32), Vec<(u64, u32)>> = HashMap::new();
    for k in 0u64.. {
        let mut name = base.as_os_str().to_owned();
        name.push(if k == 0 { ".dat".to_string() } else { format!(".dat.{k:03}") });
        let Ok(mut f) = std::fs::File::open(&name) else { break };
        let start = if k == 0 { 0 } else { end(k - 1) - header };
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).unwrap();
        let mut hits = 0;
        // (a record: fa fa, space u32, used u32, type u32, instance u32)
        for i in 0..buf.len().saturating_sub(18) {
            if buf[i] != 0xfa || buf[i + 1] != 0xfa {
                continue;
            }
            let w = |o: usize| u32::from_le_bytes(buf[i + o..i + o + 4].try_into().unwrap());
            let key = (w(10), w(14));
            if broken.contains(&key) {
                found.entry(key).or_default().push((start + i as u64, w(6)));
                hits += 1;
            }
        }
        eprintln!("segment {k}: {} MB, {hits} records of broken keys", buf.len() >> 20);
    }
    let mut out = std::fs::File::create("dbscan.txt").unwrap();
    let mut none = 0;
    let mut many = 0;
    for key in &broken {
        match found.get(key) {
            None => none += 1,
            Some(v) => {
                if v.len() > 1 {
                    many += 1;
                }
                for (pos, used) in v {
                    writeln!(out, "{:x} {:x} {pos:#x} {used}", key.0, key.1).unwrap();
                }
            }
        }
    }
    eprintln!("{} keys found, {none} nowhere, {many} with more than one copy", broken.len() - none);
}
