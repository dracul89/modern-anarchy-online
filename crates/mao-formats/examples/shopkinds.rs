//! Lists the shop terminals of the game's data by their type (stat 156; none or 0 is 61, the kind a trade with
//! which is the original's `ShopTrade`; any other is a `ShopBuy`, a shop that only sells).
//! `shopkinds <game>` prints, for each type, how many item templates with shop prices (stat 427) have it and the
//! names of the first of them.

use mao_formats::resource::{Id, ResourceDatabase};

fn main() {
    let game = std::env::args().nth(1).expect("usage: shopkinds <game>");
    let db = ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap();
    let mut kinds: std::collections::BTreeMap<u32, Vec<(u32, String)>> = Default::default();
    let ids: Vec<Id> = db.ids().unwrap().flatten().filter(|id| id.kind == mao_formats::item::TEMPLATE).collect();
    for id in ids {
        let Ok(Some(data)) = db.get(id) else { continue };
        let Some(item) = mao_formats::item::Item::parse(&data) else { continue };
        if item.stat(427).is_none() && item.stat(426).is_none() {
            continue;
        }
        let kind = item.stat(156).filter(|&k| k != 0).unwrap_or(61);
        kinds.entry(kind).or_default().push((id.instance, item.name.trim().to_string()));
    }
    for (kind, shops) in &kinds {
        println!("type {kind}: {} templates", shops.len());
        for (instance, name) in shops.iter().take(12) {
            println!("    {instance:#x} {name}");
        }
    }
}
