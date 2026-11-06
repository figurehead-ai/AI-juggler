//! AJ-E3516: the building as a side-on tower (an elevation, as tower and
//! shelter games draw theirs): the lobby at the ground, each floor a
//! strip, each App a room in its colour with its mark, dark when unused.
//! The overlay adds each floor's number and waiting count, a badge on a
//! room that needs you, and the summary of the floor under the pointer; a
//! click on a floor enters it. Drawn in the office's own place with its
//! parts on [`TOWER`], so it shows only in the building view (the office's
//! floors don't).

use crate::colour::{Rgba, hex, mix, ramp};
use crate::flags::Need;
use crate::layout::{AISLE, DEPTH, ROW_D};
use crate::math::{T, V3};
use crate::mesh::Mesh;
use crate::models::{B, Item, Skin};
use crate::tex::{self, Atlas, Full};

use super::{COVERED, Scene, Target, dimming};

/// The tower's parts are on this floor: drawn only in the building view.
pub const TOWER: i32 = -2;
/// The tower's width, metres.
pub const TOWER_W: f32 = 26.0;
/// A storey of the tower, metres.
pub const TOWER_STOREY: f32 = 3.2;
/// The lobby's height.
pub const TOWER_LOBBY: f32 = 2.4;
/// The tower's front face (rooms stand out from its back wall).
pub const TOWER_FRONT: f32 = DEPTH / 2.0 + 2.0;

/// A floor of the tower, for the overlay.
#[derive(Clone, Debug, PartialEq)]
pub struct TowerFloor {
    /// The floor.
    pub floor: usize,
    /// Its left and right ends on the front face, half a storey up.
    pub ends: (V3, V3),
    /// Its pick (the whole strip: a click enters the floor).
    pub id: u32,
    /// How many wait on you there.
    pub waiting: usize,
}

/// A room that needs you, for the overlay's badge.
#[derive(Clone, Debug, PartialEq)]
pub struct TowerRoom {
    /// Its top, on the front face.
    pub at: V3,
    /// What needs you there first.
    pub need: Need,
}

/// The tower's left edge, for a floor `width` wide.
#[must_use]
pub fn tower_x0(width: f32) -> f32 {
    width / 2.0 - TOWER_W / 2.0
}

/// The tower's middle and half its height, for `floors` floors (the
/// building view frames it).
#[must_use]
pub fn tower_middle(width: f32, floors: usize) -> (V3, f32) {
    let h = TOWER_LOBBY + floors.max(1) as f32 * TOWER_STOREY;
    ([width / 2.0, h / 2.0, TOWER_FRONT], h / 2.0)
}

impl Scene {
    /// The tower: the lobby, then each floor's strip and rooms.
    pub(super) fn tower(&mut self, out: &mut Vec<Item>, atlas: &mut Atlas) -> Result<(), Full> {
        let x0 = tower_x0(self.width);
        let (back, front) = (TOWER_FRONT - 4.0, TOWER_FRONT);
        let lobby_words = atlas.add(
            "tower:lobby",
            &tex::text_img(&[("LOBBY", hex("#e6e2f3"))], hex("#3a3550"), 2),
        )?;
        let mut b = B {
            out: &mut *out,
            floor: TOWER,
            id: 0,
            dim: 1.0,
        };
        // the ground under it, and the lobby
        b.bx(
            T::new(),
            [x0 - 1.0, -0.6, back - 1.0],
            [TOWER_W + 2.0, 0.6, front - back + 2.0],
            hex("#2a2738"),
        );
        b.bx(
            T::new(),
            [x0, 0.0, back],
            [TOWER_W, TOWER_LOBBY, front - back],
            hex("#3a3550"),
        );
        b.tex(
            Mesh::QuadWall,
            T::new()
                .t(x0 + TOWER_W / 2.0, 0.7, front + 0.01)
                .s(3.2, 1.0, 1.0),
            [255; 4],
            Skin {
                uv: lobby_words,
                repeat: [1.0, 1.0],
                glow: 0.4,
                flags: 0,
            },
        );
        let waiting = self.waiting_stops();
        for f in 0..self.floors {
            let y0 = TOWER_LOBBY + f as f32 * TOWER_STOREY;
            let summary = self.summary(Some(f));
            let id = self.pick(Target::Floor(f), format!("Floor {}", f + 1), summary);
            let mut b = B {
                out: &mut *out,
                floor: TOWER,
                id,
                dim: 1.0,
            };
            // the slab and the back wall
            b.bx(
                T::new(),
                [x0, y0, back],
                [TOWER_W, 0.22, front - back],
                hex("#1d1a2d"),
            );
            b.bx(
                T::new(),
                [x0, y0 + 0.22, back],
                [TOWER_W, TOWER_STOREY - 0.22, 0.3],
                hex("#26233a"),
            );
            // the lift shaft where the lobby is on each floor
            let shaft = TOWER_W * (crate::layout::LOBBY_W / self.width.max(1.0));
            b.bx(
                T::new(),
                [x0 + 0.2, y0 + 0.22, back + 0.3],
                [shaft - 0.4, TOWER_STOREY - 0.5, front - back - 0.8],
                hex("#4a4560"),
            );
            self.tower_rooms(&mut b, f, (x0, y0), (back, front), atlas)?;
            self.tower_floors.push(TowerFloor {
                floor: f,
                ends: (
                    [x0 - 0.5, y0 + TOWER_STOREY / 2.0, front],
                    [x0 + TOWER_W + 0.5, y0 + TOWER_STOREY / 2.0, front],
                ),
                id,
                waiting: waiting.iter().filter(|s| s.floor == f).count(),
            });
        }
        Ok(())
    }

