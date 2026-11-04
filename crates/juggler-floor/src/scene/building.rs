//! The building: the skyline, each floor's slab, back wall and glass,
//! the lobby (the lift, the directory, the usage board, the rack for this
//! computer, a bench, plants, the coffee machine) and the free cells (a
//! kitchen, then lounges).

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use super::{Light, PropKind, PropRec, Scene, Target, dimming, floor_y};
use crate::colour::{Rgba, hex};
use crate::kit::{ChairKind, DeskKind, FloorMat, Plant, Pot, WallFinish};
use crate::layout::{AISLE, COL_W, DEPTH, LOBBY_W, ROW_D, Rect, WALL_H};
use crate::math::T;
use crate::mesh::Mesh;
use crate::models::{B, CUTAWAY, Item, Skin, chair, desk};
use crate::rng::{Rng, hash};
use crate::tex::{self, Atlas, Full, Uv};

fn skin(uv: Uv, repeat: [f32; 2], glow: f32, flags: u32) -> Skin {
    Skin {
        uv,
        repeat,
        glow,
        flags,
    }
}

impl Scene {
    pub(super) fn skyline(&mut self, out: &mut Vec<Item>, atlas: &mut Atlas) -> Result<(), Full> {
        let windows = atlas.add("skyline", &tex::skyline_img())?;
        let mut b = B {
            out,
            floor: -1,
            id: 0,
            dim: 1.0,
        };
        let (cx, cz) = (self.width / 2.0, DEPTH / 2.0);
        b.put(
            Mesh::QuadUp,
            T::new().t(cx, -0.6, cz).s(900.0, 1.0, 900.0),
            hex("#1c1830"),
        );
        let mut r = Rng(77);
        for k in 0..64 {
            let a = k as f32 / 64.0 * TAU + r.range(-0.03, 0.03);
            let d = r.range(85.0, 150.0);
            let (w, h, depth) = (r.range(9.0, 22.0), r.range(14.0, 70.0), r.range(9.0, 22.0));
            let c = hex(r.pick(&["#2c2a4a", "#33304f", "#282640", "#3a3558"]));
            let t = T::new().t(cx + d * a.sin(), -0.6, cz + d * a.cos()).ry(a);
            b.tex(
                Mesh::Box,
                t.s(w, h, depth),
                c,
                skin(windows, [w / 4.0, h / 8.0], 0.75, 0),
            );
            if r.chance(0.3) {
                b.glow(
                    Mesh::Box,
                    t.t(0.0, h, 0.0).s(0.6, 3.0, 0.6),
                    hex("#ff5a5a"),
                    1.0,
                );
            }
        }
        Ok(())
    }

    pub(super) fn building_floor(
        &mut self,
        out: &mut Vec<Item>,
        atlas: &mut Atlas,
        f: usize,
        machine: &str,
    ) -> Result<(), Full> {
        let (fy, fl, w) = (floor_y(f), f as i32, self.width);
        let lobby_floor = atlas.add(
            "lobbyfloor",
            &tex::floor_img(
                &FloorMat::Concrete {
                    tone: hex("#cdc6b8"),
                    seam: 1.5,
                },
                3,
            ),
        )?;
        let lobby_wall = atlas.add(
            "lobbywall",
            &tex::wall_img(
                &WallFinish::Paint {
                    c: hex("#e6e0d4"),
                    dado: hex("#8a7a6a"),
                },
                4,
                hex("#888888"),
            ),
        )?;
        let floor_id = self.pick(
            Target::Floor(f),
            format!("Floor {}", f + 1),
            "the lift: go to another floor",
        );
        {
            let mut b = B {
                out: &mut *out,
                floor: fl,
                id: 0,
                dim: 1.0,
            };
            b.put(
                Mesh::Box,
                T::new()
                    .t(w / 2.0, fy - 0.42, DEPTH / 2.0)
                    .s(w + 0.3, 0.4, DEPTH + 0.3),
                hex("#4a4658"),
            );
            b.put(
                Mesh::Box,
                T::new()
                    .t(w / 2.0, fy - 0.42, DEPTH + 0.13)
                    .s(w + 0.3, 0.06, 0.06),
                hex("#6a6680"),
            );
            b.tex(
                Mesh::QuadUp,
                T::new().t(w / 2.0, fy, DEPTH / 2.0).s(w, 1.0, DEPTH),
                [255; 4],
                skin(lobby_floor, [w / 4.0, DEPTH / 4.0], 0.0, 0),
            );
        }
        self.back_wall(out, atlas, f, lobby_wall)?;
        self.side_walls(out, f, lobby_wall);
        self.lobby(out, atlas, f, floor_id, lobby_wall, machine)?;
        Ok(())
    }

