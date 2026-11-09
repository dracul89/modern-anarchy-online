//! What the effect scripts show: DisplaySystem's environment visuals over their FXS objects (`VisualEnvFX_t`'s kinds,
//! the effects an `FXID` starts; `AnarchyReimagined`'s `dropin/displaysystem`), made each frame into plain [`Draw`]s the
//! viewer places at the camera and draws.
//!
//! - kind 1, `EnvironmentLightObject_t` (`fxvisual.rs` `environment_frame`, FUN_1005d589): the ambient light and the
//!   ground's, clouds', water's and three space lights (each its `…Current` colour and `…Direction`'s turn of (0, 0, 1),
//!   or (0, 1, 0) without one), [`Lights`].
//! - kind 4, `GenericMeshObject_t` (`fxgmo.rs`: FUN_1005e716 the frame, FUN_1005c124 the process, FUN_1005c334 the draw):
//!   a mesh by name (`Mesh`) at its `Position` (to the camera for `PositionType` 1), turned by `Rotation` (type 0),
//!   scaled by `Scale` (type 1 by the view distance over 1000, type 2 also by how far its `InitDirection` points from up,
//!   type 3 drawn in from 950–5000 out), lit by the light its `SunlightType` names, hidden at a `Visibility` of 0.01 or
//!   less and let go when its `Use` is 1.
//! - kind 5, `GenericVisualObject_t` (`fxgeneric.rs`: FUN_1005b633, FUN_1005c917) over a state blob's blueprint (the FXS
//!   `render.rs` `blueprint_generate_mesh` / `blueprint_render`): the blueprint object's `Vertex` points times its `Size`
//!   (`PointMapping` strict), its `Color` table's colours (alpha first: the I, R, G, B rows), random texture coordinates,
//!   drawn as a strip; facing the camera about y for `RotationType` 2.
//! - kind 0x2a, `VisualSunRays_t` (`fxsunrays.rs`: FUN_1005a7f6 the fan, FUN_1005a3a9 the frame, FUN_1005abb1 the draw):
//!   a fan of `Vertices` round a centre 100 ahead (−z), `Size` × 9 wide, scaled 1.7, coloured `ColorR/G/B`, dimmed by the
//!   horizon (black below it, faded in over its first 0.01).
//!
//! The state blob drawn with is the last valid one (its criteria all nonzero, no render target); its render states give
//! the depth test (`ZFUNC` always: drawn before the world, [`Draw::backdrop`]), the blending, the lighting, the ambient
//! light, the texture factor and `MODULATE2X`; its texture names one; its first stage's texture transform (`TEXTURE0`
//! under `COUNT2`) moves the coordinates — the clouds' and horizon's `This.ScrollMatrix` works out to the identity, as in
//! the original: a transform has no owner, so `This` finds nothing on the call stack. The single clouds (0x38) are
//! `clouds.rs`'. Not here: the render targets, the screen fills (0x3d), the debug line monitor (7), the occluders and
//! the attractors' effects.

use super::math::{self, Q4, V3};
use super::parser::enum_value;
use super::{Obj, StateBlob};

/// How a draw is blended.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Blend {
    Opaque,
    Alpha,
    Add,
}

/// A light: its colour and the way it shines.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Light {
    pub colour: [f32; 3],
    pub direction: V3,
}

/// The environment light's lights (DisplaySystem's DAT_10150264 to DAT_10150278) and the ambient colour.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Lights {
    pub ambient: [f32; 3],
    /// `SpecularLightIntensity` (1 when not given): what the lights' specular colours are scaled by.
    pub specular: f32,
    pub ground: Option<Light>,
    pub cloud: Option<Light>,
    pub water: Option<Light>,
    pub space: [Option<Light>; 3],
}

/// A vertex of a made shape: its place (the visual's own frame), texture coordinates and colour (RGBA).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vert {
    pub pos: V3,
    pub uv: [f32; 2],
    pub colour: [f32; 4],
}

#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    /// A static mesh by its file name.
    Mesh(String),
    /// A triangle list.
    Triangles(Vec<Vert>),
}

