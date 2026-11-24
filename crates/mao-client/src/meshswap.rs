//! Stand-in meshes read from files: an experiment, off unless one of its two variables names a folder.
//!
//! `MAO_MESH_REPLACE=<folder>`: `<folder>/<instance in hex>.glb` is drawn in the place of the static mesh
//! 0xf6951:<instance> wherever the client loads that mesh without its animation (a head on a body, worn gear, a prop,
//! the mesh viewer). The file is a binary glTF whose positions and normals are already in the space of the original
//! mesh's part that the file's mesh is named after (else its first part), so the stand-in keeps that part's world
//! matrix and material and only brings its own triangles and its base colour picture; the original's other parts (a
//! head's separate hair) are left out, the file stands for the whole mesh. Node transforms in the file are not applied and one picture serves every primitive: whatever makes the
//! file bakes those. Which way the file's triangles wind is settled against the original's (see [`outward`]), so a
//! file made with a mirrored axis still shows its outside.
//!
//! In the same folder `<type in hex>_<instance in hex>.png` (`f6954_40f1.png`) stands in for that texture, whatever
//! draws it: a body's skin or armour, a wall, the ground. It is taken as it is (its alpha too, which the materials
//! read as the original's), at any size.
//!
//! `MAO_TEXTURE_EXPORT=<folder>`: every texture the client decodes from the database is written there under that
//! name, once: the list of what a scene uses, and what a stand-in is painted from.
//!
//! `MAO_MESH_EXPORT=<folder>`: every static mesh that is loaded is also written there as `<instance in hex>.obj`
//! (each part in its own space, its world matrix and texture in comments) with its parts' textures as PNG files: what
//! a stand-in is fitted to. Files made from the game's meshes are not for the repository.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use mao_formats::model::{Model, Part, Vertex};
use mao_formats::resource::{Id, ResourceDatabase};
use mao_formats::texture::{self, Format, Texture};

/// The texture type a stand-in's material names: none of the database's. [`crate::Textures`] asks [`texture`] for it.
pub const TEXTURE_KIND: u32 = 0x4d53_5750;

fn folder(variable: &str) -> Option<PathBuf> {
    std::env::var_os(variable).map(PathBuf::from).filter(|p| p.is_dir())
}

fn replace_folder() -> Option<&'static PathBuf> {
    static FOLDER: OnceLock<Option<PathBuf>> = OnceLock::new();
    FOLDER.get_or_init(|| folder("MAO_MESH_REPLACE")).as_ref()
}

fn export_folder() -> Option<&'static PathBuf> {
    static FOLDER: OnceLock<Option<PathBuf>> = OnceLock::new();
    FOLDER.get_or_init(|| folder("MAO_MESH_EXPORT")).as_ref()
}

fn texture_export_folder() -> Option<&'static PathBuf> {
    static FOLDER: OnceLock<Option<PathBuf>> = OnceLock::new();
    FOLDER.get_or_init(|| folder("MAO_TEXTURE_EXPORT")).as_ref()
}

/// A texture's stand-in from the folder, when it has one.
pub fn texture_stand_in(kind: u32, instance: u32) -> Option<Texture> {
    let path = replace_folder()?.join(format!("{kind:x}_{instance:x}.png"));
    if !path.is_file() {
        return None;
    }
    let rgba = image::open(&path).map_err(|e| eprintln!("texture stand-in {}: {e}", path.display())).ok()?.to_rgba8();
    Some(Texture { width: rgba.width(), height: rgba.height(), format: Format::Png, rgba: rgba.into_raw(), mips: 1, packed: None })
}

/// A texture decoded from the database, written out when that is asked for (its first level, once).
pub fn export_texture(kind: u32, instance: u32, texture: &Texture) {
    let Some(folder) = texture_export_folder() else { return };
    let picture = folder.join(format!("{kind:x}_{instance:x}.png"));
    let size = (texture.width * texture.height * 4) as usize;
    if picture.exists() || texture.rgba.len() < size {
        return;
    }
    if let Err(e) = image::save_buffer(&picture, &texture.rgba[..size], texture.width, texture.height, image::ColorType::Rgba8) {
        eprintln!("texture export {}: {e}", picture.display());
    }
}

