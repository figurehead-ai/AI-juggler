//! Option 3: the Floor in a native window of the same process, drawn with wgpu.
//! Tauri makes the window (no webview), wgpu draws into it from a render thread. Frames are drawn only
//! when something changes: a tick while people animate, a resize, or a capture. While animating, the
//! window's visibility is checked twice a second; covered or minimised windows draw nothing.

use crate::scene::{Inst, Scene};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};
use wgpu::util::DeviceExt;

pub enum Msg {
    Resize(u32, u32),
    Visible(bool),
    Capture(String),
}

#[derive(Default, Clone, Copy, serde::Serialize)]
pub struct Stats {
    pub frames: u64,
    pub draw_ms: f64,
    pub first_frame_ms: f64,
    pub visibility_checks: u64,
}

pub struct Native {
    pub tx: Sender<Msg>,
    pub stats: Arc<Mutex<Stats>>,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Quad {
    dst: [f32; 4],
    src: [f32; 4],
}

const SHADER: &str = r#"
struct Globals { art: vec2<f32>, pad: vec2<f32> };
@group(0) @binding(0) var<uniform> g: Globals;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;
struct VOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs(@builtin(vertex_index) vi: u32, @location(0) dst: vec4<f32>, @location(1) src: vec4<f32>) -> VOut {
  var c = array<vec2<f32>, 6>(vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0), vec2(0.0, 1.0), vec2(1.0, 0.0), vec2(1.0, 1.0));
  let k = c[vi];
  let p = dst.xy + k * dst.zw;
  var o: VOut;
  o.pos = vec4(p.x / g.art.x * 2.0 - 1.0, 1.0 - p.y / g.art.y * 2.0, 0.0, 1.0);
  o.uv = mix(src.xy, src.zw, k);
  return o;
}
@fragment fn fs(v: VOut) -> @location(0) vec4<f32> {
  let c = textureSample(tex, samp, v.uv);
  if (c.a < 0.5) { discard; }
  return c;
}
"#;

const MAX_QUADS: usize = 2048;

/// Everything wgpu needs to draw the scene; built once on the render thread.
struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    globals: wgpu::BindGroup,
    bg_group: wgpu::BindGroup,
    atlas_group: wgpu::BindGroup,
    bg_quad: wgpu::Buffer,
    quads_buf: wgpu::Buffer,
    inst: Vec<Inst>,
    quads: Vec<Quad>,
}

