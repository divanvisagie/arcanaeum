//! The model preview drawn on the GPU, the way the game shades it (see ADR-0015).
//!
//! `Scene` is a loaded model flattened for drawing (vertex and index data plus the textures); the
//! worker builds it once per load. `Paint` is the egui paint callback that draws a scene: in `prepare`
//! it renders into a multisampled offscreen target sized to the picture, and in `paint` it blits the
//! result into egui's pass, so nothing is read back to the CPU. `GpuResources` (pipelines, the target,
//! the uploaded scene) lives in egui-wgpu's callback resources; `install` puts it there at start-up.
//!
//! `Offscreen` runs the same render pass and reads the picture back, for tests and for looking at
//! the result without a window.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use egui_wgpu::{CallbackTrait, ScreenDescriptor};
use skyplug_core::render::{Camera, Model, Texture};
use wgpu::util::DeviceExt;

const MSAA: u32 = 4;
const COLOUR: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// How far an underlay (the body) is pushed back, in game units, so a garment lying slightly inside it still shows.
const GARMENT_PUSH: f32 = 4.0;

const FLAG_NORMAL_MAP: f32 = 1.0;
const FLAG_SPECULAR: f32 = 2.0;
const FLAG_BROKEN: f32 = 4.0;
const FLAG_NO_DIFFUSE: f32 = 8.0;
const FLAG_BLEND: f32 = 16.0;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos: [f32; 3],
    nrm: [f32; 3],
    uv: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Globals {
    center: [f32; 4],
    right: [f32; 4],
    up: [f32; 4],
    toward: [f32; 4],
    scale: [f32; 4],
    key: [f32; 4],
    fill: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct PartUniform {
    spec: [f32; 4],
    misc: [f32; 4],
}

// ------------------------------------------------------------------ the scene (CPU side)

struct ScenePart {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    diffuse: Option<Arc<Texture>>,
    normal: Option<Arc<Texture>>,
    uniform: PartUniform,
    blend: bool,
    double_sided: bool,
    centroid: [f32; 3],
}

/// A model flattened for drawing. Built off the UI thread; uploaded to the GPU on first draw.
pub struct Scene {
    id: u64,
    parts: Vec<ScenePart>,
    center: [f32; 3],
    radius: f32,
}

impl std::fmt::Debug for Scene {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Scene({} parts)", self.parts.len())
    }
}

fn smooth_normals(positions: &[[f32; 3]], triangles: &[[u16; 3]]) -> Vec<[f32; 3]> {
    let mut n = vec![[0f32; 3]; positions.len()];
    for t in triangles {
        let [a, b, c] = [t[0] as usize, t[1] as usize, t[2] as usize];
        if a >= positions.len() || b >= positions.len() || c >= positions.len() {
            continue;
        }
        let (pa, pb, pc) = (positions[a], positions[b], positions[c]);
        let e1 = [pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]];
        let e2 = [pc[0] - pa[0], pc[1] - pa[1], pc[2] - pa[2]];
        let f = [e1[1] * e2[2] - e1[2] * e2[1], e1[2] * e2[0] - e1[0] * e2[2], e1[0] * e2[1] - e1[1] * e2[0]];
        for i in [a, b, c] {
            for k in 0..3 {
                n[i][k] += f[k];
            }
        }
    }
    for v in &mut n {
        let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if l > 1e-12 {
            *v = [v[0] / l, v[1] / l, v[2] / l];
        } else {
            *v = [0.0, 0.0, 1.0];
        }
    }
    n
}

