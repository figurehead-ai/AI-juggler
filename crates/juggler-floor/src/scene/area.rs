//! An App's area: its floor and the tool's logo printed into it, the
//! divider and wall decor in its family's style, its nook, a desk for
//! each session (two spare), plants, pendants, and its sign and flag
//! over the way in. An annex is the same App's area continued.

use std::f32::consts::{FRAC_PI_2, PI};

use super::{
    DeskRec, Inputs, LOGO_OPACITY, Label, Light, PropKind, PropRec, Scene, Target, dimming,
    mark_letters,
};
use crate::colour::{Rgba, hex, mix, ramp};
use crate::family::{Family, Layout};
use crate::kit::{DeskKind, Divider, Kit, Nook, Plant, Pot, SignKind, WallFinish};
use crate::layout::{AISLE, AreaPlan, Placed, ROW_D, Rect, Size, WALL_H, plan_area, units};
use crate::math::T;
use crate::mesh::Mesh;
use crate::models::{
    self, B, Growth, Item, Look, Skin, chair, desk, desk_props, long_table, plant,
};
use crate::paint::{Img, text_width};
use crate::rng::{Rng, hash};
use crate::snapshot::{FloorApp, FloorSession};
use crate::state::FloorState;
use crate::tex::{self, Atlas, Full, SignStyle, Uv};

fn skin(uv: Uv, glow: f32, flags: u32) -> Skin {
    Skin {
        uv,
        repeat: [1.0, 1.0],
        glow,
        flags,
    }
}

/// One area being built: where, its App, its look and its desks' plan.
struct Area<'a> {
    placed: &'a Placed,
    app: &'a FloorApp,
    kit: Kit,
    plan: AreaPlan,
    seed: u32,
    floor: i32,
    fy: f32,
    dim: f32,
    level: u8,
    id: u32,
    logo: Option<&'a Img>,
}

impl Scene {
    pub(super) fn area(
        &mut self,
        out: &mut Vec<Item>,
        atlas: &mut Atlas,
        placed: &Placed,
        inputs: &Inputs<'_>,
    ) -> Result<(), Full> {
        let (Some(app), Some(kit)) = (
            self.apps.get(&placed.id).cloned(),
            self.kits.get(&placed.id).cloned(),
        ) else {
            return Ok(());
        };
        let seed = hash(&placed.id)
            ^ u32::try_from(placed.annex)
                .unwrap_or(0)
                .wrapping_mul(0x9e37_79b9);
        let level = self.unused.get(&placed.id).copied().unwrap_or(0);
        let live = app
            .sessions
            .iter()
            .filter(|s| s.state != FloorState::Ended)
            .count();
        let family = inputs
            .office
            .apps
            .get(&placed.id)
            .map_or(Family::Neutral, |a| a.family);
        let detail = format!("{live} sessions · {}", family.name());
        let id = self.pick(Target::App(app.id.clone()), app.name.clone(), detail);
        let area = Area {
            placed,
            plan: plan_area(
                placed.rect,
                placed.size,
                kit.layout,
                kit.jitter,
                seed,
                placed.row,
            ),
            kit,
            app: &app,
            seed,
            floor: placed.floor as i32,
            fy: super::floor_y(placed.floor),
            dim: dimming(level),
            level,
            id,
            logo: inputs.logos.get(&placed.id),
        };
        area_floor(out, atlas, &area)?;
        area_edges(out, atlas, &area)?;
        if let Some(nook) = area.plan.nook {
            self.nook(out, atlas, &area, nook)?;
        }
        let desks = desks_of(inputs, placed);
        self.desks_of_area(out, &area, &desks);
        let mut r = Rng(seed);
        let wilt = if level >= super::COVERED { 0.7 } else { 0.0 };
        for (k, (x, z)) in area.plan.plants.iter().enumerate().take(area.kit.greenery) {
            let kind = area.kit.plants[k % area.kit.plants.len().max(1)];
            let s = r.range(1.0, 1.35);
            self.add_plant(
                area.floor,
                T::new().t(*x, area.fy, *z),
                kind,
                area.kit.pot,
                s,
                seed ^ (k as u32 * 31),
                wilt,
            );
        }
        self.pendants(out, &area, desks.len());
        whiteboard(out, atlas, &area)?;
        {
            self.sign(out, atlas, &area)?;
        }
        Ok(())
    }

    /// The desks: one per seat, two spare; a dusty few when unused.
    fn desks_of_area(&mut self, out: &mut Vec<Item>, a: &Area<'_>, seats: &[Option<String>]) {
        let last = seats.iter().rposition(Option::is_some);
        let shown = if a.level > 0 {
            4
        } else {
            last.map_or(2, |l| l + 3)
        }
        .min(a.plan.spots.len());
        if a.plan.layout == Layout::Tables {
            let mut b = B {
                out: &mut *out,
                floor: a.floor,
                id: a.id,
                dim: a.dim,
            };
            for (gi, g) in a.plan.groups.iter().enumerate() {
                if a.plan.spots.iter().take(shown).any(|s| s.group == gi) {
                    long_table(
                        &mut b,
                        T::new().t(g.x, a.fy, g.z).ry(g.turn),
                        4.2,
                        a.kit.desk_top,
                        a.kit.desk_frame,
                    );
                }
            }
        }
        if a.plan.layout == Layout::Radial {
            for (gi, g) in a.plan.groups.iter().enumerate() {
                if a.plan.spots.iter().take(shown).any(|s| s.group == gi) {
                    let kind = a.kit.plants[gi % a.kit.plants.len().max(1)];
                    let wilt = if a.level >= super::COVERED { 0.7 } else { 0.0 };
                    self.add_plant(
                        a.floor,
                        T::new().t(g.x, a.fy, g.z),
                        kind,
                        a.kit.pot,
                        1.1,
                        a.seed ^ gi as u32,
                        wilt,
                    );
                }
            }
        }
        let account_out = self
            .account(a.app.account_id.as_deref())
            .is_some_and(|acc| acc.out);
        let busiest = a.app.hot_since_ms.and_then(|_| {
            seats
                .iter()
                .flatten()
                .filter_map(|k| a.app.sessions.iter().find(|s| &s.key == k))
                .max_by(|x, y| x.cpu_pct.total_cmp(&y.cpu_pct))
                .map(|s| s.key.clone())
        });
        for (si, spot) in a.plan.spots.iter().take(shown).enumerate() {
            let session = seats
                .get(si)
                .and_then(Option::as_ref)
                .and_then(|k| a.app.sessions.iter().find(|s| &s.key == k));
            let frame = T::new().t(spot.x, a.fy, spot.z).ry(spot.face);
            self.one_desk(
                out,
                a,
                (si, spot.shared),
                frame,
                session.filter(|_| a.level == 0),
                account_out,
                busiest.as_deref(),
            );
        }
    }

