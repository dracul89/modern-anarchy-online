//! The `wgpu` renderer: Vulkan, Metal and Direct3D 12 through one code
//! path. It draws [`mao_formats::model::Model`]s: every vertex buffer in the
//! game's meshes is the same layout (position, normal, one texture pair),
//! so one pipeline covers them all.
//!
//! The pipeline works on any target view, so the same code draws to a window
//! and, headless, to a texture that can be saved ([`Gpu::headless`],
//! [`Renderer::render_to_image`]).

use std::collections::{HashMap, HashSet};

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use mao_formats::model::{Model, Vertex};
use mao_formats::texture::Texture;
use wgpu::util::DeviceExt;

pub use glam;
pub use wgpu;

/// A device and its queue.
pub struct Gpu {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    /// The device takes block-compressed textures (BC1 to BC7: every desktop card does).
    pub bc: bool,
}

/// The graphics device the player chose (the launcher's settings: the original's "Rendering Device"), by the name
/// [`adapter_names`] gives it; empty for the system's own choice.
static PREFERRED_ADAPTER: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

/// Names the device every [`Gpu`] made from now on should use when it is there and can draw to the window.
pub fn prefer_adapter(name: &str) {
    *PREFERRED_ADAPTER.lock().unwrap_or_else(|e| e.into_inner()) = name.trim().to_string();
}

/// A device's name as the settings keep it: the card and the interface it is driven through.
pub fn adapter_name(adapter: &wgpu::Adapter) -> String {
    let info = adapter.get_info();
    format!("{} ({:?})", info.name, info.backend)
}

/// The devices this machine can draw with.
pub fn adapter_names() -> Vec<String> {
    let instance = wgpu::Instance::default();
    let mut names: Vec<String> = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all())).iter().map(adapter_name).collect();
    names.dedup();
    names
}

impl Renderer {
    /// Sets how the sun's shadows are drawn: on or off, and the shadow map made anew at the choice's size when it is
    /// another than the one there.
    pub fn set_shadows(&mut self, gpu: &Gpu, shadows: Shadows) {
        self.shadows = shadows != Shadows::Off;
        if shadows.size() != self.shadow_size {
            self.shadow_size = shadows.size();
            let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("shadow map"),
                size: wgpu::Extent3d { width: self.shadow_size, height: self.shadow_size, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            self.shadow_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            self.shadow_bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("shadow"),
                layout: &self.shadow_layout,
                entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&self.shadow_view) }, wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.shadow_sampler) }],
            });
        }
        // (the map is drawn anew with the next frame)
        self.shadow_cache = None;
    }
}

impl Gpu {
    /// A device for rendering to textures (or to `surface`, when given).
    pub fn new(instance: wgpu::Instance, surface: Option<&wgpu::Surface<'_>>) -> Result<Gpu, String> {
        let wanted = PREFERRED_ADAPTER.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let chosen = if wanted.is_empty() {
            None
        } else {
            let found = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all())).into_iter().find(|a| adapter_name(a) == wanted && surface.is_none_or(|s| a.is_surface_supported(s)));
            if found.is_none() {
                eprintln!("the graphics device of the settings ({wanted}) is not there or cannot draw to the window: using the system's choice");
            }
            found
        };
        let adapter = match chosen {
            Some(adapter) => adapter,
            None => pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: surface,
                force_fallback_adapter: false,
                ..Default::default()
            }))
            .map_err(|e| e.to_string())?,
        };
        // (timestamps where the device has them: the frame's time on the card, for the performance overlay)
        let timing = wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS;
        let mut required_features = if adapter.features().contains(timing) { timing } else { wgpu::Features::empty() };
        // (block-compressed textures where the device has them: see `block_levels`)
        let bc = adapter.features().contains(wgpu::Features::TEXTURE_COMPRESSION_BC);
        if bc {
            required_features |= wgpu::Features::TEXTURE_COMPRESSION_BC;
        }
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor { required_features, ..Default::default() })).map_err(|e| e.to_string())?;
        Ok(Gpu { instance, adapter, device, queue, bc })
    }

    pub fn headless() -> Result<Gpu, String> {
        Gpu::new(wgpu::Instance::default(), None)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuVertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
    /// The baked colour (`model::Vertex::colour`), RGBA bytes.
    colour: [u8; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Globals {
    view_proj: [f32; 16],
    /// The sun's view and projection (the shadow map's).
    light_vp: [f32; 16],
    light: [f32; 4],
    /// The player (world, Z up) and the time in seconds (plants bend away from it).
    player: [f32; 4],
    /// The eye and where the fog starts.
    eye: [f32; 4],
    /// The horizon's colour (the fog's) and where the fog ends.
    horizon: [f32; 4],
    zenith: [f32; 4],
    /// x: 1 when shadows are drawn; y: a shadow-map texel in world units;
    /// z: the water's level; w: 1 when there is water.
    params: [f32; 4],
    /// The water's box: min x, min y, max x, max y.
    rect: [f32; 4],
    /// The camera's right and up (world, Z up).
    right: [f32; 4],
    up: [f32; 4],
    /// rgb: the area's ambient light (the original's `AddAmbientLight`, kept per channel at its maximum against the
    /// sky's); w: the day's intensity, 0 at night to 1 by day.
    ambient: [f32; 4],
    /// rgb: the fog's colour; w: 1 when the fog is an area's (from the camera out), 0 for the distance haze.
    fog: [f32; 4],
    /// The effect scripts' ground light colour (w: 1 when given) and ambient colour.
    script_sun: [f32; 4],
    script_ambient: [f32; 4],
    /// For the motion buffer (temporal anti-aliasing): this frame's view and projection without the jitter, the last frame's,
    /// and the player and time the plants bent by then.
    current_vp: [f32; 16],
    previous_vp: [f32; 16],
    previous_player: [f32; 4],
    /// x: the textures' mip bias ([`mip_bias`]).
    detail: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct PartUniform {
    model: [f32; 16],
    colour: [f32; 4],
    flags: [f32; 4],
    /// Where the part was the last frame, and x: 1 when it animates in ways the matrices don't show (the motion buffer).
    previous_model: [f32; 16],
    motion: [f32; 4],
    /// rgb: an emissive colour added to the part's lighting; a: its alpha (under 1 it is drawn blended after the opaque
    /// parts, at 0 not at all). A character under a highlight effect; otherwise [`NO_TINT`].
    tint: [f32; 4],
    /// The material's blending ([`blend_of`]): x: 0 opaque, 1 blended by alpha, 2 added; y: its alpha test's reference
    /// (under 0: none); z: its opacity; w: 1 when it writes no depth.
    blend: [f32; 4],
}

/// A material's blending for the part's uniform.
fn blend_of(material: &mao_formats::model::Material) -> [f32; 4] {
    use mao_formats::model::Blend;
    // (MAO_NO_BLEND=1: every material cut out as before, to compare)
    static OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    let mode = match material.blend {
        _ if *OFF.get_or_init(|| std::env::var_os("MAO_NO_BLEND").is_some()) => 0.0,
        // (plants stay cut out at a half: blended, their crowns thin to a veil with distance, as the texture's smaller
        // levels average its alpha down, where the original's trees stand dense in its screenshots. Why the original's
        // do not thin is not known: whether its plant textures have smaller levels at all was not read.)
        _ if material.foliage => 0.0,
        Blend::None => 0.0,
        Blend::Alpha => 1.0,
        Blend::Add => 2.0,
    };
    [mode, material.alpha_ref.unwrap_or(-1.0), material.opacity, material.no_z_write as u32 as f32]
}

/// The pipeline a blended part is drawn with: by its uniform's blend (alpha or added) and whether it writes depth.
/// A number for each of the blended pipelines ([`blend_pipeline`]).
fn blend_number(blend: [f32; 4]) -> u32 {
    (blend[0] > 1.5) as u32 * 2 + (blend[3] > 0.5) as u32
}

fn blend_pipeline(pipes: &ScenePipelines, blend: [f32; 4], faces: usize) -> &wgpu::RenderPipeline {
    match (blend[0] > 1.5, blend[3] > 0.5) {
        (false, false) => &pipes.mesh_blend[faces],
        (false, true) => &pipes.mesh_blend_no_z[faces],
        (true, false) => &pipes.mesh_add[faces],
        (true, true) => &pipes.mesh_add_no_z[faces],
    }
}

/// Whether the faces turned away from the camera are drawn. The original does not draw them: its device's cull mode is
/// `D3DCULL_CCW` (randy31 `randy.rs`, the defaults: render state 22 set to 3), and a material draws both sides only when
/// its own states set the mode to none (`Material::two_sided`; about one material in thirteen of the game's meshes).
/// `MAO_CULL=off` draws both sides of everything as this client did before, and `MAO_CULL=show` draws both and paints
/// the faces that would have been left out magenta (to look for geometry whose corners run the wrong way).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Culling {
    #[default]
    On,
    Off,
    Show,
}

impl Culling {
    pub fn from_name(name: &str) -> Culling {
        match name.trim().to_ascii_lowercase().as_str() {
            "off" | "0" | "none" => Culling::Off,
            "show" => Culling::Show,
            _ => Culling::On,
        }
    }
}

/// Which of a part's faces are the ones turned away, as an index into the mesh pipelines: 0 none (both sides drawn), 1
/// and 2 by the sense of the part's matrix. Direct3D culls by the corners' order on the screen, and a matrix that
/// mirrors (a negative determinant) turns that order round; the meshes are drawn here through the swap of the game's
/// upright and the renderer's, which is such a mirror, so that is the usual case (1), and a part mirrored once more, or
/// one given in the renderer's own frame, is the other (2).
fn faces_of(two_sided: bool, world: &Mat4) -> u8 {
    if two_sided {
        0
    } else if world.determinant() < 0.0 {
        1
    } else {
        2
    }
}

/// The corners' order of a front face on the screen for the usual case (see [`faces_of`]).
const FRONT: wgpu::FrontFace = wgpu::FrontFace::Cw;

/// No emissive colour, opaque.
const NO_TINT: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

/// A texture's bind group (the texture and the sampler), with a number of its own to tell when two parts share one.
#[derive(Clone)]
struct TextureBind {
    id: u32,
    group: wgpu::BindGroup,
}

/// The most bytes of vertices, or of indices, packed into one buffer (`Renderer::upload`): a quarter of the largest
/// buffer a device must allow.
const PACK_BYTES: usize = 64 << 20;

/// A number no other bind group, buffer or geometry has.
fn next_id() -> u32 {
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// The bind group of the texture `key` (the white one without a texture), made once and kept.
fn texture_bind(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, sampler: &wgpu::Sampler, white: &wgpu::TextureView, binds: &std::sync::Mutex<HashMap<Option<(u32, u32)>, TextureBind>>, key: Option<(u32, u32)>, view: Option<&wgpu::TextureView>) -> TextureBind {
    let key = key.filter(|_| view.is_some());
    let mut binds = binds.lock().unwrap();
    binds
        .entry(key)
        .or_insert_with(|| TextureBind {
            id: next_id(),
            group: device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout,
                entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(view.unwrap_or(white)) }, wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) }],
            }),
        })
        .clone()
}

/// A part's record for the frame ([`PartUniform`], what the shader reads of it): as it was uploaded, and for a character's
/// part (the overlay's or a follower's) moved by its shift, with where it was the last frame and the mark that it animates
/// of itself (its vertices are uploaded anew as it moves, which the matrices don't show).
fn record_of(part: &GpuPart, character: bool) -> PartUniform {
    let mut u = part.uniform;
    if character {
        let base = Mat4::from_cols_array(&u.model);
        u.model = (Mat4::from_translation(part.shift) * base).to_cols_array();
        u.previous_model = (Mat4::from_translation(part.before.get()) * base).to_cols_array();
        u.motion = [1.0, 0.0, 0.0, 0.0];
    }
    u
}

/// Draws `parts` in their order, their records from `first` on in the same order. With `merge`, the parts that follow
/// one another with the same pipeline, texture and geometry go in one call, as its instances; the pipeline, the texture
/// and each of the two buffers are set only where they change. `pipeline` gives a part's pipeline and a number for it (none: the one
/// already set). Returns the calls made.
fn draw_runs<'a>(pass: &mut wgpu::RenderPass<'_>, parts: &[&GpuPart], first: u32, merge: bool, pipeline: &dyn Fn(&GpuPart) -> Option<(u32, &'a wgpu::RenderPipeline)>) -> usize {
    let (mut calls, mut i) = (0usize, 0usize);
    let (mut set_pipeline, mut set_texture, mut set_vertices, mut set_indices) = (None, None, None, None);
    while i < parts.len() {
        let p = parts[i];
        let chosen = pipeline(p);
        let mut count = 1;
        if merge {
            while i + count < parts.len() && parts[i + count].geometry == p.geometry && parts[i + count].bind.id == p.bind.id && pipeline(parts[i + count]).map(|k| k.0) == chosen.map(|k| k.0) {
                count += 1;
            }
        }
        if let Some((number, pipe)) = chosen {
            if set_pipeline != Some(number) {
                pass.set_pipeline(pipe);
                set_pipeline = Some(number);
            }
        }
        if set_texture != Some(p.bind.id) {
            pass.set_bind_group(1, &p.bind.group, &[]);
            set_texture = Some(p.bind.id);
        }
        // (the buffers are shared by many parts, see `GpuPart`: most draws set neither)
        if set_vertices != Some(p.vertex_id) {
            pass.set_vertex_buffer(0, p.vertices.slice(..));
            set_vertices = Some(p.vertex_id);
        }
        if set_indices != Some(p.index_id) {
            pass.set_index_buffer(p.indices.slice(..), wgpu::IndexFormat::Uint16);
            set_indices = Some(p.index_id);
        }
        pass.draw_indexed(p.first_index..p.first_index + p.index_count, p.base_vertex, first + i as u32..first + (i + count) as u32);
        calls += 1;
        i += count;
    }
    calls
}

struct GpuPart {
    /// The buffers the part's geometry lies in, each with a number of its own, and where: the vertices from
    /// `base_vertex` on, `index_count` indices from `first_index` on (they count from the part's first vertex). The
    /// buffers are shared: a playfield's geometry is packed into a few large ones (`Renderer::upload`) and the
    /// characters' into one of each (`Renderer::reupload`), so that drawing part after part sets no buffer (a command
    /// handed to the card costs about a microsecond whatever it does, and a frame has thousands of parts).
    vertices: wgpu::Buffer,
    vertex_id: u32,
    base_vertex: i32,
    indices: wgpu::Buffer,
    index_id: u32,
    first_index: u32,
    index_count: u32,
    /// A number for the geometry: parts with the same one draw the same vertices and indices.
    geometry: u32,
    /// The buffers are the characters' own pair, which their next upload writes over (`Renderer::reupload`).
    characters: bool,
    bind: TextureBind,
    /// A sphere around the part in the world (to skip it in the shadow pass).
    center: Vec3,
    radius: f32,
    /// What the shader reads of the part, as it was uploaded (`record_of` makes the frame's record of it).
    uniform: PartUniform,
    /// How far the part is moved from where it was uploaded.
    shift: Vec3,
    /// The model part this draw came from, and its texture (what a reused part must match: see `Renderer::reupload`).
    source: usize,
    texture: Option<(u32, u32)>,
    /// A low-detail part: drawn only when the mesh it stands in for is far (`Lod::low`), never on its own.
    low_only: bool,
    /// Which faces are left out ([`faces_of`]).
    faces: u8,
    /// On the first part of a watched mesh's instance: the mesh's sphere and what the decision covers.
    lod: Option<Lod>,
    /// A character's part (the overlay or a follower) under temporal anti-aliasing: its shift when last drawn and whether it
    /// had moved then (none until its motion is first written: see `Renderer::draw`).
    drawn: std::cell::Cell<Option<(Vec3, bool)>>,
    /// A character's part: its shift when the last frame was drawn (the same as now without the anti-aliasing).
    before: std::cell::Cell<Vec3>,
}

/// The original's `LODManager` (displaysystem `lodmanager.rs`, its globals at 0x100ae1a4) on one instance of a statel mesh:
/// the mesh's sphere in the world, how many parts after the head belong to the instance, the indices of its low-detail
/// parts (none when the database has no 0xf696a twin), and whether it was last seen far (the switch happens beyond
/// `distance² − radius² > 6000` and back below 4000). A mesh more than 50 radii off (`distance² / radius² > 2500`) is hidden.
struct Lod {
    center: Vec3,
    radius: f32,
    members: usize,
    low: Vec<usize>,
    far: std::cell::Cell<bool>,
}

/// How far the player may go from where the shadow map was last drawn before it is drawn again (m), and how much the sun may
/// turn (the cosine of the angle). The map covers 70 m around that place and only the playfield throws shadows into it (the
/// characters have blob shadows), so until then the old map is still right.
const SHADOW_MOVE: f32 = 8.0;
const SHADOW_TURN: f32 = 0.99999;

const LOD_NEAR: f32 = 4000.0;
const LOD_FAR: f32 = 6000.0;
const LOD_CULL: f32 = 2500.0;

pub use mao_formats::model::Water;

/// The daylight sky: the horizon's haze and the blue overhead.
pub const DAY_SKY: ([f32; 3], [f32; 3]) = ([0.66, 0.74, 0.84], [0.20, 0.38, 0.72]);
/// The dark an interior or a dungeon is seen against (the fog fades into it).
pub const DARK_SKY: ([f32; 3], [f32; 3]) = ([0.03, 0.035, 0.05], [0.01, 0.012, 0.02]);

/// A model on the GPU, with its bounds.
pub struct Scene {
    parts: Vec<GpuPart>,
    /// Parts of the model (by their index in it) not drawn this frame: an interior's rooms the character cannot see into
    /// (`n3RoomMonitor_t` shows a room on entering it or when a door to it opens).
    pub hidden: std::collections::HashSet<usize>,
    /// Parts drawn after the others and replaceable (see [`Scene::set_overlay`]).
    overlay: Vec<GpuPart>,
    /// Parts that follow a moving point between uploads (the player's character): see [`Scene::set_follow`].
    follow: Vec<GpuPart>,
    pub center: Vec3,
    pub radius: f32,
    /// Where the player stands (world, Z up): plants near it bend away.
    pub player: Option<Vec3>,
    /// Where the fog starts and ends (a view that wants none keeps the default).
    pub fog: (f32, f32),
    /// The sky's colours: at the horizon (the fog's colour too) and overhead.
    pub sky: ([f32; 3], [f32; 3]),
    pub water: Option<Water>,
    /// The liquid surfaces on the card, by layer (see [`LiquidMesh`]), when the playfield has them.
    liquid: Option<LiquidMesh>,
    /// The water light (the effect scripts' `WaterLightCurrent` and `WaterLightDirection`) when they give it: the liquid
    /// passes are lit by it; without one, by the sun and the day.
    pub water_light: Option<WaterLight>,
    /// Soft glowing sprites drawn over the scene, added to its colours (nano effects). Set by the caller every frame.
    pub particles: Vec<Particle>,
    /// The characters' blob shadows (the original's `GfxVisualSimpleShadow_c`): translucent triangles laid on the ground, drawn
    /// after the parts without writing depth. Set by the caller every frame.
    pub blobs: Vec<BlobVertex>,
    /// Translucent textured shells over characters (the original's `GfxVisualShield`): triangle lists of a sprite sheet, added
    /// or blended, depth-tested without writing depth, after the particles' sheets are loaded. Set by the caller every frame.
    pub shells: Vec<Shell>,
    /// The sky's backdrops, in the order drawn (see [`Backdrop`]). Set by the caller every frame.
    pub backdrops: Vec<Backdrop>,
    /// The way to the sun (world, Z up) when the effect scripts give it: the light and its shadow map follow it.
    pub sun: Option<Vec3>,
    /// The effect scripts' lighting when they give it: the ambient colour and the ground light's colour, in place of the
    /// fixed sky and sun colours scaled by the day.
    pub lighting: Option<([f32; 3], [f32; 3])>,
    /// The day's intensity, 0 at night to 1 by day (`GameTime_t::GetDayIntensity`): the sky light, the sun and the sky
    /// are scaled by it (a night keeps a little, so the world stays visible).
    pub daylight: f32,
    /// The area's ambient light at the player (`Environment_t` → `VisualAmbientLight_t::AddAmbientLight`): each channel
    /// the greater of it and the sky light.
    pub ambient: [f32; 3],
    /// The area's fog at the player, when it has one: colour and density. The original's `VisualFog_t::process` makes a
    /// linear fog from the near plane to `far - (far - near - 5) × density`, in that colour.
    pub area_fog: Option<([f32; 3], f32)>,
    /// Whether the camera is under a liquid: the liquid surfaces are drawn 0.2 higher, as `VisualLiquid_t`'s frame (DisplaySystem
    /// FUN_100406d3) raises its frame while `CameraIsUnderWater` was called. Set by the caller every frame.
    pub under_water: bool,
}

