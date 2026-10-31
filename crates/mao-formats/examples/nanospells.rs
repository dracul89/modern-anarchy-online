//! `nanospells <game> <id>`: a nano program's (0xfde85) stats and its events' spells.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&args[0])).expect("the database");
    let t: u32 = args[1].parse().unwrap();
    let data = db.get(mao_formats::resource::Id { kind: 0xfde85, instance: t }).unwrap().expect("no such nano");
    let item = mao_formats::item::Item::parse(&data).unwrap();
    println!("{:?} {:?}", item.name, item.stats);
    for (event, spells) in mao_formats::spell::lists(&data) {
        for s in spells {
            println!("event {event}: {s:?}");
        }
    }
}
