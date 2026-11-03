//! The one scene both spikes draw: an office seen from a fixed camera, high above and looking down at
//! 40° with perspective. The room (floor, walls, windows, doors, glass, plants) is projected once with
//! our own maths and baked into a low-res background. Desks, chairs, people, the bubble and the fire are
//! procedurally drawn sprites packed into one atlas. Each frame is the background plus a depth-sorted
//! list of sprite instances, which both renderers draw the same way.

use serde::Serialize;

/// The art resolution. Shown at 3 device pixels per art pixel on a 2x screen (1.5 points).
pub const ART_W: i32 = 852;
pub const ART_H: i32 = 532;

#[derive(Clone)]
pub struct Img {
    pub w: i32,
    pub h: i32,
    pub px: Vec<u8>,
}

impl Img {
    pub fn new(w: i32, h: i32) -> Self {
        Img { w, h, px: vec![0; (w * h * 4) as usize] }
    }
    fn blend(&mut self, x: i32, y: i32, c: [u8; 4]) {
        if x < 0 || y < 0 || x >= self.w || y >= self.h || c[3] == 0 {
            return;
        }
        let i = ((y * self.w + x) * 4) as usize;
        let a = c[3] as u32;
        if a == 255 {
            self.px[i..i + 4].copy_from_slice(&c);
            return;
        }
        for k in 0..3 {
            self.px[i + k] = ((c[k] as u32 * a + self.px[i + k] as u32 * (255 - a)) / 255) as u8;
        }
        self.px[i + 3] = (a + self.px[i + 3] as u32 * (255 - a) / 255).min(255) as u8;
    }
    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: [u8; 4]) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.blend(xx, yy, c);
            }
        }
    }
    /// Scanline fill of a simple polygon (pixel centres inside are filled).
    fn poly(&mut self, pts: &[(f32, f32)], c: [u8; 4]) {
        let miny = pts.iter().map(|p| p.1).fold(f32::MAX, f32::min);
        let maxy = pts.iter().map(|p| p.1).fold(f32::MIN, f32::max);
        let y0 = (miny.floor() as i32).max(0);
        let y1 = (maxy.ceil() as i32).min(self.h - 1);
        let mut xs: Vec<f32> = Vec::with_capacity(8);
        for y in y0..=y1 {
            let sy = y as f32 + 0.5;
            xs.clear();
            for i in 0..pts.len() {
                let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
                if (a.1 <= sy && b.1 > sy) || (b.1 <= sy && a.1 > sy) {
                    xs.push(a.0 + (sy - a.1) / (b.1 - a.1) * (b.0 - a.0));
                }
            }
            xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            for pair in xs.chunks(2) {
                if pair.len() < 2 {
                    break;
                }
                let xa = (pair[0] - 0.5).ceil() as i32;
                let xb = (pair[1] - 0.5).ceil() as i32;
                for x in xa.max(0)..xb.min(self.w) {
                    self.blend(x, y, c);
                }
            }
        }
    }
    fn disc(&mut self, cx: f32, cy: f32, r: f32, c: [u8; 4]) {
        for y in (cy - r).floor() as i32..=(cy + r).ceil() as i32 {
            for x in (cx - r).floor() as i32..=(cx + r).ceil() as i32 {
                let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
                if dx * dx + dy * dy <= r * r {
                    self.blend(x, y, c);
                }
            }
        }
    }
    /// A dark one-pixel outline around every opaque pixel.
    fn outline(&mut self, c: [u8; 4]) {
        let solid: Vec<bool> = (0..self.w * self.h).map(|i| self.px[(i * 4 + 3) as usize] > 0).collect();
        for y in 0..self.h {
            for x in 0..self.w {
                let i = y * self.w + x;
                if solid[i as usize] {
                    continue;
                }
                let n = (x > 0 && solid[(i - 1) as usize])
                    || (x < self.w - 1 && solid[(i + 1) as usize])
                    || (y > 0 && solid[(i - self.w) as usize])
                    || (y < self.h - 1 && solid[(i + self.w) as usize]);
                if n {
                    let k = (i * 4) as usize;
                    self.px[k..k + 4].copy_from_slice(&c);
                }
            }
        }
    }
}

// ---------- small vector maths and the fixed camera ----------
type V3 = [f32; 3];
fn sub(a: V3, b: V3) -> V3 { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
fn dot(a: V3, b: V3) -> f32 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
fn cross(a: V3, b: V3) -> V3 { [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]] }
fn norm(a: V3) -> V3 { let l = dot(a, a).sqrt(); [a[0] / l, a[1] / l, a[2] / l] }

pub struct Cam { eye: V3, r: V3, u: V3, f: V3, focal: f32 }
impl Cam {
    fn new() -> Cam {
        let target: V3 = [-2.0, 0.0, 0.6];
        let (el, dist, fov) = (40f32.to_radians(), 45.0f32, 30f32.to_radians());
        let eye = [target[0], dist * el.sin(), target[2] + dist * el.cos()];
        let f = norm(sub(target, eye));
        let r = norm(cross(f, [0.0, 1.0, 0.0]));
        let u = cross(r, f);
        Cam { eye, r, u, f, focal: (ART_H as f32 / 2.0) / (fov / 2.0).tan() }
    }
    fn p(&self, w: V3) -> (f32, f32) {
        let d = sub(w, self.eye);
        let (x, y, z) = (dot(d, self.r), dot(d, self.u), dot(d, self.f));
        (ART_W as f32 / 2.0 + x / z * self.focal, ART_H as f32 / 2.0 - y / z * self.focal)
    }
    fn quad(&self, a: V3, b: V3, c: V3, d: V3) -> [(f32, f32); 4] { [self.p(a), self.p(b), self.p(c), self.p(d)] }
}

