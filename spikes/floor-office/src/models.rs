//! The models, built from the mesh library's parts: desks, chairs, plants, screens, desk props,
//! furniture and people. Every model is drawn in its own frame (standing on y=0, its user facing
//! -z) and placed by the caller's transform. Parts are low-poly but shaped: a plant has a pot,
//! soil, stems and leaves; a task chair has a five-star base, a gas lift, a seat, a back and arms.

use crate::kit::{ChairKind, DeskKind, Plant, Pot, Screens};
use crate::math::{T, V3};
use crate::mesh::Mesh;
use crate::paint::{hex, mix, ramp, Rgba, Rng};
use std::f32::consts::{FRAC_PI_2, PI, TAU};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Inst { pub m: [[f32; 4]; 4], pub col: [f32; 4], pub tex: [f32; 4], pub misc: [f32; 4], pub off: [f32; 2], pub id: u32, pub _p: u32 }

pub const TEXTURED: u32 = 1;
pub const CUTAWAY: u32 = 2;

#[derive(Clone, Copy)]
pub struct Item { pub mesh: Mesh, pub floor: i32, pub glass: bool, pub inst: Inst }

/// Collects parts for one floor and one pickable thing (`id`).
pub struct B<'a> { pub out: &'a mut Vec<Item>, pub floor: i32, pub id: u32, pub dim: f32 }

pub fn rgba(c: Rgba) -> [f32; 4] { [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0, c[3] as f32 / 255.0] }

impl B<'_> {
    pub fn put(&mut self, mesh: Mesh, t: T, c: Rgba) { self.raw(mesh, t, c, [0.0; 4], [1.0, 1.0], 0.0, 0, false); }
    pub fn glow(&mut self, mesh: Mesh, t: T, c: Rgba, e: f32) { self.raw(mesh, t, c, [0.0; 4], [1.0, 1.0], e, 0, false); }
    pub fn tex(&mut self, mesh: Mesh, t: T, c: Rgba, tex: [f32; 4], rep: [f32; 2], e: f32, flags: u32) { self.raw(mesh, t, c, tex, rep, e, flags | TEXTURED, false); }
    pub fn glass(&mut self, mesh: Mesh, t: T, c: Rgba, flags: u32) { self.raw(mesh, t, c, [0.0; 4], [1.0, 1.0], 0.0, flags, true); }
    #[allow(clippy::too_many_arguments)]
    pub fn raw(&mut self, mesh: Mesh, t: T, c: Rgba, tex: [f32; 4], rep: [f32; 2], e: f32, flags: u32, glass: bool) {
        let mut col = rgba(c);
        if self.dim < 1.0 && e < 0.5 { for k in 0..3 { col[k] *= self.dim; } }
        self.out.push(Item { mesh, floor: self.floor, glass, inst: Inst { m: t.0 .0, col, tex, misc: [rep[0], rep[1], e, flags as f32], off: [0.0; 2], id: self.id, _p: 0 } });
    }
    /// A box from its minimum corner and size, in the frame `t`.
    pub fn bx(&mut self, t: T, lo: V3, size: V3, c: Rgba) {
        self.put(Mesh::Box, t.t(lo[0] + size[0] / 2.0, lo[1], lo[2] + size[2] / 2.0).s(size[0], size[1], size[2]), c);
    }
    pub fn cb(&mut self, t: T, lo: V3, size: V3, c: Rgba) {
        self.put(Mesh::Chamfer, t.t(lo[0] + size[0] / 2.0, lo[1], lo[2] + size[2] / 2.0).s(size[0], size[1], size[2]), c);
    }
}

// ---------- desks ----------
pub const DESK_H: f32 = 0.74;