    /// The back wall, with windows; each area's own finish behind it.
    fn back_wall(
        &mut self,
        out: &mut Vec<Item>,
        atlas: &mut Atlas,
        f: usize,
        lobby_wall: Uv,
    ) -> Result<(), Full> {
        let (fy, fl, w) = (floor_y(f), f as i32, self.width);
        let mut x = 0.0;
        while x < w - 0.01 {
            let col =
                (x >= LOBBY_W - 0.01).then(|| ((x - LOBBY_W) / COL_W + 0.01).floor() as usize);
            let end = col.map_or(LOBBY_W, |c| LOBBY_W + (c + 1) as f32 * COL_W);
            let owner = col.and_then(|c| {
                self.plan
                    .areas
                    .iter()
                    .find(|a| {
                        a.floor == f && a.row == 0 && c >= a.col && c < a.col + a.size.cells().0
                    })
                    .map(|a| a.id.clone())
            });
            let (finish, dim) = match owner.as_ref().and_then(|id| Some((id, self.kits.get(id)?))) {
                Some((id, kit)) => {
                    let level = self.unused.get(id).copied().unwrap_or(0);
                    (
                        atlas.add(
                            &format!("wall:{id}"),
                            &tex::wall_img(&kit.wall, hash(id), kit.accent),
                        )?,
                        dimming(level),
                    )
                }
                None => (lobby_wall, 1.0),
            };
            let mut b = B {
                out: &mut *out,
                floor: fl,
                id: 0,
                dim,
            };
            wall_run(&mut b, T::new().t(0.0, fy, 0.0), x, end, finish);
            x = end;
        }
        Ok(())
    }

    /// Glass on the lobby side; solid at the far end.
    fn side_walls(&mut self, out: &mut Vec<Item>, f: usize, lobby_wall: Uv) {
        let (fy, fl, w) = (floor_y(f), f as i32, self.width);
        let mut b = B {
            out,
            floor: fl,
            id: 0,
            dim: 1.0,
        };
        let left = T::new().t(0.0, fy, DEPTH).ry(FRAC_PI_2);
        let mullion = hex("#3a3848");
        for k in 0..(DEPTH / 2.4).ceil() as usize {
            let z = k as f32 * 2.4;
            b.cut(
                Mesh::Box,
                left.t(z, 0.0, 0.0).s(0.08, WALL_H, 0.08),
                mullion,
            );
            b.glass(
                Mesh::QuadWall,
                left.t(z + 1.2, 0.05, 0.0).s(2.3, WALL_H - 0.1, 1.0),
                [170, 200, 230, 60],
                CUTAWAY,
            );
        }
        b.cut(
            Mesh::Box,
            left.t(DEPTH / 2.0, WALL_H - 0.06, 0.0).s(DEPTH, 0.06, 0.1),
            mullion,
        );
        b.tex(
            Mesh::QuadWall,
            T::new()
                .t(w, fy, DEPTH / 2.0)
                .ry(-FRAC_PI_2)
                .s(DEPTH, WALL_H, 1.0),
            [255; 4],
            skin(lobby_wall, [DEPTH / 4.8, 1.0], 0.0, CUTAWAY),
        );
    }

