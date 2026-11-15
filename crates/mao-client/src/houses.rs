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

/// What the city's map draws and tests of a house's template: its type, its footprint and its doors' and guards'
/// places.
pub(crate) fn shape(db: &ResourceDatabase, template: u32) -> Option<crate::citymap::Shape> {
    let record = db.get(Id { kind: mao_formats::house::KIND, instance: template }).ok().flatten()?;
    let t = mao_formats::house::Template::parse(&record)?;
    Some(crate::citymap::Shape { kind: t.kind, tiles: (t.tiles_x, t.tiles_z), mask: t.mask, doors: t.doors.iter().map(|d| d.position).collect(), guards: t.guards })
}

/// The map's data: the city's houses as the map's nodes; the building items of the backpack (an item whose
/// `HouseTemplate`, stat 620, names a house template that reads) with the side each is for (stat 33); and the zones,
/// the playfield's size and the land's costs from the playfield's city record (the controller that is the city's:
/// the high half of its instance is the city's number).
pub(crate) fn map_data(db: &ResourceDatabase, playfield: u32, city: u32, houses: &[House], backpack: &[(u32, u32)], stat: &dyn Fn(u32, u32) -> Option<i64>) -> crate::citymap::Data {
    let mut shapes: HashMap<u32, Option<crate::citymap::Shape>> = HashMap::new();
    let mut shape_of = |template: u32| shapes.entry(template).or_insert_with(|| shape(db, template)).clone();
    let nodes = houses
        .iter()
        .filter_map(|h| Some(crate::citymap::Node { item: None, template: h.template.1, shape: shape_of(h.template.1)?, tile: h.tile(), rotation: h.rotation.rem_euclid(360), demolishing: h.flag, valid: true }))
        .collect();
    let buildables = backpack
        .iter()
        .filter_map(|&(slot, low)| {
            let template = stat(low, 620).filter(|&t| t > 0)? as u32;
            Some(crate::citymap::Buildable { slot, template, side: stat(low, 33), shape: shape_of(template)? })
        })
        .collect();
    let record = db.get(Id { kind: mao_formats::citydata::KIND, instance: playfield }).ok().flatten().and_then(|r| mao_formats::citydata::parse(&r));
    let controller = record.as_ref().and_then(|r| r.controllers.iter().find(|c| c.identity.1 >> 16 == city).or(r.controllers.first()));
    crate::citymap::Data {
        houses: nodes,
        buildables,
        zones: crate::citymap::Zones(controller.map(|c| c.zones.iter().map(|z| (z.corner, z.mask)).collect()).unwrap_or_default()),
        tiles: record.as_ref().map_or((0, 0), |r| r.tiles),
        costs: controller.map_or((0, 0), |c| (c.purchase_cost, c.upkeep)),
    }
}

#[cfg(test)]
mod tests {
    /// `MAO_GAME`: a city's nine houses (the whole list of the user's log of 2026-11-15) have their doors where the
    /// server announced them (the `DoorFullUpdate`s of the same log, the five doors in sight), to the centimetre: the
    /// houses' places, their turns and the templates' door places are read right.
    #[test]
    fn the_doors_of_a_real_city_stand_where_the_server_puts_them() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let db = mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap();
        let hex = "000000094459000040a0000044a000000000010e00000f4268006a00034459000040a0000044a980000000000000000f4268006a0011446f000040a0000044b58000000000b400000f4268006a00014480800040a00000449e00000000005a00000f4268006a00184480800040a00000449f00000000005a00000f4268006a001a4482000040a00000449e0000000000b400000f4268006a00194485000040a00000449b80000000000000000f4268006a00024487800040a0000044a980000000000000000f4268006a000b448a000040a0000044b080000000000000000f4268006a000e";
        let blob: Vec<u8> = (0..hex.len() / 2).map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap()).collect();
        let houses = mao_net::landcontrol::houses(&blob).unwrap();
        assert_eq!(houses.len(), 9);
        let mut doors = Vec::new();
        for house in &houses {
            let record = db.get(mao_formats::resource::Id { kind: house.template.0, instance: house.template.1 }).unwrap().expect("the template");
            let template = mao_formats::house::Template::parse(&record).expect("a template that reads");
            for d in &template.doors {
                let p = template.rotate_local(d.position, house.rotation);
                doors.push([house.position[0] + p[0], house.position[1] + p[1], house.position[2] + p[2]]);
            }
        }
        assert_eq!(doors.len(), 6);
        // (the same city on the controller's map: every house lies in the city's zones as the original's test wants
        // a new one to, no two share a tile, and the view is the original's 842 by 1082 pixels)
        let data = super::map_data(&db, 6010, 0xc000, &houses, &[], &|_, _| None);
        assert_eq!((data.houses.len(), data.zones.0.len(), data.tiles, data.costs), (9, 74, (600, 600), (500_000_000, 15_000_000)));
        for (i, a) in data.houses.iter().enumerate() {
            assert!(data.zones.can_build(a), "house {i} at {:?} turned {} is not in the zones", a.tile, a.rotation);
            for b in &data.houses[i + 1..] {
                assert!(!a.intersects(b), "houses at {:?} and {:?} share a tile", a.tile, b.tile);
            }
        }
        let g = crate::citymap::Geometry::new(&data.zones, data.tiles).unwrap();
        assert_eq!((g.left, g.top, g.size), (790.0, 1611.0, [842.0, 1082.0]));
        for told in [[1043.87f32, 7.94, 1265.85], [1073.77, 6.35, 1254.19], [1029.77, 7.94, 1268.04], [1029.82, 7.94, 1276.06], [915.68, 6.45, 1301.29]] {
            assert!(doors.iter().any(|d| (0..3).all(|k| (d[k] - told[k]).abs() < 0.02)), "no door at {told:?}: {doors:?}");
        }
    }
}