/// A desk centred on the frame's origin, its sitter on +z facing -z. Returns the top height.
pub fn desk(b: &mut B, t: T, kind: DeskKind, top: Rgba, frame: Rgba, w: f32) -> f32 {
    let d = 0.72;
    let edge = ramp(top, -0.18);
    match kind {
        DeskKind::FourLeg => {
            b.bx(t, [-w / 2.0, DESK_H - 0.035, -d / 2.0], [w, 0.035, d], top);
            b.bx(t, [-w / 2.0, DESK_H - 0.045, -d / 2.0], [w, 0.012, d], edge);
            for (x, z) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                b.bx(t, [x * (w / 2.0 - 0.06) - 0.025, 0.0, z * (d / 2.0 - 0.06) - 0.025], [0.05, DESK_H - 0.04, 0.05], frame);
            }
            b.bx(t, [-w / 2.0 + 0.06, DESK_H - 0.12, -d / 2.0 + 0.04], [w - 0.12, 0.07, 0.025], frame);
        }
        DeskKind::Trestle => {
            b.bx(t, [-w / 2.0, DESK_H - 0.04, -d / 2.0], [w, 0.04, d], top);
            for x in [-w / 2.0 + 0.18, w / 2.0 - 0.18] {
                for s in [-1.0f32, 1.0] {
                    b.put(Mesh::Box, t.t(x, 0.0, s * 0.2).rx(s * 0.38).s(0.06, DESK_H / 0.93, 0.06), frame);
                }
                b.bx(t, [x - 0.03, 0.32, -0.2], [0.06, 0.05, 0.4], frame);
            }
            b.bx(t, [-w / 2.0 + 0.18, 0.34, -0.03], [w - 0.36, 0.04, 0.06], frame);
        }
        DeskKind::Pedestal => {
            b.bx(t, [-w / 2.0, DESK_H - 0.035, -d / 2.0], [w, 0.035, d], top);
            b.bx(t, [-w / 2.0 + 0.03, 0.0, -d / 2.0 + 0.03], [0.04, DESK_H - 0.035, d - 0.06], frame);
            let px = w / 2.0 - 0.45;
            b.bx(t, [px, 0.02, -d / 2.0 + 0.04], [0.42, DESK_H - 0.06, d - 0.1], frame);
            for k in 0..3 {
                b.bx(t, [px + 0.02, 0.06 + k as f32 * 0.22, d / 2.0 - 0.065], [0.38, 0.19, 0.012], ramp(frame, 0.15));
                b.bx(t, [px + 0.15, 0.2 + k as f32 * 0.22, d / 2.0 - 0.055], [0.12, 0.015, 0.02], hex("#c8c8cc"));
            }
            b.bx(t, [-w / 2.0 + 0.05, 0.25, -d / 2.0 + 0.03], [w - 0.5, 0.42, 0.02], ramp(frame, 0.08));
        }
        DeskKind::Standing => {
            let h = 1.04;
            b.bx(t, [-w / 2.0, h - 0.035, -d / 2.0], [w, 0.035, d], top);
            for x in [-w / 2.0 + 0.12, w / 2.0 - 0.12] {
                b.bx(t, [x - 0.04, 0.0, -0.32], [0.08, 0.04, 0.64], frame);
                b.bx(t, [x - 0.035, 0.04, -0.035], [0.07, h - 0.075, 0.07], frame);
                b.bx(t, [x - 0.025, 0.4, -0.025], [0.05, h - 0.45, 0.05], ramp(frame, 0.2));
            }
            b.bx(t, [-w / 2.0 + 0.12, h - 0.12, -0.03], [w - 0.24, 0.05, 0.06], frame);
            return h;
        }
        DeskKind::LShape => {
            b.bx(t, [-w / 2.0, DESK_H - 0.035, -d / 2.0], [w, 0.035, d], top);
            b.bx(t, [w / 2.0 - 0.6, DESK_H - 0.035, d / 2.0], [0.6, 0.035, 0.55], top);
            b.bx(t, [-w / 2.0 + 0.02, 0.0, -d / 2.0 + 0.02], [0.04, DESK_H - 0.035, d - 0.04], frame);
            b.bx(t, [w / 2.0 - 0.06, 0.0, -d / 2.0 + 0.02], [0.04, DESK_H - 0.035, d + 0.5], frame);
            b.bx(t, [-w / 2.0 + 0.06, 0.3, -d / 2.0 + 0.02], [w - 0.12, 0.38, 0.02], ramp(frame, 0.1));
        }
        DeskKind::Round => {
            b.put(Mesh::Cyl12, t.t(0.0, DESK_H - 0.035, 0.0).s(1.0, 0.035, 0.9), top);
            b.put(Mesh::Cyl12, t.t(0.0, DESK_H - 0.045, 0.0).s(0.98, 0.012, 0.88), edge);
            b.put(Mesh::Cyl8, t.t(0.0, 0.03, 0.0).s(0.08, DESK_H - 0.07, 0.08), frame);
            b.put(Mesh::Cyl8, t.s(0.5, 0.03, 0.5), frame);
        }
    }
    DESK_H
}

/// A long shared table for `seats` per side, centred on the frame.
pub fn long_table(b: &mut B, t: T, len: f32, top: Rgba, frame: Rgba) {
    let d = 1.05;
    b.bx(t, [-len / 2.0, DESK_H - 0.045, -d / 2.0], [len, 0.045, d], top);
    b.bx(t, [-len / 2.0, DESK_H - 0.055, -d / 2.0], [len, 0.012, d], ramp(top, -0.2));
    for x in [-len / 2.0 + 0.25, len / 2.0 - 0.25] {
        b.bx(t, [x - 0.04, 0.0, -0.4], [0.08, 0.04, 0.8], frame);
        b.bx(t, [x - 0.03, 0.0, -0.03], [0.06, DESK_H - 0.05, 0.06], frame);
        b.bx(t, [x - 0.03, DESK_H - 0.1, -0.4], [0.06, 0.05, 0.8], frame);
    }
    b.bx(t, [-len / 2.0 + 0.3, DESK_H - 0.12, -0.02], [len - 0.6, 0.06, 0.04], frame);
    // a cable spine with a plant or two
    b.bx(t, [-len / 2.0 + 0.4, DESK_H, -0.05], [len - 0.8, 0.06, 0.1], ramp(frame, 0.25));
}

// ---------- chairs ----------
pub const SEAT_H: f32 = 0.48;

