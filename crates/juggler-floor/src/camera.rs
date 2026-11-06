//! The orbit camera (SPEC §9.7): a target on a floor, a turn (yaw), a
//! tilt (pitch, 18°–79°) and a distance. Drag pans; right-drag, ⌥-drag, a
//! two-finger twist, Q/E or the arrows turn and tilt; pinch, ⌘-scroll or
//! +/− zoom towards the pointer, on one floor at a time; H resets; 0, B
//! and the floor numbers pick the building or a floor; Page Up and Page
//! Down ride the lift. The building is a side-on tower (AJ-E3516), seen
//! straight on with no perspective; zoom never switches between it and a
//! floor.

use std::f32::consts::{PI, TAU};

use crate::layout::DEPTH;
use crate::math::{M4, V3, add, cross, norm, scale, sub};
use crate::scene::floor_y;

/// The vertical field of view, radians.
pub const FOV: f32 = 0.5;
/// The near plane, metres.
pub const NEAR: f32 = 0.5;
/// The far plane, metres.
pub const FAR: f32 = 700.0;
/// The lowest tilt (18°).
pub const PITCH_MIN: f32 = 0.32;
/// The highest tilt (79°).
pub const PITCH_MAX: f32 = 1.38;
const DEFAULT_YAW: f32 = 0.42;
const DEFAULT_PITCH: f32 = 0.82;
/// The closest the camera comes, metres.
pub const MIN_DIST: f32 = 4.0;
/// How long a camera move takes, seconds.
pub const TWEEN_S: f32 = 0.45;

/// Where the camera looks from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cam {
    /// What it looks at.
    pub t: V3,
    /// Its turn round the target.
    pub yaw: f32,
    /// Its tilt above the floor.
    pub pitch: f32,
    /// How far it is.
    pub dist: f32,
}

impl Cam {
    /// Where the camera is.
    #[must_use]
    pub fn eye(&self) -> V3 {
        let flat = self.dist * self.pitch.cos();
        add(
            self.t,
            [
                flat * self.yaw.sin(),
                self.dist * self.pitch.sin(),
                flat * self.yaw.cos(),
            ],
        )
    }

    /// Its view matrix.
    #[must_use]
    pub fn view(&self) -> M4 {
        M4::look_at(self.eye(), self.t, [0.0, 1.0, 0.0])
    }

    /// Its view and projection at `aspect`.
    #[must_use]
    pub fn vp(&self, aspect: f32) -> M4 {
        M4::perspective(FOV, aspect, NEAR, FAR).mul(&self.view())
    }

    /// The point on the plane y = `py` under a screen position (0–1 each
    /// way, from the top left), when the ray meets it.
    #[must_use]
    pub fn ground(&self, sx: f32, sy: f32, aspect: f32, py: f32) -> Option<V3> {
        let e = self.eye();
        let f = norm(sub(self.t, e));
        let r = norm(cross(f, [0.0, 1.0, 0.0]));
        let u = cross(r, f);
        let th = (FOV / 2.0).tan();
        let d = norm(add(
            f,
            add(
                scale(r, (sx * 2.0 - 1.0) * th * aspect),
                scale(u, (1.0 - sy * 2.0) * th),
            ),
        ));
        if d[1].abs() < 1e-4 {
            return None;
        }
        let k = (py - e[1]) / d[1];
        (k > 0.0).then(|| add(e, scale(d, k)))
    }

    /// `a` towards `b` by `k` (0–1): the shorter way round, the distance
    /// eased geometrically.
    #[must_use]
    pub fn lerp(a: Self, b: Self, k: f32) -> Self {
        let mut dy = b.yaw - a.yaw;
        while dy > PI {
            dy -= TAU;
        }
        while dy < -PI {
            dy += TAU;
        }
        Self {
            t: add(a.t, scale(sub(b.t, a.t), k)),
            yaw: a.yaw + dy * k,
            pitch: a.pitch + (b.pitch - a.pitch) * k,
            dist: a.dist * (b.dist / a.dist).powf(k),
        }
    }

