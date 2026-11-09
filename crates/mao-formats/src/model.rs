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
    /// A baked colour (a room's light map), RGBA: with alpha 255 it is added to the lighting as the original's emissive
    /// term; [`Vertex::UNBAKED`] (alpha 0) leaves the vertex to the scene's light.
    pub colour: [u8; 4],
}

impl Vertex {
    /// No baked colour: the vertex is lit by the scene alone.
    pub const UNBAKED: [u8; 4] = [255, 255, 255, 0];
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
    /// The meshes the original's `LODManager` watches (the statels): each a range of the full mesh's parts and, when the
    /// database has its low-detail mesh (type 0xf696a, the same instance), the range of that one's parts, which are drawn
    /// in the full ones' place beyond the manager's distance and never on their own. A statel more than 50 of its radii
    /// from the camera is not drawn at all.
    pub lod_groups: Vec<(std::ops::Range<usize>, Option<std::ops::Range<usize>>)>,
}

/// The water: the playfield's liquid triangles (world, Z up, each with its liquid type) when they are known, else a
/// flat sheet at `level` over the box `min`–`max`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Water {
    pub level: f32,
    pub min: [f32; 2],
    pub max: [f32; 2],
    pub triangles: Vec<([[f32; 3]; 3], u32)>,
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
                colour: Vertex::UNBAKED,
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

fn walk(a: &Archive, index: i32, parent: &Matrix, depth: usize, root: Root, out: &mut Vec<Part>, chain: &mut Vec<Step>, anims: &mut Vec<Option<Animated>>) -> Result<(), Error> {
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
    chain.push(match root {
        Root::None if depth == 0 => Step { trs: IDENTITY, saved: None, keys: None, zero_translation: false },
        _ => Step { trs: trs_of(frame), saved: saved_anim_matrix(frame), keys: keys(a, frame).map(std::sync::Arc::new), zero_translation: root == Root::Rotation && depth == 0 },
    });
    if a.class(index as usize) == Some("RTriMesh_t") {
        if let Some(data) = frame.int("0", "data").and_then(|d| a.objects.get(d as usize)) {
            let name = string(data, "name");
            for m in refs(data, "mesh") {
                if let Some(s) = a.objects.get(m as usize) {
                    let before = out.len();
                    simple_mesh(a, s, &name, world, out)?;
                    if out.len() > before {
                        anims.push(chain.iter().any(|s| s.keys.is_some()).then(|| Animated { chain: chain.clone() }));
                    }
                }
            }
        }
    }
    for child in refs(frame, "chld") {
        walk(a, child, &world, depth + 1, root, out, chain, anims)?;
    }
    chain.pop();
    Ok(())
}

/// A frame's animation (`FAFAnim_t`, the original's randy31 `RKeyFrameAnimation_t`: `AnarchyReimagined`
/// `dropin/randy31/src/keyanim.rs`): keys at times for the translation (`trans_keys`, 16 bytes: x y z, the time), the
/// rotation (`rot_keys`, 20: a quaternion x y z w, the time), the visibility (`vis_keys`, 8: the time, a flag byte) and the
/// texture transform (`uv_keys`, 24: the u and v scales, the u and v offsets, the time, whether the offsets are blended); its
/// length (`tot_time`) and whether it loops (`loop`).
#[derive(Clone, Debug, Default)]
pub struct Keys {
    pub length: f32,
    pub looped: bool,
    pub translations: Vec<([f32; 3], f32)>,
    pub rotations: Vec<([f32; 4], f32)>,
    pub visibility: Vec<(f32, bool)>,
    pub textures: Vec<([f32; 4], f32, bool)>,
}

impl Keys {
    /// FUN_10028fde: the animation's matrix (the rotation and the translation; the rest the identity's) and visibility at
    /// `time` (past the length: its remainder when looping, else the length; each track no later than its last key), the
    /// translation blended and the rotation slerped between the keys about it.
    pub fn evaluate(&self, time: f32) -> (Matrix, bool) {
        let mut t = time;
        if self.length < t {
            t = if self.looped && self.length > 0.0 { t % self.length } else { self.length };
        }
        // (the key at or before `t` and the share of the way to the next)
        fn around(times: &[f32], t: f32) -> (usize, Option<(usize, f32)>) {
            let last = times.len() - 1;
            let t = t.min(times[last]);
            let k = times.iter().rposition(|&k| k <= t).unwrap_or(0);
            if times[k] == t || k == last {
                (k, None)
            } else {
                (k, Some((k + 1, (t - times[k]) / (times[k + 1] - times[k]))))
            }
        }
        let mut p = [0.0; 3];
        if !self.translations.is_empty() {
            let times: Vec<f32> = self.translations.iter().map(|k| k.1).collect();
            p = match around(&times, t) {
                (k, None) => self.translations[k].0,
                (k, Some((n, f))) => {
                    let (a, b) = (self.translations[k].0, self.translations[n].0);
                    [0, 1, 2].map(|i| a[i] * (1.0 - f) + b[i] * f)
                }
            };
        }
        let mut q = [0.0, 0.0, 0.0, 1.0];
        if !self.rotations.is_empty() {
            let times: Vec<f32> = self.rotations.iter().map(|k| k.1).collect();
            q = match around(&times, t) {
                (k, None) => self.rotations[k].0,
                (k, Some((n, f))) => crate::catanim::slerp(self.rotations[k].0, self.rotations[n].0, f),
            };
        }
        let visible = if self.visibility.is_empty() {
            true
        } else {
            let times: Vec<f32> = self.visibility.iter().map(|k| k.0).collect();
            self.visibility[around(&times, t).0].1
        };
        (trs_matrix(p, q, [1.0; 3]), visible)
    }