/// One sprite: where (world, Z up), how wide (world units) and its colour with the alpha as the strength. A billboard faces the
/// camera; a quad with `right` and `up` set lies in the world (the original's `GfxVisualVolGrid` slices), its corners at the centre
/// plus or minus those half extents, and then `size` is a flag: 1 fades it by how squarely it faces the camera, 0 does not. A
/// quad with only `up` set stands on it and turns about it to face the camera (the original's `GfxVisualSprite3` under
/// `_GfxControlSprite_t`'s flag 0x2000), `size` its half width.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Particle {
    pub position: [f32; 3],
    pub size: f32,
    pub colour: [f32; 4],
    /// The sprite sheet it is cut from ([`Renderer::add_sprite`]; [`NO_SPRITE`] for a soft dot) and the part of it to show: left, top,
    /// width and height in 0..1.
    pub sprite: u16,
    pub rect: [f32; 4],
    /// The half extents of a quad laid in the world (both zero for a billboard).
    pub right: [f32; 3],
    pub up: [f32; 3],
}

/// The sprite number of a particle that is a soft dot, not cut from a sheet.
pub const NO_SPRITE: u16 = u16::MAX;

/// A vertex of a blob shadow: position (world, Z up) and colour with its alpha.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct BlobVertex {
    pub position: [f32; 3],
    pub colour: [f32; 4],
}

/// A vertex of a shell: position (world, Z up), texture coordinates and colour with its alpha.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct ShellVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub colour: [f32; 4],
}

/// How a backdrop is drawn over what is behind it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BackdropBlend {
    Opaque,
    Alpha,
    Add,
}

/// A backdrop: a triangle list of the sky (the effect scripts' domes, moons, suns and clouds), textured with a sprite
/// sheet, drawn in order after the sky's colours and before the world, without the depth test.
#[derive(Clone, Debug, PartialEq)]
pub struct Backdrop {
    pub sprite: u16,
    pub blend: BackdropBlend,
    pub vertices: Vec<ShellVertex>,
}

/// A liquid surface's vertex on the card: its place and normal (world, Z up) and its type's colour.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct LiquidVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub colour: [u8; 4],
}

/// The liquid surfaces on the card, sorted by layer: water (0), lava (1), toxic (2), the star water (3), the wide water
/// (4); `ranges` are each layer's vertices.
pub struct LiquidMesh {
    buffer: wgpu::Buffer,
    ranges: [std::ops::Range<u32>; 5],
}

impl LiquidMesh {
    fn new(gpu: &Gpu, surfaces: &[mao_formats::liquid::Surface]) -> LiquidMesh {
        let mut vertices: Vec<LiquidVertex> = Vec::with_capacity(surfaces.len() * 3);
        let mut ranges: [std::ops::Range<u32>; 5] = Default::default();
        for layer in 0..5u8 {
            let start = vertices.len() as u32;
            for s in surfaces.iter().filter(|s| s.layer == layer) {
                vertices.extend(s.points.iter().map(|&p| LiquidVertex { position: p, normal: s.normal, colour: s.colour }));
            }
            ranges[layer as usize] = start..vertices.len() as u32;
        }
        let buffer = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("liquid"), contents: bytemuck::cast_slice(&vertices), usage: wgpu::BufferUsages::VERTEX });
        LiquidMesh { buffer, ranges }
    }
}

/// The water light: its colour, the way to it (world, Z up) and the scripts' specular intensity (`SpecularLightIntensity`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WaterLight {
    pub colour: [f32; 3],
    pub to_light: Vec3,
    pub specular: f32,
}

/// One liquid pass's uniform (`Liquid` in `liquid.wgsl`).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct LiquidUniform {
    light: [f32; 4],
    specular: [f32; 4],
    to_light: [f32; 4],
    ambient: [f32; 4],
    phases: [f32; 4],
    drift: [f32; 4],
    which: [u32; 4],
    /// x, y: the picture's size in pixels; z: the camera's tilt (its forward's up component), for the refraction pass; w: how far
    /// the surfaces are raised (0.2 with the camera under a liquid).
    screen: [f32; 4],
}

/// The passes' uniforms sit this far apart in their buffer (the dynamic offset alignment every device allows).
const LIQUID_PASS_STRIDE: usize = 256;
const LIQUID_PASSES: usize = 8;

/// The liquid passes' textures and bind group, their uniform buffer, and the phases that move the surfaces: the water's
/// flow (`DAT_10150174`, down by the frame's time over a hundred, wrapped), the ripples (a step a frame: 0.003 for the
/// water, 0.03 for the toxic liquid and the lava, back to 0 past 1000) and the lava texture's drifts (the frame's time
/// over 10 and 20, over its tilings of 75 and 250, wrapped).
struct LiquidResources {
    layout: wgpu::BindGroupLayout,
    /// The passes' bind group, and the same with the screen copy in place of the white texture for the refraction pass (a
    /// pass that draws into the copy cannot also sample it).
    bind: wgpu::BindGroup,
    bind_screen: wgpu::BindGroup,
    buffer: wgpu::Buffer,
    flow: f32,
    ripples: [f32; 3],
    drift: [f32; 2],
    last: Option<std::time::Instant>,
    /// The water, toxic and lava textures (the white one until they are given), the samplers (repeating for them, clamped
    /// and unfiltered for the screen copy) and the blit's layout, to make the bind groups again.
    textures: [wgpu::TextureView; 3],
    white: wgpu::TextureView,
    sampler: wgpu::Sampler,
    clamp: wgpu::Sampler,
    sprite_layout: wgpu::BindGroupLayout,
    /// The picture so far, copied for the refraction pass (the original's `VisualOffscreen2Refraction_t` drawing render
    /// target 1 into target 2): its size, its view and its bind group for the blit back.
    screen: Option<((u32, u32), wgpu::TextureView, wgpu::BindGroup)>,
}

impl LiquidResources {
    fn new(gpu: &Gpu, layout: wgpu::BindGroupLayout, white: &wgpu::TextureView, sampler: &wgpu::Sampler, clamp: &wgpu::Sampler, sprite_layout: &wgpu::BindGroupLayout) -> LiquidResources {
        let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor { label: Some("liquid passes"), size: (LIQUID_PASS_STRIDE * LIQUID_PASSES) as u64, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let bind = Self::bind_group(gpu, &layout, &buffer, [white, white, white], sampler, white, clamp);
        let bind_screen = bind.clone();
        LiquidResources { layout, bind, bind_screen, buffer, flow: 0.0, ripples: [0.0; 3], drift: [0.0; 2], last: None, textures: [white.clone(), white.clone(), white.clone()], white: white.clone(), sampler: sampler.clone(), clamp: clamp.clone(), sprite_layout: sprite_layout.clone(), screen: None }
    }

    fn bind_group(gpu: &Gpu, layout: &wgpu::BindGroupLayout, buffer: &wgpu::Buffer, textures: [&wgpu::TextureView; 3], sampler: &wgpu::Sampler, screen: &wgpu::TextureView, clamp: &wgpu::Sampler) -> wgpu::BindGroup {
        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("liquid"),
            layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer, offset: 0, size: std::num::NonZeroU64::new(std::mem::size_of::<LiquidUniform>() as u64) }) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(textures[0]) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(textures[1]) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(textures[2]) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(sampler) },
                wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::TextureView(screen) },
                wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::Sampler(clamp) },
            ],
        })
    }

    /// The bind group made again from the textures and the screen copy there are.
    fn rebind(&mut self, gpu: &Gpu) {
        let screen = self.screen.as_ref().map_or(&self.white, |(_, v, _)| v);
        self.bind = Self::bind_group(gpu, &self.layout, &self.buffer, [&self.textures[0], &self.textures[1], &self.textures[2]], &self.sampler, &self.white, &self.clamp);
        self.bind_screen = Self::bind_group(gpu, &self.layout, &self.buffer, [&self.textures[0], &self.textures[1], &self.textures[2]], &self.sampler, screen, &self.clamp);
    }

    /// A screen copy of this size and format, made when there is none or the size changed (the bind groups follow).
    fn ensure_screen(&mut self, gpu: &Gpu, size: (u32, u32), format: wgpu::TextureFormat) {
        if self.screen.as_ref().is_some_and(|(s, _, _)| *s == size) {
            return;
        }
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("screen copy"),
            size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let blit = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("screen copy"),
            layout: &self.sprite_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) }, wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.clamp) }],
        });
        self.screen = Some((size, view, blit));
        self.rebind(gpu);
    }

    /// The phases moved on by a frame and every pass's uniform written for it; `screen` is the picture's size and the camera's
    /// tilt (see `LiquidUniform::screen`).
    fn update(&mut self, gpu: &Gpu, light: WaterLight, ambient: [f32; 3], screen: [f32; 4]) {
        let now = std::time::Instant::now();
        let dt = self.last.map_or(0.01, |t| now.duration_since(t).as_secs_f32().min(0.1));
        self.last = Some(now);
        self.flow -= dt * 0.01;
        while self.flow < 0.0 {
            self.flow += 1.0;
        }
        for (phase, step) in self.ripples.iter_mut().zip([0.003f32, 0.03, 0.03]) {
            *phase += step;
            if *phase > 1000.0 {
                *phase = 0.0;
            }
        }
        self.drift[0] = (self.drift[0] + dt / 10.0 / 75.0).rem_euclid(1.0);
        self.drift[1] = (self.drift[1] + dt / 20.0 / 250.0).rem_euclid(1.0);
        // (the water light's brightest channel lights the surfaces as grey, `set_light(brightest)`; its colour times the
        // specular intensity is the glint)
        let grey = light.colour.iter().cloned().fold(0.0f32, f32::max);
        let specular = light.colour.map(|c| c * light.specular);
        let mut data = vec![0u8; LIQUID_PASS_STRIDE * LIQUID_PASSES];
        for pass in 0..LIQUID_PASSES {
            let u = LiquidUniform {
                light: [grey, grey, grey, 1.0],
                specular: [specular[0], specular[1], specular[2], 0.0],
                to_light: [light.to_light.x, light.to_light.y, light.to_light.z, 0.0],
                ambient: [ambient[0], ambient[1], ambient[2], 0.0],
                phases: [self.flow, self.ripples[0], self.ripples[1], self.ripples[2]],
                drift: [self.drift[0], self.drift[1], 0.0, 0.0],
                which: [pass as u32, 0, 0, 0],
                screen,
            };
            let at = pass * LIQUID_PASS_STRIDE;
            data[at..at + std::mem::size_of::<LiquidUniform>()].copy_from_slice(bytemuck::bytes_of(&u));
        }
        gpu.queue.write_buffer(&self.buffer, 0, &data);
    }
}

/// A shell: a triangle list textured with a sprite sheet ([`Renderer::add_sprite`]), added to the scene or blended over it.
#[derive(Clone, Debug, PartialEq)]
pub struct Shell {
    pub sprite: u16,
    pub additive: bool,
    pub vertices: Vec<ShellVertex>,
}

/// The direction to the sun (world, Z up, unit): the light the scene and its shadow map use.
pub fn sun() -> Vec3 {
    Vec3::new(0.4, -0.5, 0.8).normalize()
}

impl Particle {
    /// A soft round dot.
    pub fn dot(position: [f32; 3], size: f32, colour: [f32; 4]) -> Particle {
        Particle { position, size, colour, sprite: NO_SPRITE, rect: [0.0; 4], right: [0.0; 3], up: [0.0; 3] }
    }

    /// A quad laid in the world: its centre, its half extents, its colour, the sheet and the part of it, and whether it fades by
    /// its facing.
    pub fn quad(position: [f32; 3], right: [f32; 3], up: [f32; 3], colour: [f32; 4], sprite: u16, rect: [f32; 4], facing: bool) -> Particle {
        Particle { position, size: if facing { 1.0 } else { 0.0 }, colour, sprite, rect, right, up }
    }
}

impl Scene {
    /// Replaces the overlay with the parts of `other` (which keeps this
    /// scene's bounds).
    pub fn set_overlay(&mut self, other: Scene) {
        self.overlay = other.parts;
    }

    /// Replaces the followers: parts drawn like the overlay but movable every frame with [`Scene::move_follow`]
    /// (the character between two network updates).
    pub fn set_follow(&mut self, other: Scene) {
        self.follow = other.parts;
    }

    /// Moves the followers by `delta` (world, Z up) from where they were uploaded.
    pub fn move_follow(&mut self, _gpu: &Gpu, delta: Vec3) {
        // (the frame that draws them makes their records from the shift: `record_of`)
        for part in &mut self.follow {
            part.shift = delta;
        }
    }

    /// Tints the characters (the original's `_GfxControlHighlight_t` on their meshes): each overlay part whose model part lies in
    /// a range gets that tint (rgb an emissive colour, a the alpha), the followers (the player's own character) `follow`'s, and
    /// every other overlay or follower part none. Only the tints that changed are written.
    pub fn set_tints(&mut self, gpu: &Gpu, overlay: &[(std::ops::Range<usize>, [f32; 4])], follow: Option<[f32; 4]>) {
        for part in &mut self.overlay {
            let tint = overlay.iter().find(|(range, _)| range.contains(&part.source)).map_or(NO_TINT, |(_, t)| *t);
            Scene::write_tint(gpu, part, tint);
        }
        for part in &mut self.follow {
            Scene::write_tint(gpu, part, follow.unwrap_or(NO_TINT));
        }
    }

    /// Tints every part of the scene (a headless picture of one body under a highlight).
    pub fn tint_all(&mut self, gpu: &Gpu, tint: [f32; 4]) {
        for part in &mut self.parts {
            Scene::write_tint(gpu, part, tint);
        }
    }

    fn write_tint(_gpu: &Gpu, part: &mut GpuPart, tint: [f32; 4]) {
        part.uniform.tint = tint;
    }
}

/// An orbiting camera looking at a point, Z up.
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub target: Vec3,
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
    /// Half the view's height at unit distance, when it is not the original's rule (90 degrees across the window, so the
    /// height follows the window's shape): the login's hall keeps its height in a wide window (`hall.rs` of the client).
    pub half_height: Option<f32>,
}

impl Camera {
    /// A camera that sees the whole scene.
    pub fn fit(scene: &Scene) -> Camera {
        Camera { target: scene.center, distance: scene.radius * 2.0, yaw: 0.6, pitch: 0.35, half_height: None }
    }

    /// A camera at `eye` looking at `target` (world, Z up).
    pub fn looking(eye: Vec3, target: Vec3) -> Camera {
        let d = eye - target;
        let distance = d.length().max(0.01);
        let pitch = (d.z / distance).clamp(-1.0, 1.0).asin();
        let yaw = if d.x.abs() + d.y.abs() > 1e-9 { d.y.atan2(d.x) } else { 0.0 };
        Camera { target, distance, yaw, pitch, half_height: None }
    }

    /// Where the camera is.
    pub fn eye(&self) -> Vec3 {
        let direction = Vec3::new(self.yaw.cos() * self.pitch.cos(), self.yaw.sin() * self.pitch.cos(), self.pitch.sin());
        self.target + direction * self.distance
    }

    /// Where a world point lands on the screen, as (x, y) in 0..1 from the top
    /// left, or none when it is behind the camera or off the view.
    pub fn project(&self, point: Vec3, aspect: f32, radius: f32) -> Option<[f32; 2]> {
        let clip = self.view_proj(aspect, radius) * point.extend(1.0);
        if clip.w <= 0.0 {
            return None;
        }
        let (x, y) = (clip.x / clip.w, clip.y / clip.w);
        (x.abs() <= 1.2 && y.abs() <= 1.2).then(|| [x * 0.5 + 0.5, 0.5 - y * 0.5])
    }

    /// The line from the eye through a point of the screen ((x, y) in 0..1 from the top left): its start (on the near
    /// plane) and unit direction.
    pub fn ray(&self, x: f32, y: f32, aspect: f32, radius: f32) -> (Vec3, Vec3) {
        let inverse = self.view_proj(aspect, radius).inverse();
        let (nx, ny) = (x * 2.0 - 1.0, 1.0 - y * 2.0);
        let near = inverse * glam::Vec4::new(nx, ny, 0.0, 1.0);
        let far = inverse * glam::Vec4::new(nx, ny, 1.0, 1.0);
        let (near, far) = (near.truncate() / near.w, far.truncate() / far.w);
        (near, (far - near).normalize_or_zero())
    }

    fn view_proj(&self, aspect: f32, radius: f32) -> Mat4 {
        let eye = self.eye();
        let far = (self.distance + radius * 4.0).max(1.0);
        // (Not a fixed share of the far plane: on a big playfield that cut the ground within 8 units of the camera away.)
        let near = (far * 0.001).clamp(0.01, 0.2);
        // The original's view is 90° across (`VisualCamera_t::SetViewPlaneWindow(1.5708, aspect)`: the window's half width is
        // tan(45°) at unit distance, its height that over the aspect), so the vertical field follows the window's shape.
        let fov_y = 2.0 * self.half_height.unwrap_or(1.0 / aspect.max(0.1)).atan();
        perspective(fov_y, aspect, near, far) * look_at(eye, self.target, Vec3::Z)
    }
}

/// A right-handed view matrix.
fn look_at(eye: Vec3, center: Vec3, up: Vec3) -> Mat4 {
    let f = (center - eye).normalize();
    let s = f.cross(up).normalize();
    let u = s.cross(f);
    Mat4::from_cols_array(&[s.x, u.x, -f.x, 0.0, s.y, u.y, -f.y, 0.0, s.z, u.z, -f.z, 0.0, -s.dot(eye), -u.dot(eye), f.dot(eye), 1.0])
}

/// A right-handed orthographic projection of a square `half` wide to depth 0..1.
fn orthographic(half: f32, near: f32, far: f32) -> Mat4 {
    Mat4::from_cols_array(&[1.0 / half, 0.0, 0.0, 0.0, 0.0, 1.0 / half, 0.0, 0.0, 0.0, 0.0, 1.0 / (near - far), 0.0, 0.0, 0.0, near / (near - far), 1.0])
}

/// A right-handed perspective projection to depth 0..1.
fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    let f = 1.0 / (fov_y * 0.5).tan();
    Mat4::from_cols_array(&[f / aspect, 0.0, 0.0, 0.0, 0.0, f, 0.0, 0.0, 0.0, 0.0, far / (near - far), -1.0, 0.0, 0.0, near * far / (near - far), 0.0])
}

struct FsrTargets {
    input: (u32, u32),
    output: (u32, u32),
    scene: wgpu::TextureView,
    mid: wgpu::TextureView,
}

/// What a frame drew: the parts and draw calls in the camera's pass and the shadow pass.
#[derive(Clone, Copy, Debug, Default)]
pub struct Stats {
    pub parts: usize,
    pub drawn: usize,
    pub shadow_drawn: usize,
    /// The draw calls the scene pass's parts took (parts of the same geometry and texture go in one, as instances).
    /// The shadow map's are not counted: it is drawn only now and then.
    pub calls: usize,
    /// Where `draw`'s time went (for the client's profile): choosing the levels of detail and the frame's buffers, recording
    /// the shadow pass, the scene pass and the rest, and handing the work to the queue.
    pub times: [(&'static str, std::time::Duration); 5],
}

/// The six planes (normal xyz, distance w; inside is positive) of a
/// view-projection matrix to depth 0..1.
fn frustum(m: &Mat4) -> [glam::Vec4; 6] {
    let row = |i: usize| glam::Vec4::new(m.x_axis[i], m.y_axis[i], m.z_axis[i], m.w_axis[i]);
    let (r0, r1, r2, r3) = (row(0), row(1), row(2), row(3));
    [r3 + r0, r3 - r0, r3 + r1, r3 - r1, r2, r3 - r2]
}

/// Whether a sphere is at least partly inside the planes.
fn sphere_visible(planes: &[glam::Vec4; 6], center: Vec3, radius: f32) -> bool {
    planes.iter().all(|p| {
        let n = p.truncate();
        n.dot(center) + p.w >= -radius * n.length()
    })
}

/// The shadow map's side in texels unless another is chosen ([`Shadows`]).
const SHADOW_SIZE: u32 = 4096;

/// How the sun's shadows are drawn: not at all, or from a shadow map of a side of 1024, 2048 or 4096 texels over the
/// same ground (a smaller map is quicker to draw and to read, and its shadows' edges are coarser).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Shadows {
    Off,
    Low,
    Medium,
    #[default]
    High,
}

impl Shadows {
    pub const ALL: [Shadows; 4] = [Shadows::Off, Shadows::Low, Shadows::Medium, Shadows::High];

