//! `meshparts <game> <mesh instance>...`: a mesh's drawable parts as a dynel's: each part's name, vertex and triangle counts,
//! the range of its texture coordinates and its material.
use mao_formats::resource::{Id, ResourceDatabase};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = ResourceDatabase::open_game(std::path::Path::new(&args[0])).unwrap();
    for id in &args[1..] {
        let instance: u32 = id.parse().unwrap();
        let object = db.get(Id { kind: 0xf6951, instance }).unwrap().expect("no such mesh");
        let mesh = mao_formats::mesh::parse(&object).unwrap();
        let (model, anims) = mao_formats::model::build_dynel_animated(&mesh).unwrap();
        for (p, a) in model.parts.iter().zip(&anims) {
            let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
            for v in &p.vertices {
                for k in 0..2 {
                    lo[k] = lo[k].min(v.uv[k]);
                    hi[k] = hi[k].max(v.uv[k]);
                }
            }
            println!("mesh {instance} part {:?}: {} vertices, {} triangles, uv {lo:?}..{hi:?}, animated {} (length {:?})", p.name, p.vertices.len(), p.indices.len() / 3, a.is_some(), a.as_ref().map(|a| a.length()));
            println!("  {:?}", p.material);
        }
    }
}
