//! The Floor's native window: wgpu draws the baked building and the depth-sorted sprites, then a
//! screen-space overlay (floor chips, far-zoom name chips, the red-alert banner). Zoom is a whole number
//! of device pixels per art pixel (1 building, 2 floor, 3 area, 4 desk), so the pixel art stays crisp,
//! and the camera snaps to whole device pixels. Frames are drawn only on a change: input, a tween, or a
//! tick while something animated is in view and the window can be seen.

use crate::world::{Inst, Target, World};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use wgpu::util::DeviceExt;

pub enum Msg {
    Resize(u32, u32, f32),
    Visible(bool),
    /// Scroll in points: precise (trackpad) pans, a wheel zooms; position in points from the top left.
    Scroll { dx: f32, dy: f32, precise: bool, x: f32, y: f32, zoom: bool },
    Magnify { m: f32, x: f32, y: f32 },
    Drag { dx: f32, dy: f32 },
    Click { x: f32, y: f32 },
    Key(char),
    Tour(String),
}

#[derive(Default, Clone, Copy, serde::Serialize)]
pub struct Stats { pub frames: u64, pub draw_ms: f64, pub first_frame_ms: f64, pub visibility_checks: u64, pub instances: u64 }

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Quad { dst: [f32; 4], src: [f32; 4], tint: [f32; 4] }

const SHADER: &str = r#"
struct Globals { s: vec2<f32>, o: vec2<f32> };
@group(0) @binding(0) var<uniform> g: Globals;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;
struct VOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) tint: vec4<f32> };
@vertex fn vs(@builtin(vertex_index) vi: u32, @location(0) dst: vec4<f32>, @location(1) src: vec4<f32>, @location(2) tint: vec4<f32>) -> VOut {
  var c = array<vec2<f32>, 6>(vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0), vec2(0.0, 1.0), vec2(1.0, 0.0), vec2(1.0, 1.0));
  let k = c[vi];
  let p = dst.xy + k * dst.zw;
  var o: VOut;
  o.pos = vec4(p.x * g.s.x + g.o.x, p.y * g.s.y + g.o.y, 0.0, 1.0);
  o.uv = mix(src.xy, src.zw, k);
  o.tint = tint;
  return o;
}
@fragment fn fs(v: VOut) -> @location(0) vec4<f32> {
  let c = textureSample(tex, samp, v.uv) * v.tint;
  if (c.a < 0.02) { discard; }
  return c;
}
"#;

const MAX_QUADS: usize = 4096;

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    world_g: (wgpu::Buffer, wgpu::BindGroup),
    ui_g: (wgpu::Buffer, wgpu::BindGroup),
    bg_group: wgpu::BindGroup,
    atlas_group: wgpu::BindGroup,
    bg_quad: wgpu::Buffer,
    quads_buf: wgpu::Buffer,
}

#[derive(Clone, Copy)]
struct View { k: f32, cx: f32, cy: f32 }

pub struct Floor {
    pub tx: Sender<Msg>,
    pub stats: Arc<Mutex<Stats>>,
}

/// Opens the render thread for a window whose surface was made on the main thread.
#[allow(clippy::too_many_arguments)]
pub fn start(instance: wgpu::Instance, surface: wgpu::Surface<'static>, world: Arc<World>, size: (u32, u32, f32), anim_fps: u32, opened: Instant,
    on_open: Arc<dyn Fn(Target) + Send + Sync>, ask_visible: Arc<dyn Fn(Sender<Msg>) + Send + Sync>, alert: bool) -> Floor {
    let (tx, rx) = channel::<Msg>();
    let stats = Arc::new(Mutex::new(Stats::default()));
    let (st, tx2) = (stats.clone(), tx.clone());
    std::thread::Builder::new().name("floor-render".into()).spawn(move || {
        run(instance, surface, world, size, anim_fps, opened, rx, tx2, st, on_open, ask_visible, alert);
    }).expect("render thread");
    Floor { tx, stats }
}

