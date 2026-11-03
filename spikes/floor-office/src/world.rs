//! The office: Apps, their style kits, two floors stacked as a cutaway building, baked once.
//!
//! Each floor is seen by the same fixed camera (high above, looking down at 38°). Floors and walls are
//! shaded per pixel by unprojecting each pixel onto their plane, so every kit's material (planks,
//! parquet, carpet tiles, concrete, terrazzo, rubber; paint, brick, slats, tiles, stripes, panels) sits
//! in true perspective, lit by its windows, its lamps and its own light warmth. Desks, chairs, people,
//! bubbles, signs and the fire are sprites drawn back to front over the baked floors each frame.

use crate::kit::{kit_for, Decor, DeskKind, ChairKind, FloorMat, Kit, Layout, Nook, Plant, Screens, SignKind, Tool, WallFinish};
use crate::paint::*;
use crate::people::{self, Look, Pose};

pub const FW: i32 = 1240;
pub const FH: i32 = 540;
const X0: f32 = -22.0;
const X1: f32 = 22.0;
const Z0: f32 = -9.0;
const Z1: f32 = 12.0;
const WALL: f32 = 3.4;
const HUB: f32 = -16.8; // the lift lobby sits between the glass wall and the first area
const ZONE_W: f32 = (X1 - HUB) / 4.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum St { Typing, Idle, Asks, Stuck, Limit, Done }

pub struct AppDef {
    pub id: &'static str,
    pub name: &'static str,
    pub tool: Tool,
    pub slot: usize,
    pub floor: usize,
    pub col: usize,
    pub sessions: Vec<St>,
    /// 0 in use; 2 unused for days (dust, lights off).
    pub decay: u8,
    pub overloaded: bool,
}

/// The sample company: generic names only.
pub fn sample() -> Vec<AppDef> {
    use St::*;
    let typing = |n: usize, idle_every: usize| -> Vec<St> { (0..n).map(|i| if i % idle_every == idle_every - 1 { Idle } else { Typing }).collect() };
    let mut cc = typing(17, 4);
    cc[5] = Asks;
    cc[9] = Done;
    let mut cx = typing(16, 5);
    cx[2] = Typing;
    let ca = typing(14, 3);
    let mut oc = typing(11, 4);
    oc[4] = Stuck;
    let mut cc2 = typing(12, 3);
    cc2[3] = Limit;
    let cx2 = typing(12, 3);
    vec![
        AppDef { id: "app-1", name: "Claude Code · Work", tool: Tool::ClaudeCode, slot: 0, floor: 0, col: 0, sessions: cc, decay: 0, overloaded: false },
        AppDef { id: "app-2", name: "Codex · Main", tool: Tool::Codex, slot: 1, floor: 0, col: 1, sessions: cx, decay: 0, overloaded: true },
        AppDef { id: "app-3", name: "Claude app · Work", tool: Tool::ClaudeApp, slot: 2, floor: 0, col: 2, sessions: ca, decay: 0, overloaded: false },
        AppDef { id: "app-4", name: "OpenCode · Lab", tool: Tool::OpenCode, slot: 3, floor: 0, col: 3, sessions: oc, decay: 0, overloaded: false },
        AppDef { id: "app-5", name: "Claude Code · Side", tool: Tool::ClaudeCode, slot: 4, floor: 1, col: 0, sessions: cc2, decay: 0, overloaded: false },
        AppDef { id: "app-6", name: "Gemini CLI · Lab", tool: Tool::Gemini, slot: 5, floor: 1, col: 1, sessions: vec![], decay: 2, overloaded: false },
        AppDef { id: "app-7", name: "Codex · Review", tool: Tool::Codex, slot: 6, floor: 1, col: 2, sessions: cx2, decay: 0, overloaded: false },
    ]
}

#[derive(Clone, Copy, serde::Serialize)]
pub struct Sprite { pub x: i32, pub y: i32, pub w: i32, pub h: i32 }
#[derive(Clone, Copy)]
pub struct Item { pub s: usize, pub x: f32, pub y: f32, pub depth: f32 }
#[derive(Clone)]
pub struct Actor { pub frames: Vec<usize>, pub rate: f32, pub phase: f32, pub x: f32, pub y: f32, pub depth: f32 }
#[derive(Clone)]
pub struct Walker { pub path: Vec<(f32, f32)>, pub speed: f32, pub offset: f32, pub down: [usize; 2], pub up: [usize; 2], pub left: [usize; 2], pub right: [usize; 2], pub ax: f32, pub ay: f32 }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Target { App(usize), Session(usize, usize), Floor(usize) }
#[derive(Clone, Copy)]
pub struct Hit { pub x: f32, pub y: f32, pub w: f32, pub h: f32, pub target: Target, pub z: f32 }
#[derive(Clone)]
pub struct Label { pub x: f32, pub y: f32, pub s: usize, pub floor: usize }
#[derive(Clone, Copy)]
pub struct FloorBox { pub y: f32, pub h: f32 }

pub struct World {
    pub w: i32,
    pub h: i32,
    pub bg: Img,
    pub atlas: Img,
    pub sprites: Vec<Sprite>,
    pub items: Vec<Item>,
    pub actors: Vec<Actor>,
    pub walkers: Vec<Walker>,
    pub hits: Vec<Hit>,
    /// Name chips shown in screen space when zoomed far out (sprite ids in the atlas).
    pub labels: Vec<Label>,
    pub floors: Vec<FloorBox>,
    pub apps: Vec<AppDef>,
    pub kits: Vec<Kit>,
    /// The white pixel (for tinted overlays) and the screen-space UI sprites.
    pub white: usize,
    pub ui: Vec<(String, usize)>,
    pub alert_beacons: Vec<(f32, f32)>,
    pub beacon: [usize; 2],
}

/// One draw: sprite, top-left in canvas pixels, depth for sorting.
#[derive(Clone, Copy)]
pub struct Inst { pub s: usize, pub x: f32, pub y: f32, pub depth: f32 }

impl World {
    /// Everything to draw at time `t` within the visible canvas rectangle, back to front.
    /// Returns whether anything animated is in view.
    pub fn instances(&self, t: f32, view: (f32, f32, f32, f32), out: &mut Vec<Inst>) -> bool {
        out.clear();
        let (vx0, vy0, vx1, vy1) = view;
        let vis = |x: f32, y: f32, w: i32, h: i32| x + w as f32 >= vx0 && y + h as f32 >= vy0 && x <= vx1 && y <= vy1;
        for it in &self.items {
            let s = self.sprites[it.s];
            if vis(it.x, it.y, s.w, s.h) { out.push(Inst { s: it.s, x: it.x, y: it.y, depth: it.depth }); }
        }
        let mut anim = false;
        for a in &self.actors {
            let f = a.frames[((t * a.rate + a.phase).floor().max(0.0) as usize) % a.frames.len()];
            let s = self.sprites[f];
            if vis(a.x, a.y, s.w, s.h) { out.push(Inst { s: f, x: a.x, y: a.y, depth: a.depth }); anim = true; }
        }
        for w in &self.walkers {
            let (x, y, dx, dy) = along(&w.path, t * w.speed + w.offset);
            let pair = if dx.abs() > dy.abs() { if dx > 0.0 { w.right } else { w.left } } else if dy > 0.0 { w.down } else { w.up };
            let f = pair[((t * 5.0).floor() as usize) % 2];
            let (px, py) = ((x - w.ax).round(), (y - w.ay).round());
            let s = self.sprites[f];
            if vis(px, py, s.w, s.h) { out.push(Inst { s: f, x: px, y: py, depth: y }); anim = true; }
        }
        out.sort_by(|a, b| a.depth.partial_cmp(&b.depth).unwrap());
        anim
    }
    pub fn hit(&self, x: f32, y: f32) -> Option<Target> {
        self.hits.iter().filter(|h| x >= h.x && y >= h.y && x < h.x + h.w && y < h.y + h.h).max_by(|a, b| a.z.partial_cmp(&b.z).unwrap()).map(|h| h.target)
    }
}

fn along(path: &[(f32, f32)], s: f32) -> (f32, f32, f32, f32) {
    let segs: Vec<f32> = (0..path.len()).map(|i| { let (a, b) = (path[i], path[(i + 1) % path.len()]); ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt() }).collect();
    let total: f32 = segs.iter().sum();
    let mut d = s.rem_euclid(total.max(1.0));
    for i in 0..path.len() {
        if d <= segs[i] {
            let (a, b) = (path[i], path[(i + 1) % path.len()]);
            let k = if segs[i] > 0.0 { d / segs[i] } else { 0.0 };
            return (a.0 + (b.0 - a.0) * k, a.1 + (b.1 - a.1) * k, b.0 - a.0, b.1 - a.1);
        }
        d -= segs[i];
    }
    (path[0].0, path[0].1, 1.0, 0.0)
}

struct Packer { imgs: Vec<Img> }
impl Packer {
    fn add(&mut self, im: Img) -> usize { self.imgs.push(im); self.imgs.len() - 1 }
    fn pack(self) -> (Img, Vec<Sprite>) {
        let aw = 2048;
        let mut order: Vec<usize> = (0..self.imgs.len()).collect();
        order.sort_by_key(|&i| -self.imgs[i].h);
        let (mut x, mut y, mut shelf) = (0, 0, 0);
        let mut rects = vec![Sprite { x: 0, y: 0, w: 0, h: 0 }; self.imgs.len()];
        for &i in &order {
            let im = &self.imgs[i];
            if x + im.w > aw { x = 0; y += shelf + 1; shelf = 0; }
            rects[i] = Sprite { x, y, w: im.w, h: im.h };
            x += im.w + 1;
            shelf = shelf.max(im.h);
        }
        let mut atlas = Img::new(aw, y + shelf + 1);
        for (im, r) in self.imgs.iter().zip(&rects) { atlas.blit(im, r.x, r.y); }
        (atlas, rects)
    }
}

fn zone_x(col: usize) -> (f32, f32) {
    let a = HUB + col as f32 * ZONE_W;
    (a + 0.45, a + ZONE_W - 0.45)
}

// ---------- materials ----------
fn floor_colour(m: &FloorMat, x: f32, z: f32, seed: u32) -> Rgba {
    match *m {
        FloorMat::Planks { wood, width } => {
            let row = (z / width).floor() as i32;
            let len = 1.6;
            let off = noise2(row, 7, seed) * len;
            let u = (x + off) / len;
            let plank = u.floor() as i32;
            let tone = (noise2(row, plank, seed) - 0.5) * 0.22;
            let seam = u.fract() < 0.02 || (z / width).fract() < 0.07;
            let grain = (noise2((x * 40.0) as i32, row, seed ^ 9) - 0.5) * 0.06;
            ramp(wood, if seam { -0.32 } else { tone + grain })
        }
        FloorMat::Parquet { wood, block } => {
            let (i, j) = ((x / block).floor() as i32, (z / block).floor() as i32);
            let (u, v) = ((x / block).fract(), (z / block).fract());
            let along_x = (i + j) % 2 == 0;
            let strip = if along_x { v } else { u } * 4.0;
            let k = strip.floor() as i32;
            let seam = strip.fract() < 0.1 || u < 0.03 || v < 0.03;
            let tone = (noise2(i * 7 + k, j, seed) - 0.5) * 0.25;
            ramp(wood, if seam { -0.3 } else { tone + if along_x { 0.05 } else { -0.05 } })
        }
        FloorMat::Carpet { a, b, tile } => {
            let (i, j) = ((x / tile).floor() as i32, (z / tile).floor() as i32);
            let (u, v) = ((x / tile).fract(), (z / tile).fract());
            let c = if (i + j) % 2 == 0 { a } else { b };
            let line = if (i + j) % 2 == 0 { (u * 9.0).fract() < 0.3 } else { (v * 9.0).fract() < 0.3 };
            let n = (noise2((x * 30.0) as i32, (z * 30.0) as i32, seed) - 0.5) * 0.1;
            ramp(c, if line { -0.08 } else { 0.0 } + n)
        }
        FloorMat::Concrete { tone, seam } => {
            let blot = (noise2((x / 2.3).floor() as i32, (z / 2.3).floor() as i32, seed) - 0.5) * 0.12 + (noise2((x * 2.0) as i32, (z * 2.0) as i32, seed ^ 3) - 0.5) * 0.07;
            let speck = noise2((x * 45.0) as i32, (z * 45.0) as i32, seed ^ 5) > 0.93;
            let s = (x / seam).fract() < 0.008 || (z / seam).fract() < 0.012;
            let gloss = if (x * 0.7 + z * 0.3).sin() > 0.97 { 0.08 } else { 0.0 };
            ramp(tone, if s { -0.35 } else { blot + gloss + if speck { -0.12 } else { 0.0 } })
        }
        FloorMat::Terrazzo { base, chips } => {
            let (i, j) = ((x / 0.05).floor() as i32, (z / 0.05).floor() as i32);
            let n = noise2(i, j, seed);
            if n > 0.86 { chips[((n * 100.0) as usize) % 3] } else if n > 0.8 { ramp(base, -0.25) } else { ramp(base, (noise2(i / 8, j / 8, seed ^ 2) - 0.5) * 0.06) }
        }
        FloorMat::Rubber { c } => {
            let (u, v) = ((x / 0.12).fract(), (z / 0.12).fract());
            let stud = (u - 0.5).powi(2) + (v - 0.5).powi(2) < 0.06;
            ramp(c, if stud { 0.12 } else { (noise2((x * 20.0) as i32, (z * 20.0) as i32, seed) - 0.5) * 0.05 })
        }
    }
}