impl Scene {
    /// Flatten `model` (already posed). Meshes with no triangles are dropped.
    pub fn from_model(model: &Model) -> Scene {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let mut parts = Vec::new();
        for p in model.parts() {
            let m = p.mesh;
            if m.triangles.is_empty() || m.positions.is_empty() {
                continue;
            }
            let normals = if m.normals.len() == m.positions.len() { m.normals.clone() } else { smooth_normals(&m.positions, &m.triangles) };
            let has_uv = m.uvs.len() == m.positions.len();
            let vertices: Vec<Vertex> = (0..m.positions.len())
                .map(|i| Vertex { pos: m.positions[i], nrm: normals[i], uv: if has_uv { m.uvs[i] } else { [0.5, 0.5] } })
                .collect();
            let indices = m.triangles.iter().flat_map(|t| [t[0] as u32, t[1] as u32, t[2] as u32]).collect();
            let (test, blend, threshold) = m.alpha.map_or((false, false, 0.0), |a| (a.test, a.blend, a.threshold as f32));
            let mut flags = 0.0;
            if p.normal_map.is_some() {
                flags += FLAG_NORMAL_MAP;
            }
            if p.diffuse_broken {
                flags += FLAG_BROKEN;
            } else if p.diffuse.is_none() {
                flags += FLAG_NO_DIFFUSE;
            }
            if blend {
                flags += FLAG_BLEND;
            }
            let (spec, gloss) = match m.specular {
                Some(s) => {
                    flags += FLAG_SPECULAR;
                    ([s.color[0], s.color[1], s.color[2], s.strength], s.glossiness)
                }
                None => ([0.0; 4], 1.0),
            };
            let n = m.positions.len() as f32;
            let centroid = m.positions.iter().fold([0f32; 3], |a, p| [a[0] + p[0] / n, a[1] + p[1] / n, a[2] + p[2] / n]);
            parts.push(ScenePart {
                vertices,
                indices,
                diffuse: p.diffuse,
                normal: p.normal_map,
                uniform: PartUniform { spec, misc: [gloss, if test { threshold } else { -1.0 }, if p.underlay { GARMENT_PUSH } else { 0.0 }, flags] },
                blend,
                double_sided: m.double_sided,
                centroid,
            });
        }
        let (lo, hi) = model.bounds().unwrap_or(([-1.0; 3], [1.0; 3]));
        let center = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0, (lo[2] + hi[2]) / 2.0];
        let d = [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]];
        let radius = ((d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt() / 2.0).max(1e-3);
        Scene { id: NEXT.fetch_add(1, Ordering::Relaxed), parts, center, radius }
    }
}

// ------------------------------------------------------------------ on the GPU

struct GpuPart {
    vbuf: wgpu::Buffer,
    ibuf: wgpu::Buffer,
    count: u32,
    bind: wgpu::BindGroup,
    blend: bool,
    double_sided: bool,
    centroid: [f32; 3],
}

struct Uploaded {
    id: u64,
    parts: Vec<GpuPart>,
}

struct Target {
    size: [u32; 2],
    msaa: wgpu::TextureView,
    resolve: wgpu::Texture,
    depth: wgpu::TextureView,
    blit_bind: wgpu::BindGroup,
}

/// Pipelines, the target and the uploaded scene; one per window.
pub struct GpuResources {
    globals_buf: wgpu::Buffer,
    globals_bind: wgpu::BindGroup,
    part_layout: wgpu::BindGroupLayout,
    blit_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    blit_sampler: wgpu::Sampler,
    blit_uniform: wgpu::Buffer,
    /// By `blend * 2 + double_sided`.
    pipelines: [wgpu::RenderPipeline; 4],
    blit: wgpu::RenderPipeline,
    white: wgpu::TextureView,
    uploaded: Option<Uploaded>,
    target: Option<Target>,
}

fn mip_chain(t: &Texture) -> Vec<(u32, u32, Vec<u8>)> {
    let (mut w, mut h) = (t.width.max(1), t.height.max(1));
    let mut data = t.pixels.clone();
    data.resize(w * h * 4, 0);
    let mut out = vec![(w as u32, h as u32, data.clone())];
    while w > 1 || h > 1 {
        let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
        let mut next = vec![0u8; nw * nh * 4];
        for y in 0..nh {
            for x in 0..nw {
                let mut acc = [0u32; 4];
                for (dy, dx) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
                    let (sx, sy) = ((x * 2 + dx).min(w - 1), (y * 2 + dy).min(h - 1));
                    for k in 0..4 {
                        acc[k] += data[(sy * w + sx) * 4 + k] as u32;
                    }
                }
                for k in 0..4 {
                    next[(y * nw + x) * 4 + k] = ((acc[k] + 2) / 4) as u8;
                }
            }
        }
        out.push((nw as u32, nh as u32, next.clone()));
        data = next;
        w = nw;
        h = nh;
    }
    out
}

