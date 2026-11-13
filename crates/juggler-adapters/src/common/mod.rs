//! AJ-E2016: building blocks shared by every tool's adapter, so a new tool
//! is mostly data plus a few small impls. Nothing here names a tool.
//!
//! - [`layout`]: what differs between tools' config homes.
//! - [`driven`]: the terminal launch a driven runtime starts from and
//!   resumes in.
//! - [`home`]: scanning, inspecting, copying and adopting a config home.
//! - [`jsonl`]: reading a file one bounded line at a time.
//! - [`prelude`]: the traits and types every tool's adapter module names.
//! - [`probe`]: asking an installed tool its version.
//! - [`sqlite`]: reading another tool's SQLite file without changing it.

#[cfg(test)]
pub(crate) mod bridge_sample;
pub(crate) mod driven;
pub(crate) mod home;
pub(crate) mod jsonl;
pub(crate) mod layout;
pub(crate) mod prelude;
pub(crate) mod probe;
pub(crate) mod sqlite;
