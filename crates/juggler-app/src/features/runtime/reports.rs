//! AJ-E2306 (D-40): terminal sessions' exact state from the tool's own
//! hooks.
//!
//! - **The launch** ([`prepare`]): a terminal launch of a tool that
//!   declares `STATE_REPORTS` gets its adapter's per-launch file in AI
//!   Juggler's overlays (`overlays/runtime/<session>/`, `0700`, files
//!   `0600`), a fresh **report key** in its environment (never its
//!   arguments, and good for reports only: never the agent tools), the
//!   app's socket and the report program. Nothing in the user's setup.
//! - **A report** ([`handle`]): the key names a current session or it is
//!   refused; the event sets the session's exact state — Asks you (with
//!   what it asks), Working, Idle or ended — and links its conversation.
//!   A report can't answer, start, stop or type into anything.

use std::sync::Arc;

use juggler_adapter::reports::ReportsLaunch;
use juggler_adapter::{Capabilities, HarnessAdapter, LaunchSpec};
use juggler_agentbridge::wire::{self, HookReport, Refusal, Report, Response};
use juggler_core::model::{SessionId, Timestamp};

use super::dto::AskInfo;
use crate::features::agent_bridge::{own_dir, session_ended, write_private};
use crate::state::AppState;

/// The per-launch folder in AI Juggler's overlays.
pub const DIR: &str = "runtime";

/// Adds the state reports to a terminal launch AI Juggler is about to
/// start; `false` (the launch as it was) when the tool has none or
/// anything fails: reports never block a launch.
pub fn prepare(
    state: &Arc<AppState>,
    adapter: &dyn HarnessAdapter,
    spec: &mut LaunchSpec,
    session: &SessionId,
) -> bool {
    if !adapter.capabilities().contains(Capabilities::STATE_REPORTS) {
        return false;
    }
    let Some(program) = state.agent_bridge.program() else {
        return false;
    };
    let Ok(socket) = crate::features::agent_bridge::server::ensure_serving(state) else {
        return false;
    };
    let overlays = state.overlays_dir();
    let root = overlays.join(DIR);
    let dir = root.join(session.as_str());
    let launch = ReportsLaunch {
        program: program.clone(),
        dir: dir.clone(),
        overlays,
    };
    let Some(injection) = adapter.state_reports(&launch, &spec.args) else {
        return false;
    };
    let written = own_dir(&root).and_then(|()| own_dir(&dir)).and_then(|()| {
        injection.files.iter().try_for_each(|file| {
            if file.name.contains(['/', '\\']) || file.name.starts_with('.') {
                return Err(std::io::Error::other("a file name with a folder"));
            }
            write_private(&dir.join(&file.name), &file.contents)
        })
    });
    if written.is_err() {
        return false;
    }
    let ended = |id: &str| session_ended(state, id);
    let Some(token) = state
        .agent_bridge
        .mint_reports(&state.data_dir, session, &ended)
    else {
        return false;
    };
    prune(state, &root);
    spec.env.insert(wire::REPORT_TOKEN_ENV.to_owned(), token);
    spec.env.insert(
        wire::SOCKET_ENV.to_owned(),
        socket.to_string_lossy().into_owned(),
    );
    spec.env.insert(
        wire::PROGRAM_ENV.to_owned(),
        program.to_string_lossy().into_owned(),
    );
    spec.args = injection.args;
    true
}

/// Removes the per-launch folders of sessions that hold no key any more.
fn prune(state: &Arc<AppState>, root: &std::path::Path) {
    let keep: std::collections::HashSet<String> = state
        .agent_bridge
        .sessions(&state.data_dir)
        .into_iter()
        .collect();
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) && !keep.contains(&name) {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

/// What a session's hooks said last.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observed {
    /// `working`, `idle`, `asks_you` or `ended`.
    pub state: &'static str,
    /// What it asks, while it asks.
    pub ask: Option<AskInfo>,
    /// Why its last turn failed (the tool's word).
    pub error: Option<String>,
}

