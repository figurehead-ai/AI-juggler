//! The GPU side: buffers, pipelines and the four passes.
//!
//! 1. The sun's shadow map (one light direction for the whole office).
//! 2. The scene at the pixel size, writing colour, depth and a pick id per
//!    pixel; glass after, blended.
//! 3. The post pass: the small frame scaled up with hard edges, outlines
//!    where one thing sits in front of another, the hovered thing outlined
//!    in light, the sky, the red-alert wash.
//! 4. The overlay: crisp screen-space chips, balloons and the tooltip.

use std::collections::HashMap;
use std::sync::mpsc::channel;

use juggler_floor::camera::{FAR, NEAR, View};
use juggler_floor::layout::DEPTH;
use juggler_floor::math::{M4, add, norm, scale};
use juggler_floor::mesh::{Library, Mesh, Vertex};
use juggler_floor::models::{Inst, Item};
use juggler_floor::overlay::Quad;
use juggler_floor::paint::Img;
use juggler_floor::scene::{Light, Scene};
use wgpu::util::DeviceExt;

use crate::shaders::{OVERLAY_WGSL, POST_WGSL, scene_wgsl};

/// The most moving parts a frame draws.
pub const MAX_DYN: usize = 60_000;
const MAX_LIGHTS: usize = 96;
/// The most overlay quads a frame draws.
pub const MAX_QUADS: usize = 8192;
const SHADOW: u32 = 2048;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Globals {
    vp: [[f32; 4]; 4],
    lvp: [[f32; 4]; 4],
    eye: [f32; 4],
    sun: [f32; 4],
    sky: [f32; 4],
    ground: [f32; 4],
    params: [f32; 4],
    /// The sun's colour.
    sunc: [f32; 4],
    /// `(rework, night, rim, dust)` (0.4.2's look; all 0 in the classic).
    look: [f32; 4],
    /// `(materials on, their footprint's scale, the sun's banding,
    /// highlights)` (0.4.2's second pass; all 0 without it).
    mat: [f32; 4],
    /// `(the atlas's v scale, 0, 0, 0)`: the scene's `atlas_v`.
    atl: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct PostU {
    a: [f32; 4],
    b: [f32; 4],
    c: [f32; 4],
    top: [f32; 4],
    bot: [f32; 4],
    hov: [u32; 4],
    /// `(ambient occlusion, grade, night, 0)` (0.4.2's look).
    look: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuLight {
    p: [f32; 4],
    c: [f32; 4],
}

/// A run of instances of one mesh on one floor.
#[derive(Clone, Copy, Debug)]
struct Batch {
    glass: bool,
    floor: i32,
    mesh: Mesh,
    start: u32,
    count: u32,
}

/// The frame's size-dependent targets.
struct Targets {
    w: u32,
    h: u32,
    color: wgpu::Texture,
    ids: wgpu::Texture,
    depth: wgpu::Texture,
    post_group: wgpu::BindGroup,
}

/// The pixel size: one scene pixel is this many device pixels.
#[must_use]
pub fn pixel_size(scale: f32) -> f32 {
    juggler_floor::overlay::unit(scale)
}

/// The scene's pixel in device pixels for a mode (D-80): Pixel art's
/// `pixel_size` (a finer one with 0.4.2's `fine`: two device pixels on a
/// Retina screen instead of three; never under two), or one in Full
/// resolution.
fn scene_px(full: bool, fine: bool, scale: f32) -> f32 {
    if full {
        1.0
    } else if fine {
        (pixel_size(scale) - 1.0).max(2.0)
    } else {
        pixel_size(scale)
    }
}

/// The outlines' width in the scene's pixels: one in Pixel art (one big
/// pixel); in Full resolution about half Pixel art's unit, so they read
/// at the same weight (D-80).
fn outline_width(full: bool, scale: f32) -> f32 {
    if full {
        (pixel_size(scale) * 0.5).round().max(1.0)
    } else {
        1.0
    }
}

/// The hovered thing's ring, in the scene's pixels: as wide on screen in
/// both modes (one big pixel in Pixel art), so it reads as plainly.
fn hover_width(full: bool, scale: f32) -> f32 {
    if full {
        pixel_size(scale).round().max(1.0)
    } else {
        1.0
    }
}

/// The post pass's uniforms: the targets' size and pixel, the clock and
/// the alert, the window, the mode's outline and hover widths (D-80).
fn post_uniform(
    full: bool,
    look: &crate::Look,
    frame: &Frame<'_>,
    scene_px: (u32, u32, f32),
    alert: f32,
) -> PostU {
    let (w, h, sf) = frame.size;
    let (lw, lh, px) = scene_px;
    PostU {
        a: [lw as f32, lh as f32, px, outline_width(full, sf)],
        b: [frame.t, alert, NEAR, FAR],
        c: [
            w as f32,
            h as f32,
            if full { 1.0 } else { 0.0 },
            hover_width(full, sf),
        ],
        top: rgb1(look.sky.top),
        bot: rgb1(look.sky.bottom),
        hov: [frame.hover, 0, 0, 0],
        look: if look.rework {
            // the focal length in scene pixels, for an occlusion radius
            // that's the same size in the room at every zoom
            let focal = lh as f32 * 0.5 / (juggler_floor::camera::FOV * 0.5).tan();
            [1.0, 1.0, look.sky.night, focal]
        } else {
            [0.0; 4]
        },
    }
}

fn rgb1(c: [f32; 3]) -> [f32; 4] {
    [c[0], c[1], c[2], 1.0]
}

/// Everything the GPU holds.
pub struct Gpu {
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    pub(crate) format: wgpu::TextureFormat,
    lib_v: wgpu::Buffer,
    lib_i: wgpu::Buffer,
    ranges: HashMap<Mesh, (u32, u32)>,
    stat_buf: wgpu::Buffer,
    stat: Vec<Batch>,
    dyn_buf: wgpu::Buffer,
    globals: wgpu::Buffer,
    lights: wgpu::Buffer,
    scene_layout: wgpu::BindGroupLayout,
    scene_group: wgpu::BindGroup,
    shadow_group: wgpu::BindGroup,
    shadow_view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    shadow_sampler: wgpu::Sampler,
    p_scene: wgpu::RenderPipeline,
    p_glass: wgpu::RenderPipeline,
    p_shadow: wgpu::RenderPipeline,
    p_post: wgpu::RenderPipeline,
    p_over: wgpu::RenderPipeline,
    post_u: wgpu::Buffer,
    post_layout: wgpu::BindGroupLayout,
    over_u: wgpu::Buffer,
    over_g: wgpu::BindGroup,
    tex_layout: wgpu::BindGroupLayout,
    ui_group: wgpu::BindGroup,
    /// The scene's atlas, kept to rebind beside the materials.
    atlas: wgpu::TextureView,
    /// The materials' maps (a 1 × 1 stand-in until the second pass is
    /// first drawn: they're made only then) and their sampler.
    mats: wgpu::TextureView,
    mats_made: bool,
    mat_sampler: wgpu::Sampler,
    quads: wgpu::Buffer,
    pick_buf: wgpu::Buffer,
    tg: Option<Targets>,
    /// D-80: Full resolution (the scene at device pixels, smoothed)
    /// instead of Pixel art (at `pixel_size`, scaled up).
    pub(crate) full: bool,
    /// The light and whether 0.4.2's reworked look is on.
    pub(crate) look: crate::Look,
}

fn upload(device: &wgpu::Device, queue: &wgpu::Queue, img: &Img) -> wgpu::TextureView {
    let size = wgpu::Extent3d {
        width: u32::try_from(img.w).unwrap_or(1),
        height: u32::try_from(img.h).unwrap_or(1),
        depth_or_array_layers: 1,
    };
    device
        .create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("atlas"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &img.px,
        )
        .create_view(&wgpu::TextureViewDescriptor::default())
}

/// Sorts parts into runs of one mesh on one floor (glass last).
fn batch(items: &mut [Item]) -> (Vec<Inst>, Vec<Batch>) {
    items.sort_by_key(|it| (it.glass, it.floor, it.mesh.index()));
    let mut out = Vec::with_capacity(items.len());
    let mut batches: Vec<Batch> = Vec::new();
    for it in items.iter() {
        match batches.last_mut() {
            Some(b) if b.glass == it.glass && b.floor == it.floor && b.mesh == it.mesh => {
                b.count += 1;
            }
            _ => batches.push(Batch {
                glass: it.glass,
                floor: it.floor,
                mesh: it.mesh,
                start: u32::try_from(out.len()).unwrap_or(u32::MAX),
                count: 1,
            }),
        }
        out.push(it.inst);
    }
    (out, batches)
}

fn uniform(device: &wgpu::Device, size: usize) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: size as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

struct Layouts {
    scene: wgpu::BindGroupLayout,
    post: wgpu::BindGroupLayout,
    over: wgpu::BindGroupLayout,
    tex: wgpu::BindGroupLayout,
    shadow: wgpu::BindGroupLayout,
}

fn layouts(device: &wgpu::Device) -> Layouts {
    let e = |binding: u32, visibility: wgpu::ShaderStages, ty: wgpu::BindingType| {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility,
            ty,
            count: None,
        }
    };
    let vf = wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT;
    let fr = wgpu::ShaderStages::FRAGMENT;
    let ubuf = wgpu::BindingType::Buffer {
        ty: wgpu::BufferBindingType::Uniform,
        has_dynamic_offset: false,
        min_binding_size: None,
    };
    let tex2d = |sample_type| wgpu::BindingType::Texture {
        sample_type,
        view_dimension: wgpu::TextureViewDimension::D2,
        multisampled: false,
    };
    let layout = |entries: &[wgpu::BindGroupLayoutEntry]| {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries,
        })
    };
    Layouts {
        scene: layout(&[
            e(0, vf, ubuf),
            e(
                1,
                fr,
                wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            ),
            e(
                2,
                fr,
                tex2d(wgpu::TextureSampleType::Float { filterable: true }),
            ),
            e(
                3,
                fr,
                wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            ),
            e(4, fr, tex2d(wgpu::TextureSampleType::Depth)),
            e(
                5,
                fr,
                wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
            ),
            e(
                6,
                fr,
                wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2Array,
                    multisampled: false,
                },
            ),
            e(
                7,
                fr,
                wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            ),
        ]),
        post: layout(&[
            e(0, fr, ubuf),
            e(
                1,
                fr,
                tex2d(wgpu::TextureSampleType::Float { filterable: false }),
            ),
            e(2, fr, tex2d(wgpu::TextureSampleType::Uint)),
            e(3, fr, tex2d(wgpu::TextureSampleType::Depth)),
        ]),
        over: layout(&[e(0, wgpu::ShaderStages::VERTEX, ubuf)]),
        tex: layout(&[
            e(
                0,
                fr,
                tex2d(wgpu::TextureSampleType::Float { filterable: true }),
            ),
            e(
                1,
                fr,
                wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            ),
        ]),
        shadow: layout(&[e(0, vf, ubuf)]),
    }
}

