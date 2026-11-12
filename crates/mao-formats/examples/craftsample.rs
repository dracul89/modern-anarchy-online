//! `craftsample <game> <word> [every nth]`: the descriptions of the items that mention a word, one in every n.
use mao_formats::item::Item;
use mao_formats::resource::{Id, ResourceDatabase};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = ResourceDatabase::open_game(std::path::Path::new(&args[0])).unwrap();
    let word = args[1].to_lowercase();
    let nth: usize = args.get(2).and_then(|n| n.parse().ok()).unwrap_or(1);
    let ids: Vec<Id> = db.ids().unwrap().filter_map(Result::ok).filter(|i| i.kind == mao_formats::item::TEMPLATE).collect();
    let mut seen = 0;
    for id in ids {
        let Some(item) = db.get(id).ok().flatten().and_then(|d| Item::parse(&d)) else { continue };
        if item.description.to_lowercase().contains(&word) {
            seen += 1;
            if seen % nth == 0 {
                println!("#{} {} :: {}", id.instance, item.name, item.description.replace('\n', " | ").replace('\r', ""));
            }
        }
    }
    eprintln!("{seen} items");
}
