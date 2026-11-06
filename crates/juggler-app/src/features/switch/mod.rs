//! Switching a session to another account (AJ-E205 to AJ-E209): the preflight
//! and its door, the switch itself, replay, undo, switch-back, the targets
//! and the Switch Graph, and resuming an epoch.

pub mod back;
pub mod commands;
pub mod dto;
pub mod edges;
pub mod execute;
pub mod graph;
pub mod preflight;
pub mod replay;
pub mod resume;
pub mod targets;
pub mod undo;
