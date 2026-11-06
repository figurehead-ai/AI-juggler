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
//! - [`state`] and [`snapshot`]: a session's state as the rail says it,
//!   and the company at one moment, as the backend feeds it.
//! - [`math`], [`mesh`] and [`paint`]: the matrices, the low-poly shapes
//!   and the pixel painting the scene is built from.
//!
//! It runs on every OS and in Linux CI; the renderer
//! (`juggler-floor-render`) and the window's events
//! (`juggler-window-events`) sit beside it.
//!
//! Must not know: the GPU, the operating system, Tauri or any tool by name
//! (an area's style comes as data).

// Geometry on small, bounded numbers (metres, pixels, a few hundred
// desks): the casts between them can't lose anything that matters. And
// geometry and pixels have their own short names (x, y, w, h, a, b, c).
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::many_single_char_names
)]

pub mod camera;
pub mod colour;
pub mod driver;
pub mod family;
pub mod kit;
pub mod layout;
pub mod math;
pub mod mesh;
pub mod models;
pub mod overlay;
pub mod paint;
pub mod rng;
pub mod scene;
pub mod seating;
pub mod snapshot;
pub mod state;
pub mod tex;