fn upload_texture(device: &wgpu::Device, queue: &wgpu::Queue, t: &Texture, srgb: bool) -> wgpu::TextureView {
    let chain = mip_chain(t);
    let format = if srgb { wgpu::TextureFormat::Rgba8UnormSrgb } else { wgpu::TextureFormat::Rgba8Unorm };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("model texture"),
        size: wgpu::Extent3d { width: chain[0].0, height: chain[0].1, depth_or_array_layers: 1 },
        mip_level_count: chain.len() as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (level, (w, h, data)) in chain.iter().enumerate() {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: level as u32, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            data,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * w), rows_per_image: Some(*h) },
            wgpu::Extent3d { width: *w, height: *h, depth_or_array_layers: 1 },
        );
    }
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

fn make_target(device: &wgpu::Device, res: &GpuResources, size: [u32; 2]) -> Target {
    let extent = wgpu::Extent3d { width: size[0], height: size[1], depth_or_array_layers: 1 };
    let msaa = device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("preview msaa"),
            size: extent,
            mip_level_count: 1,
            sample_count: MSAA,
            dimension: wgpu::TextureDimension::D2,
            format: COLOUR,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default());
    let resolve = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("preview picture"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: COLOUR,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let depth = device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("preview depth"),
            size: extent,
            mip_level_count: 1,
            sample_count: MSAA,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default());
    let resolve_view = resolve.create_view(&wgpu::TextureViewDescriptor::default());
    let blit_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("preview blit"),
        layout: &res.blit_layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&resolve_view) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&res.blit_sampler) },
            wgpu::BindGroupEntry { binding: 2, resource: res.blit_uniform.as_entire_binding() },
        ],
    });
    Target { size, msaa, resolve, depth, blit_bind }
}

