//! `vehicleprograms <game>`: every vehicle program (a nano that changes the body's mesh by the body), with its 0xcf4c
//! (movement flags) spells' arguments.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&args[0])).expect("the database");
    let ids: Vec<_> = db.ids().unwrap().filter_map(Result::ok).filter(|i| i.kind == 0xfde85).collect();
    for id in ids {
        let Some(data) = db.get(id).ok().flatten() else { continue };
        if !mao_formats::item::is_vehicle_program(&data) {
            continue;
        }
        let name = mao_formats::item::Item::parse(&data).map(|i| i.name).unwrap_or_default();
        let flags: Vec<String> = mao_formats::spell::lists(&data).into_iter().flat_map(|(e, s)| s.into_iter().filter(|s| s.kind == 0xcf4c).map(move |s| format!("ev{e} {:?}", s.stats))).collect();
        println!("{} {name}: {}", id.instance, flags.join("; "));
    }
}