/// One visual to draw this frame: its shape and placing (the server's frame, Y up), its state blob's look, and its
/// render list place (priority, then sorting).
#[derive(Clone, Debug, PartialEq)]
pub struct Draw {
    pub kind: u32,
    pub priority: u32,
    pub sorting: u32,
    pub shape: Shape,
    pub position: V3,
    pub rotation: Q4,
    pub scale: f32,
    pub position_type: u32,
    pub rotation_type: u32,
    pub scale_type: u32,
    pub init_direction: V3,
    /// `ZFUNC` always and no depth written: drawn before the world, behind everything.
    pub backdrop: bool,
    pub blend: Blend,
    pub cull: bool,
    /// The light when lighting is on (the one the `SunlightType` names, else the ground's), and the ambient render state.
    pub light: Option<Light>,
    pub ambient: [f32; 3],
    /// The colour factor: the texture factor's alpha (when a stage takes it) and `MODULATE2X`'s two.
    pub factor: [f32; 4],
    /// The state blob's texture by name, for shapes without their own.
    pub texture: Option<String>,
    /// A material from the blob (D3D's 17 floats: diffuse, ambient, specular, emissive RGBA, power).
    pub material: Option<[f32; 17]>,
    /// The first stage's texture transform (`TEXTURE0` with `TEXTURETRANSFORMFLAGS` `COUNT2`): the coordinates (u, v, 1)
    /// by its first three rows.
    pub texture_matrix: Option<[f32; 16]>,
}

/// The frame: the draws and the lights.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Frame {
    pub draws: Vec<Draw>,
    pub lights: Lights,
    /// The fog the generic visuals add (`AddFogR/G/B`, `VisualFog_t::AddFog` from FUN_1005b633): the atmosphere's
    /// colour of the haze at the horizon.
    pub fog: Option<[f32; 3]>,
}

/// The visuals running: each started effect's kind and its FXS object, and a generated blueprint's points by object.
#[derive(Default)]
pub struct Visuals {
    list: Vec<(u32, Obj)>,
    blueprints: std::collections::HashMap<u32, Vec<Vert>>,
    rand: crate::audio::Rand,
    /// The single clouds' field (made when the effect starts).
    clouds: Option<super::clouds::Clouds>,
}

fn e(name: &str) -> u32 {
    enum_value(name.as_bytes()).unwrap_or(u32::MAX)
}

/// The colour of a D3DCOLOR word (A, R, G, B from the top).
fn argb(w: u32) -> [f32; 4] {
    [((w >> 16) & 0xff) as f32 / 255.0, ((w >> 8) & 0xff) as f32 / 255.0, (w & 0xff) as f32 / 255.0, (w >> 24) as f32 / 255.0]
}

/// What a state blob sets that the draws use, over D3D's defaults.
struct Look {
    backdrop: bool,
    blend: Blend,
    lighting: bool,
    cull: bool,
    ambient: [f32; 3],
    factor: [f32; 4],
    texture: Option<String>,
    texture_matrix: Option<[f32; 16]>,
}