    #[allow(clippy::too_many_lines)] // the lobby's furniture, piece by piece
    fn lobby(
        &mut self,
        out: &mut Vec<Item>,
        atlas: &mut Atlas,
        f: usize,
        floor_id: u32,
        lobby_wall: Uv,
        machine: &str,
    ) -> Result<(), Full> {
        let (fy, fl) = (floor_y(f), f as i32);
        let gold = hex("#ffd27a");
        let ink = hex("#1d1a24");
        let numeral = atlas.add(
            &format!("num{f}"),
            &tex::text_img(&[(&format!("{}", f + 1), gold)], ink, 2),
        )?;
        {
            let mut b = B {
                out: &mut *out,
                floor: fl,
                id: floor_id,
                dim: 1.0,
            };
            let core = T::new().t(0.4, fy, 0.3);
            b.tex(
                Mesh::Box,
                core.t(1.4, 0.0, 1.35).s(2.8, WALL_H + 0.4, 2.7),
                [255; 4],
                skin(lobby_wall, [0.6, 1.0], 0.0, 0),
            );
            for s in [-1.0f32, 1.0] {
                b.put(
                    Mesh::Box,
                    core.t(1.4 + s * 0.31, 0.0, 2.71).s(0.6, 2.2, 0.04),
                    hex("#b9bcc4"),
                );
            }
            b.put(
                Mesh::Box,
                core.t(1.4, 2.2, 2.71).s(1.4, 0.12, 0.05),
                hex("#7a7c84"),
            );
            b.tex(
                Mesh::QuadWall,
                core.t(1.4, 2.45, 2.72).s(0.5, 0.6, 1.0),
                [255; 4],
                skin(numeral, [1.0, 1.0], 0.9, 0),
            );
            b.glow(
                Mesh::Box,
                core.t(2.35, 1.1, 2.71).s(0.08, 0.14, 0.04),
                hex("#7fd6ff"),
                1.0,
            );
            self.beacons
                .push((fl, core.0.point([1.4, WALL_H + 0.5, 1.35])));
        }
        // The directory: this floor's areas, each in its colour.
        let mut rows: Vec<(String, Rgba)> = vec![(format!("FLOOR {}", f + 1), gold)];
        let mut here: Vec<_> = self
            .plan
            .areas
            .iter()
            .filter(|a| a.floor == f && a.annex == 0)
            .collect();
        here.sort_by_key(|a| (a.col, a.row));
        for placed in here {
            if let (Some(app), Some(kit)) = (self.apps.get(&placed.id), self.kits.get(&placed.id)) {
                rows.push((app.name.to_uppercase(), kit.accent));
            }
        }
        let rows_ref: Vec<(&str, Rgba)> = rows.iter().map(|(s, c)| (s.as_str(), *c)).collect();
        let directory = atlas.add(&format!("dir{f}"), &tex::text_img(&rows_ref, ink, 3))?;
        {
            let mut b = B {
                out: &mut *out,
                floor: fl,
                id: floor_id,
                dim: 1.0,
            };
            let t = T::new().t(4.2, fy, ROW_D + AISLE / 2.0 - 1.6).ry(0.35);
            b.put(Mesh::Box, t.s(0.5, 0.06, 0.4), hex("#2a2733"));
            b.put(Mesh::Box, t.s(0.08, 1.0, 0.08), hex("#2a2733"));
            let bh = 0.32 + 0.11 * rows.len() as f32;
            b.put(
                Mesh::Box,
                t.t(0.0, 1.0, 0.0).s(1.5, bh, 0.06),
                hex("#2a2733"),
            );
            for side in [0.0, PI] {
                b.tex(
                    Mesh::QuadWall,
                    t.t(0.0, 1.02, 0.0)
                        .ry(side)
                        .t(0.0, 0.0, 0.031)
                        .s(1.42, bh - 0.04, 1.0),
                    [255; 4],
                    skin(directory, [1.0, 1.0], 0.6, 0),
                );
            }
        }
        // The usage board: each Account's name, and its gauge (moving part).
        let accounts: Vec<(String, String, Option<f32>)> = self
            .accounts
            .iter()
            .map(|a| (a.id.clone(), a.label.clone(), a.left_pct))
            .collect();
        let mut ids = Vec::new();
        for (id, label, left) in &accounts {
            let detail = left.map_or_else(
                || "usage left: not known".to_owned(),
                |l| format!("{}% of usage left", l.round()),
            );
            ids.push(self.pick(Target::Account(id.clone()), label.clone(), detail));
        }
        let names: Vec<(String, Rgba)> = std::iter::once(("USAGE LEFT".to_owned(), gold))
            .chain(
                accounts
                    .iter()
                    .map(|(_, label, _)| (label.to_uppercase(), hex("#e6e1d8"))),
            )
            .collect();
        let names_ref: Vec<(&str, Rgba)> = names.iter().map(|(s, c)| (s.as_str(), *c)).collect();
        let board = atlas.add(&format!("usage{f}"), &tex::text_img(&names_ref, ink, 3))?;
        {
            let mut b = B {
                out: &mut *out,
                floor: fl,
                id: ids.first().copied().unwrap_or(0),
                dim: 1.0,
            };
            let t = T::new().t(6.2, fy, 1.0);
            let bh = 0.32 + 0.11 * names.len() as f32;
            for s in [-1.0f32, 1.0] {
                b.put(
                    Mesh::Box,
                    t.t(s * 0.9, 0.0, 0.0).s(0.06, 1.0, 0.06),
                    hex("#2a2733"),
                );
            }
            b.put(
                Mesh::Box,
                t.t(0.0, 1.0, 0.0).s(2.1, bh, 0.06),
                hex("#2a2733"),
            );
            b.tex(
                Mesh::QuadWall,
                t.t(-0.35, 1.02, 0.031).s(1.3, bh - 0.04, 1.0),
                [255; 4],
                skin(board, [1.0, 1.0], 0.6, 0),
            );
            self.boards.push((fl, t.t(0.0, 1.0, 0.04), ids));
        }
        // This computer: a rack whose lights show its load.
        let rack_id = self.pick(Target::ThisMac, machine, "load and memory");
        let rack = atlas.add("rack", &tex::rack_img())?;
        let label = atlas.add(
            "machine",
            &tex::text_img(&[(&machine.to_uppercase(), hex("#7fd6ff"))], ink, 2),
        )?;
        {
            let mut b = B {
                out: &mut *out,
                floor: fl,
                id: rack_id,
                dim: 1.0,
            };
            let t = T::new().t(1.0, fy, 6.4).ry(FRAC_PI_2);
            b.put(Mesh::Box, t.s(0.7, 2.0, 0.8), hex("#1c1d22"));
            b.tex(
                Mesh::QuadWall,
                t.t(0.0, 0.08, 0.401).s(0.6, 1.84, 1.0),
                [255; 4],
                skin(rack, [1.0, 1.0], 0.4, 0),
            );
            b.tex(
                Mesh::QuadWall,
                t.t(0.0, 2.05, 0.2).s(0.9, 0.2, 1.0),
                [255; 4],
                skin(label, [1.0, 1.0], 0.8, 0),
            );
            self.racks.push((fl, t.t(0.0, 0.0, 0.41)));
        }
        {
            let mut b = B {
                out: &mut *out,
                floor: fl,
                id: 0,
                dim: 1.0,
            };
            let bench = T::new().t(1.2, fy, 14.5).ry(FRAC_PI_2);
            b.cb(bench, [-1.0, 0.36, -0.25], [2.0, 0.1, 0.5], hex("#b5824f"));
            for s in [-1.0f32, 1.0] {
                b.bx(
                    bench,
                    [s * 0.85 - 0.04, 0.0, -0.22],
                    [0.08, 0.36, 0.44],
                    hex("#2a2733"),
                );
            }
            let rug = atlas.add("lobbyrug", &tex::rug_img(hex("#4a4060"), gold, 9))?;
            b.tex(
                Mesh::Box,
                T::new().t(4.2, fy, 13.5).s(4.6, 0.012, 5.0),
                [255; 4],
                skin(rug, [1.0, 1.0], 0.0, 0),
            );
        }
        for (k, (x, z)) in [(0.9, 4.0), (0.9, 18.6), (7.6, 18.8), (3.6, 4.3)]
            .into_iter()
            .enumerate()
        {
            let kind = [Plant::Fig, Plant::Palm, Plant::Monstera, Plant::Snake][k];
            self.add_plant(
                fl,
                T::new().t(x, fy, z),
                kind,
                Pot::Concrete,
                1.3,
                hash(&format!("lobby{f}{k}")),
                0.0,
            );
        }
        if f == 0 {
            let id = self.pick(
                Target::Prop(self.props.len()),
                "The coffee machine",
                "make a coffee",
            );
            let mut b = B {
                out: &mut *out,
                floor: fl,
                id,
                dim: 1.0,
            };
            let t = T::new().t(6.4, fy, DEPTH - 0.6).ry(PI);
            b.cb(t, [-0.9, 0.0, -0.3], [1.8, 0.9, 0.6], hex("#3a3848"));
            b.bx(t, [-0.95, 0.9, -0.33], [1.9, 0.04, 0.66], hex("#e6e1d8"));
            for (k, c) in ["#f4efe6", "#c86f4f", "#3a5a7a", "#e8b33a"]
                .into_iter()
                .enumerate()
            {
                b.put(
                    Mesh::Cyl8,
                    t.t(0.3 + k as f32 * 0.12, 0.94, 0.05).s(0.08, 0.1, 0.08),
                    hex(c),
                );
            }
            self.props.push(PropRec {
                kind: PropKind::Coffee,
                floor: fl,
                t: t.t(-0.4, 0.94, 0.0),
                id,
            });
            self.lights.push(Light {
                floor: fl,
                p: [4.0, fy + 2.6, 8.0, 7.0],
                c: [0.45, 0.4, 0.32, 1.0],
                owner: 0,
            });
        }
        // The aisle's light (its fittings are in the ceiling, cut away).
        for k in 0..((self.width - LOBBY_W) / 4.0) as usize {
            let x = LOBBY_W + 2.0 + k as f32 * 4.0;
            self.lights.push(Light {
                floor: fl,
                p: [x, fy + WALL_H - 0.7, ROW_D + AISLE / 2.0, 5.0],
                c: [0.5, 0.42, 0.3, 1.0],
                owner: 0,
            });
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)] // a plant's every choice
    pub(super) fn add_plant(
        &mut self,
        floor: i32,
        t: T,
        kind: Plant,
        pot: Pot,
        s: f32,
        seed: u32,
        wilt: f32,
    ) {
        let id = self.pick(Target::Prop(self.props.len()), "A plant", "give it a shake");
        self.props.push(PropRec {
            kind: PropKind::Plant {
                kind,
                pot,
                s,
                seed,
                wilt,
            },
            floor,
            t,
            id,
        });
    }

