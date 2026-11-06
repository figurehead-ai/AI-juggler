//! The mesh library: a few low-poly unit shapes every model is built from.
//! Flat shaded (one normal per facet), which reads as crafted low-poly once
//! the frame is pixelated. Units: x and z span -0.5–0.5 and y spans 0–1, so
//! a shape stands on its origin.

use std::f32::consts::{PI, TAU};

use crate::math::{V3, cross, dot, len, norm, sub};

/// One vertex: where, its facet's normal, and its texture place.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    /// Where.
    pub pos: [f32; 3],
    /// The facet's normal.
    pub n: [f32; 3],
    /// Its texture coordinate.
    pub uv: [f32; 2],
}

/// A unit shape.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Mesh {
    /// A box.
    Box,
    /// A box with chamfered edges.
    Chamfer,
    /// A six-sided cylinder.
    Cyl6,
    /// An eight-sided cylinder.
    Cyl8,
    /// A twelve-sided cylinder.
    Cyl12,
    /// A pot: an eight-sided frustum, wider at the top.
    Pot,
    /// An eight-sided cone.
    Cone8,
    /// A ball.
    Ball,
    /// A coarse ball.
    BallLo,
    /// A folded leaf along +z.
    Leaf,
    /// A fern frond along +z.
    Frond,
    /// A flat square facing up.
    QuadUp,
    /// A flat square facing +z, standing.
    QuadWall,
    /// A flat disc facing up.
    Disc,
    /// A smooth ball (0.4.1's second pass: people and round things).
    Sphere,
    /// A smooth capsule standing on y = 0: its caps a quarter of its
    /// height each, so a limb scaled about four times as long as it's wide
    /// has round ends.
    Capsule,
    /// A box with smoothly rounded edges.
    RoundBox,
    /// A smooth-sided cylinder.
    CylS,
    /// A smooth torso: a chest wider than the waist, round shoulders.
    Torso,
}

impl Mesh {
    /// Every shape, in the library's order.
    pub const ALL: [Self; 19] = [
        Self::Box,
        Self::Chamfer,
        Self::Cyl6,
        Self::Cyl8,
        Self::Cyl12,
        Self::Pot,
        Self::Cone8,
        Self::Ball,
        Self::BallLo,
        Self::Leaf,
        Self::Frond,
        Self::QuadUp,
        Self::QuadWall,
        Self::Disc,
        Self::Sphere,
        Self::Capsule,
        Self::RoundBox,
        Self::CylS,
        Self::Torso,
    ];

    /// Its place in [`Self::ALL`].
    #[must_use]
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|m| *m == self).unwrap_or(0)
    }
}

/// Every shape in one vertex and index buffer, with each shape's range of
/// indices.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Library {
    /// The vertices.
    pub verts: Vec<Vertex>,
    /// The triangles' indices.
    pub idx: Vec<u32>,
    /// Each shape's `(first index, count)`, in [`Mesh::ALL`]'s order.
    pub ranges: Vec<(u32, u32)>,
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

impl Library {
    /// Builds every shape.
    #[must_use]
    pub fn build() -> Self {
        let mut lib = Self::default();
        for m in Mesh::ALL {
            let start = count(lib.idx.len());
            let mut b = Builder {
                verts: &mut lib.verts,
                idx: &mut lib.idx,
            };
            match m {
                Mesh::Box => cube(&mut b, 0.0),
                Mesh::Chamfer => cube(&mut b, 0.14),
                Mesh::Cyl6 => frustum(&mut b, 6, 1.0),
                Mesh::Cyl8 => frustum(&mut b, 8, 1.0),
                Mesh::Cyl12 => frustum(&mut b, 12, 1.0),
                Mesh::Pot => frustum(&mut b, 8, 1.28),
                Mesh::Cone8 => frustum(&mut b, 8, 0.0),
                Mesh::Ball => ball(&mut b, 10, 7),
                Mesh::BallLo => ball(&mut b, 6, 4),
                Mesh::Leaf => leaf(&mut b, 0.42, 0.18),
                Mesh::Frond => frond(&mut b),
                Mesh::QuadUp => b.poly(
                    &[
                        [-0.5, 0.0, -0.5],
                        [-0.5, 0.0, 0.5],
                        [0.5, 0.0, 0.5],
                        [0.5, 0.0, -0.5],
                    ],
                    Some([0.0, 1.0, 0.0]),
                ),
                Mesh::QuadWall => b.poly(
                    &[
                        [-0.5, 0.0, 0.0],
                        [0.5, 0.0, 0.0],
                        [0.5, 1.0, 0.0],
                        [-0.5, 1.0, 0.0],
                    ],
                    Some([0.0, 0.0, 1.0]),
                ),
                Mesh::Disc => {
                    let pts: Vec<V3> = (0..16)
                        .map(|i| {
                            let a = i as f32 / 16.0 * TAU;
                            [0.5 * a.cos(), 0.0, 0.5 * a.sin()]
                        })
                        .collect();
                    b.poly(&pts, Some([0.0, 1.0, 0.0]));
                }
                Mesh::Sphere => sphere(&mut b),
                Mesh::Capsule => capsule(&mut b),
                Mesh::RoundBox => round_box(&mut b, 0.16),
                Mesh::CylS => cyl_smooth(&mut b, 20),
                Mesh::Torso => torso(&mut b),
            }
            lib.ranges.push((start, count(lib.idx.len()) - start));
        }
        lib
    }

