//! The Modern Anarchy Online client. For now a mesh viewer:
//!
//! ```text
//! mao-client <game dir> list [name]                             the playfields
//! mao-client <game dir> map <number or name>                    the ground (add --image)
//! mao-client <game dir> <mesh type> <instance>                  a window
//! mao-client <game dir> <mesh type> <instance> --image out.png  headless
//! ```
//!
//! mao-client net <game dir> chars                               log in, list characters
//! mao-client net parse-capture <capture.log>                    read a proxy capture
//!
//! The type and instance are hexadecimal (for example `f6951 1893`). Drag to
//! orbit, scroll to zoom, WASD to move the view point, Q/E down and up.

mod actions;
mod chat_commands;
mod chatwindow;
mod ncu;
mod skills;
mod stats;
mod trickle_data;
mod vitals;
mod automap;
mod actions_data;
mod anim;
mod derived;
mod gear;
mod mailbox;
mod shop;
mod mission;
mod hotbar;
mod net;
mod settings;
mod theme;
mod ui;
mod whompah;
pub(crate) mod viewer;

use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;

use mao_formats::mesh;
use mao_formats::model::{self, Model};
use mao_formats::resource::{Id, ResourceDatabase};
use mao_formats::texture::{self, Texture};

fn number(text: &str) -> Option<u32> {
    u32::from_str_radix(text.strip_prefix("0x").unwrap_or(text), 16).ok()
}

/// A playfield number: decimal (the statel files are named so), or hex with
/// `0x`.
fn playfield_number(text: &str) -> Option<u32> {
    match text.strip_prefix("0x") {
        Some(hex) => u32::from_str_radix(hex, 16).ok(),
        None => text.parse().ok(),
    }
}

/// The textures of a game, decoded on request.
pub struct Textures {
    db: Arc<ResourceDatabase>,
    /// The name index of the database (read once), for icons found by file name.
    index: Arc<std::sync::OnceLock<Option<mao_formats::instances::Instances>>>,
}

impl Textures {
    pub fn new(db: Arc<ResourceDatabase>) -> Self {
        Textures { db, index: Default::default() }
    }

    /// An icon (type 0xf6958) by its file name, e.g. `action_burst.png`.
    pub fn icon_named(&self, name: &str) -> Option<Arc<gear::Icon>> {
        let index = self.index.get_or_init(|| self.db.get(Id { kind: 0xf424a, instance: 1 }).ok().flatten().and_then(|o| mao_formats::instances::Instances::parse(&o).ok())).as_ref()?;
        let id = index.find(0xf6958, name)?;
        let t = self.get(0xf6958, id)?;
        Some(Arc::new(gear::Icon { id, width: t.width as usize, height: t.height as usize, rgba: t.rgba }))
    }

    /// Another handle on the same database.
    pub fn clone_handle(&self) -> Textures {
        Textures { db: self.db.clone(), index: self.index.clone() }
    }

    pub fn get(&self, kind: u32, instance: u32) -> Option<Texture> {
        // The original falls back through the other texture types when one
        // is missing; try the object itself and then those.
        let order: &[u32] = match kind {
            0xf6954 => &[0xf6954, 0xf6960, 0xf6961],
            0xf6960 => &[0xf6960, 0xf6954, 0xf6961],
            0xf6961 => &[0xf6961, 0xf6960, 0xf6954],
            other => return self.load(other, instance),
        };
        order.iter().find_map(|&k| self.load(k, instance))
    }

    fn load(&self, kind: u32, instance: u32) -> Option<Texture> {
        let object = self.db.get(Id { kind, instance }).ok()??;
        match kind {
            0xf6956 | 0xf6965 | 0xf6966 => texture::decode_ground(&object).ok(),
            _ => texture::decode(&object).ok().map(skin_tint),
        }
    }
}

/// A character mesh (type 0xf6952 or 0xf696b) in its rest pose.
fn load_cat(db: &ResourceDatabase, id: Id) -> Result<Model, String> {
    let object = db.get(id).map_err(|e| e.to_string())?.ok_or("no such object")?;
    let cat = mao_formats::catmesh::parse(&object).map_err(|e| e.to_string())?;
    eprintln!("{:?}: {} parts, {} bones", cat.name, cat.groups.iter().map(|g| g.lists.len()).sum::<usize>(), cat.bones.len());
    Ok(cat.rest_model(0xf6954))
}

/// The game paints a character's skin colour into the pure-green areas of
/// its textures at runtime; until that colour is known, they get a plain tan.
fn skin_tint(mut t: Texture) -> Texture {
    const SKIN: [u8; 3] = [214, 164, 128];
    for p in t.rgba.chunks_exact_mut(4) {
        if p[0] < 70 && p[1] > 190 && p[2] < 70 {
            let shade = p[1] as f32 / 255.0;
            p[0] = (SKIN[0] as f32 * shade) as u8;
            p[1] = (SKIN[1] as f32 * shade) as u8;
            p[2] = (SKIN[2] as f32 * shade) as u8;
        }
    }
    t
}

pub(crate) fn load_model(db: &ResourceDatabase, id: Id) -> Result<Model, String> {
    let object = db.get(id).map_err(|e| e.to_string())?.ok_or("no such object")?;
    let mesh = mesh::parse(&object).map_err(|e| e.to_string())?;
    model::build(&mesh).map_err(|e| e.to_string())
}

/// The ground of a playfield.
/// The ground's height grid of a playfield.
pub(crate) fn load_terrain_data(db: &ResourceDatabase, playfield: u32) -> Result<mao_formats::terrain::Terrain, String> {
    let object = db.get(Id { kind: 0xf4249, instance: playfield }).map_err(|e| e.to_string())?.ok_or("no tile-map for that playfield")?;
    let map = mao_formats::tilemap::parse_old(&object).map_err(|e| e.to_string())?;
    mao_formats::terrain::build(&map).map_err(|e| e.to_string())
}

