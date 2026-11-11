//! The models, built from the mesh library's parts: desks, chairs,
//! screens, desk props, plants, people and the office cat. Every model is
//! drawn in its own frame (standing on y = 0, its user facing -z) and
//! placed by the caller's transform. Low-poly but shaped: a plant has a
//! pot, soil, stems and leaves; a task chair a five-star base, a gas lift,
//! a seat, a back and arms.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

mod stance;
use stance::stance;

use crate::colour::{Rgba, hex, mix, ramp};
use crate::kit::{ChairKind, DeskKind, Plant, Pot, Screens};
use crate::materials::{
    CERAMIC, CLOTH, DENIM, FABRIC, HAIR, LEAF, LEATHER, METAL, Mat, PAPER, PLASTER, PLASTIC,
    RUBBER, SKIN, SOIL, WOOD,
};
use crate::math::{T, V3};
use crate::mesh::Mesh;
use crate::rng::Rng;
use crate::tex::Uv;

/// One drawn part, as the GPU reads it: its transform, colour, texture
/// place, `(repeat u, repeat v, glow, flags)`, a texture scroll, its
/// pick id and its material ([`crate::materials`]; drawn only by 0.4.1's
/// second-pass look, the others ignore it).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Inst {
    /// The transform.
    pub m: [[f32; 4]; 4],
    /// The colour, 0–1.
    pub col: [f32; 4],
    /// Its texture place (unused when not textured).
    pub tex: [f32; 4],
    /// `(repeat u, repeat v, glow, flags)`.
    pub misc: [f32; 4],
    /// A texture offset (a scrolling screen).
    pub off: [f32; 2],
    /// What a click on it opens (0: nothing).
    pub id: u32,
    /// Its material (0: its flat colour).
    pub mat: u32,
}

/// A part is textured.
pub const TEXTURED: u32 = 1;
/// A part vanishes when it stands between the camera and the room.
pub const CUTAWAY: u32 = 2;
/// A part on a floor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Item {
    /// Its shape.
    pub mesh: Mesh,
    /// Its floor (-1: the street and the sky).
    pub floor: i32,
    /// Drawn in the glass pass.
    pub glass: bool,
    /// How it's drawn.
    pub inst: Inst,
}

/// Collects the parts of one pickable thing on one floor.
#[derive(Debug)]
pub struct B<'a> {
    /// Where parts go.
    pub out: &'a mut Vec<Item>,
    /// Their floor.
    pub floor: i32,
    /// What a click on them opens.
    pub id: u32,
    /// Dimmed (an unused App): 1 is full light.
    pub dim: f32,
}

/// A colour as 0–1 floats.
#[must_use]
pub fn rgba(c: Rgba) -> [f32; 4] {
    c.map(|v| f32::from(v) / 255.0)
}

/// A texture on a part: its place, how often it repeats, its glow and
/// flags.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Skin {
    /// The texture's place.
    pub uv: Uv,
    /// Repeats `(u, v)`.
    pub repeat: [f32; 2],
    /// Glow, 0–1.
    pub glow: f32,
    /// [`CUTAWAY`] and the like.
    pub flags: u32,
}

impl B<'_> {
    /// A plain part.
    pub fn put(&mut self, mesh: Mesh, t: T, c: Rgba) {
        self.raw(mesh, t, c, None, 0.0, 0, false);
    }

    /// A glowing part (a lamp, a badge).
    pub fn glow(&mut self, mesh: Mesh, t: T, c: Rgba, glow: f32) {
        self.raw(mesh, t, c, None, glow, 0, false);
    }

    /// A textured part.
    pub fn tex(&mut self, mesh: Mesh, t: T, c: Rgba, skin: Skin) {
        self.raw(
            mesh,
            t,
            c,
            Some((skin.uv, skin.repeat)),
            skin.glow,
            skin.flags | TEXTURED,
            false,
        );
    }

    /// A plain part that vanishes when it stands between the camera and
    /// the room (walls, mullions, sills).
    pub fn cut(&mut self, mesh: Mesh, t: T, c: Rgba) {
        self.raw(mesh, t, c, None, 0.0, CUTAWAY, false);
    }

    /// A glass part.
    pub fn glass(&mut self, mesh: Mesh, t: T, c: Rgba, flags: u32) {
        self.raw(mesh, t, c, None, 0.0, flags, true);
    }

    #[allow(clippy::too_many_arguments, reason = "every part kind is made here")]
    pub(crate) fn raw(
        &mut self,
        mesh: Mesh,
        t: T,
        c: Rgba,
        tex: Option<(Uv, [f32; 2])>,
        glow: f32,
        flags: u32,
        glass: bool,
    ) {
        let mut col = rgba(c);
        if self.dim < 1.0 && glow < 0.5 {
            for channel in &mut col[..3] {
                *channel *= self.dim;
            }
        }
        let (uv, repeat) = tex.unwrap_or(([0.0; 4], [1.0, 1.0]));
        let flags = if glass || glow >= 0.5 {
            flags
        } else {
            flags | (crate::glow::dust_of(self.dim) << crate::glow::DUST_SHIFT)
        };
        self.out.push(Item {
            mesh,
            floor: self.floor,
            glass,
            inst: Inst {
                m: t.0.0,
                col,
                tex: uv,
                misc: [repeat[0], repeat[1], glow, flags as f32],
                off: [0.0; 2],
                id: self.id,
                mat: 0,
            },
        });
    }

    /// Builds with `f`, giving every part it adds that has no material
    /// yet `mat` (an inner `with` wins).
    pub fn with<R>(&mut self, mat: Mat, f: impl FnOnce(&mut Self) -> R) -> R {
        let from = self.out.len();
        let r = f(self);
        for it in &mut self.out[from..] {
            if it.inst.mat == 0 {
                it.inst.mat = mat;
            }
        }
        r
    }

    /// A plain part of `mat`.
    pub fn put_m(&mut self, mesh: Mesh, t: T, c: Rgba, mat: Mat) {
        self.with(mat, |b| b.put(mesh, t, c));
    }

    /// A face on a head (`t` the head's ball), with `expr`: skin with
    /// eyes, brows and a mouth drawn by the second pass's shader.
    pub fn face(&mut self, t: T, c: Rgba, expr: crate::materials::Face) {
        self.put_m(Mesh::Sphere, t, c, crate::materials::FACE);
        if let Some(it) = self.out.last_mut() {
            let (tex, misc) = expr.pack();
            it.inst.tex = tex;
            it.inst.misc[0] = misc[0];
            it.inst.misc[1] = misc[1];
        }
    }

    /// A box from its lowest corner and its size, in the frame `t`.
    pub fn bx(&mut self, t: T, lo: V3, size: V3, c: Rgba) {
        self.cornered(Mesh::Box, t, lo, size, c);
    }

    /// A chamfered box from its lowest corner and its size.
    pub fn cb(&mut self, t: T, lo: V3, size: V3, c: Rgba) {
        self.cornered(Mesh::Chamfer, t, lo, size, c);
    }

    /// A unit `mesh` (centred in x and z, standing on y = 0) placed from
    /// its lowest corner and its size.
    fn cornered(&mut self, mesh: Mesh, t: T, lo: V3, size: V3, c: Rgba) {
        self.put(
            mesh,
            t.t(lo[0] + size[0] / 2.0, lo[1], lo[2] + size[2] / 2.0)
                .s(size[0], size[1], size[2]),
            c,
        );
    }
}

/// A desk's height, metres.
pub const DESK_H: f32 = 0.74;

/// A desk `w` wide, centred on the frame's origin, its sitter on +z
/// facing -z; its top's height.
pub fn desk(b: &mut B, t: T, kind: DeskKind, top: Rgba, frame: Rgba, w: f32) -> f32 {
    let tm = surface_of(top);
    b.with(METAL, |b| desk_parts(b, t, kind, (top, tm), frame, w))
}

/// The material a surface of colour `c` is made of: wood when its colour
/// is a wood's (warm, brownish, not pale), else laminate.
#[must_use]
pub fn surface_of(c: Rgba) -> Mat {
    let [r, g, bl, _] = rgba(c);
    let (hi, lo) = (r.max(g).max(bl), r.min(g).min(bl));
    let warm = r >= g && g >= bl;
    if warm && hi - lo > 0.18 && hi < 0.93 {
        WOOD
    } else {
        PLASTIC
    }
}

/// A desk's top slab, `w` wide and `d` deep, at desk height.
fn desk_top(b: &mut B, t: T, (top, tm): (Rgba, Mat), w: f32, d: f32) {
    b.with(tm, |b| {
        b.bx(t, [-w / 2.0, DESK_H - 0.035, -d / 2.0], [w, 0.035, d], top);
    });
}

