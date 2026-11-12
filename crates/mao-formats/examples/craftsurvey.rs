//! `craftsurvey <game>`: what the item templates say about crafting. The kinds of action the items have and how many
//! items have each; for each kind, the stats its requirements ask most often; and how many descriptions speak of
//! combining. (The recipes themselves are the server's: see the client's `docs/TRADESKILL_GUIDE.md`.)
use mao_formats::item::Item;
use mao_formats::resource::{Id, ResourceDatabase};
use std::collections::BTreeMap;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = ResourceDatabase::open_game(std::path::Path::new(&args[0])).unwrap();
    let ids: Vec<Id> = db.ids().unwrap().filter_map(Result::ok).filter(|i| i.kind == mao_formats::item::TEMPLATE).collect();
    let mut kinds: BTreeMap<u32, (u32, BTreeMap<u32, u32>, Vec<String>)> = BTreeMap::new();
    let mut words: BTreeMap<&str, u32> = BTreeMap::new();
    let mut total = 0;
    for id in ids {
        let Some(data) = db.get(id).ok().flatten() else { continue };
        let Some(item) = Item::parse(&data) else { continue };
        total += 1;
        for action in mao_formats::item::actions(&data) {
            let entry = kinds.entry(action.kind).or_default();
            entry.0 += 1;
            for (stat, _, _) in &action.criteria {
                *entry.1.entry(*stat).or_default() += 1;
            }
            if entry.2.len() < 4 {
                entry.2.push(item.name.clone());
            }
        }
        let text = item.description.to_lowercase();
        for word in ["combine", "tradeskill", "use this on", "used on", "can be used", "to create", "to make", "assemble", "process"] {
            if text.contains(word) {
                *words.entry(word).or_default() += 1;
            }
        }
    }
    println!("{total} item templates");
    for (kind, (count, stats, examples)) in kinds {
        let mut top: Vec<(u32, u32)> = stats.into_iter().collect();
        top.sort_by(|a, b| b.1.cmp(&a.1));
        println!("action {kind}: {count} items; stats asked most: {:?}; e.g. {:?}", &top[..top.len().min(8)], examples);
    }
    println!("descriptions that say: {words:?}");
}