    /// The texture transform at `time` (u scale, v scale, u offset, v offset), when there are two keys or more: the
    /// scales blended between the keys about it, the offsets too when the key says so, else the key's (FUN_10028fde's tail).
    pub fn texture(&self, time: f32) -> Option<[f32; 4]> {
        if self.textures.len() < 2 {
            return None;
        }
        let mut t = time;
        if self.length < t {
            t = if self.looped && self.length > 0.0 { t % self.length } else { self.length };
        }
        let last = self.textures.len() - 1;
        let t = t.min(self.textures[last].1);
        let k = self.textures.iter().rposition(|k| k.1 <= t).unwrap_or(0);
        let (v0, t0, blend) = self.textures[k];
        if t0 == t || k == last {
            return Some(v0);
        }
        let (v1, t1, _) = self.textures[k + 1];
        let f = (t - t0) / (t1 - t0);
        let mix = |i: usize| v1[i] * f + v0[i] * (1.0 - f);
        Some(if blend { [mix(0), mix(1), mix(2), mix(3)] } else { [mix(0), mix(1), v0[2], v0[3]] })
    }
}

/// One frame on the way from a mesh's root to a part: its own transform, the `anim_matrix` it was saved with and its
/// animation.
#[derive(Clone, Debug)]
pub struct Step {
    trs: Matrix,
    saved: Option<Matrix>,
    keys: Option<std::sync::Arc<Keys>>,
    /// The root of a placed mesh: its position left out (see `walk`).
    zero_translation: bool,
}

/// A part whose frames (its own or one above it) are animated: its world matrix at a time, as the original's
/// `RRefFrame_t::SetAnimationTime` (each animated frame's matrix evaluated at the time wrapped into its length, its length
/// itself for a whole multiple) and `UpdateWorldMatrix` (`anim * TRS * parent`) make it.
#[derive(Clone, Debug)]
pub struct Animated {
    chain: Vec<Step>,
}

impl Animated {
    /// The part's world matrix at `time` (seconds) and whether its frames are all visible.
    pub fn world_at(&self, time: f32) -> (Matrix, bool) {
        let mut world = IDENTITY;
        let mut visible = true;
        for s in &self.chain {
            let anim = match &s.keys {
                Some(k) => {
                    let mut t = if k.length > 0.0 { time % k.length } else { 0.0 };
                    if t == 0.0 && time > 0.0 {
                        t = k.length;
                    }
                    let (m, v) = k.evaluate(t);
                    visible &= v;
                    Some(m)
                }
                None => s.saved,
            };
            let mut m = anim.map_or(s.trs, |a| multiply(&a, &s.trs));
            if s.zero_translation {
                m[12] = 0.0;
                m[13] = 0.0;
                m[14] = 0.0;
            }
            world = multiply(&m, &world);
        }
        (world, visible)
    }

    /// The texture transform of the part's own frame's animation at `time` (visual.rs `rasterize`: stage 0's texture matrix,
    /// `u * a + c`, `v * b + d`).
    pub fn texture_at(&self, time: f32) -> Option<[f32; 4]> {
        let k = self.chain.last()?.keys.as_ref()?;
        let mut t = if k.length > 0.0 { time % k.length } else { 0.0 };
        if t == 0.0 && time > 0.0 {
            t = k.length;
        }
        k.texture(t)
    }

    /// Whether anything about the part changes with time (its place, its visibility or its texture), sampled across its
    /// length.
    pub fn moves(&self) -> bool {
        let length = self.length();
        if length <= 0.0 {
            return false;
        }
        let (w0, v0) = self.world_at(0.0);
        let u0 = self.texture_at(0.0);
        (1..=16).any(|k| {
            let t = length * k as f32 / 16.0;
            let (w, v) = self.world_at(t);
            v != v0 || w.iter().zip(&w0).any(|(a, b)| (a - b).abs() > 1e-4) || self.texture_at(t) != u0
        })
    }