fn wall_colour(w: &WallFinish, x: f32, y: f32, seed: u32, accent: Rgba) -> Rgba {
    match *w {
        WallFinish::Paint { c, dado } => {
            if y < 1.0 { ramp(dado, if (y - 0.98).abs() < 0.03 { 0.2 } else { -0.05 }) } else { ramp(c, (noise2((x * 8.0) as i32, (y * 8.0) as i32, seed) - 0.5) * 0.03) }
        }
        WallFinish::Brick { c, mortar } => {
            let row = (y / 0.075).floor() as i32;
            let off = if row % 2 == 0 { 0.0 } else { 0.11 };
            let u = (x + off) / 0.22;
            let mort = (y / 0.075).fract() < 0.18 || u.fract() < 0.06;
            if mort { mortar } else { ramp(c, (noise2(u.floor() as i32, row, seed) - 0.5) * 0.3) }
        }
        WallFinish::Slats { wood, back } => {
            let u = (x / 0.09).fract();
            if u < 0.28 { back } else { ramp(wood, (noise2((x / 0.09).floor() as i32, 3, seed) - 0.5) * 0.2 + if u > 0.9 { -0.15 } else { 0.0 }) }
        }
        WallFinish::Tiles { c, grout } => {
            let (u, v) = ((x / 0.6).fract(), (y / 0.6).fract());
            if u < 0.03 || v < 0.03 { grout } else { ramp(c, (noise2((x / 0.6) as i32, (y / 0.6) as i32, seed) - 0.5) * 0.1) }
        }
        WallFinish::Stripes { a, b } => {
            // a supergraphic: a calm base with a bold diagonal band and the App's colour as a thin line
            let d = x * 0.35 + y;
            let band = (d / 1.2).rem_euclid(3.0);
            if band < 0.9 { b } else if band < 1.0 { accent } else { a }
        }
        WallFinish::Acoustic { a, b } => {
            let (i, j) = ((x / 0.55).floor() as i32, (y / 0.55).floor() as i32);
            let (u, v) = ((x / 0.55).fract(), (y / 0.55).fract());
            if u < 0.06 || v < 0.06 { ramp(a, -0.4) } else if (i + j) % 3 == 0 { ramp(b, (v - 0.5) * 0.15) } else { ramp(a, (v - 0.5) * 0.15) }
        }
    }
}

fn dusk(y_frac: f32, x: f32, seed: u32) -> Rgba {
    // the view outside: an evening sky over a skyline with lit windows
    let sky = if y_frac > 0.55 { mix(hex("#f2a35e"), hex("#c45f6a"), (1.0 - y_frac) / 0.45) } else { mix(hex("#c45f6a"), hex("#3b3566"), (0.55 - y_frac) / 0.55) };
    let bx = (x * 1.6).floor() as i32;
    let bh = 0.25 + noise2(bx, 1, seed) * 0.5;
    if y_frac > 1.0 - bh {
        let lit = noise2((x * 9.0) as i32, (y_frac * 30.0) as i32, seed ^ 4) > 0.7;
        if lit { hex("#ffd58a") } else { mix(hex("#2c2a4a"), hex("#3d3560"), noise2(bx, 2, seed)) }
    } else { sky }
}

