//! `mao-client <game> <mesh type> <mesh instance> <animation instance>`: a
//! character mesh posed by one of its animations (a loop, or one frame to
//! an image).

use std::sync::Arc;
use std::time::{Duration, Instant};

use mao_formats::catanim::{self, CatAnim};
use mao_formats::catmesh::{self, CatMesh};
use mao_formats::model::Model;
use mao_formats::resource::{Id, ResourceDatabase};

use crate::viewer;
use crate::Textures;

fn load(db: &ResourceDatabase, kind: u32, instance: u32) -> Result<Vec<u8>, String> {
    db.get(Id { kind, instance }).map_err(|e| e.to_string())?.ok_or_else(|| format!("no object {kind:#x}:{instance:#x}"))
}

/// The animation's length: its last key (the stored length field is in the
/// same milliseconds).
fn length(anim: &CatAnim) -> f32 {
    anim.tracks.iter().flat_map(|t| t.rotations.iter().map(|k| k.0).chain(t.translations.iter().map(|k| k.0))).fold(0.0f32, f32::max).max(1.0)
}

/// `args`: mesh type, mesh instance, animation instance and an optional `--image out.png [time in ms]`.
pub fn run(db: Arc<ResourceDatabase>, mesh_kind: u32, mesh: u32, anim: u32, image: Option<(&str, f32)>) -> Result<(), String> {
    let cat: CatMesh = catmesh::parse(&load(&db, mesh_kind, mesh)?).map_err(|e| e.to_string())?;
    let animation: CatAnim = catanim::parse(&load(&db, 0xf6953, anim)?).map_err(|e| e.to_string())?;
    let length = length(&animation);
    eprintln!("{:?}: {} bones; animation of {} tracks, {length:.0} ms, {} notes", cat.name, cat.bones.len(), animation.tracks.len(), animation.notes.len());
    for n in animation.notes.iter().take(8) {
        eprintln!("  note {:?} at {} ms", n.name, n.time);
    }
    let pose = move |t: f32| -> Model { cat.skinned_model(&cat.pose(&animation, t), 0xf6954) };
    let textures = Textures::new(db);
    if let Some((out, t)) = image {
        return viewer::headless(&pose(t), &textures, out);
    }
    let (tx, rx) = std::sync::mpsc::channel();
    let (input_tx, _input_rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let start = Instant::now();
        loop {
            let t = (start.elapsed().as_secs_f32() * 1000.0) % length;
            if tx.send(viewer::Update { scene: None, model: pose(t), own: None }).is_err() {
                break;
            }
            std::thread::sleep(Duration::from_millis(33));
        }
    });
    viewer::window(Model::default(), textures, Some(viewer::Live { overlays: rx, input: input_tx, focus: Some([0.0, 0.0, 0.9]), distance: 3.0 }))
}

/// A player of a breed and sex: the body, posed by `anim`, with its head
/// (found by name, as the original does) at the head attractor.
pub fn player(db: Arc<ResourceDatabase>, breed: u8, sex: u8, anim: u32, image: Option<(&str, f32)>) -> Result<(), String> {
    let index = mao_formats::instances::Instances::parse(&load(&db, 0xf424a, 1)?).map_err(|e| e.to_string())?;
    let (body_name, head_name) = mao_formats::appearance::player_meshes(breed, sex).ok_or("no meshes for that breed and sex")?;
    let body_id = index.find(0xf6952, body_name).ok_or_else(|| format!("{body_name} not in the index"))?;
    let head_id = index.find(0xf6951, head_name).ok_or_else(|| format!("{head_name} not in the index"))?;
    eprintln!("body {body_name} = {body_id:#x}, head {head_name} = {head_id:#x}");
    let body: CatMesh = catmesh::parse(&load(&db, 0xf6952, body_id)?).map_err(|e| e.to_string())?;
    let head = crate::load_model(&db, Id { kind: 0xf6951, instance: head_id })?;
    let animation: CatAnim = catanim::parse(&load(&db, 0xf6953, anim)?).map_err(|e| e.to_string())?;
    let length = length(&animation);
    let attractor = body.attractor("head").cloned();
    let pose = move |t: f32| -> Model {
        let bones = body.pose(&animation, t);
        let mut model = body.skinned_model(&bones, 0xf6954);
        if let Some(a) = &attractor {
            model.parts.extend(body.attach(&bones, a, &head));
        }
        model
    };
    let textures = Textures::new(db);
    if let Some((out, t)) = image {
        return viewer::headless(&pose(t), &textures, out);
    }
    let (tx, rx) = std::sync::mpsc::channel();
    let (input_tx, _input_rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let start = Instant::now();
        loop {
            let t = (start.elapsed().as_secs_f32() * 1000.0) % length;
            if tx.send(viewer::Update { scene: None, model: pose(t), own: None }).is_err() {
                break;
            }
            std::thread::sleep(Duration::from_millis(33));
        }
    });
    viewer::window(Model::default(), textures, Some(viewer::Live { overlays: rx, input: input_tx, focus: Some([0.0, 0.0, 0.9]), distance: 3.0 }))
}