fn look(b: &StateBlob) -> Look {
    let (zfunc, zwrite) = (e("e_D3DRENDERSTATE_ZFUNC"), e("e_D3DRENDERSTATE_ZWRITEENABLE"));
    let (alpha_on, src, dest) = (e("e_D3DRENDERSTATE_ALPHABLENDENABLE"), e("e_D3DRENDERSTATE_SRCBLEND"), e("e_D3DRENDERSTATE_DESTBLEND"));
    let (lighting, ambient, cull) = (e("e_D3DRENDERSTATE_LIGHTING"), e("e_D3DRENDERSTATE_AMBIENT"), e("e_D3DRENDERSTATE_CULLMODE"));
    let state = |which: u32| b.states.iter().rev().find(|s| s[1] == which).map(|s| s[2]);
    let stage = |which: u32| b.stages.iter().rev().find(|s| s[0] == 0 && s[2] == which).map(|s| s[3]);
    let always = state(zfunc) == Some(e("e_D3DCMP_ALWAYS"));
    let no_write = state(zwrite) == Some(e("e_FALSE"));
    let blend = if state(alpha_on) == Some(e("e_TRUE")) {
        if state(dest) == Some(e("e_D3DBLEND_ONE")) {
            Blend::Add
        } else {
            let _ = state(src);
            Blend::Alpha
        }
    } else {
        Blend::Opaque
    };
    // (the texture factor: the special render state's value, when a stage's alpha takes it)
    let tfactor = b.special.as_ref().map(super::rt_word).unwrap_or(0xffff_ffff);
    let takes_tfactor = [e("e_D3DTSS_ALPHAARG1"), e("e_D3DTSS_ALPHAARG2")].iter().any(|&a| stage(a) == Some(e("e_D3DTA_TFACTOR")));
    let mut factor = [1.0f32; 4];
    if takes_tfactor {
        factor[3] = argb(tfactor)[3];
    }
    if stage(e("e_D3DTSS_COLOROP")) == Some(e("e_D3DTOP_MODULATE2X")) {
        for c in &mut factor[..3] {
            *c = 2.0;
        }
    }
    Look {
        backdrop: always && no_write,
        blend,
        lighting: state(lighting).is_none_or(|v| v != e("e_FALSE")),
        cull: state(cull).is_none_or(|v| v != e("e_D3DCULL_NONE")),
        ambient: state(ambient).map_or([0.0; 3], |w| {
            let c = argb(w);
            [c[0], c[1], c[2]]
        }),
        factor,
        texture: b.textures.last().map(|t| String::from_utf8_lossy(&t.2).into_owned()),
        texture_matrix: (stage(e("e_D3DTSS_TEXTURETRANSFORMFLAGS")) == Some(e("e_D3DTTFF_COUNT2")))
            .then(|| b.transforms.iter().rev().find(|t| t.1 == e("e_D3DTRANSFORMSTATE_TEXTURE0")).map(|t| super::rt_matrix(&t.2)))
            .flatten(),
    }
}

/// The state blob to draw with: from the tenth down to the first, the first valid one without a render target.
fn find_blob(o: &Obj) -> Option<std::rc::Rc<StateBlob>> {
    (0..10).rev().filter_map(|k| o.blob(k)).find(|b| b.targets.is_empty() && super::blob_valid(b))
}

/// A light block filled (`light_fill`): its colour from the current variable, its way (0, 0, 1) turned by the
/// direction's rotation, or (0, 1, 0) without one.
fn light_of(o: &Obj, current: &str, direction: &str) -> Option<Light> {
    let c = o.variable_named(current)?;
    let colour = super::vector_of(&c);
    let direction = match o.variable_named(direction) {
        Some(d) => {
            let mut v = [0.0, 0.0, 1.0];
            math::rotate_by(&mut v, &super::quaternion_of(&d));
            v
        }
        None => [0.0, 1.0, 0.0],
    };
    Some(Light { colour, direction })
}

fn word(o: &Obj, name: &str) -> Option<u32> {
    o.variable_named(name).map(|v| super::word(&v))
}

fn float(o: &Obj, name: &str) -> Option<f32> {
    o.variable_named(name).map(|v| super::float_of(&v))
}

impl Visuals {
    /// The effects the scripts started (`FrameProcess`'s requests made visuals): the kinds drawn here kept.
    pub fn start(&mut self, started: Vec<(u32, Obj)>) {
        for (kind, o) in started {
            if matches!(kind, 1 | 4 | 5 | 0x2a | 0x38) && !self.list.iter().any(|(k, x)| *k == kind && std::rc::Rc::ptr_eq(x, &o)) {
                self.list.push((kind, o));
            }
        }
    }

    pub fn clear(&mut self) {
        self.list.clear();
        self.blueprints.clear();
    }

