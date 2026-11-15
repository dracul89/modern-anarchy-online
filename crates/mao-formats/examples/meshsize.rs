//! `meshsize <game> <mesh instance>...`: a mesh's extent as a dynel's (each part's vertices through its own matrix): the
//! least and greatest of each axis.
use mao_formats::resource::{Id, ResourceDatabase};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = ResourceDatabase::open_game(std::path::Path::new(&args[0])).unwrap();
    for id in &args[1..] {
        let instance: u32 = id.parse().unwrap();
        let object = db.get(Id { kind: 0xf6951, instance }).unwrap().expect("no such mesh");
        let mesh = mao_formats::mesh::parse(&object).unwrap();
        // (MAO_PLACED=1: as a statel instead)
        let model = if std::env::var_os("MAO_PLACED").is_some() { mao_formats::model::build_placed(&mesh).unwrap() } else { mao_formats::model::build_dynel(&mesh).unwrap() };
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        for p in &model.parts {
            let m = &p.world;
            let (mut plo, mut phi) = ([f32::MAX; 3], [f32::MIN; 3]);
            for v in &p.vertices {
                let q = v.position;
                let w = [q[0] * m[0] + q[1] * m[4] + q[2] * m[8] + m[12], q[0] * m[1] + q[1] * m[5] + q[2] * m[9] + m[13], q[0] * m[2] + q[1] * m[6] + q[2] * m[10] + m[14]];
                for k in 0..3 {
                    lo[k] = lo[k].min(w[k]);
                    hi[k] = hi[k].max(w[k]);
                    plo[k] = plo[k].min(w[k]);
                    phi[k] = phi[k].max(w[k]);
                }
            }
            // (MAO_PARTS=1: each part's own extent, with its material)
            if std::env::var_os("MAO_PARTS").is_some() {
                println!("  part \"{}\" material \"{}\": from {plo:?} to {phi:?}", p.name, p.material.name);
            }
        }
        println!("mesh {instance}: {} parts, from {lo:?} to {hi:?}", model.parts.len());
    }
}