    #[allow(clippy::too_many_arguments)] // one desk and everything on it
    fn one_desk(
        &mut self,
        out: &mut Vec<Item>,
        a: &Area<'_>,
        (si, shared): (usize, bool),
        frame: T,
        session: Option<&FloorSession>,
        account_out: bool,
        busiest: Option<&str>,
    ) {
        let (id, helper_id) = match session {
            Some(s) => {
                let id = self.pick(
                    Target::Session(s.key.clone()),
                    s.title.clone(),
                    format!("{} · {}", s.word, a.app.name),
                );
                let helpers = self.pick(
                    Target::Helpers(s.key.clone()),
                    format!("{} subagents", s.subagents),
                    "working for this session",
                );
                (id, helpers)
            }
            None => (a.id, 0),
        };
        let mut b = B {
            out: &mut *out,
            floor: a.floor,
            id,
            dim: a.dim,
        };
        let (top, width) = desk_of(&mut b, &a.kit, frame, shared);
        let high = a.kit.desk == DeskKind::Standing;
        let mut dr = Rng(a.seed ^ (si as u32).wrapping_mul(2_654_435_761));
        let props_at = frame.t(0.0, 0.0, if shared { 0.1 } else { 0.0 });
        if let Some(p) = desk_props(
            &mut b,
            props_at,
            top,
            &mut dr,
            a.kit.accent,
            a.kit.lamps && session.is_some() && !shared,
            width,
        ) {
            self.lights.push(Light {
                floor: a.floor,
                p: [p[0], p[1], p[2], 2.2],
                c: [0.55, 0.45, 0.28, 1.0],
                owner: id,
            });
        }
        if a.level >= super::COVERED && self.covers {
            sheet(&mut b, frame, top, width, &mut dr);
        } else if a.level >= super::COVERED {
            clutter(&mut b, frame, top, &mut dr);
        }
        let seat_at = if session.is_some() {
            frame.t(0.0, 0.0, 0.62)
        } else if a.level >= 2 && self.covers {
            // AJ-E3518: closed for over an hour, the chairs pushed in
            frame.t(0.0, 0.0, 0.52)
        } else {
            frame
                .t(dr.range(-0.3, 0.3), 0.0, 0.75 + dr.range(0.0, 0.3))
                .ry(dr.range(-0.6, 0.6))
        };
        let seat = chair(&mut b, seat_at, a.kit.chair, a.kit.chair_colour, high);
        let Some(session) = session else {
            models::screens(
                &mut b,
                frame,
                a.kit.screens,
                top,
                self.pictures.off,
                0.0,
                -1.0,
            );
            return;
        };
        if busiest == Some(session.key.as_str()) {
            self.fires.push((a.floor, frame.t(0.45, top, -0.1)));
        }
        let state = match super::drawn_state(session, self.now_ms) {
            FloorState::Working | FloorState::Idle | FloorState::Done if account_out => {
                FloorState::Limit
            }
            other => other,
        };
        let mut lr = Rng(a.seed ^ (si as u32 + 77).wrapping_mul(97));
        self.desks.push(DeskRec {
            session: session.clone(),
            state,
            desktop: a.app.kind == crate::snapshot::FloorAppKind::Desktop,
            floor: a.floor,
            frame,
            seat,
            top,
            screens: a.kit.screens,
            look: Look::random(&mut lr, a.kit.accent),
            id,
            helper_id,
            accent: a.kit.accent,
            shared,
            width,
            phase: (hash(&session.key) % 1000) as f32 / 37.0,
            smoke: super::smoking(session, self.now_ms),
        });
    }

    /// Pendants over the desk groups in use.
    fn pendants(&mut self, out: &mut Vec<Item>, a: &Area<'_>, seats: usize) {
        if !a.kit.pendants || a.level > 0 {
            return;
        }
        let per = a.plan.groups.first().map_or(1, |g| g.seats.max(1));
        let mut b = B {
            out,
            floor: a.floor,
            id: a.id,
            dim: a.dim,
        };
        for g in a.plan.groups.iter().take(seats.max(1).div_ceil(per)) {
            let t = T::new().t(g.x, a.fy + WALL_H - 0.7, g.z);
            b.put(
                Mesh::Box,
                t.t(0.0, 0.2, 0.0).s(0.01, 0.6, 0.01),
                hex("#2a2733"),
            );
            if a.kit.family == Family::Industrial {
                b.put(Mesh::Cyl8, t.s(0.12, 0.2, 0.12), hex("#1d1a24"));
                b.glow(
                    Mesh::Ball,
                    t.t(0.0, -0.16, 0.0).s(0.11, 0.16, 0.11),
                    hex("#ffd38a"),
                    1.0,
                );
            } else {
                b.put(Mesh::Cone8, t.s(0.5, 0.25, 0.5), a.kit.accent);
                b.glow(
                    Mesh::Ball,
                    t.t(0.0, -0.06, 0.0).s(0.1, 0.1, 0.1),
                    hex("#fff1c8"),
                    1.0,
                );
            }
            let w = a.kit.warmth;
            self.lights.push(Light {
                floor: a.floor,
                p: [g.x, a.fy + WALL_H - 1.0, g.z, 4.2],
                c: [0.5 * w + 0.2, 0.42, 0.3 - 0.1 * w, 1.0],
                owner: 0,
            });
        }
    }

