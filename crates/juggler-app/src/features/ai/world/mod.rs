//! Bot's world model, the app side (ADR-0038, AJ-E3832): AI Juggler's
//! kind pack over the generic `gyrus` engine, the live resolvers, the
//! linking of ADR-0037's records, Bot's `world` context section, and the
//! idle pass inside ADR-0037's maintenance.
//!
//! - **SQLite is the record** (V56, in AI Juggler's database); the index
//!   is in memory, built lazily: after the first paint by the maintenance
//!   tick, or on Bot's first question when the graph is small
//!   ([`BUILD_NOW_BELOW`]). Until it is ready Bot answers without the
//!   `world` section (ADR-0037's retrieval still runs), so nothing waits.
//! - **Every write passes ADR-0037's write gate**
//!   ([`juggler_core::storage::memory::MemoryStore::write_world`]) with
//!   this module's own writer, taken once at start.
//! - **Entity nodes are pointers**; what they point at is read live
//!   ([`resolve`]).

pub mod assemble;
pub mod link;
pub mod maintain;
pub mod pack;
pub mod resolve;

#[cfg(test)]
mod tests;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

use gyrus::{Graph, Snapshot};
use juggler_core::storage::Storage;
use juggler_core::storage::memory::KindWriter;

use crate::error::AppResult;
use crate::features::ai::memory::{RedactionSecrets, memory_error};
use crate::state::AppState;

/// Below this many nodes the index is built on the spot when Bot needs it
/// (a few milliseconds); above, only in the background.
pub const BUILD_NOW_BELOW: i64 = 50_000;

/// The world model at run time: the graph (its pack and in-memory index)
/// and its writer.
pub struct WorldModel {
    graph: Option<Graph>,
    writer: OnceLock<Option<KindWriter>>,
    warming: AtomicBool,
}

impl std::fmt::Debug for WorldModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorldModel")
            .field("ready", &self.graph.as_ref().is_some_and(Graph::is_ready))
            .finish_non_exhaustive()
    }
}

impl Default for WorldModel {
    fn default() -> Self {
        Self::new()
    }
}

impl WorldModel {
    /// The world model, not built yet.
    #[must_use]
    pub fn new() -> Self {
        Self {
            graph: pack::pack().ok().map(Graph::new),
            writer: OnceLock::new(),
            warming: AtomicBool::new(false),
        }
    }

    /// The graph (`None` only if the pack failed to build, which a test
    /// rules out).
    #[must_use]
    pub const fn graph(&self) -> Option<&Graph> {
        self.graph.as_ref()
    }

    /// This module's writer, taken from the app's writers once.
    pub fn writer<'a>(&'a self, state: &AppState) -> Option<&'a KindWriter> {
        self.writer
            .get_or_init(|| state.memory_writers.take_world())
            .as_ref()
    }

    /// The current index: built, or rebuilt when SQLite moved on without
    /// it, right now when the graph is small; `None` while a large one
    /// warms up in the background (or before V56's tables exist).
    pub fn ready(&self, state: &Arc<AppState>, storage: &Storage) -> Option<Arc<Snapshot>> {
        let graph = self.graph.as_ref()?;
        let reader = storage.world();
        if !reader.installed() {
            return None;
        }
        if reader.check(graph).unwrap_or(false) {
            return graph.snapshot().ok();
        }
        if reader.nodes().unwrap_or(i64::MAX) < BUILD_NOW_BELOW {
            reader.build(graph).ok()?;
            return graph.snapshot().ok();
        }
        self.warm_up(state);
        None
    }

    /// Builds the index on a background thread (once at a time).
    pub fn warm_up(&self, state: &Arc<AppState>) {
        if self.warming.swap(true, Ordering::SeqCst) {
            return;
        }
        let state = Arc::clone(state);
        let spawned = std::thread::Builder::new()
            .name("world-index".into())
            .spawn(move || {
                if let (Some(graph), Ok(storage)) = (state.world.graph(), state.storage.lock()) {
                    let _ = storage.world().ensure(graph);
                }
                state.world.warming.store(false, Ordering::SeqCst);
            });
        if spawned.is_err() {
            self.warming.store(false, Ordering::SeqCst);
        }
    }

    /// Writes a batch through the write gate (published to the index
    /// after the commit). `Ok(false)`: nothing to write, or no writer.
    ///
    /// # Errors
    ///
    /// The gate's refusal; storage failures.
    pub fn write(
        &self,
        state: &AppState,
        storage: &Storage,
        batch: &gyrus::Batch,
        now_ms: i64,
    ) -> AppResult<bool> {
        let (Some(graph), Some(writer)) = (self.graph.as_ref(), self.writer(state)) else {
            return Ok(false);
        };
        if batch.is_empty() {
            return Ok(false);
        }
        storage
            .memory()
            .write_world(writer, &RedactionSecrets, graph, batch, now_ms)
            .map_err(memory_error)?;
        Ok(true)
    }

    /// Applies the uses Bot's contexts queued (ADR-0038's `touch`), in this
    /// writer hold.
    pub fn flush_uses(&self, storage: &Storage, now_ms: i64) {
        if let Some(graph) = self.graph.as_ref() {
            let _ = storage.memory().flush_world_uses(graph, now_ms);
        }
    }
}

/// The one-time linking of ADR-0037's records written before the world
/// model ([`link::link_existing`]), batch by batch, each in its own short
/// hold; `true` once every record is linked (it is marked done and never
/// runs again).
pub fn link_records(state: &Arc<AppState>) -> bool {
    let (Some(graph), Some(writer)) = (state.world.graph(), state.world.writer(state)) else {
        return false;
    };
    let mut cursor = Some(0);
    while let Some(after) = cursor {
        let Ok(storage) = state.storage.lock() else {
            return false;
        };
        if storage
            .app_settings()
            .get(link::LINKED_SETTING)
            .ok()
            .flatten()
            .is_some_and(|v| v == "1")
        {
            return true;
        }
        if state.world.ready(state, &storage).is_none() {
            return false;
        }
        cursor = link::link_existing(&storage, graph, writer, after, state.notifier.clock().now);
    }
    true
}
