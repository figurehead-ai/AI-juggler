//! The mesh library: a few low-poly unit shapes that every model is built from. All are flat shaded
//! (one normal per facet), which reads as crafted low-poly once the frame is pixelated. Units: x and z
//! span -0.5..0.5 and y spans 0..1, so a shape stands on its origin.

use crate::math::{cross, dot, norm, sub, V3};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex { pub pos: [f32; 3], pub n: [f32; 3], pub uv: [f32; 2] }

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Mesh { Box, Chamfer, Cyl6, Cyl8, Cyl12, Pot, Cone8, Ball, BallLo, Leaf, Frond, QuadUp, QuadWall, Disc }
pub const ALL: [Mesh; 14] = [Mesh::Box, Mesh::Chamfer, Mesh::Cyl6, Mesh::Cyl8, Mesh::Cyl12, Mesh::Pot, Mesh::Cone8, Mesh::Ball, Mesh::BallLo, Mesh::Leaf, Mesh::Frond, Mesh::QuadUp, Mesh::QuadWall, Mesh::Disc];

pub struct Library { pub verts: Vec<Vertex>, pub idx: Vec<u32>, pub ranges: Vec<(u32, u32)> }

impl Library {
    pub fn build() -> Library {
        let mut lib = Library { verts: vec![], idx: vec![], ranges: vec![] };
        for m in ALL {
            let start = lib.idx.len() as u32;
            let mut b = Builder { verts: &mut lib.verts, idx: &mut lib.idx };
            match m {
                Mesh::Box => cube(&mut b, 0.0),
                Mesh::Chamfer => cube(&mut b, 0.14),
                Mesh::Cyl6 => frustum(&mut b, 6, 1.0, true),
                Mesh::Cyl8 => frustum(&mut b, 8, 1.0, true),
                Mesh::Cyl12 => frustum(&mut b, 12, 1.0, true),
                Mesh::Pot => frustum(&mut b, 8, 1.28, true),
                Mesh::Cone8 => frustum(&mut b, 8, 0.0, true),
                Mesh::Ball => ball(&mut b, 10, 7),
                Mesh::BallLo => ball(&mut b, 6, 4),
                Mesh::Leaf => leaf(&mut b, 0.42, 0.18),
                Mesh::Frond => frond(&mut b),
                Mesh::QuadUp => b.poly(&[[-0.5, 0.0, -0.5], [-0.5, 0.0, 0.5], [0.5, 0.0, 0.5], [0.5, 0.0, -0.5]], Some([0.0, 1.0, 0.0])),
                Mesh::QuadWall => b.poly(&[[-0.5, 0.0, 0.0], [0.5, 0.0, 0.0], [0.5, 1.0, 0.0], [-0.5, 1.0, 0.0]], Some([0.0, 0.0, 1.0])),
                Mesh::Disc => { let n = 16; let pts: Vec<V3> = (0..n).map(|i| { let a = i as f32 / n as f32 * std::f32::consts::TAU; [0.5 * a.cos(), 0.0, 0.5 * a.sin()] }).collect(); b.poly(&pts, Some([0.0, 1.0, 0.0])); }
            }
            lib.ranges.push((start, lib.idx.len() as u32 - start));
        }
        lib
    }
    pub fn range(&self, m: Mesh) -> (u32, u32) { self.ranges[ALL.iter().position(|&x| x == m).unwrap()] }
}

struct Builder<'a> { verts: &'a mut Vec<Vertex>, idx: &'a mut Vec<u32> }