#[allow(clippy::too_many_lines, reason = "a model a desk kind")]
fn desk_parts(b: &mut B, t: T, kind: DeskKind, (top, tm): (Rgba, Mat), frame: Rgba, w: f32) -> f32 {
    let d = 0.72;
    let edge = ramp(top, -0.18);
    match kind {
        DeskKind::FourLeg => {
            desk_top(b, t, (top, tm), w, d);
            b.with(tm, |b| {
                b.bx(t, [-w / 2.0, DESK_H - 0.045, -d / 2.0], [w, 0.012, d], edge);
            });
            for (x, z) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                b.bx(
                    t,
                    [
                        x * (w / 2.0 - 0.06) - 0.025,
                        0.0,
                        z * (d / 2.0 - 0.06) - 0.025,
                    ],
                    [0.05, DESK_H - 0.04, 0.05],
                    frame,
                );
            }
            b.bx(
                t,
                [-w / 2.0 + 0.06, DESK_H - 0.12, -d / 2.0 + 0.04],
                [w - 0.12, 0.07, 0.025],
                frame,
            );
        }
        DeskKind::Trestle => {
            b.with(tm, |b| {
                b.bx(t, [-w / 2.0, DESK_H - 0.04, -d / 2.0], [w, 0.04, d], top);
            });
            for x in [-w / 2.0 + 0.18, w / 2.0 - 0.18] {
                for s in [-1.0f32, 1.0] {
                    b.put(
                        Mesh::Box,
                        t.t(x, 0.0, s * 0.2)
                            .rx(s * 0.38)
                            .s(0.06, DESK_H / 0.93, 0.06),
                        frame,
                    );
                }
                b.bx(t, [x - 0.03, 0.32, -0.2], [0.06, 0.05, 0.4], frame);
            }
            b.bx(
                t,
                [-w / 2.0 + 0.18, 0.34, -0.03],
                [w - 0.36, 0.04, 0.06],
                frame,
            );
        }
        DeskKind::Pedestal => {
            desk_top(b, t, (top, tm), w, d);
            b.bx(
                t,
                [-w / 2.0 + 0.03, 0.0, -d / 2.0 + 0.03],
                [0.04, DESK_H - 0.035, d - 0.06],
                frame,
            );
            let px = w / 2.0 - 0.45;
            b.bx(
                t,
                [px, 0.02, -d / 2.0 + 0.04],
                [0.42, DESK_H - 0.06, d - 0.1],
                frame,
            );
            for k in 0..3 {
                let y = k as f32 * 0.22;
                b.bx(
                    t,
                    [px + 0.02, 0.06 + y, d / 2.0 - 0.065],
                    [0.38, 0.19, 0.012],
                    ramp(frame, 0.15),
                );
                b.bx(
                    t,
                    [px + 0.15, 0.2 + y, d / 2.0 - 0.055],
                    [0.12, 0.015, 0.02],
                    hex("#c8c8cc"),
                );
            }
            b.bx(
                t,
                [-w / 2.0 + 0.05, 0.25, -d / 2.0 + 0.03],
                [w - 0.5, 0.42, 0.02],
                ramp(frame, 0.08),
            );
        }
        DeskKind::Standing => {
            let h = 1.04;
            b.with(tm, |b| {
                b.bx(t, [-w / 2.0, h - 0.035, -d / 2.0], [w, 0.035, d], top);
            });
            for x in [-w / 2.0 + 0.12, w / 2.0 - 0.12] {
                b.bx(t, [x - 0.04, 0.0, -0.32], [0.08, 0.04, 0.64], frame);
                b.bx(t, [x - 0.035, 0.04, -0.035], [0.07, h - 0.075, 0.07], frame);
                b.bx(
                    t,
                    [x - 0.025, 0.4, -0.025],
                    [0.05, h - 0.45, 0.05],
                    ramp(frame, 0.2),
                );
            }
            b.bx(
                t,
                [-w / 2.0 + 0.12, h - 0.12, -0.03],
                [w - 0.24, 0.05, 0.06],
                frame,
            );
            return h;
        }
        DeskKind::LShape => {
            desk_top(b, t, (top, tm), w, d);
            b.with(tm, |b| {
                b.bx(
                    t,
                    [w / 2.0 - 0.6, DESK_H - 0.035, d / 2.0],
                    [0.6, 0.035, 0.55],
                    top,
                );
            });
            b.bx(
                t,
                [-w / 2.0 + 0.02, 0.0, -d / 2.0 + 0.02],
                [0.04, DESK_H - 0.035, d - 0.04],
                frame,
            );
            b.bx(
                t,
                [w / 2.0 - 0.06, 0.0, -d / 2.0 + 0.02],
                [0.04, DESK_H - 0.035, d + 0.5],
                frame,
            );
            b.bx(
                t,
                [-w / 2.0 + 0.06, 0.3, -d / 2.0 + 0.02],
                [w - 0.12, 0.38, 0.02],
                ramp(frame, 0.1),
            );
        }
        DeskKind::Round => {
            b.with(tm, |b| {
                b.put(
                    Mesh::Cyl12,
                    t.t(0.0, DESK_H - 0.035, 0.0).s(1.0, 0.035, 0.9),
                    top,
                );
            });
            b.with(tm, |b| {
                b.put(
                    Mesh::Cyl12,
                    t.t(0.0, DESK_H - 0.045, 0.0).s(0.98, 0.012, 0.88),
                    edge,
                );
            });
            b.put(
                Mesh::Cyl8,
                t.t(0.0, 0.03, 0.0).s(0.08, DESK_H - 0.07, 0.08),
                frame,
            );
            b.put(Mesh::Cyl8, t.s(0.5, 0.03, 0.5), frame);
        }
    }
    DESK_H
}

/// A long shared table `len` long, centred on the frame.
pub fn long_table(b: &mut B, t: T, len: f32, top: Rgba, frame: Rgba) {
    b.with(METAL, |b| table_parts(b, t, len, top, frame));
}

fn table_parts(b: &mut B, t: T, len: f32, top: Rgba, frame: Rgba) {
    let d = 1.05;
    let surface = surface_of(top);
    b.with(surface, |b| {
        b.bx(
            t,
            [-len / 2.0, DESK_H - 0.045, -d / 2.0],
            [len, 0.045, d],
            top,
        );
        b.bx(
            t,
            [-len / 2.0, DESK_H - 0.055, -d / 2.0],
            [len, 0.012, d],
            ramp(top, -0.2),
        );
    });
    for x in [-len / 2.0 + 0.25, len / 2.0 - 0.25] {
        b.bx(t, [x - 0.04, 0.0, -0.4], [0.08, 0.04, 0.8], frame);
        b.bx(
            t,
            [x - 0.03, 0.0, -0.03],
            [0.06, DESK_H - 0.05, 0.06],
            frame,
        );
        b.bx(t, [x - 0.03, DESK_H - 0.1, -0.4], [0.06, 0.05, 0.8], frame);
    }
    b.bx(
        t,
        [-len / 2.0 + 0.3, DESK_H - 0.12, -0.02],
        [len - 0.6, 0.06, 0.04],
        frame,
    );
    b.bx(
        t,
        [-len / 2.0 + 0.4, DESK_H, -0.05],
        [len - 0.8, 0.06, 0.1],
        ramp(frame, 0.25),
    );
}

/// A task chair's seat height, metres.
pub const SEAT_H: f32 = 0.48;

/// A chair whose sitter faces -z (`high`: at a standing desk); its seat
/// height.
pub fn chair(b: &mut B, t: T, kind: ChairKind, c: Rgba, high: bool) -> f32 {
    b.with(PLASTIC, |b| chair_parts(b, t, kind, c, high))
}

#[allow(clippy::too_many_lines, reason = "a model a chair kind")]
fn chair_parts(b: &mut B, t: T, kind: ChairKind, c: Rgba, high: bool) -> f32 {
    let dark = hex("#2a2b30");
    let metal = hex("#9a9ca4");
    match kind {
        ChairKind::Task | ChairKind::Mesh => {
            for k in 0..5 {
                let a = k as f32 / 5.0 * TAU;
                b.put(
                    Mesh::Box,
                    t.ry(a).t(0.0, 0.03, 0.14).s(0.045, 0.035, 0.28),
                    dark,
                );
                b.put_m(
                    Mesh::BallLo,
                    t.ry(a).t(0.0, 0.0, 0.27).s(0.05, 0.05, 0.05),
                    dark,
                    RUBBER,
                );
            }
            b.put_m(
                Mesh::Cyl8,
                t.t(0.0, 0.05, 0.0).s(0.06, 0.36, 0.06),
                metal,
                METAL,
            );
            b.with(FABRIC, |b| {
                b.cb(t, [-0.24, 0.42, -0.24], [0.48, 0.08, 0.46], c);
            });
            let back = if kind == ChairKind::Mesh {
                hex("#3a3c44")
            } else {
                c
            };
            b.put_m(
                Mesh::Chamfer,
                t.t(0.0, 0.52, 0.24).rx(-0.12).s(0.44, 0.5, 0.07),
                back,
                FABRIC,
            );
            if kind == ChairKind::Mesh {
                b.put_m(
                    Mesh::Box,
                    t.t(0.0, 0.58, 0.215).rx(-0.12).s(0.36, 0.38, 0.02),
                    hex("#55575f"),
                    FABRIC,
                );
                b.put(
                    Mesh::Chamfer,
                    t.t(0.0, 1.06, 0.31).rx(-0.12).s(0.3, 0.12, 0.06),
                    back,
                );
            }
            b.put_m(
                Mesh::Box,
                t.t(0.0, 0.45, 0.24).s(0.05, 0.12, 0.04),
                metal,
                METAL,
            );
            for s in [-1.0f32, 1.0] {
                b.bx(t, [s * 0.25 - 0.02, 0.48, -0.02], [0.04, 0.2, 0.04], dark);
                b.bx(t, [s * 0.25 - 0.035, 0.67, -0.15], [0.07, 0.03, 0.26], dark);
            }
            SEAT_H
        }
        ChairKind::Stool => {
            let h = if high { 0.72 } else { 0.46 };
            let wood = hex("#7a5a3a");
            b.put_m(
                Mesh::Cyl12,
                t.t(0.0, h - 0.05, 0.0).s(0.38, 0.05, 0.38),
                c,
                LEATHER,
            );
            for k in 0..4 {
                let a = k as f32 / 4.0 * TAU + 0.785;
                b.put_m(
                    Mesh::Box,
                    t.ry(a).t(0.0, 0.0, 0.16).rx(0.12).s(0.035, h - 0.04, 0.035),
                    wood,
                    WOOD,
                );
                b.put_m(
                    Mesh::Box,
                    t.ry(a).t(0.0, h * 0.35, 0.095).s(0.2, 0.025, 0.025),
                    wood,
                    WOOD,
                );
            }
            h
        }
        ChairKind::Saddle => {
            let h = if high { 0.74 } else { 0.55 };
            b.put(Mesh::Cyl8, t.s(0.44, 0.03, 0.44), dark);
            b.put_m(
                Mesh::Cyl8,
                t.t(0.0, 0.03, 0.0).s(0.06, h - 0.12, 0.06),
                metal,
                METAL,
            );
            b.put_m(
                Mesh::Ball,
                t.t(0.0, h - 0.12, 0.0).s(0.4, 0.14, 0.34),
                c,
                LEATHER,
            );
            h
        }
        ChairKind::Ball => {
            b.put_m(Mesh::Sphere, t.s(0.6, 0.55, 0.6), c, RUBBER);
            b.put_m(
                Mesh::Ball,
                t.t(0.0, 0.22, 0.0).s(0.62, 0.06, 0.62),
                ramp(c, -0.15),
                RUBBER,
            );
            0.5
        }
    }
}