struct Pipelines {
    scene: wgpu::RenderPipeline,
    glass: wgpu::RenderPipeline,
    shadow: wgpu::RenderPipeline,
    post: wgpu::RenderPipeline,
    over: wgpu::RenderPipeline,
}

#[allow(clippy::too_many_lines)] // the five pipelines, described in full
fn pipelines(device: &wgpu::Device, l: &Layouts, format: wgpu::TextureFormat) -> Pipelines {
    let module = |label: &str, src: &str| {
        device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(label),
            source: wgpu::ShaderSource::Wgsl(src.into()),
        })
    };
    let (scene_mod, post_mod, over_mod) = (
        module("scene", &scene_wgsl()),
        module("post", POST_WGSL),
        module("overlay", OVERLAY_WGSL),
    );
    let pl = |groups: &[Option<&wgpu::BindGroupLayout>]| {
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: groups,
            ..Default::default()
        })
    };
    let (shadow_pl, scene_pl, post_pl, over_pl) = (
        pl(&[Some(&l.shadow)]),
        pl(&[Some(&l.scene)]),
        pl(&[Some(&l.post)]),
        pl(&[Some(&l.over), Some(&l.tex)]),
    );
    let vattrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2];
    let iattrs = wgpu::vertex_attr_array![3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 8 => Float32x4, 9 => Float32x4, 10 => Float32x2, 11 => Uint32, 12 => Uint32];
    let vbufs = [
        Some(wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &vattrs,
        }),
        Some(wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Inst>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &iattrs,
        }),
    ];
    let depth = |write: bool, bias: bool| {
        Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(write),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: if bias {
                wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 2.0,
                    clamp: 0.0,
                }
            } else {
                wgpu::DepthBiasState::default()
            },
        })
    };
    let mk = |label: &str, fs: Option<&str>, glass: bool| {
        let targets = [
            Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8Unorm,
                blend: glass.then_some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            }),
            Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::R32Uint,
                blend: None,
                write_mask: if glass {
                    wgpu::ColorWrites::empty()
                } else {
                    wgpu::ColorWrites::ALL
                },
            }),
        ];
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(if fs.is_some() { &scene_pl } else { &shadow_pl }),
            vertex: wgpu::VertexState {
                module: &scene_mod,
                entry_point: Some(if fs.is_some() { "vs" } else { "vs_shadow" }),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &vbufs,
            },
            fragment: fs.map(|f| wgpu::FragmentState {
                module: &scene_mod,
                entry_point: Some(f),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &targets,
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: fs.is_some().then_some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: depth(!glass, fs.is_none()),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        })
    };
    let screen = |label: &str,
                  m: &wgpu::ShaderModule,
                  layout: &wgpu::PipelineLayout,
                  buffers: &[Option<wgpu::VertexBufferLayout<'_>>],
                  blend: Option<wgpu::BlendState>| {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: m,
                entry_point: Some("vs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers,
            },
            fragment: Some(wgpu::FragmentState {
                module: m,
                entry_point: Some("fs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        })
    };
    let qattrs = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4];
    let quad = [Some(wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Quad>() as u64,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &qattrs,
    })];
    Pipelines {
        scene: mk("scene", Some("fs"), false),
        glass: mk("glass", Some("fs_glass"), true),
        shadow: mk("shadow", None, false),
        post: screen("post", &post_mod, &post_pl, &[], None),
        over: screen(
            "overlay",
            &over_mod,
            &over_pl,
            &quad,
            Some(wgpu::BlendState::ALPHA_BLENDING),
        ),
    }
}

impl Gpu {
    /// Sets everything up for `scene`, drawing into `format`.
    #[allow(clippy::too_many_lines)] // one assembly of the GPU's state, piece by piece
    pub(crate) fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        format: wgpu::TextureFormat,
        scene: &Scene,
    ) -> Self {
        let lib = Library::build();
        let lib_v = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("verts"),
            contents: bytemuck::cast_slice(&lib.verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let lib_i = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("index"),
            contents: bytemuck::cast_slice(&lib.idx),
            usage: wgpu::BufferUsages::INDEX,
        });
        let ranges: HashMap<Mesh, (u32, u32)> =
            Mesh::ALL.iter().map(|&m| (m, lib.range(m))).collect();
        let buffer = |label: &str, size: usize, usage: wgpu::BufferUsages| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: size as u64,
                usage,
                mapped_at_creation: false,
            })
        };
        let dyn_buf = buffer(
            "moving",
            MAX_DYN * std::mem::size_of::<Inst>(),
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        );
        let globals = uniform(&device, std::mem::size_of::<Globals>());
        let lights = buffer(
            "lights",
            MAX_LIGHTS * std::mem::size_of::<GpuLight>(),
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let shadow_view = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("shadow"),
                size: wgpu::Extent3d {
                    width: SHADOW,
                    height: SHADOW,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            compare: Some(wgpu::CompareFunction::LessEqual),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let l = layouts(&device);
        let p = pipelines(&device, &l, format);
        let shadow_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &l.shadow,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals.as_entire_binding(),
            }],
        });
        let post_u = uniform(&device, std::mem::size_of::<PostU>());
        let over_u = uniform(&device, 16);
        let over_g = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &l.over,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: over_u.as_entire_binding(),
            }],
        });
        let quads = buffer(
            "overlay",
            MAX_QUADS * std::mem::size_of::<Quad>(),
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        );
        let pick_buf = buffer(
            "pick",
            256,
            wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        );
        let empty = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 16,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });
        let placeholder = upload(&device, &queue, &Img::new(1, 1));
        let mats = mats_texture(&device, &queue, None);
        let mat_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            anisotropy_clamp: 8,
            ..Default::default()
        });
        let scene_group = scene_group(
            &device,
            &l.scene,
            [&globals, &lights],
            [&placeholder, &mats],
            (&sampler, &shadow_sampler, &mat_sampler),
            &shadow_view,
        );
        let ui_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &l.tex,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&placeholder),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let mut gpu = Self {
            device,
            queue,
            format,
            lib_v,
            lib_i,
            ranges,
            stat_buf: empty,
            stat: Vec::new(),
            dyn_buf,
            globals,
            lights,
            scene_layout: l.scene,
            scene_group,
            shadow_group,
            shadow_view,
            sampler,
            shadow_sampler,
            p_scene: p.scene,
            p_glass: p.glass,
            p_shadow: p.shadow,
            p_post: p.post,
            p_over: p.over,
            post_u,
            post_layout: l.post,
            over_u,
            over_g,
            tex_layout: l.tex,
            ui_group,
            atlas: placeholder,
            mats,
            mats_made: false,
            mat_sampler,
            quads,
            pick_buf,
            tg: None,
            full: false,
            look: crate::Look::default(),
        };
        gpu.load(scene);
        gpu.ensure_materials();
        gpu
    }

    /// Uploads a (re)built scene: its still parts and its atlases.
    pub(crate) fn load(&mut self, scene: &Scene) {
        let mut items = scene.stat.clone();
        let (insts, batches) = batch(&mut items);
        self.stat_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("still"),
                contents: if insts.is_empty() {
                    &[0; 16]
                } else {
                    bytemuck::cast_slice(&insts)
                },
                usage: wgpu::BufferUsages::VERTEX,
            });
        self.stat = batches;
        self.atlas = upload(&self.device, &self.queue, &scene.atlas);
        self.rebind();
        let ui = upload(&self.device, &self.queue, &scene.ui.img);
        self.ui_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.tex_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&ui),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
    }

    /// Rebuilds the scene's bindings (a new atlas, or the materials made).
    fn rebind(&mut self) {
        self.scene_group = scene_group(
            &self.device,
            &self.scene_layout,
            [&self.globals, &self.lights],
            [&self.atlas, &self.mats],
            (&self.sampler, &self.shadow_sampler, &self.mat_sampler),
            &self.shadow_view,
        );
    }

    /// The second pass's uniform: Pixel art reads the maps a little
    /// sharper (its big pixels hold the detail as texture) and keeps more
    /// of its banded sun; Full resolution softer; all 0 without it.
    fn mat_uniform(&self) -> [f32; 4] {
        if !(self.look.materials && self.mats_made) {
            [0.0; 4]
        } else if self.full {
            [1.0, 1.0, 0.3, 1.0]
        } else {
            [1.0, 0.7, 0.55, 0.85]
        }
    }

    /// Makes the materials' maps the first time the second pass is drawn
    /// (lazily: a Floor that never draws it never pays for them).
    pub(crate) fn ensure_materials(&mut self) {
        if self.look.materials && !self.mats_made {
            let maps = juggler_floor::materials::bake();
            self.mats = mats_texture(&self.device, &self.queue, Some(&maps));
            self.mats_made = true;
            self.rebind();
        }
    }

    fn targets(&self, w: u32, h: u32) -> Targets {
        let tex = |format: wgpu::TextureFormat, usage: wgpu::TextureUsages| {
            self.device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let attach = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
        let color = tex(wgpu::TextureFormat::Rgba8Unorm, attach);
        let ids = tex(
            wgpu::TextureFormat::R32Uint,
            attach | wgpu::TextureUsages::COPY_SRC,
        );
        let depth = tex(wgpu::TextureFormat::Depth32Float, attach);
        let v = |t: &wgpu::Texture| t.create_view(&wgpu::TextureViewDescriptor::default());
        let post_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.post_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.post_u.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&v(&color)),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&v(&ids)),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&v(&depth)),
                },
            ],
        });
        Targets {
            w,
            h,
            color,
            ids,
            depth,
            post_group,
        }
    }

    /// The pick id under a device-pixel position, from the last frame.
    pub(crate) fn pick(&self, x: f32, y: f32, scale: f32) -> u32 {
        let Some(tg) = &self.tg else { return 0 };
        // One id per scene pixel: a device pixel in Full resolution.
        let px = scene_px(self.full, self.look.fine, scale);
        if x < 0.0 || y < 0.0 {
            return 0;
        }
        let (qx, qy) = ((x / px) as u32, (y / px) as u32);
        if qx >= tg.w || qy >= tg.h {
            return 0;
        }
        let mut enc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("pick"),
            });
        enc.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &tg.ids,
                mip_level: 0,
                origin: wgpu::Origin3d { x: qx, y: qy, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.pick_buf,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(1),
                },
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([enc.finish()]);
        let slice = self.pick_buf.slice(..);
        let (tx, rx) = channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        let mut id = 0;
        if rx.recv().is_ok_and(|r| r.is_ok())
            && let Ok(d) = slice.get_mapped_range()
            && let Some(bytes) = d.get(..4)
        {
            id = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        }
        self.pick_buf.unmap();
        id
    }

    /// Draws one frame into `target`: the scene's still part, `moving`
    /// (this frame's moving part), `lights` and the overlay's `quads`.
    pub(crate) fn draw(&mut self, scene: &Scene, frame: &Frame<'_>, target: &wgpu::TextureView) {
        let written = self.write_frame(scene, frame);
        self.encode(frame, &written, target);
    }

    /// Writes this frame's buffers: the moving part, the nearest lights,
    /// the uniforms and the overlay.
    fn write_frame(&mut self, scene: &Scene, frame: &Frame<'_>) -> Written {
        let (w, h, sf) = frame.size;
        let px = scene_px(self.full, self.look.fine, sf);
        let (lw, lh) = (
            ((w as f32 / px).ceil() as u32).max(1),
            ((h as f32 / px).ceil() as u32).max(1),
        );
        if self.tg.as_ref().map(|t| (t.w, t.h)) != Some((lw, lh)) {
            self.tg = Some(self.targets(lw, lh));
        }
        let mut moving = frame.moving.to_vec();
        let (dyn_insts, dyn_batches) = batch(&mut moving);
        let n_dyn = dyn_insts.len().min(MAX_DYN);
        if n_dyn > 0 {
            self.queue
                .write_buffer(&self.dyn_buf, 0, bytemuck::cast_slice(&dyn_insts[..n_dyn]));
        }
        let cam = frame.view.cam;
        let tgt = cam.t;
        let mut lights: Vec<&Light> = frame.lights.iter().collect();
        let near = |l: &Light| {
            (l.p[0] - tgt[0]).powi(2) + (l.p[1] - tgt[1]).powi(2) * 4.0 + (l.p[2] - tgt[2]).powi(2)
        };
        lights.sort_by(|a, b| near(a).total_cmp(&near(b)));
        let gl: Vec<GpuLight> = lights
            .iter()
            .take(MAX_LIGHTS)
            .map(|l| GpuLight { p: l.p, c: l.c })
            .collect();
        if !gl.is_empty() {
            self.queue
                .write_buffer(&self.lights, 0, bytemuck::cast_slice(&gl));
        }
        let aspect = w as f32 / h.max(1) as f32;
        let vp = cam.vp(aspect);
        let sun = norm(self.look.sky.sun_dir);
        let centre = [
            cam.t[0].clamp(0.0, scene.width),
            cam.t[1],
            cam.t[2].clamp(0.0, DEPTH),
        ];
        let hs = if frame.view.focus.is_some() {
            (scene.width * 0.62)
                .max(cam.dist * 0.5)
                .min(scene.width * 0.75)
        } else {
            scene.width * 0.75
        };
        let lvp = M4::ortho(-hs, hs, -hs, hs, 1.0, 400.0).mul(&M4::look_at(
            add(centre, scale(sun, 150.0)),
            centre,
            [0.0, 1.0, 0.0],
        ));
        let eye = cam.eye();
        let alert = if frame.alert { 1.0 } else { 0.0 };
        let g = Globals {
            vp: vp.0,
            lvp: lvp.0,
            eye: [eye[0], eye[1], eye[2], 1.0],
            sun: [sun[0], sun[1], sun[2], self.look.sky.sun_k],
            sky: rgb1(self.look.sky.hemi_sky),
            ground: rgb1(self.look.sky.hemi_ground),
            params: [
                frame.t,
                alert,
                gl.len() as f32,
                if self.full { 1.0 } else { 0.0 },
            ],
            sunc: rgb1(self.look.sky.sun_col),
            look: if self.look.rework {
                [1.0, self.look.sky.night, 1.0, 1.0]
            } else {
                [0.0; 4]
            },
            mat: self.mat_uniform(),
            atl: [scene.atlas_v, 0.0, 0.0, 0.0],
        };
        self.queue
            .write_buffer(&self.globals, 0, bytemuck::bytes_of(&g));
        let post = post_uniform(self.full, &self.look, frame, (lw, lh, px), alert);
        self.queue
            .write_buffer(&self.post_u, 0, bytemuck::bytes_of(&post));
        let n_quads = frame.quads.len().min(MAX_QUADS);
        if n_quads > 0 {
            self.queue.write_buffer(
                &self.quads,
                0,
                bytemuck::cast_slice(&frame.quads[..n_quads]),
            );
        }
        self.queue.write_buffer(
            &self.over_u,
            0,
            bytemuck::cast_slice(&[2.0 / w.max(1) as f32, -2.0 / h.max(1) as f32, -1.0, 1.0]),
        );
        Written {
            moving: dyn_batches,
            count: n_dyn,
            quads: n_quads,
        }
    }

    /// Draws the runs of `batches` (glass or not, up to `limit`) on the
    /// floors in view.
    fn draw_runs(
        &self,
        rp: &mut wgpu::RenderPass<'_>,
        buf: &wgpu::Buffer,
        batches: &[Batch],
        (glass, limit): (bool, u32),
        floors: &[usize],
    ) {
        let shown = |f: i32| f < 0 || usize::try_from(f).is_ok_and(|f| floors.contains(&f));
        rp.set_vertex_buffer(1, buf.slice(..));
        for b in batches
            .iter()
            .filter(|b| b.glass == glass && shown(b.floor) && b.start < limit)
        {
            let (s, n) = self.ranges.get(&b.mesh).copied().unwrap_or((0, 0));
            rp.draw_indexed(s..s + n, 0, b.start..(b.start + b.count).min(limit));
        }
    }

    /// Encodes the shadow, scene and post-and-overlay passes.
    fn encode(&self, frame: &Frame<'_>, written: &Written, target: &wgpu::TextureView) {
        let (dyn_batches, n_dyn, n_quads) = (&written.moving, written.count, written.quads);
        let mut enc = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame"),
            });
        let limit = u32::try_from(n_dyn).unwrap_or(u32::MAX);
        let floors = frame.floors;
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow"),
                color_attachments: &[],
                depth_stencil_attachment: Some(depth_clear(&self.shadow_view)),
                ..Default::default()
            });
            rp.set_pipeline(&self.p_shadow);
            rp.set_bind_group(0, &self.shadow_group, &[]);
            rp.set_vertex_buffer(0, self.lib_v.slice(..));
            rp.set_index_buffer(self.lib_i.slice(..), wgpu::IndexFormat::Uint32);
            self.draw_runs(
                &mut rp,
                &self.stat_buf,
                &self.stat,
                (false, u32::MAX),
                floors,
            );
            self.draw_runs(&mut rp, &self.dyn_buf, dyn_batches, (false, limit), floors);
        }
        let Some(tg) = self.tg.as_ref() else { return };
        {
            let (cv, iv, dv) = (
                tg.color
                    .create_view(&wgpu::TextureViewDescriptor::default()),
                tg.ids.create_view(&wgpu::TextureViewDescriptor::default()),
                tg.depth
                    .create_view(&wgpu::TextureViewDescriptor::default()),
            );
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene"),
                color_attachments: &[
                    Some(colour_clear(&cv, wgpu::Color::TRANSPARENT)),
                    Some(colour_clear(&iv, wgpu::Color::TRANSPARENT)),
                ],
                depth_stencil_attachment: Some(depth_clear(&dv)),
                ..Default::default()
            });
            rp.set_bind_group(0, &self.scene_group, &[]);
            rp.set_vertex_buffer(0, self.lib_v.slice(..));
            rp.set_index_buffer(self.lib_i.slice(..), wgpu::IndexFormat::Uint32);
            rp.set_pipeline(&self.p_scene);
            self.draw_runs(
                &mut rp,
                &self.stat_buf,
                &self.stat,
                (false, u32::MAX),
                floors,
            );
            self.draw_runs(&mut rp, &self.dyn_buf, dyn_batches, (false, limit), floors);
            rp.set_pipeline(&self.p_glass);
            self.draw_runs(
                &mut rp,
                &self.stat_buf,
                &self.stat,
                (true, u32::MAX),
                floors,
            );
            self.draw_runs(&mut rp, &self.dyn_buf, dyn_batches, (true, limit), floors);
        }
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("post"),
                color_attachments: &[Some(colour_clear(target, wgpu::Color::BLACK))],
                ..Default::default()
            });
            rp.set_pipeline(&self.p_post);
            rp.set_bind_group(0, &tg.post_group, &[]);
            rp.draw(0..3, 0..1);
            if n_quads > 0 {
                rp.set_pipeline(&self.p_over);
                rp.set_bind_group(0, &self.over_g, &[]);
                rp.set_bind_group(1, &self.ui_group, &[]);
                rp.set_vertex_buffer(0, self.quads.slice(..));
                rp.draw(0..6, 0..u32::try_from(n_quads).unwrap_or(0));
            }
        }
        self.queue.submit([enc.finish()]);
    }
}

