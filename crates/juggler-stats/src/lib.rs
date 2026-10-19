//! Opt-in usage statistics (D-19, ADR-0015).
//!
//! - **Nothing until opted in.** In `off` (the default) [`record`] returns
//!   at once and no `stats/` folder exists.
//! - **Closed vocabulary.** [`StatEvent`] and [`StatGauge`] are generated
//!   from `stats/schema.v1.json` and carry enums and numbers only — no
//!   `String`, so no path, label, name or error text can be counted.
//! - **Never in the way.** [`record`] hands a `Copy` value to a bounded
//!   channel and returns; when the channel is full the event is dropped
//!   and counted in `stats.dropped`. A background thread writes
//!   `stats/counters.sqlite` every 30 s and on [`shutdown`].
//! - **Off means gone.** [`delete_all`] removes the whole folder.
//!
//! - **One report a week.** [`cycle::tick`] closes ISO weeks into
//!   reports and uploads at most one due report per tick (AJ-E1104).

#![cfg_attr(
    test,
    allow(
        clippy::pedantic,
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic
    )
)]

pub mod buckets;
pub mod config;
pub mod cycle;
mod generated;
pub mod mode;
pub mod pow;
pub mod report;
pub mod store;
pub mod week;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender, TrySendError};
use std::sync::{Arc, Mutex, PoisonError, RwLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub use buckets::CliVersion;
pub use generated::*;
pub use mode::{EffectiveMode, Environment, Mode, Override, effective};
pub use store::{Store, StoreError};

/// The channel's capacity; beyond it events are dropped (and counted).
pub const CHANNEL_CAPACITY: usize = 1024;
/// How often the background thread writes to disk.
pub const FLUSH_EVERY: Duration = Duration::from_secs(30);

enum Message {
    Event {
        at_ms: i64,
        event: StatEvent,
        by: u64,
    },
    Gauge {
        at_ms: i64,
        gauge: StatGauge,
    },
    Flush(SyncSender<()>),
    Stop,
}

/// Dimensions as the store keys them: canonical JSON, keys sorted.
pub(crate) fn dims_key(dims: &[(&str, &str)]) -> String {
    let map: BTreeMap<&str, &str> = dims.iter().copied().collect();
    serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_owned())
}

