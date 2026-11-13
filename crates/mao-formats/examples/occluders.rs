//! `occluders <game>`: the static meshes (type 0xf6951) that hold an occluder's body (a triangle mesh whose data's name
//! starts with `[OCC]`, which the original never draws: `model::is_occluder_name`), with their file names, and how many
//! statels of how many playfields place one.
use mao_formats::resource::{Id, ResourceDatabase};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = ResourceDatabase::open_game(std::path::Path::new(&args[0])).unwrap();
    let index = mao_formats::instances::Instances::parse(&db.get(Id { kind: 0xf424a, instance: 1 }).unwrap().unwrap()).unwrap();
    let names: std::collections::HashMap<u32, String> = index.iter().filter(|&(kind, _, _)| kind == 0xf6951).map(|(_, name, instance)| (instance, name.to_string())).collect();
    let mut found = Vec::new();
    for id in db.ids().unwrap().flatten().filter(|i| i.kind == 0xf6951) {
        let Some(object) = db.get(id).ok().flatten() else { continue };
        // (the tag's bytes first: parsing every mesh takes minutes)
        if !object.windows(5).any(|w| w == b"[OCC]") {
            continue;
        }
        let Ok(mesh) = mao_formats::mesh::parse(&object) else { continue };
        let a = &mesh.archive;
        let (mut bodies, mut others) = (0, 0);
        for (i, o) in a.objects.iter().enumerate() {
            if a.class(i) != Some("FAFTriMeshData_t") {
                continue;
            }
            let name = o.field("", "name").and_then(|f| f.values.first()).map(|v| String::from_utf8_lossy(v).trim_end_matches('\0').to_string()).unwrap_or_default();
            if mao_formats::model::is_occluder_name(&name) {
                bodies += 1;
            } else {
                others += 1;
            }
        }
        println!("{} {:?}: {bodies} occluder bodies, {others} other meshes", id.instance, names.get(&id.instance));
        found.push(id.instance);
    }
    println!("{} meshes", found.len());
    let (mut statels, mut playfields) = (0, 0);
    for p in mao_formats::playfield::all(&db) {
        let path = std::path::Path::new(&args[0]).join(format!("cd_image/data/statels/{}.pf", p.number));
        let Some(list) = mao_formats::files::read(&path).ok().and_then(|file| mao_formats::statels::parse(&file).ok()) else { continue };
        let n = list.iter().filter(|s| found.contains(&s.mesh)).count();
        if n > 0 {
            println!("  {} {}: {n}", p.number, p.name.trim());
            statels += n;
            playfields += 1;
        }
    }
    println!("{statels} statels in {playfields} playfields");
}