/// The materials' texture array: every map and level, or a 1 × 1 stand-in.
fn mats_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    maps: Option<&juggler_floor::materials::Maps>,
) -> wgpu::TextureView {
    use juggler_floor::materials::{LAYERS, LEVELS, SIZE};
    let (side, layers, levels, data) = match maps {
        Some(m) => (SIZE, LAYERS, LEVELS, m.data.as_slice()),
        None => (1, 1, 1, &[128u8, 128, 128, 128][..]),
    };
    device
        .create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: Some("materials"),
                size: wgpu::Extent3d {
                    width: side,
                    height: side,
                    depth_or_array_layers: layers,
                },
                mip_level_count: levels,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            data,
        )
        .create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        })
}

fn scene_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    [globals, lights]: [&wgpu::Buffer; 2],
    [atlas, mats]: [&wgpu::TextureView; 2],
    (sampler, shadow_sampler, mat_sampler): (&wgpu::Sampler, &wgpu::Sampler, &wgpu::Sampler),
    shadow: &wgpu::TextureView,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("scene"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: globals.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: lights.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(atlas),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::TextureView(shadow),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: wgpu::BindingResource::Sampler(shadow_sampler),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: wgpu::BindingResource::TextureView(mats),
            },
            wgpu::BindGroupEntry {
                binding: 7,
                resource: wgpu::BindingResource::Sampler(mat_sampler),
            },
        ],
    })
}

