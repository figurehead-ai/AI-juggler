//! AJ-E2016: building blocks shared by every tool's adapter, so a new tool
//! is mostly data plus a few small impls. Nothing here names a tool.
//!
//! - [`layout`]: what differs between tools' config homes.
//! - [`home`]: scanning, inspecting, copying and adopting a config home.
//! - [`jsonl`]: reading a file one bounded line at a time.
//! - [`probe`]: asking an installed tool its version.
//! - [`sqlite`]: reading another tool's SQLite file without changing it.

pub(crate) mod home;
pub(crate) mod jsonl;
pub(crate) mod layout;
pub(crate) mod probe;
pub(crate) mod sqlite;