    /// The sign over the way in, readable from both sides, with its
    /// Account's battery (moving part); the flag in the App's colour at
    /// the corner.
    fn sign(&mut self, out: &mut Vec<Item>, atlas: &mut Atlas, a: &Area<'_>) -> Result<(), Full> {
        let (ex, ez) = a.plan.entrance;
        let ez = if a.placed.row == 0 && a.placed.size != Size::L {
            ez - 0.2
        } else if a.placed.row == 1 {
            ez + 0.2
        } else {
            ROW_D + 0.15
        };
        let style = match a.kit.sign {
            SignKind::Neon => SignStyle::Neon,
            SignKind::Banner => SignStyle::Light,
            SignKind::Plate | SignKind::Pennant | SignKind::Blade => SignStyle::Dark,
        };
        let name = if a.placed.annex == 0 {
            a.app.name.to_uppercase()
        } else {
            format!("{} · ANNEX", a.app.name.to_uppercase())
        };
        let face = atlas.add(
            &format!("sign:{}:{}", a.app.id, a.placed.annex),
            &tex::sign_img(&mark_letters(&a.app.name), &name, a.kit.accent, style),
        )?;
        let (sw, sh) = ((text_width(&name, 1) + 22) as f32 * 0.024, 13.0 * 0.024);
        let fy = a.fy;
        let mut b = B {
            out,
            floor: a.floor,
            id: a.id,
            dim: a.dim,
        };
        let t = T::new()
            .t(ex, fy + 2.3, ez)
            .ry(if a.kit.sign == SignKind::Blade {
                0.35
            } else {
                0.0
            });
        let glow = if a.kit.sign == SignKind::Neon {
            0.95
        } else {
            0.35
        };
        for s in [-1.0f32, 1.0] {
            b.put(
                Mesh::Box,
                t.t(s * sw * 0.42, sh + 0.04, 0.0)
                    .s(0.012, WALL_H - 2.3 - sh, 0.012),
                hex("#2a2733"),
            );
        }
        let board = match a.kit.sign {
            SignKind::Banner => a.kit.accent,
            SignKind::Neon => hex("#141420"),
            SignKind::Plate | SignKind::Pennant | SignKind::Blade => hex("#2a2733"),
        };
        b.put(
            Mesh::Box,
            t.t(0.0, -0.03, 0.0).s(sw + 0.08, sh + 0.06, 0.05),
            board,
        );
        for side in [0.0, PI] {
            b.tex(
                Mesh::QuadWall,
                t.ry(side).t(0.0, 0.0, 0.026).s(sw, sh, 1.0),
                [255; 4],
                skin(face, glow, 0),
            );
        }
        sign_extras(&mut b, &a.kit, t, sw, [ex, fy, ez]);
        flag(&mut b, a);
        if a.kit.sign == SignKind::Neon {
            let c = a.kit.accent.map(|v| f32::from(v) / 400.0);
            self.lights.push(Light {
                floor: a.floor,
                p: [ex, fy + 2.3, ez, 3.0],
                c: [c[0], c[1], c[2], 1.0],
                owner: 0,
            });
        }
        self.signs
            .push((a.floor, t.t(0.0, -0.13, 0.0), a.app.account_id.clone()));
        self.beacons.push((a.floor, [ex, fy + 2.9, ez]));
        if a.placed.annex == 0 {
            self.labels.push(Label {
                floor: a.placed.floor,
                at: [ex, fy + 3.3, ez],
                app: a.app.id.clone(),
                id: a.id,
            });
        }
        Ok(())
    }