    /// The frame: each visual's draw (a visual whose object has ended, or whose `Use` is 1, let go), the lights.
    pub fn frame(&mut self, sky: &super::clouds::Sky) -> Frame {
        let mut frame = Frame::default();
        self.list.retain(|(_, o)| o.alive() && word(o, "Use") != Some(1));
        let list = self.list.clone();
        // (the environment light first: the others take its lights)
        for (kind, o) in list.iter().filter(|(k, _)| *k == 1) {
            let _ = kind;
            frame.lights = Lights {
                ambient: o.variable_named("AmbientLightCurrent").map_or([1.0; 3], |v| super::vector_of(&v)),
                specular: float(o, "SpecularLightIntensity").unwrap_or(1.0),
                ground: light_of(o, "GroundLightCurrent", "GroundLightDirection"),
                cloud: light_of(o, "CloudLightCurrent", "CloudLightDirection"),
                water: light_of(o, "WaterLightCurrent", "WaterLightDirection"),
                space: [light_of(o, "SpaceLight0Current", "SpaceLight0Direction"), light_of(o, "SpaceLight1Current", "SpaceLight1Direction"), light_of(o, "SpaceLight2Current", "SpaceLight2Direction")],
            };
        }
        for (kind, o) in list.iter().filter(|(k, _)| *k != 1) {
            if *kind == 0x38 {
                frame.draws.extend(self.clouds(o, sky, &frame.lights));
                continue;
            }
            if *kind == 5 {
                let f = ["AddFogR", "AddFogG", "AddFogB"].map(|n| float(o, n));
                if let [Some(r), Some(g), Some(b)] = f {
                    if float(o, "AddFogI").is_some() {
                        frame.fog = Some([r, g, b].map(|c| c.clamp(0.0, 1.0)));
                    }
                }
            }
            if let Some(d) = self.draw(*kind, o, &frame.lights) {
                frame.draws.push(d);
            }
        }
        // (the render lists by priority, each by its sorting)
        frame.draws.sort_by_key(|d| (d.priority, d.sorting));
        frame
    }

    fn draw(&mut self, kind: u32, o: &Obj, lights: &Lights) -> Option<Draw> {
        if float(o, "Visibility").is_some_and(|v| v <= 0.01) {
            return None;
        }
        let blob = find_blob(o)?;
        let l = look(&blob);
        let light = l.lighting.then(|| match word(o, "SunlightType") {
            Some(0) => lights.space[0],
            Some(1) => lights.space[1],
            Some(2) => lights.space[2],
            Some(3 | 4) => lights.cloud,
            Some(5) => lights.water,
            _ => lights.ground,
        });
        let material = super::blob_material(&blob, o).map(|v| {
            let mut m = [0f32; 17];
            for (k, x) in m.iter_mut().enumerate() {
                *x = super::float_at(&v, k as u32);
            }
            m
        });
        let shape = match kind {
            4 => Shape::Mesh(o.variable_named("Mesh").and_then(|v| super::string_of(&v))?),
            5 => Shape::Triangles(self.blueprint(&blob)?),
            _ => Shape::Triangles(sun_rays(o)),
        };
        Some(Draw {
            kind,
            priority: word(o, "Priority").unwrap_or(0),
            sorting: word(o, "Sorting").unwrap_or(0),
            shape,
            position: o.variable_named("Position").map_or([0.0; 3], |v| super::vector_of(&v)),
            rotation: o.variable_named("Rotation").map_or([0.0, 0.0, 0.0, 1.0], |v| super::quaternion_of(&v)),
            scale: float(o, "Scale").unwrap_or(1.0) * if kind == 0x2a { 1.7 } else { 1.0 },
            position_type: word(o, "PositionType").unwrap_or(0),
            rotation_type: word(o, "RotationType").unwrap_or(0),
            scale_type: word(o, "ScaleType").unwrap_or(0),
            init_direction: o.variable_named("InitDirection").map_or([0.0, 0.0, 1.0], |v| super::vector_of(&v)),
            backdrop: l.backdrop,
            blend: l.blend,
            cull: l.cull,
            light: light.flatten(),
            ambient: l.ambient,
            factor: l.factor,
            texture: l.texture,
            material,
            texture_matrix: l.texture_matrix,
        })
    }

