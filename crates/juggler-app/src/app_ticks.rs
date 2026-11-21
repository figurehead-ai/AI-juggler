//! The app's background ticks, started once the app is up: the read
//! model's resource and quota refreshes, attention and heuristics, the
//! processes' sampling, the conversations' index and live view, the local
//! caches, the search index and the rest (moved here from `lib.rs`
//! whole, unchanged, for the size budget).

use std::sync::Arc;

use crate::events::{AppEvent, EmitEvent as _};
use crate::{commands, features, history, machine_health, state, ticks};

/// Read-model ticks (AJ-E401 Req.4): `session://updated` stays event-driven
/// (<300 ms via the lifecycle forwarder); these two add the 5 s resource
/// and 30 s quota refreshes as separate, coalesced emissions. Each tick
/// emits at most one payload and nothing at all while no Instance is
/// active, so an idle app costs the UI zero wakeups.
pub(crate) fn spawn_read_model_ticks(
    app: tauri::AppHandle,
    resource_state: Arc<state::AppState>,
    quota_state: Arc<state::AppState>,
    attention_state: Arc<state::AppState>,
    heuristic_state: Arc<state::AppState>,
    processes_state: Arc<state::AppState>,
) {
    use features::processes::running_model::detect_running_models;
    use features::sessions::attention::{attention_recovery_drain, heuristic_tick};
    // Event-driven recovery (AJ-E405 Req.5): drain fresh-output signals
    // at 200 ms — well inside the 300 ms budget. The drain also feeds
    // running-model detection for the freshly-active Instances (AJ-E412
    // Req.2: transcript-tail parse, emitted within the same budget).
    {
        let attention_app = app.clone();
        // poll: AJ-E405 Req.5 — recovery shows within 300 ms; an idle
        // drain finds nothing and returns.
        ticks::every(
            "attention",
            std::time::Duration::from_millis(200),
            move || {
                let _ = attention_recovery_drain(&attention_app, &attention_state);
            },
        );
    }
    // The 15 s heuristic cycle (SPEC §5.1): debounced Stuck/WaitingUser
    // evaluation; quota exits classify immediately at the forwarder.
    {
        let heuristic_app = app.clone();
        ticks::every(
            "heuristics",
            std::time::Duration::from_secs(15),
            move || {
                let _ = heuristic_tick(&heuristic_app, &heuristic_state);
            },
        );
    }
    let quota_app = app.clone();
    let processes_app = app.clone();
    let subagents_app = app.clone();
    let conversations_app = app.clone();
    features::remote_config::spawn(app.clone(), quota_state.clone()); // ADR-0035: daily, after paint.
    let index_state = quota_state.clone();
    ticks::every("resources", std::time::Duration::from_secs(5), move || {
        let _ = features::sessions::read_model::emit_resources(&app, &resource_state);
        // AJ-E1509: the Mac kept awake (and App Nap off) for the sessions
        // working here, before This Mac says so.
        features::app::awake::tick(&resource_state);
        let _ = machine_health::emit_this_mac(&app, &resource_state);
        // D-49: Needs you, worked out here (no window needed); a snooze
        // whose time came brings its need back into every count.
        features::notifications::needs::refresh(&resource_state);
        // AJ-E2912: debug mode past its seven days stops, and says so.
        features::app::diagnostics::debug_log::tick(&resource_state);
    });
    // The 2 s per-tree sampling tick (AJ-E410): collector rings/rows,
    // orphan grace poll, and the opted-in Tier-1 auto-cleanup sweep.
    // Also the running-model safety net (AJ-E412): offset-tracked tail
    // scans skip unchanged transcripts with one metadata read.
    // UI emissions stay on their own coalesced cadences above.
    ticks::every("processes", std::time::Duration::from_secs(2), move || {
        features::processes::tick::processes_tick(&processes_state);
        // AJ-E2401: the Processes view's graph, from what the tick just
        // read (no scan of its own); the page hears of changes.
        if let Some(change) = features::processes::tick(&processes_state) {
            if features::processes::after_change(&processes_state, &change) {
                processes_app.emit_event(AppEvent::ProcessCounts, change.counts.clone());
            }
            processes_app.emit_event(AppEvent::ProcessesChanged, change);
        }
        detect_running_models(&processes_app, &processes_state, None);
        // AJ-E1703: what each live session is working on (its card).
        features::pools::cards::tick(&processes_state);
    });
    // 0.3.3: Keep alive, every 2 s (features::places::keep says when not).
    features::places::spawn(subagents_app.clone(), quota_state.clone());
    // ADR-0028: a switch's automation hand-over, applied as apps close.
    features::handover::automation::spawn(quota_state.clone());
    // AJ-E1520 (0.2): the rail's subagent count, on its own tick.
    features::subagents::count::start(subagents_app.clone(), quota_state.clone());
    // 0.2: an own session of a tool without hooks learns its conversation
    // (one rail row, carrying its count).
    features::sessions::linking::start(subagents_app, quota_state.clone());
    ticks::every("quota", std::time::Duration::from_secs(30), move || {
        let _ = features::sessions::read_model::emit_quota(&quota_app, &quota_state);
        // AJ-E1513: the switch prepared before the limit (never run).
        features::staging::tick(&quota_state, commands::now_millis());
        // AJ-E1907 R2: a task brief is gone within a tick of its hour.
        features::tasks::launch::sweep_briefs(&quota_state, commands::now_millis());
    });
    let live_app = conversations_app.clone();
    features::conversations::spawn_tick(conversations_app, index_state.clone());
    // AJ-E2101: what is open now, from the tools' own session files and
    // the process table (never an environment): small files every 2 s,
    // known pids every 5 s, the whole table at most once a minute. The UI
    // hears of it only when something changed.
    {
        let live_state = index_state.clone();
        // poll: the live view's cadence (scan::TICK, 2 s).
        ticks::every_from_now(
            "live-sessions",
            features::conversations::scan::TICK,
            move || {
                let moved = matches!(features::conversations::live_tick(&live_state), Ok(true));
                // Run 3: a desktop session whose process ended stops
                // counting as open at once (its App's "open now", a
                // running holder or recipient), never after a timer.
                let ended = features::agent_bridge::attest::sweep_now(&live_state);
                if moved || ended {
                    live_app.emit_event(
                        AppEvent::ConversationsChanged,
                        features::conversations::CHANGED_LIVE,
                    );
                }
            },
        );
    }
    // AJ-E1538: the opt-in index of outside sessions' words (idle while off).
    features::outside_search::spawn(index_state.clone());
    // ADR-0023: the local caches, sized by this Mac and trimmed in the
    // background (one cheap pass a minute; folders walked rarely).
    features::cache::start(&index_state);
    // AJ-E3102: every page's counts, built once the app is up.
    features::relations::start(&index_state);
    features::tool_updates::spawn(index_state.clone()); // AJ-E1611: tool versions, daily.
    history::spawn_index(index_state); // AJ-E1007: the transcript search index, every minute.
}