    /// The area's nook, in its family's kind.
    #[allow(clippy::too_many_lines)] // one nook per kind, side by side
    fn nook(
        &mut self,
        out: &mut Vec<Item>,
        atlas: &mut Atlas,
        a: &Area<'_>,
        nk: Rect,
    ) -> Result<(), Full> {
        let (cx, cz, fy) = (nk.cx(), nk.cz(), a.fy);
        let soft = mix(a.kit.chair_colour, hex("#8a7a6a"), 0.35);
        let ink = hex("#2a2733");
        let mut b = B {
            out: &mut *out,
            floor: a.floor,
            id: a.id,
            dim: a.dim,
        };
        match a.kit.nook {
            Nook::Sofa => {
                let t = T::new().t(cx, fy, cz - 0.6);
                b.cb(t, [-1.1, 0.0, -0.45], [2.2, 0.42, 0.9], soft);
                b.cb(t, [-1.1, 0.42, -0.45], [2.2, 0.45, 0.25], soft);
                for s in [-1.0f32, 1.0] {
                    b.cb(
                        t,
                        [s * 1.0 - 0.15, 0.0, -0.45],
                        [0.3, 0.62, 0.9],
                        ramp(soft, -0.1),
                    );
                }
                for k in 0..3 {
                    b.cb(
                        t,
                        [-0.9 + k as f32 * 0.62, 0.42, -0.2],
                        [0.56, 0.14, 0.55],
                        ramp(soft, 0.12),
                    );
                }
                b.put(
                    Mesh::Cyl12,
                    T::new().t(cx, fy + 0.4, cz + 0.6).s(0.9, 0.04, 0.6),
                    a.kit.desk_top,
                );
                b.put(
                    Mesh::Cyl8,
                    T::new().t(cx, fy, cz + 0.6).s(0.08, 0.4, 0.08),
                    ink,
                );
                let lamp = T::new().t(cx + 1.4, fy, cz - 0.6);
                b.put(Mesh::Cyl8, lamp.s(0.3, 0.03, 0.3), ink);
                b.put(Mesh::Cyl6, lamp.s(0.03, 1.5, 0.03), ink);
                b.put(
                    Mesh::Pot,
                    lamp.t(0.0, 1.45, 0.0).s(0.42, 0.3, 0.42),
                    hex("#f1e6d6"),
                );
                self.lights.push(Light {
                    floor: a.floor,
                    p: [cx + 1.4, fy + 1.5, cz - 0.6, 3.5],
                    c: [0.5, 0.38, 0.22, 1.0],
                    owner: 0,
                });
            }
            Nook::Library => {
                let books = atlas.add(&format!("books:{}", a.app.id), &tex::books_img(a.seed))?;
                for (k, dx) in [-0.8f32, 0.8].into_iter().enumerate() {
                    let t = T::new().t(cx + dx, fy, nk.z0 + 0.25);
                    b.put(Mesh::Box, t.s(1.5, 2.0, 0.4), hex("#6b4a32"));
                    for shelf in 0..4 {
                        let place = t
                            .t(0.0, 0.12 + shelf as f32 * 0.47, 0.201)
                            .s(1.38, 0.38, 1.0);
                        b.tex(
                            Mesh::QuadWall,
                            place,
                            [255; 4],
                            Skin {
                                uv: books,
                                repeat: [1.0 + k as f32 * 0.3, 1.0],
                                glow: 0.0,
                                flags: 0,
                            },
                        );
                    }
                }
                let seat = T::new().t(cx - 0.5, fy, cz + 0.6).ry(PI + 0.5);
                b.cb(seat, [-0.45, 0.0, -0.4], [0.9, 0.42, 0.8], soft);
                b.cb(seat, [-0.45, 0.42, 0.2], [0.9, 0.5, 0.2], soft);
                for s in [-1.0f32, 1.0] {
                    b.cb(
                        seat,
                        [s * 0.42 - 0.1, 0.0, -0.4],
                        [0.2, 0.62, 0.8],
                        ramp(soft, -0.1),
                    );
                }
                let rug = atlas.add(
                    &format!("nookrug:{}", a.app.id),
                    &tex::rug_img(a.kit.rug, a.kit.accent, 3),
                )?;
                b.tex(
                    Mesh::Box,
                    T::new().t(cx, fy + 0.008, cz + 0.4).s(2.4, 0.01, 1.8),
                    [255; 4],
                    skin(rug, 0.0, 0),
                );
            }
            Nook::Huddle => {
                let t = T::new().t(cx, fy, cz);
                desk(
                    &mut b,
                    t,
                    DeskKind::Round,
                    a.kit.desk_top,
                    a.kit.desk_frame,
                    1.1,
                );
                for k in 0..4 {
                    chair(
                        &mut b,
                        t.ry(k as f32 * FRAC_PI_2 + 0.4).t(0.0, 0.0, 0.85),
                        a.kit.chair,
                        a.kit.chair_colour,
                        false,
                    );
                }
                let board = atlas.add(&format!("wb:{}", a.app.id), &tex::board_img(a.seed ^ 5))?;
                let s = T::new().t(cx, fy, nk.z0 + 0.2);
                for x in [-0.7f32, 0.7] {
                    b.put(
                        Mesh::Box,
                        s.t(x, 0.0, 0.0).s(0.04, 1.8, 0.04),
                        hex("#9aa0a8"),
                    );
                }
                b.tex(
                    Mesh::Box,
                    s.t(0.0, 0.85, 0.0).s(1.5, 0.9, 0.03),
                    [255; 4],
                    skin(board, 0.0, 0),
                );
            }
            Nook::Workbench => {
                let t = T::new().t(cx, fy, nk.z0 + 0.5);
                b.bx(t, [-1.3, 0.85, -0.4], [2.6, 0.08, 0.8], hex("#c9a46c"));
                for (x, z) in [(-1.2, -0.3), (1.2, -0.3), (-1.2, 0.3), (1.2, 0.3)] {
                    b.bx(
                        t,
                        [x - 0.04, 0.0, z - 0.04],
                        [0.08, 0.85, 0.08],
                        hex("#3a3a3e"),
                    );
                }
                b.bx(t, [-1.25, 0.2, -0.35], [2.5, 0.04, 0.7], hex("#8a6a42"));
                let peg = atlas.add("peg", &tex::pegboard_img())?;
                b.tex(
                    Mesh::Box,
                    t.t(0.0, 0.93, -0.38).s(2.6, 1.1, 0.03),
                    [255; 4],
                    Skin {
                        uv: peg,
                        repeat: [3.0, 1.4],
                        glow: 0.0,
                        flags: 0,
                    },
                );
                for (k, c) in [
                    "#c84a3a", "#3a8fb0", "#e0a03a", "#2a2b30", "#9aa0a8", "#4a7a5a",
                ]
                .into_iter()
                .enumerate()
                {
                    b.put(
                        Mesh::Box,
                        t.t(-1.0 + k as f32 * 0.4, 1.2, -0.35).s(0.05, 0.3, 0.03),
                        hex(c),
                    );
                }
                b.put(
                    Mesh::Box,
                    t.t(0.5, 0.93, 0.0).s(0.3, 0.2, 0.25),
                    hex("#c84a3a"),
                );
                b.put(
                    Mesh::Cyl8,
                    t.t(-0.5, 0.93, 0.1).s(0.15, 0.35, 0.15),
                    hex("#3a8fb0"),
                );
                let bike = T::new().t(cx + 0.4, fy, cz + 1.0).ry(0.3);
                for s in [-0.5f32, 0.5] {
                    b.put(
                        Mesh::Cyl12,
                        bike.t(s, 0.33, 0.0).rx(FRAC_PI_2).s(0.66, 0.04, 0.66),
                        hex("#1f1f24"),
                    );
                }
                b.put(
                    Mesh::Box,
                    bike.t(0.0, 0.35, 0.0).rz(-0.5).s(0.04, 0.6, 0.04),
                    a.kit.accent,
                );
                b.put(
                    Mesh::Box,
                    bike.t(-0.1, 0.62, 0.0).rz(FRAC_PI_2).s(0.04, 0.6, 0.04),
                    a.kit.accent,
                );
            }
            Nook::Beanbags => {
                b.put(
                    Mesh::Cyl12,
                    T::new().t(cx, fy, cz + 0.1).s(0.6, 0.3, 0.6),
                    hex("#e6e1d8"),
                );
                for (k, c) in ["#d9644a", "#3f7fbf", "#e8b33a"].into_iter().enumerate() {
                    let id = self.pick(Target::Prop(self.props.len()), "A beanbag", "flop into it");
                    let t = T::new().t(
                        cx - 1.0 + k as f32,
                        fy,
                        cz + if k == 1 { -0.6 } else { 0.4 },
                    );
                    self.props.push(PropRec {
                        kind: PropKind::Beanbag(hex(c)),
                        floor: a.floor,
                        t,
                        id,
                    });
                }
            }
            Nook::PingPong => {
                let id = self.pick(Target::Prop(self.props.len()), "Ping-pong", "play a rally");
                self.props.push(PropRec {
                    kind: PropKind::PingPong,
                    floor: a.floor,
                    t: T::new().t(cx, fy, cz),
                    id,
                });
            }
            Nook::Arcade => {
                b.cb(
                    T::new().t(cx + 0.8, fy, cz + 0.4),
                    [-0.6, 0.0, -0.4],
                    [1.2, 0.4, 0.8],
                    hex("#2a2b30"),
                );
                let id = self.pick(
                    Target::Prop(self.props.len()),
                    "An arcade cabinet",
                    "insert coin",
                );
                self.props.push(PropRec {
                    kind: PropKind::Arcade,
                    floor: a.floor,
                    t: T::new().t(cx - 0.6, fy, nk.z0 + 0.5),
                    id,
                });
            }
        }
        Ok(())
    }
}