    /// The same, turned to `yaw`.
    #[must_use]
    pub fn with_yaw(self, yaw: f32) -> Self {
        Self { yaw, ..self }
    }

    /// Moved by a drag of (`dx`, `dy`) device pixels on a frame `h` high.
    pub fn pan(&mut self, dx: f32, dy: f32, h: u32) {
        let k = self.dist * 2.0 * (FOV / 2.0).tan() / h.max(1) as f32;
        let right = [self.yaw.cos(), 0.0, -self.yaw.sin()];
        let ahead = [-self.yaw.sin(), 0.0, -self.yaw.cos()];
        self.t = add(
            self.t,
            add(
                scale(right, -dx * k),
                scale(ahead, dy * k / self.pitch.sin().max(0.3)),
            ),
        );
    }

    /// Turned and tilted by a drag (tilt within its limits).
    pub fn turn(&mut self, dx: f32, dy: f32) {
        self.yaw -= dx * 0.008;
        self.pitch = (self.pitch + dy * 0.006).clamp(PITCH_MIN, PITCH_MAX);
    }
}

/// The office's size for framing: a floor's width and how many floors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    /// A floor's width, metres.
    pub width: f32,
    /// How many floors.
    pub floors: usize,
}

impl Frame {
    /// The farthest the camera goes.
    #[must_use]
    pub fn max_dist(&self) -> f32 {
        self.width * 3.2
    }

    /// The farthest the camera goes on one floor: the whole floor small
    /// in the window, never so far that it reads as the building.
    #[must_use]
    pub fn floor_max_dist(&self, aspect: f32) -> f32 {
        (self.home(Some(0), aspect).dist * 1.6).min(self.max_dist())
    }

    /// The default view of floor `focus` (or the building), framed to fit
    /// at `aspect`.
    #[must_use]
    pub fn home(&self, focus: Option<usize>, aspect: f32) -> Cam {
        let half_w = self.width / 2.0 + 2.0;
        let fit = half_w / ((FOV / 2.0).tan() * aspect.max(0.5));
        if let Some(f) = focus {
            return Cam {
                t: [self.width / 2.0, floor_y(f), DEPTH / 2.0 + 0.5],
                yaw: DEFAULT_YAW,
                pitch: DEFAULT_PITCH,
                dist: fit * 0.98,
            };
        }
        let top = floor_y(self.floors.max(1) - 1);
        let tall = (top + 14.0) / ((FOV / 2.0).tan() * 2.0) * 1.9;
        Cam {
            t: [self.width / 2.0, top / 2.0 + 1.0, DEPTH / 2.0],
            yaw: DEFAULT_YAW,
            pitch: 0.5,
            dist: (fit * 1.05).max(tall),
        }
    }
}

/// How the building view sees the tower: straight on, from in front and
/// a little above, with no perspective. Its view and projection at
/// `aspect`, and where it looks from.
#[must_use]
pub fn tower_view(frame: &Frame, aspect: f32) -> (M4, V3) {
    let (mid, half_h) = crate::scene::tower::tower_middle(frame.width, frame.floors);
    // the building with a little of the street and lawn either side
    let half_w = crate::scene::tower::TOWER_W / 2.0 + 9.0;
    let half_h = (half_h + 2.0).max(half_w / aspect.max(0.1));
    let half_w = half_h * aspect;
    // from across the street, a little above and to the right: the cut
    // face straight on enough to read, a side wall and the roof showing,
    // and the sky above the lawn
    let eye = [mid[0] + 18.0, mid[1] + 7.0, mid[2] + 80.0];
    let vp = M4::ortho(-half_w, half_w, -half_h, half_h, 1.0, 400.0).mul(&M4::look_at(
        eye,
        mid,
        [0.0, 1.0, 0.0],
    ));
    (vp, eye)
}

/// What is in view: a floor (or the building), and the camera.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    /// The floor in view; `None` for the whole building.
    pub focus: Option<usize>,
    /// The camera.
    pub cam: Cam,
}

impl View {
    /// The floors to draw (none in the building view: the tower stands in
    /// for them).
    #[must_use]
    pub fn floors(&self, frame: &Frame) -> Vec<usize> {
        let _ = frame;
        self.focus.map_or_else(Vec::new, |f| vec![f])
    }

