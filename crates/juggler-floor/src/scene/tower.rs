//! AJ-E3516: the building view, a cross-section of the office building
//! standing on its street (the maintainer, 2026-11-06: "like a real cross
//! section of a building … on the ground, with trees outside and clouds
//! and sky"). The facade is cut away: floor slabs with thickness, the
//! lift shaft through every floor, each App a small office of its own in
//! its colour, with its logo on the wall, tiny desks and people, lit when
//! in use and dark and dusty when not; the lobby at the ground with its
//! doors, reception and bench; the roof with its plant, antenna and sign.
//! Outside: the lawn, the sidewalk, the street with parked cars, trees,
//! street lamps that light at night, a bike rack. The sky (clouds, the
//! sun or the moon, stars at night) is drawn with each frame
//! ([`Scene::tower_sky`]). The overlay adds each floor's name and waiting
//! count, and a flag on each office that needs you; a click on a floor
//! or an office enters that floor. Drawn on [`TOWER`]: only in the
//! building view.

use std::f32::consts::{FRAC_PI_2, PI};

use crate::colour::{Rgba, hex, mix, ramp};
use crate::flags::Need;
use crate::layout::DEPTH;
use crate::math::{T, V3};
use crate::mesh::Mesh;
use crate::models::{self, B, Item, Look, Pose, Skin};
use crate::rng::Rng;
use crate::snapshot::FloorApp;
use crate::state::FloorState;
use crate::tex::{self, Atlas, Full};

use super::{COVERED, Light, Scene, Target, dimming, frame::Live};

/// The building view's parts are on this floor: drawn only there.
pub const TOWER: i32 = -2;
/// The building's inside width, metres.
pub const TOWER_W: f32 = 26.0;
/// A storey, metres.
pub const TOWER_STOREY: f32 = 3.2;
/// The lobby's height (the ground floor).
pub const TOWER_LOBBY: f32 = 3.6;
/// Where the facade was: the cut face, nearest the street.
pub const TOWER_FRONT: f32 = DEPTH / 2.0 + 2.0;
/// The building's depth, front to back.
const DEEP: f32 = 5.2;
/// A floor slab's thickness.
const SLAB: f32 = 0.3;
/// The side walls' thickness.
const WALL: f32 = 0.45;
/// The lift shaft's width, at the left.
const SHAFT: f32 = 2.4;
/// The widest an office gets (a few Apps don't fill a floor).
const CELL_MAX: f32 = 5.4;
/// Furniture and people in the offices, to scale with the storey.
const MINI: f32 = 0.6;
/// Glow that shows only at night (lamps, lit windows, the sign).
const NIGHT: u32 = crate::glow::NIGHT;
/// …in lamplight's colour (a window: glass by day, warm at night).
const WARM: u32 = crate::glow::WARM;

/// A floor of the building, for the overlay.
#[derive(Clone, Debug, PartialEq)]
pub struct TowerFloor {
    /// The floor.
    pub floor: usize,
    /// Its left and right ends, outside the walls, half a storey up.
    pub ends: (V3, V3),
    /// Its pick (a click enters the floor).
    pub id: u32,
    /// How many wait on you there.
    pub waiting: usize,
}

/// An office that needs you, for the overlay's flag.
#[derive(Clone, Debug, PartialEq)]
pub struct TowerRoom {
    /// Over its middle, under the slab above, on the cut face.
    pub at: V3,
    /// What needs you there first.
    pub need: Need,
    /// How many wait there.
    pub count: u32,
    /// Its App.
    pub app: String,
    /// The floor it's on.
    pub floor: usize,
}

/// The building's left edge, inside its wall, for a floor `width` wide.
#[must_use]
pub fn tower_x0(width: f32) -> f32 {
    width / 2.0 - TOWER_W / 2.0
}

/// Where floor `f` stands in the building view.
#[must_use]
pub fn tower_floor_y(f: usize) -> f32 {
    TOWER_LOBBY + f as f32 * TOWER_STOREY
}

/// The roof's height, for `floors` floors.
#[must_use]
pub fn tower_top(floors: usize) -> f32 {
    tower_floor_y(floors.max(1))
}

/// The middle of what the building view frames (the building from the
/// street to its antenna) and half its height.
#[must_use]
pub fn tower_middle(width: f32, floors: usize) -> (V3, f32) {
    let (low, high) = (-1.0, tower_top(floors) + 5.0);
    (
        [
            width / 2.0,
            f32::midpoint(low, high),
            TOWER_FRONT - DEEP / 2.0,
        ],
        (high - low) / 2.0,
    )
}