/// The area's floor, inset so the common floor shows as a border; the
/// tool's logo with the Account's initials printed into it,
/// see-through; a rug under the first group.
fn area_floor(out: &mut Vec<Item>, atlas: &mut Atlas, a: &Area<'_>) -> Result<(), Full> {
    let rect = a.placed.rect;
    let tile = tex::floor_tile(&a.kit.floor);
    let finish = atlas.add(
        &format!("floor:{}", a.app.id),
        &tex::floor_img(&a.kit.floor, a.seed),
    )?;
    let mut b = B {
        out,
        floor: a.floor,
        id: a.id,
        dim: a.dim,
    };
    let fr = rect.inset(0.12);
    b.with(a.kit.floor.mat(), |b| {
        b.tex(
            Mesh::QuadUp,
            T::new()
                .t(fr.cx(), a.fy + 0.004, fr.cz())
                .s(fr.w(), 1.0, fr.d()),
            [255; 4],
            Skin {
                uv: finish,
                repeat: [fr.w() / tile, fr.d() / tile],
                glow: 0.0,
                flags: 0,
            },
        );
    });
    if a.placed.size == Size::L {
        b.put(
            Mesh::QuadUp,
            T::new()
                .t(rect.cx(), a.fy + 0.006, ROW_D + AISLE / 2.0)
                .s(rect.w(), 1.0, AISLE),
            hex("#cfc8bb"),
        );
    }
    if a.placed.annex == 0 {
        let logo = match a.logo {
            Some(drawn) => atlas.add(&format!("logo:{}", a.app.id), &drawn.scaled(128, 128))?,
            None => atlas.add(
                &format!("logo:{}", a.app.id),
                &tex::logo_img(&mark_letters(&a.app.name), &a.app.initials, a.kit.accent),
            )?,
        };
        let side = (rect.w().min(rect.d()) * 0.62).min(6.5);
        let lz = if a.placed.size == Size::L && a.placed.row == 0 {
            rect.z0 + ROW_D * 0.5
        } else {
            rect.cz()
        };
        let see_through = [255, 255, 255, (255.0 * LOGO_OPACITY) as u8];
        b.out.push(models::Item {
            mesh: Mesh::QuadUp,
            floor: a.floor,
            glass: true,
            inst: models::Inst {
                m: T::new()
                    .t(rect.cx(), a.fy + 0.012, lz)
                    .s(side, 1.0, side)
                    .0
                    .0,
                col: models::rgba(see_through),
                tex: logo,
                misc: [1.0, 1.0, 0.0, models::TEXTURED as f32],
                off: [0.0; 2],
                id: a.id,
                mat: crate::materials::NONE,
            },
        });
    }
    if a.kit.rug != hex("#2a2d33")
        && let Some(g) = a.plan.groups.first()
    {
        let rug = atlas.add(
            &format!("rug:{}:{}", a.app.id, a.placed.annex),
            &tex::rug_img(a.kit.rug, a.kit.accent, a.seed),
        )?;
        b.with(crate::materials::CARPET, |b| {
            b.tex(
                Mesh::Box,
                T::new()
                    .t(g.x, a.fy + 0.008, g.z)
                    .ry(g.turn)
                    .s(3.6, 0.01, 2.8),
                [255; 4],
                skin(rug, 0.0, 0),
            );
        });
    }
    Ok(())
}

