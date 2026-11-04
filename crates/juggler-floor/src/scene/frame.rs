//! The moving part, built for each drawn frame of the floors in view:
//! people at their desks in their state's pose, their screens, balloons
//! and lights; subagents round a desk; a fire on a hot App; the props in
//! play; the usage gauges and each sign's battery; the rack's load; the
//! red alert's beacons; people walking the aisle; the cat.
//!
//! It also says whether anything moves, so the render thread draws again
//! only while something does.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use super::{Clicked, DeskRec, HELPERS_SHOWN, LampsOff, Light, Mark, PropKind, Scene, floor_y};
use crate::colour::{Rgba, hex, mix, ramp};
use crate::layout::{AISLE, LOBBY_W, ROW_D};
use crate::math::{T, V3};
use crate::mesh::Mesh;
use crate::models::{self, B, Growth, Item, Look, Pose, person, plant};
use crate::rng::{Rng, hash};
use crate::state::FloorState;
use crate::tex::{Icon, Uv};

/// What the scene animates from outside.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Live {
    /// The clock, seconds.
    pub t: f32,
    /// What was clicked, and when.
    pub clicked: Clicked,
    /// Desk lamps switched off.
    pub lamps_off: LampsOff,
    /// The pick under the pointer.
    pub hover: u32,
    /// This Mac's overload alert is on.
    pub alert: bool,
    /// This Mac's load, 0–1.
    pub load: f32,
    /// Where the camera is.
    pub eye: V3,
    /// Motion: people walk the aisle (off in Calm).
    pub walkers: bool,
}

/// What the frame drew besides its parts.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Drawn {
    /// The lights in view.
    pub lights: Vec<Light>,
    /// Balloons and counts for the overlay.
    pub marks: Vec<Mark>,
    /// Something moves: draw again at the next tick.
    pub moving: bool,
}

/// A gauge's colour for `left` (0–1): green, yellow under 40%, amber
/// blinking under 20%, red when out.
fn gauge(left: f32, t: f32) -> Rgba {
    if left <= 0.0 {
        hex("#ef5350")
    } else if left < 0.2 {
        if (t * 2.0).sin() > 0.0 {
            hex("#f0a640")
        } else {
            hex("#7a5a2a")
        }
    } else if left < 0.4 {
        hex("#e8c84a")
    } else {
        hex("#5fd38d")
    }
}

impl Scene {
    /// Builds the moving part for `floors` into `out`.
    pub fn frame(&self, live: &Live, floors: &[usize], out: &mut Vec<Item>) -> Drawn {
        out.clear();
        let on = |f: i32| f < 0 || usize::try_from(f).is_ok_and(|f| floors.contains(&f));
        let mut drawn = Drawn::default();
        for l in self.lights.iter().filter(|l| on(l.floor)) {
            if l.owner == 0 || !live.lamps_off.contains(&l.owner) {
                drawn.lights.push(*l);
            }
        }
        for d in self.desks.iter().filter(|d| on(d.floor)) {
            self.desk_now(d, live, out, &mut drawn);
        }
        for (floor, at) in self.fires.iter().filter(|(f, _)| on(*f)) {
            fire(
                &mut B {
                    out: &mut *out,
                    floor: *floor,
                    id: 0,
                    dim: 1.0,
                },
                *at,
                live.t,
                &mut drawn,
            );
        }
        for p in self.props.iter().filter(|p| on(p.floor)) {
            let since = live
                .clicked
                .get(&p.id)
                .map(|c| live.t - c)
                .filter(|s| (0.0..4.0).contains(s));
            let mut b = B {
                out: &mut *out,
                floor: p.floor,
                id: p.id,
                dim: 1.0,
            };
            drawn.moving |= prop(&mut b, p.kind, p.t, since, live.t);
        }
        self.gauges(live.t, &on, out, &mut drawn);
        self.machine(live, &on, out, &mut drawn);
        if live.walkers {
            for f in floors {
                self.walkers(*f, live.t, out);
            }
            drawn.moving |= !floors.is_empty();
        }
        if floors.contains(&0) {
            self.cat_now(live, out, &mut drawn);
        }
        drawn
    }