    /// The single clouds (`VisualSingleCloud_t`, `clouds.rs`): their four material groups, each a backdrop blended at the
    /// object's priority and sorting, round the camera.
    fn clouds(&mut self, o: &Obj, sky: &super::clouds::Sky, lights: &Lights) -> Vec<Draw> {
        let altitude = float(o, "Altitude").unwrap_or(0.0);
        let sky = super::clouds::Sky { ground: lights.ground, ..*sky };
        let field = self.clouds.get_or_insert_with(super::clouds::Clouds::new);
        field
            .frame(altitude, &sky)
            .into_iter()
            .filter(|(_, v)| !v.is_empty())
            .map(|(texture, vertices)| Draw {
                kind: 0x38,
                priority: word(o, "Priority").unwrap_or(0),
                sorting: word(o, "Sorting").unwrap_or(0),
                shape: Shape::Triangles(vertices),
                position: [0.0; 3],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: 1.0,
                position_type: 1,
                rotation_type: 0,
                scale_type: 0,
                init_direction: [0.0, 0.0, 1.0],
                backdrop: true,
                blend: Blend::Alpha,
                cull: false,
                light: None,
                ambient: [0.0; 3],
                factor: [1.0; 4],
                texture: Some(texture.to_string()),
                material: None,
                texture_matrix: None,
            })
            .collect()
    }

    /// `BlueprintGenerateMesh` then `BlueprintRender`: the blob's blueprint object's points (made once: strict, the
    /// points times the size; or at random on its indexed triangles), its colours each frame (the table's, alpha first,
    /// for a colour manipulation), random texture coordinates; a strip made triangles (points are not drawn).
    fn blueprint(&mut self, blob: &StateBlob) -> Option<Vec<Vert>> {
        let (_, id) = *blob.meshes.last()?;
        let o = super::with(|e| e.named.get(&id).cloned())?;
        let var = |n: &str| o.variable_named(n);
        let count = super::word(&*var("Vertices")?) as usize;
        let size = var("Size").map(|v| super::float_of(&v));
        if !self.blueprints.contains_key(&id) {
            let points = var("Vertex")?;
            let scale = |p: V3| size.map_or(p, |s| math::scale(&p, s));
            let mut made = Vec::with_capacity(count);
            let rand = &mut self.rand;
            match var("PointMapping").map(|v| super::word(&v)) {
                Some(1) => {
                    for i in 0..count as u32 {
                        made.push(Vert { pos: scale(super::vector_at(&points, i)), uv: [0.0; 2], colour: [1.0; 4] });
                    }
                }
                Some(0) => {
                    let indices = var("Indice")?;
                    let triangles = super::count_of(&indices) / 3;
                    for _ in 0..count {
                        let t = (rand.next() as u32 % triangles.max(1)) * 3;
                        let (p, q, r) = (super::word_at(&indices, t), super::word_at(&indices, t + 1), super::word_at(&indices, t + 2));
                        let (s, tt) = ((rand.next() % 10000) as f32 / 10000.0, (rand.next() % 10000) as f32 / 10000.0);
                        let a = super::vector_at(&points, p);
                        let b = super::vector_at(&points, q);
                        let c = super::vector_at(&points, r);
                        let p1 = math::add(&math::scale(&math::sub(&b, &a), tt), &a);
                        made.push(Vert { pos: scale(math::add(&math::scale(&math::sub(&c, &p1), s), &p1)), uv: [0.0; 2], colour: [1.0; 4] });
                    }
                }
                _ => return None,
            }
            if var("TextureMapping").map(|v| super::word(&v)) == Some(0) {
                for v in &mut made {
                    v.uv = [(self.rand.next() % 10000) as f32 / 10000.0, (self.rand.next() % 10000) as f32 / 10000.0];
                }
            }
            self.blueprints.insert(id, made);
        }
        let mut verts = self.blueprints.get(&id)?.clone();
        if var("ColorMapping").map(|v| super::word(&v)) == Some(1) {
            if let Some(table) = var("Color") {
                for (i, v) in verts.iter_mut().enumerate() {
                    let base = (i % count.max(1)) as u32 * 4;
                    let a = super::float_at(&table, base);
                    let b = super::float_at(&table, base + 3);
                    let g = super::float_at(&table, base + 2);
                    let r = super::float_at(&table, base + 1);
                    v.colour = [r, g, b, a].map(|c| c.clamp(0.0, 1.0));
                }
            }
        }
        match var("Visualization").map(|v| super::word(&v)) {
            Some(1) => Some(strip(&verts)),
            _ => None,
        }
    }
}

