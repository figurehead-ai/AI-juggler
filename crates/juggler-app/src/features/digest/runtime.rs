//! The digest at run time: the scheduler fed by the observation bus, its
//! next due time published to the one scheduler as a transient `At`
//! (ADR-0039 §11.4, AJ-E2027: no clock of its own), and a digest made when
//! that fires: the rollup over the window's facts, stored, told to the
//! views (Home's feed reads it), and its calm news handed to the
//! notification path as one notification.
//!
//! The AI digest stays behind its flag and the router (ADR-0034, D-131):
//! `ai_digest_on` is off ("not yet", `v5-final.md`), and no model writer
//! is built, so the rollup writes every digest; the log says why.
//!
//! Lock order: the digest's own lock is taken alone, or before the
//! machine's; never with the store's, and let go before the scheduler is
//! told.

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use juggler_core::storage::Storage;
use juggler_core::storage::memory::KindWriter;

use super::rollup::{AgentFacts, DigestAuthorInfo, Window, notice_words, rollup};
use super::scheduler::{Due, Scheduler};
use super::store::{self, Why};
use crate::features::notifications::batch::Item;
use crate::features::notifications::engine::{Kind, Target};
use crate::features::observe::bell::Bell;
use crate::features::observe::{Observation, Observed, SessionState};
use crate::state::AppState;

/// The app setting that turns the digest off (`false`); on otherwise
/// (ADR-0036 §1.8's "the rollup has no flag; it is the feature", with
/// AJ-E3712 R7's switch in Settings).
pub const ON_KEY: &str = "digest.on";
/// Observations kept per agent between two digests (the oldest go first).
const EVENTS_KEPT: usize = 64;

/// The digest is on (the user's switch; on unless turned off).
#[must_use]
pub fn enabled(storage: &Storage) -> bool {
    storage.app_settings().get(ON_KEY).ok().flatten().as_deref() != Some("false")
}

/// The digest's transient schedule on the one scheduler.
pub const TRANSIENT: &str = "digest";

#[derive(Default)]
struct Inner {
    scheduler: Scheduler,
    window: BTreeMap<String, Vec<(i64, Observed)>>,
    made: u64,
}

type Shared = Bell<Inner>;

/// The digest's run-time state, held by the app's state.
#[derive(Default)]
pub struct Digests {
    shared: Arc<Shared>,
    writer: OnceLock<Option<KindWriter>>,
}

impl std::fmt::Debug for Digests {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Digests").finish_non_exhaustive()
    }
}

impl Digests {
    /// The digest's writer, taken from the state's writers on first use.
    pub fn writer(&self, state: &AppState) -> Option<&KindWriter> {
        self.writer
            .get_or_init(|| state.memory_writers.take_digest())
            .as_ref()
    }

    /// Digests made since the app started.
    #[must_use]
    pub fn made(&self) -> u64 {
        self.shared.lock().map_or(0, |inner| inner.made)
    }

    /// When the next digest is due, if one is.
    #[must_use]
    pub fn due_at(&self) -> Option<i64> {
        self.shared
            .lock()
            .ok()
            .and_then(|inner| inner.scheduler.due_at())
    }
}

/// A new need, a session's ending or a crash brings the digest forward.
const fn notable(event: &Observed) -> bool {
    matches!(
        event,
        Observed::StateEntered {
            state: SessionState::AsksYou
                | SessionState::Stuck
                | SessionState::LimitHit
                | SessionState::Failed
        } | Observed::Ended { .. }
            | Observed::Restarted
            | Observed::AskOpened
    )
}