#[allow(clippy::too_many_arguments)]
fn run(instance: wgpu::Instance, surface: wgpu::Surface<'static>, world: Arc<World>, size: (u32, u32, f32), anim_fps: u32, opened: Instant, rx: Receiver<Msg>, tx: Sender<Msg>,
    stats: Arc<Mutex<Stats>>, on_open: Arc<dyn Fn(Target) + Send + Sync>, ask_visible: Arc<dyn Fn(Sender<Msg>) + Send + Sync>, mut alert: bool) {
    let ask = |fallback: bool| pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions { power_preference: wgpu::PowerPreference::LowPower, compatible_surface: Some(&surface), force_fallback_adapter: fallback, ..Default::default() }));
    let adapter = ask(false).or_else(|_| ask(true)).expect("adapter");
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor { label: Some("floor"), ..Default::default() })).expect("device");
    let caps = surface.get_capabilities(&adapter);
    let format = caps.formats.iter().copied().find(|f| !f.is_srgb()).unwrap_or(caps.formats[0]);
    let (mut w, mut h, mut sf) = size;
    let mut config = wgpu::SurfaceConfiguration { usage: wgpu::TextureUsages::RENDER_ATTACHMENT, format, width: w.max(1), height: h.max(1), present_mode: wgpu::PresentMode::Fifo, desired_maximum_frame_latency: 2, alpha_mode: caps.alpha_modes[0], view_formats: vec![], color_space: Default::default() };
    surface.configure(&device, &config);
    let gpu = make_gpu(device, queue, format, &world);
    let start = Instant::now();
    // start on floor 1, two device pixels per art pixel
    let f1 = world.floors[1];
    let mut view = View { k: levels(&world, w, h)[1], cx: world.w as f32 / 2.0, cy: f1.y + f1.h / 2.0 };
    let mut tween: Option<(View, View, Instant)> = None;
    let mut inst: Vec<Inst> = Vec::with_capacity(2048);
    let mut quads: Vec<Quad> = Vec::with_capacity(2048);
    let mut visible = true;
    let mut animating = false;
    let period = Duration::from_secs_f64(1.0 / anim_fps.max(1) as f64);
    let mut next_tick = Instant::now() + period;
    let mut next_check = Instant::now() + Duration::from_millis(500);
    let mut last_drawn: Option<Vec<usize>> = None;

    // None: the surface refused a frame (still being shown, occluded, outdated); the loop retries shortly.
    let present = |gpu: &Gpu, view: View, w: u32, h: u32, sf: f32, alert: bool, inst: &mut Vec<Inst>, quads: &mut Vec<Quad>| -> Option<bool> {
        let t0 = Instant::now();
        let frame = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            other => {
                let what = match other { wgpu::CurrentSurfaceTexture::Timeout => "timeout", wgpu::CurrentSurfaceTexture::Occluded => "occluded", wgpu::CurrentSurfaceTexture::Outdated => "outdated", wgpu::CurrentSurfaceTexture::Lost => "lost", _ => "other" };
                if what != "occluded" { println!("{}", serde_json::json!({ "event": "no_frame", "why": what, "size": [w, h] })); }
                if what == "outdated" || what == "lost" { surface.configure(&gpu.device, &wgpu::SurfaceConfiguration { usage: wgpu::TextureUsages::RENDER_ATTACHMENT, format, width: w.max(1), height: h.max(1), present_mode: wgpu::PresentMode::Fifo, desired_maximum_frame_latency: 2, alpha_mode: caps.alpha_modes[0], view_formats: vec![], color_space: Default::default() }); }
                return None;
            }
        };
        let anim = draw_frame(&world, gpu, view, start.elapsed().as_secs_f32(), &frame.texture.create_view(&wgpu::TextureViewDescriptor::default()), w, h, sf, alert, inst, quads);
        gpu.queue.present(frame);
        let _ = gpu.device.poll(wgpu::PollType::Poll);
        let mut s = stats.lock().unwrap();
        s.frames += 1;
        s.instances = inst.len() as u64;
        s.draw_ms += t0.elapsed().as_secs_f64() * 1000.0;
        if s.frames == 1 {
            s.first_frame_ms = opened.elapsed().as_secs_f64() * 1000.0;
            println!("{}", serde_json::json!({ "event": "first_frame", "ms": s.first_frame_ms }));
        }
        Some(anim)
    };

    let mut retry = match present(&gpu, view, w, h, sf, alert, &mut inst, &mut quads) { Some(a) => { animating = a && anim_fps > 0; false } None => true };
    let focus_floor = |f: usize, w: u32, h: u32| -> View { let b = world.floors[if f == 1 { 0 } else { 1 }]; View { k: levels(&world, w, h)[1], cx: world.w as f32 / 2.0, cy: b.y + b.h / 2.0 } };
    let building = |w: u32, h: u32| -> View { View { k: levels(&world, w, h)[0], cx: world.w as f32 / 2.0, cy: world.h as f32 / 2.0 } };
    let mut zoom_acc = 0.0f32;
    loop {
        let wake = if tween.is_some() { Instant::now() + Duration::from_millis(16) }
            else if retry && visible { Instant::now() + Duration::from_millis(100) }
            else if (animating || alert) && visible { next_tick.min(next_check) }
            else if !visible { next_check }
            else { Instant::now() + Duration::from_secs(3600) };
        let msg = match rx.recv_timeout(wake.saturating_duration_since(Instant::now())) {
            Ok(m) => Some(m),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => return,
        };
        let mut dirty = retry;
        match msg {
            Some(Msg::Resize(nw, nh, nsf)) if nw > 0 && nh > 0 => { w = nw; h = nh; sf = nsf; config.width = w; config.height = h; surface.configure(&gpu.device, &config); clamp_view(&world, &mut view, w, h); dirty = true; }
            Some(Msg::Visible(v)) => { if v != visible { println!("{}", serde_json::json!({ "event": "visible", "v": v, "ms": opened.elapsed().as_millis() as u64 })); } if v && !visible { dirty = true; } visible = v; }
            Some(Msg::Scroll { dx, dy, precise, x, y, zoom }) => {
                if precise && !zoom { view.cx -= dx * sf / view.k; view.cy -= dy * sf / view.k; clamp_view(&world, &mut view, w, h); tween = None; dirty = true; }
                else { zoom_acc += dy; if zoom_acc.abs() > if precise { 30.0 } else { 0.5 } { let step = zoom_acc.signum(); zoom_acc = 0.0; if let Some(to) = zoom_target(&world, view, view.k + step, x * sf, y * sf, w, h) { tween = Some((view, to, Instant::now())); dirty = true; } } }
            }
            Some(Msg::Magnify { m, x, y }) => { zoom_acc += m; if zoom_acc.abs() > 0.18 { let step = zoom_acc.signum(); zoom_acc = 0.0; if let Some(to) = zoom_target(&world, view, view.k + step, x * sf, y * sf, w, h) { tween = Some((view, to, Instant::now())); dirty = true; } } }
            Some(Msg::Drag { dx, dy }) => { view.cx -= dx * sf / view.k; view.cy -= dy * sf / view.k; clamp_view(&world, &mut view, w, h); tween = None; dirty = true; }
            Some(Msg::Click { x, y }) => {
                let (px, py) = (x * sf, y * sf);
                let u = (sf * 1.5).round().max(2.0);
                // the chips first
                let mut ux = 16.0 * u;
                let mut chip_hit = None;
                for (key, target) in [("bld", 9usize), ("f2", 1), ("f1", 0)] {
                    let r = world.sprites[world.ui.iter().find(|(k2, _)| k2 == key).unwrap().1];
                    if px >= ux && px < ux + r.w as f32 * u && py >= 14.0 * u && py < 14.0 * u + r.h as f32 * u { chip_hit = Some(target); }
                    ux += (r.w as f32 + 6.0) * u;
                }
                if let Some(tg) = chip_hit {
                    let to = if tg == 9 { building(w, h) } else { focus_floor(tg, w, h) };
                    tween = Some((view, to, Instant::now()));
                } else {
                    let (cxp, cyp) = (view.cx + (px - w as f32 / 2.0) / view.k, view.cy + (py - h as f32 / 2.0) / view.k);
                    match world.hit(cxp, cyp) {
                        Some(Target::Floor(f)) if view.k < levels(&world, w, h)[1] => tween = Some((view, focus_floor(f, w, h), Instant::now())),
                        Some(t @ (Target::App(_) | Target::Session(..))) => on_open(t),
                        _ => {}
                    }
                }
            }
            Some(Msg::Key(c)) => match c {
                '1' | '2' => { let to = focus_floor(if c == '2' { 1 } else { 0 }, w, h); tween = Some((view, to, Instant::now())); }
                '0' | 'b' => tween = Some((view, building(w, h), Instant::now())),
                '+' | '=' => { if let Some(to) = zoom_target(&world, view, view.k + 1.0, w as f32 / 2.0, h as f32 / 2.0, w, h) { tween = Some((view, to, Instant::now())); dirty = true; } }
                '-' => { if let Some(to) = zoom_target(&world, view, view.k - 1.0, w as f32 / 2.0, h as f32 / 2.0, w, h) { tween = Some((view, to, Instant::now())); dirty = true; } }
                'r' => { alert = !alert; dirty = true; }
                _ => {}
            },
            Some(Msg::Tour(dir)) => { tour(&gpu, &world, &dir, 2556, 1596, 2.0, &mut inst, &mut quads, start); dirty = true; }
            _ => {}
        }
        let now = Instant::now();
        if let Some((a, b, t0)) = tween {
            let k = ((now - t0).as_secs_f32() / 0.35).min(1.0);
            let e = 1.0 - (1.0 - k).powi(3);
            // zoom changes in whole steps at the end, so the pixels stay crisp throughout
            view = View { k: if k >= 1.0 { b.k } else if (b.k - a.k).abs() > 0.0 && k > 0.5 { b.k } else { a.k }, cx: a.cx + (b.cx - a.cx) * e, cy: a.cy + (b.cy - a.cy) * e };
            if k >= 1.0 { tween = None; view = b; }
            dirty = true;
        }
        if (animating || alert) && visible && now >= next_tick {
            next_tick += period;
            if next_tick < now { next_tick = now + period; }
            // a tick only draws if a frame index actually changed in view
            let t = start.elapsed().as_secs_f32();
            let frame_ids: Vec<usize> = world.actors.iter().map(|a| ((t * a.rate + a.phase).floor().max(0.0) as usize) % a.frames.len()).collect();
            if alert || !world.walkers.is_empty() || last_drawn.as_ref() != Some(&frame_ids) { dirty = true; }
            last_drawn = Some(frame_ids);
        }
        if (animating || alert || !visible) && now >= next_check {
            next_check = now + Duration::from_millis(500);
            stats.lock().unwrap().visibility_checks += 1;
            ask_visible(tx.clone());
        }
        if dirty && visible {
            match present(&gpu, view, w, h, sf, alert, &mut inst, &mut quads) { Some(a) => { animating = a && anim_fps > 0; retry = false; } None => retry = true }
        }
    }
}

