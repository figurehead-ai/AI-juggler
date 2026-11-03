//! The Floor's native window, drawn with wgpu in four passes:
//! 1. shadow: the sun's depth map (one light direction for the whole office);
//! 2. scene: low-poly models at a reduced resolution (the pixel size), writing colour, depth and
//!    a pick id per pixel; glass after, blended;
//! 3. post: the low-resolution frame scaled up with hard edges, outlines where an object sits in
//!    front of another, the hovered thing outlined in light, sky, the red-alert wash;
//! 4. overlay: crisp screen-space chips, balloons, tooltips and the floor switcher.
//! The camera orbits a target: drag pans, right-drag or ⌥-drag (or a two-finger twist) rotates and
//! tilts within limits, pinch or ⌘-scroll zooms, H resets. Frames are drawn only on a change: input,
//! a tween, or an animation tick while something moving is in view and the window can be seen.

use crate::layout::{OfficeState, AISLE, DEPTH, ROW_D, STOREY};
use crate::math::{add, cross, norm, scale, sub, M4, V3};
use crate::mesh::{Library, Mesh, Vertex, ALL};
use crate::models::{Inst, Item};
use crate::scene::{floor_y, Light, Live, Mark, Scene, Target};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use wgpu::util::DeviceExt;

pub enum Msg {
    Resize(u32, u32, f32),
    Visible(bool),
    /// Scroll in points: precise (trackpad) pans, a wheel or ⌘ zooms; position in points from the top left.
    Scroll { dx: f32, dy: f32, precise: bool, x: f32, y: f32, zoom: bool },
    Magnify { m: f32, x: f32, y: f32 },
    /// A two-finger twist on the trackpad, in degrees.
    Twist(f32),
    Drag { dx: f32, dy: f32, rotate: bool },
    Move { x: f32, y: f32 },
    Click { x: f32, y: f32 },
    Key(char),
    Tour(String),
    Recreate,
}

#[derive(Default, Clone, Copy, serde::Serialize)]
pub struct Stats { pub frames: u64, pub draw_ms: f64, pub first_frame_ms: f64, pub visibility_checks: u64, pub instances: u64 }

pub type OnOpen = Arc<dyn Fn(Target, String) + Send + Sync>;

pub struct Floor { pub tx: Sender<Msg>, pub stats: Arc<Mutex<Stats>> }

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Quad { dst: [f32; 4], src: [f32; 4], tint: [f32; 4] }
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Globals { vp: [[f32; 4]; 4], lvp: [[f32; 4]; 4], eye: [f32; 4], sun: [f32; 4], sky: [f32; 4], ground: [f32; 4], params: [f32; 4] }
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct PostU { a: [f32; 4], b: [f32; 4], c: [f32; 4], top: [f32; 4], bot: [f32; 4], hov: [u32; 4] }
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuLight { p: [f32; 4], c: [f32; 4] }

const MAX_DYN: usize = 60_000;
const MAX_LIGHTS: usize = 96;
const MAX_QUADS: usize = 8192;
const SHADOW: u32 = 2048;
const FOV: f32 = 0.5;
const NEAR: f32 = 0.5;
const FAR: f32 = 700.0;
const SUN: V3 = [-0.42, 0.82, 0.38];

const SCENE_WGSL: &str = r#"
struct G { vp: mat4x4<f32>, lvp: mat4x4<f32>, eye: vec4<f32>, sun: vec4<f32>, sky: vec4<f32>, ground: vec4<f32>, params: vec4<f32> };
struct Lt { p: vec4<f32>, c: vec4<f32> };
@group(0) @binding(0) var<uniform> g: G;
@group(0) @binding(1) var<storage, read> lights: array<Lt>;
@group(0) @binding(2) var atlas: texture_2d<f32>;
@group(0) @binding(3) var samp: sampler;
@group(0) @binding(4) var smap: texture_depth_2d;
@group(0) @binding(5) var ssamp: sampler_comparison;
struct VI {
  @location(0) pos: vec3<f32>, @location(1) n: vec3<f32>, @location(2) uv: vec2<f32>,
  @location(3) m0: vec4<f32>, @location(4) m1: vec4<f32>, @location(5) m2: vec4<f32>, @location(6) m3: vec4<f32>,
  @location(7) col: vec4<f32>, @location(8) tex: vec4<f32>, @location(9) misc: vec4<f32>, @location(10) off: vec2<f32>, @location(11) id: u32,
};
struct VO {
  @builtin(position) pos: vec4<f32>,
  @location(0) wp: vec3<f32>, @location(1) n: vec3<f32>, @location(2) uv: vec2<f32>,
  @location(3) col: vec4<f32>, @location(4) tex: vec4<f32>, @location(5) misc: vec4<f32>, @location(6) off: vec2<f32>,
  @location(7) @interpolate(flat) id: u32,
};
// things on a wall vanish with it when the camera is behind that wall (the cutaway)
fn culled(v: VI) -> bool {
  let flags = u32(v.misc.w + 0.5);
  if ((flags & 2u) == 0u) { return false; }
  return dot(g.eye.xyz - v.m3.xyz, normalize(v.m2.xyz)) < 0.0;
}
@vertex fn vs(v: VI) -> VO {
  let m = mat4x4<f32>(v.m0, v.m1, v.m2, v.m3);
  let wp = (m * vec4<f32>(v.pos, 1.0)).xyz;
  var o: VO;
  o.wp = wp;
  o.n = (m * vec4<f32>(v.n, 0.0)).xyz;
  o.pos = g.vp * vec4<f32>(wp, 1.0);
  if (culled(v)) { o.pos = vec4<f32>(0.0, 0.0, 2.0, 1.0); }
  o.uv = v.uv; o.col = v.col; o.tex = v.tex; o.misc = v.misc; o.off = v.off; o.id = v.id;
  return o;
}
@vertex fn vs_shadow(v: VI) -> @builtin(position) vec4<f32> {
  let m = mat4x4<f32>(v.m0, v.m1, v.m2, v.m3);
  if (culled(v)) { return vec4<f32>(0.0, 0.0, 2.0, 1.0); }
  return g.lvp * m * vec4<f32>(v.pos, 1.0);
}
struct FO { @location(0) c: vec4<f32>, @location(1) id: u32 };
fn shade(v: VO) -> vec4<f32> {
  var base = v.col;
  let flags = u32(v.misc.w + 0.5);
  if ((flags & 1u) != 0u) {
    let uv = v.tex.xy + fract(v.uv * v.misc.xy + v.off) * v.tex.zw;
    base = base * textureSampleLevel(atlas, samp, uv, 0.0);
  }
  let n = normalize(v.n);
  let lp = g.lvp * vec4<f32>(v.wp + n * 0.05, 1.0);
  let suv = clamp(vec2<f32>(lp.x * 0.5 + 0.5, 0.5 - lp.y * 0.5), vec2<f32>(0.0), vec2<f32>(1.0));
  let sh = textureSampleCompareLevel(smap, ssamp, suv, lp.z - 0.002);
  let ndl = max(dot(n, g.sun.xyz), 0.0);
  // the sun in three bands, for a painted look
  let sun = floor(ndl * sh * 3.0 + 0.4) / 3.0 * g.sun.w;
  let hemi = mix(g.ground.rgb, g.sky.rgb, n.y * 0.5 + 0.5);
  var light = hemi + vec3<f32>(1.0, 0.9, 0.76) * sun;
  let nl = u32(g.params.z);
  for (var i = 0u; i < nl; i = i + 1u) {
    let L = lights[i];
    let d = L.p.xyz - v.wp;
    let dist = length(d);
    if (dist < L.p.w) {
      let att = 1.0 - dist / L.p.w;
      light = light + L.c.rgb * att * att * (0.4 + 0.6 * max(dot(n, d / max(dist, 0.001)), 0.0));
    }
  }
  var c = base.rgb * light;
  c = mix(c, base.rgb * 1.1, clamp(v.misc.z, 0.0, 1.0));
  return vec4<f32>(c, base.a);
}
@fragment fn fs(v: VO) -> FO { var o: FO; o.c = shade(v); o.id = v.id; return o; }
@fragment fn fs_glass(v: VO) -> FO { var o: FO; let c = shade(v); o.c = vec4<f32>(c.rgb, v.col.a); o.id = 0u; return o; }
"#;