impl Scene {
    /// The building: the street and lawn, the shell, the lobby, then each
    /// floor's slab and offices.
    pub(super) fn tower(&mut self, out: &mut Vec<Item>, atlas: &mut Atlas) -> Result<(), Full> {
        let x0 = tower_x0(self.width);
        let top = tower_top(self.floors);
        tower_outside(out, x0, &mut self.lights);
        self.tower_shell(out, atlas, x0, top)?;
        self.tower_lobby(out, atlas, x0)?;
        let waiting = self.waiting_stops();
        for f in 0..self.floors {
            let y0 = tower_floor_y(f);
            let summary = self.summary(Some(f));
            let id = self.pick(Target::Floor(f), format!("Floor {}", f + 1), summary);
            self.tower_floor(out, atlas, f, (x0, y0), id)?;
            self.tower_floors.push(TowerFloor {
                floor: f,
                ends: (
                    [x0 - WALL - 0.4, y0 + TOWER_STOREY / 2.0, TOWER_FRONT],
                    [
                        x0 + TOWER_W + WALL + 0.4,
                        y0 + TOWER_STOREY / 2.0,
                        TOWER_FRONT,
                    ],
                ),
                id,
                waiting: waiting.iter().filter(|s| s.floor == f).count(),
            });
        }
        Ok(())
    }

    /// The side walls with their windows, the back wall, the slabs'
    /// edges, the lift shaft through every floor, and the roof.
    #[allow(clippy::too_many_lines)] // the shell, part by part: walls, windows, slabs, the shaft
    fn tower_shell(
        &mut self,
        out: &mut Vec<Item>,
        atlas: &mut Atlas,
        x0: f32,
        top: f32,
    ) -> Result<(), Full> {
        let (back, front) = (TOWER_FRONT - DEEP, TOWER_FRONT);
        let x1 = x0 + TOWER_W;
        let facade = hex("#d9d2c6");
        let band = hex("#b8ae9e");
        let mut b = B {
            out: &mut *out,
            floor: TOWER,
            id: 0,
            dim: 1.0,
        };
        // the plinth, the side walls and the back
        b.bx(
            T::new(),
            [x0 - WALL - 0.3, 0.0, back - 0.5],
            [TOWER_W + 2.0 * WALL + 0.6, 0.25, DEEP + 0.6],
            band,
        );
        b.bx(
            T::new(),
            [x0 - WALL, 0.0, back - 0.3],
            [WALL, top + SLAB, DEEP + 0.3],
            facade,
        );
        b.bx(
            T::new(),
            [x1, 0.0, back - 0.3],
            [WALL, top + SLAB, DEEP + 0.3],
            facade,
        );
        b.bx(
            T::new(),
            [x0, 0.0, back - 0.3],
            [TOWER_W, top, 0.3],
            hex("#c9c1b4"),
        );
        // each storey's band and windows on the outer side walls (lit at
        // night where someone works on that floor)
        for f in 0..=self.floors {
            let y = if f == 0 { 0.0 } else { tower_floor_y(f - 1) };
            let h = if f == 0 { TOWER_LOBBY } else { TOWER_STOREY };
            b.bx(
                T::new(),
                [x0 - WALL - 0.02, y + h - 0.18, back - 0.32],
                [TOWER_W + 2.0 * WALL + 0.04, 0.18, DEEP + 0.34],
                band,
            );
            let lit = f == 0 || self.floor_in_use(f - 1);
            for (x, side) in [(x0 - WALL - 0.03, -1.0f32), (x1 + WALL + 0.03, 1.0)] {
                for k in 0..2 {
                    let z = back + DEEP * (0.3 + 0.42 * k as f32);
                    let t = T::new().t(x, y + h * 0.52, z).s(0.06, h * 0.5, DEEP * 0.28);
                    if lit {
                        b.raw_glow(Mesh::Box, t, hex("#7f9fc2"), 0.9, NIGHT | WARM);
                        b.put(
                            Mesh::Box,
                            t.t(-side * 0.2, 0.0, 0.0).s(0.4, 1.06, 1.06),
                            hex("#8a8278"),
                        );
                    } else {
                        b.put(Mesh::Box, t, hex("#55667e"));
                    }
                }
            }
        }
        // the slabs' cut edges: a darker nosing on each
        for f in 0..=self.floors {
            let y = if f == self.floors {
                top
            } else {
                tower_floor_y(f)
            };
            b.bx(
                T::new(),
                [x0, y - SLAB, back],
                [TOWER_W, SLAB, DEEP],
                hex("#6e6a72"),
            );
            b.bx(
                T::new(),
                [x0 - 0.02, y - SLAB, front - 0.02],
                [TOWER_W + 0.04, SLAB, 0.06],
                hex("#4a4652"),
            );
        }
        // the lift shaft, ground to roof, its cab waiting at the lobby
        let shaft = hex("#4b4760");
        b.bx(T::new(), [x0, 0.0, back], [SHAFT, top, 0.25], shaft);
        b.bx(
            T::new(),
            [x0 + SHAFT - 0.12, 0.0, back],
            [0.12, top, DEEP - 0.3],
            hex("#5c5772"),
        );
        for k in 0..2 {
            b.bx(
                T::new(),
                [x0 + 0.5 + k as f32 * (SHAFT - 1.1), 0.0, back + 0.3],
                [0.06, top, 0.06],
                hex("#8d88a3"),
            );
        }
        b.bx(
            T::new(),
            [x0 + 0.35, 0.05, back + 0.4],
            [SHAFT - 0.6, 2.4, 2.2],
            hex("#c9c2d6"),
        );
        b.raw_glow(
            Mesh::Box,
            T::new()
                .t(x0 + SHAFT / 2.0 - 0.05, 2.38, back + 1.5)
                .s(1.0, 0.04, 1.2),
            hex("#fff2d8"),
            0.9,
            0,
        );
        tower_roof(&mut b, atlas, (x0, top))
    }

