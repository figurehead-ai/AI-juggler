//! The world model's upkeep inside ADR-0037's maintenance (§2.3): two
//! minutes after the first paint, then hourly, never while an AI job is
//! starting or answering; each batch at most [`BATCH`] writes in one short
//! writer hold, at most [`PASS`] a pass, stopping as soon as a job
//! arrives (the next pass carries on from its cursor). One
//! `memory.maintained` event per pass, counts only.
//!
//! Each pass, in order: build the index if it isn't (lazily, here, after
//! the paint); apply the queued uses; the one-time linking of ADR-0037's
//! records ([`super::link::link_existing`]); the entities as the records
//! relate them now; the `memory` nodes whose record is gone, forgotten;
//! then the idle pass ("dreaming": tiers, decay, archive, duplicates,
//! folds, weak links; [`gyrus::dream`]).

use std::sync::Arc;
use std::time::Duration;

use gyrus::dream::{DreamCounts, DreamOpts};
use juggler_core::storage::memory::{Actor, NewEvent, Stream};

use super::link;
use crate::state::AppState;

/// The first pass, after the first paint.
pub const FIRST_AFTER: Duration = Duration::from_secs(120);
/// Then at most hourly.
pub const PERIOD: Duration = Duration::from_secs(3600);
/// The most writes in one hold.
pub const BATCH: usize = 200;
/// The most writes in one pass.
pub const PASS: usize = 2_000;

/// What one pass did (numbers only).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Passed {
    /// The index was built in this pass.
    pub built: bool,
    /// Uses applied.
    pub uses: usize,
    /// Records linked by the one-time pass.
    pub linked: bool,
    /// Entity writes.
    pub entities: usize,
    /// Memory nodes whose record was gone, forgotten.
    pub dangling: usize,
    /// The idle pass's counts.
    pub dreamed: DreamCounts,
    /// Writes in all.
    pub writes: usize,
    /// It stopped early for a job.
    pub stopped: bool,
}

/// Starts the hourly pass (the app's ticks call it once).
pub fn start(state: &Arc<AppState>) {
    let state = Arc::clone(state);
    crate::ticks::every_after_paint("world-upkeep", FIRST_AFTER, PERIOD, move || {
        let busy = || crate::features::ai::run::busy(crate::features::ai::service::shared());
        if !busy() {
            let _ = pass(&state, &busy);
        }
    });
}

fn chunks(batch: &gyrus::Batch) -> Vec<gyrus::Batch> {
    batch
        .ops
        .chunks(BATCH)
        .map(|ops| gyrus::Batch { ops: ops.to_vec() })
        .collect()
}

/// One pass (see the module). `busy` says a job arrived: the pass stops
/// at the next batch.
pub fn pass(state: &Arc<AppState>, busy: &dyn Fn() -> bool) -> Passed {
    let mut out = Passed::default();
    let Some(graph) = state.world.graph() else {
        return out;
    };
    let now = || state.notifier.clock().now;
    {
        let Ok(storage) = state.storage.lock() else {
            return out;
        };
        let reader = storage.world();
        if !reader.installed() {
            return out;
        }
        out.built = reader.ensure(graph).unwrap_or(false);
        out.uses = storage.memory().flush_world_uses(graph, now()).unwrap_or(0);
    }
    let Some(writer) = state.world.writer(state) else {
        return out;
    };
    // The one-time pass over the records written before this version.
    let mut cursor = Some(0);
    while let Some(after) = cursor {
        if busy() {
            out.stopped = true;
            return finish(state, out);
        }
        let Ok(storage) = state.storage.lock() else {
            return out;
        };
        if storage
            .app_settings()
            .get(link::LINKED_SETTING)
            .ok()
            .flatten()
            .is_some_and(|v| v == "1")
        {
            break;
        }
        cursor = link::link_existing(&storage, graph, writer, after, now());
        out.linked = true;
        out.writes += 1;
    }
    // The entities as the records relate them now.
    if let Ok(world) = crate::features::bot::world::gather(state) {
        let snap = graph.snapshot().ok();
        let batch = link::entities_batch(&world, snap.as_deref(), graph.pack(), now());
        for part in chunks(&batch) {
            if busy() || out.writes >= PASS {
                out.stopped = busy();
                return finish(state, out);
            }
            let Ok(storage) = state.storage.lock() else {
                return out;
            };
            out.entities += part.ops.len();
            out.writes += part.ops.len();
            let _ = state.world.write(state, &storage, &part, now());
        }
    }
    // A memory node whose record is gone (cleared another way) goes.
    if let Ok(storage) = state.storage.lock() {
        out.dangling = storage
            .memory()
            .forget_dangling_world(graph, BATCH)
            .unwrap_or(0);
    }
    // The idle pass, batch by batch from its cursor.
    let mut after = 0;
    loop {
        if busy() || out.writes >= PASS {
            out.stopped = busy();
            break;
        }
        let Ok(storage) = state.storage.lock() else {
            break;
        };
        let Ok(snap) = graph.snapshot() else {
            break;
        };
        let opts = DreamOpts {
            after,
            ..DreamOpts::new(now(), BATCH)
        };
        let Ok(planned) = storage.world().dream(&snap, graph.pack(), &opts) else {
            break;
        };
        drop(snap);
        add(&mut out.dreamed, planned.counts);
        out.writes += planned.batch.ops.len();
        if !planned.batch.is_empty() {
            let _ = state.world.write(state, &storage, &planned.batch, now());
        }
        match planned.next {
            Some(next) => after = next,
            None => break,
        }
    }
    finish(state, out)
}

const fn add(total: &mut DreamCounts, more: DreamCounts) {
    total.scanned += more.scanned;
    total.promoted += more.promoted;
    total.decayed += more.decayed;
    total.salience += more.salience;
    total.archived += more.archived;
    total.folded += more.folded;
    total.proposed += more.proposed;
    total.merged += more.merged;
    total.pruned += more.pruned;
}

/// The pass's one log event, counts only.
fn finish(state: &Arc<AppState>, out: Passed) -> Passed {
    if let Ok(storage) = state.storage.lock() {
        let mut event = NewEvent::new(
            state.notifier.clock().now,
            Stream::Memory,
            "maintained",
            Actor::App,
        );
        let d = out.dreamed;
        event.payload = serde_json::json!({
            "world": {
                "built": out.built,
                "uses": out.uses,
                "entities": out.entities,
                "dangling": out.dangling,
                "promoted": d.promoted,
                "decayed": d.decayed,
                "salience": d.salience,
                "archived": d.archived,
                "folded": d.folded,
                "proposed": d.proposed,
                "merged": d.merged,
                "pruned": d.pruned,
                "writes": out.writes,
                "stopped": out.stopped,
            }
        });
        let _ = storage.ai_log().append(&event, None);
    }
    out
}