/// A chair whose sitter faces -z; returns the seat height. `out` pushes it back (empty chair).
pub fn chair(b: &mut B, t: T, kind: ChairKind, c: Rgba, high: bool) -> f32 {
    let dark = hex("#2a2b30");
    let metal = hex("#9a9ca4");
    match kind {
        ChairKind::Task | ChairKind::Mesh => {
            for k in 0..5 {
                let a = k as f32 / 5.0 * TAU;
                b.put(Mesh::Box, t.ry(a).t(0.0, 0.03, 0.14).s(0.045, 0.035, 0.28), dark);
                b.put(Mesh::BallLo, t.ry(a).t(0.0, 0.0, 0.27).s(0.05, 0.05, 0.05), dark);
            }
            b.put(Mesh::Cyl8, t.t(0.0, 0.05, 0.0).s(0.06, 0.36, 0.06), metal);
            b.cb(t, [-0.24, 0.42, -0.24], [0.48, 0.08, 0.46], c);
            let back = if kind == ChairKind::Mesh { hex("#3a3c44") } else { c };
            b.put(Mesh::Chamfer, t.t(0.0, 0.52, 0.24).rx(-0.12).s(0.44, 0.5, 0.07), back);
            if kind == ChairKind::Mesh {
                b.put(Mesh::Box, t.t(0.0, 0.58, 0.215).rx(-0.12).s(0.36, 0.38, 0.02), hex("#55575f"));
                b.put(Mesh::Chamfer, t.t(0.0, 1.06, 0.31).rx(-0.12).s(0.3, 0.12, 0.06), back);
            }
            b.put(Mesh::Box, t.t(0.0, 0.45, 0.24).s(0.05, 0.12, 0.04), metal);
            for s in [-1.0f32, 1.0] {
                b.bx(t, [s * 0.25 - 0.02, 0.48, -0.02], [0.04, 0.2, 0.04], dark);
                b.bx(t, [s * 0.25 - 0.035, 0.67, -0.15], [0.07, 0.03, 0.26], dark);
            }
            SEAT_H
        }
        ChairKind::Stool => {
            let h = if high { 0.72 } else { 0.46 };
            b.put(Mesh::Cyl12, t.t(0.0, h - 0.05, 0.0).s(0.38, 0.05, 0.38), c);
            for k in 0..4 {
                let a = k as f32 / 4.0 * TAU + 0.785;
                b.put(Mesh::Box, t.ry(a).t(0.0, 0.0, 0.16).rx(0.12).s(0.035, h - 0.04, 0.035), hex("#7a5a3a"));
                b.put(Mesh::Box, t.ry(a).t(0.0, h * 0.35, 0.095).s(0.2, 0.025, 0.025), hex("#7a5a3a"));
            }
            h
        }
        ChairKind::Saddle => {
            let h = if high { 0.74 } else { 0.55 };
            b.put(Mesh::Cyl8, t.s(0.44, 0.03, 0.44), dark);
            b.put(Mesh::Cyl8, t.t(0.0, 0.03, 0.0).s(0.06, h - 0.12, 0.06), metal);
            b.put(Mesh::Ball, t.t(0.0, h - 0.12, 0.0).s(0.4, 0.14, 0.34), c);
            h
        }
        ChairKind::Ball => {
            b.put(Mesh::Ball, t.s(0.6, 0.55, 0.6), c);
            b.put(Mesh::Ball, t.t(0.0, 0.22, 0.0).s(0.62, 0.06, 0.62), ramp(c, -0.15));
            0.5
        }
    }
}

// ---------- screens ----------
/// Screens on a desk top at `top`; `tex` is the picture (already chosen by state).
pub fn screens(b: &mut B, t: T, kind: Screens, top: f32, tex: [f32; 4], glow: f32, scroll: f32) {
    let dark = hex("#1d1e23");
    // a code picture is twice as tall as the screen and scrolls; `scroll` < 0 shows a still picture
    let mut r = tex;
    if scroll >= 0.0 { r[1] += scroll.fract() * r[3] * 0.5; r[3] *= 0.5; }
    let pic = |b: &mut B, t: T, w: f32, h: f32| {
        b.put(Mesh::Box, t.s(w + 0.03, h + 0.03, 0.03), dark);
        b.tex(Mesh::QuadWall, t.t(0.0, 0.015, 0.016).s(w, h, 1.0), [255, 255, 255, 255], r, [1.0, 1.0], glow, 0);
    };
    match kind {
        Screens::Single | Screens::Wide => {
            let w = if kind == Screens::Wide { 0.86 } else { 0.56 };
            b.put(Mesh::Box, t.t(0.0, top, -0.18).s(0.22, 0.015, 0.16), dark);
            b.put(Mesh::Box, t.t(0.0, top, -0.2).s(0.05, 0.2, 0.03), dark);
            pic(b, t.t(0.0, top + 0.14, -0.2).rx(-0.08), w, 0.33);
        }
        Screens::Dual => {
            for s in [-1.0f32, 1.0] {
                let tt = t.t(s * 0.29, 0.0, -0.18).ry(-s * 0.22);
                b.put(Mesh::Box, tt.t(0.0, top, 0.0).s(0.18, 0.015, 0.14), dark);
                b.put(Mesh::Box, tt.t(0.0, top, -0.02).s(0.04, 0.18, 0.03), dark);
                pic(b, tt.t(0.0, top + 0.13, -0.02).rx(-0.06), 0.5, 0.3);
            }
        }
        Screens::Laptop => {
            b.put(Mesh::Box, t.t(0.0, top, 0.02).s(0.34, 0.018, 0.23), hex("#b9bcc4"));
            b.put(Mesh::Box, t.t(0.0, top + 0.018, 0.03).s(0.28, 0.003, 0.11), hex("#3a3c44"));
            let lid = t.t(0.0, top + 0.012, -0.09).rx(-0.25);
            b.put(Mesh::Box, lid.s(0.34, 0.23, 0.012), hex("#b9bcc4"));
            b.tex(Mesh::QuadWall, lid.t(0.0, 0.015, 0.007).s(0.3, 0.2, 1.0), [255, 255, 255, 255], r, [1.0, 1.0], glow, 0);
            return;
        }
    }
    b.put(Mesh::Box, t.t(0.0, top, 0.08).s(0.42, 0.02, 0.14), hex("#2e2f35"));
    b.put(Mesh::Box, t.t(0.0, top + 0.02, 0.08).s(0.38, 0.006, 0.11), hex("#4a4c55"));
    b.put(Mesh::Chamfer, t.t(0.3, top, 0.1).s(0.06, 0.025, 0.1), hex("#2e2f35"));
}

