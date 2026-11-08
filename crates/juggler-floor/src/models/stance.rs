//! A figure's stance: where its legs start and how they bend, shared by
//! the first and the second-pass person.

use std::f32::consts::FRAC_PI_2;

use super::{Arm, Pose, rig};
use crate::math::T;

/// Where a figure's legs start and how they bend, and its pose's angles.
pub(super) struct Stance {
    pub(super) root: T,
    pub(super) hip_y: f32,
    pub(super) rig: (f32, f32, f32, Arm, Arm, f32),
    thigh: f32,
    knee: f32,
}

impl Stance {
    /// Each leg's hip and knee joints, the hips `spread` either side of
    /// the middle and the knees a thigh's `length` below them.
    pub(super) fn legs(&self, spread: f32, length: f32) -> [(T, T); 2] {
        let walk = self.rig.5;
        [(-1.0f32, walk), (1.0, -walk)].map(|(s, sw)| {
            let hip = self.root.t(s * spread, self.hip_y, 0.0).rx(self.thigh + sw);
            let knee = hip.t(0.0, -length, 0.0).rx(self.knee - sw.max(0.0) * 0.6);
            (hip, knee)
        })
    }
}

/// A figure `height` tall in `pose` at time `t`: seated at `seat` or
/// standing with its hips at `stand` (in its own height).
pub(super) fn stance(root: T, height: f32, pose: Pose, t: f32, seat: f32, stand: f32) -> Stance {
    let sit = pose.sits();
    let (thigh, knee) = if sit {
        (FRAC_PI_2, -FRAC_PI_2)
    } else {
        (0.0, 0.0)
    };
    Stance {
        root: root.s(height, height, height),
        hip_y: if sit { seat / height + 0.02 } else { stand },
        rig: rig(pose, t),
        thigh,
        knee,
    }
}
