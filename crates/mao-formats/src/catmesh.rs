//! Character meshes: the resource database's `RDBCATMesh_t` objects (types
//! 0xf6952 and 0xf696b), the "CAT" skinned mesh format.
//!
//! Derived from the original's `RDBCATMesh_t` (`AnarchyReimagined`:
//! `dropin/displaysystem/src/rdbcatmesh.rs`), `TextureData_t`
//! (`dropin/gamedata/src/visual.rs`) and randy31's `CATMesh_t` reader
//! (`dropin/randy31/src/cat.rs`). Everything is little-endian.
//!
//! # The object
//! The usual 12-byte head, then
//! * a name (32 bytes),
//! * the textures: an `i32` of `(count + 1) * 0x3f1`, then per texture a name
//!   (32 bytes), two `i32`s and a flag `i32` (the mode),
//! * a colour `i32` (four bytes, shown through a curve in the original),
//! * the mesh: `i32` 4, an `i32` version (0x67 or 0x104), an `i32`, two
//!   floats, then
//!   * materials: a count, and per material a name (string), flags (`i32`,
//!     bit 0 two-sided, bit 1 has an environment texture, bit 3 opaque), a
//!     texture name (string), the environment texture's name when flag 2,
//!     four colours (diffuse, ambient, specular, emissive; three floats
//!     each) and three floats (shininess, strength, opacity);
//!   * spheres: a count, each four floats and an `i32`;
//!   * bones: a count, each a name, a scale (float) and the child bones'
//!     indices (a count and `i32`s);
//!   * groups: a count, each a name; its triangle lists (a count, each a
//!     material index, a vertex count and vertices of 17 words, an index
//!     count and `u16` indices), spheres (a count, each four floats and an
//!     `i32`) and attractors (a count, each a name, eight floats and an
//!     `i32`).
//!
//! A string is an `i32` length and the bytes.
//!
//! # A vertex (17 words)
//! `[0..3]` the position in the first bone's space, `[3..6]` in the second
//! bone's, `[6..9]` the unskinned position (zero before version 0x68),
//! `[9..12]` the normal, `[12..14]` the texture coordinates, `[14]` and
//! `[15]` (as `i32`) the two bones, `[16]` the first bone's weight. Skinned,
//! a vertex's position is the second bone's matrix applied to `[3..6]` times
//! `1 - weight`, plus the first's applied to `[0..3]` times the weight (just
//! the first when the weight is above 0.99); the normal is `[9..12]` turned
//! by the first bone. Unskinned it is `[6..12]` as it is.

use std::fmt;

use crate::texture::OBJECT_HEAD;

#[derive(Debug)]
pub enum Error {
    Truncated(&'static str),
    NotCat,
    Version(i32),
    Material(i32),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Truncated(what) => write!(f, "character mesh truncated in {what}"),
            Error::NotCat => write!(f, "not a CAT mesh"),
            Error::Version(v) => write!(f, "CAT mesh version {v:#x} (0x67 and 0x104 are known)"),
            Error::Material(i) => write!(f, "triangle list with material {i}, which doesn't exist"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Clone, Debug, Default)]
pub struct TextureRef {
    pub name: String,
    pub value1: i32,
    pub value2: i32,
    pub flag: i32,
}

#[derive(Clone, Debug, Default)]
pub struct Material {
    pub name: String,
    pub flags: i32,
    pub texture: String,
    pub environment: Option<String>,
    pub diffuse: [f32; 3],
    pub ambient: [f32; 3],
    pub specular: [f32; 3],
    pub emissive: [f32; 3],
    pub shininess: f32,
    pub strength: f32,
    pub opacity: f32,
}

impl Material {
    pub fn two_sided(&self) -> bool {
        self.flags & 1 != 0
    }