/// Personal things on a desk: never the same set twice.
pub fn desk_props(b: &mut B, t: T, top: f32, r: &mut Rng, accent: Rgba, lamp: bool, w: f32) -> Option<V3> {
    let mut light = None;
    let side = if r.chance(0.5) { -1.0 } else { 1.0 };
    let x0 = side * (w / 2.0 - 0.18);
    // a mug
    if r.chance(0.75) {
        let c = hex(r.pick(&["#f4efe6", "#c86f4f", "#3a5a7a", "#e8b33a", "#2f2f3a"]));
        let m = t.t(-x0 * 0.55, top, 0.12 + r.range(-0.05, 0.05));
        b.put(Mesh::Cyl8, m.s(0.08, 0.1, 0.08), c);
        b.put(Mesh::Box, m.t(0.05, 0.03, 0.0).s(0.03, 0.05, 0.015), c);
        b.put(Mesh::Cyl8, m.t(0.0, 0.09, 0.0).s(0.065, 0.005, 0.065), hex("#4a2a1a"));
    }
    match r.int(5) {
        0 => {
            // a small succulent
            b.put(Mesh::Cyl8, t.t(x0, top, -0.05).s(0.09, 0.08, 0.09), hex("#e8e2d6"));
            for k in 0..6 { b.put(Mesh::Leaf, t.t(x0, top + 0.08, -0.05).ry(k as f32 * 1.05).rx(-0.7).s(0.06, 0.06, 0.07), hex("#6fae7a")); }
        }
        1 => {
            // a stack of books
            for k in 0..3 { b.put(Mesh::Box, t.t(x0, top + k as f32 * 0.035, -0.08).ry(r.range(-0.3, 0.3)).s(0.2, 0.033, 0.15), hex(r.pick(&["#a4543a", "#3a5a7a", "#d9b44a", "#4a7a5a"]))); }
        }
        2 => {
            // a framed photo
            b.put(Mesh::Box, t.t(x0, top, -0.1).ry(side * 0.4).rx(-0.2).s(0.12, 0.15, 0.015), hex("#2a2530"));
            b.put(Mesh::Box, t.t(x0, top + 0.02, -0.092).ry(side * 0.4).rx(-0.2).s(0.09, 0.1, 0.01), mix(accent, hex("#f2e8d5"), 0.5));
        }
        3 => {
            // sticky notes and a notebook
            b.put(Mesh::Box, t.t(x0, top, 0.05).ry(r.range(-0.4, 0.4)).s(0.16, 0.012, 0.22), hex(r.pick(&["#3d405b", "#81b29a", "#e07a5f"])));
            for k in 0..2 { b.put(Mesh::Box, t.t(x0 - 0.05 + k as f32 * 0.07, top + 0.013, -0.12).s(0.06, 0.004, 0.06), hex(r.pick(&["#ffe27a", "#ffb3c7", "#a8e6cf"]))); }
        }
        _ => {
            // a water bottle and headphones
            b.put(Mesh::Cyl8, t.t(x0, top, -0.08).s(0.07, 0.22, 0.07), hex(r.pick(&["#5aa0c8", "#e0e0e0", "#4a7a5a"])));
            b.put(Mesh::Cyl8, t.t(x0, top + 0.22, -0.08).s(0.05, 0.03, 0.05), hex("#2a2b30"));
            b.put(Mesh::Box, t.t(-x0 * 0.2, top, 0.18).ry(0.4).s(0.16, 0.03, 0.05), hex("#2a2b30"));
        }
    }
    if lamp {
        let l = t.t(-x0, top, -0.22);
        b.put(Mesh::Cyl8, l.s(0.12, 0.02, 0.12), hex("#2a2b30"));
        b.put(Mesh::Box, l.t(0.0, 0.02, 0.0).rx(0.35).s(0.02, 0.3, 0.02), hex("#2a2b30"));
        let head = l.t(0.0, 0.3, -0.1);
        b.put(Mesh::Box, head.rx(-1.1).s(0.02, 0.22, 0.02), hex("#2a2b30"));
        let shade = l.t(0.0, 0.42, 0.08);
        b.put(Mesh::Cone8, shade.rx(PI * 0.82).s(0.14, 0.12, 0.14), hex(r.pick(&["#e8b33a", "#c86f4f", "#2f4a6b", "#e6e1d8"])));
        b.glow(Mesh::Ball, shade.t(0.0, -0.06, -0.02).s(0.05, 0.05, 0.05), hex("#fff2c0"), 1.0);
        light = Some(shade.0.point([0.0, -0.1, 0.0]));
    }
    light
}