const POST_WGSL: &str = r#"
struct P { a: vec4<f32>, b: vec4<f32>, c: vec4<f32>, top: vec4<f32>, bot: vec4<f32>, hov: vec4<u32> };
@group(0) @binding(0) var<uniform> p: P;
@group(0) @binding(1) var col: texture_2d<f32>;
@group(0) @binding(2) var ids: texture_2d<u32>;
@group(0) @binding(3) var dep: texture_depth_2d;
@vertex fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
  let x = f32((i << 1u) & 2u);
  let y = f32(i & 2u);
  return vec4<f32>(x * 2.0 - 1.0, y * 2.0 - 1.0, 0.0, 1.0);
}
fn lin(d: f32) -> f32 { let n = p.b.z; let f = p.b.w; return n * f / (f - d * (f - n)); }
@fragment fn fs(@builtin(position) fc: vec4<f32>) -> @location(0) vec4<f32> {
  let size = vec2<i32>(i32(p.a.x), i32(p.a.y));
  let q = clamp(vec2<i32>(floor(fc.xy / p.a.z)), vec2<i32>(0), size - vec2<i32>(1));
  let draw = textureLoad(dep, q, 0);
  let id0 = textureLoad(ids, q, 0).r;
  var c = textureLoad(col, q, 0).rgb;
  let hov = p.hov.x;
  if (draw >= 1.0) {
    let y = fc.y / p.c.y;
    c = mix(p.top.rgb, p.bot.rgb, smoothstep(0.0, 1.0, y));
  } else {
    let d0 = lin(draw);
    var edge = false;
    var hl = false;
    let offs = array<vec2<i32>, 4>(vec2<i32>(1, 0), vec2<i32>(-1, 0), vec2<i32>(0, 1), vec2<i32>(0, -1));
    for (var k = 0; k < 4; k = k + 1) {
      let qq = clamp(q + offs[k], vec2<i32>(0), size - vec2<i32>(1));
      let idn = textureLoad(ids, qq, 0).r;
      let dn = textureLoad(dep, qq, 0);
      if (idn != id0 && dn < 1.0 && lin(dn) < d0 * 0.97) { edge = true; }
      if (hov != 0u && id0 == hov && idn != hov) { hl = true; }
    }
    if (edge) { c = c * 0.45 + vec3<f32>(0.04, 0.03, 0.07); }
    if (hl) { c = vec3<f32>(1.0, 0.93, 0.6); }
  }
  if (p.b.y > 0.5) { c = mix(c, vec3<f32>(0.95, 0.12, 0.1), 0.1 + 0.07 * sin(p.b.x * 3.1416)); }
  let uv = fc.xy / p.c.xy;
  let v = smoothstep(0.95, 0.4, length((uv - vec2<f32>(0.5)) * vec2<f32>(1.0, 0.8)));
  c = c * mix(0.72, 1.0, v);
  return vec4<f32>(c, 1.0);
}
"#;

const OVERLAY_WGSL: &str = r#"
struct U { s: vec2<f32>, o: vec2<f32> };
@group(0) @binding(0) var<uniform> u: U;
@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;
struct VOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) tint: vec4<f32> };
@vertex fn vs(@builtin(vertex_index) vi: u32, @location(0) dst: vec4<f32>, @location(1) src: vec4<f32>, @location(2) tint: vec4<f32>) -> VOut {
  var c = array<vec2<f32>, 6>(vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0), vec2(0.0, 1.0), vec2(1.0, 0.0), vec2(1.0, 1.0));
  let k = c[vi];
  let p = dst.xy + k * dst.zw;
  var o: VOut;
  o.pos = vec4(p.x * u.s.x + u.o.x, p.y * u.s.y + u.o.y, 0.0, 1.0);
  o.uv = src.xy + k * src.zw;
  o.tint = tint;
  return o;
}
@fragment fn fs(v: VOut) -> @location(0) vec4<f32> {
  let c = textureSample(tex, samp, v.uv) * v.tint;
  if (c.a < 0.02) { discard; }
  return c;
}
"#;

#[derive(Clone, Copy)]
struct Batch { glass: bool, floor: i32, mesh: Mesh, start: u32, count: u32 }

struct Targets { w: u32, h: u32, color: wgpu::Texture, ids: wgpu::Texture, depth: wgpu::Texture, post_group: wgpu::BindGroup }

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    format: wgpu::TextureFormat,
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
    ui_group: wgpu::BindGroup,
    quads: wgpu::Buffer,
    pick_buf: wgpu::Buffer,
    tg: Option<Targets>,
}

/// The orbit camera: a target on the floor, a turn (yaw), a tilt (pitch) and a distance.
#[derive(Clone, Copy, Debug)]
struct Cam { t: V3, yaw: f32, pitch: f32, dist: f32 }
const PITCH_MIN: f32 = 0.32;
const PITCH_MAX: f32 = 1.38;
const DEFAULT_YAW: f32 = 0.42;
const DEFAULT_PITCH: f32 = 0.82;

impl Cam {
    fn eye(&self) -> V3 { add(self.t, [self.dist * self.pitch.cos() * self.yaw.sin(), self.dist * self.pitch.sin(), self.dist * self.pitch.cos() * self.yaw.cos()]) }
    fn view(&self) -> M4 { M4::look_at(self.eye(), self.t, [0.0, 1.0, 0.0]) }
    fn vp(&self, aspect: f32) -> M4 { M4::perspective(FOV, aspect, NEAR, FAR).mul(&self.view()) }
    /// The point on the plane y = `py` under a screen position (0..1 each way).
    fn ground(&self, sx: f32, sy: f32, aspect: f32, py: f32) -> Option<V3> {
        let e = self.eye();
        let f = norm(sub(self.t, e));
        let r = norm(cross(f, [0.0, 1.0, 0.0]));
        let u = cross(r, f);
        let th = (FOV / 2.0).tan();
        let d = norm(add(f, add(scale(r, (sx * 2.0 - 1.0) * th * aspect), scale(u, (1.0 - sy * 2.0) * th))));
        if d[1].abs() < 1e-4 { return None; }
        let k = (py - e[1]) / d[1];
        (k > 0.0).then(|| add(e, scale(d, k)))
    }
    fn lerp(a: Cam, b: Cam, k: f32) -> Cam {
        let mut dy = b.yaw - a.yaw;
        while dy > std::f32::consts::PI { dy -= std::f32::consts::TAU; }
        while dy < -std::f32::consts::PI { dy += std::f32::consts::TAU; }
        Cam { t: add(a.t, scale(sub(b.t, a.t), k)), yaw: a.yaw + dy * k, pitch: a.pitch + (b.pitch - a.pitch) * k, dist: a.dist * (b.dist / a.dist).powf(k) }
    }
}

struct View { focus: Option<usize>, cam: Cam }

pub fn start(instance: wgpu::Instance, surface: wgpu::Surface<'static>, state_path: PathBuf, size: (u32, u32, f32), anim_fps: u32, opened: Instant, on_open: OnOpen,
    ask_visible: Arc<dyn Fn(Sender<Msg>) + Send + Sync>, alert: bool) -> Floor {
    let (tx, rx) = channel::<Msg>();
    let stats = Arc::new(Mutex::new(Stats::default()));
    let (st, tx2) = (stats.clone(), tx.clone());
    std::thread::Builder::new().name("floor-render".into()).spawn(move || {
        run(instance, surface, state_path, size, anim_fps, opened, rx, tx2, st, on_open, ask_visible, alert);
    }).expect("render thread");
    Floor { tx, stats }
}

fn load_state(path: &PathBuf) -> OfficeState { OfficeState::load(path).unwrap_or_else(|| OfficeState::new(crate::paint::hash("office") ^ 0x5eed)) }

fn build_scene(state: &mut OfficeState, path: Option<&PathBuf>, grow: Option<(&str, usize)>) -> Scene {
    let t0 = Instant::now();
    let (scene, changed) = Scene::build(state, grow);
    if changed { if let Some(p) = path { state.save(p); } }
    println!("{}", serde_json::json!({ "event": "world_built", "ms": t0.elapsed().as_secs_f64() * 1000.0, "floors": scene.floors, "static_parts": scene.stat.len(), "picks": scene.picks.len(), "lights": scene.lights.len(),
        "areas": scene.apps.iter().enumerate().filter_map(|(i, a)| { let k = scene.kits[i].as_ref()?; let pa = &scene.plan.areas[scene.area_of[i]?]; Some(serde_json::json!({ "app": a.name, "family": k.family, "floor": pa.floor + 1, "cell": [pa.col, pa.row], "size": format!("{:?}", pa.size), "variant": k.variant() })) }).collect::<Vec<_>>() }));
    scene
}