impl GpuResources {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, target_format: wgpu::TextureFormat) -> GpuResources {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("model shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gpu_model.wgsl").into()),
        });
        let uniform_entry = |binding, visibility| wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
            count: None,
        };
        let texture_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
            count: None,
        };
        let both = wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT;
        let globals_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("globals"), entries: &[uniform_entry(0, both)] });
        let part_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("part"),
            entries: &[
                uniform_entry(0, both),
                texture_entry(1),
                texture_entry(2),
                wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        let blit_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("blit"),
            entries: &[
                texture_entry(0),
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
                uniform_entry(2, wgpu::ShaderStages::FRAGMENT),
            ],
        });
        let globals_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let globals_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globals"),
            layout: &globals_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: globals_buf.as_entire_binding() }],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            anisotropy_clamp: 8,
            ..Default::default()
        });
        let blit_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let is_srgb = if target_format.is_srgb() { 1.0f32 } else { 0.0 };
        let blit_uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("blit uniform"),
            contents: bytemuck::cast_slice(&[is_srgb, 0.0f32, 0.0, 0.0]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let model_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("model"),
            bind_group_layouts: &[Some(&globals_layout), Some(&part_layout)],
            immediate_size: 0,
        });
        let attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2];
        let vertex_layout = wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<Vertex>() as u64, step_mode: wgpu::VertexStepMode::Vertex, attributes: &attrs };
        let make = |blend: bool, double_sided: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("model"),
                layout: Some(&model_layout),
                vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs"), compilation_options: Default::default(), buffers: &[vertex_layout.clone()] },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: if double_sided { None } else { Some(wgpu::Face::Back) },
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH,
                    depth_write_enabled: Some(!blend),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState { count: MSAA, mask: !0, alpha_to_coverage_enabled: false },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: COLOUR,
                        blend: if blend { Some(wgpu::BlendState::ALPHA_BLENDING) } else { None },
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let pipelines = [make(false, false), make(false, true), make(true, false), make(true, true)];
        let blit_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("blit"), bind_group_layouts: &[Some(&blit_layout)], immediate_size: 0 });
        let blit = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("blit"),
            layout: Some(&blit_pl),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("blit_vs"), compilation_options: Default::default(), buffers: &[] },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("blit_fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format: target_format, blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let white = upload_texture(device, queue, &Texture::solid([255, 255, 255, 255]), true);
        GpuResources { globals_buf, globals_bind, part_layout, blit_layout, sampler, blit_sampler, blit_uniform, pipelines, blit, white, uploaded: None, target: None }
    }

    fn ensure_scene(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, scene: &Scene) {
        if self.uploaded.as_ref().is_some_and(|u| u.id == scene.id) {
            return;
        }
        let mut cache: HashMap<(usize, bool), wgpu::TextureView> = HashMap::new();
        let mut parts = Vec::new();
        for p in &scene.parts {
            let mut view_of = |t: &Option<Arc<Texture>>, srgb: bool| -> wgpu::TextureView {
                match t {
                    Some(tex) => cache
                        .entry((Arc::as_ptr(tex) as usize, srgb))
                        .or_insert_with(|| upload_texture(device, queue, tex, srgb))
                        .clone(),
                    None => self.white.clone(),
                }
            };
            let diffuse = view_of(&p.diffuse, true);
            let normal = view_of(&p.normal, false);
            let ubuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("part"), contents: bytemuck::bytes_of(&p.uniform), usage: wgpu::BufferUsages::UNIFORM });
            let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("part"),
                layout: &self.part_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: ubuf.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&diffuse) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&normal) },
                    wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                ],
            });
            parts.push(GpuPart {
                vbuf: device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("vertices"), contents: bytemuck::cast_slice(&p.vertices), usage: wgpu::BufferUsages::VERTEX }),
                ibuf: device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("indices"), contents: bytemuck::cast_slice(&p.indices), usage: wgpu::BufferUsages::INDEX }),
                count: p.indices.len() as u32,
                bind,
                blend: p.blend,
                double_sided: p.double_sided,
                centroid: p.centroid,
            });
        }
        self.uploaded = Some(Uploaded { id: scene.id, parts });
    }

    fn ensure_target(&mut self, device: &wgpu::Device, size: [u32; 2]) {
        let size = [size[0].clamp(16, 4096), size[1].clamp(16, 4096)];
        if self.target.as_ref().is_some_and(|t| t.size == size) {
            return;
        }
        self.target = Some(make_target(device, self, size));
    }

    /// The camera's frame for `scene` at `size`, as the CPU preview lays it out.
    fn globals(scene: &Scene, camera: &Camera, size: [u32; 2]) -> Globals {
        let (az, el) = (camera.azimuth.to_radians(), camera.elevation.to_radians());
        let toward = [az.sin() * el.cos(), az.cos() * el.cos(), el.sin()];
        let f = [-toward[0], -toward[1], -toward[2]];
        let cross = |a: [f32; 3], b: [f32; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
        let norm = |a: [f32; 3]| {
            let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt().max(1e-12);
            [a[0] / l, a[1] / l, a[2] / l]
        };
        let right = norm(cross(f, [0.0, 0.0, 1.0]));
        let up = cross(right, f);
        let (w, h) = (size[0] as f32, size[1] as f32);
        let scale_px = w.min(h) / (2.0 * scene.radius) * 0.96 * camera.zoom.max(0.01);
        let mix = |a: [f32; 3], sa: f32, b: [f32; 3], sb: f32, c: [f32; 3], sc: f32| norm([a[0] * sa + b[0] * sb + c[0] * sc, a[1] * sa + b[1] * sb + c[1] * sc, a[2] * sa + b[2] * sb + c[2] * sc]);
        let key = mix(right, -0.35, up, 0.55, toward, 0.8);
        let fill = mix(right, 0.6, up, -0.15, toward, 0.35);
        let v4 = |v: [f32; 3]| [v[0], v[1], v[2], 0.0];
        Globals {
            center: v4(scene.center),
            right: v4(right),
            up: v4(up),
            toward: v4(toward),
            scale: [scale_px / (w / 2.0), scale_px / (h / 2.0), 1.0 / (8.0 * scene.radius), 0.0],
            key: v4(key),
            fill: v4(fill),
        }
    }

    /// Draw the uploaded scene into the target (multisampled, resolved into the picture).
    fn render(&self, encoder: &mut wgpu::CommandEncoder, queue: &wgpu::Queue, scene: &Scene, camera: &Camera) {
        let (Some(up), Some(target)) = (&self.uploaded, &self.target) else { return };
        queue.write_buffer(&self.globals_buf, 0, bytemuck::bytes_of(&Self::globals(scene, camera, target.size)));
        let resolve_view = target.resolve.create_view(&wgpu::TextureViewDescriptor::default());
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("model preview"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target.msaa,
                depth_slice: None,
                resolve_target: Some(&resolve_view),
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Discard },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &target.depth,
                depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Discard }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_bind_group(0, &self.globals_bind, &[]);
        let mut opaque: Vec<&GpuPart> = up.parts.iter().filter(|p| !p.blend).collect();
        let mut blended: Vec<&GpuPart> = up.parts.iter().filter(|p| p.blend).collect();
        let t = Self::globals(scene, camera, target.size).toward;
        let depth = |p: &GpuPart| (p.centroid[0] - scene.center[0]) * t[0] + (p.centroid[1] - scene.center[1]) * t[1] + (p.centroid[2] - scene.center[2]) * t[2];
        blended.sort_by(|a, b| depth(a).total_cmp(&depth(b)));
        opaque.extend(blended);
        for p in opaque {
            pass.set_pipeline(&self.pipelines[usize::from(p.blend) * 2 + usize::from(p.double_sided)]);
            pass.set_bind_group(1, &p.bind, &[]);
            pass.set_vertex_buffer(0, p.vbuf.slice(..));
            pass.set_index_buffer(p.ibuf.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..p.count, 0, 0..1);
        }
    }
}