    pub fn name(self) -> &'static str {
        match self {
            Shadows::Off => "off",
            Shadows::Low => "low",
            Shadows::Medium => "medium",
            Shadows::High => "high",
        }
    }

    pub fn from_name(name: &str) -> Option<Shadows> {
        Shadows::ALL.into_iter().find(|s| s.name().eq_ignore_ascii_case(name.trim()))
    }

    /// The shadow map's side (the smallest for none: the map is still there, cleared, and reads "lit").
    pub fn size(self) -> u32 {
        match self {
            Shadows::Off | Shadows::Low => 1024,
            Shadows::Medium => 2048,
            Shadows::High => SHADOW_SIZE,
        }
    }
}

/// The shadow map last drawn, and what it was drawn for (see `Renderer::draw`).
#[derive(Clone, Copy)]
struct ShadowCache {
    /// The scene's parts (their list's address and length) and whether shadows were on.
    scene: (usize, usize),
    shadows: bool,
    focus: Vec3,
    sun: Vec3,
    extent: f32,
    light_vp: Mat4,
    texel: f32,
}

/// The shaders and layouts the scene pass's pipelines are made from (kept to make another set: see [`ScenePipelines`]).
struct Shaders {
    mesh: wgpu::ShaderModule,
    particle: wgpu::ShaderModule,
    liquid: wgpu::ShaderModule,
    mesh_layout: wgpu::PipelineLayout,
    /// The globals alone (the sky, the water and the blob shadows).
    sky_layout: wgpu::PipelineLayout,
    particle_layout: wgpu::PipelineLayout,
    /// The globals and the liquid's textures and pass uniform.
    liquid_layout: wgpu::PipelineLayout,
    /// The screen copy's blit: its shader and its layout (a sprite's texture and sampler).
    blit: wgpu::ShaderModule,
    blit_layout: wgpu::PipelineLayout,
}

/// The format of the motion buffer (see `taa.wgsl`).
const MOTION: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
/// The format of the anti-aliased frames kept from one frame to the next.
const HISTORY: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// The pipelines of the scene pass for one way of drawing it: its samples per pixel, and whether it writes the motion buffer
/// as a second target (temporal anti-aliasing: the meshes write their motion, the rest leave it as it is).
struct ScenePipelines {
    /// The meshes' pipeline, three times: both sides drawn, and each of the two ways round culled ([`faces_of`]).
    mesh: [wgpu::RenderPipeline; 3],
    /// The mesh pipeline blending by the fragment's alpha: the tinted character parts under full alpha.
    mesh_blend: [wgpu::RenderPipeline; 3],
    /// The blended materials' other three: without depth writes, added, and added without depth writes.
    mesh_blend_no_z: [wgpu::RenderPipeline; 3],
    mesh_add: [wgpu::RenderPipeline; 3],
    mesh_add_no_z: [wgpu::RenderPipeline; 3],
    sky: wgpu::RenderPipeline,
    /// The screen copy drawn back over the whole picture (no depth), before the refraction pass continues on it.
    blit: wgpu::RenderPipeline,
    water: wgpu::RenderPipeline,
    /// The liquid surfaces: blended by their alpha, and the ripple passes blended the other way round (`1 - a` of the
    /// pass over `a` of what is there).
    liquid: wgpu::RenderPipeline,
    liquid_inverse: wgpu::RenderPipeline,
    blob: wgpu::RenderPipeline,
    particle: wgpu::RenderPipeline,
    /// The shells, added and blended.
    shell_add: wgpu::RenderPipeline,
    shell_blend: wgpu::RenderPipeline,
    backdrops: [wgpu::RenderPipeline; 3],
}

impl ScenePipelines {
    fn new(device: &wgpu::Device, shaders: &Shaders, format: wgpu::TextureFormat, samples: u32, motion: bool, culling: Culling) -> ScenePipelines {
        let multisample = wgpu::MultisampleState { count: samples, mask: !0, alpha_to_coverage_enabled: false };
        // (the colour target, and the motion buffer written or left alone)
        let targets = |blend: Option<wgpu::BlendState>, writes_motion: bool| -> Vec<Option<wgpu::ColorTargetState>> {
            let mut t = vec![Some(wgpu::ColorTargetState { format, blend, write_mask: wgpu::ColorWrites::ALL })];
            if motion {
                t.push(Some(wgpu::ColorTargetState { format: MOTION, blend: None, write_mask: if writes_motion { wgpu::ColorWrites::ALL } else { wgpu::ColorWrites::empty() } }));
            }
            t
        };
        let depth = |write: bool, compare: wgpu::CompareFunction, bias: wgpu::DepthBiasState| Some(wgpu::DepthStencilState { format: DEPTH, depth_write_enabled: Some(write), depth_compare: Some(compare), stencil: Default::default(), bias });
        let shader = &shaders.mesh;
        let attributes = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Unorm8x4];
        let mesh_targets = targets(None, true);
        // (the faces left out, by `faces_of`'s index; to show them instead, both sides are drawn and the fragment is told
        // which side it is on by the front face's sense)
        let other = match FRONT {
            wgpu::FrontFace::Cw => wgpu::FrontFace::Ccw,
            wgpu::FrontFace::Ccw => wgpu::FrontFace::Cw,
        };
        let primitive = |faces: usize| -> wgpu::PrimitiveState {
            let (cull_mode, front_face) = match (culling, faces) {
                (Culling::Off, _) | (_, 0) => (None, FRONT),
                (Culling::On, 1) => (Some(wgpu::Face::Back), FRONT),
                (Culling::On, _) => (Some(wgpu::Face::Front), FRONT),
                (Culling::Show, 1) => (None, FRONT),
                (Culling::Show, _) => (None, other),
            };
            wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleList, cull_mode, front_face, ..Default::default() }
        };
        let fragment = |faces: usize| -> &'static str {
            match (culling == Culling::Show && faces != 0, motion) {
                (false, false) => "fs_main",
                (false, true) => "fs_main_motion",
                (true, false) => "fs_main_show",
                (true, true) => "fs_main_motion_show",
            }
        };
        let mesh = [0, 1, 2].map(|faces| device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mesh"),
            layout: Some(&shaders.mesh_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<GpuVertex>() as u64, step_mode: wgpu::VertexStepMode::Vertex, attributes: &attributes })],
            },
            primitive: primitive(faces),
            depth_stencil: depth(true, wgpu::CompareFunction::Less, Default::default()),
            multisample,
            fragment: Some(wgpu::FragmentState { module: shader, entry_point: Some(fragment(faces)), compilation_options: Default::default(), targets: &mesh_targets }),
            multiview_mask: None,
            cache: None,
        }));
        // The same for a translucent part (a highlighted character, a blended material): blended by its alpha, writing
        // depth as Direct3D's `SetTransparency` did and as a material does unless its states turn the writes off; and
        // added (source alpha, one) for the materials whose `DESTBLEND` is one.
        let blended = |label: &str, state: wgpu::BlendState, z: bool| {
            let blend_targets = targets(Some(state), true);
            [0, 1, 2].map(|faces| device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&shaders.mesh_layout),
                vertex: wgpu::VertexState {
                    module: shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<GpuVertex>() as u64, step_mode: wgpu::VertexStepMode::Vertex, attributes: &attributes })],
                },
                primitive: primitive(faces),
                depth_stencil: depth(z, wgpu::CompareFunction::Less, Default::default()),
                multisample,
                fragment: Some(wgpu::FragmentState { module: shader, entry_point: Some(fragment(faces)), compilation_options: Default::default(), targets: &blend_targets }),
                multiview_mask: None,
                cache: None,
            }))
        };
        let added = wgpu::BlendState {
            color: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::SrcAlpha, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add },
            alpha: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::Zero, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add },
        };
        let mesh_blend = blended("mesh blend", wgpu::BlendState::ALPHA_BLENDING, true);
        let mesh_blend_no_z = blended("mesh blend, no depth", wgpu::BlendState::ALPHA_BLENDING, false);
        let mesh_add = blended("mesh add", added, true);
        let mesh_add_no_z = blended("mesh add, no depth", added, false);
        // The sky: one triangle behind everything, no depth.
        let opaque = targets(None, false);
        let sky = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sky"),
            layout: Some(&shaders.sky_layout),
            vertex: wgpu::VertexState { module: shader, entry_point: Some("vs_sky"), compilation_options: Default::default(), buffers: &[] },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: depth(false, wgpu::CompareFunction::Always, Default::default()),
            multisample,
            fragment: Some(wgpu::FragmentState { module: shader, entry_point: Some("fs_sky"), compilation_options: Default::default(), targets: &opaque }),
            multiview_mask: None,
            cache: None,
        });
        // The screen copy blitted back: one triangle over the picture, no depth.
        let blit = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("blit"),
            layout: Some(&shaders.blit_layout),
            vertex: wgpu::VertexState { module: &shaders.blit, entry_point: Some("vs_blit"), compilation_options: Default::default(), buffers: &[] },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: depth(false, wgpu::CompareFunction::Always, Default::default()),
            multisample,
            fragment: Some(wgpu::FragmentState { module: &shaders.blit, entry_point: Some("fs_blit"), compilation_options: Default::default(), targets: &opaque }),
            multiview_mask: None,
            cache: None,
        });
        // The water: a translucent sheet after everything else, no depth writes.
        let blended = targets(Some(wgpu::BlendState::ALPHA_BLENDING), false);
        let water = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("water"),
            layout: Some(&shaders.sky_layout),
            vertex: wgpu::VertexState { module: shader, entry_point: Some("vs_water"), compilation_options: Default::default(), buffers: &[] },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: depth(false, wgpu::CompareFunction::Less, Default::default()),
            multisample,
            fragment: Some(wgpu::FragmentState { module: shader, entry_point: Some("fs_water"), compilation_options: Default::default(), targets: &blended }),
            multiview_mask: None,
            cache: None,
        });
        // The liquid surfaces (`liquid.wgsl`): position, normal and colour per vertex; after the meshes, no depth writes.
        let liquid_attributes = [
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 12, shader_location: 1 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Unorm8x4, offset: 24, shader_location: 2 },
        ];
        let liquid_like = |label: &str, blend: wgpu::BlendState| {
            let liquid_targets = targets(Some(blend), false);
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&shaders.liquid_layout),
                vertex: wgpu::VertexState {
                    module: &shaders.liquid,
                    entry_point: Some("vs_liquid"),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<LiquidVertex>() as u64, step_mode: wgpu::VertexStepMode::Vertex, attributes: &liquid_attributes })],
                },
                primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
                depth_stencil: depth(false, wgpu::CompareFunction::Less, Default::default()),
                multisample,
                fragment: Some(wgpu::FragmentState { module: &shaders.liquid, entry_point: Some("fs_liquid"), compilation_options: Default::default(), targets: &liquid_targets }),
                multiview_mask: None,
                cache: None,
            })
        };
        let liquid = liquid_like("liquid", wgpu::BlendState::ALPHA_BLENDING);
        let inverse = wgpu::BlendComponent { src_factor: wgpu::BlendFactor::OneMinusSrcAlpha, dst_factor: wgpu::BlendFactor::SrcAlpha, operation: wgpu::BlendOperation::Add };
        let liquid_inverse = liquid_like("liquid inverse", wgpu::BlendState { color: inverse, alpha: inverse });
        // The blob shadows: coloured triangles blended over the ground, pulled a little toward the camera so they sit on it.
        let blob_attributes = [wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 }, wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 12, shader_location: 1 }];
        let blob = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("blob"),
            layout: Some(&shaders.sky_layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vs_blob"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<BlobVertex>() as u64, step_mode: wgpu::VertexStepMode::Vertex, attributes: &blob_attributes })],
            },
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            depth_stencil: depth(false, wgpu::CompareFunction::LessEqual, wgpu::DepthBiasState { constant: -8, slope_scale: -2.0, clamp: 0.0 }),
            multisample,
            fragment: Some(wgpu::FragmentState { module: shader, entry_point: Some("fs_blob"), compilation_options: Default::default(), targets: &blended }),
            multiview_mask: None,
            cache: None,
        });
        // Sparks and glows: camera-facing sprites added to the scene, tested against its depth and not writing any.
        let particle_shader = &shaders.particle;
        let particle_attributes = [
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 0, shader_location: 0 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 16, shader_location: 1 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 32, shader_location: 2 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 48, shader_location: 3 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 64, shader_location: 4 },
        ];
        let additive = wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add };
        let added = targets(Some(wgpu::BlendState { color: additive, alpha: additive }), false);
        let particle = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("particle"),
            layout: Some(&shaders.particle_layout),
            vertex: wgpu::VertexState {
                module: particle_shader,
                entry_point: Some("vs_particle"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout { array_stride: 80, step_mode: wgpu::VertexStepMode::Instance, attributes: &particle_attributes })],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: depth(false, wgpu::CompareFunction::Less, Default::default()),
            multisample,
            fragment: Some(wgpu::FragmentState { module: particle_shader, entry_point: Some("fs_particle"), compilation_options: Default::default(), targets: &added }),
            multiview_mask: None,
            cache: None,
        });
        let shell_attributes = [
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 12, shader_location: 1 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 20, shader_location: 2 },
        ];
        let shell_like = |vertex: &str, entry: &str, blend: Option<wgpu::BlendState>, compare: wgpu::CompareFunction| {
            let t = targets(blend, false);
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("shell"),
                layout: Some(&shaders.particle_layout),
                vertex: wgpu::VertexState {
                    module: particle_shader,
                    entry_point: Some(vertex),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout { array_stride: 36, step_mode: wgpu::VertexStepMode::Vertex, attributes: &shell_attributes })],
                },
                primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
                depth_stencil: depth(false, compare, Default::default()),
                multisample,
                fragment: Some(wgpu::FragmentState { module: particle_shader, entry_point: Some(entry), compilation_options: Default::default(), targets: &t }),
                multiview_mask: None,
                cache: None,
            })
        };
        let shell_add = shell_like("vs_shell", "fs_shell_add", Some(wgpu::BlendState { color: additive, alpha: additive }), wgpu::CompareFunction::Less);
        let shell_blend = shell_like("vs_shell", "fs_shell_blend", Some(wgpu::BlendState::ALPHA_BLENDING), wgpu::CompareFunction::Less);
        let backdrops = [
            shell_like("vs_backdrop", "fs_backdrop_opaque", None, wgpu::CompareFunction::Always),
            shell_like("vs_backdrop", "fs_shell_blend", Some(wgpu::BlendState::ALPHA_BLENDING), wgpu::CompareFunction::Always),
            shell_like("vs_backdrop", "fs_shell_add", Some(wgpu::BlendState { color: additive, alpha: additive }), wgpu::CompareFunction::Always),
        ];
        ScenePipelines { mesh, mesh_blend, mesh_blend_no_z, mesh_add, mesh_add_no_z, sky, blit, water, liquid, liquid_inverse, blob, particle, shell_add, shell_blend, backdrops }
    }
}

/// How the scene's edges are smoothed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Antialiasing {
    /// None: one sample per pixel.
    Off,
    /// 4x multisampling (where the device does it).
    #[default]
    Msaa,
    /// Temporal: one sample per pixel, jittered from frame to frame and blended with the frames before (see `taa.wgsl`).
    Taa,
}

impl Antialiasing {
    pub fn name(self) -> &'static str {
        match self {
            Antialiasing::Off => "off",
            Antialiasing::Msaa => "msaa",
            Antialiasing::Taa => "taa",
        }
    }

    pub fn from_name(name: &str) -> Option<Antialiasing> {
        [Antialiasing::Off, Antialiasing::Msaa, Antialiasing::Taa].into_iter().find(|a| a.name().eq_ignore_ascii_case(name.trim()))
    }
}

/// The temporal anti-aliasing's targets at the size the scene is drawn: the scene, its motion, its depth (read afterwards)
/// and the two anti-aliased frames, the last one and the one being made.
struct TaaTargets {
    size: (u32, u32),
    colour: wgpu::TextureView,
    motion: wgpu::TextureView,
    depth: wgpu::TextureView,
    history: [wgpu::TextureView; 2],
    /// Which history the next frame writes.
    next: usize,
    /// Whether the other one holds a frame of the same scene to blend with.
    valid: bool,
    scene: (usize, usize),
}

/// The frame's time on the graphics card: a timestamp at the start and the end of `Renderer::draw`'s commands, read back a few
/// frames later through a ring of buffers (so the frame never waits for the card).
struct GpuTimer {
    queries: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    /// Each read-back buffer and its state: 0 free, 1 being mapped, 2 mapped (ready to read).
    readbacks: Vec<(wgpu::Buffer, std::sync::Arc<std::sync::atomic::AtomicU8>)>,
    next: usize,
    /// Nanoseconds per tick.
    period: f32,
}

impl GpuTimer {
    fn new(gpu: &Gpu) -> Option<GpuTimer> {
        let needed = wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS;
        if !gpu.device.features().contains(needed) {
            return None;
        }
        let queries = gpu.device.create_query_set(&wgpu::QuerySetDescriptor { label: Some("frame time"), ty: wgpu::QueryType::Timestamp, count: 2 });
        let resolve = gpu.device.create_buffer(&wgpu::BufferDescriptor { label: Some("frame time"), size: 16, usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false });
        let readbacks = (0..4)
            .map(|_| (gpu.device.create_buffer(&wgpu::BufferDescriptor { label: Some("frame time"), size: 16, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false }), Default::default()))
            .collect();
        Some(GpuTimer { queries, resolve, readbacks, next: 0, period: gpu.queue.get_timestamp_period(), })
    }

    /// The latest time read back (milliseconds), freeing the buffers that hold one.
    fn collect(&mut self) -> Option<f32> {
        use std::sync::atomic::Ordering;
        let mut latest = None;
        for _ in 0..self.readbacks.len() {
            // (oldest first, from the slot after the one written last)
            let k = self.next;
            let (buffer, state) = &self.readbacks[k];
            if state.load(Ordering::Acquire) == 2 {
                if let Ok(view) = buffer.slice(..).get_mapped_range() {
                    let t: &[u64] = bytemuck::cast_slice(&view[..16]);
                    if t[1] > t[0] {
                        latest = Some((t[1] - t[0]) as f32 * self.period / 1.0e6);
                    }
                }
                buffer.unmap();
                state.store(0, Ordering::Release);
            }
            self.next = (self.next + 1) % self.readbacks.len();
        }
        latest
    }

    /// The slot this frame writes into, if one is free.
    fn slot(&self) -> Option<usize> {
        (0..self.readbacks.len()).map(|i| (self.next + i) % self.readbacks.len()).find(|&k| self.readbacks[k].1.load(std::sync::atomic::Ordering::Acquire) == 0)
    }
}

/// The textures' mip bias for a scene drawn at `scale` of the window's size and upscaled: the scale's logarithm to
/// base 2 (0 at the full size, -1 at a half), so that a texture is read at the level the window's pixels call for and not
/// the smaller picture's, which is a level blurrier at a half. It is the figure FSR 1 asks for with its steps (-0.38,
/// -0.58, -0.79, -1).
pub fn mip_bias(scale: f32) -> f32 {
    scale.clamp(0.4, 1.0).log2()
}