/// The UTC date of `at_ms` (`2026-10-18`).
fn utc_day(at_ms: i64) -> String {
    let (year, month, day) = week::civil(at_ms.div_euclid(86_400_000));
    format!("{year:04}-{month:02}-{day:02}")
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

/// What the background thread has gathered since the last write.
#[derive(Default)]
struct Pending {
    counters: BTreeMap<(String, &'static str, String), u64>,
    gauges: BTreeMap<(String, &'static str, String), String>,
    days: std::collections::BTreeSet<String>,
}

impl Pending {
    #[allow(clippy::needless_pass_by_value)] // a message is consumed by design
    fn take(&mut self, message: Message) {
        match message {
            Message::Event { at_ms, event, by } => {
                let mut dims = event.dims();
                let bucket;
                if let Some(value) = event.observation() {
                    let meta = METRICS.iter().find(|m| m.name == event.metric());
                    let buckets = meta.map_or(&[][..], |m| m.buckets);
                    bucket = buckets
                        .get(buckets::histogram_index(buckets, value))
                        .copied()
                        .unwrap_or("");
                    dims.push(("bucket", bucket));
                }
                let key = (week::label(at_ms), event.metric(), dims_key(&dims));
                let slot = self.counters.entry(key).or_default();
                *slot = slot.saturating_add(by);
                self.days.insert(utc_day(at_ms));
            }
            Message::Gauge { at_ms, gauge } => {
                let key = (week::label(at_ms), gauge.metric(), dims_key(&gauge.dims()));
                self.gauges.insert(key, gauge.value());
            }
            Message::Flush(_) | Message::Stop => {}
        }
    }

    fn is_empty(&self) -> bool {
        self.counters.is_empty() && self.gauges.is_empty() && self.days.is_empty()
    }

    fn write(&mut self, store: &Store) -> Result<(), StoreError> {
        if self.is_empty() {
            return Ok(());
        }
        let pending = std::mem::take(self);
        store.transaction(|store| {
            for ((week, metric, dims), by) in &pending.counters {
                store.add(week, metric, dims, *by)?;
            }
            for ((week, metric, dims), value) in &pending.gauges {
                store.set_gauge(week, metric, dims, value)?;
            }
            for day in &pending.days {
                store.mark_active(day)?;
            }
            Ok(())
        })
    }
}

fn run(store: &Store, rx: &Receiver<Message>, dropped: &AtomicU64) {
    let mut pending = Pending::default();
    let mut next = Instant::now() + FLUSH_EVERY;
    loop {
        let wait = next.saturating_duration_since(Instant::now());
        let (ack, stop) = match rx.recv_timeout(wait) {
            Ok(Message::Flush(ack)) => (Some(ack), false),
            Ok(Message::Stop) | Err(RecvTimeoutError::Disconnected) => (None, true),
            Ok(message) => {
                pending.take(message);
                continue;
            }
            Err(RecvTimeoutError::Timeout) => (None, false),
        };
        let lost = dropped.swap(0, Ordering::Relaxed);
        if lost > 0 {
            pending.take(Message::Event {
                at_ms: now_ms(),
                event: StatEvent::StatsDropped,
                by: lost,
            });
        }
        if let Err(e) = pending.write(store) {
            eprintln!("{e}");
        }
        next = Instant::now() + FLUSH_EVERY;
        if let Some(ack) = ack {
            let _ = ack.send(());
        }
        if stop {
            return;
        }
    }
}

/// A running (or disabled) recorder.
pub struct Stats {
    effective: EffectiveMode,
    dir: PathBuf,
    tx: Option<SyncSender<Message>>,
    dropped: Arc<AtomicU64>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl std::fmt::Debug for Stats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stats")
            .field("effective", &self.effective)
            .field("dir", &self.dir)
            .field("running", &self.tx.is_some())
            .finish_non_exhaustive()
    }
}

impl Stats {
    /// A recorder that records nothing and touches no disk.
    #[must_use]
    pub fn disabled(dir: &Path, effective: EffectiveMode) -> Self {
        Self {
            effective,
            dir: dir.to_path_buf(),
            tx: None,
            dropped: Arc::new(AtomicU64::new(0)),
            worker: Mutex::new(None),
        }
    }

    /// Starts recording into `dir` (the app's `stats/` folder). In `off`
    /// this is [`Stats::disabled`] and creates nothing.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when the store can't be opened.
    pub fn start(dir: &Path, effective: EffectiveMode) -> Result<Self, StoreError> {
        Self::start_with_capacity(dir, effective, CHANNEL_CAPACITY)
    }

    fn start_with_capacity(
        dir: &Path,
        effective: EffectiveMode,
        capacity: usize,
    ) -> Result<Self, StoreError> {
        if effective.mode == Mode::Off {
            return Ok(Self::disabled(dir, effective));
        }
        let store = Store::open(dir)?;
        let (tx, rx) = std::sync::mpsc::sync_channel(capacity);
        let dropped = Arc::new(AtomicU64::new(0));
        let lost = Arc::clone(&dropped);
        let worker = std::thread::Builder::new()
            .name("juggler-stats".into())
            .spawn(move || run(&store, &rx, &lost))?;
        Ok(Self {
            effective,
            dir: dir.to_path_buf(),
            tx: Some(tx),
            dropped,
            worker: Mutex::new(Some(worker)),
        })
    }

    /// What applies (after the overrides).
    #[must_use]
    pub const fn effective(&self) -> EffectiveMode {
        self.effective
    }

    /// The `stats/` folder.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn send(&self, message: Message) {
        let Some(tx) = &self.tx else { return };
        match tx.try_send(message) {
            Ok(()) | Err(TrySendError::Disconnected(_)) => {}
            Err(TrySendError::Full(_)) => {
                self.dropped.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Counts `event` once. Never blocks, never fails.
    pub fn record(&self, event: StatEvent) {
        self.record_n(event, 1);
    }

    /// Counts `event` `by` times (a counter's increment).
    pub fn record_n(&self, event: StatEvent, by: u64) {
        if self.tx.is_some() {
            self.send(Message::Event {
                at_ms: now_ms(),
                event,
                by,
            });
        }
    }

    /// Records `event` as if at `at_ms` (tests and week-close snapshots).
    pub fn record_at(&self, at_ms: i64, event: StatEvent, by: u64) {
        self.send(Message::Event { at_ms, event, by });
    }

    /// Sets a gauge for this week (last value wins).
    pub fn set_gauge(&self, gauge: StatGauge) {
        self.set_gauge_at(now_ms(), gauge);
    }

    /// Sets a gauge as if at `at_ms`.
    pub fn set_gauge_at(&self, at_ms: i64, gauge: StatGauge) {
        self.send(Message::Gauge { at_ms, gauge });
    }

    /// Writes everything recorded so far and waits for it (≤ 2 s).
    pub fn flush(&self) {
        let Some(tx) = &self.tx else { return };
        let (ack, done) = std::sync::mpsc::sync_channel(1);
        // A full channel means the worker is busy draining; block briefly.
        if tx.send(Message::Flush(ack)).is_ok() {
            let _ = done.recv_timeout(Duration::from_secs(2));
        }
    }

    /// Flushes and stops the background thread.
    pub fn stop(&self) {
        let worker = self
            .worker
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        // Only a running worker drains the channel, so only then can the
        // stop message wait for room.
        if let (Some(worker), Some(tx)) = (worker, &self.tx) {
            let _ = tx.send(Message::Stop);
            let _ = worker.join();
        }
    }
}

impl Drop for Stats {
    fn drop(&mut self) {
        self.stop();
    }
}

static GLOBAL: RwLock<Option<Arc<Stats>>> = RwLock::new(None);

/// Makes `stats` the process-wide recorder (stopping any previous one).
pub fn install(stats: Stats) {
    let previous = GLOBAL
        .write()
        .unwrap_or_else(PoisonError::into_inner)
        .replace(Arc::new(stats));
    if let Some(previous) = previous {
        previous.stop();
    }
}

/// The process-wide recorder, if one is installed.
#[must_use]
pub fn current() -> Option<Arc<Stats>> {
    GLOBAL
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

/// A recorder is installed (the mode isn't off). Cheap: callers check it
/// before building an event, so `off` costs a lock read.
#[must_use]
pub fn recording() -> bool {
    GLOBAL
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .is_some_and(|stats| stats.tx.is_some())
}

/// Counts `event` on the process-wide recorder. Never blocks, never fails;
/// a no-op until one is installed and the mode isn't off.
pub fn record(event: StatEvent) {
    record_n(event, 1);
}

/// Counts `event` `by` times on the process-wide recorder.
pub fn record_n(event: StatEvent, by: u64) {
    if let Some(stats) = GLOBAL
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
    {
        stats.record_n(event, by);
    }
}

/// Sets a gauge on the process-wide recorder.
pub fn set_gauge(gauge: StatGauge) {
    if let Some(stats) = GLOBAL
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
    {
        stats.set_gauge(gauge);
    }
}

/// Flushes and stops the process-wide recorder (on quit).
pub fn shutdown() {
    let stats = GLOBAL
        .write()
        .unwrap_or_else(PoisonError::into_inner)
        .take();
    if let Some(stats) = stats {
        stats.stop();
    }
}

/// Records a change of the effective mode at once (so a gap in sharing,
/// however short, makes that week `partial_consent`). Creates nothing in
/// `off`.
///
/// # Errors
///
/// Returns [`StoreError`] on storage failures.
pub fn note_mode(dir: &Path, effective: EffectiveMode) -> Result<(), StoreError> {
    if effective.mode == Mode::Off {
        return Ok(());
    }
    cycle::observe_mode(&Store::open(dir)?, effective, now_ms())
}

/// Turns stats off: stops the recorder and deletes the whole `dir`
/// (counters, reports, salt and upload log).
///
/// # Errors
///
/// Returns the filesystem error.
pub fn delete_all(dir: &Path) -> std::io::Result<()> {
    shutdown();
    store::delete_all(dir)
}

#[cfg(test)]
mod tests;
