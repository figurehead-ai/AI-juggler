//! 0.4.1's offices pass (the maintainer: "the offices still read flat and
//! empty"): each office's back wall carries art and a wall screen on its
//! piers and bookcases under its windows, in its App's colours; a ceiling light strip glows over each
//! lit office and hangs dark over an unused one; an unused office
//! gathers a shade and cobwebs, so it reads closed from across the floor.

use crate::colour::{Rgba, hex, mix, ramp};
use crate::layout::{COL_W, LOBBY_W, WALL_H};
use crate::math::T;
use crate::mesh::Mesh;
use crate::models::{self, B, Item};
use crate::rng::Rng;
use crate::tex::{self, Atlas, Full};

use super::super::building::{SILL, windows};
use super::super::{Light, Scene};
use super::{Area, skin};

/// The strip's housing under the ceiling.
const STRIP_DROP: f32 = 0.2;

impl Scene {
    /// The back wall's pieces (front-row offices have none: a sideboard
    /// instead), the ceiling strip, and an unused office's shade.
    pub(super) fn decor(
        &mut self,
        out: &mut Vec<Item>,
        atlas: &mut Atlas,
        a: &Area<'_>,
    ) -> Result<(), Full> {
        let mut b = B {
            out,
            floor: a.floor,
            id: a.id,
            dim: a.dim,
        };
        let mut r = Rng(a.seed ^ 0x51);
        if a.placed.row == 0 {
            back_wall(
                &mut b,
                atlas,
                a,
                &mut r,
                (self.pictures.code[(a.seed % 4) as usize], self.pictures.off),
            )?;
        } else {
            sideboard(&mut b, a, &mut r);
        }
        if let Some(light) = strips(&mut b, a) {
            self.lights.push(light);
        }
        if a.level >= 2 {
            shade(&mut b, a);
        }
        if a.level >= 3 {
            cobwebs(&mut b, a);
        }
        Ok(())
    }
}

/// The App's colour family: its accent, darker and lighter, and two
/// neutrals, for books, boxes and binders.
fn family(accent: Rgba) -> [Rgba; 5] {
    [
        ramp(accent, -0.35),
        accent,
        ramp(accent, 0.3),
        mix(accent, hex("#f2e8d5"), 0.55),
        hex("#3a3644"),
    ]
}

/// The back wall: on the piers between its windows a poster, the wall
/// screen and a pin board in turn (from a place the seed picks); under
/// each window a low bookcase; all clear of the tasks whiteboard
/// at the right.
fn back_wall(
    b: &mut B<'_>,
    atlas: &mut Atlas,
    a: &Area<'_>,
    r: &mut Rng,
    (code, off): (tex::Uv, tex::Uv),
) -> Result<(), Full> {
    let rect = a.placed.rect;
    let poster = atlas.add(
        &format!("poster:{}", a.app.id),
        &tex::poster_img(a.seed, a.kit.accent),
    )?;
    let board = atlas.add(&format!("board:{}", a.app.id), &tex::board_img(a.seed))?;
    let mut panes = Vec::new();
    for c in a.placed.col..a.placed.col + a.placed.size.cells().0 {
        let x = LOBBY_W + c as f32 * COL_W;
        panes.extend(windows(x, x + COL_W));
    }
    let (left, right) = (rect.x0 + 0.3, rect.x1 - 2.0);
    let mut k = a.seed as usize;
    for pair in panes.windows(2) {
        let (from, to) = (pair[0].1, pair[1].0);
        let x = f32::midpoint(from, to);
        if to - from < 0.7 || x < left || x > right {
            continue;
        }
        let t = T::new().t(x, a.fy, 0.03);
        match k % 3 {
            0 => b.tex(
                Mesh::QuadWall,
                t.t(0.0, 1.3, 0.0).s(0.56, 0.76, 1.0),
                [255; 4],
                skin(poster, 0.0, models::CUTAWAY),
            ),
            1 => {
                let lit = a.level == 0;
                b.bx(t, [-0.4, 1.36, 0.0], [0.8, 0.54, 0.05], hex("#1d1a24"));
                b.tex(
                    Mesh::QuadWall,
                    t.t(0.0, 1.39, 0.055).s(0.72, 0.46, 1.0),
                    [255; 4],
                    skin(if lit { code } else { off }, if lit { 0.6 } else { 0.0 }, 0),
                );
                b.bx(t, [-0.4, 1.32, 0.0], [0.8, 0.04, 0.05], a.kit.accent);
            }
            _ => b.tex(
                Mesh::QuadWall,
                t.t(0.0, 1.4, 0.0).s(0.74, 0.46, 1.0),
                [255; 4],
                skin(board, 0.0, models::CUTAWAY),
            ),
        }
        k += 1;
    }
    for &(wx0, wx1) in &panes {
        if wx0 > rect.x0 + 0.8 && wx1 < rect.x1 - 0.8 {
            bookcase(
                b,
                T::new().t(f32::midpoint(wx0, wx1), a.fy, 0.0),
                wx1 - wx0 - 0.1,
                a.kit.accent,
                r,
            );
        }
    }
    Ok(())
}