pub(crate) fn load_terrain(db: &ResourceDatabase, playfield: u32) -> Result<Model, String> {
    let terrain = load_terrain_data(db, playfield)?;
    eprintln!("terrain {}x{} points, {} tile types", terrain.width, terrain.height, terrain.tile_types.len());
    let model = terrain.model(0xf6966, terrain.height_scale);
    for p in model.parts.iter().take(4) {
        eprintln!("part {} {} vertices, textures {:?}", p.name, p.vertices.len(), p.material.textures);
    }
    eprintln!("heights {:?}", &terrain.heights[..8.min(terrain.heights.len())]);
    eprintln!("water: {:?}", terrain.water(terrain.height_scale).map(|w| w.level));
    // MAO_PROBE=x,z,x,z...: the ground's height at those places (to compare with the props standing there).
    if let Ok(v) = std::env::var("MAO_PROBE") {
        let n: Vec<f32> = v.split(',').filter_map(|t| t.trim().parse().ok()).collect();
        for p in n.chunks(2).filter(|c| c.len() == 2) {
            eprintln!("ground at ({}, {}): {:.2} (scale {})", p[0], p[1], terrain.height_at(p[0], p[1], terrain.height_scale), terrain.height_scale);
        }
    }
    {
        let mut beyond: std::collections::BTreeMap<u16, usize> = std::collections::BTreeMap::new();
        for w in &terrain.tiles {
            let i = w & terrain.mask;
            if i as usize >= terrain.tile_types.len() {
                *beyond.entry(i).or_default() += 1;
            }
        }
        if std::env::var_os("MAO_TILE_STATS").is_some() {
            // The tile indices of the steep cells (cliffs), to compare with the original's look.
            let mut steep: std::collections::BTreeMap<u16, usize> = std::collections::BTreeMap::new();
            let w = terrain.width;
            for cy in 0..terrain.height - 1 {
                for cx in 0..w - 1 {
                    let h = |x: usize, y: usize| terrain.heights[y * w + x] as f32 / 256.0 * terrain.height_scale;
                    let rise = (h(cx + 1, cy) - h(cx, cy)).abs().max((h(cx, cy + 1) - h(cx, cy)).abs());
                    if rise > 3.0 {
                        *steep.entry(terrain.tiles[cy * (w - 1) + cx] & terrain.mask).or_default() += 1;
                    }
                }
            }
            let mut v: Vec<_> = steep.into_iter().collect();
            v.sort_by_key(|e| std::cmp::Reverse(e.1));
            eprintln!("steep cell tiles: {:?}", &v[..v.len().min(12)]);
        }
        eprintln!("tile indices past the list ({}): mask {:#x}, {:?}", terrain.tile_types.len(), terrain.mask, beyond.iter().take(12).collect::<Vec<_>>());
    }
    Ok(model)
}

/// The rooms of an instanced interior (its playfield definition lists them), when it has any.
pub(crate) fn load_rooms(db: &ResourceDatabase, playfield: u32) -> Option<mao_formats::rooms::RoomPlayfield> {
    let object = db.get(Id { kind: mao_formats::rooms::KIND, instance: playfield }).ok().flatten()?;
    mao_formats::rooms::parse(&object).ok().flatten()
}

/// The doors, terminals and other machines a playfield has built in (its dynel table), with the mesh of each one's item template:
/// `(key, (mesh, position, rotation), door number)`. The vending machines are left out: the server announces the ones that are in use.
pub(crate) fn static_props(db: &ResourceDatabase, playfield: u32) -> Vec<(u32, (u32, [f32; 3], [f32; 4]), Option<u8>)> {
    let Some(object) = db.get(Id { kind: mao_formats::dynels::KIND, instance: playfield }).ok().flatten() else { return Vec::new() };
    let mut meshes: std::collections::HashMap<u32, Option<u32>> = std::collections::HashMap::new();
    mao_formats::dynels::parse(&object)
        .into_iter()
        .enumerate()
        .filter(|(_, d)| d.kind != 0xc75b)
        .filter_map(|(i, d)| {
            let mesh = (*meshes.entry(d.template).or_insert_with(|| {
                let data = db.get(Id { kind: mao_formats::item::TEMPLATE, instance: d.template }).ok().flatten()?;
                mao_formats::item::Item::parse(&data)?.stat(mao_formats::item::stat::MESH).filter(|&m| m > 0)
            }))?;
            // (a door's number is in its instance)
            let door = (d.kind == 0xc748).then_some((d.instance >> 16) as u8);
            Some((0xfe00_0000 | i as u32, (mesh, d.position, d.rotation), door))
        })
        .collect()
}

/// The shell of an interior: its rooms' floors and walls as a model (Z-up), when the playfield is made of rooms.
pub(crate) fn load_room_shell(db: &ResourceDatabase, playfield: u32) -> Option<Model> {
    let rooms = load_rooms(db, playfield)?;
    let object = db.get(Id { kind: 0xf4249, instance: rooms.tilemap }).ok().flatten()?;
    let map = mao_formats::tilemap::parse_new(&object).ok()?;
    let grid = mao_formats::rooms::Grid::new(&map)?;
    let material = mao_formats::model::Material { name: "room".into(), diffuse: [0.55, 0.52, 0.48], ambient: [0.55, 0.52, 0.48], opacity: 1.0, ..Default::default() };
    let model = mao_formats::rooms::model(&grid, &rooms, &material);
    (!model.parts.is_empty()).then_some(model)
}