/// Screens on a desk top at `top` showing `pic`; `scroll` ≥ 0 scrolls a
/// code picture (twice as tall as the screen), < 0 holds it still.
pub fn screens(b: &mut B, t: T, kind: Screens, top: f32, pic: Uv, glow: f32, scroll: f32) {
    b.with(PLASTIC, |b| {
        screen_parts(b, t, kind, top, (pic, glow), scroll);
    });
}

fn screen_parts(b: &mut B, t: T, kind: Screens, top: f32, (pic, glow): (Uv, f32), scroll: f32) {
    let dark = hex("#1d1e23");
    let mut uv = pic;
    if scroll >= 0.0 {
        uv[1] += scroll.fract() * uv[3] * 0.5;
        uv[3] *= 0.5;
    }
    let skin = Skin {
        uv,
        repeat: [1.0, 1.0],
        glow,
        flags: 0,
    };
    let panel = |b: &mut B, t: T, w: f32, h: f32| {
        b.put(Mesh::Box, t.s(w + 0.03, h + 0.03, 0.03), dark);
        b.tex(
            Mesh::QuadWall,
            t.t(0.0, 0.015, 0.016).s(w, h, 1.0),
            [255; 4],
            skin,
        );
    };
    match kind {
        Screens::Single | Screens::Wide => {
            let w = if kind == Screens::Wide { 0.86 } else { 0.56 };
            b.put(Mesh::Box, t.t(0.0, top, -0.18).s(0.22, 0.015, 0.16), dark);
            b.put(Mesh::Box, t.t(0.0, top, -0.2).s(0.05, 0.2, 0.03), dark);
            panel(b, t.t(0.0, top + 0.14, -0.2).rx(-0.08), w, 0.33);
        }
        Screens::Dual => {
            for s in [-1.0f32, 1.0] {
                let arm = t.t(s * 0.29, 0.0, -0.18).ry(-s * 0.22);
                b.put(Mesh::Box, arm.t(0.0, top, 0.0).s(0.18, 0.015, 0.14), dark);
                b.put(Mesh::Box, arm.t(0.0, top, -0.02).s(0.04, 0.18, 0.03), dark);
                panel(b, arm.t(0.0, top + 0.13, -0.02).rx(-0.06), 0.5, 0.3);
            }
        }
        Screens::Laptop => {
            let shell = hex("#b9bcc4");
            b.put_m(
                Mesh::Box,
                t.t(0.0, top, 0.02).s(0.34, 0.018, 0.23),
                shell,
                METAL,
            );
            b.put(
                Mesh::Box,
                t.t(0.0, top + 0.018, 0.03).s(0.28, 0.003, 0.11),
                hex("#3a3c44"),
            );
            let lid = t.t(0.0, top + 0.012, -0.09).rx(-0.25);
            b.put_m(Mesh::Box, lid.s(0.34, 0.23, 0.012), shell, METAL);
            b.tex(
                Mesh::QuadWall,
                lid.t(0.0, 0.015, 0.007).s(0.3, 0.2, 1.0),
                [255; 4],
                skin,
            );
            return;
        }
    }
    b.put(
        Mesh::Box,
        t.t(0.0, top, 0.08).s(0.42, 0.02, 0.14),
        hex("#2e2f35"),
    );
    b.put(
        Mesh::Box,
        t.t(0.0, top + 0.02, 0.08).s(0.38, 0.006, 0.11),
        hex("#4a4c55"),
    );
    b.put(
        Mesh::Chamfer,
        t.t(0.3, top, 0.1).s(0.06, 0.025, 0.1),
        hex("#2e2f35"),
    );
}

/// Personal things on a desk `w` wide, never the same set twice; where
/// its lamp's light is, when it has one.
pub fn desk_props(
    b: &mut B,
    t: T,
    top: f32,
    r: &mut Rng,
    accent: Rgba,
    lamp: bool,
    w: f32,
) -> Option<V3> {
    b.with(PLASTIC, |b| props(b, t, top, r, accent, lamp, w))
}

#[allow(clippy::too_many_lines, reason = "a set a desk kind")]
fn props(b: &mut B, t: T, top: f32, r: &mut Rng, accent: Rgba, lamp: bool, w: f32) -> Option<V3> {
    let side = if r.chance(0.5) { -1.0 } else { 1.0 };
    let x0 = side * (w / 2.0 - 0.18);
    if r.chance(0.75) {
        let c = hex(r.pick(&["#f4efe6", "#c86f4f", "#3a5a7a", "#e8b33a", "#2f2f3a"]));
        let m = t.t(-x0 * 0.55, top, 0.12 + r.range(-0.05, 0.05));
        b.put_m(Mesh::CylS, m.s(0.08, 0.1, 0.08), c, CERAMIC);
        b.put_m(
            Mesh::RoundBox,
            m.t(0.05, 0.03, 0.0).s(0.03, 0.05, 0.015),
            c,
            CERAMIC,
        );
        b.put(
            Mesh::Cyl8,
            m.t(0.0, 0.09, 0.0).s(0.065, 0.005, 0.065),
            hex("#4a2a1a"),
        );
    }
    match r.int(5) {
        0 => {
            b.put_m(
                Mesh::CylS,
                t.t(x0, top, -0.05).s(0.09, 0.08, 0.09),
                hex("#e8e2d6"),
                CERAMIC,
            );
            for k in 0..6 {
                b.put_m(
                    Mesh::Leaf,
                    t.t(x0, top + 0.08, -0.05)
                        .ry(k as f32 * 1.05)
                        .rx(-0.7)
                        .s(0.06, 0.06, 0.07),
                    hex("#6fae7a"),
                    LEAF,
                );
            }
        }
        1 => {
            for k in 0..3 {
                let c = hex(r.pick(&["#a4543a", "#3a5a7a", "#d9b44a", "#4a7a5a"]));
                b.put_m(
                    Mesh::Box,
                    t.t(x0, top + k as f32 * 0.035, -0.08)
                        .ry(r.range(-0.3, 0.3))
                        .s(0.2, 0.033, 0.15),
                    c,
                    PAPER,
                );
            }
        }
        2 => {
            b.put(
                Mesh::Box,
                t.t(x0, top, -0.1)
                    .ry(side * 0.4)
                    .rx(-0.2)
                    .s(0.12, 0.15, 0.015),
                hex("#2a2530"),
            );
            let photo = mix(accent, hex("#f2e8d5"), 0.5);
            b.put_m(
                Mesh::Box,
                t.t(x0, top + 0.02, -0.092)
                    .ry(side * 0.4)
                    .rx(-0.2)
                    .s(0.09, 0.1, 0.01),
                photo,
                PAPER,
            );
        }
        3 => {
            let book = hex(r.pick(&["#3d405b", "#81b29a", "#e07a5f"]));
            b.put_m(
                Mesh::Box,
                t.t(x0, top, 0.05)
                    .ry(r.range(-0.4, 0.4))
                    .s(0.16, 0.012, 0.22),
                book,
                LEATHER,
            );
            for k in 0..2 {
                let note = hex(r.pick(&["#ffe27a", "#ffb3c7", "#a8e6cf"]));
                b.put_m(
                    Mesh::Box,
                    t.t(x0 - 0.05 + k as f32 * 0.07, top + 0.013, -0.12)
                        .s(0.06, 0.004, 0.06),
                    note,
                    PAPER,
                );
            }
        }
        _ => {
            let bottle = hex(r.pick(&["#5aa0c8", "#e0e0e0", "#4a7a5a"]));
            b.put(Mesh::Cyl8, t.t(x0, top, -0.08).s(0.07, 0.22, 0.07), bottle);
            b.put(
                Mesh::Cyl8,
                t.t(x0, top + 0.22, -0.08).s(0.05, 0.03, 0.05),
                hex("#2a2b30"),
            );
            b.put(
                Mesh::Box,
                t.t(-x0 * 0.2, top, 0.18).ry(0.4).s(0.16, 0.03, 0.05),
                hex("#2a2b30"),
            );
        }
    }
    lamp.then(|| desk_lamp(b, t.t(-x0, top, -0.22), r))
}