/// The `index`th number (from 1) of the Halton sequence in `base`: the jitter's points, spread evenly over a pixel.
fn halton(mut index: u32, base: u32) -> f32 {
    let (mut f, mut r) = (1.0f32, 0.0f32);
    while index > 0 {
        f /= base as f32;
        r += f * (index % base) as f32;
        index /= base;
    }
    r
}

pub struct Renderer {
    shadow_cache: Option<ShadowCache>,
    shadow_pipeline: wgpu::RenderPipeline,
    /// The scene pass's pipelines by (samples per pixel, motion buffer), made as they are first wanted.
    pipeline_sets: HashMap<(u32, bool), ScenePipelines>,
    shaders: Shaders,
    /// How the edges are smoothed (multisampling by default).
    pub antialiasing: Antialiasing,
    /// Whether the faces turned away are left out (set when the renderer is made: `MAO_CULL`).
    culling: Culling,
    taa_pipeline: wgpu::RenderPipeline,
    taa_layout: wgpu::BindGroupLayout,
    taa_sampler: wgpu::Sampler,
    taa: Option<TaaTargets>,
    /// A copy of a frame onto the target (the anti-aliased frame at full size).
    copy_pipeline: wgpu::RenderPipeline,
    /// The frame's time on the card (where the device can measure it).
    timer: Option<GpuTimer>,
    /// The last frame time the card reported (milliseconds; none until one is read back or where it can't be measured).
    pub gpu_ms: Option<f32>,
    /// Frames drawn (the jitter's place in its sequence), and the last frame's unjittered view and projection and player.
    frame: u32,
    previous: Option<(Mat4, [f32; 4])>,
    /// The sprite sheets uploaded so far, by number, as the bind group the particle pass uses, and the plain white one dots use.
    sprites: HashMap<u16, wgpu::BindGroup>,
    sprite_layout: wgpu::BindGroupLayout,
    white_sprite: wgpu::BindGroup,
    shadow_view: wgpu::TextureView,
    shadow_bind: wgpu::BindGroup,
    shadow_layout: wgpu::BindGroupLayout,
    shadow_sampler: wgpu::Sampler,
    shadow_size: u32,
    /// Draw the sun's shadows (on by default).
    pub shadows: bool,
    /// Whether the water's refraction pass is drawn (`MAO_NO_REFRACTION` turns it off): the picture so far mirrored into
    /// the water about the horizon, as the original's `VisualLiquid_t::RenderRefraction` draws it.
    pub refraction: bool,
    /// What the last frame drew.
    pub stats: Stats,
    /// The share of the window's size the scene is drawn at (1 = native); less is upscaled with FSR 1.
    pub render_scale: f32,
    /// Keep the textures on the card in compressed blocks, a quarter of their size ([`block_levels`]); on by default, and
    /// only for the textures uploaded from then on.
    pub compress_textures: bool,
    /// Read the textures sharper when the scene is drawn below the window's size ([`mip_bias`]); on unless
    /// `MAO_NO_MIP_BIAS` is set (to compare the two).
    pub mip_bias: bool,
    /// FSR's sharpening in stops (0 = maximum, each stop halves it).
    pub sharpness: f32,
    fsr_easu: wgpu::RenderPipeline,
    fsr_rcas: wgpu::RenderPipeline,
    fsr_layout: wgpu::BindGroupLayout,
    fsr_sampler: wgpu::Sampler,
    /// The scaled scene and the upscaled frame, for the sizes they were made for.
    fsr: Option<FsrTargets>,
    /// Samples per pixel (4 where the device supports it, else 1).
    samples: u32,
    msaa: Option<((u32, u32), wgpu::TextureView)>,
    started: std::time::Instant,
    globals_layout: wgpu::BindGroupLayout,
    part_layout: wgpu::BindGroupLayout,
    /// The textures' bind groups by (type, instance), none for the white one, kept between uploads.
    binds: std::sync::Mutex<HashMap<Option<(u32, u32)>, TextureBind>>,
    /// The frame's part records: the layout, and the buffer with its bind group and how many records it holds.
    records_layout: wgpu::BindGroupLayout,
    records: Option<(wgpu::Buffer, wgpu::BindGroup, usize)>,
    sampler: wgpu::Sampler,
    white: wgpu::TextureView,
    format: wgpu::TextureFormat,
    depth: Option<((u32, u32), u32, wgpu::TextureView)>,
    /// The textures uploaded so far, by (type, instance), kept between uploads.
    textures: std::cell::RefCell<HashMap<(u32, u32), Option<wgpu::TextureView>>>,
    /// The liquid passes' textures, uniform and phases.
    liquid: LiquidResources,
}

const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// What is drawn: each part with its final world matrix.
fn draws(model: &Model) -> Vec<(usize, [f32; 16])> {
    if model.instances.is_empty() {
        model.parts.iter().enumerate().map(|(i, p)| (i, p.world)).collect()
    } else {
        model.instances.iter().filter_map(|(i, m)| model.parts.get(*i).map(|p| (*i, mao_formats::model::multiply(&p.world, m)))).collect()
    }
}

