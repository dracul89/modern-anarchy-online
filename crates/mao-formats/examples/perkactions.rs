//! `perkactions <game>`: every perk record (0xf4264) with its template's name and its actions' criteria.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&args[0])).expect("the database");
    let word = |d: &[u8], at: usize| u32::from_le_bytes(d[at..at + 4].try_into().unwrap());
    let mut ids: Vec<_> = db.ids().unwrap().filter_map(Result::ok).filter(|i| i.kind == 0xf4264).collect();
    ids.sort_by_key(|i| i.instance);
    for id in ids {
        let r = db.get(id).unwrap().unwrap();
        let t = word(&r, 12);
        let Some(data) = db.get(mao_formats::resource::Id { kind: mao_formats::item::TEMPLATE, instance: t }).unwrap() else { continue };
        let name = mao_formats::item::Item::parse(&data).map(|i| i.name).unwrap_or_default();
        let acts: Vec<_> = mao_formats::item::actions(&data).into_iter().map(|a| (a.kind, a.criteria)).collect();
        println!("{} prev={} flags={:#x} prof={:#x} {:?} {:?}", id.instance, word(&r, 16), word(&r, 20), word(&r, 24), name, acts);
    }
}
