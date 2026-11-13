//! `untextured <game>`: a survey of the static meshes (type 0xf6951) for geometry a level designer left as a marker:
//! the triangle meshes whose name holds "invis" in any case (the original hides the ones with the lower-case word:
//! `model::is_hidden_name`), the ones whose name starts with a bracket (a tag such as the occluder's `[OCC]`), and the other
//! parts whose material has no texture; each mesh with how many statels place it.
//! Run it with `MAO_OCCLUDERS=show MAO_HIDDEN_MESHES=show` so that the reader leaves nothing out.
use mao_formats::resource::{Id, ResourceDatabase};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = ResourceDatabase::open_game(std::path::Path::new(&args[0])).unwrap();
    let index = mao_formats::instances::Instances::parse(&db.get(Id { kind: 0xf424a, instance: 1 }).unwrap().unwrap()).unwrap();
    let names: std::collections::HashMap<u32, String> = index.iter().filter(|&(kind, _, _)| kind == 0xf6951).map(|(_, name, instance)| (instance, name.to_string())).collect();
    // How many statels place each mesh, and in how many playfields.
    let mut placed: std::collections::HashMap<u32, (usize, std::collections::BTreeSet<u32>)> = std::collections::HashMap::new();
    for p in mao_formats::playfield::all(&db) {
        let path = std::path::Path::new(&args[0]).join(format!("cd_image/data/statels/{}.pf", p.number));
        let Some(list) = mao_formats::files::read(&path).ok().and_then(|file| mao_formats::statels::parse(&file).ok()) else { continue };
        for s in list {
            let e = placed.entry(s.mesh).or_default();
            e.0 += 1;
            e.1.insert(p.number);
        }
    }
    let mut rows = Vec::new();
    let (mut meshes, mut failed) = (0usize, 0usize);
    // (kind of part) -> (parts, meshes, statels placing those meshes, of the parts how many have a texture)
    let mut tally: std::collections::BTreeMap<&str, (usize, usize, usize, usize)> = std::collections::BTreeMap::new();
    let mut tags: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut odd_case: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for id in db.ids().unwrap().flatten().filter(|i| i.kind == 0xf6951) {
        let Some(object) = db.get(id).ok().flatten() else { continue };
        meshes += 1;
        let Some(model) = mao_formats::mesh::parse(&object).ok().and_then(|m| mao_formats::model::build_placed(&m).ok()) else {
            failed += 1;
            continue;
        };
        let (count, fields) = placed.get(&id.instance).map(|(n, f)| (*n, f.len())).unwrap_or((0, 0));
        let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        let mut notes = Vec::new();
        for p in &model.parts {
            let lower = p.name.to_lowercase();
            let kind = if p.name.starts_with('[') {
                *tags.entry(p.name.split(']').next().unwrap_or("").to_string() + "]").or_default() += 1;
                "bracket tag"
            } else if p.name.contains("invis") {
                "invis (hidden by the original)"
            } else if lower.contains("invis") {
                *odd_case.entry(p.name.clone()).or_default() += 1;
                "invis in another case (drawn by the original)"
            } else if p.material.textures.is_empty() {
                "untextured, another name"
            } else {
                continue;
            };
            let t = tally.entry(kind).or_default();
            t.0 += 1;
            t.3 += !p.material.textures.is_empty() as usize;
            if seen.insert(kind) {
                t.1 += 1;
                t.2 += count;
                notes.push(format!("{kind}: {:?} diffuse {:.2?} opacity {:.2} textures {}", p.name, p.material.diffuse, p.material.opacity, p.material.textures.len()));
            }
        }
        if !notes.is_empty() {
            rows.push((count, fields, id.instance, names.get(&id.instance).cloned().unwrap_or_default(), model.parts.len(), notes.join("; ")));
        }
    }
    rows.sort_by(|a, b| b.0.cmp(&a.0).then(a.2.cmp(&b.2)));
    for (count, fields, instance, name, parts, notes) in &rows {
        println!("{count} statels in {fields} playfields: {instance} {name:?} ({parts} parts): {notes}");
    }
    println!("{} of {meshes} meshes ({failed} not read)", rows.len());
    for (kind, (parts, meshes, statels, textured)) in &tally {
        println!("{kind}: {parts} parts ({textured} with a texture) in {meshes} meshes, placed by {statels} statels");
    }
    println!("bracket tags: {tags:?}");
    println!("other cases: {odd_case:?}");
}
