//! A player city's buildings. The server lists a city playfield's houses in `PlayfieldAllCities`
//! (`mao_net::landcontrol::houses`): a place, a rotation and a template each. The original makes no object of the game
//! for a house (city.dll `PlayfieldCityHolderClient_c::AddNewHouse`, 0x10016151): for each of the template's meshes a
//! plain `VisualMesh_t`, as a statel's, at the house's place plus the mesh's own place turned with the house
//! (`RotateLocalPos`), turned about the up axis by minus the sum of the two rotations
//! (`AOPlayfieldCityHolderClient_c::EnableVisibility`, Gamecode 0x1012eadb), and the template's surface, turned the same
//! way and moved to the house's place, added to the playfield's (`MakeHouseSurfaceAtTilePos`, 0x100168e7). The doors are
//! the server's own objects.
//!
//! Here the houses go into the playfield's scene and collision as its statels do. Not done: a house added or taken
//! away while the character is in the city (`AOTransportSignal` 0x62741e15, codes 2 and 9) and the building effect.

use mao_formats::model::{self, Model};
use mao_formats::resource::{Id, ResourceDatabase};
use mao_net::landcontrol::House;
use std::collections::HashMap;

/// The houses of a blob given as hex in `MAO_HOUSES` (the `PlayfieldAllCities` message's bytes after its three-byte
/// head), for the offline views.
pub(crate) fn from_env() -> Option<Vec<House>> {
    let text = std::env::var("MAO_HOUSES").ok()?;
    let bytes: Vec<u8> = (0..text.len() / 2).filter_map(|i| u8::from_str_radix(text.get(2 * i..2 * i + 2)?, 16).ok()).collect();
    mao_net::landcontrol::houses(&bytes)
}

/// Puts the houses' meshes into `scene` and their surfaces into `collision` (and into `zones`, the surfaces the
/// character walks on, when the playfield has them). Says in the log what each house is and where the server should
/// have put its doors (`House_c::CalculateGlobalDoorPosRot`: the same turn of the door's place), to hold against the
/// doors the server announces.
pub(crate) fn add(db: &ResourceDatabase, houses: &[House], scene: &mut Model, collision: &mut mao_formats::collision::Collision, mut zones: Option<&mut mao_formats::zones::Zones>) {
    let swap = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0];
    let mut templates: HashMap<u32, Option<mao_formats::house::Template>> = HashMap::new();
    let mut meshes: HashMap<u32, Option<std::ops::Range<usize>>> = HashMap::new();
    let (mut placed, mut walls) = (0, 0);
    for (number, house) in houses.iter().enumerate() {
        let template = templates.entry(house.template.1).or_insert_with(|| {
            let record = db.get(Id { kind: house.template.0, instance: house.template.1 }).ok().flatten()?;
            mao_formats::house::Template::parse(&record)
        });
        let Some(template) = template else {
            eprintln!("city: house {number} at {:?}: no template {:#x}:{:#x}", house.position, house.template.0, house.template.1);
            continue;
        };
        let at = |local: [f32; 3]| {
            let p = template.rotate_local(local, house.rotation);
            [house.position[0] + p[0], house.position[1] + p[1], house.position[2] + p[2]]
        };
        for visual in &template.visuals {
            let range = meshes
                .entry(visual.mesh)
                .or_insert_with(|| {
                    let m = crate::load_model(db, Id { kind: 0xf6951, instance: visual.mesh }).ok()?;
                    let start = scene.parts.len();
                    scene.parts.extend(m.parts);
                    let full = start..scene.parts.len();
                    // (the far twin, as a statel's: the original starts from it and its `LODManager` swaps the full one in)
                    let low = crate::load_model(db, Id { kind: 0xf696a, instance: visual.mesh }).ok().filter(|m| !m.parts.is_empty()).map(|m| {
                        let low_start = scene.parts.len();
                        scene.parts.extend(m.parts);
                        low_start..scene.parts.len()
                    });
                    scene.lod_groups.push((full.clone(), low));
                    Some(full)
                })
                .clone();
            let Some(range) = range else {
                eprintln!("city: house {number}: mesh {} is not in the database", visual.mesh);
                continue;
            };
            let place = model::multiply(&model::trs_matrix(at(visual.position), mao_formats::house::heading(visual.rotation + house.rotation), [1.0; 3]), &swap);
            scene.instances.extend(range.map(|i| (i, place)));
            placed += 1;
        }
        if let Some(triangles) = template.triangles(house.rotation) {
            let moved: Vec<mao_formats::collision::Triangle> = triangles.into_iter().map(|t| t.map(|v| [v[0] + house.position[0], v[1] + house.position[1], v[2] + house.position[2]])).collect();
            walls += moved.len();
            if let Some(zones) = zones.as_deref_mut() {
                zones.add(&moved);
            }
            // (the collision's own frame has the height last)
            for t in moved {
                collision.add(t.map(|v| [v[0], v[2], v[1]]));
            }
        }
        let doors: Vec<String> = template.doors.iter().map(|d| { let p = at(d.position); format!("{:#x} at ({:.2}, {:.2}, {:.2})", d.template, p[0], p[1], p[2]) }).collect();
        eprintln!("city: house {number} at ({}, {}, {}) turned {}: template {:#x} (type {}, {} by {} tiles), meshes {:?}, doors [{}]", house.position[0], house.position[1], house.position[2], house.rotation, house.template.1, template.kind, template.tiles_x, template.tiles_z, template.visuals.iter().map(|v| v.mesh).collect::<Vec<_>>(), doors.join(", "));
    }
    eprintln!("city: {} houses, {placed} meshes placed, {walls} triangles of their surfaces", houses.len());
}
