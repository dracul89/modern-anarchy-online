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
}

impl Gpu {
    /// A device for rendering to textures (or to `surface`, when given).
    pub fn new(instance: wgpu::Instance, surface: Option<&wgpu::Surface<'_>>) -> Result<Gpu, String> {
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: surface,
            force_fallback_adapter: false,
            ..Default::default()
        }))
        .map_err(|e| e.to_string())?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).map_err(|e| e.to_string())?;
        Ok(Gpu { instance, adapter, device, queue })
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
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct PartUniform {
    model: [f32; 16],
    colour: [f32; 4],
    flags: [f32; 4],
}

struct GpuPart {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    index_count: u32,
    bind: wgpu::BindGroup,
    /// A sphere around the part in the world (to skip it in the shadow pass).
    center: Vec3,
    radius: f32,
    /// The part's uniform and what was written in it (so the part can be moved without uploading it again).
    uniform_buffer: wgpu::Buffer,
    uniform: PartUniform,
    /// How far the part is moved from where it was uploaded.
    shift: Vec3,
    /// The model part this draw came from.
    source: usize,
    /// A low-detail part: drawn only when the mesh it stands in for is far (`Lod::low`), never on its own.
    low_only: bool,
    /// On the first part of a watched mesh's instance: the mesh's sphere and what the decision covers.
    lod: Option<Lod>,
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
    /// The water's triangles on the card (position and liquid type per vertex), when the playfield has them.
    water_mesh: Option<(wgpu::Buffer, u32)>,
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
    pub fn move_follow(&mut self, gpu: &Gpu, delta: Vec3) {
        for part in &mut self.follow {
            let mut u = part.uniform;
            let m = Mat4::from_translation(delta) * Mat4::from_cols_array(&u.model);
            u.model = m.to_cols_array();
            gpu.queue.write_buffer(&part.uniform_buffer, 0, bytemuck::bytes_of(&u));
            part.shift = delta;
        }
    }
}

/// An orbiting camera looking at a point, Z up.
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub target: Vec3,
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
}

impl Camera {
    /// A camera that sees the whole scene.
    pub fn fit(scene: &Scene) -> Camera {
        Camera { target: scene.center, distance: scene.radius * 2.0, yaw: 0.6, pitch: 0.35 }
    }