/// The divider towards the area on the left (or the lobby), and decor
/// on the back wall.
fn area_edges(out: &mut Vec<Item>, atlas: &mut Atlas, a: &Area<'_>) -> Result<(), Full> {
    let rect = a.placed.rect;
    let mut r = Rng(a.seed ^ 0x51);
    let (z0, z1) = (rect.z0 + 0.3, rect.z1 - 0.3);
    let mut runs = Vec::new();
    if a.placed.size == Size::L {
        runs.push((z0, ROW_D - 1.2));
        runs.push((ROW_D + AISLE + 1.2, z1));
    } else if a.placed.row == 0 {
        runs.push((z0, z1 - 1.3));
    } else {
        runs.push((z0 + 1.3, z1));
    }
    let mut b = B {
        out,
        floor: a.floor,
        id: a.id,
        dim: a.dim,
    };
    for (from, to) in runs {
        divider(
            &mut b,
            &a.kit,
            (rect.x0 + 0.06, a.fy),
            (from, to),
            &mut r,
            atlas,
        )?;
    }
    if a.placed.row == 0 {
        let poster = atlas.add(
            &format!("poster:{}", a.app.id),
            &tex::poster_img(a.seed, a.kit.accent),
        )?;
        let board = atlas.add(&format!("board:{}", a.app.id), &tex::board_img(a.seed))?;
        let mut x = rect.x0 + 1.4;
        while x < rect.x1 - 1.2 {
            let t = T::new().t(x, a.fy, 0.03);
            match r.int(3) {
                0 => b.tex(
                    Mesh::QuadWall,
                    t.t(0.0, 1.35, 0.0).s(0.6, 0.8, 1.0),
                    [255; 4],
                    skin(poster, 0.0, models::CUTAWAY),
                ),
                1 => b.tex(
                    Mesh::QuadWall,
                    t.t(0.0, 1.0, 0.0).s(1.4, 0.82, 1.0),
                    [255; 4],
                    skin(board, 0.0, models::CUTAWAY),
                ),
                _ => {}
            }
            x += 2.4;
        }
    }
    Ok(())
}

/// The desk at `frame` (none for a seat at a shared table, whose table
/// is the group's): its top's height and its width.
fn desk_of(b: &mut B, kit: &Kit, frame: T, shared: bool) -> (f32, f32) {
    if shared {
        return (models::DESK_H, 1.2);
    }
    let w = if kit.desk == DeskKind::Round {
        1.0
    } else {
        1.4
    };
    (desk(b, frame, kit.desk, kit.desk_top, kit.desk_frame, w), w)
}

/// 0.4.1: a dust sheet over a long-unused desk and its chair's back:
/// pale cloth, a little crumpled, hanging to just above the floor, and a
/// box left by it.
fn sheet(b: &mut B, frame: T, top: f32, width: f32, dr: &mut Rng) {
    let cloth = hex("#d8d3c7");
    let fold = hex("#c4bfb2");
    let (w, d) = (width + 0.16, 0.86);
    // the drape: the top, then the skirt to just above the floor
    b.cb(
        frame.t(0.0, top - 0.02, 0.0).ry(dr.range(-0.04, 0.04)),
        [-w / 2.0, 0.0, -d / 2.0],
        [w, 0.55, d],
        cloth,
    );
    b.cb(
        frame.t(0.0, 0.1, 0.0),
        [-w / 2.0 + 0.02, 0.0, -d / 2.0 + 0.02],
        [w - 0.04, top - 0.1, d - 0.04],
        fold,
    );
    for k in 0..3 {
        let x = -w / 2.0 + w * (0.25 + 0.25 * k as f32);
        b.put(
            Mesh::Box,
            frame
                .t(x, top * 0.5, d / 2.0 - 0.01)
                .s(0.05, top * 0.9, 0.02),
            fold,
        );
    }
    b.cb(
        frame
            .t(dr.range(-0.8, 0.8), 0.0, 0.95)
            .ry(dr.range(-0.6, 0.6)),
        [-0.28, 0.0, -0.22],
        [0.56, 0.36, 0.44],
        hex("#b58a5a"),
    );
}

/// A long-unused desk's boxes and dust, and a box on the floor.
fn clutter(b: &mut B, frame: T, top: f32, dr: &mut Rng) {
    let cardboard = hex("#b58a5a");
    b.cb(
        frame.t(0.2, top, -0.05).ry(0.3),
        [-0.25, 0.0, -0.2],
        [0.5, 0.32, 0.4],
        cardboard,
    );
    for _ in 0..5 {
        let at = frame.t(dr.range(-0.6, 0.6), top + 0.002, dr.range(-0.3, 0.3));
        b.put(Mesh::QuadUp, at.s(0.08, 1.0, 0.06), hex("#b8b2a6"));
    }
    b.cb(
        frame.t(-1.0, 0.0, 0.9).ry(0.8),
        [-0.3, 0.0, -0.25],
        [0.6, 0.4, 0.5],
        cardboard,
    );
}