    /// Whether the original draws it with alpha transparency (not flag 8).
    pub fn transparent(&self) -> bool {
        self.flags & 8 == 0
    }
}

#[derive(Clone, Debug, Default)]
pub struct Bone {
    pub name: String,
    pub scale: f32,
    pub children: Vec<i32>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Vertex {
    /// In the first bone's space.
    pub first: [f32; 3],
    /// In the second bone's space.
    pub second: [f32; 3],
    /// Unskinned (zero before version 0x68).
    pub rest: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub bones: [i32; 2],
    /// The first bone's weight.
    pub weight: f32,
}

#[derive(Clone, Debug, Default)]
pub struct TriangleList {
    pub material: usize,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
}

#[derive(Clone, Debug, Default)]
pub struct Attractor {
    pub name: String,
    pub values: [f32; 8],
    pub extra: i32,
}

#[derive(Clone, Debug, Default)]
pub struct Group {
    pub name: String,
    pub lists: Vec<TriangleList>,
    pub spheres: Vec<([f32; 4], i32)>,
    pub attractors: Vec<Attractor>,
}

#[derive(Clone, Debug, Default)]
pub struct CatMesh {
    pub name: String,
    pub textures: Vec<TextureRef>,
    /// The object's colour word.
    pub colour: u32,
    pub version: i32,
    pub materials: Vec<Material>,
    pub spheres: Vec<([f32; 4], i32)>,
    pub bones: Vec<Bone>,
    pub groups: Vec<Group>,
}

struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize, what: &'static str) -> Result<&'a [u8], Error> {
        let end = self.at.checked_add(n).filter(|&e| e <= self.data.len()).ok_or(Error::Truncated(what))?;
        let b = &self.data[self.at..end];
        self.at = end;
        Ok(b)
    }

    fn i32(&mut self, what: &'static str) -> Result<i32, Error> {
        Ok(i32::from_le_bytes(self.take(4, what)?.try_into().unwrap()))
    }

    fn u16(&mut self, what: &'static str) -> Result<u16, Error> {
        Ok(u16::from_le_bytes(self.take(2, what)?.try_into().unwrap()))
    }

    fn f32(&mut self, what: &'static str) -> Result<f32, Error> {
        Ok(f32::from_le_bytes(self.take(4, what)?.try_into().unwrap()))
    }

    fn rgb(&mut self, what: &'static str) -> Result<[f32; 3], Error> {
        Ok([self.f32(what)?, self.f32(what)?, self.f32(what)?])
    }

    fn string(&mut self, what: &'static str) -> Result<String, Error> {
        let n = self.i32(what)?;
        if !(0..=0x10000).contains(&n) {
            return Err(Error::Truncated(what));
        }
        Ok(String::from_utf8_lossy(self.take(n as usize, what)?).trim_end_matches('\0').to_string())
    }

    /// A count that has to be plausible for what follows it.
    fn count(&mut self, what: &'static str) -> Result<usize, Error> {
        let n = self.i32(what)?;
        if n < 0 || n as usize > self.data.len() {
            return Err(Error::Truncated(what));
        }
        Ok(n as usize)
    }