    /// `m`'s `(first index, count)`.
    #[must_use]
    pub fn range(&self, m: Mesh) -> (u32, u32) {
        self.ranges.get(m.index()).copied().unwrap_or((0, 0))
    }
}

struct Builder<'a> {
    verts: &'a mut Vec<Vertex>,
    idx: &'a mut Vec<u32>,
}

impl Builder<'_> {
    /// A convex polygon, wound to face `want` (else its own normal); its
    /// texture coordinates from the plane it lies most in.
    fn poly(&mut self, pts: &[V3], want: Option<V3>) {
        if pts.len() < 3 {
            return;
        }
        let mut p: Vec<V3> = pts.to_vec();
        let facet = cross(sub(p[1], p[0]), sub(p[2], p[0]));
        if len(facet) < 1e-9 {
            return; // no area: nothing to see
        }
        let mut n = norm(facet);
        if want.is_some_and(|w| dot(n, w) < 0.0) {
            p.reverse();
            n = [-n[0], -n[1], -n[2]];
        }
        let base = count(self.verts.len());
        let axes = if n[1].abs() > 0.7 {
            (0, 2)
        } else if n[0].abs() > n[2].abs() {
            (2, 1)
        } else {
            (0, 1)
        };
        for q in &p {
            let mut u = q[axes.0] + 0.5;
            let v = if axes.1 == 1 {
                1.0 - q[1]
            } else {
                q[axes.1] + 0.5
            };
            if (axes == (0, 1) && n[2] < 0.0) || (axes == (2, 1) && n[0] > 0.0) {
                u = 1.0 - u;
            }
            self.verts.push(Vertex {
                pos: *q,
                n,
                uv: [u, v],
            });
        }
        for i in 1..count(p.len()) - 1 {
            self.idx.extend_from_slice(&[base, base + i, base + i + 1]);
        }
    }

    /// A triangle seen from both sides (thin leaves).
    fn tri2(&mut self, a: V3, b: V3, c: V3) {
        let facet = cross(sub(b, a), sub(c, a));
        if len(facet) < 1e-9 {
            return; // a leaf's tip: no area
        }
        let n = norm(facet);
        let back = [-n[0], -n[1], -n[2]];
        let base = count(self.verts.len());
        for (q, uv) in [(a, [0.0, 0.0]), (b, [1.0, 0.0]), (c, [0.0, 1.0])] {
            self.verts.push(Vertex { pos: q, n, uv });
        }
        for (q, uv) in [(a, [0.0, 0.0]), (c, [0.0, 1.0]), (b, [1.0, 0.0])] {
            self.verts.push(Vertex {
                pos: q,
                n: back,
                uv,
            });
        }
        self.idx
            .extend_from_slice(&[base, base + 1, base + 2, base + 3, base + 4, base + 5]);
    }
}

