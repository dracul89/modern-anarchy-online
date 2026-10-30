//! `dynelnames <game> <text> [playfield]`: the table dynels whose template's name contains `text`, with their tails.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&args[0])).expect("the database");
    let want = args.get(1).cloned().unwrap_or_default();
    let only: Option<u32> = args.get(2).and_then(|a| a.parse().ok());
    let ids: Vec<_> = db.ids().unwrap().filter_map(Result::ok).filter(|i| i.kind == mao_formats::dynels::KIND && only.is_none_or(|p| p == i.instance)).collect();
    for id in ids {
        let Some(object) = db.get(id).ok().flatten() else { continue };
        for d in mao_formats::dynels::parse(&object) {
            let name = db.get(mao_formats::resource::Id { kind: mao_formats::item::TEMPLATE, instance: d.template }).ok().flatten().and_then(|x| mao_formats::item::Item::parse(&x)).map_or_else(String::new, |i| i.name);
            if !name.contains(&want) {
                continue;
            }
            let words: Vec<String> = d.tail.iter().map(|w| format!("{:x}", w.swap_bytes())).collect();
            println!("pf {} {:x}:{:x} t{} {:?} at {:?} stats {:?}\n  {}", id.instance, d.kind, d.instance, d.template, name, d.position, d.stats, words.join(" "));
        }
    }
}