fn world_matrix_bounds(model: &Model, draws: &[(usize, [f32; 16])]) -> (Vec3, f32) {
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for (i, world) in draws {
        let m = Mat4::from_cols_array(world);
        // The corners of the part's box suffice and cost far less than every vertex.
        let part = &model.parts[*i];
        let (mut plo, mut phi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for v in &part.vertices {
            let p = Vec3::from(v.position);
            plo = plo.min(p);
            phi = phi.max(p);
        }
        if plo.x > phi.x {
            continue;
        }
        for corner in 0..8 {
            let c = Vec3::new(if corner & 1 == 0 { plo.x } else { phi.x }, if corner & 2 == 0 { plo.y } else { phi.y }, if corner & 4 == 0 { plo.z } else { phi.z });
            let p = m.transform_point3(c);
            lo = lo.min(p);
            hi = hi.max(p);
        }
    }
    if lo.x > hi.x {
        return (Vec3::ZERO, 1.0);
    }
    ((lo + hi) * 0.5, ((hi - lo).length() * 0.5).max(1e-3))
}

impl Renderer {
    pub fn new(gpu: &Gpu, format: wgpu::TextureFormat) -> Renderer {
        let device = &gpu.device;
        // 4x multisampling where the device does it for both the colour and depth formats.
        let supports = |f: wgpu::TextureFormat| gpu.adapter.get_texture_format_features(f).flags.sample_count_supported(4);
        let samples = if supports(format) && supports(DEPTH) { 4 } else { 1 };
        // (every scene shader starts with the frame's globals)
        let with_globals = |body: &str| format!("{}\n{}", include_str!("globals.wgsl"), body);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("mesh"), source: wgpu::ShaderSource::Wgsl(with_globals(include_str!("mesh.wgsl")).into()) });
        let liquid_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("liquid"), source: wgpu::ShaderSource::Wgsl(with_globals(include_str!("liquid.wgsl")).into()) });
        let uniform = |binding, visibility| wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
            count: None,
        };
        let globals_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("globals"),
            entries: &[uniform(0, wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT)],
        });
        let part_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("part"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        // The frame's part records: one array the vertex and the fragment stages read by the instance's index.
        let records_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("part records"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
        });
        let shadow_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Depth, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison), count: None },
            ],
        });
        let shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow map"),
            size: wgpu::Extent3d { width: SHADOW_SIZE, height: SHADOW_SIZE, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let shadow_view = shadow_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let shadow_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow"),
            layout: &shadow_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&shadow_view) }, wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&shadow_sampler) }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("mesh"), bind_group_layouts: &[Some(&globals_layout), Some(&part_layout), Some(&records_layout), Some(&shadow_layout)], immediate_size: 0 });
        let depth_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("depth"), bind_group_layouts: &[Some(&globals_layout), Some(&part_layout), Some(&records_layout)], immediate_size: 0 });
        let attributes = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Unorm8x4];
        // The shadow pass: depth only, from the sun, plants bending as in the main pass.
        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow"),
            layout: Some(&depth_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<GpuVertex>() as u64, step_mode: wgpu::VertexStepMode::Vertex, attributes: &attributes })],
            },
            primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleList, cull_mode: None, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: wgpu::DepthBiasState { constant: 2, slope_scale: 2.0, clamp: 0.0 },
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs_shadow"), compilation_options: Default::default(), targets: &[] }),
            multiview_mask: None,
            cache: None,
        });
        let sky_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("sky"), bind_group_layouts: &[Some(&globals_layout)], immediate_size: 0 });
        let particle_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("particle"), source: wgpu::ShaderSource::Wgsl(include_str!("particle.wgsl").into()) });
        let sprite_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sprite"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        let particle_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("particle"), bind_group_layouts: &[Some(&globals_layout), Some(&sprite_layout)], immediate_size: 0 });
        // The liquid's passes: one uniform block per pass in a single buffer (read at a dynamic offset), the three textures.
        let texture_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
            count: None,
        };
        let liquid_bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("liquid"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: true, min_binding_size: None },
                    count: None,
                },
                texture_entry(1),
                texture_entry(2),
                texture_entry(3),
                wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
                // (the screen copy the refraction pass samples, clamped and unfiltered)
                texture_entry(5),
                wgpu::BindGroupLayoutEntry { binding: 6, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        let liquid_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("liquid"), bind_group_layouts: &[Some(&globals_layout), Some(&liquid_bind_layout)], immediate_size: 0 });
        // The blit of the screen copy back onto the picture (`blit.wgsl`: a texture and a sampler, the sprite layout).
        let blit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("blit"), source: wgpu::ShaderSource::Wgsl(include_str!("blit.wgsl").into()) });
        let blit_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("blit"), bind_group_layouts: &[Some(&sprite_layout)], immediate_size: 0 });
        let shaders = Shaders { mesh: shader, particle: particle_shader, liquid: liquid_shader, blit: blit_shader, mesh_layout: layout, sky_layout, particle_layout, liquid_layout, blit_layout };
        let mut pipeline_sets = HashMap::new();
        let culling = std::env::var("MAO_CULL").map(|v| Culling::from_name(&v)).unwrap_or_default();
        pipeline_sets.insert((samples, false), ScenePipelines::new(device, &shaders, format, samples, false, culling));
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            anisotropy_clamp: 16,
            ..Default::default()
        });
        // FSR 1: upscale the scaled scene (EASU), then sharpen (RCAS).
        let fsr_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("fsr"), source: wgpu::ShaderSource::Wgsl(include_str!("fsr.wgsl").into()) });
        let fsr_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fsr"),
            entries: &[
                uniform(0, wgpu::ShaderStages::FRAGMENT),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        let fsr_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("fsr"), bind_group_layouts: &[Some(&fsr_layout)], immediate_size: 0 });
        let fsr_pipeline = |entry: &'static str| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(entry),
                layout: Some(&fsr_pipeline_layout),
                vertex: wgpu::VertexState { module: &fsr_shader, entry_point: Some("vs_fsr"), compilation_options: Default::default(), buffers: &[] },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState { module: &fsr_shader, entry_point: Some(entry), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })] }),
                multiview_mask: None,
                cache: None,
            })
        };
        let (fsr_easu, fsr_rcas, copy_pipeline) = (fsr_pipeline("fs_easu"), fsr_pipeline("fs_rcas"), fsr_pipeline("fs_copy"));
        // The temporal anti-aliasing's blend (`taa.wgsl`).
        let taa_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("taa"), source: wgpu::ShaderSource::Wgsl(include_str!("taa.wgsl").into()) });
        let texture = |binding, sample_type| wgpu::BindGroupLayoutEntry { binding, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None };
        let float = wgpu::TextureSampleType::Float { filterable: true };
        let taa_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("taa"),
            entries: &[
                uniform(0, wgpu::ShaderStages::FRAGMENT),
                texture(1, float),
                texture(2, float),
                texture(3, wgpu::TextureSampleType::Depth),
                texture(4, float),
                wgpu::BindGroupLayoutEntry { binding: 5, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        let taa_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("taa"), bind_group_layouts: &[Some(&taa_layout)], immediate_size: 0 });
        let taa_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("taa"),
            layout: Some(&taa_pipeline_layout),
            vertex: wgpu::VertexState { module: &taa_shader, entry_point: Some("vs_taa"), compilation_options: Default::default(), buffers: &[] },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState { module: &taa_shader, entry_point: Some("fs_taa"), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format: HISTORY, blend: None, write_mask: wgpu::ColorWrites::ALL })] }),
            multiview_mask: None,
            cache: None,
        });
        let taa_sampler = device.create_sampler(&wgpu::SamplerDescriptor { mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, ..Default::default() });
        let fsr_sampler = device.create_sampler(&wgpu::SamplerDescriptor { mag_filter: wgpu::FilterMode::Nearest, min_filter: wgpu::FilterMode::Nearest, ..Default::default() });
        let white = upload_texture(gpu, &Texture { width: 1, height: 1, format: mao_formats::texture::Format::Png, rgba: vec![255; 4], mips: 1 });
        let white_sprite = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &sprite_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&white) }, wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) }],
        });
        let liquid = LiquidResources::new(gpu, liquid_bind_layout, &white, &sampler, &fsr_sampler, &sprite_layout);
        Renderer { shadow_cache: None, shadow_pipeline, pipeline_sets, shaders, antialiasing: Antialiasing::default(), culling, binds: std::sync::Mutex::new(HashMap::new()), records_layout, records: None, taa_pipeline, taa_layout, taa_sampler, taa: None, copy_pipeline, timer: GpuTimer::new(gpu), gpu_ms: None, frame: 0, previous: None, sprites: HashMap::new(), sprite_layout, white_sprite, shadow_view, shadow_bind, shadow_layout, shadow_sampler, shadow_size: SHADOW_SIZE, shadows: true, refraction: std::env::var_os("MAO_NO_REFRACTION").is_none(), stats: Stats::default(), render_scale: 1.0, compress_textures: true, mip_bias: std::env::var_os("MAO_NO_MIP_BIAS").is_none(), sharpness: 0.2, fsr_easu, fsr_rcas, fsr_layout, fsr_sampler, fsr: None, samples, msaa: None, started: std::time::Instant::now(), globals_layout, part_layout, sampler, white, format, depth: None, textures: Default::default(), liquid }
    }

    /// Puts a model on the GPU. `texture` finds a texture by (type, instance).
    /// Each part's geometry and texture are uploaded once however many times it is drawn.
    /// Makes a texture the sprite sheet `id` of the particle pass.
    /// The liquid passes' textures: the water's (`water5.png`), the toxic liquid's (`toxicwater.png`) and the lava's glow
    /// (`env_lava2.png`); one not given stays white.
    pub fn set_liquid_textures(&mut self, gpu: &Gpu, water: Option<&Texture>, toxic: Option<&Texture>, lava: Option<&Texture>) {
        let views: Vec<wgpu::TextureView> = [water, toxic, lava].iter().map(|t| t.map_or_else(|| self.white.clone(), |t| upload_blocks(gpu, t, self.compress_textures, true))).collect();
        self.liquid.textures = [views[0].clone(), views[1].clone(), views[2].clone()];
        self.liquid.rebind(gpu);
    }

    pub fn add_sprite(&mut self, gpu: &Gpu, id: u16, texture: &Texture) {
        let view = upload_blocks(gpu, texture, self.compress_textures, true);
        let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.sprite_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) }, wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) }],
        });
        self.sprites.insert(id, bind);
    }

    /// Whether the sprite sheet `id` is on the card.
    pub fn has_sprite(&self, id: u16) -> bool {
        self.sprites.contains_key(&id)
    }

    /// Uploads the characters again ([`Scene::set_overlay`]'s parts): see [`Renderer::reupload`].
    pub fn reupload_overlay(&self, gpu: &Gpu, scene: &mut Scene, model: &Model, texture: &mut dyn FnMut(u32, u32) -> Option<Texture>) {
        let old = std::mem::take(&mut scene.overlay);
        scene.overlay = self.reupload(gpu, model, texture, old);
    }

    /// Uploads the player's character again ([`Scene::set_follow`]'s parts, unmoved): see [`Renderer::reupload`].
    pub fn reupload_follow(&self, gpu: &Gpu, scene: &mut Scene, model: &Model, texture: &mut dyn FnMut(u32, u32) -> Option<Texture>) {
        let old = std::mem::take(&mut scene.follow);
        scene.follow = self.reupload(gpu, model, texture, old);
    }

    /// `model`'s draws as parts, for the characters, which are uploaded again with every network update (a few dozen
    /// times a second) as they move: making and freeing their hundreds of buffers each time cost milliseconds, and so
    /// did writing into each (a write is some 3 microseconds whatever its size, and there were a thousand). So the
    /// vertices of all the parts go into one buffer and their indices into another, each written once and kept from
    /// the last upload while it is large enough. (A model with watched meshes or water is uploaded afresh.)
    fn reupload(&self, gpu: &Gpu, model: &Model, texture: &mut dyn FnMut(u32, u32) -> Option<Texture>, old: Vec<GpuPart>) -> Vec<GpuPart> {
        if !model.lod_groups.is_empty() || model.water.is_some() {
            return self.upload(gpu, model, texture).parts;
        }
        let draw_list: Vec<(usize, [f32; 16])> = draws(model).into_iter().filter(|(i, _)| !model.parts[*i].indices.is_empty() && !model.parts[*i].vertices.is_empty()).collect();
        if draw_list.is_empty() {
            return Vec::new();
        }
        let stride = std::mem::size_of::<GpuVertex>() as u64;
        let vertex_bytes = draw_list.iter().map(|(i, _)| model.parts[*i].vertices.len() as u64).sum::<u64>() * stride;
        let index_bytes = (draw_list.iter().map(|(i, _)| model.parts[*i].indices.len() as u64).sum::<u64>() * 2).next_multiple_of(4);
        // (the last upload's pair, when both are large enough; else a new pair with a quarter to spare)
        let kept = old.iter().find(|p| p.characters && p.vertices.size() >= vertex_bytes && p.indices.size() >= index_bytes).map(|p| (p.vertices.clone(), p.vertex_id, p.indices.clone(), p.index_id));
        drop(old);
        let (vertex_buffer, vertex_id, index_buffer, index_id) = kept.unwrap_or_else(|| {
            let make = |label: &str, size: u64, usage: wgpu::BufferUsages| gpu.device.create_buffer(&wgpu::BufferDescriptor { label: Some(label), size: (size + size / 4).next_multiple_of(stride * 4), usage: usage | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
            (make("characters' vertices", vertex_bytes, wgpu::BufferUsages::VERTEX), next_id(), make("characters' indices", index_bytes, wgpu::BufferUsages::INDEX), next_id())
        });
        let mut cache = self.textures.borrow_mut();
        let mut vertices: Vec<GpuVertex> = Vec::with_capacity((vertex_bytes / stride) as usize);
        let mut indices: Vec<u16> = Vec::with_capacity((index_bytes / 2) as usize);
        let mut parts = Vec::with_capacity(draw_list.len());
        for (index, world) in draw_list {
            let part = &model.parts[index];
            let key = part.material.textures.first().copied();
            let view = key.and_then(|(kind, instance)| cache.entry((kind, instance)).or_insert_with(|| texture(kind, instance).map(|t| upload_blocks(gpu, &t, self.compress_textures, false))).clone());
            let (base_vertex, first_index) = (vertices.len() as i32, indices.len() as u32);
            let (mut lo, mut hi, mut height) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN), 0.0f32);
            for v in &part.vertices {
                vertices.push(GpuVertex { position: v.position, normal: v.normal, uv: v.uv, colour: v.colour });
                lo = lo.min(Vec3::from(v.position));
                hi = hi.max(Vec3::from(v.position));
                height = height.max(v.position[1]);
            }
            indices.extend_from_slice(&part.indices);
            let m = Mat4::from_cols_array(&world);
            let (mut wlo, mut whi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
            for corner in 0..8 {
                let c = Vec3::new(if corner & 1 == 0 { lo.x } else { hi.x }, if corner & 2 == 0 { lo.y } else { hi.y }, if corner & 4 == 0 { lo.z } else { hi.z });
                let p = m.transform_point3(c);
                wlo = wlo.min(p);
                whi = whi.max(p);
            }
            let (center, radius) = ((wlo + whi) * 0.5, (whi - wlo).length() * 0.5);
            let d = part.material.diffuse;
            let material = &part.material;
            let uniform = PartUniform { model: world, colour: [d[0], d[1], d[2], 1.0], flags: [view.is_some() as u32 as f32, (material.ignore_alpha as u32 + material.alpha_lights as u32) as f32, material.foliage as u32 as f32, height], previous_model: world, motion: [0.0; 4], tint: NO_TINT, blend: blend_of(material) };
            let bind = texture_bind(&gpu.device, &self.part_layout, &self.sampler, &self.white, &self.binds, key, view.as_ref());
            parts.push(GpuPart {
                vertices: vertex_buffer.clone(),
                vertex_id,
                base_vertex,
                indices: index_buffer.clone(),
                index_id,
                first_index,
                index_count: part.indices.len() as u32,
                geometry: next_id(),
                characters: true,
                bind,
                center,
                radius,
                uniform,
                shift: Vec3::ZERO,
                source: index,
                texture: key,
                low_only: false,
                faces: faces_of(material.two_sided, &m),
                lod: None,
                drawn: Default::default(),
                before: Default::default(),
            });
        }
        // (a write's size is a multiple of four bytes: an odd count of indices is filled out)
        if indices.len() % 2 == 1 {
            indices.push(0);
        }
        gpu.queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        gpu.queue.write_buffer(&index_buffer, 0, bytemuck::cast_slice(&indices));
        parts
    }

    pub fn upload(&self, gpu: &Gpu, model: &Model, texture: &mut dyn FnMut(u32, u32) -> Option<Texture>) -> Scene {
        let mut draw_list = draws(model);
        let (center, radius) = world_matrix_bounds(model, &draw_list);
        // The low-detail twins of the watched meshes: a draw of each low part for every instance of its full mesh's first
        // part, after the others (never drawn on their own: see `Lod`).
        let mut low_parts: HashSet<usize> = HashSet::new();
        let mut head_of: HashMap<usize, usize> = HashMap::new();
        for (g, (full, low)) in model.lod_groups.iter().enumerate() {
            head_of.insert(full.start, g);
            if let Some(low) = low {
                low_parts.extend(low.clone());
            }
        }
        let heads: Vec<(usize, [f32; 16])> = draw_list.iter().filter(|(i, _)| head_of.contains_key(i)).copied().collect();
        for (i, world) in heads {
            if let Some(low) = &model.lod_groups[head_of[&i]].1 {
                draw_list.extend(low.clone().map(|j| (j, world)));
            }
        }
        let mut cache = self.textures.borrow_mut();
        let (mut tex_time, mut geo_time, mut tex_count) = (std::time::Duration::ZERO, std::time::Duration::ZERO, 0u32);
        let upload_started = std::time::Instant::now();
        // The textures this model needs that are not on the card yet: fetched here, then made on every core (the device takes calls from
        // any thread), which is most of an upload's time on a playfield with hundreds of them.
        {
            let mut wanted: Vec<(u32, u32)> = draw_list.iter().filter_map(|(i, _)| model.parts[*i].material.textures.first().copied()).collect();
            wanted.sort_unstable();
            wanted.dedup();
            wanted.retain(|k| !cache.contains_key(k));
            let decoded: Vec<((u32, u32), Option<Texture>)> = wanted.into_iter().map(|(kind, instance)| ((kind, instance), texture(kind, instance))).collect();
            let next = std::sync::atomic::AtomicUsize::new(0);
            let made: std::sync::Mutex<Vec<((u32, u32), Option<wgpu::TextureView>)>> = std::sync::Mutex::new(Vec::new());
            let workers = std::thread::available_parallelism().map_or(2, |n| n.get()).min(16).min(decoded.len().max(1));
            let compress = self.compress_textures;
            std::thread::scope(|scope| {
                for _ in 0..workers {
                    scope.spawn(|| loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some((key, t)) = decoded.get(i) else { break };
                        let view = t.as_ref().map(|t| upload_blocks(gpu, t, compress, false));
                        made.lock().unwrap().push((*key, view));
                    });
                }
            });
            for (key, view) in made.into_inner().unwrap() {
                cache.insert(key, view);
            }
            tex_count = cache.len() as u32;
            tex_time = upload_started.elapsed();
        }
        // The geometry, each part's once however many times it is drawn, packed: the vertices of part after part into one
        // large buffer and their indices into another, a new pair begun when one would pass `PACK_BYTES` (a buffer has
        // a largest size, and the indices are 16 bits from the part's first vertex, so nothing bounds a pair but that).
        struct Packed {
            pack: usize,
            base_vertex: i32,
            first_index: u32,
            count: u32,
            view: Option<wgpu::TextureView>,
            height: f32,
            bounds: (Vec3, Vec3),
            id: u32,
            bind: TextureBind,
        }
        let mut packs: Vec<(Vec<GpuVertex>, Vec<u16>)> = vec![(Vec::new(), Vec::new())];
        let mut geometry: HashMap<usize, Option<Packed>> = HashMap::new();
        let mut parts: Vec<GpuPart> = Vec::new();
        for &(index, _) in &draw_list {
            let geo_started = std::time::Instant::now();
            geometry.entry(index).or_insert_with(|| {
                let part = &model.parts[index];
                if part.indices.is_empty() || part.vertices.is_empty() {
                    return None;
                }
                let stride = std::mem::size_of::<GpuVertex>();
                let (v, i) = packs.last().unwrap();
                if (v.len() + part.vertices.len()) * stride > PACK_BYTES || (i.len() + part.indices.len()) * 2 > PACK_BYTES {
                    packs.push((Vec::new(), Vec::new()));
                }
                let pack = packs.len() - 1;
                let (v, i) = packs.last_mut().unwrap();
                let (base_vertex, first_index) = (v.len() as i32, i.len() as u32);
                v.extend(part.vertices.iter().map(|v: &Vertex| GpuVertex { position: v.position, normal: v.normal, uv: v.uv, colour: v.colour }));
                i.extend_from_slice(&part.indices);
                let view = part.material.textures.first().and_then(|&(kind, instance)| cache.entry((kind, instance)).or_insert_with(|| { let t0 = std::time::Instant::now(); let r = texture(kind, instance).map(|t| upload_blocks(gpu, &t, self.compress_textures, false)); tex_time += t0.elapsed(); tex_count += 1; r }).clone());
                // A plant's height (in its own Y-up space) sets how it bends.
                let height = part.vertices.iter().map(|v| v.position[1]).fold(0.0f32, f32::max);
                let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
                for v in &part.vertices {
                    lo = lo.min(Vec3::from(v.position));
                    hi = hi.max(Vec3::from(v.position));
                }
                let bind = texture_bind(&gpu.device, &self.part_layout, &self.sampler, &self.white, &self.binds, part.material.textures.first().copied(), view.as_ref());
                Some(Packed { pack, base_vertex, first_index, count: part.indices.len() as u32, view, height, bounds: (lo, hi), id: next_id(), bind })
            });
            geo_time += geo_started.elapsed();
        }
        // (the pairs on the card, each buffer with its number; an odd count of indices filled out, a buffer's size being a
        // multiple of four bytes)
        let buffers: Vec<(wgpu::Buffer, u32, wgpu::Buffer, u32)> = packs
            .into_iter()
            .filter(|(v, _)| !v.is_empty())
            .map(|(v, mut i)| {
                if i.len() % 2 == 1 {
                    i.push(0);
                }
                let vb = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("packed vertices"), contents: bytemuck::cast_slice(&v), usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST });
                let ib = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("packed indices"), contents: bytemuck::cast_slice(&i), usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST });
                (vb, next_id(), ib, next_id())
            })
            .collect();
        let buffers = &buffers;
        // Each draw's own part (a playfield draws tens of thousands of props): made on every core.
        let geometry = &geometry;
        let build = |slice: &[(usize, [f32; 16])]| -> Vec<GpuPart> {
            slice
                .iter()
                .filter_map(|&(index, world)| {
                    let packed = geometry.get(&index)?.as_ref()?;
                    let (view, height, (lo, hi)) = (&packed.view, &packed.height, packed.bounds);
                    let (vb, vertex_id, ib, index_id) = buffers.get(packed.pack)?;
                    // A sphere around the part's box in the world.
                    let m = Mat4::from_cols_array(&world);
                    let (mut wlo, mut whi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
                    for corner in 0..8 {
                        let c = Vec3::new(if corner & 1 == 0 { lo.x } else { hi.x }, if corner & 2 == 0 { lo.y } else { hi.y }, if corner & 4 == 0 { lo.z } else { hi.z });
                        let p = m.transform_point3(c);
                        wlo = wlo.min(p);
                        whi = whi.max(p);
                    }
                    let (center, radius) = ((wlo + whi) * 0.5, (whi - wlo).length() * 0.5);
                    let d = model.parts[index].material.diffuse;
                    let material = &model.parts[index].material;
                    let uniform = PartUniform { model: world, colour: [d[0], d[1], d[2], 1.0], flags: [view.is_some() as u32 as f32, (material.ignore_alpha as u32 + material.alpha_lights as u32) as f32, material.foliage as u32 as f32, *height], previous_model: world, motion: [0.0; 4], tint: NO_TINT, blend: blend_of(material) };
                    Some(GpuPart { vertices: vb.clone(), vertex_id: *vertex_id, base_vertex: packed.base_vertex, indices: ib.clone(), index_id: *index_id, first_index: packed.first_index, index_count: packed.count, geometry: packed.id, characters: false, bind: packed.bind.clone(), center, radius, uniform, shift: Vec3::ZERO, source: index, texture: model.parts[index].material.textures.first().copied(), low_only: low_parts.contains(&index), faces: faces_of(material.two_sided, &m), lod: None, drawn: Default::default(), before: Default::default() })
                })
                .collect()
        };
        let workers = std::thread::available_parallelism().map_or(2, |n| n.get()).min(8);
        if draw_list.len() < 256 || workers < 2 {
            parts = build(&draw_list);
        } else {
            let chunk = draw_list.len().div_ceil(workers);
            let results: Vec<Vec<GpuPart>> = std::thread::scope(|scope| {
                let handles: Vec<_> = draw_list.chunks(chunk).map(|c| scope.spawn(|| build(c))).collect();
                handles.into_iter().map(|h| h.join().expect("an upload thread")).collect()
            });
            parts = results.into_iter().flatten().collect();
        }
        if std::env::var_os("MAO_PROFILE").is_some() && upload_started.elapsed() > std::time::Duration::from_millis(20) {
            eprintln!("[profile] upload {:?}: textures {tex_count} in {tex_time:?}, geometry+textures {geo_time:?}, {} parts", upload_started.elapsed(), parts.len());
        }
        // The watched meshes: each instance's head takes the mesh's world sphere, the count of the instance's parts that
        // follow it, and the draws of its low-detail parts (by the same world matrix).
        if !model.lod_groups.is_empty() {
            let mut low_by_world: HashMap<(usize, [u32; 16]), Vec<usize>> = HashMap::new();
            for (k, p) in parts.iter().enumerate() {
                if p.low_only {
                    low_by_world.entry((p.source, p.uniform.model.map(f32::to_bits))).or_default().push(k);
                }
            }
            let local_box = |i: usize| {
                let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
                for v in &model.parts[i].vertices {
                    lo = lo.min(Vec3::from(v.position));
                    hi = hi.max(Vec3::from(v.position));
                }
                (lo, hi)
            };
            for h in 0..parts.len() {
                let Some(&g) = head_of.get(&parts[h].source) else { continue };
                if parts[h].low_only {
                    continue;
                }
                let (full, low) = &model.lod_groups[g];
                let world = parts[h].uniform.model;
                let m = Mat4::from_cols_array(&world);
                let (mut wlo, mut whi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
                for i in full.clone() {
                    if model.parts[i].vertices.is_empty() {
                        continue;
                    }
                    let (lo, hi) = local_box(i);
                    for corner in 0..8 {
                        let c = Vec3::new(if corner & 1 == 0 { lo.x } else { hi.x }, if corner & 2 == 0 { lo.y } else { hi.y }, if corner & 4 == 0 { lo.z } else { hi.z });
                        let p = m.transform_point3(c);
                        wlo = wlo.min(p);
                        whi = whi.max(p);
                    }
                }
                let members = parts[h + 1..].iter().take_while(|p| full.contains(&p.source) && p.uniform.model == world && !p.low_only).count();
                let low_draws: Vec<usize> = low.iter().flat_map(|r| r.clone()).filter_map(|j| low_by_world.get(&(j, world.map(f32::to_bits)))).flatten().copied().collect();
                parts[h].lod = Some(Lod { center: (wlo + whi) * 0.5, radius: ((whi - wlo).length() * 0.5).max(0.01), members, low: low_draws, far: std::cell::Cell::new(false) });
            }
        }
        let liquid = model.water.as_ref().filter(|w| !w.surfaces.is_empty()).map(|w| LiquidMesh::new(gpu, &w.surfaces));
        Scene { parts, hidden: Default::default(), overlay: Vec::new(), follow: Vec::new(), center, radius, player: None, fog: (1.0e9, 2.0e9), sky: DAY_SKY, water: model.water.clone(), liquid, water_light: None, particles: Vec::new(), blobs: Vec::new(), shells: Vec::new(), backdrops: Vec::new(), sun: None, lighting: None, daylight: 1.0, ambient: [0.0; 3], area_fog: None, under_water: false }
    }

    fn depth_view(&mut self, gpu: &Gpu, size: (u32, u32), samples: u32) {
        if self.depth.as_ref().is_none_or(|(s, n, _)| *s != size || *n != samples) {
            let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("depth"),
                size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: samples,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            self.depth = Some((size, samples, texture.create_view(&wgpu::TextureViewDescriptor::default())));
        }
    }

    /// The multisampled colour target for a frame of `size` (made once per size).
    fn msaa_view(&mut self, gpu: &Gpu, size: (u32, u32)) {
        if self.samples > 1 && self.msaa.as_ref().is_none_or(|(s, _)| *s != size) {
            let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("msaa"),
                size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: self.samples,
                dimension: wgpu::TextureDimension::D2,
                format: self.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            self.msaa = Some((size, texture.create_view(&wgpu::TextureViewDescriptor::default())));
        }
    }

    /// The temporal anti-aliasing's targets at the size the scene is drawn (made once per size: a new size starts without a
    /// history).
    fn taa_targets(&mut self, gpu: &Gpu, size: (u32, u32)) {
        if self.taa.as_ref().is_some_and(|t| t.size == size) {
            return;
        }
        let make = |label: &str, format: wgpu::TextureFormat| {
            gpu.device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        };
        self.taa = Some(TaaTargets {
            size,
            colour: make("taa scene", self.format),
            motion: make("motion", MOTION),
            depth: make("taa depth", DEPTH),
            history: [make("history 0", HISTORY), make("history 1", HISTORY)],
            next: 0,
            valid: false,
            scene: (0, 0),
        });
    }

    /// The scaled scene and the upscaled frame for these sizes (made once per size).
    fn fsr_targets(&mut self, gpu: &Gpu, input: (u32, u32), output: (u32, u32)) -> (wgpu::TextureView, wgpu::TextureView) {
        if self.fsr.as_ref().is_none_or(|f| f.input != input || f.output != output) {
            let make = |label: &str, size: (u32, u32)| {
                gpu.device
                    .create_texture(&wgpu::TextureDescriptor {
                        label: Some(label),
                        size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: self.format,
                        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    })
                    .create_view(&wgpu::TextureViewDescriptor::default())
            };
            self.fsr = Some(FsrTargets { input, output, scene: make("scaled scene", input), mid: make("upscaled", output) });
        }
        let f = self.fsr.as_ref().unwrap();
        (f.scene.clone(), f.mid.clone())
    }

    /// Draws the scene into `target`, of `size` pixels.
    pub fn draw(&mut self, gpu: &Gpu, scene: &Scene, camera: &Camera, target: &wgpu::TextureView, size: (u32, u32)) {
        let mut clock = std::time::Instant::now();
        let mut times = [("render-setup", std::time::Duration::ZERO); 5];
        let mut lap = |i: usize, name: &'static str| {
            let now = std::time::Instant::now();
            times[i] = (name, now - clock);
            clock = now;
        };
        let aspect = size.0 as f32 / size.1.max(1) as f32;
        // Below native, the scene is drawn small and upscaled afterwards.
        let scale = self.render_scale.clamp(0.4, 1.0);
        let upscale = scale < 0.999;
        let render_size = if upscale { (((size.0 as f32 * scale).round() as u32).max(2), ((size.1 as f32 * scale).round() as u32).max(2)) } else { size };
        // Temporal anti-aliasing draws one sample per pixel, the projection moved by a fraction of a pixel each frame (a
        // 16-point Halton (2, 3) sequence); multisampling four.
        let taa = self.antialiasing == Antialiasing::Taa;
        let samples = if self.antialiasing == Antialiasing::Msaa { self.samples } else { 1 };
        let unjittered = camera.view_proj(aspect, scene.radius);
        let view_proj = if taa {
            let i = self.frame % 16 + 1;
            let (jx, jy) = (halton(i, 2) - 0.5, halton(i, 3) - 0.5);
            Mat4::from_translation(Vec3::new(jx * 2.0 / render_size.0 as f32, jy * 2.0 / render_size.1 as f32, 0.0)) * unjittered
        } else {
            unjittered
        };
        self.frame = self.frame.wrapping_add(1);
        let eye = camera.eye();
        let player = scene.player.unwrap_or(Vec3::splat(1.0e9));
        // The sun's box: around the player (or the whole scene with none),
        // its edges snapped to texels so the shadows don't shimmer as it moves.
        // (the scripts' sun when it is given, held a little over the horizon so the shadow map stays above the ground)
        // (`lit_sun` is the light the shaders light by: the scripts' as it is, under the horizon at night, so a one-sided
        // light leaves the ground to the ambient then)
        let lit_sun = match scene.sun {
            Some(s) if s.length_squared() > 0.0 => s.normalize(),
            _ => Vec3::new(0.4, -0.5, 0.8).normalize(),
        };
        let sun = Vec3::new(lit_sun.x, lit_sun.y, lit_sun.z.max(0.15)).normalize();
        let (focus, extent) = match scene.player {
            Some(p) => (p, 70.0f32.min(scene.radius.max(1.0))),
            None => (scene.center, scene.radius.max(1.0)),
        };
        // (the shadow map is kept while it still holds: the same scene, the player near where it was drawn, the sun where it was)
        let scene_key = (scene.parts.as_ptr() as usize, scene.parts.len());
        let cached = self.shadow_cache.filter(|c| c.scene == scene_key && c.shadows == self.shadows && c.extent == extent && (c.focus - focus).length() < SHADOW_MOVE && c.sun.dot(sun) > SHADOW_TURN);
        let redraw_shadows = cached.is_none() || std::env::var_os("MAO_SHADOW_EVERY_FRAME").is_some();
        let (focus, light_vp, texel) = match cached {
            Some(c) => (c.focus, c.light_vp, c.texel),
            None => {
                let depth_range = extent * 3.0 + scene.radius.min(400.0);
                let light_view = look_at(focus + sun * depth_range, focus, Vec3::Z);
                let texel = 2.0 * extent / self.shadow_size as f32;
                let c = light_view.transform_point3(focus);
                let snap = Vec3::new((c.x / texel).round() * texel - c.x, (c.y / texel).round() * texel - c.y, 0.0);
                let light_vp = orthographic(extent, 0.1, depth_range * 2.0) * Mat4::from_translation(snap) * light_view;
                self.shadow_cache = Some(ShadowCache { scene: scene_key, shadows: self.shadows, focus, sun, extent, light_vp, texel });
                (focus, light_vp, texel)
            }
        };
        // (the flat sheet only where the playfield has no water triangles)
        let water = scene.water.as_ref().filter(|w| w.surfaces.is_empty());
        let planes = frustum(&camera.view_proj(aspect, scene.radius));
        let (mut parts, mut drawn, mut shadow_drawn) = (0usize, 0usize, 0usize);
        // An area's fog replaces the distance haze: from the near plane (0.2, `VisualFog_t`'s default clip plane) to the far
        // plane brought in by the density (`process`: `far -= (far - near - 5) × density`), in the area's colour.
        // The LOD manager's decision for every watched mesh this frame (see `Lod`): which of the playfield's parts draw.
        let mut lod_draw: Vec<bool> = scene.parts.iter().map(|p| !p.low_only).collect();
        // (MAO_NO_LOD=1 draws every full mesh whatever its distance, to compare)
        let no_lod = std::env::var_os("MAO_NO_LOD").is_some();
        for (h, part) in scene.parts.iter().enumerate() {
            let Some(lod) = &part.lod else { continue };
            if no_lod {
                continue;
            }
            let d2 = (lod.center + part.shift - eye).length_squared();
            let r2 = lod.radius * lod.radius;
            let hidden = d2 / r2 > LOD_CULL;
            let beyond = d2 - r2;
            if beyond > LOD_FAR {
                lod.far.set(true);
            } else if beyond < LOD_NEAR {
                lod.far.set(false);
            }
            let use_low = lod.far.get() && !lod.low.is_empty();
            if hidden || use_low {
                for k in h..(h + 1 + lod.members).min(lod_draw.len()) {
                    lod_draw[k] = false;
                }
            }
            if use_low && !hidden {
                for &k in &lod.low {
                    lod_draw[k] = true;
                }
            }
        }
        if !scene.hidden.is_empty() {
            for (k, part) in scene.parts.iter().enumerate() {
                if scene.hidden.contains(&part.source) {
                    lod_draw[k] = false;
                }
            }
        }
        let (fog_start, fog_end, fog_colour, area) = match scene.area_fog {
            Some((colour, density)) if density > 0.0 => {
                let (near, far) = (0.2f32, scene.fog.1);
                let end = if far - near > 5.0 { far - (far - near - 5.0) * density.min(1.0) } else { far };
                (near, end, colour, 1.0)
            }
            _ => (scene.fog.0, scene.fog.1, scene.sky.0, 0.0),
        };
        let player_now = [player.x, player.y, player.z, self.started.elapsed().as_secs_f32()];
        let (previous_vp, previous_player) = self.previous.unwrap_or((unjittered, player_now));
        self.previous = Some((unjittered, player_now));
        let globals = Globals {
            view_proj: view_proj.to_cols_array(),
            current_vp: unjittered.to_cols_array(),
            previous_vp: previous_vp.to_cols_array(),
            previous_player,
            detail: [if self.mip_bias { mip_bias(scale) } else { 0.0 }, 0.0, 0.0, 0.0],
            light_vp: light_vp.to_cols_array(),
            params: [self.shadows as u32 as f32, texel, water.map_or(0.0, |w| w.level), water.is_some() as u32 as f32],
            rect: water.map_or([0.0; 4], |w| [w.min[0], w.min[1], w.max[0], w.max[1]]),
            light: [lit_sun.x, lit_sun.y, lit_sun.z, 0.0],
            player: player_now,
            eye: [eye.x, eye.y, eye.z, fog_start],
            horizon: [scene.sky.0[0], scene.sky.0[1], scene.sky.0[2], fog_end],
            zenith: [scene.sky.1[0], scene.sky.1[1], scene.sky.1[2], 0.0],
            ambient: [scene.ambient[0], scene.ambient[1], scene.ambient[2], scene.daylight],
            fog: [fog_colour[0], fog_colour[1], fog_colour[2], area],
            script_sun: scene.lighting.map_or([0.0; 4], |(_, s)| [s[0], s[1], s[2], 1.0]),
            script_ambient: scene.lighting.map_or([0.0; 4], |(a, _)| [a[0], a[1], a[2], 1.0]),
            right: {
                let forward = (camera.target - eye).normalize_or_zero();
                let right = forward.cross(Vec3::Z).normalize_or_zero();
                [right.x, right.y, right.z, 0.0]
            },
            up: {
                let forward = (camera.target - eye).normalize_or_zero();
                let right = forward.cross(Vec3::Z).normalize_or_zero();
                let up = right.cross(forward);
                [up.x, up.y, up.z, 0.0]
            },
        };
        let globals_buffer = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: bytemuck::bytes_of(&globals), usage: wgpu::BufferUsages::UNIFORM });
        if scene.liquid.is_some() {
            // (without the scripts' water light: the sun, as bright as the day, with the scripts' or the sky's ambient)
            let day = 0.2 + 0.8 * scene.daylight;
            let light = scene.water_light.unwrap_or(WaterLight { colour: [day; 3], to_light: sun, specular: 1.0 });
            // (the scripts' ambient, or the day's, with the area's as `AddAmbientLight` adds it: each channel at its maximum)
            let ambient = scene.lighting.map_or([day * 0.5; 3], |(a, _)| a);
            let ambient = [ambient[0].max(scene.ambient[0]), ambient[1].max(scene.ambient[1]), ambient[2].max(scene.ambient[2])];
            // (the camera's tilt: how much its forward points up, as FUN_1003b9c9 turns the camera's axis into the world)
            let tilt = (camera.target - eye).normalize_or_zero().z;
            self.liquid.update(gpu, light, ambient, [render_size.0 as f32, render_size.1 as f32, tilt, if scene.under_water { 0.2 } else { 0.0 }]);
        }
        let globals_bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.globals_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: globals_buffer.as_entire_binding() }],
        });
        let shadow_globals = Globals { view_proj: light_vp.to_cols_array(), ..globals };
        let shadow_buffer = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: bytemuck::bytes_of(&shadow_globals), usage: wgpu::BufferUsages::UNIFORM });
        let shadow_globals_bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.globals_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: shadow_buffer.as_entire_binding() }],
        });
        if taa {
            self.taa_targets(gpu, render_size);
            // (a frame of another scene, or after one without the anti-aliasing, starts the history afresh)
            let t = self.taa.as_mut().unwrap();
            if t.scene != scene_key {
                t.valid = false;
                t.scene = scene_key;
            }
            // The characters' motion: their parts moved by a shift (the player's character between updates) are drawn
            // with where they were the last frame, and all of them are marked as animating of themselves (their vertices
            // are uploaded anew as they move, which the matrices don't show).
            for part in scene.overlay.iter().chain(&scene.follow) {
                let last = part.drawn.get();
                part.before.set(last.map_or(part.shift, |(s, _)| s));
                part.drawn.set(Some((part.shift, last.is_some_and(|(s, _)| s != part.shift))));
            }
        } else {
            if let Some(t) = self.taa.as_mut() {
                t.valid = false;
            }
            for part in scene.overlay.iter().chain(&scene.follow) {
                part.drawn.set(None);
                part.before.set(part.shift);
            }
        }
        // The frame's draws. The shadow map's, when it is drawn anew: anything that can throw a shadow into the box, within
        // it horizontally and a little beyond for the slant. Only the playfield itself: the characters and the machines (the
        // overlay and the followers) throw the original's blob shadows instead (`Scene::blobs`), and nothing else in the
        // original throws one.
        let reach = extent * 1.6;
        let mut casters: Vec<&GpuPart> = if redraw_shadows && self.shadows { scene.parts.iter().enumerate().filter(|(k, p)| lod_draw[*k] && (p.center + p.shift - focus).truncate().length() <= reach + p.radius).map(|(_, p)| p).collect() } else { Vec::new() };
        // The scene's: the parts in view. (the parts under a translucent tint, a highlighted character's, are drawn blended
        // after the opaque parts; at alpha 0 not at all. The opaque pipeline writes the fragment's alpha to the target, so a
        // tinted part must not go through it. The same goes for the parts of blended materials, as randy31 draws its
        // transparent list, 6, after the others and far to near: `RVisual_t::AddToRenderList`'s buckets by the distance from
        // the camera, DisplaySystem's `Render(5, 6, .., 0x707, 0)`. Here they are ordered by their own distance, not in
        // buckets.)
        let (mut opaque, mut translucent): (Vec<(&GpuPart, bool)>, Vec<(&GpuPart, bool)>) = (Vec::new(), Vec::new());
        for (k, part) in scene.parts.iter().enumerate().chain(scene.overlay.iter().chain(&scene.follow).map(|p| (usize::MAX, p))) {
            parts += 1;
            if k != usize::MAX && !lod_draw[k] {
                continue;
            }
            // Plants bend up to a couple of units out of their box.
            if !sphere_visible(&planes, part.center + part.shift, part.radius + 2.0) {
                continue;
            }
            if part.uniform.tint[3] < 1.0 || part.uniform.blend[0] > 0.5 {
                if part.uniform.tint[3] > 0.0 {
                    translucent.push((part, k == usize::MAX));
                }
                continue;
            }
            opaque.push((part, k == usize::MAX));
        }
        // (the order: the opaque parts by the faces they leave out, their texture and their geometry, so that the same of
        // all three follow one another and go in one call, see `draw_runs`; the shadow map's the same way; the blended
        // ones far to near)
        casters.sort_by_key(|p| (p.bind.id, p.geometry));
        opaque.sort_by_key(|(p, _)| (p.faces, p.bind.id, p.geometry));
        translucent.sort_by(|(a, _), (b, _)| (b.center + b.shift).distance_squared(eye).total_cmp(&(a.center + a.shift).distance_squared(eye)));
        drawn += opaque.len() + translucent.len();
        shadow_drawn += casters.len();
        // Their records, in the order they are drawn, in one buffer (made larger when the frame needs it).
        let mut records: Vec<PartUniform> = Vec::with_capacity(casters.len() + opaque.len() + translucent.len());
        records.extend(casters.iter().map(|p| p.uniform));
        let first_opaque = records.len() as u32;
        records.extend(opaque.iter().map(|(p, character)| record_of(p, *character)));
        let first_translucent = records.len() as u32;
        records.extend(translucent.iter().map(|(p, character)| record_of(p, *character)));
        if self.records.as_ref().is_none_or(|r| r.2 < records.len().max(1)) {
            let capacity = (records.len() * 3 / 2).max(1024);
            let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor { label: Some("part records"), size: (capacity * std::mem::size_of::<PartUniform>()) as u64, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
            let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("part records"), layout: &self.records_layout, entries: &[wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() }] });
            self.records = Some((buffer, bind, capacity));
        }
        if !records.is_empty() {
            gpu.queue.write_buffer(&self.records.as_ref().unwrap().0, 0, bytemuck::cast_slice(&records));
        }
        let opaque: Vec<&GpuPart> = opaque.into_iter().map(|(p, _)| p).collect();
        let translucent: Vec<&GpuPart> = translucent.into_iter().map(|(p, _)| p).collect();
        let mut calls = 0usize;
        self.depth_view(gpu, render_size, samples);
        if samples > 1 {
            self.msaa_view(gpu, render_size);
        }
        if !self.pipeline_sets.contains_key(&(samples, taa)) {
            let set = ScenePipelines::new(&gpu.device, &self.shaders, self.format, samples, taa, self.culling);
            self.pipeline_sets.insert((samples, taa), set);
        }
        let fsr_views = if upscale { Some(self.fsr_targets(gpu, render_size, size)) } else { None };
        let pipes = &self.pipeline_sets[&(samples, taa)];
        let records_bind = &self.records.as_ref().unwrap().1;
        let taa_targets = self.taa.as_ref().filter(|_| taa);
        let scene_target: &wgpu::TextureView = match (taa_targets, &fsr_views) {
            (Some(t), _) => &t.colour,
            (None, Some(v)) => &v.0,
            (None, None) => target,
        };
        let depth = taa_targets.map_or(&self.depth.as_ref().unwrap().2, |t| &t.depth);
        // Multisampled: draw into the MSAA target and resolve into `target`.
        let (colour_view, resolve) = match &self.msaa {
            Some((_, msaa)) if samples > 1 => (msaa, Some(scene_target)),
            _ => (scene_target, None),
        };
        lap(0, "render-setup");
        let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        // (the card's time for this frame: what earlier frames measured read back, and this one's start marked)
        let _ = gpu.device.poll(wgpu::PollType::Poll);
        let timing = match self.timer.as_mut() {
            Some(t) => {
                if let Some(ms) = t.collect() {
                    self.gpu_ms = Some(ms);
                }
                t.slot().inspect(|_| encoder.write_timestamp(&t.queries, 0))
            }
            None => None,
        };
        if redraw_shadows {
            // The shadow map: what the sun sees (always cleared, so a frame with shadows off reads "lit").
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_view,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if self.shadows {
                pass.set_pipeline(&self.shadow_pipeline);
                pass.set_bind_group(0, &shadow_globals_bind, &[]);
                pass.set_bind_group(2, records_bind, &[]);
                draw_runs(&mut pass, &casters, 0, true, &|_| None);
            }
        }
        lap(1, "render-shadow");
        // The water's refraction pass (the original's `VisualLiquid_t::RenderRefraction`, FUN_1003b9c9) draws the picture so far
        // through the water, so with water in the scene the picture up to the liquid passes goes to a copy first: with
        // multisampling the pass resolves into the copy and the next pass carries on in the multisampled target; without, the
        // first pass draws into the copy and the next blits it back before carrying on.
        let refraction = self.refraction && scene.liquid.as_ref().is_some_and(|m| !m.ranges[0].is_empty());
        if refraction {
            self.liquid.ensure_screen(gpu, render_size, self.format);
        }
        let screen = self.liquid.screen.as_ref().filter(|_| refraction).map(|(_, view, blit)| (view, blit));
        {
            let (first_view, first_resolve, first_store) = match screen {
                Some((copy, _)) if samples > 1 => (colour_view, Some(copy), wgpu::StoreOp::Store),
                Some((copy, _)) => (copy, None, wgpu::StoreOp::Store),
                None => (colour_view, resolve, if resolve.is_some() { wgpu::StoreOp::Discard } else { wgpu::StoreOp::Store }),
            };
            let mut attachments = vec![Some(wgpu::RenderPassColorAttachment {
                view: first_view,
                depth_slice: None,
                resolve_target: first_resolve,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.66, g: 0.74, b: 0.84, a: 1.0 }), store: first_store },
            })];
            if let Some(t) = taa_targets {
                attachments.push(Some(wgpu::RenderPassColorAttachment { view: &t.motion, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store } }));
            }
            let depth_store = if taa { wgpu::StoreOp::Store } else { wgpu::StoreOp::Discard };
            // (the pass after the water's copy, when there is one: its attachments live as long as the pass)
            let mut later_attachments: Vec<Option<wgpu::RenderPassColorAttachment>> = Vec::new();
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene"),
                color_attachments: &attachments,
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth,
                    // (kept for the anti-aliasing, which follows the camera's move by it where nothing moved of itself)
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: if screen.is_some() { wgpu::StoreOp::Store } else { depth_store } }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&pipes.sky);
            pass.set_bind_group(0, &globals_bind, &[]);
            pass.draw(0..3, 0..1);
            for b in scene.backdrops.iter().filter(|b| !b.vertices.is_empty()) {
                let buffer = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("backdrop"), contents: bytemuck::cast_slice(&b.vertices), usage: wgpu::BufferUsages::VERTEX });
                let k = match b.blend {
                    BackdropBlend::Opaque => 0,
                    BackdropBlend::Alpha => 1,
                    BackdropBlend::Add => 2,
                };
                pass.set_pipeline(&pipes.backdrops[k]);
                pass.set_bind_group(0, &globals_bind, &[]);
                pass.set_bind_group(1, self.sprites.get(&b.sprite).unwrap_or(&self.white_sprite), &[]);
                pass.set_vertex_buffer(0, buffer.slice(..));
                pass.draw(0..b.vertices.len() as u32, 0..1);
            }
            // (the meshes: the opaque parts, then the blended ones one by one in their order)
            pass.set_bind_group(0, &globals_bind, &[]);
            pass.set_bind_group(2, records_bind, &[]);
            pass.set_bind_group(3, &self.shadow_bind, &[]);
            calls += draw_runs(&mut pass, &opaque, first_opaque, true, &|p| Some((p.faces as u32, &pipes.mesh[p.faces as usize])));
            calls += draw_runs(&mut pass, &translucent, first_translucent, false, &|p| Some((blend_number(p.uniform.blend) * 3 + p.faces as u32, blend_pipeline(pipes, p.uniform.blend, p.faces as usize))));
            if !scene.blobs.is_empty() {
                let buffer = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("blobs"), contents: bytemuck::cast_slice(&scene.blobs), usage: wgpu::BufferUsages::VERTEX });
                pass.set_pipeline(&pipes.blob);
                pass.set_bind_group(0, &globals_bind, &[]);
                pass.set_vertex_buffer(0, buffer.slice(..));
                pass.draw(0..scene.blobs.len() as u32, 0..1);
            }
            if let Some(mesh) = &scene.liquid {
                // (the passes in the original's order: the water, the toxic liquid, the lava, the wide water; the star water
                // is drawn as water, its own star field and mists not being ported)
                pass.set_bind_group(0, &globals_bind, &[]);
                pass.set_vertex_buffer(0, mesh.buffer.slice(..));
                for (layer, passes) in [(0usize, &[0u32, 1][..]), (3, &[0, 1]), (2, &[2, 3]), (1, &[5, 6]), (4, &[4])] {
                    let range = mesh.ranges[layer].clone();
                    if range.is_empty() {
                        continue;
                    }
                    for &p in passes {
                        pass.set_pipeline(if p == 1 || p == 3 { &pipes.liquid_inverse } else { &pipes.liquid });
                        pass.set_bind_group(1, &self.liquid.bind, &[p * LIQUID_PASS_STRIDE as u32]);
                        pass.draw(range.clone(), 0..1);
                    }
                }
                if let Some((_, blit)) = screen {
                    // The refraction: the picture so far is complete in the copy; the pass carries on (the multisampled target
                    // kept, or the copy blitted back) and the water's triangles are drawn again sampling the copy mirrored
                    // about the horizon (pass 7 of `liquid.wgsl`), blended by the band the original fades it with.
                    drop(pass);
                    later_attachments.push(Some(wgpu::RenderPassColorAttachment {
                        view: if samples > 1 { colour_view } else { scene_target },
                        depth_slice: None,
                        resolve_target: if samples > 1 { Some(scene_target) } else { None },
                        ops: wgpu::Operations {
                            load: if samples > 1 { wgpu::LoadOp::Load } else { wgpu::LoadOp::Clear(wgpu::Color::BLACK) },
                            store: if samples > 1 { wgpu::StoreOp::Discard } else { wgpu::StoreOp::Store },
                        },
                    }));
                    if let Some(t) = taa_targets {
                        later_attachments.push(Some(wgpu::RenderPassColorAttachment { view: &t.motion, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store } }));
                    }
                    pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("scene after the water"),
                        color_attachments: &later_attachments,
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: depth,
                            depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Load, store: depth_store }),
                            stencil_ops: None,
                        }),
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        multiview_mask: None,
                    });
                    if samples == 1 {
                        pass.set_pipeline(&pipes.blit);
                        pass.set_bind_group(0, blit, &[]);
                        pass.draw(0..3, 0..1);
                    }
                    pass.set_pipeline(&pipes.liquid);
                    pass.set_bind_group(0, &globals_bind, &[]);
                    pass.set_bind_group(1, &self.liquid.bind_screen, &[7 * LIQUID_PASS_STRIDE as u32]);
                    pass.set_vertex_buffer(0, mesh.buffer.slice(..));
                    pass.draw(mesh.ranges[0].clone(), 0..1);
                }
            } else if water.is_some() {
                pass.set_pipeline(&pipes.water);
                pass.set_bind_group(0, &globals_bind, &[]);
                pass.draw(0..6, 0..1);
            }
            for shell in scene.shells.iter().filter(|s| !s.vertices.is_empty()) {
                let buffer = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("shell"), contents: bytemuck::cast_slice(&shell.vertices), usage: wgpu::BufferUsages::VERTEX });
                pass.set_pipeline(if shell.additive { &pipes.shell_add } else { &pipes.shell_blend });
                pass.set_bind_group(0, &globals_bind, &[]);
                pass.set_bind_group(1, self.sprites.get(&shell.sprite).unwrap_or(&self.white_sprite), &[]);
                pass.set_vertex_buffer(0, buffer.slice(..));
                pass.draw(0..shell.vertices.len() as u32, 0..1);
            }
            if !scene.particles.is_empty() {
                // grouped by sprite sheet (a sheet that is not loaded draws as dots), one draw for each
                let mut order: Vec<&Particle> = scene.particles.iter().collect();
                let sheet = |p: &Particle| if self.sprites.contains_key(&p.sprite) { p.sprite } else { NO_SPRITE };
                order.sort_by_key(|p| sheet(p));
                let data: Vec<[f32; 20]> = order
                    .iter()
                    .map(|p| {
                        let rect = if self.sprites.contains_key(&p.sprite) { p.rect } else { [0.0; 4] };
                        [
                            p.position[0], p.position[1], p.position[2], p.size, p.colour[0], p.colour[1], p.colour[2], p.colour[3], rect[0], rect[1], rect[2], rect[3],
                            p.right[0], p.right[1], p.right[2], 0.0, p.up[0], p.up[1], p.up[2], 0.0,
                        ]
                    })
                    .collect();
                let instances = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("particles"), contents: bytemuck::cast_slice(&data), usage: wgpu::BufferUsages::VERTEX });
                pass.set_pipeline(&pipes.particle);
                pass.set_bind_group(0, &globals_bind, &[]);
                pass.set_vertex_buffer(0, instances.slice(..));
                let mut start = 0;
                while start < order.len() {
                    let id = sheet(order[start]);
                    let end = start + order[start..].iter().take_while(|p| sheet(p) == id).count();
                    pass.set_bind_group(1, self.sprites.get(&id).unwrap_or(&self.white_sprite), &[]);
                    pass.draw(0..6, start as u32..end as u32);
                    start = end;
                }
            }
        }
        lap(2, "render-scene");
        if std::env::var_os("MAO_DRAW_GROUPS").is_some() {
            let mut sources = std::collections::BTreeMap::<usize, usize>::new();
            for (k, part) in scene.parts.iter().enumerate() {
                if lod_draw[k] && sphere_visible(&planes, part.center + part.shift, part.radius + 2.0) {
                    *sources.entry(part.source).or_default() += 1;
                }
            }
            let many = sources.values().filter(|&&n| n > 1).count();
            let reach = extent * 1.6;
            let casters: Vec<&GpuPart> = scene.parts.iter().enumerate().filter(|(k, p)| lod_draw[*k] && (p.center + p.shift - focus).truncate().length() <= reach + p.radius).map(|(_, p)| p).collect();
            let small = casters.iter().filter(|p| p.radius < 1.0).count();
            let medium = casters.iter().filter(|p| p.radius >= 1.0 && p.radius < 3.0).count();
            let foliage = casters.iter().filter(|p| p.uniform.flags[2] > 0.5).count();
            eprintln!("shadow casters: {} ({} under 1 m, {} 1-3 m, {} foliage), extent {extent}", casters.len(), small, medium, foliage);
            eprintln!("draw groups: {} playfield draws from {} meshes ({} repeated), {} overlay, {} follow", sources.values().sum::<usize>(), sources.len(), many, scene.overlay.len(), scene.follow.len());
        }
        self.stats = Stats { parts, drawn, shadow_drawn, calls, times: [("", std::time::Duration::ZERO); 5] };
        // The temporal anti-aliasing: this frame blended into the last (`taa.wgsl`), at the size the scene was drawn; the result
        // is what is upscaled or copied to the target, and the next frame's history.
        let mut resolved: Option<wgpu::TextureView> = None;
        if let Some(t) = taa_targets {
            let (inw, inh) = (render_size.0 as f32, render_size.1 as f32);
            let mut params: Vec<f32> = Vec::with_capacity(36);
            params.extend_from_slice(&unjittered.inverse().to_cols_array());
            params.extend_from_slice(&previous_vp.to_cols_array());
            params.extend_from_slice(&[1.0 / inw, 1.0 / inh, if t.valid { 1.0 } else { 0.0 }, 0.1]);
            let buffer = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("taa"), contents: bytemuck::cast_slice(&params), usage: wgpu::BufferUsages::UNIFORM });
            let (out, past) = (&t.history[t.next], &t.history[1 - t.next]);
            let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("taa"),
                layout: &self.taa_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&t.colour) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&t.motion) },
                    wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&t.depth) },
                    wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(past) },
                    wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::Sampler(&self.taa_sampler) },
                ],
            });
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("taa"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: out, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store } })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.taa_pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.draw(0..3, 0..1);
            resolved = Some(out.clone());
        }
        if let Some(t) = self.taa.as_mut().filter(|_| taa) {
            t.next = 1 - t.next;
            t.valid = true;
        }
        if let (Some(frame), None) = (&resolved, &fsr_views) {
            // (full size: the anti-aliased frame copied onto the target)
            let buffer = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: bytemuck::cast_slice(&[0.0f32; 20]), usage: wgpu::BufferUsages::UNIFORM });
            let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.fsr_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(frame) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.fsr_sampler) },
                ],
            });
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("copy"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: target, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store } })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.copy_pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.draw(0..3, 0..1);
        }
        if let Some((scene_view, mid_view)) = &fsr_views {
            let scene_view = resolved.as_ref().unwrap_or(scene_view);
            let (inw, inh) = (render_size.0 as f32, render_size.1 as f32);
            let (outw, outh) = (size.0 as f32, size.1 as f32);
            let params: [f32; 20] = [
                inw / outw, inh / outh, 0.5 * inw / outw - 0.5, 0.5 * inh / outh - 0.5,
                1.0 / inw, 1.0 / inh, 1.0 / inw, -1.0 / inh,
                -1.0 / inw, 2.0 / inh, 1.0 / inw, 2.0 / inh,
                0.0, 4.0 / inh, 0.0, 0.0,
                (-self.sharpness).exp2(), 0.0, 0.0, 0.0,
            ];
            let buffer = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: bytemuck::cast_slice(&params), usage: wgpu::BufferUsages::UNIFORM });
            let bind = |view: &wgpu::TextureView| {
                gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &self.fsr_layout,
                    entries: &[
                        wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() },
                        wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(view) },
                        wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.fsr_sampler) },
                    ],
                })
            };
            for (pipeline, input, output) in [(&self.fsr_easu, scene_view, mid_view), (&self.fsr_rcas, mid_view, target)] {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("fsr"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: output, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store } })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, &bind(input), &[]);
                pass.draw(0..3, 0..1);
            }
        }
        lap(3, "render-post");
        if let (Some(slot), Some(t)) = (timing, self.timer.as_ref()) {
            encoder.write_timestamp(&t.queries, 1);
            encoder.resolve_query_set(&t.queries, 0..2, &t.resolve, 0);
            encoder.copy_buffer_to_buffer(&t.resolve, 0, &t.readbacks[slot].0, 0, 16);
        }
        gpu.queue.submit(Some(encoder.finish()));
        if let (Some(slot), Some(t)) = (timing, self.timer.as_ref()) {
            let state = t.readbacks[slot].1.clone();
            state.store(1, std::sync::atomic::Ordering::Release);
            t.readbacks[slot].0.slice(..).map_async(wgpu::MapMode::Read, move |result| {
                state.store(if result.is_ok() { 2 } else { 0 }, std::sync::atomic::Ordering::Release);
            });
        }
        lap(4, "render-submit");
        self.stats.times = times;
    }

    /// The graphics memory this program's renderer holds (bytes), where the device reports it (Direct3D 12 and Vulkan).
    pub fn gpu_memory(gpu: &Gpu) -> Option<u64> {
        gpu.device.generate_allocator_report().map(|r| r.total_reserved_bytes)
    }

    /// Draws the scene to a `width` x `height` RGBA image (headless).
    pub fn render_to_image(&mut self, gpu: &Gpu, scene: &Scene, camera: &Camera, width: u32, height: u32) -> Vec<u8> {
        self.render_to_image_with(gpu, scene, camera, width, height, &mut |_| {})
    }

    /// [`Renderer::render_to_image`] with `overlay` called on the image after
    /// the scene is drawn (to paint an interface over it).
    pub fn render_to_image_with(&mut self, gpu: &Gpu, scene: &Scene, camera: &Camera, width: u32, height: u32, overlay: &mut dyn FnMut(&wgpu::TextureView)) -> Vec<u8> {
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("image"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.draw(gpu, scene, camera, &view, (width, height));
        overlay(&view);
        let row = (width * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor { label: None, size: (row * height) as u64, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });
        let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::TexelCopyBufferInfo { buffer: &buffer, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(height) } },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
        gpu.queue.submit(Some(encoder.finish()));
        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
        let data = slice.get_mapped_range().expect("the mapped image");
        let bgra = matches!(self.format, wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb);
        let mut out = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height as usize {
            for px in data[y * row as usize..y * row as usize + width as usize * 4].chunks_exact(4) {
                if bgra {
                    out.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
                } else {
                    out.extend_from_slice(px);
                }
            }
        }
        out
    }
}