/// A static mesh as it was built from the database, or its stand-in when the folder has one (and the original written
/// out first, when that is asked for).
pub fn apply(db: &ResourceDatabase, id: Id, model: Model) -> Model {
    if id.kind != 0xf6951 {
        return model;
    }
    if let Some(folder) = export_folder() {
        if let Err(e) = export(db, folder, id.instance, &model) {
            eprintln!("mesh export {:x}: {e}", id.instance);
        }
    }
    let Some(path) = replace_folder().map(|f| f.join(format!("{:x}.glb", id.instance))).filter(|p| p.is_file()) else { return model };
    match stand_in(&path, id.instance, &model) {
        Ok(new) => {
            eprintln!("mesh {:x}: stand-in {} ({} triangles in {} parts)", id.instance, path.display(), new.parts.iter().map(|p| p.indices.len() / 3).sum::<usize>(), new.parts.len());
            new
        }
        Err(e) => {
            eprintln!("mesh stand-in {}: {e}", path.display());
            model
        }
    }
}

/// The base colour picture of a mesh's stand-in.
pub fn texture(instance: u32) -> Option<Texture> {
    let glb = Glb::read(&replace_folder()?.join(format!("{instance:x}.glb"))).ok()?;
    let rgba = image::load_from_memory(glb.base_colour()?).ok()?.to_rgba8();
    Some(Texture { width: rgba.width(), height: rgba.height(), format: Format::Png, rgba: rgba.into_raw(), mips: 1, packed: None }.with_mips())
}

/// A binary glTF file: its description and its one block of bytes.
struct Glb {
    doc: serde_json::Value,
    bin: Vec<u8>,
}

/// A file's triangles, all its primitives together.
#[derive(Default)]
struct Geometry {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    triangles: Vec<[u32; 3]>,
}

impl Glb {
    fn read(path: &Path) -> Result<Glb, String> {
        let data = std::fs::read(path).map_err(|e| e.to_string())?;
        let word = |at: usize| data.get(at..at + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize).ok_or("the file is cut short");
        if data.get(..4) != Some(b"glTF") || data.get(16..20) != Some(b"JSON") {
            return Err("not a binary glTF file".into());
        }
        let json = word(12)?;
        let doc = serde_json::from_slice(data.get(20..20 + json).ok_or("the file is cut short")?).map_err(|e| e.to_string())?;
        let at = 20 + json;
        let bin = data.get(at + 8..at + 8 + word(at)?).ok_or("the file has no block of bytes")?.to_vec();
        Ok(Glb { doc, bin })
    }

    /// A buffer view's bytes and the step between its elements (0: packed).
    fn view(&self, index: u64) -> Result<(&[u8], usize), String> {
        let view = &self.doc["bufferViews"][index as usize];
        let (start, length) = (view["byteOffset"].as_u64().unwrap_or(0) as usize, view["byteLength"].as_u64().ok_or("a buffer view without a length")? as usize);
        Ok((self.bin.get(start..start + length).ok_or("a buffer view beyond the file")?, view["byteStride"].as_u64().unwrap_or(0) as usize))
    }

    /// An accessor's elements, each its `width` numbers read by `number` from the component's bytes.
    fn elements<T>(&self, accessor: &serde_json::Value, width: usize, size: usize, number: impl Fn(&[u8]) -> T) -> Result<Vec<T>, String> {
        let (bytes, stride) = self.view(accessor["bufferView"].as_u64().ok_or("an accessor without a buffer view")?)?;
        let (count, start) = (accessor["count"].as_u64().unwrap_or(0) as usize, accessor["byteOffset"].as_u64().unwrap_or(0) as usize);
        let step = if stride == 0 { width * size } else { stride };
        let mut out = Vec::with_capacity(count * width);
        for element in 0..count {
            for n in 0..width {
                let at = start + element * step + n * size;
                out.push(number(bytes.get(at..at + size).ok_or("an accessor beyond its buffer view")?));
            }
        }
        Ok(out)
    }

