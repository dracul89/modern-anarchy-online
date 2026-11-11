//! `meshboxes <game> <mesh file name>...`: each named mesh's parts as an attached mesh (`model::build`): the part's name,
//! its triangle count and the box it fills in the mesh's own space (the creation hall's meshes share one space).
use mao_formats::resource::{Id, ResourceDatabase};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = ResourceDatabase::open_game(std::path::Path::new(&args[0])).unwrap();
    let index = mao_formats::instances::Instances::parse(&db.get(Id { kind: 0xf424a, instance: 1 }).unwrap().unwrap()).unwrap();
    for name in &args[1..] {
        let Some(instance) = index.find(0xf6951, name) else {
            println!("{name}: no such mesh");
            continue;
        };
        let object = db.get(Id { kind: 0xf6951, instance }).unwrap().expect("no such mesh");
        let mesh = mao_formats::mesh::parse(&object).unwrap();
        let model = mao_formats::model::build(&mesh).unwrap();
        let mut boxes: Vec<(String, usize, [f32; 3], [f32; 3])> = Vec::new();
        for p in &model.parts {
            let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
            for v in &p.vertices {
                let (x, y, z) = (v.position[0], v.position[1], v.position[2]);
                let m = &p.world;
                let w = [m[0] * x + m[4] * y + m[8] * z + m[12], m[1] * x + m[5] * y + m[9] * z + m[13], m[2] * x + m[6] * y + m[10] * z + m[14]];
                for k in 0..3 {
                    lo[k] = lo[k].min(w[k]);
                    hi[k] = hi[k].max(w[k]);
                }
            }
            match boxes.iter_mut().find(|b| b.0 == p.name) {
                Some(b) => {
                    b.1 += p.indices.len() / 3;
                    for k in 0..3 {
                        b.2[k] = b.2[k].min(lo[k]);
                        b.3[k] = b.3[k].max(hi[k]);
                    }
                }
                None => boxes.push((p.name.clone(), p.indices.len() / 3, lo, hi)),
            }
        }
        println!("{name} ({instance}): {} parts", model.parts.len());
        for (part, triangles, lo, hi) in boxes {
            println!("  {part:40} {triangles:6} triangles  x {:8.1}..{:8.1}  y {:8.1}..{:8.1}  z {:8.1}..{:8.1}", lo[0], hi[0], lo[1], hi[1], lo[2], hi[2]);
        }
    }
}