    /// Its view and projection at `aspect` (the tower's in the building
    /// view), and where it looks from.
    #[must_use]
    pub fn sight(&self, frame: &Frame, aspect: f32) -> (M4, V3) {
        if self.focus.is_none() {
            tower_view(frame, aspect)
        } else {
            (self.cam.vp(aspect), self.cam.eye())
        }
    }

    /// Zooms by `step` (positive: in) towards the screen point (`sx`,
    /// `sy`). On a floor it stays on that floor: one floor at a time, out
    /// to [`Frame::floor_max_dist`] (0.4.1: zooming out used to switch to
    /// the stacked building, and the floor below appeared from nowhere).
    /// The building view doesn't zoom (AJ-E3516: a floor is entered by a
    /// click or its tab, never by a zoom).
    pub fn zoom_at(&mut self, frame: &Frame, step: f32, (sx, sy): (f32, f32), aspect: f32) {
        if self.focus.is_none() {
            return;
        }
        let c = &mut self.cam;
        let far = if self.focus.is_some() {
            frame.floor_max_dist(aspect)
        } else {
            frame.max_dist()
        };
        // Never pulled in by a limit: a view already farther (a tween from
        // the building) only stops growing.
        let nd = (c.dist * 0.88f32.powf(step)).clamp(MIN_DIST, far.max(c.dist));
        if let Some(g) = c.ground(sx, sy, aspect, c.t[1]) {
            c.t = add(c.t, scale(sub(g, c.t), 1.0 - nd / c.dist));
        }
        c.dist = nd;
    }
}