    fn floats(&self, index: &serde_json::Value, width: usize) -> Result<Vec<f32>, String> {
        let accessor = &self.doc["accessors"][index.as_u64().ok_or("a primitive without that attribute")? as usize];
        if accessor["componentType"].as_u64() != Some(5126) {
            return Err("an attribute that is not stored as floats".into());
        }
        self.elements(accessor, width, 4, |b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn indices(&self, index: &serde_json::Value) -> Result<Vec<u32>, String> {
        let accessor = &self.doc["accessors"][index.as_u64().ok_or("a primitive without indices")? as usize];
        match accessor["componentType"].as_u64() {
            Some(5121) => self.elements(accessor, 1, 1, |b| u32::from(b[0])),
            Some(5123) => self.elements(accessor, 1, 2, |b| u32::from(u16::from_le_bytes([b[0], b[1]]))),
            Some(5125) => self.elements(accessor, 1, 4, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])),
            _ => Err("indices of an unknown size".into()),
        }
    }

    fn geometry(&self) -> Result<Geometry, String> {
        let mut all = Geometry::default();
        let primitives = self.doc["meshes"].as_array().into_iter().flatten().flat_map(|m| m["primitives"].as_array().into_iter().flatten());
        for primitive in primitives {
            if primitive["mode"].as_u64().unwrap_or(4) != 4 {
                return Err("a primitive that is not a list of triangles".into());
            }
            let attributes = &primitive["attributes"];
            let base = all.positions.len() as u32;
            let positions = self.floats(&attributes["POSITION"], 3)?;
            all.positions.extend(positions.chunks_exact(3).map(|p| [p[0], p[1], p[2]]));
            let count = all.positions.len() - base as usize;
            match self.floats(&attributes["NORMAL"], 3) {
                Ok(normals) if normals.len() == count * 3 => all.normals.extend(normals.chunks_exact(3).map(|n| [n[0], n[1], n[2]])),
                _ => return Err("a primitive without a normal for each vertex".into()),
            }
            match self.floats(&attributes["TEXCOORD_0"], 2) {
                Ok(uvs) if uvs.len() == count * 2 => all.uvs.extend(uvs.chunks_exact(2).map(|t| [t[0], t[1]])),
                _ => return Err("a primitive without a texture coordinate for each vertex".into()),
            }
            let indices = self.indices(&primitive["indices"])?;
            if indices.iter().any(|&i| i as usize >= count) {
                return Err("an index beyond the primitive's vertices".into());
            }
            all.triangles.extend(indices.chunks_exact(3).map(|t| [base + t[0], base + t[1], base + t[2]]));
        }
        if all.triangles.is_empty() {
            return Err("no triangles in the file".into());
        }
        Ok(all)
    }

    /// The first material's base colour picture, as the file holds it (a PNG or a JPEG).
    fn base_colour(&self) -> Option<&[u8]> {
        let texture = self.doc["materials"].as_array()?.iter().find_map(|m| m["pbrMetallicRoughness"]["baseColorTexture"]["index"].as_u64())?;
        let image = self.doc["textures"][texture as usize]["source"].as_u64()?;
        Some(self.view(self.doc["images"][image as usize]["bufferView"].as_u64()?).ok()?.0)
    }
}