#[allow(clippy::too_many_arguments)]
fn run(instance: wgpu::Instance, surface: wgpu::Surface<'static>, state_path: PathBuf, size: (u32, u32, f32), anim_fps: u32, opened: Instant, rx: Receiver<Msg>, tx: Sender<Msg>,
    stats: Arc<Mutex<Stats>>, on_open: OnOpen, ask_visible: Arc<dyn Fn(Sender<Msg>) + Send + Sync>, mut alert: bool) {
    let mut state = load_state(&state_path);
    let mut scene = build_scene(&mut state, Some(&state_path), None);
    let ask = |fallback: bool| pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions { power_preference: wgpu::PowerPreference::LowPower, compatible_surface: Some(&surface), force_fallback_adapter: fallback, ..Default::default() }));
    let adapter = ask(false).or_else(|_| ask(true)).expect("adapter");
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor { label: Some("floor"), ..Default::default() })).expect("device");
    let caps = surface.get_capabilities(&adapter);
    let format = caps.formats.iter().copied().find(|f| !f.is_srgb()).unwrap_or(caps.formats[0]);
    let (mut w, mut h, mut sf) = size;
    let mut config = wgpu::SurfaceConfiguration { usage: wgpu::TextureUsages::RENDER_ATTACHMENT, format, width: w.max(1), height: h.max(1), present_mode: wgpu::PresentMode::Fifo, desired_maximum_frame_latency: 2, alpha_mode: caps.alpha_modes[0], view_formats: vec![], color_space: Default::default() };
    surface.configure(&device, &config);
    let mut gpu = make_gpu(device, queue, format, &scene);
    let start = Instant::now();
    let mut view = View { focus: Some(0), cam: home(&scene, Some(0), w as f32 / h as f32) };
    let mut tween: Option<(Cam, Cam, Instant)> = None;
    let mut live = Live { t: 0.0, clicked: HashMap::new(), lamps_off: HashSet::new(), hover: 0, alert, mac_load: 0.46, eye: [0.0; 3] };
    let mut dyn_items: Vec<Item> = Vec::with_capacity(20_000);
    let mut lights: Vec<Light> = vec![];
    let mut marks: Vec<Mark> = vec![];
    let mut chips: Vec<([f32; 4], Chip)> = vec![];
    let mut mouse: Option<(f32, f32)> = None;
    let mut visible = true;
    let mut animating = false;
    let period = Duration::from_secs_f64(1.0 / anim_fps.max(1) as f64);
    let mut next_tick = Instant::now() + period;
    let mut next_check = Instant::now() + Duration::from_millis(500);

    // None: the surface refused a frame (occluded, outdated); the loop retries shortly.
    let present = |gpu: &mut Gpu, scene: &Scene, view: &View, live: &mut Live, w: u32, h: u32, sf: f32, dyn_items: &mut Vec<Item>, lights: &mut Vec<Light>, marks: &mut Vec<Mark>, chips: &mut Vec<([f32; 4], Chip)>, mouse: Option<(f32, f32)>| -> Option<bool> {
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
        live.t = start.elapsed().as_secs_f32();
        let target = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let anim = draw(gpu, scene, view, live, &target, w, h, sf, dyn_items, lights, marks, chips, mouse);
        gpu.queue.present(frame);
        let _ = gpu.device.poll(wgpu::PollType::Poll);
        let mut s = stats.lock().unwrap();
        s.frames += 1;
        s.instances = (scene.stat.len() + dyn_items.len()) as u64;
        s.draw_ms += t0.elapsed().as_secs_f64() * 1000.0;
        if s.frames == 1 {
            s.first_frame_ms = opened.elapsed().as_secs_f64() * 1000.0;
            println!("{}", serde_json::json!({ "event": "first_frame", "ms": s.first_frame_ms }));
        }
        Some(anim)
    };

    let mut retry = match present(&mut gpu, &scene, &view, &mut live, w, h, sf, &mut dyn_items, &mut lights, &mut marks, &mut chips, mouse) { Some(a) => { animating = a && anim_fps > 0; false } None => true };
    loop {
        let wake = if tween.is_some() { Instant::now() + Duration::from_millis(16) }
            else if retry && visible { Instant::now() + Duration::from_millis(100) }
            else if (animating || alert) && visible { next_tick.min(next_check) }
            else if !visible { next_check }
            else { Instant::now() + Duration::from_secs(3600) };
        let first = match rx.recv_timeout(wake.saturating_duration_since(Instant::now())) {
            Ok(m) => Some(m),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => return,
        };
        // take everything waiting; of many mouse moves only the last matters
        let mut msgs: Vec<Msg> = first.into_iter().collect();
        while let Ok(m) = rx.try_recv() { msgs.push(m); }
        let last_move = msgs.iter().rposition(|m| matches!(m, Msg::Move { .. }));
        let mut dirty = retry;
        let aspect = w as f32 / h.max(1) as f32;
        for (mi, msg) in msgs.into_iter().enumerate() {
            if matches!(msg, Msg::Move { .. }) && Some(mi) != last_move { continue; }
            let base = tween.map(|(_, b, _)| b).unwrap_or(view.cam);
            match msg {
                Msg::Resize(nw, nh, nsf) if nw > 0 && nh > 0 => { w = nw; h = nh; sf = nsf; config.width = w; config.height = h; surface.configure(&gpu.device, &config); dirty = true; }
                Msg::Visible(v) => { if v != visible { println!("{}", serde_json::json!({ "event": "visible", "v": v, "ms": opened.elapsed().as_millis() as u64 })); } if v && !visible { dirty = true; } visible = v; }
                Msg::Scroll { dx, dy, precise, x, y, zoom } => {
                    if precise && !zoom { tween = None; pan(&mut view.cam, dx * sf, dy * sf, h); dirty = true; }
                    else { let step = if precise { dy / 30.0 } else { dy.signum() }; tween = None; zoom_at(&mut view, &scene, step, x * sf / w as f32, y * sf / h as f32, aspect); dirty = true; }
                }
                Msg::Magnify { m, x, y } => { tween = None; zoom_at(&mut view, &scene, m * 4.0, x * sf / w as f32, y * sf / h as f32, aspect); dirty = true; }
                Msg::Twist(deg) => { tween = None; view.cam.yaw -= deg.to_radians(); dirty = true; }
                Msg::Drag { dx, dy, rotate } => {
                    tween = None;
                    if rotate { view.cam.yaw -= dx * 0.008; view.cam.pitch = (view.cam.pitch + dy * 0.006).clamp(PITCH_MIN, PITCH_MAX); }
                    else { pan(&mut view.cam, dx * sf, dy * sf, h); }
                    dirty = true;
                }
                Msg::Move { x, y } => {
                    mouse = Some((x * sf, y * sf));
                    let id = gpu.pick(x * sf, y * sf, sf);
                    if id != live.hover { live.hover = id; dirty = true; }
                    else if live.hover != 0 { dirty = true; }
                }
                Msg::Click { x, y } => {
                    let (px, py) = (x * sf, y * sf);
                    if let Some((_, c)) = chips.iter().find(|(r, _)| px >= r[0] && py >= r[1] && px < r[0] + r[2] && py < r[1] + r[3]) {
                        let to = match c { Chip::Building => None, Chip::Floor(f) => Some(*f) };
                        view.focus = to;
                        tween = Some((view.cam, home(&scene, to, aspect).with_yaw(base.yaw), Instant::now()));
                        dirty = true;
                    } else {
                        let id = gpu.pick(px, py, sf);
                        if let Some(p) = scene.pick_of(id) {
                            match p.target {
                                Target::Floor(f) if view.focus != Some(f) => { view.focus = Some(f); tween = Some((view.cam, home(&scene, Some(f), aspect).with_yaw(base.yaw), Instant::now())); }
                                Target::Prop(_) | Target::Cat => { live.clicked.insert(id, live.t); }
                                Target::Floor(_) => {}
                                t => on_open(t, p.label.clone()),
                            }
                            println!("{}", serde_json::json!({ "event": "click", "what": p.label }));
                            dirty = true;
                        }
                    }
                }
                Msg::Key(c) => {
                    let to = match c {
                        '1'..='9' => { let f = c as usize - '1' as usize; (f < scene.floors).then(|| { view.focus = Some(f); home(&scene, Some(f), aspect).with_yaw(base.yaw) }) }
                        '0' | 'b' => { view.focus = None; Some(home(&scene, None, aspect).with_yaw(base.yaw)) }
                        'h' => Some(home(&scene, view.focus, aspect)),
                        'q' | '\u{f702}' => Some(Cam { yaw: base.yaw + std::f32::consts::FRAC_PI_4, ..base }),
                        'e' | '\u{f703}' => Some(Cam { yaw: base.yaw - std::f32::consts::FRAC_PI_4, ..base }),
                        '\u{f700}' => Some(Cam { pitch: (base.pitch + 0.2).min(PITCH_MAX), ..base }),
                        '\u{f701}' => Some(Cam { pitch: (base.pitch - 0.2).max(PITCH_MIN), ..base }),
                        '+' | '=' => Some(Cam { dist: (base.dist * 0.75).max(4.0), ..base }),
                        '-' => Some(Cam { dist: (base.dist / 0.75).min(max_dist(&scene)), ..base }),
                        'a' => { alert = !alert; live.alert = alert; dirty = true; None }
                        'g' => {
                            // a demo of reflow: one App grows, and the areas after it move on in order (not saved)
                            let mut demo = state.clone();
                            scene = build_scene(&mut demo, None, Some(("app-3", 9)));
                            gpu.load(&scene);
                            dirty = true;
                            None
                        }
                        _ => None,
                    };
                    if let Some(to) = to { tween = Some((view.cam, to, Instant::now())); }
                }
                Msg::Recreate => {
                    state = OfficeState::new(crate::paint::hash(&format!("office{}", start.elapsed().as_nanos())));
                    scene = build_scene(&mut state, Some(&state_path), None);
                    gpu.load(&scene);
                    dirty = true;
                }
                Msg::Tour(dir) => { tour(&mut gpu, &scene, &state, &dir, &mut live, &mut dyn_items, &mut lights, &mut marks, &mut chips); gpu.load(&scene); dirty = true; }
                _ => {}
            }
        }
        // zooming far out shows the whole building
        if tween.is_none() && view.focus.is_some() && view.cam.dist > home(&scene, view.focus, aspect).dist * 1.55 { view.focus = None; dirty = true; }
        let now = Instant::now();
        if let Some((a, b, t0)) = tween {
            let k = ((now - t0).as_secs_f32() / 0.45).min(1.0);
            let e = 1.0 - (1.0 - k).powi(3);
            view.cam = Cam::lerp(a, b, e);
            if k >= 1.0 { tween = None; view.cam = b; }
            dirty = true;
        }
        if (animating || alert) && visible && now >= next_tick {
            next_tick += period;
            if next_tick < now { next_tick = now + period; }
            dirty = true;
        }
        if (animating || alert || !visible) && now >= next_check {
            next_check = now + Duration::from_millis(500);
            stats.lock().unwrap().visibility_checks += 1;
            ask_visible(tx.clone());
        }
        if dirty && visible {
            match present(&mut gpu, &scene, &view, &mut live, w, h, sf, &mut dyn_items, &mut lights, &mut marks, &mut chips, mouse) { Some(a) => { animating = a && anim_fps > 0; retry = false; } None => retry = true }
        }
    }
}

