//! `creatureanims <game> <npc template>...`: a creature record's animations: each action's number and the animation
//! file it names (with the file's name from the instance index, when it has one).
use mao_formats::resource::{Id, ResourceDatabase};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = ResourceDatabase::open_game(std::path::Path::new(&args[0])).expect("the database");
    let index = mao_formats::instances::Instances::parse(&db.get(Id { kind: 0xf424a, instance: 1 }).unwrap().unwrap()).unwrap();
    let names: std::collections::HashMap<u32, String> = index.iter().filter(|(kind, _, _)| *kind == 0xf6953).map(|(_, name, instance)| (instance, name.to_string())).collect();
    for id in &args[1..] {
        let instance = if let Some(h) = id.strip_prefix("0x") { u32::from_str_radix(h, 16).unwrap() } else { id.parse().unwrap() };
        let Some(record) = db.get(Id { kind: mao_formats::npc::TEMPLATE, instance }).ok().flatten().and_then(|d| mao_formats::npc::Template::parse(&d)) else {
            println!("{id}: no record");
            continue;
        };
        println!("{id}: mesh {:?}, {} animations", record.mesh, record.animations.len());
        for (action, anim) in &record.animations {
            println!("  action {action:#x} ({action}) -> {anim} {}", names.get(anim).map_or("", String::as_str));
        }
    }
}