    /// This computer's rack (its lights rise with the load) and, during an
    /// overload alert, the beacons at the lift and over every area.
    fn machine(
        &self,
        live: &Live,
        on: &impl Fn(i32) -> bool,
        out: &mut Vec<Item>,
        drawn: &mut Drawn,
    ) {
        for (floor, at) in self.racks.iter().filter(|(f, _)| on(*f)) {
            let mut b = B {
                out: &mut *out,
                floor: *floor,
                id: 0,
                dim: 1.0,
            };
            let lit = ((live.load + 0.04 * (live.t * 3.0).sin()) * 10.0)
                .round()
                .max(0.0) as usize;
            for k in 0..10 {
                let c = match k {
                    _ if k >= lit => hex("#2a2c34"),
                    8.. => hex("#ef5350"),
                    6 | 7 => hex("#f0a640"),
                    _ => hex("#5fd38d"),
                };
                b.glow(
                    Mesh::Box,
                    at.t(0.0, 0.25 + k as f32 * 0.16, 0.0).s(0.44, 0.08, 0.01),
                    c,
                    if k < lit { 1.0 } else { 0.0 },
                );
            }
        }
        if live.alert {
            for (floor, p) in self.beacons.iter().filter(|(f, _)| on(*f)) {
                let mut b = B {
                    out: &mut *out,
                    floor: *floor,
                    id: 0,
                    dim: 1.0,
                };
                b.put(
                    Mesh::Cyl8,
                    T::new().t(p[0], p[1], p[2]).s(0.18, 0.08, 0.18),
                    hex("#2a2733"),
                );
                b.glow(
                    Mesh::Ball,
                    T::new().t(p[0], p[1] + 0.06, p[2]).s(0.16, 0.18, 0.16),
                    hex("#ff3b30"),
                    1.0,
                );
                let a = live.t * 5.0;
                drawn.lights.push(Light {
                    floor: *floor,
                    p: [p[0] + 1.2 * a.cos(), p[1] - 0.4, p[2] + 1.2 * a.sin(), 7.0],
                    c: [1.2, 0.08, 0.06, 1.0],
                    owner: 0,
                });
            }
            drawn.moving = true;
        }
    }

    fn desk_now(&self, d: &DeskRec, live: &Live, out: &mut Vec<Item>, drawn: &mut Drawn) {
        let t = live.t;
        let mut b = B {
            out: &mut *out,
            floor: d.floor,
            id: d.id,
            dim: 1.0,
        };
        let p = &self.pictures;
        let code = p.code[(hash(&d.session.key) % 4) as usize];
        // A desktop App's screens show a chat; a terminal's, code.
        let (pose, screen, glow, scroll): (Pose, Uv, f32, f32) = match d.state {
            FloorState::Working if d.desktop => (Pose::Typing, p.chat, 0.9, -1.0),
            FloorState::Working => (Pose::Typing, code, 0.9, (t * 0.25 + d.phase * 0.13).fract()),
            FloorState::Idle | FloorState::Ended => {
                (Pose::Idle, if d.desktop { p.chat } else { code }, 0.6, -1.0)
            }
            FloorState::Asks => (Pose::Wave, p.chat, 0.9, -1.0),
            FloorState::Stuck => (Pose::Scratch, p.frozen, 0.7, -1.0),
            FloorState::Crashed => (Pose::HandsOnHead, p.error, 1.0, -1.0),
            FloorState::Done => (Pose::Stretch, p.done, 0.9, -1.0),
            FloorState::Limit => (Pose::Asleep, p.off, 0.0, -1.0),
        };
        models::screens(&mut b, d.frame, d.screens, d.top, screen, glow, scroll);
        let turn = if live.hover == d.id {
            look_up(d, live.eye)
        } else {
            0.0
        };
        let stands = matches!(pose, Pose::Wave | Pose::HandsOnHead);
        let (root, seat) = if stands {
            (
                d.frame
                    .t(
                        if d.shared { 0.5 } else { 0.55 + d.width * 0.25 },
                        0.0,
                        0.75,
                    )
                    .ry(PI + 0.4),
                0.0,
            )
        } else {
            (d.frame.t(0.0, 0.0, 0.62), d.seat)
        };
        let head = person(&mut b, root, &d.look, pose, t + d.phase, seat, turn);
        let icon = match d.state {
            FloorState::Asks => Some(Icon::Asks),
            FloorState::Stuck => Some(Icon::Stuck),
            FloorState::Crashed => Some(Icon::Error),
            FloorState::Done => Some(Icon::Done),
            FloorState::Limit => Some(Icon::Sleep),
            FloorState::Working | FloorState::Idle | FloorState::Ended => None,
        };
        if let Some(icon) = icon {
            drawn.marks.push(Mark::Balloon {
                at: [
                    head[0],
                    head[1] + 0.12 + 0.05 * (t * 3.0 + d.phase).sin(),
                    head[2],
                ],
                icon,
            });
        }
        state_effects(&mut b, d, t, drawn);
        helpers(d, t, out, drawn);
        drawn.moving = true;
    }