/// The box the sun's (or moon's) shadow map covers: fitted round
/// everything drawn, `lo` to `hi`, as seen along the light, so every
/// part lands inside the map. (2026-11-06: a box of a fixed size round
/// the camera's target left an end of the floor outside it when panned,
/// and the map's edge, stretched across, drew pale streaks.)
#[must_use]
pub fn shadow_box(sun: V3, lo: V3, hi: V3) -> M4 {
    let centre = scale(add(lo, hi), 0.5);
    let dir = norm(sun);
    let half = scale(sub(hi, lo), 0.5);
    let reach = (half[0] * half[0] + half[1] * half[1] + half[2] * half[2]).sqrt();
    // straight up would make look_at's up vector useless
    let up = if dir[1].abs() > 0.99 {
        [0.0, 0.0, 1.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let eye = add(centre, scale(dir, reach + 10.0));
    let view = M4::look_at(eye, centre, up);
    let (mut x0, mut x1, mut y0, mut y1, mut z0, mut z1) =
        (f32::MAX, f32::MIN, f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    for k in 0..8 {
        let c = [
            if k & 1 == 0 { lo[0] } else { hi[0] },
            if k & 2 == 0 { lo[1] } else { hi[1] },
            if k & 4 == 0 { lo[2] } else { hi[2] },
        ];
        let v = view.project(c);
        x0 = x0.min(v[0]);
        x1 = x1.max(v[0]);
        y0 = y0.min(v[1]);
        y1 = y1.max(v[1]);
        // the view looks down -z
        z0 = z0.min(-v[2]);
        z1 = z1.max(-v[2]);
    }
    let m = 0.5;
    M4::ortho(x0 - m, x1 + m, y0 - m, y1 + m, (z0 - m).max(0.1), z1 + m).mul(&view)
}

#[cfg(test)]
mod tests {
    use super::{Cam, Frame, PITCH_MAX, PITCH_MIN, View};

    const FRAME: Frame = Frame {
        width: 44.4,
        floors: 2,
    };

    /// Every corner of what's drawn lands inside the shadow map, for the
    /// sun and the moon from any side, a floor high up or the ground.
    #[test]
    fn the_shadow_box_holds_all_that_is_drawn() {
        for (lo, hi) in [
            ([-2.0, -0.5, -2.0], [46.0, 9.0, 32.0]),
            ([-2.0, 59.5, -2.0], [90.0, 69.0, 32.0]),
            ([5.0, -1.0, 10.0], [40.0, 30.0, 26.0]),
        ] {
            for el in [3.0f32, 20.0, 34.0, 62.0, 89.5] {
                for az in [0.0f32, 45.0, 135.0, 250.0, 315.0] {
                    let (e, a) = (el.to_radians(), az.to_radians());
                    let sun = [e.cos() * a.sin(), e.sin(), e.cos() * a.cos()];
                    let m = super::shadow_box(sun, lo, hi);
                    for k in 0..8 {
                        let c = [
                            if k & 1 == 0 { lo[0] } else { hi[0] },
                            if k & 2 == 0 { lo[1] } else { hi[1] },
                            if k & 4 == 0 { lo[2] } else { hi[2] },
                        ];
                        let p = m.project(c);
                        let (x, y, z) = (p[0] / p[3], p[1] / p[3], p[2] / p[3]);
                        assert!(
                            x.abs() < 1.0 && y.abs() < 1.0 && (0.0..1.0).contains(&z),
                            "{c:?} at {el}°, {az}°: {x} {y} {z}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn home_frames_a_floor_and_the_building_from_above() {
        let floor = FRAME.home(Some(1), 1.6);
        assert!(floor.eye()[1] > floor.t[1], "above the floor");
        assert!((floor.t[1] - 15.0).abs() < 1e-4);
        let building = FRAME.home(None, 1.6);
        assert!(building.dist > floor.dist);
        let g = floor.ground(0.5, 0.5, 1.6, floor.t[1]).unwrap_or_default();
        assert!(
            (g[0] - floor.t[0]).abs() < 0.01 && (g[2] - floor.t[2]).abs() < 0.01,
            "the centre of the screen is the target"
        );
    }

    #[test]
    fn turning_keeps_the_tilt_within_its_limits_and_tweens_go_the_short_way() {
        let mut cam = FRAME.home(Some(0), 1.6);
        cam.turn(0.0, 10_000.0);
        assert!((cam.pitch - PITCH_MAX).abs() < 1e-6);
        cam.turn(0.0, -10_000.0);
        assert!((cam.pitch - PITCH_MIN).abs() < 1e-6);
        let a = Cam { yaw: 3.0, ..cam };
        let b = Cam { yaw: -3.0, ..cam };
        let mid = Cam::lerp(a, b, 0.5);
        assert!(mid.yaw.abs() > 3.0, "through π, not through 0: {}", mid.yaw);
        let before = cam.t;
        cam.pan(100.0, 0.0, 800);
        assert!(crate::math::len(crate::math::sub(cam.t, before)) > 0.01);
    }

    #[test]
    fn zooming_never_switches_between_a_floor_and_the_building() {
        let mut view = View {
            focus: Some(1),
            cam: FRAME.home(Some(1), 1.6),
        };
        for _ in 0..40 {
            view.zoom_at(&FRAME, -1.0, (0.5, 0.5), 1.6);
        }
        assert_eq!(view.focus, Some(1), "one floor at a time (0.4.1)");
        assert_eq!(view.floors(&FRAME), [1]);
        assert!(view.cam.dist <= FRAME.floor_max_dist(1.6) + 1e-3);
        let mut building = View {
            focus: None,
            cam: FRAME.home(None, 1.6),
        };
        assert!(building.floors(&FRAME).is_empty(), "the tower instead");
        for _ in 0..40 {
            building.zoom_at(&FRAME, 1.0, (0.5, 0.5), 1.6);
        }
        assert_eq!(
            building.focus, None,
            "AJ-E3516 R5: zoom never enters a floor"
        );
        assert_eq!(building.cam, FRAME.home(None, 1.6));
        // the tower is framed whole, straight on
        let (vp, eye) = building.sight(&FRAME, 1.6);
        let (mid, _) = crate::scene::tower::tower_middle(FRAME.width, FRAME.floors);
        let c = vp.project(mid);
        assert!((c[0] / c[3]).abs() < 0.01 && (c[1] / c[3]).abs() < 0.2);
        assert!(eye[2] > mid[2], "from in front");
    }
}