/// A low bookcase `len` long against the wall at `t`, under a window's
/// sill: two shelves of books in the App's colours, a box and a small
/// plant on top.
fn bookcase(b: &mut B<'_>, t: T, len: f32, accent: Rgba, r: &mut Rng) {
    let (h, d) = (SILL - 0.1, 0.34);
    let half = len / 2.0;
    let wood = hex("#8a6a4a");
    b.bx(t, [-half, 0.0, 0.02], [len, 0.06, d], hex("#3a3644"));
    for x in [-half, half - 0.035] {
        b.bx(t, [x, 0.0, 0.02], [0.035, h, d], wood);
    }
    b.bx(t, [-half, h - 0.035, 0.02], [len, 0.035, d + 0.02], wood);
    b.bx(t, [-half, h / 2.0 - 0.02, 0.02], [len, 0.03, d], wood);
    let pal = family(accent);
    for y in [0.06f32, h / 2.0 + 0.01] {
        let mut x = -half + 0.05;
        while x < half - 0.1 {
            let w = r.range(0.035, 0.065);
            b.bx(
                t,
                [x, y, 0.06],
                [w, r.range(0.22, 0.34), 0.24],
                pal[r.int(pal.len())],
            );
            x += w + 0.005;
            if r.chance(0.1) {
                x += 0.1;
            }
        }
    }
    b.bx(t, [-half + 0.12, h, 0.08], [0.34, 0.16, 0.22], pal[0]);
    b.put(
        Mesh::Cyl8,
        t.t(half - 0.22, h, 0.18).s(0.16, 0.14, 0.16),
        hex("#d9cfc2"),
    );
    b.put(
        Mesh::Ball,
        t.t(half - 0.22, h + 0.16, 0.18).s(0.24, 0.2, 0.24),
        hex("#4f8f4a"),
    );
    b.put(
        Mesh::Ball,
        t.t(half - 0.28, h + 0.24, 0.15).s(0.14, 0.14, 0.14),
        hex("#6aa85c"),
    );
}

/// A front-row office's low sideboard against its divider: binders in
/// the App's colours, a lamp-lit plant on top.
fn sideboard(b: &mut B<'_>, a: &Area<'_>, r: &mut Rng) {
    let rect = a.placed.rect;
    let t = T::new()
        .t(rect.x0 + 0.42, a.fy, rect.z1 - 1.6)
        .ry(std::f32::consts::FRAC_PI_2);
    b.bx(t, [-0.7, 0.0, -0.2], [1.4, 0.72, 0.4], hex("#e9e4dc"));
    b.bx(
        t,
        [-0.7, 0.72, -0.21],
        [1.4, 0.03, 0.42],
        ramp(a.kit.accent, -0.2),
    );
    let pal = family(a.kit.accent);
    let mut x = -0.62;
    while x < 0.15 {
        let w = r.range(0.06, 0.09);
        b.bx(
            t,
            [x, 0.75, -0.1],
            [w, r.range(0.22, 0.3), 0.22],
            pal[r.int(4)],
        );
        x += w + 0.01;
    }
    b.put(
        Mesh::Cyl8,
        t.t(0.42, 0.75, 0.0).s(0.2, 0.18, 0.2),
        hex("#d9cfc2"),
    );
    b.put(
        Mesh::Ball,
        t.t(0.42, 0.98, 0.0).s(0.3, 0.26, 0.3),
        hex("#4f8f4a"),
    );
    b.put(
        Mesh::Ball,
        t.t(0.34, 1.06, 0.05).s(0.18, 0.18, 0.18),
        hex("#6aa85c"),
    );
}