    /// A camera at `eye` looking at `target` (world, Z up).
    pub fn looking(eye: Vec3, target: Vec3) -> Camera {
        let d = eye - target;
        let distance = d.length().max(0.01);
        let pitch = (d.z / distance).clamp(-1.0, 1.0).asin();
        let yaw = if d.x.abs() + d.y.abs() > 1e-9 { d.y.atan2(d.x) } else { 0.0 };
        Camera { target, distance, yaw, pitch }
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
        let fov_y = 2.0 * (1.0 / aspect.max(0.1)).atan();
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

/// The shadow map's side in texels.
const SHADOW_SIZE: u32 = 4096;

pub struct Renderer {
    pipeline: wgpu::RenderPipeline,
    sky_pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    water_pipeline: wgpu::RenderPipeline,
    water_mesh_pipeline: wgpu::RenderPipeline,
    blob_pipeline: wgpu::RenderPipeline,
    particle_pipeline: wgpu::RenderPipeline,
    /// The shells, added and blended.
    shell_add: wgpu::RenderPipeline,
    shell_blend: wgpu::RenderPipeline,
    backdrop_pipelines: [wgpu::RenderPipeline; 3],
    /// The sprite sheets uploaded so far, by number, as the bind group the particle pass uses, and the plain white one dots use.
    sprites: HashMap<u16, wgpu::BindGroup>,
    sprite_layout: wgpu::BindGroupLayout,
    white_sprite: wgpu::BindGroup,
    shadow_view: wgpu::TextureView,
    shadow_bind: wgpu::BindGroup,
    /// Draw the sun's shadows (on by default).
    pub shadows: bool,
    /// What the last frame drew.
    pub stats: Stats,
    /// The share of the window's size the scene is drawn at (1 = native); less is upscaled with FSR 1.
    pub render_scale: f32,
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
    sampler: wgpu::Sampler,
    white: wgpu::TextureView,
    format: wgpu::TextureFormat,
    depth: Option<((u32, u32), wgpu::TextureView)>,
    /// The textures uploaded so far, by (type, instance), kept between uploads.
    textures: std::cell::RefCell<HashMap<(u32, u32), Option<wgpu::TextureView>>>,
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
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("mesh"), source: wgpu::ShaderSource::Wgsl(include_str!("mesh.wgsl").into()) });
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
                uniform(0, wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
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
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("mesh"), bind_group_layouts: &[Some(&globals_layout), Some(&part_layout), Some(&shadow_layout)], immediate_size: 0 });
        let depth_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("depth"), bind_group_layouts: &[Some(&globals_layout), Some(&part_layout)], immediate_size: 0 });
        let attributes = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mesh"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<GpuVertex>() as u64, step_mode: wgpu::VertexStepMode::Vertex, attributes: &attributes })],
            },
            primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleList, cull_mode: None, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState { format: DEPTH, depth_write_enabled: Some(true), depth_compare: Some(wgpu::CompareFunction::Less), stencil: Default::default(), bias: Default::default() }),
            multisample: wgpu::MultisampleState { count: samples, mask: !0, alpha_to_coverage_enabled: false },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
            }),
            multiview_mask: None,
            cache: None,
        });
        // The sky: one triangle behind everything, no depth.
        let sky_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("sky"), bind_group_layouts: &[Some(&globals_layout)], immediate_size: 0 });
        let sky_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sky"),
            layout: Some(&sky_layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_sky"), compilation_options: Default::default(), buffers: &[] },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState { format: DEPTH, depth_write_enabled: Some(false), depth_compare: Some(wgpu::CompareFunction::Always), stencil: Default::default(), bias: Default::default() }),
            multisample: wgpu::MultisampleState { count: samples, mask: !0, alpha_to_coverage_enabled: false },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_sky"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
            }),
            multiview_mask: None,
            cache: None,
        });
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
        // The water: a translucent sheet after everything else, no depth writes.
        let water_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("water"), bind_group_layouts: &[Some(&globals_layout)], immediate_size: 0 });
        let water_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("water"),
            layout: Some(&water_layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_water"), compilation_options: Default::default(), buffers: &[] },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState { format: DEPTH, depth_write_enabled: Some(false), depth_compare: Some(wgpu::CompareFunction::Less), stencil: Default::default(), bias: Default::default() }),
            multisample: wgpu::MultisampleState { count: samples, mask: !0, alpha_to_coverage_enabled: false },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_water"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })],
            }),
            multiview_mask: None,
            cache: None,
        });
        // The water as the playfield's own triangles (position and liquid type per vertex), the same look.
        let water_attributes = [wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 }, wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32, offset: 12, shader_location: 1 }];
        let water_mesh_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("water mesh"),
            layout: Some(&water_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_water_mesh"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout { array_stride: 16, step_mode: wgpu::VertexStepMode::Vertex, attributes: &water_attributes })],
            },
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState { format: DEPTH, depth_write_enabled: Some(false), depth_compare: Some(wgpu::CompareFunction::Less), stencil: Default::default(), bias: Default::default() }),
            multisample: wgpu::MultisampleState { count: samples, mask: !0, alpha_to_coverage_enabled: false },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_water"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })],
            }),
            multiview_mask: None,
            cache: None,
        });
        // The blob shadows: coloured triangles blended over the ground, pulled a little toward the camera so they sit on it.
        let blob_attributes = [wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 }, wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 12, shader_location: 1 }];
        let blob_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("blob"),
            layout: Some(&water_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_blob"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<BlobVertex>() as u64, step_mode: wgpu::VertexStepMode::Vertex, attributes: &blob_attributes })],
            },
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: wgpu::DepthBiasState { constant: -8, slope_scale: -2.0, clamp: 0.0 },
            }),
            multisample: wgpu::MultisampleState { count: samples, mask: !0, alpha_to_coverage_enabled: false },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_blob"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })],
            }),
            multiview_mask: None,
            cache: None,
        });
        // Sparks and glows: camera-facing sprites added to the scene, tested against its depth and not writing any.
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
        let particle_attributes = [
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 0, shader_location: 0 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 16, shader_location: 1 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 32, shader_location: 2 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 48, shader_location: 3 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 64, shader_location: 4 },
        ];
        let additive = wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add };
        let particle_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("particle"),
            layout: Some(&particle_layout),
            vertex: wgpu::VertexState {
                module: &particle_shader,
                entry_point: Some("vs_particle"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout { array_stride: 80, step_mode: wgpu::VertexStepMode::Instance, attributes: &particle_attributes })],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState { format: DEPTH, depth_write_enabled: Some(false), depth_compare: Some(wgpu::CompareFunction::Less), stencil: Default::default(), bias: Default::default() }),
            multisample: wgpu::MultisampleState { count: samples, mask: !0, alpha_to_coverage_enabled: false },
            fragment: Some(wgpu::FragmentState {
                module: &particle_shader,
                entry_point: Some("fs_particle"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format, blend: Some(wgpu::BlendState { color: additive, alpha: additive }), write_mask: wgpu::ColorWrites::ALL })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let shell_attributes = [
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 12, shader_location: 1 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 20, shader_location: 2 },
        ];
        let shell_like = |vertex: &str, entry: &str, blend: Option<wgpu::BlendState>, compare: wgpu::CompareFunction| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("shell"),
                layout: Some(&particle_layout),
                vertex: wgpu::VertexState {
                    module: &particle_shader,
                    entry_point: Some(vertex),
                    compilation_options: Default::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout { array_stride: 36, step_mode: wgpu::VertexStepMode::Vertex, attributes: &shell_attributes })],
                },
                primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
                depth_stencil: Some(wgpu::DepthStencilState { format: DEPTH, depth_write_enabled: Some(false), depth_compare: Some(compare), stencil: Default::default(), bias: Default::default() }),
                multisample: wgpu::MultisampleState { count: samples, mask: !0, alpha_to_coverage_enabled: false },
                fragment: Some(wgpu::FragmentState {
                    module: &particle_shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState { format, blend, write_mask: wgpu::ColorWrites::ALL })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let shell_pipeline = |entry: &str, blend: wgpu::BlendState| shell_like("vs_shell", entry, Some(blend), wgpu::CompareFunction::Less);
        let shell_add = shell_pipeline("fs_shell_add", wgpu::BlendState { color: additive, alpha: additive });
        let shell_blend = shell_pipeline("fs_shell_blend", wgpu::BlendState::ALPHA_BLENDING);
        let backdrop_pipelines = [
            shell_like("vs_backdrop", "fs_backdrop_opaque", None, wgpu::CompareFunction::Always),
            shell_like("vs_backdrop", "fs_shell_blend", Some(wgpu::BlendState::ALPHA_BLENDING), wgpu::CompareFunction::Always),
            shell_like("vs_backdrop", "fs_shell_add", Some(wgpu::BlendState { color: additive, alpha: additive }), wgpu::CompareFunction::Always),
        ];
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
        let (fsr_easu, fsr_rcas) = (fsr_pipeline("fs_easu"), fsr_pipeline("fs_rcas"));
        let fsr_sampler = device.create_sampler(&wgpu::SamplerDescriptor { mag_filter: wgpu::FilterMode::Nearest, min_filter: wgpu::FilterMode::Nearest, ..Default::default() });
        let white = upload_texture(gpu, &Texture { width: 1, height: 1, format: mao_formats::texture::Format::Png, rgba: vec![255; 4], mips: 1 });
        let white_sprite = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &sprite_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&white) }, wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) }],
        });
        Renderer { pipeline, sky_pipeline, shadow_pipeline, water_pipeline, water_mesh_pipeline, blob_pipeline, particle_pipeline, shell_add, shell_blend, backdrop_pipelines, sprites: HashMap::new(), sprite_layout, white_sprite, shadow_view, shadow_bind, shadows: true, stats: Stats::default(), render_scale: 1.0, sharpness: 0.2, fsr_easu, fsr_rcas, fsr_layout, fsr_sampler, fsr: None, samples, msaa: None, started: std::time::Instant::now(), globals_layout, part_layout, sampler, white, format, depth: None, textures: Default::default() }
    }

    /// Puts a model on the GPU. `texture` finds a texture by (type, instance).
    /// Each part's geometry and texture are uploaded once however many times it is drawn.
    /// Makes a texture the sprite sheet `id` of the particle pass.
    pub fn add_sprite(&mut self, gpu: &Gpu, id: u16, texture: &Texture) {
        let view = upload_texture(gpu, texture);
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
            let workers = std::thread::available_parallelism().map_or(2, |n| n.get()).min(8).min(decoded.len().max(1));
            std::thread::scope(|scope| {
                for _ in 0..workers {
                    scope.spawn(|| loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some((key, t)) = decoded.get(i) else { break };
                        let view = t.as_ref().map(|t| upload_texture(gpu, t));
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
        let mut geometry: HashMap<usize, Option<(wgpu::Buffer, wgpu::Buffer, u32, Option<wgpu::TextureView>, f32, (Vec3, Vec3))>> = HashMap::new();
        let mut parts: Vec<GpuPart> = Vec::new();
        for &(index, _) in &draw_list {
            let geo_started = std::time::Instant::now();
            let entry = geometry.entry(index).or_insert_with(|| {
                let part = &model.parts[index];
                if part.indices.is_empty() || part.vertices.is_empty() {
                    return None;
                }
                let vertices: Vec<GpuVertex> = part.vertices.iter().map(|v: &Vertex| GpuVertex { position: v.position, normal: v.normal, uv: v.uv }).collect();
                let vb = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: bytemuck::cast_slice(&vertices), usage: wgpu::BufferUsages::VERTEX });
                let mut indices = part.indices.clone();
                if indices.len() % 2 == 1 {
                    indices.push(0);
                }
                let ib = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: bytemuck::cast_slice(&indices), usage: wgpu::BufferUsages::INDEX });
                let view = part.material.textures.first().and_then(|&(kind, instance)| cache.entry((kind, instance)).or_insert_with(|| { let t0 = std::time::Instant::now(); let r = texture(kind, instance).map(|t| upload_texture(gpu, &t)); tex_time += t0.elapsed(); tex_count += 1; r }).clone());
                // A plant's height (in its own Y-up space) sets how it bends.
                let height = part.vertices.iter().map(|v| v.position[1]).fold(0.0f32, f32::max);
                let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
                for v in &part.vertices {
                    lo = lo.min(Vec3::from(v.position));
                    hi = hi.max(Vec3::from(v.position));
                }
                Some((vb, ib, part.indices.len() as u32, view, height, (lo, hi)))
            });
            geo_time += geo_started.elapsed();
            let _ = entry;
        }
        // Each draw's own uniform buffer and bind group: made on every core too (a playfield draws tens of thousands of props).
        let (part_layout, sampler, white) = (&self.part_layout, &self.sampler, &self.white);
        let geometry = &geometry;
        let build = |slice: &[(usize, [f32; 16])]| -> Vec<GpuPart> {
            slice
                .iter()
                .filter_map(|&(index, world)| {
                    let (vb, ib, count, view, height, (lo, hi)) = geometry.get(&index)?.as_ref()?;
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
                    let uniform = PartUniform { model: world, colour: [d[0], d[1], d[2], 1.0], flags: [view.is_some() as u32 as f32, (material.ignore_alpha as u32 + material.alpha_lights as u32) as f32, material.foliage as u32 as f32, *height] };
                    let ub = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: bytemuck::bytes_of(&uniform), usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST });
                    let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: None,
                        layout: part_layout,
                        entries: &[
                            wgpu::BindGroupEntry { binding: 0, resource: ub.as_entire_binding() },
                            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(view.as_ref().unwrap_or(white)) },
                            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(sampler) },
                        ],
                    });
                    Some(GpuPart { vertices: vb.clone(), indices: ib.clone(), index_count: *count, bind, center, radius, uniform_buffer: ub, uniform, shift: Vec3::ZERO, source: index, low_only: low_parts.contains(&index), lod: None })
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
        let water_mesh = model.water.as_ref().filter(|w| !w.triangles.is_empty()).map(|w| {
            let data: Vec<[f32; 4]> = w.triangles.iter().flat_map(|(points, kind)| points.iter().map(move |p| [p[0], p[1], p[2], *kind as f32])).collect();
            let buffer = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("water"), contents: bytemuck::cast_slice(&data), usage: wgpu::BufferUsages::VERTEX });
            (buffer, data.len() as u32)
        });
        Scene { parts, overlay: Vec::new(), follow: Vec::new(), center, radius, player: None, fog: (1.0e9, 2.0e9), sky: DAY_SKY, water: model.water.clone(), water_mesh, particles: Vec::new(), blobs: Vec::new(), shells: Vec::new(), backdrops: Vec::new(), sun: None, lighting: None, daylight: 1.0, ambient: [0.0; 3], area_fog: None }
    }

    fn depth_view(&mut self, gpu: &Gpu, size: (u32, u32)) {
        if self.depth.as_ref().is_none_or(|(s, _)| *s != size) {
            let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("depth"),
                size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: self.samples,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            self.depth = Some((size, texture.create_view(&wgpu::TextureViewDescriptor::default())));
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
        let aspect = size.0 as f32 / size.1.max(1) as f32;
        // Below native, the scene is drawn small and upscaled afterwards.
        let scale = self.render_scale.clamp(0.4, 1.0);
        let upscale = scale < 0.999;
        let render_size = if upscale { (((size.0 as f32 * scale).round() as u32).max(2), ((size.1 as f32 * scale).round() as u32).max(2)) } else { size };
        let eye = camera.eye();
        let player = scene.player.unwrap_or(Vec3::splat(1.0e9));
        // The sun's box: around the player (or the whole scene with none),
        // its edges snapped to texels so the shadows don't shimmer as it moves.
        // (the scripts' sun when it is given, held a little over the horizon so the shadow map stays above the ground)
        let sun = match scene.sun {
            Some(s) if s.length_squared() > 0.0 => {
                let s = s.normalize();
                Vec3::new(s.x, s.y, s.z.max(0.15)).normalize()
            }
            _ => Vec3::new(0.4, -0.5, 0.8).normalize(),
        };
        let (focus, extent) = match scene.player {
            Some(p) => (p, 70.0f32.min(scene.radius.max(1.0))),
            None => (scene.center, scene.radius.max(1.0)),
        };
        let depth_range = extent * 3.0 + scene.radius.min(400.0);
        let light_view = look_at(focus + sun * depth_range, focus, Vec3::Z);
        let texel = 2.0 * extent / SHADOW_SIZE as f32;
        let c = light_view.transform_point3(focus);
        let snap = Vec3::new((c.x / texel).round() * texel - c.x, (c.y / texel).round() * texel - c.y, 0.0);
        let light_vp = orthographic(extent, 0.1, depth_range * 2.0) * Mat4::from_translation(snap) * light_view;
        // (the flat sheet only where the playfield has no water triangles)
        let water = scene.water.as_ref().filter(|w| w.triangles.is_empty());
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
        let (fog_start, fog_end, fog_colour, area) = match scene.area_fog {
            Some((colour, density)) if density > 0.0 => {
                let (near, far) = (0.2f32, scene.fog.1);
                let end = if far - near > 5.0 { far - (far - near - 5.0) * density.min(1.0) } else { far };
                (near, end, colour, 1.0)
            }
            _ => (scene.fog.0, scene.fog.1, scene.sky.0, 0.0),
        };
        let globals = Globals {
            view_proj: camera.view_proj(aspect, scene.radius).to_cols_array(),
            light_vp: light_vp.to_cols_array(),
            params: [self.shadows as u32 as f32, texel, water.map_or(0.0, |w| w.level), water.is_some() as u32 as f32],
            rect: water.map_or([0.0; 4], |w| [w.min[0], w.min[1], w.max[0], w.max[1]]),
            light: [sun.x, sun.y, sun.z, 0.0],
            player: [player.x, player.y, player.z, self.started.elapsed().as_secs_f32()],
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
        self.depth_view(gpu, render_size);
        self.msaa_view(gpu, render_size);
        let fsr_views = if upscale { Some(self.fsr_targets(gpu, render_size, size)) } else { None };
        let scene_target: &wgpu::TextureView = fsr_views.as_ref().map_or(target, |v| &v.0);
        let depth = &self.depth.as_ref().unwrap().1;
        // Multisampled: draw into the MSAA target and resolve into `target`.
        let (colour_view, resolve) = match &self.msaa {
            Some((_, msaa)) if self.samples > 1 => (msaa, Some(scene_target)),
            _ => (scene_target, None),
        };
        let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
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
                // Anything that can throw a shadow into the box: within it horizontally, a little beyond for the slant. Only
                // the playfield itself: the characters and the machines (the overlay and the followers) throw the original's
                // blob shadows instead (`Scene::blobs`), and nothing else in the original throws one.
                let reach = extent * 1.6;
                for (k, part) in scene.parts.iter().enumerate() {
                    if !lod_draw[k] {
                        continue;
                    }
                    let d = part.center + part.shift - focus;
                    if d.truncate().length() > reach + part.radius {
                        continue;
                    }
                    shadow_drawn += 1;
                    pass.set_bind_group(1, &part.bind, &[]);
                    pass.set_vertex_buffer(0, part.vertices.slice(..));
                    pass.set_index_buffer(part.indices.slice(..), wgpu::IndexFormat::Uint16);
                    pass.draw_indexed(0..part.index_count, 0, 0..1);
                }
            }
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: colour_view,
                    depth_slice: None,
                    resolve_target: resolve,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.66, g: 0.74, b: 0.84, a: 1.0 }), store: if resolve.is_some() { wgpu::StoreOp::Discard } else { wgpu::StoreOp::Store } },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Discard }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.sky_pipeline);
            pass.set_bind_group(0, &globals_bind, &[]);
            pass.draw(0..3, 0..1);
            for b in scene.backdrops.iter().filter(|b| !b.vertices.is_empty()) {
                let buffer = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("backdrop"), contents: bytemuck::cast_slice(&b.vertices), usage: wgpu::BufferUsages::VERTEX });
                let k = match b.blend {
                    BackdropBlend::Opaque => 0,
                    BackdropBlend::Alpha => 1,
                    BackdropBlend::Add => 2,
                };
                pass.set_pipeline(&self.backdrop_pipelines[k]);
                pass.set_bind_group(0, &globals_bind, &[]);
                pass.set_bind_group(1, self.sprites.get(&b.sprite).unwrap_or(&self.white_sprite), &[]);
                pass.set_vertex_buffer(0, buffer.slice(..));
                pass.draw(0..b.vertices.len() as u32, 0..1);
            }
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &globals_bind, &[]);
            pass.set_bind_group(2, &self.shadow_bind, &[]);
            for (k, part) in scene.parts.iter().enumerate().chain(scene.overlay.iter().chain(&scene.follow).map(|p| (usize::MAX, p))) {
                parts += 1;
                if k != usize::MAX && !lod_draw[k] {
                    continue;
                }
                // Plants bend up to a couple of units out of their box.
                if !sphere_visible(&planes, part.center + part.shift, part.radius + 2.0) {
                    continue;
                }
                drawn += 1;
                pass.set_bind_group(1, &part.bind, &[]);
                pass.set_vertex_buffer(0, part.vertices.slice(..));
                pass.set_index_buffer(part.indices.slice(..), wgpu::IndexFormat::Uint16);
                pass.draw_indexed(0..part.index_count, 0, 0..1);
            }
            if !scene.blobs.is_empty() {
                let buffer = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("blobs"), contents: bytemuck::cast_slice(&scene.blobs), usage: wgpu::BufferUsages::VERTEX });
                pass.set_pipeline(&self.blob_pipeline);
                pass.set_bind_group(0, &globals_bind, &[]);
                pass.set_vertex_buffer(0, buffer.slice(..));
                pass.draw(0..scene.blobs.len() as u32, 0..1);
            }
            if let Some((buffer, count)) = &scene.water_mesh {
                pass.set_pipeline(&self.water_mesh_pipeline);
                pass.set_bind_group(0, &globals_bind, &[]);
                pass.set_vertex_buffer(0, buffer.slice(..));
                pass.draw(0..*count, 0..1);
            } else if water.is_some() {
                pass.set_pipeline(&self.water_pipeline);
                pass.set_bind_group(0, &globals_bind, &[]);
                pass.draw(0..6, 0..1);
            }
            for shell in scene.shells.iter().filter(|s| !s.vertices.is_empty()) {
                let buffer = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("shell"), contents: bytemuck::cast_slice(&shell.vertices), usage: wgpu::BufferUsages::VERTEX });
                pass.set_pipeline(if shell.additive { &self.shell_add } else { &self.shell_blend });
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
                pass.set_pipeline(&self.particle_pipeline);
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
        self.stats = Stats { parts, drawn, shadow_drawn };
        if let Some((scene_view, mid_view)) = &fsr_views {
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
        gpu.queue.submit(Some(encoder.finish()));
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

/// The texture's mip levels, smallest last, each half the size of the one before
/// (a box filter): all the bytes in order, and how many levels.
fn upload_texture(gpu: &Gpu, t: &Texture) -> wgpu::TextureView {
    let (data, levels) = if t.mips > 1 { (t.rgba.clone(), t.mips) } else { mao_formats::texture::mip_chain(&t.rgba, t.width, t.height) };
    let texture = gpu.device.create_texture_with_data(
        &gpu.queue,
        &wgpu::TextureDescriptor {
            label: None,
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

    #[test]
    fn the_screen_centre_looks_at_the_target() {
        let c = Camera { target: Vec3::new(10.0, 5.0, 1.0), distance: 8.0, yaw: 0.7, pitch: 0.3 };
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