/// A desk lamp on its base at `base`; where its light is.
fn desk_lamp(b: &mut B, base: T, r: &mut Rng) -> V3 {
    b.with(METAL, |b| lamp_parts(b, base, r))
}

fn lamp_parts(b: &mut B, base: T, r: &mut Rng) -> V3 {
    let ink = hex("#2a2b30");
    b.put(Mesh::Cyl8, base.s(0.12, 0.02, 0.12), ink);
    b.put(
        Mesh::Box,
        base.t(0.0, 0.02, 0.0).rx(0.35).s(0.02, 0.3, 0.02),
        ink,
    );
    b.put(
        Mesh::Box,
        base.t(0.0, 0.3, -0.1).rx(-1.1).s(0.02, 0.22, 0.02),
        ink,
    );
    let shade = base.t(0.0, 0.42, 0.08);
    let colour = hex(r.pick(&["#e8b33a", "#c86f4f", "#2f4a6b", "#e6e1d8"]));
    b.put(Mesh::Cone8, shade.rx(PI * 0.82).s(0.14, 0.12, 0.14), colour);
    b.glow(
        Mesh::Ball,
        shade.t(0.0, -0.06, -0.02).s(0.05, 0.05, 0.05),
        hex("#fff2c0"),
        1.0,
    );
    shade.0.point([0.0, -0.1, 0.0])
}

/// A pot of `kind` at scale `s`; its rim's height.
pub fn pot(b: &mut B, t: T, kind: Pot, s: f32) -> f32 {
    let m = match kind {
        Pot::Terracotta | Pot::Concrete => PLASTER,
        Pot::Ceramic(_) => CERAMIC,
        Pot::Box(_) => PLASTIC,
        Pot::Basket => FABRIC,
    };
    let h = b.with(m, |b| pot_parts(b, t, kind, s));
    b.put_m(
        Mesh::Cyl12,
        t.t(0.0, h - 0.03 * s, 0.0).s(0.34 * s, 0.031, 0.34 * s),
        hex("#4a3426"),
        SOIL,
    );
    h
}

fn pot_parts(b: &mut B, t: T, kind: Pot, s: f32) -> f32 {
    let h = 0.42 * s;
    match kind {
        Pot::Terracotta => {
            b.put(Mesh::Pot, t.s(0.36 * s, h, 0.36 * s), hex("#b5653d"));
            b.put(
                Mesh::Cyl8,
                t.t(0.0, h - 0.06 * s, 0.0).s(0.48 * s, 0.07 * s, 0.48 * s),
                hex("#c47a52"),
            );
        }
        Pot::Ceramic(c) => {
            b.put(Mesh::Cyl12, t.s(0.42 * s, h, 0.42 * s), c);
            b.put(
                Mesh::Cyl12,
                t.t(0.0, h * 0.15, 0.0).s(0.43 * s, 0.03 * s, 0.43 * s),
                ramp(c, -0.2),
            );
        }
        Pot::Box(c) => {
            b.bx(t, [-0.22 * s, 0.0, -0.22 * s], [0.44 * s, h, 0.44 * s], c);
            b.bx(
                t,
                [-0.24 * s, h - 0.04 * s, -0.24 * s],
                [0.48 * s, 0.04 * s, 0.48 * s],
                ramp(c, -0.15),
            );
        }
        Pot::Basket => {
            b.put(Mesh::Cyl8, t.s(0.44 * s, h, 0.44 * s), hex("#b08a5a"));
            for k in 0..3 {
                b.put(
                    Mesh::Cyl8,
                    t.t(0.0, (0.08 + k as f32 * 0.12) * s, 0.0)
                        .s(0.45 * s, 0.03 * s, 0.45 * s),
                    hex("#8a6a42"),
                );
            }
        }
        Pot::Concrete => {
            b.put(Mesh::Cyl8, t.s(0.44 * s, h, 0.44 * s), hex("#9a9a98"));
            b.put(
                Mesh::Cyl8,
                t.t(0.0, h - 0.02, 0.0).s(0.36 * s, 0.025, 0.36 * s),
                hex("#7f7f7d"),
            );
        }
    }
    h
}

/// How a plant stands now: its size, its seed, a sway (radians) when
/// touched, and how wilted it is (0–1: an unused App's).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Growth {
    /// Its scale.
    pub s: f32,
    /// Its seed.
    pub seed: u32,
    /// Its sway, radians.
    pub sway: f32,
    /// Its wilt, 0–1.
    pub wilt: f32,
}

/// A potted plant.
pub fn plant(b: &mut B, t: T, kind: Plant, pot_kind: Pot, g: Growth) {
    let small = matches!(kind, Plant::Succulent | Plant::Herbs);
    let top = pot(b, t, pot_kind, if small { g.s * 0.6 } else { g.s });
    let mut leaves = Leaves {
        r: Rng(g.seed | 1),
        g,
        at: t.t(0.0, top, 0.0),
    };
    let from = b.out.len();
    match kind {
        Plant::Monstera => leaves.monstera(b),
        Plant::Fern => leaves.fern(b),
        Plant::Snake => leaves.snake(b),
        Plant::Cactus => leaves.cactus(b),
        Plant::Fig => leaves.fig(b),
        Plant::Palm => leaves.palm(b),
        Plant::Herbs => leaves.herbs(b),
        Plant::Succulent => leaves.succulent(b),
    }
    // leaves and fronds veined; stems, trunks and cacti their own way
    for it in &mut b.out[from..] {
        if it.inst.mat == 0 {
            it.inst.mat = match it.mesh {
                Mesh::Leaf | Mesh::Frond => LEAF,
                Mesh::Cyl6 if matches!(kind, Plant::Fig | Plant::Palm) => WOOD,
                _ => PLASTIC,
            };
        }
    }
}

/// A plant above its pot.
struct Leaves {
    r: Rng,
    g: Growth,
    at: T,
}

impl Leaves {
    fn green(&mut self, c: &str) -> Rgba {
        mix(
            ramp(hex(c), self.r.range(-0.12, 0.12)),
            hex("#9a7a3a"),
            self.g.wilt,
        )
    }

    fn monstera(&mut self, b: &mut B) {
        let (s, sway, wilt) = (self.g.s, self.g.sway, self.g.wilt);
        for k in 0..8 {
            let a = k as f32 / 8.0 * TAU + self.r.range(-0.3, 0.3);
            let up = self.r.range(0.25, 0.7) * s;
            let stem = self.at.ry(a).rx(0.35 - sway * 0.5);
            let (stem_c, leaf_c) = (self.green("#4a7a3a"), self.green("#2f7a3e"));
            b.put(Mesh::Cyl6, stem.s(0.02, up + 0.15 * s, 0.02), stem_c);
            let bend = -0.5 - wilt * 0.8 + self.r.range(-0.2, 0.2) + sway;
            b.put(
                Mesh::Leaf,
                stem.t(0.0, up + 0.12 * s, 0.0)
                    .rx(bend)
                    .s(0.42 * s, 0.4 * s, 0.5 * s),
                leaf_c,
            );
        }
    }

    fn fern(&mut self, b: &mut B) {
        let (s, sway, wilt) = (self.g.s, self.g.sway, self.g.wilt);
        for k in 0..14 {
            let a = k as f32 / 14.0 * TAU + self.r.range(-0.2, 0.2);
            let bend = -0.6 + self.r.range(-0.3, 0.2) + sway + wilt * 0.6;
            let c = self.green("#4f9a3e");
            b.put(
                Mesh::Frond,
                self.at
                    .t(0.0, 0.02, 0.0)
                    .ry(a)
                    .rx(bend)
                    .s(0.4 * s, 0.5 * s, 0.6 * s),
                c,
            );
        }
    }

    fn snake(&mut self, b: &mut B) {
        let (s, sway, wilt) = (self.g.s, self.g.sway, self.g.wilt);
        for k in 0..9 {
            let a = k as f32 / 9.0 * TAU + self.r.range(-0.4, 0.4);
            let h = self.r.range(0.55, 0.95) * s;
            let c = self.green(if k % 3 == 0 { "#5f7f3a" } else { "#3f6e3a" });
            let bend = -1.4 + self.r.range(-0.1, 0.15) + sway * 0.4 + wilt;
            b.put(
                Mesh::Leaf,
                self.at
                    .ry(a)
                    .t(0.0, 0.0, 0.06 * s)
                    .rx(bend)
                    .s(0.12 * s, 0.1, h),
                c,
            );
        }
    }

