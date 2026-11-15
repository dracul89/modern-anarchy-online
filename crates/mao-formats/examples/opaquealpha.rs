//! `opaquealpha <game>`: the meshes with a material that neither blends (render state 27) nor tests the alpha (15 with
//! a reference, 24) nor takes the alpha as light, but whose first texture is a PNG (the only textures that can carry an
//! alpha channel). The original draws those opaque: its device has the alpha test off and a material read from a mesh
//! keeps the states it was stored with (randy31 `FAFMaterial_t`'s reader). Listed by name with the material's name.
use mao_formats::resource::{Id, ResourceDatabase};
use std::collections::BTreeMap;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = ResourceDatabase::open_game(std::path::Path::new(&args[0])).unwrap();
    let names: BTreeMap<u32, String> = db
        .get(Id { kind: 0xf424a, instance: 1 })
        .ok()
        .flatten()
        .and_then(|o| mao_formats::instances::Instances::parse(&o).ok())
        .map(|index| index.iter().filter(|&(kind, _, _)| kind == 0xf6951).map(|(_, name, instance)| (instance, name.to_string())).collect())
        .unwrap_or_default();
    let ids: Vec<_> = db.ids().unwrap().filter_map(Result::ok).filter(|i| i.kind == 0xf6951).collect();
    let mut png: BTreeMap<u32, bool> = BTreeMap::new();
    let (mut meshes, mut found) = (0usize, 0usize);
    for id in ids {
        let Some(object) = db.get(id).ok().flatten() else { continue };
        let Ok(mesh) = mao_formats::mesh::parse(&object) else { continue };
        let Ok(model) = mao_formats::model::build(&mesh) else { continue };
        meshes += 1;
        let mut hits: Vec<String> = Vec::new();
        for part in &model.parts {
            let m = &part.material;
            if m.blend != mao_formats::model::Blend::None || m.alpha_ref.is_some() || m.alpha_lights || m.ignore_alpha {
                continue;
            }
            let Some(&(kind, instance)) = m.textures.first() else { continue };
            let is_png = *png.entry(instance).or_insert_with(|| db.get(Id { kind, instance }).ok().flatten().is_some_and(|d| d.len() > 16 && d[12..16] == [0x89, b'P', b'N', b'G']));
            if is_png && !hits.contains(&m.name) {
                hits.push(m.name.clone());
            }
        }
        if !hits.is_empty() {
            found += 1;
            println!("{} {:?}: {:?}", id.instance, names.get(&id.instance).map(String::as_str).unwrap_or(""), hits);
        }
    }
    println!("{meshes} meshes, {found} with such a material");
}
