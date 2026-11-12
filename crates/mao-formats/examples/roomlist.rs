//! `roomlist <game> <playfield>...`: an interior's rooms: each one's name, its tile-map cells and its centre.
use mao_formats::resource::{Id, ResourceDatabase};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = ResourceDatabase::open_game(std::path::Path::new(&args[0])).unwrap();
    for pf in &args[1..] {
        let instance: u32 = pf.parse().unwrap();
        let Some(object) = db.get(Id { kind: mao_formats::rooms::KIND, instance }).unwrap() else { continue };
        let Ok(Some(rooms)) = mao_formats::rooms::parse(&object) else { continue };
        for (k, r) in rooms.rooms.iter().enumerate() {
            println!("{pf} room {k} {:?}: cells {:?}, centre {:?}, {} links", r.name, r.rect, r.position, r.links.len());
        }
    }
}