    fn cactus(&mut self, b: &mut B) {
        let (s, sway) = (self.g.s, self.g.sway);
        let g = self.green("#4f8a4a");
        b.put(
            Mesh::Cyl8,
            self.at.rz(sway * 0.2).s(0.2 * s, 0.75 * s, 0.2 * s),
            g,
        );
        b.put(
            Mesh::Ball,
            self.at.t(0.0, 0.66 * s, 0.0).s(0.2 * s, 0.16 * s, 0.2 * s),
            g,
        );
        for side in [-1.0f32, 1.0] {
            let arm = self.at.t(side * 0.09 * s, (0.3 + 0.1 * side) * s, 0.0);
            b.put(
                Mesh::Cyl8,
                arm.rz(-side * FRAC_PI_2).s(0.11 * s, 0.14 * s, 0.11 * s),
                g,
            );
            b.put(
                Mesh::Cyl8,
                arm.t(side * 0.14 * s, 0.0, 0.0)
                    .s(0.11 * s, 0.25 * s, 0.11 * s),
                g,
            );
        }
        b.put(
            Mesh::Ball,
            self.at
                .t(0.0, 0.78 * s, 0.0)
                .s(0.08 * s, 0.06 * s, 0.08 * s),
            hex("#ff7aa8"),
        );
    }

    fn fig(&mut self, b: &mut B) {
        let (s, sway) = (self.g.s, self.g.sway);
        let trunk = self.at.rz(sway * 0.15);
        b.put(
            Mesh::Cyl6,
            trunk.s(0.06 * s, 1.1 * s, 0.06 * s),
            hex("#6b4a32"),
        );
        for k in 0..5 {
            let a = k as f32 * 1.3;
            let p = trunk.t(
                0.18 * s * a.cos(),
                (0.95 + 0.25 * (k % 3) as f32) * s,
                0.18 * s * a.sin(),
            );
            let crown = self.green("#2f6a34");
            b.put(Mesh::BallLo, p.ry(a).s(0.42 * s, 0.34 * s, 0.42 * s), crown);
            for j in 0..3 {
                let c = self.green("#3f8a44");
                b.put(
                    Mesh::Leaf,
                    p.t(0.0, 0.2 * s, 0.0)
                        .ry(a + j as f32 * 2.1)
                        .rx(-0.2 + sway)
                        .s(0.16 * s, 0.1, 0.24 * s),
                    c,
                );
            }
        }
    }

    fn palm(&mut self, b: &mut B) {
        let (s, sway, wilt) = (self.g.s, self.g.sway, self.g.wilt);
        let mut p = self.at;
        for k in 0..4 {
            let girth = (0.1 - k as f32 * 0.012) * s;
            b.put(
                Mesh::Cyl6,
                p.s(girth, 0.3 * s, girth),
                hex(if k % 2 == 0 { "#8a6a42" } else { "#7a5a3a" }),
            );
            p = p.t(0.0, 0.28 * s, 0.0).rz(0.06 + sway * 0.1);
        }
        for k in 0..9 {
            let c = self.green("#4a8a3a");
            b.put(
                Mesh::Frond,
                p.ry(k as f32 / 9.0 * TAU).rx(-0.15 + sway + wilt * 0.5).s(
                    0.5 * s,
                    0.6 * s,
                    0.85 * s,
                ),
                c,
            );
        }
    }

    fn herbs(&mut self, b: &mut B) {
        let (s, sway) = (self.g.s, self.g.sway);
        for (k, tone) in ["#5a9a4a", "#7aaa5a", "#4a8a5a"].into_iter().enumerate() {
            let p = self.at.t((k as f32 - 1.0) * 0.1 * s, 0.0, 0.0);
            for j in 0..5 {
                let c = self.green(tone);
                b.put(
                    Mesh::Leaf,
                    p.ry(j as f32 * 1.26 + k as f32)
                        .rx(-1.0 + sway)
                        .s(0.1 * s, 0.1, 0.16 * s),
                    c,
                );
            }
        }
    }

    fn succulent(&mut self, b: &mut B) {
        let (s, sway) = (self.g.s, self.g.sway);
        for ring in 0..2 {
            for k in 0..7 {
                let c = self.green(if ring == 0 { "#7ab08a" } else { "#9ac8a0" });
                let r = ring as f32;
                b.put(
                    Mesh::Leaf,
                    self.at
                        .ry(k as f32 * 0.9 + r * 0.45)
                        .rx(-0.9 + r * 0.5 + sway)
                        .s(0.1 * s, 0.08, (0.14 - r * 0.04) * s),
                    c,
                );
            }
        }
    }
}

/// How a person looks: chosen once per desk from its seed.
#[derive(Clone, Debug, PartialEq)]
pub struct Look {
    /// Skin.
    pub skin: Rgba,
    /// Hair.
    pub hair: Rgba,
    /// One of nine hairstyles.
    pub style: u8,
    /// Their top.
    pub top: Rgba,
    /// One of four tops (plain, hoodie, shirt and tie, striped).
    pub top_style: u8,
    /// Trousers.
    pub legs: Rgba,
    /// Shoes.
    pub shoes: Rgba,
    /// Glasses.
    pub glasses: bool,
    /// A beard.
    pub beard: bool,
    /// Height, about 1.
    pub height: f32,
    /// Their badge: the App's colour.
    pub badge: Rgba,
}

impl Look {
    /// A look from `r`, with the App's colour on the badge.
    #[must_use]
    pub fn random(r: &mut Rng, badge: Rgba) -> Self {
        let skins = [
            "#f1d0b5", "#e6b896", "#c98d6a", "#a8704f", "#7a4a32", "#5a3524",
        ];
        let hairs = [
            "#2a1d17", "#4a2f22", "#7a4a2a", "#b5824f", "#d8b46a", "#a8a8ad", "#c2502f", "#2f2f3a",
            "#e8e0d0",
        ];
        let tops = [
            "#3d6fb0", "#c84a3a", "#3a8a5a", "#e8b33a", "#7a4ab0", "#2f2f3a", "#e6e1d8", "#d97aa0",
            "#4aa3a3", "#8a5a3a",
        ];
        let legs = [
            "#2a3550", "#3a3a40", "#5a4a3a", "#1f1f24", "#4a5a7a", "#7a6a5a",
        ];
        let style = r.int(9) as u8;
        Self {
            skin: hex(r.pick(&skins)),
            hair: hex(r.pick(&hairs)),
            style,
            top: hex(r.pick(&tops)),
            top_style: r.int(4) as u8,
            legs: hex(r.pick(&legs)),
            shoes: hex(r.pick(&["#1f1f24", "#e8e8e8", "#7a4a2a", "#c84a3a"])),
            glasses: r.chance(0.3),
            beard: style != 1 && r.chance(0.2),
            height: r.range(0.94, 1.06),
            badge,
        }
    }
}

impl Look {
    /// A subagent (rules-and-mechanics: "helpers in the App's colour";
    /// README: one helper look per App hue): the App's colour head to
    /// foot, a plain top, no beard or glasses, so a desk's helpers read
    /// as one team and never as more staff. Only skin and hair vary.
    #[must_use]
    pub fn helper(r: &mut Rng, accent: Rgba) -> Self {
        let mut look = Self::random(r, accent);
        look.top = accent;
        look.top_style = 0;
        look.legs = ramp(accent, -0.35);
        look.shoes = hex("#1f1f24");
        look.glasses = false;
        look.beard = false;
        look.height = 1.0;
        look
    }
}

/// How big a helper is beside a person (the guide: "small helpers round
/// a desk").
pub const HELPER_SCALE: f32 = 0.68;

/// What a person is doing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Pose {
    /// Typing (working).
    Typing,
    /// Leaning back with a mug (idle).
    Idle,
    /// Standing and waving (asks you).
    Wave,
    /// Scratching their head (stuck).
    Scratch,
    /// Hands on head (crashed).
    HandsOnHead,
    /// Stretching (done).
    Stretch,
    /// Asleep at the desk (out of usage).
    Asleep,
    /// A subagent helping, with a clipboard.
    Helper,
    /// Walking.
    Walk,
    /// Standing.
    Stand,
}

impl Pose {
    const fn sits(self) -> bool {
        matches!(
            self,
            Self::Typing | Self::Idle | Self::Scratch | Self::Stretch | Self::Asleep
        )
    }
}

/// A limb's angles: shoulder pitch, shoulder roll, elbow.
type Arm = (f32, f32, f32);