/// A banner's tail or a string of pennants, by the sign's kind.
fn sign_extras(b: &mut B, kit: &Kit, t: T, sw: f32, [ex, fy, ez]: [f32; 3]) {
    let ink = hex("#2a2733");
    match kit.sign {
        SignKind::Banner => {
            b.put(
                Mesh::Box,
                t.t(0.0, -0.75, 0.0).s(sw * 0.3, 0.72, 0.02),
                kit.accent,
            );
            b.put(
                Mesh::Cyl6,
                t.t(-sw * 0.2, -0.78, 0.0)
                    .rz(-FRAC_PI_2)
                    .s(0.03, sw * 0.4, 0.03),
                ink,
            );
        }
        SignKind::Pennant => {
            for (k, c) in ["#d9644a", "#3f7fbf", "#e8b33a"]
                .iter()
                .cycle()
                .take(9)
                .enumerate()
            {
                let at = T::new().t(ex - 2.6 + k as f32 * 0.65, fy + 2.9, ez);
                b.put(Mesh::Cone8, at.rx(PI).s(0.3, 0.38, 0.03), hex(c));
            }
            b.put(
                Mesh::Box,
                T::new().t(ex, fy + 2.9, ez).s(5.4, 0.01, 0.01),
                ink,
            );
        }
        SignKind::Plate | SignKind::Neon | SignKind::Blade => {}
    }
}

/// The flag in the App's colour at the corner of the way in, seen from
/// anywhere.
fn flag(b: &mut B, a: &Area<'_>) {
    let (rect, fy) = (a.placed.rect, a.fy);
    let fx = rect.x0 + 0.45;
    let fz = if a.placed.row == 0 && a.placed.size != Size::L {
        rect.z1 - 0.45
    } else {
        rect.z0 + 0.45
    };
    b.put(
        Mesh::Cyl6,
        T::new().t(fx, fy, fz).s(0.05, 3.2, 0.05),
        hex("#d8d2c6"),
    );
    b.put(
        Mesh::Ball,
        T::new().t(fx, fy + 3.18, fz).s(0.08, 0.08, 0.08),
        hex("#ffd27a"),
    );
    b.glow(
        Mesh::Box,
        T::new().t(fx + 0.36, fy + 2.62, fz).s(0.7, 0.48, 0.02),
        a.kit.accent,
        0.25,
    );
}

/// The App's whiteboard on its back wall (or, in the front row, a
/// stand by the way in): its open tasks in a count, SPEC §9.7's
/// "Tasks" signal. A click opens the App.
fn whiteboard(out: &mut Vec<Item>, atlas: &mut Atlas, a: &Area<'_>) -> Result<(), Full> {
    if a.placed.annex != 0 {
        return Ok(());
    }
    let open: u32 = a.app.sessions.iter().map(|s| s.tasks).sum();
    let words = if open == 1 {
        "1 TASK".to_owned()
    } else {
        format!("{open} TASKS")
    };
    let ink = if open == 0 {
        hex("#9aa0a8")
    } else {
        hex("#2f6fbf")
    };
    let face = atlas.add(
        &format!("tasks:{}", a.app.id),
        &tex::text_img(&[(&words, ink)], hex("#f4f4f0"), 3),
    )?;
    let rect = a.placed.rect;
    let mut b = B {
        out,
        floor: a.floor,
        id: a.id,
        dim: a.dim,
    };
    let (w, h) = (1.2, 0.5);
    if a.placed.row == 0 {
        let t = T::new().t(rect.x1 - 1.2, a.fy + 1.55, 0.04);
        b.cut(Mesh::Box, t.s(w + 0.08, h + 0.08, 0.03), hex("#9aa0a8"));
        b.tex(
            Mesh::QuadWall,
            t.t(0.0, 0.04, 0.02).s(w, h, 1.0),
            [255; 4],
            skin(face, 0.2, models::CUTAWAY),
        );
    } else {
        let t = T::new().t(rect.x1 - 0.9, a.fy, rect.z0 + 0.6).ry(PI);
        for x in [-0.55f32, 0.55] {
            b.put(
                Mesh::Box,
                t.t(x, 0.0, 0.0).s(0.04, 1.5, 0.04),
                hex("#9aa0a8"),
            );
        }
        b.put(
            Mesh::Box,
            t.t(0.0, 1.0, 0.0).s(w + 0.08, h + 0.08, 0.03),
            hex("#9aa0a8"),
        );
        for side in [0.0, PI] {
            b.tex(
                Mesh::QuadWall,
                t.t(0.0, 1.04, 0.0).ry(side).t(0.0, 0.0, 0.02).s(w, h, 1.0),
                [255; 4],
                skin(face, 0.2, 0),
            );
        }
    }
    Ok(())
}

/// The seats of `placed` (its slice of its App's desks: an annex's come
/// after the main area's).
fn desks_of(inputs: &Inputs<'_>, placed: &Placed) -> Vec<Option<String>> {
    let Some(all) = inputs.seats.get(&placed.id) else {
        return Vec::new();
    };
    let sizes = inputs
        .office
        .apps
        .get(&placed.id)
        .map(|a| units(a.desks))
        .unwrap_or_default();
    let start: usize = sizes.iter().take(placed.annex).map(|s| s.capacity()).sum();
    let end = (start + placed.size.capacity()).min(all.len());
    all.get(start.min(end)..end)
        .map(<[_]>::to_vec)
        .unwrap_or_default()
}