impl Cam { fn with_yaw(self, yaw: f32) -> Cam { Cam { yaw, ..self } } }

fn max_dist(scene: &Scene) -> f32 { scene.width * 3.2 }

/// The default view of a floor (or the building), framed to fit.
fn home(scene: &Scene, focus: Option<usize>, aspect: f32) -> Cam {
    let half_w = scene.width / 2.0 + 2.0;
    let fit = half_w / ((FOV / 2.0).tan() * aspect.max(0.5));
    match focus {
        Some(f) => Cam { t: [scene.width / 2.0, floor_y(f), DEPTH / 2.0 + 0.5], yaw: DEFAULT_YAW, pitch: DEFAULT_PITCH, dist: fit * 0.98 },
        None => {
            let top = floor_y(scene.floors - 1);
            Cam { t: [scene.width / 2.0, top / 2.0 + 1.0, DEPTH / 2.0], yaw: DEFAULT_YAW, pitch: 0.5, dist: (fit * 1.05).max((top + 14.0) / ((FOV / 2.0).tan() * 2.0) * 1.9) }
        }
    }
}

fn pan(c: &mut Cam, dx: f32, dy: f32, h: u32) {
    let k = c.dist * 2.0 * (FOV / 2.0).tan() / h as f32;
    let right = [c.yaw.cos(), 0.0, -c.yaw.sin()];
    let fwd = [-c.yaw.sin(), 0.0, -c.yaw.cos()];
    c.t = add(c.t, add(scale(right, -dx * k), scale(fwd, dy * k / c.pitch.sin().max(0.3))));
}

fn zoom_at(v: &mut View, scene: &Scene, step: f32, sx: f32, sy: f32, aspect: f32) {
    let c = &mut v.cam;
    let nd = (c.dist * 0.88f32.powf(step)).clamp(4.0, max_dist(scene));
    let py = c.t[1];
    if let Some(g) = c.ground(sx, sy, aspect, py) {
        let k = 1.0 - nd / c.dist;
        c.t = add(c.t, scale(sub(g, c.t), k));
    }
    c.dist = nd;
    // coming back in from the building view lands on the floor under the cursor
    if v.focus.is_none() && nd < home(scene, Some(0), aspect).dist * 1.3 {
        let f = ((c.t[1] / STOREY).round().max(0.0) as usize).min(scene.floors - 1);
        v.focus = Some(f);
        c.t[1] = floor_y(f);
    }
}

#[derive(Clone, Copy)]
enum Chip { Building, Floor(usize) }

impl Gpu {
    /// The pick id under a device-pixel position, from the last drawn frame.
    fn pick(&self, x: f32, y: f32, sf: f32) -> u32 {
        let Some(tg) = &self.tg else { return 0 };
        let px = pixel_size(sf);
        let (qx, qy) = ((x / px) as u32, (y / px) as u32);
        if qx >= tg.w || qy >= tg.h { return 0; }
        let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        enc.copy_texture_to_buffer(wgpu::TexelCopyTextureInfo { texture: &tg.ids, mip_level: 0, origin: wgpu::Origin3d { x: qx, y: qy, z: 0 }, aspect: wgpu::TextureAspect::All },
            wgpu::TexelCopyBufferInfo { buffer: &self.pick_buf, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(256), rows_per_image: Some(1) } }, wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 });
        self.queue.submit([enc.finish()]);
        let slice = self.pick_buf.slice(..);
        let (tx, rx) = channel();
        slice.map_async(wgpu::MapMode::Read, move |r| { let _ = tx.send(r); });
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        let mut id = 0;
        if rx.recv().map(|r| r.is_ok()).unwrap_or(false) {
            if let Ok(d) = slice.get_mapped_range() { id = u32::from_le_bytes([d[0], d[1], d[2], d[3]]); }
        }
        self.pick_buf.unmap();
        id
    }

    /// Uploads a (re)built scene: the static instances and the atlas.
    fn load(&mut self, scene: &Scene) {
        let mut items = scene.stat.clone();
        let (insts, batches) = batch(&mut items);
        self.stat_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("static"), contents: bytemuck::cast_slice(&insts), usage: wgpu::BufferUsages::VERTEX });
        self.stat = batches;
        let atlas = upload(&self.device, &self.queue, &scene.atlas);
        self.scene_group = scene_group(&self.device, &self.scene_layout, &self.globals, &self.lights, &atlas, &self.sampler, &self.shadow_view, &self.shadow_sampler);
    }
}

fn pixel_size(sf: f32) -> f32 { (sf * 1.5).round().max(2.0) }

fn batch(items: &mut [Item]) -> (Vec<Inst>, Vec<Batch>) {
    let mi = |m: Mesh| ALL.iter().position(|&x| x == m).unwrap();
    items.sort_by_key(|it| (it.glass, it.floor, mi(it.mesh)));
    let mut out = Vec::with_capacity(items.len());
    let mut batches: Vec<Batch> = vec![];
    for it in items.iter() {
        match batches.last_mut() {
            Some(b) if b.glass == it.glass && b.floor == it.floor && b.mesh == it.mesh => b.count += 1,
            _ => batches.push(Batch { glass: it.glass, floor: it.floor, mesh: it.mesh, start: out.len() as u32, count: 1 }),
        }
        out.push(it.inst);
    }
    (out, batches)
}

fn upload(device: &wgpu::Device, queue: &wgpu::Queue, img: &crate::paint::Img) -> wgpu::TextureView {
    device.create_texture_with_data(queue, &wgpu::TextureDescriptor { label: None, size: wgpu::Extent3d { width: img.w as u32, height: img.h as u32, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: wgpu::TextureFormat::Rgba8Unorm, usage: wgpu::TextureUsages::TEXTURE_BINDING, view_formats: &[] }, wgpu::util::TextureDataOrder::LayerMajor, &img.px)
        .create_view(&wgpu::TextureViewDescriptor::default())
}

#[allow(clippy::too_many_arguments)]
fn scene_group(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, globals: &wgpu::Buffer, lights: &wgpu::Buffer, atlas: &wgpu::TextureView, sampler: &wgpu::Sampler, shadow: &wgpu::TextureView, ssamp: &wgpu::Sampler) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout, entries: &[
        wgpu::BindGroupEntry { binding: 0, resource: globals.as_entire_binding() },
        wgpu::BindGroupEntry { binding: 1, resource: lights.as_entire_binding() },
        wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(atlas) },
        wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(sampler) },
        wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(shadow) },
        wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::Sampler(ssamp) },
    ] })
}