    /// The lobby: its floor and wall, the entrance doors at the street,
    /// reception with someone behind the desk, a bench, plants and lights.
    #[allow(clippy::too_many_lines)] // the lobby, part by part: wall, reception, doors, bench, plants, lights
    fn tower_lobby(&mut self, out: &mut Vec<Item>, atlas: &mut Atlas, x0: f32) -> Result<(), Full> {
        let (back, front) = (TOWER_FRONT - DEEP, TOWER_FRONT);
        let id = self.pick(
            Target::Floor(0),
            "The lobby",
            "Floor 1 is one storey up: click a floor to go in",
        );
        let words = atlas.add(
            "tower:lobby",
            &tex::text_img(&[("WELCOME", hex("#3a3550"))], hex("#ece6da"), 2),
        )?;
        let mut b = B {
            out: &mut *out,
            floor: TOWER,
            id,
            dim: 1.0,
        };
        let (xa, xb) = (x0 + SHAFT, x0 + TOWER_W);
        b.bx(
            T::new(),
            [xa, 0.0, back],
            [xb - xa, 0.26, DEEP],
            hex("#d8d0c2"),
        );
        b.bx(
            T::new(),
            [xa, 0.26, back],
            [xb - xa, TOWER_LOBBY - 0.26 - SLAB, 0.14],
            hex("#ece6da"),
        );
        b.bx(
            T::new(),
            [xa, 0.26, back + 0.14],
            [xb - xa, 0.5, 0.04],
            hex("#8a7a6a"),
        );
        b.tex(
            Mesh::QuadWall,
            T::new().t(xa + 5.2, 2.3, back + 0.16).s(3.0, 0.6, 1.0),
            [255; 4],
            Skin {
                uv: words,
                repeat: [1.0, 1.0],
                glow: 0.2,
                flags: 0,
            },
        );
        // reception, someone behind it
        let rx = xa + 5.2;
        b.put(
            Mesh::RoundBox,
            T::new().t(rx, 0.26, back + 2.4).s(3.0, 1.0, 0.8),
            hex("#7a5a40"),
        );
        b.put(
            Mesh::Box,
            T::new().t(rx, 1.26, back + 2.4).s(3.2, 0.06, 0.95),
            hex("#e9e2d4"),
        );
        let lk = Look::random(&mut Rng(31), hex("#6b6bd6"));
        models::person_as(
            self.detail,
            &mut b,
            T::new()
                .t(rx + 0.4, 0.26, back + 1.6)
                .s(MINI * 1.15, MINI * 1.15, MINI * 1.15),
            &lk,
            Pose::Stand,
            0.0,
            0.0,
            0.0,
        );
        // the entrance at the street: glass doors in a frame, a mat
        let ex = xb - 4.0;
        for k in [-1.0f32, 1.0] {
            b.put(
                Mesh::Box,
                T::new()
                    .t(ex + k * 1.35, 0.26, front - 0.1)
                    .s(0.14, 2.6, 0.14),
                hex("#3d3a46"),
            );
        }
        b.put(
            Mesh::Box,
            T::new().t(ex, 2.86, front - 0.1).s(2.84, 0.16, 0.16),
            hex("#3d3a46"),
        );
        b.glass(
            Mesh::Box,
            T::new().t(ex, 0.26, front - 0.1).s(2.56, 2.6, 0.04),
            [190, 220, 235, 70],
            0,
        );
        b.put(
            Mesh::Box,
            T::new().t(ex, 0.27, front - 0.9).s(2.2, 0.02, 1.2),
            hex("#5a4f60"),
        );
        // a bench and two plants
        b.put(
            Mesh::RoundBox,
            T::new().t(xa + 10.5, 0.26, back + 0.9).s(2.4, 0.45, 0.6),
            hex("#6a5a8a"),
        );
        for (k, x) in [xa + 0.8, xa + 13.6].into_iter().enumerate() {
            models::plant(
                &mut b,
                T::new().t(x, 0.26, back + 0.7),
                if k == 0 {
                    crate::kit::Plant::Monstera
                } else {
                    crate::kit::Plant::Fern
                },
                crate::kit::Pot::Terracotta,
                models::Growth {
                    s: 0.9,
                    seed: 11 + k as u32,
                    sway: 0.0,
                    wilt: 0.0,
                },
            );
        }
        for k in 0..4 {
            let x = xa + 2.0 + k as f32 * 5.2;
            b.raw_glow(
                Mesh::Box,
                T::new()
                    .t(x, TOWER_LOBBY - SLAB - 0.06, back + DEEP * 0.5)
                    .s(1.6, 0.05, 0.5),
                hex("#fff2d6"),
                0.9,
                0,
            );
        }
        self.lights.push(Light {
            floor: TOWER,
            p: [xa + (xb - xa) / 2.0, 2.4, back + 2.5, 9.0],
            c: [1.0, 0.92, 0.76, 1.0],
            owner: 0,
        });
        Ok(())
    }