impl Gpu {
    fn new(device: wgpu::Device, queue: wgpu::Queue, format: wgpu::TextureFormat, scene: &Scene) -> Gpu {
        let m = &scene.meta;
        let tex_format = if format.is_srgb() { wgpu::TextureFormat::Rgba8UnormSrgb } else { wgpu::TextureFormat::Rgba8Unorm };
        let upload = |w: i32, h: i32, px: &[u8]| {
            device
                .create_texture_with_data(
                    &queue,
                    &wgpu::TextureDescriptor {
                        label: None,
                        size: wgpu::Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: tex_format,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    },
                    wgpu::util::TextureDataOrder::LayerMajor,
                    px,
                )
                .create_view(&wgpu::TextureViewDescriptor::default())
        };
        let bg_view = upload(scene.bg.w, scene.bg.h, &scene.bg.px);
        let atlas_view = upload(scene.atlas.w, scene.atlas.h, &scene.atlas.px);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor { mag_filter: wgpu::FilterMode::Nearest, min_filter: wgpu::FilterMode::Nearest, ..Default::default() });
        let ubuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: bytemuck::cast_slice(&[m.art_w as f32, m.art_h as f32, 0.0, 0.0]), usage: wgpu::BufferUsages::UNIFORM });
        let gl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::VERTEX, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None }],
        });
        let tl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        let globals = device.create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout: &gl, entries: &[wgpu::BindGroupEntry { binding: 0, resource: ubuf.as_entire_binding() }] });
        let tex_group = |v: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &tl,
                entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(v) }, wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) }],
            })
        };
        let bg_group = tex_group(&bg_view);
        let atlas_group = tex_group(&atlas_view);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: None, source: wgpu::ShaderSource::Wgsl(SHADER.into()) });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[Some(&gl), Some(&tl)], ..Default::default() });
        let attrs = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sprites"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<Quad>() as u64, step_mode: wgpu::VertexStepMode::Instance, attributes: &attrs })],
            },
            fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs"), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })] }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let quads_buf = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: (MAX_QUADS * std::mem::size_of::<Quad>()) as u64, usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let bg_quad = device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: bytemuck::cast_slice(&[Quad { dst: [0.0, 0.0, m.art_w as f32, m.art_h as f32], src: [0.0, 0.0, 1.0, 1.0] }]), usage: wgpu::BufferUsages::VERTEX });
        Gpu { device, queue, format, pipeline, globals, bg_group, atlas_group, bg_quad, quads_buf, inst: Vec::with_capacity(1024), quads: Vec::with_capacity(1024) }
    }

    /// One frame at time `t` into `target`: the background, then the depth-sorted sprites, in two draws.
    fn draw(&mut self, scene: &Scene, t: f32, target: &wgpu::TextureView) {
        let m = &scene.meta;
        let (aw, ah) = (m.atlas_w as f32, m.atlas_h as f32);
        scene.instances(t, &mut self.inst);
        self.quads.clear();
        for i in &self.inst {
            let s = m.sprites[i.s];
            self.quads.push(Quad { dst: [i.x, i.y, s.w as f32, s.h as f32], src: [s.x as f32 / aw, s.y as f32 / ah, (s.x + s.w) as f32 / aw, (s.y + s.h) as f32 / ah] });
        }
        self.quads.truncate(MAX_QUADS);
        self.queue.write_buffer(&self.quads_buf, 0, bytemuck::cast_slice(&self.quads));
        let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: target, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.043, g: 0.055, b: 0.078, a: 1.0 }), store: wgpu::StoreOp::Store } })],
                ..Default::default()
            });
            rp.set_pipeline(&self.pipeline);
            rp.set_bind_group(0, &self.globals, &[]);
            rp.set_bind_group(1, &self.bg_group, &[]);
            rp.set_vertex_buffer(0, self.bg_quad.slice(..));
            rp.draw(0..6, 0..1);
            rp.set_bind_group(1, &self.atlas_group, &[]);
            rp.set_vertex_buffer(0, self.quads_buf.slice(..));
            rp.draw(0..6, 0..self.quads.len() as u32);
        }
        self.queue.submit([enc.finish()]);
    }

    /// Draws one frame at art resolution offscreen and writes it as raw RGBA (8-byte header: w, h).
    fn capture(&mut self, scene: &Scene, t: f32, path: &str) {
        let (w, h) = (scene.meta.art_w as u32, scene.meta.art_h as u32);
        let tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        self.draw(scene, t, &tex.create_view(&wgpu::TextureViewDescriptor::default()));
        let row = (w * 4).div_ceil(256) * 256;
        let buf = self.device.create_buffer(&wgpu::BufferDescriptor { label: None, size: (row * h) as u64, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });
        let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        enc.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo { texture: &tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::TexelCopyBufferInfo { buffer: &buf, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(h) } },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        self.queue.submit([enc.finish()]);
        let slice = buf.slice(..);
        let (tx, rx) = channel();
        slice.map_async(wgpu::MapMode::Read, move |r| { let _ = tx.send(r); });
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        if rx.recv().map(|r| r.is_ok()).unwrap_or(false) {
            let Ok(data) = slice.get_mapped_range() else { return };
            let bgra = matches!(self.format, wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb);
            let mut out = Vec::with_capacity((w * h * 4 + 8) as usize);
            out.extend_from_slice(&w.to_le_bytes());
            out.extend_from_slice(&h.to_le_bytes());
            for y in 0..h {
                for x in 0..w {
                    let i = (y * row + x * 4) as usize;
                    if bgra { out.extend_from_slice(&[data[i + 2], data[i + 1], data[i], data[i + 3]]); } else { out.extend_from_slice(&data[i..i + 4]); }
                }
            }
            let _ = std::fs::write(path, out);
        }
    }
}

