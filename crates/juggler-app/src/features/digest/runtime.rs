//! The digest at run time: the scheduler fed by the observation bus, one
//! thread that sleeps until the next digest is due (or until woken, with
//! nothing changed), and a digest made then: the rollup over the window's
//! facts, stored, told to the views (Home's feed reads it), and its calm
//! news handed to the notification path as one notification.
//!
//! The AI digest stays behind its flag and the router (ADR-0034, D-131):
//! `ai_digest_on` is off ("not yet", `v5-final.md`), and no model writer
//! is built, so the rollup writes every digest; the log says why.
//!
//! Lock order: the digest's own lock is taken alone, or before the
//! machine's; never with the store's.

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock, Weak};

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

#[derive(Default)]
struct Inner {
    scheduler: Scheduler,
    window: BTreeMap<String, Vec<(i64, Observed)>>,
    made: u64,
    closed: bool,
}

type Shared = Bell<Inner>;

/// The digest's run-time state, held by the app's state.
#[derive(Default)]
pub struct Digests {
    shared: Arc<Shared>,
    thread: OnceLock<()>,
    writer: OnceLock<Option<KindWriter>>,
}

impl Drop for Digests {
    fn drop(&mut self) {
        self.shared.close(|inner| inner.closed = true);
    }
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
        start_thread(state);
        state.digests.shared.ring();
    }
}

/// AI Juggler came to the front or left it: a return from away makes the
/// digest due at once.
pub fn presence_changed(state: &Arc<AppState>) {
    if state.digests.thread.get().is_some() {
        state.digests.shared.ring();
    }
}

fn start_thread(state: &Arc<AppState>) {
    let digests = &state.digests;
    digests.thread.get_or_init(|| {
        let shared = digests.shared.clone();
        let weak = Arc::downgrade(state);
        let _ = std::thread::Builder::new()
            .name("digest".into())
            .spawn(move || run(&shared, &weak));
    });
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

/// The thread: sleeps until the next digest is due, makes it, and sleeps
/// again; with nothing changed (or the user away) until woken.
fn run(shared: &Shared, state: &Weak<AppState>) {
    loop {
        let taken: Taken = {
            let Ok(mut inner) = shared.lock() else {
                return;
            };
            loop {
                if inner.closed {
                    return;
                }
                let now = crate::commands::now_millis();
                let Some(app) = state.upgrade() else {
                    return;
                };
                inner.scheduler.set_away(app.notifier.away(now), now);
                match inner.scheduler.due_at() {
                    Some(at) if at <= now => {
                        if battery_critical(&app) {
                            inner.scheduler.defer(now);
                            continue;
                        }
                        let Some(due) = inner.scheduler.take(now) else {
                            continue;
                        };
                        let events = due
                            .agents
                            .iter()
                            .filter_map(|agent| {
                                inner.window.remove(agent).map(|e| (agent.clone(), e))
                            })
                            .collect();
                        break (due, events);
                    }
                    due => {
                        drop(app);
                        let Some(next) = shared.sleep(inner, due, now) else {
                            return;
                        };
                        inner = next;
                    }
                }
            }
        };
        let Some(state) = state.upgrade() else {
            return;
        };
        if make(&state, &taken.0, taken.1)
            && let Ok(mut inner) = shared.lock()
        {
            inner.made += 1;
        }
    }
}

/// Why the rollup writes this digest: the AI digest's flag (D-131), and
/// with it on, no model writer in this build. The router's answer for
/// the job is the Models screen's; this never starts an engine.
fn why_rollup() -> Why {
    if crate::features::flags::is_on(&crate::features::ai::jobs::DIGEST_ON) {
        Why::NoWriter
    } else {
        Why::FlagOff
    }
}

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
    let digest = rollup(
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
    let now = crate::commands::now_millis();
    let stored = state.storage.lock().is_ok_and(|storage| {
        store::remember(
            &storage,
            writer,
            &crate::features::ai::memory::RedactionSecrets,
            &digest,
            why_rollup(),
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
