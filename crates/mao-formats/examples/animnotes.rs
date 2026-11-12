//! `animnotes <game> <animation file name>...`: an animation's length and its notes (`loopstart`, `loopend` and the rest).
use mao_formats::resource::{Id, ResourceDatabase};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = ResourceDatabase::open_game(std::path::Path::new(&args[0])).unwrap();
    let index = mao_formats::instances::Instances::parse(&db.get(Id { kind: 0xf424a, instance: 1 }).unwrap().unwrap()).unwrap();
    for name in &args[1..] {
        let Some(instance) = index.find(0xf6953, name) else {
            println!("{name}: no such animation");
            continue;
        };
        let anim = mao_formats::catanim::parse(&db.get(Id { kind: 0xf6953, instance }).unwrap().unwrap()).unwrap();
        let length = anim.tracks.iter().flat_map(|t| t.rotations.iter().map(|k| k.0)).fold(0.0f32, f32::max);
        println!("{name} ({instance}): length {length}, notes {:?}", anim.notes.iter().map(|n| (n.time, n.name.as_str())).collect::<Vec<_>>());
    }
}
