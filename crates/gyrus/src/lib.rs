//! # gyrus
//!
//! A typed graph kept in memory over SQLite, for an assistant's world
//! model: connected nodes (people, projects, sessions, memories, facts,
//! rules…), each kind with its own rules on what it links to and how it is
//! walked, and a small fixed set of bounded operations instead of a query
//! language.
//!
//! - **SQLite is the record.** The tables live in the host's database
//!   ([`schema::SCHEMA_V1`], applied as one of the host's migrations), so
//!   a host's own records and their nodes commit in one transaction, and
//!   one forget reaches both.
//! - **The index is in memory** ([`Snapshot`]): links both ways, grouped by
//!   kind, strongest first; rebuilt from SQLite at start
//!   ([`Graph::build`]) and changed only by a committed change set
//!   ([`Graph::publish`]). Readers hold a consistent snapshot; a committed
//!   batch makes the next one.
//! - **Kind packs** ([`Pack`]) declare the node kinds, the link kinds with
//!   their allowed ends, and per kind: its walk rules (which links, which
//!   way, how far, the best K per step), its decay and tiers, and the
//!   reasoning questions it brings into a context. The store refuses any
//!   kind or link the pack doesn't declare.
//! - **Forget really deletes** ([`store::forget_in`], [`Graph::forget`]):
//!   `secure_delete`, the keyword bucket rebuilt, the WAL truncated, then
//!   the in-memory removal.
//!
//! gyrus knows nothing of its host: no host crate is a dependency
//! (`tests/deps.rs`).
//!
//! Must not know: its host application, processes, the network or a UI.

pub mod error;
pub mod graph;
pub mod index;
pub mod ops;
pub mod pack;
pub mod schema;
pub mod store;
pub mod types;

pub use error::{Error, Refusal, Result};
pub use graph::{Built, Graph, Step};
pub use index::{Edge, NodeEntry, RuleOverride, Snapshot};
pub use pack::{Dir, Fold, KindSpec, LinkSpec, Pack, PackBuilder, Roles, TierRules, WalkRule};
pub use schema::{FTS_BUCKETS, SCHEMA_V1};
pub use store::{Erased, LinkRow, NodeRow, Pending};
pub use types::{
    Authority, Batch, Label, LinkId, NewLink, NewNode, NodeId, Op, Tier, key_like, normalise,
};