// ---------- plants ----------
pub fn pot(b: &mut B, t: T, kind: Pot, s: f32) -> f32 {
    let h = 0.42 * s;
    match kind {
        Pot::Terracotta => {
            b.put(Mesh::Pot, t.s(0.36 * s, h, 0.36 * s), hex("#b5653d"));
            b.put(Mesh::Cyl8, t.t(0.0, h - 0.06 * s, 0.0).s(0.48 * s, 0.07 * s, 0.48 * s), hex("#c47a52"));
        }
        Pot::Ceramic(c) => {
            b.put(Mesh::Cyl12, t.s(0.42 * s, h, 0.42 * s), c);
            b.put(Mesh::Cyl12, t.t(0.0, h * 0.15, 0.0).s(0.43 * s, 0.03 * s, 0.43 * s), ramp(c, -0.2));
        }
        Pot::Box(c) => {
            b.bx(t, [-0.22 * s, 0.0, -0.22 * s], [0.44 * s, h, 0.44 * s], c);
            b.bx(t, [-0.24 * s, h - 0.04 * s, -0.24 * s], [0.48 * s, 0.04 * s, 0.48 * s], ramp(c, -0.15));
        }
        Pot::Basket => {
            b.put(Mesh::Cyl8, t.s(0.44 * s, h, 0.44 * s), hex("#b08a5a"));
            for k in 0..3 { b.put(Mesh::Cyl8, t.t(0.0, (0.08 + k as f32 * 0.12) * s, 0.0).s(0.45 * s, 0.03 * s, 0.45 * s), hex("#8a6a42")); }
        }
        Pot::Concrete => {
            b.put(Mesh::Cyl8, t.s(0.44 * s, h, 0.44 * s), hex("#9a9a98"));
            b.put(Mesh::Cyl8, t.t(0.0, h - 0.02, 0.0).s(0.36 * s, 0.025, 0.36 * s), hex("#7f7f7d"));
        }
    }
    b.put(Mesh::Cyl8, t.t(0.0, h - 0.03 * s, 0.0).s(0.34 * s, 0.03, 0.34 * s), hex("#4a3426"));
    h
}

/// A potted plant. `sway` (radians) moves the leaves when it is touched; `wilt` 0..1 browns it.
pub fn plant(b: &mut B, t: T, kind: Plant, pot_kind: Pot, s: f32, seed: u32, sway: f32, wilt: f32) {
    let mut r = Rng(seed | 1);
    let green = |r: &mut Rng, c: &str| { let base = ramp(hex(c), r.range(-0.12, 0.12)); mix(base, hex("#9a7a3a"), wilt) };
    let top = pot(b, t, pot_kind, if kind == Plant::Succulent || kind == Plant::Herbs { s * 0.6 } else { s });
    let tt = t.t(0.0, top, 0.0);
    match kind {
        Plant::Monstera => {
            for k in 0..8 {
                let a = k as f32 / 8.0 * TAU + r.range(-0.3, 0.3);
                let up = r.range(0.25, 0.7) * s;
                let stem = tt.ry(a).rx(0.35 - sway * 0.5);
                b.put(Mesh::Cyl6, stem.s(0.02, up + 0.15 * s, 0.02), green(&mut r, "#4a7a3a"));
                b.put(Mesh::Leaf, stem.t(0.0, up + 0.12 * s, 0.0).rx(-0.5 - wilt * 0.8 + r.range(-0.2, 0.2) + sway).s(0.42 * s, 0.4 * s, 0.5 * s), green(&mut r, "#2f7a3e"));
            }
        }
        Plant::Fern => {
            for k in 0..14 {
                let a = k as f32 / 14.0 * TAU + r.range(-0.2, 0.2);
                b.put(Mesh::Frond, tt.t(0.0, 0.02, 0.0).ry(a).rx(-0.6 + r.range(-0.3, 0.2) + sway + wilt * 0.6).s(0.4 * s, 0.5 * s, 0.6 * s), green(&mut r, "#4f9a3e"));
            }
        }
        Plant::Snake => {
            for k in 0..9 {
                let a = k as f32 / 9.0 * TAU + r.range(-0.4, 0.4);
                let h = r.range(0.55, 0.95) * s;
                let c = green(&mut r, if k % 3 == 0 { "#5f7f3a" } else { "#3f6e3a" });
                b.put(Mesh::Leaf, tt.ry(a).t(0.0, 0.0, 0.06 * s).rx(-1.4 + r.range(-0.1, 0.15) + sway * 0.4 + wilt).s(0.12 * s, 0.1, h), c);
            }
        }
        Plant::Cactus => {
            let g = green(&mut r, "#4f8a4a");
            b.put(Mesh::Cyl8, tt.t(0.0, 0.0, 0.0).rz(sway * 0.2).s(0.2 * s, 0.75 * s, 0.2 * s), g);
            b.put(Mesh::Ball, tt.t(0.0, 0.66 * s, 0.0).s(0.2 * s, 0.16 * s, 0.2 * s), g);
            for side in [-1.0f32, 1.0] {
                let arm = tt.t(side * 0.09 * s, (0.3 + 0.1 * side) * s, 0.0);
                b.put(Mesh::Cyl8, arm.rz(-side * FRAC_PI_2).s(0.11 * s, 0.14 * s, 0.11 * s), g);
                b.put(Mesh::Cyl8, arm.t(side * 0.14 * s, 0.0, 0.0).s(0.11 * s, 0.25 * s, 0.11 * s), g);
            }
            b.put(Mesh::Ball, tt.t(0.0, 0.78 * s, 0.0).s(0.08 * s, 0.06 * s, 0.08 * s), hex("#ff7aa8"));
        }
        Plant::Fig => {
            let trunk = tt.rz(sway * 0.15);
            b.put(Mesh::Cyl6, trunk.s(0.06 * s, 1.1 * s, 0.06 * s), hex("#6b4a32"));
            for k in 0..5 {
                let a = k as f32 * 1.3;
                let p = trunk.t(0.18 * s * a.cos(), (0.95 + 0.25 * (k % 3) as f32) * s, 0.18 * s * a.sin());
                b.put(Mesh::BallLo, p.ry(a).s(0.42 * s, 0.34 * s, 0.42 * s), green(&mut r, "#2f6a34"));
                for j in 0..3 { b.put(Mesh::Leaf, p.t(0.0, 0.2 * s, 0.0).ry(a + j as f32 * 2.1).rx(-0.2 + sway).s(0.16 * s, 0.1, 0.24 * s), green(&mut r, "#3f8a44")); }
            }
        }
        Plant::Palm => {
            let mut p = tt;
            for k in 0..4 {
                b.put(Mesh::Cyl6, p.s((0.1 - k as f32 * 0.012) * s, 0.3 * s, (0.1 - k as f32 * 0.012) * s), hex(if k % 2 == 0 { "#8a6a42" } else { "#7a5a3a" }));
                p = p.t(0.0, 0.28 * s, 0.0).rz(0.06 + sway * 0.1);
            }
            for k in 0..9 {
                let a = k as f32 / 9.0 * TAU;
                b.put(Mesh::Frond, p.ry(a).rx(-0.15 + sway + wilt * 0.5).s(0.5 * s, 0.6 * s, 0.85 * s), green(&mut r, "#4a8a3a"));
            }
        }
        Plant::Herbs => {
            for k in 0..3 {
                let p = tt.t((k as f32 - 1.0) * 0.1 * s, 0.0, 0.0);
                for j in 0..5 { b.put(Mesh::Leaf, p.ry(j as f32 * 1.26 + k as f32).rx(-1.0 + sway).s(0.1 * s, 0.1, 0.16 * s), green(&mut r, ["#5a9a4a", "#7aaa5a", "#4a8a5a"][k])); }
            }
        }
        Plant::Succulent => {
            for ring in 0..2 { for k in 0..7 {
                b.put(Mesh::Leaf, tt.ry(k as f32 * 0.9 + ring as f32 * 0.45).rx(-0.9 + ring as f32 * 0.5 + sway).s(0.1 * s, 0.08, (0.14 - ring as f32 * 0.04) * s), green(&mut r, if ring == 0 { "#7ab08a" } else { "#9ac8a0" }));
            } }
        }
    }
}