    /// A fixed-size NUL-padded text.
    fn text(&mut self, n: usize, what: &'static str) -> Result<String, Error> {
        let b = self.take(n, what)?;
        let end = b.iter().position(|&c| c == 0).unwrap_or(n);
        Ok(String::from_utf8_lossy(&b[..end]).into_owned())
    }
}

/// Reads an object (with its 12-byte head).
pub fn parse(object: &[u8]) -> Result<CatMesh, Error> {
    let mut r = Reader { data: object, at: OBJECT_HEAD };
    let mut mesh = CatMesh { name: r.text(0x20, "the name")?, ..Default::default() };
    let prefix = r.i32("the texture count")?;
    let textures = prefix.wrapping_div(0x3f1).wrapping_sub(1);
    if textures < 0 || textures as usize > object.len() {
        return Err(Error::Truncated("the texture count"));
    }
    for _ in 0..textures {
        mesh.textures.push(TextureRef { name: r.text(0x20, "a texture name")?, value1: r.i32("a texture")?, value2: r.i32("a texture")?, flag: r.i32("a texture")? });
    }
    mesh.colour = r.i32("the colour")? as u32;
    if r.i32("the mesh marker")? != 4 {
        return Err(Error::NotCat);
    }
    mesh.version = r.i32("the version")?;
    if mesh.version != 0x104 && mesh.version != 0x67 {
        return Err(Error::Version(mesh.version));
    }
    r.i32("a word")?;
    r.f32("a float")?;
    r.f32("a float")?;
    for _ in 0..r.count("the materials")? {
        let name = r.string("a material name")?;
        let flags = r.i32("flags")?;
        let texture = r.string("a texture name")?;
        let environment = if flags & 2 != 0 { Some(r.string("an environment texture")?) } else { None };
        let (diffuse, ambient, specular, emissive) = (r.rgb("colours")?, r.rgb("colours")?, r.rgb("colours")?, r.rgb("colours")?);
        mesh.materials.push(Material { name, flags, texture, environment, diffuse, ambient, specular, emissive, shininess: r.f32("shininess")?, strength: r.f32("strength")?, opacity: r.f32("opacity")? });
    }
    for _ in 0..r.count("the spheres")? {
        mesh.spheres.push(([r.f32("a sphere")?, r.f32("a sphere")?, r.f32("a sphere")?, r.f32("a sphere")?], r.i32("a sphere")?));
    }
    for _ in 0..r.count("the bones")? {
        let name = r.string("a bone name")?;
        let scale = r.f32("a bone scale")?;
        let mut children = Vec::new();
        for _ in 0..r.count("a bone's children")? {
            children.push(r.i32("a child bone")?);
        }
        mesh.bones.push(Bone { name, scale, children });
    }
    for _ in 0..r.count("the groups")? {
        let mut group = Group { name: r.string("a group name")?, ..Default::default() };
        for _ in 0..r.count("the triangle lists")? {
            let index = r.i32("a material index")?;
            if index < 0 || index as usize > mesh.materials.len() {
                return Err(Error::Material(index));
            }
            let mut list = TriangleList { material: index as usize, ..Default::default() };
            for _ in 0..r.count("the vertices")? {
                // Words 14 and 15 are integers; before 0x68 words 6 to 8 are not stored.
                let mut w = [0u32; 17];
                for (c, v) in w.iter_mut().enumerate() {
                    *v = if (6..9).contains(&c) && mesh.version < 0x68 { 0 } else { r.i32("a vertex")? as u32 };
                }
                let f: Vec<f32> = w.iter().map(|&v| f32::from_bits(v)).collect();
                let bits = |c: usize| w[c] as i32;
                list.vertices.push(Vertex {
                    first: [f[0], f[1], f[2]],
                    second: [f[3], f[4], f[5]],
                    rest: [f[6], f[7], f[8]],
                    normal: [f[9], f[10], f[11]],
                    uv: [f[12], f[13]],
                    bones: [bits(14), bits(15)],
                    weight: f[16],
                });
            }
            for _ in 0..r.count("the indices")? {
                list.indices.push(r.u16("an index")?);
            }
            group.lists.push(list);
        }
        for _ in 0..r.count("a group's spheres")? {
            group.spheres.push(([r.f32("a sphere")?, r.f32("a sphere")?, r.f32("a sphere")?, r.f32("a sphere")?], r.i32("a sphere")?));
        }
        for _ in 0..r.count("the attractors")? {
            let name = r.string("an attractor name")?;
            let mut values = [0f32; 8];
            for v in &mut values {
                *v = r.f32("an attractor")?;
            }
            group.attractors.push(Attractor { name, values, extra: r.i32("an attractor")? });
        }
        mesh.groups.push(group);
    }
    Ok(mesh)
}

/// A bone's matrix: three rotation rows and a translation row (row vectors).
pub type BoneMatrix = [f32; 12];

const IDENTITY_BONE: BoneMatrix = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0];

/// A point through a bone's matrix (its translation added).
fn place(b: &BoneMatrix, p: [f32; 3]) -> [f32; 3] {
    [0, 1, 2].map(|c| p[0] * b[c] + p[1] * b[3 + c] + p[2] * b[6 + c] + b[9 + c])
}

/// A direction through a bone's rotation.
fn turn(b: &BoneMatrix, p: [f32; 3]) -> [f32; 3] {
    [0, 1, 2].map(|c| p[0] * b[c] + p[1] * b[3 + c] + p[2] * b[6 + c])
}

impl CatMesh {
    /// The bones' matrices when `anim` is at `time`: from the root, each
    /// bone's rotation and (scaled) translation composed with its parent's,
    /// as the original's `CATRender_t::Pose`.
    pub fn pose(&self, anim: &crate::catanim::CatAnim, time: f32) -> Vec<BoneMatrix> {
        self.pose_with(&|bone| (anim.rotation(bone, time), anim.translation(bone, time)))
    }