/// A triangle strip's triangles.
fn strip(v: &[Vert]) -> Vec<Vert> {
    let mut out = Vec::new();
    for k in 2..v.len() {
        if k % 2 == 0 {
            out.extend([v[k - 2], v[k - 1], v[k]]);
        } else {
            out.extend([v[k - 1], v[k - 2], v[k]]);
        }
    }
    out
}

/// FUN_1005a7f6 and FUN_1005a3a9: the sun-ray fan: `Vertices` (33) points round the circle, `Size` (6) × 9 out, 100
/// ahead (−z), their texture coordinates round the middle by `UVSize` (0.8), coloured `ColorR/G/B` (255, 155, 55),
/// the centre last; the brightness by the horizon (its forward's height: black below, faded in over the first 0.01, or
/// for `Horizon` 2 black above 0.17, faded out from 0.15) as every vertex's alpha (the flicker table is not kept).
fn sun_rays(o: &Obj) -> Vec<Vert> {
    let n = word(o, "Vertices").unwrap_or(0x21).max(3);
    let size = float(o, "Size").unwrap_or(6.0);
    let uv = float(o, "UVSize").unwrap_or(0.8);
    let colour = [word(o, "ColorR").unwrap_or(255), word(o, "ColorG").unwrap_or(155), word(o, "ColorB").unwrap_or(55)].map(|c| (c & 0xff) as f32 / 255.0);
    let rotation = o.variable_named("Rotation").map_or([0.0, 0.0, 0.0, 1.0], |v| super::quaternion_of(&v));
    let mut forward = [0.0, 0.0, -1.0];
    math::rotate_by(&mut forward, &rotation);
    let y = forward[1];
    let fourth = |t: f32| t * t * t * t;
    let bright = match word(o, "Horizon") {
        Some(2) => {
            if 0.17 < y {
                0.0
            } else if 0.15 < y {
                fourth(1.0 - (y - 0.15) * 100.0 * 0.5)
            } else {
                1.0
            }
        }
        _ => {
            if 0.0 > y {
                0.0
            } else if 0.01 > y {
                fourth(y * 100.0)
            } else {
                1.0
            }
        }
    };
    let alpha = ((bright * 255.0) as i32) as f32 / 255.0;
    let ring = n - 1;
    let mut points: Vec<Vert> = (0..ring)
        .map(|i| {
            let a = (360.0 / ring as f64 * i as f64 * 3.140000104904175 / 180.0) as f32;
            let (hs, hc) = (a.sin() * 0.5, a.cos() * 0.5);
            Vert { pos: [hs * 2.0 * size * 9.0, 2.0 * hc * size * 9.0, -100.0], uv: [uv * hs + 0.5, uv * hc + 0.5], colour: [colour[0], colour[1], colour[2], alpha] }
        })
        .collect();
    let centre = Vert { pos: [0.0, 0.0, -100.0], uv: [0.5, 0.5], colour: [colour[0], colour[1], colour[2], alpha] };
    let mut out = Vec::new();
    points.push(points[0]);
    for k in 0..ring as usize {
        out.extend([points[k], points[k + 1], centre]);
    }
    out
}