fn targets(gpu: &Gpu, w: u32, h: u32) -> Targets {
    let tex = |format: wgpu::TextureFormat, usage: wgpu::TextureUsages| gpu.device.create_texture(&wgpu::TextureDescriptor { label: None, size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format, usage, view_formats: &[] });
    let color = tex(wgpu::TextureFormat::Rgba8Unorm, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING);
    let ids = tex(wgpu::TextureFormat::R32Uint, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC);
    let depth = tex(wgpu::TextureFormat::Depth32Float, wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING);
    let v = |t: &wgpu::Texture| t.create_view(&wgpu::TextureViewDescriptor::default());
    let post_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout: &gpu.post_layout, entries: &[
        wgpu::BindGroupEntry { binding: 0, resource: gpu.post_u.as_entire_binding() },
        wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&v(&color)) },
        wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&v(&ids)) },
        wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&v(&depth)) },
    ] });
    Targets { w, h, color, ids, depth, post_group }
}

/// Draws one frame into `target`; says whether anything moving is in view.
#[allow(clippy::too_many_arguments)]
fn draw(gpu: &mut Gpu, scene: &Scene, view: &View, live: &mut Live, target: &wgpu::TextureView, w: u32, h: u32, sf: f32, dyn_items: &mut Vec<Item>, lights: &mut Vec<Light>, marks: &mut Vec<Mark>, chips: &mut Vec<([f32; 4], Chip)>, mouse: Option<(f32, f32)>) -> bool {
    let px = pixel_size(sf);
    let (lw, lh) = ((w as f32 / px).ceil() as u32, (h as f32 / px).ceil() as u32);
    if gpu.tg.as_ref().map(|t| (t.w, t.h)) != Some((lw, lh)) { gpu.tg = Some(targets(gpu, lw, lh)); }
    let floors: Vec<usize> = match view.focus { Some(f) => vec![f], None => (0..scene.floors).collect() };
    let cam = view.cam;
    live.eye = cam.eye();
    let moving = scene.frame(live, &floors, dyn_items, lights, marks);
    let (dyn_insts, dyn_batches) = batch(dyn_items);
    let n_dyn = dyn_insts.len().min(MAX_DYN);
    gpu.queue.write_buffer(&gpu.dyn_buf, 0, bytemuck::cast_slice(&dyn_insts[..n_dyn]));
    // the nearest lights
    let tgt = cam.t;
    lights.sort_by(|a, b| { let da = (a.p[0] - tgt[0]).powi(2) + (a.p[1] - tgt[1]).powi(2) * 4.0 + (a.p[2] - tgt[2]).powi(2); let db = (b.p[0] - tgt[0]).powi(2) + (b.p[1] - tgt[1]).powi(2) * 4.0 + (b.p[2] - tgt[2]).powi(2); da.partial_cmp(&db).unwrap() });
    let gl: Vec<GpuLight> = lights.iter().take(MAX_LIGHTS).map(|l| GpuLight { p: l.p, c: l.c }).collect();
    if !gl.is_empty() { gpu.queue.write_buffer(&gpu.lights, 0, bytemuck::cast_slice(&gl)); }
    let aspect = w as f32 / h as f32;
    let vp = cam.vp(aspect);
    let sun = norm(SUN);
    let centre = [cam.t[0].clamp(0.0, scene.width), cam.t[1], cam.t[2].clamp(0.0, DEPTH)];
    let hs = if view.focus.is_some() { (scene.width * 0.62).max(cam.dist * 0.5).min(scene.width * 0.75) } else { scene.width * 0.75 };
    let lvp = M4::ortho(-hs, hs, -hs, hs, 1.0, 400.0).mul(&M4::look_at(add(centre, scale(sun, 150.0)), centre, [0.0, 1.0, 0.0]));
    let g = Globals { vp: vp.0, lvp: lvp.0, eye: [live.eye[0], live.eye[1], live.eye[2], 1.0], sun: [sun[0], sun[1], sun[2], 0.62], sky: [0.66, 0.64, 0.76, 1.0], ground: [0.34, 0.3, 0.38, 1.0], params: [live.t, live.alert as u32 as f32, gl.len() as f32, 0.0] };
    gpu.queue.write_buffer(&gpu.globals, 0, bytemuck::bytes_of(&g));
    let tg = gpu.tg.as_ref().unwrap();
    gpu.queue.write_buffer(&gpu.post_u, 0, bytemuck::bytes_of(&PostU { a: [lw as f32, lh as f32, px, 0.0], b: [live.t, live.alert as u32 as f32, NEAR, FAR], c: [w as f32, h as f32, 0.0, 0.0], top: [0.13, 0.11, 0.24, 1.0], bot: [0.62, 0.36, 0.42, 1.0], hov: [live.hover, 0, 0, 0] }));
    let visible_floor = |f: i32| f < 0 || floors.contains(&(f as usize));
    let mut enc = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    let draw_batches = |rp: &mut wgpu::RenderPass, buf: &wgpu::Buffer, batches: &[Batch], glass: bool, limit: u32| {
        rp.set_vertex_buffer(1, buf.slice(..));
        for b in batches.iter().filter(|b| b.glass == glass && visible_floor(b.floor)) {
            if b.start >= limit { continue; }
            let (s, n) = gpu.ranges[&b.mesh];
            rp.draw_indexed(s..s + n, 0, b.start..(b.start + b.count).min(limit));
        }
    };
    {
        let sv = &gpu.shadow_view;
        let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor { label: Some("shadow"), color_attachments: &[], depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment { view: sv, depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }), stencil_ops: None }), ..Default::default() });
        rp.set_pipeline(&gpu.p_shadow);
        rp.set_bind_group(0, &gpu.shadow_group, &[]);
        rp.set_vertex_buffer(0, gpu.lib_v.slice(..));
        rp.set_index_buffer(gpu.lib_i.slice(..), wgpu::IndexFormat::Uint32);
        draw_batches(&mut rp, &gpu.stat_buf, &gpu.stat, false, u32::MAX);
        draw_batches(&mut rp, &gpu.dyn_buf, &dyn_batches, false, n_dyn as u32);
    }
    {
        let cv = tg.color.create_view(&wgpu::TextureViewDescriptor::default());
        let iv = tg.ids.create_view(&wgpu::TextureViewDescriptor::default());
        let dv = tg.depth.create_view(&wgpu::TextureViewDescriptor::default());
        let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("scene"),
            color_attachments: &[
                Some(wgpu::RenderPassColorAttachment { view: &cv, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store } }),
                Some(wgpu::RenderPassColorAttachment { view: &iv, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store } }),
            ],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment { view: &dv, depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }), stencil_ops: None }),
            ..Default::default()
        });
        rp.set_bind_group(0, &gpu.scene_group, &[]);
        rp.set_vertex_buffer(0, gpu.lib_v.slice(..));
        rp.set_index_buffer(gpu.lib_i.slice(..), wgpu::IndexFormat::Uint32);
        rp.set_pipeline(&gpu.p_scene);
        draw_batches(&mut rp, &gpu.stat_buf, &gpu.stat, false, u32::MAX);
        draw_batches(&mut rp, &gpu.dyn_buf, &dyn_batches, false, n_dyn as u32);
        rp.set_pipeline(&gpu.p_glass);
        draw_batches(&mut rp, &gpu.stat_buf, &gpu.stat, true, u32::MAX);
        draw_batches(&mut rp, &gpu.dyn_buf, &dyn_batches, true, n_dyn as u32);
    }
    // the overlay, in device pixels
    let quads = overlay(scene, view, live, &vp, w, h, sf, marks, chips, mouse);
    gpu.queue.write_buffer(&gpu.quads, 0, bytemuck::cast_slice(&quads[..quads.len().min(MAX_QUADS)]));
    gpu.queue.write_buffer(&gpu.over_u, 0, bytemuck::cast_slice(&[2.0 / w as f32, -2.0 / h as f32, -1.0, 1.0]));
    {
        let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("post"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: target, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store } })],
            ..Default::default()
        });
        rp.set_pipeline(&gpu.p_post);
        rp.set_bind_group(0, &tg.post_group, &[]);
        rp.draw(0..3, 0..1);
        rp.set_pipeline(&gpu.p_over);
        rp.set_bind_group(0, &gpu.over_g, &[]);
        rp.set_bind_group(1, &gpu.ui_group, &[]);
        rp.set_vertex_buffer(0, gpu.quads.slice(..));
        rp.draw(0..6, 0..quads.len().min(MAX_QUADS) as u32);
    }
    gpu.queue.submit([enc.finish()]);
    moving
}

