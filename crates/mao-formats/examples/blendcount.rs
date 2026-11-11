//! `blendcount <game>`: how many meshes have a material whose delta state turns alpha blending on (render state 27), and
//! which other render states those materials set, by how many materials set each (type, value).
use mao_formats::resource::ResourceDatabase;
use std::collections::BTreeMap;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = ResourceDatabase::open_game(std::path::Path::new(&args[0])).unwrap();
    let ids: Vec<_> = db.ids().unwrap().filter_map(Result::ok).filter(|i| i.kind == 0xf6951).collect();
    let (mut meshes, mut with, mut materials, mut blended) = (0usize, 0usize, 0usize, 0usize);
    let mut states: BTreeMap<(u32, u32), usize> = BTreeMap::new();
    for id in ids {
        let Some(object) = db.get(id).ok().flatten() else { continue };
        let Ok(mesh) = mao_formats::mesh::parse(&object) else { continue };
        meshes += 1;
        let a = &mesh.archive;
        let mut any = false;
        for (i, o) in a.objects.iter().enumerate() {
            if a.class(i) != Some("RDeltaState") {
                continue;
            }
            materials += 1;
            let words = |name: &str| -> Vec<u32> { o.field("", name).or_else(|| o.field("2", name)).map(|f| f.values.iter().filter(|v| v.len() == 4).map(|v| u32::from_le_bytes(v[..4].try_into().unwrap())).collect()).unwrap_or_default() };
            let set: Vec<(u32, u32)> = words("rst_type").into_iter().zip(words("rst_value")).collect();
            if set.contains(&(27, 1)) {
                blended += 1;
                any = true;
                for s in set {
                    *states.entry(s).or_default() += 1;
                }
            }
        }
        with += any as usize;
    }
    println!("{meshes} meshes, {with} with a blended material; {materials} delta states, {blended} blended");
    for ((t, v), n) in states {
        println!("  render state {t} = {v:#x}: {n}");
    }
}
