//! What a part shows of the hour and of time unused: its glow only at
//! night (AJ-E3516's building view: street lamps, lit windows, the roof's
//! sign) and its dust (AJ-E3518): the flags the renderer reads, and the
//! builder's call for them.

use crate::colour::Rgba;
use crate::math::T;
use crate::mesh::Mesh;
use crate::models::B;

/// The first of two bits holding a part's dust (0–3): what's long unused
/// gathers it (0.4.1's look draws it; the classic look ignores it).
pub const DUST_SHIFT: u32 = 3;

/// The dust on a part of an App dimmed to `dim` ([`crate::scene::dimming`]):
/// none while in use, some after a week, thick after a month.
#[must_use]
pub fn dust_of(dim: f32) -> u32 {
    // AJ-E3518: from a day (40% light) a film, thicker after a week
    // (32%) and a month (25%)
    if dim <= 0.27 {
        3
    } else if dim <= 0.34 {
        2
    } else {
        u32::from(dim <= 0.42)
    }
}

/// A part's glow shows only at night; by day it's its plain colour.
pub const NIGHT: u32 = 32;
/// With [`NIGHT`]: the glow is lamplight's warm colour, not the part's
/// own (a window, glass by day).
pub const WARM: u32 = 64;

impl B<'_> {
    /// A glowing part with `flags` (such as [`NIGHT`]).
    pub fn raw_glow(&mut self, mesh: Mesh, t: T, c: Rgba, glow: f32, flags: u32) {
        self.raw(mesh, t, c, None, glow, flags, false);
    }
}
