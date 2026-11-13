//! AJ-E2016: building blocks shared by every tool's adapter, so a new tool
//! is mostly data plus a few small impls. Nothing here names a tool.
//!
//! - [`layout`]: what differs between tools' config homes.
//! - [`driven`]: the terminal launch a driven runtime starts from and
//!   resumes in.
//! - [`home`]: scanning, inspecting, copying and adopting a config home.
//! - [`jsonl`]: reading a file one bounded line at a time.
//! - [`launch`]: the folder a launch runs in; a running tool's command.
//! - [`prelude`]: the traits and types every tool's adapter module names.
//! - [`probe`]: finding an installed tool and asking its version.
//! - [`rpc_codec`]: what the JSON-RPC driven codecs read alike.
//! - [`sqlite`]: reading another tool's SQLite file without changing it.

#[cfg(test)]
pub(crate) mod bridge_sample;
pub(crate) mod driven;
pub(crate) mod home;
#[cfg(test)]
pub(crate) mod inventory_sample;
pub(crate) mod jsonl;
pub(crate) mod launch;
pub(crate) mod layout;
pub(crate) mod prelude;
pub(crate) mod probe;
pub(crate) mod rpc_codec;
pub(crate) mod sqlite;