/// The zoom steps for this window: building (fits the building), floor (fits a floor), area, desk.
/// Each is a whole number of device pixels per art pixel.
fn levels(world: &World, w: u32, h: u32) -> [f32; 4] {
    let kb = ((w as f32 / world.w as f32).min(h as f32 / world.h as f32)).floor().max(1.0);
    let kf = (w as f32 / (world.w as f32 - 30.0)).floor().max(kb);
    [kb, kf, kf + 1.0, kf + 2.0]
}

fn clamp_view(world: &World, v: &mut View, w: u32, h: u32) {
    let (hw, hh) = (w as f32 / 2.0 / v.k, h as f32 / 2.0 / v.k);
    let (ww, wh) = (world.w as f32, world.h as f32);
    v.cx = if 2.0 * hw >= ww { ww / 2.0 } else { v.cx.clamp(hw, ww - hw) };
    v.cy = if 2.0 * hh >= wh { wh / 2.0 } else { v.cy.clamp(hh, wh - hh) };
}

/// Zooms to `nk` (a whole number, 1..4) keeping the canvas point under (px, py) still.
fn zoom_target(world: &World, view: View, nk: f32, px: f32, py: f32, w: u32, h: u32) -> Option<View> {
    let lv = levels(world, w, h);
    let nk = nk.clamp(lv[0], lv[3]);
    if (nk - view.k).abs() < 0.01 { return None; }
    let (cxp, cyp) = (view.cx + (px - w as f32 / 2.0) / view.k, view.cy + (py - h as f32 / 2.0) / view.k);
    let mut to = View { k: nk, cx: cxp - (px - w as f32 / 2.0) / nk, cy: cyp - (py - h as f32 / 2.0) / nk };
    clamp_view(world, &mut to, w, h);
    Some(to)
}