impl Builder<'_> {
    /// A smooth surface over a `nu` × `nv` grid of `f(u, v)` (u, v in
    /// 0–1), each vertex's normal from its neighbours (outward from
    /// `centre`'s axis), its texture coordinate `(u, v)`.
    fn surface(&mut self, nu: usize, nv: usize, f: impl Fn(f32, f32) -> V3, centre: V3) {
        let base = count(self.verts.len());
        let e = 1e-3;
        for j in 0..=nv {
            for i in 0..=nu {
                let (u, v) = (i as f32 / nu as f32, j as f32 / nv as f32);
                let p = f(u, v);
                let du = sub(f(u + e, v), f(u - e, v));
                let dv = sub(f(u, (v + e).min(1.0)), f(u, (v - e).max(0.0)));
                let c = cross(du, dv);
                let out = sub(
                    p,
                    [
                        centre[0],
                        p[1].clamp(centre[1] - 0.5, centre[1] + 0.5),
                        centre[2],
                    ],
                );
                let mut n = if len(c) > 1e-7 {
                    norm(c)
                } else {
                    norm(sub(p, centre))
                };
                if len(c) > 1e-7 && dot(n, out) < 0.0 {
                    n = [-n[0], -n[1], -n[2]];
                }
                if len(c) <= 1e-7 && len(out) < 1e-6 {
                    n = [0.0, if p[1] > centre[1] { 1.0 } else { -1.0 }, 0.0];
                }
                self.verts.push(Vertex {
                    pos: p,
                    n,
                    uv: [u, v],
                });
            }
        }
        let row = count(nu + 1);
        for j in 0..count(nv) {
            for i in 0..count(nu) {
                let (a, b, c, d) = (
                    base + j * row + i,
                    base + j * row + i + 1,
                    base + (j + 1) * row + i + 1,
                    base + (j + 1) * row + i,
                );
                for tri in [[a, b, c], [a, c, d]] {
                    self.tri_out(tri);
                }
            }
        }
    }

    /// A triangle of existing vertices, wound to face the way its
    /// vertices' normals do (skipped when it has no area).
    fn tri_out(&mut self, [a, b, c]: [u32; 3]) {
        let (pa, pb, pc) = (
            self.verts[a as usize].pos,
            self.verts[b as usize].pos,
            self.verts[c as usize].pos,
        );
        let facet = cross(sub(pb, pa), sub(pc, pa));
        if len(facet) < 1e-10 {
            return;
        }
        let na = self.verts[a as usize].n;
        let nb = self.verts[b as usize].n;
        let nc = self.verts[c as usize].n;
        let avg = [
            na[0] + nb[0] + nc[0],
            na[1] + nb[1] + nc[1],
            na[2] + nb[2] + nc[2],
        ];
        if dot(facet, avg) >= 0.0 {
            self.idx.extend_from_slice(&[a, b, c]);
        } else {
            self.idx.extend_from_slice(&[a, c, b]);
        }
    }
}

/// A smooth ball of radius 0.5 centred at y = 0.5.
fn sphere(b: &mut Builder) {
    b.surface(
        24,
        16,
        |u, v| {
            let (ph, th) = (u * TAU, v * PI);
            [
                0.5 * th.sin() * ph.cos(),
                0.5 + 0.5 * th.cos(),
                0.5 * th.sin() * ph.sin(),
            ]
        },
        [0.0, 0.5, 0.0],
    );
}

/// A smooth capsule on y = 0, height 1, radius 0.5; caps a quarter each.
fn capsule(b: &mut Builder) {
    b.surface(
        16,
        14,
        |u, v| {
            let ph = u * TAU;
            // v: 0 bottom pole … 1 top pole; caps on 0–0.3 and 0.7–1
            let (r, y) = if v < 0.3 {
                let th = (v / 0.3) * PI / 2.0;
                (0.5 * th.sin(), 0.25 - 0.25 * th.cos())
            } else if v > 0.7 {
                let th = ((1.0 - v) / 0.3) * PI / 2.0;
                (0.5 * th.sin(), 0.75 + 0.25 * th.cos())
            } else {
                (0.5, 0.25 + (v - 0.3) / 0.4 * 0.5)
            };
            [r * ph.cos(), y, r * ph.sin()]
        },
        [0.0, 0.5, 0.0],
    );
}