// ---------- the build ----------
pub fn build() -> World {
    let apps = sample();
    let kits: Vec<Kit> = apps.iter().map(|a| kit_for(a.tool, a.id, a.slot)).collect();
    let mut pk = Packer { imgs: Vec::new() };
    let white = { let mut i = Img::new(1, 1); i.rect(0, 0, 1, 1, [255, 255, 255, 255]); pk.add(i) };
    let mut items = Vec::new();
    let mut actors = Vec::new();
    let mut walkers = Vec::new();
    let mut hits = Vec::new();
    let mut labels = Vec::new();
    let mut beacons = Vec::new();
    let margin = 24.0;
    let gap = 64.0;
    let cw = FW + 2 * margin as i32;
    let ch = 2 * FH + gap as i32 + 2 * margin as i32;
    let mut bg = Img::new(cw, ch);
    // the evening outside the building
    for y in 0..ch {
        let t = y as f32 / ch as f32;
        let c = mix(mix(hex("#1d1b3a"), hex("#5a3f6a"), t * 1.4), hex("#c46a5a"), ((t - 0.6) / 0.4).max(0.0));
        for x in 0..cw { bg.blend(x, y, c); }
    }
    for x in 0..cw {
        let h = 60.0 + noise2(x / 22, 9, 77) * 160.0;
        for y in (ch as f32 - h) as i32..ch { bg.blend(x, y, mix(hex("#23203f"), hex("#2e2a50"), noise2(x / 22, 3, 7))); }
    }
    let edge = hex("#120f1f");
    for y in 0..ch { for x in 0..cw { let d = x.min(y).min(cw - 1 - x).min(ch - 1 - y) as f32; if d < 60.0 { let c = bg.get(x, y); let t = 1.0 - d / 60.0; bg.blend(x, y, mix(c, edge, t * t)); } } }
    let floor_boxes = vec![FloorBox { y: margin, h: FH as f32 }, FloorBox { y: margin + FH as f32 + gap, h: FH as f32 }];
    // floor 1 (index 0) is drawn below floor 2 (index 1)
    let floor_y = |f: usize| if f == 0 { floor_boxes[1].y } else { floor_boxes[0].y };
    let beacon = [
        pk.add({ let mut i = Img::new(10, 10); i.rect(1, 3, 8, 7, hex("#7a1d1d")); i.rect(2, 1, 6, 4, hex("#ff4d4d")); i.rect(3, 1, 2, 2, hex("#ffd0d0")); i.outline(); i }),
        pk.add({ let mut i = Img::new(10, 10); i.rect(1, 3, 8, 7, hex("#7a1d1d")); i.rect(2, 1, 6, 4, hex("#ff4d4d")); i.rect(5, 1, 2, 2, hex("#ffd0d0")); i.outline(); i }),
    ];

    for floor in 0..2 {
        let oy = floor_y(floor);
        let ox = margin;
        // the nearest camera at which the whole room fits the floor image, then centred
        let room = corners([X0, -0.5, Z0], [X1, WALL, Z1]);
        let mut dist = 40.0;
        let mut cam = Cam::new([0.0, 0.0, 1.5], 42.0, dist, 26.0, FW, FH);
        while { let (_, _, bw, bh) = bounds(&cam, &room, 0); bw > FW - 16 || bh > FH - 16 } { dist += 0.5; cam = Cam::new([0.0, 0.0, 1.5], 42.0, dist, 26.0, FW, FH); }
        let (bx, by, bw, bh) = bounds(&cam, &room, 0);
        cam.shift((FW - bw) as f32 / 2.0 - bx as f32, (FH - bh) as f32 / 2.0 - by as f32);
        let mut img = Img::new(FW, FH);
        let zone_apps: Vec<Option<usize>> = (0..4).map(|c| apps.iter().position(|a| a.floor == floor && a.col == c)).collect();
        let zone_of = |x: f32| -> Option<usize> { if x < HUB { None } else { Some((((x - HUB) / ZONE_W) as usize).min(3)) } };
        // lamps and desk footprints, for light pools and contact shadows
        let mut lamps: Vec<(f32, f32, f32)> = Vec::new();
        let mut shadows: Vec<(f32, f32, f32, f32)> = Vec::new();
        let mut plans: Vec<Vec<Desk>> = Vec::new();
        for c in 0..4 {
            let (zx0, zx1) = zone_x(c);
            let plan = match zone_apps[c] { Some(ai) => plan_desks(&kits[ai], &apps[ai], zx0, zx1), None => Vec::new() };
            for d in &plan { shadows.push((d.x - 0.05, d.z - 0.05, d.x + d.w + 0.15, d.z + d.d + 0.12)); }
            let lit = zone_apps[c].map(|ai| apps[ai].decay == 0).unwrap_or(true);
            if lit { for k in 0..3 { for j in 0..3 { lamps.push((zx0 + 1.6 + k as f32 * 3.0, -5.0 + j as f32 * 6.5, if zone_apps[c].is_none() { 0.8 } else { 1.0 })); } } }
            plans.push(plan);
        }
        // ---- floor, per pixel ----
        let windows_of = |c: usize| -> Vec<(f32, f32)> {
            let (zx0, zx1) = zone_x(c);
            let n = zone_apps[c].map(|ai| kits[ai].windows).unwrap_or(2);
            let w = (zx1 - zx0) / (n as f32 * 2.0 + 1.0) * 1.3;
            (0..n).map(|k| { let cx = zx0 + (zx1 - zx0) * (k as f32 + 0.5) / n as f32; (cx - w / 2.0, cx + w / 2.0) }).collect()
        };
        let all_windows: Vec<(f32, f32)> = (0..4).flat_map(windows_of).collect();
        for py in 0..FH {
            for px in 0..FW {
                let Some((x, z)) = cam.on_plane_y(px as f32 + 0.5, py as f32 + 0.5, 0.0) else { continue };
                if !(X0..=X1).contains(&x) || !(Z0..=Z1).contains(&z) { continue; }
                let zone = zone_of(x);
                let (inner, kit) = match zone {
                    Some(c) => { let (zx0, zx1) = zone_x(c); ((zx0..=zx1).contains(&x) && z > -8.4 && z < Z1 - 0.6, zone_apps[c].map(|ai| &kits[ai])) }
                    None => (false, None),
                };
                let seed = zone.map(|c| c as u32 * 31 + floor as u32 * 7).unwrap_or(99);
                let mut c = match (inner, kit) {
                    (true, Some(k)) => floor_colour(&k.floor, x, z, seed),
                    (true, None) => floor_colour(&FloorMat::Planks { wood: hex("#a98a68"), width: 0.22 }, x, z, seed),
                    _ => floor_colour(&FloorMat::Concrete { tone: hex("#b3ada3"), seam: 4.0 }, x, z, 5),
                };
                if zone.is_none() && ((x - (X0 + HUB) / 2.0) / 2.0).powi(2) + ((z + 3.0) / 1.6).powi(2) < 1.0 { c = ramp(hex("#7a6a8a"), if ((x * 3.0).sin() * (z * 3.0).cos()) > 0.6 { 0.12 } else { 0.0 }); }
                // rugs under nooks and pods
                if let (true, Some(c0)) = (inner, zone) { if let Some(ai) = zone_apps[c0] { let (zx0, zx1) = zone_x(c0); if nook_rect(&kits[ai], zx0, zx1).map(|(a, b, cz, dz)| x > a && x < b && z > cz && z < dz).unwrap_or(false) { let rug = kits[ai].rug; let edge = noise2((x * 25.0) as i32, (z * 25.0) as i32, 3) > 0.5; c = ramp(rug, if edge { -0.05 } else { 0.03 }); } } }
                let decayed = zone.and_then(|c0| zone_apps[c0]).map(|ai| apps[ai].decay > 0).unwrap_or(false);
                let mut light = if decayed { 0.62 } else { 0.84 };
                for &(wa, wb) in &all_windows { if x > wa - (z - Z0) * 0.25 && x < wb + (z - Z0) * 0.1 { light += 0.13 * (1.0 - ((z - Z0) / 4.5)).max(0.0); } }
                for &(lx, lz, k) in &lamps { let d2 = (x - lx).powi(2) + ((z - lz) * 1.3).powi(2); light += 0.11 * k * (-d2 / 3.0).exp(); }
                light -= 0.14 * (-(z - Z0) / 0.7).exp();
                for &(a, b, cz, dz) in &shadows { if x > a && x < b && z > cz && z < dz { light -= 0.2; break; } }
                let warmth = kit.map(|k| k.warmth).unwrap_or(0.7);
                let tint = [0.94 + 0.12 * warmth, 0.97, 1.06 - 0.16 * warmth];
                c = mul(c, [light * tint[0], light * tint[1], light * tint[2]]);
                if decayed { c = mix(c, hex("#6d6a72"), 0.35); }
                img.blend(px, py, c);
            }
        }
        // ---- the back wall, per pixel ----
        for py in 0..FH {
            for px in 0..FW {
                let Some((x, y)) = cam.on_plane_z(px as f32 + 0.5, py as f32 + 0.5, Z0) else { continue };
                if !(X0..=X1).contains(&x) || !(0.0..=WALL).contains(&y) { continue; }
                let zone = zone_of(x);
                let kit = zone.and_then(|c| zone_apps[c]).map(|ai| &kits[ai]);
                let decayed = zone.and_then(|c| zone_apps[c]).map(|ai| apps[ai].decay > 0).unwrap_or(false);
                let mut c = match kit {
                    Some(k) => wall_colour(&k.wall, x, y, 11 + zone.unwrap() as u32, k.accent),
                    None if x < HUB => ramp(hex("#d6d0c4"), 0.0),
                    None => wall_colour(&WallFinish::Paint { c: hex("#d9cbb4"), dado: hex("#8a6a4a") }, x, y, 3, hex("#888888")),
                };
                // windows onto the evening
                let in_window = all_windows.iter().any(|&(a, b)| x > a && x < b) && y > 0.95 && y < 2.75;
                if in_window {
                    let frame = all_windows.iter().any(|&(a, b)| (x - a).abs() < 0.06 || (x - b).abs() < 0.06 || (x - (a + b) / 2.0).abs() < 0.03) || (y - 0.95).abs() < 0.06 || (y - 2.75).abs() < 0.06 || (y - 2.2).abs() < 0.025;
                    c = if frame { hex("#efe8da") } else { dusk(1.0 - (y - 0.95) / 1.8, x, 21) };
                }
                // light: brighter low and near lamps, dimmer near the ceiling; the skirting
                let mut l = 0.9 - (y / WALL) * 0.12;
                if y < 0.12 { c = ramp(c, -0.45); }
                if decayed && !in_window { l *= 0.7; }
                let warmth = kit.map(|k| k.warmth).unwrap_or(0.7);
                c = mul(c, [l * (0.95 + 0.1 * warmth), l, l * (1.05 - 0.12 * warmth)]);
                img.blend(px, py, c);
            }
        }
        // the ceiling edge of the back wall
        img.poly(&cam.quad([[X0, WALL, Z0], [X1, WALL, Z0], [X1, WALL, Z0 - 0.35], [X0, WALL, Z0 - 0.35]]), hex("#2d2838"));
        // ---- the glass curtain wall on the left, per pixel ----
        for py in 0..FH {
            for px in 0..FW {
                let Some((y, z)) = cam.on_plane_x(px as f32 + 0.5, py as f32 + 0.5, X0) else { continue };
                if !(0.0..=WALL).contains(&y) || !(Z0..=Z1).contains(&z) { continue; }
                let frame = (z / 1.5).fract() < 0.035 || y < 0.12 || (y - 1.0).abs() < 0.03 || y > WALL - 0.08;
                let c = if frame { hex("#d9d2c6") } else { mix(dusk(1.0 - y / WALL, z, 31), hex("#bfe0f2"), 0.18) };
                img.blend(px, py, alpha(c, if frame { 255 } else { 215 }));
            }
        }
        // the slab edge and the low stubs of the cut-away front and right walls
        img.poly(&cam.quad([[X0, 0.0, Z1], [X1, 0.0, Z1], [X1, -0.5, Z1], [X0, -0.5, Z1]]), hex("#3b3446"));
        img.poly(&cam.quad([[X1, 0.0, Z0], [X1, 0.0, Z1], [X1, -0.5, Z1], [X1, -0.5, Z0]]), hex("#2d2838"));
        draw_box(&mut img, &cam, (0.0, 0.0), [X0, 0.0, Z1 - 0.15], [X1, 0.3, Z1], hex("#d8d0c2"));
        draw_box(&mut img, &cam, (0.0, 0.0), [X1 - 0.15, 0.0, Z0], [X1, 0.3, Z1], hex("#cfc7b8"));
        // ---- the lift hub: the floor's landmark, with the floor number and a directory ----
        let n_txt = format!("{}", floor + 1);
        let hub_c = cam.p([(X0 + HUB) / 2.0, 2.95, Z0]);
        let lift = cam.quad([[X0 + 0.9, 0.0, Z0], [HUB - 0.9, 0.0, Z0], [HUB - 0.9, 2.3, Z0], [X0 + 0.9, 2.3, Z0]]);
        img.poly(&lift, hex("#8d94a3"));
        let mid = cam.p([(X0 + HUB) / 2.0, 0.0, Z0]);
        img.rect(mid.0 as i32, cam.p([0.0, 2.3, Z0]).1 as i32, 1, (mid.1 - cam.p([0.0, 2.3, Z0]).1) as i32, hex("#5a5f6e"));
        let (nw, _) = (text_width(&n_txt, 3), 0);
        img.rect(hub_c.0 as i32 - nw / 2 - 5, hub_c.1 as i32 - 13, nw + 10, 25, hex("#1f2230"));
        draw_text(&mut img, hub_c.0 as i32 - nw / 2, hub_c.1 as i32 - 10, &n_txt, hex("#ffd27a"), 3);
        beacons.push((ox + hub_c.0 - 30.0, oy + hub_c.1 - 8.0));
        // a directory totem in the lift lobby: one line per area, with its colour
        let names: Vec<(String, Rgba)> = zone_apps.iter().enumerate().map(|(c, a)| match a { Some(ai) => (apps[*ai].name.to_uppercase(), kits[*ai].accent), None => (if c == 3 { "KITCHEN · LOUNGE".to_string() } else { String::new() }, hex("#c9c3b5")) }).filter(|(n, _)| !n.is_empty()).collect();
        let dw = names.iter().map(|(n, _)| text_width(n, 1)).max().unwrap_or(40) + 16;
        let dh = names.len() as i32 * 9 + 14;
        let foot = cam.p([(X0 + HUB) / 2.0 + 0.3, 0.0, -3.2]);
        let mut tot = Img::new(dw + 4, dh + 26);
        tot.rect(dw / 2 - 1, dh, 4, 24, hex("#3a3f4c"));
        tot.rect(dw / 2 - 6, dh + 22, 14, 3, hex("#2a2d36"));
        tot.rect(1, 1, dw, dh, hex("#262a36"));
        tot.rect(1, 1, dw, 2, hex("#8787f6"));
        draw_text(&mut tot, 5, 5, &format!("FLOOR {}", floor + 1), hex("#ffd27a"), 1);
        for (k, (n, col)) in names.iter().enumerate() { tot.rect(5, 15 + k as i32 * 9, 4, 7, *col); draw_text(&mut tot, 12, 15 + k as i32 * 9, n, hex("#e9e4d8"), 1); }
        tot.outline();
        let (tx, ty) = (ox + foot.0 - (dw / 2 + 2) as f32, oy + foot.1 - (dh + 25) as f32);
        items.push(Item { s: pk.add(tot), x: tx, y: ty, depth: oy + foot.1 });

        // the lobby: a bench, plants, a rug by the lift
        {
            let mut lr = Rng::of(&format!("lobby|{floor}"));
            for &(px, pz) in &[(HUB - 0.6, -7.8), (X0 + 1.0, 7.5), (HUB - 0.6, 4.0)] {
                let pts = corners([px - 0.5, 0.0, pz - 0.5], [px + 0.5, 1.9, pz + 0.5]);
                let (bx, by, bw, bh) = bounds(&cam, &pts, 6);
                let mut im = Img::new(bw, bh);
                floor_plant(&mut im, &cam, (bx as f32, by as f32), px, pz, *lr.pick(&[Plant::Monstera, Plant::Palm, Plant::Fern]), 1.7, false);
                im.outline();
                items.push(Item { s: pk.add(im), x: ox + bx as f32, y: oy + by as f32, depth: oy + cam.p([px, 0.0, pz + 0.2]).1 });
            }
            let (bx0, bz0) = (X0 + 1.6, 1.4);
            let pts = corners([bx0, 0.0, bz0], [bx0 + 2.2, 0.9, bz0 + 0.7]);
            let (bx, by, bw, bh) = bounds(&cam, &pts, 3);
            let mut im = Img::new(bw, bh);
            let off = (bx as f32, by as f32);
            draw_box(&mut im, &cam, off, [bx0, 0.0, bz0], [bx0 + 0.08, 0.42, bz0 + 0.6], hex("#2a2a30"));
            draw_box(&mut im, &cam, off, [bx0 + 2.1, 0.0, bz0], [bx0 + 2.18, 0.42, bz0 + 0.6], hex("#2a2a30"));
            draw_box(&mut im, &cam, off, [bx0, 0.42, bz0], [bx0 + 2.2, 0.5, bz0 + 0.6], hex("#b07a4a"));
            im.outline();
            items.push(Item { s: pk.add(im), x: ox + off.0, y: oy + off.1, depth: oy + cam.p([bx0 + 1.1, 0.0, bz0 + 0.6]).1 });
        }

        // ---- each area: wall decor, sign, desks, people ----
        for c in 0..4 {
            let (zx0, zx1) = zone_x(c);
            match zone_apps[c] {
                Some(ai) => {
                    let (app, kit) = (&apps[ai], &kits[ai]);
                    let mut r = Rng::of(&format!("area|{}", app.id));
                    wall_decor(&mut img, &cam, kit, zx0, zx1, &windows_of(c), app.decay > 0, &mut r);
                    let sign_rect = area_sign(&mut img, &cam, kit, app, zx0, zx1);
                    hits.push(Hit { x: ox + sign_rect.0, y: oy + sign_rect.1, w: sign_rect.2, h: sign_rect.3, target: Target::App(ai), z: 2.0 });
                    let zone_quad = [cam.p([zx0, 0.0, -8.4]), cam.p([zx1, 0.0, -8.4]), cam.p([zx1, 0.0, Z1 - 0.6]), cam.p([zx0, 0.0, Z1 - 0.6])];
                    let (qx0, qy0) = (zone_quad.iter().map(|p| p.0).fold(f32::MAX, f32::min), cam.p([zx0, WALL, Z0]).1);
                    let (qx1, qy1) = (zone_quad.iter().map(|p| p.0).fold(f32::MIN, f32::max), zone_quad[2].1);
                    hits.push(Hit { x: ox + qx0, y: oy + qy0, w: qx1 - qx0, h: qy1 - qy0, target: Target::App(ai), z: 0.0 });
                    // the far-zoom name chip
                    let chip = name_chip(app.name, kit.accent, app.tool.monogram());
                    let cpos = cam.p([(zx0 + zx1) / 2.0, WALL, Z0]);
                    labels.push(Label { x: ox + cpos.0, y: oy + cpos.1 - 6.0, s: pk.add(chip), floor });
                    // a pennant or totem at the area's front, in the App's colour
                    let pen = pennant(&cam, kit, app, zx0);
                    items.push(Item { s: pk.add(pen.0), x: ox + pen.1, y: oy + pen.2, depth: oy + pen.3 });
                    place_area(&mut pk, &mut items, &mut actors, &mut hits, &cam, (ox, oy), kit, app, ai, &plans[c], zx0, zx1, &mut r);
                }
                None => commons(&mut pk, &mut items, &mut actors, &mut img, &cam, (ox, oy), zx0, zx1, floor),
            }
        }
        // walkers in the aisles, between areas and along the back
        let lk_seed = format!("walk|{floor}");
        let mut wr = Rng::of(&lk_seed);
        for k in 0..3 {
            let lk = Look::random(&mut wr, hex("#c9c3b5"));
            let frames = |pk: &mut Packer, p: Pose| pk.add(people::paint(&lk, p));
            let down = [frames(&mut pk, Pose::WalkDown(0)), frames(&mut pk, Pose::WalkDown(1))];
            let up = [frames(&mut pk, Pose::WalkUp(0)), frames(&mut pk, Pose::WalkUp(1))];
            let right = [frames(&mut pk, Pose::WalkRight(0)), frames(&mut pk, Pose::WalkRight(1))];
            let left = [pk.add(people::paint_mirrored(&lk, Pose::WalkRight(0))), pk.add(people::paint_mirrored(&lk, Pose::WalkRight(1)))];
            let ax = HUB + ZONE_W * (k as f32 + 1.0);
            let loop_w: Vec<(f32, f32)> = match k {
                0 => vec![(ax, -8.6), (ax, Z1 - 0.9), (ax + 0.3, Z1 - 0.9), (ax + 0.3, -8.6)],
                1 => vec![(HUB + 0.5, -8.7), (X1 - 1.5, -8.7), (X1 - 1.5, -8.5), (HUB + 0.5, -8.5)],
                _ => vec![(ax, Z1 - 0.8), (ax, -8.6), (ax - 0.3, -8.6), (ax - 0.3, Z1 - 0.8)],
            };
            let path: Vec<(f32, f32)> = loop_w.iter().map(|&(x, z)| { let p = cam.p([x, 0.0, z]); (ox + p.0, oy + p.1) }).collect();
            walkers.push(Walker { path, speed: 24.0, offset: k as f32 * 140.0 + floor as f32 * 60.0, down, up, left, right, ax: people::FOOT_X as f32, ay: people::FOOT_Y_STAND as f32 });
        }
        bg.blit(&img, ox as i32, oy as i32);
        hits.push(Hit { x: ox, y: oy, w: FW as f32, h: FH as f32, target: Target::Floor(floor), z: -1.0 });
    }
    // screen-space UI chips
    let mut ui = Vec::new();
    for (key, text) in [("f2", "FLOOR 2"), ("f1", "FLOOR 1"), ("bld", "BUILDING"), ("alert", "RED ALERT · THIS MAC AT 97% FOR 6 MIN")] {
        let w = text_width(text, 1) + 12;
        let mut im = Img::new(w, 13);
        let bgc = if key == "alert" { hex("#5a1018") } else { hex("#151a23") };
        im.rect(0, 0, w, 13, bgc);
        im.rect(0, 0, w, 1, ramp(bgc, 0.3));
        draw_text(&mut im, 6, 3, text, if key == "alert" { hex("#ffd9d9") } else { hex("#e9e6f0") }, 1);
        ui.push((key.to_string(), pk.add(im)));
    }
    let (atlas, sprites) = pk.pack();
    World { w: cw, h: ch, bg, atlas, sprites, items, actors, walkers, hits, labels, floors: floor_boxes, apps, kits, white, ui, alert_beacons: beacons, beacon }
}