/// Draws one frame of `world` from `view` at time `t` into `target`; says whether anything animated is in view.
#[allow(clippy::too_many_arguments)]
fn draw_frame(world: &World, gpu: &Gpu, view: View, t: f32, target: &wgpu::TextureView, w: u32, h: u32, sf: f32, alert: bool, inst: &mut Vec<Inst>, quads: &mut Vec<Quad>) -> bool {
        let k = view.k;
    let (hw, hh) = (w as f32 / 2.0 / k, h as f32 / 2.0 / k);
    // snap the camera to whole device pixels
    let cx = (view.cx * k).round() / k;
    let cy = (view.cy * k).round() / k;
    let vis = (cx - hw, cy - hh, cx + hw, cy + hh);
    let anim = world.instances(t, vis, inst);
    let (aw, ah) = (world.atlas.w as f32, world.atlas.h as f32);
    quads.clear();
    let quad = |s: usize, x: f32, y: f32, tint: [f32; 4]| -> Quad {
        let r = world.sprites[s];
        Quad { dst: [x, y, r.w as f32, r.h as f32], src: [r.x as f32 / aw, r.y as f32 / ah, (r.x + r.w) as f32 / aw, (r.y + r.h) as f32 / ah], tint }
    };
    for i in inst.iter() { quads.push(quad(i.s, i.x, i.y, [1.0; 4])); }
    let world_n = quads.len();
    // overlay, in device pixels: UI art is drawn at 2 device pixels per pixel (1 on a 1x screen)
    let lv = levels(world, w, h);
    let u = (sf * 1.5).round().max(2.0);
    let ui = |key: &str| world.ui.iter().find(|(k2, _)| k2 == key).map(|(_, s)| *s).unwrap();
    let mut ux = 16.0 * u;
    for (key, fl) in [("bld", None), ("f2", Some(1usize)), ("f1", Some(0usize))] {
        let s = ui(key);
        let r = world.sprites[s];
        let active = match fl { Some(f) => k >= lv[1] && (cy - world.floors[if f == 1 { 0 } else { 1 }].y - world.floors[0].h / 2.0).abs() < world.floors[0].h / 2.0, None => k < lv[1] };
        let mut q = quad(s, ux, 14.0 * u, if active { [1.0, 1.0, 1.0, 1.0] } else { [0.72, 0.74, 0.8, 0.92] });
        q.dst[2] *= u; q.dst[3] *= u;
        quads.push(q);
        if active { let mut b = quad(world.white, ux, 14.0 * u + r.h as f32 * u, [0.53, 0.53, 0.96, 1.0]); b.dst[2] = r.w as f32 * u; b.dst[3] = 2.0 * u; quads.push(b); }
        ux += (r.w as f32 + 6.0) * u;
    }
    // far out, the areas' names float over them at a readable size
    if k < lv[1] {
        for l in &world.labels {
            let r = world.sprites[l.s];
            let (sx, sy) = ((l.x - cx) * k + w as f32 / 2.0, (l.y - cy) * k + h as f32 / 2.0);
            let lu = (u - 1.0).max(1.0);
            let mut q = quad(l.s, (sx - r.w as f32 * lu / 2.0).round(), (sy - r.h as f32 * lu).round(), [1.0; 4]);
            q.dst[2] *= lu; q.dst[3] *= lu;
            quads.push(q);
        }
    }
    if alert {
        let pulse = 0.12 + 0.06 * (0.5 + 0.5 * (t * std::f32::consts::PI).sin());
        let mut q = quad(world.white, 0.0, 0.0, [0.9, 0.12, 0.12, pulse]);
        q.dst[2] = w as f32; q.dst[3] = h as f32;
        quads.push(q);
        let s = ui("alert");
        let r = world.sprites[s];
        let mut b = quad(s, (w as f32 - r.w as f32 * u) / 2.0, 14.0 * u, [1.0; 4]);
        b.dst[2] *= u; b.dst[3] *= u;
        quads.push(b);
        for (bx, by) in &world.alert_beacons {
            let f = world.beacon[((t * 3.0) as usize) % 2];
            let (sx, sy) = ((bx - cx) * k + w as f32 / 2.0, (by - cy) * k + h as f32 / 2.0);
            let mut q = quad(f, sx, sy, [1.0; 4]);
            q.dst[2] *= k; q.dst[3] *= k;
            quads.push(q);
        }
    }
    quads.truncate(MAX_QUADS);
    gpu.queue.write_buffer(&gpu.quads_buf, 0, bytemuck::cast_slice(quads));
    let (sx, sy) = (2.0 * k / w as f32, -2.0 * k / h as f32);
    gpu.queue.write_buffer(&gpu.world_g.0, 0, bytemuck::cast_slice(&[sx, sy, -cx * sx, -cy * sy]));
    gpu.queue.write_buffer(&gpu.ui_g.0, 0, bytemuck::cast_slice(&[2.0 / w as f32, -2.0 / h as f32, -1.0, 1.0]));
    let mut enc = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: target, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0706, g: 0.0588, b: 0.1216, a: 1.0 }), store: wgpu::StoreOp::Store } })],
            ..Default::default()
        });
        rp.set_pipeline(&gpu.pipeline);
        rp.set_bind_group(0, &gpu.world_g.1, &[]);
        rp.set_bind_group(1, &gpu.bg_group, &[]);
        rp.set_vertex_buffer(0, gpu.bg_quad.slice(..));
        rp.draw(0..6, 0..1);
        rp.set_bind_group(1, &gpu.atlas_group, &[]);
        rp.set_vertex_buffer(0, gpu.quads_buf.slice(..));
        rp.draw(0..6, 0..world_n as u32);
        rp.set_bind_group(0, &gpu.ui_g.1, &[]);
        rp.draw(0..6, world_n as u32..quads.len() as u32);
    }
    gpu.queue.submit([enc.finish()]);
    anim
}

