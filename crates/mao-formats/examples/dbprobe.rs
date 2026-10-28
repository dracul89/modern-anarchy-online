//! `dbprobe <game> <type hex> <instance hex>`: the index entries for one key and what the data file holds there, and how many of the index's entries point at another record (an install whose index and data files disagree).
fn main() {
    let game = std::env::args().nth(1).unwrap();
    let kind = u32::from_str_radix(&std::env::args().nth(2).unwrap(), 16).unwrap();
    let inst = u32::from_str_radix(&std::env::args().nth(3).unwrap(), 16).unwrap();
    let db = mao_formats::ctree::Database::open(&std::path::Path::new(&game).join("cd_image/data/db/ResourceDatabase")).unwrap();
    let want = mao_formats::ctree::key(kind, inst);
    let mut bad = 0;
    let mut total = 0;
    for e in db.entries().unwrap() {
        let (k, pos) = e.unwrap();
        total += 1;
        let ok = db.check(&k, pos).is_ok();
        if !ok {
            bad += 1;
        }
        if k == want {
            let rec = db.record_at(pos).map(|r| (r.len(), r.get(..8).map(|b| b.to_vec())));
            println!("entry at {pos:#x}: check {:?} record {:?}", db.check(&k, pos), rec.map(|r| (r.0, r.1)));
        }
    }
    println!("{bad} of {total} entries don't check");
}
