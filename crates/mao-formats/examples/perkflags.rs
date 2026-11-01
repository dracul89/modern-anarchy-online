//! `perkflags <game>`: the perk templates' flags (stat 30) and equipment class (stat 76), counted.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&args[0])).expect("the database");
    let word = |d: &[u8], at: usize| u32::from_le_bytes(d[at..at + 4].try_into().unwrap());
    let mut counts: std::collections::BTreeMap<(Option<i32>, Option<i32>), usize> = Default::default();
    for id in db.ids().unwrap().filter_map(Result::ok).filter(|i| i.kind == 0xf4264) {
        let r = db.get(id).unwrap().unwrap();
        let Some(data) = db.get(mao_formats::resource::Id { kind: mao_formats::item::TEMPLATE, instance: word(&r, 12) }).unwrap() else { continue };
        let Some(item) = mao_formats::item::Item::parse(&data) else { continue };
        *counts.entry((item.stat(30).map(|v| v as i32), item.stat(76).map(|v| v as i32))).or_default() += 1;
    }
    for ((f, c), n) in counts {
        println!("flags {:?} class {c:?}: {n}", f.map(|f| format!("{f:#x}")));
    }
}