    /// Floor `f`'s rooms: each area of an App in its colour (dark when
    /// unused) with the App's mark, free cells grey.
    fn tower_rooms(
        &mut self,
        b: &mut B<'_>,
        f: usize,
        (x0, y0): (f32, f32),
        (back, front): (f32, f32),
        atlas: &mut Atlas,
    ) -> Result<(), Full> {
        let along = |x: f32| x0 + TOWER_W * (x / self.width.max(1.0));
        let (low, high) = (y0 + 0.3, TOWER_STOREY - 0.6);
        // the back row on the upper half, the front row below (a large
        // App, across the aisle, both)
        let rows = |z0: f32, z1: f32| {
            let (top, bottom) = (z0 < ROW_D, z1 > ROW_D + AISLE);
            match (top, bottom) {
                (true, false) => (low + high / 2.0 + 0.05, high / 2.0 - 0.05),
                (false, true) => (low, high / 2.0 - 0.05),
                _ => (low, high),
            }
        };
        let areas: Vec<_> = self
            .plan
            .areas
            .iter()
            .filter(|a| a.floor == f)
            .cloned()
            .collect();
        for area in areas {
            let Some(app) = self.apps.get(&area.id).cloned() else {
                continue;
            };
            let level = self.unused.get(&area.id).copied().unwrap_or(0);
            let accent: Rgba = self.kits.get(&area.id).map_or(hex("#888888"), |k| k.accent);
            let lit = dimming(level);
            let colour = if level >= COVERED {
                mix(hex("#2a2738"), accent, 0.25)
            } else {
                ramp(mix(hex("#2a2738"), accent, 0.55 + 0.45 * lit), -0.05)
            };
            let (xa, xb) = (along(area.rect.x0) + 0.15, along(area.rect.x1) - 0.15);
            let (low, high) = rows(area.rect.z0, area.rect.z1);
            b.bx(
                T::new(),
                [xa, low, back + 0.3],
                [xb - xa, high, front - back - 0.6],
                colour,
            );
            // its window onto the aisle: lit when in use
            b.glow(
                Mesh::Box,
                T::new()
                    .t(f32::midpoint(xa, xb), low + high * 0.15, front - 0.28)
                    .s(xb - xa - 0.5, high * 0.7, 0.04),
                if level == 0 {
                    mix(accent, hex("#fff3d6"), 0.6)
                } else {
                    hex("#1a1826")
                },
                if level == 0 { 0.55 } else { 0.0 },
            );
            if area.annex == 0 {
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
                let side = (high * 0.7).min((xb - xa) * 0.6);
                b.tex(
                    Mesh::QuadWall,
                    T::new()
                        .t(f32::midpoint(xa, xb), low + (high - side) / 2.0, front - 0.24)
                        .s(side, side, 1.0),
                    [255; 4],
                    Skin {
                        uv: mark,
                        repeat: [1.0, 1.0],
                        glow: if level == 0 { 0.6 } else { 0.2 },
                        flags: 0,
                    },
                );
            }
            if let Some((need, _)) = self.needs_of(&app).first().copied()
                && area.annex == 0
            {
                self.tower_rooms.push(TowerRoom {
                    at: [xa + 0.15, low + high, front - 0.2],
                    need,
                });
            }
        }
        for (ff, _, rect) in self.plan.free.clone() {
            if ff != f {
                continue;
            }
            let (xa, xb) = (along(rect.x0) + 0.15, along(rect.x1) - 0.15);
            let (low, high) = rows(rect.z0, rect.z1);
            b.bx(
                T::new(),
                [xa, low, back + 0.3],
                [xb - xa, high * 0.35, front - back - 0.6],
                hex("#3a3650"),
            );
        }
        Ok(())
    }
}