/// What the bus just fanned out (resolved): each session's observation
/// marks its agent changed and is kept for the window. What was replayed
/// from the journals on start is the watches' alone.
pub fn heard(state: &Arc<AppState>, observations: &[Observation]) {
    let mut any = false;
    if let Ok(mut inner) = state.digests.shared.lock() {
        for obs in observations.iter().filter(|obs| !obs.replayed) {
            let Some(session) = obs.subject.session.as_deref() else {
                continue;
            };
            inner
                .scheduler
                .heard(session, notable(&obs.event), obs.at_ms);
            let events = inner.window.entry(session.to_owned()).or_default();
            if events.len() >= EVENTS_KEPT {
                events.remove(0);
            }
            events.push((obs.at_ms, obs.event.clone()));
            any = true;
        }
    }
    if any {
        publish(state);
    }
}

/// AI Juggler came to the front or left it: a return from away makes the
/// digest due at once.
pub fn presence_changed(state: &Arc<AppState>) {
    publish(state);
}

/// The next digest's due time now (away read fresh), told to the one
/// scheduler; none with nothing changed or while the user is away.
pub fn publish(state: &AppState) {
    let now = crate::commands::now_millis();
    let due = state.digests.shared.lock().ok().and_then(|mut inner| {
        inner.scheduler.set_away(state.notifier.away(now), now);
        inner.scheduler.due_at()
    });
    state.schedules.set_transient(TRANSIENT, due);
}

/// The battery is critical: on battery and low, or in Low Power Mode on
/// battery (ADR-0036 §1.6: no digest, AI or rollup; what changed waits).
fn battery_critical(state: &AppState) -> bool {
    state
        .machine
        .lock()
        .ok()
        .and_then(|watch| watch.sampler.snapshot().power)
        .is_some_and(|power| power.on_battery && (power.battery_low || power.low_power_mode))
}

type Taken = (Due, BTreeMap<String, Vec<(i64, Observed)>>);

/// The scheduler fired the digest's time: the digest due by now is made
/// (a critical battery defers it five minutes, what changed kept), then
/// the next due time is published.
pub fn due_now(state: &Arc<AppState>) {
    due_now_at(state, crate::commands::now_millis());
}

/// [`due_now`] at `now` (a test's clock).
pub fn due_now_at(state: &Arc<AppState>, now: i64) {
    let taken: Option<Taken> = state.digests.shared.lock().ok().and_then(|mut inner| {
        inner.scheduler.set_away(state.notifier.away(now), now);
        if inner.scheduler.due_at().is_none_or(|at| at > now) {
            return None;
        }
        if battery_critical(state) {
            inner.scheduler.defer(now);
            return None;
        }
        let due = inner.scheduler.take(now)?;
        let events = due
            .agents
            .iter()
            .filter_map(|agent| inner.window.remove(agent).map(|e| (agent.clone(), e)))
            .collect();
        Some((due, events))
    });
    if let Some((due, events)) = taken
        && make(state, &due, events)
        && let Ok(mut inner) = state.digests.shared.lock()
    {
        inner.made += 1;
    }
    publish(state);
}

/// Why the rollup writes this digest alone: the AI digest's flag, else
/// its verdict (D-149). Asking the router starts no engine.
fn why_rollup() -> Why {
    if crate::features::flags::is_on(&crate::features::ai::jobs::DIGEST_ON) {
        Why::Verdict
    } else {
        Why::FlagOff
    }
}

/// How long the map step may take for one digest.
const MAP_WAIT: std::time::Duration = std::time::Duration::from_secs(60);

