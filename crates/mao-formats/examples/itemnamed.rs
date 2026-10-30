//! `itemnamed <game> <text>`: the item templates whose name contains `text`, with their stats.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&args[0])).expect("the database");
    for id in db.ids().unwrap().filter_map(Result::ok).filter(|i| i.kind == mao_formats::item::TEMPLATE) {
        let Some(item) = db.get(id).ok().flatten().and_then(|d| mao_formats::item::Item::parse(&d)) else { continue };
        if item.name.contains(&args[1]) || item.description.contains(&args[1]) {
            println!("{} {:?} {:?} {:?}", id.instance, item.name, item.description, item.stats);
        }
    }
}