/// The ask a report opens (shown with no options: a terminal session is
/// answered in its terminal).
fn ask_of(session: &SessionId, hook: &HookReport, question: bool) -> AskInfo {
    let what = match (&hook.tool, &hook.summary) {
        (Some(tool), Some(summary)) if !question => format!("{tool}: {summary}"),
        (_, Some(summary)) => summary.clone(),
        (Some(tool), None) => tool.clone(),
        (None, None) => "It waits for your answer".to_owned(),
    };
    AskInfo {
        id: format!("hook-{}", Timestamp::now().millis()),
        session_id: session.to_string(),
        kind: if question { "question" } else { "approval" }.to_owned(),
        approval_kind: None,
        summary: what,
        reason: None,
        options: Vec::new(),
        parts: Vec::new(),
        asked_at_ms: Timestamp::now().millis(),
    }
}

/// The state an event leaves, from the one before.
#[must_use]
pub fn next(session: &SessionId, before: Option<&Observed>, hook: &HookReport) -> Observed {
    let keep_ask = || before.and_then(|b| b.ask.clone());
    let working = Observed {
        state: "working",
        ask: None,
        error: None,
    };
    let idle = Observed {
        state: "idle",
        ask: None,
        error: None,
    };
    match hook.event.as_str() {
        "UserPromptSubmit" | "PostToolUse" => working,
        "PermissionRequest" => Observed {
            state: "asks_you",
            ask: Some(ask_of(session, hook, false)),
            error: None,
        },
        "PreToolUse" => Observed {
            state: "asks_you",
            ask: Some(ask_of(session, hook, true)),
            error: None,
        },
        "Notification" => match hook.kind.as_deref() {
            Some("permission_prompt" | "elicitation_dialog" | "agent_needs_input") => Observed {
                state: "asks_you",
                ask: keep_ask().or_else(|| Some(ask_of(session, hook, false))),
                error: None,
            },
            Some("idle_prompt") => idle,
            _ => before.cloned().unwrap_or(idle),
        },
        "StopFailure" => Observed {
            error: hook.reason.clone(),
            ..idle
        },
        "SessionEnd" => Observed {
            state: "ended",
            ask: None,
            error: None,
        },
        // SessionStart, Stop: waiting for the next prompt.
        _ => idle,
    }
}

/// One report, answered.
#[must_use]
pub fn handle(state: &Arc<AppState>, report: &Report) -> Response {
    if report.v != wire::VERSION || !wire::HOOK_EVENTS.contains(&report.hook.event.as_str()) {
        return Response::refused(Refusal::Bad, "AI Juggler couldn't read that report.");
    }
    let Some(session) = state
        .agent_bridge
        .session_of_reports(&state.data_dir, &report.token)
        .filter(|session| !session_ended(state, session.as_str()))
    else {
        return Response::refused(Refusal::Unknown, "This session's report key isn't current.");
    };
    // AJ-E2401: what a command the tool ran started is named on the
    // Processes page ("Started by Bash: npm run dev"), from memory only.
    if report.hook.event == "PostToolUse"
        && let (Some(tool), Some(summary)) = (&report.hook.tool, &report.hook.summary)
    {
        state.processes.note_label(
            session.as_str(),
            &format!("{tool}: {summary}"),
            Timestamp::now().millis(),
        );
    }
    // 0.3: a failed turn may have stopped at the limit (the tool's own
    // record says, in its transcript), and any other event may mean the
    // session carried on after one: looked at before the state is stored,
    // so an ask that follows a limit is stored as an ask.
    if report.hook.event != "SessionEnd" {
        crate::features::sessions::limits::check(state, &session);
    }
    state.runtime.observe(state, &session, &report.hook);
    // E18 (AJ-E1803): "N unread messages from …" at a turn boundary, once
    // per change; facts only, never a message's words.
    let note = juggler_agentbridge::hook::NOTE_EVENTS
        .contains(&report.hook.event.as_str())
        .then(|| crate::features::messages::delivery::note_for(state, &session))
        .flatten();
    Response::ok(serde_json::json!({"note": note}))
}