    /// Whether anyone works on floor `f` now (its windows light at night).
    fn floor_in_use(&self, f: usize) -> bool {
        self.plan
            .areas
            .iter()
            .filter(|a| a.floor == f && a.annex == 0 && !self.hidden.contains(&a.id))
            .any(|a| self.unused.get(&a.id).copied().unwrap_or(0) == 0)
    }

    /// Floor `f`: each App an office side by side after the shaft, in
    /// the floor's order, then the space still free.
    fn tower_floor(
        &mut self,
        out: &mut Vec<Item>,
        atlas: &mut Atlas,
        f: usize,
        (x0, y0): (f32, f32),
        floor_id: u32,
    ) -> Result<(), Full> {
        let mut areas: Vec<_> = self
            .plan
            .areas
            .iter()
            .filter(|a| a.floor == f && a.annex == 0 && !self.hidden.contains(&a.id))
            .map(|a| (a.rect.x0, a.rect.z0, a.id.clone()))
            .collect();
        areas.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
        let start = x0 + SHAFT;
        let room = TOWER_W - SHAFT;
        let cell = if areas.is_empty() {
            room
        } else {
            (room / areas.len() as f32).min(CELL_MAX)
        };
        for (k, (_, _, id)) in areas.iter().enumerate() {
            let Some(app) = self.apps.get(id).cloned() else {
                continue;
            };
            let xa = start + k as f32 * cell;
            self.tenant(out, atlas, &app, f, (xa, xa + cell, y0))?;
        }
        // the space still free on this floor
        let used = start + areas.len() as f32 * cell;
        if used < x0 + TOWER_W - 0.5 {
            let mut b = B {
                out: &mut *out,
                floor: TOWER,
                id: floor_id,
                dim: 1.0,
            };
            let back = TOWER_FRONT - DEEP;
            b.bx(
                T::new(),
                [used, y0, back],
                [x0 + TOWER_W - used, 0.04, DEEP],
                hex("#bdb6aa"),
            );
            b.bx(
                T::new(),
                [used, y0, back],
                [x0 + TOWER_W - used, TOWER_STOREY - SLAB, 0.12],
                hex("#d6d0c6"),
            );
        }
        Ok(())
    }

