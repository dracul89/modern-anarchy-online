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
    light: [f32; 4],
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
}

/// A model on the GPU, with its bounds.
pub struct Scene {
    parts: Vec<GpuPart>,
    /// Parts drawn after the others and replaceable (see [`Scene::set_overlay`]).
    overlay: Vec<GpuPart>,
    pub center: Vec3,
    pub radius: f32,
}

impl Scene {
    /// Replaces the overlay with the parts of `other` (which keeps this
    /// scene's bounds).
    pub fn set_overlay(&mut self, other: Scene) {
        self.overlay = other.parts;
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

    fn view_proj(&self, aspect: f32, radius: f32) -> Mat4 {
        let direction = Vec3::new(self.yaw.cos() * self.pitch.cos(), self.yaw.sin() * self.pitch.cos(), self.pitch.sin());
        let eye = self.target + direction * self.distance;
        let far = (self.distance + radius * 4.0).max(1.0);
        let near = (far * 0.001).max(0.01);
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

/// A right-handed perspective projection to depth 0..1.
fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    let f = 1.0 / (fov_y * 0.5).tan();
    Mat4::from_cols_array(&[f / aspect, 0.0, 0.0, 0.0, 0.0, f, 0.0, 0.0, 0.0, 0.0, far / (near - far), -1.0, 0.0, 0.0, near * far / (near - far), 0.0])
}

pub struct Renderer {
    pipeline: wgpu::RenderPipeline,
    globals_layout: wgpu::BindGroupLayout,
    part_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    white: wgpu::TextureView,
    format: wgpu::TextureFormat,
    depth: Option<((u32, u32), wgpu::TextureView)>,
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
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("mesh"), bind_group_layouts: &[Some(&globals_layout), Some(&part_layout)], immediate_size: 0 });
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
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
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
            ..Default::default()
        });
        let white = upload_texture(gpu, &Texture { width: 1, height: 1, format: mao_formats::texture::Format::Png, rgba: vec![255; 4] });
        Renderer { pipeline, globals_layout, part_layout, sampler, white, format, depth: None }
    }

    /// Puts a model on the GPU. `texture` finds a texture by (type, instance).
    /// Each part's geometry and texture are uploaded once however many times it is drawn.
    pub fn upload(&self, gpu: &Gpu, model: &Model, texture: &mut dyn FnMut(u32, u32) -> Option<Texture>) -> Scene {
        let draw_list = draws(model);
        let (center, radius) = world_matrix_bounds(model, &draw_list);
        let mut cache: HashMap<(u32, u32), Option<wgpu::TextureView>> = HashMap::new();
        let mut geometry: HashMap<usize, Option<(wgpu::Buffer, wgpu::Buffer, u32, Option<wgpu::TextureView>)>> = HashMap::new();
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
                Some((vb, ib, part.indices.len() as u32, view))
            });
            let Some((vb, ib, count, view)) = entry else { continue };
            let d = model.parts[index].material.diffuse;
            let uniform = PartUniform { model: world, colour: [d[0], d[1], d[2], 1.0], flags: [view.is_some() as u32 as f32, 0.0, 0.0, 0.0] };
            let ub = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: bytemuck::bytes_of(&uniform), usage: wgpu::BufferUsages::UNIFORM });
            let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &self.part_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: ub.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(view.as_ref().unwrap_or(&self.white)) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                ],
            });
            parts.push(GpuPart { vertices: vb.clone(), indices: ib.clone(), index_count: *count, bind });
        }
        Scene { parts, overlay: Vec::new(), center, radius }
    }

    fn depth_view(&mut self, gpu: &Gpu, size: (u32, u32)) {
        if self.depth.as_ref().is_none_or(|(s, _)| *s != size) {
            let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("depth"),
                size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            self.depth = Some((size, texture.create_view(&wgpu::TextureViewDescriptor::default())));
        }
    }

    /// Draws the scene into `target`, of `size` pixels.
    pub fn draw(&mut self, gpu: &Gpu, scene: &Scene, camera: &Camera, target: &wgpu::TextureView, size: (u32, u32)) {
        let aspect = size.0 as f32 / size.1.max(1) as f32;
        let globals = Globals { view_proj: camera.view_proj(aspect, scene.radius).to_cols_array(), light: [0.4, -0.5, 0.8, 0.0] };
        let globals_buffer = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: bytemuck::bytes_of(&globals), usage: wgpu::BufferUsages::UNIFORM });
        let globals_bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.globals_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: globals_buffer.as_entire_binding() }],
        });
        self.depth_view(gpu, size);
        let depth = &self.depth.as_ref().unwrap().1;
        let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.08, g: 0.1, b: 0.14, a: 1.0 }), store: wgpu::StoreOp::Store },
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
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &globals_bind, &[]);
            for part in scene.parts.iter().chain(&scene.overlay) {
                pass.set_bind_group(1, &part.bind, &[]);
                pass.set_vertex_buffer(0, part.vertices.slice(..));
                pass.set_index_buffer(part.indices.slice(..), wgpu::IndexFormat::Uint16);
                pass.draw_indexed(0..part.index_count, 0, 0..1);
            }
        }
        gpu.queue.submit(Some(encoder.finish()));
    }

    /// Draws the scene to a `width` x `height` RGBA image (headless).
    pub fn render_to_image(&mut self, gpu: &Gpu, scene: &Scene, camera: &Camera, width: u32, height: u32) -> Vec<u8> {
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

fn upload_texture(gpu: &Gpu, t: &Texture) -> wgpu::TextureView {
    let texture = gpu.device.create_texture_with_data(
        &gpu.queue,
        &wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d { width: t.width, height: t.height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
        wgpu::util::TextureDataOrder::LayerMajor,
        &t.rgba,
    );
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}