/// The facts of the agents that changed, and how many open agents didn't.
pub fn gather(
    state: &Arc<AppState>,
    events: BTreeMap<String, Vec<(i64, Observed)>>,
) -> (Vec<AgentFacts>, u32) {
    let rows = crate::features::conversations::titled_snapshot(state);
    let quiet = rows
        .iter()
        .filter(|row| {
            matches!(
                row.status.as_str(),
                "starting" | "running" | "waiting_user" | "stuck"
            ) && !events.contains_key(&row.id)
        })
        .count();
    let agents = events
        .into_iter()
        .map(|(session, events)| {
            let row = rows.iter().find(|row| row.id == session);
            let folder = row.and_then(|row| {
                std::path::Path::new(&row.cwd)
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .filter(|name| !name.is_empty())
            });
            AgentFacts {
                name: row
                    .and_then(|row| row.label.clone())
                    .or(folder)
                    .unwrap_or_else(|| "A session".to_owned()),
                author: DigestAuthorInfo {
                    provider: row.map(|row| row.provider.clone()),
                    folder: row.and_then(|row| row.account_id.clone()),
                    folder_label: row
                        .map(|row| row.account_label.clone())
                        .filter(|label| !label.is_empty()),
                    desktop: row.is_some_and(|row| row.kind == "desktop"),
                },
                files: row
                    .and_then(|row| row.git_dirty_count)
                    .and_then(|n| u32::try_from(n).ok()),
                session,
                events,
                // AJ-E3705 records each turn's end; until then a line has
                // none, and says nothing of turns or tests.
                turns: Vec::new(),
            }
        })
        .collect();
    (agents, u32::try_from(quiet).unwrap_or(u32::MAX))
}

/// Makes one digest: the rollup, stored; the views told; its calm news
/// handed to the notification path. `true` when one was written.
pub fn make(
    state: &Arc<AppState>,
    due: &Due,
    events: BTreeMap<String, Vec<(i64, Observed)>>,
) -> bool {
    let Some(writer) = state.digests.writer(state) else {
        return false;
    };
    make_with(state, writer, due, events)
}

/// [`make`] with the store's writer given (a test's own).
pub fn make_with(
    state: &Arc<AppState>,
    writer: &KindWriter,
    due: &Due,
    events: BTreeMap<String, Vec<(i64, Observed)>>,
) -> bool {
    let on = state.storage.lock().is_ok_and(|storage| enabled(&storage));
    if !on {
        return false;
    }
    let (agents, quiet) = gather(state, events);
    let mut digest = rollup(
        Window {
            start_ms: due.start_ms,
            end_ms: due.end_ms,
        },
        &agents,
        quiet,
    );
    if digest.lines.is_empty() {
        return false;
    }
    // D-149: when the router serves the digest job, its calm lines are the
    // model's picks; the order and counts stay the rollup's.
    let live = crate::features::ai::run::LiveJob {
        service: crate::features::ai::service::shared(),
        state,
        job: crate::features::ai::dto::AiJob::Digest,
    };
    let why = if live.served() {
        let stop = crate::features::ai::engine::Stop::at(std::time::Instant::now() + MAP_WAIT);
        if super::lines::map(&live, &agents, &mut digest, &stop) > 0 {
            Why::Mapped
        } else {
            why_rollup()
        }
    } else {
        why_rollup()
    };
    let now = crate::commands::now_millis();
    let stored = state.storage.lock().is_ok_and(|storage| {
        store::remember(
            &storage,
            writer,
            &crate::features::ai::memory::RedactionSecrets,
            &digest,
            why,
            now,
        )
        .inspect_err(|error| tracing::warn!("the digest wasn't kept: {error}"))
        .is_ok()
    });
    if !stored {
        return false;
    }
    // The views follow: Home's feed reads its first page again.
    crate::features::notifications::changed(state);
    if let Some((title, body)) = notice_words(&digest) {
        crate::features::notifications::hand_over(
            state,
            vec![Item {
                name: title.clone(),
                title,
                body,
                ..Item::new(
                    format!("digest:{}", digest.end_ms),
                    Kind::Digest,
                    Target::Feed,
                    1,
                )
            }],
        );
    }
    true
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;

/// Whether the digest carries the finishes now (ADR-0036 §1.7): it is on
/// and the user isn't away (while away no digest is written, so each
/// finish is told through the batch as before).
#[must_use]
pub fn carries_finishes(state: &AppState, storage: &Storage) -> bool {
    enabled(storage) && !state.notifier.away(crate::commands::now_millis())
}
