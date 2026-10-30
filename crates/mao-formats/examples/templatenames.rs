//! `templatenames <game> <id>...`: the item templates' names.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&args[0])).expect("the database");
    for id in &args[1..] {
        let Ok(instance) = id.parse() else { continue };
        let item = db.get(mao_formats::resource::Id { kind: mao_formats::item::TEMPLATE, instance }).ok().flatten().and_then(|d| mao_formats::item::Item::parse(&d));
        println!("{id} {:?}", item.map(|i| i.name));
    }
}
