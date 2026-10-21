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

/// A frame's own matrix.
fn local_matrix(frame: &Stream) -> Matrix {
    let animated = frame.int("0", "anim").is_some_and(|a| a >= 0);
    if animated {
        if let Some(m) = float_field(frame, "anim_matrix").filter(|m| m.len() == 16) {
            return m.try_into().unwrap();
        }
    }
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
    m
}

fn rgb(s: &Stream, name: &str) -> [f32; 3] {
    float_field(s, name).filter(|v| v.len() >= 3).map(|v| [v[0], v[1], v[2]]).unwrap_or([1.0; 3])
}

fn material(a: &Archive, index: i32) -> Material {
    let Some(m) = (index >= 0).then(|| a.objects.get(index as usize)).flatten() else { return Material { opacity: 1.0, ..Default::default() } };
    let mut textures = Vec::new();
    if let Some(state) = m.int("0", "delta_state").and_then(|i| a.objects.get(i as usize)) {
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
    let vertices = (0..count)
        .map(|i| {
            let b = i * stride;
            Vertex {
                position: [f(b), f(b + 4), f(b + 8)],
                normal: normal.map(|n| [f(b + n), f(b + n + 4), f(b + n + 8)]).unwrap_or_default(),
                uv: uv.map(|t| [f(b + t), f(b + t + 4)]).unwrap_or_default(),
            }
        })
        .collect();
    let indices = s
        .int("0", "trilist")
        .and_then(|i| a.objects.get(i as usize))
        .and_then(|t| t.field("", "triangles"))
        .and_then(|f| f.values.first())
        .map(|b| b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect())
        .unwrap_or_default();
    out.push(Part { name: name.to_string(), world, vertices, indices, material: material(a, s.int("0", "material").unwrap_or(-1)) });
    Ok(())
}

fn walk(a: &Archive, index: i32, parent: &Matrix, depth: usize, out: &mut Vec<Part>) -> Result<(), Error> {
    if depth > 64 {
        return Err(Error::Shape("a frame tree deeper than 64"));
    }
    let Some(frame) = (index >= 0).then(|| a.objects.get(index as usize)).flatten() else { return Ok(()) };
    let world = multiply(&local_matrix(frame), parent);
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
        walk(a, child, &world, depth + 1, out)?;
    }
    Ok(())
}

/// The drawable parts of a mesh object.
pub fn build(mesh: &MeshObject) -> Result<Model, Error> {
    let a = &mesh.archive;
    let mut parts = Vec::new();
    for root in refs(&a.root, "obj") {
        walk(a, root, &IDENTITY, 0, &mut parts)?;
    }
    let _ = kind::OBJECT;
    Ok(Model { parts, instances: Vec::new() })
}
