//! The Floor's model (SPEC §9.7, ADR-0026, D-67): everything about the
//! office that can be decided without a GPU or a window.
//!
//! - [`layout`]: floors, cells, sizes, the order Apps keep, and each
//!   area's desks.
//! - [`family`] and [`kit`]: the style families a tool names (its
//!   adapter's data) and each App's own variant, chosen once and kept.
//! - [`colour`] and [`rng`]: colours (the app's mark colours among them)
//!   and the seeded randomness every choice comes from, so the same seed
//!   always gives the same office.
//!
//! It runs on every OS and in Linux CI; the renderer
//! (`juggler-floor-render`) and the window's events
//! (`juggler-window-events`) sit beside it.

// Geometry on small, bounded numbers (metres, pixels, a few hundred
// desks): the casts between them can't lose anything that matters.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

pub mod colour;
pub mod family;
pub mod kit;
pub mod layout;
pub mod rng;
pub mod state;
