//! The `wgpu` renderer: Vulkan, Metal and Direct3D 12 through one code
//! path. It draws [`mao_formats::model::Model`]s: every vertex buffer in the
//! game's meshes is the same layout (position, normal, one texture pair),
//! so one pipeline covers them all.
//!
//! The pipeline works on any target view, so the same code draws to a window
//! and, headless, to a texture that can be saved ([`Gpu::headless`],
//! [`Renderer::render_to_image`]).

use std::collections::HashMap;

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
}

pub use mao_formats::model::Water;

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
    pub water: Option<Water>,
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

    fn view_proj(&self, aspect: f32, radius: f32) -> Mat4 {
        let eye = self.eye();
        let far = (self.distance + radius * 4.0).max(1.0);
        // (Not a fixed share of the far plane: on a big playfield that cut the ground within 8 units of the camera away.)
        let near = (far * 0.001).clamp(0.01, 0.2);
        perspective(0.9, aspect, near, far) * look_at(eye, self.target, Vec3::Z)
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
        let white = upload_texture(gpu, &Texture { width: 1, height: 1, format: mao_formats::texture::Format::Png, rgba: vec![255; 4] });
        Renderer { pipeline, sky_pipeline, shadow_pipeline, water_pipeline, shadow_view, shadow_bind, shadows: true, stats: Stats::default(), render_scale: 1.0, sharpness: 0.2, fsr_easu, fsr_rcas, fsr_layout, fsr_sampler, fsr: None, samples, msaa: None, started: std::time::Instant::now(), globals_layout, part_layout, sampler, white, format, depth: None, textures: Default::default() }
    }

    /// Puts a model on the GPU. `texture` finds a texture by (type, instance).
    /// Each part's geometry and texture are uploaded once however many times it is drawn.
    pub fn upload(&self, gpu: &Gpu, model: &Model, texture: &mut dyn FnMut(u32, u32) -> Option<Texture>) -> Scene {
        let draw_list = draws(model);
        let (center, radius) = world_matrix_bounds(model, &draw_list);
        let mut cache = self.textures.borrow_mut();
        let mut geometry: HashMap<usize, Option<(wgpu::Buffer, wgpu::Buffer, u32, Option<wgpu::TextureView>, f32, (Vec3, Vec3))>> = HashMap::new();
        let mut parts = Vec::new();
        for (index, world) in draw_list {
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
                let view = part.material.textures.first().and_then(|&(kind, instance)| cache.entry((kind, instance)).or_insert_with(|| texture(kind, instance).map(|t| upload_texture(gpu, &t))).clone());
                // A plant's height (in its own Y-up space) sets how it bends.
                let height = part.vertices.iter().map(|v| v.position[1]).fold(0.0f32, f32::max);
                let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
                for v in &part.vertices {
                    lo = lo.min(Vec3::from(v.position));
                    hi = hi.max(Vec3::from(v.position));
                }
                Some((vb, ib, part.indices.len() as u32, view, height, (lo, hi)))
            });
            let Some((vb, ib, count, view, height, (lo, hi))) = entry else { continue };
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
            let uniform = PartUniform { model: world, colour: [d[0], d[1], d[2], 1.0], flags: [view.is_some() as u32 as f32, material.ignore_alpha as u32 as f32, material.foliage as u32 as f32, *height] };
            let ub = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: bytemuck::bytes_of(&uniform), usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST });
            let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.part_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: ub.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(view.as_ref().unwrap_or(&self.white)) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                ],
            });
            parts.push(GpuPart { vertices: vb.clone(), indices: ib.clone(), index_count: *count, bind, center, radius, uniform_buffer: ub, uniform, shift: Vec3::ZERO });
        }
        Scene { parts, overlay: Vec::new(), follow: Vec::new(), center, radius, player: None, fog: (1.0e9, 2.0e9), water: model.water }
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
        let sun = Vec3::new(0.4, -0.5, 0.8).normalize();
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
        let water = scene.water;
        let planes = frustum(&camera.view_proj(aspect, scene.radius));
        let (mut parts, mut drawn, mut shadow_drawn) = (0usize, 0usize, 0usize);
        let globals = Globals {
            view_proj: camera.view_proj(aspect, scene.radius).to_cols_array(),
            light_vp: light_vp.to_cols_array(),
            params: [self.shadows as u32 as f32, texel, water.map_or(0.0, |w| w.level), water.is_some() as u32 as f32],
            rect: water.map_or([0.0; 4], |w| [w.min[0], w.min[1], w.max[0], w.max[1]]),
            light: [sun.x, sun.y, sun.z, 0.0],
            player: [player.x, player.y, player.z, self.started.elapsed().as_secs_f32()],
            eye: [eye.x, eye.y, eye.z, scene.fog.0],
            horizon: [0.66, 0.74, 0.84, scene.fog.1],
            zenith: [0.20, 0.38, 0.72, 0.0],
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
                // Anything that can throw a shadow into the box: within it horizontally, a little beyond for the slant.
                let reach = extent * 1.6;
                for part in scene.parts.iter().chain(&scene.overlay).chain(&scene.follow) {
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
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &globals_bind, &[]);
            pass.set_bind_group(2, &self.shadow_bind, &[]);
            for part in scene.parts.iter().chain(&scene.overlay).chain(&scene.follow) {
                parts += 1;
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
            if water.is_some() {
                pass.set_pipeline(&self.water_pipeline);
                pass.set_bind_group(0, &globals_bind, &[]);
                pass.draw(0..6, 0..1);
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
fn mip_chain(t: &Texture) -> (Vec<u8>, u32) {
    let mut data = t.rgba.clone();
    let (mut w, mut h) = (t.width as usize, t.height as usize);
    let mut level_start = 0;
    let mut levels = 1;
    while w > 1 || h > 1 {
        let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
        let mut next = Vec::with_capacity(nw * nh * 4);
        for y in 0..nh {
            for x in 0..nw {
                let mut sum = [0u32; 4];
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let (sx, sy) = ((x * 2 + dx).min(w - 1), (y * 2 + dy).min(h - 1));
                    let at = level_start + (sy * w + sx) * 4;
                    for c in 0..4 {
                        sum[c] += data[at + c] as u32;
                    }
                }
                next.extend(sum.map(|v| ((v + 2) / 4) as u8));
            }
        }
        level_start = data.len();
        data.extend_from_slice(&next);
        (w, h) = (nw, nh);
        levels += 1;
    }
    (data, levels)
}

fn upload_texture(gpu: &Gpu, t: &Texture) -> wgpu::TextureView {
    let (data, levels) = mip_chain(t);
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
    fn mip_chain_halves_down_to_one_pixel() {
        // 4x2: levels 4x2, 2x1, 1x1.
        let t = Texture { width: 4, height: 2, format: mao_formats::texture::Format::Png, rgba: (0..32).map(|i| (i * 8) as u8).collect() };
        let (data, levels) = mip_chain(&t);
        assert_eq!(levels, 3);
        assert_eq!(data.len(), 32 + 2 * 4 + 4);
        // The 1x1 level is the average of the 2x1 level's two pixels.
        let two = &data[32..40];
        for c in 0..4 {
            assert_eq!(data[40 + c], ((two[c] as u32 + two[4 + c] as u32 + 1) / 2) as u8);
        }
        // A 1x1 texture has just its one level.
        let one = Texture { width: 1, height: 1, format: mao_formats::texture::Format::Png, rgba: vec![9, 9, 9, 9] };
        assert_eq!(mip_chain(&one), (vec![9, 9, 9, 9], 1));
    }
}