    /// The usage board's gauges, and each sign's 4-segment battery.
    fn gauges(&self, t: f32, on: &impl Fn(i32) -> bool, out: &mut Vec<Item>, drawn: &mut Drawn) {
        let rows = self.accounts.len();
        for (floor, at, ids) in self.boards.iter().filter(|(f, ..)| on(*f)) {
            for (i, account) in self.accounts.iter().enumerate() {
                let Some(left) = account.left_pct.map(|l| l / 100.0) else {
                    continue;
                };
                let mut b = B {
                    out: &mut *out,
                    floor: *floor,
                    id: ids.get(i).copied().unwrap_or(0),
                    dim: 1.0,
                };
                let y = 0.32 + 0.11 * (rows - i) as f32 - 0.2;
                let fill = left.max(0.03);
                b.put(
                    Mesh::Box,
                    at.t(0.65, y, 0.0).s(0.6, 0.06, 0.01),
                    hex("#3a3848"),
                );
                b.glow(
                    Mesh::Box,
                    at.t(0.35 + 0.3 * fill, y + 0.005, 0.005)
                        .s(0.6 * fill, 0.05, 0.01),
                    gauge(left, t),
                    0.9,
                );
                drawn.moving |= left < 0.2;
            }
        }
        for (floor, at, account) in self.signs.iter().filter(|(f, ..)| on(*f)) {
            let Some(left) = self
                .account(account.as_deref())
                .and_then(|a| a.left_pct)
                .map(|l| l / 100.0)
            else {
                continue;
            };
            let mut b = B {
                out: &mut *out,
                floor: *floor,
                id: 0,
                dim: 1.0,
            };
            let lit = (left * 4.0).ceil().max(0.0) as usize;
            for k in 0..4 {
                let (c, glow) = if k < lit {
                    (gauge(left, t), 0.9)
                } else {
                    (hex("#3a3848"), 0.0)
                };
                b.glow(
                    Mesh::Box,
                    at.t(-0.17 + k as f32 * 0.115, -0.02, 0.0)
                        .s(0.1, 0.06, 0.06),
                    c,
                    glow,
                );
            }
            drawn.moving |= left < 0.2;
        }
    }

    /// Two people walking the aisle of floor `f`.
    fn walkers(&self, f: usize, t: f32, out: &mut Vec<Item>) {
        let fy = floor_y(f);
        for k in 0..2 {
            let mut r = Rng(hash(&format!("walker{f}{k}")));
            let look = Look::random(&mut r, hex("#c9c3d6"));
            let len = (self.width - LOBBY_W - 2.0).max(1.0);
            let u = (t * 0.9 / len + k as f32 * 0.5 + f as f32 * 0.21).fract() * 2.0;
            let (x, dir) = if u < 1.0 {
                (LOBBY_W + 1.0 + u * len, -FRAC_PI_2)
            } else {
                (LOBBY_W + 1.0 + (2.0 - u) * len, FRAC_PI_2)
            };
            let z = ROW_D + AISLE * (0.35 + 0.3 * k as f32);
            let mut b = B {
                out: &mut *out,
                floor: f as i32,
                id: 0,
                dim: 1.0,
            };
            person(
                &mut b,
                T::new().t(x, fy, z).ry(dir),
                &look,
                Pose::Walk,
                t * 1.3 + k as f32,
                0.0,
                0.0,
            );
        }
    }