    /// One App's office from `xa` to `xb` on the floor at `y0`: its floor
    /// and wall in its colour with its logo, a desk for each session (at
    /// least one) with its person, a plant, the ceiling light; dark and
    /// dusty (and later under dust sheets) the longer it's been unused.
    #[allow(clippy::too_many_lines)] // one office, part by part
    fn tenant(
        &mut self,
        out: &mut Vec<Item>,
        atlas: &mut Atlas,
        app: &FloorApp,
        f: usize,
        (xa, xb, y0): (f32, f32, f32),
    ) -> Result<(), Full> {
        let (back, front) = (TOWER_FRONT - DEEP, TOWER_FRONT);
        let level = self.unused.get(&app.id).copied().unwrap_or(0);
        let kit = self.kits.get(&app.id).cloned();
        let accent: Rgba = kit.as_ref().map_or(hex("#8888aa"), |k| k.accent);
        let needs = self.needs_of(app);
        let live: Vec<_> = app
            .sessions
            .iter()
            .filter(|s| s.state != FloorState::Ended)
            .collect();
        let detail = match (app.open, needs.first()) {
            (_, Some((need, n))) => format!(
                "{n} {} · {} sessions · click to go in",
                need.word(n > &1).to_lowercase(),
                live.len()
            ),
            (true, None) => format!("{} sessions · nothing waiting · click to go in", live.len()),
            (false, None) => "closed · click to go in".to_owned(),
        };
        let id = self.pick(Target::Floor(f), app.name.clone(), detail);
        let mark = match atlas.get(&format!("logo:{}", app.id)) {
            Some(uv) => uv,
            None => atlas.add(
                &format!("tower:name:{}", app.id),
                &tex::text_img(
                    &[(&super::mark_letters(&app.name), hex("#f4efe6"))],
                    hex("#1d1a24"),
                    2,
                ),
            )?,
        };
        let lit = level == 0;
        let w = xb - xa;
        let h = TOWER_STOREY - SLAB;
        let cx = f32::midpoint(xa, xb);
        let mut b = B {
            out: &mut *out,
            floor: TOWER,
            id,
            dim: if lit { 1.0 } else { dimming(level) },
        };
        // floor, back wall in the App's colour, a stripe, the logo
        b.bx(
            T::new(),
            [xa, y0, back],
            [w, 0.05, DEEP],
            mix(hex("#d6cfbf"), accent, 0.16),
        );
        b.bx(
            T::new(),
            [xa, y0 + 0.05, back + 0.25],
            [w, h - 0.05, 0.12],
            mix(hex("#ece6da"), accent, 0.42),
        );
        b.bx(
            T::new(),
            [xa, y0 + h - 0.3, back + 0.37],
            [w, 0.12, 0.04],
            accent,
        );
        let side = (h * 0.38).min(w * 0.36);
        b.tex(
            Mesh::QuadWall,
            T::new()
                .t(cx, y0 + h * 0.48, back + 0.385)
                .s(side, side, 1.0),
            [255; 4],
            Skin {
                uv: mark,
                repeat: [1.0, 1.0],
                glow: if lit { 0.5 } else { 0.1 },
                flags: 0,
            },
        );
        // the wall to the next office
        b.bx(
            T::new(),
            [xb - 0.07, y0 + 0.05, back + 0.25],
            [0.14, h - 0.05, DEEP - 0.9],
            ramp(mix(hex("#cfc8bc"), accent, 0.1), -0.05),
        );
        // the ceiling light
        let lamp = T::new()
            .t(cx, y0 + h - 0.05, back + DEEP * 0.55)
            .s(w * 0.45, 0.05, 0.5);
        if lit {
            b.raw_glow(Mesh::Box, lamp, hex("#fff3d6"), 0.95, 0);
        } else {
            b.put(Mesh::Box, lamp, hex("#56525e"));
        }
        // desks, each with its person (or an empty chair)
        let slots = (((w - 0.6) / 1.15).floor() as usize).clamp(1, 4);
        let desks = live.len().clamp(1, slots);
        let (desk, top, frame, chair_kind, chair_c) = kit.as_ref().map_or(
            (
                crate::kit::DeskKind::FourLeg,
                hex("#c8b89a"),
                hex("#4a4a52"),
                crate::kit::ChairKind::Task,
                hex("#3a3a44"),
            ),
            |k| (k.desk, k.desk_top, k.desk_frame, k.chair, k.chair_colour),
        );
        let step = (w - 0.4) / desks as f32;
        for d in 0..desks {
            let x = xa + 0.2 + step * (d as f32 + 0.5);
            let at = T::new()
                .t(x, y0 + 0.05, back + 1.9)
                .ry(PI)
                .s(MINI, MINI, MINI);
            let height = models::desk(&mut b, at, desk, top, frame, 1.2);
            // a screen, lit while in use
            let screen = at.t(0.0, height, 0.15).s(0.7, 0.42, 0.04);
            if lit {
                b.raw_glow(Mesh::Box, screen.t(0.0, 0.5, 0.0), hex("#7fc8ff"), 0.7, 0);
            } else {
                b.put(Mesh::Box, screen.t(0.0, 0.5, 0.0), hex("#2a2a30"));
            }
            let seat = at.t(0.0, 0.0, -0.62).ry(PI);
            if let Some(s) = live.get(d) {
                let mut r = Rng::of(&s.key);
                let lk = Look::random(&mut r, accent);
                models::person_as(
                    self.detail,
                    &mut b,
                    seat,
                    &lk,
                    super::pose_of(super::drawn_state(s, self.now_ms)),
                    0.0,
                    0.46,
                    0.0,
                );
            }
            models::chair(&mut b, seat, chair_kind, chair_c, false);
            if level >= COVERED {
                // a dust sheet over the desk
                b.put(
                    Mesh::RoundBox,
                    at.t(0.0, 0.0, 0.05).s(1.35, height + 0.12, 0.9),
                    hex("#ded8cc"),
                );
            }
        }
        if level >= COVERED {
            b.put(
                Mesh::Box,
                T::new()
                    .t(xb - 0.5, y0 + 0.05, front - 0.9)
                    .s(0.5, 0.36, 0.42),
                hex("#b08a5a"),
            );
        }
        // a plant in the corner
        if w > 2.2
            && let Some(k) = kit.as_ref()
            && let Some(plant) = k.plants.first().copied()
        {
            models::plant(
                &mut b,
                T::new()
                    .t(xa + 0.35, y0 + 0.05, back + 0.7)
                    .s(MINI, MINI, MINI),
                plant,
                k.pot,
                models::Growth {
                    s: 0.8,
                    seed: crate::rng::hash(&app.id),
                    sway: 0.0,
                    wilt: if lit { 0.0 } else { 0.4 },
                },
            );
        }
        if lit {
            self.lights.push(Light {
                floor: TOWER,
                p: [cx, y0 + h - 0.5, back + DEEP * 0.55, 4.5],
                c: [1.0, 0.93, 0.78, 1.0],
                owner: 0,
            });
        }
        if let Some((need, _)) = needs.first().copied() {
            let count: u32 = needs.iter().map(|(_, n)| *n).sum();
            self.tower_rooms.push(TowerRoom {
                at: [cx, y0 + h, front],
                need,
                count,
                app: app.id.clone(),
                floor: f,
            });
        }
        Ok(())
    }