// ---------- colour ----------
fn hex(s: &str) -> [u8; 4] {
    let n = u32::from_str_radix(&s[1..], 16).unwrap();
    [(n >> 16) as u8, (n >> 8) as u8, n as u8, 255]
}
fn shade(c: [u8; 4], f: f32) -> [u8; 4] {
    [(c[0] as f32 * f).min(255.0) as u8, (c[1] as f32 * f).min(255.0) as u8, (c[2] as f32 * f).min(255.0) as u8, c[3]]
}
fn hsl(h: f32, s: f32, l: f32) -> [u8; 4] {
    let (s, l) = (s / 100.0, l / 100.0);
    let a = s * l.min(1.0 - l);
    let f = |n: f32| {
        let k = (n + h / 30.0) % 12.0;
        l - a * (k - 3.0).min(9.0 - k).min(1.0).max(-1.0)
    };
    [(f(0.0) * 255.0) as u8, (f(8.0) * 255.0) as u8, (f(4.0) * 255.0) as u8, 255]
}
fn alpha(c: [u8; 4], a: u8) -> [u8; 4] { [c[0], c[1], c[2], a] }

/// Seeded randomness, the same everywhere.
struct Rng(u32);
impl Rng {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x6d2b79f5);
        let mut t = self.0;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        ((t ^ (t >> 14)) as f32) / 4294967296.0
    }
    fn pick<'a, T>(&mut self, a: &'a [T]) -> &'a T { &a[(self.next() * a.len() as f32) as usize % a.len()] }
}

// ---------- boxes: draw the faces of an axis-aligned box that face the camera ----------
fn draw_box(img: &mut Img, cam: &Cam, off: (f32, f32), lo: V3, hi: V3, base: [u8; 4]) {
    let o = |q: [(f32, f32); 4]| q.map(|(x, y)| (x - off.0, y - off.1));
    let faces: [(V3, V3, [V3; 4], f32); 5] = [
        ([0.0, 1.0, 0.0], [lo[0], hi[1], lo[2]], [[lo[0], hi[1], lo[2]], [hi[0], hi[1], lo[2]], [hi[0], hi[1], hi[2]], [lo[0], hi[1], hi[2]]], 1.0),
        ([0.0, 0.0, 1.0], [lo[0], lo[1], hi[2]], [[lo[0], lo[1], hi[2]], [hi[0], lo[1], hi[2]], [hi[0], hi[1], hi[2]], [lo[0], hi[1], hi[2]]], 0.74),
        ([0.0, 0.0, -1.0], [lo[0], lo[1], lo[2]], [[lo[0], lo[1], lo[2]], [hi[0], lo[1], lo[2]], [hi[0], hi[1], lo[2]], [lo[0], hi[1], lo[2]]], 0.55),
        ([1.0, 0.0, 0.0], [hi[0], lo[1], lo[2]], [[hi[0], lo[1], lo[2]], [hi[0], lo[1], hi[2]], [hi[0], hi[1], hi[2]], [hi[0], hi[1], lo[2]]], 0.6),
        ([-1.0, 0.0, 0.0], [lo[0], lo[1], lo[2]], [[lo[0], lo[1], lo[2]], [lo[0], lo[1], hi[2]], [lo[0], hi[1], hi[2]], [lo[0], hi[1], lo[2]]], 0.6),
    ];
    for (n, pt, q, f) in faces {
        if dot(n, sub(cam.eye, pt)) > 0.0 {
            img.poly(&o(cam.quad(q[0], q[1], q[2], q[3])), shade(base, f));
        }
    }
}

/// Bounding box in art pixels of a set of world points.
fn bounds(cam: &Cam, pts: &[V3]) -> (i32, i32, i32, i32) {
    let ps: Vec<(f32, f32)> = pts.iter().map(|p| cam.p(*p)).collect();
    let x0 = ps.iter().map(|p| p.0).fold(f32::MAX, f32::min).floor() as i32 - 2;
    let y0 = ps.iter().map(|p| p.1).fold(f32::MAX, f32::min).floor() as i32 - 2;
    let x1 = ps.iter().map(|p| p.0).fold(f32::MIN, f32::max).ceil() as i32 + 2;
    let y1 = ps.iter().map(|p| p.1).fold(f32::MIN, f32::max).ceil() as i32 + 2;
    (x0, y0, x1 - x0, y1 - y0)
}
fn corners(lo: V3, hi: V3) -> Vec<V3> {
    let mut v = Vec::new();
    for &x in &[lo[0], hi[0]] { for &y in &[lo[1], hi[1]] { for &z in &[lo[2], hi[2]] { v.push([x, y, z]); } } }
    v
}

// ---------- people, painted in art pixels (10 wide, 33 tall standing) ----------
#[derive(Clone, Copy)]
struct Look { skin: [u8; 4], hair: [u8; 4], shirt: [u8; 4], pants: [u8; 4], shoes: [u8; 4], long: bool }
#[derive(Clone, Copy, PartialEq)]
enum Pose { SitFront(u8), SitBack(u8), Wave(u8), WalkDown(u8), WalkUp(u8), WalkSide(u8, bool) }

