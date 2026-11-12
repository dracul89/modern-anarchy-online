//! `craftbooks <game> [least length]`: the items whose descriptions are long texts that speak of making things (the
//! game's own guides, written on items), with each text's length; and the names that hold `Library`, `Emitter` or
//! `Program Crystal`.
use mao_formats::item::Item;
use mao_formats::resource::{Id, ResourceDatabase};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = ResourceDatabase::open_game(std::path::Path::new(&args[0])).unwrap();
    let least: usize = args.get(1).and_then(|n| n.parse().ok()).unwrap_or(900);
    let ids: Vec<Id> = db.ids().unwrap().filter_map(Result::ok).filter(|i| i.kind == mao_formats::item::TEMPLATE).collect();
    let mut seen = std::collections::BTreeSet::new();
    let mut parts = std::collections::BTreeSet::new();
    for id in ids {
        let Some(item) = db.get(id).ok().flatten().and_then(|d| Item::parse(&d)) else { continue };
        if ["Library", "Emitter", "Program Crystal"].iter().any(|w| item.name.contains(w)) && !item.name.contains('(') {
            parts.insert(item.name.clone());
        }
        if item.description.len() >= least && mao_formats::crafting::speaks_of_making(&item.description) && seen.insert(item.name.clone()) {
            println!("{} chars  #{}  {}", item.description.len(), id.instance, item.name);
        }
    }
    println!("parts: {parts:?}");
}