    /// The building view's sky, with each frame: clouds drifting (only
    /// while the Floor animates anyway: `t` stands still otherwise), the
    /// sun by day and the moon by night where the light comes from, and
    /// stars at night.
    pub(super) fn tower_sky(&self, live: &Live, out: &mut Vec<Item>) {
        let x0 = tower_x0(self.width);
        let mid = x0 + TOWER_W / 2.0;
        let top = tower_top(self.floors);
        // just behind the building: the view is orthographic and looks a
        // little down, so what stands farther back shows higher up, and a
        // sky much farther back would rise out of the frame
        let far = TOWER_FRONT - DEEP - 24.0;
        let mut b = B {
            out,
            floor: TOWER,
            id: 0,
            dim: 1.0,
        };
        let night = live.night;
        // the sun or the moon, up where the light comes from
        let s = live.sun;
        if s[1] > -0.05 {
            // in the sky above the building, towards where the light is
            let (x, y) = (
                mid + s[0].clamp(-1.0, 1.0) * (TOWER_W / 2.0 + 2.0),
                top + 2.5 + s[1].clamp(0.0, 1.0) * 2.5,
            );
            let (c, r) = if night > 0.5 {
                (hex("#e8ecff"), 1.4)
            } else {
                (hex("#fff1c4"), 1.9)
            };
            b.raw_glow(
                Mesh::Sphere,
                T::new().t(x, y, far - 4.0).s(r, r, r),
                c,
                1.0,
                0,
            );
        }
        if night > 0.5 {
            let mut r = Rng(97);
            for _ in 0..70 {
                let (x, y) = (mid + r.range(-34.0, 34.0), r.range(top + 0.5, top + 7.0));
                let k = r.range(0.08, 0.2);
                b.raw_glow(
                    Mesh::Box,
                    T::new().t(x, y, far - 6.0).s(k, k, k),
                    hex("#f4f2ff"),
                    1.0,
                    0,
                );
            }
        }
        // clouds: puffs of balls, drifting slowly left to right
        let mut r = Rng(43);
        let tint = if night > 0.5 {
            hex("#4a4868")
        } else {
            hex("#f7f7fb")
        };
        for k in 0..5 {
            let span = 90.0;
            let x = (r.range(0.0, span) + live.t * r.range(0.25, 0.6)).rem_euclid(span)
                - span / 2.0
                + mid;
            let y = top + r.range(2.5, 5.5) + k as f32 * 0.2;
            let z = far + r.range(-2.0, 6.0);
            for p in 0..4 {
                let (dx, s) = (p as f32 * 1.2 - 1.8, r.range(1.0, 1.8));
                b.raw_glow(
                    Mesh::Sphere,
                    T::new()
                        .t(x + dx, y + r.range(-0.2, 0.3), z)
                        .s(s * 1.3, s * 0.7, s),
                    tint,
                    0.35,
                    0,
                );
            }
        }
    }
}

/// Outside the building: the lawn, the sidewalk and the street, parked
/// cars, a bike rack by the doors, street lamps (lit at night), trees and
/// bushes.
fn tower_outside(out: &mut Vec<Item>, x0: f32, lamps: &mut Vec<Light>) {
    let mut b = B {
        out,
        floor: TOWER,
        id: 0,
        dim: 1.0,
    };
    street(&mut b, x0);
    cars_and_bikes(&mut b, x0);
    street_lamps(&mut b, x0, lamps);
    trees(&mut b, x0);
}

/// The lawn, the sidewalk, the curb, the street with its lane marks, and
/// the far sidewalk and lawn.
fn street(b: &mut B<'_>, x0: f32) {
    let (back, front) = (TOWER_FRONT - DEEP, TOWER_FRONT);
    let x1 = x0 + TOWER_W;
    let mid = f32::midpoint(x0, x1);
    // the lawn, then the sidewalk, curb, street and the far sidewalk
    b.put(
        Mesh::QuadUp,
        T::new().t(mid, -0.02, back - 20.0).s(260.0, 1.0, 120.0),
        hex("#6d9a50"),
    );
    b.bx(
        T::new(),
        [mid - 130.0, -0.02, front],
        [260.0, 0.08, 3.4],
        hex("#cbc5b9"),
    );
    b.bx(
        T::new(),
        [mid - 130.0, -0.02, front + 3.4],
        [260.0, 0.14, 0.18],
        hex("#9b968d"),
    );
    b.bx(
        T::new(),
        [mid - 130.0, -0.06, front + 3.58],
        [260.0, 0.06, 7.2],
        hex("#3c3c46"),
    );
    for k in 0..48 {
        let x = mid - 120.0 + k as f32 * 5.0;
        b.bx(
            T::new(),
            [x, 0.0, front + 7.1],
            [2.2, 0.02, 0.16],
            hex("#e6e2d6"),
        );
    }
    b.bx(
        T::new(),
        [mid - 130.0, -0.02, front + 10.78],
        [260.0, 0.14, 0.18],
        hex("#9b968d"),
    );
    b.bx(
        T::new(),
        [mid - 130.0, -0.02, front + 10.96],
        [260.0, 0.08, 3.0],
        hex("#cbc5b9"),
    );
    b.put(
        Mesh::QuadUp,
        T::new().t(mid, -0.03, front + 40.0).s(260.0, 1.0, 58.0),
        hex("#6a9550"),
    );
}