fn paint_person(lk: &Look, pose: Pose) -> Img {
    let mut im = Img::new(14, 37);
    let (ox, oy) = (2, 2);
    let mut r = |x: i32, y: i32, w: i32, h: i32, c: [u8; 4]| im.rect(ox + x, oy + y, w, h, c);
    let shirt_d = shade(lk.shirt, 0.78);
    let pants_d = shade(lk.pants, 0.82);
    let eye = hex("#1d1726");
    let sitting = matches!(pose, Pose::SitFront(_) | Pose::SitBack(_));
    let front = matches!(pose, Pose::SitFront(_) | Pose::Wave(_) | Pose::WalkDown(_));
    let side = matches!(pose, Pose::WalkSide(..));
    let dy = if sitting { 6 } else { 0 }; // a seated person is shorter
    // legs
    if sitting {
        r(2, 20 + dy - 6, 6, 3, lk.pants);
        r(2, 23 + dy - 6, 2, 5, pants_d); r(6, 23 + dy - 6, 2, 5, pants_d);
        r(1, 28 + dy - 6, 3, 1, lk.shoes); r(6, 28 + dy - 6, 3, 1, lk.shoes);
    } else if side {
        let Pose::WalkSide(f, _) = pose else { unreachable!() };
        if f == 0 { r(3, 20, 2, 11, lk.pants); r(6, 20, 2, 11, pants_d); r(2, 31, 3, 1, lk.shoes); r(6, 31, 3, 1, lk.shoes); }
        else { r(4, 20, 3, 11, lk.pants); r(4, 31, 4, 1, lk.shoes); }
    } else {
        let step = match pose { Pose::WalkDown(f) | Pose::WalkUp(f) => f as i32 + 1, _ => 0 };
        r(2, 20, 3, if step == 1 { 10 } else { 11 }, lk.pants);
        r(5, 20, 3, if step == 2 { 10 } else { 11 }, pants_d);
        r(2, if step == 1 { 30 } else { 31 }, 3, 1, lk.shoes); r(5, if step == 2 { 30 } else { 31 }, 3, 1, lk.shoes);
    }
    // torso and arms
    let ty = 9 + dy;
    if side { r(3, ty, 5, 11, lk.shirt); r(3, ty, 1, 11, shirt_d); }
    else {
        r(1, ty, 8, 11, lk.shirt); r(7, ty, 2, 11, shirt_d);
        if front { r(4, ty, 2, 1, lk.skin); r(3, ty, 1, 1, hex("#f4f1ea")); r(6, ty, 1, 1, hex("#f4f1ea")); }
    }
    match pose {
        Pose::SitFront(f) | Pose::SitBack(f) => {
            let (a, b) = if f == 0 { (0, 1) } else { (1, 0) };
            r(0, ty + 1 + a, 1, 6, shirt_d); r(9, ty + 1 + b, 1, 6, shirt_d);
            r(0, ty + 7 + a, 1, 1, lk.skin); r(9, ty + 7 + b, 1, 1, lk.skin);
        }
        Pose::Wave(f) => {
            r(0, ty + 1, 1, 9, shirt_d); r(0, ty + 10, 1, 1, lk.skin);
            r(9, ty - 6, 1, 7, shirt_d);
            if f == 0 { r(9, ty - 9, 2, 3, lk.skin); } else { r(8, ty - 10, 2, 3, lk.skin); }
        }
        Pose::WalkSide(f, _) => { let x = if f == 0 { 4 } else { 6 }; r(x, ty + 1, 1, 8, shirt_d); r(x, ty + 9, 1, 1, lk.skin); }
        Pose::WalkDown(f) | Pose::WalkUp(f) => {
            let s = f as i32;
            r(0, ty + 1 + s, 1, 9, shirt_d); r(9, ty + 2 - s, 1, 9, shirt_d);
            r(0, ty + 10 + s, 1, 1, lk.skin); r(9, ty + 11 - s, 1, 1, lk.skin);
        }
    }
    // head
    let hy = dy;
    if side {
        r(3, hy + 1, 5, 7, lk.skin); r(2, hy, 6, 3, lk.hair); r(2, hy + 3, 2, if lk.long { 6 } else { 3 }, lk.hair); r(7, hy + 4, 1, 1, eye);
    } else if front {
        r(2, hy + 1, 6, 7, lk.skin); r(2, hy, 6, 3, lk.hair); r(2, hy + 3, 1, if lk.long { 7 } else { 2 }, lk.hair); r(7, hy + 3, 1, if lk.long { 7 } else { 2 }, lk.hair);
        r(3, hy + 4, 1, 1, eye); r(6, hy + 4, 1, 1, eye);
        if matches!(pose, Pose::Wave(_)) { r(4, hy + 6, 2, 1, hex("#7a2e35")); }
    } else {
        r(2, hy + 1, 6, 7, lk.hair); r(2, hy, 6, 2, lk.hair); r(4, hy + 8, 2, 1, lk.skin);
        if lk.long { r(1, hy + 2, 8, 8, lk.hair); }
        r(3, hy + 1, 1, 1, shade(lk.hair, 1.3));
    }
    if let Pose::WalkSide(_, true) = pose {
        // mirror for walking left
        let mut m = Img::new(im.w, im.h);
        for y in 0..im.h { for x in 0..im.w { let s = ((y * im.w + x) * 4) as usize; let d = ((y * im.w + (im.w - 1 - x)) * 4) as usize; m.px[d..d + 4].copy_from_slice(&im.px[s..s + 4]); } }
        im = m;
    }
    im.outline(hex("#1d1726"));
    im
}