/// A level's side in blocks of four texels.
fn blocks(side: u32) -> u32 {
    side.div_ceil(4)
}

/// A texture in the card's block formats: its levels one after the other, how many, and the format.
pub struct Blocks {
    pub data: Vec<u8>,
    pub levels: u32,
    pub format: wgpu::TextureFormat,
}

/// The block encoders used (Intel's, `intel_tex_2`). All three take 8 bits a texel where the plain texture takes 32.
#[derive(Clone, Copy, PartialEq)]
enum Encoder {
    /// BC7 for a texture without alpha, at the quickest setting: a core does about 50 megapixels a second.
    Bc7Opaque,
    /// BC7 with alpha, at the quickest setting: about 20 megapixels a second.
    Bc7Alpha,
    /// BC3 (the old DXT5): a 565 colour pair and a full alpha pair a block; hundreds of megapixels a second.
    Bc3,
}

impl Encoder {
    fn encode(self, data: &[u8], width: u32, height: u32) -> Vec<u8> {
        let surface = intel_tex_2::RgbaSurface { data, width, height, stride: width * 4 };
        match self {
            Encoder::Bc7Opaque => intel_tex_2::bc7::compress_blocks(&intel_tex_2::bc7::opaque_ultra_fast_settings(), &surface),
            Encoder::Bc7Alpha => intel_tex_2::bc7::compress_blocks(&intel_tex_2::bc7::alpha_ultra_fast_settings(), &surface),
            Encoder::Bc3 => intel_tex_2::bc3::compress_blocks(&surface),
        }
    }