/// Screen-space things: the floor switcher, area chips when far, balloons, "+N", tooltips, alert.
#[allow(clippy::too_many_arguments)]
fn overlay(scene: &Scene, view: &View, live: &Live, vp: &M4, w: u32, h: u32, sf: f32, marks: &[Mark], chips: &mut Vec<([f32; 4], Chip)>, mouse: Option<(f32, f32)>) -> Vec<Quad> {
    let ui = &scene.ui;
    let (aw, ah) = (ui.img.w as f32, ui.img.h as f32);
    let _ = (aw, ah);
    let mut q: Vec<Quad> = vec![];
    let u = (sf * 1.5).round().max(2.0);
    let rect = |q: &mut Vec<Quad>, x: f32, y: f32, ww: f32, hh: f32, c: [f32; 4]| q.push(Quad { dst: [x, y, ww, hh], src: ui.white, tint: c });
    let text_w = |s: &str, k: f32| -> f32 { s.chars().map(|c| (ui.glyph.get(&c.to_ascii_uppercase()).map(|g| g.1).unwrap_or(3) + 1) as f32 * k).sum::<f32>() - k };
    let text = |q: &mut Vec<Quad>, x: f32, y: f32, s: &str, k: f32, c: [f32; 4]| {
        let mut cx = x;
        for ch in s.chars() {
            if let Some((r, gw)) = ui.glyph.get(&ch.to_ascii_uppercase()) { q.push(Quad { dst: [cx, y, (*gw as f32) * k, 7.0 * k], src: *r, tint: c }); cx += (*gw + 1) as f32 * k; } else { cx += 4.0 * k; }
        }
    };
    let project = |p: V3| -> Option<(f32, f32)> {
        let c = vp.project(p);
        if c[3] <= 0.01 { return None; }
        Some(((c[0] / c[3] * 0.5 + 0.5) * w as f32, (0.5 - c[1] / c[3] * 0.5) * h as f32))
    };
    let ink = [0.11, 0.1, 0.15, 0.92];
    // area chips, when the areas are small on screen
    let far = view.focus.is_none() || view.cam.dist > 52.0;
    if far {
        for l in &scene.labels {
            if view.focus.is_some() && Some(l.floor) != view.focus { continue; }
            let Some((sx, sy)) = project(l.at) else { continue };
            let k = (u - 1.0).max(1.0);
            let name = scene.apps[l.app].name.to_uppercase();
            let (asks, stuck, err) = scene.counts(l.app);
            let mut tags: Vec<(String, [f32; 4])> = vec![];
            if asks > 0 { tags.push((format!("!{asks}"), [1.0, 0.83, 0.3, 1.0])); }
            if stuck > 0 { tags.push((format!("?{stuck}"), [0.94, 0.65, 0.25, 1.0])); }
            if err > 0 { tags.push((format!("X{err}"), [0.94, 0.33, 0.31, 1.0])); }
            let acc = scene.accounts[scene.apps[l.app].account].left;
            if acc < 0.2 { tags.push((if acc <= 0.0 { "OUT".into() } else { format!("{}%", (acc * 100.0).round()) }, if acc <= 0.0 { [0.94, 0.33, 0.31, 1.0] } else { [0.94, 0.65, 0.25, 1.0] })); }
            if scene.apps[l.app].overloaded { tags.push(("HOT".into(), [1.0, 0.55, 0.2, 1.0])); }
            if scene.apps[l.app].decay > 0 { tags.push(("UNUSED".into(), [0.7, 0.7, 0.75, 1.0])); }
            let tw = text_w(&name, k) + tags.iter().map(|(s, _)| text_w(s, k) + 5.0 * k).sum::<f32>();
            let (cw, ch) = (tw + 10.0 * k, 11.0 * k);
            let (x0, y0) = ((sx - cw / 2.0).round(), (sy - ch).round());
            rect(&mut q, x0, y0, cw, ch, ink);
            let acc_c = crate::models::rgba(scene.kits[l.app].as_ref().unwrap().accent);
            rect(&mut q, x0, y0, 2.0 * k, ch, acc_c);
            text(&mut q, x0 + 5.0 * k, y0 + 2.0 * k, &name, k, [0.96, 0.94, 0.9, 1.0]);
            let mut tx = x0 + 5.0 * k + text_w(&name, k) + 5.0 * k;
            for (s, c) in &tags { text(&mut q, tx, y0 + 2.0 * k, s, k, *c); tx += text_w(s, k) + 5.0 * k; }
        }
    }
    // balloons over people, at the pixel size of the scene so they match it
    if !far || view.cam.dist < 70.0 {
        for m in marks {
            match m {
                Mark::Balloon { at, icon } => {
                    let Some((sx, sy)) = project(*at) else { continue };
                    let Some((r, iw, ih)) = ui.icon.get(icon) else { continue };
                    let k = u;
                    q.push(Quad { dst: [(sx - *iw as f32 * k / 2.0).round(), (sy - *ih as f32 * k).round(), *iw as f32 * k, *ih as f32 * k], src: *r, tint: [1.0; 4] });
                }
                Mark::Plus { at, n } => {
                    let Some((sx, sy)) = project(*at) else { continue };
                    let s = format!("+{n}");
                    let k = u - 1.0;
                    let tw = text_w(&s, k);
                    rect(&mut q, (sx - tw / 2.0 - 3.0 * k).round(), (sy - 11.0 * k).round(), tw + 6.0 * k, 11.0 * k, [0.98, 0.95, 0.88, 0.95]);
                    text(&mut q, (sx - tw / 2.0).round(), (sy - 9.0 * k).round(), &s, k, [0.12, 0.1, 0.16, 1.0]);
                }
            }
        }
    }
    // the floor switcher
    chips.clear();
    let k = u;
    let mut x = 16.0 * k;
    let mut items: Vec<(String, Chip, bool)> = vec![("BUILDING".into(), Chip::Building, view.focus.is_none())];
    for f in (0..scene.floors).rev() { items.push((format!("FLOOR {}", f + 1), Chip::Floor(f), view.focus == Some(f))); }
    for (s, c, active) in items {
        let tw = text_w(&s, k);
        let r = [x, 14.0 * k, tw + 10.0 * k, 12.0 * k];
        rect(&mut q, r[0], r[1], r[2], r[3], if active { [0.16, 0.14, 0.22, 0.95] } else { [0.11, 0.1, 0.15, 0.75] });
        if active { rect(&mut q, r[0], r[1] + r[3], r[2], 2.0 * k / 2.0, [0.53, 0.53, 0.96, 1.0]); }
        text(&mut q, r[0] + 5.0 * k, r[1] + 2.5 * k, &s, k, if active { [1.0, 1.0, 1.0, 1.0] } else { [0.7, 0.72, 0.8, 1.0] });
        chips.push((r, c));
        x += r[2] + 4.0 * k;
    }
    // a hint line
    let hint = "DRAG: PAN  ·  RIGHT-DRAG / OPTION-DRAG: TURN AND TILT  ·  PINCH: ZOOM  ·  H: RESET";
    let hk = (u - 1.0).max(1.0);
    text(&mut q, 16.0 * k, h as f32 - 14.0 * hk, hint, hk, [0.85, 0.83, 0.9, 0.7]);
    // the tooltip for what is under the mouse
    if let (Some(p), Some((mx, my))) = (scene.pick_of(live.hover), mouse) {
        let k = u - 1.0;
        let l1 = p.label.to_uppercase();
        let l2 = p.detail.to_uppercase();
        let tw = text_w(&l1, k).max(text_w(&l2, k));
        let (x0, y0) = ((mx + 14.0 * k).min(w as f32 - tw - 14.0 * k).round(), (my + 14.0 * k).min(h as f32 - 26.0 * k).round());
        rect(&mut q, x0, y0, tw + 10.0 * k, 22.0 * k, [0.98, 0.95, 0.88, 0.97]);
        text(&mut q, x0 + 5.0 * k, y0 + 3.0 * k, &l1, k, [0.12, 0.1, 0.16, 1.0]);
        text(&mut q, x0 + 5.0 * k, y0 + 12.0 * k, &l2, k, [0.38, 0.33, 0.46, 1.0]);
    }
    if live.alert {
        let s = format!("RED ALERT · THIS MAC AT {}% FOR 6 MIN", (live.mac_load * 100.0).max(97.0).round());
        let tw = text_w(&s, k);
        let x0 = ((w as f32 - tw) / 2.0 - 6.0 * k).round();
        rect(&mut q, x0, 14.0 * k, tw + 12.0 * k, 12.0 * k, [0.55, 0.06, 0.08, 0.95]);
        text(&mut q, x0 + 6.0 * k, 16.5 * k, &s, k, [1.0, 0.9, 0.88, 1.0]);
    }
    let _ = (AISLE, ROW_D);
    q
}