/// The pose at time `t`: lean, head pitch, extra head turn, left and right
/// arms, and the walk's swing.
fn rig(pose: Pose, t: f32) -> (f32, f32, f32, Arm, Arm, f32) {
    let w = |f: f32| (t * f).sin();
    let rest = (0.05, 0.08, 0.1);
    match pose {
        Pose::Typing => (
            0.12,
            0.1,
            0.0,
            (0.55, -0.05, 1.0 + 0.1 * w(14.0)),
            (0.55, -0.05, 1.0 - 0.1 * w(14.0)),
            0.0,
        ),
        Pose::Idle => (
            -0.18,
            -0.05,
            0.0,
            (0.1, 0.1, 0.2),
            (0.35, -0.15, 1.9 + 0.15 * w(1.3).max(0.0)),
            0.0,
        ),
        Pose::Scratch => (
            0.05,
            0.15,
            0.2,
            (0.5, -0.05, 1.0),
            (0.4, 2.3, 1.6 + 0.25 * w(9.0)),
            0.0,
        ),
        Pose::Stretch => {
            let up = (0.0, 2.9, 0.2 + 0.1 * w(2.0));
            (-0.22, -0.25, 0.0, up, up, 0.0)
        }
        Pose::Asleep => (
            0.75,
            0.55 + 0.03 * w(1.5),
            0.0,
            (1.35, -0.25, 1.4),
            (1.35, -0.25, 1.4),
            0.0,
        ),
        Pose::Wave => (
            0.0,
            -0.1,
            0.0,
            (0.05, 0.1, 0.15),
            (0.2, 2.55, 0.35 + 0.45 * w(9.0)),
            0.0,
        ),
        Pose::HandsOnHead => (
            0.0,
            0.1 * w(6.0),
            0.0,
            (0.6, 2.2, 1.8),
            (0.6, 2.2, 1.8),
            0.0,
        ),
        Pose::Helper => (
            0.0,
            0.25,
            0.0,
            (0.75, -0.25, 1.5),
            (0.9 + 0.15 * w(3.0), 0.15, 0.4),
            0.0,
        ),
        Pose::Walk => {
            let walk = 0.5 * w(7.0);
            (0.0, 0.0, 0.0, (-walk, 0.08, 0.3), (walk, 0.08, 0.3), walk)
        }
        Pose::Stand => (0.0, 0.0, 0.0, rest, rest, 0.0),
    }
}

/// A person standing on the frame's origin, facing -z (a sitting pose
/// sits at `seat`). `t` animates; `turn` turns the head (towards the
/// viewer when hovered). Where a balloon goes, above the head.
pub fn person(b: &mut B, root: T, lk: &Look, pose: Pose, t: f32, seat: f32, turn: f32) -> V3 {
    let st = stance(root, lk.height, pose, t, seat, 0.86);
    let (root, hip_y, (lean, head_p, head_y, al, ar, _)) = (st.root, st.hip_y, st.rig);
    for (hip, kn) in st.legs(0.09, 0.4) {
        b.bx(hip, [-0.07, -0.42, -0.075], [0.14, 0.42, 0.15], lk.legs);
        b.bx(kn, [-0.06, -0.4, -0.065], [0.12, 0.4, 0.13], lk.legs);
        b.cb(
            kn.t(0.0, -0.44, 0.0),
            [-0.06, 0.0, -0.16],
            [0.12, 0.08, 0.24],
            lk.shoes,
        );
    }
    let up = root.t(0.0, hip_y, 0.0).rx(-lean).t(0.0, -0.86, 0.0);
    body(b, up, lk);
    let head = up.t(0.0, 1.4, 0.0).rx(-head_p).ry(turn + head_y);
    face(b, head, lk, t);
    let long = lk.top_style != 0;
    for (s, (p, roll, el)) in [(-1.0f32, al), (1.0, ar)] {
        let sh = up.t(s * 0.23, 1.32, 0.0).rz(s * roll).rx(p);
        b.cb(sh, [-0.05, -0.3, -0.055], [0.1, 0.32, 0.11], lk.top);
        let fo = sh.t(0.0, -0.29, 0.0).rx(el);
        b.bx(
            fo,
            [-0.042, -0.27, -0.045],
            [0.084, 0.27, 0.09],
            if long { lk.top } else { lk.skin },
        );
        b.cb(
            fo.t(0.0, -0.27, 0.0),
            [-0.04, -0.08, -0.04],
            [0.08, 0.09, 0.08],
            lk.skin,
        );
        if pose == Pose::Idle && s > 0.0 {
            b.put(
                Mesh::Cyl8,
                fo.t(0.0, -0.33, -0.04).rx(-el - p).s(0.08, 0.1, 0.08),
                hex("#f4efe6"),
            );
        }
        if pose == Pose::Helper && s < 0.0 {
            b.put(
                Mesh::Box,
                fo.t(0.05, -0.3, -0.06).rx(-0.4).s(0.2, 0.26, 0.015),
                hex("#c9a46c"),
            );
            b.put(
                Mesh::Box,
                fo.t(0.05, -0.18, -0.07).rx(-0.4).s(0.16, 0.18, 0.01),
                hex("#f4efe6"),
            );
        }
    }
    head.0.point([0.0, 0.42, 0.0])
}

/// Hips, torso, the top's detail, the badge and the neck.
fn body(b: &mut B, up: T, lk: &Look) {
    b.cb(up, [-0.17, 0.8, -0.11], [0.34, 0.16, 0.22], lk.legs);
    b.cb(up, [-0.19, 0.92, -0.12], [0.38, 0.46, 0.24], lk.top);
    match lk.top_style {
        1 => {
            b.put(
                Mesh::BallLo,
                up.t(0.0, 1.24, 0.12).s(0.3, 0.16, 0.12),
                ramp(lk.top, -0.12),
            );
            b.bx(
                up,
                [-0.1, 0.96, -0.125],
                [0.2, 0.08, 0.01],
                ramp(lk.top, -0.1),
            );
        }
        2 => {
            b.bx(up, [-0.1, 1.31, -0.125], [0.2, 0.05, 0.03], hex("#f4f1ea"));
            b.bx(
                up,
                [-0.012, 0.95, -0.124],
                [0.024, 0.36, 0.01],
                ramp(lk.top, -0.25),
            );
        }
        3 => b.bx(
            up,
            [-0.191, 1.08, -0.122],
            [0.382, 0.06, 0.244],
            ramp(lk.top, 0.2),
        ),
        _ => {}
    }
    b.glow(
        Mesh::Box,
        up.t(0.1, 1.2, -0.125).s(0.06, 0.08, 0.012),
        lk.badge,
        0.35,
    );
    b.put(Mesh::Cyl6, up.t(0.0, 1.36, 0.0).s(0.1, 0.06, 0.1), lk.skin);
}

/// The head: face, eyes, mouth, glasses, beard and hair.
fn face(b: &mut B, head: T, lk: &Look, t: f32) {
    let ink = hex("#1d1a24");
    b.put(
        Mesh::Ball,
        head.t(0.0, -0.01, 0.0).s(0.31, 0.32, 0.3),
        lk.skin,
    );
    for s in [-1.0f32, 1.0] {
        b.bx(
            head,
            [s * 0.065 - 0.018, 0.15, -0.155],
            [0.036, 0.045, 0.02],
            ink,
        );
    }
    b.bx(
        head,
        [-0.035, 0.06, -0.152],
        [0.07, 0.018, 0.02],
        ramp(lk.skin, -0.25),
    );
    if lk.glasses {
        b.bx(
            head,
            [-0.13, 0.155, -0.172],
            [0.26, 0.035, 0.02],
            hex("#2a2530"),
        );
    }
    if lk.beard {
        b.cb(head, [-0.12, -0.02, -0.15], [0.24, 0.12, 0.12], lk.hair);
    }
    let h = lk.hair;
    let cap = |b: &mut B| b.put(Mesh::Ball, head.t(0.0, 0.12, 0.02).s(0.33, 0.21, 0.32), h);
    match lk.style {
        0 => cap(b),
        1 => {
            cap(b);
            b.cb(head, [-0.15, -0.16, 0.02], [0.3, 0.36, 0.13], h);
        }
        2 => {
            b.put(Mesh::Ball, head.t(0.0, 0.14, 0.02).s(0.32, 0.19, 0.31), h);
            b.put(Mesh::Ball, head.t(0.0, 0.27, 0.1).s(0.13, 0.13, 0.13), h);
        }
        3 => {
            for k in 0..7 {
                let a = k as f32 / 7.0 * TAU;
                let p = head.t(
                    0.11 * a.cos(),
                    0.2 + 0.03 * (k % 2) as f32,
                    0.11 * a.sin() + 0.02,
                );
                b.put(Mesh::BallLo, p.s(0.15, 0.14, 0.15), h);
            }
        }
        4 => {
            cap(b);
            b.put(
                Mesh::Cyl6,
                head.t(0.0, 0.18, 0.16)
                    .rx(2.6 + 0.1 * (t * 4.0).sin())
                    .s(0.07, 0.24, 0.07),
                h,
            );
        }
        5 => b.put(Mesh::Ball, head.t(0.0, 0.16, 0.01).s(0.32, 0.17, 0.31), h),
        6 => {
            b.put(Mesh::Ball, head.t(0.0, 0.11, 0.02).s(0.34, 0.22, 0.33), h);
            for s in [-1.0f32, 1.0] {
                b.cb(head, [s * 0.15 - 0.03, 0.0, -0.08], [0.06, 0.2, 0.2], h);
            }
        }
        7 => {}
        _ => {
            b.put(
                Mesh::Cyl8,
                head.t(0.0, 0.17, 0.01).s(0.34, 0.14, 0.33),
                ramp(lk.top, 0.15),
            );
            b.put(
                Mesh::BallLo,
                head.t(0.0, 0.3, 0.01).s(0.09, 0.09, 0.09),
                hex("#f4efe6"),
            );
        }
    }
}

/// [`person`] or, in 0.4.1's second pass (`detail`), [`person_v2`].
#[allow(clippy::too_many_arguments, reason = "the person's inputs and pass")]
pub fn person_as(
    detail: bool,
    b: &mut B,
    root: T,
    lk: &Look,
    pose: Pose,
    t: f32,
    seat: f32,
    turn: f32,
) -> V3 {
    if detail {
        person_v2(b, root, lk, pose, t, seat, turn)
    } else {
        person(b, root, lk, pose, t, seat, turn)
    }
}