    /// Whether every animation in its chain loops (the keyframe animation's `loop` flag).
    pub fn looped(&self) -> bool {
        self.chain.iter().filter_map(|s| s.keys.as_ref()).all(|k| k.looped)
    }

    /// The longest of its animations (the "animation tree's total time", `GetAnimationTreeTotalTime`).
    pub fn length(&self) -> f32 {
        self.chain.iter().filter_map(|s| s.keys.as_ref().map(|k| k.length)).fold(0.0, f32::max)
    }
}

/// A frame's own transform: its rotation, scale and position.
fn trs_of(frame: &Stream) -> Matrix {
    let scale = float_field(frame, "scale").and_then(|v| v.first().copied()).unwrap_or(1.0);
    let q = float_field(frame, "local_rot").filter(|q| q.len() == 4).unwrap_or(vec![0.0, 0.0, 0.0, 1.0]);
    let p = float_field(frame, "local_pos").filter(|p| p.len() == 3).unwrap_or(vec![0.0; 3]);
    trs_matrix([p[0], p[1], p[2]], [q[0], q[1], q[2], q[3]], [scale; 3])
}

fn saved_anim_matrix(frame: &Stream) -> Option<Matrix> {
    float_field(frame, "anim_matrix").filter(|a| a.len() == 16).map(|a| a.try_into().unwrap())
}

/// A frame's animation's keys, when it has an animation.
fn keys(a: &Archive, frame: &Stream) -> Option<Keys> {
    let anim = a.objects.get(usize::try_from(frame.int("0", "anim")?).ok()?)?;
    let data = |name: &str| anim.field("", name).and_then(|f| f.values.first()).map(|v| floats(v)).unwrap_or_default();
    let translations: Vec<([f32; 3], f32)> = data("trans_keys").chunks_exact(4).map(|k| ([k[0], k[1], k[2]], k[3])).collect();
    let rotations: Vec<([f32; 4], f32)> = data("rot_keys").chunks_exact(5).map(|k| ([k[0], k[1], k[2], k[3]], k[4])).collect();
    let visibility: Vec<(f32, bool)> = anim
        .field("", "vis_keys")
        .and_then(|f| f.values.first())
        .map(|v| v.chunks_exact(8).map(|k| (f32::from_le_bytes(k[..4].try_into().unwrap()), k[4] != 0)).collect())
        .unwrap_or_default();
    let textures: Vec<([f32; 4], f32, bool)> = anim
        .field("", "uv_keys")
        .and_then(|f| f.values.first())
        .map(|v| v.chunks_exact(24).map(|k| { let f = floats(k); ([f[0], f[1], f[2], f[3]], f[4], u32::from_le_bytes(k[20..24].try_into().unwrap()) != 0) }).collect())
        .unwrap_or_default();
    if translations.is_empty() && rotations.is_empty() && visibility.is_empty() && textures.is_empty() {
        return None;
    }
    let length = float_field(anim, "tot_time").and_then(|v| v.first().copied()).unwrap_or(0.0);
    let looped = anim.field("", "loop").and_then(|f| f.values.first()).and_then(|v| v.first()).is_some_and(|&b| b != 0);
    Some(Keys { length, looped, translations, rotations, visibility, textures })
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
    let _ = a;
    Ok(build_animated(mesh, root)?.0)
}

/// The parts and, for each, its animation (when a frame on its way from the root has one).
fn build_animated(mesh: &MeshObject, root: Root) -> Result<(Model, Vec<Option<Animated>>), Error> {
    let a = &mesh.archive;
    let (mut parts, mut anims, mut chain) = (Vec::new(), Vec::new(), Vec::new());
    for root_frame in refs(&a.root, "obj") {
        walk(a, root_frame, &IDENTITY, 0, root, &mut parts, &mut chain, &mut anims)?;
    }
    let _ = kind::OBJECT;
    Ok((Model { parts, instances: Vec::new(), water: None, lod_groups: Vec::new() }, anims))
}

/// [`build_dynel`]'s parts with their animations (a door's leaves, a Whom-Pah's creature), one per part.
pub fn build_dynel_animated(mesh: &MeshObject) -> Result<(Model, Vec<Option<Animated>>), Error> {
    build_animated(mesh, Root::None)
}

/// [`build_placed`]'s parts with their animations, one per part.
pub fn build_placed_animated(mesh: &MeshObject) -> Result<(Model, Vec<Option<Animated>>), Error> {
    build_animated(mesh, Root::Rotation)
}