impl Builder<'_> {
    /// A convex polygon, wound to face `want` (or its own normal). UVs come from the dominant plane.
    fn poly(&mut self, pts: &[V3], want: Option<V3>) {
        let mut p: Vec<V3> = pts.to_vec();
        let mut n = norm(cross(sub(p[1], p[0]), sub(p[2], p[0])));
        if let Some(w) = want { if dot(n, w) < 0.0 { p.reverse(); n = [-n[0], -n[1], -n[2]]; } }
        let base = self.verts.len() as u32;
        let ax = if n[1].abs() > 0.7 { (0, 2) } else if n[0].abs() > n[2].abs() { (2, 1) } else { (0, 1) };
        for q in &p {
            let mut u = q[ax.0] + 0.5;
            let v = if ax.1 == 1 { 1.0 - q[1] } else { q[ax.1] + 0.5 };
            if (ax == (0, 1) && n[2] < 0.0) || (ax == (2, 1) && n[0] > 0.0) { u = 1.0 - u; }
            self.verts.push(Vertex { pos: *q, n, uv: [u, v] });
        }
        for i in 1..p.len() as u32 - 1 { self.idx.extend_from_slice(&[base, base + i, base + i + 1]); }
    }
    fn tri2(&mut self, a: V3, b: V3, c: V3) {
        // both sides, for thin leaves
        let n = norm(cross(sub(b, a), sub(c, a)));
        let base = self.verts.len() as u32;
        for (q, uv) in [(a, [0.0, 0.0]), (b, [1.0, 0.0]), (c, [0.0, 1.0])] { self.verts.push(Vertex { pos: q, n, uv }); }
        let m = [-n[0], -n[1], -n[2]];
        for (q, uv) in [(a, [0.0, 0.0]), (c, [0.0, 1.0]), (b, [1.0, 0.0])] { self.verts.push(Vertex { pos: q, n: m, uv }); }
        self.idx.extend_from_slice(&[base, base + 1, base + 2, base + 3, base + 4, base + 5]);
    }
}

/// A unit box standing on y=0, its edges chamfered by `b` (0 = sharp).
fn cube(bd: &mut Builder, b: f32) {
    let a = 0.5 - b;
    let c = |x: f32, y: f32, z: f32| -> V3 { [x, y + 0.5, z] };
    let axes = [[1.0, 0.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0, -1.0]];
    // the six faces
    for n in axes {
        let k = n.iter().position(|v: &f32| v.abs() > 0.5).unwrap();
        let (i, j) = ((k + 1) % 3, (k + 2) % 3);
        let pts: Vec<V3> = [(-a, -a), (a, -a), (a, a), (-a, a)].iter().map(|&(u, v)| { let mut p = [0.0; 3]; p[k] = 0.5 * n[k]; p[i] = u; p[j] = v; c(p[0], p[1], p[2]) }).collect();
        bd.poly(&pts, Some(n));
    }
    if b <= 0.0 { return; }
    // the twelve edge strips and eight corners
    for k in 0..3 {
        let (i, j) = ((k + 1) % 3, (k + 2) % 3);
        for si in [-1.0f32, 1.0] { for sj in [-1.0f32, 1.0] {
            let mut pts = vec![];
            for sk in [-1.0f32, 1.0] {
                let mut p1 = [0.0; 3]; p1[i] = 0.5 * si; p1[j] = a * sj; p1[k] = a * sk;
                let mut p2 = [0.0; 3]; p2[i] = a * si; p2[j] = 0.5 * sj; p2[k] = a * sk;
                pts.push(c(p1[0], p1[1], p1[2]));
                pts.push(c(p2[0], p2[1], p2[2]));
            }
            let mut n = [0.0; 3]; n[i] = si; n[j] = sj;
            bd.poly(&[pts[0], pts[1], pts[3], pts[2]], Some(norm(n)));
        } }
    }
    for sx in [-1.0f32, 1.0] { for sy in [-1.0f32, 1.0] { for sz in [-1.0f32, 1.0] {
        bd.poly(&[c(0.5 * sx, a * sy, a * sz), c(a * sx, 0.5 * sy, a * sz), c(a * sx, a * sy, 0.5 * sz)], Some(norm([sx, sy, sz])));
    } } }
}