/// The playfield's static objects (its `.pf` statel file), as parts placed on
/// the ground. The world is Y-up, the viewer Z-up, so a statel's (x, y, z)
/// goes to (x, z, y).
/// Returns, for each prop part added, the instance of the mesh it came from.
pub(crate) fn add_props(game: &str, db: &ResourceDatabase, playfield: u32, model: &mut Model) -> std::collections::HashMap<usize, u32> {
    let mut part_mesh = std::collections::HashMap::new();
    let path = Path::new(game).join(format!("cd_image/data/statels/{playfield}.pf"));
    let Ok(file) = std::fs::read(&path) else { return part_mesh };
    let Ok((mut statels, zone_lists)) = mao_formats::statels::parse_zoned(&file) else { return part_mesh };
    let (zones, failed) = (zone_lists.len(), zone_lists.iter().filter(|z| z.is_none()).count());
    if failed > 0 {
        eprintln!("{failed} of {zones} statel zones could not be read (their props are missing)");
    }
    // An instanced interior: each zone is a room, and its statels sit relative to the room's centre.
    let rooms = load_rooms(db, playfield);
    for (k, zone) in zone_lists.into_iter().enumerate() {
        for mut s in zone.into_iter().flatten() {
            if let Some(room) = rooms.as_ref().and_then(|r| r.rooms.get(k)) {
                (s.position, s.rotation) = mao_formats::rooms::place_statel(room, s.position, s.rotation);
            }
            statels.push(s);
        }
    }
    // Each mesh's parts go in once; a statel is an instance of each.
    // (A statel may carry texture overrides for some of its mesh's materials: those get their own copy of the parts.)
    let mut meshes: std::collections::HashMap<(u32, Vec<(u32, u32)>), Option<std::ops::Range<usize>>> = std::collections::HashMap::new();
    let swap = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0];
    // The ground itself is one instance of each of its parts.
    let ground = model.parts.len();
    model.instances.extend((0..ground).map(|i| (i, model::IDENTITY)));
    let mut placed = 0;
    // MAO_NEAR=x,y,z,radius: the statels within that distance (to see what stands where the server announces a machine).
    if let Ok(v) = std::env::var("MAO_NEAR") {
        let n: Vec<f32> = v.split(',').filter_map(|t| t.trim().parse().ok()).collect();
        if let [x, y, z, r] = n[..] {
            let names: std::collections::HashMap<u32, String> = db
                .get(Id { kind: 0xf424a, instance: 1 })
                .ok()
                .flatten()
                .and_then(|o| mao_formats::instances::Instances::parse(&o).ok())
                .map(|index| index.iter().filter(|&(kind, _, _)| kind == 0xf6951).map(|(_, name, instance)| (instance, name.to_string())).collect())
                .unwrap_or_default();
            for s in &statels {
                let d = ((s.position[0] - x).powi(2) + (s.position[1] - y).powi(2) + (s.position[2] - z).powi(2)).sqrt();
                if d < r {
                    eprintln!("near {d:.1}: mesh {} {:?} at {:?} rot {:?}", s.mesh, names.get(&s.mesh), s.position, s.rotation);
                }
            }
        }
    }
    // The plants are found by their mesh's file name.
    let plant_meshes: std::collections::HashSet<u32> = db
        .get(Id { kind: 0xf424a, instance: 1 })
        .ok()
        .flatten()
        .and_then(|o| mao_formats::instances::Instances::parse(&o).ok())
        .map(|index| index.iter().filter(|&(kind, name, _)| kind == 0xf6951 && model::is_foliage_name(name)).map(|(_, _, instance)| instance).collect())
        .unwrap_or_default();
    // Marker volumes (`primitive_*`: zone and trigger boxes, left in the data with a placeholder texture) are not drawn.
    let hidden: std::collections::HashSet<u32> = db
        .get(Id { kind: 0xf424a, instance: 1 })
        .ok()
        .flatten()
        .and_then(|o| mao_formats::instances::Instances::parse(&o).ok())
        .map(|index| index.iter().filter(|&(kind, name, _)| kind == 0xf6951 && name.to_lowercase().starts_with("primitive_")).map(|(_, _, instance)| instance).collect())
        .unwrap_or_default();
    let mut plants = 0usize;
    for s in &statels {
        if hidden.contains(&s.mesh) && std::env::var_os("MAO_HIDE_PRIMITIVES").is_some() {
            continue;
        }
        // The extras of the first mask are the materials' textures (bit = the material's number).
        let overrides: Vec<(u32, u32)> = s.extras.iter().filter(|e| e.0 == 0).map(|e| (e.1, e.2)).collect();
        let range = meshes
            .entry((s.mesh, overrides.clone()))
            .or_insert_with(|| {
                let mut m = load_model(db, Id { kind: 0xf6951, instance: s.mesh }).ok()?;
                for (material, texture) in &overrides {
                    if let Some(part) = m.parts.get_mut(*material as usize) {
                        part.material.textures = vec![(0xf6954, *texture)];
                    }
                }
                if plant_meshes.contains(&s.mesh) {
                    plants += 1;
                    for part in &mut m.parts {
                        part.material.foliage = true;
                    }
                }
                let start = model.parts.len();
                model.parts.extend(m.parts);
                part_mesh.extend((start..model.parts.len()).map(|i| (i, s.mesh)));
                Some(start..model.parts.len())
            })
            .clone();
        let Some(range) = range else { continue };
        let place = model::multiply(&model::trs_matrix(s.position, s.rotation, s.scale_xyz), &swap);
        model.instances.extend(range.map(|i| (i, place)));
        placed += 1;
    }
    eprintln!("{placed} of {} statels placed ({} meshes, {plants} of them plants)", statels.len(), meshes.keys().map(|k| k.0).collect::<std::collections::HashSet<_>>().len());
    part_mesh
}

/// Where the whompah booths stand in a playfield (x, height, z as the statels
/// give them): the statels of the booth mesh `whompa_entrance_default`.
pub(crate) fn whompah_booths(game: &str, db: &ResourceDatabase, playfield: u32) -> Vec<[f32; 3]> {
    let booth_meshes: std::collections::HashSet<u32> = db
        .get(Id { kind: 0xf424a, instance: 1 })
        .ok()
        .flatten()
        .and_then(|o| mao_formats::instances::Instances::parse(&o).ok())
        .map(|index| index.iter().filter(|&(kind, name, _)| kind == 0xf6951 && name == "whompa_entrance_default.abiff").map(|(_, _, instance)| instance).collect())
        .unwrap_or_default();
    let path = Path::new(game).join(format!("cd_image/data/statels/{playfield}.pf"));
    let Ok(file) = std::fs::read(&path) else { return Vec::new() };
    let Ok(statels) = mao_formats::statels::parse(&file) else { return Vec::new() };
    statels.iter().filter(|s| booth_meshes.contains(&s.mesh)).map(|s| s.position).collect()
}