// ---------- areas ----------
#[derive(Clone, Copy)]
struct Desk { x: f32, z: f32, w: f32, d: f32, faces_cam: bool, table: bool }

/// Desks for an area, from its kit's layout. Fixed by the kit, so the area keeps its shape.
fn plan_desks(kit: &Kit, app: &AppDef, zx0: f32, zx1: f32) -> Vec<Desk> {
    let mut v = Vec::new();
    let w = zx1 - zx0;
    let nook_left = nook_rect(kit, zx0, zx1).map(|(a, _, _, _)| a < zx0 + w / 2.0).unwrap_or(false);
    let x_start = if nook_left { zx0 + w * 0.42 } else { zx0 + 0.5 };
    let avail = w * 0.58 - 0.6;
    let row = |v: &mut Vec<Desk>, x: f32, z: f32, n: usize, dw: f32, two: bool, table: bool| {
        for i in 0..n {
            v.push(Desk { x: x + i as f32 * dw, z, w: dw, d: if table { 0.5 } else { 0.7 }, faces_cam: true, table });
            if two { v.push(Desk { x: x + i as f32 * dw, z: z + if table { 0.5 } else { 0.7 }, w: dw, d: if table { 0.5 } else { 0.7 }, faces_cam: false, table }); }
        }
    };
    match kit.layout {
        Layout::Pods => { for rr in 0..4 { for cc in 0..2 { row(&mut v, x_start + cc as f32 * (avail / 2.0 + 0.1), -6.6 + rr as f32 * 4.6, 2, 1.4, true, false); } } }
        Layout::Bench => { for rr in 0..4 { row(&mut v, x_start, -6.4 + rr as f32 * 4.5, ((avail + 0.2) / 1.4) as usize, 1.4, true, false); } }
        Layout::Clusters => { for rr in 0..3 { row(&mut v, x_start + if rr == 1 { 0.6 } else { 0.0 }, -6.0 + rr as f32 * 5.8, 3, 1.35, true, false); } }
        Layout::Studio => { for rr in 0..4 { for cc in 0..3 { v.push(Desk { x: x_start + cc as f32 * (avail / 3.0 + 0.2), z: -6.6 + rr as f32 * 4.6 + if cc % 2 == 1 { 0.8 } else { 0.0 }, w: 1.4, d: 0.7, faces_cam: false, table: false }); } } }
        Layout::Tables => { for rr in 0..4 { row(&mut v, x_start, -6.4 + rr as f32 * 4.5, ((avail + 0.2) / 1.2) as usize, 1.2, true, true); } }
    }
    let _ = app;
    v
}

/// Where the area's shared nook goes (x0, x1, z0, z1), on a rug.
fn nook_rect(kit: &Kit, zx0: f32, zx1: f32) -> Option<(f32, f32, f32, f32)> {
    let left = matches!(kit.tool, Tool::Codex | Tool::OpenCode);
    let w = zx1 - zx0;
    let (a, b) = if left { (zx0 + 0.3, zx0 + w * 0.38) } else { (zx1 - w * 0.4, zx1 - 0.3) };
    Some((a, b, 5.2, 10.8))
}

fn wall_decor(img: &mut Img, cam: &Cam, kit: &Kit, zx0: f32, zx1: f32, windows: &[(f32, f32)], decayed: bool, r: &mut Rng) {
    let z = Z0 + 0.02;
    // gaps between windows get the kit's wall things
    let mut gaps = vec![];
    let mut x = zx0 + 0.3;
    for &(a, b) in windows { if a - x > 1.0 { gaps.push((x, a - 0.2)); } x = b + 0.2; }
    if zx1 - 0.3 - x > 1.0 { gaps.push((x, zx1 - 0.3)); }
    for (gi, &(a, b)) in gaps.iter().enumerate() {
        let w = b - a;
        let decor = kit.decor[(gi + r.int(3)) % kit.decor.len()];
        match decor {
            Decor::Bookshelf | Decor::Records | Decor::Shelving => {
                let (x0, x1) = (a + 0.1, (a + 1.6).min(b));
                let frame = if decor == Decor::Shelving { hex("#2a2c31") } else { hex("#5a3d2a") };
                draw_box(img, cam, (0.0, 0.0), [x0, 0.0, Z0], [x1, 2.1, Z0 + 0.35], frame);
                for s in 0..5 {
                    let y = 0.15 + s as f32 * 0.4;
                    let mut bx = x0 + 0.05;
                    while bx < x1 - 0.08 {
                        let bw = r.range(0.04, 0.09);
                        let bh = r.range(0.2, 0.33);
                        let col = hex(r.pick(&["#a8584a", "#3f5f8a", "#c9a46c", "#4a7a5a", "#d9c7a0", "#6b4a7a", "#2f3a4a"]));
                        if !(decayed && r.chance(0.3)) { img.poly(&cam.quad([[bx, y, Z0 + 0.35], [bx + bw, y, Z0 + 0.35], [bx + bw, y + bh, Z0 + 0.35], [bx, y + bh, Z0 + 0.35]]), col); }
                        bx += bw + 0.01;
                    }
                }
            }
            Decor::Posters => {
                for k in 0..((w / 1.1) as usize).min(3) {
                    let px0 = a + 0.2 + k as f32 * 1.1;
                    let q = cam.quad([[px0, 1.3, z], [px0 + 0.8, 1.3, z], [px0 + 0.8, 2.3, z], [px0, 2.3, z]]);
                    img.poly(&q, hex("#f4efe6"));
                    let inner = cam.quad([[px0 + 0.06, 1.36, z], [px0 + 0.74, 1.36, z], [px0 + 0.74, 2.24, z], [px0 + 0.06, 2.24, z]]);
                    let (c1, c2) = (hex(r.pick(&["#e8b33a", "#d9644a", "#3f7fbf", "#4aa37a", "#c86f4f"])), hex(r.pick(&["#2f3a4a", "#f1e6d6", "#7a3b4a"])));
                    let shape = r.int(3);
                    img.poly_with(&inner, |px, py| {
                        let p = cam.on_plane_z(px as f32 + 0.5, py as f32 + 0.5, z)?;
                        let (u, v) = ((p.0 - px0) / 0.8, (p.1 - 1.3) / 1.0);
                        Some(match shape { 0 => if (u - 0.5).powi(2) + (v - 0.55).powi(2) < 0.06 { c1 } else { c2 }, 1 => if v > u { c1 } else { c2 }, _ => if ((u * 4.0) as i32 + (v * 5.0) as i32) % 2 == 0 { c1 } else { c2 } })
                    });
                }
            }
            Decor::Whiteboard => {
                let q = cam.quad([[a + 0.1, 0.95, z], [(a + 2.2).min(b), 0.95, z], [(a + 2.2).min(b), 2.1, z], [a + 0.1, 2.1, z]]);
                img.poly(&q, hex("#9aa2b0"));
                let qi = cam.quad([[a + 0.15, 1.0, z], [(a + 2.15).min(b - 0.05), 1.0, z], [(a + 2.15).min(b - 0.05), 2.05, z], [a + 0.15, 2.05, z]]);
                img.poly(&qi, hex("#f5f6f8"));
                for n in 0..7 { let (nx, ny) = (a + 0.3 + (n % 4) as f32 * 0.42, 1.75 - (n / 4) as f32 * 0.38); if nx + 0.25 < b { img.poly(&cam.quad([[nx, ny, z], [nx + 0.25, ny, z], [nx + 0.25, ny + 0.2, z], [nx, ny + 0.2, z]]), hex(r.pick(&["#ffe28a", "#8fd3ff", "#9be8b5", "#ffb3c7"]))); } }
            }
            Decor::Pegboard => {
                let q = cam.quad([[a + 0.1, 1.0, z], [(a + 1.8).min(b), 1.0, z], [(a + 1.8).min(b), 2.2, z], [a + 0.1, 2.2, z]]);
                img.poly_with(&q, |px, py| { let p = cam.on_plane_z(px as f32 + 0.5, py as f32 + 0.5, z)?; Some(if (p.0 * 12.0).fract() < 0.3 && (p.1 * 12.0).fract() < 0.3 { hex("#8a6a42") } else { hex("#c9a46c") }) });
                for t in 0..6 { let tx = a + 0.25 + t as f32 * 0.25; if tx < b - 0.1 { let p0 = cam.p([tx, 1.85, z]); let p1 = cam.p([tx, 1.45 - (t % 3) as f32 * 0.1, z]); img.line(p0, p1, hex(r.pick(&["#d9644a", "#3f7fbf", "#2a2a2a", "#e8b33a"]))); } }
            }
            Decor::Bikes => {
                for k in 0..2 { let bx = a + 0.4 + k as f32 * 0.9; if bx + 0.7 > b { break; } let c0 = cam.p([bx, 1.6, z]); let c1 = cam.p([bx + 0.6, 1.6, z]); let rr = cam.scale_at([bx, 1.6, z]) * 0.22; let col = hex(r.pick(&["#d9644a", "#3f7fbf", "#4aa37a"])); for cc in [c0, c1] { img.disc(cc.0, cc.1, rr, hex("#25252b")); img.disc(cc.0, cc.1, rr - 1.5, alpha(hex("#000000"), 0)); } img.line(c0, c1, col); img.line(c0, ((c0.0 + c1.0) / 2.0, c0.1 - rr), col); img.line(((c0.0 + c1.0) / 2.0, c0.1 - rr), c1, col); }
            }
            Decor::Lamps | Decor::Pendants | Decor::Sofa | Decor::Beanbags | Decor::Fishtank => {
                // a framed picture and a wall light
                let q = cam.quad([[a + 0.3, 1.4, z], [a + 1.2, 1.4, z], [a + 1.2, 2.1, z], [a + 0.3, 2.1, z]]);
                img.poly(&q, hex("#3a2a20"));
                let qi = cam.quad([[a + 0.36, 1.46, z], [a + 1.14, 1.46, z], [a + 1.14, 2.04, z], [a + 0.36, 2.04, z]]);
                let (c1, c2) = (hex(r.pick(&["#7a9a6e", "#c9925c", "#6b7fa8"])), hex(r.pick(&["#e6dcc0", "#c45f6a"])));
                img.poly_with(&qi, |px, py| { let p = cam.on_plane_z(px as f32 + 0.5, py as f32 + 0.5, z)?; Some(if p.1 < 1.62 + (p.0 * 3.0).sin() * 0.08 { c1 } else { c2 }) });
            }
        }
    }
    // a plant or two against the wall
    let plants = (kit.greenery * 3.0).round() as usize;
    for k in 0..plants.min(gaps.len().max(1)) {
        let (a, b) = gaps.get(k).copied().unwrap_or((zx0 + 0.3, zx0 + 1.2));
        let px = if k % 2 == 0 { b - 0.35 } else { a + 0.35 };
        floor_plant(img, cam, (0.0, 0.0), px, Z0 + 0.45, *r.pick(&kit.plants), 1.3, decayed);
    }
}