/// The edge between an area and its left neighbour (or the lobby), in
/// the area's own style.
#[allow(clippy::too_many_lines)] // one divider per kind, side by side
fn divider(
    b: &mut B,
    kit: &Kit,
    (x, fy): (f32, f32),
    (z0, z1): (f32, f32),
    r: &mut Rng,
    atlas: &mut Atlas,
) -> Result<(), Full> {
    let len = z1 - z0;
    if len < 0.5 {
        return Ok(());
    }
    let t = T::new().t(x, fy, f32::midpoint(z0, z1)).ry(FRAC_PI_2);
    let metal = hex("#9aa0a8");
    match kit.divider {
        Divider::Planters => {
            let n = len.floor().max(1.0) as usize;
            let trough: Rgba = match kit.pot {
                Pot::Box(c) | Pot::Ceramic(c) => c,
                Pot::Terracotta | Pot::Basket | Pot::Concrete => hex("#6a6a68"),
            };
            for k in 0..n {
                let p = t.t(
                    -len / 2.0 + 0.5 + k as f32 * (len - 1.0) / (n.max(2) - 1) as f32,
                    0.0,
                    0.0,
                );
                b.bx(p, [-0.45, 0.0, -0.22], [0.9, 0.42, 0.44], trough);
                let kind = if k % 2 == 0 {
                    Plant::Snake
                } else {
                    Plant::Fern
                };
                plant(
                    b,
                    p,
                    kind,
                    Pot::Box(hex("#3a3a3a")),
                    Growth {
                        s: 0.75,
                        seed: r.int(1000) as u32,
                        sway: 0.0,
                        wilt: 0.0,
                    },
                );
            }
        }
        Divider::Shelves => {
            let books = atlas.add("divbooks", &tex::books_img(91))?;
            for k in 0..(len / 1.2).floor().max(1.0) as usize {
                let p = t.t(-len / 2.0 + 0.6 + k as f32 * 1.2, 0.0, 0.0);
                b.put_m(
                    Mesh::Box,
                    p.s(1.15, 1.15, 0.36),
                    hex("#6b4a32"),
                    crate::materials::WOOD,
                );
                for shelf in 0..2 {
                    for side in [0.0f32, PI] {
                        b.tex(
                            Mesh::QuadWall,
                            p.ry(side)
                                .t(0.0, 0.1 + shelf as f32 * 0.52, 0.181)
                                .s(1.05, 0.42, 1.0),
                            [255; 4],
                            skin(books, 0.0, 0),
                        );
                    }
                }
            }
        }
        Divider::Glass => {
            let n = (len / 1.4).ceil().max(1.0) as usize;
            for k in 0..=n {
                b.put(
                    Mesh::Box,
                    t.t(-len / 2.0 + k as f32 * len / n as f32, 0.0, 0.0)
                        .s(0.05, 2.0, 0.05),
                    metal,
                );
            }
            b.put(Mesh::Box, t.t(0.0, 1.98, 0.0).s(len, 0.04, 0.06), metal);
            b.put(Mesh::Box, t.s(len, 0.08, 0.06), metal);
            for side in [0.0, PI] {
                b.glass(
                    Mesh::QuadWall,
                    t.t(0.0, 0.08, 0.0).ry(side).s(len, 1.9, 1.0),
                    [200, 220, 235, 50],
                    0,
                );
            }
            b.put(Mesh::Box, t.t(0.0, 1.0, 0.0).s(len, 0.12, 0.02), kit.accent);
        }
        Divider::LowWall => {
            let c = match kit.wall {
                WallFinish::Brick { c, .. } | WallFinish::Tiles { c, .. } => c,
                _ => hex("#8a8f96"),
            };
            b.put(Mesh::Box, t.s(len, 1.1, 0.2), c);
            b.put(
                Mesh::Box,
                t.t(0.0, 1.1, 0.0).s(len + 0.04, 0.06, 0.26),
                hex("#3a3d43"),
            );
        }
        Divider::Lockers => {
            for k in 0..(len / 0.42).floor() as usize {
                let p = t.t(-len / 2.0 + 0.21 + k as f32 * 0.42, 0.0, 0.0);
                let c = if k % 5 == 2 {
                    kit.accent
                } else {
                    hex(if k % 2 == 0 { "#5a6a7a" } else { "#6a7a8a" })
                };
                b.put(Mesh::Box, p.s(0.4, 1.7, 0.45), c);
                for side in [-1.0f32, 1.0] {
                    b.put(
                        Mesh::Box,
                        p.t(0.12, 1.0, side * 0.23).s(0.03, 0.12, 0.02),
                        hex("#c8c8cc"),
                    );
                }
            }
        }
        Divider::Pegboard => {
            let peg = atlas.add("peg", &tex::pegboard_img())?;
            for k in 0..(len / 1.6).floor().max(1.0) as usize {
                let p = t.t(-len / 2.0 + 0.8 + k as f32 * 1.6, 0.0, 0.0);
                for s in [-0.7f32, 0.7] {
                    b.put(
                        Mesh::Box,
                        p.t(s, 0.0, 0.0).s(0.05, 1.6, 0.3),
                        hex("#3a3a3e"),
                    );
                }
                b.tex(
                    Mesh::Box,
                    p.t(0.0, 0.3, 0.0).s(1.4, 1.2, 0.03),
                    [255; 4],
                    Skin {
                        uv: peg,
                        repeat: [2.0, 1.6],
                        glow: 0.0,
                        flags: 0,
                    },
                );
                for (j, c) in ["#c84a3a", "#3a8fb0", "#e0a03a", "#2a2b30"]
                    .into_iter()
                    .enumerate()
                {
                    b.put(
                        Mesh::Box,
                        p.t(-0.5 + j as f32 * 0.33, 0.9, 0.04).s(0.05, 0.28, 0.03),
                        hex(c),
                    );
                }
            }
        }
    }
    Ok(())
}
