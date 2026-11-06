//! The orbit camera (SPEC §9.7): a target on a floor, a turn (yaw), a
//! tilt (pitch, 18°–79°) and a distance. Drag pans; right-drag, ⌥-drag, a
//! two-finger twist, Q/E or the arrows turn and tilt; pinch, ⌘-scroll or
//! +/− zoom towards the pointer, out to the building and back to the
//! floor under it; H resets; 0 and the floor numbers pick the building or
//! a floor.

use std::f32::consts::{PI, TAU};

use crate::layout::{DEPTH, STOREY};
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

/// What is in view: a floor (or the building), and the camera.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    /// The floor in view; `None` for the whole building.
    pub focus: Option<usize>,
    /// The camera.
    pub cam: Cam,
}

impl View {
    /// The floors to draw.
    #[must_use]
    pub fn floors(&self, frame: &Frame) -> Vec<usize> {
        self.focus
            .map_or_else(|| (0..frame.floors).collect(), |f| vec![f])
    }

    /// Zooms by `step` (positive: in) towards the screen point (`sx`,
    /// `sy`). Coming back in from the building lands on the floor under
    /// the pointer; going far out shows the building.
    pub fn zoom_at(&mut self, frame: &Frame, step: f32, (sx, sy): (f32, f32), aspect: f32) {
        let c = &mut self.cam;
        let nd = (c.dist * 0.88f32.powf(step)).clamp(MIN_DIST, frame.max_dist());
        if let Some(g) = c.ground(sx, sy, aspect, c.t[1]) {
            c.t = add(c.t, scale(sub(g, c.t), 1.0 - nd / c.dist));
        }
        c.dist = nd;
        if self.focus.is_none() && nd < frame.home(Some(0), aspect).dist * 1.3 {
            let f = ((c.t[1] / STOREY).round().max(0.0) as usize).min(frame.floors.max(1) - 1);
            self.focus = Some(f);
            c.t[1] = floor_y(f);
        }
        if self.focus.is_some() && nd > frame.home(self.focus, aspect).dist * 1.55 {
            self.focus = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Cam, Frame, PITCH_MAX, PITCH_MIN, View};

    const FRAME: Frame = Frame {
        width: 44.4,
        floors: 2,
    };

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
    fn zooming_far_out_shows_the_building_and_back_in_lands_on_a_floor() {
        let mut view = View {
            focus: Some(1),
            cam: FRAME.home(Some(1), 1.6),
        };
        for _ in 0..40 {
            view.zoom_at(&FRAME, -1.0, (0.5, 0.5), 1.6);
        }
        assert_eq!(view.focus, None);
        assert_eq!(view.floors(&FRAME), [0, 1]);
        for _ in 0..40 {
            view.zoom_at(&FRAME, 1.0, (0.5, 0.5), 1.6);
        }
        assert!(view.focus.is_some());
        assert!(view.cam.dist >= super::MIN_DIST);
    }
}