    /// A level of whole blocks encoded; one of a megapixel or more in strips, a core each (the blocks do not depend on
    /// one another), so that a 4096 by 4096 sheet does not hold its thread for most of a second.
    fn level(self, data: &[u8], width: u32, height: u32) -> Vec<u8> {
        let threads = if width as usize * height as usize >= 1 << 20 { std::thread::available_parallelism().map_or(1, |n| n.get()).min(16) } else { 1 };
        if threads < 2 {
            return self.encode(data, width, height);
        }
        let strip = (height / 4).div_ceil(threads as u32).max(1) as usize * 4 * width as usize * 4;
        std::thread::scope(|scope| {
            let handles: Vec<_> = data.chunks(strip).map(|rows| scope.spawn(move || self.encode(rows, width, (rows.len() / (width as usize * 4)) as u32))).collect();
            handles.into_iter().flat_map(|h| h.join().expect("an encoder thread")).collect()
        })
    }
}

/// The texture's levels in the card's block formats, a quarter of their plain size; none for a texture whose sides are
/// not multiples of four (the card wants whole blocks in the first level) or that is smaller than 8 texels. A level
/// whose sides are not multiples of four (the last two of every chain) is filled out to whole blocks with its edge
/// texels.
///
/// The format: BC7 for a texture with no texel under full alpha. One with alpha takes BC3 unless `fine_alpha` asks for
/// BC7: measured on forty of the game's own large textures with alpha (the `bcquality` example), BC3 keeps the colour
/// to 40 dB and the alpha to 60 dB where BC7's quickest setting keeps 45 dB and 54 dB, and takes a thirty-sixth of the
/// time (0.3 s against 11.6 s). The meshes' textures, a thousand of them in a playfield, go the quick way; the sky's and
/// the effects' sheets and the liquids', few and of soft gradients where BC3's colour steps would show, the fine one.
pub fn block_levels(t: &Texture, fine_alpha: bool) -> Option<Blocks> {
    if t.width < 8 || t.height < 8 || t.width % 4 != 0 || t.height % 4 != 0 {
        return None;
    }
    let chain;
    let (data, levels): (&[u8], u32) = if t.mips > 1 {
        (&t.rgba, t.mips)
    } else {
        chain = mao_formats::texture::mip_chain(&t.rgba, t.width, t.height);
        (&chain.0, chain.1)
    };
    let first = (t.width as usize * t.height as usize * 4).min(data.len());
    let opaque = data[..first].chunks_exact(4).all(|p| p[3] >= 250);
    let encoder = match (opaque, fine_alpha) {
        (true, _) => Encoder::Bc7Opaque,
        (false, true) => Encoder::Bc7Alpha,
        (false, false) => Encoder::Bc3,
    };
    let mut out = Vec::with_capacity(first / 3);
    let (mut w, mut h, mut at) = (t.width, t.height, 0usize);
    for _ in 0..levels {
        let size = w as usize * h as usize * 4;
        let level = data.get(at..at + size)?;
        let (bw, bh) = (blocks(w) * 4, blocks(h) * 4);
        if (bw, bh) == (w, h) {
            out.extend(encoder.level(level, w, h));
        } else {
            let mut padded = Vec::with_capacity((bw * bh * 4) as usize);
            for y in 0..bh {
                for x in 0..bw {
                    let from = ((y.min(h - 1) * w + x.min(w - 1)) * 4) as usize;
                    padded.extend_from_slice(&level[from..from + 4]);
                }
            }
            out.extend(encoder.level(&padded, bw, bh));
        }
        at += size;
        (w, h) = ((w / 2).max(1), (h / 2).max(1));
    }
    Some(Blocks { data: out, levels, format: if encoder == Encoder::Bc3 { wgpu::TextureFormat::Bc3RgbaUnormSrgb } else { wgpu::TextureFormat::Bc7RgbaUnormSrgb } })
}