/// A scripted walk through the zoom levels, each step captured as a frame (raw RGBA, 8-byte header).
#[allow(clippy::too_many_arguments)]
fn tour(gpu: &Gpu, world: &World, dir: &str, w: u32, h: u32, sf: f32, inst: &mut Vec<Inst>, quads: &mut Vec<Quad>, start: Instant) {
    let _ = std::fs::create_dir_all(dir);
    let f0 = world.floors[1];
    let f1 = world.floors[0];
    let a1 = |col: f32| world.w as f32 * (0.215 + col * 0.198);
    let lv = levels(world, w, h);
    let (kb, kf, ka, kd) = (lv[0], lv[1], lv[2], lv[3]);
    let steps: Vec<(View, bool, usize, &str)> = vec![
        (View { k: kb, cx: world.w as f32 / 2.0, cy: world.h as f32 / 2.0 }, false, 3, "building"),
        (View { k: kf, cx: world.w as f32 / 2.0, cy: f0.y + f0.h / 2.0 }, false, 4, "floor1"),
        (View { k: ka, cx: a1(0.0), cy: f0.y + f0.h * 0.55 }, false, 4, "area-workshop"),
        (View { k: kd, cx: a1(0.15), cy: f0.y + f0.h * 0.45 }, false, 5, "desk"),
        (View { k: ka, cx: a1(1.0), cy: f0.y + f0.h * 0.55 }, false, 3, "area-industrial"),
        (View { k: ka, cx: a1(2.0), cy: f0.y + f0.h * 0.55 }, false, 3, "area-studio"),
        (View { k: ka, cx: a1(3.0), cy: f0.y + f0.h * 0.55 }, false, 3, "area-garage"),
        (View { k: kf, cx: world.w as f32 / 2.0, cy: f1.y + f1.h / 2.0 }, false, 3, "floor2"),
        (View { k: ka, cx: a1(1.0), cy: f1.y + f1.h * 0.55 }, false, 2, "area-unused"),
        (View { k: kf, cx: world.w as f32 / 2.0, cy: f0.y + f0.h / 2.0 }, true, 3, "red-alert"),
    ];
    let mut n = 0;
    for (view, alert, frames, name) in steps {
        for f in 0..frames {
            let t = start.elapsed().as_secs_f32() + f as f32 * 0.3;
            let tex = gpu.device.create_texture(&wgpu::TextureDescriptor { label: None, size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: gpu.format, usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC, view_formats: &[] });
            draw_frame(world, gpu, view, t, &tex.create_view(&wgpu::TextureViewDescriptor::default()), w, h, sf, alert, inst, quads);
            let row = (w * 4).div_ceil(256) * 256;
            let buf = gpu.device.create_buffer(&wgpu::BufferDescriptor { label: None, size: (row * h) as u64, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });
            let mut enc = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            enc.copy_texture_to_buffer(wgpu::TexelCopyTextureInfo { texture: &tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All }, wgpu::TexelCopyBufferInfo { buffer: &buf, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(h) } }, wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 });
            gpu.queue.submit([enc.finish()]);
            let slice = buf.slice(..);
            let (tx, rx) = channel();
            slice.map_async(wgpu::MapMode::Read, move |r| { let _ = tx.send(r); });
            let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
            if rx.recv().map(|r| r.is_ok()).unwrap_or(false) {
                let Ok(data) = slice.get_mapped_range() else { continue };
                let bgra = matches!(gpu.format, wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb);
                let mut out = Vec::with_capacity((w * h * 4 + 8) as usize);
                out.extend_from_slice(&w.to_le_bytes());
                out.extend_from_slice(&h.to_le_bytes());
                for y in 0..h { for x in 0..w { let i = (y * row + x * 4) as usize; if bgra { out.extend_from_slice(&[data[i + 2], data[i + 1], data[i], 255]); } else { out.extend_from_slice(&[data[i], data[i + 1], data[i + 2], 255]); } } }
                let _ = std::fs::write(format!("{dir}/{n:02}-{name}.rgba"), out);
            }
            n += 1;
            std::thread::sleep(Duration::from_millis(60));
        }
    }
    println!("{}", serde_json::json!({ "event": "tour_done", "frames": n }));
}