/// A unit box on y = 0 with its edges rounded by `r`, smooth normals.
fn round_box(b: &mut Builder, r: f32) {
    let a = 0.5 - r;
    let steps = [
        -0.5,
        -0.5 + r / 3.0,
        -0.5 + 2.0 * r / 3.0,
        -a,
        a,
        0.5 - 2.0 * r / 3.0,
        0.5 - r / 3.0,
        0.5,
    ];
    let n = steps.len();
    for axis in 0..3 {
        for side in [-1.0f32, 1.0] {
            let (i, j) = ((axis + 1) % 3, (axis + 2) % 3);
            let base = count(b.verts.len());
            for sv in steps {
                for su in steps {
                    let mut g = [0.0f32; 3];
                    g[axis] = 0.5 * side;
                    g[i] = su;
                    g[j] = sv;
                    let inner = [g[0].clamp(-a, a), g[1].clamp(-a, a), g[2].clamp(-a, a)];
                    let d = sub(g, inner);
                    let nn = norm(d);
                    let p = [
                        inner[0] + nn[0] * r,
                        inner[1] + nn[1] * r + 0.5,
                        inner[2] + nn[2] * r,
                    ];
                    b.verts.push(Vertex {
                        pos: p,
                        n: nn,
                        uv: [su + 0.5, sv + 0.5],
                    });
                }
            }
            let row = count(n);
            for v in 0..row - 1 {
                for u in 0..row - 1 {
                    let (p, q, s, t) = (
                        base + v * row + u,
                        base + v * row + u + 1,
                        base + (v + 1) * row + u + 1,
                        base + (v + 1) * row + u,
                    );
                    b.tri_out([p, q, s]);
                    b.tri_out([p, s, t]);
                }
            }
        }
    }
}

/// A cylinder of radius 0.5 on y = 0–1 with `n` smooth sides and flat
/// caps.
fn cyl_smooth(b: &mut Builder, n: usize) {
    b.surface(
        n,
        1,
        |u, v| {
            let a = u * TAU;
            [0.5 * a.cos(), v, 0.5 * a.sin()]
        },
        [0.0, 0.5, 0.0],
    );
    for (y, up) in [(0.0f32, -1.0f32), (1.0, 1.0)] {
        let pts: Vec<V3> = (0..n)
            .map(|i| {
                let a = i as f32 / n as f32 * TAU;
                [0.5 * a.cos(), y, 0.5 * a.sin()]
            })
            .collect();
        b.poly(&pts, Some([0.0, up, 0.0]));
    }
}

/// A torso on y = 0–1: hips at the bottom, the waist a little in, the
/// chest out, round shoulders closing the top; x is its width (±0.5 at
/// the chest), z its depth (flatter).
fn torso(b: &mut Builder) {
    // half width and half depth up the body (the depth's taken as a share
    // of the width in the instance's own scale)
    let bump = |y: f32, c: f32, w: f32| (-((y - c) / w).powi(2)).exp();
    let w = |y: f32| 0.42 - 0.06 * bump(y, 0.3, 0.2) + 0.08 * bump(y, 0.74, 0.22);
    b.surface(
        24,
        16,
        |u, v| {
            let a = u * TAU;
            // v: 0 bottom … 1 top; the top 20% rounds into the shoulders
            let (y, k) = if v < 0.8 {
                (v / 0.8 * 0.86, 1.0)
            } else {
                let th = (v - 0.8) / 0.2 * PI / 2.0;
                (0.86 + 0.14 * th.sin(), th.cos())
            };
            let hw = w(y.min(0.86)) * (0.3 + 0.7 * k);
            let hd = (0.5 - 0.06 * (y - 0.5).abs()) * (0.3 + 0.7 * k);
            // a squarish ellipse: shoulders read broad
            let (c, s) = (a.cos(), a.sin());
            let sq = |t: f32| t.signum() * t.abs().powf(0.8);
            [hw * sq(c), y, hd * sq(s)]
        },
        [0.0, 0.5, 0.0],
    );
    let pts: Vec<V3> = (0..24)
        .map(|i| {
            let a = i as f32 / 24.0 * TAU;
            let (c, s) = (a.cos(), a.sin());
            let sq = |t: f32| t.signum() * t.abs().powf(0.8);
            [0.43 * sq(c), 0.0, 0.47 * sq(s)]
        })
        .collect();
    b.poly(&pts, Some([0.0, -1.0, 0.0]));
}