/// A scripted walk: views of the office captured as frames (raw RGBA with an 8-byte header).
#[allow(clippy::too_many_arguments)]
fn tour(gpu: &mut Gpu, scene: &Scene, state: &OfficeState, dir: &str, live: &mut Live, dyn_items: &mut Vec<Item>, lights: &mut Vec<Light>, marks: &mut Vec<Mark>, chips: &mut Vec<([f32; 4], Chip)>) {
    let _ = std::fs::create_dir_all(dir);
    let (w, h, sf) = (2556u32, 1596u32, 2.0f32);
    let aspect = w as f32 / h as f32;
    let area = |id: &str| -> (usize, V3) {
        let pa = scene.plan.areas.iter().find(|a| a.id == id).unwrap();
        (pa.floor, [pa.rect.cx(), floor_y(pa.floor), pa.rect.cz()])
    };
    let session_id = |app: &str, s: usize| -> u32 { let ai = scene.apps.iter().position(|a| a.id == app).unwrap(); scene.picks.iter().position(|p| p.target == Target::Session(ai, s)).map(|i| i as u32 + 1).unwrap_or(0) };
    let prop_near = |f: usize, kind: &str| -> u32 {
        let _ = f;
        scene.props.iter().find(|p| matches!((kind, p.kind), ("pingpong", crate::scene::PropKind::PingPong) | ("coffee", crate::scene::PropKind::Coffee))).map(|p| p.id).unwrap_or(0)
    };
    struct Step { name: String, focus: Option<usize>, cam: Cam, frames: usize, alert: bool, hover: u32, mouse: Option<(f32, f32)>, click: u32, dt: f32 }
    let mut steps: Vec<Step> = vec![];
    let st = |name: &str, focus: Option<usize>, cam: Cam, frames: usize| Step { name: name.into(), focus, cam, frames, alert: false, hover: 0, mouse: None, click: 0, dt: 0.25 };
    let f1 = home(scene, Some(0), aspect);
    steps.push(st("building", None, home(scene, None, aspect), 2));
    steps.push(st("floor1", Some(0), f1, 3));
    for k in 0..12 { steps.push(st(&format!("turn{k:02}"), Some(0), Cam { yaw: f1.yaw + k as f32 * std::f32::consts::TAU / 12.0, ..f1 }, 1)); }
    steps.push(st("tilt-low", Some(0), Cam { pitch: PITCH_MIN + 0.05, dist: f1.dist * 0.8, ..f1 }, 2));
    steps.push(st("tilt-high", Some(0), Cam { pitch: PITCH_MAX - 0.05, ..f1 }, 2));
    let (_, a1) = area("app-1");
    steps.push(st("team-swarm", Some(0), Cam { t: [a1[0], a1[1], a1[2] - 2.5], yaw: 0.6, pitch: 0.75, dist: 20.0 }, 4));
    steps.push(st("desk-detail", Some(0), Cam { t: [a1[0] - 2.0, a1[1] + 0.6, a1[2] - 4.5], yaw: 0.9, pitch: 0.55, dist: 8.5 }, 4));
    let (_, a2) = area("app-2");
    steps.push(st("hot-and-crash", Some(0), Cam { t: [a2[0], a2[1], a2[2]], yaw: -0.5, pitch: 0.8, dist: 15.0 }, 4));
    let (_, a3) = area("app-3");
    steps.push(st("studio", Some(0), Cam { t: [a3[0], a3[1], a3[2]], yaw: 0.3, pitch: 0.8, dist: 11.0 }, 2));
    let (_, a4) = area("app-4");
    steps.push(st("garage", Some(0), Cam { t: [a4[0], a4[1], a4[2]], yaw: 2.4, pitch: 0.75, dist: 11.0 }, 2));
    let (_, a5) = area("app-5");
    steps.push(st("unused", Some(0), Cam { t: [a5[0], a5[1], a5[2]], yaw: 0.2, pitch: 0.85, dist: 11.0 }, 2));
    let (_, a6) = area("app-6");
    steps.push(st("lab", Some(0), Cam { t: [a6[0], a6[1], a6[2]], yaw: -2.6, pitch: 0.8, dist: 11.0 }, 2));
    steps.push(st("lobby", Some(0), Cam { t: [5.0, 0.8, 5.0], yaw: 0.9, pitch: 0.5, dist: 10.0 }, 3));
    steps.push(st("floor2", Some(1), home(scene, Some(1), aspect), 3));
    let (_, a7) = area("app-7");
    steps.push(st("low-usage", Some(1), Cam { t: [a7[0], a7[1], a7[2]], yaw: 0.4, pitch: 0.75, dist: 14.0 }, 3));
    let (_, a12) = area("app-12");
    steps.push(st("swarm-cap", Some(1), Cam { t: [a12[0], a12[1], a12[2]], yaw: -0.3, pitch: 0.7, dist: 14.0 }, 3));
    // hover a person who asks for you, then play with things
    let asker = session_id("app-1", 2);
    let mut hv = st("hover", Some(0), Cam { t: [a1[0], a1[1], a1[2] - 2.5], yaw: 0.6, pitch: 0.75, dist: 14.0 }, 2);
    hv.hover = asker;
    steps.push(hv);
    let pp = match prop_near(0, "pingpong") { 0 => prop_near(0, "coffee"), id => id };
    if pp != 0 {
        let p = scene.props.iter().find(|p| p.id == pp).unwrap().t.0.point([0.0, 0.0, 0.0]);
        let mut s = st("pingpong", Some(scene.props.iter().find(|p| p.id == pp).unwrap().floor as usize), Cam { t: p, yaw: 0.5, pitch: 0.7, dist: 7.0 }, 6);
        s.click = pp; s.dt = 0.12;
        steps.push(s);
    }
    let mut al = st("red-alert", Some(0), f1, 3);
    al.alert = true;
    steps.push(al);
    let mut n = 0;
    let base_t = live.t;
    let mut run_steps = |gpu: &mut Gpu, scene: &Scene, steps: Vec<Step>, live: &mut Live, n: &mut usize| {
        for s in steps {
            let view = View { focus: s.focus, cam: s.cam };
            if s.click != 0 { live.clicked.insert(s.click, base_t); }
            for f in 0..s.frames {
                live.t = base_t + f as f32 * s.dt;
                live.alert = s.alert;
                live.hover = s.hover;
                let mouse = s.mouse.or_else(|| {
                    let p = scene.anchor(s.hover)?;
                    let c = s.cam.vp(aspect).project(p);
                    Some(((c[0] / c[3] * 0.5 + 0.5) * w as f32, (0.5 - c[1] / c[3] * 0.5) * h as f32))
                });
                capture(gpu, scene, &view, live, w, h, sf, dyn_items, lights, marks, chips, mouse, &format!("{dir}/{:02}-{}.rgba", *n, s.name));
                *n += 1;
            }
            live.clicked.clear();
        }
    };
    run_steps(gpu, scene, steps, live, &mut n);
    // reflow: one App grows past its size, and the areas after it move on, in order
    let mut demo = state.clone();
    let grown = build_scene(&mut demo, None, Some(("app-3", 9)));
    gpu.load(&grown);
    let g1 = home(&grown, Some(0), aspect);
    let after = vec![st("grown-floor1", Some(0), g1, 1), st("grown-floor2", Some(1), home(&grown, Some(1), aspect), 1)];
    run_steps(gpu, &grown, after, live, &mut n);
    live.alert = false;
    live.hover = 0;
    println!("{}", serde_json::json!({ "event": "tour_done", "frames": n }));
}

#[allow(clippy::too_many_arguments)]
fn capture(gpu: &mut Gpu, scene: &Scene, view: &View, live: &mut Live, w: u32, h: u32, sf: f32, dyn_items: &mut Vec<Item>, lights: &mut Vec<Light>, marks: &mut Vec<Mark>, chips: &mut Vec<([f32; 4], Chip)>, mouse: Option<(f32, f32)>, path: &str) {
    let tex = gpu.device.create_texture(&wgpu::TextureDescriptor { label: None, size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: gpu.format, usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC, view_formats: &[] });
    draw(gpu, scene, view, live, &tex.create_view(&wgpu::TextureViewDescriptor::default()), w, h, sf, dyn_items, lights, marks, chips, mouse);
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
        let Ok(data) = slice.get_mapped_range() else { return };
        let bgra = matches!(gpu.format, wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb);
        let mut out = Vec::with_capacity((w * h * 4 + 8) as usize);
        out.extend_from_slice(&w.to_le_bytes());
        out.extend_from_slice(&h.to_le_bytes());
        for y in 0..h { for x in 0..w { let i = (y * row + x * 4) as usize; if bgra { out.extend_from_slice(&[data[i + 2], data[i + 1], data[i], 255]); } else { out.extend_from_slice(&[data[i], data[i + 1], data[i + 2], 255]); } } }
        let _ = std::fs::write(path, out);
    }
}