/// The area's sign: a plate high on its back wall with the App's colour, the tool's mark and the App's
/// name, always in the same place and the same layout; the kit sets only its material and shape.
fn area_sign(img: &mut Img, cam: &Cam, kit: &Kit, app: &AppDef, zx0: f32, zx1: f32) -> (f32, f32, f32, f32) {
    let name = app.name.to_uppercase();
    let mono = app.tool.monogram();
    let cx = (zx0 + zx1) / 2.0;
    let mid = cam.p([cx, 3.0, Z0 + 0.03]);
    let tw = text_width(&name, 1);
    let (w, h) = (tw + 30, 19);
    let (x, y) = (mid.0 as i32 - w / 2, mid.1 as i32 - h / 2);
    let acc = kit.accent;
    let dim = app.decay > 0;
    let ink = if (acc[0] as u32 * 299 + acc[1] as u32 * 587 + acc[2] as u32 * 114) / 1000 > 150 { hex("#1d1726") } else { hex("#ffffff") };
    match kit.sign {
        SignKind::Banner => {
            img.rect(x - 4, y - 3, w + 8, 2, hex("#4a3a2c"));
            img.rect(x, y - 1, w, h, acc);
            img.rect(x, y - 1, w, 1, ramp(acc, 0.3));
            for k in 0..w / 6 { img.rect(x + 3 + k * 6, y + h - 1, 3, 2, ramp(acc, -0.3)); }
        }
        SignKind::Neon => {
            img.rect(x - 2, y - 2, w + 4, h + 4, hex("#121318"));
            for gy in -3..h + 3 { for gx in -3..w + 3 { if gx < 0 || gy < 0 || gx >= w || gy >= h { img.blend(x + gx, y + gy, alpha(acc, 40)); } } }
        }
        SignKind::Plaque => {
            img.rect(x - 2, y - 2, w + 4, h + 4, hex("#5a3d2a"));
            img.rect(x, y, w, h, hex("#c9965e"));
            img.rect(x, y + h - 3, w, 3, acc);
        }
        SignKind::Blade => {
            img.rect(x - 1, y - 1, w + 2, h + 2, hex("#1c1d21"));
            img.rect(x, y, w, h, hex("#2d3036"));
            img.rect(x, y, 3, h, acc);
        }
        SignKind::Pennant => {
            for k in 0..(w / 8) { let fx = x + k * 8; let col = if k % 2 == 0 { acc } else { hex("#f0e8da") }; for t in 0..5 { img.rect(fx + t / 2, y - 6 + t, 6 - t, 1, col); } }
            img.rect(x, y, w, h, hex("#f0e8da"));
            img.rect(x, y, w, 2, acc);
        }
    }
    // the tool's mark: a tile in the App's colour with the monogram
    let (mx, my) = (x + 4, y + 3);
    img.rect(mx, my, 15, 13, if kit.sign == SignKind::Banner { ramp(acc, -0.35) } else { acc });
    draw_text(img, mx + 2, my + 3, mono, if kit.sign == SignKind::Banner { hex("#ffffff") } else { ink }, 1);
    let text_col = match kit.sign {
        SignKind::Banner => ink,
        SignKind::Neon => ramp(acc, 0.55),
        SignKind::Plaque => hex("#2a1c12"),
        SignKind::Blade => hex("#f0eef5"),
        SignKind::Pennant => hex("#1d1726"),
    };
    draw_text(img, x + 23, y + 6, &name, text_col, 1);
    if dim { img.rect(x - 4, y - 6, w + 8, h + 10, alpha(hex("#2a2a35"), 110)); }
    (x as f32, y as f32 - 6.0, w as f32, h as f32 + 8.0)
}

fn name_chip(name: &str, acc: Rgba, mono: &str) -> Img {
    // two lines, the tool then the profile, so a chip fits over its area when zoomed out
    let name = name.to_uppercase();
    let (a, b) = name.split_once(" · ").unwrap_or((&name, ""));
    let tw = text_width(a, 1).max(text_width(b, 1));
    let mut im = Img::new(tw + 26, 21);
    im.rect(0, 0, tw + 26, 21, hex("#151a23"));
    im.rect(0, 0, 2, 21, acc);
    im.rect(4, 4, 15, 13, acc);
    draw_text(&mut im, 6, 7, mono, hex("#ffffff"), 1);
    draw_text(&mut im, 22, 3, a, hex("#f3f5f9"), 1);
    draw_text(&mut im, 22, 12, b, ramp(acc, 0.45), 1);
    im
}

fn pennant(cam: &Cam, kit: &Kit, app: &AppDef, zx0: f32) -> (Img, f32, f32, f32) {
    let base = [zx0 + 0.35, 0.0, Z1 - 1.0];
    let pts = vec![base, [base[0], 1.9, base[2]], [base[0] + 0.7, 1.9, base[2]]];
    let (bx, by, bw, bh) = bounds(cam, &pts, 4);
    let mut im = Img::new(bw, bh);
    let o = |p: V3| { let q = cam.p(p); (q.0 - bx as f32, q.1 - by as f32) };
    let (b0, t0) = (o(base), o([base[0], 1.9, base[2]]));
    im.rect(b0.0 as i32 - 1, t0.1 as i32, 2, (b0.1 - t0.1) as i32, hex("#3a3f4c"));
    im.ellipse(b0.0, b0.1, 4.0, 1.5, hex("#2a2d36"));
    let acc = if app.decay > 0 { mix(kit.accent, hex("#6d6a72"), 0.5) } else { kit.accent };
    let f0 = o([base[0], 1.9, base[2]]);
    let f1 = o([base[0] + 0.7, 1.75, base[2]]);
    let f2 = o([base[0], 1.45, base[2]]);
    im.poly(&[f0, f1, f2], acc);
    im.poly(&[f0, ((f0.0 + f1.0) / 2.0, (f0.1 + f1.1) / 2.0), f2], ramp(acc, 0.2));
    draw_text(&mut im, f0.0 as i32 + 2, f0.1 as i32 + 3, &app.tool.monogram()[..1], hex("#ffffff"), 1);
    im.outline();
    (im, bx as f32, by as f32, cam.p(base).1)
}