fn make_gpu(device: wgpu::Device, queue: wgpu::Queue, format: wgpu::TextureFormat, world: &World) -> Gpu {
    let tex_format = if format.is_srgb() { wgpu::TextureFormat::Rgba8UnormSrgb } else { wgpu::TextureFormat::Rgba8Unorm };
    let upload = |w: i32, h: i32, px: &[u8]| {
        device.create_texture_with_data(&queue, &wgpu::TextureDescriptor { label: None, size: wgpu::Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: tex_format, usage: wgpu::TextureUsages::TEXTURE_BINDING, view_formats: &[] }, wgpu::util::TextureDataOrder::LayerMajor, px)
            .create_view(&wgpu::TextureViewDescriptor::default())
    };
    let bg_view = upload(world.bg.w, world.bg.h, &world.bg.px);
    let atlas_view = upload(world.atlas.w, world.atlas.h, &world.atlas.px);
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor { mag_filter: wgpu::FilterMode::Nearest, min_filter: wgpu::FilterMode::Nearest, ..Default::default() });
    let gl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: None, entries: &[wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::VERTEX, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None }] });
    let tl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: None, entries: &[
        wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
        wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
    ] });
    let globals = |device: &wgpu::Device| {
        let b = device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: bytemuck::cast_slice(&[1.0f32, 1.0, 0.0, 0.0]), usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST });
        let g = device.create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout: &gl, entries: &[wgpu::BindGroupEntry { binding: 0, resource: b.as_entire_binding() }] });
        (b, g)
    };
    let world_g = globals(&device);
    let ui_g = globals(&device);
    let tex_group = |v: &wgpu::TextureView| device.create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout: &tl, entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(v) }, wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) }] });
    let bg_group = tex_group(&bg_view);
    let atlas_group = tex_group(&atlas_view);
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: None, source: wgpu::ShaderSource::Wgsl(SHADER.into()) });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[Some(&gl), Some(&tl)], ..Default::default() });
    let attrs = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4];
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("sprites"),
        layout: Some(&layout),
        vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs"), compilation_options: Default::default(), buffers: &[Some(wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<Quad>() as u64, step_mode: wgpu::VertexStepMode::Instance, attributes: &attrs })] },
        fragment: Some(wgpu::FragmentState { module: &shader, entry_point: Some("fs"), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })] }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });
    let quads_buf = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: (MAX_QUADS * std::mem::size_of::<Quad>()) as u64, usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    let bg_quad = device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: None, contents: bytemuck::cast_slice(&[Quad { dst: [0.0, 0.0, world.w as f32, world.h as f32], src: [0.0, 0.0, 1.0, 1.0], tint: [1.0; 4] }]), usage: wgpu::BufferUsages::VERTEX });
    Gpu { device, queue, format, pipeline, world_g, ui_g, bg_group, atlas_group, bg_quad, quads_buf }
}