/// The door blockers of an interior (the statels that stand in its doorway), x, height, z.
pub(crate) fn door_markers(game: &str, db: &ResourceDatabase, playfield: u32) -> Vec<[f32; 3]> {
    let meshes: std::collections::HashSet<u32> = db
        .get(Id { kind: 0xf424a, instance: 1 })
        .ok()
        .flatten()
        .and_then(|o| mao_formats::instances::Instances::parse(&o).ok())
        .map(|index| index.iter().filter(|&(kind, name, _)| kind == 0xf6951 && name == "door_blocker.abiff").map(|(_, _, instance)| instance).collect())
        .unwrap_or_default();
    let Ok(file) = std::fs::read(Path::new(game).join(format!("cd_image/data/statels/{playfield}.pf"))) else { return Vec::new() };
    let Ok(statels) = mao_formats::statels::parse(&file) else { return Vec::new() };
    statels.iter().filter(|s| meshes.contains(&s.mesh)).map(|s| s.position).collect()
}

/// A playfield by number (decimal, or hex with `0x`) or by part of its name.
fn find_playfield(db: &ResourceDatabase, text: &str) -> Result<u32, String> {
    if let Some(n) = playfield_number(text) {
        return Ok(n);
    }
    let wanted = text.to_lowercase();
    let matches: Vec<_> = mao_formats::playfield::all(db).into_iter().filter(|p| p.name.to_lowercase().contains(&wanted)).collect();
    if let Some(exact) = matches.iter().find(|p| p.name.trim().to_lowercase() == wanted) {
        return Ok(exact.number);
    }
    match matches.as_slice() {
        [one] => Ok(one.number),
        [] => Err(format!("no playfield matches {text:?} (try `list`)")),
        many => Err(format!("{text:?} matches {}: {}", many.len(), many.iter().take(8).map(|p| format!("{} {}", p.number, p.name)).collect::<Vec<_>>().join(", "))),
    }
}

fn list(db: &ResourceDatabase, filter: Option<&str>) {
    let wanted = filter.map(str::to_lowercase);
    for p in mao_formats::playfield::all(db) {
        if wanted.as_ref().is_some_and(|w| !p.name.to_lowercase().contains(w)) {
            continue;
        }
        let kind = match db.get(Id { kind: 0xf4249, instance: p.number }).ok().flatten() {
            Some(o) if o.get(12..16) == Some(b"CHGA") => "ground",
            Some(_) => "other ground layout (not drawn yet)",
            None => "no ground",
        };
        println!("{:>6} {} [{kind}]", p.number, p.name);
    }
}

