//! A mesh object turned into drawable parts: the frame tree flattened, each
//! triangle mesh with its world matrix, vertices, indices and material.
//!
//! The classes (from the original's randy31: `AnarchyReimagined`:
//! `dropin/randy31/src/{refframe,trimesh,mesh,trilist,material}.rs`):
//! * `RRefFrame_t` and its subclasses: `local_pos`, `local_rot` (a
//!   quaternion x, y, z, w), `scale`, an `anim_matrix` and an `anim` (an
//!   object, or -1); `chld` lists the children.
//! * `RTriMesh_t` (a frame): `data` is an `FAFTriMeshData_t` whose `mesh`
//!   values are `SimpleMesh`es.
//! * `SimpleMesh`: a `material`, a `trilist` (`TriList`: `triangles`, 16-bit
//!   indices), `vb_desc` (size 0x10, 0, the vertex format, the count) and
//!   `vertices` (that many vertices of the format).
//! * `FAFMaterial_t`: colours, `opac`ity and a `delta_state` (`RDeltaState`)
//!   whose `tch_text` values are `FAFTexture_t`s; a texture's `creator` is an
//!   `AnarchyTexCreator_t` giving the resource's type and instance.
//!
//! Matrices are Direct3D's: 16 floats, row-major, for row vectors (a point
//! `p` goes to `p * M`, the translation in the last row), so a child's
//! world matrix is `local * parent`. A frame with an animation takes its
//! matrix from `anim_matrix`; one without composes scale, rotation and
//! translation from its fields.
//!
//! Vertices have Direct3D's flexible vertex format: a position (XYZ, 12
//! bytes), then a normal (0x10, 12), diffuse (0x40, 4) and specular (0x80,
//! 4) colours and the texture coordinates (bits 8 to 11 count the pairs).

use std::fmt;

use crate::archive::{kind, Archive, Stream};
use crate::mesh::MeshObject;

pub type Matrix = [f32; 16];

#[derive(Debug)]
pub enum Error {
    /// The archive isn't shaped as a mesh is.
    Shape(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Shape(what) => write!(f, "unexpected mesh archive: {what}"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Clone, Debug, Default)]
pub struct Material {
    pub name: String,
    pub diffuse: [f32; 3],
    pub specular: [f32; 3],
    pub ambient: [f32; 3],
    pub emissive: [f32; 3],
    pub opacity: f32,
    /// The resource (type, instance) of each texture, in channel order.
    pub textures: Vec<(u32, u32)>,
    /// Draw the texture opaque: its alpha channel isn't a cut-out mask (the
    /// character meshes' textures).
    pub ignore_alpha: bool,
    /// The texture's alpha channel is baked light, added to the lighting before the colour is multiplied in (the original's
    /// texture stages: stage 0 `COLOROP ADD, COLORARG1 TEXTURE|ALPHAREPLICATE, COLORARG2 DIFFUSE, ALPHAOP DISABLE`, stage 1
    /// `MODULATE TEXTURE, CURRENT`: the "lightedge"/"shadowtech" materials of the city buildings). Implies `ignore_alpha`.
    pub alpha_lights: bool,
    /// Plants (trees, bushes, grass ...): the renderer sways them in the wind
    /// and bends them away from the player.
    pub foliage: bool,
}

/// Whether a mesh's file name is a plant. The name is split into words at
/// anything but a letter, and a word counts when it ends (or, for a few, starts)
/// with a plant: `tropical_palmtree01` and `Nascense_wierdtrees_01` are plants,
/// `street_lights`, `Inferno_trophy` and `corner` are not.
pub fn is_foliage_name(name: &str) -> bool {
    const ENDS: &[&str] = &["tree", "trees", "bush", "bushes", "palm", "plant", "plants", "grass", "flower", "flowers", "shrub", "shrubs", "weed", "weeds", "vine", "vines", "cactus", "cacti", "fern", "ferns", "ivy", "wheat", "pine", "kelp", "seaweed"];
    const STARTS: &[&str] = &["grass", "flower", "shrub", "palm", "cactus", "weed", "bush"];
    let lower = name.to_ascii_lowercase();
    lower.split(|c: char| !c.is_ascii_alphabetic()).filter(|w| w.len() >= 3).any(|w| ENDS.iter().any(|e| w.ends_with(e)) || STARTS.iter().any(|s| w.starts_with(s)))
}

#[cfg(test)]
mod foliage_tests {
    use super::is_foliage_name;

