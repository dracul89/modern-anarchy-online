//! `blendnames <game> <text>`: the meshes whose names hold the text, each material with its render states (type = value)
//! and its first texture.
use mao_formats::resource::{Id, ResourceDatabase};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = ResourceDatabase::open_game(std::path::Path::new(&args[0])).unwrap();
    let index = mao_formats::instances::Instances::parse(&db.get(Id { kind: 0xf424a, instance: 1 }).unwrap().unwrap()).unwrap();
    let want = args[1].to_lowercase();
    for (kind, name, instance) in index.iter() {
        if kind != 0xf6951 || !name.to_lowercase().contains(&want) {
            continue;
        }
        let Some(object) = db.get(Id { kind, instance }).ok().flatten() else { continue };
        let Ok(mesh) = mao_formats::mesh::parse(&object) else { continue };
        let a = &mesh.archive;
        println!("{instance} {name}");
        for (i, o) in a.objects.iter().enumerate() {
            if a.class(i) != Some("RDeltaState") {
                continue;
            }
            let words = |n: &str| -> Vec<u32> { o.field("", n).map(|f| f.values.iter().filter(|v| v.len() == 4).map(|v| u32::from_le_bytes(v[..4].try_into().unwrap())).collect()).unwrap_or_default() };
            let states: Vec<String> = words("rst_type").into_iter().zip(words("rst_value")).map(|(t, v)| format!("{t}={v:#x}")).collect();
            let texture = o.field("0", "tch_text").and_then(|f| f.values.first()).and_then(|v| a.objects.get(u32::from_le_bytes(v[..4].try_into().unwrap()) as usize)).and_then(|t| t.int("0", "creator")).and_then(|c| a.objects.get(c as usize)).and_then(|c| c.int("", "inst"));
            println!("  states [{}] texture {texture:?}", states.join(" "));
        }
    }
}