/// Three cars parked along the near curb; a bike rack by the doors.
fn cars_and_bikes(b: &mut B<'_>, x0: f32) {
    let front = TOWER_FRONT;
    let x1 = x0 + TOWER_W;
    // parked cars along the near curb
    for (k, (x, c)) in [
        (x0 - 9.0, "#b94a48"),
        (x0 + 7.0, "#4a6fb0"),
        (x1 + 6.5, "#d8b04a"),
    ]
    .into_iter()
    .enumerate()
    {
        let z = front + 4.6;
        let body = hex(c);
        b.put(
            Mesh::RoundBox,
            T::new().t(x, 0.32, z).s(3.8, 0.7, 1.7),
            body,
        );
        b.put(
            Mesh::RoundBox,
            T::new().t(x - 0.2, 1.0, z).s(2.1, 0.58, 1.5),
            mix(body, hex("#2a3040"), 0.55),
        );
        for (dx, dz) in [
            (-1.25f32, -0.82f32),
            (1.25, -0.82),
            (-1.25, 0.82),
            (1.25, 0.82),
        ] {
            b.put(
                Mesh::Cyl12,
                T::new()
                    .t(x + dx, 0.36, z + dz)
                    .rx(FRAC_PI_2)
                    .s(0.72, 0.22, 0.72),
                hex("#1e1e24"),
            );
        }
        b.raw_glow(
            Mesh::Box,
            T::new()
                .t(x + 1.88, 0.62, z + 0.55 - 1.1 * (k % 2) as f32)
                .s(0.06, 0.14, 0.3),
            hex("#fff2c8"),
            0.8,
            NIGHT,
        );
    }
    // a bike rack by the doors, two bikes in it
    let rx = x1 - 7.5;
    b.put(
        Mesh::Box,
        T::new().t(rx, 0.06, front + 1.6).s(2.6, 0.08, 0.08),
        hex("#5d6068"),
    );
    for k in 0..2 {
        let x = rx - 0.6 + k as f32 * 1.2;
        for dz in [-0.45f32, 0.45] {
            b.put(
                Mesh::Cyl12,
                T::new()
                    .t(x, 0.38, front + 1.6 + dz)
                    .rz(FRAC_PI_2)
                    .s(0.66, 0.04, 0.66),
                hex("#26262c"),
            );
        }
        b.put(
            Mesh::Box,
            T::new().t(x, 0.5, front + 1.6).s(0.05, 0.05, 0.9),
            hex(if k == 0 { "#2f8f6f" } else { "#c0503a" }),
        );
        b.put(
            Mesh::Box,
            T::new().t(x, 0.72, front + 1.25).s(0.05, 0.4, 0.05),
            hex("#2a2a2a"),
        );
    }
}

/// Street lamps either side of the building: lit only at night.
fn street_lamps(b: &mut B<'_>, x0: f32, lamps: &mut Vec<Light>) {
    let front = TOWER_FRONT;
    let x1 = x0 + TOWER_W;
    // street lamps along the sidewalk, either side of the building (never
    // in front of its cut face): lit at night
    for x in [x0 - 18.0, x0 - 5.0, x1 + 5.0, x1 + 18.0] {
        let z = front + 2.9;
        b.put(
            Mesh::Cyl8,
            T::new().t(x, 0.0, z).s(0.16, 4.6, 0.16),
            hex("#3a3d46"),
        );
        b.put(
            Mesh::Box,
            T::new().t(x, 4.55, z + 0.45).s(0.1, 0.1, 0.9),
            hex("#3a3d46"),
        );
        b.raw_glow(
            Mesh::Box,
            T::new().t(x, 4.42, z + 0.85).s(0.42, 0.16, 0.42),
            hex("#ffe7ad"),
            0.95,
            NIGHT,
        );
        lamps.push(Light {
            floor: TOWER,
            p: [x, 3.6, z + 0.9, 8.0],
            // a = 0: lit only at night
            c: [1.0, 0.85, 0.55, 0.0],
            owner: 0,
        });
    }
}