// ---------- people ----------
#[derive(Clone, Debug)]
pub struct Look { pub skin: Rgba, pub hair: Rgba, pub style: u8, pub top: Rgba, pub top_style: u8, pub legs: Rgba, pub shoes: Rgba, pub glasses: bool, pub beard: bool, pub height: f32, pub badge: Rgba }

impl Look {
    pub fn random(r: &mut Rng, badge: Rgba) -> Look {
        let skins = ["#f1d0b5", "#e6b896", "#c98d6a", "#a8704f", "#7a4a32", "#5a3524"];
        let hairs = ["#2a1d17", "#4a2f22", "#7a4a2a", "#b5824f", "#d8b46a", "#a8a8ad", "#c2502f", "#2f2f3a", "#e8e0d0"];
        let tops = ["#3d6fb0", "#c84a3a", "#3a8a5a", "#e8b33a", "#7a4ab0", "#2f2f3a", "#e6e1d8", "#d97aa0", "#4aa3a3", "#8a5a3a"];
        let legs = ["#2a3550", "#3a3a40", "#5a4a3a", "#1f1f24", "#4a5a7a", "#7a6a5a"];
        let style = r.int(9) as u8;
        Look {
            skin: hex(r.pick(&skins)), hair: hex(r.pick(&hairs)), style,
            top: hex(r.pick(&tops)), top_style: r.int(4) as u8, legs: hex(r.pick(&legs)), shoes: hex(r.pick(&["#1f1f24", "#e8e8e8", "#7a4a2a", "#c84a3a"])),
            glasses: r.chance(0.3), beard: style != 1 && r.chance(0.2), height: r.range(0.94, 1.06), badge,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Pose { Typing, Idle, Wave, Scratch, HandsOnHead, Stretch, Asleep, Helper, Walk, Stand }

/// A person standing on the frame's origin, facing -z (sitting poses sit at seat height `seat`).
/// `t` animates; `turn` turns the head (towards the viewer when hovered). Returns the point above
/// the head where a balloon goes.
pub fn person(b: &mut B, root: T, lk: &Look, pose: Pose, t: f32, seat: f32, turn: f32) -> V3 {
    let sit = matches!(pose, Pose::Typing | Pose::Idle | Pose::Scratch | Pose::Stretch | Pose::Asleep);
    let root = root.s(lk.height, lk.height, lk.height);
    let hip_y = if sit { seat / lk.height + 0.02 } else { 0.86 };
    let (mut thigh, mut knee) = (0.0f32, 0.0f32);
    let (mut lean, mut head_p, mut head_y) = (0.0f32, 0.0f32, turn);
    // arms: (shoulder pitch, shoulder roll, elbow) for left (-x) and right (+x)
    let (mut al, mut ar) = ((0.05f32, 0.08f32, 0.1f32), (0.05f32, 0.08f32, 0.1f32));
    let w = |f: f32| (t * f).sin();
    let mut walk = 0.0;
    if sit { thigh = FRAC_PI_2; knee = -FRAC_PI_2; }
    match pose {
        Pose::Typing => { lean = 0.12; al = (0.55, -0.05, 1.0 + 0.1 * w(14.0)); ar = (0.55, -0.05, 1.0 - 0.1 * w(14.0)); head_p = 0.1; }
        Pose::Idle => { lean = -0.18; al = (0.1, 0.1, 0.2); ar = (0.35, -0.15, 1.9 + 0.15 * w(1.3).max(0.0)); head_p = -0.05; }
        Pose::Scratch => { lean = 0.05; al = (0.5, -0.05, 1.0); ar = (0.4, 2.3, 1.6 + 0.25 * w(9.0)); head_p = 0.15; head_y += 0.2; }
        Pose::Stretch => { lean = -0.22; al = (0.0, 2.9, 0.2 + 0.1 * w(2.0)); ar = (0.0, 2.9, 0.2 + 0.1 * w(2.0)); head_p = -0.25; }
        Pose::Asleep => { lean = 0.75; al = (1.35, -0.25, 1.4); ar = (1.35, -0.25, 1.4); head_p = 0.55 + 0.03 * w(1.5); }
        Pose::Wave => { ar = (0.2, 2.55, 0.35 + 0.45 * w(9.0)); al = (0.05, 0.1, 0.15); head_p = -0.1; }
        Pose::HandsOnHead => { al = (0.6, 2.2, 1.8); ar = (0.6, 2.2, 1.8); head_p = 0.1 * w(6.0); }
        Pose::Helper => { al = (0.75, -0.25, 1.5); ar = (0.9 + 0.15 * w(3.0), 0.15, 0.4); head_p = 0.25; }
        Pose::Walk => { walk = 0.5 * w(7.0); al = (-walk, 0.08, 0.3); ar = (walk, 0.08, 0.3); }
        Pose::Stand => {}
    }
    // legs
    for (s, sw) in [(-1.0f32, walk), (1.0, -walk)] {
        let hip = root.t(s * 0.09, hip_y, 0.0).rx(thigh + sw);
        b.bx(hip, [-0.07, -0.42, -0.075], [0.14, 0.42, 0.15], lk.legs);
        let kn = hip.t(0.0, -0.4, 0.0).rx(knee - sw.max(0.0) * 0.6);
        b.bx(kn, [-0.06, -0.4, -0.065], [0.12, 0.4, 0.13], lk.legs);
        b.cb(kn.t(0.0, -0.44, 0.0), [-0.06, 0.0, -0.16], [0.12, 0.08, 0.24], lk.shoes);
    }
    // the upper body turns at the hips
    let up = root.t(0.0, hip_y, 0.0).rx(-lean).t(0.0, -0.86, 0.0);
    b.cb(up, [-0.17, 0.8, -0.11], [0.34, 0.16, 0.22], lk.legs);
    b.cb(up, [-0.19, 0.92, -0.12], [0.38, 0.46, 0.24], lk.top);
    match lk.top_style {
        1 => { b.put(Mesh::BallLo, up.t(0.0, 1.24, 0.12).s(0.3, 0.16, 0.12), ramp(lk.top, -0.12)); b.bx(up, [-0.1, 0.96, -0.125], [0.2, 0.08, 0.01], ramp(lk.top, -0.1)); }
        2 => { b.bx(up, [-0.1, 1.31, -0.125], [0.2, 0.05, 0.03], hex("#f4f1ea")); b.bx(up, [-0.012, 0.95, -0.124], [0.024, 0.36, 0.01], ramp(lk.top, -0.25)); }
        3 => { b.bx(up, [-0.191, 1.08, -0.122], [0.382, 0.06, 0.244], ramp(lk.top, 0.2)); }
        _ => {}
    }
    b.glow(Mesh::Box, up.t(0.1, 1.2, -0.125).s(0.06, 0.08, 0.012), lk.badge, 0.35);
    b.put(Mesh::Cyl6, up.t(0.0, 1.36, 0.0).s(0.1, 0.06, 0.1), lk.skin);
    // head
    let head = up.t(0.0, 1.4, 0.0).rx(-head_p).ry(head_y);
    b.put(Mesh::Ball, head.t(0.0, -0.01, 0.0).s(0.31, 0.32, 0.3), lk.skin);
    for s in [-1.0f32, 1.0] { b.bx(head, [s * 0.065 - 0.018, 0.15, -0.155], [0.036, 0.045, 0.02], hex("#1d1a24")); }
    b.bx(head, [-0.035, 0.06, -0.152], [0.07, 0.018, 0.02], ramp(lk.skin, -0.25));
    if lk.glasses { b.bx(head, [-0.13, 0.155, -0.172], [0.26, 0.035, 0.02], hex("#2a2530")); }
    if lk.beard { b.cb(head, [-0.12, -0.02, -0.15], [0.24, 0.12, 0.12], lk.hair); }
    let h = lk.hair;
    match lk.style {
        0 => { b.put(Mesh::Ball, head.t(0.0, 0.12, 0.02).s(0.33, 0.21, 0.32), h); }
        1 => { b.put(Mesh::Ball, head.t(0.0, 0.12, 0.02).s(0.33, 0.21, 0.32), h); b.cb(head, [-0.15, -0.16, 0.02], [0.3, 0.36, 0.13], h); }
        2 => { b.put(Mesh::Ball, head.t(0.0, 0.14, 0.02).s(0.32, 0.19, 0.31), h); b.put(Mesh::Ball, head.t(0.0, 0.27, 0.1).s(0.13, 0.13, 0.13), h); }
        3 => { for k in 0..7 { let a = k as f32 / 7.0 * TAU; b.put(Mesh::BallLo, head.t(0.11 * a.cos(), 0.2 + 0.03 * (k % 2) as f32, 0.11 * a.sin() + 0.02).s(0.15, 0.14, 0.15), h); } }
        4 => { b.put(Mesh::Ball, head.t(0.0, 0.12, 0.02).s(0.33, 0.21, 0.32), h); b.put(Mesh::Cyl6, head.t(0.0, 0.18, 0.16).rx(2.6 + 0.1 * w(4.0)).s(0.07, 0.24, 0.07), h); }
        5 => { b.put(Mesh::Ball, head.t(0.0, 0.16, 0.01).s(0.32, 0.17, 0.31), h); }
        6 => { b.put(Mesh::Ball, head.t(0.0, 0.11, 0.02).s(0.34, 0.22, 0.33), h); for s in [-1.0f32, 1.0] { b.cb(head, [s * 0.15 - 0.03, 0.0, -0.08], [0.06, 0.2, 0.2], h); } }
        7 => {}
        _ => { b.put(Mesh::Cyl8, head.t(0.0, 0.17, 0.01).s(0.34, 0.14, 0.33), ramp(lk.top, 0.15)); b.put(Mesh::BallLo, head.t(0.0, 0.3, 0.01).s(0.09, 0.09, 0.09), hex("#f4efe6")); }
    }
    // arms
    let long = lk.top_style != 0;
    for (s, (p, roll, el)) in [(-1.0f32, al), (1.0, ar)] {
        let sh = up.t(s * 0.23, 1.32, 0.0).rz(s * roll).rx(p);
        b.cb(sh, [-0.05, -0.3, -0.055], [0.1, 0.32, 0.11], lk.top);
        let fo = sh.t(0.0, -0.29, 0.0).rx(el);
        b.bx(fo, [-0.042, -0.27, -0.045], [0.084, 0.27, 0.09], if long { lk.top } else { lk.skin });
        b.cb(fo.t(0.0, -0.27, 0.0), [-0.04, -0.08, -0.04], [0.08, 0.09, 0.08], lk.skin);
        if pose == Pose::Idle && s > 0.0 {
            let mug = fo.t(0.0, -0.33, -0.04).rx(-el - p);
            b.put(Mesh::Cyl8, mug.s(0.08, 0.1, 0.08), hex("#f4efe6"));
        }
        if pose == Pose::Helper && s < 0.0 {
            b.put(Mesh::Box, fo.t(0.05, -0.3, -0.06).rx(-0.4).s(0.2, 0.26, 0.015), hex("#c9a46c"));
            b.put(Mesh::Box, fo.t(0.05, -0.18, -0.07).rx(-0.4).s(0.16, 0.18, 0.01), hex("#f4efe6"));
        }
    }
    head.0.point([0.0, 0.42, 0.0])
}

/// The office cat: walks, sits when clicked.
pub fn cat(b: &mut B, t: T, walk: f32, sitting: bool) {
    let c = hex("#e0913a");
    let d = hex("#b86a24");
    let body_y = if sitting { 0.16 } else { 0.18 };
    if sitting {
        b.put(Mesh::Ball, t.t(0.0, 0.0, 0.05).s(0.26, 0.34, 0.32), c);
    } else {
        b.cb(t, [-0.1, body_y, -0.2], [0.2, 0.16, 0.42], c);
        for (x, z, ph) in [(-0.06, -0.15, 0.0), (0.06, -0.15, PI), (-0.06, 0.15, PI), (0.06, 0.15, 0.0)] {
            b.put(Mesh::Box, t.t(x, 0.0, z).rx(0.4 * (walk + ph).sin()).s(0.05, body_y + 0.02, 0.05), d);
        }
    }
    let head = t.t(0.0, if sitting { 0.3 } else { 0.28 }, -0.24);
    b.put(Mesh::Ball, head.s(0.19, 0.17, 0.18), c);
    for s in [-1.0f32, 1.0] {
        b.put(Mesh::Cone8, head.t(s * 0.055, 0.14, 0.0).s(0.06, 0.08, 0.05), d);
        b.bx(head, [s * 0.045 - 0.012, 0.08, -0.092], [0.024, 0.024, 0.01], hex("#1d1a24"));
    }
    let mut tail = t.t(0.0, body_y + 0.12, 0.2);
    for k in 0..4 {
        tail = tail.rx(-0.35 - 0.1 * (walk * 0.5 + k as f32).sin());
        b.put(Mesh::Box, tail.s(0.04, 0.09, 0.04), if k % 2 == 0 { c } else { d });
        tail = tail.t(0.0, 0.08, 0.0);
    }
}