/// Which way a mesh's triangles wind against its normals: the sum, over the triangles, of the corner order's normal
/// along the vertices' own. The game's meshes and a file's are both made with normals that point out of the surface,
/// so where the two sums differ in sign the file's corners run the other way round from what the renderer takes as
/// the front, whatever axes the file was fitted with.
fn outward(triangles: impl Iterator<Item = [usize; 3]>, position: impl Fn(usize) -> [f32; 3], normal: impl Fn(usize) -> [f32; 3]) -> f32 {
    let sub = |a: [f32; 3], b: [f32; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    triangles
        .map(|[a, b, c]| {
            let (u, v) = (sub(position(b), position(a)), sub(position(c), position(a)));
            let face = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
            let n = [a, b, c].map(&normal).iter().fold([0.0f32; 3], |s, n| [s[0] + n[0], s[1] + n[1], s[2] + n[2]]);
            face[0] * n[0] + face[1] * n[1] + face[2] * n[2]
        })
        .sum()
}

fn stand_in(path: &Path, instance: u32, original: &Model) -> Result<Model, String> {
    let glb = Glb::read(path)?;
    // (the original's part the file is named after, a head's `head` and not its `hair`; else the first)
    let named = glb.doc["meshes"][0]["name"].as_str();
    let template = original.parts.iter().find(|p| Some(p.name.as_str()) == named).or(original.parts.first()).ok_or("the mesh has no part to stand in for")?;
    let geometry = glb.geometry()?;
    let theirs = outward(template.indices.chunks_exact(3).map(|t| [t[0] as usize, t[1] as usize, t[2] as usize]), |i| template.vertices[i].position, |i| template.vertices[i].normal);
    let ours = outward(geometry.triangles.iter().map(|t| t.map(|i| i as usize)), |i| geometry.positions[i], |i| geometry.normals[i]);
    let turned = (theirs < 0.0) != (ours < 0.0);
    let mut material = template.material.clone();
    material.textures = vec![(TEXTURE_KIND, instance)];
    material.ignore_alpha = true;
    // (a part's indices are 16 bits wide: a file with more vertices than that goes into several parts)
    let empty = || Part { name: template.name.clone(), world: template.world, vertices: Vec::new(), indices: Vec::new(), material: material.clone() };
    let (mut parts, mut part, mut placed) = (Vec::new(), empty(), std::collections::HashMap::new());
    for triangle in &geometry.triangles {
        if part.vertices.len() + 3 > usize::from(u16::MAX) {
            parts.push(std::mem::replace(&mut part, empty()));
            placed.clear();
        }
        let corners = if turned { [triangle[0], triangle[2], triangle[1]] } else { *triangle };
        for corner in corners {
            let index = *placed.entry(corner).or_insert_with(|| {
                part.vertices.push(Vertex { position: geometry.positions[corner as usize], normal: geometry.normals[corner as usize], uv: geometry.uvs[corner as usize], colour: Vertex::UNBAKED });
                (part.vertices.len() - 1) as u16
            });
            part.indices.push(index);
        }
    }
    parts.push(part);
    Ok(Model { parts, ..Default::default() })
}

fn export(db: &ResourceDatabase, folder: &Path, instance: u32, model: &Model) -> Result<(), String> {
    let mut text = format!("# static mesh f6951:{instance:x}; each part in its own space, v of vt downwards as stored\n");
    let mut base = 1;
    for (index, matrix) in &model.instances {
        let _ = writeln!(text, "# instance {index} {}", matrix.map(|n| n.to_string()).join(" "));
    }
    for part in &model.parts {
        let _ = writeln!(text, "o {}\n# world {}", part.name, part.world.map(|n| n.to_string()).join(" "));
        if let Some(&(kind, skin)) = part.material.textures.first() {
            let _ = writeln!(text, "# texture {kind:x} {skin:x}");
            let picture = folder.join(format!("{kind:x}_{skin:x}.png"));
            if !picture.exists() {
                if let Some(t) = db.get(Id { kind, instance: skin }).ok().flatten().and_then(|o| texture::decode(&o).ok()) {
                    image::save_buffer(&picture, &t.rgba[..(t.width * t.height * 4) as usize], t.width, t.height, image::ColorType::Rgba8).map_err(|e| e.to_string())?;
                }
            }
        }
        for v in &part.vertices {
            let _ = writeln!(text, "v {} {} {}", v.position[0], v.position[1], v.position[2]);
        }
        for v in &part.vertices {
            let _ = writeln!(text, "vt {} {}", v.uv[0], v.uv[1]);
        }
        for v in &part.vertices {
            let _ = writeln!(text, "vn {} {} {}", v.normal[0], v.normal[1], v.normal[2]);
        }
        for t in part.indices.chunks_exact(3) {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| base + usize::from(i));
            let _ = writeln!(text, "f {a}/{a}/{a} {b}/{b}/{b} {c}/{c}/{c}");
        }
        base += part.vertices.len();
    }
    std::fs::write(folder.join(format!("{instance:x}.obj")), text).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::outward;

    /// A triangle whose corners run counter-clockwise seen from +Z, with normals along +Z, winds outward; the same
    /// corners the other way round do not.
    #[test]
    fn winding_is_told_against_the_normals() {
        let positions = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let sum = |order: [usize; 3]| outward(std::iter::once(order), |i| positions[i], |_| [0.0, 0.0, 1.0]);
        assert!(sum([0, 1, 2]) > 0.0);
        assert!(sum([0, 2, 1]) < 0.0);
    }
}
