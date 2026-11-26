//! The turn review (AJ-E3820, ADR-0033 §6, D-131, D-149): when an agent's
//! turn ends, a short, checked review of what it did, without opening the
//! transcript: what changed, what was reverted, what failed, what it asks
//! and the risk.
//!
//! - [`facts`]: the code-only review, from the turn's own facts (its edit
//!   calls and the agent's own words, never git), the default and the
//!   no-model path.
//! - [`lines`]: the model's Beta lines on it (a description per changed
//!   file and the risk), each through Bot's shared guard and check.
//! - [`read`]: reading a turn (only an App whose turn ends are seen, R5;
//!   only while Last messages is on).
//! - [`commands`]: the IPC.
//!
//! Shown where the turn's end is shown (the session page and the turn's
//! feed post), behind the same "show" as the last reply; never a
//! notification of its own.

pub mod commands;
pub mod dto;
pub mod facts;
pub mod lines;
pub mod read;

#[cfg(test)]
mod tests;