/// A faceted frustum: radius 0.5 at the bottom, 0.5×`top` at the top (0 makes a cone).
fn frustum(b: &mut Builder, n: usize, top: f32, caps: bool) {
    let ring = |r: f32, y: f32| -> Vec<V3> { (0..n).map(|i| { let a = (i as f32 + 0.5) / n as f32 * std::f32::consts::TAU; [r * a.cos(), y, r * a.sin()] }).collect() };
    let (lo, hi) = (ring(0.5, 0.0), ring(0.5 * top, 1.0));
    for i in 0..n {
        let j = (i + 1) % n;
        let mid = [(lo[i][0] + lo[j][0]) * 0.5, 0.0, (lo[i][2] + lo[j][2]) * 0.5];
        if top > 0.0 { b.poly(&[lo[i], lo[j], hi[j], hi[i]], Some(mid)); } else { b.poly(&[lo[i], lo[j], hi[i]], Some(mid)); }
    }
    if caps {
        b.poly(&lo, Some([0.0, -1.0, 0.0]));
        if top > 0.0 { b.poly(&hi, Some([0.0, 1.0, 0.0])); }
    }
}

/// A faceted ball of radius 0.5 centred at y=0.5.
fn ball(b: &mut Builder, lon: usize, lat: usize) {
    let p = |i: usize, j: usize| -> V3 {
        let th = j as f32 / lat as f32 * std::f32::consts::PI;
        let ph = i as f32 / lon as f32 * std::f32::consts::TAU;
        [0.5 * th.sin() * ph.cos(), 0.5 + 0.5 * th.cos(), 0.5 * th.sin() * ph.sin()]
    };
    for j in 0..lat { for i in 0..lon {
        let (a, bb, c, d) = (p(i, j), p(i + 1, j), p(i + 1, j + 1), p(i, j + 1));
        let mid = [(a[0] + c[0]) * 0.5, (a[1] + c[1]) * 0.5 - 0.5, (a[2] + c[2]) * 0.5];
        if j == 0 { b.poly(&[a, c, d], Some(mid)); } else if j == lat - 1 { b.poly(&[a, bb, c], Some(mid)); } else { b.poly(&[a, bb, c, d], Some(mid)); }
    } }
}

/// A leaf along +z (0..1): pointed, widest at `wide` of its length, folded along the midrib.
fn leaf(b: &mut Builder, wide: f32, fold: f32) {
    let steps = 5;
    let at = |t: f32| -> (V3, V3, V3) {
        let w = if t < wide { (t / wide).powf(0.6) } else { ((1.0 - t) / (1.0 - wide)).powf(0.8) } * 0.5;
        let y = -0.18 * t * t;
        ([0.0, y, t], [-w, y - fold * w, t], [w, y - fold * w, t])
    };
    for s in 0..steps {
        let (t0, t1) = (s as f32 / steps as f32, (s + 1) as f32 / steps as f32);
        let ((m0, l0, r0), (m1, l1, r1)) = (at(t0), at(t1));
        b.tri2(m0, l0, l1); b.tri2(m0, l1, m1);
        b.tri2(m0, m1, r1); b.tri2(m0, r1, r0);
    }
}

/// A fern frond: a narrow arching midrib with leaflets along it.
fn frond(b: &mut Builder) {
    let n = 7;
    let rib = |t: f32| -> V3 { [0.0, 0.35 * t - 0.55 * t * t, t] };
    for i in 0..n {
        let (t0, t1) = (i as f32 / n as f32, (i + 1) as f32 / n as f32);
        let (p0, p1) = (rib(t0), rib(t1));
        let w = 0.32 * (1.0 - t0 * 0.8);
        let tip = |s: f32| -> V3 { [s * w, p0[1] - 0.05, (p0[2] + p1[2]) * 0.5 + 0.06] };
        b.tri2(p0, p1, tip(-1.0));
        b.tri2(p0, tip(1.0), p1);
    }
}