/// A draw's world matrix (row vectors, the server's frame), as the visuals' draws place themselves: the scale (type 1
/// by `distance` / 1000, type 2 also by 0.1 to 1 as its initial direction turned by the rotation points less up), the
/// rotation (type 0 its own; type 2 facing the camera's forward about y), the place (type 1 the camera's `eye` plus the
/// position, times `distance` / 1000 for a mesh; type 3's drawn in from 950–5000 out, lowered and shrunk).
pub fn place(d: &Draw, eye: V3, forward: V3, distance: f32) -> [f32; 16] {
    let mut scale = d.scale;
    let mut at = d.position;
    match d.scale_type {
        1 => scale = d.scale * distance / 1000.0,
        2 => {
            let mut dir = d.init_direction;
            math::rotate_by(&mut dir, &d.rotation);
            let up = if 0.0 < dir[1] { 1.0 - dir[1] } else { 1.0 };
            scale = d.scale * distance / 1000.0 * (up * 0.9 + 0.1);
        }
        3 => {
            let rel = math::sub(&at, &eye);
            let dd = math::len2(&rel).sqrt();
            if 950.0 < dd && 5000.0 > dd {
                let t = (dd - 950.0) / 4050.0;
                let towards = math::scale(&math::with_len(&rel, 1.0), 950.0);
                let mut p = math::add(&eye, &towards);
                p[1] *= 1.0 - t * t * t * t;
                at = p;
                scale = 1.0 - t;
            } else {
                scale = 1.0;
            }
        }
        _ => {}
    }
    let rotation = if d.rotation_type == 2 {
        let a = forward[2].atan2(forward[0]);
        math::axis_angle(&[0.0, 1.0, 0.0], 1.5700000524520874 - a)
    } else {
        d.rotation
    };
    let mut m = math::q_matrix(&rotation);
    math::m_scale(&mut m, &[scale, scale, scale]);
    if d.position_type == 1 {
        let off = if d.kind == 4 { math::scale(&d.position, distance / 1000.0) } else { d.position };
        at = math::add(&eye, &off);
    }
    m[12] += at[0];
    m[13] += at[1];
    m[14] += at[2];
    m
}

/// Texture coordinates by a draw's texture transform: (u, v, 1) by its first three rows (D3D's `COUNT2`).
pub fn texture_uv(d: &Draw, uv: [f32; 2]) -> [f32; 2] {
    match &d.texture_matrix {
        Some(m) => [uv[0] * m[0] + uv[1] * m[4] + m[8], uv[0] * m[1] + uv[1] * m[5] + m[9]],
        None => uv,
    }
}

/// A point (row vector) by a world matrix.
pub fn transform(m: &[f32; 16], p: V3) -> V3 {
    [0, 1, 2].map(|j| p[0] * m[j] + p[1] * m[4 + j] + p[2] * m[8 + j] + m[12 + j])
}

/// A direction by a world matrix's turn (unit).
pub fn turn(m: &[f32; 16], n: V3) -> V3 {
    let v = [0, 1, 2].map(|j| n[0] * m[j] + n[1] * m[4 + j] + n[2] * m[8 + j]);
    let l = math::len2(&v).sqrt();
    if l > 0.0 { v.map(|c| c / l) } else { v }
}