/// A texture on the card: in blocks when that is asked for and the device and the texture allow it ([`block_levels`]),
/// else as it is.
fn upload_blocks(gpu: &Gpu, t: &Texture, compress: bool, fine_alpha: bool) -> wgpu::TextureView {
    let Some(made) = (compress && gpu.bc).then(|| block_levels(t, fine_alpha)).flatten() else { return upload_texture(gpu, t) };
    let texture = gpu.device.create_texture_with_data(
        &gpu.queue,
        &wgpu::TextureDescriptor {
            label: Some("block texture"),
            size: wgpu::Extent3d { width: t.width, height: t.height, depth_or_array_layers: 1 },
            mip_level_count: made.levels,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: made.format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
        wgpu::util::TextureDataOrder::LayerMajor,
        &made.data,
    );
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

/// The texture's mip levels, smallest last, each half the size of the one before
/// (a box filter): all the bytes in order, and how many levels.
fn upload_texture(gpu: &Gpu, t: &Texture) -> wgpu::TextureView {
    let (data, levels) = if t.mips > 1 { (t.rgba.clone(), t.mips) } else { mao_formats::texture::mip_chain(&t.rgba, t.width, t.height) };
    let texture = gpu.device.create_texture_with_data(
        &gpu.queue,
        &wgpu::TextureDescriptor {
            label: Some("plain texture"),
            size: wgpu::Extent3d { width: t.width, height: t.height, depth_or_array_layers: 1 },
            mip_level_count: levels,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
        wgpu::util::TextureDataOrder::LayerMajor,
        &data,
    );
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The mip bias: none at the window's size, a level at a half, FSR 1's figures for its steps between.
    #[test]
    fn the_mip_bias_is_the_scales_logarithm() {
        assert_eq!(mip_bias(1.0), 0.0);
        assert_eq!(mip_bias(0.5), -1.0);
        for (scale, want) in [(0.77, -0.38), (0.67, -0.58), (0.59, -0.76)] {
            assert!((mip_bias(scale) - want).abs() < 0.01, "{scale}");
        }
        // (the scale is held to what the renderer draws at)
        assert_eq!(mip_bias(0.1), mip_bias(0.4));
        assert_eq!(mip_bias(3.0), 0.0);
    }

    /// The faces left out: none for a material of two sides, else by the sense of the part's matrix.
    #[test]
    fn the_faces_left_out_follow_the_material_and_the_matrix() {
        let swap = Mat4::from_cols_array(&[1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0]);
        assert_eq!(faces_of(true, &swap), 0);
        assert_eq!(faces_of(false, &swap), 1);
        assert_eq!(faces_of(false, &Mat4::IDENTITY), 2);
        // (a statel mirrored once more is the other way round again)
        assert_eq!(faces_of(false, &(swap * Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0)))), 2);
        assert_eq!(Culling::from_name("off"), Culling::Off);
        assert_eq!(Culling::from_name("Show"), Culling::Show);
        assert_eq!(Culling::from_name(""), Culling::On);
    }

    /// A square seen from its back is not drawn, unless its material has two sides (the original's cull mode).
    #[test]
    fn a_face_turned_away_is_not_drawn() {
        // (skipped on a machine without a graphics device, and when the run asks for no culling)
        let Ok(gpu) = Gpu::headless() else { return };
        if std::env::var_os("MAO_CULL").is_some() {
            return;
        }
        let camera = Camera { target: Vec3::ZERO, distance: 30.0, yaw: 0.3, pitch: 1.2, half_height: None };
        // (a red square on the ground, its corners counter-clockwise seen from above as the terrain's are, or the other
        // way round)
        let square = |reversed: bool, two_sided: bool| {
            let vertices = [[-10.0, -10.0, 0.0], [10.0, -10.0, 0.0], [10.0, 10.0, 0.0], [-10.0, 10.0, 0.0]].iter().map(|&p| mao_formats::model::Vertex { position: p, normal: [0.0, 0.0, 1.0], uv: [0.0, 0.0], colour: mao_formats::model::Vertex::UNBAKED }).collect();
            let indices = if reversed { vec![0, 2, 1, 0, 3, 2] } else { vec![0, 1, 2, 0, 2, 3] };
            let material = mao_formats::model::Material { diffuse: [1.0, 0.0, 0.0], opacity: 1.0, two_sided, ..Default::default() };
            Model { parts: vec![mao_formats::model::Part { name: String::new(), world: Mat4::IDENTITY.to_cols_array(), vertices, indices, material }], ..Default::default() }
        };
        let red = |model: &Model| {
            let mut renderer = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm);
            let scene = renderer.upload(&gpu, model, &mut |_, _| None);
            let image = renderer.render_to_image(&gpu, &scene, &camera, 160, 120);
            image.chunks(4).filter(|p| p[0] > 2 * p[1].max(p[2]) && p[0] > 40).count()
        };
        assert!(red(&square(false, false)) > 500, "the front is drawn");
        assert_eq!(red(&square(true, false)), 0, "the back is not");
        assert!(red(&square(true, true)) > 500, "a material of two sides shows its back");
    }

    /// A part placed three times is drawn in one call, each of the three at its own place (its own record); a part of
    /// another geometry takes a call of its own.
    #[test]
    fn the_same_part_in_several_places_is_one_call() {
        // (skipped on a machine without a graphics device)
        let Ok(gpu) = Gpu::headless() else { return };
        let camera = Camera { target: Vec3::ZERO, distance: 40.0, yaw: 0.0, pitch: 1.4, half_height: None };
        let square = |half: f32| {
            let vertices = [[-half, -half, 0.0], [half, -half, 0.0], [half, half, 0.0], [-half, half, 0.0]].iter().map(|&p| mao_formats::model::Vertex { position: p, normal: [0.0, 0.0, 1.0], uv: [0.0, 0.0], colour: mao_formats::model::Vertex::UNBAKED }).collect();
            mao_formats::model::Part { name: String::new(), world: Mat4::IDENTITY.to_cols_array(), vertices, indices: vec![0, 1, 2, 0, 2, 3], material: mao_formats::model::Material { diffuse: [1.0, 0.0, 0.0], opacity: 1.0, two_sided: true, ..Default::default() } }
        };
        let at = |x: f32| Mat4::from_translation(Vec3::new(x, 0.0, 0.0)).to_cols_array();
        let picture = |model: &Model| {
            let mut renderer = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm);
            let scene = renderer.upload(&gpu, model, &mut |_, _| None);
            let image = renderer.render_to_image(&gpu, &scene, &camera, 320, 120);
            let red = image.chunks(4).filter(|p| p[0] > 2 * p[1].max(p[2]) && p[0] > 40).count();
            (red, renderer.stats.calls, renderer.stats.drawn)
        };
        let (one, calls, drawn) = picture(&Model { parts: vec![square(2.0)], instances: vec![(0, at(0.0))], ..Default::default() });
        assert_eq!((calls, drawn), (1, 1));
        assert!(one > 50, "{one}");
        let (three, calls, drawn) = picture(&Model { parts: vec![square(2.0)], instances: vec![(0, at(-12.0)), (0, at(0.0)), (0, at(12.0))], ..Default::default() });
        assert_eq!((calls, drawn), (1, 3), "three places, one call");
        // (apart from one another: three times the red of one, give or take the perspective)
        assert!(three > one * 5 / 2 && three < one * 7 / 2, "each of the three is drawn at its own place: {three} against {one}");
        let (_, calls, drawn) = picture(&Model { parts: vec![square(2.0), square(1.0)], instances: vec![(0, at(-12.0)), (0, at(0.0)), (1, at(12.0))], ..Default::default() });
        assert_eq!((calls, drawn), (2, 3), "another geometry, another call");
    }

    /// A texture in BC7 blocks: a quarter of the bytes, every level in whole blocks, and none for sides that are not
    /// multiples of four.
    #[test]
    fn a_texture_in_blocks_is_a_quarter_the_size() {
        let texture = |w: u32, h: u32, alpha: u8| Texture { width: w, height: h, format: mao_formats::texture::Format::Png, rgba: (0..w * h).flat_map(|i| [(i % w) as u8, (i / w) as u8, 90, alpha]).collect(), mips: 1 };
        let made = block_levels(&texture(64, 32, 255), false).unwrap();
        // (64x32, 32x16, 16x8, 8x4, then 4x2, 2x1 and 1x1 as one block each)
        assert_eq!(made.levels, 7);
        assert_eq!(made.data.len(), (16 * 8 + 8 * 4 + 4 * 2 + 2 + 1 + 1 + 1) * 16);
        assert!(made.format == wgpu::TextureFormat::Bc7RgbaUnormSrgb);
        let plain = mao_formats::texture::mip_chain(&texture(64, 32, 255).rgba, 64, 32).0.len();
        assert!(made.data.len() * 3 < plain, "{} against {plain}", made.data.len());
        // (with alpha: BC3 the quick way, BC7 the fine one; the same size)
        let (quick, fine) = (block_levels(&texture(64, 32, 128), false).unwrap(), block_levels(&texture(64, 32, 128), true).unwrap());
        assert!(quick.format == wgpu::TextureFormat::Bc3RgbaUnormSrgb && fine.format == wgpu::TextureFormat::Bc7RgbaUnormSrgb);
        assert_eq!((quick.data.len(), fine.data.len()), (made.data.len(), made.data.len()));
        assert!(block_levels(&texture(30, 32, 255), false).is_none() && block_levels(&texture(4, 4, 255), false).is_none());
    }

    /// A large level encoded in strips on several cores is the same bytes as in one piece.
    #[test]
    fn a_level_in_strips_is_the_same_blocks() {
        let (w, h) = (1024u32, 1028u32);
        let mut seed = 7u32;
        let data: Vec<u8> = (0..w * h * 4).map(|i| { seed = seed.wrapping_mul(1664525).wrapping_add(1013904223); ((i / 4 % w) as u8 / 2).wrapping_add((seed >> 28) as u8) }).collect();
        for encoder in [Encoder::Bc7Opaque, Encoder::Bc3] {
            assert!(encoder.level(&data, w, h) == encoder.encode(&data, w, h));
        }
    }

    /// The picture of a textured square is the same, near enough, with its texture in blocks.
    #[test]
    fn a_compressed_texture_draws_the_same_picture() {
        // (skipped on a machine without a graphics device, or one that takes no block-compressed textures)
        let Ok(gpu) = Gpu::headless() else { return };
        if !gpu.bc {
            return;
        }
        let camera = Camera { target: Vec3::ZERO, distance: 30.0, yaw: 0.3, pitch: 1.3, half_height: None };
        let vertices = [([-10.0, -10.0, 0.0], [0.0, 0.0]), ([10.0, -10.0, 0.0], [1.0, 0.0]), ([10.0, 10.0, 0.0], [1.0, 1.0]), ([-10.0, 10.0, 0.0], [0.0, 1.0])].iter().map(|&(p, uv)| mao_formats::model::Vertex { position: p, normal: [0.0, 0.0, 1.0], uv, colour: mao_formats::model::Vertex::UNBAKED }).collect();
        let material = mao_formats::model::Material { diffuse: [1.0; 3], opacity: 1.0, two_sided: true, textures: vec![(1, 1)], ..Default::default() };
        let model = Model { parts: vec![mao_formats::model::Part { name: String::new(), world: Mat4::IDENTITY.to_cols_array(), vertices, indices: vec![0, 1, 2, 0, 2, 3], material }], ..Default::default() };
        // (soft stripes of colour, 256 texels square)
        let texture = Texture { width: 256, height: 256, format: mao_formats::texture::Format::Png, rgba: (0..256u32 * 256).flat_map(|i| [(i % 256) as u8, (i / 256) as u8, (((i % 256) / 32) * 32) as u8, 255]).collect(), mips: 1 };
        let picture = |compress: bool| {
            let mut renderer = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm);
            renderer.compress_textures = compress;
            let scene = renderer.upload(&gpu, &model, &mut |_, _| Some(texture.clone()));
            renderer.render_to_image(&gpu, &scene, &camera, 160, 120)
        };
        let (plain, blocks) = (picture(false), picture(true));
        let worst = plain.iter().zip(&blocks).map(|(a, b)| a.abs_diff(*b)).max().unwrap();
        let mean = plain.iter().zip(&blocks).map(|(a, b)| a.abs_diff(*b) as f64).sum::<f64>() / plain.len() as f64;
        assert!(worst <= 24 && mean < 1.5, "worst {worst}, mean {mean}");
        // (and it is a picture of the texture, not of nothing)
        assert!(plain.chunks(4).filter(|p| p[0] > 100).count() > 500);
    }

    /// The characters' upload: their parts share one vertex buffer and one index buffer, each its own stretch of them,
    /// and a second upload writes into the same pair; the picture is each part at its place.
    #[test]
    fn the_characters_share_their_buffers_from_upload_to_upload() {
        // (skipped on a machine without a graphics device)
        let Ok(gpu) = Gpu::headless() else { return };
        let camera = Camera { target: Vec3::ZERO, distance: 40.0, yaw: 0.0, pitch: 1.4, half_height: None };
        let square = |at: f32, half: f32, colour: [f32; 3]| {
            let vertices = [[at - half, -half, 0.0], [at + half, -half, 0.0], [at + half, half, 0.0], [at - half, half, 0.0]].iter().map(|&p| mao_formats::model::Vertex { position: [p[1], p[0], p[2]], normal: [0.0, 0.0, 1.0], uv: [0.0, 0.0], colour: mao_formats::model::Vertex::UNBAKED }).collect();
            mao_formats::model::Part { name: String::new(), world: Mat4::IDENTITY.to_cols_array(), vertices, indices: vec![0, 2, 1, 0, 3, 2], material: mao_formats::model::Material { diffuse: colour, opacity: 1.0, two_sided: true, ..Default::default() } }
        };
        let model = |shift: f32| Model { parts: vec![square(-8.0 + shift, 2.0, [1.0, 0.0, 0.0]), square(8.0 + shift, 2.0, [0.0, 0.0, 1.0])], ..Default::default() };
        let mut renderer = Renderer::new(&gpu, wgpu::TextureFormat::Rgba8Unorm);
        let mut scene = renderer.upload(&gpu, &Model::default(), &mut |_, _| None);
        renderer.reupload_overlay(&gpu, &mut scene, &model(0.0), &mut |_, _| None);
        assert_eq!(scene.overlay.len(), 2);
        assert_eq!((scene.overlay[0].base_vertex, scene.overlay[0].first_index, scene.overlay[1].base_vertex, scene.overlay[1].first_index), (0, 0, 4, 6));
        assert!(scene.overlay[0].vertices == scene.overlay[1].vertices && scene.overlay[0].indices == scene.overlay[1].indices, "one pair of buffers");
        assert!(scene.overlay[0].vertex_id == scene.overlay[1].vertex_id && scene.overlay[0].geometry != scene.overlay[1].geometry);
        let (first_indices, first_vertices) = (scene.overlay[0].indices.clone(), scene.overlay[0].vertices.clone());
        // (both colours are in the picture, apart: left and right of the middle or above and below it)
        let count = |image: &[u8], red: bool| image.chunks(4).filter(|p| if red { p[0] > 2 * p[1].max(p[2]) && p[0] > 40 } else { p[2] > 2 * p[0].max(p[1]) && p[2] > 40 }).count();
        let image = renderer.render_to_image(&gpu, &scene, &camera, 160, 120);
        assert!(count(&image, true) > 30 && count(&image, false) > 30, "{} red, {} blue", count(&image, true), count(&image, false));
        // (moved: the same buffers, the vertices written anew)
        renderer.reupload_overlay(&gpu, &mut scene, &model(1.0), &mut |_, _| None);
        assert!(scene.overlay[0].vertices == first_vertices && scene.overlay[1].indices == first_indices);
        let moved = renderer.render_to_image(&gpu, &scene, &camera, 160, 120);
        assert!(count(&moved, true) > 30 && count(&moved, false) > 30 && moved != image);
        // (a playfield's parts are packed the same way: two meshes, one pair of buffers, and the picture as before)
        let packed = renderer.upload(&gpu, &model(0.0), &mut |_, _| None);
        assert!(packed.parts[0].vertices == packed.parts[1].vertices && packed.parts[0].index_id == packed.parts[1].index_id);
        assert_eq!((packed.parts[1].base_vertex, packed.parts[1].first_index), (4, 6));
        let again = renderer.render_to_image(&gpu, &packed, &camera, 160, 120);
        assert_eq!((count(&again, true), count(&again, false)), (count(&image, true), count(&image, false)));
    }

    #[test]
    fn the_screen_centre_looks_at_the_target() {
        let c = Camera { target: Vec3::new(10.0, 5.0, 1.0), distance: 8.0, yaw: 0.7, pitch: 0.3, half_height: None };
        let (from, dir) = c.ray(0.5, 0.5, 16.0 / 9.0, 100.0);
        let want = (c.target - c.eye()).normalize();
        assert!(dir.dot(want) > 0.9999, "{dir:?} {want:?}");
        assert!((from - c.eye()).length() < 0.5);
        // (and a point projects back to where its ray starts)
        let p = Vec3::new(12.0, 6.0, 0.5);
        let [x, y] = c.project(p, 16.0 / 9.0, 100.0).unwrap();
        let (from, dir) = c.ray(x, y, 16.0 / 9.0, 100.0);
        assert!(dir.dot((p - from).normalize()) > 0.9999);
    }

    /// A flat ground 200 m across with a pillar 2 m wide and 12 m tall standing on it at the origin (Z up).
    fn pillar_scene() -> Model {
        let quad = |c: [[f32; 3]; 4], n: [f32; 3]| -> (Vec<mao_formats::model::Vertex>, Vec<u16>) {
            (c.iter().map(|&p| mao_formats::model::Vertex { position: p, normal: n, uv: [0.0, 0.0], colour: mao_formats::model::Vertex::UNBAKED }).collect(), vec![0, 1, 2, 0, 2, 3])
        };
        let mut parts = Vec::new();
        let identity: [f32; 16] = Mat4::IDENTITY.to_cols_array();
        let mut add = |(vertices, indices): (Vec<mao_formats::model::Vertex>, Vec<u16>)| {
            parts.push(mao_formats::model::Part { name: String::new(), world: identity, vertices, indices, material: mao_formats::model::Material { diffuse: [0.8, 0.8, 0.8], opacity: 1.0, ..Default::default() } });
        };
        add(quad([[-100.0, -100.0, 0.0], [100.0, -100.0, 0.0], [100.0, 100.0, 0.0], [-100.0, 100.0, 0.0]], [0.0, 0.0, 1.0]));
        let (a, b, h) = (-1.0f32, 1.0f32, 12.0f32);
        add(quad([[a, a, 0.0], [b, a, 0.0], [b, a, h], [a, a, h]], [0.0, -1.0, 0.0]));
        add(quad([[b, a, 0.0], [b, b, 0.0], [b, b, h], [b, a, h]], [1.0, 0.0, 0.0]));
        add(quad([[b, b, 0.0], [a, b, 0.0], [a, b, h], [b, b, h]], [0.0, 1.0, 0.0]));
        add(quad([[a, b, 0.0], [a, a, 0.0], [a, a, h], [a, b, h]], [-1.0, 0.0, 0.0]));
        add(quad([[a, a, h], [b, a, h], [b, b, h], [a, b, h]], [0.0, 0.0, 1.0]));
        Model { parts, ..Default::default() }
    }

    #[test]
    fn a_kept_shadow_map_draws_the_same_shadows() {
        // (skipped on a machine without a graphics device)
        let Ok(gpu) = Gpu::headless() else { return };
        let format = wgpu::TextureFormat::Rgba8Unorm;
        let camera = Camera { target: Vec3::new(-6.0, 0.0, 0.0), distance: 40.0, yaw: 0.3, pitch: 1.2, half_height: None };
        let (start, moved) = (Vec3::new(0.0, 6.0, 0.0), Vec3::new(5.0, 6.0, 0.0));
        let picture = |renderer: &mut Renderer, scene: &mut Scene, at: Vec3| {
            scene.player = Some(at);
            renderer.render_to_image(&gpu, scene, &camera, 160, 120)
        };
        let mut setup = || {
            let mut renderer = Renderer::new(&gpu, format);
            let mut scene = renderer.upload(&gpu, &pillar_scene(), &mut |_, _| None);
            scene.sun = Some(Vec3::new(1.0, 0.0, 0.7));
            (renderer, scene)
        };
        // drawn afresh where the player stands now, and kept from 5 m back (under `SHADOW_MOVE`)
        let (mut fresh, mut fresh_scene) = setup();
        let want = picture(&mut fresh, &mut fresh_scene, moved);
        let (mut kept, mut kept_scene) = setup();
        picture(&mut kept, &mut kept_scene, start);
        let got = picture(&mut kept, &mut kept_scene, moved);
        assert_eq!(kept.shadow_cache.map(|c| c.focus), Some(start), "the map was kept");
        let differ = |a: &[u8], b: &[u8]| a.chunks(4).zip(b.chunks(4)).filter(|(p, q)| p.iter().zip(q.iter()).any(|(x, y)| x.abs_diff(*y) > 24)).count();
        let pixels = want.len() / 4;
        assert!(differ(&want, &got) * 200 < pixels, "{} of {pixels} pixels differ", differ(&want, &got));
        // (and there is a shadow to compare: without shadows the picture changes)
        let (mut dark, mut dark_scene) = setup();
        dark.shadows = false;
        let unlit = picture(&mut dark, &mut dark_scene, moved);
        if let Some(dir) = std::env::var_os("MAO_TEST_DUMP") {
            for (name, img) in [("fresh", &want), ("kept", &got), ("unlit", &unlit)] {
                let mut ppm = b"P6 160 120 255
".to_vec();
                ppm.extend(img.chunks(4).flat_map(|p| [p[0], p[1], p[2]]));
                let _ = std::fs::write(std::path::Path::new(&dir).join(format!("{name}.ppm")), ppm);
            }
        }
        assert!(differ(&want, &unlit) * 400 > pixels, "the pillar throws a shadow ({} pixels differ)", differ(&want, &unlit));
    }

    #[test]
    fn the_jitter_spreads_over_the_pixel() {
        assert_eq!((halton(1, 2), halton(2, 2), halton(3, 2)), (0.5, 0.25, 0.75));
        assert!((halton(1, 3) - 1.0 / 3.0).abs() < 1e-6 && (halton(2, 3) - 2.0 / 3.0).abs() < 1e-6);
        let mean: f32 = (1..=16).map(|i| halton(i, 2) - 0.5).sum::<f32>() / 16.0;
        assert!(mean.abs() < 0.04, "{mean}");
    }

    #[test]
    fn temporal_antialiasing_settles_and_follows_the_camera() {
        // (skipped on a machine without a graphics device)
        let Ok(gpu) = Gpu::headless() else { return };
        let format = wgpu::TextureFormat::Rgba8Unorm;
        let (w, h) = (160, 120);
        let still = Camera { target: Vec3::new(0.0, 0.0, 4.0), distance: 30.0, yaw: 0.3, pitch: 0.4, half_height: None };
        let turned = Camera { yaw: 0.55, ..still };
        let mut renderer = Renderer::new(&gpu, format);
        let mut scene = renderer.upload(&gpu, &pillar_scene(), &mut |_, _| None);
        scene.player = Some(Vec3::new(0.0, 6.0, 0.0));
        let msaa_still = renderer.render_to_image(&gpu, &scene, &still, w, h);
        let msaa_turned = renderer.render_to_image(&gpu, &scene, &turned, w, h);
        renderer.antialiasing = Antialiasing::Taa;
        let mut settled = Vec::new();
        for _ in 0..16 {
            settled = renderer.render_to_image(&gpu, &scene, &still, w, h);
        }
        let differ = |a: &[u8], b: &[u8], by: u8| a.chunks(4).zip(b.chunks(4)).filter(|(p, q)| p.iter().zip(q.iter()).take(3).any(|(x, y)| x.abs_diff(*y) > by)).count();
        let pixels = (w * h) as usize;
        // and when the camera turns, the history follows it: no ghost of the pillar where it was
        let moved = renderer.render_to_image(&gpu, &scene, &turned, w, h);
        let mut later = Vec::new();
        for _ in 0..4 {
            later = renderer.render_to_image(&gpu, &scene, &turned, w, h);
        }
        if let Some(dir) = std::env::var_os("MAO_TEST_DUMP") {
            for (name, img) in [("taa_settled", &settled), ("msaa_still", &msaa_still), ("taa_moved", &moved), ("taa_later", &later), ("msaa_turned", &msaa_turned)] {
                let mut ppm = format!("P6 {w} {h} 255\n").into_bytes();
                ppm.extend(img.chunks(4).flat_map(|p| [p[0], p[1], p[2]]));
                let _ = std::fs::write(std::path::Path::new(&dir).join(format!("{name}.ppm")), ppm);
            }
        }
        // a still view comes to the multisampled picture (its edges are smoothed either way)
        assert!(differ(&settled, &msaa_still, 40) * 100 < pixels, "{} of {pixels} pixels differ from the multisampled frame", differ(&settled, &msaa_still, 40));
        assert!(differ(&moved, &msaa_turned, 60) * 50 < pixels, "{} of {pixels} pixels differ after the turn", differ(&moved, &msaa_turned, 60));
        assert!(differ(&later, &msaa_turned, 40) * 100 < pixels, "{} of {pixels} pixels differ a few frames on", differ(&later, &msaa_turned, 40));
        // (and back to multisampling draws as before)
        renderer.antialiasing = Antialiasing::Msaa;
        assert_eq!(renderer.render_to_image(&gpu, &scene, &still, w, h), msaa_still);
    }

    #[test]
    fn mip_chain_halves_down_to_one_pixel() {
        // 4x2: levels 4x2, 2x1, 1x1.
        let t = Texture { width: 4, height: 2, format: mao_formats::texture::Format::Png, rgba: (0..32).map(|i| (i * 8) as u8).collect(), mips: 1 };
        let (data, levels) = mao_formats::texture::mip_chain(&t.rgba, t.width, t.height);
        assert_eq!(levels, 3);
        assert_eq!(data.len(), 32 + 2 * 4 + 4);
        // The 1x1 level is the average of the 2x1 level's two pixels.
        let two = &data[32..40];
        for c in 0..4 {
            assert_eq!(data[40 + c], ((two[c] as u32 + two[4 + c] as u32 + 1) / 2) as u8);
        }
        // A 1x1 texture has just its one level.
        let one = Texture { width: 1, height: 1, format: mao_formats::texture::Format::Png, rgba: vec![9, 9, 9, 9], mips: 1 };
        assert_eq!(mao_formats::texture::mip_chain(&one.rgba, 1, 1), (vec![9, 9, 9, 9], 1));
    }
}