    /// A free cell: the floor's kitchen, or a lounge.
    pub(super) fn free_cell(&mut self, out: &mut Vec<Item>, f: usize, r: Rect, kitchen: bool) {
        let (fy, fl) = (floor_y(f), f as i32);
        let (label, detail) = if kitchen {
            ("Kitchen and lounge", "where idle people go")
        } else {
            ("Free space", "room for a new App")
        };
        let id = self.pick(Target::Floor(f), label, detail);
        let mut b = B {
            out,
            floor: fl,
            id,
            dim: 1.0,
        };
        let wood = hex("#b5824f");
        let (px, pz) = (r.cx(), r.cz());
        b.put(
            Mesh::QuadUp,
            T::new()
                .t(px, fy + 0.006, pz)
                .s(r.w() - 0.4, 1.0, r.d() - 0.4),
            hex("#a07850"),
        );
        if kitchen {
            let k = T::new().t(r.x1 - 0.5, fy, r.cz()).ry(-FRAC_PI_2);
            b.cb(k, [-2.4, 0.0, -0.32], [4.8, 0.9, 0.64], hex("#e6e1d8"));
            b.bx(k, [-2.45, 0.9, -0.35], [4.9, 0.05, 0.7], hex("#3a3848"));
            b.cb(k, [2.5, 0.0, -0.35], [0.8, 1.9, 0.7], hex("#c9ccd4"));
            b.bx(k, [2.55, 0.95, 0.36], [0.05, 0.4, 0.02], hex("#8a8c94"));
            for (dx, dz) in [(-1.6, -2.4), (0.6, 2.0)] {
                let t = T::new().t(px + dx, fy, pz + dz);
                desk(&mut b, t, DeskKind::Round, wood, hex("#2a2733"), 1.0);
                for k in 0..3 {
                    chair(
                        &mut b,
                        t.ry(k as f32 * TAU / 3.0 + 0.3).t(0.0, 0.0, 0.8),
                        ChairKind::Stool,
                        hex("#e8b33a"),
                        false,
                    );
                }
            }
        } else {
            let sofa = hex("#5a6a8a");
            b.cb(
                T::new().t(px - 1.0, fy, pz),
                [-1.0, 0.0, -0.45],
                [2.0, 0.45, 0.9],
                sofa,
            );
            b.cb(
                T::new().t(px - 1.0, fy, pz + 0.4),
                [-1.0, 0.45, -0.05],
                [2.0, 0.4, 0.2],
                sofa,
            );
            let sign = T::new().t(px + 1.2, fy, pz);
            b.put(Mesh::Box, sign.s(0.06, 0.9, 0.06), hex("#2a2733"));
            b.put(
                Mesh::Box,
                sign.t(0.0, 0.9, 0.0).s(0.9, 0.5, 0.04),
                hex("#f4efe6"),
            );
        }
        for (x, z) in [(r.x0 + 0.6, r.z0 + 0.6), (r.x0 + 0.6, r.z1 - 0.6)] {
            self.add_plant(
                fl,
                T::new().t(x, fy, z),
                Plant::Monstera,
                Pot::Ceramic(hex("#e6e1d8")),
                1.1,
                hash(&format!("free{f}{x}{z}")),
                0.0,
            );
        }
        if kitchen {
            for (k, c) in ["#d9644a", "#3f7fbf", "#e8b33a"].into_iter().enumerate() {
                let id = self.pick(Target::Prop(self.props.len()), "A beanbag", "flop into it");
                let t = T::new().t(r.x0 + 1.4 + k as f32 * 0.9, fy, r.z1 - 1.0);
                self.props.push(PropRec {
                    kind: PropKind::Beanbag(hex(c)),
                    floor: fl,
                    t,
                    id,
                });
            }
        }
    }
}