/// `art_surface`: draw into a surface the size of the art (852x532) and let the window system scale it up
/// with hard edges, as the web canvas does, instead of a surface the size of the window.
pub fn open(app: &AppHandle, scene: &'static Scene, anim: u32, on_top: bool, art_surface: bool, size: (f64, f64), opened: Instant) -> tauri::Result<Native> {
    let window = tauri::window::WindowBuilder::new(app, "floor-native").title("Floor — native wgpu").inner_size(size.0, size.1).always_on_top(on_top).focused(true).build()?;
    let size = window.inner_size()?;
    // wgpu 30 takes the display handle when the instance is made (Metal, Vulkan on X11/Wayland, DX12).
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(Box::new(window.clone())));
    // On macOS the surface must be made on the main thread.
    let (stx, srx) = channel();
    {
        let (w, inst) = (window.clone(), instance.clone());
        app.run_on_main_thread(move || {
            let target = unsafe { wgpu::SurfaceTargetUnsafe::from_window(&w) }.expect("window handle");
            let surface = unsafe { inst.create_surface_unsafe(target) }.expect("surface");
            if art_surface { nearest_layer(&w); }
            let _ = stx.send(surface);
        })?;
    }
    let surface = srx.recv().expect("surface from the main thread");
    let (tx, rx) = channel::<Msg>();
    let stats = Arc::new(Mutex::new(Stats::default()));
    let (st, w2, app2, tx2) = (stats.clone(), window.clone(), app.clone(), tx.clone());
    std::thread::Builder::new().name("floor-render".into()).spawn(move || {
        let size = if art_surface { (scene.meta.art_w as u32, scene.meta.art_h as u32) } else { (size.width, size.height) };
        render_loop(instance, surface, scene, anim, size, art_surface, rx, tx2, st, w2, app2, opened);
    })?;
    Ok(Native { tx, stats })
}

#[allow(clippy::too_many_arguments)]
fn render_loop(instance: wgpu::Instance, surface: wgpu::Surface<'static>, scene: &'static Scene, anim: u32, size: (u32, u32), art_surface: bool, rx: Receiver<Msg>, tx: Sender<Msg>, stats: Arc<Mutex<Stats>>, window: tauri::Window, app: AppHandle, opened: Instant) {
    // A real GPU when there is one; the software adapter (WARP, lavapipe) on machines without, such as CI.
    let ask = |fallback: bool| pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions { power_preference: wgpu::PowerPreference::LowPower, compatible_surface: Some(&surface), force_fallback_adapter: fallback, ..Default::default() }));
    let adapter = ask(false).or_else(|_| ask(true)).expect("adapter");
    println!("{}", serde_json::json!({ "event": "adapter", "info": format!("{:?}", adapter.get_info()) }));
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor { label: Some("floor"), ..Default::default() })).expect("device");
    let caps = surface.get_capabilities(&adapter);
    let format = caps.formats.iter().copied().find(|f| !f.is_srgb()).unwrap_or(caps.formats[0]);
    let mut config = wgpu::SurfaceConfiguration { usage: wgpu::TextureUsages::RENDER_ATTACHMENT, format, width: size.0.max(1), height: size.1.max(1), present_mode: wgpu::PresentMode::Fifo, desired_maximum_frame_latency: 2, alpha_mode: caps.alpha_modes[0], view_formats: vec![], color_space: Default::default() };
    surface.configure(&device, &config);
    let mut gpu = Gpu::new(device, queue, format, scene);
    let start = Instant::now();
    let present = |gpu: &mut Gpu, t: f32| {
        let t0 = Instant::now();
        let frame = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            _ => return,
        };
        gpu.draw(scene, t, &frame.texture.create_view(&wgpu::TextureViewDescriptor::default()));
        gpu.queue.present(frame);
        let mut s = stats.lock().unwrap();
        s.frames += 1;
        s.draw_ms += t0.elapsed().as_secs_f64() * 1000.0;
        if s.frames == 1 {
            s.first_frame_ms = opened.elapsed().as_secs_f64() * 1000.0;
            println!("{}", serde_json::json!({ "event": "first_frame", "kind": "native", "ms": s.first_frame_ms }));
        }
    };
    present(&mut gpu, 0.0);
    let period = if anim > 0 { Duration::from_secs_f64(1.0 / anim as f64) } else { Duration::from_secs(3600) };
    let mut next = Instant::now() + period;
    let mut visible = true;
    let mut next_check = Instant::now() + Duration::from_millis(500);
    loop {
        // Nothing animating: sleep until a message arrives. Animating: wake for the next tick or the next check.
        let msg = if anim == 0 {
            match rx.recv() { Ok(m) => Some(m), Err(_) => return }
        } else {
            let wake = if visible { next.min(next_check) } else { next_check };
            match rx.recv_timeout(wake.saturating_duration_since(Instant::now())) {
                Ok(m) => Some(m),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => return,
            }
        };
        match msg {
            Some(Msg::Resize(w, h)) if w > 0 && h > 0 => {
                if !art_surface {
                    config.width = w;
                    config.height = h;
                }
                surface.configure(&gpu.device, &config);
                present(&mut gpu, start.elapsed().as_secs_f32());
            }
            Some(Msg::Visible(v)) => {
                if v && !visible { next = Instant::now(); }
                visible = v;
            }
            Some(Msg::Capture(path)) => gpu.capture(scene, start.elapsed().as_secs_f32(), &path),
            _ => {}
        }
        let now = Instant::now();
        if anim > 0 && now >= next_check {
            next_check = now + Duration::from_millis(500);
            stats.lock().unwrap().visibility_checks += 1;
            ask_visibility(&app, &window, &tx);
        }
        if anim > 0 && visible && now >= next {
            present(&mut gpu, start.elapsed().as_secs_f32());
            next += period;
            if next < now { next = now + period; }
        }
    }
}

