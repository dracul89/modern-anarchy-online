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

mod anim;
mod net;
mod settings;
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
}

impl Textures {
    pub fn new(db: Arc<ResourceDatabase>) -> Self {
        Textures { db }
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

/// The vertical scale of the ground model.
pub(crate) const HEIGHT_SCALE: f32 = 0.2;

pub(crate) fn load_terrain(db: &ResourceDatabase, playfield: u32) -> Result<Model, String> {
    let terrain = load_terrain_data(db, playfield)?;
    eprintln!("terrain {}x{} points, {} tile types", terrain.width, terrain.height, terrain.tile_types.len());
    let model = terrain.model(0xf6966, HEIGHT_SCALE);
    for p in model.parts.iter().take(4) {
        eprintln!("part {} {} vertices, textures {:?}", p.name, p.vertices.len(), p.material.textures);
    }
    eprintln!("heights {:?}", &terrain.heights[..8.min(terrain.heights.len())]);
    Ok(model)
}

/// The playfield's static objects (its `.pf` statel file), as parts placed on
/// the ground. The world is Y-up, the viewer Z-up, so a statel's (x, y, z)
/// goes to (x, z, y).
/// Returns, for each prop part added, the instance of the mesh it came from.
pub(crate) fn add_props(game: &str, db: &ResourceDatabase, playfield: u32, model: &mut Model) -> std::collections::HashMap<usize, u32> {
    let mut part_mesh = std::collections::HashMap::new();
    let path = Path::new(game).join(format!("cd_image/data/statels/{playfield}.pf"));
    let Ok(file) = std::fs::read(&path) else { return part_mesh };
    let Ok(statels) = mao_formats::statels::parse(&file) else { return part_mesh };
    // Each mesh's parts go in once; a statel is an instance of each.
    let mut meshes: std::collections::HashMap<u32, Option<std::ops::Range<usize>>> = std::collections::HashMap::new();
    let swap = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0];
    // The ground itself is one instance of each of its parts.
    let ground = model.parts.len();
    model.instances.extend((0..ground).map(|i| (i, model::IDENTITY)));
    let mut placed = 0;
    // The plants are found by their mesh's file name.
    let plant_meshes: std::collections::HashSet<u32> = db
        .get(Id { kind: 0xf424a, instance: 1 })
        .ok()
        .flatten()
        .and_then(|o| mao_formats::instances::Instances::parse(&o).ok())
        .map(|index| index.iter().filter(|&(kind, name, _)| kind == 0xf6951 && model::is_foliage_name(name)).map(|(_, _, instance)| instance).collect())
        .unwrap_or_default();
    let mut plants = 0usize;
    for s in &statels {
        let range = meshes
            .entry(s.mesh)
            .or_insert_with(|| {
                let mut m = load_model(db, Id { kind: 0xf6951, instance: s.mesh }).ok()?;
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
    eprintln!("{placed} of {} statels placed ({} meshes, {plants} of them plants)", statels.len(), meshes.len());
    part_mesh
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
            let textures = Textures { db };
            return match rest {
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
    let textures = Textures { db };
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