    /// The pose cross-faded from `a` at `ta` to `b` at `tb` (`weight` 0 is all `a`, 1 all `b`): each bone's
    /// rotation and translation blended, so a change of animation doesn't pop.
    pub fn pose_blend(&self, a: &crate::catanim::CatAnim, ta: f32, b: &crate::catanim::CatAnim, tb: f32, weight: f32) -> Vec<BoneMatrix> {
        self.pose_with(&|bone| {
            let (ra, rb) = (a.rotation(bone, ta), b.rotation(bone, tb));
            // The shorter way round, then normalised.
            let dot: f32 = (0..4).map(|i| ra[i] * rb[i]).sum();
            let sign = if dot < 0.0 { -1.0 } else { 1.0 };
            let mut r = [0.0f32; 4];
            for i in 0..4 {
                r[i] = ra[i] * (1.0 - weight) + rb[i] * sign * weight;
            }
            let length = r.iter().map(|v| v * v).sum::<f32>().sqrt().max(1e-6);
            let (ta_, tb_) = (a.translation(bone, ta), b.translation(bone, tb));
            (r.map(|v| v / length), [0, 1, 2].map(|i| ta_[i] * (1.0 - weight) + tb_[i] * weight))
        })
    }

    fn pose_with(&self, sample: &dyn Fn(usize) -> ([f32; 4], [f32; 3])) -> Vec<BoneMatrix> {
        let mut out = vec![IDENTITY_BONE; self.bones.len()];
        if !self.bones.is_empty() {
            self.pose_bone(sample, 0, &IDENTITY_BONE, 1.0, &mut out, 0);
        }
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn pose_bone(&self, sample: &dyn Fn(usize) -> ([f32; 4], [f32; 3]), bone: usize, parent: &BoneMatrix, scale: f32, out: &mut [BoneMatrix], depth: usize) {
        if depth > 64 {
            return;
        }
        let ([x, y, z, w], t) = sample(bone);
        let (xx, xy, xz, xw) = (2.0 * x * x, 2.0 * x * y, 2.0 * x * z, 2.0 * x * w);
        let (yy, yz, yw) = (2.0 * y * y, 2.0 * y * z, 2.0 * y * w);
        let (zz, zw) = (2.0 * z * z, 2.0 * z * w);
        let rows = [[1.0 - yy - zz, zw + xy, xz - yw], [xy - zw, 1.0 - xx - zz, yz + xw], [yw + xz, yz - xw, 1.0 - xx - yy]];
        let combine = |r: &[f32; 3], c: usize| r[0] * parent[c] + r[1] * parent[3 + c] + r[2] * parent[6 + c];
        let mut m = [0.0f32; 12];
        for (r, row) in rows.iter().enumerate() {
            for c in 0..3 {
                m[r * 3 + c] = combine(row, c);
            }
        }
        let scaled = [t[0] * scale, t[1] * scale, t[2] * scale];
        for c in 0..3 {
            m[9 + c] = combine(&scaled, c) + parent[9 + c];
        }
        out[bone] = m;
        let child_scale = self.bones[bone].scale * scale;
        for &child in &self.bones[bone].children {
            if child >= 0 && (child as usize) < self.bones.len() {
                self.pose_bone(sample, child as usize, &m, child_scale, out, depth + 1);
            }
        }
    }

    /// The mesh drawn with `bones` (from [`CatMesh::pose`]): each vertex
    /// blended between its two bones by its weight (the first only when the
    /// weight is above 0.99). Z-up.
    pub fn skinned_model(&self, bones: &[BoneMatrix], texture_kind: u32) -> crate::model::Model {
        self.build_model(texture_kind, |v| {
            let (b1, b2) = (v.bones[0], v.bones[1]);
            let (Some(first), Some(second)) = (usize::try_from(b1).ok().and_then(|i| bones.get(i)), usize::try_from(b2).ok().and_then(|i| bones.get(i))) else {
                return (v.rest, v.normal);
            };
            let w = v.weight;
            let position = if w > 0.99 {
                place(first, v.first)
            } else {
                let (a, b) = (place(first, v.first), place(second, v.second));
                [0, 1, 2].map(|c| b[c] * (1.0 - w) + a[c] * w)
            };
            (position, turn(first, v.normal))
        })
    }

    /// The mesh as drawable parts in its unskinned (rest) pose, one part per
    /// triangle list, Z-up (the meshes are Y-up). `texture_kind` is the
    /// resource type the texture instances are of (0xf6954 for these).
    pub fn rest_model(&self, texture_kind: u32) -> crate::model::Model {
        self.build_model(texture_kind, |v| (v.rest, v.normal))
    }

    fn build_model(&self, texture_kind: u32, vertex: impl Fn(&Vertex) -> ([f32; 3], [f32; 3])) -> crate::model::Model {
        use crate::model::{Material as ModelMaterial, Model, Part, Vertex as ModelVertex};
        // Y-up to Z-up.
        let swap = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0];
        let mut parts = Vec::new();
        for g in &self.groups {
            for l in &g.lists {
                let Some(m) = self.materials.get(l.material) else { continue };
                // The object's texture list names the instance of each texture.
                let instance = self.textures.iter().find(|t| t.name.eq_ignore_ascii_case(&m.name)).map(|t| t.value1 as u32);
                let triangles: Vec<u16> = l.indices.iter().copied().filter(|&i| (i as usize) < l.vertices.len()).collect();
                if triangles.len() < 3 || l.vertices.is_empty() {
                    continue;
                }
                parts.push(Part {
                    name: format!("{}:{}", g.name, m.name),
                    world: swap,
                    vertices: l
                        .vertices
                        .iter()
                        .map(|v| {
                            let (position, normal) = vertex(v);
                            ModelVertex { position, normal, uv: v.uv }
                        })
                        .collect(),
                    indices: triangles,
                    material: ModelMaterial { name: m.name.clone(), diffuse: m.diffuse, specular: m.specular, ambient: m.ambient, emissive: m.emissive, opacity: 1.0, textures: instance.map(|i| vec![(texture_kind, i)]).unwrap_or_default(), ignore_alpha: true, foliage: false },
                });
            }
        }
        Model { parts, instances: Vec::new(), water: None }
    }
}