/// A face for each pose (the state it shows), blinking now and then.
#[must_use]
pub fn expression(pose: Pose, t: f32) -> crate::materials::Face {
    use crate::materials::Face;
    let f = |open: f32, tilt: f32, raise: f32, smile: f32, mouth: f32| Face {
        open,
        tilt,
        raise,
        smile,
        mouth,
        happy: false,
    };
    let mut e = match pose {
        // working: focused, a little frown of concentration
        Pose::Typing => f(0.78, -0.25, 0.0, 0.05, 0.0),
        // idle: relaxed, half-lidded, content
        Pose::Idle => f(0.62, 0.0, 0.0, 0.55, 0.0),
        // asks you: brows up, eyes open, a hopeful smile, mouth open
        Pose::Wave => f(1.0, 0.15, 1.0, 0.7, 0.45),
        // stuck: worried brows, a frown
        Pose::Scratch => f(0.9, 0.75, 0.35, -0.45, 0.0),
        // crashed: alarmed, wide eyes, an open mouth
        Pose::HandsOnHead => f(1.15, 0.9, 0.9, -0.55, 0.8),
        // done: eyes shut happily, a big smile
        Pose::Stretch => Face {
            happy: true,
            ..f(0.0, 0.1, 0.4, 0.9, 0.35)
        },
        // out of usage: asleep, mouth a little open
        Pose::Asleep => f(0.0, 0.0, 0.0, 0.0, 0.18),
        Pose::Helper => f(0.85, -0.1, 0.1, 0.25, 0.0),
        Pose::Walk | Pose::Stand => f(0.9, 0.0, 0.0, 0.2, 0.0),
    };
    // a blink every few seconds (not while asleep or beaming)
    if e.open > 0.0 && (t * 0.29).rem_euclid(1.0) < 0.035 {
        e.open = 0.0;
    }
    e
}

/// 0.4.1's second-pass person: the same poses on smooth, proportioned
/// shapes, cloth, denim and leather, hands, ears and a nose, real hair,
/// and a face whose eyes, brows and mouth show the state; breathing.
#[allow(clippy::too_many_lines, reason = "one figure, part by part")]
pub fn person_v2(b: &mut B, root: T, lk: &Look, pose: Pose, t: f32, seat: f32, turn: f32) -> V3 {
    let st = stance(root, lk.height, pose, t, seat, 0.88);
    let (root, hip_y, (lean, head_p, head_y, al, ar, _)) = (st.root, st.hip_y, st.rig);
    let breathe = 1.0 + 0.012 * (t * 1.7).sin();
    for (hip, kn) in st.legs(0.085, 0.41) {
        b.put_m(
            Mesh::Capsule,
            hip.t(0.0, -0.46, 0.0).s(0.15, 0.5, 0.16),
            lk.legs,
            DENIM,
        );
        b.put_m(
            Mesh::Capsule,
            kn.t(0.0, -0.44, 0.0).s(0.12, 0.48, 0.125),
            lk.legs,
            DENIM,
        );
        let foot = kn.t(0.0, -0.45, 0.0);
        b.put_m(
            Mesh::RoundBox,
            foot.t(0.0, 0.0, -0.05).s(0.11, 0.08, 0.25),
            lk.shoes,
            LEATHER,
        );
        b.put_m(
            Mesh::RoundBox,
            foot.t(0.0, -0.012, -0.05).s(0.115, 0.022, 0.255),
            ramp(lk.shoes, -0.3),
            RUBBER,
        );
    }
    let up = root.t(0.0, hip_y, 0.0).rx(-lean).t(0.0, -0.86, 0.0);
    // a round pelvis the thighs and the torso both sink into, then the
    // torso breathing
    b.put_m(
        Mesh::Sphere,
        up.t(0.0, 0.7, 0.0).s(0.34, 0.3, 0.235),
        lk.legs,
        DENIM,
    );
    b.put_m(
        Mesh::Torso,
        up.t(0.0, 0.9, 0.0).s(0.4, 0.5 * breathe, 0.25 * breathe),
        lk.top,
        CLOTH,
    );
    b.with(CLOTH, |b| top_v2(b, up, lk));
    b.glow(
        Mesh::RoundBox,
        up.t(0.1, 1.2, -0.122).s(0.055, 0.075, 0.012),
        lk.badge,
        0.35,
    );
    b.put_m(
        Mesh::CylS,
        up.t(0.0, 1.36, 0.0).s(0.1, 0.08, 0.1),
        lk.skin,
        SKIN,
    );
    let sway = 0.03 * (t * 0.7).sin();
    // asleep: the head rests on its side on the arms, the face to us
    let head = if pose == Pose::Asleep {
        up.t(0.0, 1.4, -0.02).rx(lean * 0.7).ry(1.25).rz(0.45)
    } else {
        up.t(0.0, 1.42, 0.0).rx(-head_p).ry(turn + head_y + sway)
    };
    head_v2(b, head, lk, expression(pose, t));
    let long = lk.top_style != 0;
    for (s, (p, roll, el)) in [(-1.0f32, al), (1.0, ar)] {
        let sh = up.t(s * 0.215, 1.32, 0.0).rz(s * roll).rx(p);
        // the upper arm's round end is the shoulder, tucked into the
        // torso: no ball on top
        b.put_m(
            Mesh::Capsule,
            sh.t(0.0, -0.33, 0.0).s(0.1, 0.42, 0.105),
            lk.top,
            CLOTH,
        );
        // the elbow: as round as the forearm, whatever its bend
        b.put_m(
            Mesh::Sphere,
            sh.t(0.0, -0.335, 0.0).s(0.088, 0.088, 0.092),
            if long { lk.top } else { lk.skin },
            if long { CLOTH } else { SKIN },
        );
        let fo = sh.t(0.0, -0.29, 0.0).rx(el);
        if long {
            b.put_m(
                Mesh::Capsule,
                fo.t(0.0, -0.28, 0.0).s(0.09, 0.32, 0.095),
                lk.top,
                CLOTH,
            );
        } else {
            b.put_m(
                Mesh::Capsule,
                fo.t(0.0, -0.28, 0.0).s(0.08, 0.32, 0.085),
                lk.skin,
                SKIN,
            );
        }
        let hand = fo.t(0.0, -0.3, 0.0);
        b.put_m(
            Mesh::Sphere,
            hand.t(0.0, -0.06, 0.0).s(0.075, 0.1, 0.06),
            lk.skin,
            SKIN,
        );
        b.put_m(
            Mesh::Capsule,
            hand.t(-s * 0.03, -0.04, -0.02)
                .rz(s * 0.5)
                .s(0.03, 0.06, 0.03),
            lk.skin,
            SKIN,
        );
        if pose == Pose::Idle && s > 0.0 {
            b.put_m(
                Mesh::CylS,
                fo.t(0.0, -0.33, -0.04).rx(-el - p).s(0.08, 0.1, 0.08),
                hex("#f4efe6"),
                CERAMIC,
            );
        }
        if pose == Pose::Helper && s < 0.0 {
            b.put_m(
                Mesh::RoundBox,
                fo.t(0.05, -0.3, -0.06).rx(-0.4).s(0.2, 0.26, 0.015),
                hex("#c9a46c"),
                WOOD,
            );
            b.put_m(
                Mesh::Box,
                fo.t(0.05, -0.18, -0.072).rx(-0.4).s(0.16, 0.18, 0.006),
                hex("#f4efe6"),
                PAPER,
            );
        }
    }
    head.0.point([0.0, 0.4, 0.0])
}

/// A top's details: a hoodie's hood and pocket, a shirt's collar and
/// tie, a stripe.
fn top_v2(b: &mut B, up: T, lk: &Look) {
    match lk.top_style {
        1 => {
            b.put(
                Mesh::Sphere,
                up.t(0.0, 1.22, 0.1).s(0.3, 0.17, 0.14),
                ramp(lk.top, -0.1),
            );
            b.put(
                Mesh::RoundBox,
                up.t(0.0, 0.95, -0.112).s(0.24, 0.09, 0.012),
                ramp(lk.top, -0.05),
            );
            for s in [-1.0f32, 1.0] {
                b.put_m(
                    Mesh::CylS,
                    up.t(s * 0.04, 1.12, -0.125).s(0.008, 0.16, 0.008),
                    hex("#f4efe6"),
                    FABRIC,
                );
            }
        }
        2 => {
            for s in [-1.0f32, 1.0] {
                b.put(
                    Mesh::RoundBox,
                    up.t(s * 0.055, 1.31, -0.105).rz(s * 0.5).s(0.1, 0.06, 0.03),
                    hex("#f4f1ea"),
                );
            }
            b.put_m(
                Mesh::RoundBox,
                up.t(0.0, 0.98, -0.128).s(0.035, 0.32, 0.012),
                ramp(lk.top, -0.3),
                FABRIC,
            );
        }
        3 => b.put(
            Mesh::Torso,
            up.t(0.0, 1.1, 0.0).s(0.402, 0.05, 0.252),
            ramp(lk.top, 0.18),
        ),
        _ => {}
    }
}