    fn cat_now(&self, live: &Live, out: &mut Vec<Item>, drawn: &mut Drawn) {
        let clicked = live.clicked.get(&self.cat).copied();
        let sitting = clicked.is_some_and(|c| live.t - c < 5.0);
        let u = if sitting {
            clicked.unwrap_or(live.t)
        } else {
            live.t
        } * 0.12;
        let (cx, cz) = (4.0 + 2.2 * (u * TAU).cos(), 13.5 + 1.8 * (u * TAU).sin());
        let mut b = B {
            out,
            floor: 0,
            id: self.cat,
            dim: 1.0,
        };
        models::cat(
            &mut b,
            T::new().t(cx, 0.0, cz).ry(-(u * TAU) + PI),
            live.t * 9.0,
            sitting,
        );
        if sitting {
            drawn.marks.push(Mark::Balloon {
                at: [cx, 0.75, cz],
                icon: Icon::Heart,
            });
        }
        drawn.moving = true;
    }
}

/// What a state adds round a desk: a spotlight for an ask, an amber lamp
/// for stuck, smoke and a red glow for a crash.
fn state_effects(b: &mut B, d: &DeskRec, t: f32, drawn: &mut Drawn) {
    let ph = d.phase;
    match d.state {
        FloorState::Asks => {
            let p = d.frame.0.point([0.0, 0.0, 0.3]);
            b.glass(
                Mesh::Cone8,
                T::new().t(p[0], p[1] + 3.4, p[2]).rx(PI).s(1.5, 3.4, 1.5),
                [255, 220, 120, 22],
                0,
            );
            drawn.lights.push(Light {
                floor: d.floor,
                p: [p[0], p[1] + 2.2, p[2], 4.5],
                c: [0.9, 0.75, 0.35, 1.0],
                owner: 0,
            });
        }
        FloorState::Stuck => {
            let p = d.frame.0.point([0.0, d.top + 0.5, -0.2]);
            let k = 0.5 + 0.5 * (t * 2.5 + ph).sin();
            drawn.lights.push(Light {
                floor: d.floor,
                p: [p[0], p[1], p[2], 2.6],
                c: [0.9 * k, 0.5 * k, 0.1 * k, 1.0],
                owner: 0,
            });
        }
        FloorState::Crashed => {
            for k in 0..4 {
                let a = (t * 0.6 + k as f32 * 0.25 + ph).rem_euclid(1.0);
                let p = d
                    .frame
                    .0
                    .point([0.1 * (k as f32 - 1.5), d.top + 0.3 + a * 1.4, -0.25]);
                let s = 0.18 + a * 0.3;
                b.put(
                    Mesh::BallLo,
                    T::new().t(p[0], p[1], p[2]).s(s, s, s),
                    mix(hex("#5a5560"), hex("#9a96a0"), a),
                );
            }
            let p = d.frame.0.point([0.0, d.top + 0.4, -0.2]);
            let k = 0.6 + 0.4 * (t * 9.0).sin();
            drawn.lights.push(Light {
                floor: d.floor,
                p: [p[0], p[1], p[2], 2.8],
                c: [0.9 * k, 0.12, 0.1, 1.0],
                owner: 0,
            });
        }
        FloorState::Working
        | FloorState::Idle
        | FloorState::Done
        | FloorState::Limit
        | FloorState::Ended => {}
    }
}