/// Makes the window's Metal layer scale its (smaller) drawable up with hard edges.
#[cfg(target_os = "macos")]
fn nearest_layer(w: &tauri::Window) {
    use objc2::msg_send;
    use objc2::runtime::AnyObject;
    let Ok(view) = w.ns_view() else { return };
    unsafe {
        let view = &*(view as *mut AnyObject);
        let layer: *mut AnyObject = msg_send![view, layer];
        if layer.is_null() { return; }
        let nearest = objc2_foundation::NSString::from_str("nearest");
        let _: () = msg_send![&*layer, setMagnificationFilter: &*nearest];
    }
}
#[cfg(not(target_os = "macos"))]
fn nearest_layer(_: &tauri::Window) {}

/// Whether the window can be seen. AppKit wants this asked on the main thread.
fn ask_visibility(app: &AppHandle, window: &tauri::Window, tx: &Sender<Msg>) {
    let (w, tx) = (window.clone(), tx.clone());
    let _ = app.run_on_main_thread(move || {
        let _ = tx.send(Msg::Visible(window_visible(&w)));
    });
}

#[cfg(target_os = "macos")]
fn window_visible(w: &tauri::Window) -> bool {
    let Ok(ptr) = w.ns_window() else { return true };
    let nsw: &objc2_app_kit::NSWindow = unsafe { &*(ptr as *const objc2_app_kit::NSWindow) };
    nsw.occlusionState().contains(objc2_app_kit::NSWindowOcclusionState::Visible) && !nsw.isMiniaturized()
}

#[cfg(not(target_os = "macos"))]
fn window_visible(w: &tauri::Window) -> bool {
    !w.is_minimized().unwrap_or(false) && w.is_visible().unwrap_or(true)
}

pub fn handle_event(app: &AppHandle, label: &str, ev: &tauri::WindowEvent) {
    if label != "floor-native" {
        return;
    }
    if let tauri::WindowEvent::Resized(s) = ev {
        if let Some(n) = app.try_state::<Native>() {
            let _ = n.tx.send(Msg::Resize(s.width, s.height));
        }
    }
}