/// A unit box standing on y = 0, its edges chamfered by `bevel` (0: sharp).
fn cube(bd: &mut Builder, bevel: f32) {
    let a = 0.5 - bevel;
    let c = |p: V3| -> V3 { [p[0], p[1] + 0.5, p[2]] };
    let axes: [V3; 6] = [
        [1.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, -1.0, 0.0],
        [0.0, 0.0, 1.0],
        [0.0, 0.0, -1.0],
    ];
    for n in axes {
        let k = n.iter().position(|v| v.abs() > 0.5).unwrap_or(0);
        let (i, j) = ((k + 1) % 3, (k + 2) % 3);
        let pts: Vec<V3> = [(-a, -a), (a, -a), (a, a), (-a, a)]
            .iter()
            .map(|&(u, v)| {
                let mut p = [0.0; 3];
                p[k] = 0.5 * n[k];
                p[i] = u;
                p[j] = v;
                c(p)
            })
            .collect();
        bd.poly(&pts, Some(n));
    }
    if bevel <= 0.0 {
        return;
    }
    // The twelve edge strips, then the eight corners.
    for k in 0..3 {
        let (i, j) = ((k + 1) % 3, (k + 2) % 3);
        for si in [-1.0f32, 1.0] {
            for sj in [-1.0f32, 1.0] {
                let mut pts = Vec::with_capacity(4);
                for sk in [-1.0f32, 1.0] {
                    let mut p1 = [0.0; 3];
                    p1[i] = 0.5 * si;
                    p1[j] = a * sj;
                    p1[k] = a * sk;
                    let mut p2 = [0.0; 3];
                    p2[i] = a * si;
                    p2[j] = 0.5 * sj;
                    p2[k] = a * sk;
                    pts.push(c(p1));
                    pts.push(c(p2));
                }
                let mut n = [0.0; 3];
                n[i] = si;
                n[j] = sj;
                bd.poly(&[pts[0], pts[1], pts[3], pts[2]], Some(norm(n)));
            }
        }
    }
    for sx in [-1.0f32, 1.0] {
        for sy in [-1.0f32, 1.0] {
            for sz in [-1.0f32, 1.0] {
                bd.poly(
                    &[
                        c([0.5 * sx, a * sy, a * sz]),
                        c([a * sx, 0.5 * sy, a * sz]),
                        c([a * sx, a * sy, 0.5 * sz]),
                    ],
                    Some(norm([sx, sy, sz])),
                );
            }
        }
    }
}

/// A faceted frustum with caps: radius 0.5 at the bottom, 0.5 × `top` at
/// the top (0 makes a cone).
fn frustum(b: &mut Builder, n: usize, top: f32) {
    let ring = |r: f32, y: f32| -> Vec<V3> {
        (0..n)
            .map(|i| {
                let a = (i as f32 + 0.5) / n as f32 * TAU;
                [r * a.cos(), y, r * a.sin()]
            })
            .collect()
    };
    let (lo, hi) = (ring(0.5, 0.0), ring(0.5 * top, 1.0));
    for i in 0..n {
        let j = (i + 1) % n;
        let out = [
            f32::midpoint(lo[i][0], lo[j][0]),
            0.0,
            f32::midpoint(lo[i][2], lo[j][2]),
        ];
        if top > 0.0 {
            b.poly(&[lo[i], lo[j], hi[j], hi[i]], Some(out));
        } else {
            b.poly(&[lo[i], lo[j], hi[i]], Some(out));
        }
    }
    b.poly(&lo, Some([0.0, -1.0, 0.0]));
    if top > 0.0 {
        b.poly(&hi, Some([0.0, 1.0, 0.0]));
    }
}

/// A faceted ball of radius 0.5 centred at y = 0.5.
fn ball(b: &mut Builder, lon: usize, lat: usize) {
    let p = |i: usize, j: usize| -> V3 {
        let th = j as f32 / lat as f32 * PI;
        let ph = i as f32 / lon as f32 * TAU;
        [
            0.5 * th.sin() * ph.cos(),
            0.5 + 0.5 * th.cos(),
            0.5 * th.sin() * ph.sin(),
        ]
    };
    for j in 0..lat {
        for i in 0..lon {
            let (a, bb, c, d) = (p(i, j), p(i + 1, j), p(i + 1, j + 1), p(i, j + 1));
            let out = [
                f32::midpoint(a[0], c[0]),
                f32::midpoint(a[1], c[1]) - 0.5,
                f32::midpoint(a[2], c[2]),
            ];
            if j == 0 {
                b.poly(&[a, c, d], Some(out));
            } else if j == lat - 1 {
                b.poly(&[a, bb, c], Some(out));
            } else {
                b.poly(&[a, bb, c, d], Some(out));
            }
        }
    }
}