/// Subagents round a desk: a few drawn, the rest a "+N".
fn helpers(d: &DeskRec, t: f32, out: &mut Vec<Item>, drawn: &mut Drawn) {
    let helpers = d.session.subagents as usize;
    if helpers > 0 {
        let mut hb = B {
            out: &mut *out,
            floor: d.floor,
            id: d.helper_id,
            dim: 1.0,
        };
        let shown = helpers.min(HELPERS_SHOWN);
        for k in 0..shown {
            let a = -1.1 + k as f32 * (2.2 / (shown.max(2) - 1) as f32);
            let r = 1.05 + 0.15 * (k % 2) as f32;
            let at = d
                .frame
                .t(r * a.sin(), 0.0, 0.35 + r * a.cos() * 0.8)
                .ry(a + PI);
            let mut lr = Rng(hash(&format!("h{}{k}", d.session.key)));
            let mut look = Look::random(&mut lr, d.accent);
            look.height *= 0.9;
            look.top = mix(d.accent, hex("#f4efe6"), 0.45);
            look.top_style = 3;
            let above = person(
                &mut hb,
                at,
                &look,
                Pose::Helper,
                t * 1.1 + k as f32,
                0.0,
                0.0,
            );
            if k == 0 && helpers > HELPERS_SHOWN {
                drawn.marks.push(Mark::Plus {
                    at: [above[0], above[1] + 0.1, above[2]],
                    n: helpers - HELPERS_SHOWN,
                });
            }
        }
    }
}

/// How far a hovered sitter turns their head to look at the camera.
fn look_up(d: &DeskRec, eye: V3) -> f32 {
    let p = d.frame.0.point([0.0, 1.2, 0.62]);
    let dir = [eye[0] - p[0], eye[2] - p[2]];
    let (ahead, base) = (
        d.frame.0.point([0.0, 0.0, -1.0]),
        d.frame.0.point([0.0, 0.0, 0.0]),
    );
    let f = [ahead[0] - base[0], ahead[2] - base[2]];
    -(f[0] * dir[1] - f[1] * dir[0])
        .atan2(f[0] * dir[0] + f[1] * dir[1])
        .clamp(-1.2, 1.2)
}

/// A small fire on a desk (never on a person), with smoke and its glow.
fn fire(b: &mut B, at: T, t: f32, drawn: &mut Drawn) {
    for k in 0..5 {
        let flicker = (t * 7.0 + k as f32 * 1.7).sin() * 0.5 + 0.5;
        let s = 0.14 + 0.08 * flicker - k as f32 * 0.015;
        let c = if k % 2 == 0 {
            hex("#ff8a2a")
        } else {
            hex("#ffd24a")
        };
        b.glow(
            Mesh::Cone8,
            at.t(
                0.07 * (k as f32 - 2.0),
                0.0,
                0.04 * ((k * 3) % 5) as f32 - 0.08,
            )
            .s(s, 0.28 + 0.2 * flicker, s),
            c,
            1.0,
        );
    }
    for k in 0..3 {
        let a = (t * 0.4 + k as f32 * 0.33).rem_euclid(1.0);
        let p = at.0.point([0.0, 0.5 + a * 1.6, 0.0]);
        let s = 0.25 + a * 0.4;
        b.put(
            Mesh::BallLo,
            T::new().t(p[0] + a * 0.3, p[1], p[2]).s(s, s, s),
            mix(hex("#4a4650"), hex("#8a8690"), a),
        );
    }
    let p = at.0.point([0.0, 0.4, 0.0]);
    let k = 0.75 + 0.25 * (t * 11.0).sin();
    drawn.lights.push(Light {
        floor: b.floor,
        p: [p[0], p[1], p[2], 4.0],
        c: [k, 0.45 * k, 0.1, 1.0],
        owner: 0,
    });
    drawn.moving = true;
}