fn run(args: &[String]) -> Result<(), String> {
    if let [word, rest @ ..] = args {
        if word == "net" {
            return net::run(rest);
        }
    }
    // `rooms-check <game>`: how many playfields are made of rooms, and whether every one reads and builds.
    if let [game, word] = args {
        if word == "rooms-check" {
            let db = ResourceDatabase::open_game(Path::new(game)).map_err(|e| e.to_string())?;
            let (mut rooms, mut outdoor, mut broken) = (0, 0, 0);
            for p in mao_formats::playfield::all(&db) {
                let Some(object) = db.get(Id { kind: mao_formats::rooms::KIND, instance: p.number }).ok().flatten() else { continue };
                match mao_formats::rooms::parse(&object) {
                    Ok(Some(r)) => {
                        rooms += 1;
                        let shell = load_room_shell(&db, p.number);
                        println!("{} {:?}: {} rooms, tile-map {:#x}, shell {}", p.number, p.name, r.rooms.len(), r.tilemap, shell.map_or("none".to_string(), |m| format!("{} parts", m.parts.len())));
                    }
                    Ok(None) => outdoor += 1,
                    Err(e) => {
                        broken += 1;
                        println!("{} {:?}: {e}", p.number, p.name);
                    }
                }
            }
            println!("{rooms} of rooms, {outdoor} outdoor, {broken} unreadable");
            return Ok(());
        }
    }
    // `n3 <capture.log> <type hex>`: the parsed messages of one type in a capture.
    if let [word, log, kind] = args {
        if word == "n3" {
            let kind = u32::from_str_radix(kind.trim_start_matches("0x"), 16).map_err(|e| e.to_string())?;
            for m in net::capture_n3(log, kind)? {
                println!("{m}");
            }
            return Ok(());
        }
    }
    // `stats-check <game> <capture.log> <instance hex> <extras.txt> <compare.txt> [level]`: the computed shown stats of a character
    // (its captured items plus the extras file's `perks`, `nanos` and `items` lines) against the original's own values
    // (compare.txt lines: `stat stored shown`), per stat.
    if let [game, word, log, instance, extras, compare, rest @ ..] = args {
        if word == "stats-check" {
            let db = ResourceDatabase::open_game(Path::new(game)).map_err(|e| e.to_string())?;
            let instance = u32::from_str_radix(instance.trim_start_matches("0x"), 16).map_err(|e| e.to_string())?;
            let mut entries = net::capture_entries_of(log, instance)?;
            let level: i64 = rest.first().and_then(|l| l.parse().ok()).unwrap_or(220);
            let (mut perks, mut nanos, mut org) = (Vec::new(), Vec::new(), Vec::new());
            for line in std::fs::read_to_string(extras).map_err(|e| e.to_string())?.lines() {
                let (key, list) = line.split_once(' ').unwrap_or((line, ""));
                let ids: Vec<u32> = list.split(',').filter_map(|v| v.trim().parse().ok()).collect();
                match key {
                    "perks" => perks = ids,
                    "nanos" => nanos = ids,
                    "org" => {
                        // `org quality,low,high;...`: the organization benefits (the OrgServer type 6 messages)
                        org = list.split(';').filter_map(|e| {
                            let v: Vec<u32> = e.split(',').filter_map(|n| n.trim().parse().ok()).collect();
                            let [quality, low_id, high_id] = v[..] else { return None };
                            Some(mao_net::inventory::Entry { slot: 33, flags: 1, count: 1, container: (0, 0), low_id, high_id, quality, extra: 0 })
                        }).collect();
                    }
                    "worn" => {
                        // `worn slot,low,high,quality;...` replaces the captured items (a live read of the inventory).
                        entries = list.split(';').filter_map(|e| {
                            let v: Vec<u32> = e.split(',').filter_map(|n| n.trim().parse().ok()).collect();
                            let [slot, low_id, high_id, quality] = v[..] else { return None };
                            Some(mao_net::inventory::Entry { slot, flags: 1, count: 1, container: (0, 0), low_id, high_id, quality, extra: 0 })
                        }).collect();
                    }
                    "items" => entries.extend(ids.iter().map(|&id| mao_net::inventory::Entry { slot: 33, flags: 1, count: 1, container: (0, 0), low_id: id, high_id: id, quality: 1, extra: 0 })),
                    _ => {}
                }
            }
            let mods = derived::modifiers(&db, &entries, &perks, &nanos, &org, level);
            let rows: Vec<(i64, i64, i64)> = std::fs::read_to_string(compare)
                .map_err(|e| e.to_string())?
                .lines()
                .filter_map(|line| {
                    let f: Vec<i64> = line.split_whitespace().take(3).filter_map(|v| v.parse().ok()).collect();
                    <[i64; 3]>::try_from(f).ok().map(|[a, b, c]| (a, b, c))
                })
                .collect();
            let stored: std::collections::BTreeMap<u32, i64> = rows.iter().map(|&(s, d0, _)| (s as u32, d0)).collect();
            let computed = derived::shown(&stored, &mods);
            let (mut same, mut different) = (0, 0);
            for &(stat, stored, shown) in &rows {
                let value = computed.get(&(stat as u32)).copied().unwrap_or(stored);
                if value == shown {
                    same += 1;
                } else {
                    different += 1;
                    println!("stat {stat:#x} ({stat}): stored {stored}, original shows {shown}, computed {value} (off by {})", value - shown);
                }
            }
            // The worn items' requirements against the computed stats: a character wears what it can wear, so none should be unmet.
            let (mut judged, mut unmet) = (0, 0);
            for e in entries.iter().filter(|e| !e.is_bag() && e.slot < 46) {
                for action in derived::actions(&db, e).iter().filter(|a| matches!(a.kind, 6 | 8)) {
                    for (text, met) in gear::requirement_lines(&action.criteria, &computed) {
                        if met.is_some() { judged += 1; }
                        if met == Some(false) { unmet += 1; println!("unmet: slot {} item {} needs {text}", e.slot, e.low_id); }
                    }
                }
            }
            println!("worn items' requirements: {judged} conditions judged, {unmet} unmet");
            println!("{same} stats equal, {different} different; {} modifiers computed", mods.len());
            return Ok(());
        }
    }
    if let [game, word, rest @ ..] = args {
        if word == "list" {
            let db = ResourceDatabase::open_game(Path::new(game)).map_err(|e| e.to_string())?;
            list(&db, rest.first().map(String::as_str));
            return Ok(());
        }
    }
    if let [game, word, playfield, rest @ ..] = args {
        if word == "map" {
            let db = Arc::new(ResourceDatabase::open_game(Path::new(game)).map_err(|e| e.to_string())?);
            let number = find_playfield(&db, playfield)?;
            let mut model = load_terrain(&db, number).unwrap_or_default();
            if std::env::var_os("MAO_NO_PROPS").is_none() {
                add_props(game, &db, number, &mut model);
            }
            let textures = Textures::new(db.clone());
            // MAO_UI_PREVIEW=1 (or `options`) paints the interface over the image.
            let preview = std::env::var("MAO_UI_PREVIEW").ok();
            theme::set_game_dir(game);
            return match rest {
                [flag, out] if flag == "--image" && preview.is_some() => {
                    let names: std::collections::HashMap<u32, String> = mao_formats::playfield::all(&db).into_iter().map(|p| (p.number, p.name.trim().to_string())).collect();
                    let destinations = whompah::Destinations::load();
                    let mut hud = ui::Hud { location: names.get(&number).cloned().unwrap_or_default(), position: Some([449.0, 8.0, 313.0]), ..Default::default() };
                    hud.labels = net::booth_labels(number, &whompah_booths(game, &db, number), &destinations, &names).into_iter().map(|(p, text)| ui::Label { world: mao_render::glam::Vec3::from(p), text }).collect();
                    hud.target = Some("Rollerrat  (12 m)".into());
                    // MAO_UI_PREVIEW=gear + MAO_CAPTURE=<capture.log>: the inventory window from a captured session.
                    if preview.as_deref() == Some("stats") {
                        use stats::stat::*;
                        hud.character_name = "Conjecture".into();
                        hud.stats = [(LEVEL, 220), (HEALTH, 35196), (MAX_HEALTH, 35196), (CURRENT_NANO, 11380), (MAX_NANO, 11380), (ADD_ALL_OFF, 2865), (XP, 14935041 + 1500000), (LAST_XP, 14935041), (NEXT_XP, 18002049), (ALIEN_XP, 306522), (ALIEN_NEXT_XP, 400000), (ALIEN_LEVEL, 30), (MELEE_AC, 14516), (PROJECTILE_AC, 15106), (ENERGY_AC, 14446), (POISON_AC, 14376), (CHEMICAL_AC, 14266), (RADIATION_AC, 14266), (COLD_AC, 14751), (FIRE_AC, 14646)].into_iter().collect();
                        // MAO_CAPTURE=<capture.log> as well: the worn items of that session build the shown values (stored level 8).
                        if let Some(entries) = std::env::var_os("MAO_CAPTURE").and_then(|log| net::capture_entries(&log.to_string_lossy()).ok()) {
                            let level: i64 = std::env::var("MAO_LEVEL").ok().and_then(|l| l.parse().ok()).unwrap_or(8);
                            let perks: Vec<u32> = std::env::var_os("MAO_CAPTURE").and_then(|log| net::capture_message(&log.to_string_lossy(), mao_net::inventory::WORLD_ENTRY)).map(|v| match v.get("Perks") {
                                Some(mao_net::schema::Value::List(items)) => items.iter().filter(|i| i.get("Unknown3").and_then(mao_net::schema::Value::as_int).unwrap_or(0) == 0).filter_map(|i| i.get("SkillId").and_then(mao_net::schema::Value::as_int)).map(|x| x as u32).collect(),
                                _ => Vec::new(),
                            }).unwrap_or_default();
                            eprintln!("perks: {perks:?}");
                            let modifiers = derived::modifiers(&db, &entries, &perks, &[], &[], level);
                            eprintln!("worn modifiers: {modifiers:?}");
                            let stored: std::collections::BTreeMap<u32, i64> = [(LEVEL, level)].into();
                            hud.stat_parts = derived::breakdown(&stored, &derived::worn_modifiers(&db, &entries, level), &derived::perk_modifiers(&db, &perks, level), &Default::default(), &Default::default());
                            hud.stats = derived::totals(&hud.stat_parts);
                        }
                        let mut window = stats::StatsWindow::default();
                        window.open = true;
                        *hud.stats_preview.lock().unwrap() = Some(window);
                    }
                    // MAO_UI_PREVIEW=skills (MAO_COMPARE=<`stat stored shown` file> for real numbers): the Skills window.
                    if preview.as_deref() == Some("skills") {
                        hud.character_name = "Conjecture".into();
                        let shown: std::collections::BTreeMap<u32, i64> = std::env::var_os("MAO_COMPARE")
                            .and_then(|f| std::fs::read_to_string(f).ok())
                            .map(|t| {
                                t.lines()
                                    .filter_map(|l| {
                                        let v: Vec<i64> = l.split_whitespace().take(3).filter_map(|x| x.parse().ok()).collect();
                                        (v.len() == 3).then(|| (v[0] as u32, v[2]))
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();
                        // the stored value from the file's second column, the rest shown as one bonus
                        let stored: std::collections::BTreeMap<u32, i64> = std::env::var_os("MAO_COMPARE")
                            .and_then(|f| std::fs::read_to_string(f).ok())
                            .map(|t| t.lines().filter_map(|l| { let v: Vec<i64> = l.split_whitespace().take(3).filter_map(|x| x.parse().ok()).collect(); (v.len() == 3).then(|| (v[0] as u32, v[1])) }).collect())
                            .unwrap_or_default();
                        hud.stat_parts = shown.iter().map(|(&s, &total)| { let base = stored.get(&s).copied().unwrap_or(0); (s, derived::Breakdown { stored: base, worn: total - base, ..Default::default() }) }).collect();
                        hud.stats = shown;
                        let mut window = skills::SkillsWindow::default();
                        window.open = true;
                        *hud.skills_preview.lock().unwrap() = Some(window);
                    }
                    // MAO_UI_PREVIEW=ncu MAO_NANOS=<id:ms,id:ms,...> (MAO_COMPARE=<stat file> for the NCU memory): the NCU window.
                    if preview.as_deref() == Some("ncu") {
                        let running: Vec<(u32, u32)> = std::env::var("MAO_NANOS")
                            .unwrap_or_default()
                            .split(',')
                            .filter_map(|e| { let (id, ms) = e.split_once(':')?; Some((id.trim().parse().ok()?, ms.trim().parse().ok()?)) })
                            .collect();
                        hud.nanos = ncu::describe(&db, &textures, &running, 220, std::time::Instant::now());
                        if let Some(t) = std::env::var_os("MAO_COMPARE").and_then(|f| std::fs::read_to_string(f).ok()) {
                            hud.stats = t.lines().filter_map(|l| { let v: Vec<i64> = l.split_whitespace().take(3).filter_map(|x| x.parse().ok()).collect(); (v.len() == 3).then(|| (v[0] as u32, v[2])) }).collect();
                        }
                        let mut window = ncu::NcuWindow::default();
                        window.open = true;
                        *hud.ncu_preview.lock().unwrap() = Some(window);
                    }
                    // MAO_UI_PREVIEW=chat (MAO_CHAT_INPUT=/te): the chat window with sample lines and, with an input, the command list.
                    if preview.as_deref() == Some("chat") {
                        use chatwindow::{Kind, Line};
                        let mut window = chatwindow::ChatWindow::default();
                        window.push(Line::new(Kind::System, "Welcome to AnarchyOnline"));
                        window.push(Line::new(Kind::Channel, "[Clan OOC] Mothra: anyone want to team?"));
                        window.push(Line { kind: Kind::Tell, text: "Scurlock tells you: hey, got a minute?".into(), from: Some("Scurlock".into()) });
                        let _ = window.run("/tell Scurlock sure, what's up");
                        window.push(Line::new(Kind::Channel, "[Tour Announcements] The shuttle to Newland leaves in five minutes"));
                        if let Ok(text) = std::env::var("MAO_CHAT_INPUT") {
                            window.type_text(&text);
                        }
                        *hud.chat_preview.lock().unwrap() = Some(window);
                    }
                    if preview.as_deref() == Some("map") {
                        // MAO_UI_PREVIEW=map: walk a path through the playfield, revealing the map as the game would.
                        let area = net::load_area(game, &db, number)?;
                        let (cx, cz) = (hud.position.map_or(449.0, |p| p[0]), hud.position.map_or(313.0, |p| p[2]));
                        let mut builder = automap::MapBuilder::new(area.terrain.as_deref(), [cx, 8.0, cz]);
                        let mut map = automap::AutoMap::default();
                        let offset = area.terrain.as_deref().map_or(0.0, |t| 8.0 - t.height_at(cx, cz, t.height_scale));
                        for step in 0..30 {
                            let (x, z) = (cx - 90.0 + step as f32 * 6.0, cz - 60.0 + (step as f32 * 0.4).sin() * 40.0);
                            if let Some(patch) = builder.reveal([x, 8.0, z], area.terrain.as_deref(), offset, &area.collision) {
                                map.apply(patch);
                            }
                        }
                        if let Some(patch) = builder.reveal([cx, 8.0, cz], area.terrain.as_deref(), offset, &area.collision) {
                            map.apply(patch);
                        }
                        map.overlay = std::env::var_os("MAO_MAP_OVERLAY").is_some();
                        hud.heading = 0.6;
                        hud.camera_yaw = 4.0;
                        hud.map_dots = vec![([cx + 20.0, cz + 10.0], automap::DotKind::Player), ([cx - 15.0, cz + 25.0], automap::DotKind::Hostile), ([cx + 5.0, cz - 30.0], automap::DotKind::Friendly)];
                        *hud.map_preview.lock().unwrap() = Some(map);
                    }
                    if preview.as_deref() == Some("actions") {
                        let mut window = actions::ActionsWindow::default();
                        window.open = true;
                        window.load_icons(&textures.clone_handle());
                        *hud.actions_preview.lock().unwrap() = Some(window);
                    }
                    if preview.as_deref() == Some("shop") {
                        // The captured stock (MAO_CAPTURE=<capture.log>) in the window.
                        if let Some(log) = std::env::var_os("MAO_CAPTURE") {
                            if let Some(value) = net::capture_message(&log.to_string_lossy(), 0x58362220) {
                                let tex = textures.clone_handle();
                                let int = |v: &mao_net::schema::Value, f: &str| v.get(f).and_then(mao_net::schema::Value::as_int).unwrap_or(0) as u32;
                                let mut window = shop::ShopWindow::default();
                                if let Some(mao_net::schema::Value::List(list)) = value.get("VendingMachineSlots") {
                                    let stock = list
                                        .iter()
                                        .map(|s| {
                                            let (low, high, quality) = (int(s, "ItemLowId"), int(s, "ItemHighId"), int(s, "Quality"));
                                            let template = db.get(mao_formats::resource::Id { kind: mao_formats::item::TEMPLATE, instance: low }).ok().flatten().and_then(|d| mao_formats::item::Item::parse(&d));
                                            match template {
                                                Some(t) => {
                                                    let icon = t.icon().and_then(|id| tex.get(mao_formats::item::ICON_TYPE, id).map(|x| std::sync::Arc::new(gear::Icon { id, width: x.width as usize, height: x.height as usize, rgba: x.rgba })));
                                                    shop::Slot { low, high, quality, name: t.name, description: t.description, icon }
                                                }
                                                None => shop::Slot { low, high, quality, name: format!("Item #{low:x}"), description: String::new(), icon: None },
                                            }
                                        })
                                        .collect();
                                    window.set(shop::ShopState { shop: (0xc75b, 0xc000021c), stock });
                                    window.selected = Some(0);
                                }
                                *hud.shop_preview.lock().unwrap() = Some(window);
                            }
                        }
                    }
                    if preview.as_deref() == Some("mail") {
                        // The recipient's inbox and the opened mail from the capture (the bytes are in mao_net::mail's tests).
                        let hex = |s: &str| -> Vec<u8> { (0..s.len() / 2).map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap()).collect() };
                        let mut window = mailbox::MailWindow::sample();
                        window.compose_open = false;
                        if let (Some(entries), Some(mail)) = (
                            mao_net::mail::parse_list(&hex("000000000007e200000000015600060004bc80000a436f6e6a656374757265000b68656c6c6f2074686572656adadeb66aed53b60000002801")),
                            mao_net::mail::parse_mail(&hex("00000200000000015600060004bc80000a436f6e6a656374757265000b68656c6c6f2074686572656adadeb66aed53b6000000280000009c4000035501000355010000006400000000000000010000")),
                        ) {
                            window.set_entries(entries);
                            window.selected = Some(mail.id);
                            let tex = textures.clone_handle();
                            let item = db.get(mao_formats::resource::Id { kind: mao_formats::item::TEMPLATE, instance: 0x35501 }).ok().flatten().and_then(|d| mao_formats::item::Item::parse(&d)).map(|t| {
                                let icon = t.icon().and_then(|id| tex.get(mao_formats::item::ICON_TYPE, id).map(|x| std::sync::Arc::new(gear::Icon { id, width: x.width as usize, height: x.height as usize, rgba: x.rgba })));
                                (t.name, icon)
                            });
                            window.opened = Some(mailbox::Opened { mail, item });
                        }
                        *hud.mail_preview.lock().unwrap() = Some(window);
                    }
                    if preview.as_deref() == Some("mission") {
                        // The captured offers (MAO_CAPTURE=<capture.log>) in the window.
                        if let Some(log) = std::env::var_os("MAO_CAPTURE") {
                            let textures2 = textures.clone_handle();
                            let item = |low: i64| {
                                let t = db.get(mao_formats::resource::Id { kind: mao_formats::item::TEMPLATE, instance: low as u32 }).ok().flatten().and_then(|d| mao_formats::item::Item::parse(&d))?;
                                let icon = t.icon().and_then(|id| textures2.get(mao_formats::item::ICON_TYPE, id).map(|x| std::sync::Arc::new(gear::Icon { id, width: x.width as usize, height: x.height as usize, rgba: x.rgba })));
                                Some((t.name, icon))
                            };
                            let icon = |id: u32| textures2.get(mao_formats::item::ICON_TYPE, id).map(|x| std::sync::Arc::new(gear::Icon { id, width: x.width as usize, height: x.height as usize, rgba: x.rgba }));
                            if let Some(value) = net::capture_quest_alternative(&log.to_string_lossy()) {
                                let mut window = mission::MissionWindow { open: true, ..Default::default() };
                                window.set_offers(mission::offers_from(&value, &item, &icon));
                                window.selected = Some(3);
                                *hud.mission_preview.lock().unwrap() = Some(window);
                            }
                        }
                    }
                    if preview.as_deref() == Some("whompah") {
                        hud.whompah_open = true;
                        hud.destinations = mao_net::destinations::parse(&{
                            let hex = "000000c79c00000227000023790000001b000a53747978204d61676d61010100000015000e436861727265642047726f6f7665010100000019000f4e6f727468206f6620597574746f7301010000001f001157657374206f6620506572646974696f6e0101";
                            (0..hex.len() / 2).map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap()).collect::<Vec<u8>>()
                        });
                    }
                    if preview.as_deref() == Some("hotbar") {
                        // A sample: stims then an attack in slot 1 (the stim cooling down, so the attack shows), a recharger in slot 2.
                        if let Some(log) = std::env::var_os("MAO_CAPTURE") {
                            hud.gear = net::capture_entries(&log.to_string_lossy()).map(|e| gear::from_entries(&db, &textures, &e, 220)).unwrap_or_default();
                            let pick = |name: &str| hud.gear.iter().find(|i| i.name.starts_with(name)).map(|i| hotbar::Action::Item { id: i.item_id, name: i.name.clone(), icon: i.icon.as_ref().map(|c| c.id) });
                            let mut bars = hotbar::Hotbars::default();
                            if let Some(stim) = pick("Health and Nano Stim") {
                                bars.add(0, 0, stim, 6.0);
                            }
                            bars.add(0, 0, hotbar::Action::Attack, 2.0);
                            if let Some(recharger) = pick("Health and Nano Recharger") {
                                bars.add(0, 1, recharger.clone(), 10.0);
                                bars.add(0, 1, recharger, 10.0);
                            }
                            bars.slot_mut(0, 0).entries[0].ready_at = 4.0;
                            bars.slot_mut(0, 1).entries[0].ready_at = 3.5;
                            bars.slot_mut(0, 1).entries[1].ready_at = 5.0;
                            hud.preview_hotbars = bars;
                        }
                    }
                    if preview.as_deref() == Some("gear") {
                        if let Some(log) = std::env::var_os("MAO_CAPTURE") {
                            hud.gear = net::capture_entries(&log.to_string_lossy()).map(|e| gear::from_entries(&db, &textures, &e, 220)).unwrap_or_default();
                            hud.gear_open = true;
                        }
                        // MAO_ENTRIES=<file of `slot low high quality` lines> (what MAO_STATS prints as `worn`): a live character's items.
                        if let Some(file) = std::env::var_os("MAO_ENTRIES") {
                            let entries: Vec<mao_net::inventory::Entry> = std::fs::read_to_string(file)
                                .unwrap_or_default()
                                .lines()
                                .filter_map(|l| {
                                    let v: Vec<u32> = l.split_whitespace().filter_map(|x| x.parse().ok()).collect();
                                    let [slot, low_id, high_id, quality] = v[..] else { return None };
                                    Some(mao_net::inventory::Entry { slot, flags: 1, count: 1, container: (0, 0), low_id, high_id, quality, extra: 0 })
                                })
                                .collect();
                            hud.gear = gear::from_entries(&db, &textures, &entries, std::env::var("MAO_LEVEL").ok().and_then(|l| l.parse().ok()).unwrap_or(220));
                            // MAO_COMPARE=<`stat stored shown` file>: the character's shown stats, to colour the requirements.
                            if let Some(t) = std::env::var_os("MAO_COMPARE").and_then(|f| std::fs::read_to_string(f).ok()) {
                                hud.stats = t.lines().filter_map(|l| { let v: Vec<i64> = l.split_whitespace().take(3).filter_map(|x| x.parse().ok()).collect(); (v.len() == 3).then(|| (v[0] as u32, v[2])) }).collect();
                            }
                            hud.gear_open = true;
                        }
                    }
                    hud.say("Welcome to Old Athens");
                    hud.say("Whom-Pah: this booth leads to Tir (remembered)");
                    viewer::headless_with(&model, &textures, out, Some((&hud, preview.as_deref() == Some("options"))))
                }
                [flag, out] if flag == "--image" => viewer::headless(&model, &textures, out),
                [] => viewer::window(model, textures, None),
                _ => Err("unknown arguments".into()),
            };
        }
    }
    // A player: <game> player <breed> <sex> <animation> [--image out.png [ms]]
    if let [game, word, breed, sex, animation, rest @ ..] = args {
        if word == "player" {
            let db = Arc::new(ResourceDatabase::open_game(Path::new(game)).map_err(|e| e.to_string())?);
            let image = match rest {
                [flag, out] if flag == "--image" => Some((out.as_str(), 0.0)),
                [flag, out, ms] if flag == "--image" => Some((out.as_str(), ms.parse().map_err(|_| "bad time")?)),
                _ => None,
            };
            return anim::player(db, breed.parse().map_err(|_| "bad breed")?, sex.parse().map_err(|_| "bad sex")?, number(animation).ok_or("bad animation")?, image);
        }
    }
    // A character mesh with an animation: <game> <mesh type> <mesh> <animation> [--image out.png [ms]]
    if let [game, kind, mesh, animation, rest @ ..] = args {
        if let (Some(k), Some(m), Some(a)) = (number(kind), number(mesh), number(animation)) {
            if matches!(k, 0xf6952 | 0xf696b) {
                let db = Arc::new(ResourceDatabase::open_game(Path::new(game)).map_err(|e| e.to_string())?);
                let image = match rest {
                    [flag, out] if flag == "--image" => Some((out.as_str(), 0.0)),
                    [flag, out, ms] if flag == "--image" => Some((out.as_str(), ms.parse().map_err(|_| "bad time")?)),
                    _ => None,
                };
                return anim::run(db, k, m, a, image);
            }
        }
    }
    let (game, kind, instance, rest) = match args {
        [game, kind, instance, rest @ ..] => (game, kind, instance, rest),
        _ => return Err("usage: mao-client <game dir> <mesh type> <instance> [--image out.png]".into()),
    };
    let db = Arc::new(ResourceDatabase::open_game(Path::new(game)).map_err(|e| e.to_string())?);
    let id = Id { kind: number(kind).ok_or("bad type")?, instance: number(instance).ok_or("bad instance")? };
    let model = if matches!(id.kind, 0xf6952 | 0xf696b) { load_cat(&db, id)? } else { load_model(&db, id)? };
    let textures = Textures::new(db);
    match rest {
        [flag, out] if flag == "--image" => viewer::headless(&model, &textures, out),
        [] => viewer::window(model, textures, None),
        _ => Err("unknown arguments".into()),
    }
}

fn main() -> ExitCode {
    // The display flags are read by the settings (settings.rs); the commands never see them.
    let mut args: Vec<String> = Vec::new();
    let mut raw = std::env::args().skip(1);
    while let Some(arg) = raw.next() {
        if matches!(arg.as_str(), "--window-mode" | "--width" | "--height") {
            raw.next();
        } else {
            args.push(arg);
        }
    }
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("mao-client: {e}");
            ExitCode::FAILURE
        }
    }
}