fn floor_plant(img: &mut Img, cam: &Cam, off: (f32, f32), x: f32, z: f32, kind: Plant, size: f32, wilted: bool) {
    let pot = match kind { Plant::Cactus | Plant::Herbs => hex("#c9b8a0"), Plant::Olive | Plant::Pothos => hex("#e6e0d6"), _ => hex("#b4643a") };
    let s = size;
    draw_box(img, cam, off, [x - 0.2 * s, 0.0, z - 0.2 * s], [x + 0.2 * s, 0.42 * s, z + 0.2 * s], pot);
    let top = cam.p([x, 0.42 * s, z]);
    let (tx, ty) = (top.0 - off.0, top.1 - off.1);
    let k = cam.scale_at([x, 0.0, z]);
    let (leaf, dark, light) = if wilted { (hex("#8a7a4a"), hex("#6a5a34"), hex("#a8955a")) } else {
        match kind { Plant::Snake => (hex("#4a7a4a"), hex("#2f5a3a"), hex("#a8c070")), Plant::Cactus => (hex("#5a9a5a"), hex("#3a7040"), hex("#8ac07a")), Plant::Olive => (hex("#8aa07a"), hex("#5a7050"), hex("#b8c8a0")), _ => (hex("#3f9b55"), hex("#2b7240"), hex("#6cc77c")) }
    };
    match kind {
        Plant::Snake | Plant::Cactus => {
            for i in 0..5 { let lx = tx + (i as f32 - 2.0) * k * 0.08; let h = k * s * (0.5 + (i % 3) as f32 * 0.15); img.rect(lx as i32, (ty - h) as i32, (k * 0.06).max(2.0) as i32, h as i32, if i % 2 == 0 { leaf } else { dark }); img.rect(lx as i32, (ty - h) as i32, 1, (h * 0.6) as i32, light); }
        }
        Plant::Palm | Plant::Olive => {
            img.rect(tx as i32, (ty - k * s * 0.7) as i32, 2, (k * s * 0.7) as i32, hex("#6b4a32"));
            let cy = ty - k * s * 0.75;
            img.ellipse(tx, cy, k * s * 0.42, k * s * 0.28, dark);
            img.ellipse(tx - 2.0, cy - 2.0, k * s * 0.34, k * s * 0.22, leaf);
            img.ellipse(tx - 3.0, cy - 4.0, k * s * 0.14, k * s * 0.09, light);
        }
        _ => {
            img.ellipse(tx, ty - k * s * 0.3, k * s * 0.36, k * s * 0.3, dark);
            img.ellipse(tx - 1.5, ty - k * s * 0.38, k * s * 0.3, k * s * 0.24, leaf);
            img.ellipse(tx - 3.0, ty - k * s * 0.48, k * s * 0.12, k * s * 0.1, light);
            if kind == Plant::Pothos { for t in 0..4 { img.rect((tx - k * 0.3 + t as f32 * k * 0.2) as i32, ty as i32, 1, (k * 0.25) as i32, leaf); } }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn place_area(pk: &mut Packer, items: &mut Vec<Item>, actors: &mut Vec<Actor>, hits: &mut Vec<Hit>, cam: &Cam, (ox, oy): (f32, f32), kit: &Kit, app: &AppDef, ai: usize, plan: &[Desk], zx0: f32, zx1: f32, r: &mut Rng) {
    let decayed = app.decay > 0;
    // the nook: a sofa and armchairs, beanbags, a huddle table, a reading corner or a workbench
    if let Some((a, b, z0, z1)) = nook_rect(kit, zx0, zx1) {
        let (cx, cz) = ((a + b) / 2.0, (z0 + z1) / 2.0);
        let mut pts = corners([a, 0.0, z0], [b, 1.6, z1]);
        pts.push([a, 0.0, z1]);
        let (bx, by, bw, bh) = bounds(cam, &pts, 4);
        let mut im = Img::new(bw, bh);
        let off = (bx as f32, by as f32);
        match kit.nook {
            Nook::Sofa | Nook::Library => {
                let sofa = if kit.nook == Nook::Library { hex("#7a4a32") } else { ramp(kit.chair, 0.1) };
                draw_box(&mut im, cam, off, [cx - 1.0, 0.0, cz - 1.0], [cx + 1.0, 0.45, cz - 0.3], sofa);
                draw_box(&mut im, cam, off, [cx - 1.0, 0.45, cz - 1.0], [cx + 1.0, 0.85, cz - 0.8], ramp(sofa, -0.1));
                draw_box(&mut im, cam, off, [cx - 0.4, 0.0, cz + 0.1], [cx + 0.4, 0.35, cz + 0.6], hex("#c99764"));
                draw_box(&mut im, cam, off, [cx + 1.2, 0.0, cz + 0.2], [cx + 1.8, 0.45, cz + 0.9], sofa);
                if kit.nook == Nook::Library { draw_box(&mut im, cam, off, [cx - 1.6, 0.0, cz + 0.4], [cx - 1.2, 1.4, cz + 1.4], hex("#5a3d2a")); }
                let lamp = cam.p([cx + 1.6, 1.5, cz - 0.6]);
                im.rect((lamp.0 - off.0) as i32, (lamp.1 - off.1) as i32, 1, (cam.p([cx + 1.6, 0.0, cz - 0.6]).1 - lamp.1) as i32, hex("#2a2a2a"));
                im.ellipse(lamp.0 - off.0, lamp.1 - off.1, 5.0, 3.0, hex("#f2d49a"));
            }
            Nook::Beanbags => {
                for (k, col) in ["#d9644a", "#3f7fbf", "#e8b33a", "#4aa37a"].iter().enumerate() {
                    let p = cam.p([cx - 1.0 + (k % 2) as f32 * 1.6, 0.25, cz - 0.8 + (k / 2) as f32 * 1.4]);
                    im.ellipse(p.0 - off.0, p.1 - off.1, 9.0, 6.0, ramp(hex(col), -0.25));
                    im.ellipse(p.0 - off.0 - 1.0, p.1 - off.1 - 2.0, 7.0, 4.5, hex(col));
                }
            }
            Nook::Huddle => {
                draw_box(&mut im, cam, off, [cx - 0.8, 0.7, cz - 0.6], [cx + 0.8, 0.76, cz + 0.6], kit.desk_top);
                draw_box(&mut im, cam, off, [cx - 0.08, 0.0, cz - 0.08], [cx + 0.08, 0.7, cz + 0.08], kit.desk_frame);
                for k in 0..4 { let (sx, sz) = [(-1.1, 0.0), (1.1, 0.0), (0.0, -0.9), (0.0, 0.9)][k]; draw_box(&mut im, cam, off, [cx + sx - 0.22, 0.0, cz + sz - 0.22], [cx + sx + 0.22, 0.45, cz + sz + 0.22], kit.chair); }
                let wb = [cx - 0.5, 0.0, cz + 1.6];
                draw_box(&mut im, cam, off, [cx - 0.5, 0.0, cz + 1.6], [cx - 0.46, 1.6, cz + 1.66], hex("#8d94a3"));
                draw_box(&mut im, cam, off, [cx + 0.96, 0.0, cz + 1.6], [cx + 1.0, 1.6, cz + 1.66], hex("#8d94a3"));
                draw_box(&mut im, cam, off, [wb[0], 0.7, wb[2]], [cx + 1.0, 1.5, cz + 1.66], hex("#f2f3f7"));
                for n in 0..6 { let p = cam.p([cx - 0.35 + (n % 3) as f32 * 0.42, 1.38 - (n / 3) as f32 * 0.28, cz + 1.67]); im.rect((p.0 - off.0) as i32, (p.1 - off.1) as i32, 5, 4, hex(r.pick(&["#ffe28a", "#8fd3ff", "#9be8b5", "#ffb3c7"]))); }
                let p = cam.p([cx - 0.3, 0.95, cz + 1.67]);
                im.line((p.0 - off.0, p.1 - off.1), (p.0 - off.0 + 18.0, p.1 - off.1 - 3.0), hex("#3f5f8a"));
            }
            Nook::Workbench => {
                draw_box(&mut im, cam, off, [cx - 1.2, 0.0, cz - 0.5], [cx + 1.2, 0.9, cz + 0.4], hex("#b8935a"));
                for k in 0..5 { let p = cam.p([cx - 1.0 + k as f32 * 0.45, 0.92, cz - 0.1]); im.rect((p.0 - off.0) as i32, (p.1 - off.1) as i32 - 3, 4, 3, hex(r.pick(&["#3f7fbf", "#d9644a", "#2f6b3a", "#e8b33a", "#5a5f6e"]))); }
                let s0 = cam.p([cx - 1.2, 0.95, cz - 0.45]);
                im.rect((s0.0 - off.0) as i32 + 4, (s0.1 - off.1) as i32 - 6, 8, 5, hex("#2f6b3a"));
            }
        }
        if decayed { dust(&mut im, r); }
        im.outline();
        items.push(Item { s: pk.add(im), x: ox + bx as f32, y: oy + by as f32, depth: oy + cam.p([cx, 0.0, z1]).1 });
    }
    // a second setting beside the desks: a reading corner, a round table, lockers and a printer, ping-pong
    {
        let w = zx1 - zx0;
        let nook_left = matches!(kit.tool, Tool::Codex | Tool::OpenCode);
        let (fx0, fx1) = if nook_left { (zx0 + 0.4, zx0 + w * 0.38) } else { (zx0 + w * 0.68, zx1 - 0.4) };
        let cx = (fx0 + fx1) / 2.0;
        let mut add = |items: &mut Vec<Item>, pk: &mut Packer, lo: V3, hi: V3, paint: &dyn Fn(&mut Img, (f32, f32))| {
            let (bx, by, bw, bh) = bounds(cam, &corners(lo, hi), 4);
            let mut im = Img::new(bw, bh);
            paint(&mut im, (bx as f32, by as f32));
            if decayed { dust(&mut im, &mut Rng::of(&format!("d|{}", lo[2]))); }
            im.outline();
            items.push(Item { s: pk.add(im), x: ox + bx as f32, y: oy + by as f32, depth: oy + cam.p([(lo[0] + hi[0]) / 2.0, 0.0, hi[2]]).1 });
        };
        match kit.tool {
            Tool::ClaudeCode => {
                for k in 0..2 {
                    let (x0, z0) = (fx0 + 0.2 + k as f32 * 1.4, -7.2);
                    add(items, pk, [x0, 0.0, z0], [x0 + 1.2, 1.8, z0 + 0.4], &|im, off| {
                        draw_box(im, cam, off, [x0, 0.0, z0], [x0 + 1.2, 1.8, z0 + 0.4], hex("#6b4a32"));
                        let mut br = Rng::of(&format!("books|{x0}"));
                        for sh in 0..4 { let y = 0.1 + sh as f32 * 0.43; let mut bx = x0 + 0.05; while bx < x0 + 1.12 { let bw = br.range(0.04, 0.08); let q = cam.quad([[bx, y, z0 + 0.401], [bx + bw, y, z0 + 0.401], [bx + bw, y + br.range(0.22, 0.34), z0 + 0.401], [bx, y + 0.3, z0 + 0.401]]); im.poly(&q.map(|(a, b)| (a - off.0, b - off.1)), hex(br.pick(&["#a8584a", "#3f5f8a", "#c9a46c", "#4a7a5a", "#d9c7a0", "#6b4a7a"]))); bx += bw + 0.01; } }
                    });
                }
                add(items, pk, [cx - 0.5, 0.0, -3.6], [cx + 0.5, 1.0, -2.6], &|im, off| {
                    draw_box(im, cam, off, [cx - 0.45, 0.0, -3.5], [cx + 0.45, 0.45, -2.7], kit.chair);
                    draw_box(im, cam, off, [cx - 0.45, 0.45, -3.5], [cx + 0.45, 0.95, -3.3], ramp(kit.chair, -0.1));
                    draw_box(im, cam, off, [cx - 0.5, 0.0, -3.5], [cx - 0.38, 0.65, -2.7], ramp(kit.chair, -0.15));
                    draw_box(im, cam, off, [cx + 0.38, 0.0, -3.5], [cx + 0.5, 0.65, -2.7], ramp(kit.chair, -0.15));
                });
            }
            Tool::ClaudeApp => {
                add(items, pk, [cx - 1.2, 0.0, -4.4], [cx + 1.2, 1.0, -2.0], &|im, off| {
                    for (sx, sz) in [(-0.85, 0.0), (0.85, 0.0), (0.0, -0.8), (0.0, 0.8)] { draw_box(im, cam, off, [cx + sx - 0.2, 0.0, -3.2 + sz - 0.2], [cx + sx + 0.2, 0.46, -3.2 + sz + 0.2], kit.chair); }
                    draw_box(im, cam, off, [cx - 0.05, 0.0, -3.25], [cx + 0.05, 0.72, -3.15], hex("#c9c5bb"));
                    let c = cam.p([cx, 0.74, -3.2]);
                    im.ellipse(c.0 - off.0, c.1 - off.1, cam.scale_at([cx, 0.7, -3.2]) * 0.62, cam.scale_at([cx, 0.7, -3.2]) * 0.34, hex("#f2f0ea"));
                    im.ellipse(c.0 - off.0 - 1.0, c.1 - off.1 - 1.0, 3.0, 1.5, hex("#e6a99a"));
                });
            }
            Tool::Codex | Tool::Cursor => {
                add(items, pk, [fx0 + 0.1, 0.0, -7.4], [fx0 + 2.5, 1.9, -6.9], &|im, off| {
                    draw_box(im, cam, off, [fx0 + 0.1, 0.0, -7.4], [fx0 + 2.5, 1.9, -6.9], hex("#5a6370"));
                    for k in 0..6 { let x = fx0 + 0.1 + k as f32 * 0.4; let p0 = cam.p([x, 0.05, -6.899]); let p1 = cam.p([x, 1.85, -6.899]); im.line((p0.0 - off.0, p0.1 - off.1), (p1.0 - off.0, p1.1 - off.1), hex("#3a414c")); let v = cam.p([x + 0.3, 1.6, -6.899]); im.rect((v.0 - off.0) as i32, (v.1 - off.1) as i32, 3, 1, hex("#2a2f38")); }
                });
                add(items, pk, [cx - 0.4, 0.0, -2.6], [cx + 0.4, 1.0, -2.0], &|im, off| {
                    draw_box(im, cam, off, [cx - 0.35, 0.0, -2.5], [cx + 0.35, 0.95, -2.05], hex("#d9dde5"));
                    draw_box(im, cam, off, [cx - 0.3, 0.95, -2.45], [cx + 0.3, 1.02, -2.2], hex("#f4f5f8"));
                    let l = cam.p([cx + 0.2, 0.85, -2.049]);
                    im.rect((l.0 - off.0) as i32, (l.1 - off.1) as i32, 2, 1, hex("#34d399"));
                });
            }
            Tool::OpenCode | Tool::Gemini => {
                add(items, pk, [cx - 1.4, 0.0, -4.0], [cx + 1.4, 1.0, -2.4], &|im, off| {
                    for (lx, lz) in [(cx - 1.2, -3.85), (cx + 1.15, -3.85), (cx - 1.2, -2.6), (cx + 1.15, -2.6)] { draw_box(im, cam, off, [lx, 0.0, lz], [lx + 0.05, 0.74, lz + 0.05], hex("#2a2a30")); }
                    draw_box(im, cam, off, [cx - 1.35, 0.72, -3.95], [cx + 1.35, 0.76, -2.45], hex("#2f6b4a"));
                    let a = cam.p([cx - 1.33, 0.761, -3.2]); let b = cam.p([cx + 1.33, 0.761, -3.2]);
                    im.line((a.0 - off.0, a.1 - off.1), (b.0 - off.0, b.1 - off.1), hex("#f4f1ea"));
                    let n0 = cam.p([cx, 0.76, -3.95]); let n1 = cam.p([cx, 0.76, -2.45]); let nt = cam.p([cx, 0.92, -3.95]);
                    im.line((n0.0 - off.0, n0.1 - off.1), (n1.0 - off.0, n1.1 - off.1), hex("#f4f1ea"));
                    im.line((n0.0 - off.0, nt.1 - off.1), (n1.0 - off.0, nt.1 - off.1 + (n1.1 - n0.1)), alpha(hex("#e8e8e8"), 180));
                    let ball = cam.p([cx + 0.6, 1.1, -3.0]);
                    im.rect((ball.0 - off.0) as i32, (ball.1 - off.1) as i32, 2, 2, hex("#ff9a3d"));
                });
            }
        }
    }
    // two floor plants at the area's edges
    for k in 0..((kit.greenery * 3.0).round() as usize).min(3) {
        let (px, pz) = [(zx0 + 0.4, 0.4), (zx1 - 0.4, -2.0), (zx1 - 0.5, Z1 - 1.0)][k];
        let pts = corners([px - 0.5, 0.0, pz - 0.5], [px + 0.5, 1.8, pz + 0.5]);
        let (bx, by, bw, bh) = bounds(cam, &pts, 6);
        let mut im = Img::new(bw, bh);
        floor_plant(&mut im, cam, (bx as f32, by as f32), px, pz, *r.pick(&kit.plants), 1.5, decayed);
        im.outline();
        items.push(Item { s: pk.add(im), x: ox + bx as f32, y: oy + by as f32, depth: oy + cam.p([px, 0.0, pz + 0.2]).1 });
    }
    // pendant lights over the desks (a warm pool already lights the floor below)
    if kit.decor.contains(&Decor::Pendants) && !decayed {
        for k in 0..3 {
            let (lx, lz) = (zx0 + 1.6 + k as f32 * 3.0, -1.2);
            let top = cam.p([lx, WALL, lz]);
            let bulb = cam.p([lx, 2.45, lz]);
            let mut im = Img::new(14, (bulb.1 - top.1) as i32 + 10);
            im.rect(6, 0, 1, (bulb.1 - top.1) as i32, hex("#2a2a2a"));
            let by = (bulb.1 - top.1) as i32;
            im.rect(3, by, 8, 3, if kit.tool == Tool::Codex { hex("#1c1d21") } else { hex("#f2efe6") });
            im.rect(5, by + 3, 4, 2, hex("#ffd98a"));
            for gy in 0..3 { im.rect(4 - gy, by + 5 + gy, 6 + gy * 2, 1, alpha(hex("#ffd98a"), (60 - gy * 18) as u8)); }
            items.push(Item { s: pk.add(im), x: ox + top.0 - 6.0, y: oy + top.1, depth: oy + cam.p([lx, 0.0, lz]).1 });
        }
    }
    // desks, chairs and people
    let mut seat = 0usize;
    for (di, d) in plan.iter().enumerate() {
        let mut dr = Rng::of(&format!("desk|{}|{di}", app.id));
        let st = if decayed { None } else { app.sessions.get(seat).copied() };
        if !decayed && seat < app.sessions.len() { seat += 1; }
        let lo = [d.x + 0.02, 0.0, d.z];
        let hi = [d.x + d.w - 0.02, 0.74, d.z + d.d];
        let mut pts = corners(lo, hi);
        pts.extend(corners([d.x, 0.74, d.z], [d.x + d.w, 1.2, d.z + d.d]));
        let (bx, by, bw, bh) = bounds(cam, &pts, 3);
        let mut im = Img::new(bw, bh);
        let off = (bx as f32, by as f32);
        // the desk: top, legs or panels by kit
        match kit.desk {
            DeskKind::Steel | DeskKind::Standing => {
                let top_y = if kit.desk == DeskKind::Standing { 0.98 } else { 0.74 };
                for (lx, lz) in [(lo[0] + 0.04, lo[2] + 0.04), (hi[0] - 0.08, lo[2] + 0.04), (lo[0] + 0.04, hi[2] - 0.08), (hi[0] - 0.08, hi[2] - 0.08)] { draw_box(&mut im, cam, off, [lx, 0.0, lz], [lx + 0.04, top_y, lz + 0.04], kit.desk_frame); }
                draw_box(&mut im, cam, off, [lo[0], top_y - 0.04, lo[2]], [hi[0], top_y, hi[2]], kit.desk_top);
            }
            _ => {
                draw_box(&mut im, cam, off, [lo[0], 0.0, lo[2] + 0.05], [lo[0] + 0.04, 0.72, hi[2] - 0.05], kit.desk_frame);
                draw_box(&mut im, cam, off, [hi[0] - 0.04, 0.0, lo[2] + 0.05], [hi[0], 0.72, hi[2] - 0.05], kit.desk_frame);
                if !d.table { draw_box(&mut im, cam, off, [lo[0] + 0.04, 0.3, if d.faces_cam { hi[2] - 0.04 } else { lo[2] }], [hi[0] - 0.04, 0.72, if d.faces_cam { hi[2] } else { lo[2] + 0.04 }], ramp(kit.desk_frame, 0.1)); }
                draw_box(&mut im, cam, off, [lo[0], 0.72, lo[2]], [hi[0], 0.76, hi[2]], kit.desk_top);
            }
        }
        let ty = if kit.desk == DeskKind::Standing { 0.98 } else { 0.76 };
        // wear on the top: a scuff and a ring, fixed per desk
        let wear = cam.p([d.x + dr.range(0.2, d.w - 0.2), ty, d.z + dr.range(0.15, d.d - 0.15)]);
        im.blend((wear.0 - off.0) as i32, (wear.1 - off.1) as i32, alpha(ramp(kit.desk_top, -0.4), 120));
        // screens: facing the person, so a person facing us shows us the back of their monitor
        let mz = if d.faces_cam { d.z + 0.12 } else { d.z + d.d - 0.18 };
        let on = st.is_some() && !decayed;
        let screen = match st { Some(St::Asks) => hex("#ffd36b"), Some(St::Done) => hex("#5ef0a8"), Some(St::Stuck) => hex("#f6c25a"), Some(St::Limit) => hex("#7a5a3a"), Some(St::Idle) => hex("#3a4560"), Some(_) => hex("#7cc8ff"), None => hex("#15161c") };
        let cx = d.x + d.w / 2.0;
        let mons: Vec<(f32, f32)> = match kit.screens { Screens::Single => vec![(cx - 0.27, 0.54)], Screens::Dual => vec![(cx - 0.5, 0.48), (cx + 0.02, 0.48)], Screens::Laptop => vec![] };
        for (mx, mw) in &mons {
            draw_box(&mut im, cam, off, [mx + mw / 2.0 - 0.03, ty, mz], [mx + mw / 2.0 + 0.03, ty + 0.12, mz + 0.04], hex("#2a2a30"));
            draw_box(&mut im, cam, off, [*mx, ty + 0.1, mz], [mx + mw, ty + 0.42, mz + 0.035], hex("#1b1b22"));
            if !d.faces_cam {
                let q = cam.quad([[mx + 0.03, ty + 0.13, mz + 0.036], [mx + mw - 0.03, ty + 0.13, mz + 0.036], [mx + mw - 0.03, ty + 0.39, mz + 0.036], [mx + 0.03, ty + 0.39, mz + 0.036]]);
                im.poly(&q.map(|(x, y)| (x - off.0, y - off.1)), if on { screen } else { hex("#15161c") });
                if on && matches!(st, Some(St::Typing)) { for l in 0..3 { let p = cam.p([mx + 0.06, ty + 0.33 - l as f32 * 0.07, mz + 0.037]); im.rect((p.0 - off.0) as i32, (p.1 - off.1) as i32, 3 + (l * 2) % 5, 1, hex("#dff4ff")); } }
            }
        }
        if kit.screens == Screens::Laptop {
            let lz = if d.faces_cam { d.z + 0.15 } else { d.z + d.d - 0.4 };
            draw_box(&mut im, cam, off, [cx - 0.18, ty, lz], [cx + 0.18, ty + 0.02, lz + 0.25], hex("#c9cdd6"));
            let (sz0, sz1) = if d.faces_cam { (lz, lz + 0.02) } else { (lz + 0.23, lz + 0.25) };
            draw_box(&mut im, cam, off, [cx - 0.18, ty, sz0], [cx + 0.18, ty + 0.24, sz1], hex("#b8bcc6"));
            if !d.faces_cam && on { let q = cam.quad([[cx - 0.16, ty + 0.03, sz1 + 0.001], [cx + 0.16, ty + 0.03, sz1 + 0.001], [cx + 0.16, ty + 0.22, sz1 + 0.001], [cx - 0.16, ty + 0.22, sz1 + 0.001]]); im.poly(&q.map(|(x, y)| (x - off.0, y - off.1)), screen); }
        }
        // personal things, fixed per desk and favoured by the kit
        let props: &[&str] = match kit.tool { Tool::ClaudeCode => &["plant", "lamp", "books", "mug", "photo"], Tool::ClaudeApp => &["ceramic", "notebook", "mug", "plant"], Tool::Codex => &["can", "mug", "cable", "books"], Tool::OpenCode => &["board", "cactus", "tools", "mug"], Tool::Gemini => &["toy", "bottle", "mug", "plant"], Tool::Cursor => &["mug", "lamp"] };
        for k in 0..(1 + dr.int(3)) {
            let what = *dr.pick(props);
            let side = if k % 2 == 0 { d.x + 0.12 } else { d.x + d.w - 0.22 };
            let p = cam.p([side, ty, d.z + dr.range(0.2, d.d - 0.15)]);
            let (px, py) = ((p.0 - off.0) as i32, (p.1 - off.1) as i32);
            match what {
                "plant" | "cactus" => { im.rect(px, py - 3, 3, 3, hex("#b4643a")); im.rect(px - 1, py - 7, 5, 4, if what == "cactus" { hex("#5a9a5a") } else { hex("#3f9b55") }); im.rect(px, py - 8, 2, 2, hex("#6cc77c")); }
                "lamp" => { im.rect(px + 1, py - 8, 1, 8, hex("#2a2a2a")); im.rect(px - 1, py - 10, 5, 3, hex("#e8b04a")); if on { im.blend(px + 1, py - 7, hex("#fff2c0")); } }
                "books" | "notebook" => { im.rect(px - 1, py - 3, 6, 2, hex(dr.pick(&["#a8584a", "#3f5f8a", "#4a7a5a"]))); im.rect(px, py - 5, 5, 2, hex(dr.pick(&["#d9c7a0", "#6b4a7a", "#c86f4f"]))); }
                "mug" | "ceramic" => { im.rect(px, py - 4, 3, 4, if what == "ceramic" { hex("#9cc7b1") } else { hex("#f4f1ea") }); im.rect(px + 3, py - 3, 1, 2, hex("#d9d4ca")); }
                "photo" => { im.rect(px, py - 5, 4, 5, hex("#5a4030")); im.rect(px + 1, py - 4, 2, 3, hex("#a9d0e8")); }
                "can" | "bottle" => { im.rect(px, py - 5, 2, 5, hex(dr.pick(&["#3f7fbf", "#d9644a", "#4aa37a", "#e8b33a"]))); im.blend(px, py - 5, hex("#ffffff")); }
                "cable" => { im.line(((px - 3) as f32, py as f32), ((px + 4) as f32, (py - 2) as f32), hex("#25252b")); }
                "board" => { im.rect(px - 1, py - 2, 6, 3, hex("#2f6b3a")); im.blend(px + 1, py - 2, hex("#e8c070")); }
                "tools" => { im.rect(px, py - 2, 6, 1, hex("#d9644a")); im.rect(px + 1, py - 4, 1, 2, hex("#8d94a3")); }
                "toy" => { im.rect(px, py - 4, 3, 4, hex(dr.pick(&["#e8b33a", "#d9644a", "#3f7fbf"]))); im.rect(px, py - 5, 1, 1, hex("#1d1726")); }
                _ => {}
            }
        }
        if decayed { dust(&mut im, &mut dr); let p = cam.p([d.x + 0.3, ty, d.z + 0.3]); im.rect((p.0 - off.0) as i32, (p.1 - off.1) as i32 - 2, 6, 3, hex("#efe8d8")); }
        im.outline();
        let desk_depth = oy + cam.p([cx, 0.0, d.z + d.d]).1;
        items.push(Item { s: pk.add(im), x: ox + off.0, y: oy + off.1, depth: desk_depth });
        // the chair, and the person on it
        let cz = if d.faces_cam { d.z - 0.55 } else { d.z + d.d + 0.5 };
        let askew = if decayed { dr.range(-0.4, 0.5) } else { 0.0 };
        let chair_img = chair(cam, kit, cx + askew, cz, d.faces_cam);
        let foot = cam.p([cx, 0.0, cz]);
        let person_depth = oy + foot.1;
        let chair_depth = if d.faces_cam { person_depth - 0.5 } else { person_depth + 0.5 };
        items.push(Item { s: pk.add(chair_img.0), x: ox + chair_img.1, y: oy + chair_img.2, depth: chair_depth });
        let Some(st) = st else { continue };
        let lk = Look::random(&mut dr, kit.accent);
        let (fx, fy) = (ox + foot.0 - people::FOOT_X as f32, oy + foot.1 - people::FOOT_Y_SIT as f32 + 2.0);
        let face = d.faces_cam;
        let mut add = |pose: Pose| pk.add(people::paint(&lk, pose));
        match st {
            St::Typing => {
                let (a, b) = if face { (add(Pose::SitFront(0)), add(Pose::SitFront(1))) } else { (add(Pose::SitBack(0)), add(Pose::SitBack(1))) };
                actors.push(Actor { frames: vec![a, b], rate: 3.5, phase: dr.range(0.0, 2.0), x: fx, y: fy, depth: person_depth });
            }
            St::Idle => { let s = add(if face { Pose::SitFront(0) } else { Pose::SitBack(0) }); items.push(Item { s, x: fx, y: fy, depth: person_depth }); }
            St::Asks => {
                let (a, b) = (add(Pose::Wave(0)), add(Pose::Wave(1)));
                let stand = cam.p([cx - 0.75, 0.0, cz + if face { -0.1 } else { 0.25 }]);
                let (sx, sy) = (ox + stand.0 - people::FOOT_X as f32, oy + stand.1 - people::FOOT_Y_STAND as f32);
                actors.push(Actor { frames: vec![a, b], rate: 3.0, phase: 0.0, x: sx, y: sy, depth: oy + stand.1 + 0.3 });
                let bub = bubble('!', hex("#e8a321"));
                items.push(Item { s: pk.add(bub), x: sx - 1.0, y: sy - 15.0, depth: 1.0e6 });
            }
            St::Stuck => { let s = add(Pose::Scratch); items.push(Item { s, x: fx, y: fy, depth: person_depth }); items.push(Item { s: pk.add(bubble('?', hex("#e8a321"))), x: fx, y: fy - 10.0, depth: 1.0e6 }); }
            St::Limit => { let s = add(Pose::Sleep); items.push(Item { s, x: fx, y: fy, depth: person_depth }); let z = pk.add(bubble('Z', hex("#8f7fe0"))); let z2 = pk.add(bubble_raised('Z', hex("#8f7fe0"))); actors.push(Actor { frames: vec![z, z2], rate: 1.2, phase: 0.0, x: fx, y: fy - 8.0, depth: 1.0e6 }); }
            St::Done => { let s = add(Pose::Lean); items.push(Item { s, x: fx, y: fy, depth: person_depth }); }
        }
        hits.push(Hit { x: fx, y: fy, w: people::PW as f32, h: people::PH as f32, target: Target::Session(ai, di), z: 1.0 });
        if app.overloaded && di == 2 {
            let fp = cam.p([d.x + d.w - 0.25, ty, d.z + 0.3]);
            let fr: Vec<usize> = (0..3).map(|f| pk.add(flame(f))).collect();
            actors.push(Actor { frames: fr, rate: 8.0, phase: 0.0, x: ox + fp.0 - 5.0, y: oy + fp.1 - 13.0, depth: desk_depth + 0.2 });
        }
    }
    // clutter for an area left unused: boxes and bags by the desks
    if decayed {
        for k in 0..6 {
            let (bx, bz) = (zx0 + r.range(0.5, zx1 - zx0 - 0.5), r.range(-6.0, Z1 - 2.0));
            let pts = corners([bx - 0.4, 0.0, bz - 0.4], [bx + 0.4, 0.6, bz + 0.4]);
            let (x0, y0, w, h) = bounds(cam, &pts, 2);
            let mut im = Img::new(w, h);
            if k % 2 == 0 { draw_box(&mut im, cam, (x0 as f32, y0 as f32), [bx - 0.3, 0.0, bz - 0.25], [bx + 0.3, 0.4, bz + 0.25], hex("#b98b57")); }
            else { let p = cam.p([bx, 0.2, bz]); im.ellipse(p.0 - x0 as f32, p.1 - y0 as f32, 7.0, 5.0, hex("#2c2f36")); im.ellipse(p.0 - x0 as f32 - 2.0, p.1 - y0 as f32 - 2.0, 3.0, 2.0, hex("#4a505c")); }
            im.outline();
            items.push(Item { s: pk.add(im), x: ox + x0 as f32, y: oy + y0 as f32, depth: oy + cam.p([bx, 0.0, bz + 0.3]).1 });
        }
    }
}

fn chair(cam: &Cam, kit: &Kit, cx: f32, cz: f32, behind_desk: bool) -> (Img, f32, f32) {
    let pts = corners([cx - 0.3, 0.0, cz - 0.3], [cx + 0.3, 1.0, cz + 0.3]);
    let (bx, by, bw, bh) = bounds(cam, &pts, 2);
    let mut im = Img::new(bw, bh);
    let off = (bx as f32, by as f32);
    match kit.chair_kind {
        ChairKind::Stool => {
            draw_box(&mut im, cam, off, [cx - 0.03, 0.0, cz - 0.03], [cx + 0.03, 0.62, cz + 0.03], hex("#3a3a3e"));
            draw_box(&mut im, cam, off, [cx - 0.2, 0.62, cz - 0.2], [cx + 0.2, 0.68, cz + 0.2], kit.chair);
        }
        _ => {
            draw_box(&mut im, cam, off, [cx - 0.22, 0.0, cz - 0.22], [cx + 0.22, 0.03, cz + 0.22], hex("#25252b"));
            draw_box(&mut im, cam, off, [cx - 0.03, 0.0, cz - 0.03], [cx + 0.03, 0.44, cz + 0.03], hex("#3a3a3e"));
            draw_box(&mut im, cam, off, [cx - 0.24, 0.44, cz - 0.22], [cx + 0.24, 0.5, cz + 0.22], kit.chair);
            let (b0, b1) = if behind_desk { (cz - 0.24, cz - 0.18) } else { (cz + 0.18, cz + 0.24) };
            let back = if kit.chair_kind == ChairKind::Mesh { ramp(kit.chair, 0.15) } else { kit.chair };
            draw_box(&mut im, cam, off, [cx - 0.22, 0.5, b0], [cx + 0.22, 0.98, b1], back);
        }
    }
    im.outline();
    (im, off.0, off.1)
}

fn dust(im: &mut Img, r: &mut Rng) {
    for _ in 0..(im.w * im.h / 30).max(4) {
        let (x, y) = (r.int(im.w as usize) as i32, r.int(im.h as usize) as i32);
        if im.get(x, y)[3] > 0 { im.blend(x, y, alpha(hex("#ddd6c6"), 150)); }
    }
}

fn bubble(c: char, col: Rgba) -> Img {
    let mut im = Img::new(15, 15);
    im.rect(2, 1, 11, 10, col);
    im.rect(3, 2, 9, 8, hex("#fffaf0"));
    im.rect(6, 11, 3, 2, col);
    im.rect(7, 11, 1, 1, hex("#fffaf0"));
    let s = c.to_string();
    let w = text_width(&s, 1);
    draw_text(&mut im, 7 - w / 2 + if c == '!' { 0 } else { 0 }, 3, &s, if c == 'Z' { hex("#5a4ab0") } else { hex("#7a4a00") }, 1);
    im
}
fn bubble_raised(c: char, col: Rgba) -> Img {
    let b = bubble(c, col);
    let mut im = Img::new(15, 15);
    im.blit(&b, 0, -1);
    im
}
fn flame(f: i32) -> Img {
    let mut im = Img::new(12, 15);
    let o = [0, 1, -1][f as usize];
    im.rect(2, 5 + o, 7, 9, hex("#e5452f"));
    im.rect(3, 3, 5, 3, hex("#e5452f"));
    im.rect(4 + o, 1, 2, 3, hex("#e5452f"));
    im.rect(3, 7, 5, 6, hex("#ff9a3d"));
    im.rect(4, 5 + o, 3, 3, hex("#ff9a3d"));
    im.rect(4, 9, 3, 4, hex("#ffe08a"));
    im
}

/// The shared corner of a floor: a kitchen along the wall, tables, a lounge and plants.
#[allow(clippy::too_many_arguments)]
fn commons(pk: &mut Packer, items: &mut Vec<Item>, actors: &mut Vec<Actor>, img: &mut Img, cam: &Cam, (ox, oy): (f32, f32), zx0: f32, zx1: f32, floor: usize) {
    let mut r = Rng::of(&format!("commons|{floor}"));
    // kitchen run along the back wall
    draw_box(img, cam, (0.0, 0.0), [zx0 + 0.4, 0.0, Z0], [zx1 - 1.4, 0.92, Z0 + 0.65], hex("#e9e2d3"));
    draw_box(img, cam, (0.0, 0.0), [zx0 + 0.4, 0.92, Z0], [zx1 - 1.4, 0.97, Z0 + 0.7], hex("#9a8f7c"));
    draw_box(img, cam, (0.0, 0.0), [zx0 + 0.4, 1.7, Z0], [zx1 - 1.4, 2.3, Z0 + 0.35], hex("#e9e2d3"));
    draw_box(img, cam, (0.0, 0.0), [zx1 - 1.3, 0.0, Z0], [zx1 - 0.4, 2.0, Z0 + 0.7], hex("#dfe4ea"));
    draw_box(img, cam, (0.0, 0.0), [zx0 + 1.0, 0.97, Z0 + 0.1], [zx0 + 1.4, 1.4, Z0 + 0.45], hex("#2a2a30"));
    let steam = cam.p([zx0 + 1.2, 1.45, Z0 + 0.3]);
    let st0 = { let mut i = Img::new(5, 8); i.rect(2, 0, 1, 2, alpha(hex("#ffffff"), 170)); i.rect(1, 3, 1, 2, alpha(hex("#ffffff"), 140)); i.rect(2, 6, 1, 2, alpha(hex("#ffffff"), 110)); i };
    let st1 = { let mut i = Img::new(5, 8); i.rect(1, 0, 1, 2, alpha(hex("#ffffff"), 170)); i.rect(2, 3, 1, 2, alpha(hex("#ffffff"), 140)); i.rect(1, 6, 1, 2, alpha(hex("#ffffff"), 110)); i };
    actors.push(Actor { frames: vec![pk.add(st0), pk.add(st1)], rate: 2.0, phase: 0.0, x: ox + steam.0 - 2.0, y: oy + steam.1 - 8.0, depth: 1.0e6 });
    // round tables with chairs, a lounge on a rug
    for k in 0..3 {
        let (tx, tz) = (zx0 + 1.8 + (k % 2) as f32 * 3.6, -4.5 + k as f32 * 3.6);
        let pts = corners([tx - 1.2, 0.0, tz - 1.2], [tx + 1.2, 1.0, tz + 1.2]);
        let (bx, by, bw, bh) = bounds(cam, &pts, 2);
        let mut im = Img::new(bw, bh);
        let off = (bx as f32, by as f32);
        for (sx, sz) in [(-0.8, 0.0), (0.8, 0.0), (0.0, -0.8), (0.0, 0.8)] { draw_box(&mut im, cam, off, [tx + sx - 0.2, 0.0, tz + sz - 0.2], [tx + sx + 0.2, 0.45, tz + sz + 0.2], hex(r.pick(&["#c86f4f", "#4a7a8a", "#e8b33a"]))); }
        draw_box(&mut im, cam, off, [tx - 0.05, 0.0, tz - 0.05], [tx + 0.05, 0.72, tz + 0.05], hex("#3a3a3e"));
        let c = cam.p([tx, 0.74, tz]);
        im.ellipse(c.0 - off.0, c.1 - off.1, cam.scale_at([tx, 0.7, tz]) * 0.6, cam.scale_at([tx, 0.7, tz]) * 0.32, hex("#d4a873"));
        im.outline();
        items.push(Item { s: pk.add(im), x: ox + off.0, y: oy + off.1, depth: oy + cam.p([tx, 0.0, tz + 0.8]).1 });
    }
    let (lx, lz) = (zx1 - 2.6, 8.0);
    let pts = corners([lx - 1.6, 0.0, lz - 1.4], [lx + 1.6, 1.0, lz + 1.2]);
    let (bx, by, bw, bh) = bounds(cam, &pts, 2);
    let mut im = Img::new(bw, bh);
    let off = (bx as f32, by as f32);
    draw_box(&mut im, cam, off, [lx - 1.4, 0.0, lz - 1.2], [lx + 1.4, 0.45, lz - 0.5], hex("#4f6a8f"));
    draw_box(&mut im, cam, off, [lx - 1.4, 0.45, lz - 1.2], [lx + 1.4, 0.85, lz - 1.0], hex("#435c80"));
    draw_box(&mut im, cam, off, [lx - 0.5, 0.0, lz - 0.1], [lx + 0.5, 0.35, lz + 0.5], hex("#c99764"));
    im.outline();
    items.push(Item { s: pk.add(im), x: ox + off.0, y: oy + off.1, depth: oy + cam.p([lx, 0.0, lz + 0.6]).1 });
    for &(px, pz) in &[(zx1 - 0.6, -2.0), (zx0 + 0.6, Z1 - 1.2), (zx1 - 0.6, Z1 - 1.0)] {
        let pts = corners([px - 0.5, 0.0, pz - 0.5], [px + 0.5, 1.8, pz + 0.5]);
        let (bx, by, bw, bh) = bounds(cam, &pts, 6);
        let mut im = Img::new(bw, bh);
        floor_plant(&mut im, cam, (bx as f32, by as f32), px, pz, *r.pick(&[Plant::Monstera, Plant::Palm, Plant::Fern]), 1.6, false);
        im.outline();
        items.push(Item { s: pk.add(im), x: ox + bx as f32, y: oy + by as f32, depth: oy + cam.p([px, 0.0, pz + 0.2]).1 });
    }
    let sign = cam.p([(zx0 + zx1) / 2.0, 3.0, Z0 + 0.03]);
    let t = "KITCHEN · LOUNGE";
    let tw = text_width(t, 1);
    img.rect(sign.0 as i32 - tw / 2 - 6, sign.1 as i32 - 8, tw + 12, 15, hex("#2a2d36"));
    draw_text(img, sign.0 as i32 - tw / 2, sign.1 as i32 - 4, t, hex("#f0e8da"), 1);
}