/// Trees beside the building and across the street; bushes by its walls.
fn trees(b: &mut B<'_>, x0: f32) {
    let (back, front) = (TOWER_FRONT - DEEP, TOWER_FRONT);
    let x1 = x0 + TOWER_W;
    // trees beside the building and across the street; bushes by the walls
    let mut r = Rng(212);
    let spots: Vec<(f32, f32)> = [
        (x0 - 6.0, back + 2.0),
        (x0 - 12.5, back - 1.0),
        (x0 - 20.0, back + 1.5),
        (x1 + 6.0, back + 1.0),
        (x1 + 13.0, back + 2.5),
        (x1 + 21.0, back - 1.5),
        (x0 - 9.0, front + 16.0),
        (x0 - 22.0, front + 17.5),
        (x1 + 10.0, front + 16.5),
        (x1 + 23.0, front + 17.0),
    ]
    .into_iter()
    .collect();
    for (x, z) in spots {
        let h = r.range(2.6, 3.8);
        b.put(
            Mesh::Cyl8,
            T::new().t(x, 0.0, z).s(0.32, h, 0.32),
            hex("#6b4a32"),
        );
        let green = hex(r.pick(&["#4c8a3c", "#5a9a46", "#3f7a36"]));
        for k in 0..3 {
            let s = r.range(1.4, 2.1);
            let (dx, dz) = (r.range(-0.7, 0.7), r.range(-0.6, 0.6));
            b.put(
                Mesh::Sphere,
                T::new()
                    .t(x + dx, h + 0.4 + k as f32 * 0.7, z + dz)
                    .s(s, s * 0.9, s),
                ramp(green, k as f32 * 0.04),
            );
        }
    }
    for side in [-1.0f32, 1.0] {
        for k in 0..4 {
            let x = if side < 0.0 {
                x0 - WALL - 0.8
            } else {
                x1 + WALL + 0.8
            };
            let z = back + 0.6 + k as f32 * 1.3;
            let s = r.range(0.7, 1.0);
            b.put(
                Mesh::Sphere,
                T::new().t(x, 0.35, z).s(s, s * 0.8, s),
                hex("#4f8a40"),
            );
        }
    }
}

/// The roof: its parapet, the plant on it (two units and a water tank),
/// the antenna with its light, and the sign that names the company.
fn tower_roof(b: &mut B<'_>, atlas: &mut Atlas, (x0, top): (f32, f32)) -> Result<(), Full> {
    let (back, front) = (TOWER_FRONT - DEEP, TOWER_FRONT);
    let x1 = x0 + TOWER_W;
    let facade = hex("#d9d2c6");
    // the roof: a parapet, the plant, the antenna, the sign
    b.bx(
        T::new(),
        [x0 - WALL, top, back - 0.3],
        [TOWER_W + 2.0 * WALL, 0.7, 0.3],
        facade,
    );
    b.bx(
        T::new(),
        [x0 - WALL, top, front - 0.25],
        [TOWER_W + 2.0 * WALL, 0.7, 0.25],
        facade,
    );
    let unit = hex("#9aa3ad");
    for (k, x) in [x1 - 4.5, x1 - 8.2].into_iter().enumerate() {
        b.bx(T::new(), [x, top, back + 0.8], [2.6, 1.3, 2.2], unit);
        b.put(
            Mesh::Cyl12,
            T::new()
                .t(x + 1.3, top + 1.33, back + 1.9)
                .s(1.5, 0.05, 1.5),
            hex("#5d646c"),
        );
        if k == 0 {
            b.put(
                Mesh::Cyl8,
                T::new()
                    .t(x - 0.4, top + 0.6, back + 1.2)
                    .rz(FRAC_PI_2)
                    .s(0.25, 1.0, 0.25),
                hex("#7d858e"),
            );
        }
    }
    b.put(
        Mesh::Cyl12,
        T::new().t(x0 + 5.0, top, back + 1.6).s(2.0, 2.2, 2.0),
        hex("#8f7f6f"),
    );
    b.put(
        Mesh::Cone8,
        T::new().t(x0 + 5.0, top + 2.2, back + 1.6).s(2.2, 0.6, 2.2),
        hex("#6f6256"),
    );
    b.put(
        Mesh::Cyl8,
        T::new().t(x0 + 1.2, top, back + 1.0).s(0.12, 5.0, 0.12),
        hex("#7a7f88"),
    );
    b.raw_glow(
        Mesh::Ball,
        T::new()
            .t(x0 + 1.2, top + 5.05, back + 1.0)
            .s(0.28, 0.28, 0.28),
        hex("#ff4a4a"),
        1.0,
        0,
    );
    let sign = atlas.add(
        "tower:sign",
        &tex::text_img(&[("AI JUGGLER", hex("#fff4dc"))], hex("#2b2740"), 2),
    )?;
    let (sx, sw) = (x0 + TOWER_W / 2.0 + 1.0, 7.5);
    for k in [-1.0f32, 1.0] {
        b.put(
            Mesh::Box,
            T::new()
                .t(sx + k * (sw / 2.0 - 0.5), top + 0.9, back + 0.6)
                .s(0.15, 1.8, 0.15),
            hex("#4a4652"),
        );
    }
    b.tex(
        Mesh::QuadWall,
        T::new().t(sx, top + 1.3, back + 0.7).s(sw, sw * 0.16, 1.0),
        [255; 4],
        Skin {
            uv: sign,
            repeat: [1.0, 1.0],
            glow: 0.7,
            flags: NIGHT,
        },
    );
    Ok(())
}