    #[test]
    fn plants_are_told_from_lookalikes() {
        for plant in ["tropical_palmtree01.abiff", "Nascense_wierdtrees_01.abiff", "athens_grasspatch1.abiff", "Tir_fern2.abiff", "apt_plant01.abiff", "athens_cactus3.abiff", "hanging_ivy_dense.abiff", "enigma_bush1.abiff", "shadowlands_shrub0_new.abiff"] {
            assert!(is_foliage_name(plant), "{plant}");
        }
        for other in ["adonis_street_lights.abiff", "Inferno_dragon_trophy.abiff", "acg_machinery_corner-in1.abiff", "Alien_breeding_room.abiff", "ai_cloaktower.abiff", "treepinehut.abiff", "weapon_xan_1hedged.abiff"] {
            assert!(!is_foliage_name(other), "{other}");
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

/// A triangle mesh with its world matrix.
#[derive(Clone, Debug)]
pub struct Part {
    pub name: String,
    pub world: Matrix,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub material: Material,
}

#[derive(Clone, Debug, Default)]
pub struct Model {
    pub parts: Vec<Part>,
    /// Extra placements: each `(part index, matrix)` draws that part again with
    /// its world matrix followed by `matrix`. When there are any, the parts are
    /// drawn only through them (so a prop's geometry is stored once).
    pub instances: Vec<(usize, Matrix)>,
    /// A sheet of water over the ground, where the playfield has any.
    pub water: Option<Water>,
}

/// A sheet of water: its level (world Z) and the box it fills (world X, Y).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Water {
    pub level: f32,
    pub min: [f32; 2],
    pub max: [f32; 2],
}

pub const IDENTITY: Matrix = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];

/// `a * b` for row vectors: apply `a`, then `b`.
pub fn multiply(a: &Matrix, b: &Matrix) -> Matrix {
    let mut m = [0.0; 16];
    for r in 0..4 {
        for c in 0..4 {
            m[r * 4 + c] = (0..4).map(|k| a[r * 4 + k] * b[k * 4 + c]).sum();
        }
    }
    m
}

fn floats(bytes: &[u8]) -> Vec<f32> {
    bytes.chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect()
}

fn float_field(s: &Stream, name: &str) -> Option<Vec<f32>> {
    Some(floats(s.field("", name)?.values.first()?))
}

fn refs(s: &Stream, name: &str) -> Vec<i32> {
    s.field("0", name)
        .map(|f| f.values.iter().filter(|v| v.len() >= 4).map(|v| i32::from_le_bytes(v[..4].try_into().unwrap())).collect())
        .unwrap_or_default()
}

fn string(s: &Stream, name: &str) -> String {
    s.field("", name).and_then(|f| f.values.first()).map(|v| String::from_utf8_lossy(v).trim_end_matches('\0').to_string()).unwrap_or_default()
}

/// The matrix (for row vectors) that scales by `scale`, rotates by the
/// quaternion (x, y, z, w) and moves by `position`.
pub fn trs_matrix(position: [f32; 3], q: [f32; 4], scale: [f32; 3]) -> Matrix {
    let (x, y, z, w) = (q[0], q[1], q[2], q[3]);
    let r = [
        [1.0 - 2.0 * (y * y + z * z), 2.0 * (x * y + z * w), 2.0 * (x * z - y * w)],
        [2.0 * (x * y - z * w), 1.0 - 2.0 * (x * x + z * z), 2.0 * (y * z + x * w)],
        [2.0 * (x * z + y * w), 2.0 * (y * z - x * w), 1.0 - 2.0 * (x * x + y * y)],
    ];
    let mut m = IDENTITY;
    for i in 0..3 {
        for j in 0..3 {
            m[i * 4 + j] = r[i][j] * scale[i];
        }
    }
    m[12] = position[0];
    m[13] = position[1];
    m[14] = position[2];
    m
}

/// The first keys of a frame's animation (`FAFAnim_t`: `trans_keys` of 16 bytes each, a time and x y z; `rot_keys` of 20, a time
/// and a quaternion), as the pose the thing rests in. A sliding door's mesh saves its `anim_matrix` with the leaves slid apart (the
/// open pose, the last keys); the closed door is the first keys.
fn rest_pose(a: &Archive, frame: &Stream) -> Option<([f32; 3], [f32; 4])> {
    let anim = a.objects.get(frame.int("0", "anim")? as usize)?;
    let trans = anim.field("", "trans_keys")?.values.first()?;
    let rot = anim.field("", "rot_keys")?.values.first()?;
    if trans.len() < 16 || rot.len() < 20 {
        return None;
    }
    let t = floats(&trans[..16]);
    let r = floats(&rot[..20]);
    Some(([t[1], t[2], t[3]], [r[1], r[2], r[3], r[4]]))
}

/// A frame's own matrix, as the original's `UpdateWorldMatrix` makes it: the
/// rotation, scale and position as a matrix, taken through the frame's
/// `anim_matrix` first (`anim_matrix * TRS`; a frame keeps one when it isn't
/// the identity, with or without an animation). A frame with an animation
/// takes the animation's first keys in place of the saved `anim_matrix`.
fn local_matrix(a: &Archive, frame: &Stream) -> Matrix {
    let scale = float_field(frame, "scale").and_then(|v| v.first().copied()).unwrap_or(1.0);
    let q = float_field(frame, "local_rot").filter(|q| q.len() == 4).unwrap_or(vec![0.0, 0.0, 0.0, 1.0]);
    let p = float_field(frame, "local_pos").filter(|p| p.len() == 3).unwrap_or(vec![0.0; 3]);
    let (x, y, z, w) = (q[0], q[1], q[2], q[3]);
    // The rotation as Direct3D's row-vector matrix.
    let r = [
        [1.0 - 2.0 * (y * y + z * z), 2.0 * (x * y + z * w), 2.0 * (x * z - y * w)],
        [2.0 * (x * y - z * w), 1.0 - 2.0 * (x * x + z * z), 2.0 * (y * z + x * w)],
        [2.0 * (x * z + y * w), 2.0 * (y * z - x * w), 1.0 - 2.0 * (x * x + y * y)],
    ];
    let mut m = IDENTITY;
    for i in 0..3 {
        for j in 0..3 {
            m[i * 4 + j] = r[i][j] * scale;
        }
    }
    m[12] = p[0];
    m[13] = p[1];
    m[14] = p[2];
    // (the animation's rest pose is not used yet: the key layout is not settled, see `rest_pose`)
    let _ = rest_pose(a, frame);
    match float_field(frame, "anim_matrix").filter(|a| a.len() == 16) {
        Some(a) => {
            let a: Matrix = a.try_into().unwrap();
            multiply(&a, &m)
        }
        None => m,
    }
}

fn rgb(s: &Stream, name: &str) -> [f32; 3] {
    float_field(s, name).filter(|v| v.len() >= 3).map(|v| [v[0], v[1], v[2]]).unwrap_or([1.0; 3])
}

fn material(a: &Archive, index: i32) -> Material {
    let Some(m) = (index >= 0).then(|| a.objects.get(index as usize)).flatten() else { return Material { opacity: 1.0, ..Default::default() } };
    let mut textures = Vec::new();
    let mut alpha_lights = false;
    if let Some(state) = m.int("0", "delta_state").and_then(|i| a.objects.get(i as usize)) {
        // The texture stage states of stage 0 (`tstm_count` says how many belong to each stage): an `ALPHAOP` (4) of `DISABLE` (1)
        // leaves the pixel's alpha at the vertex's, so the texture's alpha is not a mask; with `COLORARG1` (2) = `TEXTURE |
        // ALPHAREPLICATE` (0x22) it is light added to the diffuse.
        let ints = |name: &str| -> Vec<i32> { state.field("", name).map(|f| f.values.iter().map(|v| i32::from_le_bytes(v[..4].try_into().unwrap_or([0; 4]))).collect()).unwrap_or_default() };
        let stage0 = ints("tstm_count").first().copied().unwrap_or(0).max(0) as usize;
        let states: Vec<(i32, i32)> = ints("tst_type").into_iter().zip(ints("tst_value")).take(stage0).collect();
        let alpha_off = states.contains(&(4, 1));
        let replicate = states.iter().any(|&(t, v)| t == 2 && v & 0x20 != 0);
        alpha_lights = alpha_off && replicate;
        for t in refs(state, "tch_text") {
            let creator = a.objects.get(t as usize).and_then(|t| t.int("0", "creator")).and_then(|c| a.objects.get(c as usize));
            if let Some(c) = creator {
                if let (Some(kind), Some(instance)) = (c.int("", "type"), c.int("", "inst")) {
                    textures.push((kind as u32, instance as u32));
                }
            }
        }
    }
    Material {
        name: string(m, "name"),
        diffuse: rgb(m, "diff"),
        specular: rgb(m, "spec"),
        ambient: rgb(m, "ambi"),
        emissive: rgb(m, "emis"),
        opacity: float_field(m, "opac").and_then(|v| v.first().copied()).unwrap_or(1.0),
        textures,
        ignore_alpha: alpha_lights,
        alpha_lights,
        foliage: false,
    }
}

/// The bytes a vertex of the format takes, and where its normal and first
/// texture coordinates are (if it has them).
fn layout(format: u32) -> (usize, Option<usize>, Option<usize>) {
    let mut at = match format & 0xe {
        2 => 12,
        4 => 16,
        6 => 16,
        8 => 20,
        10 => 24,
        12 => 28,
        14 => 32,
        _ => 0,
    };
    let mut normal = None;
    if format & 0x10 != 0 {
        normal = Some(at);
        at += 12;
    }
    if format & 0x40 != 0 {
        at += 4;
    }
    if format & 0x80 != 0 {
        at += 4;
    }
    let textures = (format >> 8) & 0xf;
    let uv = (textures > 0).then_some(at);
    at += textures as usize * 8;
    (at, normal, uv)
}

fn simple_mesh(a: &Archive, s: &Stream, name: &str, world: Matrix, out: &mut Vec<Part>) -> Result<(), Error> {
    let desc = s.field("", "vb_desc").and_then(|f| f.values.first()).ok_or(Error::Shape("a mesh without a vertex description"))?;
    if desc.len() < 16 {
        return Err(Error::Shape("a short vertex description"));
    }
    let format = u32::from_le_bytes(desc[8..12].try_into().unwrap());
    let count = u32::from_le_bytes(desc[12..16].try_into().unwrap()) as usize;
    let data = s.field("", "vertices").and_then(|f| f.values.first()).ok_or(Error::Shape("a mesh without vertices"))?;
    let (stride, normal, uv) = layout(format);
    if stride == 0 || data.len() < stride * count {
        return Err(Error::Shape("vertex data shorter than its description"));
    }
    let f = |at: usize| f32::from_le_bytes(data[at..at + 4].try_into().unwrap());
    let vertices: Vec<Vertex> = (0..count)
        .map(|i| {
            let b = i * stride;
            Vertex {
                position: [f(b), f(b + 4), f(b + 8)],
                normal: normal.map(|n| [f(b + n), f(b + n + 4), f(b + n + 8)]).unwrap_or_default(),
                uv: uv.map(|t| [f(b + t), f(b + t + 4)]).unwrap_or_default(),
            }
        })
        .collect();
    let indices: Vec<u16> = s
        .int("0", "trilist")
        .and_then(|i| a.objects.get(i as usize))
        .and_then(|t| t.field("", "triangles"))
        .and_then(|f| f.values.first())
        .map(|b| b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect())
        .unwrap_or_default();
    // MAO_PART_SKIP=<text>: leave out the parts whose material name contains the text (to tell which part is which in a view).
    if let Ok(skip) = std::env::var("MAO_PART_SKIP") {
        if material(a, s.int("0", "material").unwrap_or(-1)).name.contains(&skip) {
            return Ok(());
        }
    }
    // MAO_PART_BOUNDS=1: each part's extent and index range as read, to tell a bad decode from a bad placement.
    if std::env::var_os("MAO_PART_BOUNDS").is_some() {
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        for v in &vertices {
            for k in 0..3 {
                lo[k] = lo[k].min(v.position[k]);
                hi[k] = hi[k].max(v.position[k]);
            }
        }
        let top = indices.iter().copied().max().unwrap_or(0);
        let m = material(a, s.int("0", "material").unwrap_or(-1));
        eprintln!("part {name:?}: format {format:#x} stride {stride} {count} vertices, {} indices (max {top}), from {lo:?} to {hi:?}, material {:?} textures {:x?} opacity {} alpha lights {}", indices.len(), m.name, m.textures, m.opacity, m.alpha_lights);
    }
    out.push(Part { name: name.to_string(), world, vertices, indices, material: material(a, s.int("0", "material").unwrap_or(-1)) });
    Ok(())
}

fn walk(a: &Archive, index: i32, parent: &Matrix, depth: usize, root: Root, out: &mut Vec<Part>) -> Result<(), Error> {
    if depth > 64 {
        return Err(Error::Shape("a frame tree deeper than 64"));
    }
    let Some(frame) = (index >= 0).then(|| a.objects.get(index as usize)).flatten() else { return Ok(()) };
    // A placed mesh (a statel, a door or machine of the playfield) does not get its root frame's position: that is the pivot the
    // exporter stored there (a door mesh keeps `local_pos` (0, 0.96, 3.81) on its root, cancelled on the leaf's frame and added back by
    // the leaf's animation, which hung the leaves 3.8 m from the door's place), while its rotation and scale do apply (signs came out
    // upside down without them). An attached mesh (a head, worn gear) keeps the whole root transform: its position is the neck's.
    // A dynel (a door, a machine) gets none of it: `RRefFrame_t::UpdateWorldMatrix` composes the animation's matrix with the frame's
    // local position and rotation, and the game sets a dynel's root to the object's place, so the exporter's root transform is
    // replaced; the animated frames under it carry the root's inverse in their local transform and their pose in the animation's
    // matrix, which puts them at the object's place (Old Athen's sliding doors, whose root turns a quarter: with the root's rotation
    // kept the leaves stood across the opening).
    let world = match root {
        Root::Rotation if depth == 0 => {
            let mut m = local_matrix(a, frame);
            m[12] = 0.0;
            m[13] = 0.0;
            m[14] = 0.0;
            multiply(&m, parent)
        }
        Root::None if depth == 0 => *parent,
        _ => multiply(&local_matrix(a, frame), parent),
    };
    if a.class(index as usize) == Some("RTriMesh_t") {
        if let Some(data) = frame.int("0", "data").and_then(|d| a.objects.get(d as usize)) {
            let name = string(data, "name");
            for m in refs(data, "mesh") {
                if let Some(s) = a.objects.get(m as usize) {
                    simple_mesh(a, s, &name, world, out)?;
                }
            }
        }
    }
    for child in refs(frame, "chld") {
        walk(a, child, &world, depth + 1, root, out)?;
    }
    Ok(())
}

/// The drawable parts of a mesh object, as an attached mesh (a head, worn gear): the whole root transform applies.
pub fn build(mesh: &MeshObject) -> Result<Model, Error> {
    build_with(mesh, Root::Full)
}

/// The drawable parts of a mesh object placed in the world (a statel, a door, a machine): the root frame's position is left out
/// (see `walk`).
pub fn build_placed(mesh: &MeshObject) -> Result<Model, Error> {
    build_with(mesh, Root::Rotation)
}

/// The drawable parts of a mesh object that is a dynel of the playfield (a door, a machine): the game sets the root frame's
/// local transform to the object's place, so none of the exporter's root transform applies (see `walk`).
pub fn build_dynel(mesh: &MeshObject) -> Result<Model, Error> {
    build_with(mesh, Root::None)
}

/// How much of a mesh's root frame transform applies.
#[derive(Clone, Copy, PartialEq)]
enum Root {
    /// All of it (attached meshes).
    Full,
    /// Its rotation and scale, not its position (statels).
    Rotation,
    /// None of it (dynels: the game overwrites the root's local transform with the object's place).
    None,
}

fn build_with(mesh: &MeshObject, root: Root) -> Result<Model, Error> {
    let a = &mesh.archive;
    let mut parts = Vec::new();
    for root_frame in refs(&a.root, "obj") {
        walk(a, root_frame, &IDENTITY, 0, root, &mut parts)?;
    }
    let _ = kind::OBJECT;
    Ok(Model { parts, instances: Vec::new(), water: None })
}
