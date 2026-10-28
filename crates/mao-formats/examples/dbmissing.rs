//! `dbmissing <game> <dbscan.txt>`: the broken keys with no record anywhere, by name from the instance index.
use std::collections::HashSet;
use std::io::BufRead;

fn main() {
    let mut args = std::env::args().skip(1);
    let game = args.next().unwrap();
    let list = args.next().unwrap();
    let rdb = mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap();
    let db = mao_formats::ctree::Database::open(&std::path::Path::new(&game).join("cd_image/data/db/ResourceDatabase")).unwrap();
    let found: HashSet<String> = std::io::BufReader::new(std::fs::File::open(list).unwrap()).lines().map(|l| l.unwrap().split(' ').take(2).collect::<Vec<_>>().join(" ")).collect();
    let index = mao_formats::instances::Instances::parse(&rdb.get(mao_formats::resource::Id { kind: 0xf424a, instance: 1 }).unwrap().unwrap()).unwrap();
    let mut missing = Vec::new();
    for e in db.entries().unwrap() {
        let (k, pos) = e.unwrap();
        if db.check(&k, pos).is_ok() {
            continue;
        }
        let (kind, inst) = (u32::from_be_bytes(k[..4].try_into().unwrap()), u32::from_be_bytes(k[4..].try_into().unwrap()));
        if !found.contains(&format!("{kind:x} {inst:x}")) {
            let name = index.iter().find(|(t, _, i)| *t == kind && *i == inst).map(|(_, n, _)| n.to_string()).unwrap_or_default();
            missing.push(format!("{kind:x}:{inst:x} {pos:#x} {name}"));
        }
    }
    println!("{} missing", missing.len());
    for m in &missing {
        println!("{m}");
    }
}