/// A leaf along +z (0–1): pointed, widest at `wide` of its length, folded
/// along the midrib by `fold`.
fn leaf(b: &mut Builder, wide: f32, fold: f32) {
    let steps = 5;
    let at = |t: f32| -> (V3, V3, V3) {
        let w = if t < wide {
            (t / wide).powf(0.6)
        } else {
            ((1.0 - t) / (1.0 - wide)).powf(0.8)
        } * 0.5;
        let y = -0.18 * t * t;
        ([0.0, y, t], [-w, y - fold * w, t], [w, y - fold * w, t])
    };
    for s in 0..steps {
        let (t0, t1) = (s as f32 / steps as f32, (s + 1) as f32 / steps as f32);
        let ((m0, l0, r0), (m1, l1, r1)) = (at(t0), at(t1));
        b.tri2(m0, l0, l1);
        b.tri2(m0, l1, m1);
        b.tri2(m0, m1, r1);
        b.tri2(m0, r1, r0);
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
        let tip = |s: f32| -> V3 { [s * w, p0[1] - 0.05, f32::midpoint(p0[2], p1[2]) + 0.06] };
        b.tri2(p0, p1, tip(-1.0));
        b.tri2(p0, tip(1.0), p1);
    }
}

#[cfg(test)]
mod tests {
    use super::{Library, Mesh};

    #[test]
    fn every_shape_has_triangles_inside_its_unit_box() {
        let lib = Library::build();
        assert_eq!(lib.ranges.len(), Mesh::ALL.len());
        for m in Mesh::ALL {
            let (_, n) = lib.range(m);
            assert!(n >= 3 && n % 3 == 0, "{m:?}: {n}");
        }
        for v in &lib.verts {
            assert!(v.pos.iter().all(|c| (-1.0..=1.1).contains(c)), "{v:?}");
            assert!(v.n.iter().all(|c| c.is_finite()), "{v:?}");
            let len = (v.n[0] * v.n[0] + v.n[1] * v.n[1] + v.n[2] * v.n[2]).sqrt();
            assert!((len - 1.0).abs() < 1e-3);
        }
        assert!(lib.idx.iter().all(|i| (*i as usize) < lib.verts.len()));
    }
}

#[cfg(test)]
mod smooth_tests {
    use super::{Library, Mesh, Vertex};
    use crate::math::{cross, dot, sub};

    /// Every triangle of a smooth shape faces the way its vertices'
    /// normals do (so back-face culling keeps the outside).
    #[test]
    fn smooth_shapes_face_outwards() {
        let lib = Library::build();
        for m in [
            Mesh::Sphere,
            Mesh::Capsule,
            Mesh::RoundBox,
            Mesh::CylS,
            Mesh::Torso,
        ] {
            let (s, n) = lib.range(m);
            assert!(n >= 200, "{m:?} has {n} indices");
            for t in lib.idx[s as usize..(s + n) as usize].chunks(3) {
                let v: Vec<Vertex> = t.iter().map(|&i| lib.verts[i as usize]).collect();
                let facet = cross(sub(v[1].pos, v[0].pos), sub(v[2].pos, v[0].pos));
                let avg = [
                    v[0].n[0] + v[1].n[0] + v[2].n[0],
                    v[0].n[1] + v[1].n[1] + v[2].n[1],
                    v[0].n[2] + v[1].n[2] + v[2].n[2],
                ];
                assert!(dot(facet, avg) >= 0.0, "{m:?}");
                // and outward from the shape's middle
                let c = [
                    (v[0].pos[0] + v[1].pos[0] + v[2].pos[0]) / 3.0,
                    (v[0].pos[1] + v[1].pos[1] + v[2].pos[1]) / 3.0 - 0.5,
                    (v[0].pos[2] + v[1].pos[2] + v[2].pos[2]) / 3.0,
                ];
                if m != Mesh::Torso {
                    assert!(dot(avg, c) > -1e-3, "{m:?} points in at {c:?}");
                }
            }
        }
    }
}