/// A colour attachment cleared to `colour`.
fn colour_clear(
    view: &wgpu::TextureView,
    colour: wgpu::Color,
) -> wgpu::RenderPassColorAttachment<'_> {
    wgpu::RenderPassColorAttachment {
        view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(colour),
            store: wgpu::StoreOp::Store,
        },
    }
}

/// A depth attachment cleared to the far plane.
fn depth_clear(view: &wgpu::TextureView) -> wgpu::RenderPassDepthStencilAttachment<'_> {
    wgpu::RenderPassDepthStencilAttachment {
        view,
        depth_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Clear(1.0),
            store: wgpu::StoreOp::Store,
        }),
        stencil_ops: None,
    }
}

/// What [`Gpu::write_frame`] wrote: the moving part's runs, how many
/// moving instances and overlay quads.
struct Written {
    moving: Vec<Batch>,
    count: usize,
    quads: usize,
}

/// One frame's inputs, already built on the CPU.
pub struct Frame<'a> {
    /// What's in view.
    pub view: &'a View,
    /// The floors to draw.
    pub floors: &'a [usize],
    /// The window, device pixels, and its scale.
    pub size: (u32, u32, f32),
    /// The clock, seconds.
    pub t: f32,
    /// The overload alert is on.
    pub alert: bool,
    /// The pick under the pointer.
    pub hover: u32,
    /// The moving part.
    pub moving: &'a [Item],
    /// The lights in view.
    pub lights: &'a [Light],
    /// The overlay.
    pub quads: &'a [Quad],
}