fn make_gpu(device: wgpu::Device, queue: wgpu::Queue, format: wgpu::TextureFormat, scene: &Scene) -> Gpu {
    let lib = Library::build();
    let lib_v = device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("verts"), contents: bytemuck::cast_slice(&lib.verts), usage: wgpu::BufferUsages::VERTEX });
    let lib_i = device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("index"), contents: bytemuck::cast_slice(&lib.idx), usage: wgpu::BufferUsages::INDEX });
    let ranges: HashMap<Mesh, (u32, u32)> = ALL.iter().map(|&m| (m, lib.range(m))).collect();
    let dyn_buf = device.create_buffer(&wgpu::BufferDescriptor { label: Some("dynamic"), size: (MAX_DYN * std::mem::size_of::<Inst>()) as u64, usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    let globals = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: std::mem::size_of::<Globals>() as u64, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    let lights = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: (MAX_LIGHTS * std::mem::size_of::<GpuLight>()) as u64, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    let shadow = device.create_texture(&wgpu::TextureDescriptor { label: Some("shadow"), size: wgpu::Extent3d { width: SHADOW, height: SHADOW, depth_or_array_layers: 1 }, mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format: wgpu::TextureFormat::Depth32Float, usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING, view_formats: &[] });
    let shadow_view = shadow.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor { mag_filter: wgpu::FilterMode::Nearest, min_filter: wgpu::FilterMode::Nearest, ..Default::default() });
    let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor { compare: Some(wgpu::CompareFunction::LessEqual), mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, ..Default::default() });
    let e = |binding: u32, visibility: wgpu::ShaderStages, ty: wgpu::BindingType| wgpu::BindGroupLayoutEntry { binding, visibility, ty, count: None };
    let vf = wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT;
    let fr = wgpu::ShaderStages::FRAGMENT;
    let scene_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: None, entries: &[
        e(0, vf, wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }),
        e(1, fr, wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }),
        e(2, fr, wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }),
        e(3, fr, wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)),
        e(4, fr, wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Depth, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }),
        e(5, fr, wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison)),
    ] });
    let post_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: None, entries: &[
        e(0, fr, wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }),
        e(1, fr, wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }),
        e(2, fr, wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Uint, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }),
        e(3, fr, wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Depth, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }),
    ] });
    let over_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: None, entries: &[e(0, wgpu::ShaderStages::VERTEX, wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None })] });
    let tex_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: None, entries: &[
        e(0, fr, wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }),
        e(1, fr, wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)),
    ] });
    let atlas = upload(&device, &queue, &scene.atlas);
    let scene_group = scene_group(&device, &scene_layout, &globals, &lights, &atlas, &sampler, &shadow_view, &shadow_sampler);
    let ui_view = upload(&device, &queue, &scene.ui.img);
    let ui_group = device.create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout: &tex_layout, entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&ui_view) }, wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) }] });
    let post_u = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: std::mem::size_of::<PostU>() as u64, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    let over_u = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: 16, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    let over_g = device.create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout: &over_layout, entries: &[wgpu::BindGroupEntry { binding: 0, resource: over_u.as_entire_binding() }] });
    let quads = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: (MAX_QUADS * std::mem::size_of::<Quad>()) as u64, usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    let pick_buf = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: 256, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });

    let scene_mod = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("scene"), source: wgpu::ShaderSource::Wgsl(SCENE_WGSL.into()) });
    let post_mod = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("post"), source: wgpu::ShaderSource::Wgsl(POST_WGSL.into()) });
    let over_mod = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("overlay"), source: wgpu::ShaderSource::Wgsl(OVERLAY_WGSL.into()) });
    // the shadow pass draws into the shadow map, so it binds only the uniforms
    let shadow_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: None, entries: &[e(0, vf, wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None })] });
    let shadow_group = device.create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout: &shadow_layout, entries: &[wgpu::BindGroupEntry { binding: 0, resource: globals.as_entire_binding() }] });
    let shadow_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[Some(&shadow_layout)], ..Default::default() });
    let scene_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[Some(&scene_layout)], ..Default::default() });
    let post_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[Some(&post_layout)], ..Default::default() });
    let over_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: None, bind_group_layouts: &[Some(&over_layout), Some(&tex_layout)], ..Default::default() });
    let vattrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2];
    let iattrs = wgpu::vertex_attr_array![3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 8 => Float32x4, 9 => Float32x4, 10 => Float32x2, 11 => Uint32];
    let vbufs = [
        Some(wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<Vertex>() as u64, step_mode: wgpu::VertexStepMode::Vertex, attributes: &vattrs }),
        Some(wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<Inst>() as u64, step_mode: wgpu::VertexStepMode::Instance, attributes: &iattrs }),
    ];
    let depth = |write: bool, bias: bool| Some(wgpu::DepthStencilState { format: wgpu::TextureFormat::Depth32Float, depth_write_enabled: Some(write), depth_compare: Some(wgpu::CompareFunction::Less), stencil: Default::default(), bias: if bias { wgpu::DepthBiasState { constant: 2, slope_scale: 2.0, clamp: 0.0 } } else { Default::default() } });
    let mk = |label: &str, fs: Option<&str>, glass: bool| {
        let targets = [
            Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::Rgba8Unorm, blend: if glass { Some(wgpu::BlendState::ALPHA_BLENDING) } else { None }, write_mask: wgpu::ColorWrites::ALL }),
            Some(wgpu::ColorTargetState { format: wgpu::TextureFormat::R32Uint, blend: None, write_mask: if glass { wgpu::ColorWrites::empty() } else { wgpu::ColorWrites::ALL } }),
        ];
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(if fs.is_some() { &scene_pl } else { &shadow_pl }),
        vertex: wgpu::VertexState { module: &scene_mod, entry_point: Some(if fs.is_some() { "vs" } else { "vs_shadow" }), compilation_options: Default::default(), buffers: &vbufs },
        fragment: fs.map(|f| wgpu::FragmentState { module: &scene_mod, entry_point: Some(f), compilation_options: Default::default(), targets: &targets }),
        primitive: wgpu::PrimitiveState { cull_mode: if fs.is_some() { Some(wgpu::Face::Back) } else { None }, ..Default::default() },
        depth_stencil: depth(!glass, fs.is_none()),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    }) };
    let p_scene = mk("scene", Some("fs"), false);
    let p_glass = mk("glass", Some("fs_glass"), true);
    let p_shadow = mk("shadow", None, false);
    let p_post = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("post"), layout: Some(&post_pl),
        vertex: wgpu::VertexState { module: &post_mod, entry_point: Some("vs"), compilation_options: Default::default(), buffers: &[] },
        fragment: Some(wgpu::FragmentState { module: &post_mod, entry_point: Some("fs"), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })] }),
        primitive: wgpu::PrimitiveState::default(), depth_stencil: None, multisample: wgpu::MultisampleState::default(), multiview_mask: None, cache: None,
    });
    let qattrs = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4];
    let p_over = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("overlay"), layout: Some(&over_pl),
        vertex: wgpu::VertexState { module: &over_mod, entry_point: Some("vs"), compilation_options: Default::default(), buffers: &[Some(wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<Quad>() as u64, step_mode: wgpu::VertexStepMode::Instance, attributes: &qattrs })] },
        fragment: Some(wgpu::FragmentState { module: &over_mod, entry_point: Some("fs"), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format, blend: Some(wgpu::BlendState::ALPHA_BLENDING), write_mask: wgpu::ColorWrites::ALL })] }),
        primitive: wgpu::PrimitiveState::default(), depth_stencil: None, multisample: wgpu::MultisampleState::default(), multiview_mask: None, cache: None,
    });
    let mut items = scene.stat.clone();
    let (insts, stat) = batch(&mut items);
    let stat_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("static"), contents: bytemuck::cast_slice(&insts), usage: wgpu::BufferUsages::VERTEX });
    Gpu { device, queue, format, lib_v, lib_i, ranges, stat_buf, stat, dyn_buf, globals, lights, scene_layout, scene_group, shadow_group, shadow_view, sampler, shadow_sampler, p_scene, p_glass, p_shadow, p_post, p_over, post_u, post_layout, over_u, over_g, ui_group, quads, pick_buf, tg: None }
}