/// The head: a face that shows the state, ears, a nose, glasses, a beard
/// and hair.
#[allow(clippy::too_many_lines, reason = "one head, part by part")]
fn head_v2(b: &mut B, head: T, lk: &Look, expr: crate::materials::Face) {
    b.face(head.t(0.0, 0.0, 0.0).s(0.27, 0.3, 0.27), lk.skin, expr);
    b.with(SKIN, |b| {
        for s in [-1.0f32, 1.0] {
            b.put(
                Mesh::Sphere,
                head.t(s * 0.135, 0.12, 0.01).s(0.035, 0.06, 0.04),
                lk.skin,
            );
        }
        b.put(
            Mesh::Sphere,
            head.t(0.0, 0.1, -0.13).s(0.04, 0.05, 0.04),
            ramp(lk.skin, -0.04),
        );
    });
    if lk.glasses {
        b.with(PLASTIC, |b| {
            b.put(
                Mesh::RoundBox,
                head.t(0.0, 0.186, -0.142).s(0.19, 0.012, 0.012),
                hex("#2a2530"),
            );
            b.put(
                Mesh::RoundBox,
                head.t(0.0, 0.17, -0.145).s(0.02, 0.008, 0.01),
                hex("#2a2530"),
            );
            for s in [-1.0f32, 1.0] {
                b.put(
                    Mesh::RoundBox,
                    head.t(s * 0.11, 0.18, -0.08).s(0.008, 0.012, 0.13),
                    hex("#2a2530"),
                );
            }
        });
    }
    let h = lk.hair;
    b.with(HAIR, |b| {
        if lk.beard {
            b.put(
                Mesh::Sphere,
                head.t(0.0, -0.005, -0.03).s(0.25, 0.13, 0.22),
                h,
            );
        }
        let cap = |b: &mut B, y: f32, sy: f32| {
            b.put(Mesh::Sphere, head.t(0.0, y, 0.012).s(0.29, sy, 0.29), h);
        };
        match lk.style {
            0 => cap(b, 0.12, 0.2),
            1 => {
                cap(b, 0.11, 0.21);
                b.put(
                    Mesh::Capsule,
                    head.t(0.0, -0.14, 0.05).s(0.28, 0.36, 0.14),
                    h,
                );
            }
            2 => {
                cap(b, 0.13, 0.19);
                b.put(Mesh::Sphere, head.t(0.0, 0.25, 0.1).s(0.12, 0.12, 0.12), h);
            }
            3 => {
                for k in 0..9 {
                    let a = k as f32 / 9.0 * TAU;
                    let p = head.t(
                        0.1 * a.cos(),
                        0.19 + 0.03 * (k % 2) as f32,
                        0.1 * a.sin() + 0.02,
                    );
                    b.put(Mesh::Sphere, p.s(0.13, 0.12, 0.13), h);
                }
            }
            4 => {
                cap(b, 0.12, 0.2);
                b.put(
                    Mesh::Capsule,
                    head.t(0.0, 0.16, 0.15).rx(2.6).s(0.07, 0.26, 0.07),
                    h,
                );
            }
            5 => cap(b, 0.15, 0.16),
            6 => {
                cap(b, 0.1, 0.22);
                for s in [-1.0f32, 1.0] {
                    b.put(
                        Mesh::Capsule,
                        head.t(s * 0.12, -0.05, 0.02).s(0.07, 0.28, 0.2),
                        h,
                    );
                }
            }
            7 => {}
            _ => {
                // a beanie: a snug knitted dome, a turned-up band, a bobble
                b.put_m(
                    Mesh::Sphere,
                    head.t(0.0, 0.135, 0.01).s(0.295, 0.25, 0.295),
                    ramp(lk.top, 0.15),
                    CLOTH,
                );
                b.put_m(
                    Mesh::CylS,
                    head.t(0.0, 0.205, 0.01).s(0.305, 0.045, 0.305),
                    ramp(lk.top, 0.05),
                    CLOTH,
                );
                b.put_m(
                    Mesh::Sphere,
                    head.t(0.0, 0.36, 0.01).s(0.08, 0.08, 0.08),
                    hex("#f4efe6"),
                    FABRIC,
                );
            }
        }
    });
}

/// The office cat: walks (`walk` is its gait's phase), or sits when
/// clicked.
pub fn cat(b: &mut B, t: T, walk: f32, sitting: bool) {
    b.with(HAIR, |b| cat_parts(b, t, walk, sitting));
}

fn cat_parts(b: &mut B, t: T, walk: f32, sitting: bool) {
    let (c, d) = (hex("#e0913a"), hex("#b86a24"));
    let body_y = if sitting { 0.16 } else { 0.18 };
    if sitting {
        b.put(Mesh::Ball, t.t(0.0, 0.0, 0.05).s(0.26, 0.34, 0.32), c);
    } else {
        b.cb(t, [-0.1, body_y, -0.2], [0.2, 0.16, 0.42], c);
        for (x, z, ph) in [
            (-0.06, -0.15, 0.0),
            (0.06, -0.15, PI),
            (-0.06, 0.15, PI),
            (0.06, 0.15, 0.0),
        ] {
            b.put(
                Mesh::Box,
                t.t(x, 0.0, z)
                    .rx(0.4 * (walk + ph).sin())
                    .s(0.05, body_y + 0.02, 0.05),
                d,
            );
        }
    }
    let head = t.t(0.0, if sitting { 0.3 } else { 0.28 }, -0.24);
    b.put(Mesh::Ball, head.s(0.19, 0.17, 0.18), c);
    for s in [-1.0f32, 1.0] {
        b.put(
            Mesh::Cone8,
            head.t(s * 0.055, 0.14, 0.0).s(0.06, 0.08, 0.05),
            d,
        );
        b.bx(
            head,
            [s * 0.045 - 0.012, 0.08, -0.092],
            [0.024, 0.024, 0.01],
            hex("#1d1a24"),
        );
    }
    let mut tail = t.t(0.0, body_y + 0.12, 0.2);
    for k in 0..4 {
        tail = tail.rx(-0.35 - 0.1 * (walk * 0.5 + k as f32).sin());
        b.put(
            Mesh::Box,
            tail.s(0.04, 0.09, 0.04),
            if k % 2 == 0 { c } else { d },
        );
        tail = tail.t(0.0, 0.08, 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::{B, Growth, Look, Pose, chair, desk, desk_props, person, plant, screens};
    use crate::colour::hex;
    use crate::kit::{ChairKind, DeskKind, Plant, Pot, Screens};
    use crate::math::T;
    use crate::rng::Rng;

    #[test]
    fn every_model_makes_parts_on_its_floor_with_its_pick_id() {
        let mut out = Vec::new();
        let mut b = B {
            out: &mut out,
            floor: 1,
            id: 7,
            dim: 1.0,
        };
        for kind in [
            DeskKind::FourLeg,
            DeskKind::Trestle,
            DeskKind::Pedestal,
            DeskKind::Standing,
            DeskKind::LShape,
            DeskKind::Round,
        ] {
            let top = desk(&mut b, T::new(), kind, hex("#c8a070"), hex("#333333"), 1.4);
            assert!(top > 0.7);
        }
        for kind in [
            ChairKind::Task,
            ChairKind::Mesh,
            ChairKind::Stool,
            ChairKind::Saddle,
            ChairKind::Ball,
        ] {
            assert!(chair(&mut b, T::new(), kind, hex("#3a5a7a"), false) > 0.4);
        }
        for kind in [
            Screens::Single,
            Screens::Wide,
            Screens::Dual,
            Screens::Laptop,
        ] {
            screens(&mut b, T::new(), kind, 0.74, [0.0, 0.0, 0.1, 0.1], 0.8, 0.3);
        }
        let mut r = Rng(9);
        assert!(desk_props(&mut b, T::new(), 0.74, &mut r, hex("#ff0000"), true, 1.4).is_some());
        for kind in [
            Plant::Monstera,
            Plant::Fern,
            Plant::Snake,
            Plant::Cactus,
            Plant::Fig,
            Plant::Palm,
            Plant::Herbs,
            Plant::Succulent,
        ] {
            plant(
                &mut b,
                T::new(),
                kind,
                Pot::Terracotta,
                Growth {
                    s: 1.0,
                    seed: 3,
                    sway: 0.1,
                    wilt: 0.5,
                },
            );
        }
        let look = Look::random(&mut r, hex("#4a7aff"));
        for pose in [
            Pose::Typing,
            Pose::Idle,
            Pose::Wave,
            Pose::Scratch,
            Pose::HandsOnHead,
            Pose::Stretch,
            Pose::Asleep,
            Pose::Helper,
            Pose::Walk,
            Pose::Stand,
        ] {
            let above = person(&mut b, T::new(), &look, pose, 1.0, 0.48, 0.0);
            assert!(above[1] > 1.0, "{pose:?}: a balloon goes above the head");
        }
        assert!(out.len() > 500);
        assert!(out.iter().all(|item| item.floor == 1 && item.inst.id == 7));
    }

    #[test]
    fn a_dimmed_area_darkens_all_but_what_glows() {
        let mut out = Vec::new();
        let mut b = B {
            out: &mut out,
            floor: 0,
            id: 0,
            dim: 0.5,
        };
        b.put(crate::mesh::Mesh::Box, T::new(), hex("#ffffff"));
        b.glow(crate::mesh::Mesh::Box, T::new(), hex("#ffffff"), 1.0);
        assert!((out[0].inst.col[0] - 0.5).abs() < 1e-6);
        assert!((out[1].inst.col[0] - 1.0).abs() < 1e-6);
    }
}
