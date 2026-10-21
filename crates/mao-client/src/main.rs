//! The Modern Anarchy Online client. For now a mesh viewer:
//!
//! ```text
//! mao-client <game dir> map <playfield>                         the ground (add --image)
//! mao-client <game dir> <mesh type> <instance>                  a window
//! mao-client <game dir> <mesh type> <instance> --image out.png  headless
//! ```
//!
//! The type and instance are hexadecimal (for example `f6951 1893`). Drag to
//! orbit, scroll to zoom.

mod viewer;

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

/// The textures of a game, decoded on request.
pub struct Textures {
    db: Arc<ResourceDatabase>,
}

impl Textures {
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
            _ => texture::decode(&object).ok(),
        }
    }
}

fn load_model(db: &ResourceDatabase, id: Id) -> Result<Model, String> {
    let object = db.get(id).map_err(|e| e.to_string())?.ok_or("no such object")?;
    let mesh = mesh::parse(&object).map_err(|e| e.to_string())?;
    model::build(&mesh).map_err(|e| e.to_string())
}

/// The ground of a playfield.
fn load_terrain(db: &ResourceDatabase, playfield: u32) -> Result<Model, String> {
    let object = db.get(Id { kind: 0xf4249, instance: playfield }).map_err(|e| e.to_string())?.ok_or("no tile-map for that playfield")?;
    let map = mao_formats::tilemap::parse_old(&object).map_err(|e| e.to_string())?;
    let terrain = mao_formats::terrain::build(&map).map_err(|e| e.to_string())?;
    eprintln!("terrain {}x{} points, {} tile types", terrain.width, terrain.height, terrain.tile_types.len());
    let model = terrain.model(0xf6966, 0.2);
    for p in model.parts.iter().take(4) {
        eprintln!("part {} {} vertices, textures {:?}", p.name, p.vertices.len(), p.material.textures);
    }
    eprintln!("heights {:?}", &terrain.heights[..8.min(terrain.heights.len())]);
    Ok(model)
}

fn run(args: &[String]) -> Result<(), String> {
    if let [game, word, playfield, rest @ ..] = args {
        if word == "map" {
            let db = Arc::new(ResourceDatabase::open_game(Path::new(game)).map_err(|e| e.to_string())?);
            let model = load_terrain(&db, number(playfield).ok_or("bad playfield")?)?;
            let textures = Textures { db };
            return match rest {
                [flag, out] if flag == "--image" => viewer::headless(&model, &textures, out),
                [] => viewer::window(model, textures),
                _ => Err("unknown arguments".into()),
            };
        }
    }
    let (game, kind, instance, rest) = match args {
        [game, kind, instance, rest @ ..] => (game, kind, instance, rest),
        _ => return Err("usage: mao-client <game dir> <mesh type> <instance> [--image out.png]".into()),
    };
    let db = Arc::new(ResourceDatabase::open_game(Path::new(game)).map_err(|e| e.to_string())?);
    let id = Id { kind: number(kind).ok_or("bad type")?, instance: number(instance).ok_or("bad instance")? };
    let model = load_model(&db, id)?;
    let textures = Textures { db };
    match rest {
        [flag, out] if flag == "--image" => viewer::headless(&model, &textures, out),
        [] => viewer::window(model, textures),
        _ => Err("unknown arguments".into()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("mao-client: {e}");
            ExitCode::FAILURE
        }
    }
}
