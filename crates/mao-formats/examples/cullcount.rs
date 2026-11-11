//! `cullcount <game>`: how the meshes' materials set the cull mode (render state 22: 1 none, both sides drawn; 2 the
//! clockwise faces culled; 3 the counter-clockwise ones, which is also the device's own state in randy31), by delta
//! states and by the meshes that have one of each; and how many of the both-sided ones are also blended.
use mao_formats::resource::ResourceDatabase;
use std::collections::BTreeMap;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = ResourceDatabase::open_game(std::path::Path::new(&args[0])).unwrap();
    for kind in [0xf6951u32, 0xf696a] {
        let ids: Vec<_> = db.ids().unwrap().filter_map(Result::ok).filter(|i| i.kind == kind).collect();
        let (mut meshes, mut states) = (0usize, 0usize);
        let mut by_value: BTreeMap<Option<u32>, (usize, usize, usize)> = BTreeMap::new();
        for id in ids {
            let Some(object) = db.get(id).ok().flatten() else { continue };
            let Ok(mesh) = mao_formats::mesh::parse(&object) else { continue };
            meshes += 1;
            let a = &mesh.archive;
            let mut seen: Vec<Option<u32>> = Vec::new();
            for (i, o) in a.objects.iter().enumerate() {
                if a.class(i) != Some("RDeltaState") {
                    continue;
                }
                states += 1;
                let words = |name: &str| -> Vec<u32> { o.field("", name).or_else(|| o.field("2", name)).map(|f| f.values.iter().filter(|v| v.len() == 4).map(|v| u32::from_le_bytes(v[..4].try_into().unwrap())).collect()).unwrap_or_default() };
                let set: Vec<(u32, u32)> = words("rst_type").into_iter().zip(words("rst_value")).collect();
                let cull = set.iter().find(|s| s.0 == 22).map(|s| s.1);
                let entry = by_value.entry(cull).or_default();
                entry.0 += 1;
                entry.2 += set.contains(&(27, 1)) as usize;
                if !seen.contains(&cull) {
                    seen.push(cull);
                    entry.1 += 1;
                }
            }
        }
        println!("kind {kind:#x}: {meshes} meshes, {states} delta states");
        for (value, (n, m, blended)) in by_value {
            match value {
                Some(v) => println!("  cull mode {v}: {n} delta states in {m} meshes ({blended} blended)"),
                None => println!("  not set: {n} delta states in {m} meshes ({blended} blended)"),
            }
        }
    }
}