// ---------- the scene description both renderers consume ----------
#[derive(Serialize, Clone, Copy)]
pub struct Sprite { pub x: i32, pub y: i32, pub w: i32, pub h: i32 }

/// A static thing drawn every frame at its place: a desk, a chair, a seated person who doesn't move.
#[derive(Serialize, Clone, Copy)]
pub struct Item { pub s: usize, pub x: i32, pub y: i32, pub depth: f32 }

/// Something that cycles through frames in place: someone typing, someone waving, the fire.
#[derive(Serialize, Clone)]
pub struct Actor { pub frames: Vec<usize>, pub rate: f32, pub phase: f32, pub x: i32, pub y: i32, pub depth: f32 }

/// Someone walking a loop through the aisles (screen-space path; feet at the point).
#[derive(Serialize, Clone)]
pub struct Walker { pub path: Vec<(f32, f32)>, pub speed: f32, pub offset: f32, pub down: [usize; 2], pub up: [usize; 2], pub left: [usize; 2], pub right: [usize; 2], pub ax: i32, pub ay: i32 }

#[derive(Serialize)]
pub struct Meta { pub art_w: i32, pub art_h: i32, pub atlas_w: i32, pub atlas_h: i32, pub sprites: Vec<Sprite>, pub items: Vec<Item>, pub actors: Vec<Actor>, pub walkers: Vec<Walker>, pub desks: usize, pub people: usize, pub animated: usize }

pub struct Scene { pub meta: Meta, pub bg: Img, pub atlas: Img }

/// One sprite instance to draw this frame.
#[derive(Clone, Copy)]
#[allow(dead_code)] // read by the native renderer; the web renderer has its own copy in floor.html
pub struct Inst { pub s: usize, pub x: f32, pub y: f32, pub depth: f32 }

#[allow(dead_code)]
impl Scene {
    /// The draw list at time `t` seconds, back to front. Floor-plan maths only; no drawing.
    pub fn instances(&self, t: f32, out: &mut Vec<Inst>) {
        out.clear();
        let m = &self.meta;
        for it in &m.items { out.push(Inst { s: it.s, x: it.x as f32, y: it.y as f32, depth: it.depth }); }
        for a in &m.actors {
            let f = a.frames[((t * a.rate + a.phase).floor() as usize) % a.frames.len()];
            out.push(Inst { s: f, x: a.x as f32, y: a.y as f32, depth: a.depth });
        }
        for w in &m.walkers {
            let (x, y, dx, dy) = along(&w.path, t * w.speed + w.offset);
            let pair = if dx.abs() > dy.abs() { if dx > 0.0 { w.right } else { w.left } } else if dy > 0.0 { w.down } else { w.up };
            let f = pair[((t * 6.0).floor() as usize) % 2];
            out.push(Inst { s: f, x: (x - w.ax as f32).round(), y: (y - w.ay as f32).round(), depth: y });
        }
        out.sort_by(|a, b| a.depth.partial_cmp(&b.depth).unwrap());
    }
}