impl CatMesh {
    /// The attractor (attachment point) with this name prefix, such as
    /// `Attractor01_head`.
    pub fn attractor(&self, name_part: &str) -> Option<&Attractor> {
        let wanted = name_part.to_lowercase();
        self.groups.iter().flat_map(|g| &g.attractors).find(|a| a.name.to_lowercase().contains(&wanted))
    }

    /// A static mesh (`piece`, such as a head, a helmet or a weapon) held at
    /// an attractor: its vertices put through the attractor's placement
    /// (a position, a quaternion x y z w and a scale, in the bone's space)
    /// and then the bone's matrix from `bones`. Returned as parts ready to
    /// add to the skinned model (Z-up).
    pub fn attach(&self, bones: &[BoneMatrix], attractor: &Attractor, piece: &crate::model::Model) -> Vec<crate::model::Part> {
        use crate::model::{multiply, trs_matrix};
        let swap = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0];
        let v = attractor.values;
        let quat = [v[3], v[4], v[5], v[6]];
        let placement = trs_matrix([v[0], v[1], v[2]], quat, [v[7]; 3]);
        let bone = usize::try_from(attractor.extra).ok().and_then(|i| bones.get(i)).copied().unwrap_or(IDENTITY_BONE);
        let bone4 = [bone[0], bone[1], bone[2], 0.0, bone[3], bone[4], bone[5], 0.0, bone[6], bone[7], bone[8], 0.0, bone[9], bone[10], bone[11], 1.0];
        let mut parts = Vec::new();
        let instances: Vec<(usize, [f32; 16])> = if piece.instances.is_empty() { (0..piece.parts.len()).map(|i| (i, crate::model::IDENTITY)).collect() } else { piece.instances.clone() };
        for (index, instance) in instances {
            let Some(part) = piece.parts.get(index) else { continue };
            let m = multiply(&multiply(&multiply(&part.world, &instance), &placement), &bone4);
            let point = |p: [f32; 3]| [p[0] * m[0] + p[1] * m[4] + p[2] * m[8] + m[12], p[0] * m[1] + p[1] * m[5] + p[2] * m[9] + m[13], p[0] * m[2] + p[1] * m[6] + p[2] * m[10] + m[14]];
            let direction = |p: [f32; 3]| [p[0] * m[0] + p[1] * m[4] + p[2] * m[8], p[0] * m[1] + p[1] * m[5] + p[2] * m[9], p[0] * m[2] + p[1] * m[6] + p[2] * m[10]];
            let mut moved = part.clone();
            moved.world = swap;
            for vertex in &mut moved.vertices {
                vertex.position = point(vertex.position);
                let n = direction(vertex.normal);
                let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-9);
                vertex.normal = [n[0] / len, n[1] / len, n[2] / len];
            }
            parts.push(moved);
        }
        parts
    }
}