/// A run of the back wall from `x0` to `x1` in the finish `finish`, with a
/// window every 2.4 m.
fn wall_run(b: &mut B, t: T, x0: f32, x1: f32, finish: Uv) {
    let piece = |b: &mut B, a: f32, c: f32, y0: f32, y1: f32| {
        if c - a < 0.01 {
            return;
        }
        b.tex(
            Mesh::QuadWall,
            t.t(f32::midpoint(a, c), y0, 0.0).s(c - a, y1 - y0, 1.0),
            [255; 4],
            skin(finish, [(c - a) / 4.8, (y1 - y0) / 3.0], 0.0, CUTAWAY),
        );
        if let Some(last) = b.out.last_mut() {
            last.inst.off = [a / 4.8, (WALL_H - y1) / 3.0];
        }
    };
    let (sill, lintel) = (0.95, 2.35);
    piece(b, x0, x1, 0.0, sill);
    piece(b, x0, x1, lintel, WALL_H);
    let mut x = x0;
    while x < x1 - 0.01 {
        let (wx0, wx1) = (x + 0.5, (x + 2.0).min(x1 - 0.4));
        if wx1 - wx0 < 0.6 {
            piece(b, x, x1, sill, lintel);
            break;
        }
        piece(b, x, wx0, sill, lintel);
        let mid = f32::midpoint(wx0, wx1);
        b.glass(
            Mesh::QuadWall,
            t.t(mid, sill, 0.0).s(wx1 - wx0, lintel - sill, 1.0),
            [150, 170, 220, 45],
            CUTAWAY,
        );
        b.cut(
            Mesh::Box,
            t.t(mid, sill - 0.04, 0.03).s(wx1 - wx0 + 0.1, 0.05, 0.12),
            hex("#e6e1d8"),
        );
        b.cut(
            Mesh::Box,
            t.t(mid, sill, 0.0).s(0.04, lintel - sill, 0.05),
            hex("#3a3848"),
        );
        let next = (x + 2.4).min(x1);
        piece(b, wx1, next, sill, lintel);
        x = next;
    }
}
