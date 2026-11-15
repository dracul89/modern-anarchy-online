//! `pfdynels <game> <playfield>`: the playfield's built-in objects (its dynel table): type, instance, template and its
//! name, the mesh (the record's own stat 12 or the template's) and the place.
use mao_formats::resource::{Id, ResourceDatabase};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = ResourceDatabase::open_game(std::path::Path::new(&args[0])).unwrap();
    let playfield: u32 = args[1].parse().unwrap();
    let Some(object) = db.get(Id { kind: mao_formats::dynels::KIND, instance: playfield }).ok().flatten() else {
        println!("no dynel table");
        return;
    };
    for (i, d) in mao_formats::dynels::parse(&object).into_iter().enumerate() {
        let item = db.get(Id { kind: mao_formats::item::TEMPLATE, instance: d.template }).ok().flatten().and_then(|data| mao_formats::item::Item::parse(&data));
        let name = item.as_ref().map_or_else(String::new, |t| t.name.clone());
        let mesh = d.stat(12).map(|m| (m, "own")).or_else(|| item.as_ref().and_then(|t| t.stat(12)).map(|m| (m, "template")));
        println!("#{i} type {:#x} instance {:#x} template {:#x} {name:?} mesh {mesh:?} at {:?} rot {:?} stats {:?}", d.kind, d.instance, d.template, d.position, d.rotation, d.stats);
    }
}
