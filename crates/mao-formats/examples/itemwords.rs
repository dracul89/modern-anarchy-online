//! `itemwords <game> <type hex> <instance>`: an item template's words after its name and description (the tables).
use mao_formats::item::Item;
use mao_formats::resource::{Id, ResourceDatabase};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let db = ResourceDatabase::open_game(std::path::Path::new(&args[1])).unwrap();
    let kind = u32::from_str_radix(&args[2], 16).unwrap();
    let instance: u32 = args[3].parse().unwrap();
    let data = db.get(Id { kind, instance }).unwrap().unwrap();
    let it = Item::parse(&data).unwrap();
    let skip = 4 * (4 + 2 * it.stats.len() + 1) + it.name.len() + it.description.len();
    println!("{} ({} bytes, tables from {skip})", it.name, data.len());
    let rest = &data[skip..];
    let words: Vec<String> = rest.chunks(4).map(|c| {
        let mut b = [0u8; 4];
        b[..c.len()].copy_from_slice(c);
        format!("{:x}", u32::from_le_bytes(b))
    }).collect();
    println!("{}", words.join(" "));
}
