//! One scheduler for everything time-based (ADR-0039, D-151; AJ-E2027).
//!
//! - **The trigger maths** ([`trigger`]): pure; next due, missed runs,
//!   DST, windows, jitter, in the user's local time zone (`jiff`).

pub mod trigger;