/// A ceiling strip across the office (by the back wall; in the front
/// row behind the sign), clear of the people below: glowing, with a
/// soft pool of light on the floor under it, while the office is lit;
/// dark once it's unused. The light, when it adds one.
fn strips(b: &mut B<'_>, a: &Area<'_>) -> Option<Light> {
    let rect = a.placed.rect;
    let lit = a.level == 0;
    // the back row's along its wall; the front row's well past its sign
    // over the way in, which a strip there would hide
    let z = if a.placed.row == 0 {
        rect.z0 + 0.7
    } else {
        rect.z0 + 2.6
    };
    let (x0, x1) = (rect.x0 + 0.9, rect.x1 - 0.9);
    let (x, len) = (f32::midpoint(x0, x1), (x1 - x0).max(1.0));
    let y = a.fy + WALL_H - STRIP_DROP;
    let t = T::new().t(x, y, z);
    for dx in [-len / 2.0 + 0.2, len / 2.0 - 0.2] {
        b.put(
            Mesh::Box,
            t.t(dx, 0.04, 0.0).s(0.012, STRIP_DROP, 0.012),
            hex("#2a2733"),
        );
    }
    b.put(
        Mesh::Box,
        t.t(0.0, -0.01, 0.0).s(len + 0.06, 0.03, 0.14),
        hex("#3a3644"),
    );
    if lit {
        b.glow(Mesh::Box, t.s(len, 0.05, 0.1), hex("#fff4d6"), 1.0);
        b.glass(
            Mesh::QuadUp,
            T::new().t(x, a.fy + 0.014, z + 0.5).s(len + 0.4, 1.0, 2.0),
            [255, 236, 190, 34],
            0,
        );
    } else {
        b.put(Mesh::Box, t.s(len, 0.05, 0.1), hex("#5b5866"));
    }
    (lit && !a.kit.pendants).then(|| {
        let w = a.kit.warmth;
        Light {
            floor: a.floor,
            p: [x, y - 0.4, z + 0.4, 4.8],
            c: [0.42 * w + 0.3, 0.42, 0.34 - 0.08 * w, 1.0],
            owner: 0,
        }
    })
}

/// A long-unused office's shade: its floor in the dusk of drawn blinds.
fn shade(b: &mut B<'_>, a: &Area<'_>) {
    let fr = a.placed.rect.inset(0.14);
    let alpha = if a.level >= 4 { 70 } else { 45 };
    b.glass(
        Mesh::QuadUp,
        T::new()
            .t(fr.cx(), a.fy + 0.012, fr.cz())
            .s(fr.w(), 1.0, fr.d()),
        [24, 20, 34, alpha],
        0,
    );
}

/// Cobwebs in the back corners, under the ceiling: threads fanning from
/// the corner and two rings across them.
fn cobwebs(b: &mut B<'_>, a: &Area<'_>) {
    let rect = a.placed.rect;
    let silk = [214, 210, 220, 255];
    let top = a.fy + WALL_H - 0.02;
    let (zb, zf) = if a.placed.row == 0 {
        (0.04, 1.0)
    } else {
        (rect.z1 - 0.04, -1.0)
    };
    for (cx, sx) in [(rect.x0 + 0.1, 1.0f32), (rect.x1 - 0.1, -1.0)] {
        let corner = T::new().t(cx, top, zb);
        for k in 0..4 {
            let ang = 0.15 + k as f32 * 0.42;
            let (dx, dz) = (ang.cos() * sx, ang.sin() * zf);
            let len = 0.75;
            b.glow(
                Mesh::Box,
                corner
                    .t(dx * len / 2.0, -0.18, dz * len / 2.0)
                    .ry(-dz.atan2(dx))
                    .rz(0.35)
                    .s(len, 0.008, 0.008),
                silk,
                0.3,
            );
        }
        for rr in [0.28f32, 0.52] {
            for k in 0..3 {
                let ang = 0.36 + k as f32 * 0.42;
                let (dx, dz) = (ang.cos() * sx * rr, ang.sin() * zf * rr);
                b.glow(
                    Mesh::Box,
                    corner
                        .t(dx, -0.12 - rr * 0.3, dz)
                        .ry((-ang.sin() * sx).atan2(ang.cos() * zf))
                        .s(0.008, 0.008, rr * 0.45),
                    silk,
                    0.3,
                );
            }
        }
    }
}