/// Position and direction at distance `s` along a closed polyline.
#[allow(dead_code)]
fn along(path: &[(f32, f32)], s: f32) -> (f32, f32, f32, f32) {
    let segs: Vec<f32> = (0..path.len()).map(|i| { let (a, b) = (path[i], path[(i + 1) % path.len()]); ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt() }).collect();
    let total: f32 = segs.iter().sum();
    let mut d = s.rem_euclid(total);
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

/// Packs sprite images into one atlas with simple shelves.
struct Packer { imgs: Vec<Img> }
impl Packer {
    fn add(&mut self, im: Img) -> usize { self.imgs.push(im); self.imgs.len() - 1 }
    fn pack(self) -> (Img, Vec<Sprite>) {
        let aw = 1024;
        let (mut x, mut y, mut shelf) = (0, 0, 0);
        let mut rects = Vec::new();
        for im in &self.imgs {
            if x + im.w > aw { x = 0; y += shelf; shelf = 0; }
            rects.push(Sprite { x, y, w: im.w, h: im.h });
            x += im.w + 1; shelf = shelf.max(im.h + 1);
        }
        let ah = y + shelf;
        let mut atlas = Img::new(aw, ah.max(1));
        for (im, r) in self.imgs.iter().zip(&rects) {
            for yy in 0..im.h { for xx in 0..im.w {
                let s = ((yy * im.w + xx) * 4) as usize; let d = (((r.y + yy) * aw + r.x + xx) * 4) as usize;
                atlas.px[d..d + 4].copy_from_slice(&im.px[s..s + 4]);
            } }
        }
        (atlas, rects)
    }
}

// The office: 8 App zones (4 columns x 2 rows), 150 desks in pods of four.
const ROOM_X: (f32, f32) = (-18.0, 18.0);
const ROOM_Z: (f32, f32) = (-8.0, 8.0);
const WALL_H: f32 = 3.2;
const HUES: [f32; 8] = [325.0, 115.0, 245.0, 50.0, 355.0, 185.0, 140.0, 305.0];
const DESKS_PER_ZONE: [usize; 8] = [20, 20, 20, 20, 20, 20, 16, 14];

pub fn build() -> Scene {
    let cam = Cam::new();
    let mut rng = Rng(7);
    let mut bg = Img::new(ART_W, ART_H);
    bg.rect(0, 0, ART_W, ART_H, hex("#0b0e14"));
    let q = |a: V3, b: V3, c: V3, d: V3| cam.quad(a, b, c, d);
    let (x0, x1, z0, z1) = (ROOM_X.0, ROOM_X.1, ROOM_Z.0, ROOM_Z.1);
    // floor slab and carpet tiles
    bg.poly(&q([x0 - 6.0, 0.0, z1], [x1, 0.0, z1], [x1, -0.6, z1], [x0 - 6.0, -0.6, z1]), hex("#2a2433"));
    for zi in 0..16 {
        for xi in 0..36 {
            let (fx, fz) = (x0 + xi as f32, z0 + zi as f32);
            let c = if (xi + zi) % 2 == 0 { hex("#5f6272") } else { hex("#5a5d6c") };
            bg.poly(&q([fx, 0.0, fz], [fx + 1.0, 0.0, fz], [fx + 1.0, 0.0, fz + 1.0], [fx, 0.0, fz + 1.0]), c);
        }
    }
    // the annex behind the glass: meeting room and a server closet
    bg.poly(&q([x0 - 6.0, 0.0, z0], [x0, 0.0, z0], [x0, 0.0, 0.0], [x0 - 6.0, 0.0, 0.0]), hex("#4c4458"));
    bg.poly(&q([x0 - 6.0, 0.0, 0.0], [x0, 0.0, 0.0], [x0, 0.0, z1], [x0 - 6.0, 0.0, z1]), hex("#33384a"));
    // zone carpets
    let zone_origin = |zi: usize| -> (f32, f32) { (x0 + 0.6 + (zi % 4) as f32 * 9.0, z0 + 1.2 + (zi / 4) as f32 * 7.4) };
    for zi in 0..8 {
        let (zx, zz) = zone_origin(zi);
        bg.poly(&q([zx, 0.0, zz], [zx + 8.2, 0.0, zz], [zx + 8.2, 0.0, zz + 6.6], [zx, 0.0, zz + 6.6]), shade(hsl(HUES[zi], 32.0, 42.0), 1.0));
    }
    // back wall with windows, doors, a whiteboard and clocks
    let wall = hex("#bdb2c4");
    bg.poly(&q([x0 - 6.0, 0.0, z0], [x1, 0.0, z0], [x1, WALL_H, z0], [x0 - 6.0, WALL_H, z0]), wall);
    bg.poly(&q([x0 - 6.0, WALL_H, z0], [x1, WALL_H, z0], [x1, WALL_H, z0 - 0.3], [x0 - 6.0, WALL_H, z0 - 0.3]), hex("#3a3346"));
    bg.poly(&q([x0 - 6.0, 0.0, z0 + 0.01], [x1, 0.0, z0 + 0.01], [x1, 0.15, z0 + 0.01], [x0 - 6.0, 0.15, z0 + 0.01]), hex("#7e7488"));
    let z = z0 + 0.02;
    let mut wx = x0 - 4.5;
    let mut k = 0;
    while wx < x1 - 2.0 {
        k += 1;
        if k == 6 { // the elevator
            bg.poly(&q([wx - 0.1, 0.0, z], [wx + 2.1, 0.0, z], [wx + 2.1, 2.5, z], [wx - 0.1, 2.5, z]), hex("#5a5f6e"));
            bg.poly(&q([wx, 0.0, z], [wx + 2.0, 0.0, z], [wx + 2.0, 2.4, z], [wx, 2.4, z]), hex("#8d94a3"));
            bg.poly(&q([wx + 0.98, 0.0, z], [wx + 1.02, 0.0, z], [wx + 1.02, 2.4, z], [wx + 0.98, 2.4, z]), hex("#5a5f6e"));
            bg.poly(&q([wx + 0.8, 2.6, z], [wx + 1.2, 2.6, z], [wx + 1.2, 2.85, z], [wx + 0.8, 2.85, z]), hex("#ffb347"));
            wx += 3.2; continue;
        }
        if k == 9 { // the whiteboard with sticky notes and the clock wall
            bg.poly(&q([wx, 0.9, z], [wx + 2.4, 0.9, z], [wx + 2.4, 2.2, z], [wx, 2.2, z]), hex("#f2f3f7"));
            let notes = ["#ffe28a", "#8fd3ff", "#9be8b5", "#ffe28a", "#8fd3ff", "#ffe28a"];
            for (n, c) in notes.iter().enumerate() {
                let (nx, ny) = (wx + 0.25 + (n % 3) as f32 * 0.75, 1.75 - (n / 3) as f32 * 0.45);
                bg.poly(&q([nx, ny, z], [nx + 0.3, ny, z], [nx + 0.3, ny + 0.28, z], [nx, ny + 0.28, z]), hex(c));
            }
            for c in 0..3 { let (cx, cy) = cam.p([wx + 0.5 + c as f32 * 0.7, 2.65, z]); bg.disc(cx, cy, 3.2, hex("#f4f1ea")); bg.rect(cx as i32, cy as i32 - 2, 1, 3, hex("#222")); }
            wx += 3.2; continue;
        }
        if k == 12 { // the kitchen door
            bg.poly(&q([wx - 0.1, 0.0, z], [wx + 1.1, 0.0, z], [wx + 1.1, 2.3, z], [wx - 0.1, 2.3, z]), hex("#5a5f6e"));
            bg.poly(&q([wx, 0.0, z], [wx + 1.0, 0.0, z], [wx + 1.0, 2.2, z], [wx, 2.2, z]), hex("#9b6f43"));
            wx += 2.2; continue;
        }
        // a window onto the night city
        bg.poly(&q([wx - 0.1, 0.85, z], [wx + 2.3, 0.85, z], [wx + 2.3, 2.45, z], [wx - 0.1, 2.45, z]), hex("#ece6d8"));
        bg.poly(&q([wx, 0.95, z], [wx + 2.2, 0.95, z], [wx + 2.2, 2.35, z], [wx, 2.35, z]), hex("#1d2547"));
        bg.poly(&q([wx, 1.85, z], [wx + 2.2, 1.85, z], [wx + 2.2, 2.35, z], [wx, 2.35, z]), hex("#18203f"));
        for b in 0..5 {
            let (bx, bh) = (wx + 0.1 + b as f32 * 0.43, 0.2 + rng.next() * 0.75);
            bg.poly(&q([bx, 0.95, z], [bx + 0.36, 0.95, z], [bx + 0.36, 0.95 + bh, z], [bx, 0.95 + bh, z]), hex("#2c3560"));
            for l in 0..4 { if rng.next() < 0.45 { let (lx, ly) = (bx + 0.06 + (l % 2) as f32 * 0.16, 0.98 + (l / 2) as f32 * 0.16); if ly + 0.08 < 0.95 + bh { bg.poly(&q([lx, ly, z], [lx + 0.07, ly, z], [lx + 0.07, ly + 0.08, z], [lx, ly + 0.08, z]), hex("#ffd27a")); } } }
        }
        bg.poly(&q([wx + 1.08, 0.95, z], [wx + 1.12, 0.95, z], [wx + 1.12, 2.35, z], [wx + 1.08, 2.35, z]), hex("#ece6d8"));
        wx += 3.0;
    }
    // meeting table, chairs and racks in the annex
    draw_box(&mut bg, &cam, (0.0, 0.0), [x0 - 4.6, 0.0, -5.6], [x0 - 1.4, 0.75, -4.0], hex("#e9e7e1"));
    for c in 0..4 { let cx = x0 - 4.4 + c as f32 * 0.8; draw_box(&mut bg, &cam, (0.0, 0.0), [cx, 0.0, -6.4], [cx + 0.45, 0.9, -6.0], hex("#6a4a7a")); draw_box(&mut bg, &cam, (0.0, 0.0), [cx, 0.0, -3.6], [cx + 0.45, 0.9, -3.2], hex("#6a4a7a")); }
    for rk in 0..3 { let rz = 2.0 + rk as f32 * 1.6; draw_box(&mut bg, &cam, (0.0, 0.0), [x0 - 5.6, 0.0, rz], [x0 - 4.8, 2.0, rz + 1.0], hex("#1f2230")); for l in 0..6 { let (lx, ly) = cam.p([x0 - 4.8, 0.4 + l as f32 * 0.25, rz + 0.5]); bg.rect(lx as i32, ly as i32, 3, 1, if l < 4 { hex("#34d399") } else { hex("#30364a") }); } }
    // kitchen counter and fridge on the right of the back wall
    draw_box(&mut bg, &cam, (0.0, 0.0), [x1 - 6.0, 0.0, z0], [x1 - 1.2, 0.95, z0 + 0.7], hex("#e9e2d3"));
    draw_box(&mut bg, &cam, (0.0, 0.0), [x1 - 1.1, 0.0, z0], [x1 - 0.2, 2.0, z0 + 0.8], hex("#e8ecf2"));
    // the glass wall on the left, with mullions
    bg.poly(&q([x0, 0.0, z0], [x0, 0.0, z1], [x0, WALL_H, z1], [x0, WALL_H, z0]), alpha(hex("#bfe0f2"), 46));
    let mut gz = z0;
    while gz <= z1 + 0.01 { bg.poly(&q([x0, 0.0, gz - 0.05], [x0, 0.0, gz + 0.05], [x0, WALL_H, gz + 0.05], [x0, WALL_H, gz - 0.05]), hex("#ece6d8")); gz += 2.0; }
    bg.poly(&q([x0, WALL_H - 0.08, z0], [x0, WALL_H - 0.08, z1], [x0, WALL_H, z1], [x0, WALL_H, z0]), hex("#ece6d8"));
    // the right and front walls are cut down to low stubs, so you see in
    draw_box(&mut bg, &cam, (0.0, 0.0), [x1, 0.0, z0], [x1 + 0.15, 0.35, z1], hex("#d8d2c6"));
    draw_box(&mut bg, &cam, (0.0, 0.0), [x0 - 6.0, 0.0, z1], [x1 + 0.15, 0.35, z1 + 0.15], hex("#d8d2c6"));
    // plants in the corners
    for &(px, pz) in &[(x0 + 0.6, z0 + 0.6), (x1 - 7.0, z0 + 0.6), (x1 - 0.8, z1 - 0.8), (x0 + 0.8, z1 - 0.8), (-0.3, z0 + 0.6)] {
        draw_box(&mut bg, &cam, (0.0, 0.0), [px - 0.25, 0.0, pz - 0.25], [px + 0.25, 0.45, pz + 0.25], hex("#b4643a"));
        let (cx, cy) = cam.p([px, 0.95, pz]);
        bg.disc(cx, cy, 6.5, hex("#2b7240")); bg.disc(cx - 1.5, cy - 2.5, 5.0, hex("#3f9b55")); bg.disc(cx - 2.0, cy - 4.0, 2.0, hex("#6cc77c"));
    }

    // ---- sprites: desks, chairs, people, bubble, fire ----
    let mut pk = Packer { imgs: Vec::new() };
    let mut items: Vec<Item> = Vec::new();
    let mut actors: Vec<Actor> = Vec::new();
    let skins = ["#f6d5bd", "#ebbb97", "#d29b70", "#b07650", "#8a5634", "#62391f"].map(hex);
    let hairs = ["#2b1d14", "#4a3020", "#7a4b25", "#b07a3a", "#dcb766", "#e6e1d6", "#a33b2e", "#1f1f24"].map(hex);
    let pants = ["#2f3a5c", "#3a3a42", "#7a6a4a", "#3d5a80"].map(hex);
    let shoes = ["#1f1b22", "#5b3b2a", "#e8e4dc"].map(hex);
    let look = |hue: f32, rng: &mut Rng| Look { skin: *rng.pick(&skins), hair: *rng.pick(&hairs), shirt: hsl(hue + rng.next() * 24.0 - 12.0, 55.0, 46.0 + rng.next() * 12.0), pants: *rng.pick(&pants), shoes: *rng.pick(&shoes), long: rng.next() < 0.3 };
    let person_sprite = |pk: &mut Packer, lk: &Look, pose: Pose| pk.add(paint_person(lk, pose));
    let (mut desks, mut people, mut typing, mut waver, mut fire) = (0usize, 0usize, 0usize, false, false);
    for zi in 0..8 {
        let (zx, zz) = zone_origin(zi);
        let hue = HUES[zi];
        for d in 0..DESKS_PER_ZONE[zi] {
            let pod = d / 4; let k = d % 4;
            let (pc, pr) = (pod % 2, pod / 2);
            let dx = zx + 0.5 + pc as f32 * 4.0 + (k % 2) as f32 * 1.55;
            let dz = zz + 0.8 + pr as f32 * 2.15 + (k / 2) as f32 * 0.82;
            if pr > 2 { continue; }
            let far = k / 2 == 0; // far row: the person sits behind the desk, facing the camera
            desks += 1;
            // desk with monitor
            let (lo, hi) = ([dx, 0.0, dz], [dx + 1.4, 0.74, dz + 0.75]);
            let mz = if far { dz + 0.2 } else { dz + 0.45 };
            let (mlo, mhi) = ([dx + 0.45, 0.74, mz], [dx + 0.95, 1.02, mz + 0.06]);
            let mut pts = corners(lo, hi); pts.extend(corners(mlo, mhi));
            let (bx, by, bw, bh) = bounds(&cam, &pts);
            let mut im = Img::new(bw, bh);
            let off = (bx as f32, by as f32);
            draw_box(&mut im, &cam, off, lo, hi, if zi == 7 { hex("#a89a86") } else { hex("#d6b282") });
            draw_box(&mut im, &cam, off, mlo, mhi, hex("#1b1b22"));
            let screen = if zi == 7 { hex("#0b0e14") } else { hex("#38bdf8") };
            if !far { let sq = cam.quad([mlo[0] + 0.03, mlo[1] + 0.03, mhi[2] + 0.001], [mhi[0] - 0.03, mlo[1] + 0.03, mhi[2] + 0.001], [mhi[0] - 0.03, mhi[1] - 0.03, mhi[2] + 0.001], [mlo[0] + 0.03, mhi[1] - 0.03, mhi[2] + 0.001]); im.poly(&sq.map(|(x, y)| (x - off.0, y - off.1)), screen); }
            let ds = pk.add(im);
            items.push(Item { s: ds, x: bx, y: by, depth: cam.p([dx + 0.7, 0.0, dz + 0.75]).1 });
            // chair
            let cz = if far { dz - 0.62 } else { dz + 0.95 };
            let (clo, chi) = ([dx + 0.45, 0.0, cz], [dx + 0.95, 0.5, cz + 0.45]);
            let (blo, bhi) = if far { ([dx + 0.45, 0.45, cz - 0.02], [dx + 0.95, 1.05, cz + 0.06]) } else { ([dx + 0.45, 0.45, cz + 0.4], [dx + 0.95, 1.05, cz + 0.48]) };
            let mut cp = corners(clo, chi); cp.extend(corners(blo, bhi));
            let (cx, cy, cw, ch) = bounds(&cam, &cp);
            let mut ci = Img::new(cw, ch);
            draw_box(&mut ci, &cam, (cx as f32, cy as f32), [dx + 0.65, 0.0, cz + 0.15], [dx + 0.75, 0.45, cz + 0.25], hex("#2a2a33"));
            draw_box(&mut ci, &cam, (cx as f32, cy as f32), [dx + 0.45, 0.42, cz], [dx + 0.95, 0.5, cz + 0.45], hex("#3b3f4c"));
            draw_box(&mut ci, &cam, (cx as f32, cy as f32), blo, bhi, hex("#3b3f4c"));
            let chs = pk.add(ci);
            // the person: most sit, a third of them type, one stands and waves
            if zi == 7 { items.push(Item { s: chs, x: cx, y: cy, depth: cam.p([dx + 0.7, 0.0, cz + 0.45]).1 + if far { -3.0 } else { 3.0 } }); continue; }
            let lk = look(hue, &mut rng);
            let foot = cam.p([dx + 0.7, 0.0, cz + 0.22]);
            let (ax, ay) = (7, 31);
            let pdepth = foot.1;
            people += 1;
            let chair_depth = if far { pdepth - 0.5 } else { pdepth + 0.5 };
            items.push(Item { s: chs, x: cx, y: cy, depth: chair_depth });
            if !waver && zi == 1 && d == 5 {
                waver = true;
                let f0 = person_sprite(&mut pk, &lk, Pose::Wave(0)); let f1 = person_sprite(&mut pk, &lk, Pose::Wave(1));
                let stand = cam.p([dx + 0.2, 0.0, cz + 0.1]);
                actors.push(Actor { frames: vec![f0, f1], rate: 3.0, phase: 0.0, x: (stand.0 - 7.0).round() as i32, y: (stand.1 - 33.0).round() as i32, depth: stand.1 + 0.8 });
                let b = bubble(&mut pk);
                items.push(Item { s: b, x: (stand.0 - 7.0).round() as i32, y: (stand.1 - 50.0).round() as i32, depth: 10_000.0 });
                continue;
            }
            let anim = typing < 33 && (d + zi) % 3 == 0;
            let (p0, p1) = if far { (Pose::SitFront(0), Pose::SitFront(1)) } else { (Pose::SitBack(0), Pose::SitBack(1)) };
            let s0 = person_sprite(&mut pk, &lk, p0);
            let (px, py) = ((foot.0 - ax as f32).round() as i32, (foot.1 - ay as f32).round() as i32);
            if anim {
                typing += 1;
                let s1 = person_sprite(&mut pk, &lk, p1);
                actors.push(Actor { frames: vec![s0, s1], rate: 4.0, phase: rng.next() * 2.0, x: px, y: py, depth: pdepth });
            } else {
                items.push(Item { s: s0, x: px, y: py, depth: pdepth });
            }
            if !fire && zi == 3 && d == 2 {
                fire = true;
                let fr: Vec<usize> = (0..3).map(|f| flame(&mut pk, f)).collect();
                let fp = cam.p([dx + 1.2, 0.74, dz + 0.35]);
                actors.push(Actor { frames: fr, rate: 8.0, phase: 0.0, x: (fp.0 - 5.0).round() as i32, y: (fp.1 - 12.0).round() as i32, depth: cam.p([dx + 0.7, 0.0, dz + 0.75]).1 + 0.1 });
            }
        }
    }
    // six people walking loops through the aisles
    let mut walkers = Vec::new();
    // Thin loops along the aisles between zones, so people walk there and back.
    let loops: [[(f32, f32); 4]; 6] = [
        [(-17.4, -7.5), (16.0, -7.5), (16.0, -7.2), (-17.4, -7.2)],
        [(-17.4, 0.1), (17.4, 0.1), (17.4, 0.35), (-17.4, 0.35)],
        [(-8.85, -7.3), (-8.6, -7.3), (-8.6, 7.6), (-8.85, 7.6)],
        [(0.1, -6.9), (0.35, -6.9), (0.35, 7.6), (0.1, 7.6)],
        [(9.05, -7.3), (9.3, -7.3), (9.3, 7.6), (9.05, 7.6)],
        [(-17.4, 7.55), (17.4, 7.55), (17.4, 7.8), (-17.4, 7.8)],
    ];
    for (wi, lp) in loops.iter().enumerate() {
        let lk = look(HUES[wi], &mut rng);
        let s = |pk: &mut Packer, p: Pose| person_sprite(pk, &lk, p);
        let down = [s(&mut pk, Pose::WalkDown(0)), s(&mut pk, Pose::WalkDown(1))];
        let up = [s(&mut pk, Pose::WalkUp(0)), s(&mut pk, Pose::WalkUp(1))];
        let right = [s(&mut pk, Pose::WalkSide(0, false)), s(&mut pk, Pose::WalkSide(1, false))];
        let left = [s(&mut pk, Pose::WalkSide(0, true)), s(&mut pk, Pose::WalkSide(1, true))];
        let path: Vec<(f32, f32)> = lp.iter().map(|&(x, z)| cam.p([x, 0.0, z])).collect();
        walkers.push(Walker { path, speed: 26.0, offset: wi as f32 * 97.0, down, up, left, right, ax: 7, ay: 33 });
        people += 1;
    }
    let animated = typing + 1 + walkers.len();
    let (atlas, sprites) = pk.pack();
    Scene { meta: Meta { art_w: ART_W, art_h: ART_H, atlas_w: atlas.w, atlas_h: atlas.h, sprites, items, actors, walkers, desks, people, animated }, bg, atlas }
}

fn bubble(pk: &mut Packer) -> usize {
    let mut im = Img::new(17, 17);
    let (w, c) = (hex("#fffaf0"), hex("#d69e2e"));
    im.rect(2, 1, 13, 11, c); im.rect(3, 2, 11, 9, w); im.rect(7, 12, 3, 2, c); im.rect(8, 12, 1, 1, w);
    im.rect(8, 3, 1, 5, c); im.rect(8, 9, 1, 1, c);
    pk.add(im)
}
fn flame(pk: &mut Packer, f: i32) -> usize {
    let mut im = Img::new(10, 14);
    let o = [0, 1, -1][f as usize];
    im.rect(2, 4 + o, 6, 9, hex("#e5452f")); im.rect(3, 2, 4, 3, hex("#e5452f")); im.rect(4 + o, 0, 2, 3, hex("#e5452f"));
    im.rect(3, 6, 4, 6, hex("#ff9a3d")); im.rect(4, 4 + o, 2, 3, hex("#ff9a3d")); im.rect(4, 8, 2, 4, hex("#ffe08a"));
    pk.add(im)
}