/// Put the renderer where paint callbacks find it. Call once, with eframe's render state.
pub fn install(render_state: &egui_wgpu::RenderState) {
    let resources = GpuResources::new(&render_state.device, &render_state.queue, render_state.target_format);
    render_state.renderer.write().callback_resources.insert(resources);
}

/// The paint callback: draws `scene` from `camera` into the rect it is added for.
pub struct Paint {
    pub scene: Arc<Scene>,
    pub camera: Camera,
    /// The size of the rect on screen, in physical pixels.
    pub size_px: [u32; 2],
}

impl CallbackTrait for Paint {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen: &ScreenDescriptor,
        encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if let Some(res) = resources.get_mut::<GpuResources>() {
            res.ensure_scene(device, queue, &self.scene);
            res.ensure_target(device, self.size_px);
            res.render(encoder, queue, &self.scene, &self.camera);
        }
        Vec::new()
    }

    fn paint(&self, _info: egui::PaintCallbackInfo, pass: &mut wgpu::RenderPass<'static>, resources: &egui_wgpu::CallbackResources) {
        if let Some(res) = resources.get::<GpuResources>() {
            if let Some(target) = &res.target {
                pass.set_pipeline(&res.blit);
                pass.set_bind_group(0, &target.blit_bind, &[]);
                pass.draw(0..3, 0..1);
            }
        }
    }
}

/// Offscreen rendering with the same pipelines and render pass as the window: keeps the pipelines, the
/// uploaded scene and the target between pictures, so repeated pictures cost what frames cost.
/// For tests and for looking at the result without a window.
pub struct Offscreen {
    res: GpuResources,
}

impl Offscreen {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Offscreen {
        Offscreen { res: GpuResources::new(device, queue, wgpu::TextureFormat::Rgba8Unorm) }
    }

    /// `scene` from `camera` at `size`, read back: RGBA, straight alpha over `background`.
    pub fn image(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, scene: &Scene, camera: &Camera, size: [u32; 2], background: [u8; 3]) -> (u32, u32, Vec<u8>) {
        let res = &mut self.res;
        res.ensure_scene(device, queue, scene);
        res.ensure_target(device, size);
        let target_size = res.target.as_ref().expect("made above").size;
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("offscreen preview") });
        res.render(&mut encoder, queue, scene, camera);
        let (w, h) = (target_size[0], target_size[1]);
        let padded = (w * 4).div_ceil(256) * 256;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor { label: Some("readback"), size: (padded * h) as u64, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo { texture: &res.target.as_ref().expect("made above").resolve, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::TexelCopyBufferInfo { buffer: &buffer, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(padded), rows_per_image: Some(h) } },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        queue.submit([encoder.finish()]);
        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |r| r.expect("map the readback buffer"));
        device.poll(wgpu::PollType::wait_indefinitely()).expect("the GPU finishes");
        let mapped = slice.get_mapped_range();
        let mut out = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            let row = &mapped[(y * padded) as usize..(y * padded + w * 4) as usize];
            for px in row.chunks_exact(4) {
                // Premultiplied colour over the background.
                let a = px[3] as f32 / 255.0;
                for k in 0..3 {
                    out.push((px[k] as f32 + background[k] as f32 * (1.0 - a)).round().clamp(0.0, 255.0) as u8);
                }
                out.push(255);
            }
        }
        (w, h, out)
    }
}