/// A prop as it is now (`since`: seconds since it was clicked, while in
/// play); whether it moves.
fn prop(b: &mut B, kind: PropKind, t0: T, since: Option<f32>, t: f32) -> bool {
    match kind {
        PropKind::Plant {
            kind,
            pot,
            s,
            seed,
            wilt,
        } => {
            let sway = since.map_or(0.0, |a| 0.35 * (a * 14.0).sin() * (1.0 - a / 1.6).max(0.0));
            plant(
                b,
                t0,
                kind,
                pot,
                Growth {
                    s,
                    seed,
                    sway,
                    wilt,
                },
            );
        }
        PropKind::Beanbag(c) => {
            let squash = since.map_or(1.0, |a| {
                1.0 - 0.35 * (1.0 - a / 1.2).max(0.0) * (a * 10.0).cos().abs()
            });
            b.put(Mesh::Ball, t0.s(0.85, 0.55 * squash, 0.8), c);
            b.put(
                Mesh::Ball,
                t0.t(0.0, 0.32 * squash, 0.18).s(0.6, 0.3 * squash, 0.4),
                ramp(c, 0.1),
            );
        }
        PropKind::Coffee => coffee(b, t0, since),
        PropKind::PingPong => ping_pong(b, t0, since),
        PropKind::Arcade => {
            b.cb(t0, [-0.35, 0.0, -0.3], [0.7, 1.75, 0.6], hex("#3a2a6a"));
            b.bx(t0, [-0.36, 1.45, -0.31], [0.72, 0.3, 0.62], hex("#ff5aa8"));
            let screen = if since.is_some() {
                mix(
                    hex("#5aff9a"),
                    hex("#ffe27a"),
                    f32::midpoint((t * 6.0).sin(), 1.0),
                )
            } else {
                hex("#3a5aff")
            };
            b.glow(
                Mesh::Box,
                t0.t(0.0, 1.0, 0.301).rx(-0.3).s(0.5, 0.38, 0.01),
                screen,
                1.0,
            );
            b.bx(t0, [-0.32, 0.9, 0.22], [0.64, 0.05, 0.2], hex("#1d1a24"));
        }
    }
    since.is_some()
}

/// The coffee machine, steaming for a while when clicked.
fn coffee(b: &mut B, t0: T, since: Option<f32>) {
    b.cb(t0, [-0.2, 0.0, -0.2], [0.4, 0.5, 0.36], hex("#2a2b30"));
    b.bx(t0, [-0.12, 0.08, 0.12], [0.24, 0.06, 0.1], hex("#7a7c84"));
    b.glow(
        Mesh::Box,
        t0.t(0.1, 0.38, 0.17).s(0.04, 0.04, 0.02),
        hex("#5fd38d"),
        1.0,
    );
    b.put(
        Mesh::Cyl8,
        t0.t(0.0, 0.14, 0.17).s(0.08, 0.1, 0.08),
        hex("#f4efe6"),
    );
    if let Some(a) = since {
        for k in 0..4 {
            let u = (a * 0.8 + k as f32 * 0.25).rem_euclid(1.0);
            let s = 0.06 + u * 0.08;
            b.put(
                Mesh::BallLo,
                t0.t(0.03 * (u * 9.0).sin(), 0.26 + u * 0.6, 0.17)
                    .s(s, s, s),
                [235, 235, 240, 255],
            );
        }
    }
}

/// The ping-pong table, playing a rally when clicked.
fn ping_pong(b: &mut B, t0: T, since: Option<f32>) {
    let line = hex("#f4efe6");
    b.bx(t0, [-1.37, 0.72, -0.76], [2.74, 0.04, 1.52], hex("#2f6a4a"));
    b.bx(t0, [-0.01, 0.76, -0.76], [0.02, 0.006, 1.52], line);
    b.bx(t0, [-1.37, 0.76, -0.01], [2.74, 0.006, 0.02], line);
    b.bx(t0, [-0.01, 0.76, -0.8], [0.02, 0.15, 1.6], hex("#e6e1d8"));
    for (x, z) in [(-1.2, -0.6), (1.2, -0.6), (-1.2, 0.6), (1.2, 0.6)] {
        b.bx(
            t0,
            [x - 0.03, 0.0, z - 0.03],
            [0.06, 0.72, 0.06],
            hex("#2a2b30"),
        );
    }
    for s in [-1.0f32, 1.0] {
        b.put(
            Mesh::Cyl12,
            t0.t(s * 0.9, 0.77, s * 0.4).s(0.16, 0.012, 0.16),
            hex("#c84a3a"),
        );
    }
    if let Some(a) = since {
        let u = (a * 1.4) % 2.0;
        let x = if u < 1.0 {
            -1.2 + 2.4 * u
        } else {
            1.2 - 2.4 * (u - 1.0)
        };
        let y = 0.78 + 0.35 * (u.fract() * PI).sin();
        b.glow(
            Mesh::Ball,
            t0.t(x, y, 0.1 * (a * 3.0).sin()).s(0.05, 0.05, 0.05),
            hex("#ffffff"),
            0.5,
        );
    }
}