/// D3D's lighting of a vertex: the material's emissive, the ambient render state by its ambient, the light by its
/// diffuse as the normal faces it; the material's diffuse alpha.
pub fn lit(d: &Draw, normal: V3, material: ([f32; 4], [f32; 3], [f32; 3])) -> [f32; 4] {
    let (diffuse, ambient, emissive) = material;
    let Some(light) = d.light else { return [1.0; 4] };
    let k = -(normal[0] * light.direction[0] + normal[1] * light.direction[1] + normal[2] * light.direction[2]);
    let k = k.max(0.0);
    let c = |i: usize| emissive[i] + d.ambient[i] * ambient[i] + light.colour[i] * diffuse[i] * k;
    [c(0), c(1), c(2), diffuse[3]]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_outdoor_sky_s_draws() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        super::super::reset();
        super::super::set_data_path(&std::path::Path::new(&game).join("cd_image"));
        super::super::load_playfield(999_999, 0);
        super::super::start_automatic();
        let (sun1, sun2) = super::super::sun::rotations(12.0 * 3600.0);
        super::super::set_game(&super::super::Game { delta_time: 0.05, day_time: 12.0 * 3600.0 / 15.0, sun1, sun2, ..Default::default() });
        let mut v = Visuals::default();
        v.start(super::super::take_started());
        super::super::frame_process();
        super::super::set_game(&super::super::Game { delta_time: 0.05, day_time: 12.0 * 3600.0 / 15.0, sun1, sun2, ..Default::default() });
        let sky = super::super::clouds::Sky { sun_direction: [0.0, 1.0, 0.0], ..Default::default() };
        let f = v.frame(&sky);
        assert!(f.draws.iter().any(|d| d.kind == 0x38));
        // (the thick clouds' and the horizon's texture transforms are taken, but their `This.ScrollMatrix` finds no object:
        // a state blob's transform has no owner, so nothing is on the call stack when it is worked out and the lazy pointer
        // drops its path — the identity, as in the original, where nothing else pushes the call stack)
        let clouds = f.draws.iter().find(|d| d.shape == Shape::Mesh("skydome_clouds.abiff".into())).unwrap();
        assert_eq!(clouds.texture_matrix, Some(math::IDENTITY));
        let horizon = f.draws.iter().find(|d| d.shape == Shape::Mesh("horizon_object.abiff".into())).unwrap();
        assert_eq!(horizon.texture_matrix, Some(math::IDENTITY));
        let shifted = Draw { texture_matrix: Some([3.0, 0.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, 0.25, 0.5, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]), ..clouds.clone() };
        assert_eq!(texture_uv(&shifted, [0.5, 0.25]), [1.75, 1.25]);
        assert!(f.fog.is_some());
        for d in &f.draws {
            let shape = match &d.shape {
                Shape::Mesh(m) => m.clone(),
                Shape::Triangles(t) => format!("{} triangles", t.len() / 3),
            };
            eprintln!("kind {} priority {} sorting {} {shape} backdrop {} {:?} lit {:?} factor {:?} texture {:?}", d.kind, d.priority, d.sorting, d.backdrop, d.blend, d.light.map(|l| l.colour), d.factor, d.texture);
        }
        eprintln!("lights {:?}", f.lights);
        assert!(f.draws.iter().any(|d| d.shape == Shape::Mesh("star_dome.abiff".into()) || d.shape == Shape::Mesh("nebulas_sphere.abiff".into())));
        assert!(f.draws.iter().any(|d| d.kind == 5 && matches!(&d.shape, Shape::Triangles(t) if t.len() == 12)));
        assert!(f.lights.ground.is_some());
    }
}

#[cfg(test)]
mod orientation {
    /// The horizon's dish (`horizon_object.abiff`, rim at 0, centre 16 out along its z) turned by the script's
    /// `v(0,0,1),90 [ROT] v(0,1,0),90` lies level below the eye with its data's z negated and `[ROT]` as `fxs` composes it.
    #[test]
    fn the_horizon_dish_lies_below_the_eye() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let db = std::sync::Arc::new(mao_formats::resource::ResourceDatabase::open_game(std::path::Path::new(&game)).unwrap());
        let textures = crate::Textures::new(db);
        let m = textures.mesh_named("horizon_object.abiff").unwrap();
        let ax = |axis: [f32; 3], deg: f32| super::math::axis_angle(&axis, deg.to_radians());
        // (the composition `fxs` uses: the operand's turn first)
        let mut q = ax([0.0, 1.0, 0.0], 90.0);
        super::math::q_mul_in(&mut q, &ax([0.0, 0.0, 1.0], 90.0));
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        for v in &m.parts[0].vertices {
            let mut p = [v.position[0], v.position[1], -v.position[2]];
            super::math::rotate_by(&mut p, &q);
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        assert!(lo[1] < -16.0 && hi[1].abs() < 0.01, "{lo:?} {hi:?}");
        assert!(lo[0] < -99.0 && hi[0] > 99.0 && lo[2] < -99.0 && hi[2] > 99.0);
    }
}
